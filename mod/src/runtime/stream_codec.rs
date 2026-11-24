//! Exact construction and wire-format encoding/decoding of MC 26.3 network `StreamCodec`s.
//!
//! `StreamCodec.composite`, `StreamCodec.map`, `StreamCodec.apply(CodecOperation)`,
//! `ByteBufCodecs.list(...)` / `collection(...)` / `fixedSizeList(...)` / `fixedSizeCollection(...)`
//! / `optional(...)` / `map(...)` / `stringUtf8(...)`, and `CustomPacketPayload.codec` construct
//! exact codec handles retaining their component codecs and callbacks. Calling `encode`/`decode`
//! with an unmodelled/null buffer remains counted on the host; [`encode_bytes`] and
//! [`decode_bytes`] execute exact MC 26.3 `ByteBuf` wire round trips.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::callback_marker;
use crate::runtime::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, exact_class_rules_out,
};
use crate::runtime::{Boxed, Host, MethodRef, NativeError, Runtime, Value};

pub(crate) const STREAM_CODEC: &str = "net/minecraft/network/codec/StreamCodec";
const CODEC_OPERATION: &str = "net/minecraft/network/codec/StreamCodec$CodecOperation";
const STREAM_ENCODER: &str = "net/minecraft/network/codec/StreamEncoder";
const STREAM_DECODER: &str = "net/minecraft/network/codec/StreamDecoder";
const BYTE_BUF_CODECS: &str = "net/minecraft/network/codec/ByteBufCodecs";
const CUSTOM_CODEC: &str = "net/minecraft/network/protocol/common/custom/CustomPacketPayload$1";
const STREAM_CODEC_PREFIX: &str = "net/minecraft/network/codec/StreamCodec$";
const BYTE_BUF_CODECS_STRING: &str = "net/minecraft/network/codec/ByteBufCodecs$14";
const BYTE_BUF_CODECS_OPTIONAL: &str = "net/minecraft/network/codec/ByteBufCodecs$25";
const BYTE_BUF_CODECS_COLLECTION: &str = "net/minecraft/network/codec/ByteBufCodecs$26";
const BYTE_BUF_CODECS_FIXED_COLLECTION: &str = "net/minecraft/network/codec/ByteBufCodecs$27";
const BYTE_BUF_CODECS_MAP: &str = "net/minecraft/network/codec/ByteBufCodecs$28";

const OP_LIST: &str = "list";
const OP_COLLECTION: &str = "collection";
const OP_FIXED_LIST: &str = "fixed-list";
const OP_FIXED_COLLECTION: &str = "fixed-collection";
const ARRAY_LIST_CTOR_SENTINEL: &str = "default:ArrayList";

const MAX_STREAM_DEPTH: usize = 64;
const DEFAULT_STRING_MAX_LEN: i32 = 32767;

static NEXT_CODEC: AtomicU64 = AtomicU64::new(0);

/// The static factories whose body is only allocation/lambda construction.
pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Value> {
    if r.owner == BYTE_BUF_CODECS {
        match (r.name.as_str(), r.descriptor.as_str(), args) {
            ("list", "()Lnet/minecraft/network/codec/StreamCodec$CodecOperation;", []) => {
                return Some(codec_op(OP_LIST, &[Value::Int(i32::MAX)]));
            }
            ("list", "(I)Lnet/minecraft/network/codec/StreamCodec$CodecOperation;", [max]) => {
                return Some(codec_op(OP_LIST, std::slice::from_ref(max)));
            }
            (
                "collection",
                "(Ljava/util/function/IntFunction;)Lnet/minecraft/network/codec/StreamCodec$CodecOperation;",
                [ctor],
            ) => {
                return Some(codec_op(
                    OP_COLLECTION,
                    &[Value::Int(i32::MAX), ctor.clone()],
                ));
            }
            (
                "collection",
                "(Ljava/util/function/IntFunction;I)Lnet/minecraft/network/codec/StreamCodec$CodecOperation;",
                [ctor, max],
            ) => {
                return Some(codec_op(OP_COLLECTION, &[max.clone(), ctor.clone()]));
            }
            (
                "fixedSizeList",
                "(I)Lnet/minecraft/network/codec/StreamCodec$CodecOperation;",
                [size],
            ) => {
                return Some(codec_op(OP_FIXED_LIST, std::slice::from_ref(size)));
            }
            (
                "fixedSizeCollection",
                "(Ljava/util/function/IntFunction;I)Lnet/minecraft/network/codec/StreamCodec$CodecOperation;",
                [ctor, size],
            ) => {
                return Some(codec_op(OP_FIXED_COLLECTION, &[size.clone(), ctor.clone()]));
            }
            (
                "collection",
                "(Ljava/util/function/IntFunction;Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;",
                [ctor, elem],
            ) => {
                return Some(codec(
                    BYTE_BUF_CODECS_COLLECTION,
                    &[Value::Int(i32::MAX), ctor.clone(), elem.clone()],
                ));
            }
            (
                "collection",
                "(Ljava/util/function/IntFunction;Lnet/minecraft/network/codec/StreamCodec;I)Lnet/minecraft/network/codec/StreamCodec;",
                [ctor, elem, max],
            ) => {
                return Some(codec(
                    BYTE_BUF_CODECS_COLLECTION,
                    &[max.clone(), ctor.clone(), elem.clone()],
                ));
            }
            (
                "fixedSizeCollection",
                "(Ljava/util/function/IntFunction;Lnet/minecraft/network/codec/StreamCodec;I)Lnet/minecraft/network/codec/StreamCodec;",
                [ctor, elem, size],
            ) => {
                return Some(codec(
                    BYTE_BUF_CODECS_FIXED_COLLECTION,
                    &[ctor.clone(), size.clone(), elem.clone()],
                ));
            }
            (
                "map",
                "(Ljava/util/function/IntFunction;Lnet/minecraft/network/codec/StreamCodec;Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;",
                [ctor, key, val],
            ) => {
                return Some(codec(
                    BYTE_BUF_CODECS_MAP,
                    &[Value::Int(i32::MAX), key.clone(), val.clone(), ctor.clone()],
                ));
            }
            (
                "map",
                "(Ljava/util/function/IntFunction;Lnet/minecraft/network/codec/StreamCodec;Lnet/minecraft/network/codec/StreamCodec;I)Lnet/minecraft/network/codec/StreamCodec;",
                [ctor, key, val, max],
            ) => {
                return Some(codec(
                    BYTE_BUF_CODECS_MAP,
                    &[max.clone(), key.clone(), val.clone(), ctor.clone()],
                ));
            }
            (
                "optional",
                "(Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;",
                [inner],
            ) => {
                return Some(codec(BYTE_BUF_CODECS_OPTIONAL, std::slice::from_ref(inner)));
            }
            ("stringUtf8", "(I)Lnet/minecraft/network/codec/StreamCodec;", [max]) => {
                return Some(codec(BYTE_BUF_CODECS_STRING, std::slice::from_ref(max)));
            }
            _ => {}
        }
    }

    let result = format!(")L{STREAM_CODEC};");
    if r.descriptor.ends_with(&result) {
        if r.owner == STREAM_CODEC
            && r.name == "composite"
            && args.len() >= 3
            && args.len() % 2 == 1
        {
            let components = (args.len() - 1) / 2;
            // 26.3 has composite arities 1 through 14, implemented by
            // StreamCodec$7 through StreamCodec$20.
            if (1..=14).contains(&components) {
                return Some(codec(
                    &format!("{STREAM_CODEC_PREFIX}{}", components + 6),
                    args,
                ));
            }
        }
        if r.owner == STREAM_CODEC && r.name == "of" {
            return Some(codec(&format!("{STREAM_CODEC_PREFIX}1"), args));
        }
        if r.owner == STREAM_CODEC && r.name == "ofMember" {
            return Some(codec(&format!("{STREAM_CODEC_PREFIX}2"), args));
        }
        if r.owner == STREAM_CODEC
            && r.name == "unit"
            && r.descriptor == "(Ljava/lang/Object;)Lnet/minecraft/network/codec/StreamCodec;"
        {
            return Some(codec(&format!("{STREAM_CODEC_PREFIX}3"), args));
        }
        if r.owner == "net/minecraft/network/protocol/common/custom/CustomPacketPayload"
            && r.name == "codec"
        {
            let class = match r.descriptor.as_str() {
                "(Lnet/minecraft/network/codec/StreamMemberEncoder;Lnet/minecraft/network/codec/StreamDecoder;)Lnet/minecraft/network/codec/StreamCodec;" =>
                {
                    // The two-argument overload delegates to
                    // StreamCodec.ofMember, whose implementation is $2.
                    Some(format!("{STREAM_CODEC_PREFIX}2"))
                }
                "(Lnet/minecraft/network/protocol/common/custom/CustomPacketPayload$FallbackProvider;Ljava/util/List;)Lnet/minecraft/network/codec/StreamCodec;" => {
                    Some(CUSTOM_CODEC.to_owned())
                }
                _ => None,
            }?;
            return Some(codec(&class, args));
        }
    }
    None
}

fn codec_op(kind: &str, args: &[Value]) -> Value {
    let serial = NEXT_CODEC.fetch_add(1, Ordering::Relaxed);
    let mut values = Vec::with_capacity(args.len() + 2);
    values.push(Value::Long(serial as i64));
    values.push(Value::Ref(Some(kind.to_owned())));
    values.extend_from_slice(args);
    Value::Host(HostRef::of_values(HostKind::StreamCodecOperation, &values))
}

fn apply_codec_operation(op: &Value, element_codec: &Value) -> Option<Value> {
    let Value::Host(handle) = op else {
        return None;
    };
    if handle.kind() != HostKind::StreamCodecOperation {
        return None;
    }
    let held = handle.held();
    let Value::Ref(Some(kind)) = held.get(1)? else {
        return None;
    };
    match kind.as_str() {
        OP_LIST => {
            let max_size = held.get(2)?.clone();
            Some(codec(
                BYTE_BUF_CODECS_COLLECTION,
                &[
                    max_size,
                    Value::Ref(Some(ARRAY_LIST_CTOR_SENTINEL.to_owned())),
                    element_codec.clone(),
                ],
            ))
        }
        OP_COLLECTION => {
            let max_size = held.get(2)?.clone();
            let ctor = held.get(3)?.clone();
            Some(codec(
                BYTE_BUF_CODECS_COLLECTION,
                &[max_size, ctor, element_codec.clone()],
            ))
        }
        OP_FIXED_LIST => {
            let size = held.get(2)?.clone();
            Some(codec(
                BYTE_BUF_CODECS_FIXED_COLLECTION,
                &[
                    Value::Ref(Some(ARRAY_LIST_CTOR_SENTINEL.to_owned())),
                    size,
                    element_codec.clone(),
                ],
            ))
        }
        OP_FIXED_COLLECTION => {
            let size = held.get(2)?.clone();
            let ctor = held.get(3)?.clone();
            Some(codec(
                BYTE_BUF_CODECS_FIXED_COLLECTION,
                &[ctor, size, element_codec.clone()],
            ))
        }
        _ => None,
    }
}

/// Run a jar-supplied `CodecOperation` (`op.apply(receiver)`) and return its
/// result unchanged, as vanilla's `operation.apply(this)` does. The operation
/// is a callback (a lambda or method reference) run by the runtime's own
/// callback dispatch, which passes the captures and then the receiver in the
/// JVM's order and handles static, instance and constructor references alike.
/// A modelled static target (`ByteBufCodecs::optional`) is answered by this
/// module's own model first. `None` when the operation is not a callback, so
/// the caller keeps `apply` counted.
///
/// A counted unknown inside the operation still counts `apply`: if any inner
/// call was counted, the result is refused and `apply` stays counted, so there
/// is no silent answer.
fn apply_jar_operation<H: Host>(
    runtime: &mut Runtime<'_, H>,
    op: &Value,
    receiver: &Value,
) -> Result<Option<Value>, NativeError> {
    let Value::Host(handle) = op else {
        return Ok(None);
    };
    if handle.kind() != HostKind::Callback {
        return Ok(None);
    }
    let Some(callback) = callback_marker::lambda_of(op) else {
        return Ok(None);
    };
    if callback.ref_kind == Some(6) {
        let mut values = callback.captures.clone();
        values.push(receiver.clone());
        if let Some(value) = static_call(&callback.target, &values) {
            return Ok(Some(value));
        }
    }
    let before = runtime.host.counted_unknown_calls();
    match runtime.invoke_callback(op, std::slice::from_ref(receiver)) {
        Ok(value) if runtime.host.counted_unknown_calls() == before => Ok(value),
        Ok(_) => Ok(None),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before =>
        {
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

/// A fresh implementation object retaining every constructor argument in
/// declaration order. The serial keeps repeated construction distinct even
/// when the same components are supplied.
fn codec(class: &str, args: &[Value]) -> Value {
    let serial = NEXT_CODEC.fetch_add(1, Ordering::Relaxed);
    let mut values = Vec::with_capacity(args.len() + 2);
    values.push(Value::Long(serial as i64));
    values.push(Value::Ref(Some(class.to_owned())));
    values.extend_from_slice(args);
    Value::Host(HostRef::of_values(HostKind::StreamCodec, &values))
}

/// Pure `StreamCodec` and `StreamCodec$CodecOperation` instance combinators (`cast`, `map`,
/// `apply` with modelled `CodecOperation`s). Calls with unmodelled operations or buffers stay
/// counted.
pub(crate) fn instance_call(
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if let Value::Host(handle) = receiver
        && handle.kind() == HostKind::StreamCodecOperation
        && r.owner == CODEC_OPERATION
        && r.name == "apply"
        && r.descriptor
            == "(Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;"
        && let [target] = args
        && is_stream_codec(target)
    {
        return apply_codec_operation(receiver, target).map(Ok);
    }

    if !is_stream_codec(receiver) {
        return None;
    }
    if class_of_value(receiver).as_deref() == Some("net/minecraft/network/codec/StreamCodec$3") {
        let held = match receiver {
            Value::Host(handle) => handle.held(),
            _ => &[],
        };
        let instance = held.get(2).cloned();
        match r.name.as_str() {
            "decode" if r.descriptor == "(Ljava/lang/Object;)Ljava/lang/Object;" => {
                return instance.map(Ok);
            }
            "encode" if r.descriptor == "(Ljava/lang/Object;Ljava/lang/Object;)V" => {
                let expected = instance?;
                let actual = args.get(1)?.clone();
                if actual == expected {
                    return Some(Ok(Value::Void));
                }
                let text = |value: &Value| match value {
                    Value::Ref(Some(text)) => text.clone(),
                    Value::Int(value) => value.to_string(),
                    Value::Long(value) => value.to_string(),
                    Value::Float(value) => value.to_string(),
                    Value::Void => "null".to_owned(),
                    other => format!("{other:?}"),
                };
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalStateException".into(),
                    message: Some(format!(
                        "Can't encode '{}', expected '{}'",
                        text(&actual),
                        text(&expected)
                    )),
                    frame: None,
                }));
            }
            _ => {}
        }
    }
    if r.owner == STREAM_CODEC
        && r.name == "cast"
        && r.descriptor == "()Lnet/minecraft/network/codec/StreamCodec;"
        && args.is_empty()
    {
        return Some(Ok(receiver.clone()));
    }
    if r.owner == STREAM_CODEC
        && r.name == "apply"
        && r.descriptor
            == "(Lnet/minecraft/network/codec/StreamCodec$CodecOperation;)Lnet/minecraft/network/codec/StreamCodec;"
        && let [op] = args
    {
        return apply_codec_operation(op, receiver).map(Ok);
    }
    if r.owner != STREAM_CODEC
        || r.name != "map"
        || r.descriptor
            != "(Ljava/util/function/Function;Ljava/util/function/Function;)Lnet/minecraft/network/codec/StreamCodec;"
        || args.len() != 2
    {
        return None;
    }
    let mut values = Vec::with_capacity(3);
    values.push(receiver.clone());
    values.extend_from_slice(args);
    Some(Ok(codec(&format!("{STREAM_CODEC_PREFIX}4"), &values)))
}

/// Exact MC 26.3 `StreamCodec$3.encode(buf, value)` (`StreamCodec.unit(expected)`):
/// calls `value.equals(expected)` (so `value == null` throws NPE even when `expected == null`,
/// while `expected == null` and non-null `value` throws `IllegalStateException` with Java's
/// `String.valueOf` of both values). Returns `Ok(false)` if any inner call was counted.
fn encode_unit<H: Host>(
    runtime: &mut Runtime<'_, H>,
    expected: &Value,
    actual: &Value,
) -> Result<bool, NativeError> {
    if matches!(actual, Value::Ref(None)) {
        return Err(NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: Some(
                "Cannot invoke \"Object.equals(Object)\" because \"value\" is null".into(),
            ),
            frame: None,
        });
    }
    let before_eq = runtime.host.counted_unknown_calls();
    let equal = match runtime.call_virtual(
        crate::runtime::jdk_call::Site::Virtual,
        "java/lang/Object",
        "equals",
        "(Ljava/lang/Object;)Z",
        actual,
        std::slice::from_ref(expected),
    ) {
        Ok(Value::Int(value)) if runtime.host.counted_unknown_calls() == before_eq => value != 0,
        Ok(_) => return Ok(false),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before_eq =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if equal {
        return Ok(true);
    }
    let before_str = runtime.host.counted_unknown_calls();
    let actual_text = match runtime.value_of_text(
        actual,
        "Ljava/lang/Object;",
        "net/minecraft/network/codec/StreamCodec$3",
        "encode",
    ) {
        Ok(text) if runtime.host.counted_unknown_calls() == before_str => text,
        Ok(_) => return Ok(false),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before_str =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    let expected_text = match runtime.value_of_text(
        expected,
        "Ljava/lang/Object;",
        "net/minecraft/network/codec/StreamCodec$3",
        "encode",
    ) {
        Ok(text) if runtime.host.counted_unknown_calls() == before_str => text,
        Ok(_) => return Ok(false),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before_str =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    Err(NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: Some(format!(
            "Can't encode '{actual_text}', expected '{expected_text}'"
        )),
        frame: None,
    })
}

/// Runtime-dependent behavior of `StreamCodec$3` (`StreamCodec.unit`). The encoded value's
/// own virtual `equals` is invoked (`value.equals(expected)`); comparing interpreter keys
/// would silently change Java.
pub(crate) fn runtime_instance_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let receiver = receiver?;
    if !is_stream_codec(receiver) {
        return None;
    }
    // `StreamCodec.apply(CodecOperation)`: a modelled `CodecOperation` handle
    // (list, collection, fixed-list or fixed-collection) is answered exactly;
    // otherwise the operation is a jar-supplied callable (a lambda or method
    // reference) and vanilla runs `operation.apply(this)` once, with the
    // receiver's identity, returning its result unchanged. The jar callable is
    // invoked through the interpreter with the receiver as the argument.
    if r.owner == STREAM_CODEC
        && r.name == "apply"
        && r.descriptor
            == "(Lnet/minecraft/network/codec/StreamCodec$CodecOperation;)Lnet/minecraft/network/codec/StreamCodec;"
        && let [op] = args
    {
        // `apply(null)` throws the game's helpful NPE.
        if matches!(op, Value::Ref(None)) {
            return Some(Err(NativeError::UncaughtException {
                class: "java/lang/NullPointerException".into(),
                message: Some(
                    "Cannot invoke \"net.minecraft.network.codec.StreamCodec$CodecOperation.apply(net.minecraft.network.codec.StreamCodec)\" because \"operation\" is null".into(),
                ),
                frame: None,
            }));
        }
        // A modelled `CodecOperation` handle (list, collection, fixed-list or
        // fixed-collection) is answered exactly.
        if let Some(value) = apply_codec_operation(op, receiver) {
            return Some(Ok(value));
        }
        // Otherwise the operation is a jar-supplied callable (a lambda or method
        // reference). Vanilla runs `operation.apply(this)` once, with the
        // receiver's identity, returning its result unchanged. The jar callable
        // is invoked through the interpreter with the receiver as the argument.
        return match apply_jar_operation(runtime, op, receiver) {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        };
    }
    // Everything below is `StreamCodec$3` (`StreamCodec.unit`) only.
    if class_of_value(receiver).as_deref() != Some("net/minecraft/network/codec/StreamCodec$3") {
        return None;
    }
    let expected = match receiver {
        Value::Host(handle) => handle.held().get(2)?.clone(),
        _ => return None,
    };
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("decode", "(Ljava/lang/Object;)Ljava/lang/Object;") => Some(Ok(expected)),
        ("encode", "(Ljava/lang/Object;Ljava/lang/Object;)V") => {
            let actual = args.get(1)?.clone();
            match encode_unit(runtime, &expected, &actual) {
                Ok(true) => Some(Ok(Value::Void)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            }
        }
        _ => None,
    }
}

fn encoder_exception(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "io/netty/handler/codec/EncoderException".into(),
        message: Some(message),
        frame: None,
    }
}

fn decoder_exception(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "io/netty/handler/codec/DecoderException".into(),
        message: Some(message),
        frame: None,
    }
}

fn runtime_exception(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/RuntimeException".into(),
        message: Some(message.to_owned()),
        frame: None,
    }
}

fn index_out_of_bounds(reader_index: usize, length: usize, capacity: usize) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IndexOutOfBoundsException".into(),
        message: Some(format!(
            "readerIndex({reader_index}) + length({length}) exceeds writerIndex({capacity})"
        )),
        frame: None,
    }
}

fn npe() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn write_var_int(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if (v & !0x7f) == 0 {
            out.push(v as u8);
            return;
        }
        out.push(((v & 0x7f) | 0x80) as u8);
        v >>= 7;
    }
}

fn read_var_int(buf: &[u8], pos: &mut usize) -> Result<i32, NativeError> {
    let mut result = 0_i32;
    let mut bytes = 0_i32;
    loop {
        if *pos >= buf.len() {
            return Err(index_out_of_bounds(*pos, 1, buf.len()));
        }
        let b = buf[*pos] as i8;
        *pos += 1;
        result |= i32::from(b & 0x7f) << (bytes * 7);
        bytes += 1;
        if bytes > 5 {
            return Err(runtime_exception("VarInt too big"));
        }
        if (b as u8 & 0x80) == 0 {
            return Ok(result);
        }
    }
}

fn write_var_long(out: &mut Vec<u8>, value: i64) {
    let mut v = value as u64;
    loop {
        if (v & !0x7f) == 0 {
            out.push(v as u8);
            return;
        }
        out.push(((v & 0x7f) | 0x80) as u8);
        v >>= 7;
    }
}

fn read_var_long(buf: &[u8], pos: &mut usize) -> Result<i64, NativeError> {
    let mut result = 0_i64;
    let mut bytes = 0_i32;
    loop {
        if *pos >= buf.len() {
            return Err(index_out_of_bounds(*pos, 1, buf.len()));
        }
        let b = buf[*pos] as i8;
        *pos += 1;
        result |= i64::from(b & 0x7f) << (bytes * 7);
        bytes += 1;
        if bytes > 10 {
            return Err(runtime_exception("VarLong too big"));
        }
        if (b as u8 & 0x80) == 0 {
            return Ok(result);
        }
    }
}

fn read_exact<'a>(buf: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], NativeError> {
    if *pos + len > buf.len() {
        return Err(index_out_of_bounds(*pos, len, buf.len()));
    }
    let slice = &buf[*pos..*pos + len];
    *pos += len;
    Ok(slice)
}

fn write_utf8_string(out: &mut Vec<u8>, text: &str, max_len: i32) -> Result<(), NativeError> {
    let char_len = text.encode_utf16().count() as i32;
    if char_len > max_len {
        return Err(encoder_exception(format!(
            "String too big (was {char_len} characters, max {max_len})"
        )));
    }
    let bytes = text.as_bytes();
    let max_bytes = max_len.wrapping_mul(3);
    if (bytes.len() as i32) > max_bytes {
        return Err(encoder_exception(format!(
            "String too big (was {} bytes encoded, max {max_bytes})",
            bytes.len()
        )));
    }
    write_var_int(out, bytes.len() as i32);
    out.extend_from_slice(bytes);
    Ok(())
}

fn read_utf8_string(buf: &[u8], pos: &mut usize, max_len: i32) -> Result<String, NativeError> {
    let max_bytes = max_len.wrapping_mul(3);
    let byte_len = read_var_int(buf, pos)?;
    if byte_len > max_bytes {
        return Err(decoder_exception(format!(
            "The received encoded string buffer length is longer than maximum allowed ({byte_len} > {max_bytes})"
        )));
    }
    if byte_len < 0 {
        return Err(decoder_exception(
            "The received encoded string buffer length is less than zero! Weird string!".into(),
        ));
    }
    let readable = (buf.len() - *pos) as i32;
    if byte_len > readable {
        return Err(decoder_exception(format!(
            "Not enough bytes in buffer, expected {byte_len}, but got {readable}"
        )));
    }
    let raw = &buf[*pos..*pos + byte_len as usize];
    *pos += byte_len as usize;
    let decoded = String::from_utf8_lossy(raw).into_owned();
    let char_len = decoded.encode_utf16().count() as i32;
    if char_len > max_len {
        return Err(decoder_exception(format!(
            "The received string length is longer than maximum allowed ({char_len} > {max_len})"
        )));
    }
    Ok(decoded)
}

fn invoke_stream_callback<H: Host>(
    runtime: &mut Runtime<'_, H>,
    cb: &Value,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if matches!(cb, Value::Ref(None)) {
        return Err(npe());
    }
    let before = runtime.host.counted_unknown_calls();
    match runtime.invoke_callback(cb, args) {
        Ok(Some(v)) => {
            if runtime.host.counted_unknown_calls() != before {
                Ok(None)
            } else {
                Ok(Some(v))
            }
        }
        Ok(None) => Ok(None),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before =>
        {
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

fn read_collection_elements<H: Host>(
    runtime: &mut Runtime<'_, H>,
    collection: &Value,
) -> Result<Option<Vec<Value>>, NativeError> {
    if matches!(collection, Value::Ref(None)) {
        return Err(npe());
    }
    let before = runtime.host.counted_unknown_calls();
    let size = match runtime.call_virtual(
        crate::runtime::jdk_call::Site::Interface,
        "java/util/Collection",
        "size",
        "()I",
        collection,
        &[],
    ) {
        Ok(Value::Int(n)) if runtime.host.counted_unknown_calls() == before => n,
        Ok(_) => return Ok(None),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before =>
        {
            return Ok(None);
        }
        Err(err) => return Err(err),
    };
    let iter = match runtime.call_virtual(
        crate::runtime::jdk_call::Site::Interface,
        "java/util/Collection",
        "iterator",
        "()Ljava/util/Iterator;",
        collection,
        &[],
    ) {
        Ok(v) if runtime.host.counted_unknown_calls() == before => v,
        Ok(_) => return Ok(None),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before =>
        {
            return Ok(None);
        }
        Err(err) => return Err(err),
    };
    let mut elements = Vec::with_capacity(size.max(0) as usize);
    loop {
        let before_step = runtime.host.counted_unknown_calls();
        let has_next = match runtime.call_virtual(
            crate::runtime::jdk_call::Site::Interface,
            "java/util/Iterator",
            "hasNext",
            "()Z",
            &iter,
            &[],
        ) {
            Ok(Value::Int(n)) if runtime.host.counted_unknown_calls() == before_step => n != 0,
            Ok(_) => return Ok(None),
            Err(NativeError::UncaughtException { .. })
                if runtime.host.counted_unknown_calls() != before_step =>
            {
                return Ok(None);
            }
            Err(err) => return Err(err),
        };
        if !has_next {
            break;
        }
        let next = match runtime.call_virtual(
            crate::runtime::jdk_call::Site::Interface,
            "java/util/Iterator",
            "next",
            "()Ljava/lang/Object;",
            &iter,
            &[],
        ) {
            Ok(v) if runtime.host.counted_unknown_calls() == before_step => v,
            Ok(_) => return Ok(None),
            Err(NativeError::UncaughtException { .. })
                if runtime.host.counted_unknown_calls() != before_step =>
            {
                return Ok(None);
            }
            Err(err) => return Err(err),
        };
        elements.push(next);
    }
    Ok(Some(elements))
}

fn new_collection_from_ctor<H: Host>(
    runtime: &mut Runtime<'_, H>,
    ctor: &Value,
    capacity: i32,
) -> Result<Option<Value>, NativeError> {
    if matches!(ctor, Value::Ref(Some(s)) if s == ARRAY_LIST_CTOR_SENTINEL) {
        let list = runtime.new_object("java/util/ArrayList");
        let init = MethodRef {
            owner: "java/util/ArrayList".into(),
            name: "<init>".into(),
            descriptor: "(I)V".into(),
        };
        let before = runtime.host.counted_unknown_calls();
        match runtime
            .host
            .invoke_instance(&init, &list, &[Value::Int(capacity)])
        {
            Ok(_) if runtime.host.counted_unknown_calls() == before => return Ok(Some(list)),
            Ok(_) => return Ok(None),
            Err(NativeError::UncaughtException { .. })
                if runtime.host.counted_unknown_calls() != before =>
            {
                return Ok(None);
            }
            Err(err) => return Err(err),
        }
    }
    invoke_stream_callback(runtime, ctor, &[Value::Int(capacity)])
}

/// Encode `value` to exact MC 26.3 wire bytes using a modelled `StreamCodec`.
///
/// Returns `Ok(Some(bytes))` on exact completion with zero unknown calls, `Ok(None)` when the
/// codec or a callback depends on unmodelled state, or `Err(NativeError::UncaughtException { .. })`
/// for exact Java exceptions (`EncoderException`, `IllegalStateException`, etc.).
pub(crate) fn encode_bytes<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    value: &Value,
) -> Result<Option<Vec<u8>>, NativeError> {
    let before = runtime.host.counted_unknown_calls();
    let mut out = Vec::new();
    let ok = encode_bytes_depth(runtime, codec, value, &mut out, 0)?;
    if !ok || runtime.host.counted_unknown_calls() != before {
        return Ok(None);
    }
    Ok(Some(out))
}

/// Decode a value from exact MC 26.3 wire bytes using a modelled `StreamCodec`, returning
/// `(decoded_value, bytes_consumed)`.
pub(crate) fn decode_bytes<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    bytes: &[u8],
) -> Result<Option<(Value, usize)>, NativeError> {
    let before = runtime.host.counted_unknown_calls();
    let mut pos = 0_usize;
    let Some(value) = decode_bytes_depth(runtime, codec, bytes, &mut pos, 0)? else {
        return Ok(None);
    };
    if runtime.host.counted_unknown_calls() != before {
        return Ok(None);
    }
    Ok(Some((value, pos)))
}

fn encode_bytes_depth<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    value: &Value,
    out: &mut Vec<u8>,
    depth: usize,
) -> Result<bool, NativeError> {
    if depth > MAX_STREAM_DEPTH {
        return Ok(false);
    }
    if let Some((declared, owner, name)) = crate::runtime::host_codec::static_codec_field(codec)
        && declared == STREAM_CODEC
    {
        return encode_static_codec(&owner, &name, value, out);
    }
    let Value::Host(handle) = codec else {
        return Ok(false);
    };
    if handle.kind() != HostKind::StreamCodec {
        return Ok(false);
    }
    let held = handle.held();
    let Some(Value::Ref(Some(class))) = held.get(1) else {
        return Ok(false);
    };
    match class.as_str() {
        "net/minecraft/network/codec/StreamCodec$3" => {
            let Some(expected) = held.get(2) else {
                return Ok(false);
            };
            encode_unit(runtime, expected, value)
        }
        "net/minecraft/network/codec/StreamCodec$4" => {
            let (Some(inner), Some(from_cb)) = (held.get(2), held.get(4)) else {
                return Ok(false);
            };
            let Some(mapped) =
                invoke_stream_callback(runtime, from_cb, std::slice::from_ref(value))?
            else {
                return Ok(false);
            };
            encode_bytes_depth(runtime, inner, &mapped, out, depth + 1)
        }
        BYTE_BUF_CODECS_STRING => {
            let Some(Value::Int(max_len)) = held.get(2) else {
                return Ok(false);
            };
            match value {
                Value::Ref(None) => Err(npe()),
                Value::Ref(Some(text)) if crate::runtime::value_class::is_plain_string(text) => {
                    write_utf8_string(out, text, *max_len)?;
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        BYTE_BUF_CODECS_OPTIONAL => {
            let Some(inner) = held.get(2) else {
                return Ok(false);
            };
            if matches!(value, Value::Ref(None)) {
                return Err(npe());
            }
            if !crate::runtime::host_optional::is_optional(value) {
                return Ok(false);
            }
            if let Some(present) = crate::runtime::host_optional::optional_present(value) {
                out.push(1);
                encode_bytes_depth(runtime, inner, &present, out, depth + 1)
            } else {
                out.push(0);
                Ok(true)
            }
        }
        BYTE_BUF_CODECS_COLLECTION => {
            let (Some(Value::Int(max_size)), Some(elem_codec)) = (held.get(2), held.get(4)) else {
                return Ok(false);
            };
            let Some(elements) = read_collection_elements(runtime, value)? else {
                return Ok(false);
            };
            let size = elements.len() as i32;
            if size > *max_size {
                return Err(encoder_exception(format!(
                    "{size} elements exceeded max size of: {max_size}"
                )));
            }
            write_var_int(out, size);
            for elem in &elements {
                if !encode_bytes_depth(runtime, elem_codec, elem, out, depth + 1)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        BYTE_BUF_CODECS_FIXED_COLLECTION => {
            let (Some(Value::Int(expected_size)), Some(elem_codec)) = (held.get(3), held.get(4))
            else {
                return Ok(false);
            };
            let Some(elements) = read_collection_elements(runtime, value)? else {
                return Ok(false);
            };
            let size = elements.len() as i32;
            if size != *expected_size {
                return Err(encoder_exception(format!(
                    "Invalid list size, expected {expected_size}, but got {size}"
                )));
            }
            for elem in &elements {
                if !encode_bytes_depth(runtime, elem_codec, elem, out, depth + 1)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        other => {
            let Some(n) = other
                .strip_prefix(STREAM_CODEC_PREFIX)
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|n| (7..=20).contains(n))
            else {
                return Ok(false);
            };
            let arity = n - 6;
            // Held args after [serial, class]: codec1, getter1, ..., codecN, getterN, ctor.
            if held.len() != 2 + arity * 2 + 1 {
                return Ok(false);
            }
            for i in 0..arity {
                let comp_codec = &held[2 + i * 2];
                let getter = &held[2 + i * 2 + 1];
                let Some(part) =
                    invoke_stream_callback(runtime, getter, std::slice::from_ref(value))?
                else {
                    return Ok(false);
                };
                if !encode_bytes_depth(runtime, comp_codec, &part, out, depth + 1)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn encode_static_codec(
    owner: &str,
    name: &str,
    value: &Value,
    out: &mut Vec<u8>,
) -> Result<bool, NativeError> {
    match (owner, name) {
        (BYTE_BUF_CODECS, "BOOL") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Boolean(b)) => {
                out.push(u8::from(*b));
                Ok(true)
            }
            Value::Int(n) => {
                out.push(u8::from(*n != 0));
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "BYTE") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Byte(b)) => {
                out.push(*b as u8);
                Ok(true)
            }
            Value::Int(n) => {
                out.push(*n as u8);
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "SHORT") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Short(s)) => {
                out.extend_from_slice(&s.to_be_bytes());
                Ok(true)
            }
            Value::Int(n) => {
                out.extend_from_slice(&(*n as i16).to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "UNSIGNED_SHORT") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Integer(n)) | Value::Int(n) => {
                out.extend_from_slice(&(*n as u16).to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "INT") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Integer(n)) | Value::Int(n) => {
                out.extend_from_slice(&n.to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "VAR_INT") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Integer(n)) | Value::Int(n) => {
                write_var_int(out, *n);
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "LONG") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Long(n) => {
                out.extend_from_slice(&n.to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "VAR_LONG") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Long(n) => {
                write_var_long(out, *n);
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "FLOAT") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Boxed(Boxed::Float(bits)) => {
                out.extend_from_slice(&bits.to_be_bytes());
                Ok(true)
            }
            Value::Float(v) => {
                out.extend_from_slice(&(*v as f32).to_bits().to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "DOUBLE") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Float(v) => {
                out.extend_from_slice(&v.to_bits().to_be_bytes());
                Ok(true)
            }
            _ => Ok(false),
        },
        (BYTE_BUF_CODECS, "STRING_UTF8") => match value {
            Value::Ref(None) => Err(npe()),
            Value::Ref(Some(text)) if crate::runtime::value_class::is_plain_string(text) => {
                write_utf8_string(out, text, DEFAULT_STRING_MAX_LEN)?;
                Ok(true)
            }
            _ => Ok(false),
        },
        ("net/minecraft/resources/Identifier", "STREAM_CODEC") => {
            if matches!(value, Value::Ref(None)) {
                return Err(npe());
            }
            let Some(id) = crate::runtime::host_ids::identifier_key(value) else {
                return Ok(false);
            };
            write_utf8_string(out, id, DEFAULT_STRING_MAX_LEN)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn decode_bytes_depth<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    buf: &[u8],
    pos: &mut usize,
    depth: usize,
) -> Result<Option<Value>, NativeError> {
    if depth > MAX_STREAM_DEPTH {
        return Ok(None);
    }
    if let Some((declared, owner, name)) = crate::runtime::host_codec::static_codec_field(codec)
        && declared == STREAM_CODEC
    {
        return decode_static_codec(&owner, &name, buf, pos);
    }
    let Value::Host(handle) = codec else {
        return Ok(None);
    };
    if handle.kind() != HostKind::StreamCodec {
        return Ok(None);
    }
    let held = handle.held();
    let Some(Value::Ref(Some(class))) = held.get(1) else {
        return Ok(None);
    };
    match class.as_str() {
        "net/minecraft/network/codec/StreamCodec$3" => Ok(held.get(2).cloned()),
        "net/minecraft/network/codec/StreamCodec$4" => {
            let (Some(inner), Some(to_cb)) = (held.get(2), held.get(3)) else {
                return Ok(None);
            };
            let Some(decoded) = decode_bytes_depth(runtime, inner, buf, pos, depth + 1)? else {
                return Ok(None);
            };
            invoke_stream_callback(runtime, to_cb, &[decoded])
        }
        BYTE_BUF_CODECS_STRING => {
            let Some(Value::Int(max_len)) = held.get(2) else {
                return Ok(None);
            };
            let text = read_utf8_string(buf, pos, *max_len)?;
            Ok(Some(Value::Ref(Some(text))))
        }
        BYTE_BUF_CODECS_OPTIONAL => {
            let Some(inner) = held.get(2) else {
                return Ok(None);
            };
            let b = read_exact(buf, pos, 1)?[0];
            if b != 0 {
                let Some(decoded) = decode_bytes_depth(runtime, inner, buf, pos, depth + 1)? else {
                    return Ok(None);
                };
                if matches!(decoded, Value::Ref(None)) {
                    return Err(npe());
                }
                Ok(Some(crate::runtime::optional_wrap(decoded)))
            } else {
                Ok(Some(crate::runtime::empty_optional()))
            }
        }
        BYTE_BUF_CODECS_COLLECTION => {
            let (Some(Value::Int(max_size)), Some(ctor), Some(elem_codec)) =
                (held.get(2), held.get(3), held.get(4))
            else {
                return Ok(None);
            };
            let count = read_var_int(buf, pos)?;
            if count > *max_size {
                return Err(decoder_exception(format!(
                    "{count} elements exceeded max size of: {max_size}"
                )));
            }
            let init_cap = count.min(65536);
            let Some(collection) = new_collection_from_ctor(runtime, ctor, init_cap)? else {
                return Ok(None);
            };
            for _ in 0..count {
                let Some(elem) = decode_bytes_depth(runtime, elem_codec, buf, pos, depth + 1)?
                else {
                    return Ok(None);
                };
                let before = runtime.host.counted_unknown_calls();
                match runtime.call_virtual(
                    crate::runtime::jdk_call::Site::Interface,
                    "java/util/Collection",
                    "add",
                    "(Ljava/lang/Object;)Z",
                    &collection,
                    &[elem],
                ) {
                    Ok(_) if runtime.host.counted_unknown_calls() == before => {}
                    Ok(_) => return Ok(None),
                    Err(NativeError::UncaughtException { .. })
                        if runtime.host.counted_unknown_calls() != before =>
                    {
                        return Ok(None);
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(Some(collection))
        }
        BYTE_BUF_CODECS_FIXED_COLLECTION => {
            let (Some(ctor), Some(Value::Int(size)), Some(elem_codec)) =
                (held.get(2), held.get(3), held.get(4))
            else {
                return Ok(None);
            };
            let Some(collection) = new_collection_from_ctor(runtime, ctor, *size)? else {
                return Ok(None);
            };
            for _ in 0..*size {
                let Some(elem) = decode_bytes_depth(runtime, elem_codec, buf, pos, depth + 1)?
                else {
                    return Ok(None);
                };
                let before = runtime.host.counted_unknown_calls();
                match runtime.call_virtual(
                    crate::runtime::jdk_call::Site::Interface,
                    "java/util/Collection",
                    "add",
                    "(Ljava/lang/Object;)Z",
                    &collection,
                    &[elem],
                ) {
                    Ok(_) if runtime.host.counted_unknown_calls() == before => {}
                    Ok(_) => return Ok(None),
                    Err(NativeError::UncaughtException { .. })
                        if runtime.host.counted_unknown_calls() != before =>
                    {
                        return Ok(None);
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(Some(collection))
        }
        other => {
            let Some(n) = other
                .strip_prefix(STREAM_CODEC_PREFIX)
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|n| (7..=20).contains(n))
            else {
                return Ok(None);
            };
            let arity = n - 6;
            if held.len() != 2 + arity * 2 + 1 {
                return Ok(None);
            }
            let ctor = &held[2 + arity * 2];
            let mut decoded_parts = Vec::with_capacity(arity);
            for i in 0..arity {
                let comp_codec = &held[2 + i * 2];
                let Some(part) = decode_bytes_depth(runtime, comp_codec, buf, pos, depth + 1)?
                else {
                    return Ok(None);
                };
                decoded_parts.push(part);
            }
            invoke_stream_callback(runtime, ctor, &decoded_parts)
        }
    }
}

fn decode_static_codec(
    owner: &str,
    name: &str,
    buf: &[u8],
    pos: &mut usize,
) -> Result<Option<Value>, NativeError> {
    match (owner, name) {
        (BYTE_BUF_CODECS, "BOOL") => {
            let b = read_exact(buf, pos, 1)?[0];
            Ok(Some(Value::Boxed(Boxed::Boolean(b != 0))))
        }
        (BYTE_BUF_CODECS, "BYTE") => {
            let b = read_exact(buf, pos, 1)?[0] as i8;
            Ok(Some(Value::Boxed(Boxed::Byte(b))))
        }
        (BYTE_BUF_CODECS, "SHORT") => {
            let bytes = read_exact(buf, pos, 2)?;
            let val = i16::from_be_bytes([bytes[0], bytes[1]]);
            Ok(Some(Value::Boxed(Boxed::Short(val))))
        }
        (BYTE_BUF_CODECS, "UNSIGNED_SHORT") => {
            let bytes = read_exact(buf, pos, 2)?;
            let val = u16::from_be_bytes([bytes[0], bytes[1]]);
            Ok(Some(Value::Boxed(Boxed::Integer(i32::from(val)))))
        }
        (BYTE_BUF_CODECS, "INT") => {
            let bytes = read_exact(buf, pos, 4)?;
            let val = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            Ok(Some(Value::Boxed(Boxed::Integer(val))))
        }
        (BYTE_BUF_CODECS, "VAR_INT") => {
            let val = read_var_int(buf, pos)?;
            Ok(Some(Value::Boxed(Boxed::Integer(val))))
        }
        (BYTE_BUF_CODECS, "LONG") => {
            let bytes = read_exact(buf, pos, 8)?;
            let val = i64::from_be_bytes(bytes.try_into().unwrap());
            Ok(Some(Value::Long(val)))
        }
        (BYTE_BUF_CODECS, "VAR_LONG") => {
            let val = read_var_long(buf, pos)?;
            Ok(Some(Value::Long(val)))
        }
        (BYTE_BUF_CODECS, "FLOAT") => {
            let bytes = read_exact(buf, pos, 4)?;
            let bits = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            Ok(Some(Value::Boxed(Boxed::Float(bits))))
        }
        (BYTE_BUF_CODECS, "DOUBLE") => {
            let bytes = read_exact(buf, pos, 8)?;
            let bits = u64::from_be_bytes(bytes.try_into().unwrap());
            Ok(Some(Value::Float(f64::from_bits(bits))))
        }
        (BYTE_BUF_CODECS, "STRING_UTF8") => {
            let text = read_utf8_string(buf, pos, DEFAULT_STRING_MAX_LEN)?;
            Ok(Some(Value::Ref(Some(text))))
        }
        ("net/minecraft/resources/Identifier", "STREAM_CODEC") => {
            let raw = read_utf8_string(buf, pos, DEFAULT_STRING_MAX_LEN)?;
            let Some(id) = crate::runtime::recipe_models::normalize_id(&raw) else {
                return Err(NativeError::UncaughtException {
                    class: "net/minecraft/IdentifierException".into(),
                    message: Some(format!(
                        "Non [a-z0-9/._-] character in path of location: {raw}"
                    )),
                    frame: None,
                });
            };
            Ok(Some(crate::runtime::host_ids::identifier_of_text(&id)))
        }
        _ => Ok(None),
    }
}

/// The implementation class carried by a modeled network codec handle.
fn class_of_value(value: &Value) -> Option<String> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::StreamCodec => {
            Some(class_of_key(handle.key()).to_owned())
        }
        _ => None,
    }
}

/// Whether a value is a modeled network codec handle (either constructed by this module or
/// a canonical static game `StreamCodec` constant such as `ItemStackTemplate.STREAM_CODEC` or
/// `ByteBufCodecs.VAR_INT`). Non-canonical opaque codecs from unmodelled calls return `false`.
pub(crate) fn is_stream_codec(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::StreamCodec)
        || crate::runtime::host_codec::static_codec_field(value)
            .is_some_and(|(declared, ..)| declared == STREAM_CODEC)
}

/// The declared type used by content declarations.
pub(crate) fn codec_type(value: &Value) -> Option<&'static str> {
    is_stream_codec(value).then_some(STREAM_CODEC)
}

pub(crate) fn codec_operation_types(_: &str) -> &'static [&'static str] {
    &[CODEC_OPERATION]
}

fn codec_op_crosses(handle: &HostRef, crosses: &dyn Fn(&Value) -> bool) -> bool {
    handle.held().iter().all(crosses)
}

pub(crate) const CODEC_OPERATION_METHODS: Methods = Methods {
    crosses_jars: codec_op_crosses,
    ..crate::runtime::host_events::DECLARED_TYPE_METHODS
};

pub(crate) fn class_of_key(key: &str) -> &str {
    key.split(',')
        .nth(1)
        .and_then(|marker| marker.strip_prefix('r'))
        .unwrap_or(STREAM_CODEC)
}

fn instance_of_class(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if wanted == class
        || wanted == "java/lang/Object"
        || matches!(wanted, STREAM_CODEC | STREAM_ENCODER | STREAM_DECODER)
    {
        return Some(true);
    }
    if hierarchy.is_instance(class, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

fn stream_codec_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    instance_of_class(class_of_key(handle.key()), wanted, hierarchy)
}

pub(crate) const STREAM_CODEC_METHODS: Methods = Methods {
    instance_of: stream_codec_instance_of,
    ..crate::runtime::host_events::DECLARED_TYPE_METHODS
};

fn owns(class: &str) -> bool {
    matches!(
        class,
        CUSTOM_CODEC
            | BYTE_BUF_CODECS_STRING
            | BYTE_BUF_CODECS_OPTIONAL
            | BYTE_BUF_CODECS_COLLECTION
            | BYTE_BUF_CODECS_FIXED_COLLECTION
            | BYTE_BUF_CODECS_MAP
    ) || class
        .strip_prefix(STREAM_CODEC_PREFIX)
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=20).contains(&number))
}

fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    instance_of_class(class, wanted, hierarchy)
}

fn superclass(_: &str) -> Option<String> {
    Some("java/lang/Object".to_owned())
}

fn is_interface(_: &str) -> Option<bool> {
    Some(false)
}

fn classes() -> Vec<String> {
    let mut classes = (1..=20)
        .map(|number| format!("{STREAM_CODEC_PREFIX}{number}"))
        .collect::<Vec<_>>();
    classes.extend([
        CUSTOM_CODEC.to_owned(),
        BYTE_BUF_CODECS_STRING.to_owned(),
        BYTE_BUF_CODECS_OPTIONAL.to_owned(),
        BYTE_BUF_CODECS_COLLECTION.to_owned(),
        BYTE_BUF_CODECS_FIXED_COLLECTION.to_owned(),
        BYTE_BUF_CODECS_MAP.to_owned(),
    ]);
    classes
}

pub(crate) const STREAM_CODEC_CLASS_SIDE: ClassSide = ClassSide {
    owns,
    instance_of: class_instance_of,
    superclass,
    is_interface,
    classes,
};

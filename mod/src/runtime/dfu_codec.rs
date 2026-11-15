//! Bounded DataFixerUpper codec construction handles (PR1).
//!
//! Construction is exact and structural; encode/decode/parse stay counted until
//! the JsonOps phase. Nodes are HostRefs, so callbacks are held as values rather
//! than re-encoded as strings.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::host_codec;
use crate::runtime::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested};
use crate::runtime::{MethodRef, NativeError, Value};

const CODEC: &str = "com/mojang/serialization/Codec";
const MAP_CODEC: &str = "com/mojang/serialization/MapCodec";
const PRIMITIVE: &str = "com/mojang/serialization/codecs/PrimitiveCodec";
const RECORD_BUILDER: &str = "com/mojang/serialization/codecs/RecordCodecBuilder";
const APP: &str = "com/mojang/datafixers/kinds/App";
const OBJECT: &str = "java/lang/Object";
const MAX_FIELD_NAME_BYTES: usize = 256;
const MAX_NODE_DEPTH: usize = 64;
static NEXT_NODE: AtomicU64 = AtomicU64::new(1);

const CODEC_TYPES: &[&str] = &[
    CODEC,
    "com/mojang/serialization/Encoder",
    "com/mojang/serialization/Decoder",
];
const MAP_TYPES: &[&str] = &[
    MAP_CODEC,
    "com/mojang/serialization/CompressorHolder",
    "com/mojang/serialization/Compressable",
    "com/mojang/serialization/MapDecoder",
    "com/mojang/serialization/MapEncoder",
    "com/mojang/serialization/Keyable",
];
const BUILDER_TYPES: &[&str] = &[RECORD_BUILDER, APP];

fn tag(value: &Value) -> Option<String> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::DfuCodec {
        return None;
    }
    match handle.values()?.first()? {
        Value::Ref(Some(tag)) => Some(tag.clone()),
        _ => None,
    }
}

#[cfg(test)]
fn held(value: &Value) -> Option<Vec<Value>> {
    let Value::Host(handle) = value else {
        return None;
    };
    (handle.kind() == HostKind::DfuCodec)
        .then(|| handle.values())
        .flatten()
}

fn node(tag: &str, values: Vec<Value>) -> Option<Value> {
    if values.len() > MAX_NODE_DEPTH {
        return None;
    }
    let mut held = Vec::with_capacity(values.len() + 2);
    held.push(Value::Ref(Some(tag.to_owned())));
    held.push(Value::Long(NEXT_NODE.fetch_add(1, Ordering::Relaxed) as i64));
    held.extend(values);
    Some(Value::Host(HostRef::of_values(HostKind::DfuCodec, &held)))
}

fn callback(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::Callback)
}

fn map_codec(value: &Value) -> bool {
    match tag(value) {
        Some(name) => name.starts_with("dfu-map-"),
        None => host_codec::opaque_codec_type(value).as_deref() == Some(MAP_CODEC),
    }
}

fn codec_receiver(value: &Value) -> bool {
    if tag(value).is_some() {
        return !map_codec(value);
    }
    matches!(
        host_codec::opaque_codec_type(value).as_deref(),
        Some(CODEC | PRIMITIVE)
    )
}

fn instance_of(handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    let declared = dfu_codec_types(handle.key());
    if wanted == OBJECT
        || declared.contains(&wanted)
        || hierarchy.is_instance(declared[0], wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false)
        || crate::runtime::host_ref::exact_class_rules_out(declared[0], wanted))
    .then_some(false)
}

fn text(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    // DFU's private wrapper classes do not promise a stable structural
    // toString beyond the oracle-covered constants.
    Ok(None)
}

fn crosses(handle: &HostRef, f: &dyn Fn(&Value) -> bool) -> bool {
    handle.held().iter().all(f)
}

pub(crate) const DFU_CODEC_METHODS: Methods = Methods {
    to_string: text,
    hash_code: crate::runtime::host_events::DECLARED_TYPE_METHODS.hash_code,
    equals: crate::runtime::host_events::DECLARED_TYPE_METHODS.equals,
    instance_of,
    crosses_jars: crosses,
    compare_to: None,
};

pub(crate) fn dfu_codec_types(key: &str) -> &'static [&'static str] {
    let tag = key.split(',').next().unwrap_or_default();
    let tag = tag.strip_prefix('r').unwrap_or(tag);
    if tag == "dfu-builder-getter" {
        BUILDER_TYPES
    } else if tag.starts_with("dfu-map-") {
        MAP_TYPES
    } else {
        CODEC_TYPES
    }
}

pub(crate) fn codec_type(value: &Value) -> Option<&'static str> {
    let Value::Host(handle) = value else {
        return None;
    };
    (handle.kind() == HostKind::DfuCodec).then(|| dfu_codec_types(handle.key())[0])
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner != CODEC
        || !matches!(
            r.descriptor.as_str(),
            "Lcom/mojang/serialization/codecs/PrimitiveCodec;"
                | "Lcom/mojang/serialization/Codec;"
                | "Lcom/mojang/serialization/MapCodec;"
        )
    {
        return None;
    }
    let names = [
        "BOOL",
        "BYTE",
        "SHORT",
        "INT",
        "LONG",
        "FLOAT",
        "DOUBLE",
        "STRING",
        "BYTE_BUFFER",
        "INT_STREAM",
        "LONG_STREAM",
        "PASSTHROUGH",
        "EMPTY",
    ];
    names.contains(&r.name.as_str()).then(|| {
        host_codec::static_codec(&r.descriptor[1..r.descriptor.len() - 1], &r.owner, &r.name)
    })
}

fn field_name(args: &[Value]) -> Option<String> {
    let Value::Ref(Some(name)) = args.first()? else {
        return None;
    };
    (name.len() <= MAX_FIELD_NAME_BYTES).then(|| name.clone())
}

pub(crate) fn invoke(r: &MethodRef, receiver: Option<&Value>, args: &[Value]) -> Option<Value> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            CODEC | PRIMITIVE,
            "fieldOf",
            "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;",
        ) => {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            node(
                "dfu-map-field",
                vec![inner, Value::Ref(Some(field_name(args)?))],
            )
        }
        (CODEC | PRIMITIVE, "optionalFieldOf", descriptor)
            if descriptor.starts_with("(Ljava/lang/String;")
                && descriptor.ends_with(")Lcom/mojang/serialization/MapCodec;") =>
        {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let name = field_name(args)?;
            node(
                "dfu-map-optional",
                vec![
                    inner,
                    Value::Ref(Some(name)),
                    args.get(1).cloned().unwrap_or(Value::Ref(None)),
                ],
            )
        }
        (
            MAP_CODEC,
            "forGetter",
            "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;",
        ) => {
            let inner = receiver.filter(|value| map_codec(value))?.clone();
            let getter = args.first().filter(|value| callback(value))?.clone();
            node("dfu-builder-getter", vec![inner, getter])
        }
        (
            CODEC | PRIMITIVE | MAP_CODEC,
            "xmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
        )
        | (
            CODEC | PRIMITIVE | MAP_CODEC,
            "xmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/MapCodec;",
        ) => {
            let inner = receiver?;
            let to = args.first().filter(|value| callback(value))?.clone();
            let from = args.get(1).filter(|value| callback(value))?.clone();
            let kind = if map_codec(inner) {
                "dfu-map-xmap"
            } else {
                "dfu-xmap"
            };
            node(kind, vec![inner.clone(), to, from])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::callback_marker;

    #[test]
    fn nodes_hold_callbacks_and_reject_forged_strings() {
        let callback = callback_marker::lambda_from_text(
            "demo/P",
            0,
            "java/util/function/Function",
            "demo/P#f(Ljava/lang/Object;)Ljava/lang/Object;",
            "",
        );
        let inner = host_codec::static_codec(
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "INT",
        );
        let field = invoke(
            &MethodRef {
                owner: CODEC.into(),
                name: "fieldOf".into(),
                descriptor: "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;".into(),
            },
            Some(&inner),
            &[Value::Ref(Some("n".into()))],
        )
        .unwrap();
        assert_eq!(tag(&field).as_deref(), Some("dfu-map-field"));
        let getter = invoke(
            &MethodRef {
                owner: MAP_CODEC.into(),
                name: "forGetter".into(),
                descriptor:
                    "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;"
                        .into(),
            },
            Some(&field),
            std::slice::from_ref(&callback),
        )
        .unwrap();
        assert_eq!(tag(&getter).as_deref(), Some("dfu-builder-getter"));
        assert!(
            held(&getter)
                .unwrap()
                .iter()
                .any(|value| crate::runtime::callback_marker::callback_text(value).is_some())
        );
        let forged = Value::Ref(Some("dfu-map-field,1".into()));
        assert!(invoke(
            &MethodRef {
                owner: MAP_CODEC.into(),
                name: "forGetter".into(),
                descriptor:
                    "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;"
                        .into(),
            },
            Some(&forged),
            std::slice::from_ref(&callback),
        )
        .is_none());
    }
}

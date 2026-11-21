//! Vanilla 26.3's `net.minecraft.world.item.crafting.RecipeSerializer` record.
//!
//! Verified from `server-26.3.jar` (`javap -c -p -v`):
//!
//! ```java
//! public final class RecipeSerializer<T extends Recipe<?>> extends Record {
//!     private final MapCodec<T> codec;
//!     private final StreamCodec<RegistryFriendlyByteBuf, T> streamCodec;
//!     public RecipeSerializer(MapCodec<T> codec, StreamCodec<RegistryFriendlyByteBuf, T> streamCodec);
//!     public MapCodec<T> codec();
//!     public StreamCodec<RegistryFriendlyByteBuf, T> streamCodec();
//!     // generated record equals / hashCode / toString via ObjectMethods.bootstrap
//! }
//! ```
//!
//! The constructor stores both components unchecked (either may be `null`). Each
//! `new RecipeSerializer(codec, streamCodec)` allocates a distinct canonical
//! [`HostKind::RecipeSerializer`] handle carrying a monotonic serial plus the
//! two component values so reference identity (`==`, `ReferenceOpenHashSet`)
//! stays distinct from record `equals`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, exact_class_rules_out};
use crate::runtime::{Host, MethodRef, NativeError, Runtime, Value, jar_object};

pub(crate) const OWNER: &str = "net/minecraft/world/item/crafting/RecipeSerializer";
const INIT_DESC: &str =
    "(Lcom/mojang/serialization/MapCodec;Lnet/minecraft/network/codec/StreamCodec;)V";
const CODEC_DESC: &str = "()Lcom/mojang/serialization/MapCodec;";
const STREAM_CODEC_DESC: &str = "()Lnet/minecraft/network/codec/StreamCodec;";

static NEXT_SERIALIZER: AtomicU64 = AtomicU64::new(0);

/// Allocate a fresh `RecipeSerializer` record handle holding `(codec, stream_codec)`.
pub(crate) fn make(codec: &Value, stream_codec: &Value) -> Value {
    let serial = NEXT_SERIALIZER.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::of_values(
        HostKind::RecipeSerializer,
        &[
            Value::Long(serial as i64),
            codec.clone(),
            stream_codec.clone(),
        ],
    ))
}

/// Extract `(codec, streamCodec)` from a `RecipeSerializer` handle.
pub(crate) fn parts(handle: &HostRef) -> Option<(Value, Value)> {
    if handle.kind() != HostKind::RecipeSerializer {
        return None;
    }
    match <[Value; 3]>::try_from(handle.values()?) {
        Ok([_, codec, stream_codec]) => Some((codec, stream_codec)),
        Err(_) => None,
    }
}

/// Extract `(codec, streamCodec)` from a `Value` when it is a `RecipeSerializer` handle.
fn value_parts(value: &Value) -> Option<(Value, Value)> {
    match value {
        Value::Host(handle) => parts(handle),
        _ => None,
    }
}

/// Constructor reference (`RecipeSerializer::new`) when the jar does not ship its own class file.
pub(crate) fn construct<H: Host>(
    runtime: &Runtime<'_, H>,
    r: &MethodRef,
    args: &[Value],
) -> Option<Value> {
    if r.owner != OWNER || r.name != "<init>" || r.descriptor != INIT_DESC {
        return None;
    }
    if runtime.class(OWNER).is_some() {
        return None;
    }
    let [codec, stream_codec] = args else {
        return None;
    };
    Some(make(codec, stream_codec))
}

/// Runtime dispatch for `RecipeSerializer.<init>`, `codec()`, and `streamCodec()`.
pub(crate) fn runtime_call<H: Host>(
    runtime: &Runtime<'_, H>,
    r: &MethodRef,
    op: u8,
    receiver: Option<&Value>,
    args: &[Value],
    stack: &mut [Value],
    locals: &mut [Value],
) -> Option<Value> {
    if r.owner != OWNER {
        return None;
    }
    let receiver = receiver?;
    if op == 0xb7 && r.name == "<init>" && r.descriptor == INIT_DESC {
        if runtime.class(OWNER).is_some() || jar_object::object_class(receiver) != Some(OWNER) {
            return None;
        }
        let [codec, stream_codec] = args else {
            return None;
        };
        let built = make(codec, stream_codec);
        for slot in stack.iter_mut().chain(locals.iter_mut()) {
            if slot == receiver {
                *slot = built.clone();
            }
        }
        return Some(Value::Void);
    }
    instance_call(r, receiver, args)
}

/// Host/runtime instance dispatch for `RecipeSerializer.codec()` and `streamCodec()`.
pub(crate) fn instance_call(r: &MethodRef, receiver: &Value, args: &[Value]) -> Option<Value> {
    if r.owner != OWNER || !args.is_empty() {
        return None;
    }
    let (codec, stream_codec) = value_parts(receiver)?;
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("codec", CODEC_DESC) => Some(codec),
        ("streamCodec", STREAM_CODEC_DESC) => Some(stream_codec),
        _ => None,
    }
}

fn to_string(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    let Some((codec, stream_codec)) = parts(handle) else {
        return Ok(None);
    };
    let Some(codec_text) = nested.text(&codec)? else {
        return Ok(None);
    };
    let Some(stream_text) = nested.text(&stream_codec)? else {
        return Ok(None);
    };
    Ok(Some(format!(
        "RecipeSerializer[codec={codec_text}, streamCodec={stream_text}]"
    )))
}

fn hash_code(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    let Some((codec, stream_codec)) = parts(handle) else {
        return Ok(None);
    };
    let Some(codec_hash) = nested.hash(&codec)? else {
        return Ok(None);
    };
    let Some(stream_hash) = nested.hash(&stream_codec)? else {
        return Ok(None);
    };
    Ok(Some(codec_hash.wrapping_mul(31).wrapping_add(stream_hash)))
}

fn equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some((codec, stream_codec)) = parts(handle) else {
        return Ok(None);
    };
    let (other_codec, other_stream_codec) = match other {
        Value::Host(other) if other.kind() == HostKind::RecipeSerializer => {
            if handle.same_object(other) == Some(true) {
                return Ok(Some(true));
            }
            let Some(fields) = parts(other) else {
                return Ok(None);
            };
            fields
        }
        Value::Host(other) if other.kind() == HostKind::GameTransient => return Ok(None),
        Value::Ref(Some(text)) if !crate::runtime::value_class::is_plain_string(text) => return Ok(None),
        _ => return Ok(Some(false)),
    };
    // JDK 25 `ObjectMethods.bootstrap` compares record components last-to-first.
    match nested.equals(&stream_codec, &other_stream_codec)? {
        Some(false) => Ok(Some(false)),
        Some(true) => nested.equals(&codec, &other_codec),
        None => Ok(None),
    }
}

fn instance_of(_: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if matches!(wanted, OWNER | "java/lang/Record" | "java/lang/Object") {
        return Some(true);
    }
    if hierarchy.is_instance(OWNER, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(OWNER, wanted))
        .then_some(false)
}

fn crosses_jars(handle: &HostRef, crosses: &dyn Fn(&Value) -> bool) -> bool {
    parts(handle).is_some_and(|(codec, stream_codec)| crosses(&codec) && crosses(&stream_codec))
}

pub(crate) const METHODS: Methods = Methods {
    to_string,
    hash_code,
    equals,
    instance_of,
    crosses_jars,
    compare_to: None,
};

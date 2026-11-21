//! Exact Fabric API phase identifiers and ArrayBackedEvent validation.
//!
//! Registrations remain invocation-log data for the existing events::order
//! sorter. This module answers canonical phase identifiers and validates
//! Fabric Event calls without accepting forged receivers or ids.

use crate::host_ref::HostKind;
use crate::{Host, MethodRef, NativeError, Runtime, Value, host_ids};

fn thrown(class: &str, message: &str) -> Result<Value, NativeError> {
    Err(NativeError::UncaughtException {
        class: class.to_owned(),
        message: Some(message.to_owned()),
        frame: None,
    })
}

fn is_null(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Ref(None)) | None)
}

fn identifier_key(value: &Value) -> Option<&str> {
    host_ids::identifier_key(value)
}

fn event_receiver(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::FabricEvent)
}

fn same_identifier(first: &Value, second: &Value) -> bool {
    identifier_key(first)
        .zip(identifier_key(second))
        .is_some_and(|(first, second)| first == second)
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    let id = match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            "net/fabricmc/fabric/api/message/v1/ServerMessageDecoratorEvent",
            "CONTENT_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:content",
        (
            "net/fabricmc/fabric/api/message/v1/ServerMessageDecoratorEvent",
            "STYLING_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:styling",
        (
            "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization",
            "RECIPE_SYNC_EVENT_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:recipe_sync",
        _ => return None,
    };
    Some(host_ids::static_identifier(id))
}

pub(crate) fn event_call(
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/fabricmc/fabric/api/event/Event" || !event_receiver(receiver) {
        return None;
    }
    match r.name.as_str() {
        "register" => {
            if args.len() >= 2 {
                if is_null(args.first()) {
                    return Some(thrown(
                        "java/lang/NullPointerException",
                        "Tried to register a listener for a null phase!",
                    ));
                }
                identifier_key(args.first()?)?;
                if is_null(args.last()) {
                    return Some(thrown(
                        "java/lang/NullPointerException",
                        "Tried to register a null listener!",
                    ));
                }
            } else if is_null(args.first()) {
                return Some(thrown(
                    "java/lang/NullPointerException",
                    "Tried to register a null listener!",
                ));
            }
            Some(Ok(Value::Void))
        }
        "addPhaseOrdering" => {
            if is_null(args.first()) || is_null(args.get(1)) {
                return Some(thrown(
                    "java/lang/NullPointerException",
                    "Tried to add an ordering for a null phase.",
                ));
            }
            if identifier_key(args.first()?).is_none() || identifier_key(args.get(1)?).is_none() {
                return None;
            }
            if let [first, second] = args
                && same_identifier(first, second)
            {
                return Some(thrown(
                    "java/lang/IllegalArgumentException",
                    "Tried to add a phase that depends on itself.",
                ));
            }
            Some(Ok(Value::Void))
        }
        _ => None,
    }
}

const RECIPE_SYNCHRONIZATION: &str = "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization";
const SYNC_METHOD: &str = "synchronizeRecipeSerializer";
const SYNC_DESCRIPTOR: &str = "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V";
const STREAM_CODEC_DESCRIPTOR: &str = "()Lnet/minecraft/network/codec/StreamCodec;";

/// Canonical reference-identity key for Fabric's `ReferenceOpenHashSet`
/// (`HostRef::encode()` includes the `HostKind` tag, so distinct handle
/// families never collide). Non-canonical handles (`GameTransient`,
/// `GameValue`, non-canonical `OpaqueCodec`, ...) and non-object values return
/// `None` and stay counted.
pub(crate) fn canonical_serializer_key(value: &Value) -> Option<String> {
    match value {
        Value::Host(handle) if handle.canonical() => Some(handle.encode()),
        _ => None,
    }
}

/// Fabric's `RecipeSynchronization.synchronizeRecipeSerializer(RecipeSerializer)`
/// (fabric-recipe-api-v1 10.0.9, `javap -c`):
///
/// 1. Offsets 0-6: `Objects.requireNonNull(serializer, "serializer can't be null!")`.
/// 2. Offsets 7-16: `Objects.requireNonNull(serializer.streamCodec(), "StreamCodec can't be null!")`,
///    dispatched through [`Runtime::call_virtual`] so record accessors and any
///    jar override (including inherited jar superclasses) execute normally.
/// 3. Offsets 17-21: `RecipeSyncImpl.addSynchronizedSerializer(serializer)`
///    into a fastutil `ReferenceOpenHashSet` (static strong root).
///
/// Descriptor, arity, host support, and canonical identity are checked before
/// invoking `streamCodec()` so refused calls have no side effects. If
/// `streamCodec()` gets counted (for instance returning a placeholder
/// `OpaqueCodec`), `synchronizeRecipeSerializer` is counted too and nothing is
/// registered. Direct calls to `RecipeSyncImpl.addSynchronizedSerializer` stay
/// counted alongside `isSynced` and `getSyncedSerializers`.
pub(crate) fn invoke_recipe_sync<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    op: u8,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if op != 0xb8
        || receiver.is_some()
        || r.owner != RECIPE_SYNCHRONIZATION
        || r.name != SYNC_METHOD
        || r.descriptor != SYNC_DESCRIPTOR
        || args.len() != 1
        || !runtime.host.supports_recipe_sync()
    {
        return Ok(None);
    }
    let serializer = &args[0];
    if matches!(serializer, Value::Ref(None)) {
        return Err(NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: Some("serializer can't be null!".into()),
            frame: None,
        });
    }
    let Some(key) = canonical_serializer_key(serializer) else {
        return runtime.host.unknown_operand_call(r).map(Some);
    };
    let unknown_before = runtime.host.counted_unknown_calls();
    let codec = runtime.call_virtual(
        crate::jdk_call::Site::Virtual,
        crate::recipe_serializer::OWNER,
        "streamCodec",
        STREAM_CODEC_DESCRIPTOR,
        serializer,
        &[],
    )?;
    if runtime.host.counted_unknown_calls() > unknown_before {
        return runtime.host.unknown_operand_call(r).map(Some);
    }
    if matches!(codec, Value::Ref(None)) {
        return Err(NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: Some("StreamCodec can't be null!".into()),
            frame: None,
        });
    }
    runtime
        .host
        .record_synced_recipe_serializer(key, serializer.clone());
    Ok(Some(Value::Void))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_ref::HostRef;

    fn event() -> Value {
        Value::Host(HostRef::new(HostKind::FabricEvent, "array,0"))
    }

    fn call(name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: "net/fabricmc/fabric/api/event/Event".into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    #[test]
    fn forged_receivers_and_string_phases_stay_counted() {
        let register = call(
            "register",
            "(Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)V",
        );
        let order = call(
            "addPhaseOrdering",
            "(Lnet/minecraft/resources/Identifier;Lnet/minecraft/resources/Identifier;)V",
        );
        let string = Value::Ref(Some("fabric:default".into()));
        let listener = Value::Ref(Some("listener".into()));
        assert!(
            event_call(
                &register,
                &Value::Ref(Some("event".into())),
                &[string.clone(), listener.clone()]
            )
            .is_none()
        );
        assert!(
            event_call(
                &order,
                &Value::Ref(Some("event".into())),
                &[string.clone(), string.clone()]
            )
            .is_none()
        );
        assert!(event_call(&order, &event(), &[string.clone(), string]).is_none());
    }

    #[test]
    fn identifier_handles_validate_self_ordering() {
        let order = call(
            "addPhaseOrdering",
            "(Lnet/minecraft/resources/Identifier;Lnet/minecraft/resources/Identifier;)V",
        );
        let id = host_ids::identifier("demo", "phase");
        assert!(matches!(
            event_call(&order, &event(), &[id.clone(), id]),
            Some(Err(NativeError::UncaughtException { class, message: Some(message), .. }))
                if class == "java/lang/IllegalArgumentException"
                    && message == "Tried to add a phase that depends on itself."
        ));
        let static_id = host_ids::static_identifier("fabric:default");
        assert!(matches!(
            event_call(&order, &event(), &[static_id.clone(), static_id]),
            Some(Err(NativeError::UncaughtException { class, message: Some(message), .. }))
                if class == "java/lang/IllegalArgumentException"
                    && message == "Tried to add a phase that depends on itself."
        ));
    }

    #[test]
    fn canonical_serializer_keys_include_kind_and_reject_non_canonical_values() {
        let jar_obj = Value::Host(HostRef::new(HostKind::JarObject, "shared"));
        let reg_obj = Value::Host(HostRef::new(HostKind::GameRegistryObject, "shared"));
        let key_jar = canonical_serializer_key(&jar_obj).expect("JarObject is canonical");
        let key_reg = canonical_serializer_key(&reg_obj).expect("GameRegistryObject is canonical");
        assert_ne!(key_jar, key_reg, "distinct HostKinds must never collide");

        let ser_a = crate::recipe_serializer::make(&Value::Ref(None), &Value::Ref(None));
        let ser_b = crate::recipe_serializer::make(&Value::Ref(None), &Value::Ref(None));
        let key_a = canonical_serializer_key(&ser_a).expect("RecipeSerializer is canonical");
        let key_b = canonical_serializer_key(&ser_b).expect("RecipeSerializer is canonical");
        assert_ne!(
            key_a, key_b,
            "two equal records have distinct reference identity"
        );

        // Non-canonical handles and non-object values are rejected.
        for rejected in [
            Value::Host(HostRef::new(HostKind::GameTransient, "game/transient/0")),
            Value::Host(HostRef::new(HostKind::GameValue, "block-pos/0,0,0")),
            crate::host_codec::opaque_codec("com/mojang/serialization/MapCodec"),
            host_ids::identifier("demo", "id"),
            Value::Ref(Some("demo/Serializer#0".into())),
            Value::Int(7),
            Value::Void,
        ] {
            assert_eq!(
                canonical_serializer_key(&rejected),
                None,
                "{rejected:?} must not produce a canonical identity key"
            );
        }
    }
}

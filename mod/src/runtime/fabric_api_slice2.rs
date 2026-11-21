//! Exact Fabric API phase identifiers and ArrayBackedEvent validation.
//!
//! Registrations remain invocation-log data for the existing events::order
//! sorter. This module answers canonical phase identifiers and validates
//! Fabric Event calls without accepting forged receivers or ids.

use crate::runtime::host_ref::HostKind;
use crate::runtime::{MethodRef, NativeError, Value, host_ids};
use std::collections::BTreeSet;

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

/// The Fabric recipe synchronizer keeps a reference-identity set of serializers.
/// Its public entry point validates only the serializer and its stream codec;
/// registration itself is an idempotent set insertion and returns `void`.
pub(crate) fn recipe_sync_call(
    r: &MethodRef,
    args: &[Value],
    stream_codec: Option<Result<Value, NativeError>>,
    synced: &mut BTreeSet<String>,
) -> Option<Result<Value, NativeError>> {
    let is_add = r.owner == "net/fabricmc/fabric/impl/recipe/sync/RecipeSyncImpl"
        && r.name == "addSynchronizedSerializer"
        && r.descriptor == "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V";
    let is_public = r.owner == "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization"
        && r.name == "synchronizeRecipeSerializer"
        && r.descriptor == "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V";
    if !is_add && !is_public {
        return None;
    }
    let serializer = args.first();
    if is_public && matches!(serializer, None | Some(Value::Ref(None))) {
        return Some(thrown(
            "java/lang/NullPointerException",
            "serializer can't be null!",
        ));
    }
    if is_public {
        match stream_codec {
            Some(Ok(Value::Ref(None))) => {
                return Some(thrown(
                    "java/lang/NullPointerException",
                    "StreamCodec can't be null!",
                ));
            }
            Some(Ok(_)) => {}
            Some(Err(error)) => return Some(Err(error)),
            None => return None,
        }
    }
    if let Some(value) = serializer {
        let key = match value {
            Value::Host(handle) => handle.key().to_owned(),
            _ => return None,
        };
        synced.insert(key);
    }
    Some(Ok(Value::Void))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::host_ref::HostRef;

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
    fn recipe_serializer_registration_is_idempotent_and_null_checked() {
        let method = MethodRef {
            owner: "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization".into(),
            name: "synchronizeRecipeSerializer".into(),
            descriptor: "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V".into(),
        };
        let mut synced = BTreeSet::new();
        let serializer = Value::Host(HostRef::new(HostKind::JarObject, "demo/Serializer#0"));
        assert!(matches!(
            recipe_sync_call(
                &method,
                std::slice::from_ref(&serializer),
                Some(Ok(Value::Ref(Some("codec".into())))),
                &mut synced,
            ),
            Some(Ok(Value::Void))
        ));
        assert!(matches!(
            recipe_sync_call(
                &method,
                std::slice::from_ref(&serializer),
                Some(Ok(Value::Ref(Some("codec".into())))),
                &mut synced,
            ),
            Some(Ok(Value::Void))
        ));
        assert_eq!(
            synced.into_iter().collect::<Vec<_>>(),
            ["demo/Serializer#0"]
        );
        assert!(matches!(
            recipe_sync_call(&method, &[Value::Ref(None)], None, &mut BTreeSet::new()),
            Some(Err(NativeError::UncaughtException { class, message: Some(message), .. }))
                if class == "java/lang/NullPointerException" && message == "serializer can't be null!"
        ));
    }

    #[test]
    fn recipe_serializer_codec_null_and_throw_propagate_exactly() {
        let method = MethodRef {
            owner: "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization".into(),
            name: "synchronizeRecipeSerializer".into(),
            descriptor: "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V".into(),
        };
        let serializer = Value::Host(HostRef::new(HostKind::JarObject, "demo/Serializer#0"));
        let mut synced = BTreeSet::new();
        assert!(matches!(
            recipe_sync_call(
                &method,
                std::slice::from_ref(&serializer),
                Some(Ok(Value::Ref(None))),
                &mut synced,
            ),
            Some(Err(NativeError::UncaughtException { class, message: Some(message), .. }))
                if class == "java/lang/NullPointerException" && message == "StreamCodec can't be null!"
        ));
        let thrown = NativeError::UncaughtException {
            class: "java/lang/IllegalStateException".into(),
            message: Some("codec boom".into()),
            frame: None,
        };
        assert!(matches!(
            recipe_sync_call(
                &method,
                std::slice::from_ref(&serializer),
                Some(Err(thrown)),
                &mut synced,
            ),
            Some(Err(NativeError::UncaughtException { class, message: Some(message), .. }))
                if class == "java/lang/IllegalStateException" && message == "codec boom"
        ));
        assert!(synced.is_empty());
    }
}

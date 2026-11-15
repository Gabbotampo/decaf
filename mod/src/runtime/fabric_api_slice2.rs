//! Exact Fabric API phase identifiers and ArrayBackedEvent validation.
//!
//! Registrations remain invocation-log data for the existing events::order
//! sorter. This module answers canonical phase identifiers and validates
//! Fabric Event calls without accepting forged receivers or ids.

use crate::runtime::host_ref::HostKind;
use crate::runtime::{MethodRef, NativeError, Value, host_ids};

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
                if identifier_key(args.first()?).is_none() {
                    return None;
                }
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
}

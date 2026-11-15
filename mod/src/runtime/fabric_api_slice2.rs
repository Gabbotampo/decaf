//! Exact Fabric API phase identifiers and ArrayBackedEvent validation.
//!
//! The event's listener and phase data stays in the invocation log. The
//! existing event reader and events::order sorter consume that log for
//! named Fabric fields, including cycles. This module only answers the
//! canonical phase identifiers and the validation that Fabric performs before
//! recording a registration.

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

fn same_identifier(first: &Value, second: &Value) -> bool {
    match (
        host_ids::identifier_key(first),
        host_ids::identifier_key(second),
    ) {
        (Some(first), Some(second)) => first == second,
        _ => first == second,
    }
}

/// Canonical Fabric phase identifiers. Other Fabric API calls deliberately
/// fall through to the main host and remain counted there.
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

/// Fabric's validation for an Event registration or phase ordering.
///
/// The actual registrations remain invocation-log data, which the existing
/// event reader feeds to events::order::sort_phases. In particular, a cycle
/// is retained as a strongly connected component and does not loop forever.
pub(crate) fn event_call(
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/fabricmc/fabric/api/event/Event" {
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

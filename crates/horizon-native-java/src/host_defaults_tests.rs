//! The `Host` trait's defaults never answer a Java value silently.
//!
//! [`NullHost`] implements nothing, so every member it is asked is the
//! trait's default. A default that would answer the jar (a field, a call, a
//! counted operand) errors with [`NativeError::UnknownHostCall`]; a default
//! that says "this host keeps no such state" must lead its callers to a
//! counted path, which `tests/fixture_host_defaults_loud.rs` runs end to end.

use crate::mod_count::{ModStamp, ModVerdict};
use crate::{Host, MethodRef, NativeError, NullHost, Value};

fn method(owner: &str, name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: owner.into(),
        name: name.into(),
        descriptor: descriptor.into(),
    }
}

fn refused(answer: Result<Value, NativeError>, r: &MethodRef) {
    match answer {
        Err(NativeError::UnknownHostCall(called)) => assert_eq!(&called, r),
        other => panic!("{}#{} answered {other:?}", r.owner, r.name),
    }
}

#[test]
fn the_defaults_that_would_answer_the_jar_refuse() {
    let mut host = NullHost;
    let field = method(
        "net/minecraft/world/item/Items",
        "DIAMOND",
        "Lnet/minecraft/world/item/Item;",
    );
    let call = method("lib/Lib", "answer", "()I");
    let to_string = method("java/lang/Object", "toString", "()Ljava/lang/String;");
    let receiver = Value::Ref(Some("demo".into()));
    refused(host.get_static(&field), &field);
    refused(host.get_field(&receiver, &field), &field);
    refused(host.invoke(&call, &[]), &call);
    refused(host.invoke_instance(&call, &receiver, &[]), &call);
    refused(host.unknown_operand_call(&to_string), &to_string);
    refused(host.send_feedback(&call, &receiver, false), &call);
}

#[test]
fn a_host_that_keeps_no_mod_counts_makes_every_check_counted() {
    let host = NullHost;
    let live = host.mod_stamp("java/util/ArrayList#0");
    // Neither "unchanged" (no `ConcurrentModificationException` missed
    // silently) nor "changed" (none thrown that Java would not throw).
    assert_eq!(live.verdict(live), ModVerdict::Unknown);
    assert_eq!(live.verdict(ModStamp::default()), ModVerdict::Unknown);
    assert_eq!(ModStamp::default().verdict(live), ModVerdict::Unknown);
}

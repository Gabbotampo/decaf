//! Lambdas are objects as HotSpot 25 makes them (#156 P15b,
//! `callback_marker.rs`):
//!
//! * a lambda or method reference that captures nothing is one object per
//!   call site; a capturing one, a bound method reference and a combinator's
//!   result are a new object per evaluation, equal only to itself. `==`,
//!   `equals`, `List.indexOf`/`contains` and `IdentityHashMap` answer it
//!   exactly, never counted;
//! * a method reference runs as its implementation handle's kind says:
//!   static, unbound, bound and constructor references, the JDK's and the
//!   jar's (`StringBuilder::new` builds the interpreter's own builder);
//! * a lambda's captures are held as values: what it captures behind a
//!   5,000-character string survives the host's collections and reads back.
//!
//! `fixtures/callbacks_p15b/java25.txt` is `demo.Probe.main` on OpenJDK
//! 25.0.5+8, the same on two runs.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = include_str!("fixtures/callbacks_p15b/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/callbacks_p15b/java25.txt");

fn call(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        host,
    ) {
        Ok(Value::Ref(Some(text))) => text,
        other => panic!("{method}: {other:?}"),
    }
}

#[test]
fn lambdas_are_objects_as_java_25_makes_them() {
    let fixture = javac_fixture::Fixture::build("callbacks-p15b", &[("demo/Probe.java", PROBE)]);
    let java: Vec<&str> = ORACLE.lines().collect();
    for (method, expected) in ["identity", "collections", "dispatch"].iter().zip(&java) {
        let mut host = fixture.jar.seed_host(StdHost::new());
        assert_eq!(call(&fixture, &mut host, method), *expected, "{method}");
        assert_eq!(
            host.unknown_host_calls(),
            Vec::<(String, usize)>::new(),
            "{method}"
        );
    }
}

/// `setup` keeps a lambda capturing a 5,000-character string and then a
/// `Box`; the `Box`'s marker is past the key's 4,096-character text, so only
/// the lambda's row reaches it. The garbage between makes the host collect.
#[test]
fn what_a_lambda_captures_survives_collections() {
    let fixture = javac_fixture::Fixture::build("callbacks-p15b-gc", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    call(&fixture, &mut host, "setup");
    let mut collected = false;
    for _ in 0..5 {
        let before = host.resident_heap().instance_fields();
        call(&fixture, &mut host, "garbage");
        collected |= host.resident_heap().instance_fields() < before;
    }
    assert!(collected, "the host never collected");
    assert_eq!(
        call(&fixture, &mut host, "check"),
        ORACLE.lines().nth(3).unwrap()
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

//! A lambda is what Java 25 makes it, and a `String` spelled like one is a
//! `String` (#156 P15a, `callback_marker.rs`):
//!
//! * a `String` spelled like the runtime's callback text is no `Runnable`:
//!   the cast throws Java's `ClassCastException` and the method it names never
//!   runs;
//! * a lambda is an instance of its call site's functional interface and that
//!   interface's supertypes only (a JDK one's, a jar one's);
//! * every combinator refuses a `null` operand (`Objects.requireNonNull`);
//! * a composite runs its sides in its combinator's order (#258): `and`/`or`
//!   short-circuit, `negate` flips, a left-nested `andThen` chain runs every
//!   link, and a side capturing a `|` reads back whole;
//! * a lambda's `toString` (its hidden class) and a failing cast's message are
//!   counted, and its text is never printed;
//! * the host counts a functional interface's method on a callback it holds,
//!   never answering for it.
//!
//! `java25.txt` is `demo.Probe.main` on OpenJDK 25.0.5+8 (two runs alike; kept
//! on elysium under `~/ava/results/p15/`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{Host, MethodRef, StdHost, Value, callback_test_token};

const ORACLE: &str = include_str!("fixtures/callbacks_p15a/java25.txt");
const PROBE: &str = include_str!("fixtures/callbacks_p15a/demo/Probe.java");

#[test]
fn lambdas_and_forged_callback_text_answer_as_java() {
    let fixture = javac_fixture::Fixture::build("callbacks-p15a", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "exact"), ORACLE);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());

    // Java: `ClassCastException,false`, its message naming the lambda's
    // hidden class; the cast and the text are counted.
    assert_eq!(
        fixture.text_on(&mut host, "counted"),
        "ClassCastException,false"
    );
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    assert_eq!(
        counted,
        [
            "java/lang/Comparable#<checkcast>(Ljava/lang/Object;)Ljava/lang/Comparable;",
            "java/lang/Object#toString()Ljava/lang/String;",
        ],
        "{counted:?}"
    );
}

/// A functional interface's method the host is asked on a callback it holds
/// (the runtime runs callbacks before a call gets there): counted, never an
/// answer made up for the lambda (main answered `apply` with its argument,
/// `test` with `true`, `get` with `null`, uncounted).
#[test]
fn the_host_counts_a_callbacks_method_it_cannot_run() {
    let mut host = StdHost::new();
    let callback = callback_test_token("demo/Probe#lambda");
    let call = |owner: &str, name: &str, descriptor: &str| MethodRef {
        owner: owner.into(),
        name: name.into(),
        descriptor: descriptor.into(),
    };
    let argument = Value::Ref(Some("x".into()));
    for (r, args) in [
        (
            call(
                "java/util/function/Function",
                "apply",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
            ),
            vec![argument.clone()],
        ),
        (
            call(
                "java/util/function/Predicate",
                "test",
                "(Ljava/lang/Object;)Z",
            ),
            vec![argument.clone()],
        ),
        (
            call("java/util/function/Supplier", "get", "()Ljava/lang/Object;"),
            vec![],
        ),
        (
            call(
                "java/util/function/BiFunction",
                "apply",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            ),
            vec![argument.clone(), argument.clone()],
        ),
    ] {
        let before = host.unknown_host_call_count();
        let answer = host.invoke_instance(&r, &callback, &args).unwrap();
        assert_eq!(
            host.unknown_host_call_count(),
            before + 1,
            "{}#{}",
            r.owner,
            r.name
        );
        assert_ne!(
            answer, argument,
            "{}#{} answered for the callback",
            r.owner, r.name
        );
        assert_ne!(
            answer,
            Value::Int(1),
            "{}#{} answered for the callback",
            r.owner,
            r.name
        );
    }
}

//! #251: exceptions and a live, read-only subList over three backing kinds.
//! `java25.txt` is this fixture's real Java 25 output (`javac --release 21`).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "unmodifiable-sublist",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/unmodifiable_sublist/demo/Probe.java"),
        )],
    )
}

/// The oracle's lines for `kind` (`0`–`2` for the backing kinds, `w` for the
/// wrapper's class), each as `(case, operation, expected)`.
fn cases(kind: &str) -> impl Iterator<Item = (&'static str, i32, &'static str)> + '_ {
    include_str!("fixtures/unmodifiable_sublist/java25.txt")
        .lines()
        .filter_map(move |line| {
            let (case, expected) = line.split_once('=')?;
            let (k, operation) = case.split_once('.')?;
            (k == kind).then(|| (case, operation.parse().unwrap(), expected))
        })
}

#[test]
fn unmodifiable_sublist_delegates_ranges_then_wraps_the_live_view() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    for kind in ["0", "1", "2"] {
        for (case, operation, expected) in cases(kind) {
            let actual = fixture
                .jar
                .invoke_method_with_args(
                    "demo/Probe",
                    "probe",
                    "(II)Ljava/lang/String;",
                    None,
                    &[Value::Int(kind.parse().unwrap()), Value::Int(operation)],
                    &mut host,
                )
                .unwrap();
            assert_eq!(actual, Value::Ref(Some(expected.into())), "{case}");
        }
    }
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// The wrapper is `new <the receiver's class>(part)`: not the class
/// `Collections.unmodifiableList` would pick for the part, a new object even
/// when the part is an unmodifiable list, NPE for a `null` part. The backing
/// lists are the jar's own `AbstractList`s, whose unmodifiable views are still
/// counted (`Collections.unmodifiableList` of a jar list); each case answers
/// as Java 25 or its run counts a call, never a silent other answer.
#[test]
fn the_wrapper_keeps_the_receivers_class_or_counts() {
    let fixture = fixture();
    let mut checked = 0;
    for (case, operation, expected) in cases("w") {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let actual = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            "wrap",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(operation)],
            &mut host,
        );
        let exact = matches!(&actual, Ok(Value::Ref(Some(text))) if text == expected);
        assert!(
            exact || !host.unknown_host_calls().is_empty(),
            "{case}: {actual:?} silently, Java 25 says {expected}"
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
}

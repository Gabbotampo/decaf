//! #251: exceptions and a live, read-only subList over three backing kinds.
//! `java25.txt` is this fixture's real Java 25 output (`javac --release 21`).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

#[test]
fn unmodifiable_sublist_delegates_ranges_then_wraps_the_live_view() {
    let fixture = javac_fixture::Fixture::build(
        "unmodifiable-sublist",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/unmodifiable_sublist/demo/Probe.java"),
        )],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    for line in include_str!("fixtures/unmodifiable_sublist/java25.txt").lines() {
        let (case, expected) = line.split_once('=').unwrap();
        let (kind, operation) = case.split_once('.').unwrap();
        let actual = fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                "probe",
                "(II)Ljava/lang/String;",
                None,
                &[
                    Value::Int(kind.parse().unwrap()),
                    Value::Int(operation.parse().unwrap()),
                ],
                &mut host,
            )
            .unwrap();
        assert_eq!(actual, Value::Ref(Some(expected.into())), "{case}");
    }
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

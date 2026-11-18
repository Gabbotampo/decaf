#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};

#[test]
fn sublist_writes_match_java25() {
    let f = javac_fixture::Fixture::build(
        "sublist-writes",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/sublist_writes/demo/Probe.java"),
        )],
    );
    let expected = [
        (0, "[a, x, c, d]|[x, c]"),
        (1, "[a, b, c, y, d]|[b, c, y]"),
        (2, "[a, c, d]|[c]"),
        (3, "[a, d]|[]"),
        (6, "java.util.ConcurrentModificationException"),
    ];
    for (op, expected) in expected {
        let mut host = f.jar.seed_host(StdHost::new());
        let got = f.jar.invoke_method_with_args(
            "demo/Probe",
            "probe",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut host,
        );
        assert!(matches!(got, Ok(Value::Ref(Some(ref x))) if x == expected));
        assert_eq!(
            host.unknown_host_call_count(),
            0,
            "op {op}: {:?}",
            host.unknown_host_calls()
        );
    }
    for (op, expected_call) in [
        (
            4,
            "java/util/List#replaceAll(Ljava/util/function/UnaryOperator;)V",
        ),
        (5, "java/util/List#sort(Ljava/util/Comparator;)V"),
        (7, "java/util/Collections#sort(Ljava/util/List;)V"),
    ] {
        let mut counted = f.jar.seed_host(StdHost::new());
        let _ = f.jar.invoke_method_with_args(
            "demo/Probe",
            "probe",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut counted,
        );
        assert_eq!(
            counted.unknown_host_calls(),
            vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2
                ),
                (expected_call.into(), 1),
            ],
            "op {op}"
        );
    }
}

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
    for (op, expected) in include_str!("fixtures/sublist_writes/java25.txt")
        .lines()
        .filter_map(|line| {
            line.split_once('=')
                .map(|(op, expected)| (op.parse::<i32>().unwrap(), expected))
        })
    {
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
    for op in [4, 5, 7, 8] {
        let mut counted = f.jar.seed_host(StdHost::new());
        let got = f.jar.invoke_method_with_args(
            "demo/Probe",
            "review",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut counted,
        );
        assert!(matches!(
            got,
            Ok(Value::Ref(Some(ref result))) if result == "null|null|null|null"
        ));
        let expected_calls = match op {
            4 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
                (
                    "java/util/List#replaceAll(Ljava/util/function/UnaryOperator;)V".into(),
                    1,
                ),
            ],
            5 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
                ("java/util/List#sort(Ljava/util/Comparator;)V".into(), 1),
            ],
            7 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/Collections#sort(Ljava/util/List;)V".into(), 1),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
            ],
            8 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/Collections#reverse(Ljava/util/List;)V".into(), 1),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
            ],
            _ => unreachable!(),
        };
        assert_eq!(counted.unknown_host_calls(), expected_calls, "op {op}");
    }
}

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
    let mut h = f.jar.seed_host(StdHost::new());
    for (op, e) in include_str!("fixtures/sublist_writes/java25.txt")
        .lines()
        .filter_map(|l| {
            l.split_once('=')
                .map(|(a, b)| (a.parse::<i32>().unwrap(), b))
        })
        .filter(|(op, _)| *op != 4)
    {
        let got = f.jar.invoke_method_with_args(
            "demo/Probe",
            "probe",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut h,
        );
        if op == 4 {
            assert!(got.is_err() || matches!(got, Ok(Value::Ref(Some(ref x))) if x == e));
        } else {
            assert!(matches!(got, Ok(Value::Ref(Some(ref x))) if x == e));
        }
    }
    for (op, expected_call) in [
        (
            4,
            "java/util/List#replaceAll(Ljava/util/function/UnaryOperator;)V",
        ),
        (5, "java/util/List#sort(Ljava/util/Comparator;)V"),
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
        assert!(
            counted
                .unknown_host_calls()
                .iter()
                .any(|(call, n)| call == expected_call && *n == 1)
        );
    }
}

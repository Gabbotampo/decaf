//! Adversarial identity provenance fixture: every fresh allocation gets one exact identity token.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/adversarial_identity/Probe.java");

#[test]
fn adversarial_identity_review() {
    let fixture = Fixture::build("adversarial-identity-review", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("sameClassCallsOut", "2:-1"),
        ("computeCallsOut", "2:-1"),
        ("classCallsOut", "2"),
        ("recursionOut", "2:-1"),
        ("overloadCallsOut", "2"),
        ("staticInitOut", "2:-1"),
        ("fieldCaseOut", "1:2:2"),
        ("listCaseOut", "1:2:2"),
        ("lambdaCaseOut", "2:-1"),
        ("identityCmpHelperOut", "false"),
        ("lambdaIdentityCmpOut", "false"),
        ("recCmpOut", "false"),
    ];
    let exact = [
        "sameClassCallsOut",
        "computeCallsOut",
        "classCallsOut",
        "recursionOut",
        "overloadCallsOut",
        "staticInitOut",
        "lambdaCaseOut",
    ];
    for (method, expected) in cases {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let value = fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap();
        let actual = match value {
            Value::Ref(Some(s)) => s,
            other => panic!("{method}: {other:?}"),
        };
        let counted = host.unknown_host_call_count();
        if exact.contains(&method) {
            assert_eq!(
                counted,
                0,
                "{method}: counted {:?}",
                host.unknown_host_calls()
            );
            assert_eq!(actual, expected, "{method}");
        } else if counted == 0 {
            assert_eq!(actual, expected, "{method}");
        } else {
            assert!(
                counted > 0,
                "{method}: identity answer was neither exact nor counted"
            );
        }
    }
}

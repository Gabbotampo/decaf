#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

#[test]
fn synchronized_list_cast_matches_java25() {
    let fixture = javac_fixture::Fixture::build(
        "synchronized-list-cast",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/synchronized_list_cast/Probe.java"),
        )],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let result = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "probe",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let expected = include_str!("fixtures/synchronized_list_cast/java25.txt").trim();
    assert!(matches!(result, Ok(Value::Ref(Some(ref text))) if text == expected));
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

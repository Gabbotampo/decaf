#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const PROBE: &str = include_str!("fixtures/properties-load/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/properties-load/java25.txt");
#[test]
fn properties_load_matches_java25() {
    let extras: &[(&str, &[u8])] = &[];
    let f = javac_fixture::Fixture::build_with_lib(
        "properties-load",
        &[("demo/Probe.java", PROBE)],
        extras,
    );
    let mut h = f.jar.seed_host(horizon_native_java::StdHost::new());
    let got = match f.jar.invoke_method_with_args(
        "demo/Probe",
        "rows",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut h,
    ) {
        Ok(horizon_native_java::Value::Ref(Some(s))) => s,
        x => panic!("{x:?}"),
    };
    assert_eq!(got, ORACLE);
    assert!(
        h.unknown_host_calls().is_empty(),
        "{:?}",
        h.unknown_host_calls()
    );
}

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const PROBE: &str = include_str!("fixtures/properties-load/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/properties-load/java25.txt");
#[test]
fn properties_load_matches_java25() {
    let extras: &[(&str, &[u8])] = &[
        ("p0.properties", b"# c\n! d\nkey=value\n"),
        ("p1.properties", b"a=b\nc:d\ne f\ng   h\ni= j\n"),
        ("p2.properties", b"cont=one\\\n  two\neven=x\\\\\ny=z\n"),
        ("p3.properties", b"u=\\u00E9\n"),
        ("p4.properties", b"=emptykey\nempty=\n"),
        ("p5.properties", b"dup=one\ndup=two\n"),
        ("p6.properties", b"trail=x\\"),
        ("p7.properties", b"bad=\\u12G4\n"),
    ];
    let f = javac_fixture::Fixture::build_with_mod_json_and_extra(
        "properties-load",
        &[("demo/Probe.java", PROBE)],
        "{}",
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

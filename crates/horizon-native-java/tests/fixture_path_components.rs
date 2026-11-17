#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const PROBE: &str = include_str!("fixtures/path_components/demo/Probe.java");
#[test]
fn path_components() {
    let f = javac_fixture::Fixture::build("path-components", &[("demo/Probe.java", PROBE)]);
    let mut h = f.jar.seed_host(horizon_native_java::StdHost::new());
    let v = f
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut h,
        )
        .unwrap();
    let s = match v {
        horizon_native_java::Value::Ref(Some(x)) => x,
        _ => panic!("{v:?}"),
    };
    assert_eq!(
        s,
        "file=c|parent=/a/b/..|root=null|count=4|name=a|oor=IllegalArgumentException:null|sub=a/b|abs=true|empty=null|eq=true"
    );
    assert!(
        h.unknown_host_calls().is_empty(),
        "{:?}",
        h.unknown_host_calls()
    );
}

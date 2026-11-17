#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/string_marker_shapes/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_marker_shapes/java25.txt");
#[test]
fn runtime_built_strings_match_java25() {
    let fixture = Fixture::build("string-marker-shapes", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let rows = fixture.text_on(&mut host, "rows");
    assert_eq!(rows, ORACLE);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn undecidable_instanceof_is_counted() {
    let fixture = Fixture::build("string-marker-shapes-count", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, "counted");
    assert_eq!(
        host.unknown_host_calls(),
        vec![(
            "java/lang/String#<checkcast>(Ljava/lang/Object;)Ljava/lang/String;".into(),
            1
        )]
    );
}

#[test]
fn undecidable_instanceof_exact_count() {
    let f = Fixture::build("shape-i", &[("demo/Probe.java", PROBE)]);
    let mut h = f.jar.seed_host(StdHost::new());
    let _ = f.text_on(&mut h, "instanceOfUndecidable");
    assert_eq!(
        h.unknown_host_calls(),
        vec![(
            "java/lang/String#<instanceof>(Ljava/lang/Object;)Z".into(),
            1
        )]
    );
}
#[test]
fn undecidable_cast_exact_count() {
    let f = Fixture::build("shape-c", &[("demo/Probe.java", PROBE)]);
    let mut h = f.jar.seed_host(StdHost::new());
    let _ = f.text_on(&mut h, "castUndecidable");
    assert_eq!(
        h.unknown_host_calls(),
        vec![(
            "java/lang/String#<checkcast>(Ljava/lang/Object;)Ljava/lang/String;".into(),
            1
        )]
    );
}
#[test]
fn undecidable_get_class_exact_count() {
    let f = Fixture::build("shape-g", &[("demo/Probe.java", PROBE)]);
    let mut h = f.jar.seed_host(StdHost::new());
    let _ = f.text_on(&mut h, "getClassUndecidable");
    assert_eq!(
        h.unknown_host_calls(),
        vec![("java/lang/Object#getClass()Ljava/lang/Class;".into(), 1)]
    );
}

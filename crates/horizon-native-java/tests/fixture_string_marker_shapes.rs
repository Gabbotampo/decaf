#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/string_marker_shapes/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_marker_shapes/java25.txt");
#[test]
fn runtime_built_strings_match_java25() {
 let fixture=Fixture::build("string-marker-shapes", &[("demo/Probe.java", PROBE)]);
 let mut host=fixture.jar.seed_host(StdHost::new());
 let rows=fixture.text_on(&mut host,"rows");
 assert_eq!(rows, ORACLE);
 assert!(host.unknown_host_calls().is_empty(), "{:?}", host.unknown_host_calls());
}

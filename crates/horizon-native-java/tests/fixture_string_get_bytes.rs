#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/string_get_bytes/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_get_bytes/java25.txt");
#[test]
fn string_get_bytes_rows_match_java25() {
    let fixture = Fixture::build("string_get_bytes", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let got = fixture.text_on(&mut host, "rows");
    assert_eq!(got, ORACLE);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

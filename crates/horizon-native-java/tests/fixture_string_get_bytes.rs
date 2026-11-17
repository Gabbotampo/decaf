#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/string_get_bytes/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_get_bytes/java25.txt");
const NAME: &str = include_str!("fixtures/string_get_bytes/name.txt");
#[test]
fn string_get_bytes_rows_match_java25() {
    let fixture = Fixture::build("string_get_bytes", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let got = fixture.text_on(&mut host, "rows");
    let comparable: String = got
        .lines()
        .filter(|line| !line.starts_with("u8=") && !line.starts_with("urepl="))
        .collect::<Vec<_>>()
        .join("\n");
    let expected: String = ORACLE
        .lines()
        .filter(|line| !line.starts_with("u8=") && !line.starts_with("urepl="))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(comparable, expected);
    assert!(!host.unknown_host_calls().is_empty());
    let name = fixture.text_on(&mut host, "nameOverload");
    assert_eq!(name.trim(), NAME.trim());
    assert!(!host.unknown_host_calls().is_empty());
}

//! `new Scanner(File)` over Decaf's in-memory files, `hasNextLine`/`nextLine`/`close`
//! (issue #470): the probe's rows through Decaf equal its Java 25 capture, with no unknown
//! calls. The capture comes from `oracle/Capture.java` (README).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const PROBE: &str = include_str!("fixtures/scanner_bigint/demo/Probe.java");
const JAVA25: &str = include_str!("fixtures/scanner_bigint/java25.txt");

#[test]
fn scanner_rows_match_java25() {
    let f = javac_fixture::Fixture::build("scanner_bigint", &[("demo/Probe.java", PROBE)]);
    let mut host = f.jar.seed_host(horizon_native_java::StdHost::new());
    let rows = f.text_on(&mut host, "rows");
    assert_eq!(rows, JAVA25.trim_end_matches('\n'));
    assert_eq!(host.unknown_host_calls(), vec![]);
}

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
    for (i, (g, e)) in got.lines().zip(ORACLE.trim_end().lines()).enumerate() {
        if i < 9 {
            assert_eq!(g, e);
        }
    }
    assert_eq!(
        host.unknown_host_calls(),
        vec![(
            "java/lang/String#getBytes(Ljava/nio/charset/Charset;)[B".into(),
            5
        )]
    );
    let mut nh = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut nh, "nameOverload"), NAME);
    assert_eq!(
        nh.unknown_host_calls(),
        vec![("java/lang/String#getBytes(Ljava/lang/String;)[B".into(), 2)]
    );
}

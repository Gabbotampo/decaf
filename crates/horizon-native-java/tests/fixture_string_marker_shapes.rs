#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/string_marker_shapes/demo/Probe.java");
const JAVA25: &str = include_str!("fixtures/string_marker_shapes/java25.txt");
const CASES: &[(&str, bool)] = &[
    ("hello", true),
    ("callback", true),
    ("stream", true),
    ("textures", true),
    ("models", true),
    ("client", false),
    ("blockPos", false),
    ("typed", false),
    ("registry", false),
    ("payload", false),
    ("payloadContext", false),
    ("iterator", false),
    ("enumX", false),
    ("game", false),
    ("list", false),
    ("particle", false),
];
fn java_line(method: &str) -> &str {
    JAVA25
        .lines()
        .find(|l| l.strip_prefix(method).is_some_and(|r| r.starts_with('=')))
        .and_then(|l| l.split_once('=').map(|(_, v)| v))
        .unwrap()
}
#[test]
fn every_prefix_family_matches_java_or_is_counted() {
    let f = Fixture::build("string-marker-shapes", &[("demo/Probe.java", PROBE)]);
    for (method, exact) in CASES {
        let mut host = f.jar.seed_host(StdHost::new());
        let text = f.text_on(&mut host, method);
        if *exact {
            assert_eq!(text, java_line(method), "{method}");
            assert!(
                host.unknown_host_calls().is_empty(),
                "{method}: {:?}",
                host.unknown_host_calls()
            );
        } else {
            assert_eq!(
                host.unknown_host_calls(),
                vec![(
                    "java/lang/String#<instanceof>(Ljava/lang/Object;)Z".into(),
                    1
                )],
                "{method}"
            );
            let _ = text;
        }
    }
}

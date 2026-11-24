//! A `null` key in a `TreeMap`/`TreeSet` with natural ordering: which call
//! throws `NullPointerException:null`, which throws the JDK's detailed
//! `TreeMap.compare` message, and which returns `null` (an empty tree is never
//! walked). Every expected row is OpenJDK 25.0.5.1's output for the same
//! probe, `fixtures/tree_map_null_key/java25.txt` (see the README there).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const ORACLE: &str = include_str!("fixtures/tree_map_null_key/java25.txt");
const PROBE: &str = include_str!("fixtures/tree_map_null_key/demo/Probe.java");

fn call(fixture: &Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap_or_else(|error| panic!("{method} failed: {error}"));
    let Value::Ref(Some(text)) = answer else {
        panic!("{method} did not return a string: {answer:?}");
    };
    (text, host.unknown_host_calls())
}

#[test]
fn null_keys_match_java25() {
    let fixture = Fixture::build("tree_map_null_key", &[("demo/Probe.java", PROBE)]);
    let (text, unknown) = call(&fixture, "rows");
    let got: Vec<&str> = text.lines().collect();
    let want: Vec<&str> = ORACLE.lines().collect();
    let differing: Vec<String> = want
        .iter()
        .zip(&got)
        .filter(|(want, got)| want != got)
        .map(|(want, got)| format!("java:   {want}\ndecaf:  {got}"))
        .collect();
    assert!(differing.is_empty(), "{}", differing.join("\n"));
    assert_eq!(got.len(), want.len(), "row count");
    assert!(unknown.is_empty(), "counted calls: {unknown:?}");
}

/// Views the model does not build stay counted, not answered.
#[test]
fn unmodelled_views_stay_counted() {
    let fixture = Fixture::build("tree_map_null_key_counted", &[("demo/Probe.java", PROBE)]);
    for method in [
        "countedSubMap",
        "countedHeadMapInclusive",
        "countedHeadSetInclusive",
        "countedHeadMapComparator",
    ] {
        let (_, unknown) = call(&fixture, method);
        assert!(!unknown.is_empty(), "{method} was answered, not counted");
    }
}

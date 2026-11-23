//! Case-insensitive `String` comparisons: `equalsIgnoreCase`,
//! `compareToIgnoreCase`, both `regionMatches` overloads and
//! `String.CASE_INSENSITIVE_ORDER`, answered as Java 25 answers them.
//!
//! `equalsIgnoreCase` was modelled with `eq_ignore_ascii_case`, so a pair such
//! as `"\u00e4"`/`"\u00c4"`, `"k"`/`"\u212a"` (Kelvin sign) or `"I"`/`"\u0131"`
//! answered `false` where Java answers `true`, with nothing counted. The other
//! three were not modelled, so every call stayed a counted unknown host call
//! (`CASE_INSENSITIVE_ORDER` was not even a known field).
//!
//! `fixtures/string_ignore_case/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on Java 25 (output is `java25.txt`), its static probes in
//! Horizon. Every row is answered, and answered exactly: nothing is counted.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

/// What `demo.Probe`'s `main` printed on OpenJDK 25.0.5.1 (`javac --release
/// 21`), the same on two runs; see the README beside it.
const ORACLE: &str = include_str!("fixtures/string_ignore_case/java25.txt");
const PROBE: &str = include_str!("fixtures/string_ignore_case/demo/Probe.java");

fn fixture() -> Fixture {
    Fixture::build("string-ignore-case", &[("demo/Probe.java", PROBE)])
}

/// Every probe answers as Java 25 did, with nothing counted.
#[test]
fn string_ignore_case_matches_java_25() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for line in ORACLE.lines() {
        let (method, java) = line.split_once('=').unwrap();
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
        let text = match answer {
            horizon_native_java::Value::Ref(Some(text)) => text,
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != java || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {java}\n  native: {text}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

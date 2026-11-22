//! `String.strip`, `stripLeading`, `stripTrailing`, `trim` and `isBlank`,
//! plus `Character.isWhitespace` and `Character.isSpaceChar`, answered one code
//! point at a time and measured against Java 25.
//
//! `demo/Probe.java` is the only class the test ships; it needs nothing but the
//! JDK. `java25.txt` holds its raw output, one `<row>=<value>` line per case,
//! prefixed with a unique label the test reads them by. Captured with:
//
//! ```sh
//! $J/javac --release 21 -d out demo/Probe.java
//! $J/java -Xmx512m -cp out demo.Probe | head -c 2000000 > java25.txt
//! ```
#![allow(dead_code)]

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/string_strip_whitespace/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_strip_whitespace/java25.txt");

#[test]
fn string_strip_whitespace_matches_java25() {
    let fixture = Fixture::build("string-strip-whitespace", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let got = match answer {
        horizon_native_java::Value::Ref(Some(text)) => text,
        other => panic!("rows did not return a string: {other:?}"),
    };
    let answer_lines: Vec<&str> = got.lines().collect();
    let oracle_lines: Vec<&str> = ORACLE.lines().collect();
    assert_eq!(
        answer_lines.len(),
        oracle_lines.len(),
        "row count differs; oracle rows:\n{ORACLE}\ngot:\n{got}"
    );
    for (line, want) in answer_lines.iter().zip(&oracle_lines) {
        let (label, value) = want
            .split_once('=')
            .expect("oracle row lacks the = separator");
        let got_value = line
            .strip_prefix(label)
            .and_then(|rest| rest.strip_prefix('='));
        assert_eq!(
            got_value.as_deref(),
            Some(value),
            "row {label}: got {line:?}"
        );
    }
    // Every row above is answered exactly, so nothing may stay counted.
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

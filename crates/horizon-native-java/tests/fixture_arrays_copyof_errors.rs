//! `Arrays.copyOf` / `Arrays.copyOfRange` argument errors (`fixtures/arrays_copyof_errors`).
//!
//! `java25.txt` is `demo.Probe.rows()` run on JDK 25.0.5.1, unedited; `java25-counted.txt` is
//! `countedRows()`, calls on an array whose class the runtime does not know, which stay counted.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/arrays_copyof_errors/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/arrays_copyof_errors/java25.txt");
const COUNTED: &str = include_str!("fixtures/arrays_copyof_errors/java25-counted.txt");

#[test]
fn copy_argument_errors_answer_as_java_25() {
    let fixture = Fixture::build("arrays-copyof-errors", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, "rows");
    assert_eq!(text.lines().count(), ORACLE.lines().count());
    for (got, expected) in text.lines().zip(ORACLE.lines()) {
        assert_eq!(got, expected);
    }
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn a_class_the_runtime_does_not_know_stays_counted() {
    let fixture = Fixture::build("arrays-copyof-counted", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    // `COUNTED` is what Java prints. The runtime answers none of these rows: each call is
    // counted by name, and the text it leaves is not a Java answer.
    let text = fixture.text_on(&mut host, "countedRows");
    for (got, expected) in text.lines().zip(COUNTED.lines()) {
        assert_ne!(got, expected, "answered, not counted");
    }
    let calls = host.unknown_host_calls();
    assert_eq!(
        calls
            .iter()
            .filter(|(call, _)| call.starts_with("java/util/Arrays#copyOf"))
            .map(|(_, count)| count)
            .sum::<usize>(),
        COUNTED.lines().count(),
        "{calls:?}"
    );
}

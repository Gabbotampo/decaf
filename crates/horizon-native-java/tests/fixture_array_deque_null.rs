//! `ArrayDeque` (and `ConcurrentLinkedQueue`/`ConcurrentLinkedDeque`) reject
//! null elements: an insertion of null throws `NullPointerException` (message
//! `null`) whether the call names the class or an interface (`Deque.add`,
//! `Queue.offer`, `Collection.addAll`), `ArrayDeque.addAll` of a collection
//! holding a null keeps what precedes the null, the two concurrent queues add
//! nothing, and a constructor given such a collection throws.
//!
//! The probe runs unchanged on both sides: `main` on Java 25 (its output is
//! `fixtures/array_deque_null/java25.txt`, one row per line), each row's
//! method in Horizon. A row is answered exactly or the test fails, except
//! `forEachAfterFailedAddAll`, which stays counted: the deque's layout is
//! unknown after the failed `addAll`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const ORACLE: &str = include_str!("fixtures/array_deque_null/java25.txt");

const LAYOUT_ROW: &str = "forEachAfterFailedAddAll";

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "array-deque-null",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/array_deque_null/demo/Probe.java"),
        )],
    )
}

/// Every oracle row as `(method, whole line)`.
fn rows() -> impl Iterator<Item = (&'static str, &'static str)> {
    ORACLE
        .lines()
        .map(|line| (line.split_once('=').expect("a row is name=value").0, line))
}

#[test]
fn array_deque_rejects_null_elements_exactly_as_java_25_does() {
    let fixture = fixture();
    let mut failures = Vec::new();
    let mut answered = 0;
    for (method, expected) in rows().filter(|(method, _)| *method != LAYOUT_ROW) {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let text = fixture.text_on(&mut host, method);
        let unknown = host.unknown_host_calls();
        if text != expected || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {expected}\n  native: {text}\n  unknown: {unknown:?}"
            ));
        }
        answered += 1;
    }
    assert!(answered >= 40, "only {answered} rows ran");
    assert!(
        failures.is_empty(),
        "{} of {answered} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A failed `addAll` has added the elements before the null and has grown
/// the array first, so the layout `forEach` reads is no longer known: the
/// `forEach` after it is counted, not answered from the layout that predates
/// the call. (Java's answer is in the oracle; the layout model does not
/// follow a call that threw, so Horizon does not claim it.)
#[test]
fn for_each_after_a_failed_add_all_stays_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.text_on(&mut host, LAYOUT_ROW);
    let counted = host.unknown_host_calls();
    assert!(
        counted
            .iter()
            .any(|(call, _)| call == "java/util/ArrayDeque#forEach(Ljava/util/function/Consumer;)V"),
        "forEach was answered after a failed addAll: {counted:?}"
    );
}

//! `ArrayDeque` rejects null elements: every insertion of null throws
//! `NullPointerException` (message `null`), `addAll` of a list containing null
//! fails part-way and keeps the elements added before the null, and `new
//! ArrayDeque<>(collection)` with a null inside throws too.
//! `contains(null)`, `remove(null)` and `removeFirstOccurrence(null)` answer
//! `false` without throwing.
//!
//! The probe runs unchanged on both sides: `main` on Java 25 (its output is
//! `fixtures/array_deque_null/java25.txt`), the static `String` probes in
//! Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const ORACLE: &str = include_str!("fixtures/array_deque_null/java25.txt");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

#[test]
fn array_deque_rejects_null_elements_exactly_as_java_25_does() {
    let fixture = javac_fixture::Fixture::build(
        "array-deque-null",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/array_deque_null/demo/Probe.java"),
        )],
    );
    let mut failures = Vec::new();
    for method in [
        "add",
        "addFirst",
        "addLast",
        "offer",
        "offerFirst",
        "offerLast",
        "push",
        "addAll",
        "addAllPrefix",
        "constructor",
        "containsNull",
        "removeNull",
        "removeFirstOccurrenceNull",
        "addValue",
        "addAllOk",
    ] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let raw = match answer {
            Ok(horizon_native_java::Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let text = raw.strip_prefix(&format!("{method}=")).unwrap_or(&raw);
        let unknown = host.unknown_host_calls();
        if text != expected(method) || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} modelled calls differ (answered exactly, none stay counted):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

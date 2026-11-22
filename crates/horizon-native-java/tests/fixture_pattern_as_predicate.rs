//! `java.util.regex.Pattern#asPredicate()` and the returned
//! `Predicate#test(Object)`, run through the interpreter against the JDK 25
//! answers in `fixtures/pattern_as_predicate/java25.txt`.
//!
//! Modelled exactly: a `Pattern` from `Pattern.compile(String)` (the only
//! `Pattern` the runtime makes, so flags 0) and a `test` on a `String`, which
//! is `matcher(s).find()` on a fresh matcher through the `java_regex` engine.
//! A jar's own `Predicate` (a lambda, a method reference, an anonymous or a
//! named class) runs as it always did. Everything else stays counted, never
//! guessed: a `Pattern` with flags, and a `test` argument that is not a
//! `String`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/pattern_as_predicate/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/pattern_as_predicate/java25.txt");

/// Every oracle row, answered with no unknown host call.
#[test]
fn pattern_as_predicate_rows_match_java25() {
    let fixture = Fixture::build("pattern-as-predicate", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "rows"), ORACLE);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

/// `Pattern.compile(String, int)` is not modelled, so a `Pattern` with flags is
/// never a handle `asPredicate` answers for (the JDK prints `true` for
/// `CASE_INSENSITIVE`): the calls are counted, never answered as flags 0.
#[test]
fn a_pattern_with_flags_is_counted() {
    let fixture = Fixture::build("pattern-as-predicate-flags", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let outcome = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "flagged",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let unknown = host.unknown_host_calls();
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call
                .starts_with("java/util/regex/Pattern#compile(Ljava/lang/String;I)")),
        "outcome {outcome:?}, counted {unknown:?}"
    );
    // The flag-0 answer for "ABC" against "abc" is `false`: it must not appear.
    assert_ne!(
        outcome.ok(),
        Some(horizon_native_java::Value::Ref(Some(
            "flagged ABC = false".into()
        )))
    );
}

/// A supplementary character in the input: the engine refuses it, so `test` is
/// counted (the JDK prints `true`, `false`, and `removeIf` removes both elements),
/// never answered as UTF-16 units, also when `removeIf` runs the predicate.
#[test]
fn a_supplementary_input_is_counted() {
    let fixture = Fixture::build("pattern-as-predicate-sup", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let outcome = fixture.text_on(&mut host, "supplementary");
    assert_eq!(
        host.unknown_host_calls(),
        [(
            "java/util/function/Predicate#test(Ljava/lang/Object;)Z".to_owned(),
            3
        )],
        "answered {outcome:?}"
    );
}

/// `test` on a `StringBuilder`, `null` or an `Integer`: the JDK throws
/// (`ClassCastException`, `NullPointerException`, `ClassCastException`), the
/// runtime counts the call and does not answer it as a match against the
/// object's marker.
#[test]
fn a_test_argument_that_is_not_a_string_is_counted() {
    let fixture = Fixture::build("pattern-as-predicate-args", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let outcome = fixture.text_on(&mut host, "nonString");
    assert_eq!(
        host.unknown_host_calls(),
        [(
            "java/util/function/Predicate#test(Ljava/lang/Object;)Z".to_owned(),
            3
        )],
        "answered {outcome:?}"
    );
}

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/matcher_find/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/matcher_find/java25.txt");
const COUNTED: &str = "--counted--\n";
const CHANGED: &str = "--changed--\n";
const LOST: &str = "--lost--\n";

/// The oracle's four parts: the answered rows, the rows the runtime must
/// count (Java's answers), the `matches()` rows after a state-changing call
/// (counted too) and the `matches()` rows after a lost position (answered).
fn parts() -> (&'static str, &'static str, &'static str, &'static str) {
    let (exact, rest) = ORACLE.split_once(COUNTED).expect("counted split line");
    let (counted, rest) = rest.split_once(CHANGED).expect("changed split line");
    let (changed, lost) = rest.split_once(LOST).expect("lost split line");
    (exact, counted, changed, lost)
}

/// A fixture of its own per test: two tests building into one directory race.
fn fixture(test: &str) -> Fixture {
    Fixture::build(test, &[("demo/Probe.java", PROBE)])
}

#[test]
fn matcher_find_rows_match_java25() {
    let (exact, ..) = parts();
    let fixture = fixture("matcher-find-rows");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let actual = fixture.text_on(&mut host, "rows");
    // Line by line, so a failure names the row.
    for (got, want) in actual.lines().zip(exact.lines()) {
        assert_eq!(got, want);
    }
    assert_eq!(actual.lines().count(), exact.lines().count());
    assert_eq!(actual, exact);
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

/// A `find()` after an attempt that failed over a pattern with a quantifier
/// resumes where Java's repeated atoms left `last`, which the engine does not
/// write: it is counted, with the `group()` after it, and never answered.
#[test]
fn a_find_after_a_failed_quantified_attempt_stays_counted() {
    let (_, counted, ..) = parts();
    let fixture = fixture("matcher-find-counted");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, "counted");
    let rows = counted.lines().count();
    assert!(rows > 0);
    let mut calls = host.unknown_host_calls();
    calls.sort();
    assert_eq!(
        calls,
        vec![
            ("java/util/regex/Matcher#find()Z".to_string(), rows),
            (
                "java/util/regex/Matcher#group()Ljava/lang/String;".to_string(),
                rows
            ),
        ]
    );
}

/// `matches()` over a matcher a call changed (`region`, `usePattern`,
/// `reset(CharSequence)`): Java answers over what the call left, not over the
/// handle's pattern, input and region, so the runtime counts it, and the call
/// that changed the matcher, and answers neither.
#[test]
fn matches_after_a_state_changing_call_is_counted() {
    let (.., changed, _) = parts();
    let fixture = fixture("matcher-find-changed");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, "changed");
    // Six rows: region(0,1), usePattern(b), region(1,2), reset("ab"), a
    // builder's region(0,1) and a method reference to matches().
    assert_eq!(changed.lines().count(), 6);
    let mut calls = host.unknown_host_calls();
    calls.sort();
    assert_eq!(
        calls,
        vec![
            ("java/util/regex/Matcher#matches()Z".to_string(), 6),
            (
                "java/util/regex/Matcher#region(II)Ljava/util/regex/Matcher;".to_string(),
                4
            ),
            (
                "java/util/regex/Matcher#reset(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;"
                    .to_string(),
                1
            ),
            (
                "java/util/regex/Matcher#usePattern(Ljava/util/regex/Pattern;)Ljava/util/regex/Matcher;"
                    .to_string(),
                1
            ),
        ]
    );
}

/// `matches()` after a `find()` that was counted: the matcher lost its
/// position but not its text, so `matches()` is answered, and is Java's.
#[test]
fn matches_after_a_counted_find_is_answered() {
    let (.., lost) = parts();
    let fixture = fixture("matcher-find-lost");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let actual = fixture.text_on(&mut host, "lost");
    for (got, want) in actual.lines().zip(lost.lines()) {
        assert_eq!(got, want);
    }
    assert_eq!(actual, lost);
    let rows = lost.lines().count();
    let mut calls = host.unknown_host_calls();
    calls.sort();
    assert_eq!(
        calls,
        vec![("java/util/regex/Matcher#find()Z".to_string(), rows)]
    );
}

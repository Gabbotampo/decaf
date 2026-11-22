#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/matcher_find/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/matcher_find/java25.txt");
const SPLIT: &str = "--counted--\n";

/// A fixture of its own per test: two tests building into one directory race.
fn fixture(test: &str) -> Fixture {
    Fixture::build(test, &[("demo/Probe.java", PROBE)])
}

#[test]
fn matcher_find_rows_match_java25() {
    let (exact, _) = ORACLE.split_once(SPLIT).expect("oracle split line");
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
    let (_, counted) = ORACLE.split_once(SPLIT).expect("oracle split line");
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

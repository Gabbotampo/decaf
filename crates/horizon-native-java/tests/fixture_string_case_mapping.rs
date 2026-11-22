//! `String#toUpperCase`/`toLowerCase` with and without a `Locale`, and
//! `Character#toUpperCase`/`toLowerCase` of a `char` or an `int`: the runtime
//! answers exactly Java 25's answer only where it can model it exactly --
//! ASCII receivers (any non-tailored `Locale` for the `Locale` overloads),
//! single code points for `Character` -- and counts every other case instead
//! of silently returning the wrong text.
//!
//! `demo/Probe` runs unchanged on both sides: `main` on Java 25 (its output
//! is `java25.txt`), the same static probes in Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{NativeError, StdHost, Value};

const ORACLE: &str = include_str!("fixtures/string_case_mapping/java25.txt");

const SOURCES: &[(&str, &str)] = &[(
    "demo/Probe.java",
    include_str!("fixtures/string_case_mapping/demo/Probe.java"),
)];

/// Rows the runtime answers exactly: the runtime's text must equal the oracle
/// line and nothing may be counted.
const ANSWERED: &[&str] = &[
    // No `Locale`: ASCII receivers fold the same in every locale.
    "emptyUpper",
    "emptyLower",
    "asciiLowerUpper",
    "asciiLowerLower",
    "asciiUpperUpper",
    "asciiUpperLower",
    // The `Locale` overloads: ASCII with a non-tailored locale.
    "asciiRootLower",
    "asciiRootUpper",
    // `Character`: single code points, simple mapping.
    "charUpperSharp",
    "charLowerSharp",
    "charUpperE",
    "charLowerE",
    "charUpperAMacron",
    "charLowerAMacron",
    "charLowerSigma",
    "charUpperSigma",
    "charUpperIDotless",
    "charLowerIDot",
    "charUpperIDot",
    "charLowerIDotless",
    "charLowerKelvin",
    "charUpperRoman",
    "charLowerRoman",
    "intLowerKelvin",
    "intUpperDeseret",
    "intLowerDeseret",
    "intUpperKelvin",
    "intLowerRoman",
    "intUpperRoman",
];

/// Rows the runtime must not answer: each is recorded as an unknown host call
/// and never compared against the oracle.
const COUNTED: &[&str] = &[
    // No `Locale`: anything outside ASCII.
    "l1EUpper",
    "l1ELower",
    "l1YenUpper",
    "l1YenLower",
    "l1SharpUpper",
    "l1SharpLower",
    "l1MicroUpper",
    "l1MicroLower",
    "leAMacronUpper",
    "leAMacronLower",
    "greekSigmaLower",
    "greekSigmaUpper",
    "turkishIDotUpper",
    "turkishIDotLower",
    "turkishIDotlessUpper",
    "turkishIDotlessLower",
    "kelvinUpper",
    "kelvinLower",
    "romanUpper",
    "romanLower",
    "deseretUpper",
    "deseretLower",
    "loneLowSurrogateUpper",
    "loneLowSurrogateLower",
    "loneHighSurrogateUpper",
    "loneHighSurrogateLower",
    // The `Locale` overloads: tailored locales (Turkish here), a null
    // locale, and every non-ASCII receiver.
    "asciiTrLower",
    "asciiTrUpper",
    "idotRootLower",
    "idotTrLower",
    "idotlessRootUpper",
    "idotlessTrUpper",
    "sharpRootUpper",
    // A null `Locale`: Java throws `NullPointerException`.
    "asciiNullLocaleUpper",
    "asciiNullLocaleLower",
];

/// The `Locale` overload rows among `COUNTED`.
const COUNTED_LOCALE: &[&str] = &[
    "asciiTrLower",
    "asciiTrUpper",
    "idotRootLower",
    "idotTrLower",
    "idotlessRootUpper",
    "idotlessTrUpper",
    "sharpRootUpper",
    "asciiNullLocaleUpper",
    "asciiNullLocaleLower",
];

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

struct Run {
    text: Option<String>,
    unknown: Vec<(String, usize)>,
    error: Option<NativeError>,
}

/// Run one static `String demo.Probe#method()` on a fresh seeded host,
/// keeping what the run reported as unknown host calls.
fn run(fixture: &javac_fixture::Fixture, method: &str) -> Run {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let result = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let text = match &result {
        Ok(Value::Ref(Some(text))) => Some(text.clone()),
        Ok(other) => panic!("{method} did not return a string: {other:?}"),
        Err(_) => None,
    };
    Run {
        text,
        unknown: host.unknown_host_calls(),
        error: result.err(),
    }
}

/// The unknown-host-call key one row's case method is counted under.
fn counted_key(method: &str) -> String {
    let case = if method.ends_with("Upper") {
        "Upper"
    } else {
        "Lower"
    };
    let descriptor = if COUNTED_LOCALE.contains(&method) {
        "(Ljava/util/Locale;)Ljava/lang/String;"
    } else {
        "()Ljava/lang/String;"
    };
    format!("java/lang/String#to{case}Case{descriptor}")
}

#[test]
fn answered_case_mappings_match_java_25_with_nothing_counted() {
    let fixture = javac_fixture::Fixture::build("string-case-mapping", SOURCES);
    for method in ANSWERED {
        let run = run(&fixture, method);
        assert!(run.error.is_none(), "{method} failed: {:?}", run.error);
        assert_eq!(
            run.text.as_deref(),
            Some(expected(method)),
            "{method}: unknown calls {:#?}",
            run.unknown
        );
        assert!(
            run.unknown.is_empty(),
            "{method}: answered but counted {:#?}",
            run.unknown
        );
    }
}

#[test]
fn counted_case_mappings_are_never_answered() {
    let fixture = javac_fixture::Fixture::build("string-case-mapping", SOURCES);
    for method in COUNTED {
        let run = run(&fixture, method);
        assert!(
            run.error.is_some(),
            "{method}: answered {:?} although it must stay counted",
            run.text
        );
        assert!(
            !run.unknown.is_empty(),
            "{method}: no unknown host calls although it must stay counted"
        );
        assert!(
            run.unknown
                .iter()
                .any(|(key, _)| key.as_str() == counted_key(method)),
            "{method}: expected a counted {expected} call, got {unknown:#?}",
            expected = counted_key(method),
            unknown = run.unknown
        );
    }
}

#[test]
fn every_oracle_row_is_accounted_for() {
    let covered: Vec<&str> = ANSWERED.iter().chain(COUNTED.iter()).copied().collect();
    let missing: Vec<&str> = ORACLE
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split_once('=').unwrap_or((line, "")).0)
        .filter(|label| !covered.contains(label))
        .collect();
    assert!(
        missing.is_empty(),
        "oracle rows not covered by the fixture: {missing:?}"
    );
    assert_eq!(
        ANSWERED.len() + COUNTED.len(),
        ORACLE.lines().filter(|line| !line.is_empty()).count(),
        "fixture rows and oracle rows differ in number"
    );
}

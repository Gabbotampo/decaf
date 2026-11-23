//! `String.indexOf(int)` / `indexOf(int,int)` / `lastIndexOf(int)` /
//! `lastIndexOf(int,int)` accept invalid code points exactly as Java 25 does:
//! values outside `0..=0x10FFFF` never match, a supplementary code point
//! matches its surrogate pair, and `fromIndex` is clamped. Cases involving
//! lone surrogates that the runtime cannot answer exactly stay counted
//! instead of being answered wrong.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;
use std::collections::HashMap;

const PROBE: &str = include_str!("fixtures/string_indexof_codepoint/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_indexof_codepoint/java25.txt");

fn parse_map(text: &str) -> HashMap<&str, &str> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .collect()
}

#[test]
fn string_indexof_codepoint_answers_as_java_25() {
    let fixture = Fixture::build("string-indexof-codepoint", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, "run");
    let unknown = host.unknown_host_calls();

    let oracle = parse_map(ORACLE);
    let runtime = parse_map(&text);

    let mut failures = Vec::new();

    for (label, oracle_value) in &oracle {
        let runtime_value = runtime.get(*label).copied().unwrap_or("<missing>");

        if runtime_value != *oracle_value {
            failures.push(format!(
                "{label}\n  java:   {oracle_value}\n  native: {runtime_value}"
            ));
        }
    }

    // All rows must be answered exactly (no counted calls for this probe,
    // since every haystack is a well-formed string).
    if !unknown.is_empty() {
        failures.push(format!("expected no unknown host calls, got {unknown:?}"));
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// When a `String` receiver is a jar-object marker (e.g. an unmodelled
/// constructor left a `String#n` reference instead of real text), the
/// call must be counted, never answered with the marker text.
#[test]
fn string_indexof_marked_receiver_stays_counted() {
    // A probe whose `run` returns a `String` built from an unmodelled
    // constructor leaves a marker receiver; `indexOf` on it must be counted.
    let source = r#"package demo;
public final class Probe {
    public static String run() {
        // `new String(char[])` with a lone surrogate is not modelled: the
        // constructor stays counted and the receiver is a marker.
        String s = new String(new char[] {'a', '\uD83D', 'c'});
        return String.valueOf(s.indexOf(97));
    }
}"#;
    let fixture = Fixture::build("string-indexof-counted", &[("demo/Probe.java", source)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _text = fixture.text_on(&mut host, "run");
    let unknown = host.unknown_host_calls();
    let has_indexof_unknown = unknown.iter().any(|(call, _)| {
        call.contains("java/lang/String#indexOf") || call.contains("java/lang/String#lastIndexOf")
    });
    assert!(
        has_indexof_unknown,
        "expected counted indexOf call, unknown_host_calls={unknown:?}"
    );
}

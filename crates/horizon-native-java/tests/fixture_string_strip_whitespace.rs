//! `String.strip`, `stripLeading`, `stripTrailing`, `trim` and `isBlank`,
//! plus `Character.isWhitespace` and `Character.isSpaceChar`, measured against
//! Java 25.
//
//! `demo/Probe.java` is the only class the test ships; it needs nothing but the
//! JDK. `java25.txt` holds its raw output, one `<row>=<value>` line per case,
//! prefixed with a unique label the test reads them by. The `ws`/`sp` rows are
//! the JDK's whole-range scans (and `Probe.main` throws unless `strip` and
//! friends agree with `Character.isWhitespace` on every code point); the test
//! sweeps the runtime over all 0..=0x10FFFF natively against them, and runs the
//! framed rows (`<label>.<op>`) through the interpreter. Captured with:
//
//! ```sh
//! $J/javac --release 21 -d out demo/Probe.java
//! $J/java -Xmx512m -cp out demo.Probe | head -c 2000000 > java25.txt
//! ```
#![allow(dead_code)]

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{Host, MethodRef, StdHost, Value};
use javac_fixture::Fixture;
use std::collections::BTreeSet;

const PROBE: &str = include_str!("fixtures/string_strip_whitespace/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/string_strip_whitespace/java25.txt");

fn method(owner: &str, name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: owner.into(),
        name: name.into(),
        descriptor: descriptor.into(),
    }
}

fn oracle_set(prefix: &str) -> BTreeSet<u32> {
    ORACLE
        .lines()
        .filter_map(|line| {
            let (label, value) = line.split_once('=')?;
            let hex = label.strip_prefix(prefix)?;
            (hex.len() == 6 && !hex.contains('.')).then(|| {
                assert_eq!(value, "1", "{line}");
                u32::from_str_radix(hex, 16).unwrap()
            })
        })
        .collect()
}

/// One slice of the sweep: the runtime's `isWhitespace`/`isSpaceChar` are the
/// JDK's sets on `range`, and `strip`, `stripLeading`, `stripTrailing` and
/// `isBlank` strip exactly the whitespace set there (the JDK probe checked that
/// equivalence for every code point). A lone surrogate is not a Rust `char`;
/// the two Character predicates are still checked there.
fn sweep(range: std::ops::Range<u32>, whitespace: &BTreeSet<u32>, space_char: &BTreeSet<u32>) {
    let is_whitespace = method("java/lang/Character", "isWhitespace", "(I)Z");
    let is_space_char = method("java/lang/Character", "isSpaceChar", "(I)Z");
    let strip = method("java/lang/String", "strip", "()Ljava/lang/String;");
    let lead = method("java/lang/String", "stripLeading", "()Ljava/lang/String;");
    let trail = method("java/lang/String", "stripTrailing", "()Ljava/lang/String;");
    let blank = method("java/lang/String", "isBlank", "()Z");
    let mut host = StdHost::new();
    let text = |value: Value| match value {
        Value::Ref(Some(text)) => text,
        other => panic!("not a string: {other:?}"),
    };
    for cp in range {
        let ws = whitespace.contains(&cp);
        assert_eq!(
            host.invoke(&is_whitespace, &[Value::Int(cp as i32)])
                .unwrap(),
            Value::Int(ws as i32),
            "isWhitespace U+{cp:06X}"
        );
        assert_eq!(
            host.invoke(&is_space_char, &[Value::Int(cp as i32)])
                .unwrap(),
            Value::Int(space_char.contains(&cp) as i32),
            "isSpaceChar U+{cp:06X}"
        );
        let Some(ch) = char::from_u32(cp) else {
            continue;
        };
        let subject = Value::Ref(Some(format!("{ch}a{ch}")));
        let (want_strip, want_lead, want_trail) = if ws {
            ("a".to_owned(), format!("a{ch}"), format!("{ch}a"))
        } else {
            (
                format!("{ch}a{ch}"),
                format!("{ch}a{ch}"),
                format!("{ch}a{ch}"),
            )
        };
        let got = |host: &mut StdHost, r: &MethodRef| {
            text(host.invoke_instance(r, &subject, &[]).unwrap())
        };
        assert_eq!(got(&mut host, &strip), want_strip, "strip U+{cp:06X}");
        assert_eq!(got(&mut host, &lead), want_lead, "stripLeading U+{cp:06X}");
        assert_eq!(
            got(&mut host, &trail),
            want_trail,
            "stripTrailing U+{cp:06X}"
        );
        assert_eq!(
            host.invoke_instance(&blank, &Value::Ref(Some(ch.to_string())), &[])
                .unwrap(),
            Value::Int(ws as i32),
            "isBlank U+{cp:06X}"
        );
    }
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// Every code point 0..=0x10FFFF, natively, in four slices on their own hosts.
#[test]
fn string_strip_whitespace_sweeps_every_code_point() {
    let whitespace = oracle_set("ws");
    let space_char = oracle_set("sp");
    assert_eq!(whitespace.len(), 25, "{whitespace:?}");
    assert_eq!(space_char.len(), 19, "{space_char:?}");
    const SLICE: u32 = 0x110000 / 4;
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|n| {
                let (whitespace, space_char) = (&whitespace, &space_char);
                scope.spawn(move || sweep(n * SLICE..(n + 1) * SLICE, whitespace, space_char))
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

/// The framed rows, through the interpreter: every whitespace code point and
/// the lookalikes U+0085, U+00A0, U+180E, U+2007, U+200B, U+202F and U+FEFF,
/// each in `"a"+c+"b"`, `c+"a"+c` and `c`, under every method.
#[test]
fn string_strip_whitespace_framed_rows_match_java25() {
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
    let oracle_lines: Vec<&str> = ORACLE
        .lines()
        .filter(|line| {
            line.split_once('=')
                .is_some_and(|(label, _)| label.contains('.'))
        })
        .collect();
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
        assert_eq!(got_value, Some(value), "row {label}: got {line:?}");
    }
    // Every row above is answered exactly, so nothing may stay counted.
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

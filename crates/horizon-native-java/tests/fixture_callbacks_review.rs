//! The #261 review's forgery attempts and lambda facts, one case per method,
//! against Java 25 (#156 P15a). A callback is a handle, never a `String`
//! (`callback_marker.rs`), so:
//!
//! * no composite, and no lambda of a held or unheld interface, casts to
//!   `String`/`CharSequence`/`Number`: Java's `ClassCastException`, its
//!   message (the hidden class) counted. The tag cannot be read out, and a
//!   `String` spelling a callback, a combinator (`@fn:identity/function`) or a
//!   handle inside one (`hsbld:0`) is a `String` (B1–B3);
//! * `instanceof`, casts, type switches and `Class.isInstance` answer from the
//!   lambda's interfaces, intersection-cast markers included (`Serializable`,
//!   a marker interface); an undecidable one is counted;
//! * captures and composites read back whole, however long (#156 P15b: a
//!   lambda's row holds its captures as values).
//!
//! `fixtures/callbacks_review/` is the reviewer's probe as written
//! (`lib.Cb`/`lib.Cb2` sit in a jar of their own, so the run does not hold
//! them); `java25.txt` is its output on OpenJDK 25.0.5+8.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = include_str!("fixtures/callbacks_review/demo/Probe.java");
const CB: &str = include_str!("fixtures/callbacks_review/lib/Cb.java");
const CB2: &str = include_str!("fixtures/callbacks_review/lib/Cb2.java");
const ORACLE: &str = include_str!("fixtures/callbacks_review/java25.txt");

/// The calls a case counts. Every other case counts nothing.
const COUNTED: &[(&str, &[&str])] = &[
    (
        "forgeComposite",
        &["java/lang/CharSequence#<checkcast>(Ljava/lang/Object;)Ljava/lang/CharSequence;"],
    ),
    (
        "forgeNegate",
        &["java/lang/String#<checkcast>(Ljava/lang/Object;)Ljava/lang/String;"],
    ),
    (
        "forgeUnknownSam",
        &["java/lang/String#<checkcast>(Ljava/lang/Object;)Ljava/lang/String;"],
    ),
    (
        "forgeUnknownSamCharSeq",
        &["java/lang/CharSequence#<checkcast>(Ljava/lang/Object;)Ljava/lang/CharSequence;"],
    ),
    (
        "forgeKnownSam",
        &["java/lang/CharSequence#<checkcast>(Ljava/lang/Object;)Ljava/lang/CharSequence;"],
    ),
    (
        "unkInstanceRunnable",
        &["java/lang/Runnable#<instanceof>(Ljava/lang/Object;)Z"],
    ),
    (
        "unkCastNumber",
        &["java/lang/Number#<checkcast>(Ljava/lang/Object;)Ljava/lang/Number;"],
    ),
    (
        "serIsInstance",
        &["java/lang/Comparable#<instanceof>(Ljava/lang/Object;)Z"],
    ),
    (
        "getClassAssignable",
        &["java/lang/Object#getClass()Ljava/lang/Class;"],
    ),
    ("hashTwice", &["java/lang/Runnable#hashCode()I"]),
    (
        "unkSuperRunnable",
        &["java/lang/Runnable#<instanceof>(Ljava/lang/Object;)Z"],
    ),
];

/// The cases whose answer is not Java's, each counted (above), and why.
const DEPARTURES: &[(&str, &str)] = &[
    // A lambda's `getClass()` names its hidden class: counted, and the `null`
    // it answers fails `isAssignableFrom` (P15b models the class).
    ("getClassAssignable", "NullPointerException"),
    // `lib.Cb2` is not in the jar, so whether it extends `Runnable` is
    // unknown: counted.
    ("unkSuperRunnable", "false,false"),
];

#[test]
fn every_review_case_answers_as_java_or_counts() {
    let fixture = javac_fixture::Fixture::build(
        "callbacks-review",
        &[
            ("demo/Probe.java", PROBE),
            ("lib/Cb.java", CB),
            ("lib/Cb2.java", CB2),
        ],
    );
    let lookup = |table: &[(&'static str, &'static [&'static str])], method: &str| {
        table
            .iter()
            .find(|(name, _)| *name == method)
            .map_or(&[][..], |(_, calls)| *calls)
    };
    let mut wrong = Vec::new();
    for line in ORACLE.lines() {
        let (method, java) = line.split_once('=').expect("case=answer");
        let mut host = fixture.jar.seed_host(StdHost::new());
        let shown = match fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        ) {
            Ok(Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let expected = DEPARTURES
            .iter()
            .find(|(name, _)| *name == method)
            .map_or(java, |(_, ours)| ours);
        let counted: Vec<String> = host
            .unknown_host_calls()
            .into_iter()
            .map(|(call, _)| call)
            .collect();
        if shown != expected || counted != lookup(COUNTED, method) {
            wrong.push(format!(
                "{method}: {shown} (want {expected}), counted {counted:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    // A departure is never silent.
    for (method, _) in DEPARTURES {
        assert!(!lookup(COUNTED, method).is_empty(), "{method}");
    }
}

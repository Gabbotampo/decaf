//! A `String` spelled like the host's old collection literals is a `String`
//! everywhere (#156 P14d2). Collections are handles now, so the `list/…`,
//! `set/…`, `map/…` text and the bare `list`/`set`/`map` words name nothing:
//! no reader parses them, and `value_class::RETIRED_PREFIXES` lists them.
//!
//! `fixtures/collection_forgery/demo/Probe.java` runs unchanged on both
//! sides (its `main` on Java 25 is `java25.txt`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/collection_forgery/java25.txt");

const PROBE: &str = include_str!("fixtures/collection_forgery/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// For `list/a,b`, `set/x`, `map/k=v`, the bare words and the generation and
/// immutable spellings (`list#i1/a,b`, `map#i2/k,v`, `list#3/a`, `set#i0`),
/// built at run time:
/// the class is `String`; `instanceof` is `String`'s and no collection's;
/// `equals` both ways and `hashCode` match an equal `String`; it is a
/// `HashMap` key and a list element as a `String`; `String.valueOf` and
/// concatenation print it; and a cast to `List`, `Set` or `Map` throws
/// `ClassCastException`. Java 25's answers, nothing counted.
#[test]
fn a_string_spelled_like_a_collection_literal_is_a_string() {
    let fixture =
        javac_fixture::Fixture::build("collection-forgery", &[("demo/Probe.java", PROBE)]);
    let mut failures = Vec::new();
    for method in ["list", "set", "map", "bare", "hashed"] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let text = match answer {
            Ok(horizon_native_java::Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != expected(method) || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

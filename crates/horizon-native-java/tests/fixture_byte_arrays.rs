//! `byte[]` end to end on a compact backing (#274).
//!
//! The runtime kept every array as `Value`s (32 bytes each), so EntityCulling's
//! `new byte[4 << 20]` cache held 128 MiB. A `byte[]` now keeps one byte per element
//! (`array_data.rs`); these are the paths a mod uses on one, each as Java 25 answers it:
//! `baload` sign extension, `arraylength`, the bounds exception, `System.arraycopy` within one
//! array in both directions and its refusals (kinds, `null`, bounds), `Arrays.copyOf`/
//! `copyOfRange`/`fill`/`equals`/`hashCode`/`toString`, `clone`, a 4 MiB array written and read
//! by index and copied, and text and streams over bytes.
//!
//! `fixtures/byte_arrays/demo/Probe.java` runs unchanged on both sides; its `main` on Java 25
//! (Temurin 25.0.5, `javac --release 21`) printed `java25.txt`, the same on two runs. `bastore`'s
//! narrowing of an `int` javac did not narrow is `lib.rs`'s raw-bytecode test.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const ORACLE: &str = include_str!("fixtures/byte_arrays/java25.txt");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

#[test]
fn byte_arrays_answer_as_java_25() {
    let fixture = javac_fixture::Fixture::build(
        "byte-arrays",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/byte_arrays/demo/Probe.java"),
        )],
    );
    let mut failures = Vec::new();
    for method in ["basics", "overlap", "copyErrors", "arraysOps", "big", "text"] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let text = fixture.text_on(&mut host, method);
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

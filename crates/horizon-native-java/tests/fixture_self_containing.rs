//! Collections that contain themselves, and other cycles a jar can build (#250).
//!
//! `l.add(l); l.toString()` overflowed the native stack and aborted the whole process. Java
//! prints `(this Collection)`/`(this Map)` for a collection inside itself, `[...]` for an array
//! inside itself in `Arrays.deepToString`, and throws a catchable `StackOverflowError` where it
//! recurses through a cycle (a deeper cycle's `toString`, `hashCode`, `equals`,
//! `Arrays.deepHashCode`, Gson's writer, a `compareTo` that recurses). The host answers the
//! same, and its walks never overflow the native stack (`host_recursion.rs`; a composed
//! predicate as deep as a jar builds it is that module's own test).
//!
//! Stage chains (`limit`, `sorted`, `skip`/`distinct`, `IntStream.limit`) thousands deep answer
//! as Java does on the test's 2 MiB thread, `(this Collection)` is printed only by the classes
//! whose `toString` checks for it (by the receiver's identity; a `Collections.unmodifiable*`
//! view prints its list), and a jar with an unused class annotated 66 levels deep loads.
//!
//! `fixtures/self_containing/demo/Probe.java` runs unchanged on both sides; its `main` on Java 25
//! (with the 26.3 server's gson-2.14.0) printed `java25.txt`. The Gson stubs exist only for
//! `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java demo.Probe` on Java 25 (Temurin 25.0.5, `javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/self_containing/java25.txt");

const GSON_STUB: &str = "package com.google.gson;\n\
public final class Gson { public String toJson(Object src) { throw new AssertionError(); } }\n";
const JSON_ELEMENT_STUB: &str = "package com.google.gson;\npublic abstract class JsonElement {}\n";
const JSON_OBJECT_STUB: &str = "package com.google.gson;\n\
public final class JsonObject extends JsonElement {\n\
public void add(String property, JsonElement value) { throw new AssertionError(); }\n}\n";

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

#[test]
fn self_containing_collections_answer_as_java_25() {
    let fixture = javac_fixture::Fixture::build(
        "self-containing",
        &[
            ("com/google/gson/Gson.java", GSON_STUB),
            ("com/google/gson/JsonElement.java", JSON_ELEMENT_STUB),
            ("com/google/gson/JsonObject.java", JSON_OBJECT_STUB),
            (
                "demo/Probe.java",
                include_str!("fixtures/self_containing/demo/Probe.java"),
            ),
            // An unused class annotated 66 levels deep: the jar still loads (#260 review).
            (
                "demo/DeepAnnotated.java",
                include_str!("fixtures/self_containing/demo/DeepAnnotated.java"),
            ),
        ],
    );
    let mut failures = Vec::new();
    for method in [
        "listSelf",
        "setSelf",
        "mapSelf",
        "nestedCycle",
        "hashSelf",
        "equalsSelf",
        "arraysSelf",
        "gsonCycle",
        "comparatorChain",
        "compareToCycle",
        "streamChains",
        "selfPrinting",
        "viewChain",
    ] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let text = fixture.text_on(&mut host, method);
        let unknown = host.unknown_host_calls();
        // A jar recursion caught at the jar-frame bound is counted (`fixture_deep_recursion.rs`):
        // `compareToCycle` recurses without end, `streamChains` runs its last chain where such a
        // recursion was caught. The bound cannot tell an endless recursion from a deep one.
        let counted: &[&str] = if matches!(method, "streamChains" | "compareToCycle") {
            &["java/lang/StackOverflowError#<frame-bound>()V"]
        } else {
            &[]
        };
        let names: Vec<&str> = unknown.iter().map(|(call, _)| call.as_str()).collect();
        if text != expected(method) || names != counted {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

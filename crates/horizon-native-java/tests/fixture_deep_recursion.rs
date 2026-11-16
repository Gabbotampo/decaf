//! Deep Java recursion, and the Gson writer that aborted Sodium (the client census).
//!
//! The census probe aborted on every Sodium build ("thread 'main' has overflowed its stack"):
//! `SodiumOptions.writeToDisk` writes its options through a `Gson` built with
//! `excludeFieldsWithModifiers(PRIVATE)`, which writes static fields, and the writer reflected over
//! enum constants, whose `public static` constant fields are the enum again, without end. Gson
//! writes an enum by its constant's `@SerializedName` value or `name()` (`EnumTypeAdapter`), and
//! `@SerializedName` names a field's key and the alternates it reads.
//!
//! A jar's own recursion is bounded by the thread's stack (`frame_bound.rs`) and caught as Java's
//! `StackOverflowError`. That bound nests far fewer frames than HotSpot's default stack, so a
//! caught overflow from it is counted, naming the bound and its depth
//! (`java/lang/StackOverflowError#<frame-bound:DEPTH>`, #260): the jar's recursion may have
//! finished on the JVM.
//!
//! `fixtures/deep_recursion/demo/Probe.java` runs unchanged on both sides; its `main` on Java 25
//! with the 26.3 server's gson-2.14.0 printed `java25.txt`. The Gson stubs exist only for `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java demo.Probe` on Java 25 (Temurin 25.0.5, `javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/deep_recursion/java25.txt");

const GSON_STUB: &str = "package com.google.gson;\n\
public final class Gson {\n\
public String toJson(Object src) { throw new AssertionError(); }\n\
public <T> T fromJson(String json, Class<T> type) { throw new AssertionError(); }\n}\n";
const GSON_BUILDER_STUB: &str = "package com.google.gson;\n\
public final class GsonBuilder {\n\
public GsonBuilder setFieldNamingPolicy(FieldNamingPolicy policy) { return this; }\n\
public GsonBuilder setPrettyPrinting() { return this; }\n\
public GsonBuilder excludeFieldsWithModifiers(int... modifiers) { return this; }\n\
public GsonBuilder enableComplexMapKeySerialization() { return this; }\n\
public Gson create() { throw new AssertionError(); }\n}\n";
const FIELD_NAMING_STUB: &str = "package com.google.gson;\n\
public class FieldNamingPolicy { public static FieldNamingPolicy LOWER_CASE_WITH_UNDERSCORES; }\n";
const SERIALIZED_NAME_STUB: &str = "package com.google.gson.annotations;\n\
@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.RUNTIME)\n\
@java.lang.annotation.Target({java.lang.annotation.ElementType.FIELD})\n\
public @interface SerializedName { String value(); String[] alternate() default {}; }\n";

/// The count a frame-bound `StackOverflowError` leaves when a jar catches it (its depth follows).
const FRAME_BOUND: &str = "java/lang/StackOverflowError#<frame-bound:";

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "deep-recursion",
        &[
            ("com/google/gson/Gson.java", GSON_STUB),
            ("com/google/gson/GsonBuilder.java", GSON_BUILDER_STUB),
            ("com/google/gson/FieldNamingPolicy.java", FIELD_NAMING_STUB),
            (
                "com/google/gson/annotations/SerializedName.java",
                SERIALIZED_NAME_STUB,
            ),
            (
                "demo/Probe.java",
                include_str!("fixtures/deep_recursion/demo/Probe.java"),
            ),
        ],
    )
}

/// The method's text and the unknown calls it counted, by name.
fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<String>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    let unknown = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    (text, unknown)
}

/// Gson writes jar enums by name (or `@SerializedName`), and `@SerializedName` names fields both
/// ways: Sodium's options document is Java's, uncounted.
#[test]
fn gson_writes_enums_and_serialized_names_as_java_25() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for method in [
        "shallowRecursion",
        "sodiumOptions",
        "enumValues",
        "enumMapKeys",
        "enumBodyValue",
        "enumBodyMapKey",
    ] {
        let (text, unknown) = run(&fixture, method);
        if text != expected(method) || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A recursion a million frames deep ends in a `StackOverflowError` the jar catches and keeps
/// running after, never a native overflow; the bound's throw is counted.
#[test]
fn a_million_frame_recursion_is_a_caught_stack_overflow() {
    let fixture = fixture();
    let (text, unknown) = run(&fixture, "deepRecursion");
    assert_eq!(text, expected("deepRecursion"), "{unknown:?}");
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert!(unknown[0].starts_with(FRAME_BOUND), "{unknown:?}");
}

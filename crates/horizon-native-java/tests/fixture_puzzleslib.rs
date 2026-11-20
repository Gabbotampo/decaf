//! Java 25 fixture for the small Guava/Commons calls used by puzzleslib.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const STRING_UTILS_STUB: &str = r#"package org.apache.commons.lang3;
public final class StringUtils {
    public static boolean isNotEmpty(CharSequence value) { throw new AssertionError(); }
}
"#;

const QUEUES_STUB: &str = r#"package com.google.common.collect;
public final class Queues {
    public static <E> java.util.concurrent.ConcurrentLinkedQueue<E> newConcurrentLinkedQueue() {
        throw new AssertionError();
    }
}
"#;

const MAPS_STUB: &str = r#"package com.google.common.collect;
public final class Maps {
    public static <K extends Enum<K>, V> ImmutableMap<K, V> immutableEnumMap(java.util.Map<K, V> source) {
        throw new AssertionError();
    }
}
"#;

const IMMUTABLE_MAP_STUB: &str = r#"package com.google.common.collect;
public class ImmutableMap<K, V> extends java.util.AbstractMap<K, V> {
    public java.util.Set<java.util.Map.Entry<K, V>> entrySet() { throw new AssertionError(); }
}
"#;

const PROBE: &str = include_str!("fixtures/puzzleslib/Probe.java");

// These two tiny library classes exercise the same cross-jar linkage shape as
// Collective's `ShouldLoadCheck` and Puzzles Lib's static `ModConstructor`:
// the answer must come from the linked bytecode, rather than a host special
// case for either owner or method name.
const LINKED_SHOULD_LOAD: &str = r#"package lib;
public final class ShouldLoadCheck {
    public static boolean shouldLoad(String id) { return id.equals("enabled"); }
}
"#;

const LINKED_MOD_CONSTRUCTOR: &str = r#"package lib;
public interface ModConstructor {
    static void construct(String id, java.util.function.Supplier<ModConstructor> supplier) {
        if (id.equals("run")) supplier.get().onConstructMod();
    }
    void onConstructMod();
}
"#;

const LINKED_CALL_PROBE: &str = r#"package demo;
public final class LinkedCallProbe {
    static boolean constructed;
    public static String run() {
        lib.ModConstructor.construct("run", () -> () -> constructed = true);
        return lib.ShouldLoadCheck.shouldLoad("enabled") + "|" + constructed + "|"
            + lib.ShouldLoadCheck.shouldLoad("disabled");
    }
}
"#;

#[test]
fn puzzleslib_dependency_calls_match_java25() {
    let fixture = Fixture::build(
        "puzzleslib",
        &[
            (
                "org/apache/commons/lang3/StringUtils.java",
                STRING_UTILS_STUB,
            ),
            ("com/google/common/collect/Queues.java", QUEUES_STUB),
            ("com/google/common/collect/Maps.java", MAPS_STUB),
            (
                "com/google/common/collect/ImmutableMap.java",
                IMMUTABLE_MAP_STUB,
            ),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "modId"), "true|false|false");
    assert_eq!(fixture.text_on(&mut host, "queue"), "true|one|true");
    assert_eq!(fixture.text_on(&mut host, "phase"), "add|post");
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn linked_library_bytecode_decides_static_calls() {
    let fixture = Fixture::build_with_lib(
        "linked-library-calls",
        &[
            ("lib/ShouldLoadCheck.java", LINKED_SHOULD_LOAD),
            ("lib/ModConstructor.java", LINKED_MOD_CONSTRUCTOR),
            ("demo/LinkedCallProbe.java", LINKED_CALL_PROBE),
        ],
        &[],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    host.link_jar("library", fixture.lib.as_ref().unwrap())
        .unwrap();
    let result = fixture
        .jar
        .invoke_method_with_args(
            "demo/LinkedCallProbe",
            "run",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        result,
        horizon_native_java::Value::Ref(Some("true|true|false".into()))
    );
    assert_eq!(host.unknown_host_calls(), []);
}

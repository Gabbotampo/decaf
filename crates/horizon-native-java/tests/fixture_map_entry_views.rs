//! Entries read through `Collections.unmodifiableMap` (#221 review 6, B10).
//!
//! Java's view hands out a new `UnmodifiableEntry` per `next()`, wrapping the entry the source map
//! hands out, which answers every read; `setValue` throws. The view's entry used to read its
//! source live, guarded only by the source's `modCount` stamp, which Gson's writes do not move and
//! a `ConcurrentHashMap`/`ConcurrentSkipListMap` entry does not follow (it keeps its own value):
//! `5|a=5` where Java gives `1|a=1`, silently. Now the wrapper answers exactly what a direct read
//! of the source answers (`host_map_entry.rs`, `Life::Wrapped`): a Gson reading only while the
//! object is unwritten (counted after), a concurrent map's entry its own value, a `HashMap` node
//! live.
//!
//! The stubs exist only for `javac`: the host models Gson itself. Expected values: OpenJDK 25.0.5
//! running `fixtures/map_entry_views/demo/Probe.java` with the 26.3 server's gson-2.14.0, the same
//! on two runs.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const ORACLE: &str = include_str!("fixtures/map_entry_views/java25.txt");

const JSON_ELEMENT_STUB: &str = "package com.google.gson;\n\
public abstract class JsonElement {\n\
public JsonObject getAsJsonObject() { throw new AssertionError(); }\n}\n";
const JSON_PRIMITIVE_STUB: &str = "package com.google.gson;\n\
public final class JsonPrimitive extends JsonElement {\n\
public JsonPrimitive(Number n) { throw new AssertionError(); }\n}\n";
const JSON_OBJECT_STUB: &str = "package com.google.gson;\n\
public final class JsonObject extends JsonElement {\n\
public void add(String p, JsonElement v) { throw new AssertionError(); }\n\
public void addProperty(String p, Number v) { throw new AssertionError(); }\n\
public JsonElement remove(String p) { throw new AssertionError(); }\n\
public JsonObject getAsJsonObject(String p) { throw new AssertionError(); }\n\
public java.util.Map<String, JsonElement> asMap() { throw new AssertionError(); }\n}\n";
const JSON_ARRAY_STUB: &str = "package com.google.gson;\n\
public final class JsonArray extends JsonElement {\n\
public void add(JsonElement v) { throw new AssertionError(); }\n\
public JsonElement get(int i) { throw new AssertionError(); }\n}\n";
const JSON_PARSER_STUB: &str = "package com.google.gson;\n\
public final class JsonParser {\n\
public static JsonElement parseString(String json) { throw new AssertionError(); }\n}\n";

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "map-entry-views",
        &[
            ("com/google/gson/JsonElement.java", JSON_ELEMENT_STUB),
            ("com/google/gson/JsonPrimitive.java", JSON_PRIMITIVE_STUB),
            ("com/google/gson/JsonObject.java", JSON_OBJECT_STUB),
            ("com/google/gson/JsonArray.java", JSON_ARRAY_STUB),
            ("com/google/gson/JsonParser.java", JSON_PARSER_STUB),
            (
                "demo/Probe.java",
                include_str!("fixtures/map_entry_views/demo/Probe.java"),
            ),
        ],
    )
}

/// The method's text and the names of the calls it counted.
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

const GET_VALUE: &str = "java/util/Map$Entry#getValue()Ljava/lang/Object;";

/// A `ConcurrentHashMap.MapEntry` keeps the value it was made with and a
/// `ConcurrentSkipListMap` entry is a snapshot, through one view or two; a `HashMap` node reads
/// live and the view refuses `setValue`: all as Java 25, nothing counted.
#[test]
fn views_of_concurrent_and_hash_maps_answer_as_java_25() {
    let fixture = fixture();
    for method in [
        "v18ChmView",
        "v19ChmViewRemovePut",
        "c02ChmViewOfView",
        "a02CslmView",
        "hashMapView",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(unknown.is_empty(), "{method}: {unknown:?}");
    }
}

/// A Gson object's entry read through a view, then the object written (its key removed and
/// added again, through `remove`/`addProperty`, `asMap()`, `clear`, a nested or array-held or
/// parsed object, a view of a view, a held `entrySet()`, `synchronizedMap` over the view): Java's
/// entry is the removed node, which keeps `1`. As a direct reading, it answers only while the
/// object is unwritten, so its value is counted (`Map.Entry.getValue`), never the re-added `5`.
#[test]
fn views_of_gson_objects_count_a_reading_after_a_write() {
    let fixture = fixture();
    for method in [
        "v01UnmodView",
        "v09UnmodOfUnmod",
        "v10NestedObjView",
        "v11ArrayObjView",
        "v12ViewEntrySetHeld",
        "v14ViewAsMapWrites",
        "v16ViewClear",
        "v17ViewOfParsed",
        "g04SyncOfUnmodGson",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(expected(method).split('|').next(), Some("1"), "{method}");
        assert!(
            !text.starts_with("5|"),
            "{method}: the re-added value, {text}"
        );
        assert!(
            unknown.iter().any(|call| call == GET_VALUE),
            "{method}: {text}, {unknown:?}"
        );
    }
    // The entry's text from an array the view's `toArray` filled (Java `a=1`): counted.
    let (text, unknown) = run(&fixture, "g01ViewToArray");
    assert_ne!(text, "a=5");
    assert!(
        unknown
            .iter()
            .any(|call| call == "java/lang/Object#toString()Ljava/lang/String;"),
        "g01ViewToArray: {text}, {unknown:?}"
    );
    // Two readings across the write (Java `false|2|96`): their `equals` and `hashCode` are
    // counted, never the re-added node's (`true|1|100`).
    let (text, unknown) = run(&fixture, "v13ViewEqualsHashSet");
    assert_ne!(text, "true|1|100");
    for call in [
        "java/util/Map$Entry#equals(Ljava/lang/Object;)Z",
        "java/util/Map$Entry#hashCode()I",
    ] {
        assert!(
            unknown.iter().any(|counted| counted == call),
            "v13ViewEqualsHashSet: {call} in {unknown:?}"
        );
    }
}

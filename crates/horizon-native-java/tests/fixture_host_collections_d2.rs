//! Collections models make are objects of their own, of the class Java
//! makes (#156 P14d2): `Map.ofEntries`, `List.copyOf`/`Set.copyOf`,
//! `Stream.toList()`, `Collectors.toList()`/`toUnmodifiableList()`, a
//! stream's iterator. Each used to be a `list/…`/`map/…` text literal. Held
//! in a static, each lives through collections (#226's collector).
//!
//! `fixtures/host_collections_d2/demo/Probe.java` runs unchanged on both
//! sides (its `main` on Java 25 is `java25.txt`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/host_collections_d2/java25.txt");

const PROBE: &str = include_str!("fixtures/host_collections_d2/demo/Probe.java");

fn expected(name: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {name}"))
}

/// Made in one call and kept in a static, then 40 garbage-making calls (each
/// store may collect): every collection answers its lookups, text, class
/// and mutability as Java 25 does, and the iterator goes on where it was.
/// Nothing is counted.
#[test]
fn model_made_collections_are_javas_and_survive_collections() {
    let fixture =
        javac_fixture::Fixture::build("host-collections-d2", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let mut call = |method: &str| match fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    ) {
        Ok(horizon_native_java::Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    };
    assert_eq!(call("setup"), expected("setup"));
    for _ in 0..40 {
        call("garbage");
    }
    assert_eq!(call("check"), expected("check"));
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// What `demo.Review`'s `main` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`) with Gson 2.14.0 and Guava 33.6.0-jre, the same on two
/// runs. Horizon compiles it against the stubs beside it.
const REVIEW_ORACLE: &str = include_str!("fixtures/host_collections_d2/review25.txt");

const REVIEW: &str = include_str!("fixtures/host_collections_d2/demo/Review.java");

const STUBS: &[(&str, &str)] = &[
    (
        "com/google/gson/Gson.java",
        include_str!("fixtures/host_collections_d2/stubs/com/google/gson/Gson.java"),
    ),
    (
        "com/google/common/collect/ImmutableMap.java",
        include_str!(
            "fixtures/host_collections_d2/stubs/com/google/common/collect/ImmutableMap.java"
        ),
    ),
    (
        "com/google/common/collect/ImmutableSet.java",
        include_str!(
            "fixtures/host_collections_d2/stubs/com/google/common/collect/ImmutableSet.java"
        ),
    ),
    (
        "com/google/common/collect/Sets.java",
        include_str!("fixtures/host_collections_d2/stubs/com/google/common/collect/Sets.java"),
    ),
    (
        "com/google/common/collect/Multimap.java",
        include_str!("fixtures/host_collections_d2/stubs/com/google/common/collect/Multimap.java"),
    ),
    (
        "com/google/common/collect/HashMultimap.java",
        include_str!(
            "fixtures/host_collections_d2/stubs/com/google/common/collect/HashMultimap.java"
        ),
    ),
];

fn review_expected(method: &str) -> &'static str {
    REVIEW_ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// #253's review, as Java 25 answers with nothing counted:
/// - `gsonArrays`: each element read as the component type (`1` into a
///   `String[]` is `"1"`, whose `length()` is 1), the array of its class;
/// - `gsonSets`: a `TreeSet` target sorted and deduplicated, a `Set` target
///   a deduplicated `LinkedHashSet`;
/// - `guavaIdentity`: `ImmutableMap.of()` (and an empty `toImmutableMap`) is
///   one object, a multimap's `keySet()`/`values()` the same view each call;
/// - `guavaClasses`: `RegularImmutableMap`, `SingletonImmutableBiMap`,
///   `ImmutableEnumSet` in ordinal order, `SingletonImmutableSet`, and
///   `Map.copyOf`/`Set.copyOf` of them a copy, not the argument.
///
/// `gsonForeign` (a jar class's array) needs Gson's reflective adapter: Java
/// 25's answer, or the `fromJson` counted, never an element of another type.
#[test]
fn guava_and_gson_collections_are_javas() {
    let mut sources = vec![("demo/Review.java", REVIEW)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("host-collections-d2-review", &sources);
    let mut failures = Vec::new();
    for method in [
        "gsonArrays",
        "gsonSets",
        "guavaIdentity",
        "guavaClasses",
        "gsonForeign",
    ] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Review",
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
        let exact = text == review_expected(method) && unknown.is_empty();
        let counted = method == "gsonForeign"
            && unknown
                .iter()
                .any(|(call, _)| call.starts_with("com/google/gson/Gson#fromJson"));
        if !(exact || counted) {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                review_expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

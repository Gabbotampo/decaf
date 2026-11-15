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
/// - `gsonNarrow`: Gson's `short`/`byte` narrowing, a `float` as a float;
/// - `gsonObjects`: `ObjectTypeAdapter`'s `Boolean`/`Double`/`ArrayList`;
/// - `gsonSorted`: `SortedMap`/`SortedSet`/`NavigableSet` targets are a
///   `TreeMap`/`TreeSet`;
/// - `gsonGuavaJson`: a Guava map serializes as a JSON object;
/// - `guavaClassSurface`: Guava's superclasses, and `isAssignableFrom`.
///
/// - `gsonForeign`, `gsonShortBox`, `gsonFloatBox`: a jar class's array
///   (its objects through the reflective adapter), `Short[]` and `Float[]`.
///
/// Counted, never another value: a `byte` past 255 (`JsonSyntaxException`)
/// and a JDK enum's ordinal order.
#[test]
fn guava_and_gson_collections_are_javas() {
    let mut sources = vec![("demo/Review.java", REVIEW)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("host-collections-d2-review", &sources);
    let mut failures = Vec::new();
    // Each case and, where Horizon cannot hold Java's value, the call it
    // counts instead (`None`: exact only).
    let cases: &[(&str, Option<&str>)] = &[
        ("gsonArrays", None),
        ("gsonSets", None),
        ("guavaIdentity", None),
        ("guavaClasses", None),
        ("gsonNarrow", None),
        ("gsonObjects", None),
        ("gsonSorted", None),
        ("gsonGuavaJson", None),
        ("guavaClassSurface", None),
        ("gsonForeign", None),
        ("gsonByteOverflow", Some("com/google/gson/Gson#fromJson")),
        ("gsonShortBox", None),
        ("gsonFloatBox", None),
        (
            "guavaJdkEnum",
            Some("com/google/common/collect/Sets#immutableEnumSet"),
        ),
    ];
    for (method, counts) in cases {
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
        let counted =
            counts.is_some_and(|call| unknown.iter().any(|(counted, _)| counted.starts_with(call)));
        if !(exact || counted) {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                review_expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What `demo.Recheck`'s `main` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`) with Gson 2.14.0, the same on two runs.
const RECHECK_ORACLE: &str = include_str!("fixtures/host_collections_d2/recheck25.txt");

const RECHECK: &str = include_str!("fixtures/host_collections_d2/demo/Recheck.java");

/// One oracle line of `oracle`.
fn line_of(oracle: &'static str, method: &str) -> &'static str {
    oracle
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// #253's re-check, `Gson.fromJson` into collections:
/// - exact, nothing counted: a `Map[]`'s maps are `LinkedTreeMap`s in
///   document order holding `Double`s, `HashMap[]`/`TreeMap[]`/`SortedMap[]`
///   their own classes, `List[]`/`Collection[]` `ArrayList`s of `Double` and
///   `Boolean`, a `SortedSet`'s numbers `Double`s, a raw `Map`'s values
///   `ObjectTypeAdapter`'s (a nested `LinkedTreeMap` in document order), a
///   `LinkedHashMap` target in document order, and a jar class's
///   `Map<String, Integer>`, `List<Double>`, `Set<String>`, `int[]`,
///   `Map<Integer, String>` and `List<List<Long>>` fields as declared;
/// - a `String` spelled `<collection class>#<word>` is a `String`: no
///   collection type's `instanceof`, a `List` cast throws;
/// - counted, where Java throws or makes a class not modelled here:
///   `Iterable[]` (`JsonIOException`), a `TreeSet` of a `String` and a number
///   (`ClassCastException`), a `Queue` (`ArrayDeque`), a repeated name
///   (`JsonSyntaxException`).
#[test]
fn recheck_gson_reads_are_javas_or_counted() {
    let mut sources = vec![("demo/Recheck.java", RECHECK)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("host-collections-d2-recheck", &sources);
    let exact = [
        "gsonMapArray",
        "gsonHashMapArray",
        "gsonSortedMapArrays",
        "gsonListArray",
        "gsonCollectionArray",
        "gsonSortedSetNumbers",
        "gsonMapValues",
        "gsonLinkedHashMap",
        "gsonTypedFields",
        "forgedCollectionText",
    ];
    let counted = [
        "gsonIterableArray",
        "gsonTreeSetMixed",
        "gsonQueue",
        "gsonDuplicateKey",
    ];
    let mut failures = Vec::new();
    for (method, must_count) in exact
        .iter()
        .map(|method| (*method, false))
        .chain(counted.iter().map(|method| (*method, true)))
    {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Recheck",
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
        let ok = if must_count {
            unknown
                .iter()
                .any(|(call, _)| call.starts_with("com/google/gson/Gson#fromJson"))
        } else {
            text == line_of(RECHECK_ORACLE, method) && unknown.is_empty()
        };
        if !ok {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                line_of(RECHECK_ORACLE, method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What `demo.Inv`'s `main` printed on Java 25 with 26.3's real
/// `net.minecraft.core.NonNullList` (from `server-game.jar`) held as
/// `Inventory.items` is (the `fakes/` classes), the same on two runs.
const INV_ORACLE: &str = include_str!("fixtures/host_collections_d2/inv25.txt");

const INV: &str = include_str!("fixtures/host_collections_d2/demo/Inv.java");

/// `demo.Inv`'s compile-time classes: the game's client classes as the
/// oracle's fakes (only `demo/` is packed, so the host answers them) and a
/// `NonNullList` stub.
const INV_SOURCES: &[(&str, &str)] = &[
    ("demo/Inv.java", INV),
    (
        "net/minecraft/client/Minecraft.java",
        include_str!("fixtures/host_collections_d2/fakes/net/minecraft/client/Minecraft.java"),
    ),
    (
        "net/minecraft/client/player/LocalPlayer.java",
        include_str!(
            "fixtures/host_collections_d2/fakes/net/minecraft/client/player/LocalPlayer.java"
        ),
    ),
    (
        "net/minecraft/world/entity/player/Inventory.java",
        include_str!(
            "fixtures/host_collections_d2/fakes/net/minecraft/world/entity/player/Inventory.java"
        ),
    ),
    (
        "net/minecraft/core/NonNullList.java",
        include_str!("fixtures/host_collections_d2/stubs/net/minecraft/core/NonNullList.java"),
    ),
];

/// #253's re-check on the inventory's list, inside a client frame:
/// - `surface`: its class is `NonNullList`, whose superclass is
///   `AbstractList`, a `List` and an `AbstractList` to `isAssignableFrom`,
///   no `RandomAccess`, no interfaces of its own: Java 25's answer, nothing
///   counted;
/// - every `subList` (through `NonNullList` or `List`): Java's is an
///   `AbstractList$SubList` with `NonNullList`'s rules (`add`/`clear`/
///   `remove` throw, `set(null)` throws, `set` writes through), which is not
///   modelled: counted, never a writable copy.
#[test]
fn the_inventory_list_has_its_class_surface_and_its_sublists_are_counted() {
    use horizon_native_java::client_surface::{ClientPlayer, ClientSnapshot};
    let fixture = javac_fixture::Fixture::build("host-collections-d2-inventory", INV_SOURCES);
    let mut failures = Vec::new();
    for method in [
        "surface",
        "subListClass",
        "subListSet",
        "subListAdd",
        "subListClear",
        "subListRemove",
        "subListSetNull",
        "listSubListSet",
        "listSubListClear",
    ] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        host.begin_client_frame(
            std::sync::Arc::new(ClientSnapshot {
                player: Some(ClientPlayer::default()),
                ..ClientSnapshot::default()
            }),
            &[],
        );
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Inv",
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
        let ok = if method == "surface" {
            text == line_of(INV_ORACLE, method) && unknown.is_empty()
        } else {
            unknown.iter().any(|(call, _)| call.contains("#subList("))
        };
        if !ok {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                line_of(INV_ORACLE, method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

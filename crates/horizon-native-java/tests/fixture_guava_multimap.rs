//! Guava 33.6.0 multimap fixture compiled against the real jar on arcweld.
//! Expected strings below come from Temurin Java 25.0.5 with `javac --release 21`
//! and `guava-33.6.0-jre.jar`, never from the native implementation.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use javac_fixture::Fixture;
use std::path::PathBuf;

const AS_MAP_ORACLE: &str = include_str!("fixtures/guava_multimap/java25.txt");
const PROBE: &str = include_str!("fixtures/guava_multimap/Probe.java");

fn guava_jar() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("GUAVA_26_JAR").or_else(|_| std::env::var("GUAVA_JAR")) {
        candidates.push(PathBuf::from(path));
    }
    candidates.extend([
        PathBuf::from("/Users/sparks/ava/scratch/game-pure-values/mc/libraries/server/com/google/guava/guava/33.6.0-jre/guava-33.6.0-jre.jar"),
        PathBuf::from("/home/solenopsisbot/ava/hj-tools/guava-33.6.0-jre.jar"),
    ]);
    candidates.into_iter().find(|path| path.is_file())
}

#[test]
fn real_guava_multimaps_match_java25() {
    let Some(guava) = guava_jar() else {
        eprintln!("GUAVA_JAR is unavailable; real-Guava fixture is arcweld-only");
        return;
    };
    let fixture =
        Fixture::build_with_classpath("guava-multimap", &[("demo/Probe.java", PROBE)], &guava);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let mut oracle = AS_MAP_ORACLE.lines();
    for (method, expected) in [
        ("array", "true|true|true|2|true|true|true|2|true|1|false"),
        ("hash", "true|true|true|2|true|true|true|2|true|1|false"),
        ("linked", "true|true|true|2|true|true|true|2|true|1|false"),
    ] {
        assert_eq!(fixture.text_on(&mut host, method), expected, "{method}");
    }
    for method in ["entryArray", "entryHash", "entryLinked"] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            oracle.next().unwrap(),
            "{method}"
        );
    }
    assert_eq!(fixture.text_on(&mut host, "sync"), "true");
    assert_eq!(fixture.text_on(&mut host, "excluded"), "done");
    assert_eq!(
        host.unknown_host_call_count(),
        16,
        "{:?}",
        host.unknown_host_calls()
    );
    for method in [
        "asMapIdentity",
        "asMapRemoveArray",
        "asMapRemoveHash",
        "asMapRemoveLinked",
        "iteratorRemoveArray",
        "iteratorRemoveHash",
        "iteratorRemoveLinked",
    ] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            oracle.next().unwrap(),
            "{method}"
        );
    }
    for method in [
        "wrappersArray",
        "wrappersHash",
        "wrappersLinked",
        "entrySetHandlesArray",
        "entrySetHandlesHash",
        "entrySetHandlesLinked",
        "instanceofKinds",
        "freshGet",
    ] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            oracle.next().unwrap(),
            "{method}"
        );
    }
    for method in [
        "removeAllRefresh",
        "instanceofArrayAll",
        "instanceofHashAll",
        "instanceofLinkedAll",
    ] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            oracle.next().unwrap(),
            "{method}"
        );
    }
    for method in [
        "entries",
        "keys",
        "replaceValues",
        "removeAll",
        "equals",
        "toString",
        "containsKey",
        "containsEntry",
        "put",
        "remove",
    ] {
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(call, _)| call.contains(&format!("#{method}"))),
            "excluded method {method} was not counted: {:?}",
            host.unknown_host_calls()
        );
    }
    for method in [
        "countedWrappersArray",
        "countedWrappersHash",
        "countedWrappersLinked",
        "countedValues",
        "countedEntrySetToString",
        "taintedRead",
    ] {
        let _ = fixture.text_on(&mut host, method);
    }
    for call in [
        "#listIterator()Ljava/util/ListIterator;",
        "#subList(II)Ljava/util/List;",
        "#toString()Ljava/lang/String;",
        "#contains(Ljava/lang/Object;)Z",
        "#remove(Ljava/lang/Object;)Z",
    ] {
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(name, _)| name.contains(call)),
            "reviewer case was not counted: {call}; calls: {:?}",
            host.unknown_host_calls()
        );
    }
    for method in [
        "countedMutationEntries",
        "countedMutationKeys",
        "countedMutationReplaceValues",
        "countedMutationAsMapValuesRemove",
    ] {
        assert_eq!(fixture.text_on(&mut host, method), "done", "{method}");
        for call in [
            "#size()I",
            "#get(Ljava/lang/Object;)",
            "#containsKey(Ljava/lang/Object;)",
            "#asMap()Ljava/util/Map;",
            "#entries()Ljava/util/Collection;",
        ] {
            assert!(
                host.unknown_host_calls()
                    .iter()
                    .any(|(name, _)| name.contains(call)),
                "{method} did not count later read {call}: {:?}",
                host.unknown_host_calls()
            );
        }
    }
    let before = host.unknown_host_calls();
    let count = |calls: &[(String, usize)], needle: &str| {
        calls
            .iter()
            .filter(|(name, _)| name.contains(needle))
            .map(|(_, count)| *count)
            .sum::<usize>()
    };
    let before_identity_hash = count(&before, "java/lang/System#identityHashCode");
    let before_size = count(&before, "#size()I");
    assert_eq!(
        fixture.text_on(&mut host, "countedEntryTaintsMultimap"),
        "done"
    );
    let after = host.unknown_host_calls();
    assert!(
        count(&after, "java/lang/System#identityHashCode") > before_identity_hash,
        "entry counted call missing: {after:?}"
    );
    assert!(
        count(&after, "#size()I") > before_size,
        "entry-tainted size was not counted: {after:?}"
    );
    let before_entry_key = count(&after, "java/util/Map$Entry#getKey()Ljava/lang/Object;");
    assert_eq!(fixture.text_on(&mut host, "entryAfterTaintCounts"), "done");
    assert!(
        count(
            &host.unknown_host_calls(),
            "java/util/Map$Entry#getKey()Ljava/lang/Object;",
        ) > before_entry_key,
        "entry after taint was not counted: {:?}",
        host.unknown_host_calls()
    );
    let before_read_size = count(&host.unknown_host_calls(), "#size()I");
    assert_eq!(
        fixture.text_on(&mut host, "countedReadTaintsMultimap"),
        "done"
    );
    assert!(
        count(&host.unknown_host_calls(), "#size()I") > before_read_size,
        "read-only counted call did not taint later size: {:?}",
        host.unknown_host_calls()
    );
    let before_iterator = host.unknown_host_calls();
    let before_iterator_size = count(&before_iterator, "#size()I");
    assert_eq!(
        fixture.text_on(&mut host, "iteratorReadPreservesMultimap"),
        "done"
    );
    let after_iterator = host.unknown_host_calls();
    assert_eq!(
        count(&after_iterator, "#size()I"),
        before_iterator_size,
        "read-only iterator call tainted later size: {after_iterator:?}"
    );
    for method in [
        "iterationArray",
        "iterationHash",
        "iterationLinked",
        "cmePutArray",
        "cmePutHash",
        "cmePutLinked",
        "cmeReplaceArray",
        "cmeReplaceHash",
        "cmeReplaceLinked",
        "removeIteratorArray",
        "removeIteratorHash",
        "removeIteratorLinked",
        "puzzleLoop",
    ] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            oracle.next().unwrap(),
            "{method}"
        );
    }
}

/// Removing the first of two values through a wrapped iterator leaves the
/// second one next. Java 25 + Guava 33.6.0 print `1|true|2|1` for every kind
/// (`ArrayList$Itr.remove` sets `cursor = lastRet`; hash iterators advance).
#[test]
fn wrapped_iterator_remove_keeps_the_next_value() {
    let Some(guava) = guava_jar() else {
        eprintln!("GUAVA_JAR is unavailable; real-Guava fixture is arcweld-only");
        return;
    };
    let fixture = Fixture::build_with_classpath(
        "guava-multimap-remove",
        &[("demo/Probe.java", PROBE)],
        &guava,
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    for method in ["removeFirstArray", "removeFirstHash", "removeFirstLinked"] {
        assert_eq!(fixture.text_on(&mut host, method), "1|true|2|1", "{method}");
    }
}

/// The wrapped iterator walks its creation-time snapshot, so removing through
/// it never skips or repeats an element and an exhausted walk throws.
/// Java 25 + Guava 33.6.0: remove-all `123|NSEE|0|false`, remove-last
/// `false|NSEE|1`, for every kind.
#[test]
fn wrapped_iterator_remove_walks_to_exhaustion() {
    let Some(guava) = guava_jar() else {
        eprintln!("GUAVA_JAR is unavailable; real-Guava fixture is arcweld-only");
        return;
    };
    let fixture = Fixture::build_with_classpath(
        "guava-multimap-exhaust",
        &[("demo/Probe.java", PROBE)],
        &guava,
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    for method in ["removeAllArray", "removeAllHash", "removeAllLinked"] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            "123|NSEE|0|false",
            "{method}"
        );
    }
    for method in ["removeLastArray", "removeLastHash", "removeLastLinked"] {
        assert_eq!(
            fixture.text_on(&mut host, method),
            "false|NSEE|1",
            "{method}"
        );
    }
}

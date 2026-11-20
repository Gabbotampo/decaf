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
        13,
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
        "#iterator()Ljava/util/Iterator;",
        "#listIterator()Ljava/util/ListIterator;",
        "#subList(II)Ljava/util/List;",
        "#toString()Ljava/lang/String;",
        "#contains(Ljava/lang/Object;)Z",
        "#remove(Ljava/lang/Object;)Z",
        "#size()I",
    ] {
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(name, _)| name.contains(call)),
            "reviewer case was not counted: {call}; calls: {:?}",
            host.unknown_host_calls()
        );
    }
}

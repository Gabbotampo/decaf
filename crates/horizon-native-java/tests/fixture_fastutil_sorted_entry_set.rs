//! fastutil 8.5.18 `Object2BooleanLinkedOpenHashMap.object2BooleanEntrySet().iterator()`, compiled
//! against the real jar. `java25.txt` and `java25_counted.txt` are the raw output of `demo.Probe`
//! on JDK 25 with that jar on the class path (commands in the fixture README), never the native
//! implementation's.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use std::path::PathBuf;

const ORACLE: &str = include_str!("fixtures/fastutil_sorted_entry_set/java25.txt");
const COUNTED_ORACLE: &str = include_str!("fixtures/fastutil_sorted_entry_set/java25_counted.txt");
const PROBE: &str = include_str!("fixtures/fastutil_sorted_entry_set/demo/Probe.java");

fn fastutil_jar() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("FASTUTIL_JAR") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("ava/fastutil-8.5.18.jar"));
        candidates.push(home.join(
            ".local/share/PrismLauncher/libraries/it/unimi/dsi/fastutil/8.5.18/fastutil-8.5.18.jar",
        ));
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn build() -> Option<javac_fixture::Fixture> {
    let Some(jar) = fastutil_jar() else {
        eprintln!("FASTUTIL_JAR is unavailable; real-fastutil fixture needs fastutil-8.5.18.jar");
        return None;
    };
    Some(javac_fixture::Fixture::build_with_classpath(
        "fastutil_sorted_entry_set",
        &[("demo/Probe.java", PROBE)],
        &jar,
    ))
}

#[test]
fn fastutil_sorted_entry_set_matches_java25() {
    let Some(fixture) = build() else {
        return;
    };
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let text = fixture.text_on(&mut host, "rows");
    for (got, want) in text.lines().zip(ORACLE.lines()) {
        assert_eq!(got, want);
    }
    assert_eq!(text, ORACLE);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// What stays counted: `fastIterator()` (one reused entry), a map changed under a live iterator,
/// an entry read after the iterator's `remove`, and the tree maps (natural key order). Each is
/// counted, never answered with a value the oracle does not show.
#[test]
fn fastutil_sorted_entry_set_leaves_the_rest_counted() {
    let Some(fixture) = build() else {
        return;
    };
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let text = fixture.text_on(&mut host, "countedRows");
    // Every row of the oracle ran (a row's value is the counted call's neutral answer).
    let names = |text: &str| -> Vec<String> {
        text.lines()
            .map(|line| line.split('\t').next().unwrap_or_default().to_owned())
            .collect()
    };
    assert_eq!(names(&text), names(COUNTED_ORACLE));
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    let objects = "it/unimi/dsi/fastutil/objects/";
    for expected in [
        // The tree maps' natural key order.
        format!(
            "{objects}Object2BooleanAVLTreeMap#object2BooleanEntrySet()L{objects}ObjectSortedSet;"
        ),
        format!(
            "{objects}Object2BooleanRBTreeMap#object2BooleanEntrySet()L{objects}ObjectSortedSet;"
        ),
        format!(
            "{objects}Object2BooleanSortedMap#object2BooleanEntrySet()L{objects}ObjectSortedSet;"
        ),
        // One entry object reused by every `next()`.
        format!(
            "{objects}Object2BooleanSortedMap$FastSortedEntrySet#fastIterator()L{objects}ObjectBidirectionalIterator;"
        ),
        // A map changed under the live iterator, and an entry read after `remove()`.
        "java/util/Iterator#hasNext()Z".to_owned(),
        "java/util/Iterator#next()Ljava/lang/Object;".to_owned(),
        "java/util/Map$Entry#getKey()Ljava/lang/Object;".to_owned(),
    ] {
        assert!(
            counted.contains(&expected),
            "{expected} not counted: {counted:?}"
        );
    }
}

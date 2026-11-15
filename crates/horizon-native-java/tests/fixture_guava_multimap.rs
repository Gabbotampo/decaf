//! Guava 33.6.0 multimap fixture compiled against the real jar on arcweld.
//! Expected strings below come from Temurin Java 25.0.5 with `javac --release 21`
//! and `guava-33.6.0-jre.jar`, never from the native implementation.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use javac_fixture::Fixture;
use std::path::PathBuf;

const PROBE: &str = include_str!("fixtures/guava_multimap/Probe.java");

fn guava_jar() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("GUAVA_JAR") {
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
    for (method, expected) in [
        (
            "array",
            "{a=[1, 3, 4], b=[2, 5, 6]}|true|true|6|[a, b]|[1, 3, 4, 2, 5, 6]|[a=1, a=3, a=4, b=2, b=5, b=6]|{a=[1, 3, 4], b=[2, 5, 6]}|true|true|true|[2, 5, 6]|{a=[1, 4]}",
        ),
        (
            "linked",
            "{a=[1, 3, 4], b=[2, 5, 6]}|true|true|6|[a, b]|[1, 2, 3, 4, 5, 6]|[a=1, b=2, a=3, a=4, b=5, b=6]|{a=[1, 3, 4], b=[2, 5, 6]}|true|true|true|[2, 5, 6]|{a=[1, 4]}",
        ),
        (
            "sync",
            "{a=[1, 3, 4], b=[2, 5, 6]}|true|true|6|[a, b]|[1, 3, 4, 2, 5, 6]|[a=1, a=3, a=4, b=2, b=5, b=6]|{a=[1, 3, 4], b=[2, 5, 6]}|true|true|true|[2, 5, 6]|{a=[1, 4]}",
        ),
    ] {
        assert_eq!(fixture.text_on(&mut host, method), expected, "{method}");
    }
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

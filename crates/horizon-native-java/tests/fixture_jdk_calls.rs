#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use javac_fixture::Fixture;
use std::path::PathBuf;

const PROBE: &str = include_str!("fixtures/jdk_calls/demo/Probe.java");
const LIB: &str = include_str!("fixtures/jdk_calls/lib/Calls.java");
const ORACLE: &str = include_str!("fixtures/jdk_calls/java25.txt");

fn guava_jar() -> Option<PathBuf> {
    std::env::var_os("GUAVA_26_JAR")
        .or_else(|| std::env::var_os("GUAVA_JAR"))
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            let path = PathBuf::from(
                "/home/solenopsisbot/coding/hj-scratch/mc263-server/META-INF/libraries/com/google/guava/guava/33.6.0-jre/guava-33.6.0-jre.jar",
            );
            path.is_file().then_some(path)
        })
}

#[test]
fn jdk_and_guava_calls_match_java25() {
    let Some(guava) = guava_jar() else {
        eprintln!("Guava is unavailable; fixture skipped");
        return;
    };
    let fixture = Fixture::build_with_lib_classpath(
        "jdk-calls",
        &[("demo/Probe.java", PROBE), ("lib/Calls.java", LIB)],
        &guava,
    );
    for method in [
        "enumSet",
        "streamArray",
        "enumValuesStream",
        "arrayMap",
        "hashMap",
        "linkedMap",
    ] {
        let expected = ORACLE
            .lines()
            .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
            .unwrap();
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        host.link_jar("calls", fixture.lib.as_ref().unwrap())
            .unwrap();
        // Keep Guava on the linked class path too: real packs may execute the
        // library's bytecode, while the runtime must still dispatch its
        // modelled collection contracts from a library caller.
        let guava_link = horizon_native_java::NativeJar::from_path(&guava).unwrap();
        host.link_jar("guava", &guava_link).unwrap();
        assert_eq!(fixture.text_on(&mut host, method), expected, "{method}");
        assert!(
            host.unknown_host_calls().is_empty(),
            "{method}: {:?}",
            host.unknown_host_calls()
        );
    }
}

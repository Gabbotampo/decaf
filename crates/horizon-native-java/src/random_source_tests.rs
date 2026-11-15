//! Java 25 oracle for explicit seeded Minecraft RandomSource values.

use crate::{NativeJar, StdHost, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            java_files(&path, out)
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path)
        }
    }
}

fn jar() -> &'static NativeJar {
    static JAR: OnceLock<NativeJar> = OnceLock::new();
    JAR.get_or_init(|| {
        use std::io::Write;
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures = [
            manifest.join("tests/fixtures/random_sources"),
            manifest.join("../crates/horizon-native-java/tests/fixtures/random_sources"),
        ]
        .into_iter()
        .find(|dir| dir.join("demo/RandomSources.java").is_file())
        .expect("random source fixture directory");
        let scratch =
            std::env::temp_dir().join(format!("horizon-random-sources-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("stubs"), &mut sources);
        java_files(&fixtures.join("demo"), &mut sources);
        let javac = std::env::var_os("JAVA_HOME")
            .map(|h| PathBuf::from(h).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        assert!(
            Command::new(javac)
                .args(["--release", "21", "-nowarn", "-d"])
                .arg(&classes)
                .args(&sources)
                .status()
                .unwrap()
                .success()
        );
        let mut bytes = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
        let opts = zip::write::SimpleFileOptions::default();
        let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("demo"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        for path in files {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            zip.start_file(format!("demo/{name}"), opts).unwrap();
            zip.write_all(&std::fs::read(path).unwrap()).unwrap();
        }
        zip.finish().unwrap();
        let _ = std::fs::remove_dir_all(&scratch);
        NativeJar::from_bytes(&bytes).unwrap()
    })
}

#[test]
fn seeded_sources_match_java_25_over_large_draws() {
    let mut host = StdHost::new();
    let answer = jar().invoke_method_with_args(
        "demo/RandomSources",
        "all",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let expected = "-7456144223115433903|-7456144223115433903|4544695051674328014|2382620503608921707|2673347604655833840|-7200039346697818759|-7086810981241049760|-6101034867912175575|done|-3478802880076970620|-7456144223115433903|-5247859587829850673|-6872393971165896881|-6945938047419056854|-2794455449125661773|-6872393971165896881";
    assert!(
        matches!(answer,Ok(Value::Ref(Some(ref text))) if text==expected),
        "{answer:?}"
    );
    assert_eq!(
        host.unknown_host_calls(),
        vec![(
            "net/minecraft/util/RandomSource#nextGaussian()D".into(),
            100003,
        )]
    );
}

#[test]
fn seeded_bound_edges_keep_java_exceptions() {
    let mut host = StdHost::new();
    let answer = jar().invoke_method_with_args(
        "demo/RandomSources",
        "errors",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(
        matches!(answer,Ok(Value::Ref(Some(ref text))) if text=="java.lang.IllegalArgumentException|Bound must be positive;java.lang.IllegalArgumentException|Bound must be positive;java.lang.IllegalArgumentException|Bound must be positive;java.lang.IllegalArgumentException|Bound must be positive"),
        "{answer:?}"
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn quick_seeded_factory_smoke() {
    let mut host = StdHost::new();
    let answer = jar().invoke_method_with_args(
        "demo/RandomSources",
        "quick",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(
        matches!(answer, Ok(Value::Ref(Some(ref text))) if text == "net.minecraft.world.level.levelgen.LegacyRandomSource|-1155869325"),
        "{answer:?} unknown={:?}",
        host.unknown_host_calls()
    );
}

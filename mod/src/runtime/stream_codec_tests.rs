//! Java 25 oracle for pure network codec construction.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::runtime::{NativeJar, StdHost, Value};

fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            java_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            out.push(path);
        }
    }
}

fn jar() -> &'static NativeJar {
    static JAR: OnceLock<NativeJar> = OnceLock::new();
    JAR.get_or_init(|| {
        use std::io::Write;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures = [
            manifest.join("tests/fixtures/stream_codecs"),
            manifest.join("../crates/horizon-native-java/tests/fixtures/stream_codecs"),
        ]
        .into_iter()
        .find(|dir| dir.join("demo/StreamCodecs.java").is_file())
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/stream_codecs");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-stream-codecs-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();

        let mut sources = Vec::new();
        java_files(&fixtures.join("stubs"), &mut sources);
        java_files(&fixtures.join("demo"), &mut sources);
        let javac = std::env::var_os("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let status = Command::new(javac)
            .args(["--release", "21", "-nowarn", "-d"])
            .arg(&classes)
            .args(&sources)
            .status()
            .expect("javac runs");
        assert!(status.success(), "stream-codecs fixture javac failed");

        let mut bytes = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default();
        let mut class_files: Vec<PathBuf> = std::fs::read_dir(classes.join("demo"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        class_files.sort();
        for path in class_files {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            zip.start_file(format!("demo/{name}"), options).unwrap();
            zip.write_all(&std::fs::read(path).unwrap()).unwrap();
        }
        zip.finish().unwrap();
        let _ = std::fs::remove_dir_all(&scratch);
        NativeJar::from_bytes(&bytes).unwrap()
    })
}

#[test]
fn constructions_match_java_25_classes_and_identity() {
    let mut host = StdHost::new();
    let answer = jar().invoke_method_with_args(
        "demo/StreamCodecs",
        "constructions",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(
        matches!(answer, Ok(Value::Ref(Some(ref text))) if text == "net.minecraft.network.codec.StreamCodec$7|true|true|net.minecraft.network.codec.StreamCodec$8|true|true|net.minecraft.network.codec.StreamCodec$2|true|true|net.minecraft.network.protocol.common.custom.CustomPacketPayload$1|true|true|true|false|true|false"),
        "{answer:?}"
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn network_methods_remain_counted() {
    let mut host = StdHost::new();
    let answer = jar().invoke_method_with_args(
        "demo/StreamCodecs",
        "encodeDecode",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(
        matches!(answer, Ok(Value::Ref(Some(ref text))) if text == "null"),
        "{answer:?}"
    );
    let unknown = host.unknown_host_calls();
    assert!(
        unknown.iter().any(|(call, _)| call.contains("#encode")),
        "{unknown:?}"
    );
    assert!(
        unknown.iter().any(|(call, _)| call.contains("#decode")),
        "{unknown:?}"
    );
}

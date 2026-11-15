//! Java 25 fixture compiled against the real Gson 2.14.0 jar.

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::write::SimpleFileOptions;

const PROBE: &str = r#"package demo;
import com.google.gson.*;
import com.google.gson.annotations.*;
import java.util.*;

public final class Probe {
  static class Base {
    int base = 1;
    transient int transientValue = 2;
    static int staticValue = 3;
    @SerializedName("renamed") String label = "x";
    @Expose String exposed = "e";
  }
  static class Child extends Base {
    Integer boxed = 2;
    String nullable = null;
    List<String> list = List.of("a", "b");
    Map<String, Integer> map = Map.of("k", 3);
    int[] array = {4, 5};
    Kind kind = Kind.B;
    String someValue = "z";
  }
  enum Kind { A, B }

  public static String plain() {
    return new Gson().toJsonTree(new Child()).toString();
  }
  public static String nulls() {
    return new GsonBuilder().serializeNulls().create().toJsonTree(new Child()).toString();
  }
  public static String naming() {
    return new GsonBuilder().setFieldNamingPolicy(FieldNamingPolicy.LOWER_CASE_WITH_UNDERSCORES)
        .create().toJsonTree(new Child()).toString();
  }
  public static String counted() {
    Gson gson = new GsonBuilder().registerTypeAdapter(Child.class,
        (JsonSerializer<Child>) (value, type, context) -> new JsonPrimitive("custom")).create();
    return String.valueOf(gson.toJsonTree(new Child()));
  }
}
"#;

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}
fn gson_jar() -> PathBuf {
    std::env::var_os("HORIZON_GSON_JAR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/home/infer/ava/mc-local/minecraft/26.3/libraries/client/com/google/code/gson/gson/2.14.0/gson-2.14.0.jar"))
}
fn files_under(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

#[test]
fn gson_to_json_tree_matches_real_gson() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "horizon-gson-to-json-tree-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let classes = root.join("classes");
    let src = root.join("Probe.java");
    fs::create_dir_all(&classes).unwrap();
    fs::write(&src, PROBE).unwrap();
    let gson = gson_jar();
    assert!(gson.is_file(), "missing real Gson jar: {}", gson.display());
    let status = Command::new(javac())
        .args(["--release", "25", "-cp"])
        .arg(&gson)
        .args(["-d"])
        .arg(&classes)
        .arg(&src)
        .status()
        .unwrap();
    assert!(status.success());
    let jar_path = root.join("fixture.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        br#"{"id":"gson-to-json-tree","version":"1.0","entrypoints":{"main":"demo.Probe"}}"#,
    )
    .unwrap();
    for class in files_under(&classes) {
        let entry = class
            .strip_prefix(&classes)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(entry, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    let jar = NativeJar::from_path(&jar_path).unwrap();
    for (method, expected) in [
        (
            "plain",
            r#"{"boxed":2,"list":["a","b"],"map":{"k":3},"array":[4,5],"kind":"B","someValue":"z","base":1,"renamed":"x","exposed":"e"}"#,
        ),
        (
            "nulls",
            r#"{"boxed":2,"nullable":null,"list":["a","b"],"map":{"k":3},"array":[4,5],"kind":"B","someValue":"z","base":1,"renamed":"x","exposed":"e"}"#,
        ),
        (
            "naming",
            r#"{"boxed":2,"list":["a","b"],"map":{"k":3},"array":[4,5],"kind":"B","some_value":"z","base":1,"renamed":"x","exposed":"e"}"#,
        ),
    ] {
        let mut host = jar.seed_host(StdHost::new());
        let value = jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap();
        assert_eq!(
            value,
            Value::Ref(Some(expected.into())),
            "{method}: {:?}",
            host.unknown_host_calls()
        );
        assert!(host.unknown_host_calls().is_empty());
    }
    let mut host = jar.seed_host(StdHost::new());
    let value = jar
        .invoke_method_with_args(
            "demo/Probe",
            "counted",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(value, Value::Ref(Some("null".into())));
    assert_eq!(host.unknown_host_calls().len(), 1);
    let _ = fs::remove_dir_all(root);
}

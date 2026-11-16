//! Reflective Gson deserialization against the shapes config jars use.

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::write::SimpleFileOptions;

const GSON: &str = r#"package com.google.gson;
public class Gson {
    public Gson() {}
    public <T> T fromJson(String json, Class<T> type) { return null; }
    public <T> T fromJson(java.io.Reader reader, Class<T> type) { return null; }
    public <T> T fromJson(String json, java.lang.reflect.Type type) { return null; }
    public <T> T fromJson(String json, com.google.gson.reflect.TypeToken<T> type) { return null; }
    public <T> T fromJson(JsonElement json, Class<T> type) { return null; }
    public <T> T fromJson(JsonElement json, java.lang.reflect.Type type) { return null; }
}
"#;
const JSON_ELEMENT: &str = r#"package com.google.gson;
public class JsonElement {}
"#;
const PARSER: &str = r#"package com.google.gson;
public final class JsonParser {
    public static JsonElement parseString(String json) { return null; }
}
"#;
const BUILDER: &str = r#"package com.google.gson;
public class GsonBuilder {
    public GsonBuilder() {}
    public Gson create() { return new Gson(); }
}
"#;
const SERIALIZED: &str = r#"package com.google.gson.annotations;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.FIELD)
public @interface SerializedName { String value(); String[] alternate() default {}; }
"#;
const TOKEN: &str = r#"package com.google.gson.reflect;
public class TypeToken<T> {
    protected TypeToken() {}
}
"#;
const PROBE: &str = r#"package demo;
import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.annotations.SerializedName;
import java.util.List;
import java.util.Map;

public final class Probe {
    public static class Nested { public String label; }
    public enum Mode { FIRST, @SerializedName("wire-second") SECOND }
    public static class Config {
        public String name;
        public int size;
        public boolean enabled;
        public List<Nested> items;
        public Map<String, Integer> nums;
        public int[] arr;
        public Mode mode;
        @SerializedName(value="wire_name", alternate={"old_name"}) public String alias;
        public String ctorDefault = "field";
        public Config() { ctorDefault = "constructor"; }
    }
    public static class UnsafeConfig {
        public String value;
        private UnsafeConfig(String ignored) { value = "constructor"; }
    }
    public abstract static class Abs { public int x; }
    public interface Intf {}
    public record Rec(int x, String s) { public Rec { s = s + "|ctor"; } }
    public static final class NestedToken extends com.google.gson.reflect.TypeToken<List<Nested>> {
        public NestedToken() { super(); }
    }
    public static String config() {
        Config c = new GsonBuilder().create().fromJson(
            "{\"name\":123,\"size\":\"7\",\"enabled\":true,\"items\":[{\"label\":\"inner\"}],\"nums\":{\"n\":2},\"arr\":[3,4],\"mode\":\"wire-second\",\"old_name\":\"alias\",\"unknown\":9}",
            Config.class);
        return c.name+"|"+c.size+"|"+c.enabled+"|"+c.items.get(0).label+"|"+c.nums.get("n")+"|"+c.arr[1]+"|"+c.mode.name()+"|"+c.alias+"|"+c.ctorDefault;
    }
    public static String defaults() {
        Config c = new Gson().fromJson("{\"unknown\":1}", Config.class);
        return String.valueOf(c.name)+"|"+c.size+"|"+c.enabled+"|"+c.ctorDefault;
    }
    public static String booleanFieldBoxing() {
        Config c = new Gson().fromJson("{\"enabled\":true}", Config.class);
        return Boolean.valueOf(c.enabled).toString();
    }
    public static String unsafe() {
        UnsafeConfig c = new Gson().fromJson("{\"value\":\"json\"}", UnsafeConfig.class);
        return c.value;
    }
    public static String token() {
        List<Nested> values = new Gson().fromJson("[{\"label\":\"token\"}]", new NestedToken());
        return values.get(0).label;
    }
    public static String element() {
        JsonElement element = com.google.gson.JsonParser.parseString("{\"name\":\"element\"}");
        return new Gson().fromJson(element, Config.class).name;
    }
    public static String abstractCase() {
        try { new Gson().fromJson("{\"x\":1}", Abs.class); return "object"; }
        catch (Throwable t) { return t.getClass().getName()+"|"+t.getMessage(); }
    }
    public static String interfaceCase() {
        try { new Gson().fromJson("{}", Intf.class); return "object"; }
        catch (Throwable t) { return t.getClass().getName()+"|"+t.getMessage(); }
    }
    public static String recordCase() {
        Rec value = new Gson().fromJson("{\"x\":4,\"s\":\"ok\"}", Rec.class);
        return value.x()+"|"+value.s();
    }
}
"#;

const ORACLE: &str = include_str!("fixtures/gson_fromjson/oracle/java25.txt");
const ORACLE_METHODS: &[&str] = &[
    "config",
    "defaults",
    "booleanFieldBoxing",
    "unsafe",
    "token",
    "element",
    "abstractCase",
    "interfaceCase",
    "recordCase",
];

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
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
struct Fixture {
    root: PathBuf,
    jar: NativeJar,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn build(test: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "horizon-native-java-gson-fromjson-{test}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let classes = root.join("classes");
        let sources = [
            ("com/google/gson/Gson.java", GSON),
            ("com/google/gson/JsonElement.java", JSON_ELEMENT),
            ("com/google/gson/JsonParser.java", PARSER),
            ("com/google/gson/GsonBuilder.java", BUILDER),
            (
                "com/google/gson/annotations/SerializedName.java",
                SERIALIZED,
            ),
            ("com/google/gson/reflect/TypeToken.java", TOKEN),
            ("demo/Probe.java", PROBE),
        ];
        let mut javac = Command::new(javac());
        javac.args(["--release", "17", "-d"]).arg(&classes);
        for (path, source) in sources {
            let file = root.join("src").join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, source).unwrap();
            javac.arg(file);
        }
        assert!(javac.status().unwrap().success());
        let jar_path = root.join("gson-fromjson.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"id":"gson-fromjson","version":"1.0","entrypoints":{"main":"demo.Probe"}}"#,
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
        Self {
            root,
            jar: NativeJar::from_path(&jar_path).unwrap(),
        }
    }
    fn text(&self, method: &str) -> String {
        match self
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut StdHost::new(),
            )
            .unwrap()
        {
            Value::Ref(Some(text)) => text,
            other => panic!("{method}: {other:?}"),
        }
    }
}
#[test]
fn all_rows_match_the_captured_java25_oracle_byte_for_byte() {
    let fixture = Fixture::build("oracle");
    let actual = ORACLE_METHODS
        .iter()
        .map(|method| format!("{method}={}\n", fixture.text(method)))
        .collect::<String>();
    assert_eq!(actual.as_bytes(), ORACLE.as_bytes());
}

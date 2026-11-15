//! Gson 2.14 writer-slice parity fixture, compiled with Java 25's javac.
//! Expected writer text was captured from the real Gson 2.14.0 jar on Java 25.

use horizon_native_java::{NativeError, NativeJar, StdHost, Value};
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
  public String toJson(Object src) { return null; }
  public void toJson(Object src, Appendable out) {}
  public void toJson(JsonElement src, Appendable out) {}
  public void toJson(JsonElement src, com.google.gson.stream.JsonWriter out) {}
  public com.google.gson.stream.JsonWriter newJsonWriter(java.io.Writer out) { return null; }
  public <T> T fromJson(java.io.Reader in, Class<T> type) { return null; }
}
"#;
const BUILDER: &str = r#"package com.google.gson;
public class GsonBuilder {
  public GsonBuilder() {}
  public GsonBuilder setPrettyPrinting() { return this; }
  public GsonBuilder serializeNulls() { return this; }
  public GsonBuilder disableHtmlEscaping() { return this; }
  public Gson create() { return new Gson(); }
}
"#;
const ELEMENT: &str = "package com.google.gson; public class JsonElement {}\n";
const OBJECT: &str = r#"package com.google.gson;
public final class JsonObject extends JsonElement {
  public JsonObject() {}
  public void addProperty(String key, String value) {}
  public void addProperty(String key, Number value) {}
  public void addProperty(String key, Boolean value) {}
  public void addProperty(String key, Character value) {}
}
"#;
const IOEX: &str = r#"package com.google.gson;
public class JsonIOException extends RuntimeException { public JsonIOException(String s) { super(s); } }
"#;
const WRITER: &str = r#"package com.google.gson.stream;
public class JsonWriter { public JsonWriter(java.io.Writer out) {} public void flush() {} }
"#;
const PROBE: &str = r#"package demo;
import com.google.gson.*; import java.io.*;
public final class Probe {
  public static String writers() throws Exception {
    Gson gson = new GsonBuilder().setPrettyPrinting().serializeNulls().disableHtmlEscaping().create();
    JsonObject o = new JsonObject();
    o.addProperty("text", "<&>"); o.addProperty("bool", Boolean.TRUE);
    o.addProperty("num", (Number) 2.5); o.addProperty("char", Character.valueOf((char)120));
    o.addProperty("nil", (String) null);
    StringBuilder b = new StringBuilder(); gson.toJson(o, b);
    StringWriter s = new StringWriter(); gson.toJson(o, s);
    File file = new File("gson.json");
    FileWriter fw = new FileWriter(file); gson.toJson(o, fw); fw.close();
    JsonObject fromFile = gson.fromJson(new FileReader(file), JsonObject.class);
    StringWriter jtext = new StringWriter();
    com.google.gson.stream.JsonWriter jw = gson.newJsonWriter(jtext);
    gson.toJson((JsonElement)o, jw); jw.flush();
    String compact = new Gson().toJson(o);
    String omitted = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create().toJson(o);
    String failure;
    try {
      gson.toJson(o, new Appendable() {
        public Appendable append(CharSequence x) throws IOException { throw new IOException("boom"); }
        public Appendable append(CharSequence x, int a, int z) throws IOException { throw new IOException("boom"); }
        public Appendable append(char x) throws IOException { throw new IOException("boom"); }
      }); failure = "none";
    } catch (JsonIOException e) { failure = e.getClass().getName(); }
    return b.toString().replace("\n", "\\n") + "|S=" + s.toString().replace("\n", "\\n")
      + "|W=" + gson.toJson(fromFile).replace("\n", "\\n")
      + "|J=" + jtext.toString().replace("\n", "\\n") + "|C=" + compact + "|O="
      + omitted.replace("\n", "\\n") + "|F=" + failure;
  }
  public static String nullKey() { try { new JsonObject().addProperty((String)null, "x"); return "no"; } catch (NullPointerException e) { return "npe"; } }
}
"#;

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|h| PathBuf::from(h).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}
fn files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() { files(&p) } else { vec![p] }
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
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "horizon-gson-writer-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let classes = root.join("classes");
        let srcs = [
            ("com/google/gson/Gson.java", GSON),
            ("com/google/gson/GsonBuilder.java", BUILDER),
            ("com/google/gson/JsonElement.java", ELEMENT),
            ("com/google/gson/JsonObject.java", OBJECT),
            ("com/google/gson/JsonIOException.java", IOEX),
            ("com/google/gson/stream/JsonWriter.java", WRITER),
            ("demo/Probe.java", PROBE),
        ];
        let mut cmd = Command::new(javac());
        cmd.args(["--release", "21", "-d"]).arg(&classes);
        for (name, text) in srcs {
            let p = root.join("src").join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
            cmd.arg(p);
        }
        assert!(cmd.status().unwrap().success());
        let jar_path = root.join("fixture.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"id":"gson-writer-fixture","version":"1","entrypoints":{"main":"demo.Probe"}}"#,
        )
        .unwrap();
        for p in files(&classes) {
            let n = p
                .strip_prefix(&classes)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(n, SimpleFileOptions::default()).unwrap();
            zip.write_all(&fs::read(p).unwrap()).unwrap();
        }
        zip.finish().unwrap();
        Self {
            root,
            jar: NativeJar::from_path(&jar_path).unwrap(),
        }
    }
    fn call(&self, name: &str) -> Result<Value, NativeError> {
        self.jar.invoke_method_with_args(
            "demo/Probe",
            name,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut StdHost::new(),
        )
    }
    fn text(&self, name: &str) -> String {
        match self.call(name).unwrap() {
            Value::Ref(Some(s)) => s,
            v => panic!("{name}: {v:?}"),
        }
    }
}

#[test]
fn writers_options_tree_properties_and_ioexception_match_real_gson() {
    let f = Fixture::new();
    assert_eq!(
        f.text("writers"),
        r#"{\n  "text": "<&>",\n  "bool": true,\n  "num": 2.5,\n  "char": "x",\n  "nil": null\n}|S={\n  "text": "<&>",\n  "bool": true,\n  "num": 2.5,\n  "char": "x",\n  "nil": null\n}|W={\n  "text": "<&>",\n  "bool": true,\n  "num": 2.5,\n  "char": "x",\n  "nil": null\n}|J={\n  "text": "<&>",\n  "bool": true,\n  "num": 2.5,\n  "char": "x",\n  "nil": null\n}|C={"text":"\u003c\u0026\u003e","bool":true,"num":2.5,"char":"x"}|O={\n  "text": "<&>",\n  "bool": true,\n  "num": 2.5,\n  "char": "x"\n}|F=com.google.gson.JsonIOException"#
    );
}

#[test]
fn json_object_null_property_name_is_an_npe() {
    let f = Fixture::new();
    assert_eq!(f.text("nullKey"), "npe");
}

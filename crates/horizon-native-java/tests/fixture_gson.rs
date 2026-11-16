//! Bounded `com.google.gson` fixtures compiled from real Java at test time.
//!
//! The stubs declare the signatures a config bootstrap compiles against; the
//! interpreter answers them from its own modelled surface (`invoke_gson`).
//! The fixture proves the two directions a real mod needs: JSON text becomes
//! modelled primitives, strings, lists and maps, and those values serialize
//! back. Malformed and over-deep documents are refused with a bounded reason
//! instead of answering an empty value.
//!
//! Java 25 oracle rows for the bit-conversion fixture (OpenJDK 25.0.5+8):
//! double non-NaN (raw/normal):
//! 0/0;8000000000000000/8000000000000000;7ff0000000000000/7ff0000000000000;fff0000000000000/fff0000000000000;3ff4000000000000/3ff4000000000000;1/1;8000000000000001/8000000000000001;
//! float non-NaN (raw/normal):
//! 0/0;80000000/80000000;7f800000/7f800000;ff800000/ff800000;3fa00000/3fa00000;1/1;80000001/80000001;
//! canonical NaN: double raw/normal=7ff8000000000000/7ff8000000000000;
//! float raw/normal=7fc00000/7fc00000.
//! The recorded oracle command was `javac --release 25 BitOracle.java && java BitOracle`;
//! raw NaN calls are deliberately COUNTED by the native model because its f64 Value
//! cannot preserve every Java NaN payload.

#[path = "support/scratch.rs"]
mod scratch;

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
    public void toJson(Object src, Appendable writer) {}
    public <T> T fromJson(String json, Class<T> type) { return null; }
    public <T> T fromJson(java.io.Reader reader, Class<T> type) { return null; }
}
"#;

const STRICTNESS: &str = r#"package com.google.gson;

public enum Strictness { LENIENT, LEGACY_STRICT, STRICT }
"#;

const GSON_BUILDER: &str = r#"package com.google.gson;

public class GsonBuilder {
    public GsonBuilder() {}
    public GsonBuilder setPrettyPrinting() { return this; }
    public GsonBuilder setLenient() { return this; }
    public GsonBuilder setStrictness(Strictness strictness) { return this; }
    public Gson create() { return new Gson(); }
}
"#;

const PROBE: &str = r#"package demo;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.Strictness;
import java.io.File;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.io.OutputStreamWriter;
import java.io.StringReader;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

public final class Probe {
    private static String scalar(Object value) {
        if (value == null) return "null";
        if (value instanceof Double number) return "D:" + scalarDouble(number);
        if (value instanceof Float number) return "F:" + scalarFloat(number);
        return value.toString();
    }

    private static String scalarDouble(double value) {
        return value + ":" + Long.toHexString(Double.doubleToRawLongBits(value));
    }

    private static String scalarFloat(float value) {
        return value + ":" + Integer.toHexString(Float.floatToRawIntBits(value));
    }

    public static String bitConversions() {
        double[] doubles = { 0.0, -0.0, Double.POSITIVE_INFINITY,
            Double.NEGATIVE_INFINITY, 1.25, Double.MIN_VALUE, -Double.MIN_VALUE };
        float[] floats = { 0.0f, -0.0f, Float.POSITIVE_INFINITY,
            Float.NEGATIVE_INFINITY, 1.25f, Float.MIN_VALUE, -Float.MIN_VALUE };
        StringBuilder out = new StringBuilder();
        for (double value : doubles) {
            out.append(Long.toHexString(Double.doubleToRawLongBits(value)))
                .append('/').append(Long.toHexString(Double.doubleToLongBits(value))).append(';');
        }
        for (float value : floats) {
            out.append(Integer.toHexString(Float.floatToRawIntBits(value)))
                .append('/').append(Integer.toHexString(Float.floatToIntBits(value))).append(';');
        }
        return out.toString();
    }

    public static String canonicalNaN() {
        return Long.toHexString(Double.doubleToLongBits(Double.NaN)) + "/"
            + Integer.toHexString(Float.floatToIntBits(Float.NaN));
    }

    public static String rawNaN() {
        return Long.toHexString(Double.doubleToRawLongBits(Double.NaN)) + "|"
            + Integer.toHexString(Float.floatToRawIntBits(Float.NaN));
    }

    public static String numeric(String mode, String json) {
        GsonBuilder builder = new GsonBuilder();
        if (mode.equals("lenient")) {
            builder.setLenient();
        } else if (mode.equals("strict")) {
            builder.setStrictness(Strictness.STRICT);
        }
        Gson gson = builder.create();
        Object object = null;
        String string = null;
        Double boxedDouble = null;
        Float boxedFloat = null;
        try { object = gson.fromJson(json, Object.class); } catch (RuntimeException ignored) {}
        try { string = gson.fromJson(json, String.class); } catch (RuntimeException ignored) {}
        try { boxedDouble = gson.fromJson(json, Double.class); } catch (RuntimeException ignored) {}
        try { boxedFloat = gson.fromJson(json, Float.class); } catch (RuntimeException ignored) {}
        return scalar(object) + "|" + scalar(string) + "|"
            + (boxedDouble == null ? "null" : scalarDouble(boxedDouble)) + "|"
            + (boxedFloat == null ? "null" : scalarFloat(boxedFloat));
    }

    public static final class ArmorConfig {}

    public static final class StaticOnlyConfig {
        public static int value = 7;
    }

    public static String armorConfig() {
        Gson gson = new GsonBuilder().create();
        return scalar(gson.fromJson(new StringReader("{}"), ArmorConfig.class));
    }

    public static String staticOnlyConfig() {
        Gson gson = new GsonBuilder().create();
        return scalar(gson.fromJson(new StringReader("{}"), StaticOnlyConfig.class));
    }

    public static String roundTrip() {
        Gson gson = new GsonBuilder().setPrettyPrinting().create();
        int number = gson.fromJson("42", Integer.class);
        String text = gson.fromJson("\"hello\"", String.class);
        List<?> items = gson.fromJson("[1,\"two\",[3]]", List.class);
        Map<?, ?> table = gson.fromJson("{\"a\":1,\"b\":\"x\"}", Map.class);
        List<?> reader = gson.fromJson(new StringReader("[7,8]"), List.class);
        StringBuilder sink = new StringBuilder();
        gson.toJson(items, sink);
        return number + "|" + text + "|" + items.size() + "|" + table.size() + "|"
            + reader.size() + "|" + gson.toJson(table) + "|" + sink.toString();
    }

    // Fabric reads its metadata with Gson's lenient parser, which accepts
    // `//` comments and a leading byte-order mark.
    public static String lenient(String json) {
        Gson gson = new GsonBuilder().create();
        Map<?, ?> values = gson.fromJson(json, Map.class);
        return String.valueOf(values.size());
    }

    public static String parse(String json) {
        Gson gson = new GsonBuilder().create();
        List<?> values = gson.fromJson(json, List.class);
        return String.valueOf(values.size());
    }

    // A config bootstrap that writes defaults through `FileWriter` and reads
    // them back through `FileReader`, exactly the shape of a JSON config file.
    public static String fileRoundTrip() {
        Gson gson = new GsonBuilder().create();
        Path path = Path.of("cfg.json");
        File file = path.toFile();
        String before = String.valueOf(file.exists());
        try {
            FileWriter writer = new FileWriter(file);
            gson.toJson(gson.fromJson("{\"a\":7}", Map.class), writer);
            writer.close();
        } catch (IOException error) {
            return "write-failed:" + error;
        }
        String after = String.valueOf(file.exists());
        try {
            FileReader reader = new FileReader(file);
            Map<?, ?> back = gson.fromJson(reader, Map.class);
            reader.close();
            return before + "|" + after + "|" + back.get("a");
        } catch (IOException error) {
            return "read-failed:" + error;
        }
    }

    // `OutputStreamWriter` over a `FileOutputStream` is the other writer a
    // config loader builds; the bytes must land in the same virtual file.
    public static String streamRoundTrip() {
        Gson gson = new GsonBuilder().create();
        Path path = Path.of("stream.json");
        try {
            OutputStreamWriter writer =
                new OutputStreamWriter(Files.newOutputStream(path), StandardCharsets.UTF_8);
            gson.toJson(gson.fromJson("{\"b\":2}", Map.class), writer);
            writer.close();
            java.io.BufferedReader reader = Files.newBufferedReader(path);
            Map<?, ?> back = gson.fromJson(reader, Map.class);
            reader.close();
            return String.valueOf(back.get("b"));
        } catch (IOException error) {
            return "failed:" + error;
        }
    }

    public static String stringWriter() {
        Gson gson = new GsonBuilder().create();
        StringWriter writer = new StringWriter();
        gson.toJson(gson.fromJson("[1,2]", List.class), writer);
        return writer.toString();
    }

    // A file the invocation never wrote raises the exception Java raises, and
    // a Java catch block sees it as an `IOException`.
    public static String missingFile() {
        try {
            FileReader reader = new FileReader(new File("absent.json"));
            reader.close();
            return "read";
        } catch (IOException error) {
            return "caught";
        }
    }
}
"#;

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
        let root = scratch::unique_path(&format!(
            "horizon-native-java-gson-{test}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let mut fixture = Self {
            root,
            jar: NativeJar::default(),
        };
        let classes = fixture.root.join("classes");
        let sources = [
            ("com/google/gson/Gson.java", GSON),
            ("com/google/gson/GsonBuilder.java", GSON_BUILDER),
            ("com/google/gson/Strictness.java", STRICTNESS),
            ("demo/Probe.java", PROBE),
        ];
        let mut javac = Command::new(javac());
        javac.args(["--release", "17", "-d"]).arg(&classes);
        for (path, source) in sources {
            let file = fixture.root.join("src").join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, source).unwrap();
            javac.arg(file);
        }
        let status = javac.status().unwrap();
        assert!(status.success(), "javac failed for gson fixture {test}");

        let jar_path = fixture.root.join("gson.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"id":"gson-fixture","version":"1.0","entrypoints":{"main":"demo.Probe"}}"#,
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

        fixture.jar = NativeJar::from_path(&jar_path).unwrap();
        fixture
    }

    fn call(&self, method: &str, descriptor: &str, args: &[Value]) -> Result<Value, NativeError> {
        let mut host = StdHost::new();
        self.call_with_host(method, descriptor, args, &mut host)
    }

    fn call_with_host(
        &self,
        method: &str,
        descriptor: &str,
        args: &[Value],
        host: &mut StdHost,
    ) -> Result<Value, NativeError> {
        self.jar
            .invoke_method_with_args("demo/Probe", method, descriptor, None, args, host)
    }

    fn text_with(&self, method: &str, argument: &str) -> String {
        match self
            .call(
                method,
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[Value::Ref(Some(argument.to_owned()))],
            )
            .unwrap()
        {
            Value::Ref(Some(text)) => text,
            other => panic!("{method} did not return a string: {other:?}"),
        }
    }

    fn text(&self, method: &str) -> String {
        match self.call(method, "()Ljava/lang/String;", &[]).unwrap() {
            Value::Ref(Some(text)) => text,
            other => panic!("{method} did not return a string: {other:?}"),
        }
    }
}

#[test]
fn bit_conversions_match_java25_for_non_nan_values() {
    let fixture = Fixture::build("bit-conversions");
    assert_eq!(
        fixture.text("bitConversions"),
        "0/0;8000000000000000/8000000000000000;7ff0000000000000/7ff0000000000000;fff0000000000000/fff0000000000000;3ff4000000000000/3ff4000000000000;1/1;8000000000000001/8000000000000001;0/0;80000000/80000000;7f800000/7f800000;ff800000/ff800000;3fa00000/3fa00000;1/1;80000001/80000001;"
    );
}

#[test]
fn canonical_nan_bit_conversions_match_java25() {
    let fixture = Fixture::build("canonical-nan");
    assert_eq!(fixture.text("canonicalNaN"), "7ff8000000000000/7fc00000");
}

#[test]
fn raw_nan_bit_conversions_are_counted() {
    let fixture = Fixture::build("raw-nan");
    let mut host = StdHost::new();
    let value = fixture
        .call_with_host("rawNaN", "()Ljava/lang/String;", &[], &mut host)
        .unwrap();
    assert_eq!(value, Value::Ref(Some("0|0".to_owned())));
    assert_eq!(
        host.unknown_host_calls(),
        vec![
            ("java/lang/Double#doubleToRawLongBits(D)J".into(), 1),
            ("java/lang/Float#floatToRawIntBits(F)I".into(), 1),
        ]
    );
}

#[test]
fn gson_round_trips_primitives_strings_lists_and_maps() {
    let fixture = Fixture::build("round-trip");
    // `roundTrip` builds a pretty-printing `Gson`; gson-2.14.0 on a real JVM
    // writes the two documents indented, so the expected text is that output.
    assert_eq!(
        fixture.text("roundTrip"),
        "42|hello|3|2|2|{\n  \"a\": 1.0,\n  \"b\": \"x\"\n}|[\n  1.0,\n  \"two\",\n  [\n    3.0\n  ]\n]"
    );
}

/// `Gson.fromJson` that Gson would refuse: a malformed document (Gson's
/// `JsonSyntaxException`, whose message carries the reader's position) and
/// one nested past `JsonReader`'s 255 levels. Neither is reproduced: the
/// call is counted.
#[test]
fn malformed_and_too_deep_json_are_counted() {
    let fixture = Fixture::build("malformed");
    for json in [
        "[".to_owned(),
        format!("{}{}", "[".repeat(256), "]".repeat(256)),
    ] {
        let mut host = StdHost::new();
        let _ = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            "parse",
            "(Ljava/lang/String;)Ljava/lang/String;",
            None,
            &[Value::Ref(Some(json))],
            &mut host,
        );
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(call, _)| call.starts_with("com/google/gson/Gson#fromJson")),
            "{:?}",
            host.unknown_host_calls()
        );
    }
}

#[test]
fn lenient_json_accepts_comments_and_a_byte_order_mark() {
    let fixture = Fixture::build("lenient");
    let document = "\u{feff}{\n // a comment\n \"a\": 1, /* inline */ \"b\": 2 }";
    assert_eq!(fixture.text_with("lenient", document), "2");
}

/// 40 nested arrays are well within `JsonReader`'s 255 levels: a raw `List`
/// holding one list, as Gson 2.14.0 reads it.
#[test]
fn nested_json_within_gsons_limit_is_read() {
    let fixture = Fixture::build("depth");
    let json = format!("{}{}", "[".repeat(40), "]".repeat(40));
    assert_eq!(fixture.text_with("parse", &json), "1");
}

/// A raw `Map`'s numbers are `ObjectTypeAdapter`'s `Double`s, so the value
/// read back prints `7.0` (Gson 2.14.0 writes `{"a":7.0}` and reads it back
/// as `7.0`).
#[test]
fn file_writer_and_file_reader_round_trip_through_the_virtual_file() {
    let fixture = Fixture::build("file-round-trip");
    assert_eq!(fixture.text("fileRoundTrip"), "false|true|7.0");
}

#[test]
fn output_stream_writer_and_files_reader_share_the_same_file() {
    let fixture = Fixture::build("stream-round-trip");
    assert_eq!(fixture.text("streamRoundTrip"), "2.0");
}

#[test]
fn string_writer_is_an_appendable_gson_can_write_to() {
    let fixture = Fixture::build("string-writer");
    assert_eq!(fixture.text("stringWriter"), "[1.0,2.0]");
}

#[test]
fn a_missing_file_raises_an_ioexception_a_java_catch_sees() {
    let fixture = Fixture::build("missing-file");
    assert_eq!(fixture.text("missingFile"), "caught");
}

#[test]
fn gson_numeric_edge_cases_match_java25() {
    let fixture = Fixture::build("numeric-edge-cases");
    // Invoke the descriptor directly so Java's two String arguments remain
    // unambiguous.
    let call = |mode: &str, json: &str| match fixture.call(
        "numeric",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[
            Value::Ref(Some(mode.to_owned())),
            Value::Ref(Some(json.to_owned())),
        ],
    ) {
        Ok(Value::Ref(Some(text))) => text,
        other => panic!("numeric returned {other:?}"),
    };
    for mode in ["default", "lenient"] {
        assert_eq!(
            call(mode, "-0"),
            "D:-0.0:8000000000000000|-0|-0.0:8000000000000000|-0.0:80000000"
        );
        assert_eq!(
            call(mode, "1e400"),
            "D:Infinity:7ff0000000000000|1e400|Infinity:7ff0000000000000|Infinity:7f800000"
        );
        assert_eq!(
            call(mode, "NaN"),
            // Raw NaN payloads are deliberately counted by the bounded host.
            "NaN|NaN|NaN:0|NaN:0"
        );
        assert_eq!(
            call(mode, "Infinity"),
            "Infinity|Infinity|Infinity:7ff0000000000000|Infinity:7f800000"
        );
        assert_eq!(
            call(mode, "-Infinity"),
            "-Infinity|-Infinity|-Infinity:fff0000000000000|-Infinity:ff800000"
        );
    }
    assert_eq!(
        call("strict", "-0"),
        "D:-0.0:8000000000000000|-0|-0.0:8000000000000000|-0.0:80000000"
    );
    assert_eq!(call("strict", "1e400"), "null|1e400|null|null");
    assert_eq!(call("strict", "NaN"), "null|null|null|null");
    assert_eq!(call("strict", "Infinity"), "null|null|null|null");
    assert_eq!(call("strict", "-Infinity"), "null|null|null|null");
}

#[test]
fn unsupported_jar_class_from_reader_is_counted() {
    let fixture = Fixture::build("unsupported-jar-class");
    let mut host = StdHost::new();
    let value = fixture
        .call_with_host("armorConfig", "()Ljava/lang/String;", &[], &mut host)
        .unwrap();
    assert_eq!(value, Value::Ref(Some("null".into())));
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call.starts_with("com/google/gson/Gson#fromJson"))
    );
}

#[test]
fn static_only_jar_class_from_reader_is_counted() {
    let fixture = Fixture::build("static-only-jar-class");
    let mut host = StdHost::new();
    let value = fixture
        .call_with_host("staticOnlyConfig", "()Ljava/lang/String;", &[], &mut host)
        .unwrap();
    assert_eq!(value, Value::Ref(Some("null".into())));
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call.starts_with("com/google/gson/Gson#fromJson"))
    );
}

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const PROBE: &str = include_str!("fixtures/path_starts_with/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/path_starts_with/java25.txt");
#[test]
fn path_starts_with() {
    let f = javac_fixture::Fixture::build("path-starts-with", &[("demo/Probe.java", PROBE)]);
    let mut h = f.jar.seed_host(horizon_native_java::StdHost::new());
    let v = f
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut h,
        )
        .unwrap();
    let s = match v {
        horizon_native_java::Value::Ref(Some(x)) => x,
        _ => panic!("{v:?}"),
    };
    assert_eq!(
        (s, h.unknown_host_calls()),
        (ORACLE.to_string(), Vec::new())
    );
}

// Cases the oracle does not show stay counted: a NUL in the text (`InvalidPathException`), text that
// is not ASCII (`sun.jnu.encoding`) and a null argument (`NullPointerException`).
const COUNTED: &str = r#"package demo;
import java.nio.file.*;
public final class Counted {
 public static boolean nul() { return Path.of("a/b").startsWith("a\0"); }
 public static boolean nonAscii() { return Path.of("a/b").startsWith("é"); }
 public static boolean nullPath() { return Path.of("a/b").startsWith((Path) null); }
 public static boolean nullString() { return Path.of("a/b").startsWith((String) null); }
}
"#;

#[test]
fn path_starts_with_stays_counted_outside_the_oracle() {
    let f = javac_fixture::Fixture::build(
        "path-starts-with-counted",
        &[("demo/Counted.java", COUNTED)],
    );
    for method in ["nul", "nonAscii", "nullPath", "nullString"] {
        let mut h = f.jar.seed_host(horizon_native_java::StdHost::new());
        let _ = f
            .jar
            .invoke_method_with_args("demo/Counted", method, "()Z", None, &[], &mut h);
        let calls = h.unknown_host_calls();
        assert!(
            calls
                .iter()
                .any(|(m, _)| m.starts_with("java/nio/file/Path#startsWith(")),
            "{method}: {calls:?}"
        );
    }
}

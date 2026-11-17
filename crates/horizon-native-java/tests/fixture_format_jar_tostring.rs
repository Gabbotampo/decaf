#[path = "support/scratch.rs"]
mod scratch;
use horizon_native_java::{NativeJar, StdHost, Value};
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;
const SOURCE: &str = include_str!("fixtures/format_jar_tostring/demo/Probe.java");
fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|h| PathBuf::from(h).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}
fn jar() -> NativeJar {
    let r = scratch::Scratch::new("format-jar-tostring");
    let src = r.join("Probe.java");
    fs::write(&src, SOURCE).unwrap();
    fs::create_dir_all(r.join("classes")).unwrap();
    assert!(
        Command::new(javac())
            .args(["-d"])
            .arg(r.join("classes"))
            .arg(&src)
            .status()
            .unwrap()
            .success()
    );
    let p = r.join("x.jar");
    let mut z = zip::ZipWriter::new(fs::File::create(&p).unwrap());
    for f in [
        "Probe.class",
        "Probe$Named.class",
        "Probe$Nully.class",
        "Probe$Boom.class",
        "Probe$Fmt.class",
    ] {
        z.start_file(format!("demo/{f}"), SimpleFileOptions::default())
            .unwrap();
        z.write_all(&fs::read(r.join("classes/demo").join(f)).unwrap())
            .unwrap();
    }
    z.finish().unwrap();
    NativeJar::from_path(&p).unwrap()
}
fn call(j: &NativeJar, h: &mut StdHost, n: &str) -> Value {
    j.invoke_method_with_args("demo.Probe", n, "()Ljava/lang/String;", None, &[], h)
        .unwrap()
}
#[test]
fn formatter_jar_tostring_capture() {
    let j = jar();
    let mut h = StdHost::new();
    let v = call(&j, &mut h, "rows");
    assert_eq!(
        v,
        Value::Ref(Some(
            include_str!("fixtures/format_jar_tostring/java25.txt").to_owned()
        ))
    );
    assert_eq!(h.unknown_host_calls(), vec![]);
}
#[test]
fn formatter_counted_cases() {
    for (name, sig) in [
        (
            "boom",
            "java/lang/String#formatted([Ljava/lang/Object;)Ljava/lang/String;",
        ),
        (
            "fmt",
            "java/lang/String#formatted([Ljava/lang/Object;)Ljava/lang/String;",
        ),
        (
            "mixed",
            "java/lang/String#format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;",
        ),
    ] {
        let j = jar();
        let mut h = StdHost::new();
        let v = call(&j, &mut h, name);
        if name == "mixed" {
            assert_eq!(
                v,
                Value::Ref(Some(
                    "java.util.IllegalFormatConversionException:d != demo.Probe$Named".into()
                ))
            );
        } else {
            assert_eq!(v, Value::Ref(None));
        }
        assert_eq!(h.unknown_host_calls(), vec![(sig.into(), 1)]);
    }
}

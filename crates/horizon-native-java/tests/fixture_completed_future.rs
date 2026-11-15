//! Java 25 fixture for the completed `CompletableFuture` slice.

use horizon_native_java::{NativeError, NativeJar, StdHost, Value};

const ORACLE: &str = include_str!("fixtures/completed_future/java25.txt");
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;

const SOURCE: &str = r#"
package demo;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.Future;

public final class CompletedFutureProbe {
  private static String seen;

  public static String chain() throws Exception {
    CompletableFuture<String> base = CompletableFuture.completedFuture("a");
    CompletableFuture<String> mapped = base.thenApply(value -> value + "b");
    CompletableFuture<String> composed = mapped.thenCompose(
        value -> CompletableFuture.completedFuture(value + "c"));
    CompletableFuture<Void> accepted = composed.thenAccept(value -> seen = value);
    return base.isDone() + "|" + mapped.get() + "|" + composed.join()
        + "|" + accepted.isDone() + "|" + seen;
  }

  public static String interfaces() throws Exception {
    CompletionStage<String> stage = CompletableFuture.completedFuture("x");
    Future<String> future = CompletableFuture.completedFuture("y");
    return stage.thenApply(value -> value + "1").toCompletableFuture().join()
        + "|" + future.get() + "|" + future.isDone();
  }

  public static String async() {
    return CompletableFuture.supplyAsync(() -> "never").join();
  }
}
"#;

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn fixture() -> NativeJar {
    let root = std::env::temp_dir().join(format!(
        "horizon-native-java-completed-future-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/demo")).unwrap();
    fs::create_dir_all(root.join("classes")).unwrap();
    let source = root.join("src/demo/CompletedFutureProbe.java");
    fs::write(&source, SOURCE).unwrap();
    let status = Command::new(javac())
        .args(["--release", "21", "-d"])
        .arg(root.join("classes"))
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed");
    let jar_path = root.join("completed-future.jar");
    let mut jar = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    jar.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    jar.write_all(
        br#"{"id":"completed-future-fixture","version":"1.0","entrypoints":{"main":"demo.CompletedFutureProbe"}}"#,
    )
    .unwrap();
    jar.start_file(
        "demo/CompletedFutureProbe.class",
        SimpleFileOptions::default(),
    )
    .unwrap();
    jar.write_all(&fs::read(root.join("classes/demo/CompletedFutureProbe.class")).unwrap())
        .unwrap();
    jar.finish().unwrap();
    let result = NativeJar::from_path(&jar_path).unwrap();
    let _ = fs::remove_dir_all(root);
    result
}

fn call(jar: &NativeJar, method: &str, host: &mut StdHost) -> Result<Value, NativeError> {
    jar.invoke_method_with_args(
        "demo/CompletedFutureProbe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        host,
    )
}

#[test]
fn completed_future_chain_matches_java25_oracle() {
    let jar = fixture();
    let mut host = StdHost::new().with_strict_unknown_host_calls();
    let expected: Vec<_> = ORACLE.lines().collect();
    assert_eq!(
        call(&jar, "chain", &mut host).unwrap(),
        Value::Ref(Some(expected[0].into()))
    );
    assert_eq!(
        call(&jar, "interfaces", &mut host).unwrap(),
        Value::Ref(Some(expected[1].into()))
    );
}

#[test]
fn async_factory_remains_counted() {
    let jar = fixture();
    let mut host = StdHost::new().with_strict_unknown_host_calls();
    let error = call(&jar, "async", &mut host).unwrap_err();
    assert!(matches!(error, NativeError::UnknownHostCall(_)));
    assert!(host.unknown_host_calls().iter().any(|(call, _)| {
        call == "java/util/concurrent/CompletableFuture#supplyAsync(Ljava/util/function/Supplier;)Ljava/util/concurrent/CompletableFuture;"
    }));
}

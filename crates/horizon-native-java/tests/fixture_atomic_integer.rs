#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const ORACLE: &str = include_str!("fixtures/atomic_integer/java25.txt");
const PROBE: &str = include_str!("fixtures/atomic_integer/demo/Probe.java");
#[test]
fn atomic_integer_matches_java25() {
    let fixture = javac_fixture::Fixture::build("atomic_integer", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let text = match answer {
        horizon_native_java::Value::Ref(Some(text)) => text,
        other => panic!("{other:?}"),
    };
    assert_eq!(text, ORACLE);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// `Number.intValue()` on an `AtomicInteger` is the `Number` model's call: that model holds no
/// atomic's value, so the call is counted rather than answered or refused as malformed.
#[test]
fn an_atomic_read_through_number_is_counted() {
    const SOURCE: &str = r#"package demo;
import java.util.concurrent.atomic.AtomicInteger;
public class ViaNumber {
  public static int read() { Number n = new AtomicInteger(3); return n.intValue(); }
}"#;
    let fixture =
        javac_fixture::Fixture::build("atomic_integer_number", &[("demo/ViaNumber.java", SOURCE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let _ =
        fixture
            .jar
            .invoke_method_with_args("demo/ViaNumber", "read", "()I", None, &[], &mut host);
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call == "java/lang/Number#intValue()I"),
        "{:?}",
        host.unknown_host_calls()
    );
}

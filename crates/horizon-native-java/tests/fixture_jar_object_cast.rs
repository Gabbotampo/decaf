//! A jar object cast to a JDK type it is not throws Java's
//! `ClassCastException` (#242), and `instanceof` agrees. Its class chain is
//! known: the held class files up to the class it leaves the jar at, then
//! Java 25's table above (`jdk_closure.rs`). The cast used to pass silently
//! whenever its target was a JDK type.
//!
//! `fixtures/jar_object_cast/demo/Probe.java` runs unchanged on both sides
//! (its `main` on Java 25 is `java25.txt`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/jar_object_cast/java25.txt");

const PROBE: &str = include_str!("fixtures/jar_object_cast/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Each case casts with `checkcast` and tests with `instanceof`, as Java 25
/// answers, its messages exact, with nothing counted:
/// - `klass`: a plain jar class against `String`, `Number`, `Comparable`,
///   `Runnable`, `List`, `Object[]`, `String[]`, `int[]` and `Object`;
/// - `myList`: a jar subclass of `ArrayList` is a `List`, `Collection`,
///   `RandomAccess`, `ArrayList`, `AbstractList`, `Cloneable` and
///   `Serializable`, and no `Set`, `LinkedList` or `String`;
/// - `color`: a jar enum is an `Enum`, `Comparable` and `Serializable`;
/// - `point`: a jar record is a `Record`, and nothing it does not declare;
/// - `square`: a jar class with a JDK and a jar interface is both, and
///   nothing else.
#[test]
fn a_jar_object_casts_as_its_class_chain_says() {
    let fixture = javac_fixture::Fixture::build("jar-object-cast", &[("demo/Probe.java", PROBE)]);
    let mut failures = Vec::new();
    for method in ["klass", "myList", "color", "point", "square"] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let text = match answer {
            Ok(horizon_native_java::Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != expected(method) || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A jar class implementing a JDK interface Java 25's table holds no row
/// for (`Closeable`): whether it reaches the cast's type is not known here,
/// so the cast and `instanceof` are counted, never a silent answer. Java 25
/// answers `ok|ok|CCE(…)|TF`.
#[test]
fn a_chain_through_an_undescribed_jdk_interface_is_counted() {
    let fixture =
        javac_fixture::Fixture::build("jar-object-cast-res", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let answer = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "res",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let text = match answer {
        Ok(horizon_native_java::Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    };
    let unknown = host.unknown_host_calls();
    let counted = |call: &str| unknown.iter().any(|(counted, _)| counted.starts_with(call));
    assert!(
        text == expected("res")
            || (counted("java/lang/AutoCloseable#<checkcast>")
                && counted("java/lang/AutoCloseable#<instanceof>")
                && counted("java/lang/Runnable#<checkcast>")),
        "{text} {unknown:?}"
    );
}

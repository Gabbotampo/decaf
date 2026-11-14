//! Collections models make are objects of their own, of the class Java
//! makes (#156 P14d2): `Map.ofEntries`, `List.copyOf`/`Set.copyOf`,
//! `Stream.toList()`, `Collectors.toList()`/`toUnmodifiableList()`, a
//! stream's iterator. Each used to be a `list/…`/`map/…` text literal. Held
//! in a static, each lives through collections (#226's collector).
//!
//! `fixtures/host_collections_d2/demo/Probe.java` runs unchanged on both
//! sides (its `main` on Java 25 is `java25.txt`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/host_collections_d2/java25.txt");

const PROBE: &str = include_str!("fixtures/host_collections_d2/demo/Probe.java");

fn expected(name: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {name}"))
}

/// Made in one call and kept in a static, then 40 garbage-making calls (each
/// store may collect): every collection answers its lookups, text, class
/// and mutability as Java 25 does, and the iterator goes on where it was.
/// Nothing is counted.
#[test]
fn model_made_collections_are_javas_and_survive_collections() {
    let fixture =
        javac_fixture::Fixture::build("host-collections-d2", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let mut call = |method: &str| match fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    ) {
        Ok(horizon_native_java::Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    };
    assert_eq!(call("setup"), expected("setup"));
    for _ in 0..40 {
        call("garbage");
    }
    assert_eq!(call("check"), expected("check"));
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

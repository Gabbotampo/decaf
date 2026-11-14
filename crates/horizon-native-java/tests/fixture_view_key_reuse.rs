//! A view's key is never reused while the view lives (#243's review): the
//! unmodifiable and reversed views were numbered by their table's size, and
//! since #226's collector prunes those tables, a view made after a
//! collection took a live view's number and with it the live view's
//! contents. A view kept in a static then read another view's elements.
//!
//! `fixtures/view_key_reuse/demo/KeyReuse.java` runs unchanged on both sides:
//! its `main` on Java 25 (Temurin 25.0.5, `javac --release 21`, the same on
//! two runs) is `java25.txt`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const ORACLE: &str = include_str!("fixtures/view_key_reuse/java25.txt");

fn expected(name: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {name}"))
}

fn call(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match fixture.jar.invoke_method_with_args(
        "demo/KeyReuse",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        host,
    ) {
        Ok(Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    }
}

/// Views kept in statics read their own contents through three rounds of 40
/// calls that each make and drop 80 views, each round past collections.
#[test]
fn a_kept_view_keeps_its_own_contents_across_collections() {
    let fixture = javac_fixture::Fixture::build(
        "view-key-reuse",
        &[(
            "demo/KeyReuse.java",
            include_str!("fixtures/view_key_reuse/demo/KeyReuse.java"),
        )],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(call(&fixture, &mut host, "setup"), expected("setup"));
    assert_eq!(call(&fixture, &mut host, "check"), expected("check"));
    for round in 1..=3 {
        for _ in 0..40 {
            assert_eq!(call(&fixture, &mut host, "garbage"), "ok");
        }
        assert_eq!(
            call(&fixture, &mut host, "check"),
            expected("check"),
            "round {round}"
        );
    }
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

//! `Arrays.asList` is JDK 25's fixed-size `Arrays$ArrayList` view over its
//! array (#156 P14d2, `array_views.rs`). It used to answer a `list/a,b` text
//! literal: a growable copy that no write reached.
//!
//! `fixtures/arrays_as_list/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on Java 25 (its output is `java25.txt`), its static probes
//! in Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/arrays_as_list/java25.txt");

const PROBE: &str = include_str!("fixtures/arrays_as_list/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Each probe answers as Java 25 did, with nothing counted:
/// - `writeThrough`: an array write shows in both views of it, a `set`
///   through one shows in the array and the other, `System.arraycopy` into
///   the array shows in both; the two views are two objects, equal;
/// - `fixedSize`: `add`, `add(i, e)`, `remove(int)` throw; `remove`,
///   `addAll`, `removeIf`, `retainAll`, `removeAll`, `clear` throw only when
///   they would add or remove, else answer as having done nothing;
///   `removeFirst` on an empty view is `NoSuchElementException`;
/// - `reorder`: `Collections.sort` and `set` through the view reach the
///   array, and the array's writes reach the view;
/// - `asAList`: its class (`Arrays$ArrayList`, `RandomAccess`), `List`
///   equality and hash both ways, a view inside another list and a copy see
///   the array as written, `indexOf`/`contains`, and `Arrays.asList()`;
/// - `storeCheck`: `set` checks the value against the array's component type
///   as `aastore` does (`ArrayStoreException`, the array untouched);
/// - `subLists`: a view's `subList` is a window on the same array: `set`
///   writes through both ways, size changes throw, nested windows, the
///   window's range checks and class;
/// - `jarEquals`: elements with their own `equals` keep the size fixed
///   (`remove`, `removeAll`, `retainAll`, with `ArrayItr`'s "remove");
/// - `live`: iteration and `forEach` read the array as it is when reached,
///   reflection's `Array.set` reaches the view, and out of range is the
///   array's `ArrayIndexOutOfBoundsException`.
#[test]
fn arrays_as_list_is_a_fixed_size_view_of_its_array() {
    let fixture = javac_fixture::Fixture::build("arrays-as-list", &[("demo/Probe.java", PROBE)]);
    let mut failures = Vec::new();
    for method in [
        "writeThrough",
        "fixedSize",
        "reorder",
        "asAList",
        "storeCheck",
        "subLists",
        "jarEquals",
        "live",
    ] {
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

/// A view and its window held only in statics keep their array through
/// collections (#226's collector: the view's array is an edge from it): 40
/// garbage-making calls later, a `set` through the window shows in the view,
/// which is still fixed-size. Java 25's answer, nothing counted.
#[test]
fn a_held_view_keeps_its_array_through_collections() {
    let fixture =
        javac_fixture::Fixture::build("arrays-as-list-held", &[("demo/Probe.java", PROBE)]);
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
    assert_eq!(call("hold"), "held");
    for _ in 0..40 {
        call("garbage");
    }
    assert_eq!(call("afterCollections"), expected("afterCollections"));
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// An iterator over a view, held without the view, keeps following the
/// array through collections (#247 re-check: the collector freed the view,
/// and the iterator stopped seeing the array's writes): its view is an edge
/// from it. Java 25's answer, nothing counted.
#[test]
fn a_held_iterator_keeps_its_view_through_collections() {
    let fixture =
        javac_fixture::Fixture::build("arrays-as-list-iterator", &[("demo/Probe.java", PROBE)]);
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
    assert_eq!(call("holdIterator"), "held");
    for _ in 0..40 {
        call("garbage");
    }
    assert_eq!(
        call("afterIteratorCollections"),
        expected("afterIteratorCollections")
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

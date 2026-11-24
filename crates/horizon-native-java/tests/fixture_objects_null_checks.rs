//! `java.util.Objects` null-check messages and the `checkIndex` family as
//! JDK 25 defines them.
//!
//! `fixtures/objects_null_checks/java25.txt` is the raw output of the
//! committed probe on JDK 25 (see the README there). Every row is either
//! answered exactly with nothing counted, or counted as an unknown host call
//! because the answer needs a live `Supplier.get()`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/objects_null_checks/demo/Probe.java");
const JAVA25: &str = include_str!("fixtures/objects_null_checks/java25.txt");

const REQUIRE_NON_NULL_SUPPLIER: &str = "java/util/Objects#requireNonNull(Ljava/lang/Object;Ljava/util/function/Supplier;)Ljava/lang/Object;";
const REQUIRE_NON_NULL_ELSE_GET: &str = "java/util/Objects#requireNonNullElseGet(Ljava/lang/Object;Ljava/util/function/Supplier;)Ljava/lang/Object;";

/// Rows that need a live supplier's `get()`: counted, never answered.
const COUNTED: &[(&str, &str)] = &[
    ("case_rn_supplier_null", REQUIRE_NON_NULL_SUPPLIER),
    ("case_rn_supplier_null_return", REQUIRE_NON_NULL_SUPPLIER),
    ("case_rneg_null_supplier", REQUIRE_NON_NULL_ELSE_GET),
    ("case_rneg_null_null_return", REQUIRE_NON_NULL_ELSE_GET),
];

#[test]
fn objects_null_checks_match_java25() {
    let fixture = Fixture::build("objects-null-checks", &[("demo/Probe.java", PROBE)]);
    let mut seen_counted = 0;
    for line in JAVA25.lines() {
        let (label, expected) = line.split_once('=').expect("oracle row");
        let mut host = fixture.jar.seed_host(StdHost::new());
        let result = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            label,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let unknown = host.unknown_host_calls();
        if let Some((_, key)) = COUNTED.iter().find(|(counted, _)| *counted == label) {
            seen_counted += 1;
            assert_eq!(unknown, vec![((*key).to_string(), 1)], "{label}");
            continue;
        }
        let Ok(Value::Ref(Some(text))) = result else {
            panic!("{label}: {result:?}");
        };
        assert_eq!(text, expected, "{label}");
        assert!(unknown.is_empty(), "{label}: {unknown:?}");
    }
    assert_eq!(seen_counted, COUNTED.len());
}

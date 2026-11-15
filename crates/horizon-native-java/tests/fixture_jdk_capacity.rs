//! Java 25 capacity-constructor and ensureCapacity cases for the plain JDK collections.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{NativeError, StdHost, Value};

const BOUND: usize = 1 << 24;
const PROBE: &str = include_str!("fixtures/jdk_capacity/Probe.java");

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build("jdk-capacity", &[("demo/Probe.java", PROBE)])
}

fn invoke(fixture: &javac_fixture::Fixture, method: &str) -> Result<Value, NativeError> {
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    )
}

fn refusal(fixture: &javac_fixture::Fixture, method: &str, elements: usize) {
    match invoke(fixture, method) {
        Err(NativeError::Metadata(message)) => assert!(
            message.starts_with(&format!(
                "array of {elements} elements exceeds the runtime's bound of {BOUND}"
            )),
            "{method}: {message}"
        ),
        other => panic!("{method} returned {other:?}"),
    }
}

#[test]
fn zero_and_negative_values_match_java25() {
    let fixture = fixture();
    assert_eq!(fixture.text("zero"), "ok");
    assert_eq!(
        fixture.text("negatives"),
        "java.lang.IllegalArgumentException:Illegal Capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
java.lang.IllegalArgumentException:Illegal initial capacity: -1|\
ok|\
java.lang.NegativeArraySizeException:-1|\
java.lang.IllegalArgumentException:Illegal Capacity: -1|\
ok|ok|ok"
    );
}

#[test]
fn bound_is_accepted_when_the_backing_array_is_at_most_the_limit() {
    let fixture = fixture();
    for method in [
        "arrayListBound",
        "hashMapBound",
        "hashMapFloatBound",
        "hashSetBound",
        "linkedHashMapBound",
        "linkedHashMapFloatBound",
        "linkedHashMapOrderBound",
        "dequeUnderBound",
        "stringBuilderBound",
        "vectorBound",
        "ensureArrayListBound",
        "ensureVectorBound",
        "ensureBuilderBound",
    ] {
        assert_eq!(fixture.text(method), "ok", "{method}");
    }
}

#[test]
fn oversized_backing_arrays_are_refused() {
    let fixture = fixture();
    for (method, elements) in [
        ("arrayListOver", BOUND + 1),
        ("arrayListMax", i32::MAX as usize),
        ("hashMapOver", 1 << 25),
        ("hashMapMax", 1 << 30),
        ("hashMapFloatOver", 1 << 25),
        ("hashMapFloatMax", 1 << 30),
        ("hashSetOver", 1 << 25),
        ("hashSetMax", 1 << 30),
        ("linkedHashMapOver", 1 << 25),
        ("linkedHashMapMax", 1 << 30),
        ("linkedHashMapFloatOver", 1 << 25),
        ("linkedHashMapFloatMax", 1 << 30),
        ("linkedHashMapOrderOver", 1 << 25),
        ("linkedHashMapOrderMax", 1 << 30),
        ("dequeBound", BOUND + 1),
        ("dequeOver", BOUND + 2),
        ("dequeMax", i32::MAX as usize),
        ("stringBuilderOver", BOUND + 1),
        ("stringBuilderMax", i32::MAX as usize),
        ("vectorOver", BOUND + 1),
        ("vectorMax", i32::MAX as usize),
        ("ensureArrayListOver", BOUND + 1),
        ("ensureArrayListMax", i32::MAX as usize),
        ("ensureVectorOver", BOUND + 1),
        ("ensureVectorMax", i32::MAX as usize),
        ("ensureBuilderOver", BOUND + 1),
        ("ensureBuilderMax", i32::MAX as usize),
    ] {
        refusal(&fixture, method, elements);
    }
}

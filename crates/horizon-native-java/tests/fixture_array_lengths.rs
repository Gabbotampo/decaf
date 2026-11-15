//! Java array creation keeps the exact computed length from Java 25.
//!
//! The first case is EntityCulling's `ArrayOcclusionCache` constructor:
//! `reach=128` gives `(reach * 2)^3 / 4 == 4_194_304`, a valid byte-array
//! length. The remaining cases pin zero and partial `multianewarray`
//! dimensions and Java's negative-size exception message.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = r#"package demo;

import java.util.function.Supplier;

public final class Probe {
    static String attempt(Supplier<String> s) {
        try { return "ok:" + s.get(); }
        catch (Throwable e) { return e.getClass().getName() + ":" + e.getMessage(); }
    }

    public static String computed() {
        int reach = 128;
        int reachX2 = reach * 2;
        int length = reachX2 * reachX2 * reachX2 / 4;
        byte[] cache = new byte[length];
        return length + ":" + cache.length + ":" + cache[0] + ":" + cache[length - 1];
    }

    public static String dimensions() {
        int[][][] zero = new int[0][7][9];
        int[][][] partial = new int[2][3][];
        int[][][] leafZero = new int[2][3][0];
        int[][][] innerZero = new int[2][0][];
        return zero.length + ":" + partial.length + ":" + partial[0].length + ":" + (partial[0][0] == null)
            + ":" + leafZero[1][2].length + ":" + innerZero[1].length + ":" + (innerZero[1] != null);
    }

    public static String negatives() {
        return attempt(() -> String.valueOf(new int[-1].length)) + "|"
            + attempt(() -> String.valueOf(new int[2][-3].length)) + "|"
            + attempt(() -> String.valueOf(new int[0][-3].length)) + "|"
            + attempt(() -> String.valueOf(new int[2][3][-4].length));
    }

    public static String zeroAndEmpty() {
        int[] a = new int[0];
        String[] b = new String[0];
        return a.length + ":" + b.length + ":" + java.lang.reflect.Array.getLength(
            java.lang.reflect.Array.newInstance(byte.class, 0));
    }
}
"#;

#[test]
fn array_lengths_match_java_25() {
    let fixture = javac_fixture::Fixture::build("array-lengths", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("computed", "4194304:4194304:0:0"),
        ("dimensions", "0:2:3:true:0:0:true"),
        (
            "negatives",
            "java.lang.NegativeArraySizeException:-1|java.lang.NegativeArraySizeException:-3|java.lang.NegativeArraySizeException:-3|java.lang.NegativeArraySizeException:-4",
        ),
        ("zeroAndEmpty", "0:0:0"),
    ];
    let mut failures = Vec::new();
    for (method, expected) in cases {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let text = match &answer {
            Ok(Value::Ref(Some(text))) => text.clone(),
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != expected || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  jdk 25: {expected}\n  native: {text}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "differ from JDK 25:\n{}",
        failures.join("\n")
    );
}

//! `i2c` (0x92) keeps the low 16 bits of an int as an unsigned char: JVMS 6.5 truncates to `char`, then
//! zero-extends. `i2b` and `i2s` truncate to 8 and 16 bits and sign-extend. The probe feeds values outside
//! each target range (from a static array, so javac cannot fold the casts) through casts, compound
//! assignments, increments, field and array stores, calls and strings. `java25.txt` is Java 25's output
//! for each method, one `name=value` line per method (see `fixtures/i2c_char_cast/README.md`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const ORACLE: &str = include_str!("fixtures/i2c_char_cast/java25.txt");
const PROBE: &str = include_str!("fixtures/i2c_char_cast/demo/Probe.java");

#[test]
fn i2c_char_cast_matches_java25() {
    let fixture = javac_fixture::Fixture::build("i2c-char-cast", &[("demo/Probe.java", PROBE)]);
    let mut mismatches = Vec::new();
    let mut checked = 0;
    for line in ORACLE.lines() {
        let (method, expected) = line.split_once('=').unwrap();
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let got = fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap();
        let got = match got {
            horizon_native_java::Value::Ref(Some(s)) => s,
            other => format!("{other:?}"),
        };
        if got != expected {
            mismatches.push(format!("{method}\n  java25: {expected}\n  ours:   {got}"));
        }
        assert!(
            host.unknown_host_calls().is_empty(),
            "{method}: unmodelled host calls {:?}",
            host.unknown_host_calls()
        );
        checked += 1;
    }
    assert_eq!(checked, 13, "every probe method has an oracle line");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

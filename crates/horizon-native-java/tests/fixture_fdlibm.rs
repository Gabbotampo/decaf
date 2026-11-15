//! Exact fdlibm dispatch for the Math/StrictMath transcendental subset.

use horizon_native_java::{Host, MethodRef, StdHost, Value};

fn call(owner: &str, name: &str, descriptor: &str, args: &[Value]) -> Value {
    let method = MethodRef {
        owner: owner.into(),
        name: name.into(),
        descriptor: descriptor.into(),
    };
    StdHost::new().invoke(&method, args).unwrap()
}

fn unary(owner: &str, name: &str, value: f64) -> f64 {
    match call(owner, name, "(D)D", &[Value::Float(value)]) {
        Value::Float(value) => value,
        other => panic!("{owner}.{name} returned {other:?}"),
    }
}

#[test]
fn strictmath_fdlibm_specials_and_values() {
    assert_eq!(
        call(
            "java/lang/StrictMath",
            "pow",
            "(DD)D",
            &[Value::Float(2.0), Value::Float(10.0)]
        ),
        Value::Float(1024.0)
    );
    assert_eq!(
        unary("java/lang/StrictMath", "tan", 0.0).to_bits(),
        0.0f64.to_bits()
    );
    assert_eq!(unary("java/lang/StrictMath", "exp", 0.0), 1.0);
    assert_eq!(unary("java/lang/StrictMath", "log", 1.0), 0.0);
    assert_eq!(unary("java/lang/StrictMath", "log10", 1.0), 0.0);
    assert_eq!(unary("java/lang/StrictMath", "cbrt", -8.0), -2.0);
    assert_eq!(
        unary("java/lang/StrictMath", "tanh", 0.0).to_bits(),
        0.0f64.to_bits()
    );
    let nan = f64::from_bits(0x7ff0_0000_0000_0001);
    assert_eq!(
        unary("java/lang/StrictMath", "cbrt", nan).to_bits(),
        nan.to_bits()
    );
    let negative_nan = f64::from_bits(0xfff0_0000_0000_0001);
    assert_eq!(
        unary("java/lang/StrictMath", "log", negative_nan).to_bits(),
        0xfff8_0000_0000_0001
    );
}

#[cfg(target_arch = "aarch64")]
#[test]
fn math_fdlibm_is_enabled_on_aarch64() {
    assert_eq!(
        call(
            "java/lang/Math",
            "pow",
            "(DD)D",
            &[Value::Float(2.0), Value::Float(10.0)]
        ),
        Value::Float(1024.0)
    );
    assert_eq!(unary("java/lang/Math", "log", 1.0), 0.0);
    assert_eq!(
        unary(
            "java/lang/Math",
            "log",
            f64::from_bits(0xfff0_0000_0000_0001)
        )
        .to_bits(),
        0x7ff8_0000_0000_0000
    );
}

#[cfg(not(target_arch = "aarch64"))]
#[test]
fn math_fdlibm_stays_counted_off_aarch64() {
    let mut host = StdHost::new();
    let method = MethodRef {
        owner: "java/lang/Math".into(),
        name: "pow".into(),
        descriptor: "(DD)D".into(),
    };
    let _ = host.invoke(&method, &[Value::Float(2.0), Value::Float(10.0)]);
    assert_eq!(host.unknown_host_call_count(), 1);
}

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

#[test]
fn strictmath_pow_signed_exponents_match_java_25_rows() {
    // Raw bits recorded from a Java 25.0.5 run. The rows cover both signs,
    // signed zeroes, infinities and NaN for every small exponent branch.
    let exponents = [2.0, -2.0, 0.5, -0.5, 1.0, -1.0, 3.0];
    let rows: &[(u64, [u64; 7])] = &[
        (
            0xfff0_0000_0000_0000,
            [
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0,
                0xfff0_0000_0000_0000,
                0x8000_0000_0000_0000,
                0xfff0_0000_0000_0000,
            ],
        ),
        (
            0xc024_0000_0000_0000,
            [
                0x4059_0000_0000_0000,
                0x3f84_7ae1_47ae_147b,
                0x7ff8_0000_0000_0000,
                0x7ff8_0000_0000_0000,
                0xc024_0000_0000_0000,
                0xbfb9_9999_9999_999a,
                0xc08f_4000_0000_0000,
            ],
        ),
        (
            0xbff0_0000_0000_0000,
            [
                0x3ff0_0000_0000_0000,
                0x3ff0_0000_0000_0000,
                0x7ff8_0000_0000_0000,
                0x7ff8_0000_0000_0000,
                0xbff0_0000_0000_0000,
                0xbff0_0000_0000_0000,
                0xbff0_0000_0000_0000,
            ],
        ),
        (
            0x8000_0000_0000_0000,
            [
                0,
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0x8000_0000_0000_0000,
                0xfff0_0000_0000_0000,
                0x8000_0000_0000_0000,
            ],
        ),
        (
            0x0000_0000_0000_0000,
            [
                0,
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0,
            ],
        ),
        (0x3ff0_0000_0000_0000, [0x3ff0_0000_0000_0000; 7]),
        (
            0x4024_0000_0000_0000,
            [
                0x4059_0000_0000_0000,
                0x3f84_7ae1_47ae_147b,
                0x4009_4c58_3ada_5b53,
                0x3fd4_3d13_6248_490f,
                0x4024_0000_0000_0000,
                0x3fb9_9999_9999_999a,
                0x408f_4000_0000_0000,
            ],
        ),
        (
            0x7ff0_0000_0000_0000,
            [
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
                0,
                0x7ff0_0000_0000_0000,
            ],
        ),
        (0x7ff8_0000_0000_0000, [0x7ff8_0000_0000_0000; 7]),
    ];
    for &(x_bits, expected) in rows {
        for (index, &y) in exponents.iter().enumerate() {
            let method = MethodRef {
                owner: "java/lang/StrictMath".into(),
                name: "pow".into(),
                descriptor: "(DD)D".into(),
            };
            let mut host = StdHost::new();
            let got = host
                .invoke(
                    &method,
                    &[Value::Float(f64::from_bits(x_bits)), Value::Float(y)],
                )
                .unwrap();
            #[cfg(not(target_arch = "aarch64"))]
            if f64::from_bits(expected[index]).is_nan() {
                assert_eq!(
                    host.unknown_host_call_count(),
                    1,
                    "x={x_bits:016x} y={y} NaN result must stay counted"
                );
                continue;
            }
            assert_eq!(
                got,
                Value::Float(f64::from_bits(expected[index])),
                "x={x_bits:016x} y={y}"
            );
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[test]
fn math_pow_signed_exponents_match_strictmath_rows() {
    for &(x, y) in &[
        (10.0, -2.0),
        (2.0, -0.5),
        (0.0, -2.0),
        (-0.0, -0.5),
        (4.0, 0.5),
    ] {
        assert_eq!(
            call(
                "java/lang/Math",
                "pow",
                "(DD)D",
                &[Value::Float(x), Value::Float(y)]
            ),
            call(
                "java/lang/StrictMath",
                "pow",
                "(DD)D",
                &[Value::Float(x), Value::Float(y)]
            )
        );
    }
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
fn strictmath_pow_nan_is_counted_on_x86_due_tier_payloads() {
    let mut host = StdHost::new();
    let method = MethodRef {
        owner: "java/lang/StrictMath".into(),
        name: "pow".into(),
        descriptor: "(DD)D".into(),
    };
    let result = host.invoke(
        &method,
        &[
            Value::Float(f64::from_bits(0x7ff8_0000_0000_0001)),
            Value::Float(2.0),
        ],
    );
    assert!(result.is_ok());
    assert_eq!(host.unknown_host_call_count(), 1);
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

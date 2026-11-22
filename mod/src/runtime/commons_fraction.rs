//! Exact Apache Commons Lang 3 (`commons-lang3-3.20.0.jar`) `Fraction` model.
//!
//! Every constant, arithmetic method, rounding/reduction rule, overflow check,
//! exception message, and object-identity rule matches `org.apache.commons.lang3.math.Fraction`
//! bytecode in `commons-lang3-3.20.0.jar`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::runtime::{MethodRef, NativeError, Value, expect_ref};

pub(crate) const FRACTION: &str = "org/apache/commons/lang3/math/Fraction";
const NUMBER: &str = "java/lang/Number";
const COMPARABLE: &str = "java/lang/Comparable";
const SERIALIZABLE: &str = "java/io/Serializable";
const OBJECT: &str = "java/lang/Object";

static NEXT_FRACTION: AtomicU64 = AtomicU64::new(0);

const CONSTANTS: &[(&str, i32, i32)] = &[
    ("ZERO", 0, 1),
    ("ONE", 1, 1),
    ("ONE_HALF", 1, 2),
    ("ONE_THIRD", 1, 3),
    ("TWO_THIRDS", 2, 3),
    ("ONE_QUARTER", 1, 4),
    ("TWO_QUARTERS", 2, 4),
    ("THREE_QUARTERS", 3, 4),
    ("ONE_FIFTH", 1, 5),
    ("TWO_FIFTHS", 2, 5),
    ("THREE_FIFTHS", 3, 5),
    ("FOUR_FIFTHS", 4, 5),
];

fn constant_value(name: &str, num: i32, den: i32) -> Value {
    Value::Host(HostRef::new(
        HostKind::CommonsFraction,
        format!("c:{name}:{num}:{den}"),
    ))
}

fn alloc_fraction(num: i32, den: i32) -> Value {
    let serial = NEXT_FRACTION.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(
        HostKind::CommonsFraction,
        format!("s:{serial}:{num}:{den}"),
    ))
}

fn zero_constant() -> Value {
    constant_value("ZERO", 0, 1)
}

fn one_constant() -> Value {
    constant_value("ONE", 1, 1)
}

pub(crate) fn parse_key(key: &str) -> Option<(i32, i32)> {
    let mut parts = key.split(':');
    let prefix = parts.next()?;
    if prefix != "c" && prefix != "s" {
        return None;
    }
    let _tag = parts.next()?;
    let num = parts.next()?.parse::<i32>().ok()?;
    let den = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() || den <= 0 {
        return None;
    }
    Some((num, den))
}

pub(crate) fn fraction_parts(value: &Value) -> Option<(i32, i32)> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::CommonsFraction => {
            parse_key(handle.key())
        }
        _ => None,
    }
}

fn arithmetic_exception(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/ArithmeticException".into(),
        message: Some(message.to_owned()),
        frame: None,
    }
}

fn npe(message: Option<&str>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: message.map(str::to_owned),
        frame: None,
    }
}

fn number_format(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NumberFormatException".into(),
        message: Some(message),
        frame: None,
    }
}

fn value_class_name(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(text)) => {
            if let Some(class) = crate::runtime::jar_object::object_class(value) {
                Some(class)
            } else if crate::runtime::value_class::is_plain_string(text) {
                Some("java/lang/String")
            } else {
                None
            }
        }
        Value::Host(handle) => handle.class(),
        Value::Boxed(b) => Some(match b {
            crate::runtime::Boxed::Boolean(_) => "java/lang/Boolean",
            crate::runtime::Boxed::Byte(_) => "java/lang/Byte",
            crate::runtime::Boxed::Short(_) => "java/lang/Short",
            crate::runtime::Boxed::Character(_) => "java/lang/Character",
            crate::runtime::Boxed::Integer(_) => "java/lang/Integer",
            crate::runtime::Boxed::Float(_) => "java/lang/Float",
        }),
        Value::Int(_) => Some("java/lang/Integer"),
        Value::Long(_) => Some("java/lang/Long"),
        Value::Float(_) => Some("java/lang/Double"),
        Value::Ref(None) | Value::Void => None,
    }
}

fn add_and_check(x: i32, y: i32) -> Result<i32, NativeError> {
    let s = i64::from(x) + i64::from(y);
    if s < i64::from(i32::MIN) || s > i64::from(i32::MAX) {
        return Err(arithmetic_exception("overflow: add"));
    }
    Ok(s as i32)
}

fn sub_and_check(x: i32, y: i32) -> Result<i32, NativeError> {
    let s = i64::from(x) - i64::from(y);
    if s < i64::from(i32::MIN) || s > i64::from(i32::MAX) {
        // Commons Lang 3 `subAndCheck` intentionally throws `"overflow: add"`.
        return Err(arithmetic_exception("overflow: add"));
    }
    Ok(s as i32)
}

fn mul_and_check(x: i32, y: i32) -> Result<i32, NativeError> {
    let m = i64::from(x) * i64::from(y);
    if m < i64::from(i32::MIN) || m > i64::from(i32::MAX) {
        return Err(arithmetic_exception("overflow: mul"));
    }
    Ok(m as i32)
}

fn mul_pos_and_check(x: i32, y: i32) -> Result<i32, NativeError> {
    let m = i64::from(x) * i64::from(y);
    if m > i64::from(i32::MAX) {
        return Err(arithmetic_exception("overflow: mulPos"));
    }
    Ok(m as i32)
}

fn greatest_common_divisor(mut u: i32, mut v: i32) -> Result<i32, NativeError> {
    if u == 0 || v == 0 {
        if u == i32::MIN || v == i32::MIN {
            return Err(arithmetic_exception("overflow: gcd is 2^31"));
        }
        return Ok(u.wrapping_abs().wrapping_add(v.wrapping_abs()));
    }
    if u.wrapping_abs() == 1 || v.wrapping_abs() == 1 {
        return Ok(1);
    }
    if u > 0 {
        u = -u;
    }
    if v > 0 {
        v = -v;
    }
    let mut k = 0;
    while (u & 1) == 0 && (v & 1) == 0 && k < 31 {
        u /= 2;
        v /= 2;
        k += 1;
    }
    if k == 31 {
        return Err(arithmetic_exception("overflow: gcd is 2^31"));
    }
    let mut t = if (u & 1) == 1 { v } else { -(u / 2) };
    loop {
        while (t & 1) == 0 {
            t /= 2;
        }
        if t > 0 {
            u = -t;
        } else {
            v = t;
        }
        t = (v - u) / 2;
        if t == 0 {
            break;
        }
    }
    Ok((-u).wrapping_mul(1_i32 << k))
}

fn get_fraction_int_int(mut num: i32, mut den: i32) -> Result<Value, NativeError> {
    if den == 0 {
        return Err(arithmetic_exception("The denominator must not be zero"));
    }
    if den < 0 {
        if num == i32::MIN || den == i32::MIN {
            return Err(arithmetic_exception("overflow: can't negate"));
        }
        num = -num;
        den = -den;
    }
    Ok(alloc_fraction(num, den))
}

fn get_fraction_whole_num_den(whole: i32, num: i32, den: i32) -> Result<Value, NativeError> {
    if den == 0 {
        return Err(arithmetic_exception("The denominator must not be zero"));
    }
    if den < 0 {
        return Err(arithmetic_exception("The denominator must not be negative"));
    }
    if num < 0 {
        return Err(arithmetic_exception("The numerator must not be negative"));
    }
    let numerator_value = if whole < 0 {
        i64::from(whole) * i64::from(den) - i64::from(num)
    } else {
        i64::from(whole) * i64::from(den) + i64::from(num)
    };
    if numerator_value < i64::from(i32::MIN) || numerator_value > i64::from(i32::MAX) {
        return Err(arithmetic_exception(
            "Numerator too large to represent as an Integer.",
        ));
    }
    Ok(alloc_fraction(numerator_value as i32, den))
}

fn get_reduced_fraction(mut num: i32, mut den: i32) -> Result<Value, NativeError> {
    if den == 0 {
        return Err(arithmetic_exception("The denominator must not be zero"));
    }
    if num == 0 {
        return Ok(zero_constant());
    }
    if den == i32::MIN && (num & 1) == 0 {
        num /= 2;
        den /= 2;
    }
    if den < 0 {
        if num == i32::MIN || den == i32::MIN {
            return Err(arithmetic_exception("overflow: can't negate"));
        }
        num = -num;
        den = -den;
    }
    let gcd = greatest_common_divisor(num, den)?;
    Ok(alloc_fraction(num / gcd, den / gcd))
}

fn get_fraction_double(mut value: f64) -> Result<Value, NativeError> {
    let sign = if value < 0.0 { -1_i32 } else { 1_i32 };
    value = value.abs();
    if value > f64::from(i32::MAX) || value.is_nan() {
        return Err(arithmetic_exception(
            "The value must not be greater than Integer.MAX_VALUE or NaN",
        ));
    }
    let whole_number = value as i32;
    value -= f64::from(whole_number);

    let mut numer0 = 0_i32;
    let mut denom0 = 1_i32;
    let mut numer1 = 1_i32;
    let mut denom1 = 0_i32;
    let mut a1 = value as i32;
    let mut x1 = 1.0_f64;
    let mut y1 = value - f64::from(a1);
    let mut delta2 = f64::MAX;
    let mut i = 1_i32;

    loop {
        let delta1 = delta2;
        let a2 = (x1 / y1) as i32;
        let x2 = y1;
        let y2 = x1 - f64::from(a2) * y1;
        let numer2 = a1.wrapping_mul(numer1).wrapping_add(numer0);
        let denom2 = a1.wrapping_mul(denom1).wrapping_add(denom0);
        let fraction = f64::from(numer2) / f64::from(denom2);
        delta2 = (value - fraction).abs();
        a1 = a2;
        x1 = x2;
        y1 = y2;
        numer0 = numer1;
        denom0 = denom1;
        numer1 = numer2;
        denom1 = denom2;
        i += 1;
        if !(delta1 > delta2 && denom2 <= 10000 && denom2 > 0 && i < 25) {
            break;
        }
    }
    if i == 25 {
        return Err(arithmetic_exception("Unable to convert double to fraction"));
    }
    let combined = numer0
        .wrapping_add(whole_number.wrapping_mul(denom0))
        .wrapping_mul(sign);
    get_reduced_fraction(combined, denom0)
}

fn parse_java_int(text: &str) -> Result<i32, NativeError> {
    let call = MethodRef {
        owner: "java/lang/Integer".into(),
        name: "parseInt".into(),
        descriptor: "(Ljava/lang/String;)I".into(),
    };
    match crate::runtime::java_numbers::static_call(&call, &[Value::Ref(Some(text.to_owned()))]) {
        Some(Ok(Value::Int(n))) => Ok(n),
        Some(Err(err)) => Err(err),
        _ => Err(number_format(format!("For input string: \"{text}\""))),
    }
}

fn parse_java_hex_double(body: &str, negative: bool) -> Option<f64> {
    let body = body.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(body);
    let rest = body
        .strip_prefix("0x")
        .or_else(|| body.strip_prefix("0X"))?;
    let exp_pos = rest.find(['p', 'P'])?;
    let sig_part = &rest[..exp_pos];
    let exp_part = &rest[exp_pos + 1..];

    let (exp_neg, exp_digits) = if let Some(d) = exp_part.strip_prefix('-') {
        (true, d)
    } else if let Some(d) = exp_part.strip_prefix('+') {
        (false, d)
    } else {
        (false, exp_part)
    };
    if exp_digits.is_empty() || !exp_digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut exp_mag = 0_i64;
    for b in exp_digits.bytes() {
        exp_mag = exp_mag
            .saturating_mul(10)
            .saturating_add(i64::from(b - b'0'))
            .min(1_000_000_000);
    }
    let bin_exp = if exp_neg { -exp_mag } else { exp_mag };

    let (int_hex, frac_hex) = match sig_part.split_once('.') {
        Some((i, f)) => {
            if f.contains('.') {
                return None;
            }
            (i, f)
        }
        None => (sig_part, ""),
    };
    if int_hex.is_empty() && frac_hex.is_empty() {
        return None;
    }
    if !int_hex.bytes().all(|b| b.is_ascii_hexdigit())
        || !frac_hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }

    let hex_val = |b: u8| -> u64 {
        match b {
            b'0'..=b'9' => u64::from(b - b'0'),
            b'a'..=b'f' => u64::from(b - b'a' + 10),
            b'A'..=b'F' => u64::from(b - b'A' + 10),
            _ => unreachable!(),
        }
    };

    let mut started = false;
    let mut sig = 0_u64;
    let mut sig_bits = 0_u32;
    let mut sticky = false;
    let mut msb_exp = 0_i64;

    for b in int_hex.bytes() {
        let d = hex_val(b);
        for shift in (0..4).rev() {
            let bit = (d >> shift) & 1;
            if !started {
                if bit == 1 {
                    started = true;
                    sig = 1;
                    sig_bits = 1;
                    msb_exp = 0;
                }
            } else {
                msb_exp = msb_exp.saturating_add(1);
                if sig_bits < 55 {
                    sig = (sig << 1) | bit;
                    sig_bits += 1;
                } else if bit == 1 {
                    sticky = true;
                }
            }
        }
    }

    let mut frac_shift = 0_i64;
    for b in frac_hex.bytes() {
        let d = hex_val(b);
        for shift in (0..4).rev() {
            let bit = (d >> shift) & 1;
            if !started {
                frac_shift = frac_shift.saturating_sub(1);
                if bit == 1 {
                    started = true;
                    sig = 1;
                    sig_bits = 1;
                    msb_exp = frac_shift;
                }
            } else if sig_bits < 55 {
                sig = (sig << 1) | bit;
                sig_bits += 1;
            } else if bit == 1 {
                sticky = true;
            }
        }
    }

    let sign_bit = u64::from(negative) << 63;
    if !started {
        return Some(f64::from_bits(sign_bit));
    }

    sig <<= 55 - sig_bits;
    let exp = bin_exp.saturating_add(msb_exp);
    if exp >= -1022 {
        let mantissa = sig >> 2;
        let guard = (sig >> 1) & 1 != 0;
        let round_sticky = (sig & 1 != 0) || sticky;
        let round_up = guard && (round_sticky || (mantissa & 1) == 1);
        let mut m = mantissa + u64::from(round_up);
        let mut e = exp;
        if m == (1_u64 << 53) {
            m >>= 1;
            e += 1;
        }
        if e > 1023 {
            return Some(f64::from_bits(sign_bit | (0x7ff_u64 << 52)));
        }
        let biased = (e + 1023) as u64;
        return Some(f64::from_bits(
            sign_bit | (biased << 52) | (m & ((1_u64 << 52) - 1)),
        ));
    }

    // Subnormal or underflow: `sig` has leading 1 at bit 54, value = sig * 2^(exp - 54).
    // Align to 2^-1074 by shifting right by `(-1020 - exp)` (at least 3).
    let rshift_i64 = -1020_i64 - exp;
    if rshift_i64 > 55 {
        return Some(f64::from_bits(sign_bit));
    }
    let rshift = rshift_i64 as u32;
    let mut m = sig >> rshift;
    let half_bit = (sig >> (rshift - 1)) & 1 != 0;
    let lower_mask = (1_u64 << (rshift - 1)) - 1;
    let rest_sticky = sticky || (sig & lower_mask) != 0;
    if half_bit && (rest_sticky || (m & 1) == 1) {
        m += 1;
    }
    Some(f64::from_bits(sign_bit | m))
}

fn is_java_decimal_float(body: &str) -> bool {
    let (mant, exp) = match body.find(['e', 'E']) {
        Some(pos) => (&body[..pos], Some(&body[pos + 1..])),
        None => (body, None),
    };
    if let Some(exp_str) = exp {
        let digits = exp_str.strip_prefix(['+', '-']).unwrap_or(exp_str);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    let (int_part, frac_part) = match mant.split_once('.') {
        Some((i, f)) => {
            if f.contains('.') {
                return false;
            }
            (i, f)
        }
        None => (mant, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return false;
    }
    int_part.bytes().all(|b| b.is_ascii_digit()) && frac_part.bytes().all(|b| b.is_ascii_digit())
}

fn parse_java_double(raw: &str) -> Result<f64, NativeError> {
    let trimmed = raw.trim_matches(|ch: char| ch <= ' ');
    if trimmed.is_empty() {
        return Err(number_format("empty String".to_owned()));
    }
    let (negative, body) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };
    if body == "NaN" {
        return Ok(f64::NAN);
    }
    if body == "Infinity" {
        return Ok(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if body.starts_with("0x") || body.starts_with("0X") {
        return parse_java_hex_double(body, negative)
            .ok_or_else(|| number_format(format!("For input string: \"{trimmed}\"")));
    }
    let stripped = body.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(body);
    if !is_java_decimal_float(stripped) {
        return Err(number_format(format!("For input string: \"{trimmed}\"")));
    }
    let parsed = stripped
        .parse::<f64>()
        .map_err(|_| number_format(format!("For input string: \"{trimmed}\"")))?;
    Ok(if negative { -parsed } else { parsed })
}

fn get_fraction_string(value: &Value) -> Option<Result<Value, NativeError>> {
    if matches!(value, Value::Ref(None)) {
        return Some(Err(npe(Some("str"))));
    }
    // Parameter type is `Ljava/lang/String;`: strings like `"3/4"` contain `/` and are
    // valid fraction strings, so accept any `Value::Ref(Some(text))` rather than
    // filtering with `is_plain_string` (which rejects `a/b` shapes).
    let text = expect_ref(value)?;
    Some(parse_fraction_string(text))
}

fn parse_fraction_string(text: &str) -> Result<Value, NativeError> {
    if text.contains('.') {
        let parsed = parse_java_double(text)?;
        return get_fraction_double(parsed);
    }
    if let Some(pos) = text.find(' ')
        && pos > 0
    {
        let whole = parse_java_int(&text[..pos])?;
        let rest = &text[pos + 1..];
        let Some(slash) = rest.find('/') else {
            return Err(number_format(
                "The fraction could not be parsed as the format X Y/Z".to_owned(),
            ));
        };
        let numer = parse_java_int(&rest[..slash])?;
        let denom = parse_java_int(&rest[slash + 1..])?;
        return get_fraction_whole_num_den(whole, numer, denom);
    }
    let Some(slash) = text.find('/') else {
        return get_fraction_int_int(parse_java_int(text)?, 1);
    };
    let numer = parse_java_int(&text[..slash])?;
    let denom = parse_java_int(&text[slash + 1..])?;
    get_fraction_int_int(numer, denom)
}

fn negate(num: i32, den: i32) -> Result<Value, NativeError> {
    if num == i32::MIN {
        return Err(arithmetic_exception("overflow: too large to negate"));
    }
    Ok(alloc_fraction(-num, den))
}

fn invert(num: i32, den: i32) -> Result<Value, NativeError> {
    if num == 0 {
        return Err(arithmetic_exception("Unable to invert zero."));
    }
    if num == i32::MIN {
        return Err(arithmetic_exception("overflow: can't negate numerator"));
    }
    if num < 0 {
        Ok(alloc_fraction(-den, -num))
    } else {
        Ok(alloc_fraction(den, num))
    }
}

fn reduce(receiver: &Value, num: i32, den: i32) -> Result<Value, NativeError> {
    if num == 0 {
        return Ok(if den == 1 {
            receiver.clone()
        } else {
            zero_constant()
        });
    }
    let gcd = greatest_common_divisor(num.wrapping_abs(), den)?;
    if gcd == 1 {
        return Ok(receiver.clone());
    }
    get_fraction_int_int(num / gcd, den / gcd)
}

fn add_sub(
    receiver: &Value,
    num1: i32,
    den1: i32,
    other: &Value,
    is_add: bool,
) -> Option<Result<Value, NativeError>> {
    if matches!(other, Value::Ref(None)) {
        return Some(Err(npe(Some("fraction"))));
    }
    let (num2, den2) = fraction_parts(other)?;
    Some(add_sub_parts(
        receiver, num1, den1, other, num2, den2, is_add,
    ))
}

fn add_sub_parts(
    receiver: &Value,
    num1: i32,
    den1: i32,
    other: &Value,
    num2: i32,
    den2: i32,
    is_add: bool,
) -> Result<Value, NativeError> {
    if num1 == 0 {
        return if is_add {
            Ok(other.clone())
        } else {
            negate(num2, den2)
        };
    }
    if num2 == 0 {
        return Ok(receiver.clone());
    }
    let d1 = greatest_common_divisor(den1, den2)?;
    if d1 == 1 {
        let uvp = mul_and_check(num1, den2)?;
        let upv = mul_and_check(num2, den1)?;
        let numer = if is_add {
            add_and_check(uvp, upv)?
        } else {
            sub_and_check(uvp, upv)?
        };
        let denom = mul_pos_and_check(den1, den2)?;
        return Ok(alloc_fraction(numer, denom));
    }
    let uvp = i128::from(num1) * i128::from(den2 / d1);
    let upv = i128::from(num2) * i128::from(den1 / d1);
    let t = if is_add { uvp + upv } else { uvp - upv };
    let tmodd1 = t.rem_euclid(i128::from(d1)) as i32;
    let d2 = if tmodd1 == 0 {
        d1
    } else {
        greatest_common_divisor(tmodd1, d1)?
    };
    let w = t / i128::from(d2);
    if w < i128::from(i32::MIN) || w > i128::from(i32::MAX) {
        return Err(arithmetic_exception(
            "overflow: numerator too large after multiply",
        ));
    }
    let denom = mul_pos_and_check(den1 / d1, den2 / d2)?;
    Ok(alloc_fraction(w as i32, denom))
}

fn multiply_by(num1: i32, den1: i32, other: &Value) -> Option<Result<Value, NativeError>> {
    if matches!(other, Value::Ref(None)) {
        return Some(Err(npe(Some("fraction"))));
    }
    let (num2, den2) = fraction_parts(other)?;
    Some(multiply_by_parts(num1, den1, num2, den2))
}

fn multiply_by_parts(num1: i32, den1: i32, num2: i32, den2: i32) -> Result<Value, NativeError> {
    if num1 == 0 || num2 == 0 {
        return Ok(zero_constant());
    }
    let d1 = greatest_common_divisor(num1, den2)?;
    let d2 = greatest_common_divisor(num2, den1)?;
    let numer = mul_and_check(num1 / d1, num2 / d2)?;
    let denom = mul_pos_and_check(den1 / d2, den2 / d1)?;
    get_reduced_fraction(numer, denom)
}

fn divide_by(num1: i32, den1: i32, other: &Value) -> Option<Result<Value, NativeError>> {
    if matches!(other, Value::Ref(None)) {
        return Some(Err(npe(Some("fraction"))));
    }
    let (num2, den2) = fraction_parts(other)?;
    Some(divide_by_parts(num1, den1, num2, den2))
}

fn divide_by_parts(num1: i32, den1: i32, num2: i32, den2: i32) -> Result<Value, NativeError> {
    if num2 == 0 {
        return Err(arithmetic_exception(
            "The fraction to divide by must not be zero",
        ));
    }
    let inv = invert(num2, den2)?;
    let (inv_num, inv_den) = fraction_parts(&inv).unwrap();
    multiply_by_parts(num1, den1, inv_num, inv_den)
}

fn pow(receiver: &Value, num: i32, den: i32, power: i32) -> Result<Value, NativeError> {
    if power == 1 {
        return Ok(receiver.clone());
    }
    if power == 0 {
        return Ok(one_constant());
    }
    if power < 0 {
        let inv = invert(num, den)?;
        let (in_num, in_den) = fraction_parts(&inv).unwrap();
        if power == i32::MIN {
            let sq = pow(&inv, in_num, in_den, 2)?;
            let (sq_num, sq_den) = fraction_parts(&sq).unwrap();
            return pow(&sq, sq_num, sq_den, -(power / 2));
        }
        return pow(&inv, in_num, in_den, -power);
    }
    let f = multiply_by_parts(num, den, num, den)?;
    let (fn_num, fn_den) = fraction_parts(&f).unwrap();
    if power % 2 == 0 {
        pow(&f, fn_num, fn_den, power / 2)
    } else {
        let half = pow(&f, fn_num, fn_den, power / 2)?;
        let (h_num, h_den) = fraction_parts(&half).unwrap();
        multiply_by_parts(h_num, h_den, num, den)
    }
}

fn proper_numerator(num: i32, den: i32) -> i32 {
    num.wrapping_rem(den).wrapping_abs()
}

fn proper_whole(num: i32, den: i32) -> i32 {
    num.wrapping_div(den)
}

fn to_proper_string(num: i32, den: i32) -> String {
    if num == 0 {
        return "0".to_owned();
    }
    if num == den {
        return "1".to_owned();
    }
    if num == (-1_i32).wrapping_mul(den) {
        return "-1".to_owned();
    }
    let neg_num = if num > 0 { -num } else { num };
    if neg_num < -den {
        let proper = proper_numerator(num, den);
        if proper == 0 {
            proper_whole(num, den).to_string()
        } else {
            format!("{} {proper}/{den}", proper_whole(num, den))
        }
    } else {
        format!("{num}/{den}")
    }
}

fn hash_code_of(num: i32, den: i32) -> i32 {
    31_i32.wrapping_add(den).wrapping_mul(31).wrapping_add(num)
}

fn compare_parts(num1: i32, den1: i32, num2: i32, den2: i32) -> i32 {
    if num1 == num2 && den1 == den2 {
        return 0;
    }
    let first = i64::from(num1) * i64::from(den2);
    let second = i64::from(num2) * i64::from(den1);
    match first.cmp(&second) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner != FRACTION || r.descriptor != format!("L{FRACTION};") {
        return None;
    }
    CONSTANTS
        .iter()
        .find(|(name, _, _)| *name == r.name)
        .map(|(name, num, den)| constant_value(name, *num, *den))
}

pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner != FRACTION {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str(), args) {
        (
            "getFraction",
            "(II)Lorg/apache/commons/lang3/math/Fraction;",
            [Value::Int(num), Value::Int(den)],
        ) => Some(get_fraction_int_int(*num, *den)),
        (
            "getFraction",
            "(III)Lorg/apache/commons/lang3/math/Fraction;",
            [Value::Int(whole), Value::Int(num), Value::Int(den)],
        ) => Some(get_fraction_whole_num_den(*whole, *num, *den)),
        ("getFraction", "(D)Lorg/apache/commons/lang3/math/Fraction;", [Value::Float(value)]) => {
            Some(get_fraction_double(*value))
        }
        ("getFraction", "(Ljava/lang/String;)Lorg/apache/commons/lang3/math/Fraction;", [text]) => {
            get_fraction_string(text)
        }
        (
            "getReducedFraction",
            "(II)Lorg/apache/commons/lang3/math/Fraction;",
            [Value::Int(num), Value::Int(den)],
        ) => Some(get_reduced_fraction(*num, *den)),
        _ => None,
    }
}

pub(crate) fn instance_call(
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let (num, den) = fraction_parts(receiver)?;
    if !matches!(r.owner.as_str(), FRACTION | NUMBER | COMPARABLE | OBJECT) {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str(), args) {
        ("getNumerator", "()I", []) => Some(Ok(Value::Int(num))),
        ("getDenominator", "()I", []) => Some(Ok(Value::Int(den))),
        ("getProperNumerator", "()I", []) => Some(Ok(Value::Int(proper_numerator(num, den)))),
        ("getProperWhole", "()I", []) => Some(Ok(Value::Int(proper_whole(num, den)))),
        ("intValue", "()I", []) => Some(Ok(Value::Int(num.wrapping_div(den)))),
        ("longValue", "()J", []) => Some(Ok(Value::Long(i64::from(num) / i64::from(den)))),
        ("floatValue", "()F", []) => Some(Ok(Value::Float(f64::from((num as f32) / (den as f32))))),
        ("doubleValue", "()D", []) => Some(Ok(Value::Float(f64::from(num) / f64::from(den)))),
        ("byteValue", "()B", []) => Some(Ok(Value::Int(i32::from(num.wrapping_div(den) as i8)))),
        ("shortValue", "()S", []) => Some(Ok(Value::Int(i32::from(num.wrapping_div(den) as i16)))),
        ("abs", "()Lorg/apache/commons/lang3/math/Fraction;", []) => {
            if num >= 0 {
                Some(Ok(receiver.clone()))
            } else {
                Some(negate(num, den))
            }
        }
        ("negate", "()Lorg/apache/commons/lang3/math/Fraction;", []) => Some(negate(num, den)),
        ("invert", "()Lorg/apache/commons/lang3/math/Fraction;", []) => Some(invert(num, den)),
        ("reduce", "()Lorg/apache/commons/lang3/math/Fraction;", []) => {
            Some(reduce(receiver, num, den))
        }
        ("pow", "(I)Lorg/apache/commons/lang3/math/Fraction;", [Value::Int(power)]) => {
            Some(pow(receiver, num, den, *power))
        }
        (
            "add",
            "(Lorg/apache/commons/lang3/math/Fraction;)Lorg/apache/commons/lang3/math/Fraction;",
            [other],
        ) => add_sub(receiver, num, den, other, true),
        (
            "subtract",
            "(Lorg/apache/commons/lang3/math/Fraction;)Lorg/apache/commons/lang3/math/Fraction;",
            [other],
        ) => add_sub(receiver, num, den, other, false),
        (
            "multiplyBy",
            "(Lorg/apache/commons/lang3/math/Fraction;)Lorg/apache/commons/lang3/math/Fraction;",
            [other],
        ) => multiply_by(num, den, other),
        (
            "divideBy",
            "(Lorg/apache/commons/lang3/math/Fraction;)Lorg/apache/commons/lang3/math/Fraction;",
            [other],
        ) => divide_by(num, den, other),
        ("compareTo", "(Lorg/apache/commons/lang3/math/Fraction;)I", [other])
        | ("compareTo", "(Ljava/lang/Object;)I", [other]) => {
            if matches!(other, Value::Ref(None)) {
                return Some(Err(npe(Some(
                    "Cannot read field \"numerator\" because \"other\" is null",
                ))));
            }
            if let Some((num2, den2)) = fraction_parts(other) {
                Some(Ok(Value::Int(compare_parts(num, den, num2, den2))))
            } else {
                let class = value_class_name(other)?;
                Some(Err(crate::runtime::checkcast::class_cast_exception(class, FRACTION)))
            }
        }
        ("toString", "()Ljava/lang/String;", []) => {
            Some(Ok(Value::Ref(Some(format!("{num}/{den}")))))
        }
        ("toProperString", "()Ljava/lang/String;", []) => {
            Some(Ok(Value::Ref(Some(to_proper_string(num, den)))))
        }
        ("hashCode", "()I", []) => Some(Ok(Value::Int(hash_code_of(num, den)))),
        ("equals", "(Ljava/lang/Object;)Z", [other]) => {
            if matches!(other, Value::Ref(None)) {
                return Some(Ok(Value::Int(0)));
            }
            if let Some((n2, d2)) = fraction_parts(other) {
                return Some(Ok(Value::Int(i32::from(n2 == num && d2 == den))));
            }
            if value_class_name(other).is_some() {
                Some(Ok(Value::Int(0)))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn fraction_class(_: &str) -> &str {
    FRACTION
}

fn fraction_to_string(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(parse_key(handle.key()).map(|(num, den)| format!("{num}/{den}")))
}

fn fraction_hash_code(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(parse_key(handle.key()).map(|(num, den)| hash_code_of(num, den)))
}

fn fraction_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some((num1, den1)) = parse_key(handle.key()) else {
        return Ok(None);
    };
    if matches!(other, Value::Ref(None)) {
        return Ok(Some(false));
    }
    if let Some((num2, den2)) = fraction_parts(other) {
        return Ok(Some(num1 == num2 && den1 == den2));
    }
    Ok(value_class_name(other).map(|_| false))
}

fn fraction_compare_to(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let Some((num1, den1)) = parse_key(handle.key()) else {
        return Ok(None);
    };
    if matches!(other, Value::Ref(None)) {
        return Err(npe(Some(
            "Cannot read field \"numerator\" because \"other\" is null",
        )));
    }
    if let Some((num2, den2)) = fraction_parts(other) {
        return Ok(Some(compare_parts(num1, den1, num2, den2)));
    }
    match value_class_name(other) {
        Some(class) => Err(crate::runtime::checkcast::class_cast_exception(class, FRACTION)),
        None => Ok(None),
    }
}

fn fraction_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if [FRACTION, NUMBER, COMPARABLE, SERIALIZABLE, OBJECT].contains(&wanted)
        || hierarchy.is_instance(class, wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

pub(crate) const METHODS: Methods = Methods {
    to_string: fraction_to_string,
    hash_code: fraction_hash_code,
    equals: fraction_equals,
    instance_of: |_, wanted, hierarchy| fraction_instance_of(FRACTION, wanted, hierarchy),
    crosses_jars: |_, _| true,
    compare_to: Some(fraction_compare_to),
};

pub(crate) const CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == FRACTION,
    instance_of: fraction_instance_of,
    superclass: |_| Some(NUMBER.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![FRACTION.to_owned()],
};

//! Exact, bounded Apache Commons Lang operations reached by the 26.3 census.
//
// These are pure calls whose answers do not depend on a host filesystem or
// clock. A shape outside the table remains counted; this module never turns
// an unknown CharSequence or surrogate into a guessed String.

use crate::{
    Host, MethodRef, NativeError, Runtime, StdHost, Value, expect_ref,
    host_ref::{HostKind, HostRef},
};

const STRING_UTILS: &str = "org/apache/commons/lang3/StringUtils";
const CONSUMERS: &str = "org/apache/commons/lang3/function/Consumers";
const VALIDATE: &str = "org/apache/commons/lang3/Validate";

pub(crate) fn consumer_types(_: &str) -> &'static [&'static str] {
    &["java/util/function/Consumer"]
}

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == CONSUMERS && r.name == "nop" && r.descriptor == "()Ljava/util/function/Consumer;"
    {
        return Some(Ok(Value::Host(HostRef::new(
            HostKind::CommonsNop,
            "commons-lang3/Consumers.nop",
        ))));
    }

    if r.owner == VALIDATE {
        return validate_static(host, r, args);
    }

    if r.owner != STRING_UTILS {
        return None;
    }
    Some(match (r.name.as_str(), r.descriptor.as_str(), args) {
        ("isNotEmpty", "(Ljava/lang/CharSequence;)Z", [Value::Ref(None)]) => Ok(Value::Int(0)),
        ("isNotEmpty", "(Ljava/lang/CharSequence;)Z", [Value::Ref(Some(text))]) => {
            Ok(Value::Int((!text.is_empty()) as i32))
        }
        ("capitalize", "(Ljava/lang/String;)Ljava/lang/String;", [value]) => {
            string_result(capitalize(value))
        }
        ("rightPad", "(Ljava/lang/String;I)Ljava/lang/String;", [value, width]) => {
            string_result(right_pad(value, width, " "))
        }
        ("rightPad", "(Ljava/lang/String;IC)Ljava/lang/String;", [value, width, pad]) => {
            let Some(pad) = char_arg(pad) else {
                return Some(host.unknown_host_call(r));
            };
            string_result(right_pad(value, width, &pad.to_string()))
        }
        (
            "rightPad",
            "(Ljava/lang/String;ILjava/lang/String;)Ljava/lang/String;",
            [value, width, pad],
        ) => {
            let Some(pad) = expect_ref(pad) else {
                return Some(host.unknown_host_call(r));
            };
            string_result(right_pad(value, width, pad))
        }
        ("leftPad", "(Ljava/lang/String;I)Ljava/lang/String;", [value, width]) => {
            string_result(left_pad(value, width, " "))
        }
        ("leftPad", "(Ljava/lang/String;IC)Ljava/lang/String;", [value, width, pad]) => {
            let Some(pad) = char_arg(pad) else {
                return Some(host.unknown_host_call(r));
            };
            string_result(left_pad(value, width, &pad.to_string()))
        }
        (
            "leftPad",
            "(Ljava/lang/String;ILjava/lang/String;)Ljava/lang/String;",
            [value, width, pad],
        ) => {
            let Some(pad) = expect_ref(pad) else {
                return Some(host.unknown_host_call(r));
            };
            string_result(left_pad(value, width, pad))
        }
        ("repeat", "(Ljava/lang/String;I)Ljava/lang/String;", [value, count]) => {
            let Some(value) = expect_ref(value) else {
                return Some(Ok(Value::Ref(None)));
            };
            let Some(count) = int_arg(count) else {
                return Some(host.unknown_host_call(r));
            };
            Ok(Value::Ref(Some(if count <= 0 {
                String::new()
            } else {
                value.repeat(count as usize)
            })))
        }
        ("repeat", "(CI)Ljava/lang/String;", [unit, count]) => {
            let Some(unit) = char_arg(unit) else {
                return Some(host.unknown_host_call(r));
            };
            let Some(count) = int_arg(count) else {
                return Some(host.unknown_host_call(r));
            };
            Ok(Value::Ref(Some(if count <= 0 {
                String::new()
            } else {
                unit.to_string().repeat(count as usize)
            })))
        }
        (
            "containsIgnoreCase",
            "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Z",
            [haystack, needle],
        ) => Ok(Value::Int(contains_ignore_case(haystack, needle) as i32)),
        (
            "defaultIfBlank",
            "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/lang/CharSequence;",
            [value, fallback],
        ) => Ok(if is_blank(value) {
            fallback.clone()
        } else {
            value.clone()
        }),
        _ => host.unknown_host_call(r),
    })
}

pub(crate) fn runtime_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if r.owner == "java/util/function/Consumer"
        && r.name == "accept"
        && r.descriptor == "(Ljava/lang/Object;)V"
        && receiver.is_some_and(
            |value| matches!(value, Value::Host(handle) if handle.kind() == HostKind::CommonsNop),
        )
    {
        return Ok(Some(Value::Void));
    }
    if receiver.is_some() {
        return Ok(None);
    }
    if r.owner == VALIDATE
        && r.name == "isTrue"
        && r.descriptor == "(ZLjava/lang/String;[Ljava/lang/Object;)V"
    {
        let Some(Value::Int(condition)) = args.first() else {
            return Ok(None);
        };
        if *condition != 0 {
            return Ok(Some(Value::Void));
        }
        let Some(Value::Ref(Some(_))) = args.get(1) else {
            return Err(NativeError::UncaughtException {
                class: "java/lang/NullPointerException".into(),
                message: None,
                frame: None,
            });
        };
        let Some(values) = args.get(2) else {
            return Ok(None);
        };
        let Some(format) = args.get(1) else {
            return Ok(None);
        };
        let Some(message) = runtime.java_format(None, format, values) else {
            return Ok(None);
        };
        return Err(invalid(message));
    }
    Ok(None)
}

fn validate_static(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Some(Value::Int(condition)) = args.first() else {
        return Some(host.unknown_host_call(r));
    };
    if *condition != 0 {
        return Some(Ok(Value::Void));
    }
    match r.descriptor.as_str() {
        "(Z)V" => Some(Err(invalid("The validated expression is false".to_owned()))),
        _ => None,
    }
}

fn invalid(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(message),
        frame: None,
    }
}

fn string_result(value: Option<String>) -> Result<Value, NativeError> {
    Ok(Value::Ref(value))
}

fn int_arg(value: &Value) -> Option<i32> {
    match value {
        Value::Int(value) => Some(*value),
        _ => None,
    }
}

fn char_arg(value: &Value) -> Option<char> {
    let unit = int_arg(value)?;
    char::from_u32(unit as u32)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn capitalize(value: &Value) -> Option<String> {
    let text = expect_ref(value)?;
    if text.is_empty() {
        return Some(text.clone());
    }
    let mut chars = text.chars();
    let first = chars.next()?;
    let title = title_case(first)?;
    if title == first {
        return Some(text.clone());
    }
    Some(format!("{title}{}", chars.collect::<String>()))
}

fn title_case(value: char) -> Option<char> {
    let mapped = match value {
        'ǆ' => return Some('ǅ'),
        'ǉ' => return Some('ǈ'),
        'ǌ' => return Some('ǋ'),
        'ǳ' => return Some('ǲ'),
        _ => value,
    };
    let mut upper = mapped.to_uppercase();
    let first = upper.next()?;
    if upper.next().is_some() {
        Some(value)
    } else {
        Some(first)
    }
}

fn right_pad(value: &Value, width: &Value, pad: &str) -> Option<String> {
    let text = expect_ref(value)?;
    let width = int_arg(width)?;
    let length = utf16_len(text);
    if width <= length as i32 {
        return Some(text.clone());
    }
    let needed = width as usize - length;
    Some(format!("{text}{}", repeat_utf16(pad, needed)?))
}

fn left_pad(value: &Value, width: &Value, pad: &str) -> Option<String> {
    let text = expect_ref(value)?;
    let width = int_arg(width)?;
    let length = utf16_len(text);
    if width <= length as i32 {
        return Some(text.clone());
    }
    let needed = width as usize - length;
    Some(format!("{}{text}", repeat_utf16(pad, needed)?))
}

fn repeat_utf16(pattern: &str, needed: usize) -> Option<String> {
    if pattern.is_empty() {
        return Some(String::new());
    }
    let units = pattern.encode_utf16().collect::<Vec<_>>();
    let mut out = Vec::new();
    while out.len() < needed {
        let take = (needed - out.len()).min(units.len());
        out.extend_from_slice(&units[..take]);
    }
    String::from_utf16(&out).ok()
}

fn contains_ignore_case(haystack: &Value, needle: &Value) -> bool {
    let Some(haystack) = expect_ref(haystack) else {
        return false;
    };
    let Some(needle) = expect_ref(needle) else {
        return false;
    };
    if needle.is_empty() {
        return true;
    }
    let needle_lower = needle.to_lowercase();
    haystack.char_indices().any(|(start, _)| {
        haystack[start..]
            .chars()
            .take(needle.chars().count())
            .collect::<String>()
            .to_lowercase()
            == needle_lower
    })
}

fn is_blank(value: &Value) -> bool {
    let Some(text) = expect_ref(value) else {
        return true;
    };
    text.chars().all(java_whitespace)
}

fn java_whitespace(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'..='\u{000d}'
            | '\u{001c}'..='\u{0020}'
            | '\u{1680}'
            | '\u{2000}'..='\u{2006}'
            | '\u{2008}'..='\u{200a}'
            | '\u{2028}'..='\u{2029}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

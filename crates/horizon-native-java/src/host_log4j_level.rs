//! Canonical Log4j 2 `Level` objects and their deterministic API.

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{MethodRef, NativeError, Value, expect_ref};

const OWNER: &str = "org/apache/logging/log4j/Level";
const LEVEL_DESC: &str = "Lorg/apache/logging/log4j/Level;";
const LEVELS: &[(&str, i32)] = &[
    ("OFF", 0),
    ("FATAL", 100),
    ("ERROR", 200),
    ("WARN", 300),
    ("INFO", 400),
    ("DEBUG", 500),
    ("TRACE", 600),
    ("ALL", i32::MAX),
];

fn level_value(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Log4jLevel, name))
}

fn level_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Log4jLevel => LEVELS
            .iter()
            .find(|(name, _)| *name == handle.key())
            .map(|(name, _)| *name),
        _ => None,
    }
}

fn level_int(name: &str) -> Option<i32> {
    LEVELS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, value)| *value)
}

fn lookup(name: &str) -> Option<&'static str> {
    let normalized = name.trim().to_uppercase();
    LEVELS
        .iter()
        .find(|(known, _)| *known == normalized)
        .map(|(known, _)| *known)
}

fn illegal(message: impl Into<String>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn null_pointer(message: impl Into<String>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn level_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(level_name(&Value::Host(handle.clone())).map(str::to_owned))
}

fn level_hash(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(level_name(&Value::Host(handle.clone())).map(|text| {
        text.encode_utf16().fold(0i32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i32::from(unit))
        })
    }))
}

fn level_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(Some(matches!(
        other,
        Value::Host(other)
            if other.kind() == HostKind::Log4jLevel && other.key() == handle.key()
    )))
}

fn level_compare(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let Some(left) = level_name(&Value::Host(handle.clone())).and_then(level_int) else {
        return Ok(None);
    };
    let Some(right) = level_name(other).and_then(level_int) else {
        return match other {
            Value::Ref(None) => Err(null_pointer(
                "Cannot invoke \"org.apache.logging.log4j.Level.intLevel()\" because \"other\" is null",
            )),
            _ => Ok(None),
        };
    };
    Ok(Some(match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}

pub(crate) const METHODS: Methods = Methods {
    to_string: level_text,
    hash_code: level_hash,
    equals: level_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: Some(level_compare),
};

pub(crate) fn static_field(owner: &str, name: &str) -> Option<Value> {
    (owner == OWNER)
        .then(|| lookup(name).map(level_value))
        .flatten()
}

pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner != OWNER {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("getLevel", "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;") => {
            let Some(Value::Ref(Some(raw))) = args.first() else {
                return Some(Err(illegal("Illegal null or empty Level name.")));
            };
            if raw.is_empty() {
                return Some(Err(illegal("Illegal null or empty Level name.")));
            }
            Some(Ok(lookup(raw).map_or(Value::Ref(None), level_value)))
        }
        ("valueOf", "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;") => {
            let Some(Value::Ref(Some(raw))) = args.first() else {
                return Some(Err(null_pointer("No level name given.")));
            };
            Some(match lookup(raw) {
                Some(name) => Ok(level_value(name)),
                None => Err(illegal(format!(
                    "Unknown level constant [{}].",
                    raw.trim().to_uppercase()
                ))),
            })
        }
        ("toLevel", "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;") => {
            let value = args
                .first()
                .and_then(expect_ref)
                .and_then(|raw| lookup(raw));
            Some(Ok(value.map_or_else(|| level_value("DEBUG"), level_value)))
        }
        ("toLevel", descriptor)
            if descriptor == format!("(Ljava/lang/String;{LEVEL_DESC}){LEVEL_DESC}") =>
        {
            let value = args
                .first()
                .and_then(expect_ref)
                .and_then(|raw| lookup(raw));
            Some(Ok(value.map_or_else(
                || args.get(1).cloned().unwrap_or_else(|| level_value("DEBUG")),
                level_value,
            )))
        }
        _ => None,
    }
}

pub(crate) fn instance_call(
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let name = level_name(receiver)?;
    let value = level_int(name)?;
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("intLevel", "()I") => Some(Ok(Value::Int(value))),
        ("name" | "toString", "()Ljava/lang/String;") => {
            Some(Ok(Value::Ref(Some(name.to_owned()))))
        }
        ("isMoreSpecificThan", descriptor) if descriptor == format!("({LEVEL_DESC})Z") => {
            let other = match level_argument(args)? {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            Some(Ok(Value::Int(i32::from(value <= other))))
        }
        ("isLessSpecificThan", descriptor) if descriptor == format!("({LEVEL_DESC})Z") => {
            let other = match level_argument(args)? {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            Some(Ok(Value::Int(i32::from(value >= other))))
        }
        ("isInRange", descriptor) if descriptor == format!("({LEVEL_DESC}{LEVEL_DESC})Z") => {
            let low = match level_argument(args)? {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            let high = match level_argument(args.get(1..).unwrap_or_default())? {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            Some(Ok(Value::Int(i32::from(value >= low && value <= high))))
        }
        _ => None,
    }
}

fn level_argument(args: &[Value]) -> Option<Result<i32, NativeError>> {
    match args.first() {
        Some(Value::Ref(None)) | None => Some(Err(null_pointer("Level argument is null"))),
        Some(value) => level_name(value).and_then(level_int).map(Ok),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::Ref(Some(value.into()))
    }

    fn method(name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: OWNER.into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    #[test]
    fn lookup_defaults_and_errors_match_log4j() {
        let get = method(
            "getLevel",
            "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;",
        );
        assert_eq!(
            static_call(&get, &[text("missing")]).unwrap().unwrap(),
            Value::Ref(None)
        );
        assert!(matches!(
            static_call(&get, &[Value::Ref(None)]),
            Some(Err(NativeError::UncaughtException { class, .. }))
                if class == "java/lang/IllegalArgumentException"
        ));
        let value_of = method(
            "valueOf",
            "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;",
        );
        assert!(matches!(
            static_call(&value_of, &[text("missing")]),
            Some(Err(NativeError::UncaughtException { class, .. }))
                if class == "java/lang/IllegalArgumentException"
        ));
        let to = method(
            "toLevel",
            "(Ljava/lang/String;)Lorg/apache/logging/log4j/Level;",
        );
        assert_eq!(
            static_call(&to, &[text("missing")]).unwrap().unwrap(),
            static_field(OWNER, "DEBUG").unwrap()
        );
    }

    #[test]
    fn standard_levels_have_exact_names_and_int_levels() {
        for (name, int) in LEVELS {
            let value = static_field(OWNER, name).unwrap();
            assert_eq!(
                instance_call(&method("intLevel", "()I"), &value, &[])
                    .unwrap()
                    .unwrap(),
                Value::Int(*int)
            );
            assert_eq!(
                instance_call(&method("name", "()Ljava/lang/String;"), &value, &[])
                    .unwrap()
                    .unwrap(),
                text(name)
            );
        }
    }
}

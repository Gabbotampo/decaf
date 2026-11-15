//! The host's map entries: a `HashMap`/`LinkedHashMap` node, a `TreeMap`
//! entry and `Map.entry(..)`'s `KeyValueHolder`, as [`HostKind::MapEntry`]
//! handles keyed by their key and value (#261 re-review: as `map-entry/k,v`
//! text an entry holding a lambda handed its key to any jar that cast it to
//! `CharSequence`, and a jar `String` spelled like one was an entry).
//!
//! `toString()` is `key + "=" + value`, which is `String.valueOf(key) + "=" +
//! String.valueOf(value)`: a jar key or value prints through its own
//! `toString` (#70), a box or string as Java does. `hashCode()` is
//! `Objects.hashCode(key) ^ Objects.hashCode(value)`; `equals` compares an
//! entry's key and value with `Objects.equals`.

use super::*;
use crate::runtime::host_ref::{ClassHierarchy, HostKind, Methods, Nested};

/// A new entry of `key` and `value`.
pub(crate) fn entry(key: &Value, value: &Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::MapEntry,
        &[key.clone(), value.clone()],
    ))
}

/// An entry's key and value; `None` for any other value.
pub(crate) fn parts(value: &Value) -> Option<(Value, Value)> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::MapEntry => {
            match handle.values()?.as_slice() {
                [key, value] => Some((key.clone(), value.clone())),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The type an entry surely is.
pub(crate) fn entry_types(_: &str) -> &'static [&'static str] {
    &["java/util/Map$Entry"]
}

fn handle_parts(handle: &HostRef) -> Option<(Value, Value)> {
    parts(&Value::Host(handle.clone()))
}

fn entry_text(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    let Some((key, value)) = handle_parts(handle) else {
        return Ok(None);
    };
    let (Some(key), Some(value)) = (nested.text(&key)?, nested.text(&value)?) else {
        return Ok(None);
    };
    Ok(Some(format!("{key}={value}")))
}

fn entry_hash(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    let Some((key, value)) = handle_parts(handle) else {
        return Ok(None);
    };
    let (Some(key), Some(value)) = (nested.hash(&key)?, nested.hash(&value)?) else {
        return Ok(None);
    };
    Ok(Some(key ^ value))
}

/// `entry.equals(other)`: any `Map.Entry` with `Objects.equals` key and
/// value. Another entry handle is compared; a value that is surely no entry
/// (a `String`, a box, `null`) is not equal; anything else (a jar object
/// that may implement `Map.Entry`) is unknown.
fn entry_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let (Some((key, value)), Some((other_key, other_value))) = (handle_parts(handle), parts(other))
    else {
        return Ok(match other {
            Value::Ref(_) | Value::Boxed(_) | Value::Int(_) | Value::Long(_) | Value::Float(_) => {
                Some(false)
            }
            _ => None,
        });
    };
    Ok(match nested.equals(&key, &other_key)? {
        Some(true) => nested.equals(&value, &other_value)?,
        other => other,
    })
}

/// `entry instanceof wanted`. Every class a host entry may be
/// (`KeyValueHolder`, `HashMap$Node`, `LinkedHashMap$Entry`, `TreeMap$Entry`,
/// `AbstractMap$SimpleImmutableEntry`) is a JDK class implementing
/// `Map.Entry` and at most `Serializable`/`Cloneable`, and no JDK class is a
/// jar's or a game's type: so `Object` and `Map.Entry` are `true`, those two
/// are unknown, and every other type is an exact `false` (no entry is a
/// `CharSequence`).
fn entry_instance_of(_: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    match wanted {
        "java/lang/Object" | "java/util/Map$Entry" => Some(true),
        "java/io/Serializable" | "java/lang/Cloneable" => None,
        _ => Some(false),
    }
}

pub(crate) const METHODS: Methods = Methods {
    to_string: entry_text,
    hash_code: entry_hash,
    equals: entry_equals,
    instance_of: entry_instance_of,
    crosses_jars: |handle, crosses| {
        handle_parts(handle).is_some_and(|(key, value)| crosses(&key) && crosses(&value))
    },
    compare_to: None,
};

impl<H: Host> Runtime<'_, H> {
    /// `entry.toString()` for a host map entry; `None` for any other value.
    pub(crate) fn host_entry_text(
        &mut self,
        value: &Value,
        class: &str,
        method: &str,
    ) -> Result<Option<Value>, NativeError> {
        // Only a text that parses as an entry is one; a string that merely
        // starts like one stays a string.
        let Some((key, entry_value)) = parts(value) else {
            return Ok(None);
        };
        let key = self.value_of_text(&key, "Ljava/lang/Object;", class, method)?;
        let entry_value = self.value_of_text(&entry_value, "Ljava/lang/Object;", class, method)?;
        Ok(Some(Value::Ref(Some(format!("{key}={entry_value}")))))
    }
}

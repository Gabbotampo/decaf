//! `Holder.Reference` handles produced by a jar's own
//! `Registry.registerForHolder` calls.
//!
//! A registration carries the registry key, entry id and registered value. A
//! jar can only obtain one of these handles from the host call; spelling the
//! old marker text in a `String` never aliases it. Vanilla holders whose
//! registry contents or tags belong to the game stay outside this model.

use crate::runtime::dfu_either;
use crate::runtime::host_ids;
use crate::runtime::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, exact_class_rules_out};
use crate::runtime::{MethodRef, NativeError, StdHost, Value};

const OWNER: &str = "net/minecraft/core/Holder$Reference";
const HOLDER: &str = "net/minecraft/core/Holder";
const KIND: &str = "net/minecraft/core/Holder$Kind";
const HOLDER_TYPES: &[&str] = &[OWNER, HOLDER, "java/lang/Object"];

/// A holder reference is a concrete game object. Its object hash is the
/// identity hash and therefore remains counted; the identity itself is exact
/// because each registration key is one host-created allocation.
fn holder(registry_key: Value, id: Value, value: Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::RegistryHolder,
        &[registry_key, id, value],
    ))
}

struct Parts {
    registry_key: Value,
    id: String,
    value: Value,
}

fn parts(value: &Value) -> Option<Parts> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::RegistryHolder {
        return None;
    }
    let [registry_key, id, value] = <[Value; 3]>::try_from(handle.values()?).ok()?;
    let id = host_ids::identifier_key(&id)?.to_owned();
    host_ids::resource_key_parts(&registry_key)?;
    Some(Parts {
        registry_key,
        id,
        value,
    })
}

fn entry_key(parts: &Parts) -> Value {
    host_ids::resource_key(
        &parts.registry_key,
        &host_ids::identifier_of_text(&parts.id),
        Some(parts.id.clone()),
    )
}

fn registry_key_for_registry(registry: &Value) -> Option<Value> {
    let id = crate::runtime::host_registry::registry_id(registry)?;
    Some(host_ids::registry_key_of(id))
}

fn key_parts(key: &Value) -> Option<(Value, Value, String)> {
    let registry_key = host_ids::key_registry(key)?;
    let id = host_ids::key_location(key)?;
    let id_text = host_ids::identifier_key(&id)?.to_owned();
    Some((registry_key, id, id_text))
}

fn same_registry(registry: &Value, registry_key: &Value) -> bool {
    let Some(registry_id) = crate::runtime::host_registry::registry_id(registry) else {
        return false;
    };
    host_ids::registry_name(registry_key).as_deref() == Some(registry_id)
}

fn make_for_identifier(registry: &Value, id: &Value, value: &Value) -> Option<Value> {
    let registry_key = registry_key_for_registry(registry)?;
    let id_text = host_ids::identifier_key(id)?.to_owned();
    Some(holder(
        registry_key,
        host_ids::identifier_of_text(&id_text),
        value.clone(),
    ))
}

fn make_for_key(registry: &Value, key: &Value, value: &Value) -> Option<Value> {
    let (registry_key, id, _id_text) = key_parts(key)?;
    if !same_registry(registry, &registry_key) {
        return None;
    }
    Some(holder(registry_key, id, value.clone()))
}

/// `Registry.registerForHolder`'s two 26.3 overloads. An unmodelled registry,
/// id or value shape stays counted instead of inventing a holder.
pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/minecraft/core/Registry" || r.name != "registerForHolder" {
        return None;
    }
    let [registry, id_or_key, value] = args else {
        return Some(host.unknown_host_call(r));
    };
    let answer = match r.descriptor.as_str() {
        "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;" => {
            make_for_identifier(registry, id_or_key, value)
        }
        "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/ResourceKey;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;" => {
            make_for_key(registry, id_or_key, value)
        }
        _ => None,
    };
    Some(answer.map_or_else(|| host.unknown_host_call(r), Ok))
}

fn is_id(parts: &Parts, id: &Value) -> Option<bool> {
    Some(host_ids::identifier_key(id)? == parts.id)
}

fn is_key(parts: &Parts, key: &Value) -> Option<bool> {
    Some(entry_key(parts) == *key)
}

fn holder_to_string(
    handle: &HostRef,
    nested: &mut dyn Nested,
) -> Result<Option<String>, NativeError> {
    let value = Value::Host(handle.clone());
    let parts =
        parts(&value).ok_or_else(|| NativeError::Metadata("malformed registry holder".into()))?;
    let key = nested.text(&entry_key(&parts))?;
    let value = nested.text(&parts.value)?;
    Ok(key
        .zip(value)
        .map(|(key, value)| format!("Reference{{{key}={value}}}")))
}

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == HostKind::RegistryHolder => handle.same_object(other),
        _ => Some(false),
    })
}

fn instance_of(_handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if HOLDER_TYPES.contains(&wanted) || hierarchy.is_instance(OWNER, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(OWNER, wanted))
        .then_some(false)
}

pub(crate) const METHODS: Methods = Methods {
    to_string: holder_to_string,
    // `Holder.Reference` inherits `Object.hashCode`: an identity hash.
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of,
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let parts = parts(receiver)?;
    let answer = match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (HOLDER | OWNER, "key", "()Lnet/minecraft/resources/ResourceKey;") => Ok(entry_key(&parts)),
        (HOLDER | OWNER, "value", "()Ljava/lang/Object;") => Ok(parts.value.clone()),
        (HOLDER | OWNER, "getRegisteredName", "()Ljava/lang/String;") => {
            Ok(Value::Ref(Some(parts.id.clone())))
        }
        (HOLDER | OWNER, "getRegisteredNameIfPresent", "()Ljava/util/Optional;") => Ok(
            crate::runtime::host_optional::optional_wrap(Value::Ref(Some(parts.id.clone()))),
        ),
        (HOLDER | OWNER, "unwrapKey", "()Ljava/util/Optional;") => {
            Ok(crate::runtime::host_optional::optional_wrap(entry_key(&parts)))
        }
        (HOLDER | OWNER, "unwrap", "()Lcom/mojang/datafixers/util/Either;") => {
            Ok(dfu_either::left(&entry_key(&parts)))
        }
        (HOLDER | OWNER, "kind", "()Lnet/minecraft/core/Holder$Kind;") => {
            Ok(Value::Ref(Some(format!("enum/{KIND}/REFERENCE"))))
        }
        (HOLDER | OWNER, "isBound", "()Z") => Ok(Value::Int(1)),
        (HOLDER | OWNER, "areComponentsBound", "()Z") => Ok(Value::Int(0)),
        (HOLDER | OWNER, "is", "(Lnet/minecraft/resources/Identifier;)Z") => {
            let Some(id) = args.first().and_then(|value| is_id(&parts, value)) else {
                return Some(host.unknown_host_call(r));
            };
            Ok(Value::Int(i32::from(id)))
        }
        (HOLDER | OWNER, "is", "(Lnet/minecraft/resources/ResourceKey;)Z") => {
            let Some(key) = args.first().and_then(|value| is_key(&parts, value)) else {
                return Some(host.unknown_host_call(r));
            };
            Ok(Value::Int(i32::from(key)))
        }
        // Tags, predicates, holder comparisons and component state depend on
        // game-owned binding, so they remain counted by design.
        _ => return Some(host.unknown_host_call(r)),
    };
    Some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Host, MethodRef};

    fn method(name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: "net/minecraft/core/Registry".into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    #[test]
    fn register_for_holder_keeps_registry_id_value_and_forgery_separate() {
        let mut host = StdHost::new();
        let registry = crate::runtime::host_registry::registry("minecraft:item");
        let id = crate::runtime::host_ids::identifier("demo", "item");
        let holder = host
            .invoke(
                &method(
                    "registerForHolder",
                    "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;",
                ),
                &[registry, id, Value::Ref(Some("value".into()))],
            )
            .unwrap();
        assert!(
            matches!(&holder, Value::Host(handle) if handle.kind() == HostKind::RegistryHolder)
        );
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "getRegisteredName".into(),
                    descriptor: "()Ljava/lang/String;".into(),
                },
                &holder,
                &[],
            )
            .unwrap(),
            Value::Ref(Some("demo:item".into()))
        );
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "value".into(),
                    descriptor: "()Ljava/lang/Object;".into(),
                },
                &holder,
                &[],
            )
            .unwrap(),
            Value::Ref(Some("value".into()))
        );
        assert!(matches!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "is".into(),
                    descriptor: "(Lnet/minecraft/resources/Identifier;)Z".into(),
                },
                &holder,
                &[crate::runtime::host_ids::identifier("demo", "item")],
            )
            .unwrap(),
            Value::Int(1)
        ));
    }
}

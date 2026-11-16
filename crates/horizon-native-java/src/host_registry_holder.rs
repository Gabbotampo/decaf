//! `Holder.Reference` handles produced by a jar's own
//! `Registry.registerForHolder` calls.
//!
//! A registration carries the registry key, entry id, registered value and
//! binding state. MappedRegistry.register creates an unbound reference. Fabric
//! API's fabric-registry-sync-v0 mixin binds it immediately at register return;
//! NeoForge and vanilla leave it unbound until their real registry lifecycle.
//! A jar can only
//! obtain one of these handles from the host call; spelling the
//! old marker text in a `String` never aliases it. Vanilla holders whose
//! registry contents or tags belong to the game stay outside this model.

use crate::dfu_either;
use crate::host_ids;
use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, exact_class_rules_out};
use crate::{MethodRef, NativeError, StdHost, Value};

const OWNER: &str = "net/minecraft/core/Holder$Reference";
const HOLDER: &str = "net/minecraft/core/Holder";
const KIND: &str = "net/minecraft/core/Holder$Kind";
const HOLDER_TYPES: &[&str] = &[OWNER, HOLDER, "java/lang/Object"];

/// A holder reference is a concrete game object. Its object hash is the
/// identity hash and therefore remains counted; the identity itself is exact
/// because each registration key is one host-created allocation.
/// The bound flag starts false at registration; only a future real registry-freeze hook may set it true.
fn holder(registry_key: Value, id: Value, value: Value, bound: bool) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::RegistryHolder,
        &[registry_key, id, value, Value::Int(i32::from(bound))],
    ))
}

struct Parts {
    registry_key: Value,
    id: String,
    value: Value,
    bound: bool,
}

fn parts(value: &Value) -> Option<Parts> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::RegistryHolder {
        return None;
    }
    let [registry_key, id, value, bound] = <[Value; 4]>::try_from(handle.values()?).ok()?;
    let id = host_ids::identifier_key(&id)?.to_owned();
    host_ids::resource_key_parts(&registry_key)?;
    let bound = match bound {
        Value::Int(0) => false,
        Value::Int(1) => true,
        _ => return None,
    };
    Some(Parts {
        registry_key,
        id,
        value,
        bound,
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
    let id = crate::host_registry::registry_id(registry)?;
    Some(host_ids::registry_key_of(id))
}

fn key_parts(key: &Value) -> Option<(Value, Value, String)> {
    let registry_key = host_ids::key_registry(key)?;
    let id = host_ids::key_location(key)?;
    let id_text = host_ids::identifier_key(&id)?.to_owned();
    Some((registry_key, id, id_text))
}

fn same_registry(registry: &Value, registry_key: &Value) -> bool {
    let Some(registry_id) = crate::host_registry::registry_id(registry) else {
        return false;
    };
    host_ids::registry_name(registry_key).as_deref() == Some(registry_id)
}

fn make_for_identifier(registry: &Value, id: &Value, value: &Value, bound: bool) -> Option<Value> {
    let registry_key = registry_key_for_registry(registry)?;
    let id_text = host_ids::identifier_key(id)?.to_owned();
    Some(holder(
        registry_key,
        host_ids::identifier_of_text(&id_text),
        value.clone(),
        bound,
    ))
}

fn make_for_key(registry: &Value, key: &Value, value: &Value, bound: bool) -> Option<Value> {
    let (registry_key, id, _id_text) = key_parts(key)?;
    if !same_registry(registry, &registry_key) {
        return None;
    }
    Some(holder(registry_key, id, value.clone(), bound))
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
    // Fabric API's fabric-registry-sync-v0 MappedRegistryMixin injects at
    // register RETURN and calls Holder.Reference.bindValue(entry). This state
    // is exact only when that module is present in the explicitly linked run
    // metadata; vanilla, Fabric without it, and NeoForge stay unbound here.
    let fabric_registry_sync = host.run_loader == crate::class_precedence::RunLoader::Fabric
        && host.fabric_mod_present("fabric-registry-sync-v0");
    let answer = match r.descriptor.as_str() {
        "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;" => {
            make_for_identifier(registry, id_or_key, value, fabric_registry_sync)
        }
        "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/ResourceKey;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;" => {
            make_for_key(registry, id_or_key, value, fabric_registry_sync)
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
    let value = if parts.bound {
        nested.text(&parts.value)?
    } else {
        Some("null".into())
    };
    Ok(key
        .zip(value)
        .map(|(key, value)| format!("Reference{{{key}={value}}}")))
}

fn unbound_value(parts: &Parts) -> NativeError {
    let registry =
        host_ids::registry_name(&parts.registry_key).unwrap_or_else(|| "unknown:registry".into());
    let key = format!("ResourceKey[{registry} / {}]", parts.id);
    let owner = format!("Registry[ResourceKey[minecraft:root / {registry}] (Stable)]");
    NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: Some(format!(
            "Trying to access unbound value '{key}' from registry {owner}"
        )),
        frame: None,
    }
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
        (HOLDER | OWNER, "value", "()Ljava/lang/Object;") => {
            if parts.bound {
                Ok(parts.value.clone())
            } else {
                Err(unbound_value(&parts))
            }
        }
        (HOLDER | OWNER, "getRegisteredName", "()Ljava/lang/String;") => {
            Ok(Value::Ref(Some(parts.id.clone())))
        }
        (HOLDER | OWNER, "getRegisteredNameIfPresent", "()Ljava/util/Optional;") => Ok(
            crate::host_optional::optional_wrap(Value::Ref(Some(parts.id.clone()))),
        ),
        (HOLDER | OWNER, "unwrapKey", "()Ljava/util/Optional;") => {
            Ok(crate::host_optional::optional_wrap(entry_key(&parts)))
        }
        (HOLDER | OWNER, "unwrap", "()Lcom/mojang/datafixers/util/Either;") => {
            Ok(dfu_either::left(&entry_key(&parts)))
        }
        (HOLDER | OWNER, "kind", "()Lnet/minecraft/core/Holder$Kind;") => {
            Ok(Value::Ref(Some(format!("enum/{KIND}/REFERENCE"))))
        }
        (HOLDER | OWNER, "isBound", "()Z") => Ok(Value::Int(i32::from(parts.bound))),
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
    use crate::{Host, MethodRef};

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
        let registry = crate::host_registry::registry("minecraft:item");
        let id = crate::host_ids::identifier("demo", "item");
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
        let error = host
            .invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "value".into(),
                    descriptor: "()Ljava/lang/Object;".into(),
                },
                &holder,
                &[],
            )
            .unwrap_err();
        assert!(matches!(
            error,
            NativeError::UncaughtException {
                ref class,
                ref message,
                ..
            } if class == "java/lang/IllegalStateException"
                && message.as_deref().is_some_and(|text| text.contains(
                    "Trying to access unbound value 'ResourceKey[minecraft:item / demo:item]'"
                ))
        ));
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "isBound".into(),
                    descriptor: "()Z".into(),
                },
                &holder,
                &[],
            )
            .unwrap(),
            Value::Int(0)
        );
        assert!(matches!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "is".into(),
                    descriptor: "(Lnet/minecraft/resources/Identifier;)Z".into(),
                },
                &holder,
                &[crate::host_ids::identifier("demo", "item")],
            )
            .unwrap(),
            Value::Int(1)
        ));
    }

    #[test]
    fn fabric_registry_sync_binds_at_register_return() {
        let mut host = StdHost::new()
            .with_run_loader(crate::class_precedence::RunLoader::Fabric)
            .with_mod("fabric-registry-sync-v0", "8.0.4");
        let holder = host
            .invoke(
                &method(
                    "registerForHolder",
                    "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;",
                ),
                &[
                    crate::host_registry::registry("minecraft:item"),
                    crate::host_ids::identifier("demo", "item"),
                    Value::Ref(Some("value".into())),
                ],
            )
            .unwrap();
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
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "isBound".into(),
                    descriptor: "()Z".into(),
                },
                &holder,
                &[],
            )
            .unwrap(),
            Value::Int(1)
        );
    }

    #[test]
    fn fabric_without_registry_sync_stays_unbound() {
        let mut host = StdHost::new().with_run_loader(crate::class_precedence::RunLoader::Fabric);
        let holder = host
            .invoke(
                &method(
                    "registerForHolder",
                    "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;Ljava/lang/Object;)Lnet/minecraft/core/Holder$Reference;",
                ),
                &[
                    crate::host_registry::registry("minecraft:item"),
                    crate::host_ids::identifier("demo", "item"),
                    Value::Ref(Some("value".into())),
                ],
            )
            .unwrap();
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "isBound".into(),
                    descriptor: "()Z".into(),
                },
                &holder,
                &[],
            )
            .unwrap(),
            Value::Int(0)
        );
    }

    #[test]
    fn a_bound_reference_answers_value_and_prints_it() {
        let mut host = StdHost::new();
        let bound = holder(
            crate::host_ids::registry_key_of("minecraft:item"),
            crate::host_ids::identifier("demo", "item"),
            Value::Ref(Some("value".into())),
            true,
        );
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "value".into(),
                    descriptor: "()Ljava/lang/Object;".into(),
                },
                &bound,
                &[],
            )
            .unwrap(),
            Value::Ref(Some("value".into()))
        );
        assert_eq!(
            host.invoke_instance(
                &MethodRef {
                    owner: HOLDER.into(),
                    name: "isBound".into(),
                    descriptor: "()Z".into(),
                },
                &bound,
                &[],
            )
            .unwrap(),
            Value::Int(1)
        );
    }
}

//! Registration-only models for Fabric resource-loader v0/v1 and NeoForge's
//! client reload listener event.  The listener callbacks themselves never run.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::{Host, MethodRef, NativeError, Runtime, StdHost, Value};

const HELPER_API: &str = "net/fabricmc/fabric/api/resource/ResourceManagerHelper";
const CONVERTER: &str = "net/minecraft/resources/FileToIdConverter";
const PREFIX: &str = "\u{0}client-reload/";

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == handle.kind() => handle.same_object(other),
        _ => Some(false),
    })
}
fn converter_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == HostKind::FileToIdConverter => {
            Some(handle.values() == other.values())
        }
        _ => Some(false),
    })
}

pub(crate) const HELPER_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};
pub(crate) const CONVERTER_METHODS: Methods = Methods {
    to_string: |handle, _| {
        let Some((prefix, extension)) = parts(handle) else {
            return Ok(None);
        };
        Ok(Some(format!(
            "FileToIdConverter[prefix={prefix}, extension={extension}]"
        )))
    },
    hash_code: |handle, _| {
        Ok(parts(handle)
            .map(|(prefix, extension)| {
                Some(
                    crate::runtime::jdk_model::java_string_hash(&prefix)
                        .wrapping_mul(31)
                        .wrapping_add(crate::runtime::jdk_model::java_string_hash(&extension)),
                )
            })
            .unwrap_or(None))
    },
    equals: converter_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: None,
};

pub(crate) fn helper_types(_: &str) -> &'static [&'static str] {
    &[HELPER_API]
}
pub(crate) fn converter_class(_: &str) -> &str {
    CONVERTER
}

fn parts(handle: &HostRef) -> Option<(String, String)> {
    let values = handle.values()?;
    match values.as_slice() {
        [Value::Ref(Some(prefix)), Value::Ref(Some(extension))] => {
            Some((prefix.clone(), extension.clone()))
        }
        _ => None,
    }
}
fn pack_name(value: &Value) -> Option<&str> {
    let Value::Ref(Some(text)) = value else {
        return None;
    };
    text.strip_prefix("enum/net/minecraft/server/packs/PackType/")
}
fn helper(pack: &Value) -> Option<Value> {
    Some(Value::Host(HostRef::of_values(
        HostKind::FabricResourceManagerHelper,
        &[Value::Ref(Some(pack_name(pack)?.to_owned()))],
    )))
}
fn converter(prefix: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FileToIdConverter,
        &[
            Value::Ref(Some(prefix.to_owned())),
            Value::Ref(Some(".json".into())),
        ],
    ))
}
fn id_text(value: &Value) -> Option<&str> {
    crate::runtime::host_ids::identifier_key(value)
}
fn registration_id(value: &Value) -> Option<String> {
    id_text(value)
        .map(str::to_owned)
        .or_else(|| crate::runtime::jar_object::object_key(value).map(|key| format!("object/{key}")))
}
fn state_key(resource: &str, id: &str) -> String {
    format!("{PREFIX}{resource}/{id}")
}
fn duplicate(class: &str, message: String) -> NativeError {
    NativeError::UncaughtException {
        class: class.into(),
        message: Some(message),
        frame: None,
    }
}

pub(crate) fn static_field(owner: &str, name: &str, descriptor: &str) -> Option<Value> {
    if matches!(
        owner,
        "net/fabricmc/fabric/api/resource/v1/pack/PackActivationType"
            | "net/fabricmc/fabric/api/resource/ResourcePackActivationType"
    ) && matches!(name, "NORMAL" | "DEFAULT_ENABLED" | "ALWAYS_ENABLED")
        && descriptor == format!("L{owner};")
    {
        return Some(Value::Ref(Some(format!("enum/{owner}/{name}"))));
    }
    let text = match owner {
        "net/fabricmc/fabric/api/resource/v1/reloader/ResourceReloaderKeys" => match name {
            "BEFORE_VANILLA" => "fabric:before_vanilla",
            "AFTER_VANILLA" => "fabric:after_vanilla",
            _ => return None,
        },
        "net/fabricmc/fabric/api/resource/v1/reloader/ResourceReloaderKeys$Client" => match name {
            "BLOCK_ENTITY_RENDER_DISPATCHER" => "minecraft:block_entity_render_dispatcher",
            "CLOUD_RENDERER" => "minecraft:cloud_renderer",
            "EQUIPMENT_ASSETS" => "minecraft:equipment_assets",
            "ENTITY_RENDER_DISPATCHER" => "minecraft:entity_render_dispatcher",
            "DRY_FOLIAGE_COLOR" => "minecraft:dry_foliage_color",
            "FOLIAGE_COLOR" => "minecraft:foliage_color",
            "FONTS" => "minecraft:fonts",
            "GAME_RENDERER" => "minecraft:game_renderer",
            "GRASS_COLOR" => "minecraft:grass_color",
            "ATLAS" => "minecraft:atlas",
            "LANGUAGES" => "minecraft:languages",
            "MODELS" => "minecraft:models",
            "PARTICLES" => "minecraft:particles",
            "SHADERS" => "minecraft:shaders",
            "SOUNDS" => "minecraft:sounds",
            "SPLASH_TEXTS" => "minecraft:splash_texts",
            "TEXTURES" => "minecraft:textures",
            "WAYPOINT_STYLE" => "minecraft:waypoint_style",
            _ => return None,
        },
        _ => return None,
    };
    (descriptor == "Lnet/minecraft/resources/Identifier;")
        .then(|| crate::runtime::host_ids::identifier_of_text(text))
}

pub(crate) fn static_call(
    _host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            "net/fabricmc/fabric/api/resource/ResourceManagerHelper",
            "get",
            "(Lnet/minecraft/server/packs/PackType;)Lnet/fabricmc/fabric/api/resource/ResourceManagerHelper;",
        ) => Some(
            helper(args.first()?)
                .map(Ok)
                .unwrap_or_else(|| Err(NativeError::UnknownHostCall(r.clone()))),
        ),
        (
            "net/fabricmc/fabric/api/resource/ResourceManagerHelper",
            "registerBuiltinResourcePack",
            _,
        ) => Some(Ok(Value::Int(1))),
        ("net/fabricmc/fabric/api/resource/v1/ResourceLoader", "registerBuiltinPack", _) => {
            Some(Ok(Value::Int(1)))
        }
        (
            "net/fabricmc/fabric/api/resource/v1/DataResourceLoader",
            "get",
            "()Lnet/fabricmc/fabric/api/resource/v1/DataResourceLoader;",
        ) => Some(Ok(Value::Host(HostRef::of_values(
            HostKind::FabricResourceLoader,
            &[Value::Ref(Some("DATA_RESOURCE".into()))],
        )))),
        (
            "net/minecraft/resources/FileToIdConverter",
            "json",
            "(Ljava/lang/String;)Lnet/minecraft/resources/FileToIdConverter;",
        ) => Some(Ok(converter(crate::runtime::expect_string(args.first()?)))),
        _ => None,
    }
}

fn id_transform(handle: &HostRef, id: &Value, to_file: bool) -> Option<Value> {
    let (prefix, extension) = parts(handle)?;
    let id = id_text(id)?;
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let path = if to_file {
        format!("{prefix}/{path}{extension}")
    } else {
        path.strip_prefix(&format!("{prefix}/"))?
            .strip_suffix(&extension)?
            .to_owned()
    };
    Some(crate::runtime::host_ids::identifier(namespace, &path))
}

pub(crate) fn host_instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if crate::runtime::host_game_event::fired_event(receiver)
        == Some(crate::runtime::events::GameEvent::ClientReloadListeners)
    {
        if r.name == "addListener" {
            let Some(id) = id_text(args.first()?) else {
                return Some(Err(NativeError::UnknownHostCall(r.clone())));
            };
            let key = state_key("neoforge-client", id);
            if host.persistent_static_fields.contains_key(&key) {
                return Some(Err(duplicate(
                    "java/lang/IllegalArgumentException",
                    format!("Attempted to register two reload listeners for the same key: {id}"),
                )));
            }
            host.persistent_static_fields
                .insert(key, Value::Ref(Some("registered".into())));
            return Some(Ok(Value::Void));
        }
        if r.name == "addDependency" {
            let first = registration_id(args.first()?)?;
            let second = registration_id(args.get(1)?)?;
            for id in [first, second] {
                let known = id.starts_with("minecraft:")
                    || id.starts_with("fabric:")
                    || host
                        .persistent_static_fields
                        .contains_key(&state_key("neoforge-client", &id));
                if !known {
                    return Some(Err(duplicate(
                        "java/lang/IllegalArgumentException",
                        format!("Unknown reload listener: {id}"),
                    )));
                }
            }
            return Some(Ok(Value::Void));
        }
    }
    let Value::Host(handle) = receiver else {
        return None;
    };
    if handle.kind() != HostKind::FileToIdConverter {
        let _ = host;
        return None;
    }
    let missing = || NativeError::UnknownHostCall(r.clone());
    Some(match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "idToFile",
            "(Lnet/minecraft/resources/Identifier;)Lnet/minecraft/resources/Identifier;",
        ) => id_transform(handle, args.first()?, true).ok_or_else(missing),
        (
            "fileToId",
            "(Lnet/minecraft/resources/Identifier;)Lnet/minecraft/resources/Identifier;",
        ) => id_transform(handle, args.first()?, false).ok_or_else(missing),
        ("extensionMatches", "(Lnet/minecraft/resources/Identifier;)Z") => {
            let answer = id_text(args.first()?)
                .is_some_and(|id| parts(handle).is_some_and(|(_, ext)| id.ends_with(&ext)));
            Ok(Value::Int(i32::from(answer)))
        }
        ("prefixMatches", "(Lnet/minecraft/resources/Identifier;)Z") => {
            let answer = id_text(args.first()?).is_some_and(|id| {
                parts(handle).is_some_and(|(prefix, _)| {
                    id.split_once(':')
                        .is_some_and(|(_, path)| path.starts_with(&prefix))
                })
            });
            Ok(Value::Int(i32::from(answer)))
        }
        ("matches", "(Lnet/minecraft/resources/Identifier;)Z") => {
            let answer = id_text(args.first()?).is_some_and(|id| {
                parts(handle).is_some_and(|(prefix, ext)| {
                    id.split_once(':')
                        .is_some_and(|(_, path)| path.starts_with(&prefix) && path.ends_with(&ext))
                })
            });
            Ok(Value::Int(i32::from(answer)))
        }
        ("prefix", "()Ljava/lang/String;") => parts(handle)
            .map(|(prefix, _)| Value::Ref(Some(prefix.to_owned())))
            .ok_or_else(missing),
        ("extension", "()Ljava/lang/String;") => parts(handle)
            .map(|(_, extension)| Value::Ref(Some(extension.to_owned())))
            .ok_or_else(missing),
        _ => return None,
    })
}

fn register(
    runtime: &mut Runtime<'_, impl Host>,
    resource: &str,
    id: &str,
    class: &str,
    message: String,
) -> Result<Value, NativeError> {
    let key = state_key(resource, id);
    if runtime.static_fields.contains_key(&key) {
        return Err(duplicate(class, message));
    }
    runtime
        .static_fields
        .insert(key, Value::Ref(Some("registered".into())));
    Ok(Value::Void)
}

pub(crate) fn runtime_instance_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let kind = match receiver {
        Value::Host(handle) => handle.kind(),
        _ => return None,
    };
    if matches!(
        kind,
        HostKind::FabricResourceManagerHelper | HostKind::FabricResourceLoader
    ) {
        let resource = match receiver {
            Value::Host(handle) => handle.values()?.first()?.clone(),
            _ => return None,
        };
        let resource = crate::runtime::expect_ref(&resource)?.to_owned();
        if matches!(
            r.name.as_str(),
            "registerReloadListener" | "registerReloader" | "addReloadListener"
        ) {
            let id = if r
                .descriptor
                .starts_with("(Lnet/minecraft/resources/Identifier;")
            {
                registration_id(args.first()?)?
            } else {
                let listener = args.first()?.clone();
                let class = crate::runtime::jar_object::object_class(&listener)?.to_owned();
                let id = match runtime.invoke(
                    &class,
                    "getFabricId",
                    "()Lnet/minecraft/resources/Identifier;",
                    Some(listener),
                    &[],
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                registration_id(&id)?
            };
            return Some(register(
                runtime,
                &resource,
                &id,
                "java/lang/IllegalStateException",
                format!("Tried to register resource listener {id} twice!"),
            ));
        }
        if r.name == "addListenerOrdering" {
            let first = id_text(args.first()?)?;
            let second = id_text(args.get(1)?)?;
            if first == second {
                return Some(Err(duplicate(
                    "java/lang/IllegalArgumentException",
                    "Tried to add a phase that depends on itself.".into(),
                )));
            }
            return Some(Ok(Value::Void));
        }
        return None;
    }
    if crate::runtime::host_game_event::fired_event(receiver)
        == Some(crate::runtime::events::GameEvent::ClientReloadListeners)
    {
        if r.name == "addListener" {
            let id = id_text(args.first()?)?.to_owned();
            return Some(register(
                runtime,
                "neoforge-client",
                &id,
                "java/lang/IllegalArgumentException",
                format!("Attempted to register two reload listeners for the same key: {id}"),
            ));
        }
        if r.name == "addDependency" {
            let first = id_text(args.first()?)?;
            let second = id_text(args.get(1)?)?;
            for id in [first, second] {
                let known = id.starts_with("minecraft:")
                    || id.starts_with("fabric:")
                    || runtime
                        .static_fields
                        .contains_key(&state_key("neoforge-client", id));
                if !known {
                    return Some(Err(duplicate(
                        "java/lang/IllegalArgumentException",
                        format!("Unknown reload listener: {id}"),
                    )));
                }
            }
            return Some(Ok(Value::Void));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converter_transformations_follow_java_bytecode() {
        let converter = converter("textures");
        let Value::Host(ref handle) = converter else {
            unreachable!()
        };
        let id = crate::runtime::host_ids::identifier_of_text("demo:foo/bar");
        assert_eq!(
            id_transform(handle, &id, true)
                .and_then(|v| crate::runtime::host_ids::identifier_key(&v).map(str::to_owned)),
            Some("demo:textures/foo/bar.json".into())
        );
        assert_eq!(
            id_transform(
                handle,
                &crate::runtime::host_ids::identifier_of_text("demo:textures/foo/bar.json"),
                false
            )
            .and_then(|v| crate::runtime::host_ids::identifier_key(&v).map(str::to_owned)),
            Some("demo:foo/bar".into())
        );
        let mut host = StdHost::new();
        let call = MethodRef {
            owner: CONVERTER.into(),
            name: "fileToId".into(),
            descriptor:
                "(Lnet/minecraft/resources/Identifier;)Lnet/minecraft/resources/Identifier;".into(),
        };
        assert!(
            host.invoke_instance(
                &call,
                &converter,
                &[crate::runtime::host_ids::identifier_of_text(
                    "demo:textures/foo.json"
                )]
            )
            .is_ok()
        );
    }
}

//! Bounded DataFixerUpper codec construction handles.
//!
//! This family records construction structure and callback roots exactly. Codec
//! execution (encode/decode/parse) and the broad factory surface remain counted.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested};
use crate::{MethodRef, NativeError, Value};

const CODEC: &str = "com/mojang/serialization/Codec";
const MAP_CODEC: &str = "com/mojang/serialization/MapCodec";
const PRIMITIVE: &str = "com/mojang/serialization/codecs/PrimitiveCodec";
const RECORD_BUILDER: &str = "com/mojang/serialization/codecs/RecordCodecBuilder";
const APP: &str = "com/mojang/datafixers/kinds/App";
const OBJECT: &str = "java/lang/Object";

const MAX_FIELD_NAME_BYTES: usize = 256;
const MAX_GRAPH_NODES: usize = 256;
const MAX_CALLBACK_ROOTS: usize = 64;
const MAX_NODE_DEPTH: usize = 64;
static NEXT_NODE: AtomicU64 = AtomicU64::new(1);

const CONSTANT: &str = "dfu-constant";
const FIELD: &str = "dfu-field";
const OPTIONAL: &str = "dfu-optional";
const XMAP: &str = "dfu-xmap";
const MAP_XMAP: &str = "dfu-map-xmap";
const GETTER: &str = "dfu-builder-getter";

const CODEC_TYPES: &[&str] = &[
    CODEC,
    "com/mojang/serialization/Encoder",
    "com/mojang/serialization/Decoder",
];
const MAP_TYPES: &[&str] = &[
    MAP_CODEC,
    "com/mojang/serialization/CompressorHolder",
    "com/mojang/serialization/Compressable",
    "com/mojang/serialization/MapDecoder",
    "com/mojang/serialization/MapEncoder",
    "com/mojang/serialization/Keyable",
];
const BUILDER_TYPES: &[&str] = &[RECORD_BUILDER, APP];

fn held_values(value: &Value) -> Option<Vec<Value>> {
    let Value::Host(handle) = value else {
        return None;
    };
    (handle.kind() == HostKind::DfuCodec).then(|| handle.held().to_vec())
}

fn tag(value: &Value) -> Option<String> {
    held_values(value)?.first().and_then(|value| match value {
        Value::Ref(Some(tag)) => Some(tag.clone()),
        _ => None,
    })
}

fn constant_info(value: &Value) -> Option<(String, String)> {
    let values = held_values(value)?;
    let [
        Value::Ref(Some(tag)),
        Value::Ref(Some(declared)),
        _,
        Value::Ref(Some(name)),
    ] = values.as_slice()
    else {
        return None;
    };
    (tag == CONSTANT).then(|| (declared.clone(), name.clone()))
}

fn is_callback(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::Callback)
}

fn map_codec(value: &Value) -> bool {
    let Some(tag) = tag(value) else {
        return false;
    };
    if matches!(tag.as_str(), FIELD | OPTIONAL | MAP_XMAP | GETTER) {
        return tag != GETTER;
    }
    constant_info(value).is_some_and(|(declared, name)| declared == MAP_CODEC || name == "EMPTY")
}

fn codec_receiver(value: &Value) -> bool {
    let Some(tag) = tag(value) else {
        return false;
    };
    matches!(tag.as_str(), CONSTANT | XMAP) && !map_codec(value)
}

fn map_receiver(value: &Value) -> bool {
    map_codec(value) && !matches!(tag(value).as_deref(), Some(GETTER))
}

#[derive(Default)]
struct GraphStats {
    nodes: usize,
    depth: usize,
    callback_roots: HashSet<String>,
    seen: HashSet<String>,
}

fn walk(value: &Value, depth: usize, stats: &mut GraphStats) {
    if is_callback(value) {
        if let Value::Host(handle) = value {
            stats.callback_roots.insert(handle.key().to_owned());
            for child in handle.held() {
                walk(child, depth, stats);
            }
        }
        return;
    }
    let Value::Host(handle) = value else {
        return;
    };
    if handle.kind() != HostKind::DfuCodec || !stats.seen.insert(handle.key().to_owned()) {
        return;
    }
    stats.nodes = stats.nodes.saturating_add(1);
    stats.depth = stats.depth.max(depth);
    for child in handle.held() {
        walk(child, depth.saturating_add(1), stats);
    }
}

fn node(tag: &str, values: Vec<Value>) -> Result<Value, &'static str> {
    let mut stats = GraphStats::default();
    for value in &values {
        walk(value, 1, &mut stats);
    }
    if stats.nodes.saturating_add(1) > MAX_GRAPH_NODES {
        return Err("DFU codec graph budget exceeded (256 nodes)");
    }
    if stats.depth > MAX_NODE_DEPTH {
        return Err("DFU codec nesting depth exceeded (64)");
    }
    if stats.callback_roots.len() > MAX_CALLBACK_ROOTS {
        return Err("DFU codec callback-root budget exceeded (64)");
    }
    let mut held = Vec::with_capacity(values.len() + 2);
    held.push(Value::Ref(Some(tag.to_owned())));
    held.push(Value::Long(NEXT_NODE.fetch_add(1, Ordering::Relaxed) as i64));
    held.extend(values);
    Ok(Value::Host(HostRef::of_values(HostKind::DfuCodec, &held)))
}

fn constant(declared: &str, owner: &str, name: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::DfuCodec,
        &[
            Value::Ref(Some(CONSTANT.to_owned())),
            Value::Ref(Some(declared.to_owned())),
            Value::Ref(Some(owner.to_owned())),
            Value::Ref(Some(name.to_owned())),
        ],
    ))
}

fn instance_of(handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    let declared = dfu_codec_types(handle.key());
    if wanted == OBJECT
        || declared.contains(&wanted)
        || hierarchy.is_instance(declared[0], wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false)
        || crate::host_ref::exact_class_rules_out(declared[0], wanted))
    .then_some(false)
}

fn primitive_text(name: &str) -> Option<&'static str> {
    Some(match name {
        "BOOL" => "Bool",
        "BYTE" => "Byte",
        "SHORT" => "Short",
        "INT" => "Int",
        "LONG" => "Long",
        "FLOAT" => "Float",
        "DOUBLE" => "Double",
        "STRING" => "String",
        "BYTE_BUFFER" => "ByteBuffer",
        "INT_STREAM" => "IntStream",
        "LONG_STREAM" => "LongStream",
        "PASSTHROUGH" => "passthrough",
        "EMPTY" => "Unit[Unit]",
        _ => return None,
    })
}

fn text(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    if handle.kind() != HostKind::DfuCodec {
        return Ok(None);
    }
    let values = handle.held();
    let Some(Value::Ref(Some(kind))) = values.first() else {
        return Ok(None);
    };
    match kind.as_str() {
        CONSTANT => {
            let Some(Value::Ref(Some(name))) = values.get(3) else {
                return Ok(None);
            };
            Ok(primitive_text(name).map(str::to_owned))
        }
        FIELD | OPTIONAL => {
            let (Some(inner), Some(Value::Ref(Some(name)))) = (values.get(2), values.get(3)) else {
                return Ok(None);
            };
            let Some(inner) = nested.text(inner)? else {
                return Ok(None);
            };
            let mut text = if kind == FIELD {
                format!("Field[{name}: {inner}]")
            } else {
                format!("OptionalFieldCodec[{name}: {inner}]")
            };
            if kind == OPTIONAL {
                if !matches!(values.get(5), Some(Value::Void) | None) {
                    text.push_str("[xmapped]");
                } else if !matches!(values.get(6), Some(Value::Void) | None) {
                    text.push_str("[flatXmapped]");
                }
            }
            Ok(Some(text))
        }
        XMAP | MAP_XMAP => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            Ok(nested.text(inner)?.map(|text| format!("{text}[xmapped]")))
        }
        _ => Ok(None),
    }
}

fn crosses(handle: &HostRef, f: &dyn Fn(&Value) -> bool) -> bool {
    handle.held().iter().all(f)
}

pub(crate) const DFU_CODEC_METHODS: Methods = Methods {
    to_string: text,
    hash_code: crate::host_events::DECLARED_TYPE_METHODS.hash_code,
    equals: crate::host_events::DECLARED_TYPE_METHODS.equals,
    instance_of,
    crosses_jars: crosses,
    compare_to: None,
};

pub(crate) fn dfu_codec_types(key: &str) -> &'static [&'static str] {
    let tag = key
        .split(',')
        .next()
        .unwrap_or_default()
        .strip_prefix('r')
        .unwrap_or_default();
    let declared = key
        .split(',')
        .nth(1)
        .and_then(|part| part.strip_prefix('r'));
    if matches!(tag, FIELD | OPTIONAL | MAP_XMAP)
        || (tag == CONSTANT && declared == Some(MAP_CODEC))
    {
        MAP_TYPES
    } else if tag == GETTER {
        BUILDER_TYPES
    } else {
        CODEC_TYPES
    }
}

pub(crate) fn codec_type(value: &Value) -> Option<&'static str> {
    let Value::Host(handle) = value else {
        return None;
    };
    (handle.kind() == HostKind::DfuCodec).then(|| dfu_codec_types(handle.key())[0])
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner != CODEC
        || !matches!(
            r.descriptor.as_str(),
            "Lcom/mojang/serialization/codecs/PrimitiveCodec;"
                | "Lcom/mojang/serialization/Codec;"
                | "Lcom/mojang/serialization/MapCodec;"
        )
    {
        return None;
    }
    let names = [
        "BOOL",
        "BYTE",
        "SHORT",
        "INT",
        "LONG",
        "FLOAT",
        "DOUBLE",
        "STRING",
        "BYTE_BUFFER",
        "INT_STREAM",
        "LONG_STREAM",
        "PASSTHROUGH",
        "EMPTY",
    ];
    names
        .contains(&r.name.as_str())
        .then(|| constant(&r.descriptor[1..r.descriptor.len() - 1], &r.owner, &r.name))
}

fn field_name(args: &[Value]) -> Result<String, &'static str> {
    let Some(Value::Ref(Some(name))) = args.first() else {
        return Err("DFU codec field name is not a String");
    };
    (name.len() <= MAX_FIELD_NAME_BYTES)
        .then(|| name.clone())
        .ok_or("DFU codec field-name budget exceeded (256 bytes)")
}

fn optional_parts<'a>(descriptor: &str, args: &'a [Value]) -> Option<(Value, Value)> {
    let has_lifecycle_first =
        descriptor.starts_with("(Ljava/lang/String;Lcom/mojang/serialization/Lifecycle;");
    match (has_lifecycle_first, args.get(1), args.get(2)) {
        (true, Some(lifecycle), Some(default)) => Some((default.clone(), lifecycle.clone())),
        (false, Some(default), Some(lifecycle)) => Some((default.clone(), lifecycle.clone())),
        (false, Some(default), None) => Some((default.clone(), Value::Void)),
        (true, _, _) => None,
        _ => Some((Value::Void, Value::Void)),
    }
}

pub(crate) fn invoke(r: &MethodRef, receiver: Option<&Value>, args: &[Value]) -> Option<Value> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            CODEC | PRIMITIVE,
            "fieldOf",
            "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;",
        ) => {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let name = field_name(args).ok()?;
            node(
                FIELD,
                vec![
                    inner,
                    Value::Ref(Some(name)),
                    Value::Ref(Some("required".to_owned())),
                    Value::Void,
                    Value::Void,
                ],
            )
            .ok()
        }
        (CODEC | PRIMITIVE, "optionalFieldOf", descriptor)
            if descriptor.starts_with("(Ljava/lang/String;")
                && descriptor.ends_with(")Lcom/mojang/serialization/MapCodec;") =>
        {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let name = field_name(args).ok()?;
            let (default, lifecycle) = optional_parts(descriptor, args)?;
            node(
                OPTIONAL,
                vec![
                    inner,
                    Value::Ref(Some(name)),
                    Value::Ref(Some("optional".to_owned())),
                    default,
                    lifecycle,
                ],
            )
            .ok()
        }
        (
            MAP_CODEC,
            "forGetter",
            "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;",
        ) => {
            let inner = receiver.filter(|value| map_receiver(value))?.clone();
            let getter = args.first().filter(|value| is_callback(value))?.clone();
            node(GETTER, vec![inner, getter]).ok()
        }
        (
            CODEC | PRIMITIVE | MAP_CODEC,
            "xmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
        )
        | (
            CODEC | PRIMITIVE | MAP_CODEC,
            "xmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/MapCodec;",
        ) => {
            let inner = receiver?;
            let is_map = map_codec(inner);
            if (r.owner == MAP_CODEC && !is_map) || (r.owner != MAP_CODEC && is_map) {
                return None;
            }
            if !is_map && !codec_receiver(inner) || is_map && !map_receiver(inner) {
                return None;
            }
            let to = args.first().filter(|value| is_callback(value))?.clone();
            let from = args.get(1).filter(|value| is_callback(value))?.clone();
            node(
                if is_map { MAP_XMAP } else { XMAP },
                vec![inner.clone(), to, from],
            )
            .ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback_marker;

    fn method(owner: &str, name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    #[test]
    fn nodes_hold_callbacks_and_reject_forged_strings_and_other_families() {
        let callback = callback_marker::lambda_from_text(
            "demo/P",
            0,
            "java/util/function/Function",
            "demo/P#f(Ljava/lang/Object;)Ljava/lang/Object;",
            "",
        );
        let inner = constant(
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "INT",
        );
        let field = invoke(
            &method(
                CODEC,
                "fieldOf",
                "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;",
            ),
            Some(&inner),
            &[Value::Ref(Some("n".into()))],
        )
        .unwrap();
        assert_eq!(tag(&field).as_deref(), Some(FIELD));
        let forged_text = held_values(&field).unwrap();
        assert!(matches!(forged_text.first(), Some(Value::Ref(Some(tag))) if tag == FIELD));
        let getter = invoke(
            &method(
                MAP_CODEC,
                "forGetter",
                "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;",
            ),
            Some(&field),
            std::slice::from_ref(&callback),
        )
        .unwrap();
        assert_eq!(tag(&getter).as_deref(), Some(GETTER));
        assert!(
            held_values(&getter)
                .unwrap()
                .iter()
                .any(|value| is_callback(value))
        );
        let stream = Value::Host(HostRef::new(HostKind::StreamCodec, "stream"));
        assert!(
            invoke(
                &method(
                    CODEC,
                    "xmap",
                    "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
                ),
                Some(&stream),
                &[callback.clone(), callback.clone()],
            )
            .is_none()
        );
        let forged = Value::Ref(Some("dfu-field,1".into()));
        assert!(
            invoke(
                &method(
                    MAP_CODEC,
                    "forGetter",
                    "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;",
                ),
                Some(&forged),
                std::slice::from_ref(&callback),
            )
            .is_none()
        );
    }

    #[test]
    fn collector_keeps_xmap_callback_after_sources_are_dropped() {
        let callback = callback_marker::lambda_from_text(
            "demo/P",
            1,
            "java/util/function/Function",
            "demo/P#g(Ljava/lang/Object;)Ljava/lang/Object;",
            "",
        );
        let inner = constant(
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "INT",
        );
        let mapped = invoke(
            &method(
                CODEC,
                "xmap",
                "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
            ),
            Some(&inner),
            &[callback.clone(), callback.clone()],
        )
        .unwrap();
        let mut host = crate::StdHost::new();
        host.persistent_static_fields
            .insert("demo/Main.CODEC".into(), mapped.clone());
        drop(callback);
        drop(inner);
        let mut heap = crate::heap::JavaHeap::default();
        for _ in 0..3 {
            crate::heap::collect(&mut heap, &mut host, &[]);
        }
        assert!(
            held_values(&mapped)
                .unwrap()
                .iter()
                .any(|value| is_callback(value))
        );
    }
}

//! Bounded DataFixerUpper codec construction handles.
//!
//! This family records construction structure and callback roots exactly. Codec
//! execution (encode/decode/parse) and the broad factory surface remain counted.

use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested};
use crate::{Host, MethodRef, NativeError, Runtime, Value};

const CODEC: &str = "com/mojang/serialization/Codec";
const MAP_CODEC: &str = "com/mojang/serialization/MapCodec";
const PRIMITIVE: &str = "com/mojang/serialization/codecs/PrimitiveCodec";
const RECORD_BUILDER: &str = "com/mojang/serialization/codecs/RecordCodecBuilder";
const APP: &str = "com/mojang/datafixers/kinds/App";
const OBJECT: &str = "java/lang/Object";
const DATA_RESULT: &str = "com/mojang/serialization/DataResult";

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
const OR_ELSE: &str = "dfu-or-else";
const UNIT: &str = "dfu-unit";
const UNIT_CODEC: &str = "dfu-unit-codec";
const MAP_TO_CODEC: &str = "dfu-map-to-codec";
const INT_RANGE: &str = "dfu-int-range";
const LIST: &str = "dfu-list";
const FLAT_XMAP: &str = "dfu-flat-xmap";
const COMAP_FLAT_MAP: &str = "dfu-comap-flat-map";
const RECORD: &str = "dfu-record";
const RECORD_MAP: &str = "dfu-record-map";
const INSTANCE: &str = "dfu-builder-instance";
const PRODUCT: &str = "dfu-product";
const APPLIED: &str = "dfu-applied";
const POINT: &str = "dfu-point";
const LIFECYCLE: &str = "dfu-lifecycle";

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
const INSTANCE_TYPES: &[&str] = &[
    "com/mojang/serialization/codecs/RecordCodecBuilder$Instance",
    APP,
];
const PRODUCT_TYPES: &[&str] = &["com/mojang/datafixers/Products", APP];
const LIFECYCLE_TYPES: &[&str] = &["com/mojang/serialization/Lifecycle"];

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
    if crate::host_codec::opaque_codec_type(value).is_some_and(|t| t == MAP_CODEC) {
        return true;
    }
    let Some(tag) = tag(value) else {
        return false;
    };
    if matches!(
        tag.as_str(),
        FIELD | OPTIONAL | MAP_XMAP | GETTER | UNIT | OR_ELSE | RECORD_MAP
    ) {
        return tag != GETTER;
    }
    constant_info(value).is_some_and(|(declared, name)| declared == MAP_CODEC || name == "EMPTY")
}

fn codec_receiver(value: &Value) -> bool {
    if crate::host_codec::opaque_codec_type(value).is_some_and(|t| t == CODEC || t == PRIMITIVE) {
        return true;
    }
    let Some(tag) = tag(value) else {
        return false;
    };
    (matches!(
        tag.as_str(),
        CONSTANT
            | XMAP
            | FLAT_XMAP
            | COMAP_FLAT_MAP
            | UNIT_CODEC
            | MAP_TO_CODEC
            | INT_RANGE
            | LIST
            | RECORD
    ) || crate::host_codec::opaque_codec_type(value).is_some_and(|t| t == CODEC || t == PRIMITIVE))
        && !map_codec(value)
}

fn map_receiver(value: &Value) -> bool {
    if crate::host_codec::opaque_codec_type(value).is_some_and(|t| t == MAP_CODEC) {
        return true;
    }
    map_codec(value) && !matches!(tag(value).as_deref(), Some(GETTER))
}

fn opaque_codec_result(descriptor: &str) -> Option<Value> {
    let declared = if descriptor.ends_with("Lcom/mojang/serialization/MapCodec;") {
        MAP_CODEC
    } else if descriptor.ends_with("Lcom/mojang/serialization/Codec;") {
        CODEC
    } else if descriptor.ends_with("Lcom/mojang/serialization/codecs/RecordCodecBuilder;") {
        RECORD_BUILDER
    } else {
        return None;
    };
    Some(crate::host_codec::opaque_codec(declared))
}

fn is_opaque_product(value: &Value) -> bool {
    product_arity(value).is_some() && product_parts(value).is_none()
}

fn is_opaque_app(value: &Value) -> bool {
    tag(value).as_deref() == Some(APPLIED) && held_values(value).is_some_and(|v| v.len() < 5)
}

#[derive(Default)]
struct GraphStats {
    nodes: usize,
    depth: usize,
    callback_roots: BTreeSet<String>,
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

fn node(host: &mut dyn Host, tag: &str, values: Vec<Value>) -> Result<Value, String> {
    let mut stats = GraphStats::default();
    for value in &values {
        walk(value, 1, &mut stats);
    }
    if stats.nodes.saturating_add(1) > MAX_GRAPH_NODES {
        return Err(
            "DFU codec graph budget exceeded: current graph exceeds maximum 256 nodes".into(),
        );
    }
    if stats.depth > MAX_NODE_DEPTH {
        return Err("DFU codec nesting depth exceeded: current depth exceeds maximum 64".into());
    }
    if stats.callback_roots.len() > MAX_CALLBACK_ROOTS {
        return Err(
            "DFU codec callback-root budget exceeded: current graph exceeds maximum 64 roots"
                .into(),
        );
    }
    host.reserve_dfu_codec(1, &stats.callback_roots)?;
    let mut held = Vec::with_capacity(values.len() + 2);
    held.push(Value::Ref(Some(tag.to_owned())));
    held.push(Value::Long(NEXT_NODE.fetch_add(1, Ordering::Relaxed) as i64));
    held.extend(values);
    let value = Value::Host(HostRef::of_values(HostKind::DfuCodec, &held));
    Ok(value)
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

fn lifecycle(name: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::DfuCodec,
        &[
            Value::Ref(Some(LIFECYCLE.to_owned())),
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
                if !matches!(values.get(6), Some(Value::Void) | None) {
                    text.push_str("[flatXmapped]");
                } else if !matches!(values.get(5), Some(Value::Void) | None) {
                    text.push_str("[xmapped]");
                }
            }
            Ok(Some(text))
        }
        // DFU's `orElse` is `mapResult(OrElse[...])`, whose text includes the fallback's own
        // `toString`: not modelled, so the text stays counted rather than invented.
        OR_ELSE => Ok(None),
        UNIT | UNIT_CODEC => {
            let Some(value) = values.get(2) else {
                return Ok(None);
            };
            Ok(nested.text(value)?.map(|text| format!("Unit[{text}]")))
        }
        LIST => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            Ok(nested.text(inner)?.map(|text| format!("ListCodec[{text}]")))
        }
        XMAP | MAP_XMAP => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            Ok(nested.text(inner)?.map(|text| format!("{text}[xmapped]")))
        }
        FLAT_XMAP | COMAP_FLAT_MAP => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            let suffix = if kind == FLAT_XMAP {
                "flatXmapped"
            } else {
                "comapFlatMapped"
            };
            Ok(nested.text(inner)?.map(|text| format!("{text}[{suffix}]")))
        }
        LIFECYCLE => {
            let Some(Value::Ref(Some(name))) = values.get(1) else {
                return Ok(None);
            };
            Ok(Some(
                match name.as_str() {
                    "stable" => "Stable",
                    "experimental" => "Experimental",
                    _ => return Ok(None),
                }
                .to_owned(),
            ))
        }
        _ => Ok(None),
    }
}

fn crosses(handle: &HostRef, f: &dyn Fn(&Value) -> bool) -> bool {
    handle.held().iter().all(f)
}

/// `Products.P1`..`P16` are Java records: equal when their components are. The same handle is
/// equal to itself; two distinct product handles may hold equal components, which this model does
/// not compare, so that `equals` is counted (#410 review). Codec handles keep identity equality.
fn dfu_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    if tag(&Value::Host(handle.clone())).as_deref() == Some(PRODUCT) {
        return Ok(match other {
            Value::Host(other) if handle.same_object(other) == Some(true) => Some(true),
            _ => None,
        });
    }
    (crate::host_events::DECLARED_TYPE_METHODS.equals)(handle, other, nested)
}

pub(crate) const DFU_CODEC_METHODS: Methods = Methods {
    to_string: text,
    hash_code: crate::host_events::DECLARED_TYPE_METHODS.hash_code,
    equals: dfu_equals,
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
    if matches!(tag, FIELD | OPTIONAL | MAP_XMAP | UNIT | RECORD_MAP)
        || (tag == CONSTANT && declared == Some(MAP_CODEC))
    {
        MAP_TYPES
    } else if tag == GETTER {
        BUILDER_TYPES
    } else if tag == INSTANCE {
        INSTANCE_TYPES
    } else if tag == PRODUCT || tag == APPLIED || tag == POINT {
        PRODUCT_TYPES
    } else if tag == LIFECYCLE {
        LIFECYCLE_TYPES
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
    if matches!(
        (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()),
        (
            "net/minecraft/resources/Identifier",
            "CODEC",
            "Lcom/mojang/serialization/Codec;"
        ) | (
            "net/minecraft/world/item/crafting/Recipe$CommonInfo",
            "MAP_CODEC",
            "Lcom/mojang/serialization/MapCodec;"
        ) | (
            "net/minecraft/world/item/crafting/Ingredient",
            "CODEC",
            "Lcom/mojang/serialization/Codec;"
        ) | (
            "net/minecraft/world/item/ItemStackTemplate",
            "CODEC",
            "Lcom/mojang/serialization/Codec;"
        ) | (
            "net/minecraft/world/item/ItemStackTemplate",
            "MAP_CODEC",
            "Lcom/mojang/serialization/MapCodec;"
        ) | (
            "net/minecraft/core/component/DataComponentPatch",
            "CODEC",
            "Lcom/mojang/serialization/Codec;"
        ) | (
            "net/minecraft/world/item/Item",
            "CODEC",
            "Lcom/mojang/serialization/Codec;"
        ) | (
            "net/minecraft/util/ExtraCodecs",
            "POSITIVE_INT" | "NON_NEGATIVE_INT",
            "Lcom/mojang/serialization/Codec;"
        )
    ) {
        return Some(constant(
            &r.descriptor[1..r.descriptor.len() - 1],
            &r.owner,
            &r.name,
        ));
    }
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

fn field_name(args: &[Value]) -> Result<String, String> {
    let Some(Value::Ref(Some(name))) = args.first() else {
        return Err("DFU codec field name is not a String".into());
    };
    if name.len() > MAX_FIELD_NAME_BYTES {
        return Err(format!(
            "DFU codec field-name budget exceeded: current {}, maximum {}",
            name.len(),
            MAX_FIELD_NAME_BYTES
        ));
    }
    Ok(name.clone())
}

fn refuse(host: &mut dyn Host, r: &MethodRef, reason: String) -> Option<Value> {
    host.dfu_codec_refused(format!(
        "{}#{}{}: {}",
        r.owner, r.name, r.descriptor, reason
    ));
    None
}

fn build(host: &mut dyn Host, r: &MethodRef, tag: &str, values: Vec<Value>) -> Option<Value> {
    match node(host, tag, values) {
        Ok(value) => Some(value),
        Err(reason) => refuse(host, r, reason),
    }
}

fn optional_parts(descriptor: &str, args: &[Value]) -> Option<(Value, Value)> {
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

fn receiver_held_value(value: &Value) -> Option<Value> {
    held_values(value)?.get(2).cloned()
}

fn unit_source(value: &Value) -> Option<Value> {
    let tag = tag(value)?;
    matches!(tag.as_str(), UNIT | UNIT_CODEC).then(|| receiver_held_value(value))?
}

/// Evaluate a `MapCodec.unit` / `Codec.unit` value or supplier callback.
///
/// Returns `Ok(None)` if evaluating the callback triggered any counted unknown call (including
/// throwing after a counted unknown call) or if the callback could not be invoked, so the
/// enclosing codec operation is counted rather than answering with a value derived from an
/// unmodelled call.
fn unit_value<H: Host>(
    runtime: &mut Runtime<'_, H>,
    source: &Value,
) -> Result<Option<Value>, NativeError> {
    if !is_callback(source) {
        return Ok(Some(source.clone()));
    }
    let before = runtime.host.counted_unknown_calls();
    match runtime.invoke_callback(source, &[]) {
        Ok(Some(value)) => {
            if runtime.host.counted_unknown_calls() != before {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        Ok(None) => Ok(None),
        Err(NativeError::UncaughtException { .. })
            if runtime.host.counted_unknown_calls() != before =>
        {
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

fn value_text<H: Host>(
    _runtime: &mut Runtime<'_, H>,
    value: &Value,
) -> Result<Option<String>, NativeError> {
    Ok(match value {
        Value::Ref(Some(text)) => Some(text.clone()),
        Value::Ref(None) => Some("null".to_owned()),
        Value::Int(value) => Some(value.to_string()),
        Value::Long(value) => Some(value.to_string()),
        Value::Float(value) => Some(value.to_string()),
        _ => None,
    })
}

fn runtime_text<H: Host>(
    runtime: &mut Runtime<'_, H>,
    value: &Value,
) -> Result<Option<String>, NativeError> {
    let Some(handle) = (match value {
        Value::Host(handle) if handle.kind() == HostKind::DfuCodec => Some(handle),
        _ => None,
    }) else {
        return Ok(None);
    };
    let values = handle.held();
    let Some(Value::Ref(Some(kind))) = values.first() else {
        return Ok(None);
    };
    match kind.as_str() {
        UNIT | UNIT_CODEC => {
            let Some(source) = values.get(2) else {
                return Ok(None);
            };
            let Some(value) = unit_value(runtime, source)? else {
                return Ok(None);
            };
            Ok(value_text(runtime, &value)?.map(|text| format!("Unit[{text}]")))
        }
        FIELD | OPTIONAL => {
            let (Some(inner), Some(Value::Ref(Some(name)))) = (values.get(2), values.get(3)) else {
                return Ok(None);
            };
            let Some(inner) = runtime_text(runtime, inner)? else {
                return Ok(None);
            };
            let mut text = if kind == FIELD {
                format!("Field[{name}: {inner}]")
            } else {
                format!("OptionalFieldCodec[{name}: {inner}]")
            };
            if kind == OPTIONAL {
                if !matches!(values.get(6), Some(Value::Void) | None) {
                    text.push_str("[flatXmapped]");
                } else if !matches!(values.get(5), Some(Value::Void) | None) {
                    text.push_str("[xmapped]");
                }
            }
            Ok(Some(text))
        }
        XMAP | MAP_XMAP => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            Ok(runtime_text(runtime, inner)?.map(|text| format!("{text}[xmapped]")))
        }
        FLAT_XMAP | COMAP_FLAT_MAP => {
            let Some(inner) = values.get(2) else {
                return Ok(None);
            };
            let suffix = if kind == FLAT_XMAP {
                "flatXmapped"
            } else {
                "comapFlatMapped"
            };
            Ok(runtime_text(runtime, inner)?.map(|text| format!("{text}[{suffix}]")))
        }
        CONSTANT => {
            let Some(Value::Ref(Some(name))) = values.get(3) else {
                return Ok(None);
            };
            Ok(primitive_text(name).map(str::to_owned))
        }
        _ => Ok(None),
    }
}

fn product_arity_from_descriptor(descriptor: &str) -> Option<usize> {
    descriptor
        .rsplit_once("$P")?
        .1
        .strip_suffix(';')?
        .parse()
        .ok()
}

fn product_parts(value: &Value) -> Option<(usize, Vec<Value>)> {
    let values = held_values(value)?;
    if tag(value).as_deref() != Some(PRODUCT) {
        return None;
    }
    let Value::Int(arity) = values.get(2)? else {
        return None;
    };
    let arity = usize::try_from(*arity).ok()?;
    let parts = values.get(3..3 + arity)?.to_vec();
    (parts.len() == arity).then_some((arity, parts))
}

/// A counted product keeps DFU's non-null construction invariant after a bounded graph refusal.
/// It carries only its arity; component reads remain unknown calls rather than fabricated values.
fn opaque_product(arity: usize) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::DfuCodec,
        &[
            Value::Ref(Some(PRODUCT.to_owned())),
            Value::Long(NEXT_NODE.fetch_add(1, Ordering::Relaxed) as i64),
            Value::Int(arity as i32),
        ],
    ))
}

fn opaque_app() -> Value {
    Value::Host(HostRef::of_values(
        HostKind::DfuCodec,
        &[
            Value::Ref(Some(APPLIED.to_owned())),
            Value::Long(NEXT_NODE.fetch_add(1, Ordering::Relaxed) as i64),
        ],
    ))
}

/// Counted fallback for a vanilla product constructor whose full codec graph is over budget.
pub(crate) fn counted_product(host: &mut dyn Host, r: &MethodRef) -> Option<Value> {
    let arity = r
        .descriptor
        .rsplit_once("$P")
        .and_then(|(_, rest)| rest.strip_suffix(';'))
        .and_then(|rest| rest.parse().ok())?;
    if !(1..=16).contains(&arity) {
        return None;
    }
    host.unknown_operand_call(r).ok()?;
    Some(opaque_product(arity))
}

/// Counted fallback for a product application whose receiver was lost after an earlier bounded
/// refusal. The real DFU application is non-null; keeping an opaque App lets the enclosing codec
/// construction remain counted instead of turning the refusal into an impossible receiver NPE.
pub(crate) fn counted_app(host: &mut dyn Host, r: &MethodRef) -> Option<Value> {
    host.unknown_operand_call(r).ok()?;
    Some(opaque_app())
}

fn product_arity(value: &Value) -> Option<usize> {
    let values = held_values(value)?;
    if tag(value).as_deref() != Some(PRODUCT) {
        return None;
    }
    let Value::Int(arity) = values.get(2)? else {
        return None;
    };
    usize::try_from(*arity)
        .ok()
        .filter(|arity| (1..=16).contains(arity))
}

fn product_and_values(receiver: &Value, argument: &Value) -> Option<Vec<Value>> {
    // An opaque product carries only its arity.  Treating it as one component
    // would let an exact product meet the descriptor's arity and fabricate a
    // precise result after a counted refusal.
    if (product_arity(receiver).is_some() && product_parts(receiver).is_none())
        || (product_arity(argument).is_some() && product_parts(argument).is_none())
    {
        return None;
    }
    let (_, receiver_parts) = product_parts(receiver)?;
    let mut values = receiver_parts;
    if let Some((_, argument_parts)) = product_parts(argument) {
        values.extend(argument_parts);
    } else {
        values.push(argument.clone());
    }
    Some(values)
}

fn runtime_refuse<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    reason: String,
) -> Result<Option<Value>, NativeError> {
    Ok(refuse(runtime.host, r, reason))
}

fn runtime_record_builder<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    args: &[Value],
    output_tag: &str,
) -> Result<Option<Value>, NativeError> {
    let Some(callback) = args.first().filter(|value| is_callback(value)) else {
        return Ok(None);
    };
    let instance = match node(runtime.host, INSTANCE, Vec::new()) {
        Ok(value) => value,
        Err(reason) => return runtime_refuse(runtime, r, reason),
    };
    let declared = if output_tag == RECORD_MAP {
        MAP_CODEC
    } else {
        CODEC
    };
    let before = runtime.host.counted_unknown_calls();
    let app_result = runtime.invoke_callback(callback, &[instance]);
    let counted_delta = runtime.host.counted_unknown_calls() != before;
    let app_opt = match app_result {
        Ok(value) => value,
        Err(NativeError::UncaughtException { .. }) if counted_delta => {
            runtime_refuse(
                runtime,
                r,
                "RecordCodecBuilder callback threw after a counted unknown call".into(),
            )?;
            runtime.host.unknown_operand_call(r)?;
            return Ok(Some(crate::host_codec::opaque_codec(declared)));
        }
        Err(err) => return Err(err),
    };
    if counted_delta {
        runtime_refuse(
            runtime,
            r,
            "RecordCodecBuilder callback invoked a counted unknown call".into(),
        )?;
        runtime.host.unknown_operand_call(r)?;
        return Ok(Some(crate::host_codec::opaque_codec(declared)));
    }
    let Some(app) = app_opt else {
        runtime_refuse(
            runtime,
            r,
            "RecordCodecBuilder callback returned null".into(),
        )?;
        runtime.host.unknown_operand_call(r)?;
        return Ok(Some(crate::host_codec::opaque_codec(declared)));
    };
    if matches!(app, Value::Ref(None) | Value::Void) {
        runtime.host.dfu_codec_refused(format!(
            "{}#{}{}: RecordCodecBuilder callback returned null",
            r.owner, r.name, r.descriptor
        ));
        return Ok(Some(crate::host_codec::opaque_codec(declared)));
    }
    if !matches!(tag(&app).as_deref(), Some(APPLIED) | Some(POINT)) || is_opaque_app(&app) {
        runtime_refuse(
            runtime,
            r,
            if is_opaque_app(&app) {
                "RecordCodecBuilder callback returned an opaque App node".into()
            } else {
                "RecordCodecBuilder callback did not return an App node".into()
            },
        )?;
        runtime.host.unknown_operand_call(r)?;
        return Ok(Some(crate::host_codec::opaque_codec(declared)));
    }
    match node(runtime.host, output_tag, vec![app]) {
        Ok(value) => Ok(Some(value)),
        Err(reason) => {
            runtime_refuse(runtime, r, reason)?;
            runtime.host.unknown_operand_call(r)?;
            Ok(Some(crate::host_codec::opaque_codec(declared)))
        }
    }
}

/// Runtime-dependent members of the unit MapCodec nodes. Construction stays in [`invoke`]
/// because it only needs the host's per-jar budget; execution needs the callback/runtime.
pub(crate) fn runtime_instance_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if receiver.is_none() {
        if r.owner == "com/mojang/serialization/Lifecycle"
            && r.descriptor == "()Lcom/mojang/serialization/Lifecycle;"
        {
            return Ok(match r.name.as_str() {
                "stable" => Some(lifecycle("stable")),
                "experimental" => Some(lifecycle("experimental")),
                _ => None,
            });
        }
        if r.owner == RECORD_BUILDER {
            if r.name == "instance"
                && r.descriptor == "()Lcom/mojang/serialization/codecs/RecordCodecBuilder$Instance;"
            {
                return Ok(Some(match node(runtime.host, INSTANCE, Vec::new()) {
                    Ok(value) => value,
                    Err(reason) => {
                        return runtime_refuse(runtime, r, reason);
                    }
                }));
            }
            if r.name == "create"
                && r.descriptor == "(Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;"
            {
                return runtime_record_builder(runtime, r, args, RECORD);
            }
            if r.name == "mapCodec"
                && r.descriptor
                    == "(Ljava/util/function/Function;)Lcom/mojang/serialization/MapCodec;"
            {
                return runtime_record_builder(runtime, r, args, RECORD_MAP);
            }
            if matches!(r.name.as_str(), "stable" | "point")
                && r.descriptor.starts_with("(Ljava/lang/Object;")
            {
                let Some(value) = args.first().cloned() else {
                    return Ok(None);
                };
                let mut values = vec![value];
                values.push(args.get(1).cloned().unwrap_or(Value::Void));
                return match node(runtime.host, POINT, values) {
                    Ok(value) => Ok(Some(value)),
                    Err(reason) => runtime_refuse(runtime, r, reason),
                };
            }
        }
    }
    if r.owner == DATA_RESULT && r.name == "result" && r.descriptor == "()Ljava/util/Optional;" {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        return Ok(match runtime.read_data_result(receiver) {
            Ok(Value::Ref(None)) => Some(crate::empty_optional()),
            Ok(value) => Some(crate::optional_wrap(value)),
            Err(None) => Some(crate::empty_optional()),
            Err(Some(_)) => None,
        });
    }
    if r.owner == DATA_RESULT && r.name == "isSuccess" && r.descriptor == "()Z" {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        return Ok(match runtime.read_data_result(receiver) {
            Ok(_) => Some(Value::Int(1)),
            Err(None) => Some(Value::Int(0)),
            Err(Some(_)) => None,
        });
    }
    let Some(receiver) = receiver else {
        return Ok(None);
    };
    let receiver_tag = tag(receiver);
    if receiver_tag.as_deref() == Some(PRODUCT)
        && r.owner.starts_with("com/mojang/datafixers/Products$P")
        && r.name.starts_with('t')
        && r.descriptor
            .starts_with("()Lcom/mojang/datafixers/kinds/App;")
    {
        let Some(index) = r.name[1..]
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
        else {
            return Ok(None);
        };
        let Some((_, parts)) = product_parts(receiver) else {
            return runtime.host.unknown_operand_call(r).map(Some);
        };
        return match parts.get(index).cloned() {
            Some(value) => Ok(Some(value)),
            None => runtime.host.unknown_operand_call(r).map(Some),
        };
    }
    if receiver_tag.as_deref() == Some(INSTANCE)
        && r.owner == "com/mojang/serialization/codecs/RecordCodecBuilder$Instance"
    {
        if matches!(r.name.as_str(), "stable" | "point")
            && r.descriptor.starts_with("(Ljava/lang/Object;")
        {
            if args.iter().any(is_opaque_product) {
                runtime.host.unknown_operand_call(r)?;
                return Ok(Some(opaque_app()));
            }
            let Some(value) = args.first().cloned() else {
                return Ok(None);
            };
            let mut values = vec![value];
            values.push(args.get(1).cloned().unwrap_or(Value::Void));
            return match node(runtime.host, POINT, values) {
                Ok(value) => Ok(Some(value)),
                Err(reason) => runtime_refuse(runtime, r, reason),
            };
        }
        if r.name == "group" {
            let Some(arity) = product_arity_from_descriptor(&r.descriptor) else {
                return Ok(None);
            };
            if !(1..=16).contains(&arity) || args.len() != arity {
                return runtime_refuse(
                    runtime,
                    r,
                    format!("RecordCodecBuilder group arity {} is outside 1..16", arity),
                );
            }
            if args.iter().any(|arg| {
                is_opaque_product(arg) || crate::host_codec::opaque_codec_type(arg).is_some()
            }) {
                runtime.host.unknown_operand_call(r)?;
                return Ok(Some(opaque_product(arity)));
            }
            let mut values = vec![Value::Int(arity as i32)];
            values.extend_from_slice(args);
            return match node(runtime.host, PRODUCT, values) {
                Ok(value) => Ok(Some(value)),
                Err(reason) => {
                    runtime_refuse(runtime, r, reason)?;
                    runtime
                        .host
                        .unknown_operand_call(r)
                        .map(|_| Some(opaque_product(arity)))
                }
            };
        }
    }
    if r.owner.starts_with("com/mojang/datafixers/Products$P") && r.name == "and" {
        let Some(argument) = args.first() else {
            return Ok(None);
        };
        // An `and` whose result arity the descriptor does not name is counted, never guessed.
        let Some(arity) = product_arity_from_descriptor(&r.descriptor) else {
            return Ok(None);
        };
        if !(1..=16).contains(&arity) {
            return Ok(None);
        }
        let Some(parts) = product_and_values(receiver, argument) else {
            if is_opaque_product(receiver) || is_opaque_product(argument) {
                runtime.host.unknown_operand_call(r)?;
                return Ok(Some(opaque_product(arity)));
            }
            return Ok(None);
        };
        if parts.len() != arity {
            return Ok(None);
        }
        let mut values = vec![Value::Int(arity as i32)];
        values.extend(parts);
        return match node(runtime.host, PRODUCT, values) {
            Ok(value) => Ok(Some(value)),
            Err(reason) => {
                runtime_refuse(runtime, r, reason)?;
                runtime
                    .host
                    .unknown_operand_call(r)
                    .map(|_| Some(opaque_product(arity)))
            }
        };
    }
    if receiver_tag.as_deref() == Some(PRODUCT)
        && r.owner.starts_with("com/mojang/datafixers/Products$P")
        && r.name == "apply"
    {
        if is_opaque_product(receiver) || args.iter().any(is_opaque_product) {
            runtime.host.unknown_operand_call(r)?;
            return Ok(Some(opaque_app()));
        }
        let Some(constructor) = args.get(1).filter(|value| is_callback(value)) else {
            runtime.host.unknown_operand_call(r)?;
            return Ok(Some(opaque_app()));
        };
        return match node(
            runtime.host,
            APPLIED,
            vec![receiver.clone(), args[0].clone(), constructor.clone()],
        ) {
            Ok(value) => Ok(Some(value)),
            Err(reason) => {
                runtime_refuse(runtime, r, reason)?;
                runtime
                    .host
                    .unknown_operand_call(r)
                    .map(|_| Some(opaque_app()))
            }
        };
    }
    if r.name == "toString" && r.descriptor == "()Ljava/lang/String;" {
        return Ok(runtime_text(runtime, receiver)?.map(|text| Value::Ref(Some(text))));
    }
    let Some(source) = unit_source(receiver) else {
        return Ok(None);
    };
    let tag = tag(receiver).unwrap_or_default();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("keys", "(Lcom/mojang/serialization/DynamicOps;)Ljava/util/stream/Stream;")
            if tag == UNIT =>
        {
            Ok(Some(runtime.new_stream(
                "java/util/stream/Stream",
                "empty",
                Vec::new(),
            )))
        }
        (
            "decode",
            "(Lcom/mojang/serialization/DynamicOps;Lcom/mojang/serialization/MapLike;)Lcom/mojang/serialization/DataResult;",
        ) if tag == UNIT => {
            let Some(value) = unit_value(runtime, &source)? else {
                return Ok(None);
            };
            if matches!(value, Value::Ref(None)) {
                return Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: None,
                    frame: None,
                });
            }
            Ok(Some(runtime.data_result("success", value)))
        }
        (
            "encode",
            "(Ljava/lang/Object;Lcom/mojang/serialization/DynamicOps;Lcom/mojang/serialization/RecordBuilder;)Lcom/mojang/serialization/RecordBuilder;",
        ) if tag == UNIT => Ok(Some(args.get(2).cloned().unwrap_or(Value::Ref(None)))),
        _ => Ok(None),
    }
}

pub(crate) fn invoke(
    host: &mut dyn Host,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Option<Value> {
    // Mojang's loot base class is a tiny DFU helper. Mirror its bytecode's
    // optionalFieldOf -> forGetter -> group sequence, including the callback
    // captured by the method reference in the real class.
    if r.owner == "net/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction"
        && r.name == "commonFields"
        && r.descriptor
            == "(Lcom/mojang/serialization/codecs/RecordCodecBuilder$Instance;)Lcom/mojang/datafixers/Products$P1;"
    {
        let instance = args.first()?;
        if matches!(instance, Value::Ref(None)) {
            return None;
        }
        let codec = constant(
            CODEC,
            "net/minecraft/world/level/storage/loot/predicates/LootItemCondition",
            "CODEC",
        );
        let optional = invoke(
            host,
            &MethodRef {
                owner: CODEC.into(),
                name: "optionalFieldOf".into(),
                descriptor: "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;".into(),
            },
            Some(&codec),
            &[Value::Ref(Some("condition".into()))],
        )?;
        let callback = crate::callback_marker::lambda(
            r.owner.as_str(),
            11,
            "java/util/function/Function",
            Some(MethodRef {
                owner: r.owner.clone(),
                name: "lambda$commonFields$0".into(),
                descriptor: "(Lnet/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction;)Ljava/util/Optional;".into(),
            }),
            Some(6),
            Vec::new(),
            false,
        );
        let getter = match invoke(
            host,
            &MethodRef {
                owner: MAP_CODEC.into(),
                name: "forGetter".into(),
                descriptor: "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;".into(),
            },
            Some(&optional),
            &[callback],
        ) {
            Some(getter) => getter,
            None => {
                // The exact getter is known, but this jar has exhausted the bounded callback
                // graph. Count the commonFields call and keep Java's non-null P1 invariant so a
                // following `P1.and` cannot turn a bounded refusal into an impossible NPE.
                host.unknown_operand_call(r).ok()?;
                return Some(opaque_product(1));
            }
        };
        return match build(host, r, PRODUCT, vec![Value::Int(1), getter]) {
            Some(value) => Some(value),
            None => {
                host.unknown_operand_call(r).ok()?;
                Some(opaque_product(1))
            }
        };
    }
    if receiver.is_some_and(|value| crate::host_codec::opaque_codec_type(value).is_some()) {
        let result = opaque_codec_result(&r.descriptor)?;
        host.unknown_operand_call(r).ok()?;
        return Some(result);
    }
    if r.owner.starts_with("com/mojang/datafixers/Products$P") && r.name == "and" {
        let receiver = receiver?.clone();
        let argument = args.first()?.clone();
        let arity = product_arity_from_descriptor(&r.descriptor)?;
        if !(1..=16).contains(&arity) {
            return None;
        }
        let Some(parts) = product_and_values(&receiver, &argument) else {
            if is_opaque_product(&receiver) || is_opaque_product(&argument) {
                host.unknown_operand_call(r).ok()?;
                return Some(opaque_product(arity));
            }
            return None;
        };
        return (parts.len() == arity).then(|| {
            let mut values = vec![Value::Int(arity as i32)];
            values.extend(parts);
            build(host, r, PRODUCT, values)
        })?;
    }
    if r.owner.starts_with("com/mojang/datafixers/Products$P")
        && r.name == "apply"
        && receiver.is_some_and(is_opaque_product)
    {
        host.unknown_operand_call(r).ok()?;
        return Some(opaque_app());
    }
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        ("net/minecraft/util/ExtraCodecs", "intRange", "(II)Lcom/mojang/serialization/Codec;") => {
            let (Some(Value::Int(min)), Some(Value::Int(max))) = (args.first(), args.get(1)) else {
                return None;
            };
            build(host, r, INT_RANGE, vec![Value::Int(*min), Value::Int(*max)])
        }
        (MAP_CODEC, "unit", "(Ljava/lang/Object;)Lcom/mojang/serialization/MapCodec;")
        | (
            MAP_CODEC,
            "unit",
            "(Ljava/util/function/Supplier;)Lcom/mojang/serialization/MapCodec;",
        ) => build(host, r, UNIT, vec![args.first().cloned()?]),
        (
            CODEC | PRIMITIVE,
            "fieldOf",
            "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;",
        ) => {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let name = match field_name(args) {
                Ok(name) => name,
                Err(reason) => return refuse(host, r, reason),
            };
            build(
                host,
                r,
                FIELD,
                vec![
                    inner,
                    Value::Ref(Some(name)),
                    Value::Ref(Some("required".to_owned())),
                    Value::Void,
                    Value::Void,
                ],
            )
        }
        (CODEC | PRIMITIVE, "optionalFieldOf", descriptor)
            if descriptor.starts_with("(Ljava/lang/String;")
                && descriptor.ends_with(")Lcom/mojang/serialization/MapCodec;") =>
        {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let name = match field_name(args) {
                Ok(name) => name,
                Err(reason) => return refuse(host, r, reason),
            };
            let (default, lifecycle) = optional_parts(descriptor, args)?;
            build(
                host,
                r,
                OPTIONAL,
                vec![
                    inner,
                    Value::Ref(Some(name)),
                    Value::Ref(Some("optional".to_owned())),
                    default,
                    lifecycle,
                ],
            )
        }
        (
            MAP_CODEC,
            "forGetter",
            "(Ljava/util/function/Function;)Lcom/mojang/serialization/codecs/RecordCodecBuilder;",
        ) => {
            let inner = receiver.filter(|value| map_receiver(value))?.clone();
            let getter = args.first().filter(|value| is_callback(value))?.clone();
            build(host, r, GETTER, vec![inner, getter])
        }
        (MAP_CODEC, name, descriptor)
            if matches!(name, "orElse" | "orElseGet")
                && descriptor.ends_with(")Lcom/mojang/serialization/MapCodec;") =>
        {
            let inner = receiver.filter(|value| map_receiver(value))?.clone();
            let fallback = if args.len() >= 2 {
                args.get(1).cloned()
            } else {
                args.first().cloned()
            };
            build(
                host,
                r,
                OR_ELSE,
                vec![inner, fallback.unwrap_or(Value::Void)],
            )
        }
        (MAP_CODEC, "codec", "()Lcom/mojang/serialization/Codec;") => {
            let receiver = receiver.filter(|value| map_receiver(value))?;
            match tag(receiver).as_deref() {
                Some(UNIT) => build(host, r, UNIT_CODEC, vec![receiver_held_value(receiver)?]),
                Some(
                    RECORD_MAP | FIELD | OPTIONAL | MAP_XMAP | FLAT_XMAP | COMAP_FLAT_MAP | OR_ELSE,
                ) => build(host, r, MAP_TO_CODEC, vec![receiver.clone()]),
                Some(CONSTANT) => {
                    let values = held_values(receiver)?;
                    match (values.get(2), values.get(3)) {
                        (Some(Value::Ref(Some(owner))), Some(Value::Ref(Some(name))))
                            if matches!(
                                (owner.as_str(), name.as_str()),
                                (
                                    "net/minecraft/world/item/crafting/Recipe$CommonInfo",
                                    "MAP_CODEC"
                                ) | ("net/minecraft/world/item/ItemStackTemplate", "MAP_CODEC")
                            ) =>
                        {
                            build(host, r, MAP_TO_CODEC, vec![receiver.clone()])
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        (CODEC | PRIMITIVE, "listOf", descriptor)
            if (descriptor == "()Lcom/mojang/serialization/Codec;"
                || descriptor == "(II)Lcom/mojang/serialization/Codec;") =>
        {
            let inner = receiver.filter(|value| codec_receiver(value))?.clone();
            let mut values = vec![inner];
            if descriptor == "(II)Lcom/mojang/serialization/Codec;" {
                values.push(args.first().cloned()?);
                values.push(args.get(1).cloned()?);
            }
            build(host, r, LIST, values)
        }
        (
            CODEC | PRIMITIVE | MAP_CODEC,
            "flatXmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
        )
        | (
            CODEC | PRIMITIVE | MAP_CODEC,
            "comapFlatMap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
        )
        | (
            MAP_CODEC,
            "flatXmap",
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
            build(
                host,
                r,
                if r.name == "flatXmap" {
                    FLAT_XMAP
                } else {
                    COMAP_FLAT_MAP
                },
                vec![inner.clone(), to, from],
            )
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
            build(
                host,
                r,
                if is_map { MAP_XMAP } else { XMAP },
                vec![inner.clone(), to, from],
            )
        }
        _ => None,
    }
}

/// Invoke a mod callback and reject the result if any inner call was counted as unknown.
fn invoke_callback_exact<H: Host>(
    runtime: &mut Runtime<'_, H>,
    cb: &Value,
    args: &[Value],
    label: &str,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::recipe_models::DecodeError;
    let before = runtime.host.counted_unknown_calls();
    match runtime.invoke_callback(cb, args) {
        Ok(Some(value)) => {
            if runtime.host.counted_unknown_calls() != before {
                Ok(Err(DecodeError::Unmodelled(format!(
                    "counted unknown call inside {label}"
                ))))
            } else {
                Ok(Ok(value))
            }
        }
        Ok(None) => Ok(Err(DecodeError::Unmodelled(format!(
            "{label} callback could not be invoked"
        )))),
        Err(NativeError::UncaughtException { class, message, .. }) => {
            if runtime.host.counted_unknown_calls() != before {
                Ok(Err(DecodeError::Unmodelled(format!(
                    "counted unknown call before exception in {label}"
                ))))
            } else {
                Ok(Err(DecodeError::Invalid(format!(
                    "{}{}",
                    class.replace('/', "."),
                    message.map(|m| format!(": {m}")).unwrap_or_default()
                ))))
            }
        }
        Err(other) => Err(other),
    }
}

fn unit_value_exact<H: Host>(
    runtime: &mut Runtime<'_, H>,
    source: &Value,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::recipe_models::DecodeError;
    match unit_value(runtime, source)? {
        Some(value) => Ok(Ok(value)),
        None => Ok(Err(DecodeError::Unmodelled(
            "counted unknown call inside MapCodec.unit supplier".into(),
        ))),
    }
}

/// Decode a JSON object using a DFU `MapCodec` node built by the mod's own bytecode.
///
/// Reuses the exact DFU construction graph recorded by [`invoke`] and [`runtime_instance_call`].
/// Any unmodelled node, opaque codec, or counted inner call returns
/// `Ok(Err(DecodeError::Unmodelled(_)))` rather than guessing a value.
pub(crate) fn decode_map_json<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    decode_map_json_depth(runtime, codec, map, 0)
}

fn decode_map_json_depth<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    map: &serde_json::Map<String, serde_json::Value>,
    depth: usize,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::recipe_models::DecodeError;
    if depth > MAX_NODE_DEPTH {
        return Ok(Err(DecodeError::Unmodelled(
            "DFU decode recursion depth exceeded".into(),
        )));
    }
    let Some(values) = held_values(codec) else {
        return Ok(Err(DecodeError::Unmodelled(
            "MapCodec handle is not a modelled DFU node".into(),
        )));
    };
    let Some(Value::Ref(Some(kind))) = values.first() else {
        return Ok(Err(DecodeError::Unmodelled("DFU node missing tag".into())));
    };
    match kind.as_str() {
        RECORD_MAP => {
            let Some(app) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "RecordCodecBuilder map node missing App".into(),
                )));
            };
            decode_app_json(runtime, app, map, depth + 1)
        }
        APPLIED | POINT => decode_app_json(runtime, codec, map, depth + 1),
        GETTER => {
            let Some(inner) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "RecordCodecBuilder getter node missing inner MapCodec".into(),
                )));
            };
            decode_map_json_depth(runtime, inner, map, depth + 1)
        }
        FIELD => {
            let (Some(inner), Some(Value::Ref(Some(name)))) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "fieldOf node missing inner codec or field name".into(),
                )));
            };
            // DFU JsonOps MapLike.get(String) treats JsonNull identically to a missing key.
            let Some(field_val) = map.get(name).filter(|v| !v.is_null()) else {
                return Ok(Err(DecodeError::Invalid(format!(
                    "No key {name} in MapLike"
                ))));
            };
            decode_value_json_depth(runtime, inner, field_val, depth + 1)
        }
        OPTIONAL => {
            let (Some(inner), Some(Value::Ref(Some(name)))) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "optionalFieldOf node missing inner codec or field name".into(),
                )));
            };
            let default = values.get(5).cloned().unwrap_or(Value::Void);
            let has_default = !matches!(default, Value::Void);
            // DFU JsonOps MapLike.get(String) converts JsonNull to Java null, so OptionalFieldCodec
            // treats JSON `null` the same as an absent field (returning Optional.empty() / default).
            match map.get(name).filter(|v| !v.is_null()) {
                None => {
                    if has_default {
                        Ok(Ok(default))
                    } else {
                        Ok(Ok(crate::empty_optional()))
                    }
                }
                Some(field_val) => {
                    match decode_value_json_depth(runtime, inner, field_val, depth + 1)? {
                        Ok(decoded) => {
                            if has_default {
                                Ok(Ok(decoded))
                            } else {
                                Ok(Ok(crate::optional_wrap(decoded)))
                            }
                        }
                        Err(err) => Ok(Err(err)),
                    }
                }
            }
        }
        OR_ELSE => {
            let Some(inner) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "orElse node missing inner MapCodec".into(),
                )));
            };
            let fallback = values.get(3).cloned().unwrap_or(Value::Void);
            match decode_map_json_depth(runtime, inner, map, depth + 1)? {
                Ok(decoded) => Ok(Ok(decoded)),
                Err(DecodeError::Invalid(_)) => {
                    if is_callback(&fallback) {
                        invoke_callback_exact(runtime, &fallback, &[], "MapCodec.orElseGet")
                    } else if !matches!(fallback, Value::Void) {
                        Ok(Ok(fallback))
                    } else {
                        Ok(Err(DecodeError::Unmodelled(
                            "orElse node missing fallback".into(),
                        )))
                    }
                }
                Err(other) => Ok(Err(other)),
            }
        }
        UNIT => {
            let Some(source) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "MapCodec.unit node missing value".into(),
                )));
            };
            unit_value_exact(runtime, source)
        }
        MAP_XMAP => {
            let (Some(inner), Some(to_cb)) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "MapCodec.xmap node missing inner or function".into(),
                )));
            };
            let decoded = match decode_map_json_depth(runtime, inner, map, depth + 1)? {
                Ok(v) => v,
                Err(err) => return Ok(Err(err)),
            };
            invoke_callback_exact(runtime, to_cb, &[decoded], "MapCodec.xmap")
        }
        FLAT_XMAP | COMAP_FLAT_MAP => {
            let (Some(inner), Some(to_cb)) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "MapCodec flatXmap/comapFlatMap node missing inner or function".into(),
                )));
            };
            let decoded = match decode_map_json_depth(runtime, inner, map, depth + 1)? {
                Ok(v) => v,
                Err(err) => return Ok(Err(err)),
            };
            run_data_result_callback(runtime, to_cb, decoded)
        }
        CONSTANT => {
            let (Some(Value::Ref(Some(owner))), Some(Value::Ref(Some(name)))) =
                (values.get(2), values.get(3))
            else {
                return Ok(Err(DecodeError::Unmodelled(
                    "DFU constant MapCodec missing owner/name".into(),
                )));
            };
            let Some(std_host) = runtime.host.as_std_host_mut() else {
                return Ok(Err(DecodeError::Unmodelled(
                    "host does not support recipe codec decoding".into(),
                )));
            };
            match (owner.as_str(), name.as_str()) {
                ("net/minecraft/world/item/crafting/Recipe$CommonInfo", "MAP_CODEC") => {
                    Ok(crate::recipe_models::decode_common_info_map(std_host, map))
                }
                ("net/minecraft/world/item/ItemStackTemplate", "MAP_CODEC") => Ok(
                    crate::recipe_models::decode_item_stack_template_map(std_host, map),
                ),
                _ => Ok(Err(DecodeError::Unmodelled(format!(
                    "constant MapCodec {owner}.{name}"
                )))),
            }
        }
        other => Ok(Err(DecodeError::Unmodelled(format!(
            "unsupported MapCodec node kind: {other}"
        )))),
    }
}

fn decode_app_json<H: Host>(
    runtime: &mut Runtime<'_, H>,
    app: &Value,
    map: &serde_json::Map<String, serde_json::Value>,
    depth: usize,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::recipe_models::DecodeError;
    if depth > MAX_NODE_DEPTH {
        return Ok(Err(DecodeError::Unmodelled(
            "DFU App decode depth exceeded".into(),
        )));
    }
    let Some(values) = held_values(app) else {
        return Ok(Err(DecodeError::Unmodelled(
            "App is not a modelled DFU node".into(),
        )));
    };
    let Some(Value::Ref(Some(kind))) = values.first() else {
        return Ok(Err(DecodeError::Unmodelled("App node missing tag".into())));
    };
    match kind.as_str() {
        POINT => {
            let Some(val) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "RecordCodecBuilder.point missing value".into(),
                )));
            };
            Ok(Ok(val.clone()))
        }
        GETTER => {
            let Some(inner) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "RecordCodecBuilder getter missing MapCodec".into(),
                )));
            };
            decode_map_json_depth(runtime, inner, map, depth + 1)
        }
        APPLIED => {
            let (Some(product), Some(ctor_cb)) = (values.get(2), values.get(4)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "opaque or incomplete Products.apply node".into(),
                )));
            };
            let Some((_, parts)) = product_parts(product) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "opaque Products node in apply".into(),
                )));
            };
            // RecordCodecBuilder evaluates every field in field order, accumulating all errors
            // (and running observable field callbacks) before deciding whether to call ctor_cb.
            let mut decoded_args = Vec::with_capacity(parts.len());
            let mut invalid_errors = Vec::new();
            let mut first_unmodelled: Option<String> = None;
            for part in &parts {
                match decode_app_json(runtime, part, map, depth + 1)? {
                    Ok(v) => decoded_args.push(v),
                    Err(DecodeError::Invalid(msg)) => invalid_errors.push(msg),
                    Err(DecodeError::Unmodelled(msg)) => {
                        if first_unmodelled.is_none() {
                            first_unmodelled = Some(msg);
                        }
                    }
                }
            }
            if let Some(msg) = first_unmodelled {
                return Ok(Err(DecodeError::Unmodelled(msg)));
            }
            if !invalid_errors.is_empty() {
                return Ok(Err(DecodeError::Invalid(invalid_errors.join("; "))));
            }
            invoke_callback_exact(
                runtime,
                ctor_cb,
                &decoded_args,
                "RecordCodecBuilder constructor",
            )
        }
        _ => decode_map_json_depth(runtime, app, map, depth + 1),
    }
}

fn run_data_result_callback<H: Host>(
    runtime: &mut Runtime<'_, H>,
    cb: &Value,
    arg: Value,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::recipe_models::DecodeError;
    let result = match invoke_callback_exact(runtime, cb, &[arg], "codec DataResult")? {
        Ok(v) => v,
        Err(err) => return Ok(Err(err)),
    };
    let before = runtime.host.counted_unknown_calls();
    let parsed = runtime.read_data_result(&result);
    if runtime.host.counted_unknown_calls() != before {
        return Ok(Err(DecodeError::Unmodelled(
            "counted unknown call reading DataResult".into(),
        )));
    }
    match parsed {
        Ok(v) => Ok(Ok(v)),
        Err(None) => Ok(Err(DecodeError::Invalid(
            "codec callback returned DataResult.error".into(),
        ))),
        Err(Some(why)) => Ok(Err(DecodeError::Unmodelled(why))),
    }
}

fn decode_value_json_depth<H: Host>(
    runtime: &mut Runtime<'_, H>,
    codec: &Value,
    value: &serde_json::Value,
    depth: usize,
) -> Result<Result<Value, crate::recipe_models::DecodeError>, NativeError> {
    use crate::Boxed;
    use crate::recipe_models::DecodeError;
    if depth > MAX_NODE_DEPTH {
        return Ok(Err(DecodeError::Unmodelled(
            "DFU value decode depth exceeded".into(),
        )));
    }
    let Some(values) = held_values(codec) else {
        return Ok(Err(DecodeError::Unmodelled(
            "Codec handle is not a modelled DFU node".into(),
        )));
    };
    let Some(Value::Ref(Some(kind))) = values.first() else {
        return Ok(Err(DecodeError::Unmodelled(
            "Codec node missing tag".into(),
        )));
    };
    match kind.as_str() {
        RECORD => {
            let Some(app) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "RecordCodecBuilder.create node missing App".into(),
                )));
            };
            let Some(obj) = value.as_object() else {
                return Ok(Err(DecodeError::Invalid(format!(
                    "Not a JSON object: {value}"
                ))));
            };
            decode_app_json(runtime, app, obj, depth + 1)
        }
        MAP_TO_CODEC => {
            let Some(inner) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "MapCodec.codec() node missing inner MapCodec".into(),
                )));
            };
            let Some(obj) = value.as_object() else {
                return Ok(Err(DecodeError::Invalid(format!(
                    "Not a JSON object: {value}"
                ))));
            };
            decode_map_json_depth(runtime, inner, obj, depth + 1)
        }
        UNIT_CODEC => {
            let Some(source) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "Unit codec missing value".into(),
                )));
            };
            unit_value_exact(runtime, source)
        }
        INT_RANGE => {
            let (Some(Value::Int(min)), Some(Value::Int(max))) = (values.get(2), values.get(3))
            else {
                return Ok(Err(DecodeError::Unmodelled(
                    "ExtraCodecs.intRange missing bounds".into(),
                )));
            };
            let Some(n) = crate::recipe_models::json_number_to_i64(value).map(|n| n as i32) else {
                return Ok(Err(DecodeError::Invalid(format!("Not a number: {value}"))));
            };
            if n < *min || n > *max {
                return Ok(Err(DecodeError::Invalid(format!(
                    "Value must be within range [{min};{max}]: {n}"
                ))));
            }
            Ok(Ok(Value::Boxed(Boxed::Integer(n))))
        }
        LIST => {
            let Some(inner) = values.get(2) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "ListCodec missing element codec".into(),
                )));
            };
            let Some(arr) = value.as_array() else {
                return Ok(Err(DecodeError::Invalid(format!(
                    "Not a JSON array: {value}"
                ))));
            };
            if let (Some(Value::Int(min)), Some(Value::Int(max))) = (values.get(3), values.get(4)) {
                let len = arr.len() as i64;
                if len < i64::from(*min) || len > i64::from(*max) {
                    return Ok(Err(DecodeError::Invalid(format!(
                        "List size {len} outside [{min}; {max}]"
                    ))));
                }
            }
            let mut decoded_elems = Vec::with_capacity(arr.len());
            let mut invalid_errors = Vec::new();
            let mut first_unmodelled: Option<String> = None;
            for elem in arr {
                match decode_value_json_depth(runtime, inner, elem, depth + 1)? {
                    Ok(v) => decoded_elems.push(v),
                    Err(DecodeError::Invalid(msg)) => invalid_errors.push(msg),
                    Err(DecodeError::Unmodelled(msg)) => {
                        if first_unmodelled.is_none() {
                            first_unmodelled = Some(msg);
                        }
                    }
                }
            }
            if let Some(msg) = first_unmodelled {
                return Ok(Err(DecodeError::Unmodelled(msg)));
            }
            if !invalid_errors.is_empty() {
                return Ok(Err(DecodeError::Invalid(invalid_errors.join("; "))));
            }
            let Some(std_host) = runtime.host.as_std_host_mut() else {
                return Ok(Err(DecodeError::Unmodelled(
                    "host cannot construct decoded List".into(),
                )));
            };
            Ok(Ok(std_host.unmodifiable_values("list", decoded_elems)))
        }
        XMAP => {
            let (Some(inner), Some(to_cb)) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "Codec.xmap node missing inner or function".into(),
                )));
            };
            let decoded = match decode_value_json_depth(runtime, inner, value, depth + 1)? {
                Ok(v) => v,
                Err(err) => return Ok(Err(err)),
            };
            invoke_callback_exact(runtime, to_cb, &[decoded], "Codec.xmap")
        }
        FLAT_XMAP | COMAP_FLAT_MAP => {
            let (Some(inner), Some(to_cb)) = (values.get(2), values.get(3)) else {
                return Ok(Err(DecodeError::Unmodelled(
                    "Codec flatXmap/comapFlatMap node missing inner or function".into(),
                )));
            };
            let decoded = match decode_value_json_depth(runtime, inner, value, depth + 1)? {
                Ok(v) => v,
                Err(err) => return Ok(Err(err)),
            };
            run_data_result_callback(runtime, to_cb, decoded)
        }
        CONSTANT => {
            let (Some(Value::Ref(Some(owner))), Some(Value::Ref(Some(name)))) =
                (values.get(2), values.get(3))
            else {
                return Ok(Err(DecodeError::Unmodelled(
                    "DFU constant Codec missing owner/name".into(),
                )));
            };
            match (owner.as_str(), name.as_str()) {
                (CODEC, "BOOL") => match value.as_bool() {
                    Some(b) => Ok(Ok(Value::Boxed(Boxed::Boolean(b)))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a boolean: {value}")))),
                },
                (CODEC, "BYTE") => match crate::recipe_models::json_number_to_i64(value) {
                    Some(n) => Ok(Ok(Value::Boxed(Boxed::Byte(n as i8)))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                (CODEC, "SHORT") => match crate::recipe_models::json_number_to_i64(value) {
                    Some(n) => Ok(Ok(Value::Boxed(Boxed::Short(n as i16)))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                (CODEC, "INT") => match crate::recipe_models::json_number_to_i64(value) {
                    Some(n) => Ok(Ok(Value::Boxed(Boxed::Integer(n as i32)))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                (CODEC, "LONG") => match crate::recipe_models::json_number_to_i64(value) {
                    Some(n) => Ok(Ok(Value::Long(n))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                (CODEC, "FLOAT") => match value.as_f64() {
                    Some(n) => Ok(Ok(Value::Boxed(Boxed::Float((n as f32).to_bits())))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                (CODEC, "DOUBLE") => match value.as_f64() {
                    Some(n) => Ok(Ok(Value::Float(n))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a number: {value}")))),
                },
                ("net/minecraft/util/ExtraCodecs", "POSITIVE_INT") => {
                    let Some(n) = crate::recipe_models::json_number_to_i64(value).map(|n| n as i32)
                    else {
                        return Ok(Err(DecodeError::Invalid(format!("Not a number: {value}"))));
                    };
                    if n < 1 {
                        return Ok(Err(DecodeError::Invalid(format!(
                            "Value must be positive: {n}"
                        ))));
                    }
                    Ok(Ok(Value::Boxed(Boxed::Integer(n))))
                }
                ("net/minecraft/util/ExtraCodecs", "NON_NEGATIVE_INT") => {
                    let Some(n) = crate::recipe_models::json_number_to_i64(value).map(|n| n as i32)
                    else {
                        return Ok(Err(DecodeError::Invalid(format!("Not a number: {value}"))));
                    };
                    if n < 0 {
                        return Ok(Err(DecodeError::Invalid(format!(
                            "Value must be non-negative: {n}"
                        ))));
                    }
                    Ok(Ok(Value::Boxed(Boxed::Integer(n))))
                }
                (CODEC, "STRING") => match value.as_str() {
                    Some(s) => Ok(Ok(Value::Ref(Some(s.to_owned())))),
                    None => Ok(Err(DecodeError::Invalid(format!("Not a string: {value}")))),
                },
                ("net/minecraft/resources/Identifier", "CODEC") => match value.as_str() {
                    Some(raw) => match crate::recipe_models::normalize_id(raw) {
                        Some(id) => Ok(Ok(crate::host_ids::identifier_of_text(&id))),
                        None => Ok(Err(DecodeError::Invalid(format!(
                            "Not a valid resource location: {raw}"
                        )))),
                    },
                    None => Ok(Err(DecodeError::Invalid(format!(
                        "Not a string for Identifier: {value}"
                    )))),
                },
                ("net/minecraft/world/item/crafting/Ingredient", "CODEC") => {
                    let Some(std_host) = runtime.host.as_std_host_mut() else {
                        return Ok(Err(DecodeError::Unmodelled(
                            "host does not support Ingredient decoding".into(),
                        )));
                    };
                    Ok(crate::recipe_models::decode_ingredient_json(
                        std_host, value,
                    ))
                }
                ("net/minecraft/world/item/ItemStackTemplate", "CODEC") => {
                    let Some(std_host) = runtime.host.as_std_host_mut() else {
                        return Ok(Err(DecodeError::Unmodelled(
                            "host does not support ItemStackTemplate decoding".into(),
                        )));
                    };
                    Ok(crate::recipe_models::decode_item_stack_template_json(
                        std_host, value,
                    ))
                }
                ("net/minecraft/core/component/DataComponentPatch", "CODEC") => {
                    let Some(std_host) = runtime.host.as_std_host_mut() else {
                        return Ok(Err(DecodeError::Unmodelled(
                            "host does not support DataComponentPatch decoding".into(),
                        )));
                    };
                    Ok(
                        crate::recipe_models::decode_component_patch_json(std_host, value)
                            .map(|patch| std_host.recipes.alloc_patch(patch)),
                    )
                }
                ("net/minecraft/world/item/Item", "CODEC") => {
                    let Some(raw) = value.as_str() else {
                        return Ok(Err(DecodeError::Invalid(format!(
                            "Not a string for Item: {value}"
                        ))));
                    };
                    let Some(std_host) = runtime.host.as_std_host() else {
                        return Ok(Err(DecodeError::Unmodelled(
                            "host does not support Item decoding".into(),
                        )));
                    };
                    Ok(crate::recipe_models::decode_non_air_item_id(std_host, raw)
                        .map(|id| crate::recipe_models::item_holder_for_id(std_host, &id)))
                }
                _ => Ok(Err(DecodeError::Unmodelled(format!(
                    "constant Codec {owner}.{name}"
                )))),
            }
        }
        other => Ok(Err(DecodeError::Unmodelled(format!(
            "unsupported Codec node kind: {other}"
        )))),
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
        let mut host = crate::StdHost::new();
        let inner = constant(
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "INT",
        );
        let field = invoke(
            &mut host,
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
            &mut host,
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
        assert!(held_values(&getter).unwrap().iter().any(is_callback));
        let stream = Value::Host(HostRef::new(HostKind::StreamCodec, "stream"));
        assert!(
            invoke(
                &mut host,
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
                &mut host,
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
    fn common_fields_and_products_and_keep_builder_construction_non_null() {
        let mut host = crate::StdHost::new();
        let instance = node(&mut host, INSTANCE, Vec::new()).unwrap();
        let common = invoke(
            &mut host,
            &method(
                "net/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction",
                "commonFields",
                "(Lcom/mojang/serialization/codecs/RecordCodecBuilder$Instance;)Lcom/mojang/datafixers/Products$P1;",
            ),
            None,
            std::slice::from_ref(&instance),
        )
        .unwrap();
        let extra = node(
            &mut host,
            PRODUCT,
            vec![
                Value::Int(3),
                constant(MAP_CODEC, MAP_CODEC, "extra-1"),
                constant(MAP_CODEC, MAP_CODEC, "extra-2"),
                constant(MAP_CODEC, MAP_CODEC, "extra-3"),
            ],
        )
        .unwrap();
        let combined = invoke(
            &mut host,
            &method(
                "com/mojang/datafixers/Products$P1",
                "and",
                "(Lcom/mojang/datafixers/Products$P3;)Lcom/mojang/datafixers/Products$P4;",
            ),
            Some(&common),
            &[extra],
        )
        .unwrap();
        assert_eq!(tag(&common).as_deref(), Some(PRODUCT));
        assert_eq!(tag(&combined).as_deref(), Some(PRODUCT));
        assert!(host.unknown_host_calls().is_empty());
    }

    #[test]
    fn opaque_products_never_become_exact_products_or_apps() {
        let and = method(
            "com/mojang/datafixers/Products$P1",
            "and",
            "(Lcom/mojang/datafixers/Products$P1;)Lcom/mojang/datafixers/Products$P2;",
        );
        let apply = method(
            "com/mojang/datafixers/Products$P1",
            "apply",
            "(Ljava/util/function/Function;)Lcom/mojang/datafixers/kinds/App;",
        );
        for (receiver_opaque, argument_opaque) in [(false, true), (true, false), (true, true)] {
            let mut host = crate::StdHost::new();
            let receiver = if receiver_opaque {
                opaque_product(1)
            } else {
                node(&mut host, PRODUCT, vec![Value::Int(1), Value::Void]).unwrap()
            };
            let argument = if argument_opaque {
                opaque_product(1)
            } else {
                node(&mut host, PRODUCT, vec![Value::Int(1), Value::Void]).unwrap()
            };
            let result = invoke(&mut host, &and, Some(&receiver), &[argument]).unwrap();
            assert_eq!(tag(&result).as_deref(), Some(PRODUCT));
            assert_eq!(held_values(&result).unwrap().len(), 3);
            assert_eq!(host.unknown_host_call_count(), 1);
        }

        let mut host = crate::StdHost::new();
        let opaque = opaque_product(1);
        let result = invoke(&mut host, &apply, Some(&opaque), &[Value::Ref(None)]).unwrap();
        assert_eq!(tag(&result).as_deref(), Some(APPLIED));
        assert_eq!(host.unknown_host_call_count(), 1);
    }

    /// `Products.Pn` are records: a product equals itself, and the model counts `equals` between
    /// two distinct products rather than answering identity (their components may be equal).
    #[test]
    fn product_equals_is_identity_for_itself_and_counted_otherwise() {
        use crate::Host;
        let mut host = crate::StdHost::new();
        let instance = node(&mut host, INSTANCE, Vec::new()).unwrap();
        let common = |host: &mut crate::StdHost| {
            invoke(
                host,
                &method(
                    "net/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction",
                    "commonFields",
                    "(Lcom/mojang/serialization/codecs/RecordCodecBuilder$Instance;)Lcom/mojang/datafixers/Products$P1;",
                ),
                None,
                std::slice::from_ref(&instance),
            )
            .unwrap()
        };
        let first = common(&mut host);
        let second = common(&mut host);
        let equals = method(
            "com/mojang/datafixers/Products$P1",
            "equals",
            "(Ljava/lang/Object;)Z",
        );
        let itself = host
            .invoke_instance(&equals, &first, std::slice::from_ref(&first))
            .unwrap();
        assert_eq!(itself, Value::Int(1));
        assert!(host.unknown_host_calls().is_empty());
        let _ = host.invoke_instance(&equals, &first, &[second]);
        assert_eq!(
            host.unknown_host_call_count(),
            1,
            "{:?}",
            host.unknown_host_calls()
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
        let mut host = crate::StdHost::new();
        let mapped = invoke(
            &mut host,
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
        assert!(held_values(&mapped).unwrap().iter().any(is_callback));
    }
    #[test]
    fn per_jar_budgets_refuse_with_current_and_maximum() {
        let field_method = method(
            CODEC,
            "fieldOf",
            "(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;",
        );
        let inner = constant(
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "INT",
        );
        let mut host = crate::StdHost::new();
        for index in 0..256 {
            let name = Value::Ref(Some(format!("field-{index}")));
            assert!(invoke(&mut host, &field_method, Some(&inner), &[name]).is_some());
        }
        assert!(
            invoke(
                &mut host,
                &field_method,
                Some(&inner),
                &[Value::Ref(Some("overflow".into()))]
            )
            .is_none()
        );
        assert!(host.dfu_codec_refusals().iter().any(|text| {
            text.contains("node budget exceeded")
                && text.contains("current 256")
                && text.contains("maximum 256")
        }));

        let mut callback_host = crate::StdHost::new();
        let xmap_method = method(
            CODEC,
            "xmap",
            "(Ljava/util/function/Function;Ljava/util/function/Function;)Lcom/mojang/serialization/Codec;",
        );
        for index in 0..64 {
            let callback = callback_marker::lambda_from_text(
                "demo/Budget",
                index as u16,
                "java/util/function/Function",
                "demo/Budget#map(Ljava/lang/Object;)Ljava/lang/Object;",
                "",
            );
            assert!(
                invoke(
                    &mut callback_host,
                    &xmap_method,
                    Some(&inner),
                    &[callback.clone(), callback],
                )
                .is_some()
            );
        }
        let callback = callback_marker::lambda_from_text(
            "demo/Budget",
            64,
            "java/util/function/Function",
            "demo/Budget#map(Ljava/lang/Object;)Ljava/lang/Object;",
            "",
        );
        assert!(
            invoke(
                &mut callback_host,
                &xmap_method,
                Some(&inner),
                &[callback.clone(), callback],
            )
            .is_none()
        );
        assert!(callback_host.dfu_codec_refusals().iter().any(|text| {
            text.contains("callback-root budget exceeded")
                && text.contains("current 64")
                && text.contains("maximum 64")
        }));

        let mut name_host = crate::StdHost::new();
        let long_name = Value::Ref(Some("x".repeat(MAX_FIELD_NAME_BYTES + 1)));
        assert!(invoke(&mut name_host, &field_method, Some(&inner), &[long_name]).is_none());
        assert!(name_host.dfu_codec_refusals().iter().any(|text| {
            text.contains("field-name budget exceeded")
                && text.contains("current 257")
                && text.contains("maximum 256")
        }));
    }
}

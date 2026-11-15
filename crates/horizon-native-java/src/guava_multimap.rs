//! Exact bounded Guava multimaps used by the corpus.
//!
//! The multimap itself and each live view are host handles.  The backing map
//! and its value collections remain ordinary host collections, while the
//! synthetic fields below keep the source/view relationship reachable by the
//! heap collector.

use super::*;
use crate::host_ref::{HostKind, HostRef};

const STATE: &str = "|guava-multimap#";

fn state_key(key: &str, name: &str) -> String {
    format!("{key}{STATE}{name}")
}

fn handle_key(value: &Value) -> Option<String> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::GuavaMultimap => {
            Some(handle.key().to_owned())
        }
        _ => None,
    }
}

fn handle(key: String) -> Value {
    Value::Host(HostRef::new(HostKind::GuavaMultimap, key))
}

fn ref_text(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(value)) => Some(value),
        _ => None,
    }
}

fn put_field<H: Host>(this: &mut Runtime<'_, H>, key: &str, name: &str, value: Value) {
    this.fields.insert(state_key(key, name), value);
}

fn field<H: Host>(this: &Runtime<'_, H>, key: &str, name: &str) -> Option<Value> {
    this.fields.get(&state_key(key, name)).cloned()
}

fn kind<H: Host>(this: &Runtime<'_, H>, key: &str) -> Option<String> {
    ref_text(&field(this, key, "kind")?).map(str::to_owned)
}

fn target_key<H: Host>(this: &Runtime<'_, H>, key: &str) -> String {
    let mut current = key.to_owned();
    for _ in 0..8 {
        let Some(Value::Host(target)) = field(this, &current, "target") else {
            break;
        };
        if target.kind() != HostKind::GuavaMultimap {
            break;
        }
        current = target.key().to_owned();
    }
    current
}

fn is_view<H: Host>(this: &Runtime<'_, H>, key: &str) -> bool {
    field(this, key, "view").is_some()
}

fn value_collection_class(kind: &str) -> &'static str {
    match kind {
        "hash" => "java/util/HashSet",
        "linked" => "java/util/LinkedHashSet",
        _ => "java/util/ArrayList",
    }
}

fn invoke_instance<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Result<Value, NativeError> {
    let mut args_copy = args.to_vec();
    if let Some(value) = this.invoke_hash_collection(r, 0xb6, Some(receiver), &mut args_copy)? {
        return Ok(value);
    }
    this.host.invoke_instance(r, receiver, args)
}

fn map_ref(name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: "java/util/Map".into(),
        name: name.into(),
        descriptor: descriptor.into(),
    }
}

fn collection_ref(name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: "java/util/Collection".into(),
        name: name.into(),
        descriptor: descriptor.into(),
    }
}

fn backing<H: Host>(this: &Runtime<'_, H>, key: &str) -> Option<Value> {
    field(this, &target_key(this, key), "backing")
}

fn source_kind<H: Host>(this: &Runtime<'_, H>, key: &str) -> Option<String> {
    kind(this, &target_key(this, key))
}

fn values_for_key<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
    create: bool,
) -> Result<Option<Value>, NativeError> {
    let Some(map) = backing(this, source) else {
        return Ok(None);
    };
    let get = invoke_instance(
        this,
        &map_ref("get", "(Ljava/lang/Object;)Ljava/lang/Object;"),
        &map,
        std::slice::from_ref(key),
    )?;
    if !matches!(get, Value::Ref(None)) || !create {
        return Ok(Some(get));
    }
    let map_kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    let values = this
        .host
        .empty_mutable_collection(value_collection_class(&map_kind))
        .ok_or_else(|| NativeError::Metadata("Guava value collection is unavailable".into()))?;
    invoke_instance(
        this,
        &map_ref(
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        ),
        &map,
        &[key.clone(), values.clone()],
    )?;
    Ok(Some(values))
}

fn map_keys<H: Host>(this: &mut Runtime<'_, H>, source: &str) -> Result<Vec<Value>, NativeError> {
    let Some(map) = backing(this, source) else {
        return Ok(Vec::new());
    };
    let keys = this
        .host
        .invoke_instance(&map_ref("keySet", "()Ljava/util/Set;"), &map, &[])?;
    crate::runtime_collection_values(&*this.host, &keys)
}

fn pairs<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
) -> Result<Vec<(Value, Value)>, NativeError> {
    let Some(map) = backing(this, source) else {
        return Ok(Vec::new());
    };
    let map_kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    if map_kind == "linked"
        && let Some(order) = field(this, source, "order")
        && let Some(markers) = this.host.mutable_collection_values(&order)
    {
        let mut out = Vec::new();
        for marker in markers {
            let Ok((key, value)) = crate::map_entry_parts(&marker) else {
                continue;
            };
            if invoke_instance(
                this,
                &map_ref("containsKey", "(Ljava/lang/Object;)Z"),
                &map,
                std::slice::from_ref(&key),
            )? == Value::Int(0)
            {
                continue;
            }
            if let Some(values) = values_for_key(this, source, &key, false)? {
                let contains = invoke_instance(
                    this,
                    &collection_ref("contains", "(Ljava/lang/Object;)Z"),
                    &values,
                    std::slice::from_ref(&value),
                )?;
                if contains != Value::Int(0) {
                    out.push((key, value));
                }
            }
        }
        return Ok(out);
    }
    let mut out = Vec::new();
    for key in map_keys(this, source)? {
        let Some(values) = values_for_key(this, source, &key, false)? else {
            continue;
        };
        let _ = invoke_instance(this, &collection_ref("size", "()I"), &values, &[])?;
        for value in crate::runtime_collection_values(&*this.host, &values)? {
            out.push((key.clone(), value));
        }
    }
    Ok(out)
}

fn append_order<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
    value: &Value,
) -> Result<(), NativeError> {
    let Some(order) = field(this, source, "order") else {
        return Ok(());
    };
    invoke_instance(
        this,
        &collection_ref("add", "(Ljava/lang/Object;)Z"),
        &order,
        &[crate::map_entry_value(key, value)],
    )?;
    Ok(())
}

fn make_multimap<H: Host>(
    this: &mut Runtime<'_, H>,
    class: &str,
    kind: &str,
) -> Result<Value, NativeError> {
    let value = handle(this.new_object_key(class));
    let key = handle_key(&value).unwrap();
    put_field(this, &key, "kind", Value::Ref(Some(kind.into())));
    let map_class = if kind == "hash" {
        "java/util/HashMap"
    } else {
        "java/util/LinkedHashMap"
    };
    let map = this
        .host
        .empty_mutable_collection(map_class)
        .ok_or_else(|| NativeError::Metadata("Guava backing map unavailable".into()))?;
    put_field(this, &key, "backing", map);
    if kind == "linked" {
        let order = this
            .host
            .empty_mutable_collection("java/util/ArrayList")
            .ok_or_else(|| NativeError::Metadata("Guava order list unavailable".into()))?;
        put_field(this, &key, "order", order);
    }
    Ok(value)
}

fn make_sync<H: Host>(
    this: &mut Runtime<'_, H>,
    target: &Value,
    list: bool,
) -> Result<Value, NativeError> {
    let class = if list {
        "com/google/common/collect/Synchronized$SynchronizedListMultimap"
    } else {
        "com/google/common/collect/Synchronized$SynchronizedMultimap"
    };
    let value = handle(this.new_object_key(class));
    let key = handle_key(&value).unwrap();
    put_field(this, &key, "kind", Value::Ref(Some("sync".into())));
    put_field(this, &key, "target", target.clone());
    Ok(value)
}

fn put_one<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
    value: &Value,
) -> Result<Value, NativeError> {
    if matches!(key, Value::Ref(None)) || matches!(value, Value::Ref(None)) {
        return Err(NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: None,
            frame: None,
        });
    }
    let values = values_for_key(this, source, key, true)?.unwrap();
    let kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    let added = invoke_instance(
        this,
        &collection_ref("add", "(Ljava/lang/Object;)Z"),
        &values,
        std::slice::from_ref(value),
    )?;
    if kind == "hash" {
        this.resync_hash_table(&values)?;
    }
    if added == Value::Int(1) && kind == "linked" {
        append_order(this, source, key, value)?;
    }
    Ok(added)
}

fn remove_one<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
    value: &Value,
) -> Result<Value, NativeError> {
    let Some(values) = values_for_key(this, source, key, false)? else {
        return Ok(Value::Int(0));
    };
    let out = invoke_instance(
        this,
        &collection_ref("remove", "(Ljava/lang/Object;)Z"),
        &values,
        std::slice::from_ref(value),
    )?;
    if out == Value::Int(1)
        && invoke_instance(this, &collection_ref("isEmpty", "()Z"), &values, &[])? == Value::Int(1)
        && let Some(map) = backing(this, source)
    {
        invoke_instance(
            this,
            &map_ref("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
            &map,
            std::slice::from_ref(key),
        )?;
    }
    Ok(out)
}

pub(crate) fn invoke<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if let Some(receiver) = receiver
        && let Some(key) = handle_key(receiver)
        && (is_view(this, &key) || kind(this, &key).is_some())
    {
        if kind(this, &key).as_deref() == Some("sync") || is_view(this, &key) {
            return this.host.unknown_operand_call(r).map(Some);
        }
        return invoke_multimap(this, r, receiver, args, &key);
    }
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        ("com/google/common/collect/ArrayListMultimap", "create", _) => Ok(Some(make_multimap(
            this,
            "com/google/common/collect/ArrayListMultimap",
            "array",
        )?)),
        ("com/google/common/collect/HashMultimap", "create", _) => Ok(Some(make_multimap(
            this,
            "com/google/common/collect/HashMultimap",
            "hash",
        )?)),
        ("com/google/common/collect/LinkedHashMultimap", "create", _) => Ok(Some(make_multimap(
            this,
            "com/google/common/collect/LinkedHashMultimap",
            "linked",
        )?)),
        ("com/google/common/collect/Multimaps", "synchronizedListMultimap", _) => {
            Ok(Some(make_sync(this, &args[0], true)?))
        }
        ("com/google/common/collect/Multimaps", "synchronizedMultimap", _) => {
            Ok(Some(make_sync(this, &args[0], false)?))
        }
        _ => Ok(None),
    }
}

fn invoke_multimap<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    _receiver: &Value,
    args: &[Value],
    key: &str,
) -> Result<Option<Value>, NativeError> {
    let source = target_key(this, key);
    if args.iter().any(|value| matches!(value, Value::Ref(None))) {
        return this.host.unknown_operand_call(r).map(Some);
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("put", "(Ljava/lang/Object;Ljava/lang/Object;)Z") => {
            Ok(Some(put_one(this, &source, &args[0], &args[1])?))
        }
        ("get", "(Ljava/lang/Object;)Ljava/util/Collection;")
        | ("get", "(Ljava/lang/Object;)Ljava/util/List;")
        | ("get", "(Ljava/lang/Object;)Ljava/util/Set;") => {
            if let Some(values) = values_for_key(this, &source, &args[0], false)? {
                return Ok(Some(values));
            }
            let kind = source_kind(this, &source).unwrap_or_else(|| "array".into());
            let class = value_collection_class(&kind);
            Ok(Some(this.host.empty_mutable_collection(class).ok_or_else(
                || NativeError::Metadata("Guava value collection is unavailable".into()),
            )?))
        }
        ("containsKey", "(Ljava/lang/Object;)Z") => Ok(Some(Value::Int(
            values_for_key(this, &source, &args[0], false)?.is_some_and(|v| {
                crate::runtime_collection_values(&*this.host, &v).is_ok_and(|x| !x.is_empty())
            }) as i32,
        ))),
        ("containsEntry", "(Ljava/lang/Object;Ljava/lang/Object;)Z") => Ok(Some(Value::Int(
            values_for_key(this, &source, &args[0], false)?.is_some_and(|v| {
                this.host
                    .invoke_instance(
                        &collection_ref("contains", "(Ljava/lang/Object;)Z"),
                        &v,
                        &args[1..2],
                    )
                    .is_ok_and(|x| x == Value::Int(1))
            }) as i32,
        ))),
        ("remove", "(Ljava/lang/Object;Ljava/lang/Object;)Z") => {
            Ok(Some(remove_one(this, &source, &args[0], &args[1])?))
        }
        ("size", "()I") => Ok(Some(Value::Int(pairs(this, &source)?.len() as i32))),
        _ => this.host.unknown_operand_call(r).map(Some),
    }
}

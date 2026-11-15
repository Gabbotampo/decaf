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

fn view_class<H: Host>(this: &Runtime<'_, H>, source: &str, view: &str) -> &'static str {
    let wrapped = kind(this, source).as_deref() == Some("sync");
    match (wrapped, view, source_kind(this, source).as_deref()) {
        (true, "get", _) => "com/google/common/collect/Synchronized$SynchronizedRandomAccessList",
        (true, "keyset", _) => "com/google/common/collect/Synchronized$SynchronizedSet",
        (true, "values", _) => "com/google/common/collect/Synchronized$SynchronizedCollection",
        (true, "entries", _) => "com/google/common/collect/Synchronized$SynchronizedCollection",
        (true, "asmap", _) => "com/google/common/collect/Synchronized$SynchronizedAsMap",
        (_, "get", Some("array")) => {
            "com/google/common/collect/AbstractMapBasedMultimap$RandomAccessWrappedList"
        }
        (_, "get", Some("hash" | "linked")) => {
            "com/google/common/collect/AbstractMapBasedMultimap$WrappedSet"
        }
        (_, "keyset", _) => "com/google/common/collect/AbstractMapBasedMultimap$KeySet",
        (_, "values", _) => "com/google/common/collect/AbstractMultimap$Values",
        (_, "entries", Some("hash")) => "com/google/common/collect/AbstractMultimap$EntrySet",
        (_, "entries", _) => "com/google/common/collect/AbstractMultimap$Entries",
        (_, "asmap", _) => "com/google/common/collect/AbstractMapBasedMultimap$AsMap",
        _ => "java/util/ArrayList",
    }
}

fn make_view<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    view: &str,
    key: Option<Value>,
) -> Value {
    let class = view_class(this, source, view);
    let value = handle(this.new_object_key(class));
    let key_text = handle_key(&value).unwrap();
    put_field(this, &key_text, "source", handle(source.to_owned()));
    put_field(this, &key_text, "view", Value::Ref(Some(view.into())));
    if let Some(key) = key {
        put_field(this, &key_text, "key", key);
    }
    value
}

fn cached_view<H: Host>(this: &mut Runtime<'_, H>, source: &str, view: &str) -> Value {
    if let Some(value) = field(this, source, view) {
        return value;
    }
    let value = make_view(this, source, view, None);
    put_field(this, source, view, value.clone());
    value
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

fn view_values<H: Host>(
    this: &mut Runtime<'_, H>,
    view_key: &str,
) -> Result<Vec<Value>, NativeError> {
    let source = handle_key(&field(this, view_key, "source").unwrap()).unwrap();
    let view_value = field(this, view_key, "view").unwrap();
    let view = ref_text(&view_value).unwrap_or_default();
    let all = pairs(this, &target_key(this, &source))?;
    if view == "get" {
        let key = field(this, view_key, "key").unwrap();
        return Ok(all
            .into_iter()
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .collect());
    }
    if view == "keyset" {
        let mut out = Vec::new();
        for (key, _) in all {
            if !out.contains(&key) {
                out.push(key);
            }
        }
        return Ok(out);
    }
    if view == "values" {
        return Ok(all.into_iter().map(|(_, v)| v).collect());
    }
    if view == "entries" {
        return Ok(all
            .into_iter()
            .map(|(k, v)| crate::map_entry_value(&k, &v))
            .collect());
    }
    Ok(Vec::new())
}

fn snapshot<H: Host>(this: &mut Runtime<'_, H>, view_key: &str) -> Result<Value, NativeError> {
    let values = view_values(this, view_key)?;
    let view_value = field(this, view_key, "view").unwrap();
    let view = ref_text(&view_value).unwrap_or_default();
    let source = handle_key(&field(this, view_key, "source").unwrap()).unwrap();
    let kind = if view == "get"
        && source_kind(this, &source).as_deref() != Some("hash")
        && source_kind(this, &source).as_deref() != Some("linked")
    {
        "list"
    } else if view == "entries" && source_kind(this, &source).as_deref() == Some("hash")
        || view == "keyset"
    {
        "set"
    } else {
        "list"
    };
    Ok(crate::collection_value(kind, &values))
}

fn invoke_view<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    _receiver: &Value,
    args: &[Value],
    key: &str,
) -> Result<Option<Value>, NativeError> {
    let view_value = field(this, key, "view").unwrap();
    let view = ref_text(&view_value).unwrap_or_default();
    if view == "asmap" {
        let source = handle_key(&field(this, key, "source").unwrap()).unwrap();
        let root = target_key(this, &source);
        let pairs = pairs(this, &root)?;
        return Ok(Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("size", "()I") => Value::Int(pairs.len() as i32),
            ("isEmpty", "()Z") => Value::Int((pairs.is_empty()) as i32),
            ("containsKey", "(Ljava/lang/Object;)Z") => {
                Value::Int(pairs.iter().any(|(k, _)| *k == args[0]) as i32)
            }
            ("get", "(Ljava/lang/Object;)Ljava/util/Collection;") => {
                make_view(this, &root, "get", Some(args[0].clone()))
            }
            ("keySet", "()Ljava/util/Set;") => cached_view(this, &root, "keyset"),
            ("values", "()Ljava/util/Collection;") => cached_view(this, &root, "values"),
            ("toString", "()Ljava/lang/String;") => {
                let map = backing(this, &root).unwrap();
                return Ok(Some(
                    this.host
                        .invoke_instance(&map_ref("toString", "()Ljava/lang/String;"), &map, &[])?
                        .clone(),
                ));
            }
            _ => return Ok(None),
        }));
    }
    let values = view_values(this, key)?;
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("size", "()I") => Ok(Some(Value::Int(values.len() as i32))),
        ("isEmpty", "()Z") => Ok(Some(Value::Int(values.is_empty() as i32))),
        ("contains", "(Ljava/lang/Object;)Z") => {
            Ok(Some(Value::Int(values.contains(&args[0]) as i32)))
        }
        ("toString", "()Ljava/lang/String;") => {
            let copy = snapshot(this, key)?;
            Ok(Some(
                this.host
                    .invoke_instance(
                        &collection_ref("toString", "()Ljava/lang/String;"),
                        &copy,
                        &[],
                    )?
                    .clone(),
            ))
        }
        ("hashCode", "()I") => {
            let copy = snapshot(this, key)?;
            Ok(Some(
                this.host
                    .invoke_instance(&collection_ref("hashCode", "()I"), &copy, &[])?
                    .clone(),
            ))
        }
        ("iterator", d) if d.ends_with("Iterator;") => {
            let copy = snapshot(this, key)?;
            Ok(Some(this.invoke(
                "java/util/Collection",
                "iterator",
                "()Ljava/util/Iterator;",
                Some(copy),
                &[],
            )?))
        }
        ("add", "(Ljava/lang/Object;)Z") if view == "get" => {
            let source = handle_key(&field(this, key, "source").unwrap()).unwrap();
            let root = target_key(this, &source);
            let k = field(this, key, "key").unwrap();
            let out = put_one(this, &root, &k, &args[0])?;
            Ok(Some(out))
        }
        ("remove", "(Ljava/lang/Object;)Z") if view == "get" => {
            let source = handle_key(&field(this, key, "source").unwrap()).unwrap();
            let root = target_key(this, &source);
            let k = field(this, key, "key").unwrap();
            Ok(Some(remove_one(this, &root, &k, &args[0])?))
        }
        ("clear", "()V") if view == "get" => {
            let source = handle_key(&field(this, key, "source").unwrap()).unwrap();
            let root = target_key(this, &source);
            let k = field(this, key, "key").unwrap();
            remove_all(this, &root, &k)?;
            Ok(Some(Value::Void))
        }
        _ => Ok(None),
    }
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
    Ok(out)
}

fn remove_all<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
) -> Result<Value, NativeError> {
    let Some(map) = backing(this, source) else {
        return Ok(crate::collection_value("list", &[]));
    };
    let old = values_for_key(this, source, key, false)?
        .unwrap_or_else(|| crate::collection_value("list", &[]));
    let old_values = crate::runtime_collection_values(&*this.host, &old)?;
    invoke_instance(
        this,
        &map_ref("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
        &map,
        std::slice::from_ref(key),
    )?;
    Ok(this
        .host
        .immutable_collection(
            if source_kind(this, source).as_deref() == Some("hash") {
                "set"
            } else {
                "list"
            },
            old_values,
        )
        .unwrap_or(old))
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
        if is_view(this, &key) {
            return invoke_view(this, r, receiver, args, &key);
        }
        let root = target_key(this, &key);
        if r.name == "toString" && r.descriptor == "()Ljava/lang/String;" {
            let map = backing(this, &root).unwrap();
            return Ok(Some(invoke_instance(
                this,
                &map_ref("toString", r.descriptor.as_str()),
                &map,
                &[],
            )?));
        }
        if r.name == "hashCode" && r.descriptor == "()I" {
            let map = backing(this, &root).unwrap();
            return Ok(Some(invoke_instance(
                this,
                &map_ref("hashCode", r.descriptor.as_str()),
                &map,
                &[],
            )?));
        }
        if r.name == "equals" && r.descriptor == "(Ljava/lang/Object;)Z" {
            let Some(other_key) = args.first().and_then(handle_key) else {
                return Ok(Some(Value::Int(0)));
            };
            let other_root = target_key(this, &other_key);
            let Some(map) = backing(this, &root) else {
                return Ok(Some(Value::Int(0)));
            };
            let Some(other) = backing(this, &other_root) else {
                return Ok(Some(Value::Int(0)));
            };
            return Ok(Some(invoke_instance(
                this,
                &map_ref("equals", r.descriptor.as_str()),
                &map,
                &[other],
            )?));
        }
        return invoke_multimap(this, r, receiver, args, &key);
    }
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        ("com/google/common/collect/ArrayListMultimap", "create", _) => Ok(Some(make_multimap(
            this,
            "com/google/common/collect/ArrayListMultimap",
            "array",
        )?)),
        ("com/google/common/collect/HashMultimap", "create", _) => {
            Ok(Some(this.host.unknown_operand_call(r)?))
        }
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
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("put", "(Ljava/lang/Object;Ljava/lang/Object;)Z") => {
            Ok(Some(put_one(this, &source, &args[0], &args[1])?))
        }
        ("putAll", d) if d.starts_with("(Ljava/lang/Object;Ljava/lang/Iterable;") => {
            let values = crate::runtime_collection_values(&*this.host, &args[1])?;
            let mut changed = false;
            for value in values {
                changed |= put_one(this, &source, &args[0], &value)? == Value::Int(1);
            }
            Ok(Some(Value::Int(changed as i32)))
        }
        ("putAll", d) if d.contains("Multimap;") => {
            let Some(other) = args.first().and_then(handle_key) else {
                return Ok(None);
            };
            let other = target_key(this, &other);
            let mut changed = false;
            for (k, v) in pairs(this, &other)? {
                changed |= put_one(this, &source, &k, &v)? == Value::Int(1);
            }
            Ok(Some(Value::Int(changed as i32)))
        }
        ("get", "(Ljava/lang/Object;)Ljava/util/Collection;")
        | ("get", "(Ljava/lang/Object;)Ljava/util/List;")
        | ("get", "(Ljava/lang/Object;)Ljava/util/Set;") => {
            Ok(Some(make_view(this, key, "get", Some(args[0].clone()))))
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
        ("removeAll", "(Ljava/lang/Object;)Ljava/util/Collection;") => {
            Ok(Some(remove_all(this, &source, &args[0])?))
        }
        ("keySet", "()Ljava/util/Set;") => Ok(Some(cached_view(this, key, "keyset"))),
        ("values", "()Ljava/util/Collection;") => Ok(Some(cached_view(this, key, "values"))),
        ("entries", "()Ljava/util/Collection;") | ("entries", "()Ljava/util/Set;") => {
            Ok(Some(cached_view(this, key, "entries")))
        }
        ("asMap", "()Ljava/util/Map;") => Ok(Some(cached_view(this, key, "asmap"))),
        ("size", "()I") => Ok(Some(Value::Int(pairs(this, &source)?.len() as i32))),
        ("isEmpty", "()Z") => Ok(Some(Value::Int(pairs(this, &source)?.is_empty() as i32))),
        ("clear", "()V") => {
            if let Some(map) = backing(this, &source) {
                this.host
                    .invoke_instance(&map_ref("clear", "()V"), &map, &[])?;
            }
            Ok(Some(Value::Void))
        }
        _ => Ok(None),
    }
}

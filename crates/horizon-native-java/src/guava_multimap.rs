//! Exact bounded Guava multimaps used by the corpus.
//!
//! The multimap itself and each live view are host handles.  The backing map
//! and its value collections remain ordinary host collections, while the
//! synthetic fields below keep the source/view relationship reachable by the
//! heap collector.

use super::*;
use crate::host_ref::{HostKind, HostRef};

const STATE: &str = "|guava-multimap#";
const MAP_ITERATOR: &str = "guava-as-map-iterator|";

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

fn null_argument() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
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

fn view_name<H: Host>(this: &Runtime<'_, H>, key: &str) -> Option<String> {
    ref_text(&field(this, key, "view")?).map(str::to_owned)
}

fn cached_view<H: Host>(this: &mut Runtime<'_, H>, source: &str, name: &str, class: &str) -> Value {
    if let Some(view) = field(this, source, name) {
        return view;
    }
    let view = handle(this.new_object_key(class));
    let view_key = handle_key(&view).expect("Guava view is a handle");
    put_field(this, &view_key, "view", Value::Ref(Some(name.to_owned())));
    put_field(this, &view_key, "target", handle(source.to_owned()));
    put_field(this, source, name, view.clone());
    view
}

fn wrapped_value_class(kind: &str) -> &'static str {
    match kind {
        "array" => "com/google/common/collect/AbstractMapBasedMultimap$RandomAccessWrappedList",
        _ => "com/google/common/collect/AbstractMapBasedMultimap$WrappedSet",
    }
}

fn fresh_wrapped_value<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
    view: &str,
    class: &str,
) -> Value {
    let value = handle(this.new_object_key(class));
    let value_key = handle_key(&value).expect("Guava wrapped value is a handle");
    put_field(this, &value_key, "view", Value::Ref(Some(view.into())));
    put_field(this, &value_key, "target", handle(source.to_owned()));
    put_field(this, &value_key, "map-key", key.clone());
    value
}

/// `Multimap.asMap().get(key)` is a fresh wrapped value on every call. It is
/// a live view over one key, rather than the cached collection returned by
/// `multimap.get(key)`.
fn fresh_as_map_value<H: Host>(this: &mut Runtime<'_, H>, source: &str, key: &Value) -> Value {
    let kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    fresh_wrapped_value(this, source, key, "asMap.get", wrapped_value_class(&kind))
}

fn fresh_get_value<H: Host>(this: &mut Runtime<'_, H>, source: &str, key: &Value) -> Value {
    let kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    fresh_wrapped_value(this, source, key, "get", wrapped_value_class(&kind))
}

fn tainted<H: Host>(this: &Runtime<'_, H>, source: &str) -> bool {
    matches!(field(this, source, "tainted"), Some(Value::Int(1)))
}

fn taint<H: Host>(this: &mut Runtime<'_, H>, source: &str) {
    put_field(this, source, "tainted", Value::Int(1));
}

fn is_multimap_write(r: &MethodRef) -> bool {
    matches!(
        r.name.as_str(),
        "add" | "clear" | "put" | "remove" | "replaceValues"
    )
}

fn count_tainted_read<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    source: &str,
) -> Option<Result<Option<Value>, NativeError>> {
    (tainted(this, source) && !is_multimap_write(r))
        .then(|| this.host.unknown_operand_call(r).map(Some))
}

fn fresh_as_map_view<H: Host>(this: &mut Runtime<'_, H>, source: &str, name: &str) -> Value {
    let class = match name {
        "asMap.entrySet" => "java/util/AbstractMap$SimpleEntrySet",
        "asMap.keySet" => "java/util/AbstractMap$KeySet",
        _ => "java/util/AbstractMap$Values",
    };
    let view = handle(this.new_object_key(class));
    let view_key = handle_key(&view).expect("Guava asMap view is a handle");
    put_field(this, &view_key, "view", Value::Ref(Some(name.into())));
    put_field(this, &view_key, "target", handle(source.to_owned()));
    view
}

fn value_collection_class(kind: &str) -> &'static str {
    match kind {
        "hash" => "java/util/HashSet",
        "linked" => "java/util/LinkedHashSet",
        _ => "java/util/ArrayList",
    }
}

fn backing_generation<H: Host>(this: &Runtime<'_, H>, source: &str) -> Option<u64> {
    let Value::Host(map) = backing(this, source)? else {
        return None;
    };
    this.host.collection_generation(map.key_text())
}

fn clone_values<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    values: &Value,
) -> Result<Value, NativeError> {
    let kind = source_kind(this, source).unwrap_or_else(|| "array".into());
    let copy = this
        .host
        .empty_mutable_collection(value_collection_class(&kind))
        .ok_or_else(|| NativeError::Metadata("Guava detached collection unavailable".into()))?;
    for value in runtime_collection_values(&*this.host, values)? {
        invoke_instance(
            this,
            &collection_ref("add", "(Ljava/lang/Object;)Z"),
            &copy,
            &[value],
        )?;
    }
    Ok(copy)
}

fn remove_key<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    key: &Value,
) -> Result<Option<Value>, NativeError> {
    let Some(values) = values_for_key(this, source, key, false)? else {
        return Ok(None);
    };
    let detached = clone_values(this, source, &values)?;
    if let Some(map) = backing(this, source) {
        invoke_instance(
            this,
            &map_ref("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
            &map,
            std::slice::from_ref(key),
        )?;
    }
    // AbstractMapBasedMultimap clears the original collection after copying it;
    // handles returned by get(key) therefore remain live, but become empty.
    invoke_instance(this, &collection_ref("clear", "()V"), &values, &[])?;
    Ok(Some(detached))
}

fn map_iterator_source<H: Host>(
    this: &Runtime<'_, H>,
    receiver: &Value,
) -> Option<(String, String, u64, bool)> {
    let marker = crate::host_iterator::marker(receiver)?;
    let source = this.host.host_iterator(marker)?.2?;
    let rest = source.strip_prefix(MAP_ITERATOR)?;
    let (source, rest) = rest.split_once("|view=")?;
    let (view, generation) = rest.rsplit_once("|generation=")?;
    let removed = generation.strip_suffix("|removed").is_some();
    let generation = generation
        .strip_suffix("|removed")
        .unwrap_or(generation)
        .parse()
        .ok()?;
    Some((source.to_owned(), view.to_owned(), generation, removed))
}

fn invoke_map_iterator<H: Host>(
    this: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: &Value,
) -> Result<Option<Value>, NativeError> {
    let Some(marker) = crate::host_iterator::marker(receiver).cloned() else {
        return Ok(None);
    };
    let Some((source, view, expected, removed)) = map_iterator_source(this, receiver) else {
        return Ok(None);
    };
    if !matches!(
        r.owner.as_str(),
        "java/util/Iterator" | "java/util/ListIterator"
    ) {
        return Ok(None);
    }
    let (cursor, length, _) = this
        .host
        .host_iterator(&marker)
        .ok_or_else(|| NativeError::Metadata("unknown Guava asMap iterator".into()))?;
    let current = backing_generation(this, &source).unwrap_or(expected);
    if current != expected && matches!(r.name.as_str(), "hasNext" | "next" | "remove") {
        return Err(NativeError::UncaughtException {
            class: "java/util/ConcurrentModificationException".into(),
            message: None,
            frame: None,
        });
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("hasNext", "()Z") => Ok(Some(Value::Int((cursor < length) as i32))),
        ("next", "()Ljava/lang/Object;") => {
            let value = this.host.host_iterator_next(&marker)?;
            let generation = backing_generation(this, &source).unwrap_or(current);
            this.host.set_host_iterator_source(
                &marker,
                format!("{MAP_ITERATOR}{source}|view={view}|generation={generation}"),
            );
            Ok(Some(value))
        }
        ("remove", "()V") => {
            if cursor == 0 || removed {
                return Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalStateException".into(),
                    message: None,
                    frame: None,
                });
            }
            let values = this.host.host_iterator_values(&marker).ok_or_else(|| {
                NativeError::Metadata("Guava asMap iterator values missing".into())
            })?;
            let last = values[cursor - 1].clone();
            let map_key = match view.as_str() {
                "keySet" => Some(last),
                "entrySet" => this.host.map_entry_parts(&last).map(|(key, _)| key),
                "values" => {
                    let mut found = None;
                    for key in map_keys(this, &source)? {
                        if let Some(collection) = values_for_key(this, &source, &key, false)?
                            && crate::host_map_entry::same_reference(&collection, &last)
                                == Some(true)
                        {
                            found = Some(key);
                            break;
                        }
                    }
                    found
                }
                _ => None,
            };
            let Some(map_key) = map_key else {
                return this.host.unknown_operand_call(r).map(Some);
            };
            remove_key(this, &source, &map_key)?;
            let generation = backing_generation(this, &source).unwrap_or(current);
            this.host.set_host_iterator_source(
                &marker,
                format!("{MAP_ITERATOR}{source}|view={view}|generation={generation}|removed"),
            );
            Ok(Some(Value::Void))
        }
        _ => Ok(None),
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

fn collection_contents_equal<H: Host>(
    this: &mut Runtime<'_, H>,
    left: &Value,
    right: &Value,
    set: bool,
) -> Result<bool, NativeError> {
    let left = runtime_collection_values(&*this.host, left)?;
    let right = runtime_collection_values(&*this.host, right)?;
    if !set {
        return Ok(left.len() == right.len()
            && left
                .iter()
                .zip(right.iter())
                .all(|(a, b)| this.java_equals(a, b).unwrap_or(false)));
    }
    Ok(left.len() == right.len()
        && left.iter().all(|a| {
            right
                .iter()
                .any(|b| this.java_equals(a, b).unwrap_or(false))
        }))
}

fn as_map_entry_matches<H: Host>(
    this: &mut Runtime<'_, H>,
    source: &str,
    map_key: &Value,
    collection: &Value,
    candidate: &Value,
) -> Result<bool, NativeError> {
    let Some((entry_key, entry_value)) = this.host.map_entry_parts(candidate) else {
        return Ok(false);
    };
    if !this.java_equals(map_key, &entry_key).unwrap_or(false) {
        return Ok(false);
    }
    let candidate_values = if let Value::Host(handle) = &entry_value {
        if view_name(this, handle.key()) == Some("asMap.get".into()) {
            let candidate_source = target_key(this, handle.key());
            let Some(key) = field(this, handle.key(), "map-key") else {
                return Ok(false);
            };
            values_for_key(this, &candidate_source, &key, false)?.unwrap_or(Value::Ref(None))
        } else {
            entry_value
        }
    } else {
        entry_value
    };
    let set = matches!(
        source_kind(this, source).as_deref(),
        Some("hash" | "linked")
    );
    collection_contents_equal(this, collection, &candidate_values, set)
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
    if !matches!(get, Value::Ref(None)) {
        return Ok(Some(get));
    }
    if !create {
        return Ok(None);
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
    let keys = invoke_instance(this, &map_ref("keySet", "()Ljava/util/Set;"), &map, &[])?;
    crate::runtime_collection_values(&*this.host, &keys)
}

fn size<H: Host>(this: &mut Runtime<'_, H>, source: &str) -> Result<i32, NativeError> {
    let mut total = 0;
    for key in map_keys(this, source)? {
        let Some(values) = values_for_key(this, source, &key, false)? else {
            continue;
        };
        if let Value::Int(count) =
            invoke_instance(this, &collection_ref("size", "()I"), &values, &[])?
        {
            total += count;
        }
    }
    Ok(total)
}

fn make_multimap<H: Host>(
    this: &mut Runtime<'_, H>,
    class: &str,
    kind: &str,
) -> Result<Value, NativeError> {
    let value = handle(this.new_object_key(class));
    let key = handle_key(&value).unwrap();
    put_field(this, &key, "kind", Value::Ref(Some(kind.into())));
    // ArrayListMultimap uses a HashMap for keys; only the linked variant
    // preserves insertion order.
    let map_class = if kind == "linked" {
        "java/util/LinkedHashMap"
    } else {
        "java/util/HashMap"
    };
    let map = this
        .host
        .empty_mutable_collection(map_class)
        .ok_or_else(|| NativeError::Metadata("Guava backing map unavailable".into()))?;
    put_field(this, &key, "backing", map);
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
    if let Some(Value::Host(handle)) = receiver
        && handle.kind() == HostKind::GuavaMultimap
        && r.name == "getClass"
        && r.descriptor == "()Ljava/lang/Class;"
    {
        return Ok(handle.class().map(crate::host_class::class_value));
    }
    if let Some(receiver) = receiver
        && map_iterator_source(this, receiver).is_some()
    {
        return invoke_map_iterator(this, r, receiver);
    }
    if let Some(receiver) = receiver
        && let Some(key) = handle_key(receiver)
        && (is_view(this, &key) || kind(this, &key).is_some())
    {
        if kind(this, &key).as_deref() == Some("sync") {
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
    let view = view_name(this, key);
    if let Some(counted) = count_tainted_read(this, r, &source) {
        return counted;
    }
    if matches!(view.as_deref(), Some("asMap.get" | "get")) {
        let Some(map_key) = field(this, key, "map-key") else {
            return this.host.unknown_operand_call(r).map(Some);
        };
        let current = values_for_key(this, &source, &map_key, false)?;
        let value = match (r.name.as_str(), r.descriptor.as_str()) {
            ("size", "()I") => Value::Int(
                current
                    .as_ref()
                    .map(|values| {
                        invoke_instance(this, &collection_ref("size", "()I"), values, &[])
                    })
                    .transpose()?
                    .and_then(|value| match value {
                        Value::Int(size) => Some(size),
                        _ => None,
                    })
                    .unwrap_or(0),
            ),
            ("isEmpty", "()Z") => Value::Int(i32::from(current.as_ref().is_none_or(|values| {
                invoke_instance(this, &collection_ref("isEmpty", "()Z"), values, &[])
                    .is_ok_and(|value| value == Value::Int(1))
            }))),
            ("contains", "(Ljava/lang/Object;)Z") => {
                Value::Int(i32::from(current.as_ref().is_some_and(|values| {
                    args.first().is_some_and(|needle| {
                        invoke_instance(
                            this,
                            &collection_ref("contains", "(Ljava/lang/Object;)Z"),
                            values,
                            std::slice::from_ref(needle),
                        )
                        .is_ok_and(|value| value == Value::Int(1))
                    })
                })))
            }
            ("add", "(Ljava/lang/Object;)Z") => {
                let value = args.first().ok_or_else(null_argument)?;
                put_one(this, &source, &map_key, value)?
            }
            ("remove", "(Ljava/lang/Object;)Z") => {
                let value = args.first().ok_or_else(null_argument)?;
                remove_one(this, &source, &map_key, value)?
            }
            ("clear", "()V") => {
                if let Some(map) = backing(this, &source) {
                    invoke_instance(
                        this,
                        &map_ref("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
                        &map,
                        std::slice::from_ref(&map_key),
                    )?;
                }
                Value::Void
            }
            ("iterator", "()Ljava/util/Iterator;")
            | ("subList", "(II)Ljava/util/List;")
            | ("listIterator", "()Ljava/util/ListIterator;")
            | ("listIterator", "(I)Ljava/util/ListIterator;") => {
                // Guava's WrappedIterator and WrappedListIterator are live and check the
                // multimap generation. A host snapshot cannot answer those semantics safely;
                // count the operation and taint later multimap reads instead of guessing.
                taint(this, &source);
                return this.host.unknown_operand_call(r).map(Some);
            }
            ("get", "(I)Ljava/lang/Object;") => {
                let Some(values) = current else {
                    return this.host.unknown_operand_call(r).map(Some);
                };
                if r.name == "listIterator" {
                    let start = args
                        .first()
                        .and_then(|value| match value {
                            Value::Int(value) => usize::try_from(*value).ok(),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let iterator = this
                        .host
                        .new_host_iterator(runtime_collection_values(&*this.host, &values)?, None)
                        .ok_or_else(|| {
                            NativeError::Metadata("Guava list iterator unavailable".into())
                        })?;
                    if let Some(marker) = crate::host_iterator::marker(&iterator) {
                        this.host.set_host_iterator_cursor(marker, start);
                    }
                    return Ok(Some(iterator));
                }
                return invoke_instance(this, r, &values, args).map(Some);
            }
            ("toString", "()Ljava/lang/String;") => {
                let values = current
                    .map(|value| runtime_collection_values(&*this.host, &value))
                    .transpose()?
                    .unwrap_or_default();
                Value::Ref(Some(format!(
                    "[{}]",
                    values.iter().map(value_text).collect::<Vec<_>>().join(", ")
                )))
            }
            _ => return this.host.unknown_operand_call(r).map(Some),
        };
        return Ok(Some(value));
    }
    if matches!(
        view.as_deref(),
        Some("asMap.entrySet" | "asMap.keySet" | "asMap.values")
    ) {
        let keys = map_keys(this, &source)?;
        let values = match view.as_deref() {
            Some("asMap.keySet") => keys.clone(),
            Some("asMap.values") => keys
                .iter()
                .filter_map(|candidate| {
                    values_for_key(this, &source, candidate, false)
                        .ok()
                        .flatten()
                })
                .collect(),
            _ => {
                let map_view = handle(key.to_owned());
                let mut entries = Vec::new();
                for candidate in keys {
                    if values_for_key(this, &source, &candidate, false)?.is_some() {
                        // Guava's AsMap wraps each backing collection and returns it through
                        // `Maps.immutableEntry`: the entry is a SimpleImmutableEntry whose value is
                        // this live wrapped collection.  It is immutable (`setValue` throws),
                        // while the wrapped collection continues to follow the multimap key.
                        let collection = fresh_as_map_value(this, &source, &candidate);
                        entries.push(this.host.mint_map_entry(
                            Some(&map_view),
                            &candidate,
                            &collection,
                            crate::host_map_entry::EntryMade::Snapshot,
                        ));
                    }
                }
                entries
            }
        };
        let value = match (r.name.as_str(), r.descriptor.as_str()) {
            ("toString", "()Ljava/lang/String;")
                if matches!(view.as_deref(), Some("asMap.values" | "asMap.entrySet")) =>
            {
                return this.host.unknown_operand_call(r).map(Some);
            }
            ("contains", "(Ljava/lang/Object;)Z") | ("remove", "(Ljava/lang/Object;)Z")
                if view.as_deref() == Some("asMap.values") =>
            {
                return this.host.unknown_operand_call(r).map(Some);
            }
            ("size", "()I") => Value::Int(values.len() as i32),
            ("isEmpty", "()Z") => Value::Int(i32::from(values.is_empty())),
            ("contains", "(Ljava/lang/Object;)Z") => {
                let found = if view.as_deref() == Some("asMap.entrySet") {
                    args.first().is_some_and(|needle| {
                        map_keys(this, &source)
                            .unwrap_or_default()
                            .into_iter()
                            .any(|map_key| {
                                values_for_key(this, &source, &map_key, false)
                                    .ok()
                                    .flatten()
                                    .is_some_and(|collection| {
                                        as_map_entry_matches(
                                            this,
                                            &source,
                                            &map_key,
                                            &collection,
                                            needle,
                                        )
                                        .unwrap_or(false)
                                    })
                            })
                    })
                } else {
                    args.first().is_some_and(|needle| {
                        values
                            .iter()
                            .any(|value| this.java_equals(value, needle).unwrap_or(false))
                    })
                };
                Value::Int(i32::from(found))
            }
            ("toString", "()Ljava/lang/String;") => Value::Ref(Some(format!(
                "[{}]",
                values.iter().map(value_text).collect::<Vec<_>>().join(", ")
            ))),
            ("iterator", "()Ljava/util/Iterator;") => {
                return this
                    .host
                    .new_host_iterator(
                        values,
                        Some(format!(
                            "{MAP_ITERATOR}{source}|view={}|generation={}",
                            view.as_deref().unwrap().strip_prefix("asMap.").unwrap(),
                            backing_generation(this, &source).unwrap_or(0)
                        )),
                    )
                    .map_or_else(
                        || this.host.unknown_operand_call(r).map(Some),
                        |iterator| Ok(Some(iterator)),
                    );
            }
            ("remove", "(Ljava/lang/Object;)Z") if view.as_deref() == Some("asMap.keySet") => {
                let Some(candidate) = args.first() else {
                    return Ok(Some(Value::Int(0)));
                };
                let present = values_for_key(this, &source, candidate, false)?.is_some();
                if present {
                    remove_key(this, &source, candidate)?;
                }
                Value::Int(i32::from(present))
            }
            ("remove", "(Ljava/lang/Object;)Z") => {
                let Some(candidate) = args.first() else {
                    return Ok(Some(Value::Int(0)));
                };
                let mut removed = false;
                for map_key in map_keys(this, &source)? {
                    let Some(collection) = values_for_key(this, &source, &map_key, false)? else {
                        continue;
                    };
                    let matches = if view.as_deref() == Some("asMap.entrySet") {
                        as_map_entry_matches(this, &source, &map_key, &collection, candidate)?
                    } else {
                        this.java_equals(&collection, candidate).unwrap_or(false)
                    };
                    if matches {
                        remove_key(this, &source, &map_key)?;
                        removed = true;
                        break;
                    }
                }
                Value::Int(i32::from(removed))
            }
            ("clear", "()V") => {
                if let Some(map) = backing(this, &source) {
                    for candidate in map_keys(this, &source)? {
                        invoke_instance(
                            this,
                            &map_ref("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
                            &map,
                            std::slice::from_ref(&candidate),
                        )?;
                    }
                }
                Value::Void
            }
            _ => return this.host.unknown_operand_call(r).map(Some),
        };
        return Ok(Some(value));
    }
    if view.as_deref() == Some("asMap") {
        let keys = map_keys(this, &source)?;
        let mut present = |candidate: &Value| {
            values_for_key(this, &source, candidate, false)
                .ok()
                .flatten()
                .is_some()
        };
        let value = match (r.name.as_str(), r.descriptor.as_str()) {
            ("size", "()I") => {
                Value::Int(keys.iter().filter(|candidate| present(candidate)).count() as i32)
            }
            ("isEmpty", "()Z") => Value::Int(i32::from(!keys.iter().any(present))),
            ("containsKey", "(Ljava/lang/Object;)Z") => {
                Value::Int(i32::from(args.first().is_some_and(present)))
            }
            ("get", "(Ljava/lang/Object;)Ljava/lang/Object;") => args
                .first()
                .and_then(|candidate| {
                    values_for_key(this, &source, candidate, false)
                        .ok()
                        .flatten()
                        .map(|_| fresh_as_map_value(this, &source, candidate))
                })
                .unwrap_or(Value::Ref(None)),
            ("remove", "(Ljava/lang/Object;)Ljava/lang/Object;") => {
                let Some(candidate) = args.first() else {
                    return Ok(Some(Value::Ref(None)));
                };
                remove_key(this, &source, candidate)?.unwrap_or(Value::Ref(None))
            }
            ("entrySet", "()Ljava/util/Set;") => fresh_as_map_view(this, &source, "asMap.entrySet"),
            ("keySet", "()Ljava/util/Set;") => fresh_as_map_view(this, &source, "asMap.keySet"),
            ("values", "()Ljava/util/Collection;") => {
                let mut all = Vec::new();
                for candidate in keys {
                    if let Some(collection) = values_for_key(this, &source, &candidate, false)? {
                        all.push(collection);
                    }
                }
                fresh_as_map_view(this, &source, "asMap.values")
            }
            _ => return this.host.unknown_operand_call(r).map(Some),
        };
        return Ok(Some(value));
    }
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
            let _ = values_for_key(this, &source, &args[0], true)?;
            Ok(Some(fresh_get_value(this, &source, &args[0])))
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
        ("size", "()I") => Ok(Some(Value::Int(size(this, &source)?))),
        ("keySet", "()Ljava/util/Set;") => Ok(Some(cached_view(
            this,
            &source,
            "keySet",
            "com/google/common/collect/AbstractMapBasedMultimap$KeySet",
        ))),
        ("values", "()Ljava/util/Collection;") => Ok(Some(cached_view(
            this,
            &source,
            "values",
            "com/google/common/collect/AbstractMultimap$Values",
        ))),
        ("asMap", "()Ljava/util/Map;") => Ok(Some(cached_view(
            this,
            &source,
            "asMap",
            "com/google/common/collect/AbstractMapBasedMultimap$AsMap",
        ))),
        _ => this.host.unknown_operand_call(r).map(Some),
    }
}

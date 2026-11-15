//! Exact bounded model of `Collections.newSetFromMap` over the host's
//! existing mutable collection handles.

use crate::{MethodRef, NativeError, StdHost, Value, expect_ref};

const SET_CLASS: &str = "java/util/Collections$SetFromMap";

impl StdHost {
    /// The JDK's live `SetFromMap` factory. Unsupported map implementations
    /// stay counted; the existing mutable map table remains the source of
    /// truth for supported views.
    pub(crate) fn new_set_from_map(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        if r.owner != "java/util/Collections"
            || r.name != "newSetFromMap"
            || r.descriptor != "(Ljava/util/Map;)Ljava/util/Set;"
        {
            return None;
        }
        let Some(map) = args.first().and_then(expect_ref) else {
            return Some(Err(NativeError::UncaughtException {
                class: "java/lang/NullPointerException".into(),
                message: Some(
                    "Cannot invoke \"java.util.Map.isEmpty()\" because \"map\" is null".into(),
                ),
                frame: None,
            }));
        };
        let values = self.mutable_collections.get(map)?;
        if !values.is_empty() {
            return Some(Err(NativeError::UncaughtException {
                class: "java/lang/IllegalArgumentException".into(),
                message: Some("Map is non-empty".into()),
                frame: None,
            }));
        }
        let class = map.split_once('#').map_or(map.as_str(), |(class, _)| class);
        if matches!(
            class,
            "java/util/WeakHashMap"
                | "java/util/concurrent/ConcurrentHashMap"
                | "java/util/concurrent/ConcurrentSkipListMap"
        ) {
            return None;
        }
        let marker = crate::jar_object::collection_key_for(
            SET_CLASS,
            crate::jar_object::CollectionFamily::Neutral,
            self.next_set_from_map as usize,
        );
        self.next_set_from_map = self.next_set_from_map.saturating_add(1);
        self.mutable_collections.insert(marker.clone(), Vec::new());
        self.set_from_map_views
            .insert(marker.clone(), map.to_owned());
        Some(Ok(crate::jar_object::collection(marker)))
    }

    /// Mutators that SetFromMap must route to its backing map. Reads and
    /// iteration use `set_from_map_values` through `mutable_values`.
    pub(crate) fn set_from_map_member(
        &mut self,
        r: &MethodRef,
        marker: &str,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        let map = self.set_from_map_views.get(marker)?.clone();
        let identity = map
            .split_once('#')
            .map(|(class, _)| class == "java/util/IdentityHashMap")
            .unwrap_or(false);
        let values = self.mutable_collections.get_mut(&map)?;
        let identity_equal = |left: &Value, right: &Value| -> Option<bool> {
            match (left, right) {
                (Value::Host(left), Value::Host(right)) => left.same_object(right),
                (Value::Ref(None), Value::Ref(None)) => Some(true),
                _ => None,
            }
        };
        let position = |values: &[Value], key: &Value| -> Option<Option<usize>> {
            if identity {
                for (index, pair) in values.chunks_exact(2).enumerate() {
                    match identity_equal(&pair[0], key) {
                        Some(true) => return Some(Some(index)),
                        Some(false) => {}
                        None => return None,
                    }
                }
                Some(None)
            } else {
                Some(values.chunks_exact(2).position(|pair| pair[0] == *key))
            }
        };
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("add", "(Ljava/lang/Object;)Z") => {
                let key = args.first()?.clone();
                let Some(position) = position(values, &key) else {
                    return Some(self.unknown_host_call(r));
                };
                if position.is_some() {
                    return Some(Ok(Value::Int(0)));
                }
                values.extend([key, Value::Int(1)]);
                Some(Ok(Value::Int(1)))
            }
            ("remove", "(Ljava/lang/Object;)Z") => {
                let key = args.first()?;
                let Some(position) = position(values, key) else {
                    return Some(self.unknown_host_call(r));
                };
                let Some(index) = position else {
                    return Some(Ok(Value::Int(0)));
                };
                values.drain(index * 2..index * 2 + 2);
                Some(Ok(Value::Int(1)))
            }
            ("clear", "()V") => {
                values.clear();
                Some(Ok(Value::Void))
            }
            _ => None,
        }
    }
}

pub(crate) fn known_set_hash(value: &Value) -> Option<i32> {
    match value {
        Value::Int(value) => Some(*value),
        Value::Long(value) => Some((*value ^ ((*value as u64) >> 32) as i64) as i32),
        Value::Float(value) => Some(value.to_bits() as i32),
        Value::Ref(None) => Some(0),
        Value::Ref(Some(marker)) if !marker.contains('#') && !marker.contains('/') => {
            Some(crate::jdk_model::java_string_hash(marker))
        }
        _ => None,
    }
}

pub(crate) fn is_set_value(value: &Value) -> bool {
    let Some(marker) = expect_ref(value) else {
        return false;
    };
    let class = marker
        .split_once('#')
        .map_or(marker.as_str(), |(class, _)| class);
    class == "set" || crate::is_set_owner(class)
}

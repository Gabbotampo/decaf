//! `Arrays.asList`: a fixed-size `java.util.Arrays$ArrayList` view over its
//! array (#156 P14d2).
//!
//! It used to answer the text literal `list/a,b`: a copy (a write to the
//! array or through the list never reached the other), as growable as an
//! `ArrayList`, and equal to every other literal with the same text.
//!
//! The view is now a collection handle of class `Arrays$ArrayList`, one per
//! call (two views of one array are two objects that share it). Its elements
//! sit in the host's collection table like any list's, mirroring the array,
//! which lives in the runtime during a call:
//!
//! - **array → views:** after the runtime writes a viewed array (`aastore`,
//!   `System.arraycopy`, `Arrays.fill`, a `toArray(T[])` fill), it pushes the
//!   array into every view of it ([`StdHost::array_written`]);
//! - **view → array:** the collection table watches the views' rows. Before
//!   the runtime reads or writes an array or makes a call, it pulls the
//!   elements of every view written since its last sync into the array, and
//!   from there into the array's other views
//!   ([`StdHost::changed_array_views`]). While no view was written, this is
//!   one counter comparison.
//!
//! The structural members throw `UnsupportedOperationException` exactly where
//! JDK 25's `AbstractList`/`AbstractCollection` reach `add` or `remove`
//! ([`StdHost::fixed_size_member`]); `set`, `sort` and `replaceAll` change
//! the array.

use std::collections::{BTreeMap, BTreeSet};

use crate::runtime::{MethodRef, NativeError, StdHost, Value, expect_ref};

/// The class Java 25's `Arrays.asList` answers.
pub(crate) const CLASS: &str = "java/util/Arrays$ArrayList";

/// Which arrays the host's views mirror.
#[derive(Clone, Debug, Default)]
pub(crate) struct ArrayViews {
    /// A view's marker → its array's key, and its row's generation when it
    /// last matched the array.
    views: BTreeMap<String, (String, u64)>,
    /// An array's key → its views.
    by_array: BTreeMap<String, BTreeSet<String>>,
    /// The collection table's watched writes at the last pull.
    pulled: u64,
}

fn unsupported() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/UnsupportedOperationException".into(),
        message: None,
        frame: None,
    }
}

impl StdHost {
    /// A new view of the array `array`, holding its `values` now.
    pub(crate) fn new_array_view(&mut self, array: &str, values: Vec<Value>) -> Value {
        let view = self.new_collection(CLASS, values);
        if let Some(marker) = expect_ref(&view).cloned() {
            self.mutable_collections.watch(&marker);
            let synced = self.mutable_collections.generation(&marker);
            self.array_views
                .by_array
                .entry(array.to_owned())
                .or_default()
                .insert(marker.clone());
            self.array_views
                .views
                .insert(marker, (array.to_owned(), synced));
        }
        view
    }

    pub(crate) fn has_array_views_of(&self, array: &str) -> bool {
        self.array_views.by_array.contains_key(array)
    }

    /// The runtime wrote the array `array`, which now holds `values`: every
    /// view of it holds them too (a view of another length is no view of it
    /// any more, and is left alone).
    pub(crate) fn array_written(&mut self, array: &str, values: &[Value]) {
        let Some(views) = self.array_views.by_array.get(array).cloned() else {
            return;
        };
        let current = self.array_views.pulled == self.mutable_collections.watched_writes();
        for view in views {
            if let Some(row) = self.mutable_collections.get_mut(&view)
                && row.len() == values.len()
            {
                row.clone_from_slice(values);
            }
            let synced = self.mutable_collections.generation(&view);
            if let Some(entry) = self.array_views.views.get_mut(&view) {
                entry.1 = synced;
            }
        }
        // What this push wrote needs no pull.
        if current {
            self.array_views.pulled = self.mutable_collections.watched_writes();
        }
    }

    /// Each array one of whose views was written since it last matched it,
    /// with that view's elements, which the array's other views now hold
    /// too. Empty, at the cost of one comparison, while no view was written.
    pub(crate) fn changed_array_views(&mut self) -> Vec<(String, Vec<Value>)> {
        if self.array_views.pulled == self.mutable_collections.watched_writes() {
            return Vec::new();
        }
        let changed: Vec<(String, String)> = self
            .array_views
            .views
            .iter()
            .filter(|(view, (_, synced))| {
                self.mutable_collections.contains_key(*view)
                    && self.mutable_collections.generation(view) != *synced
            })
            .map(|(view, (array, _))| (view.clone(), array.clone()))
            .collect();
        let mut pulled: Vec<(String, Vec<Value>)> = Vec::new();
        for (view, array) in changed {
            if pulled.iter().any(|(done, _)| *done == array) {
                continue;
            }
            let Some(values) = self.mutable_collections.get(&view).cloned() else {
                continue;
            };
            self.array_written(&array, &values);
            pulled.push((array, values));
        }
        self.array_views.pulled = self.mutable_collections.watched_writes();
        pulled
    }

    /// A structural member of a view, answered as JDK 25's `Arrays$ArrayList`
    /// (`AbstractList`/`AbstractCollection`) answers it: `UnsupportedOperationException`
    /// where it would add or remove, else what it found to do (nothing). `None`
    /// for every other member, and for a receiver that is no view.
    pub(crate) fn fixed_size_member(
        &mut self,
        r: &MethodRef,
        marker: &str,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        if !self.array_views.views.contains_key(marker) {
            return None;
        }
        let size = self.mutable_collections.get(marker).map_or(0, Vec::len);
        let found = |host: &mut Self, needles: &Value| -> Result<bool, NativeError> {
            let mine = host
                .mutable_collections
                .get(marker)
                .cloned()
                .unwrap_or_default();
            let theirs = crate::runtime::host_collection_values(host, needles)?;
            Ok(mine.iter().any(|value| {
                theirs.iter().any(|needle| {
                    crate::runtime::same_items(std::slice::from_ref(value), std::slice::from_ref(needle))
                })
            }))
        };
        let answer = match (r.name.as_str(), r.descriptor.as_str()) {
            // `AbstractList.add`/`add(int, E)`/`remove(int)`, and
            // `List.addFirst`/`addLast` (`add(0, e)`/`add(e)`): always.
            ("add", _) | ("addFirst" | "addLast", _) => Err(unsupported()),
            ("remove", "(I)Ljava/lang/Object;") => Err(unsupported()),
            // `List.removeFirst`/`removeLast`: an empty list throws first.
            ("removeFirst" | "removeLast", "()Ljava/lang/Object;") => Err(if size == 0 {
                NativeError::UncaughtException {
                    class: "java/util/NoSuchElementException".into(),
                    message: None,
                    frame: None,
                }
            } else {
                unsupported()
            }),
            // `AbstractCollection.addAll` adds each element, `AbstractList`'s
            // `addAll(int, c)` likewise: nothing to add is `false`.
            ("addAll", _) => {
                let added = args.last()?;
                match crate::runtime::host_collection_values(self, added) {
                    Ok(values) if values.is_empty() => Ok(Value::Int(0)),
                    Ok(_) => Err(unsupported()),
                    Err(error) => Err(error),
                }
            }
            // `AbstractList.clear` is `removeRange(0, size())`.
            ("clear", "()V") => {
                if size == 0 {
                    Ok(Value::Void)
                } else {
                    Err(unsupported())
                }
            }
            // `AbstractCollection.remove`/`removeAll`: the iterator's
            // `remove` at the first element found.
            ("remove", "(Ljava/lang/Object;)Z") => {
                let needle = args.first()?.clone();
                let mine = self
                    .mutable_collections
                    .get(marker)
                    .cloned()
                    .unwrap_or_default();
                if mine.iter().any(|value| {
                    crate::runtime::same_items(std::slice::from_ref(value), std::slice::from_ref(&needle))
                }) {
                    Err(unsupported())
                } else {
                    Ok(Value::Int(0))
                }
            }
            ("removeAll", "(Ljava/util/Collection;)Z") => match found(self, args.first()?) {
                Ok(true) => Err(unsupported()),
                Ok(false) => Ok(Value::Int(0)),
                Err(error) => Err(error),
            },
            // `AbstractCollection.retainAll`: at the first element `c` lacks.
            ("retainAll", "(Ljava/util/Collection;)Z") => {
                let mine = self
                    .mutable_collections
                    .get(marker)
                    .cloned()
                    .unwrap_or_default();
                match crate::runtime::host_collection_values(self, args.first()?) {
                    Ok(theirs) => {
                        let kept = |value: &Value| {
                            theirs.iter().any(|needle| {
                                crate::runtime::same_items(
                                    std::slice::from_ref(value),
                                    std::slice::from_ref(needle),
                                )
                            })
                        };
                        if mine.iter().all(kept) {
                            Ok(Value::Int(0))
                        } else {
                            Err(unsupported())
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            // `Collection.removeIf`: each element's test in order, the
            // iterator's `remove` at the first that passes.
            ("removeIf", "(Ljava/util/function/Predicate;)Z") => {
                let predicate = args.first()?.clone();
                let test = MethodRef {
                    owner: "java/util/function/Predicate".into(),
                    name: "test".into(),
                    descriptor: "(Ljava/lang/Object;)Z".into(),
                };
                let mine = self
                    .mutable_collections
                    .get(marker)
                    .cloned()
                    .unwrap_or_default();
                for value in mine {
                    match crate::runtime::Host::invoke_instance(self, &test, &predicate, &[value]) {
                        Ok(Value::Int(0)) => {}
                        Ok(_) => return Some(Err(unsupported())),
                        Err(error) => return Some(Err(error)),
                    }
                }
                Ok(Value::Int(0))
            }
            _ => return None,
        };
        Some(answer)
    }
}

impl<H: crate::runtime::Host> crate::runtime::Runtime<'_, H> {
    /// Bring every array a view was written through up to date
    /// ([`StdHost::changed_array_views`]); one comparison while none was.
    pub(crate) fn pull_array_views(&mut self) {
        for (array, values) in self.host.changed_array_views() {
            if let Some(slots) = self.arrays.get_mut(&array)
                && slots.len() == values.len()
            {
                *slots = values;
            }
        }
    }

    /// The runtime wrote the array `key`: its views hold what it holds now.
    pub(crate) fn push_array_views(&mut self, key: &str) {
        if self.host.has_array_views(key)
            && let Some(values) = self.arrays.get(key).cloned()
        {
            self.host.array_written(key, &values);
        }
    }

    /// `Arrays.asList(array)`: a view of an array this runtime owns, an
    /// empty one of the shared empty array, else counted (an array an
    /// unmodelled call made has no elements to show).
    pub(crate) fn as_list(&mut self, r: &MethodRef, array: &Value) -> Result<Value, NativeError> {
        let Some(key) = crate::runtime::host_array::array_key(array) else {
            return self.host.unknown_operand_call(r);
        };
        let values = if crate::runtime::host_array::is_shared_empty(key) {
            Vec::new()
        } else {
            match self.arrays.get(key) {
                Some(values) => values.clone(),
                None => return self.host.unknown_operand_call(r),
            }
        };
        let key = key.to_owned();
        match self.host.array_view(&key, values) {
            Some(view) => Ok(view),
            None => self.host.unknown_operand_call(r),
        }
    }
}

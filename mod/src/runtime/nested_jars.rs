//! Declared Fabric Jar-in-Jar and NeoForge JarJar members.
use crate::runtime::{NativeJar, StdHost};
use std::cmp::Ordering;
const MAX_NESTING: usize = 8;
const MAX_NESTED_JARS: usize = 256;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NestedJar {
    pub entry: String,
    pub result: Result<usize, String>,
}
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NestedCandidate {
    pub entry: String,
    pub parent: String,
    pub jar: NativeJar,
    pub depth: usize,
    pub order: usize,
}
fn fabric_entries(jar: &NativeJar) -> Vec<String> {
    let Some(bytes) = jar.resource_bytes.get("fabric.mod.json") else {
        return Vec::new();
    };
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v.get("jars").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| {
            v.get("file")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .filter(|p| p.ends_with(".jar") && !p.starts_with('/') && !p.contains(".."))
        .collect()
}
fn neoforge_entries(jar: &NativeJar) -> Vec<String> {
    let Some(bytes) = jar.resource_bytes.get("META-INF/jarjar/metadata.json") else {
        return Vec::new();
    };
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v.get("jars").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| {
            v.get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .filter(|p| p.ends_with(".jar") && !p.starts_with('/') && !p.contains(".."))
        .collect()
}
fn declared_entries(jar: &NativeJar) -> Vec<String> {
    let mut entries = fabric_entries(jar)
        .into_iter()
        .chain(neoforge_entries(jar))
        .collect::<Vec<_>>();
    // Compatibility with pre-Jar-in-Jar metadata fixtures: an absent `jars`
    // key historically meant direct META-INF/jars entries were linked. An
    // explicit list (including []) is authoritative and never falls back.
    if entries.is_empty()
        && jar.resource_bytes.get("fabric.mod.json").is_some_and(|b| {
            serde_json::from_slice::<serde_json::Value>(b)
                .ok()
                .is_some_and(|v| v.get("jars").is_none())
        })
    {
        entries.extend(
            jar.resource_bytes
                .keys()
                .filter(|p| p.starts_with("META-INF/jars/") && p.ends_with(".jar"))
                .cloned(),
        );
        entries.sort();
    }
    entries
}
pub fn candidates(jar: &NativeJar) -> Vec<NestedCandidate> {
    let mut out = Vec::new();
    let mut ordinal = 0;
    fn walk(
        parent: &str,
        jar: &NativeJar,
        depth: usize,
        out: &mut Vec<NestedCandidate>,
        ordinal: &mut usize,
    ) {
        if depth >= MAX_NESTING || out.len() >= MAX_NESTED_JARS {
            return;
        };
        for entry in declared_entries(jar) {
            if out.len() >= MAX_NESTED_JARS {
                break;
            };
            let Some(bytes) = jar.resource_bytes.get(&entry) else {
                continue;
            };
            let Ok(nested) = NativeJar::from_bytes(bytes) else {
                continue;
            };
            let child_parent = format!("{parent}|{entry}");
            out.push(NestedCandidate {
                entry: entry.clone(),
                parent: parent.to_owned(),
                jar: nested.clone(),
                depth,
                order: *ordinal,
            });
            *ordinal += 1;
            walk(&child_parent, &nested, depth + 1, out, ordinal);
        }
    }
    walk("outer", jar, 0, &mut out, &mut ordinal);
    out
}
fn version_cmp(a: Option<&str>, b: Option<&str>) -> Ordering {
    let key = |s: Option<&str>| {
        s.unwrap_or("0")
            .split(['.', '-', '+'])
            .map(|p| {
                p.parse::<u64>()
                    .map(|n| format!("0{n:020}"))
                    .unwrap_or_else(|_| format!("1{}", p.to_ascii_lowercase()))
            })
            .collect::<Vec<_>>()
    };
    key(a).cmp(&key(b))
}
pub fn select(jar: &NativeJar) -> Vec<NestedCandidate> {
    let all = candidates(jar);
    let mut selected: Vec<NestedCandidate> = Vec::new();
    for candidate in all {
        let ids = std::iter::once(candidate.jar.fabric_id.as_deref())
            .chain(
                candidate
                    .jar
                    .fabric_provides
                    .iter()
                    .map(|s| Some(s.as_str())),
            )
            .flatten()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut replace = None;
        for id in ids {
            if let Some(index) = selected.iter().position(|c| {
                c.jar.fabric_id.as_deref() == Some(id.as_str())
                    || c.jar.fabric_provides.iter().any(|a| a == &id)
            }) {
                let current = &selected[index];
                let cmp = version_cmp(
                    candidate.jar.fabric_version.as_deref(),
                    current.jar.fabric_version.as_deref(),
                );
                if cmp == Ordering::Greater
                    || (cmp == Ordering::Equal
                        && (candidate.parent.as_str(), candidate.order)
                            < (current.parent.as_str(), current.order))
                {
                    replace = Some(index)
                }
                break;
            }
        }
        if let Some(index) = replace {
            selected[index] = candidate
        } else if candidate.jar.fabric_id.is_some() || !candidate.jar.fabric_provides.is_empty() {
            selected.push(candidate)
        }
    }
    selected.sort_by_key(|c| c.order);
    selected
}
pub fn parsed(jar: &NativeJar) -> Vec<(String, NativeJar)> {
    select(jar).into_iter().map(|c| (c.entry, c.jar)).collect()
}
pub fn link_nested_jars(host: &mut StdHost, jar: &NativeJar) -> Vec<NestedJar> {
    let mut out = Vec::new();
    for candidate in select(jar) {
        let id = format!("{}!{}", candidate.parent, candidate.entry);
        let result = host.link_jar(id, &candidate.jar).map_err(|e| e.to_string());
        if result.is_ok() {
            let parent = jar
                .fabric_id
                .as_deref()
                .or(jar.neoforge_id.as_deref())
                .unwrap_or("jar");
            if let Some(id) = candidate.jar.fabric_id.as_deref() {
                host.fabric_origins.insert(
                    id.to_owned(),
                    ("nested".into(), format!("{parent}|{}", candidate.entry)),
                );
            }
            for alias in &candidate.jar.fabric_provides {
                host.fabric_origins.insert(
                    alias.clone(),
                    ("nested".into(), format!("{parent}|{}", candidate.entry)),
                );
            }
        }
        out.push(NestedJar {
            entry: candidate.entry,
            result,
        });
    }
    // Compatibility for old JarJar fixtures: classes are linked, but no metadata means no mod candidate.
    if neoforge_entries(jar).is_empty() {
        let mut entries = jar
            .resource_bytes
            .keys()
            .filter(|p| p.starts_with("META-INF/jarjar/") && p.ends_with(".jar"))
            .cloned()
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries.into_iter().take(MAX_NESTED_JARS) {
            let Some(bytes) = jar.resource_bytes.get(&entry) else {
                continue;
            };
            let Ok(nested) = NativeJar::from_bytes(bytes) else {
                continue;
            };
            host.record_fabric_metadata(&nested);
            let result = host
                .link_jar(format!("legacy!{entry}"), &nested)
                .map_err(|e| e.to_string());
            out.push(NestedJar { entry, result });
        }
    }
    out
}

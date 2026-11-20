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
    /// NeoForge JarJar's `group:artifact` identity, when declared.
    pub neoforge_coordinate: Option<String>,
    /// JarJar's selected artifact version and requested Maven range.
    pub neoforge_artifact_version: Option<String>,
    pub neoforge_version_range: Option<String>,
    /// A JarJar declaration is a class-path entry even when its jar has no
    /// `neoforge.mods.toml` (the common library case).
    pub neoforge_declared: bool,
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
fn neoforge_declarations(
    jar: &NativeJar,
) -> std::collections::BTreeMap<String, (Option<String>, Option<String>, Option<String>)> {
    let Some(bytes) = jar.resource_bytes.get("META-INF/jarjar/metadata.json") else {
        return Default::default();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Default::default();
    };
    value
        .get("jars")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let path = item.get("path")?.as_str()?.to_owned();
            let identifier = item.get("identifier");
            let coordinate = identifier
                .and_then(|v| v.get("group").and_then(serde_json::Value::as_str))
                .zip(identifier.and_then(|v| v.get("artifact").and_then(serde_json::Value::as_str)))
                .map(|(group, artifact)| format!("{group}:{artifact}"));
            let range = item
                .get("version")
                .and_then(|v| v.get("range").and_then(serde_json::Value::as_str))
                .map(str::to_owned);
            let artifact_version = item
                .get("version")
                .and_then(|v| v.get("artifactVersion").and_then(serde_json::Value::as_str))
                .map(str::to_owned);
            Some((path, (coordinate, artifact_version, range)))
        })
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
            let jarjar = neoforge_declarations(jar);
            let (neoforge_coordinate, neoforge_artifact_version, neoforge_version_range) =
                jarjar.get(&entry).cloned().unwrap_or_default();
            let child_parent = format!("{parent}|{entry}");
            out.push(NestedCandidate {
                entry: entry.clone(),
                parent: parent.to_owned(),
                jar: nested.clone(),
                depth,
                order: *ordinal,
                neoforge_coordinate,
                neoforge_artifact_version,
                neoforge_version_range,
                neoforge_declared: !neoforge_entries(jar).is_empty(),
            });
            *ordinal += 1;
            walk(&child_parent, &nested, depth + 1, out, ordinal);
        }
    }
    walk("outer", jar, 0, &mut out, &mut ordinal);
    out
}
fn legacy_candidates(jar: &NativeJar) -> Vec<NestedCandidate> {
    let mut entries = jar
        .resource_bytes
        .keys()
        .filter(|name| {
            (name.starts_with("META-INF/jars/") || name.starts_with("META-INF/jarjar/"))
                && name.ends_with(".jar")
        })
        .cloned()
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .into_iter()
        .enumerate()
        .filter_map(|(order, entry)| {
            let nested = NativeJar::from_bytes(jar.resource_bytes.get(&entry)?).ok()?;
            Some(NestedCandidate {
                entry,
                parent: "outer".into(),
                jar: nested,
                depth: 0,
                order,
                neoforge_coordinate: None,
                neoforge_artifact_version: None,
                neoforge_version_range: None,
                neoforge_declared: false,
            })
        })
        .take(MAX_NESTED_JARS)
        .collect()
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
fn version_in_range(version: Option<&str>, range: Option<&str>) -> bool {
    let (Some(version), Some(range)) = (version, range) else {
        return true;
    };
    if range.len() < 2 {
        return version == range;
    }
    let open = &range[..1];
    let close = &range[range.len() - 1..];
    let body = &range[1..range.len() - 1];
    let Some((low, high)) = body.split_once(',') else {
        return version == body;
    };
    let low_cmp = version_cmp(Some(version), Some(low));
    let low_ok = low.is_empty()
        || low_cmp == Ordering::Greater
        || (low_cmp == Ordering::Equal && open == "[");
    let high_cmp = if high.is_empty() {
        Ordering::Less
    } else {
        version_cmp(Some(version), Some(high))
    };
    let high_ok = high.is_empty()
        || high_cmp == Ordering::Less
        || (high_cmp == Ordering::Equal && close == "]");
    low_ok && high_ok
}
pub fn select(jar: &NativeJar) -> Vec<NestedCandidate> {
    let all = candidates(jar);
    let mut selected: Vec<NestedCandidate> = Vec::new();
    for candidate in all {
        if candidate.neoforge_declared {
            if !version_in_range(
                candidate.neoforge_artifact_version.as_deref(),
                candidate.neoforge_version_range.as_deref(),
            ) {
                continue;
            }
            if let Some(coordinate) = candidate.neoforge_coordinate.as_deref() {
                if let Some(index) = selected.iter().position(|c| {
                    c.neoforge_declared && c.neoforge_coordinate.as_deref() == Some(coordinate)
                }) {
                    let current = &selected[index];
                    let cmp = version_cmp(
                        candidate.neoforge_artifact_version.as_deref(),
                        current.neoforge_artifact_version.as_deref(),
                    );
                    if cmp == Ordering::Greater
                        || (cmp == Ordering::Equal && candidate.order < current.order)
                    {
                        selected[index] = candidate;
                    }
                } else {
                    selected.push(candidate);
                }
            } else {
                selected.push(candidate);
            }
            continue;
        }
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
        } else {
            // A declared Fabric Jar-in-Jar library need not carry
            // `fabric.mod.json`.  It is still on the outer loader's class
            // path; only its absence from the mod-id graph is significant.
            selected.push(candidate)
        }
    }
    selected.sort_by_key(|c| c.order);
    selected
}
pub fn parsed(jar: &NativeJar) -> Vec<(String, NativeJar)> {
    // Libraries linked from a selected NeoForge member retain the historical
    // direct class-path expansion used by the main loader path.
    legacy_candidates(jar)
        .into_iter()
        .map(|c| (c.entry, c.jar))
        .collect()
}
pub fn link_nested_jars(host: &mut StdHost, jar: &NativeJar) -> Vec<NestedJar> {
    let mut out = Vec::new();
    let selected = if neoforge_entries(jar).is_empty() {
        legacy_candidates(jar)
    } else {
        select(jar)
    };
    for candidate in selected {
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn jar(entries: &[(&str, &[u8])]) -> NativeJar {
        let mut bytes = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut bytes);
            for (name, content) in entries {
                zip.start_file(*name, SimpleFileOptions::default()).unwrap();
                zip.write_all(content).unwrap();
            }
            zip.finish().unwrap();
        }
        NativeJar::from_bytes(&bytes.into_inner()).unwrap()
    }

    #[test]
    fn jarjar_library_without_mod_metadata_is_selected() {
        let outer = jar(&[
            (
                "META-INF/jarjar/metadata.json",
                br#"{"jars":[{"path":"META-INF/jarjar/lib.jar"}]}"#,
            ),
            ("META-INF/jarjar/lib.jar", &empty_zip()),
        ]);
        let selected = select(&outer);
        assert_eq!(selected.len(), 1);
        assert!(selected[0].jar.neoforge_id.is_none());
    }

    #[test]
    fn jarjar_duplicate_coordinate_keeps_highest_artifact_version() {
        let empty_bytes = empty_zip();
        let outer = jar(&[
            (
                "META-INF/jarjar/metadata.json",
                br#"{"jars":[{"path":"META-INF/jarjar/old.jar","identifier":{"group":"g","artifact":"a"},"version":{"range":"[1,3)","artifactVersion":"1.0"}},{"path":"META-INF/jarjar/new.jar","identifier":{"group":"g","artifact":"a"},"version":{"range":"[1,3)","artifactVersion":"2.0"}}]}"#,
            ),
            ("META-INF/jarjar/old.jar", &empty_bytes),
            ("META-INF/jarjar/new.jar", &empty_bytes),
        ]);
        let selected = select(&outer);
        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0].neoforge_artifact_version.as_deref(),
            Some("2.0")
        );
    }

    fn empty_zip() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        zip::ZipWriter::new(&mut bytes).finish().unwrap();
        bytes.into_inner()
    }
}

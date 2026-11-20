//! Declared Fabric Jar-in-Jar and NeoForge JarJar members.
use crate::{NativeJar, StdHost};
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
    /// Parent candidates, nearest first, for Fabric's recursive tie-break.
    parent_chain: Vec<(String, Option<String>, usize)>,
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
        // Fabric checks only ZipEntry.isDirectory and the .jar suffix. The
        // entry is read by exact name from the in-memory ZIP, so `..` is not a
        // filesystem traversal and is harmless here.
        .filter(|p| p.ends_with(".jar"))
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
        // JarJar also reads the named entry from the in-memory ZIP by its
        // exact name; `..` cannot escape to the filesystem.
        .filter(|p| p.ends_with(".jar"))
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
    fabric_entries(jar)
        .into_iter()
        .chain(neoforge_entries(jar))
        .collect()
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
        parent_chain: &[(String, Option<String>, usize)],
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
                parent_chain: parent_chain.to_vec(),
                neoforge_coordinate,
                neoforge_artifact_version,
                neoforge_version_range,
                neoforge_declared: !neoforge_entries(jar).is_empty(),
            });
            *ordinal += 1;
            let mut child_chain = parent_chain.to_vec();
            child_chain.insert(0, (candidate_id(jar), candidate_version(jar), depth));
            walk(
                &child_parent,
                &nested,
                depth + 1,
                out,
                ordinal,
                &child_chain,
            );
        }
    }
    walk("outer", jar, 0, &mut out, &mut ordinal, &[]);
    out
}

fn candidate_id(jar: &NativeJar) -> String {
    jar.fabric_id
        .clone()
        .or_else(|| jar.neoforge_id.clone())
        .unwrap_or_default()
}

fn candidate_version(jar: &NativeJar) -> Option<String> {
    jar.fabric_version
        .clone()
        .or_else(|| jar.neoforge_version.clone())
}

fn parent_cmp(a: &NestedCandidate, b: &NestedCandidate) -> Ordering {
    for ((aid, av, ad), (bid, bv, bd)) in a.parent_chain.iter().zip(&b.parent_chain) {
        let cmp = aid
            .cmp(bid)
            .then_with(|| version_cmp(bv.as_deref(), av.as_deref()))
            .then_with(|| ad.cmp(bd));
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    a.parent_chain.len().cmp(&b.parent_chain.len())
}
#[derive(Clone, Debug)]
enum FabricVersion<'a> {
    Semantic {
        components: Vec<i64>,
        wildcard: bool,
        prerelease: Option<&'a str>,
    },
    String(&'a str),
}

fn fabric_version(value: Option<&str>) -> FabricVersion<'_> {
    let value = value.unwrap_or("0");
    let (without_build, _) = value
        .split_once('+')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    let (numbers, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(a, b)| (a, Some(b)));
    // Version.parse(String) calls VersionParser.parse(..., false): X-ranges
    // are accepted only by the resolver's private parser, and therefore are
    // deliberately StringVersion values on the public API.
    let components = numbers.split('.').collect::<Vec<_>>();
    let valid_prerelease = prerelease.is_none_or(|v| {
        v.is_empty()
            || v.split('.').all(|part| {
                !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            })
    });
    let mut parsed = Vec::with_capacity(components.len());
    for component in components {
        if component.is_empty() || !component.chars().all(|c| c.is_ascii_digit()) {
            return FabricVersion::String(value);
        }
        let Ok(component) = component.parse::<i32>() else {
            return FabricVersion::String(value);
        };
        if component < 0 {
            return FabricVersion::String(value);
        }
        parsed.push(i64::from(component));
    }
    if parsed.is_empty() || !valid_prerelease {
        FabricVersion::String(value)
    } else {
        FabricVersion::Semantic {
            components: parsed,
            wildcard: false,
            prerelease,
        }
    }
}

fn semantic_cmp(a: &FabricVersion<'_>, b: &FabricVersion<'_>) -> Ordering {
    let (
        FabricVersion::Semantic {
            components: ac,
            wildcard: aw,
            prerelease: ap,
        },
        FabricVersion::Semantic {
            components: bc,
            wildcard: bw,
            prerelease: bp,
        },
    ) = (a, b)
    else {
        return Ordering::Equal;
    };
    for i in 0..ac.len().max(bc.len()) {
        let av = ac.get(i).copied().unwrap_or(if *aw { i64::MIN } else { 0 });
        let bv = bc.get(i).copied().unwrap_or(if *bw { i64::MIN } else { 0 });
        if av != i64::MIN && bv != i64::MIN {
            match av.cmp(&bv) {
                Ordering::Equal => {}
                other => return other,
            }
        }
    }
    match (ap, bp) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => {
            if *bw {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }
        (None, Some(_)) => {
            if *aw {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        }
        (Some(a), Some(b)) => {
            let mut ai = a.split('.');
            let mut bi = b.split('.');
            loop {
                match (ai.next(), bi.next()) {
                    (None, None) => return Ordering::Equal,
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(x), Some(y)) => {
                        let xn = x.chars().all(|c| c.is_ascii_digit());
                        let yn = y.chars().all(|c| c.is_ascii_digit());
                        let cmp = match (xn, yn) {
                            (true, true) => x.len().cmp(&y.len()).then_with(|| x.cmp(y)),
                            (true, false) => Ordering::Less,
                            (false, true) => Ordering::Greater,
                            (false, false) => x.cmp(y),
                        };
                        if cmp != Ordering::Equal {
                            return cmp;
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn version_cmp(left: Option<&str>, right: Option<&str>) -> Ordering {
    let av = fabric_version(left);
    let bv = fabric_version(right);
    match (&av, &bv) {
        (FabricVersion::Semantic { .. }, FabricVersion::Semantic { .. }) => semantic_cmp(&av, &bv),
        (FabricVersion::String(a), FabricVersion::String(b)) => a.cmp(b),
        (FabricVersion::Semantic { .. }, FabricVersion::String(right)) => {
            semantic_friendly(&av).as_str().cmp(right)
        }
        (FabricVersion::String(left), FabricVersion::Semantic { .. }) => {
            left.cmp(&semantic_friendly(&bv).as_str())
        }
    }
}

fn semantic_friendly(version: &FabricVersion<'_>) -> String {
    let FabricVersion::Semantic {
        components,
        prerelease,
        ..
    } = version
    else {
        return String::new();
    };
    let mut result = components
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(".");
    if let Some(prerelease) = prerelease {
        result.push('-');
        result.push_str(prerelease);
    }
    result
}

#[derive(Clone, Debug)]
struct MavenInterval {
    low: Option<String>,
    high: Option<String>,
    low_inclusive: bool,
    high_inclusive: bool,
}

fn maven_cmp(a: &str, b: &str) -> Ordering {
    // Maven ComparableVersion's Item tree, reduced to its observable ordering.
    // Delimiters and digit/letter transitions split items; a missing item is
    // the release (empty qualifier), while numeric items sort after strings.
    fn tokens(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut previous_digit = None;
        for c in s.chars() {
            // ComparableVersion's parser has only these two delimiters. A
            // plus (or any other punctuation) stays in its string item.
            let delimiter = matches!(c, '.' | '-');
            let transition = previous_digit.is_some_and(|digit| digit != c.is_ascii_digit());
            if (delimiter || transition) && !current.is_empty() {
                if transition && c.is_ascii_digit() && matches!(current.as_str(), "a" | "b" | "m") {
                    current = match current.as_str() {
                        "a" => "alpha",
                        "b" => "beta",
                        _ => "milestone",
                    }
                    .to_owned();
                }
                out.push(current.to_ascii_lowercase());
                current.clear();
            }
            if !delimiter {
                current.push(c);
            }
            previous_digit = Some(c.is_ascii_digit());
        }
        if !current.is_empty() {
            out.push(current.to_ascii_lowercase());
        }
        while out.last().is_some_and(|part| {
            !part.is_empty()
                && part.chars().all(|c| c.is_ascii_digit())
                && part.trim_start_matches('0').is_empty()
        }) {
            out.pop();
        }
        out
    }
    fn qualifier(s: &str) -> (&str, i32) {
        if let Some(s) = s.strip_prefix('@') {
            return (s, 9);
        }
        let (canonical, rank) = match s {
            "alpha" => ("alpha", 1),
            "beta" => ("beta", 2),
            "milestone" => ("milestone", 3),
            "rc" | "cr" => ("rc", 4),
            "snapshot" => ("snapshot", 5),
            "" | "ga" | "final" | "release" => ("", 6),
            "sp" => ("sp", 7),
            other => (other, 8),
        };
        (canonical, rank)
    }
    fn item_cmp(x: Option<&str>, y: Option<&str>) -> Ordering {
        let x = x.unwrap_or("");
        let y = y.unwrap_or("");
        let xn = !x.is_empty() && x.chars().all(|c| c.is_ascii_digit());
        let yn = !y.is_empty() && y.chars().all(|c| c.is_ascii_digit());
        match (xn, yn) {
            (true, true) => {
                let x = x.trim_start_matches('0');
                let y = y.trim_start_matches('0');
                let x = if x.is_empty() { "0" } else { x };
                let y = if y.is_empty() { "0" } else { y };
                x.len().cmp(&y.len()).then_with(|| x.cmp(y))
            }
            (true, false) => {
                if y.is_empty() && x.trim_start_matches('0').is_empty() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
            (false, true) => {
                if x.is_empty() && y.trim_start_matches('0').is_empty() {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            }
            (false, false) => {
                let (x, xr) = qualifier(x);
                let (y, yr) = qualifier(y);
                xr.cmp(&yr).then_with(|| x.cmp(y))
            }
        }
    }
    let at = tokens(a);
    let bt = tokens(b);
    for i in 0..at.len().max(bt.len()) {
        let cmp = item_cmp(at.get(i).map(String::as_str), bt.get(i).map(String::as_str));
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    Ordering::Equal
}

fn maven_range(value: &str) -> Option<Vec<MavenInterval>> {
    let value = value.trim();
    // Maven's createFromVersionSpec accepts a bare version as a recommended
    // version (an exact [v] restriction is not equivalent when selecting).
    if !value.starts_with('[') && !value.starts_with('(') {
        if value.is_empty() || value.contains(']') || value.contains(')') || value.contains(',') {
            return None;
        }
        return Some(vec![MavenInterval {
            low: None,
            high: None,
            low_inclusive: false,
            high_inclusive: false,
        }]);
    }
    let mut ranges = Vec::new();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, c) in value.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    for part in parts {
        let part = part.trim();
        let bytes = part.as_bytes();
        if bytes.len() < 2 {
            return None;
        }
        let low_inclusive = bytes[0] == b'[';
        let high_inclusive = bytes[bytes.len() - 1] == b']';
        let body = &part[1..part.len() - 1];
        if let Some((low, high)) = body.split_once(',') {
            ranges.push(MavenInterval {
                low: (!low.trim().is_empty()).then(|| low.trim().to_owned()),
                high: (!high.trim().is_empty()).then(|| high.trim().to_owned()),
                low_inclusive,
                high_inclusive,
            });
        } else if low_inclusive && high_inclusive {
            ranges.push(MavenInterval {
                low: Some(body.trim().to_owned()),
                high: Some(body.trim().to_owned()),
                low_inclusive: true,
                high_inclusive: true,
            });
        } else {
            return None;
        }
    }
    Some(ranges)
}

fn intersect(a: &[MavenInterval], b: &[MavenInterval]) -> Vec<MavenInterval> {
    let mut out = Vec::new();
    for left in a {
        for right in b {
            let (low, low_inclusive) = match (&left.low, &right.low) {
                (None, x) => (x.clone(), right.low_inclusive),
                (x, None) => (x.clone(), left.low_inclusive),
                (Some(x), Some(y)) => match maven_cmp(x, y) {
                    Ordering::Greater => (Some(x.clone()), left.low_inclusive),
                    Ordering::Less => (Some(y.clone()), right.low_inclusive),
                    Ordering::Equal => (Some(x.clone()), left.low_inclusive && right.low_inclusive),
                },
            };
            let (high, high_inclusive) = match (&left.high, &right.high) {
                (None, x) => (x.clone(), right.high_inclusive),
                (x, None) => (x.clone(), left.high_inclusive),
                (Some(x), Some(y)) => match maven_cmp(x, y) {
                    Ordering::Less => (Some(x.clone()), left.high_inclusive),
                    Ordering::Greater => (Some(y.clone()), right.high_inclusive),
                    Ordering::Equal => {
                        (Some(x.clone()), left.high_inclusive && right.high_inclusive)
                    }
                },
            };
            if let (Some(low), Some(high)) = (&low, &high) {
                let c = maven_cmp(low, high);
                if c == Ordering::Greater
                    || (c == Ordering::Equal && !(low_inclusive && high_inclusive))
                {
                    continue;
                }
            }
            out.push(MavenInterval {
                low,
                high,
                low_inclusive,
                high_inclusive,
            });
        }
    }
    out
}

fn maven_in_range(version: &str, ranges: &[MavenInterval]) -> bool {
    ranges.iter().any(|r| {
        let low = r.low.as_ref().is_none_or(|low| {
            maven_cmp(version, low) == Ordering::Greater
                || (r.low_inclusive && maven_cmp(version, low) == Ordering::Equal)
        });
        let high = r.high.as_ref().is_none_or(|high| {
            maven_cmp(version, high) == Ordering::Less
                || (r.high_inclusive && maven_cmp(version, high) == Ordering::Equal)
        });
        low && high
    })
}
pub fn select(jar: &NativeJar) -> Vec<NestedCandidate> {
    let all = candidates(jar);
    let mut selected: Vec<NestedCandidate> = Vec::new();
    let mut neoforge_groups: std::collections::BTreeMap<String, Vec<NestedCandidate>> =
        Default::default();
    for candidate in all {
        if let Some(coordinate) = candidate.neoforge_coordinate.clone() {
            neoforge_groups
                .entry(coordinate)
                .or_default()
                .push(candidate);
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
        // ModPrioSorter compares every id/alias shared by two candidates. A
        // candidate that wins one key but loses another cannot remain beside
        // the incompatible winner; it is discarded until the resolver has a
        // single candidate for every overlapping key.
        let overlaps = selected
            .iter()
            .enumerate()
            .filter_map(|(index, current)| {
                let current_ids = std::iter::once(current.jar.fabric_id.as_deref())
                    .chain(
                        current
                            .jar
                            .fabric_provides
                            .iter()
                            .map(|alias| Some(alias.as_str())),
                    )
                    .flatten();
                ids.iter()
                    .any(|id| current_ids.clone().any(|other| other == id))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let had_overlaps = !overlaps.is_empty();
        let mut replace = Vec::new();
        let mut loses = false;
        for index in overlaps {
            let current = &selected[index];
            let cmp = version_cmp(
                candidate.jar.fabric_version.as_deref(),
                current.jar.fabric_version.as_deref(),
            );
            let cmp = if cmp == Ordering::Equal {
                parent_cmp(&candidate, current).then_with(|| candidate.order.cmp(&current.order))
            } else {
                cmp
            };
            if cmp == Ordering::Greater {
                replace.push(index);
            } else if cmp == Ordering::Less {
                loses = true;
                break;
            }
        }
        if !loses && !replace.is_empty() {
            for index in replace.into_iter().rev() {
                selected.remove(index);
            }
            selected.push(candidate);
        } else if !had_overlaps {
            // A declared Fabric Jar-in-Jar library need not carry
            // `fabric.mod.json`.  It is still on the outer loader's class
            // path; only its absence from the mod-id graph is significant.
            selected.push(candidate)
        }
    }
    for (_coordinate, group) in neoforge_groups {
        if group.len() == 1 {
            selected.extend(group);
            continue;
        }
        let mut allowed: Option<Vec<MavenInterval>> = None;
        let mut recommended: Option<String> = None;
        for candidate in &group {
            let Some(range) = candidate
                .neoforge_version_range
                .as_deref()
                .and_then(maven_range)
            else {
                allowed = Some(Vec::new());
                break;
            };
            if let Some(version) = candidate.neoforge_version_range.as_deref()
                && !version.trim_start().starts_with(['[', '('])
            {
                recommended.get_or_insert_with(|| version.trim().to_owned());
            }
            allowed = Some(match allowed {
                Some(ref current) => intersect(current, &range),
                None => range,
            });
            if allowed.as_ref().is_some_and(Vec::is_empty) {
                break;
            }
        }
        let Some(allowed) = allowed else {
            continue;
        };
        if allowed.is_empty() {
            continue;
        }
        let matches = |candidate: &NestedCandidate| {
            candidate
                .neoforge_artifact_version
                .as_deref()
                .is_some_and(|v| maven_in_range(v, &allowed))
        };
        if let Some(recommended) = recommended {
            if let Some(candidate) = group.iter().find(|candidate| {
                candidate.neoforge_artifact_version.as_deref() == Some(recommended.as_str())
            }) {
                selected.push(candidate.clone());
            }
        } else if let Some(candidate) = group.into_iter().find(matches) {
            // JarSelector uses Collection.stream().findFirst(), preserving
            // metadata iteration order; it never chooses the maximum.
            selected.push(candidate);
        }
    }
    selected.sort_by_key(|c| c.order);
    selected
}
pub fn parsed(jar: &NativeJar) -> Vec<(String, NativeJar)> {
    fn walk(jar: &NativeJar, out: &mut Vec<(String, NativeJar)>, prefix: &str) {
        for candidate in select(jar) {
            let path = if prefix.is_empty() {
                candidate.entry.clone()
            } else {
                format!("{prefix}|{}", candidate.entry)
            };
            out.push((path.clone(), candidate.jar.clone()));
            walk(&candidate.jar, out, &path);
        }
    }
    let mut out = Vec::new();
    walk(jar, &mut out, "");
    out
}
pub fn link_nested_jars(host: &mut StdHost, jar: &NativeJar) -> Vec<NestedJar> {
    let mut out = Vec::new();
    let selected = select(jar);
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
            Some("1.0")
        );
    }

    #[test]
    fn fabric_only_links_declared_entries_and_explicit_empty_lists_link_none() {
        let nested = empty_zip();
        let unlisted = jar(&[("META-INF/jars/unlisted.jar", &nested)]);
        assert!(candidates(&unlisted).is_empty());
        let explicit_empty = jar(&[
            (
                "fabric.mod.json",
                br#"{"schemaVersion":1,"id":"x","version":"1","jars":[]}"#,
            ),
            ("META-INF/jars/unlisted.jar", &nested),
        ]);
        assert!(candidates(&explicit_empty).is_empty());
        let listed = jar(&[
            ("fabric.mod.json", br#"{"schemaVersion":1,"id":"x","version":"1","jars":[{"file":"META-INF/jars/child.jar"}]}"#),
            ("META-INF/jars/child.jar", &nested),
        ]);
        assert_eq!(candidates(&listed).len(), 1);
    }

    #[test]
    fn overlapping_fabric_aliases_leave_only_the_compatible_winner() {
        let one = jar(&[
            (
                "fabric.mod.json",
                br#"{"schemaVersion":1,"id":"one","version":"1","provides":["shared"]}"#,
            ),
            ("one.txt", b"one"),
        ]);
        let two = jar(&[
            (
                "fabric.mod.json",
                br#"{"schemaVersion":1,"id":"two","version":"2","provides":["shared"]}"#,
            ),
            ("two.txt", b"two"),
        ]);
        let outer = jar(&[
            (
                "fabric.mod.json",
                br#"{"schemaVersion":1,"id":"outer","version":"1","jars":[{"file":"one.jar"},{"file":"two.jar"}]}"#,
            ),
            ("one.jar", &zip_bytes(&one)),
            ("two.jar", &zip_bytes(&two)),
        ]);
        let selected = select(&outer);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].jar.fabric_id.as_deref(), Some("two"));
    }

    fn zip_bytes(jar: &NativeJar) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut bytes);
        for (name, payload) in &jar.resource_bytes {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(payload).unwrap();
        }
        zip.finish().unwrap();
        bytes.into_inner()
    }

    #[test]
    fn semantic_version_comparison_matches_fabric_loader() {
        let cases = [
            ("1.0-alpha", "1.0", Ordering::Less),
            ("1.0+build1", "1.0+build2", Ordering::Equal),
            ("1.0.0", "1.0", Ordering::Equal),
            ("1.0-alpha.1", "1.0-alpha.beta", Ordering::Less),
            ("1.0-alpha.2", "1.0-alpha.10", Ordering::Less),
            ("1.0-rc", "1.0-beta", Ordering::Greater),
            ("01.0", "1.0", Ordering::Equal),
            ("1.0.foo", "1.0", Ordering::Greater),
            ("1.0-X", "1.0", Ordering::Less),
            ("1.x", "1.1.1", Ordering::Greater),
            ("1.*", "1.1.1", Ordering::Less),
            ("1.0-", "1.0", Ordering::Less),
        ];
        for (left, right, expected) in cases {
            assert_eq!(
                version_cmp(Some(left), Some(right)),
                expected,
                "{left} vs {right}"
            );
        }
    }

    #[test]
    fn maven_comparable_version_examples_match_jarjar() {
        assert_eq!(maven_cmp("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(maven_cmp("1.0-alpha", "1.0"), Ordering::Less);
        assert_eq!(maven_cmp("1.0-rc1", "1.0-rc2"), Ordering::Less);
        assert_eq!(maven_cmp("1.0-SNAPSHOT", "1.0"), Ordering::Less);
        assert_eq!(maven_cmp("1.0-sp", "1.0"), Ordering::Greater);
        assert_eq!(maven_cmp("1.0", "1.0+build"), Ordering::Less);
        assert_eq!(maven_cmp("1.0-ga", "1.0"), Ordering::Equal);
        assert_eq!(maven_cmp("1.0-final", "1.0"), Ordering::Equal);
        assert_eq!(maven_cmp("1.0-cr", "1.0-rc"), Ordering::Equal);
        assert_eq!(maven_cmp("1.0a", "1.0"), Ordering::Greater);
        assert_eq!(
            maven_cmp(
                "1.0.27-26.3-neoforge-SNAPSHOT",
                "1.0.26-26.3-neoforge-SNAPSHOT"
            ),
            Ordering::Greater
        );
    }

    #[test]
    fn maven_comparison_matches_the_jdk25_oracle() {
        for line in include_str!("jarjar_version_java25.txt").lines() {
            let mut fields = line.split('|');
            if fields.next() != Some("cmp") {
                continue;
            }
            let left = fields.next().unwrap();
            let right = fields.next().unwrap();
            let expected = fields.next().unwrap().parse::<i32>().unwrap();
            assert_eq!(
                maven_cmp(left, right).cmp(&Ordering::Equal) as i32,
                expected.signum(),
                "{line}"
            );
        }
    }

    #[test]
    fn jarjar_range_intersection_can_have_no_solution() {
        let empty = empty_zip();
        let outer = jar(&[
            ("META-INF/jarjar/metadata.json", br#"{"jars":[
                {"path":"META-INF/jarjar/one.jar","identifier":{"group":"g","artifact":"a"},"version":{"range":"[1,2)","artifactVersion":"1.5"}},
                {"path":"META-INF/jarjar/two.jar","identifier":{"group":"g","artifact":"a"},"version":{"range":"[3,4)","artifactVersion":"3.5"}}
            ]}"#),
            ("META-INF/jarjar/one.jar", &empty),
            ("META-INF/jarjar/two.jar", &empty),
        ]);
        assert!(select(&outer).is_empty());
    }

    fn empty_zip() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        zip::ZipWriter::new(&mut bytes).finish().unwrap();
        bytes.into_inner()
    }
}

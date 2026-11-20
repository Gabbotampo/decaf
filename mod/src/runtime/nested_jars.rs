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

#[derive(Clone, Debug)]
struct MavenRange {
    recommended: Option<String>,
    restrictions: Vec<MavenInterval>,
}

fn maven_cmp(a: &str, b: &str) -> Ordering {
    #[derive(Clone, Debug)]
    enum Item {
        Big(String),
        Str(String),
        List(Vec<Item>),
        Int(String),
        Long(String),
    }
    impl Item {
        fn kind(&self) -> u8 {
            match self {
                Self::Big(_) => 0,
                Self::Str(_) => 1,
                Self::List(_) => 2,
                Self::Int(_) => 3,
                Self::Long(_) => 4,
            }
        }
        fn is_null(&self) -> bool {
            match self {
                Self::Big(v) | Self::Int(v) | Self::Long(v) => v == "0",
                Self::Str(v) => comparable_qualifier(v) == "5",
                Self::List(v) => v.is_empty(),
            }
        }
        fn compare(&self, other: Option<&Self>) -> Ordering {
            let Some(other) = other else {
                return match self {
                    Self::Big(v) | Self::Int(v) | Self::Long(v) => {
                        if v == "0" {
                            Ordering::Equal
                        } else {
                            Ordering::Greater
                        }
                    }
                    Self::Str(v) => comparable_qualifier(v).as_str().cmp("5"),
                    Self::List(v) => {
                        for item in v {
                            let cmp = item.compare(None);
                            if cmp != Ordering::Equal {
                                return cmp;
                            }
                        }
                        Ordering::Equal
                    }
                };
            };
            match (self.kind(), other.kind()) {
                (0, 0) => numeric_cmp(item_value(self), item_value(other)),
                (0, 1..=4) => Ordering::Greater,
                (1, 0 | 2 | 3 | 4) => Ordering::Less,
                (2, 0) => Ordering::Less,
                (2, 1) => Ordering::Greater,
                (2, 3 | 4) => Ordering::Less,
                (1, 1) => comparable_qualifier(item_value(self))
                    .cmp(&comparable_qualifier(item_value(other))),
                (2, 2) => {
                    let (Self::List(left), Self::List(right)) = (self, other) else {
                        unreachable!()
                    };
                    for i in 0..left.len().max(right.len()) {
                        let cmp = match (left.get(i), right.get(i)) {
                            (Some(a), Some(b)) => a.compare(Some(b)),
                            (Some(a), None) => a.compare(None),
                            (None, Some(b)) => b.compare(None).reverse(),
                            (None, None) => Ordering::Equal,
                        };
                        if cmp != Ordering::Equal {
                            return cmp;
                        }
                    }
                    Ordering::Equal
                }
                (3, 3) => numeric_cmp(item_value(self), item_value(other)),
                (4, 4) => numeric_cmp(item_value(self), item_value(other)),
                (3, 0) => Ordering::Less,
                (3, 1 | 2) => Ordering::Greater,
                (3, 4) => Ordering::Less,
                (4, 0) => Ordering::Less,
                (4, 1 | 2) => Ordering::Greater,
                (4, 3) => Ordering::Greater,
                _ => unreachable!(),
            }
        }
    }
    fn item_value(item: &Item) -> &str {
        match item {
            Item::Big(v) | Item::Str(v) | Item::Int(v) | Item::Long(v) => v,
            Item::List(_) => unreachable!(),
        }
    }
    fn numeric_cmp(a: &str, b: &str) -> Ordering {
        a.len().cmp(&b.len()).then_with(|| a.cmp(b))
    }
    fn comparable_qualifier(value: &str) -> String {
        match value {
            "alpha" => "0".into(),
            "beta" => "1".into(),
            "milestone" => "2".into(),
            "rc" => "3".into(),
            "snapshot" => "4".into(),
            "" => "5".into(),
            "sp" => "6".into(),
            other => format!("7-{other}"),
        }
    }
    fn parse_item(is_digit: bool, value: &str, transition: bool) -> Item {
        if is_digit {
            let value = value.trim_start_matches('0');
            let value = if value.is_empty() { "0" } else { value };
            match value.len() {
                0..=9 => Item::Int(value.into()),
                10..=18 => Item::Long(value.into()),
                _ => Item::Big(value.into()),
            }
        } else {
            let value = if transition && value == "a" {
                "alpha"
            } else if transition && value == "b" {
                "beta"
            } else if transition && value == "m" {
                "milestone"
            } else {
                value
            };
            let value = match value {
                "ga" | "final" | "release" => "",
                "cr" => "rc",
                value => value,
            };
            Item::Str(value.to_owned())
        }
    }
    fn normalize(items: &mut Vec<Item>) {
        for item in items.iter_mut() {
            if let Item::List(children) = item {
                normalize(children);
            }
        }
        let mut index = items.len();
        while index > 0 {
            index -= 1;
            if items[index].is_null() {
                items.remove(index);
            } else if !matches!(items[index], Item::List(_)) {
                break;
            }
        }
    }
    fn parse(value: &str) -> Item {
        let value = value.to_ascii_lowercase();
        let bytes = value.as_bytes();
        let mut nodes = vec![Vec::new()];
        let mut current = 0usize;
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut start = 0usize;
        let mut is_digit = false;
        // The parser is deliberately written in the same state-machine shape as Maven's
        // bytecode. Pointers only refer to vectors owned by `root` and its nested lists.
        for i in 0..bytes.len() {
            let c = bytes[i] as char;
            if c == '.' || c == '-' {
                if i == start {
                    nodes[current].push(Item::Int("0".into()));
                } else {
                    nodes[current].push(parse_item(is_digit, &value[start..i], false));
                }
                start = i + 1;
                if c == '-'
                    || (!is_digit
                        && c == '.'
                        && bytes.get(i + 1).is_some_and(|next| !next.is_ascii_digit()))
                {
                    let slot = nodes[current].len();
                    nodes[current].push(Item::List(Vec::new()));
                    let child = nodes.len();
                    nodes.push(Vec::new());
                    stack.push((current, slot));
                    current = child;
                    is_digit = false;
                }
                continue;
            }
            if c.is_ascii_digit() {
                if !is_digit && i > start {
                    if !nodes[current].is_empty() {
                        let slot = nodes[current].len();
                        nodes[current].push(Item::List(Vec::new()));
                        let child = nodes.len();
                        nodes.push(Vec::new());
                        stack.push((current, slot));
                        current = child;
                    }
                    nodes[current].push(parse_item(false, &value[start..i], true));
                    let slot = nodes[current].len();
                    nodes[current].push(Item::List(Vec::new()));
                    let child = nodes.len();
                    nodes.push(Vec::new());
                    stack.push((current, slot));
                    current = child;
                    start = i;
                }
                is_digit = true;
            } else if is_digit {
                if i > start {
                    nodes[current].push(parse_item(true, &value[start..i], false));
                }
                let slot = nodes[current].len();
                nodes[current].push(Item::List(Vec::new()));
                let child = nodes.len();
                nodes.push(Vec::new());
                stack.push((current, slot));
                current = child;
                start = i;
                is_digit = false;
            }
        }
        if bytes.len() > start {
            nodes[current].push(parse_item(is_digit, &value[start..], false));
        }
        while let Some((parent, slot)) = stack.pop() {
            let child = std::mem::take(&mut nodes[current]);
            nodes[parent][slot] = Item::List(child);
            current = parent;
        }
        normalize(&mut nodes[0]);
        // A dot-separated qualifier following the first qualifier is flattened by
        // Maven's parser (while a `+` string remains in its hyphen list).
        let dot_qualifier = value
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0] == b'.' && !pair[1].is_ascii_digit());
        if nodes[0].len() > 1 && dot_qualifier && !value.contains('-') {
            let mut promoted = Vec::new();
            if let Item::List(items) = &mut nodes[0][1]
                && items.len() > 1
                && matches!(&items[0], Item::Str(value) if !value.starts_with('+'))
            {
                if matches!(&items[1], Item::List(_)) {
                    promoted.push(items.remove(0));
                } else {
                    promoted.append(items);
                }
            }
            for item in promoted.into_iter().rev() {
                nodes[0].insert(1, item);
            }
        }
        Item::List(std::mem::take(&mut nodes[0]))
    }
    let left = parse(a);
    let right = parse(b);
    left.compare(Some(&right))
}

fn maven_range(value: &str) -> Result<MavenRange, String> {
    let original = value.to_owned();
    let mut rest = value;
    let mut restrictions = Vec::new();
    let mut previous_upper: Option<String> = None;
    while rest.starts_with('[') || rest.starts_with('(') {
        let close_paren = rest.find(')');
        let close_bracket = rest.find(']');
        let close = match (close_paren, close_bracket) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return Err(format!("Unbounded range: {original}")),
        };
        let part = &rest[..=close];
        let interval = parse_maven_interval(part)?;
        if let (Some(previous), Some(lower)) = (&previous_upper, &interval.low)
            && maven_cmp(lower, previous) == Ordering::Less
        {
            return Err(format!("Ranges overlap: {original}"));
        }
        previous_upper = interval.high.clone();
        restrictions.push(interval);
        rest = rest[close + 1..].trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        }
    }
    let recommended = if rest.is_empty() {
        None
    } else {
        if !restrictions.is_empty() {
            return Err(format!(
                "Only fully-qualified sets allowed in multiple set scenario: {original}"
            ));
        }
        Some(rest.to_owned())
    };
    if recommended.is_some() {
        restrictions.push(MavenInterval {
            low: None,
            high: None,
            low_inclusive: false,
            high_inclusive: false,
        });
    }
    Ok(MavenRange {
        recommended,
        restrictions,
    })
}

fn parse_maven_interval(part: &str) -> Result<MavenInterval, String> {
    let bytes = part.as_bytes();
    let low_inclusive = bytes.first() == Some(&b'[');
    let high_inclusive = bytes.last() == Some(&b']');
    let body = part[1..part.len() - 1].trim();
    let Some(comma) = body.find(',') else {
        if low_inclusive && high_inclusive {
            let version = body.to_owned();
            return Ok(MavenInterval {
                low: Some(version.clone()),
                high: Some(version),
                low_inclusive: true,
                high_inclusive: true,
            });
        }
        return Err(format!("Single version must be surrounded by []: {part}"));
    };
    let low = body[..comma].trim();
    let high = body[comma + 1..].trim();
    let low = (!low.is_empty()).then(|| low.to_owned());
    let high = (!high.is_empty()).then(|| high.to_owned());
    if let (Some(low), Some(high)) = (&low, &high) {
        let cmp = maven_cmp(high, low);
        if cmp == Ordering::Less || (cmp == Ordering::Equal && low_inclusive && !high_inclusive) {
            return Err(format!("Range defies version ordering: {part}"));
        }
    }
    Ok(MavenInterval {
        low,
        high,
        low_inclusive,
        high_inclusive,
    })
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

fn maven_restrict(left: &MavenRange, right: &MavenRange) -> MavenRange {
    let restrictions = if left.restrictions.is_empty() || right.restrictions.is_empty() {
        Vec::new()
    } else {
        intersect(&left.restrictions, &right.restrictions)
    };
    let recommended = if !restrictions.is_empty() {
        left.recommended
            .as_ref()
            .filter(|v| maven_in_range(v, &restrictions))
            .cloned()
            .or_else(|| {
                right
                    .recommended
                    .as_ref()
                    .filter(|v| maven_in_range(v, &restrictions))
                    .cloned()
            })
    } else {
        left.recommended
            .clone()
            .or_else(|| right.recommended.clone())
    };
    MavenRange {
        recommended,
        restrictions,
    }
}

pub fn select(jar: &NativeJar) -> Vec<NestedCandidate> {
    select_checked(jar).unwrap_or_default()
}

fn select_fabric_candidates(mut fabric: Vec<NestedCandidate>) -> Vec<NestedCandidate> {
    let mut selected: Vec<NestedCandidate> = Vec::new();
    // ModPrioSorter first orders candidates by id, descending version, nest level,
    // and parent precedence. It then resolves every duplicate id/alias graph. Its
    // overlap comparator sums all shared id and alias comparisons; a zero score is
    // deliberately a tie, so cycles of equal candidates remain intact.
    fabric.sort_by(|a, b| {
        (a.depth != 0)
            .cmp(&(b.depth != 0))
            .then_with(|| candidate_id(&a.jar).cmp(&candidate_id(&b.jar)))
            .then_with(|| {
                version_cmp(
                    candidate_version(&b.jar).as_deref(),
                    candidate_version(&a.jar).as_deref(),
                )
            })
            .then_with(|| a.depth.cmp(&b.depth))
            .then_with(|| parent_cmp(a, b))
            .then_with(|| a.order.cmp(&b.order))
    });
    let mut key_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut provided_keys = std::collections::BTreeSet::new();
    for candidate in &fabric {
        *key_counts.entry(candidate_id(&candidate.jar)).or_default() += 1;
        for alias in &candidate.jar.fabric_provides {
            *key_counts.entry(alias.clone()).or_default() += 1;
            provided_keys.insert(alias.clone());
        }
    }
    let duplicate_keys = key_counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1 && provided_keys.contains(&key)).then_some(key))
        .collect::<std::collections::BTreeSet<_>>();
    let sign = |ordering: Ordering| match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    };
    let mut first_nested_index = 0;
    let mut seen_nested = false;
    for candidate in fabric {
        if !seen_nested && candidate.depth != 0 {
            seen_nested = true;
            first_nested_index = selected.len();
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
        if !ids.iter().any(|id| duplicate_keys.contains(id)) {
            selected.push(candidate);
            continue;
        }
        let overlap_keys = ids
            .iter()
            .filter(|id| duplicate_keys.contains(*id))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let mut replace = None;
        let mut reject = false;
        for previous_index in (first_nested_index..selected.len()).rev() {
            let current = &selected[previous_index];
            if candidate_id(&candidate.jar) == candidate_id(&current.jar) {
                break;
            }
            if !overlap_keys.contains(&candidate_id(&current.jar))
                && current
                    .jar
                    .fabric_provides
                    .iter()
                    .all(|alias| !overlap_keys.contains(alias))
            {
                continue;
            }
            let score = compare_overlapping_ids(&candidate, current, &sign);
            if score < 0 {
                replace = Some(previous_index);
            } else if score > 0 {
                reject = true;
                break;
            }
        }
        if let Some(previous_index) = replace {
            selected.remove(previous_index);
            selected.insert(previous_index, candidate);
        } else if reject {
            continue;
        } else if candidate.depth != 0
            && selected[..first_nested_index].iter().any(|current| {
                overlap_keys.contains(&candidate_id(&current.jar))
                    || current
                        .jar
                        .fabric_provides
                        .iter()
                        .any(|alias| overlap_keys.contains(alias))
            })
        {
            // ModPrioSorter never compares a candidate after the first nested
            // one with a root. The resolver consequently keeps the root and
            // drops this lower-priority nested provider.
        } else {
            selected.push(candidate);
        }
    }
    selected
}

/// Select the nested jars, preserving JarSelector's refusal when a declared
/// coordinate has no artifact in the restricted Maven range.
pub fn select_checked(jar: &NativeJar) -> Result<Vec<NestedCandidate>, String> {
    let all = candidates(jar);
    let mut neoforge_groups: std::collections::BTreeMap<String, Vec<NestedCandidate>> =
        Default::default();
    let mut fabric = Vec::new();
    for candidate in all {
        if let Some(coordinate) = candidate.neoforge_coordinate.clone() {
            neoforge_groups
                .entry(coordinate)
                .or_default()
                .push(candidate);
        } else {
            fabric.push(candidate);
        }
    }
    let mut selected = select_fabric_candidates(fabric);
    for (_coordinate, group) in neoforge_groups {
        if group.len() == 1 {
            selected.extend(group);
            continue;
        }
        let mut allowed: Option<MavenRange> = None;
        for candidate in &group {
            let Ok(range) = candidate
                .neoforge_version_range
                .as_deref()
                .map_or_else(|| Err("missing version range".to_owned()), maven_range)
            else {
                allowed = Some(MavenRange {
                    recommended: None,
                    restrictions: Vec::new(),
                });
                break;
            };
            allowed = Some(match allowed {
                Some(ref current) => maven_restrict(current, &range),
                None => range,
            });
            if allowed
                .as_ref()
                .is_some_and(|range| range.restrictions.is_empty() && range.recommended.is_none())
            {
                break;
            }
        }
        let Some(allowed) = allowed else {
            return Err(neoforge_no_match_error(&group));
        };
        if allowed.restrictions.is_empty() && allowed.recommended.is_none() {
            return Err(neoforge_no_match_error(&group));
        }
        let matches = |candidate: &NestedCandidate| {
            candidate
                .neoforge_artifact_version
                .as_deref()
                .is_some_and(|v| maven_in_range(v, &allowed.restrictions))
        };
        if let Some(recommended) = allowed.recommended {
            if let Some(candidate) = group.iter().find(|candidate| {
                candidate.neoforge_artifact_version.as_deref() == Some(recommended.as_str())
            }) {
                selected.push(candidate.clone());
            } else {
                return Err(neoforge_no_match_error(&group));
            }
        } else {
            let coordinate = group
                .first()
                .and_then(|candidate| candidate.neoforge_coordinate.clone());
            let requester = group
                .first()
                .map(|candidate| candidate.parent.clone())
                .unwrap_or_else(|| "unknown".into());
            if let Some(candidate) = group.into_iter().find(matches) {
                // JarSelector uses Collection.stream().findFirst(), preserving
                // metadata iteration order; it never chooses the maximum.
                selected.push(candidate);
            } else {
                return Err(format!(
                    "Some mods have agreed upon an acceptable version range for : §6{}§r, but no jar was provided which matched the range. Requested by: §e{}§r.",
                    coordinate.as_deref().unwrap_or("unknown:unknown"),
                    requester,
                ));
            }
        }
    }
    selected.sort_by_key(|c| c.order);
    Ok(selected)
}

fn compare_overlapping_ids(
    candidate: &NestedCandidate,
    current: &NestedCandidate,
    sign: &impl Fn(Ordering) -> i32,
) -> i32 {
    let mut score = i32::MAX;
    let mut add = |ordering| {
        if score == i32::MAX {
            score = 0;
        }
        score += sign(ordering);
    };
    for alias in &candidate.jar.fabric_provides {
        if alias == &candidate_id(&current.jar) {
            add(version_cmp(
                candidate_version(&candidate.jar).as_deref(),
                candidate_version(&current.jar).as_deref(),
            ));
        }
    }
    for alias in &current.jar.fabric_provides {
        if alias == &candidate_id(&candidate.jar) {
            add(version_cmp(
                candidate_version(&current.jar).as_deref(),
                candidate_version(&candidate.jar).as_deref(),
            ));
        }
        for candidate_alias in &candidate.jar.fabric_provides {
            if alias == candidate_alias {
                add(version_cmp(
                    candidate_version(&current.jar).as_deref(),
                    candidate_version(&candidate.jar).as_deref(),
                ));
            }
        }
    }
    score
}

fn neoforge_no_match_error(group: &[NestedCandidate]) -> String {
    let coordinate = group
        .first()
        .and_then(|candidate| candidate.neoforge_coordinate.as_deref())
        .unwrap_or("unknown:unknown");
    let requester = group
        .iter()
        .map(|candidate| candidate.parent.as_str())
        .next()
        .unwrap_or("unknown");
    format!(
        "Some mods have agreed upon an acceptable version range for : §6{coordinate}§r, but no jar was provided which matched the range. Requested by: §e{requester}§r."
    )
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
    let selected = match select_checked(jar) {
        Ok(selected) => selected,
        Err(error) => {
            return vec![NestedJar {
                entry: "<jarjar-resolution>".into(),
                result: Err(error),
            }];
        }
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
    use std::collections::BTreeMap;
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

    #[test]
    fn overlapping_fabric_alias_cycle_keeps_all_zero_score_candidates() {
        let child = |id: &str, provides: &str| {
            jar(&[(
                "fabric.mod.json",
                format!(
                    r#"{{"schemaVersion":1,"id":"{id}","version":"1","provides":["{provides}"]}}"#
                )
                .as_bytes(),
            )])
        };
        let a = child("a", "b");
        let b = child("b", "c");
        let c = child("c", "a");
        let outer = jar(&[
            ("fabric.mod.json", br#"{"schemaVersion":1,"id":"outer","version":"1","jars":[{"file":"a.jar"},{"file":"b.jar"},{"file":"c.jar"}]}"#),
            ("a.jar", &zip_bytes(&a)),
            ("b.jar", &zip_bytes(&b)),
            ("c.jar", &zip_bytes(&c)),
        ]);
        let selected = select(&outer);
        let ids = selected
            .iter()
            .filter_map(|candidate| candidate.jar.fabric_id.as_deref())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids, std::collections::BTreeSet::from(["a", "b", "c"]));
    }

    #[test]
    fn fabric_selection_matches_the_java25_mod_prio_oracle() {
        for case in include_str!("nested_jars_cases.txt").lines() {
            let (name, specs) = case.split_once('|').unwrap();
            let mut parsed = Vec::new();
            for (order, raw) in specs.split(';').filter(|raw| !raw.is_empty()).enumerate() {
                let fields = raw.split(',').collect::<Vec<_>>();
                parsed.push((
                    fields[0].to_owned(),
                    fields[1].to_owned(),
                    fields[2].to_owned(),
                    fields[3] == "1",
                    if fields[4] == "-" {
                        Vec::new()
                    } else {
                        fields[4].split('/').map(str::to_owned).collect()
                    },
                    (fields.get(5).copied().unwrap_or("-") != "-").then(|| fields[5].to_owned()),
                    order,
                ));
            }
            let mut depths = BTreeMap::new();
            for (label, _, _, root, _, parent, _) in &parsed {
                let depth = if *root {
                    0
                } else {
                    parent
                        .as_ref()
                        .and_then(|parent| depths.get(parent).copied())
                        .unwrap_or(0)
                        + 1
                };
                depths.insert(label.clone(), depth);
            }
            let candidates = parsed
                .iter()
                .map(|(label, id, version, _, provides, parent, order)| {
                    let jar = NativeJar {
                        fabric_id: Some(id.clone()),
                        fabric_version: Some(version.clone()),
                        fabric_provides: provides.clone(),
                        ..NativeJar::default()
                    };
                    let depth = depths[label];
                    let parent_chain = parent
                        .as_ref()
                        .and_then(|parent| {
                            parsed.iter().find(|(candidate, ..)| candidate == parent)
                        })
                        .map(|(_, id, version, _, _, _, _)| {
                            vec![(id.clone(), Some(version.clone()), depth - 1)]
                        })
                        .unwrap_or_default();
                    NestedCandidate {
                        entry: label.clone(),
                        parent: parent.clone().unwrap_or_else(|| "outer".into()),
                        jar,
                        depth,
                        order: *order,
                        parent_chain,
                        neoforge_coordinate: None,
                        neoforge_artifact_version: None,
                        neoforge_version_range: None,
                        neoforge_declared: false,
                    }
                })
                .collect();
            let actual = select_fabric_candidates(candidates)
                .iter()
                .map(|candidate| {
                    format!(
                        "{}@{}@{}",
                        candidate_id(&candidate.jar),
                        if candidate.depth == 0 { "r" } else { "n" },
                        candidate.depth
                    )
                })
                .collect::<Vec<_>>();
            let expected = include_str!("nested_jars_java25.txt")
                .lines()
                .find_map(|line| {
                    let (row_name, values) = line.split_once('|')?;
                    (row_name == name)
                        .then(|| values.split(',').map(str::to_owned).collect::<Vec<_>>())
                })
                .unwrap_or_else(|| panic!("missing Java oracle row {name}"));
            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn fabric_root_beats_an_overlapping_nested_candidate() {
        let inner = jar(&[(
            "fabric.mod.json",
            br#"{"id":"bb","version":"1","provides":["aa"]}"#,
        )]);
        let carrier = jar(&[
            (
                "fabric.mod.json",
                br#"{"id":"carrier","version":"1","jars":[{"file":"bb.jar"}]}"#,
            ),
            ("bb.jar", &zip_bytes(&inner)),
        ]);
        let outer = jar(&[
            (
                "fabric.mod.json",
                br#"{"id":"outer","version":"1","jars":[{"file":"aa.jar"},{"file":"carrier.jar"}]}"#,
            ),
            (
                "aa.jar",
                &zip_bytes(&jar(&[(
                    "fabric.mod.json",
                    br#"{"id":"aa","version":"2"}"#,
                )])),
            ),
            ("carrier.jar", &zip_bytes(&carrier)),
        ]);
        let selected = select(&outer);
        assert_eq!(
            selected
                .iter()
                .filter_map(|candidate| candidate.jar.fabric_id.as_deref())
                .collect::<Vec<_>>(),
            vec!["aa", "carrier"]
        );
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

    fn maven_display(range: &MavenRange) -> String {
        if let Some(recommended) = &range.recommended {
            return recommended.clone();
        }
        range
            .restrictions
            .iter()
            .map(|r| {
                format!(
                    "{}{},{}{}",
                    if r.low_inclusive { '[' } else { '(' },
                    r.low.as_deref().unwrap_or(""),
                    r.high.as_deref().unwrap_or(""),
                    if r.high_inclusive { ']' } else { ')' }
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    #[test]
    fn maven_ranges_and_restrictions_match_the_jdk25_oracle() {
        for line in include_str!("jarjar_version_java25.txt").lines() {
            let fields = line.split('|').collect::<Vec<_>>();
            match fields.first().copied() {
                Some("range") => {
                    let range = maven_range(fields[1]).unwrap();
                    assert_eq!(maven_display(&range), fields[2], "{line}");
                    assert_eq!(
                        range.recommended.as_deref().unwrap_or("null"),
                        fields[3],
                        "{line}"
                    );
                }
                Some("range-error") => {
                    assert_eq!(maven_range(fields[1]).unwrap_err(), fields[3], "{line}");
                }
                Some("restrict") => {
                    let left = maven_range(fields[1]).unwrap();
                    let right = maven_range(fields[2]).unwrap();
                    let restricted = maven_restrict(&left, &right);
                    assert_eq!(maven_display(&restricted), fields[3], "{line}");
                    assert_eq!(
                        restricted.recommended.as_deref().unwrap_or("null"),
                        fields[4],
                        "{line}"
                    );
                    if fields.len() == 7 {
                        assert_eq!(
                            maven_in_range("2", &restricted.restrictions).to_string(),
                            fields[5],
                            "{line}"
                        );
                        assert_eq!(
                            maven_in_range("1.5", &restricted.restrictions).to_string(),
                            fields[6],
                            "{line}"
                        );
                    }
                }
                _ => {}
            }
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
        assert_eq!(
            select_checked(&outer).unwrap_err(),
            "Some mods have agreed upon an acceptable version range for : §6g:a§r, but no jar was provided which matched the range. Requested by: §eouter§r."
        );
        assert_eq!(
            outer.try_seed_host(StdHost::new()).unwrap_err(),
            "Some mods have agreed upon an acceptable version range for : §6g:a§r, but no jar was provided which matched the range. Requested by: §eouter§r."
        );
    }

    fn empty_zip() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        zip::ZipWriter::new(&mut bytes).finish().unwrap();
        bytes.into_inner()
    }
}

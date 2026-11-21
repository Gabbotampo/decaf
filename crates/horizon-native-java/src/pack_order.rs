//! Fabric 26.3 (`fabric-resource-loader-v1` 3.1.0 + `fabric-api-base` 2.0.6) and Minecraft 26.3
//! (`PackRepository`) server data-pack ordering.
//!
//! ## Bytecode-verified two-stage pipeline (MC 26.3 `server-26.3.jar` + Fabric API 0.166.2+26.3)
//!
//! All bytecode offsets below were verified with `javap -c -p` against:
//! - `fabric-resource-loader-v1-3.1.0+5af7744b5d.jar` (bytecode-identical in these classes to
//!   `3.0.4+ba0d6c0d5d` from `fabric-api-0.162.0+26.3` and `3.1.0` from `0.165.0+26.3`),
//! - `fabric-api-base-2.0.6+fcdff87f5d.jar`, and
//! - Minecraft 26.3 `server-26.3.jar` (SHA-256 `a362163...`).
//!
//! ### Stage 1 — `ModPackResourcesUtil.getModResourcePacks` + `ModPackResourcesSorter` + `NodeSorting`
//! - `ModPackResourcesUtil.getModResourcePacks(FabricLoader, PackType, String)` (offsets `0..233`):
//!   - Reads `loader.getAllMods()` (`offset 9`) and collects `allModIds` (`16..48`).
//!   - Iterates every mod (`59..226`):
//!     - Skips built-in mods (`meta.getType().equals("builtin")`, `99..114`: `java`, `minecraft`).
//!     - Creates `ModNioPackResources` (`117..127`), which returns `null` if the mod exposes no
//!       valid namespace matching `[a-z0-9-_.]+` under `data/` (`ModNioPackResources.readNamespaces`
//!       `186..240`, `create` `314..334`); when `null`, skips both `addPack` and
//!       `fabric:resource_load_order` (`132..137`).
//!     - Calls `sorter.addPack(pack)` (`140..143`).
//!     - Reads `meta.getCustomValue("fabric:resource_load_order")` (`146..155`):
//!       - If `CvType.OBJECT` (`165..175`), invokes `addLoadOrdering(..., Order.BEFORE, modId)`
//!         (`187..197`) and `addLoadOrdering(..., Order.AFTER, modId)` (`200..210`).
//!       - Otherwise logs an error (`216..221`; recorded as unsupported here).
//! - `ModPackResourcesUtil.addLoadOrdering` (`0..202`):
//!   - Reads `obj.get(order.jsonKey)` where `Order.BEFORE.jsonKey == "before"` and
//!     `Order.AFTER.jsonKey == "after"` (`Order.<clinit>` `7, 22`).
//!   - Accepts `CvType.STRING` (`68..81`) or `CvType.ARRAY` of `STRING` elements (`84..149`;
//!     non-string items are skipped in bytecode and recorded as unsupported here). Any other type
//!     logs an error and returns (`152..166`; recorded as unsupported here).
//!   - Filters target mod ids by `allModIds.contains(targetId)` (`172..183`).
//!   - Calls `sorter.addLoadOrdering(targetId, modId, order)` (`lambda$addLoadOrdering$0` `0..7`).
//! - `ModPackResourcesSorter.addLoadOrdering(first=targetId, second=modId, order)` (`0..150`):
//!   - Throws `IllegalArgumentException("Tried to add a phase that depends on itself.")` when
//!     `first.equals(second)` (`14..31`; recorded as unsupported here).
//!   - `Order.BEFORE` (`ModPackResourcesSorter$1` case 1, `92..99`): `SortableNode.link(targetId, modId)`
//!     adds edge `targetId -> modId` (putting `targetId` earlier and `modId` later in `sortedPhases`,
//!     so `modId` has higher priority in `MultiPackResourceManager`).
//!   - `Order.AFTER` (`ModPackResourcesSorter$1` case 2, `102..106`): `SortableNode.link(modId, targetId)`
//!     adds edge `modId -> targetId` (putting `modId` earlier and `targetId` later in `sortedPhases`).
//!   - Calls `NodeSorting.sort(sortedPhases, "event phases", Comparator.comparing(p -> p.modId))`
//!     (`109..126`) and rebuilds `packs` (`127..136`).
//! - `NodeSorting.sort` (`fabric-api-base-2.0.6`, offsets `0..662`):
//!   - Kosaraju strongly connected components (`forwardVisit` `0..60`, reverse `58..59`,
//!     `backwardVisit` `0..57`), sorting each SCC's nodes by `modId` (`String::compareTo`, UTF-16
//!     code unit order, `125..128`), followed by Kahn's topological sort using a `PriorityQueue`
//!     ordered by `scc.nodes.get(0).modId` (`355..657`). Cycles (`scc.nodes.size() > 1`) emit a
//!     warning and output SCC members in ascending `modId` order.
//!
//! ### Stage 2 — `ModResourcePackCreator` + MC 26.3 `PackRepository` + `refreshAutoEnabledPacks`
//! - Unlike Fabric v0's single `"fabric"` group pack, `fabric-resource-loader-v1` registers every
//!   mod pack from `getModResourcePacks` as an individual `Pack` with `pack.getId() == modId`
//!   (`ModResourcePackCreator.registerModPack` `0..107`, `ModNioPackResources.create` `177..178, 251`)
//!   and sets `fabric$setParentsPredicate(BASE_PARENT)` (`registerModPack` `85..91`, where
//!   `BASE_PARENT` is `set -> set.contains("vanilla")`, `lambda$static$0` `0..8`), making every
//!   mod pack hidden (`PackMixin.fabric$isHidden` `0..15`).
//! - Nested JiJ mods are ordinary `ModContainer`s in `FabricLoader.getAllMods()` (`type == "fabric"`),
//!   NOT ordered beneath their parent mod.
//! - `PackRepository.discoverAvailable()` (`server-26.3.jar` `0..52`) collects all `RepositorySource`
//!   packs into `Maps.newTreeMap()` (`offset 0`, keyed by `pack.getId()`, ascending UTF-16
//!   `String::compareTo`) and returns `ImmutableMap.copyOf(map)` (`offset 49`).
//! - `PackRepository.rebuildSelected` (`server-26.3.jar` `0..96`) invokes
//!   `PackRepositoryMixin.handleAutoEnableDisable` (`0..8`, `@Inject` before `ImmutableList.copyOf`
//!   at `offset 92`), which calls `ModPackResourcesUtil.refreshAutoEnabledPacks(selected, available)`
//!   (`0..279`):
//!   - Removes all hidden packs (`selected.removeIf(FabricPack::fabric$isHidden)`, `31..42`).
//!   - Iterates remaining visible packs (`["vanilla", ...world datapacks...]`, `74..247`); when
//!     visiting `"vanilla"`, iterates `available.values()` (`121..201`) and inserts every hidden
//!     mod pack immediately after `"vanilla"` and before world/external datapacks (`file/*`).
//! - Because `available` is `ImmutableMap.copyOf(TreeMap)`, real MC 26.3 + `fabric-resource-loader-v1`
//!   `3.0.4`..`3.1.0` loads mod data packs into `MultiPackResourceManager` in ascending `modId`
//!   (`TreeMap`) order, between `vanilla` (lowest) and world/external datapacks (highest). This was
//!   confirmed directly on a live Fabric 26.3 server (`tests/oracles/recipes/FabricPackOrderOracle.java`,
//!   `tests/fixtures/recipes/fabric263_pack_order.txt`).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// A single jar's `data/` resource layer along with its Fabric metadata for pack ordering.
#[derive(Clone, Debug)]
pub(crate) struct JarDataLayer {
    /// Caller-supplied link label (fallback when a jar has no `fabric.mod.json` `id`).
    pub(crate) link_id: String,
    /// `fabric.mod.json` `"id"`, when this jar is a Fabric mod.
    pub(crate) fabric_mod_id: Option<String>,
    /// Raw `custom["fabric:resource_load_order"]` value from `fabric.mod.json`, if present.
    pub(crate) resource_load_order: Option<serde_json::Value>,
    /// Valid immediate `data/<ns>/` namespace directories discovered in the jar (including empty
    /// `data/<ns>/` directory entries with no files inside, matching `ModNioPackResources.readNamespaces`).
    pub(crate) data_namespaces: BTreeSet<String>,
    /// All `data/` file entries belonging to this jar.
    pub(crate) resources: BTreeMap<String, Vec<u8>>,
}

/// An unsupported or malformed `fabric:resource_load_order` constraint form discovered while
/// running `ModPackResourcesUtil.getModResourcePacks` + `ModPackResourcesSorter`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UnsupportedLoadOrder {
    /// `custom["fabric:resource_load_order"]` is not a JSON object (`getModResourcePacks:165..221`).
    NotAnObject { mod_id: String },
    /// `custom["fabric:resource_load_order"]` contains a key other than `"before"` or `"after"`.
    UnknownKey { mod_id: String, key: String },
    /// `"before"` or `"after"` is neither a string nor an array (`addLoadOrdering:27..166`).
    InvalidTargetList { mod_id: String, key: String },
    /// `"before"` or `"after"` array contains a non-string element (`addLoadOrdering:120..146`).
    NonStringTarget { mod_id: String, key: String },
    /// A mod declared `before` or `after` against its own mod id, which throws
    /// `IllegalArgumentException` in `ModPackResourcesSorter.addLoadOrdering:14..31`.
    SelfDependency { mod_id: String },
}

impl UnsupportedLoadOrder {
    pub(crate) fn method_ref(&self) -> crate::MethodRef {
        match self {
            Self::SelfDependency { .. } => crate::MethodRef {
                owner: "net/fabricmc/fabric/impl/resource/pack/ModPackResourcesSorter".into(),
                name: "addLoadOrdering".into(),
                descriptor: "(Ljava/lang/String;Ljava/lang/String;Lnet/fabricmc/fabric/impl/resource/pack/ModPackResourcesUtil$Order;)V".into(),
            },
            Self::NotAnObject { .. }
            | Self::UnknownKey { .. }
            | Self::InvalidTargetList { .. }
            | Self::NonStringTarget { .. } => crate::MethodRef {
                owner: "net/fabricmc/fabric/impl/resource/pack/ModPackResourcesUtil".into(),
                name: "addLoadOrdering".into(),
                descriptor: "(Lnet/fabricmc/loader/api/metadata/CustomValue$CvObject;Ljava/util/List;Lnet/fabricmc/fabric/impl/resource/pack/ModPackResourcesSorter;Lnet/fabricmc/fabric/impl/resource/pack/ModPackResourcesUtil$Order;Ljava/lang/String;)V".into(),
            },
        }
    }
}

/// Input describing one candidate mod for [`sort_mod_pack_resources`].
#[derive(Clone, Debug)]
pub(crate) struct ModPackInput<'a> {
    pub(crate) mod_id: &'a str,
    pub(crate) is_builtin: bool,
    pub(crate) has_valid_data_namespace: bool,
    pub(crate) resource_load_order: Option<&'a serde_json::Value>,
}

/// Result of Stage 1 (`ModPackResourcesUtil.getModResourcePacks` + `ModPackResourcesSorter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SorterResult {
    /// Mod ids in `ModPackResourcesSorter.getPacks()` order (lowest -> highest priority).
    pub(crate) sorted_mod_ids: Vec<String>,
    /// Unsupported/malformed constraint forms encountered.
    pub(crate) unsupported: Vec<UnsupportedLoadOrder>,
    /// `false` if `NodeSorting.sort` detected a cycle (matching `NodeSorting.sort` return value).
    pub(crate) acyclic: bool,
}

/// Result of the full two-stage MC 26.3 + Fabric 3.1.0 `PackRepository` ordering.
pub(crate) struct OrderedDataPacks<'a> {
    /// Effective `data/` layers from lowest to highest priority (above vanilla):
    /// mod packs in MC 26.3 `PackRepository` + `refreshAutoEnabledPacks` order, followed by
    /// non-Fabric layers (if any) and explicit world/external datapack layers.
    pub(crate) layers: Vec<(&'a str, &'a BTreeMap<String, Vec<u8>>)>,
    /// Stage 1 `ModPackResourcesSorter` order (prior to `PackRepository.discoverAvailable`'s
    /// `TreeMap`), retained for diagnostics and direct parity tests.
    #[allow(dead_code)]
    pub(crate) sorter_mod_order: Vec<String>,
    /// Unsupported/malformed `fabric:resource_load_order` forms encountered in Stage 1.
    pub(crate) unsupported_constraints: Vec<UnsupportedLoadOrder>,
}

/// Java `String.compareTo` (UTF-16 code unit lexicographical order).
pub(crate) fn cmp_java_utf16(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Wrapper implementing `Ord` via Java UTF-16 `String::compareTo` (matching `TreeMap<String, _>`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct JavaStringKey(String);

impl Ord for JavaStringKey {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_java_utf16(&self.0, &other.0)
    }
}

impl PartialOrd for JavaStringKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Check whether `ns` matches `ModNioPackResources.RESOURCE_PACK_PATH` (`[a-z0-9-_.]+`,
/// `ModNioPackResources.<clinit>:8..14`, `readNamespaces:186..197`, verified on Java 25).
pub(crate) fn is_valid_namespace(ns: &str) -> bool {
    !ns.is_empty()
        && ns
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.'))
}

/// Extract a valid Fabric `ModNioPackResources` server-data namespace from a ZIP entry path
/// (`ModNioPackResources.readNamespaces:55..244`, `Files.isDirectory` at `87, 152`,
/// `RESOURCE_PACK_PATH` match at `186..214`).
///
/// Matches JDK `ZipFileSystem` directory semantics:
/// - Any entry under `data/<ns>/...` — including an empty directory entry `data/<ns>/` with no
///   files inside — yields an immediate child directory `<ns>` of `data/`.
/// - A bare file directly inside `data/` (`data/<file>`, no slash after `<file>`) fails
///   `Files.isDirectory` (`readNamespaces:152`) and returns `None`.
/// - `.` and `..` path segments normalize as in `ZipFileSystem`.
/// - `<ns>` must match `[a-z0-9-_.]+`.
pub(crate) fn valid_data_namespace_from_entry(entry_name: &str) -> Option<String> {
    let mut normalized: Vec<&str> = Vec::new();
    for seg in entry_name.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    if normalized.first().copied() != Some("data") {
        return None;
    }
    let ns = *normalized.get(1)?;
    let is_dir = normalized.len() >= 3 || (normalized.len() == 2 && entry_name.ends_with('/'));
    (is_dir && is_valid_namespace(ns)).then(|| ns.to_owned())
}

/// Check whether a jar exposes at least one valid `data/<ns>/` namespace directory matching
/// `ModNioPackResources.readNamespaces` (`0..338`) and `ModNioPackResources.create` (`314..334`).
pub(crate) fn has_valid_data_namespace(
    resources: &BTreeMap<String, Vec<u8>>,
    namespaces: &BTreeSet<String>,
) -> bool {
    namespaces.iter().any(|ns| is_valid_namespace(ns))
        || resources
            .keys()
            .any(|path| valid_data_namespace_from_entry(path).is_some())
}

#[derive(Default)]
struct PhaseNode {
    mod_id: String,
    has_pack: bool,
    subsequent: Vec<usize>,
    previous: Vec<usize>,
    visited: bool,
}

#[derive(Clone, Copy)]
enum LoadOrder {
    Before,
    After,
}

impl LoadOrder {
    fn json_key(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

/// Exact port of `ModPackResourcesUtil.getModResourcePacks` (`0..233`), `addLoadOrdering`
/// (`0..202`), `ModPackResourcesSorter` (`addPack:0..61`, `addLoadOrdering:0..150`), and
/// `fabric-api-base` `NodeSorting.sort` (`0..662`).
pub(crate) fn sort_mod_pack_resources(
    mods: &[ModPackInput<'_>],
    all_loaded_mod_ids: &BTreeSet<String>,
) -> SorterResult {
    // FabricLoader.getAllMods() is in mod-id ascending order (`ModResolver.findCompatibleSet:841`).
    let mut ordered_mods: Vec<&ModPackInput<'_>> = mods.iter().collect();
    ordered_mods.sort_by(|a, b| cmp_java_utf16(a.mod_id, b.mod_id));

    let mut all_ids: BTreeSet<String> = all_loaded_mod_ids.clone();
    for m in &ordered_mods {
        all_ids.insert(m.mod_id.to_owned());
    }

    let mut phases: Vec<PhaseNode> = Vec::new();
    let mut phase_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut sorted_phases: Vec<usize> = Vec::new();
    let mut unsupported: Vec<UnsupportedLoadOrder> = Vec::new();
    let mut acyclic = true;

    let get_or_create = |mod_id: &str,
                         phases: &mut Vec<PhaseNode>,
                         phase_index: &mut BTreeMap<String, usize>,
                         sorted_phases: &mut Vec<usize>|
     -> usize {
        if let Some(&idx) = phase_index.get(mod_id) {
            return idx;
        }
        let idx = phases.len();
        phases.push(PhaseNode {
            mod_id: mod_id.to_owned(),
            has_pack: false,
            subsequent: Vec::new(),
            previous: Vec::new(),
            visited: false,
        });
        phase_index.insert(mod_id.to_owned(), idx);
        sorted_phases.push(idx);
        idx
    };

    for m in ordered_mods {
        // ModPackResourcesUtil.getModResourcePacks:99..114 — skip builtin mods.
        if m.is_builtin {
            continue;
        }
        // ModPackResourcesUtil.getModResourcePacks:117..137 — skip mods with no valid `data/<ns>/`.
        if !m.has_valid_data_namespace {
            continue;
        }
        // ModPackResourcesSorter.addPack:28..35
        let idx = get_or_create(m.mod_id, &mut phases, &mut phase_index, &mut sorted_phases);
        phases[idx].has_pack = true;
        // ModPackResourcesSorter.getOrCreatePhase:50..71 sorts on each new phase.
        acyclic &= node_sorting_sort(&mut phases, &mut sorted_phases);

        // ModPackResourcesUtil.getModResourcePacks:146..221
        let Some(cv) = m.resource_load_order else {
            continue;
        };
        let Some(obj) = cv.as_object() else {
            unsupported.push(UnsupportedLoadOrder::NotAnObject {
                mod_id: m.mod_id.to_owned(),
            });
            continue;
        };
        for key in obj.keys() {
            if key != "before" && key != "after" {
                unsupported.push(UnsupportedLoadOrder::UnknownKey {
                    mod_id: m.mod_id.to_owned(),
                    key: key.clone(),
                });
            }
        }
        for order in [LoadOrder::Before, LoadOrder::After] {
            let Some(val) = obj.get(order.json_key()) else {
                continue;
            };
            // ModPackResourcesUtil.addLoadOrdering:27..166
            let mut targets: Vec<&str> = Vec::new();
            match val {
                serde_json::Value::String(s) => targets.push(s.as_str()),
                serde_json::Value::Array(arr) => {
                    for elem in arr {
                        if let Some(s) = elem.as_str() {
                            targets.push(s);
                        } else {
                            unsupported.push(UnsupportedLoadOrder::NonStringTarget {
                                mod_id: m.mod_id.to_owned(),
                                key: order.json_key().to_owned(),
                            });
                        }
                    }
                }
                _ => {
                    unsupported.push(UnsupportedLoadOrder::InvalidTargetList {
                        mod_id: m.mod_id.to_owned(),
                        key: order.json_key().to_owned(),
                    });
                    continue;
                }
            }
            // ModPackResourcesUtil.addLoadOrdering:167..202 -> lambda$addLoadOrdering$0:0..7
            for target_id in targets {
                if !all_ids.contains(target_id) {
                    continue;
                }
                // ModPackResourcesSorter.addLoadOrdering(first=target_id, second=m.mod_id, order)
                // Offset 14..31: self-dependency throws IllegalArgumentException in Fabric.
                if target_id == m.mod_id {
                    unsupported.push(UnsupportedLoadOrder::SelfDependency {
                        mod_id: m.mod_id.to_owned(),
                    });
                    continue;
                }
                let first =
                    get_or_create(target_id, &mut phases, &mut phase_index, &mut sorted_phases);
                let second =
                    get_or_create(m.mod_id, &mut phases, &mut phase_index, &mut sorted_phases);
                // ModPackResourcesSorter.addLoadOrdering:56..106 + SortableNode.link:15..25
                match order {
                    // Case 1 (BEFORE): SortableNode.link(first=targetId, second=modId)
                    LoadOrder::Before => {
                        phases[first].subsequent.push(second);
                        phases[second].previous.push(first);
                    }
                    // Case 2 (AFTER): SortableNode.link(second=modId, first=targetId)
                    LoadOrder::After => {
                        phases[second].subsequent.push(first);
                        phases[first].previous.push(second);
                    }
                }
                // ModPackResourcesSorter.addLoadOrdering:109..126
                acyclic &= node_sorting_sort(&mut phases, &mut sorted_phases);
            }
        }
    }

    let sorted_mod_ids = sorted_phases
        .into_iter()
        .filter(|&idx| phases[idx].has_pack)
        .map(|idx| phases[idx].mod_id.clone())
        .collect();

    SorterResult {
        sorted_mod_ids,
        unsupported,
        acyclic,
    }
}

/// Exact port of `net.fabricmc.fabric.impl.base.toposort.NodeSorting.sort`
/// (`fabric-api-base-2.0.6+fcdff87f5d.jar`, offsets `0..662`).
fn node_sorting_sort(phases: &mut [PhaseNode], sorted_phases: &mut Vec<usize>) -> bool {
    // 1. Forward post-order DFS (`offsets 0..51`, `forwardVisit:0..60`).
    fn forward_visit(idx: usize, phases: &mut [PhaseNode], out: &mut Vec<usize>) {
        if phases[idx].visited {
            return;
        }
        phases[idx].visited = true;
        let nexts = phases[idx].subsequent.clone();
        for next in nexts {
            forward_visit(next, phases, out);
        }
        out.push(idx);
    }

    let mut all_nodes = Vec::with_capacity(sorted_phases.len());
    for &idx in sorted_phases.iter() {
        forward_visit(idx, phases, &mut all_nodes);
    }
    // `clearStatus` (`54..55`) + `Collections.reverse` (`58..59`).
    for &idx in &all_nodes {
        phases[idx].visited = false;
    }
    all_nodes.reverse();

    // 2. Backward DFS over `previousNodes` to collect Kosaraju SCCs (`71..190`, `backwardVisit:0..57`).
    fn backward_visit(idx: usize, phases: &mut [PhaseNode], scc: &mut Vec<usize>) {
        if phases[idx].visited {
            return;
        }
        phases[idx].visited = true;
        scc.push(idx);
        let prevs = phases[idx].previous.clone();
        for prev in prevs {
            backward_visit(prev, phases, scc);
        }
    }

    struct Scc {
        nodes: Vec<usize>,
        subsequent_sccs: Vec<usize>,
        in_degree: i32,
    }

    let mut sccs: Vec<Scc> = Vec::new();
    let mut node_to_scc: Vec<usize> = vec![usize::MAX; phases.len()];
    for &idx in &all_nodes {
        if !phases[idx].visited {
            let mut nodes = Vec::new();
            backward_visit(idx, phases, &mut nodes);
            // `nodes.sort(comparator)` (`125..128`): ascending UTF-16 `modId`.
            nodes.sort_by(|&a, &b| cmp_java_utf16(&phases[a].mod_id, &phases[b].mod_id));
            let scc_id = sccs.len();
            for &n in &nodes {
                node_to_scc[n] = scc_id;
            }
            sccs.push(Scc {
                nodes,
                subsequent_sccs: Vec::new(),
                in_degree: 0,
            });
        }
    }
    for &idx in &all_nodes {
        phases[idx].visited = false;
    }

    // 3. Build condensation DAG (`197..352`).
    for scc_id in 0..sccs.len() {
        let nodes = sccs[scc_id].nodes.clone();
        for n in nodes {
            for &next in &phases[n].subsequent {
                let target_scc = node_to_scc[next];
                if target_scc != scc_id {
                    sccs[scc_id].subsequent_sccs.push(target_scc);
                    sccs[target_scc].in_degree += 1;
                }
            }
        }
    }

    // 4. Kahn topological sort with min-heap comparing `scc.nodes[0].modId` (`355..662`).
    #[derive(Eq, PartialEq)]
    struct HeapEntry {
        first_mod_id: String,
        scc_id: usize,
    }

    impl Ord for HeapEntry {
        fn cmp(&self, other: &Self) -> Ordering {
            // Reversed for min-heap on Rust's max-heap `BinaryHeap`.
            cmp_java_utf16(&other.first_mod_id, &self.first_mod_id)
                .then_with(|| other.scc_id.cmp(&self.scc_id))
        }
    }

    impl PartialOrd for HeapEntry {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut pq: BinaryHeap<HeapEntry> = BinaryHeap::new();
    sorted_phases.clear();
    for (scc_id, scc) in sccs.iter_mut().enumerate() {
        if scc.in_degree == 0 {
            pq.push(HeapEntry {
                first_mod_id: phases[scc.nodes[0]].mod_id.clone(),
                scc_id,
            });
            scc.in_degree = -1;
        }
    }

    let mut no_cycles = true;
    while let Some(entry) = pq.pop() {
        let scc_id = entry.scc_id;
        sorted_phases.extend_from_slice(&sccs[scc_id].nodes);
        if sccs[scc_id].nodes.len() > 1 {
            no_cycles = false;
        }
        let next_sccs = sccs[scc_id].subsequent_sccs.clone();
        for target_scc in next_sccs {
            sccs[target_scc].in_degree -= 1;
            if sccs[target_scc].in_degree == 0 {
                pq.push(HeapEntry {
                    first_mod_id: phases[sccs[target_scc].nodes[0]].mod_id.clone(),
                    scc_id: target_scc,
                });
            }
        }
    }

    no_cycles
}

/// Resolve the effective `data/` pack priority order (lowest -> highest, above vanilla) across:
/// 1. All Fabric mod jars (primary seeded jar + linked jars + nested JiJ jars) via Stage 1
///    (`ModPackResourcesUtil.getModResourcePacks` + `ModPackResourcesSorter` + `NodeSorting`)
///    followed by Stage 2 (MC 26.3 `PackRepository.discoverAvailable` `Maps.newTreeMap()` +
///    `PackRepositoryMixin.handleAutoEnableDisable` -> `ModPackResourcesUtil.refreshAutoEnabledPacks`),
/// 2. Non-Fabric jar / bare `with_resource_bytes` layers (if any), and
/// 3. Explicit world/external datapack layers (`add_data_pack_layer`, highest priority).
pub(crate) fn resolve_data_pack_order<'a>(
    linked_layers: &'a [JarDataLayer],
    primary_mod_id: Option<&'a str>,
    primary_load_order: Option<&'a serde_json::Value>,
    primary_data_layer: &'a BTreeMap<String, Vec<u8>>,
    primary_data_namespaces: &BTreeSet<String>,
    all_loaded_mod_ids: &BTreeSet<String>,
    explicit_datapacks: &'a [(String, BTreeMap<String, Vec<u8>>)],
) -> OrderedDataPacks<'a> {
    let mut fabric_inputs: Vec<ModPackInput<'a>> = Vec::new();
    let mut fabric_layers_by_id: BTreeMap<JavaStringKey, (&'a str, &'a BTreeMap<String, Vec<u8>>)> =
        BTreeMap::new();
    let mut non_fabric_layers: Vec<(&'a str, &'a BTreeMap<String, Vec<u8>>)> = Vec::new();

    for entry in linked_layers {
        if let Some(mod_id) = entry.fabric_mod_id.as_deref() {
            let valid_ns = has_valid_data_namespace(&entry.resources, &entry.data_namespaces);
            fabric_inputs.push(ModPackInput {
                mod_id,
                is_builtin: false,
                has_valid_data_namespace: valid_ns,
                resource_load_order: entry.resource_load_order.as_ref(),
            });
            if valid_ns {
                fabric_layers_by_id
                    .insert(JavaStringKey(mod_id.to_owned()), (mod_id, &entry.resources));
            }
        } else if !entry.resources.is_empty() {
            non_fabric_layers.push((entry.link_id.as_str(), &entry.resources));
        }
    }

    if !primary_data_layer.is_empty() || !primary_data_namespaces.is_empty() {
        if let Some(mod_id) = primary_mod_id {
            let valid_ns = has_valid_data_namespace(primary_data_layer, primary_data_namespaces);
            fabric_inputs.push(ModPackInput {
                mod_id,
                is_builtin: false,
                has_valid_data_namespace: valid_ns,
                resource_load_order: primary_load_order,
            });
            if valid_ns {
                fabric_layers_by_id.insert(
                    JavaStringKey(mod_id.to_owned()),
                    (mod_id, primary_data_layer),
                );
            }
        } else if !primary_data_layer.is_empty() {
            non_fabric_layers.push(("primary", primary_data_layer));
        }
    }

    // Stage 1: run ModPackResourcesUtil.getModResourcePacks + ModPackResourcesSorter + NodeSorting.
    let sorter = sort_mod_pack_resources(&fabric_inputs, all_loaded_mod_ids);

    // Stage 2: MC 26.3 PackRepository.discoverAvailable (`Maps.newTreeMap()`, server-26.3.jar:0..52)
    // + PackRepositoryMixin.handleAutoEnableDisable -> ModPackResourcesUtil.refreshAutoEnabledPacks
    // (`fabric-resource-loader-v1-3.1.0:31..42, 121..201`), which places all hidden mod packs
    // immediately after `"vanilla"` in `available.values()` (`TreeMap`, UTF-16 `modId` ascending)
    // order, before world/external datapacks.
    let mut tree_available: BTreeMap<JavaStringKey, (&'a str, &'a BTreeMap<String, Vec<u8>>)> =
        BTreeMap::new();
    for mod_id in &sorter.sorted_mod_ids {
        let key = JavaStringKey(mod_id.clone());
        if let Some(&layer) = fabric_layers_by_id.get(&key) {
            tree_available.insert(key, layer);
        }
    }

    let mut layers = Vec::with_capacity(
        tree_available.len() + non_fabric_layers.len() + explicit_datapacks.len(),
    );
    for layer in tree_available.into_values() {
        layers.push(layer);
    }
    for layer in non_fabric_layers {
        layers.push(layer);
    }
    for (id, map) in explicit_datapacks {
        layers.push((id.as_str(), map));
    }

    OrderedDataPacks {
        layers,
        sorter_mod_order: sorter.sorted_mod_ids,
        unsupported_constraints: sorter.unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with_recipe(ns: &str) -> BTreeMap<String, Vec<u8>> {
        let mut m = BTreeMap::new();
        m.insert(format!("data/{ns}/recipe/r.json"), b"{}".to_vec());
        m
    }

    #[test]
    fn sorter_and_pack_repository_match_real_fabric_263_oracle() {
        let before_c: serde_json::Value = serde_json::json!({"before": ["mod_c"]});
        let after_x: serde_json::Value = serde_json::json!({"after": ["mod_x"]});
        let cyc_b: serde_json::Value = serde_json::json!({"before": ["cyc_b"]});
        let cyc_a: serde_json::Value = serde_json::json!({"before": ["cyc_a"]});
        let bad_form: serde_json::Value = serde_json::json!("not_an_object");
        let empty_bridge_order: serde_json::Value =
            serde_json::json!({"before": ["emp_z"], "after": ["emp_a"]});
        let empty_invalid_order: serde_json::Value =
            serde_json::json!({"before": ["inv_z"], "after": ["inv_a"]});

        // Supply in a deliberately scrambled link order that disagrees with both id order
        // and `fabric:resource_load_order`.
        let linked = vec![
            JarDataLayer {
                link_id: "link_1".into(),
                fabric_mod_id: Some("mod_c".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_2".into(),
                fabric_mod_id: Some("mod_a".into()),
                resource_load_order: Some(before_c),
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_3".into(),
                fabric_mod_id: Some("mod_b".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_4".into(),
                fabric_mod_id: Some("mod_z".into()),
                resource_load_order: Some(after_x),
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_5".into(),
                fabric_mod_id: Some("mod_x".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_6".into(),
                fabric_mod_id: Some("mod_y".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "mod_a!META-INF/jars/nested_zzz.jar".into(),
                fabric_mod_id: Some("nested_zzz".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "mod_c!META-INF/jars/nested_aaa.jar".into(),
                fabric_mod_id: Some("nested_aaa".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_9".into(),
                fabric_mod_id: Some("cyc_b".into()),
                resource_load_order: Some(cyc_a),
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_10".into(),
                fabric_mod_id: Some("cyc_a".into()),
                resource_load_order: Some(cyc_b),
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_11".into(),
                fabric_mod_id: Some("bad_form".into()),
                resource_load_order: Some(bad_form),
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_12".into(),
                fabric_mod_id: Some("emp_a".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_13".into(),
                fabric_mod_id: Some("emp_z".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            // Empty `data/empty_ns/` directory with ZERO files inside `resources`:
            JarDataLayer {
                link_id: "link_14".into(),
                fabric_mod_id: Some("empty_bridge".into()),
                resource_load_order: Some(empty_bridge_order),
                data_namespaces: BTreeSet::from(["empty_ns".to_owned()]),
                resources: BTreeMap::new(),
            },
            JarDataLayer {
                link_id: "link_15".into(),
                fabric_mod_id: Some("inv_a".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            JarDataLayer {
                link_id: "link_16".into(),
                fabric_mod_id: Some("inv_z".into()),
                resource_load_order: None,
                data_namespaces: BTreeSet::new(),
                resources: map_with_recipe("orderprobe"),
            },
            // Invalid uppercase dir + bare file under `data/`: must be excluded with its edges.
            JarDataLayer {
                link_id: "link_17".into(),
                fabric_mod_id: Some("empty_invalid".into()),
                resource_load_order: Some(empty_invalid_order),
                data_namespaces: BTreeSet::new(),
                resources: BTreeMap::from([(
                    "data/plain_file.txt".to_owned(),
                    b"not a directory".to_vec(),
                )]),
            },
        ];

        let dp = vec![("file/dp_override".to_owned(), map_with_recipe("orderprobe"))];
        let empty = BTreeMap::new();
        let empty_ns = BTreeSet::new();
        let all_ids = BTreeSet::new();
        let ordered =
            resolve_data_pack_order(&linked, None, None, &empty, &empty_ns, &all_ids, &dp);

        // Stage 1 matches `SORTER_MOD_ORDER` captured from real Fabric 26.3:
        assert_eq!(
            ordered.sorter_mod_order,
            vec![
                "bad_form",
                "cyc_a",
                "cyc_b",
                "emp_z",
                "empty_bridge",
                "emp_a",
                "inv_a",
                "inv_z",
                "mod_b",
                "mod_c",
                "mod_a",
                "mod_y",
                "mod_z",
                "mod_x",
                "nested_aaa",
                "nested_zzz",
            ]
        );
        // Stage 2 matches `RM_PACK_ORDER` (above `vanilla`) captured from real Fabric 26.3:
        let effective_ids: Vec<&str> = ordered.layers.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            effective_ids,
            vec![
                "bad_form",
                "cyc_a",
                "cyc_b",
                "emp_a",
                "emp_z",
                "empty_bridge",
                "inv_a",
                "inv_z",
                "mod_a",
                "mod_b",
                "mod_c",
                "mod_x",
                "mod_y",
                "mod_z",
                "nested_aaa",
                "nested_zzz",
                "file/dp_override",
            ]
        );
        assert_eq!(
            ordered.unsupported_constraints,
            vec![UnsupportedLoadOrder::NotAnObject {
                mod_id: "bad_form".into(),
            }]
        );
    }

    #[test]
    fn valid_data_namespace_matches_mod_nio_read_namespaces() {
        assert_eq!(
            valid_data_namespace_from_entry("data/empty_ns/"),
            Some("empty_ns".to_owned())
        );
        assert_eq!(
            valid_data_namespace_from_entry("data/a-b_c.0/"),
            Some("a-b_c.0".to_owned())
        );
        assert_eq!(
            valid_data_namespace_from_entry("data/mod_x/recipe/a.json"),
            Some("mod_x".to_owned())
        );
        assert_eq!(
            valid_data_namespace_from_entry("data/./empty_ns/"),
            Some("empty_ns".to_owned())
        );
        assert_eq!(valid_data_namespace_from_entry("data/"), None);
        assert_eq!(valid_data_namespace_from_entry("data/plain_file.txt"), None);
        assert_eq!(valid_data_namespace_from_entry("data/Bad_NS/"), None);
        assert_eq!(valid_data_namespace_from_entry("data/bad space/"), None);
        assert_eq!(valid_data_namespace_from_entry("data/bad:ns/"), None);
        assert_eq!(valid_data_namespace_from_entry("data/./"), None);
        assert_eq!(valid_data_namespace_from_entry("data/../"), None);
        assert_eq!(valid_data_namespace_from_entry("assets/empty_ns/"), None);
    }

    #[test]
    fn unsupported_constraint_forms_are_recorded() {
        let self_dep = serde_json::json!({"before": ["m1"]});
        let bad_target = serde_json::json!({"before": 42, "after": ["m1", 99], "priority": 1});
        let all_ids = BTreeSet::from(["m1".to_owned(), "m2".to_owned()]);
        let res = sort_mod_pack_resources(
            &[
                ModPackInput {
                    mod_id: "m1",
                    is_builtin: false,
                    has_valid_data_namespace: true,
                    resource_load_order: Some(&self_dep),
                },
                ModPackInput {
                    mod_id: "m2",
                    is_builtin: false,
                    has_valid_data_namespace: true,
                    resource_load_order: Some(&bad_target),
                },
            ],
            &all_ids,
        );
        assert!(
            res.unsupported
                .contains(&UnsupportedLoadOrder::SelfDependency {
                    mod_id: "m1".into(),
                })
        );
        assert!(res.unsupported.contains(&UnsupportedLoadOrder::UnknownKey {
            mod_id: "m2".into(),
            key: "priority".into(),
        }));
        assert!(
            res.unsupported
                .contains(&UnsupportedLoadOrder::InvalidTargetList {
                    mod_id: "m2".into(),
                    key: "before".into(),
                })
        );
        assert!(
            res.unsupported
                .contains(&UnsupportedLoadOrder::NonStringTarget {
                    mod_id: "m2".into(),
                    key: "after".into(),
                })
        );
    }
}

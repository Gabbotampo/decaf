//! Exact Minecraft 26.3 `VoxelShape` / `Shapes` / `Block.box` / `AABB` / `BooleanOp` /
//! `CollisionContext` models and post-load `Block.getShape` evaluation across all states.
//!
//! Geometry follows the real 26.3 `BitSetDiscreteVoxelShape`, `CubePointRange`,
//! `IndexMerger` (`DiscreteCubeMerger`, `NonOverlappingMerger`, `IdenticalMerger`,
//! `IndirectMerger`), `Shapes`, and `VoxelShape` bytecode:
//!
//! * `Block.box(x0, y0, z0, x1, y1, z1)` divides each coordinate by `16.0` and delegates to
//!   `Shapes.box`.
//! * `Shapes.box` checks `min > max` on each axis (`IllegalArgumentException`) and delegates to
//!   `Shapes.create`, which returns `Shapes.empty()` when any span is `< 1.0e-7`, `Shapes.block()`
//!   for the unit cube `[0, 0, 0, 1, 1, 1]`, a `CubeVoxelShape` for power-of-two grids in
//!   `[0, 1]` (`findBits`), or an `ArrayVoxelShape` otherwise.
//! * `Shapes.or` runs `Shapes.join(..., BooleanOp.OR)` = `joinUnoptimized(...).optimize()`.
//! * `VoxelShape.toAabbs()` / `to_boxes()` runs `BitSetDiscreteVoxelShape.forAllBoxes(..., true)`,
//!   producing the exact box list and order of real 26.3 `VoxelShape.toAabbs()`.
//! * Unmodelled methods on `VoxelShape`, `Shapes`, `BlockGetter`, or `CollisionContext` fall
//!   through to `StdHost::unknown_host_call` (counted).

use std::collections::BTreeMap;

use crate::runtime::{
    ExecutionBudget, Host, MethodRef, NativeError, NativeJar, RegistryDeclaration, StdHost, Value,
    expect_float, expect_int, expect_ref,
    host_ref::{ClassHierarchy, HostKind, HostRef, Methods},
};

pub(crate) const VOXEL_SHAPE: &str = "net/minecraft/world/phys/shapes/VoxelShape";
pub(crate) const CUBE_VOXEL_SHAPE: &str = "net/minecraft/world/phys/shapes/CubeVoxelShape";
pub(crate) const ARRAY_VOXEL_SHAPE: &str = "net/minecraft/world/phys/shapes/ArrayVoxelShape";
pub(crate) const SHAPES: &str = "net/minecraft/world/phys/shapes/Shapes";
pub(crate) const BOOLEAN_OP: &str = "net/minecraft/world/phys/shapes/BooleanOp";
pub(crate) const AABB: &str = "net/minecraft/world/phys/AABB";
pub(crate) const COLLISION_CONTEXT: &str = "net/minecraft/world/phys/shapes/CollisionContext";
pub(crate) const ENTITY_COLLISION_CONTEXT: &str =
    "net/minecraft/world/phys/shapes/EntityCollisionContext";
pub(crate) const EMPTY_COLLISION_CONTEXT: &str =
    "net/minecraft/world/phys/shapes/EntityCollisionContext$Empty";
pub(crate) const BLOCK_CLASS: &str = "net/minecraft/world/level/block/Block";

pub(crate) const GET_SHAPE_DESC: &str = "(Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/BlockGetter;Lnet/minecraft/core/BlockPos;Lnet/minecraft/world/phys/shapes/CollisionContext;)Lnet/minecraft/world/phys/shapes/VoxelShape;";
pub(crate) const BLOCK_STATE_GET_SHAPE_2_DESC: &str = "(Lnet/minecraft/world/level/BlockGetter;Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/phys/shapes/VoxelShape;";
pub(crate) const BLOCK_STATE_GET_SHAPE_3_DESC: &str = "(Lnet/minecraft/world/level/BlockGetter;Lnet/minecraft/core/BlockPos;Lnet/minecraft/world/phys/shapes/CollisionContext;)Lnet/minecraft/world/phys/shapes/VoxelShape;";

const EPSILON: f64 = 1.0e-7;
/// Safety cap on discrete cells in a single joined shape, preventing adversarial memory blowups.
const MAX_VOXEL_CELLS: usize = 65_536;
/// Safety cap on states evaluated per block in `NativeJar::block_shapes`.
const MAX_BLOCK_SHAPE_STATES: usize = 4_096;
const MAX_SUPERCLASS_WALK: usize = 64;

const EMPTY_KEY: &str = "empty";
const BLOCK_KEY: &str = "block";
const INFINITY_KEY: &str = "infinity";

/// One block state's evaluated outline or collision shape, in block units (`0.0..=1.0` for a full
/// cube), matching MC 26.3 `VoxelShape.toAabbs()`.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockStateShape {
    /// Property name -> serialized property value (e.g. `{"facing": "north"}`).
    pub state: BTreeMap<String, String>,
    /// Exact boxes `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    pub boxes: Vec<[f64; 6]>,
}

/// Resolved outline and collision shapes for a block across all of its states.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockShapes {
    /// Ordered state property names (`BTreeMap` key order). Empty for stateless blocks.
    pub properties: Vec<String>,
    /// Per-state outline box lists in Cartesian product order of `properties`, or `None` when the
    /// block's outline shape could not be determined without unmodelled calls / dependencies.
    pub states: Option<Vec<BlockStateShape>>,
    /// `true` when outline shapes were resolved by invoking a jar-side `getShape` override;
    /// `false` when inherited directly from vanilla `BlockBehaviour.getShape` (`Shapes.block()`).
    pub overridden: bool,
    /// Per-state collision box lists in the same order as `states`, or `None` when collision
    /// equivalence/shape could not be proven (e.g. `getCollisionShape` read position/world/context,
    /// vanilla superclass overrides `getCollisionShape`, or `Properties.ofFullCopy` left
    /// `hasCollision` unknown).
    pub collision_states: Option<Vec<BlockStateShape>>,
    /// `true` when collision shapes were resolved by invoking a jar-side `getCollisionShape`
    /// override; `false` when inherited from `BlockBehaviour.getCollisionShape`.
    pub collision_overridden: bool,
}

fn encode_content_shapes(
    properties: &[String],
    states: &[BlockStateShape],
) -> Option<crate::runtime::content::BlockShapes> {
    if states.is_empty() {
        return None;
    }
    if !states
        .iter()
        .all(|entry| entry.boxes.iter().all(is_valid_sunrise_box))
    {
        return None;
    }
    if let Some(first) = states.first()
        && states.iter().all(|entry| entry.boxes == first.boxes)
    {
        return Some(crate::runtime::content::BlockShapes {
            properties: Vec::new(),
            values: BTreeMap::new(),
            default: Some(first.boxes.clone()),
        });
    }
    // State-dependent shapes: drop properties that never distinguish two shapes so the
    // exported Sunrise table is minimal and every tuple is matched by real states.
    let mut active_props: Vec<String> = Vec::new();
    for prop in properties {
        if !is_sunrise_state_name(prop) {
            return None;
        }
        let varies = states.iter().any(|a| {
            states.iter().any(|b| {
                a.boxes != b.boxes
                    && properties
                        .iter()
                        .all(|other| other == prop || a.state.get(other) == b.state.get(other))
            })
        });
        if varies {
            active_props.push(prop.clone());
        }
    }
    if active_props.is_empty() {
        return None;
    }
    let mut values: BTreeMap<String, Vec<[f64; 6]>> = BTreeMap::new();
    for entry in states {
        let mut key_parts = Vec::with_capacity(active_props.len());
        for prop in &active_props {
            let value = entry.state.get(prop)?;
            if !is_sunrise_state_name(value) {
                return None;
            }
            key_parts.push(value.as_str());
        }
        let key = key_parts.join(",");
        if let Some(existing) = values.get(&key) {
            if existing != &entry.boxes {
                return None;
            }
        } else {
            values.insert(key, entry.boxes.clone());
        }
    }
    Some(crate::runtime::content::BlockShapes {
        properties: active_props,
        values,
        default: None,
    })
}

impl BlockShapes {
    /// If every state resolved cleanly and agrees on one outline box list, returns that box list.
    pub fn uniform_boxes(&self) -> Option<&[[f64; 6]]> {
        let states = self.states.as_deref()?;
        let first = states.first()?;
        if states.iter().all(|entry| entry.boxes == first.boxes) {
            Some(&first.boxes)
        } else {
            None
        }
    }

    /// Convert into Sunrise's top-level `(shapes, collision_shapes)` pair.
    ///
    /// Because Sunrise falls back from `collision_shapes` to `shapes`, returns `(None, None)`
    /// unless BOTH outline and collision shapes resolved cleanly for every state.
    /// * `shapes` is emitted only when `getShape` was overridden by the jar (inherited
    ///   `BlockBehaviour.getShape` is already Sunrise's full-cube default).
    /// * `collision_shapes` is emitted only when `getCollisionShape` differs from `getShape` on
    ///   at least one state (e.g. `Properties.noCollision()` or a separate `getCollisionShape`
    ///   override).
    pub fn to_content_export(
        &self,
    ) -> (
        Option<crate::runtime::content::BlockShapes>,
        Option<crate::runtime::content::BlockShapes>,
    ) {
        let (Some(outline), Some(collision)) =
            (self.states.as_deref(), self.collision_states.as_deref())
        else {
            return (None, None);
        };
        if outline.len() != collision.len() || outline.is_empty() {
            return (None, None);
        }
        let collision_matches_outline = outline
            .iter()
            .zip(collision.iter())
            .all(|(o, c)| o.state == c.state && o.boxes == c.boxes);
        let shapes_out = if self.overridden {
            let Some(encoded) = encode_content_shapes(&self.properties, outline) else {
                return (None, None);
            };
            Some(encoded)
        } else {
            None
        };
        let collision_out = if collision_matches_outline {
            None
        } else {
            let Some(encoded) = encode_content_shapes(&self.properties, collision) else {
                return (None, None);
            };
            Some(encoded)
        };
        (shapes_out, collision_out)
    }

    /// Convert into Sunrise's content `BlockShapes` outline table when both outline and collision
    /// resolved cleanly and the block overrode `getShape`.
    pub fn to_content_shapes(&self) -> Option<crate::runtime::content::BlockShapes> {
        self.to_content_export().0
    }
}

fn is_valid_sunrise_box(bounds: &[f64; 6]) -> bool {
    bounds.iter().all(|v| v.is_finite())
        && bounds[0] <= bounds[3]
        && bounds[1] <= bounds[4]
        && bounds[2] <= bounds[5]
}

fn is_sunrise_state_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Exact coordinate axis of a `VoxelShape`: `CubePointRange(parts)` or `DoubleArrayList`.
#[derive(Clone, Debug, PartialEq)]
enum AxisCoords {
    Cube(usize),
    Array(Vec<f64>),
}

impl AxisCoords {
    fn len(&self) -> usize {
        match self {
            Self::Cube(parts) => parts + 1,
            Self::Array(values) => values.len(),
        }
    }

    fn size(&self) -> usize {
        self.len().saturating_sub(1)
    }

    fn get(&self, index: usize) -> f64 {
        match self {
            Self::Cube(parts) => (index as f64) / (*parts as f64),
            Self::Array(values) => values[index],
        }
    }

    fn exact_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        (0..self.len()).all(|i| self.get(i).to_bits() == other.get(i).to_bits())
    }

    fn to_vec(&self) -> Vec<f64> {
        match self {
            Self::Cube(parts) => (0..=*parts).map(|i| (i as f64) / (*parts as f64)).collect(),
            Self::Array(values) => values.clone(),
        }
    }
}

/// Exact discrete-grid representation of a Minecraft 26.3 `VoxelShape`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VoxelShapeData {
    xs: AxisCoords,
    ys: AxisCoords,
    zs: AxisCoords,
    bits: Vec<bool>,
    is_cube: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BooleanOpKind {
    False,
    NotOr,
    OnlySecond,
    NotFirst,
    OnlyFirst,
    NotSecond,
    NotSame,
    NotAnd,
    And,
    Same,
    Second,
    Causes,
    First,
    CausedBy,
    Or,
    True,
}

impl BooleanOpKind {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "FALSE" => Self::False,
            "NOT_OR" => Self::NotOr,
            "ONLY_SECOND" => Self::OnlySecond,
            "NOT_FIRST" => Self::NotFirst,
            "ONLY_FIRST" => Self::OnlyFirst,
            "NOT_SECOND" => Self::NotSecond,
            "NOT_SAME" => Self::NotSame,
            "NOT_AND" => Self::NotAnd,
            "AND" => Self::And,
            "SAME" => Self::Same,
            "SECOND" => Self::Second,
            "CAUSES" => Self::Causes,
            "FIRST" => Self::First,
            "CAUSED_BY" => Self::CausedBy,
            "OR" => Self::Or,
            "TRUE" => Self::True,
            _ => return None,
        })
    }

    pub(crate) fn apply(self, a: bool, b: bool) -> bool {
        match self {
            Self::False => false,
            Self::NotOr => !a && !b,
            Self::OnlySecond => b && !a,
            Self::NotFirst => !a,
            Self::OnlyFirst => a && !b,
            Self::NotSecond => !b,
            Self::NotSame => a != b,
            Self::NotAnd => !a || !b,
            Self::And => a && b,
            Self::Same => a == b,
            Self::Second => b,
            Self::Causes => !a || b,
            Self::First => a,
            Self::CausedBy => a || !b,
            Self::Or => a || b,
            Self::True => true,
        }
    }
}

enum IndexMerger {
    DiscreteCube {
        parts: usize,
        first_div: usize,
        second_div: usize,
    },
    NonOverlapping {
        lower: Vec<f64>,
        upper: Vec<f64>,
        swap: bool,
    },
    Identical {
        coords: AxisCoords,
    },
    Indirect {
        result: Vec<f64>,
        first_indices: Vec<isize>,
        second_indices: Vec<isize>,
    },
}

impl IndexMerger {
    fn is_discrete_cube(&self) -> bool {
        matches!(self, Self::DiscreteCube { .. })
    }

    fn size(&self) -> usize {
        match self {
            Self::DiscreteCube { parts, .. } => parts + 1,
            Self::NonOverlapping { lower, upper, .. } => lower.len() + upper.len(),
            Self::Identical { coords } => coords.len(),
            Self::Indirect { result, .. } => result.len(),
        }
    }

    fn to_axis_coords(&self) -> AxisCoords {
        match self {
            Self::DiscreteCube { parts, .. } => AxisCoords::Cube(*parts),
            Self::NonOverlapping { lower, upper, .. } => {
                let mut out = Vec::with_capacity(lower.len() + upper.len());
                out.extend_from_slice(lower);
                out.extend_from_slice(upper);
                AxisCoords::Array(out)
            }
            Self::Identical { coords } => coords.clone(),
            Self::Indirect { result, .. } => {
                if result.len() <= 1 {
                    AxisCoords::Array(vec![0.0])
                } else {
                    AxisCoords::Array(result.clone())
                }
            }
        }
    }

    fn for_merged_indexes(&self, mut f: impl FnMut(isize, isize, usize)) {
        match self {
            Self::DiscreteCube {
                parts,
                first_div,
                second_div,
            } => {
                for i in 0..*parts {
                    f((i / *second_div) as isize, (i / *first_div) as isize, i);
                }
            }
            Self::NonOverlapping { lower, upper, swap } => {
                let lower_len = lower.len();
                for i in 0..lower_len {
                    if *swap {
                        f(-1, i as isize, i);
                    } else {
                        f(i as isize, -1, i);
                    }
                }
                let upper_cells = upper.len().saturating_sub(1);
                for j in 0..upper_cells {
                    let li = (lower_len - 1) as isize;
                    if *swap {
                        f(j as isize, li, lower_len + j);
                    } else {
                        f(li, j as isize, lower_len + j);
                    }
                }
            }
            Self::Identical { coords } => {
                for i in 0..coords.size() {
                    f(i as isize, i as isize, i);
                }
            }
            Self::Indirect {
                result,
                first_indices,
                second_indices,
            } => {
                let cells = result.len().saturating_sub(1);
                for i in 0..cells {
                    f(first_indices[i], second_indices[i], i);
                }
            }
        }
    }
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn lcm(a: usize, b: usize) -> u64 {
    let g = gcd(a, b) as u64;
    (a as u64) * ((b as u64) / g)
}

fn create_index_merger(
    size: usize,
    first: &AxisCoords,
    second: &AxisCoords,
    include_first: bool,
    include_second: bool,
) -> IndexMerger {
    let n1 = first.size();
    let n2 = second.size();
    if let (AxisCoords::Cube(p1), AxisCoords::Cube(p2)) = (first, second) {
        let l = lcm(*p1, *p2);
        if (size as u64).saturating_mul(l) <= 256 {
            let g = gcd(*p1, *p2);
            return IndexMerger::DiscreteCube {
                parts: l as usize,
                first_div: p1 / g,
                second_div: p2 / g,
            };
        }
    }
    if first.get(n1) < second.get(0) - EPSILON {
        return IndexMerger::NonOverlapping {
            lower: first.to_vec(),
            upper: second.to_vec(),
            swap: false,
        };
    }
    if second.get(n2) < first.get(0) - EPSILON {
        return IndexMerger::NonOverlapping {
            lower: second.to_vec(),
            upper: first.to_vec(),
            swap: true,
        };
    }
    if n1 == n2 && first.exact_eq(second) {
        return IndexMerger::Identical {
            coords: first.clone(),
        };
    }
    indirect_merger(first, second, include_first, include_second)
}

/// Exact port of `net.minecraft.world.phys.shapes.IndirectMerger.<init>`.
fn indirect_merger(
    first: &AxisCoords,
    second: &AxisCoords,
    include_first: bool,
    include_second: bool,
) -> IndexMerger {
    let len1 = first.len();
    let len2 = second.len();
    let skip_first = !include_first;
    let skip_second = !include_second;
    let mut result = Vec::with_capacity(len1 + len2);
    let mut first_indices = Vec::with_capacity(len1 + len2);
    let mut second_indices = Vec::with_capacity(len1 + len2);
    let mut last_value = f64::NAN;
    let mut i = 0usize;
    let mut j = 0usize;

    loop {
        let done1 = i >= len1;
        let done2 = j >= len2;
        if done1 && done2 {
            break;
        }
        let take1 = !done1 && (done2 || first.get(i) < second.get(j) + EPSILON);
        if take1 {
            i += 1;
            if skip_first && (j == 0 || done2) {
                continue;
            }
        } else {
            j += 1;
            if skip_second && (i == 0 || done1) {
                continue;
            }
        }
        let i1 = i as isize - 1;
        let i2 = j as isize - 1;
        let val = if take1 {
            first.get(i1 as usize)
        } else {
            second.get(i2 as usize)
        };
        // Bytecode `dcmpl; ifge`: NaN pushes -1 so the first element always enters the push branch.
        if !matches!(
            last_value.partial_cmp(&(val - EPSILON)),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ) {
            first_indices.push(i1);
            second_indices.push(i2);
            result.push(val);
            last_value = val;
        } else if let Some(last) = first_indices.len().checked_sub(1) {
            first_indices[last] = i1;
            second_indices[last] = i2;
        }
    }
    if result.is_empty() {
        result.push(0.0);
        first_indices.push(0);
        second_indices.push(0);
    }
    IndexMerger::Indirect {
        result,
        first_indices,
        second_indices,
    }
}

/// Exact port of `Shapes.findBits(double min, double max)`.
fn find_bits(min: f64, max: f64) -> i32 {
    if min.is_nan() || max.is_nan() || min < -EPSILON || max > 1.0 + EPSILON {
        return -1;
    }
    for bits in 0..=3 {
        let parts = 1_i32 << bits;
        let a = min * f64::from(parts);
        let b = max * f64::from(parts);
        let a_ok = (a - (a + 0.5).floor()).abs() < EPSILON * f64::from(parts);
        let b_ok = (b - (b + 0.5).floor()).abs() < EPSILON * f64::from(parts);
        if a_ok && b_ok {
            return bits;
        }
    }
    -1
}

impl VoxelShapeData {
    pub(crate) fn empty() -> Self {
        Self {
            xs: AxisCoords::Array(vec![0.0]),
            ys: AxisCoords::Array(vec![0.0]),
            zs: AxisCoords::Array(vec![0.0]),
            bits: Vec::new(),
            is_cube: false,
        }
    }

    pub(crate) fn block() -> Self {
        Self {
            xs: AxisCoords::Cube(1),
            ys: AxisCoords::Cube(1),
            zs: AxisCoords::Cube(1),
            bits: vec![true],
            is_cube: true,
        }
    }

    fn x_size(&self) -> usize {
        self.xs.size()
    }
    fn y_size(&self) -> usize {
        self.ys.size()
    }
    fn z_size(&self) -> usize {
        self.zs.size()
    }

    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (x * self.y_size() + y) * self.z_size() + z
    }

    fn is_full(&self, x: usize, y: usize, z: usize) -> bool {
        self.bits[self.idx(x, y, z)]
    }

    fn is_full_wide(&self, x: isize, y: isize, z: isize) -> bool {
        if x < 0 || y < 0 || z < 0 {
            return false;
        }
        let (x, y, z) = (x as usize, y as usize, z as usize);
        x < self.x_size() && y < self.y_size() && z < self.z_size() && self.is_full(x, y, z)
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self.bits.iter().any(|&filled| filled)
    }

    /// Exact port of `Shapes.box(double, double, double, double, double, double)`.
    #[cfg(test)]
    pub(crate) fn box_checked(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Result<Self, NativeError> {
        // Java `dcmpl; ifgt`: strictly greater (false for NaN).
        if min_x > max_x || min_y > max_y || min_z > max_z {
            return Err(NativeError::UncaughtException {
                class: "java/lang/IllegalArgumentException".into(),
                message: Some(
                    "The min values need to be smaller or equals to the max values".into(),
                ),
                frame: None,
            });
        }
        Ok(Self::create(min_x, min_y, min_z, max_x, max_y, max_z))
    }

    /// Exact port of `Shapes.create(double, double, double, double, double, double)`.
    pub(crate) fn create(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Self {
        // Java `dcmpg; iflt`: strictly less than 1e-7 (false for NaN).
        if max_x - min_x < EPSILON || max_y - min_y < EPSILON || max_z - min_z < EPSILON {
            return Self::empty();
        }
        let bx = find_bits(min_x, max_x);
        let by = find_bits(min_y, max_y);
        let bz = find_bits(min_z, max_z);
        if bx < 0 || by < 0 || bz < 0 {
            return Self {
                xs: AxisCoords::Array(vec![min_x, max_x]),
                ys: AxisCoords::Array(vec![min_y, max_y]),
                zs: AxisCoords::Array(vec![min_z, max_z]),
                bits: vec![true],
                is_cube: false,
            };
        }
        if bx == 0 && by == 0 && bz == 0 {
            return Self::block();
        }
        let nx = 1usize << bx;
        let ny = 1usize << by;
        let nz = 1usize << bz;
        let x0 = ((min_x * nx as f64) + 0.5).floor() as usize;
        let y0 = ((min_y * ny as f64) + 0.5).floor() as usize;
        let z0 = ((min_z * nz as f64) + 0.5).floor() as usize;
        let x1 = ((max_x * nx as f64) + 0.5).floor() as usize;
        let y1 = ((max_y * ny as f64) + 0.5).floor() as usize;
        let z1 = ((max_z * nz as f64) + 0.5).floor() as usize;
        let mut bits = vec![false; nx * ny * nz];
        for x in x0..x1.min(nx) {
            for y in y0..y1.min(ny) {
                for z in z0..z1.min(nz) {
                    bits[(x * ny + y) * nz + z] = true;
                }
            }
        }
        Self {
            xs: AxisCoords::Cube(nx),
            ys: AxisCoords::Cube(ny),
            zs: AxisCoords::Cube(nz),
            bits,
            is_cube: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn block_box(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Result<Self, NativeError> {
        Self::box_checked(
            min_x / 16.0,
            min_y / 16.0,
            min_z / 16.0,
            max_x / 16.0,
            max_y / 16.0,
            max_z / 16.0,
        )
    }

    /// Exact port of `Shapes.joinUnoptimized`. Returns `Ok(None)` if the merged grid exceeds the
    /// cell safety cap (caller counts the call).
    pub(crate) fn join_unoptimized(
        &self,
        other: &Self,
        op: BooleanOpKind,
        same_object: bool,
    ) -> Result<Option<Self>, NativeError> {
        if op.apply(false, false) {
            return Err(NativeError::UncaughtException {
                class: "java/lang/IllegalArgumentException".into(),
                message: None,
                frame: None,
            });
        }
        if same_object {
            return Ok(Some(if op.apply(true, true) {
                self.clone()
            } else {
                Self::empty()
            }));
        }
        let only_first = op.apply(true, false);
        let only_second = op.apply(false, true);
        if self.is_empty() {
            return Ok(Some(if only_second {
                other.clone()
            } else {
                Self::empty()
            }));
        }
        if other.is_empty() {
            return Ok(Some(if only_first {
                self.clone()
            } else {
                Self::empty()
            }));
        }
        let mx = create_index_merger(1, &self.xs, &other.xs, only_first, only_second);
        let nx = mx.size().saturating_sub(1);
        let my = create_index_merger(nx, &self.ys, &other.ys, only_first, only_second);
        let ny = my.size().saturating_sub(1);
        let mz = create_index_merger(
            nx.saturating_mul(ny),
            &self.zs,
            &other.zs,
            only_first,
            only_second,
        );
        let nz = mz.size().saturating_sub(1);
        let Some(total) = nx.checked_mul(ny).and_then(|v| v.checked_mul(nz)) else {
            return Ok(None);
        };
        if total > MAX_VOXEL_CELLS {
            return Ok(None);
        }
        let mut bits = vec![false; total];
        mx.for_merged_indexes(|ix1, ix2, ox| {
            my.for_merged_indexes(|iy1, iy2, oy| {
                mz.for_merged_indexes(|iz1, iz2, oz| {
                    if op.apply(
                        self.is_full_wide(ix1, iy1, iz1),
                        other.is_full_wide(ix2, iy2, iz2),
                    ) {
                        bits[(ox * ny + oy) * nz + oz] = true;
                    }
                });
            });
        });
        let is_cube = mx.is_discrete_cube() && my.is_discrete_cube() && mz.is_discrete_cube();
        Ok(Some(Self {
            xs: mx.to_axis_coords(),
            ys: my.to_axis_coords(),
            zs: mz.to_axis_coords(),
            bits,
            is_cube,
        }))
    }

    /// Exact port of `VoxelShape.optimize()`.
    #[cfg(test)]
    pub(crate) fn optimize(&self) -> Option<Self> {
        let mut acc = Self::empty();
        for [x0, y0, z0, x1, y1, z1] in self.to_boxes() {
            let part = Self::create(x0, y0, z0, x1, y1, z1);
            acc = acc
                .join_unoptimized(&part, BooleanOpKind::Or, false)
                .ok()??;
        }
        Some(acc)
    }

    /// Exact port of `Shapes.join(first, second, op)`.
    #[cfg(test)]
    pub(crate) fn join(
        &self,
        other: &Self,
        op: BooleanOpKind,
        same_object: bool,
    ) -> Result<Option<Self>, NativeError> {
        let Some(unopt) = self.join_unoptimized(other, op, same_object)? else {
            return Ok(None);
        };
        Ok(unopt.optimize())
    }

    /// Exact port of `BitSetDiscreteVoxelShape.forAllBoxes(shape, consumer, true)` + `AABB` min/max
    /// ordering (`AABB.<init>(DDDDDD)` uses `Math.min`/`Math.max`).
    pub(crate) fn to_boxes(&self) -> Vec<[f64; 6]> {
        let nx = self.x_size();
        let ny = self.y_size();
        let nz = self.z_size();
        if nx == 0 || ny == 0 || nz == 0 {
            return Vec::new();
        }
        let mut copy = self.bits.clone();
        let get = |bits: &[bool], x: usize, y: usize, z: usize| -> bool {
            x < nx && y < ny && z < nz && bits[(x * ny + y) * nz + z]
        };
        let is_z_strip_full = |bits: &[bool], z0: usize, z1: usize, x: usize, y: usize| -> bool {
            x < nx && y < ny && z1 <= nz && (z0..z1).all(|z| bits[(x * ny + y) * nz + z])
        };
        let clear_z_strip = |bits: &mut [bool], z0: usize, z1: usize, x: usize, y: usize| {
            for z in z0..z1 {
                bits[(x * ny + y) * nz + z] = false;
            }
        };
        let is_xz_rect_full =
            |bits: &[bool], x0: usize, x1: usize, z0: usize, z1: usize, y: usize| -> bool {
                y < ny
                    && x1 <= nx
                    && z1 <= nz
                    && (x0..x1).all(|x| (z0..z1).all(|z| bits[(x * ny + y) * nz + z]))
            };

        let mut out = Vec::new();
        for y in 0..ny {
            for x in 0..nx {
                let mut z_start: Option<usize> = None;
                for z in 0..=nz {
                    if get(&copy, x, y, z) {
                        if z_start.is_none() {
                            z_start = Some(z);
                        }
                    } else if let Some(z0) = z_start {
                        let mut x_end = x;
                        let mut y_end = y;
                        clear_z_strip(&mut copy, z0, z, x, y);
                        while is_z_strip_full(&copy, z0, z, x_end + 1, y) {
                            clear_z_strip(&mut copy, z0, z, x_end + 1, y);
                            x_end += 1;
                        }
                        while is_xz_rect_full(&copy, x, x_end + 1, z0, z, y_end + 1) {
                            for cx in x..=x_end {
                                clear_z_strip(&mut copy, z0, z, cx, y_end + 1);
                            }
                            y_end += 1;
                        }
                        let ax0 = self.xs.get(x);
                        let ay0 = self.ys.get(y);
                        let az0 = self.zs.get(z0);
                        let ax1 = self.xs.get(x_end + 1);
                        let ay1 = self.ys.get(y_end + 1);
                        let az1 = self.zs.get(z);
                        out.push(aabb_normalize([ax0, ay0, az0, ax1, ay1, az1]));
                        z_start = None;
                    }
                }
            }
        }
        out
    }

    pub(crate) fn min_axis(&self, axis: usize) -> f64 {
        let (nx, ny, nz) = (self.x_size(), self.y_size(), self.z_size());
        let first = match axis {
            0 => (0..nx).find(|&x| (0..ny).any(|y| (0..nz).any(|z| self.is_full(x, y, z)))),
            1 => (0..ny).find(|&y| (0..nx).any(|x| (0..nz).any(|z| self.is_full(x, y, z)))),
            2 => (0..nz).find(|&z| (0..nx).any(|x| (0..ny).any(|y| self.is_full(x, y, z)))),
            _ => None,
        };
        match (axis, first) {
            (0, Some(i)) => self.xs.get(i),
            (1, Some(i)) => self.ys.get(i),
            (2, Some(i)) => self.zs.get(i),
            _ => f64::INFINITY,
        }
    }

    pub(crate) fn max_axis(&self, axis: usize) -> f64 {
        let (nx, ny, nz) = (self.x_size(), self.y_size(), self.z_size());
        let last = match axis {
            0 => (0..nx)
                .rev()
                .find(|&x| (0..ny).any(|y| (0..nz).any(|z| self.is_full(x, y, z))))
                .map(|x| x + 1),
            1 => (0..ny)
                .rev()
                .find(|&y| (0..nx).any(|x| (0..nz).any(|z| self.is_full(x, y, z))))
                .map(|y| y + 1),
            2 => (0..nz)
                .rev()
                .find(|&z| (0..nx).any(|x| (0..ny).any(|y| self.is_full(x, y, z))))
                .map(|z| z + 1),
            _ => None,
        };
        match (axis, last) {
            (0, Some(i)) => self.xs.get(i),
            (1, Some(i)) => self.ys.get(i),
            (2, Some(i)) => self.zs.get(i),
            _ => f64::NEG_INFINITY,
        }
    }

    pub(crate) fn bounds(&self) -> Result<[f64; 6], NativeError> {
        if self.is_empty() {
            return Err(NativeError::UncaughtException {
                class: "java/lang/UnsupportedOperationException".into(),
                message: Some("No bounds for empty shape.".into()),
                frame: None,
            });
        }
        Ok(aabb_normalize([
            self.min_axis(0),
            self.min_axis(1),
            self.min_axis(2),
            self.max_axis(0),
            self.max_axis(1),
            self.max_axis(2),
        ]))
    }

    pub(crate) fn move_by(&self, dx: f64, dy: f64, dz: f64) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        Self {
            xs: AxisCoords::Array(self.xs.to_vec().into_iter().map(|v| v + dx).collect()),
            ys: AxisCoords::Array(self.ys.to_vec().into_iter().map(|v| v + dy).collect()),
            zs: AxisCoords::Array(self.zs.to_vec().into_iter().map(|v| v + dz).collect()),
            bits: self.bits.clone(),
            is_cube: false,
        }
    }

    pub(crate) fn to_java_string(&self) -> String {
        if self.is_empty() {
            return "EMPTY".to_owned();
        }
        let b = self.bounds().expect("non-empty shape has bounds");
        format!("VoxelShape[{}]", aabb_to_string(b))
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        self.xs.len() * 8 + self.ys.len() * 8 + self.zs.len() * 8 + self.bits.len()
    }
}

fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else if a < b {
        a
    } else {
        b
    }
}

fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_positive() || b.is_sign_positive() {
            0.0
        } else {
            -0.0
        }
    } else if a > b {
        a
    } else {
        b
    }
}

fn aabb_normalize(b: [f64; 6]) -> [f64; 6] {
    [
        java_min(b[0], b[3]),
        java_min(b[1], b[4]),
        java_min(b[2], b[5]),
        java_max(b[0], b[3]),
        java_max(b[1], b[4]),
        java_max(b[2], b[5]),
    ]
}

fn aabb_to_string(b: [f64; 6]) -> String {
    format!(
        "AABB[{}, {}, {}] -> [{}, {}, {}]",
        crate::runtime::format_java_float(b[0]),
        crate::runtime::format_java_float(b[1]),
        crate::runtime::format_java_float(b[2]),
        crate::runtime::format_java_float(b[3]),
        crate::runtime::format_java_float(b[4]),
        crate::runtime::format_java_float(b[5]),
    )
}

pub(crate) fn aabb_value(bounds: [f64; 6]) -> Value {
    let b = aabb_normalize(bounds);
    let key = format!(
        "{:016x},{:016x},{:016x},{:016x},{:016x},{:016x}",
        b[0].to_bits(),
        b[1].to_bits(),
        b[2].to_bits(),
        b[3].to_bits(),
        b[4].to_bits(),
        b[5].to_bits(),
    );
    Value::Host(HostRef::new(HostKind::Aabb, key))
}

pub(crate) fn aabb_bounds(key: &str) -> Option<[f64; 6]> {
    let mut parts = key.split(',');
    let mut out = [0.0; 6];
    for slot in &mut out {
        let bits = u64::from_str_radix(parts.next()?, 16).ok()?;
        *slot = f64::from_bits(bits);
    }
    parts.next().is_none().then_some(out)
}

fn java_double_compare(a: f64, b: f64) -> std::cmp::Ordering {
    if a < b {
        std::cmp::Ordering::Less
    } else if a > b {
        std::cmp::Ordering::Greater
    } else {
        let ab = if a.is_nan() {
            0x7ff8_0000_0000_0000_u64
        } else {
            a.to_bits()
        };
        let bb = if b.is_nan() {
            0x7ff8_0000_0000_0000_u64
        } else {
            b.to_bits()
        };
        (ab as i64).cmp(&(bb as i64))
    }
}

fn java_double_long_bits(v: f64) -> u64 {
    if v.is_nan() {
        0x7ff8_0000_0000_0000_u64
    } else {
        v.to_bits()
    }
}

fn aabb_hash_code(b: [f64; 6]) -> i32 {
    let fold = |v: f64| -> i32 {
        let bits = java_double_long_bits(v);
        (bits ^ (bits >> 32)) as i32
    };
    let mut h = fold(b[0]);
    for &v in &b[1..] {
        h = h.wrapping_mul(31).wrapping_add(fold(v));
    }
    h
}

pub(crate) fn voxel_shape_class(key: &str) -> &'static str {
    if key == BLOCK_KEY || key.starts_with("cube/") {
        CUBE_VOXEL_SHAPE
    } else {
        ARRAY_VOXEL_SHAPE
    }
}

fn voxel_shape_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    let exact = voxel_shape_class(handle.key());
    if wanted == exact || wanted == VOXEL_SHAPE || wanted == "java/lang/Object" {
        return Some(true);
    }
    if matches!(
        wanted,
        CUBE_VOXEL_SHAPE | ARRAY_VOXEL_SHAPE | AABB | "java/lang/String"
    ) {
        return Some(false);
    }
    hierarchy.is_instance(exact, wanted)
}

pub(crate) const VOXEL_SHAPE_METHODS: Methods = Methods {
    to_string: |handle, nested| {
        Ok(match handle.key() {
            EMPTY_KEY => Some("EMPTY".to_owned()),
            BLOCK_KEY => Some("VoxelShape[AABB[0.0, 0.0, 0.0] -> [1.0, 1.0, 1.0]]".to_owned()),
            INFINITY_KEY => Some(
                "VoxelShape[AABB[-Infinity, -Infinity, -Infinity] -> [Infinity, Infinity, Infinity]]"
                    .to_owned(),
            ),
            key => nested.voxel_shape_to_string(key),
        })
    },
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(Some(matches!(
            other,
            Value::Host(other) if other.kind() == HostKind::VoxelShape && handle == other
        )))
    },
    instance_of: voxel_shape_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

fn aabb_instance_of(_: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if matches!(wanted, AABB | "java/lang/Object") {
        return Some(true);
    }
    if matches!(wanted, VOXEL_SHAPE | "java/lang/String") {
        return Some(false);
    }
    hierarchy.is_instance(AABB, wanted)
}

pub(crate) const AABB_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(aabb_bounds(handle.key()).map(aabb_to_string)),
    hash_code: |handle, _| Ok(aabb_bounds(handle.key()).map(aabb_hash_code)),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::Aabb => {
                let (Some(a), Some(b)) = (aabb_bounds(handle.key()), aabb_bounds(other.key()))
                else {
                    return Ok(None);
                };
                Some((0..6).all(|i| java_double_compare(a[i], b[i]) == std::cmp::Ordering::Equal))
            }
            _ => Some(false),
        })
    },
    instance_of: aabb_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

pub(crate) fn boolean_op_declared_types(_key: &str) -> &'static [&'static str] {
    &[BOOLEAN_OP]
}

fn boolean_op_instance_of(
    _: &HostRef,
    wanted: &str,
    _hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if matches!(wanted, BOOLEAN_OP | "java/lang/Object") {
        return Some(true);
    }
    if matches!(wanted, VOXEL_SHAPE | AABB | "java/lang/String") {
        return Some(false);
    }
    None
}

pub(crate) const BOOLEAN_OP_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(Some(matches!(
            other,
            Value::Host(other) if other.kind() == HostKind::BooleanOp && handle == other
        )))
    },
    instance_of: boolean_op_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

fn collision_context_instance_of(
    _handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if matches!(
        wanted,
        EMPTY_COLLISION_CONTEXT | ENTITY_COLLISION_CONTEXT | COLLISION_CONTEXT | "java/lang/Object"
    ) {
        return Some(true);
    }
    if matches!(wanted, VOXEL_SHAPE | AABB | "java/lang/String") {
        return Some(false);
    }
    hierarchy.is_instance(EMPTY_COLLISION_CONTEXT, wanted)
}

pub(crate) const COLLISION_CONTEXT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        if matches!(other, Value::Host(other) if other.kind() == HostKind::ShapeProbe) {
            return Ok(None);
        }
        Ok(Some(matches!(
            other,
            Value::Host(other) if other.kind() == HostKind::CollisionContext && handle == other
        )))
    },
    instance_of: collision_context_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

/// Opaque extraction probes (`BlockGetter`, `BlockPos`, `CollisionContext`): every `Object`
/// observation (`toString`, `hashCode`, `equals`, `instanceof`, `checkcast`, `compareTo`) returns
/// unknown so the runtime counts it on the host and refuses static shape export.
pub(crate) const SHAPE_PROBE_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |_, _, _| Ok(None),
    instance_of: |_, _, _| None,
    crosses_jars: |_, _| true,
    compare_to: None,
};

pub(crate) fn empty_shape() -> Value {
    Value::Host(HostRef::new(HostKind::VoxelShape, EMPTY_KEY))
}

pub(crate) fn block_shape() -> Value {
    Value::Host(HostRef::new(HostKind::VoxelShape, BLOCK_KEY))
}

pub(crate) fn infinity_shape() -> Value {
    Value::Host(HostRef::new(HostKind::VoxelShape, INFINITY_KEY))
}

pub(crate) fn empty_collision_context() -> Value {
    Value::Host(HostRef::new(HostKind::CollisionContext, "empty"))
}

pub(crate) fn parse_shape_probe_key(key: &str) -> Option<(u64, &str)> {
    let (id, role) = key.split_once(':')?;
    let id = id.parse::<u64>().ok()?;
    matches!(role, "block-getter" | "block-pos" | "collision-context").then_some((id, role))
}

fn probe_collision_context(extraction_id: u64) -> Value {
    Value::Host(HostRef::new(
        HostKind::ShapeProbe,
        format!("{extraction_id}:collision-context"),
    ))
}

fn probe_block_getter(extraction_id: u64) -> Value {
    Value::Host(HostRef::new(
        HostKind::ShapeProbe,
        format!("{extraction_id}:block-getter"),
    ))
}

fn probe_block_pos(extraction_id: u64) -> Value {
    Value::Host(HostRef::new(
        HostKind::ShapeProbe,
        format!("{extraction_id}:block-pos"),
    ))
}

/// Allocate a fresh `VoxelShape` handle without collapsing to `Shapes.empty()` / `Shapes.block()`.
/// Matches MC 26.3 `new CubeVoxelShape(...)` / `new ArrayVoxelShape(...)` allocations in
/// `Shapes.joinUnoptimized`, `VoxelShape.move`, and non-unit `Shapes.create`.
pub(crate) fn store_fresh_shape(host: &mut StdHost, shape: VoxelShapeData) -> Value {
    let prefix = if shape.is_cube { "cube" } else { "array" };
    let key = format!("{prefix}/{}", host.next_voxel_shape);
    host.next_voxel_shape = host.next_voxel_shape.saturating_add(1);
    host.voxel_shapes.insert(key.clone(), shape);
    Value::Host(HostRef::new(HostKind::VoxelShape, key))
}

/// Exact MC 26.3 `Shapes.create(minX, minY, minZ, maxX, maxY, maxZ)` identity rules:
/// * returns canonical `Shapes.empty()` when any span `< 1.0e-7`;
/// * returns canonical `Shapes.block()` when `findBits` returns `(0, 0, 0)`;
/// * otherwise allocates a fresh `CubeVoxelShape` or `ArrayVoxelShape`.
pub(crate) fn create_shape_value(
    host: &mut StdHost,
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
) -> Value {
    if max_x - min_x < EPSILON || max_y - min_y < EPSILON || max_z - min_z < EPSILON {
        return empty_shape();
    }
    let bx = find_bits(min_x, max_x);
    let by = find_bits(min_y, max_y);
    let bz = find_bits(min_z, max_z);
    if bx == 0 && by == 0 && bz == 0 {
        return block_shape();
    }
    store_fresh_shape(
        host,
        VoxelShapeData::create(min_x, min_y, min_z, max_x, max_y, max_z),
    )
}

fn box_checked_value(
    host: &mut StdHost,
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
) -> Result<Value, NativeError> {
    if min_x > max_x || min_y > max_y || min_z > max_z {
        return Err(NativeError::UncaughtException {
            class: "java/lang/IllegalArgumentException".into(),
            message: Some("The min values need to be smaller or equals to the max values".into()),
            frame: None,
        });
    }
    Ok(create_shape_value(
        host, min_x, min_y, min_z, max_x, max_y, max_z,
    ))
}

fn block_box_value(
    host: &mut StdHost,
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
) -> Result<Value, NativeError> {
    box_checked_value(
        host,
        min_x / 16.0,
        min_y / 16.0,
        min_z / 16.0,
        max_x / 16.0,
        max_y / 16.0,
        max_z / 16.0,
    )
}

/// Exact MC 26.3 `Shapes.joinUnoptimized(first, second, op)` identity semantics:
/// * `op(false, false)` throws `new IllegalArgumentException()` (`message: None`);
/// * `first == second` returns `op(true, true) ? first : Shapes.empty()`;
/// * `first.isEmpty()` returns `op(false, true) ? second : Shapes.empty()`;
/// * `second.isEmpty()` returns `op(true, false) ? first : Shapes.empty()`;
/// * otherwise allocates a NEW `CubeVoxelShape` or `ArrayVoxelShape` — never collapsing to
///   `Shapes.block()` or `Shapes.empty()`.
fn join_unoptimized_values(
    host: &mut StdHost,
    first: &Value,
    second: &Value,
    op: BooleanOpKind,
) -> Result<Option<Value>, NativeError> {
    if op.apply(false, false) {
        return Err(NativeError::UncaughtException {
            class: "java/lang/IllegalArgumentException".into(),
            message: None,
            frame: None,
        });
    }
    if *first == Value::Ref(None) || *second == Value::Ref(None) {
        return Err(npe());
    }
    let (Some(sa), Some(sb)) = (shape_data(host, first), shape_data(host, second)) else {
        return Ok(None);
    };
    if first == second {
        return Ok(Some(if op.apply(true, true) {
            first.clone()
        } else {
            empty_shape()
        }));
    }
    let only_first = op.apply(true, false);
    let only_second = op.apply(false, true);
    if sa.is_empty() {
        return Ok(Some(if only_second {
            second.clone()
        } else {
            empty_shape()
        }));
    }
    if sb.is_empty() {
        return Ok(Some(if only_first {
            first.clone()
        } else {
            empty_shape()
        }));
    }
    let Some(joined) = sa.join_unoptimized(&sb, op, false)? else {
        return Ok(None);
    };
    Ok(Some(store_fresh_shape(host, joined)))
}

/// Exact MC 26.3 `VoxelShape.optimize()` identity semantics: starts at `Shapes.empty()` and folds
/// `forAllBoxes` via `Shapes.joinUnoptimized(acc, Shapes.box(...), BooleanOp.OR)`.
fn optimize_value(host: &mut StdHost, val: &Value) -> Result<Option<Value>, NativeError> {
    let Some(shape) = shape_data(host, val) else {
        return Ok(None);
    };
    let mut acc = empty_shape();
    for [x0, y0, z0, x1, y1, z1] in shape.to_boxes() {
        let part = box_checked_value(host, x0, y0, z0, x1, y1, z1)?;
        let Some(next) = join_unoptimized_values(host, &acc, &part, BooleanOpKind::Or)? else {
            return Ok(None);
        };
        acc = next;
    }
    Ok(Some(acc))
}

fn join_values(
    host: &mut StdHost,
    first: &Value,
    second: &Value,
    op: BooleanOpKind,
) -> Result<Option<Value>, NativeError> {
    let Some(unopt) = join_unoptimized_values(host, first, second, op)? else {
        return Ok(None);
    };
    optimize_value(host, &unopt)
}

pub(crate) fn shape_data(host: &StdHost, value: &Value) -> Option<VoxelShapeData> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::VoxelShape {
        return None;
    }
    match handle.key() {
        EMPTY_KEY => Some(VoxelShapeData::empty()),
        BLOCK_KEY => Some(VoxelShapeData::block()),
        INFINITY_KEY => Some(VoxelShapeData::create(
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
        )),
        key => host.voxel_shapes.get(key).cloned(),
    }
}

pub(crate) fn static_field(
    _host: &mut StdHost,
    r: &MethodRef,
) -> Option<Result<Value, NativeError>> {
    if r.owner == BOOLEAN_OP
        && r.descriptor == format!("L{BOOLEAN_OP};")
        && BooleanOpKind::from_name(&r.name).is_some()
    {
        return Some(Ok(Value::Host(HostRef::new(
            HostKind::BooleanOp,
            r.name.clone(),
        ))));
    }
    if r.owner == SHAPES && r.name == "INFINITY" && r.descriptor == format!("L{VOXEL_SHAPE};") {
        return Some(Ok(infinity_shape()));
    }
    if r.owner == SHAPES && r.name == "EPSILON" && r.descriptor == "D" {
        return Some(Ok(Value::Float(EPSILON)));
    }
    None
}

fn npe() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn axis_index(value: &Value) -> Option<usize> {
    let marker = expect_ref(value)?;
    match marker.strip_prefix("enum/net/minecraft/core/Direction$Axis/")? {
        "X" => Some(0),
        "Y" => Some(1),
        "Z" => Some(2),
        _ => None,
    }
}

fn boolean_op_of(value: &Value) -> Option<BooleanOpKind> {
    let Value::Host(handle) = value else {
        return None;
    };
    (handle.kind() == HostKind::BooleanOp)
        .then(|| BooleanOpKind::from_name(handle.key()))
        .flatten()
}

pub(crate) fn or_array_call(
    host: &mut StdHost,
    r: &MethodRef,
    first: &Value,
    others: &[Value],
) -> Result<Value, NativeError> {
    if *first == Value::Ref(None) {
        return Err(npe());
    }
    if shape_data(host, first).is_none() {
        return host.unknown_host_call(r);
    }
    let mut acc_val = first.clone();
    for next in others {
        if *next == Value::Ref(None) {
            return Err(npe());
        }
        match join_values(host, &acc_val, next, BooleanOpKind::Or)? {
            Some(joined) => acc_val = joined,
            None => return host.unknown_host_call(r),
        }
    }
    Ok(acc_val)
}

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == COLLISION_CONTEXT
        && r.name == "empty"
        && r.descriptor == format!("()L{COLLISION_CONTEXT};")
    {
        return Some(Ok(empty_collision_context()));
    }
    if r.owner == BLOCK_CLASS {
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("box", "(DDDDDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                let coords = match six_doubles(r, args) {
                    Ok(c) => c,
                    Err(e) => return Some(Err(e)),
                };
                return Some(block_box_value(
                    host, coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
                ));
            }
            ("cube", "(D)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                let s = match double_arg(r, args, 0) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                let half = s / 2.0;
                return Some(block_box_value(
                    host,
                    8.0 - half,
                    8.0 - half,
                    8.0 - half,
                    8.0 + half,
                    8.0 + half,
                    8.0 + half,
                ));
            }
            ("cube", "(DDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                let (x, y, z) = match (
                    double_arg(r, args, 0),
                    double_arg(r, args, 1),
                    double_arg(r, args, 2),
                ) {
                    (Ok(x), Ok(y), Ok(z)) => (x, y, z),
                    (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Some(Err(e)),
                };
                let hy = y / 2.0;
                let hx = x / 2.0;
                let hz = z / 2.0;
                return Some(block_box_value(
                    host,
                    8.0 - hx,
                    8.0 - hy,
                    8.0 - hz,
                    8.0 + hx,
                    8.0 + hy,
                    8.0 + hz,
                ));
            }
            ("column", "(DDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                let (size, min_y, max_y) = match (
                    double_arg(r, args, 0),
                    double_arg(r, args, 1),
                    double_arg(r, args, 2),
                ) {
                    (Ok(a), Ok(b), Ok(c)) => (a, b, c),
                    (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Some(Err(e)),
                };
                let half = size / 2.0;
                return Some(block_box_value(
                    host,
                    8.0 - half,
                    min_y,
                    8.0 - half,
                    8.0 + half,
                    max_y,
                    8.0 + half,
                ));
            }
            ("column", "(DDDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                let (xs, zs, min_y, max_y) = match (
                    double_arg(r, args, 0),
                    double_arg(r, args, 1),
                    double_arg(r, args, 2),
                    double_arg(r, args, 3),
                ) {
                    (Ok(a), Ok(b), Ok(c), Ok(d)) => (a, b, c, d),
                    (Err(e), _, _, _)
                    | (_, Err(e), _, _)
                    | (_, _, Err(e), _)
                    | (_, _, _, Err(e)) => return Some(Err(e)),
                };
                let hx = xs / 2.0;
                let hz = zs / 2.0;
                return Some(block_box_value(
                    host,
                    8.0 - hx,
                    min_y,
                    8.0 - hz,
                    8.0 + hx,
                    max_y,
                    8.0 + hz,
                ));
            }
            _ => return None,
        }
    }
    if r.owner != SHAPES {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("empty", "()Lnet/minecraft/world/phys/shapes/VoxelShape;") => Some(Ok(empty_shape())),
        ("block", "()Lnet/minecraft/world/phys/shapes/VoxelShape;") => Some(Ok(block_shape())),
        ("box", "(DDDDDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
            let coords = match six_doubles(r, args) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            Some(box_checked_value(
                host, coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
            ))
        }
        ("create", "(DDDDDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
            let coords = match six_doubles(r, args) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            Some(Ok(create_shape_value(
                host, coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
            )))
        }
        (
            "create",
            "(Lnet/minecraft/world/phys/AABB;)Lnet/minecraft/world/phys/shapes/VoxelShape;",
        ) => {
            let Some(arg) = args.first() else {
                return Some(Err(npe()));
            };
            if *arg == Value::Ref(None) {
                return Some(Err(npe()));
            }
            let Value::Host(handle) = arg else {
                return None;
            };
            if handle.kind() != HostKind::Aabb {
                return None;
            }
            let b = aabb_bounds(handle.key())?;
            Some(Ok(create_shape_value(
                host, b[0], b[1], b[2], b[3], b[4], b[5],
            )))
        }
        (
            "or",
            "(Lnet/minecraft/world/phys/shapes/VoxelShape;Lnet/minecraft/world/phys/shapes/VoxelShape;)Lnet/minecraft/world/phys/shapes/VoxelShape;",
        ) => {
            let (Some(a), Some(b)) = (args.first(), args.get(1)) else {
                return Some(Err(npe()));
            };
            if *a == Value::Ref(None) || *b == Value::Ref(None) {
                return Some(Err(npe()));
            }
            if shape_data(host, a).is_none() || shape_data(host, b).is_none() {
                return None;
            }
            Some(match join_values(host, a, b, BooleanOpKind::Or) {
                Ok(Some(joined)) => Ok(joined),
                Ok(None) => host.unknown_host_call(r),
                Err(err) => Err(err),
            })
        }
        (
            "join" | "joinUnoptimized",
            "(Lnet/minecraft/world/phys/shapes/VoxelShape;Lnet/minecraft/world/phys/shapes/VoxelShape;Lnet/minecraft/world/phys/shapes/BooleanOp;)Lnet/minecraft/world/phys/shapes/VoxelShape;",
        ) => {
            let (Some(a), Some(b), Some(op_val)) = (args.first(), args.get(1), args.get(2)) else {
                return Some(Err(npe()));
            };
            if *op_val == Value::Ref(None) {
                return Some(Err(npe()));
            }
            let op = boolean_op_of(op_val)?;
            if op.apply(false, false) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalArgumentException".into(),
                    message: None,
                    frame: None,
                }));
            }
            if *a == Value::Ref(None) || *b == Value::Ref(None) {
                return Some(Err(npe()));
            }
            if shape_data(host, a).is_none() || shape_data(host, b).is_none() {
                return None;
            }
            let res = if r.name == "join" {
                join_values(host, a, b, op)
            } else {
                join_unoptimized_values(host, a, b, op)
            };
            Some(match res {
                Ok(Some(joined)) => Ok(joined),
                Ok(None) => host.unknown_host_call(r),
                Err(err) => Err(err),
            })
        }
        _ => None,
    }
}

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Value::Host(handle) = receiver else {
        return None;
    };
    match handle.kind() {
        HostKind::VoxelShape => {
            let shape = shape_data(host, receiver)?;
            match (r.name.as_str(), r.descriptor.as_str()) {
                ("isEmpty", "()Z") => Some(Ok(Value::Int(i32::from(shape.is_empty())))),
                ("bounds", "()Lnet/minecraft/world/phys/AABB;") => {
                    Some(shape.bounds().map(aabb_value))
                }
                ("toAabbs", "()Ljava/util/List;") => {
                    let items: Vec<Value> = shape.to_boxes().into_iter().map(aabb_value).collect();
                    Some(Ok(host.new_collection("java/util/ArrayList", items)))
                }
                ("optimize", "()Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                    Some(match optimize_value(host, receiver) {
                        Ok(Some(opt)) => Ok(opt),
                        Ok(None) => host.unknown_host_call(r),
                        Err(err) => Err(err),
                    })
                }
                ("move", "(DDD)Lnet/minecraft/world/phys/shapes/VoxelShape;") => {
                    let (dx, dy, dz) = match (
                        double_arg(r, args, 0),
                        double_arg(r, args, 1),
                        double_arg(r, args, 2),
                    ) {
                        (Ok(x), Ok(y), Ok(z)) => (x, y, z),
                        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Some(Err(e)),
                    };
                    if shape.is_empty() {
                        Some(Ok(empty_shape()))
                    } else {
                        Some(Ok(store_fresh_shape(host, shape.move_by(dx, dy, dz))))
                    }
                }
                ("min", "(Lnet/minecraft/core/Direction$Axis;)D") => {
                    let arg = args.first()?;
                    if *arg == Value::Ref(None) {
                        return Some(Err(npe()));
                    }
                    let axis = axis_index(arg)?;
                    Some(Ok(Value::Float(shape.min_axis(axis))))
                }
                ("max", "(Lnet/minecraft/core/Direction$Axis;)D") => {
                    let arg = args.first()?;
                    if *arg == Value::Ref(None) {
                        return Some(Err(npe()));
                    }
                    let axis = axis_index(arg)?;
                    Some(Ok(Value::Float(shape.max_axis(axis))))
                }
                ("toString", "()Ljava/lang/String;") => {
                    Some(Ok(Value::Ref(Some(shape.to_java_string()))))
                }
                ("equals", "(Ljava/lang/Object;)Z") => {
                    let other = args.first().unwrap_or(&Value::Ref(None));
                    Some(Ok(Value::Int(i32::from(
                        matches!(other, Value::Host(o) if o.kind() == HostKind::VoxelShape && o == handle),
                    ))))
                }
                _ => None,
            }
        }
        HostKind::Aabb => {
            let b = aabb_bounds(handle.key())?;
            match (r.name.as_str(), r.descriptor.as_str()) {
                ("getXsize", "()D") => Some(Ok(Value::Float(b[3] - b[0]))),
                ("getYsize", "()D") => Some(Ok(Value::Float(b[4] - b[1]))),
                ("getZsize", "()D") => Some(Ok(Value::Float(b[5] - b[2]))),
                ("min", "(Lnet/minecraft/core/Direction$Axis;)D") => {
                    let arg = args.first()?;
                    if *arg == Value::Ref(None) {
                        return Some(Err(npe()));
                    }
                    let axis = axis_index(arg)?;
                    Some(Ok(Value::Float(b[axis])))
                }
                ("max", "(Lnet/minecraft/core/Direction$Axis;)D") => {
                    let arg = args.first()?;
                    if *arg == Value::Ref(None) {
                        return Some(Err(npe()));
                    }
                    let axis = axis_index(arg)?;
                    Some(Ok(Value::Float(b[axis + 3])))
                }
                ("move", "(DDD)Lnet/minecraft/world/phys/AABB;") => {
                    let (dx, dy, dz) = match (
                        double_arg(r, args, 0),
                        double_arg(r, args, 1),
                        double_arg(r, args, 2),
                    ) {
                        (Ok(x), Ok(y), Ok(z)) => (x, y, z),
                        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Some(Err(e)),
                    };
                    Some(Ok(aabb_value([
                        b[0] + dx,
                        b[1] + dy,
                        b[2] + dz,
                        b[3] + dx,
                        b[4] + dy,
                        b[5] + dz,
                    ])))
                }
                _ => None,
            }
        }
        HostKind::BooleanOp => {
            if r.name == "apply" && r.descriptor == "(ZZ)Z" {
                let op = BooleanOpKind::from_name(handle.key())?;
                let a = match expect_int(r, args.first().unwrap_or(&Value::Int(0))) {
                    Ok(v) => v != 0,
                    Err(e) => return Some(Err(e)),
                };
                let b = match expect_int(r, args.get(1).unwrap_or(&Value::Int(0))) {
                    Ok(v) => v != 0,
                    Err(e) => return Some(Err(e)),
                };
                return Some(Ok(Value::Int(i32::from(op.apply(a, b)))));
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn get_field(receiver: &Value, r: &MethodRef) -> Option<Result<Value, NativeError>> {
    let Value::Host(handle) = receiver else {
        return None;
    };
    if handle.kind() != HostKind::Aabb || r.owner != AABB || r.descriptor != "D" {
        return None;
    }
    let b = aabb_bounds(handle.key())?;
    let val = match r.name.as_str() {
        "minX" => b[0],
        "minY" => b[1],
        "minZ" => b[2],
        "maxX" => b[3],
        "maxY" => b[4],
        "maxZ" => b[5],
        _ => return None,
    };
    Some(Ok(Value::Float(val)))
}

/// Pure static methods on `net/minecraft/core/Direction` (`from2DDataValue`, `from3DDataValue`),
/// verified against MC 26.3 bytecode.
pub(crate) fn direction_static_call(
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/minecraft/core/Direction" {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("from3DDataValue", "(I)Lnet/minecraft/core/Direction;") => {
            const BY_3D: [&str; 6] = ["DOWN", "UP", "NORTH", "SOUTH", "WEST", "EAST"];
            let v = match expect_int(r, args.first().unwrap_or(&Value::Int(0))) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let idx = (v % 6).unsigned_abs() as usize;
            Some(Ok(Value::Ref(Some(format!(
                "enum/net/minecraft/core/Direction/{}",
                BY_3D[idx]
            )))))
        }
        ("from2DDataValue", "(I)Lnet/minecraft/core/Direction;") => {
            const BY_2D: [&str; 4] = ["SOUTH", "WEST", "NORTH", "EAST"];
            let v = match expect_int(r, args.first().unwrap_or(&Value::Int(0))) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let idx = (v % 4).unsigned_abs() as usize;
            Some(Ok(Value::Ref(Some(format!(
                "enum/net/minecraft/core/Direction/{}",
                BY_2D[idx]
            )))))
        }
        _ => None,
    }
}

/// Pure instance methods on `net/minecraft/core/Direction` and `Direction$Axis`, verified against
/// MC 26.3 bytecode and JDK 25 captures.
pub(crate) fn direction_instance_call(
    r: &MethodRef,
    receiver: &Value,
) -> Option<Result<Value, NativeError>> {
    let marker = expect_ref(receiver)?;
    if let Some(name) = marker.strip_prefix("enum/net/minecraft/core/Direction/")
        && matches!(
            r.owner.as_str(),
            "net/minecraft/core/Direction" | "net/minecraft/util/StringRepresentable"
        )
    {
        let dir = |d: &str| Value::Ref(Some(format!("enum/net/minecraft/core/Direction/{d}")));
        let axis =
            |a: &str| Value::Ref(Some(format!("enum/net/minecraft/core/Direction$Axis/{a}")));
        return match (r.name.as_str(), r.descriptor.as_str()) {
            ("get3DDataValue", "()I") => Some(Ok(Value::Int(match name {
                "DOWN" => 0,
                "UP" => 1,
                "NORTH" => 2,
                "SOUTH" => 3,
                "WEST" => 4,
                "EAST" => 5,
                _ => return None,
            }))),
            ("get2DDataValue", "()I") => Some(Ok(Value::Int(match name {
                "DOWN" | "UP" => -1,
                "SOUTH" => 0,
                "WEST" => 1,
                "NORTH" => 2,
                "EAST" => 3,
                _ => return None,
            }))),
            ("getOpposite", "()Lnet/minecraft/core/Direction;") => Some(Ok(dir(match name {
                "DOWN" => "UP",
                "UP" => "DOWN",
                "NORTH" => "SOUTH",
                "SOUTH" => "NORTH",
                "WEST" => "EAST",
                "EAST" => "WEST",
                _ => return None,
            }))),
            ("getClockWise", "()Lnet/minecraft/core/Direction;") => Some(match name {
                "NORTH" => Ok(dir("EAST")),
                "EAST" => Ok(dir("SOUTH")),
                "SOUTH" => Ok(dir("WEST")),
                "WEST" => Ok(dir("NORTH")),
                "DOWN" | "UP" => Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalStateException".into(),
                    message: Some(format!(
                        "Unable to get Y-rotated facing of {}",
                        name.to_ascii_lowercase()
                    )),
                    frame: None,
                }),
                _ => return None,
            }),
            ("getCounterClockWise", "()Lnet/minecraft/core/Direction;") => Some(match name {
                "NORTH" => Ok(dir("WEST")),
                "EAST" => Ok(dir("NORTH")),
                "SOUTH" => Ok(dir("EAST")),
                "WEST" => Ok(dir("SOUTH")),
                "DOWN" | "UP" => Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalStateException".into(),
                    message: Some(format!(
                        "Unable to get CCW facing of {}",
                        name.to_ascii_lowercase()
                    )),
                    frame: None,
                }),
                _ => return None,
            }),
            ("getAxis", "()Lnet/minecraft/core/Direction$Axis;") => Some(Ok(axis(match name {
                "DOWN" | "UP" => "Y",
                "NORTH" | "SOUTH" => "Z",
                "WEST" | "EAST" => "X",
                _ => return None,
            }))),
            ("getStepX", "()I") => Some(Ok(Value::Int(match name {
                "WEST" => -1,
                "EAST" => 1,
                "DOWN" | "UP" | "NORTH" | "SOUTH" => 0,
                _ => return None,
            }))),
            ("getStepY", "()I") => Some(Ok(Value::Int(match name {
                "DOWN" => -1,
                "UP" => 1,
                "NORTH" | "SOUTH" | "WEST" | "EAST" => 0,
                _ => return None,
            }))),
            ("getStepZ", "()I") => Some(Ok(Value::Int(match name {
                "NORTH" => -1,
                "SOUTH" => 1,
                "DOWN" | "UP" | "WEST" | "EAST" => 0,
                _ => return None,
            }))),
            ("getName" | "getSerializedName", "()Ljava/lang/String;") => {
                Some(Ok(Value::Ref(Some(name.to_ascii_lowercase()))))
            }
            _ => None,
        };
    }
    if let Some(axis) = marker.strip_prefix("enum/net/minecraft/core/Direction$Axis/")
        && matches!(
            r.owner.as_str(),
            "net/minecraft/core/Direction$Axis" | "net/minecraft/util/StringRepresentable"
        )
    {
        return match (r.name.as_str(), r.descriptor.as_str()) {
            ("isHorizontal", "()Z") => Some(Ok(Value::Int(i32::from(matches!(axis, "X" | "Z"))))),
            ("isVertical", "()Z") => Some(Ok(Value::Int(i32::from(axis == "Y")))),
            ("getName" | "getSerializedName", "()Ljava/lang/String;") => {
                Some(Ok(Value::Ref(Some(axis.to_ascii_lowercase()))))
            }
            _ => None,
        };
    }
    None
}

fn double_arg(r: &MethodRef, args: &[Value], index: usize) -> Result<f64, NativeError> {
    let val = args
        .get(index)
        .ok_or_else(|| NativeError::Metadata(format!("{} missing argument {index}", r.name)))?;
    expect_float(r, val)
}

fn six_doubles(r: &MethodRef, args: &[Value]) -> Result<[f64; 6], NativeError> {
    Ok([
        double_arg(r, args, 0)?,
        double_arg(r, args, 1)?,
        double_arg(r, args, 2)?,
        double_arg(r, args, 3)?,
        double_arg(r, args, 4)?,
        double_arg(r, args, 5)?,
    ])
}

impl NativeJar {
    /// Walk the jar-side + linked superclass chain of `class` looking for a method `(name, desc)`.
    /// Returns `(Some(jar_declaring_owner), class_where_walk_stopped)` or `(None, first_external_super)`.
    fn find_jar_shape_override(
        &self,
        host: &StdHost,
        class: &str,
        name: &str,
    ) -> (Option<String>, String) {
        let mut current = class.to_owned();
        for _ in 0..MAX_SUPERCLASS_WALK {
            let file = self
                .classes
                .get(&current)
                .or_else(|| host.linked_class_table().and_then(|t| t.get(&current)));
            let Some(file) = file else {
                break;
            };
            if file.method(name, GET_SHAPE_DESC).is_some() {
                return (Some(current.clone()), current);
            }
            if file.super_name.is_empty() {
                break;
            }
            current = file.super_name.clone();
        }
        (None, current)
    }

    /// Invoke a jar-side `getShape` or `getCollisionShape` override for every state in
    /// `state_combos`, passing opaque probe `BlockGetter` / `BlockPos` / `CollisionContext`
    /// handles so any read of world, position, or entity context is counted and refuses the block.
    fn evaluate_jar_shape_states(
        &self,
        target: &MethodRef,
        block: &Value,
        marker: &str,
        state_combos: &[BTreeMap<String, String>],
        host: &mut StdHost,
        budget: &ExecutionBudget,
    ) -> Result<Option<Vec<BlockStateShape>>, NativeError> {
        let mut results = Vec::with_capacity(state_combos.len());
        for state_map in state_combos {
            let extraction_id = host.next_shape_extraction_id();
            let level = probe_block_getter(extraction_id);
            let pos = probe_block_pos(extraction_id);
            let ctx = probe_collision_context(extraction_id);
            let state_val = host.mint_block_state(marker, state_map.clone());
            let unknown_before = host.unknown_host_call_count();
            let args = [state_val, level, pos, ctx];
            let call = self.invoke_persistent_shape_extraction(
                target,
                Some(block.clone()),
                &args,
                host,
                budget,
                extraction_id,
            );
            let unknown_after = host.unknown_host_call_count();
            match call {
                Ok(value) => {
                    if unknown_after > unknown_before {
                        return Ok(None);
                    }
                    let Some(shape) = shape_data(host, &value) else {
                        if !matches!(value, Value::Ref(None)) {
                            let _ = host.unknown_host_call(target);
                        }
                        return Ok(None);
                    };
                    results.push(BlockStateShape {
                        state: state_map.clone(),
                        boxes: shape.to_boxes(),
                    });
                }
                Err(err) => {
                    if unknown_after > unknown_before
                        && matches!(err, NativeError::UncaughtException { .. })
                    {
                        // A counted unmodelled read returned a neutral value and caused a Java
                        // exception downstream: treat as no shape (keeping the count).
                        return Ok(None);
                    }
                    return Err(err);
                }
            }
        }
        Ok(Some(results))
    }

    /// Resolve a block's outline (`getShape`) and collision (`getCollisionShape`) shapes across
    /// every state of its state definition.
    ///
    /// * Outline (`getShape`): if no jar class overrides `getShape` and the first vanilla
    ///   superclass inherits `BlockBehaviour.getShape` (`Shapes.block()`), returns the unit cube
    ///   without running jar bytecode (`overridden: false`). Otherwise invokes the override per
    ///   state with opaque `BlockGetter`, `BlockPos`, and `CollisionContext` probes so any read
    ///   of position, world, or context is counted and refuses the block.
    /// * Collision (`getCollisionShape`): if a jar class overrides `getCollisionShape`, invokes
    ///   that override per state with the same opaque probes. Otherwise, when the vanilla
    ///   superclass inherits `BlockBehaviour.getCollisionShape` (`hasCollision ? state.getShape :
    ///   Shapes.empty()`), derives collision from `Properties.noCollision()` (empty) or proves
    ///   equivalence to outline (`hasCollision == true`). If `hasCollision` is unknown (e.g.
    ///   `Properties.ofFullCopy`) or the vanilla superclass overrides `getCollisionShape`, counts
    ///   it and yields `collision_states: None`.
    pub fn block_shapes(
        &self,
        host: &mut StdHost,
        block: &Value,
    ) -> Result<BlockShapes, NativeError> {
        self.block_shapes_with_budget(host, block, &ExecutionBudget::UNBOUNDED)
    }

    /// Bounded variant of [`Self::block_shapes`].
    pub fn block_shapes_with_budget(
        &self,
        host: &mut StdHost,
        block: &Value,
        budget: &ExecutionBudget,
    ) -> Result<BlockShapes, NativeError> {
        let Some(marker) = crate::runtime::jar_object::object_key(block).map(str::to_owned) else {
            return Ok(BlockShapes {
                properties: Vec::new(),
                states: None,
                overridden: false,
                collision_states: None,
                collision_overridden: false,
            });
        };
        let Some(class) = crate::runtime::jar_object::key_class(&marker).map(str::to_owned) else {
            return Ok(BlockShapes {
                properties: Vec::new(),
                states: None,
                overridden: false,
                collision_states: None,
                collision_overridden: false,
            });
        };

        let (shape_owner, shape_super) = self.find_jar_shape_override(host, &class, "getShape");
        let (collision_owner, collision_super) =
            self.find_jar_shape_override(host, &class, "getCollisionShape");
        let overridden = shape_owner.is_some();
        let collision_overridden = collision_owner.is_some();

        // If the block's state space wasn't captured cleanly, we cannot enumerate its states.
        if host.has_unknown_block_state_definition(&marker) {
            return Ok(BlockShapes {
                properties: Vec::new(),
                states: None,
                overridden,
                collision_states: None,
                collision_overridden,
            });
        }

        let (properties, state_combos) =
            match host.enumerate_block_state_combinations(&marker, MAX_BLOCK_SHAPE_STATES) {
                Some(enumerated) => enumerated,
                None => {
                    // State count exceeded safety cap: count and refuse.
                    let _ = host.unknown_host_call(&MethodRef {
                        owner: shape_owner
                            .clone()
                            .unwrap_or_else(|| BLOCK_CLASS.to_owned()),
                        name: "getShape".into(),
                        descriptor: GET_SHAPE_DESC.into(),
                    });
                    return Ok(BlockShapes {
                        properties: Vec::new(),
                        states: None,
                        overridden,
                        collision_states: None,
                        collision_overridden,
                    });
                }
            };

        // 1. Outline shape (`getShape`).
        let states = if let Some(owner) = shape_owner {
            let target = MethodRef {
                owner,
                name: "getShape".into(),
                descriptor: GET_SHAPE_DESC.into(),
            };
            self.evaluate_jar_shape_states(&target, block, &marker, &state_combos, host, budget)?
        } else if crate::runtime::vanilla_block_shapes::inherits_block_behaviour_get_shape(&shape_super) {
            let unit = VoxelShapeData::block().to_boxes();
            Some(
                state_combos
                    .iter()
                    .map(|state| BlockStateShape {
                        state: state.clone(),
                        boxes: unit.clone(),
                    })
                    .collect(),
            )
        } else {
            let _ = host.unknown_host_call(&MethodRef {
                owner: shape_super,
                name: "getShape".into(),
                descriptor: GET_SHAPE_DESC.into(),
            })?;
            None
        };

        // 2. Collision shape (`getCollisionShape`).
        let collision_states = if let Some(owner) = collision_owner {
            let target = MethodRef {
                owner,
                name: "getCollisionShape".into(),
                descriptor: GET_SHAPE_DESC.into(),
            };
            self.evaluate_jar_shape_states(&target, block, &marker, &state_combos, host, budget)?
        } else if crate::runtime::vanilla_block_shapes::inherits_block_behaviour_get_collision_shape(
            &collision_super,
        ) {
            // MC 26.3 `BlockBehaviour.getCollisionShape`:
            // `return this.hasCollision ? state.getShape(level, pos) : Shapes.empty();`
            match host.block_has_collision_by_marker(&marker) {
                Some(false) => Some(
                    state_combos
                        .iter()
                        .map(|state| BlockStateShape {
                            state: state.clone(),
                            boxes: Vec::new(),
                        })
                        .collect(),
                ),
                Some(true) => states.clone(),
                None => {
                    let _ = host.unknown_host_call(&MethodRef {
                        owner: "net/minecraft/world/level/block/state/BlockBehaviour".into(),
                        name: "getCollisionShape".into(),
                        descriptor: GET_SHAPE_DESC.into(),
                    })?;
                    None
                }
            }
        } else {
            let _ = host.unknown_host_call(&MethodRef {
                owner: collision_super,
                name: "getCollisionShape".into(),
                descriptor: GET_SHAPE_DESC.into(),
            })?;
            None
        };

        Ok(BlockShapes {
            properties,
            states,
            overridden,
            collision_states,
            collision_overridden,
        })
    }

    /// Evaluate `getShape` and `getCollisionShape` for every typed `minecraft:block` registration
    /// on `host` and attach valid Sunrise shape tables (`shapes` and, when distinct,
    /// `collision_shapes`) to their declarations so `content::declarations` (and both
    /// `tools/bake.py` and `mod/src/content.rs`) export them.
    pub fn populate_block_shapes(&self, host: &mut StdHost, budget: &ExecutionBudget) {
        let targets: Vec<(usize, Value)> = host
            .registrations()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.registry == "minecraft:block"
                    && matches!(entry.declaration, Some(RegistryDeclaration::Block { .. }))
            })
            .map(|(index, entry)| (index, entry.value.clone()))
            .collect();
        for (index, block) in targets {
            let Ok(shapes) = self.block_shapes_with_budget(host, &block, budget) else {
                continue;
            };
            let (outline, collision) = shapes.to_content_export();
            if outline.is_none() && collision.is_none() {
                continue;
            }
            host.set_block_declaration_shapes(
                index,
                outline.map(|s| s.to_json().to_string()),
                collision.map(|s| s.to_json().to_string()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verified against MC 26.3 (`ShapeOracle.java` on JDK 25.0.5+8 with `server-26.3.jar`).
    #[test]
    fn voxel_shape_matches_mc263_oracle_captures() {
        // Empty & full block
        assert!(VoxelShapeData::empty().is_empty());
        assert_eq!(VoxelShapeData::empty().to_boxes(), Vec::<[f64; 6]>::new());
        assert!(!VoxelShapeData::block().is_empty());
        assert_eq!(
            VoxelShapeData::block().to_boxes(),
            vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
        );
        assert_eq!(
            VoxelShapeData::block().to_java_string(),
            "VoxelShape[AABB[0.0, 0.0, 0.0] -> [1.0, 1.0, 1.0]]"
        );
        assert_eq!(VoxelShapeData::empty().to_java_string(), "EMPTY");

        // Epsilon threshold: < 1e-7 -> empty, == 1e-7 -> non-empty
        assert!(
            VoxelShapeData::box_checked(0.0, 0.0, 0.0, 9.9e-8, 1.0, 1.0)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            VoxelShapeData::box_checked(0.0, 0.0, 0.0, 1.0e-7, 1.0, 1.0)
                .unwrap()
                .to_boxes(),
            vec![[0.0, 0.0, 0.0, 1.0e-7, 1.0, 1.0]]
        );

        // min > max throws IllegalArgumentException
        let err = VoxelShapeData::box_checked(1.0, 0.0, 0.0, 0.5, 1.0, 1.0).unwrap_err();
        assert!(matches!(
            err,
            NativeError::UncaughtException { ref class, ref message, .. }
                if class == "java/lang/IllegalArgumentException"
                && message.as_deref() == Some("The min values need to be smaller or equals to the max values")
        ));

        // empty.bounds() throws UnsupportedOperationException
        let err = VoxelShapeData::empty().bounds().unwrap_err();
        assert!(matches!(
            err,
            NativeError::UncaughtException { ref class, ref message, .. }
                if class == "java/lang/UnsupportedOperationException"
                && message.as_deref() == Some("No bounds for empty shape.")
        ));

        // Three-tier pedestal shape: exact 6-box decomposition from real MC 26.3 `toAabbs()`
        let top = VoxelShapeData::block_box(1.0, 12.0, 1.0, 15.0, 16.0, 15.0).unwrap();
        let mid = VoxelShapeData::block_box(4.0, 3.0, 4.0, 12.0, 12.0, 12.0).unwrap();
        let bot = VoxelShapeData::block_box(1.0, 0.0, 1.0, 15.0, 3.0, 15.0).unwrap();
        let combined = top
            .join(&mid, BooleanOpKind::Or, false)
            .unwrap()
            .unwrap()
            .join(&bot, BooleanOpKind::Or, false)
            .unwrap()
            .unwrap();
        assert_eq!(
            combined.to_boxes(),
            vec![
                [0.0625, 0.0, 0.0625, 0.9375, 0.1875, 0.9375],
                [0.25, 0.1875, 0.25, 0.75, 1.0, 0.75],
                [0.0625, 0.75, 0.0625, 0.25, 1.0, 0.9375],
                [0.25, 0.75, 0.0625, 0.9375, 1.0, 0.25],
                [0.25, 0.75, 0.75, 0.9375, 1.0, 0.9375],
                [0.75, 0.75, 0.25, 0.9375, 1.0, 0.75],
            ]
        );
        assert_eq!(
            combined.bounds().unwrap(),
            [0.0625, 0.0, 0.0625, 0.9375, 1.0, 0.9375]
        );

        // Overlapping, touching, L-shape, move, and BooleanOps
        let b1 = VoxelShapeData::block_box(0.0, 0.0, 0.0, 10.0, 10.0, 10.0).unwrap();
        let b2 = VoxelShapeData::block_box(6.0, 0.0, 0.0, 16.0, 10.0, 10.0).unwrap();
        assert_eq!(
            b1.join(&b2, BooleanOpKind::Or, false)
                .unwrap()
                .unwrap()
                .to_boxes(),
            vec![[0.0, 0.0, 0.0, 1.0, 0.625, 0.625]]
        );
        assert_eq!(
            b1.join(&b2, BooleanOpKind::And, false)
                .unwrap()
                .unwrap()
                .to_boxes(),
            vec![[0.375, 0.0, 0.0, 0.625, 0.625, 0.625]]
        );
        assert_eq!(
            b1.join(&b2, BooleanOpKind::OnlyFirst, false)
                .unwrap()
                .unwrap()
                .to_boxes(),
            vec![[0.0, 0.0, 0.0, 0.375, 0.625, 0.625]]
        );
        assert_eq!(
            b1.join(&b2, BooleanOpKind::OnlySecond, false)
                .unwrap()
                .unwrap()
                .to_boxes(),
            vec![[0.625, 0.0, 0.0, 1.0, 0.625, 0.625]]
        );
        assert_eq!(
            b1.join(&b2, BooleanOpKind::NotSame, false)
                .unwrap()
                .unwrap()
                .to_boxes(),
            vec![
                [0.0, 0.0, 0.0, 0.375, 0.625, 0.625],
                [0.625, 0.0, 0.0, 1.0, 0.625, 0.625],
            ]
        );

        let moved = VoxelShapeData::block_box(0.0, 0.0, 0.0, 8.0, 8.0, 8.0)
            .unwrap()
            .move_by(0.5, 0.25, 0.0);
        assert_eq!(moved.to_boxes(), vec![[0.5, 0.25, 0.0, 1.0, 0.75, 0.5]]);
    }
}

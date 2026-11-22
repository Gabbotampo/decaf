//! Interface-injection `@Mixin` classes on host-modelled game objects.
//!
//! Real Mixin merges a `@Mixin(Target.class) class M implements I` class onto
//! `Target`: `Target` implements `I` (and its superinterfaces), `M`'s added
//! (`@Unique` or shadowless) fields live on each `Target` instance for that
//! object's lifetime, `M`'s concrete methods dispatch on `Target` with `this`
//! bound to the `Target` instance, and `M`'s no-argument `<init>()V` field
//! initialisers run once per `Target` instance.
//!
//! For host-modelled game classes (`Player`, `ServerPlayer`, `LivingEntity`,
//! `Entity`, `Level`, `ItemStack`, etc.) there is no target bytecode to weave,
//! so this module plans those interface-injection mixins and applies them at
//! runtime over the host's game handles (`game_object.rs`), keeping per-object
//! fields in the resident heap keyed by the handle's canonical identity so they
//! survive calls and GC and are cleared when the object is forgotten.
//!
//! Anything this module cannot apply exactly — targets outside the modelled
//! host surface, unlisted/wrong-side/plugin-gated configs, ambiguous overrides,
//! unrunnable non-`Object` field initialisers, and `@Shadow` reads/writes to
//! unmodelled game members — stays skipped in the weave plan or is counted /
//! refused loudly at runtime, never silently guessed.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

use crate::checkcast::class_access;
use crate::game_hooks::{MIXIN_SHADOW_DESCRIPTOR, member_annotations};
use crate::hook_shapes::mixin_config_gate_with_plugins;
use crate::mixin_plugins::ConfigPlugins;
use crate::{
    Annotation, ClassFile, Host, LoadTimeMixinWeave, MethodRef, MixinConfig, MixinExtrasKind,
    MixinSide, MixinSkipReason, NativeError, NativeJar, Runtime, SkippedMixinPlan, Value,
    instance_field_key, mixin_config_gate, mixin_target_owners, resolve_mixin_accessor,
    return_desc,
};

const ACC_PRIVATE: u16 = 0x0002;
const ACC_FINAL: u16 = 0x0010;
const ACC_INTERFACE: u16 = 0x0200;
const ACC_ABSTRACT: u16 = 0x0400;
const ACC_SYNTHETIC: u16 = 0x1000;

const MIXIN_FINAL_DESCRIPTOR: &str = "Lorg/spongepowered/asm/mixin/Final;";
const MIXIN_MUTABLE_DESCRIPTOR: &str = "Lorg/spongepowered/asm/mixin/Mutable;";

const PLANNED_INJECTOR_ANNOTATIONS: [&str; 5] = [
    "Lorg/spongepowered/asm/mixin/injection/Inject;",
    "Lorg/spongepowered/asm/mixin/injection/Redirect;",
    "Lorg/spongepowered/asm/mixin/injection/ModifyArg;",
    "Lorg/spongepowered/asm/mixin/injection/ModifyVariable;",
    "Lorg/spongepowered/asm/mixin/Overwrite;",
];

/// Vanilla classes represented by live host game handles (`game_object::declared_types`
/// and `game_hooks::GameObject::hierarchy`). Interface-injection mixins targeting any
/// class outside this allow-list are skipped as `target-outside-jar` (or
/// `unsupported-family` when jar-local) rather than falsely claimed as applied.
const HOST_MODELLED_GAME_TARGETS: &[&str] = &[
    "com/mojang/authlib/GameProfile",
    "com/mojang/blaze3d/platform/InputConstants$Key",
    "com/mojang/renderpearl/api/pipeline/RenderPipeline",
    "net/minecraft/client/KeyMapping$Category",
    "net/minecraft/client/Minecraft",
    "net/minecraft/client/Options",
    "net/minecraft/client/gui/Font",
    "net/minecraft/client/gui/Gui",
    "net/minecraft/client/multiplayer/ClientLevel",
    "net/minecraft/client/player/AbstractClientPlayer",
    "net/minecraft/client/player/LocalPlayer",
    "net/minecraft/commands/Commands",
    "net/minecraft/core/BlockPos",
    "net/minecraft/core/component/DataComponentGetter",
    "net/minecraft/core/component/DataComponentHolder",
    "net/minecraft/core/component/DataComponentType",
    "net/minecraft/network/chat/Style",
    "net/minecraft/network/chat/TextColor",
    "net/minecraft/server/MinecraftServer",
    "net/minecraft/server/dedicated/DedicatedServer",
    "net/minecraft/server/level/ServerLevel",
    "net/minecraft/server/level/ServerPlayer",
    "net/minecraft/server/players/PlayerList",
    "net/minecraft/sounds/SoundEvent",
    "net/minecraft/world/Container",
    "net/minecraft/world/Nameable",
    "net/minecraft/world/damagesource/DamageSource",
    "net/minecraft/world/effect/MobEffect",
    "net/minecraft/world/entity/Avatar",
    "net/minecraft/world/entity/Entity",
    "net/minecraft/world/entity/LivingEntity",
    "net/minecraft/world/entity/ai/attributes/Attribute",
    "net/minecraft/world/entity/item/ItemEntity",
    "net/minecraft/world/entity/player/Inventory",
    "net/minecraft/world/entity/player/Player",
    "net/minecraft/world/item/Item",
    "net/minecraft/world/item/ItemInstance",
    "net/minecraft/world/item/ItemStack",
    "net/minecraft/world/item/crafting/RecipeAccess",
    "net/minecraft/world/item/crafting/RecipeManager",
    "net/minecraft/world/level/ItemLike",
    "net/minecraft/world/level/Level",
    "net/minecraft/world/level/block/Block",
    "net/minecraft/world/level/block/entity/BlockEntityType",
    "net/minecraft/world/level/block/state/BlockState",
    "net/minecraft/world/level/border/WorldBorder",
    "net/minecraft/world/level/gamerules/GameRule",
    "net/minecraft/world/level/storage/LevelResource",
    "net/minecraft/world/phys/Vec3",
];

/// Whether `target` (internal name) is a host-modelled game class that can carry
/// runtime interface-injection Mixin state.
pub(crate) fn is_host_modelled_game_target(target: &str) -> bool {
    HOST_MODELLED_GAME_TARGETS.binary_search(&target).is_ok()
}

/// One interface-injection `@Mixin` plan applied natively to a host-modelled game class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixinInterfacePlan {
    /// Internal name of the `@Mixin` class.
    pub mixin: String,
    /// Internal name of the host-modelled target class.
    pub target: String,
    /// Direct interfaces implemented by the mixin class (internal names).
    pub interfaces: Vec<String>,
    /// Non-`@Shadow`, non-synthetic fields declared on the mixin class.
    pub added_fields: Vec<MethodRef>,
    /// Non-`@Shadow`, non-synthetic concrete non-static methods declared on the mixin class
    /// (excluding `<init>`, `<clinit>`, injectors and `@Accessor`/`@Invoker`).
    pub added_methods: Vec<MethodRef>,
    /// Whether the mixin class has a runnable no-argument `<init>()V` field initialiser.
    pub has_initializer: bool,
}

/// What [`NativeJar::plan_interface_mixins`] reports back to `unwoven_mixin_members`.
#[derive(Debug, Default)]
pub(crate) struct InterfaceWeaveSummary {
    /// Mixin classes with at least one applied [`MixinInterfacePlan`].
    pub(crate) applied_mixins: BTreeSet<String>,
    /// Pure interface-injection mixin classes that were already rejected with an explicit
    /// skip reason (`NotInConfig`, `OtherSide`, `ConfigPlugin`, `TargetOutsideJar`,
    /// `AmbiguousTarget`, etc.) so `unwoven_mixin_members` does not emit a duplicate row.
    pub(crate) accounted_mixins: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticShadowField {
    is_final: bool,
    is_mutable: bool,
}

/// Resolution of a `getstatic`/`putstatic` on a `@Mixin` class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StaticShadowResolution {
    /// Not a static `@Shadow` (or inherited game static): use the class's own static slot.
    NotShadow,
    /// Unambiguous static `@Shadow` resolved to `target` with its Mixin `@Final`/`@Mutable` flags.
    Resolved {
        target: MethodRef,
        is_final: bool,
        is_mutable: bool,
    },
    /// Ambiguous multi-target static `@Shadow`: caller must count or refuse without reading/writing
    /// a mixin-local slot.
    Ambiguous,
}

/// Detailed member breakdown of one `@Mixin` class.
#[derive(Debug, Clone)]
struct MixinClassDetails {
    mixin: String,
    targets: Vec<String>,
    is_interface: bool,
    interfaces: Vec<String>,
    added_fields: Vec<MethodRef>,
    added_instance_fields: BTreeSet<(String, String)>,
    shadow_fields: BTreeSet<(String, String)>,
    static_shadow_fields: BTreeMap<(String, String), StaticShadowField>,
    added_methods: Vec<MethodRef>,
    added_public_methods: BTreeSet<(String, String)>,
    shadow_methods: BTreeSet<(String, String)>,
    has_initializer: bool,
    unrunnable_initializer: bool,
    extends_game_class: bool,
    declared_chain_fields: BTreeSet<(String, String)>,
}

/// Runtime metadata for one active interface-injection mixin on a host target.
#[derive(Debug, Clone)]
struct ActiveInterfaceMixin {
    mixin: String,
    interfaces: Vec<String>,
    added_public_methods: BTreeSet<(String, String)>,
}

/// Runtime metadata for any `@Mixin` class (active or inactive, interface-injecting or not).
#[derive(Debug, Clone)]
struct MixinClassMeta {
    targets: Vec<String>,
    active_interface_targets: BTreeSet<String>,
    added_instance_fields: BTreeSet<(String, String)>,
    shadow_fields: BTreeSet<(String, String)>,
    static_shadow_fields: BTreeMap<(String, String), StaticShadowField>,
    shadow_methods: BTreeSet<(String, String)>,
    has_initializer: bool,
    extends_game_class: bool,
    declared_chain_fields: BTreeSet<(String, String)>,
}

/// Cached per-`Runtime` index of active `@Mixin` classes across the primary jar and
/// linked jars, built lazily on first type-check or game-handle dispatch.
#[derive(Debug, Default)]
pub(crate) struct RuntimeMixinIndex {
    /// Target internal name → active `@Mixin` classes targeting it (accessors and classes).
    by_target: BTreeMap<String, Vec<String>>,
    /// Host-modelled target internal name → active interface-injection mixins targeting it.
    interface_by_target: BTreeMap<String, Vec<ActiveInterfaceMixin>>,
    /// Mixin internal name → member classification metadata.
    meta: BTreeMap<String, MixinClassMeta>,
}

/// Non-private, non-static methods declared by `java/lang/Object`.
const JAVA_OBJECT_METHODS: &[(&str, &str)] = &[
    ("clone", "()Ljava/lang/Object;"),
    ("equals", "(Ljava/lang/Object;)Z"),
    ("finalize", "()V"),
    ("getClass", "()Ljava/lang/Class;"),
    ("hashCode", "()I"),
    ("notify", "()V"),
    ("notifyAll", "()V"),
    ("toString", "()Ljava/lang/String;"),
    ("wait", "()V"),
    ("wait", "(J)V"),
    ("wait", "(JI)V"),
];

const VANILLA_HOST_METHODS_TSV: &str = include_str!("vanilla_host_methods.tsv");

#[derive(Default)]
struct VanillaTypeInfo {
    super_name: &'static str,
    interfaces: Vec<&'static str>,
    methods: BTreeSet<(&'static str, &'static str)>,
}

#[derive(Default)]
struct VanillaHostMethods {
    classes: BTreeMap<&'static str, VanillaTypeInfo>,
    interfaces: BTreeMap<&'static str, VanillaTypeInfo>,
}

#[derive(Debug, PartialEq, Eq)]
enum VanillaMethodCheck {
    /// A class in the target's superclass chain (or `java/lang/Object`) declares the
    /// method; under JVMS §5.4.3.3, class methods always win over interface defaults.
    ClassMethod(&'static str),
    /// No class in the superclass chain declares the method, and the full class +
    /// interface closure of the target is present in `vanilla_host_methods.tsv`.
    /// Lists all maximally-specific vanilla/JDK interfaces declaring a `default` method.
    VanillaDefaults(Vec<&'static str>),
    /// A class or interface edge in the target's hierarchy closure is missing from
    /// `vanilla_host_methods.tsv`, so callers must fail closed (count/refuse) rather
    /// than letting an injected interface default win.
    MissingHierarchyEdge(String),
}

impl VanillaHostMethods {
    fn parse(tsv: &'static str) -> Self {
        let mut table = Self::default();
        for line in tsv.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            match parts.next() {
                Some("C") => {
                    let (Some(name), Some(super_name), Some(ifaces)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        continue;
                    };
                    let interfaces = if ifaces.is_empty() {
                        Vec::new()
                    } else {
                        ifaces.split(',').collect()
                    };
                    table.classes.insert(
                        name,
                        VanillaTypeInfo {
                            super_name,
                            interfaces,
                            methods: BTreeSet::new(),
                        },
                    );
                }
                Some("I") => {
                    let (Some(name), Some(_empty), Some(ifaces)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        continue;
                    };
                    let interfaces = if ifaces.is_empty() {
                        Vec::new()
                    } else {
                        ifaces.split(',').collect()
                    };
                    table.interfaces.insert(
                        name,
                        VanillaTypeInfo {
                            super_name: "",
                            interfaces,
                            methods: BTreeSet::new(),
                        },
                    );
                }
                Some("M") => {
                    let (Some(owner), Some(name), Some(desc)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        continue;
                    };
                    if let Some(info) = table.classes.get_mut(owner) {
                        info.methods.insert((name, desc));
                    } else if let Some(info) = table.interfaces.get_mut(owner) {
                        info.methods.insert((name, desc));
                    }
                }
                _ => {}
            }
        }
        table
    }

    /// Whether `iface` equals or transitively extends `wanted` within the vanilla/JDK table.
    fn interface_extends(&self, iface: &str, wanted: &str) -> bool {
        if iface == wanted {
            return true;
        }
        let mut queue: VecDeque<&str> = VecDeque::from([iface]);
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        while let Some(cur) = queue.pop_front() {
            if !seen.insert(cur) {
                continue;
            }
            if let Some(info) = self.interfaces.get(cur) {
                for &parent in &info.interfaces {
                    if parent == wanted {
                        return true;
                    }
                    queue.push_back(parent);
                }
            }
        }
        false
    }

    /// Inspect `declared`'s full vanilla/JDK class and interface hierarchy for
    /// `(name, descriptor)` under JVMS §5.4.3.3 / §5.4.3.4:
    ///
    /// 1. Any non-private, non-static method declared on a class in `declared`'s
    ///    superclass chain (or `java/lang/Object`) wins over every interface default.
    /// 2. Otherwise, walk the full vanilla/JDK interface closure for `default` methods.
    /// 3. If ANY class or interface edge in `declared`'s closure is missing from the
    ///    table, return [`VanillaMethodCheck::MissingHierarchyEdge`] so callers fail
    ///    closed rather than silently skipping the missing edge.
    fn check_target_methods(
        &self,
        declared: &[String],
        name: &str,
        descriptor: &str,
    ) -> VanillaMethodCheck {
        if JAVA_OBJECT_METHODS
            .iter()
            .any(|(m, d)| *m == name && *d == descriptor)
        {
            return VanillaMethodCheck::ClassMethod("java/lang/Object");
        }
        if declared.is_empty() {
            return VanillaMethodCheck::MissingHierarchyEdge("<empty>".to_owned());
        }

        // 1. Walk class hierarchy (most specific first) and collect direct vanilla interfaces.
        let mut seen_classes: BTreeSet<&str> = BTreeSet::new();
        let mut iface_queue: VecDeque<&'static str> = VecDeque::new();
        let mut missing_edge: Option<String> = None;
        for start in declared {
            let mut cur = start.as_str();
            for _ in 0..32 {
                if cur.is_empty() || cur == "java/lang/Object" || !seen_classes.insert(cur) {
                    break;
                }
                if let Some((&key, info)) = self.classes.get_key_value(cur) {
                    if info.methods.contains(&(name, descriptor)) {
                        return VanillaMethodCheck::ClassMethod(key);
                    }
                    iface_queue.extend(info.interfaces.iter().copied());
                    cur = info.super_name;
                } else if let Some((&key, _)) = self.interfaces.get_key_value(cur) {
                    iface_queue.push_back(key);
                    break;
                } else {
                    missing_edge.get_or_insert_with(|| cur.to_owned());
                    break;
                }
            }
        }

        // 2. Walk the target's full vanilla/JDK interface closure for concrete default methods.
        let mut seen_ifaces: BTreeSet<&'static str> = BTreeSet::new();
        let mut default_owners: Vec<&'static str> = Vec::new();
        while let Some(iface) = iface_queue.pop_front() {
            if !seen_ifaces.insert(iface) {
                continue;
            }
            let Some((&key, info)) = self.interfaces.get_key_value(iface) else {
                missing_edge.get_or_insert_with(|| iface.to_owned());
                continue;
            };
            if info.methods.contains(&(name, descriptor)) {
                default_owners.push(key);
            }
            iface_queue.extend(info.interfaces.iter().copied());
        }

        // Fail closed when any class or interface edge in the target's closure is missing
        // from the table rather than silently letting an injected default win.
        if let Some(missing) = missing_edge {
            return VanillaMethodCheck::MissingHierarchyEdge(missing);
        }

        let maximal = default_owners
            .iter()
            .copied()
            .filter(|&owner| {
                !default_owners
                    .iter()
                    .any(|&other| other != owner && self.interface_extends(other, owner))
            })
            .collect();
        VanillaMethodCheck::VanillaDefaults(maximal)
    }
}

/// Whether `iface` equals or transitively extends `wanted`, walking both jar classfiles
/// and `vanilla_host_methods.tsv` (including JDK superinterfaces).
fn iface_extends_with_vanilla<H: Host>(
    runtime: &Runtime<'_, H>,
    table: &VanillaHostMethods,
    iface: &str,
    wanted: &str,
) -> bool {
    if iface == wanted {
        return true;
    }
    if runtime.interface_extends(iface, wanted) || table.interface_extends(iface, wanted) {
        return true;
    }
    let mut queue = VecDeque::from([iface.to_owned()]);
    let mut seen = BTreeSet::new();
    while let Some(cur) = queue.pop_front() {
        if cur == wanted {
            return true;
        }
        if seen.len() >= crate::MAX_SUPERCLASS_WALK || !seen.insert(cur.clone()) {
            continue;
        }
        if let Some(file) = runtime.class(&cur) {
            queue.extend(file.interface_names());
        } else if let Some(info) = table.interfaces.get(cur.as_str()) {
            queue.extend(info.interfaces.iter().map(|s| (*s).to_owned()));
        }
    }
    false
}

fn vanilla_host_methods() -> &'static VanillaHostMethods {
    static TABLE: OnceLock<VanillaHostMethods> = OnceLock::new();
    TABLE.get_or_init(|| VanillaHostMethods::parse(VANILLA_HOST_METHODS_TSV))
}

/// Resolve the owning target class for a `@Shadow` member on a `@Mixin` class given its
/// declared targets and the actual receiver's vanilla class hierarchy.
///
/// - Single-target mixins resolve unambiguously to their sole target.
/// - Multi-target mixins resolve to the unique target present in `receiver_hierarchy`.
/// - Zero or multiple matching targets are ambiguous (`None`), so callers count/refuse
///   instead of silently routing through `targets.first()`.
pub(crate) fn resolve_shadow_target<'a>(
    targets: &'a [String],
    receiver_hierarchy: &[String],
) -> Option<&'a str> {
    if targets.len() == 1 {
        return targets.first().map(String::as_str);
    }
    let mut matched: Option<&'a str> = None;
    for target in targets {
        if receiver_hierarchy.iter().any(|ty| ty == target) {
            if matched.is_some() {
                return None;
            }
            matched = Some(target.as_str());
        }
    }
    matched
}

/// Active host-modelled targets of `@Mixin` `class` on `side`, filtered through the
/// shared config, physical-side, `IMixinConfigPlugin`, and initialiser-soundness gate.
///
/// Used by `game_hooks::MixinModel::build` so game-hook `instanceof` and call dispatch
/// obey the exact same gating as `RuntimeMixinIndex`.
pub(crate) fn active_host_mixin_targets(
    jar: &NativeJar,
    configs: &[MixinConfig],
    config_sides: &BTreeMap<String, MixinSide>,
    plugins: &ConfigPlugins,
    side: MixinSide,
    class: &ClassFile,
) -> Vec<String> {
    let Some(details) = inspect_mixin_class(class, |name| jar.classes.get(name)) else {
        return Vec::new();
    };
    let is_interface_injection = !details.is_interface && !details.interfaces.is_empty();
    let mut active = Vec::new();
    for target in details.targets {
        if jar.classes.contains_key(&target) {
            continue;
        }
        if configs.is_empty() {
            continue;
        }
        if mixin_config_gate_with_plugins(
            configs,
            config_sides,
            plugins,
            &details.mixin,
            &target,
            side,
        )
        .is_err()
        {
            continue;
        }
        if is_interface_injection
            && (details.unrunnable_initializer || !is_host_modelled_game_target(&target))
        {
            continue;
        }
        active.push(target);
    }
    active
}

/// Per-`Runtime` mixin configuration gate and lazy index.
#[derive(Debug, Default)]
pub(crate) struct RuntimeMixinState {
    configs: Vec<MixinConfig>,
    config_sides: BTreeMap<String, MixinSide>,
    plugins: ConfigPlugins,
    side: Option<MixinSide>,
    require_config: bool,
    index: OnceCell<RuntimeMixinIndex>,
}

impl RuntimeMixinState {
    /// Build state for a real `NativeJar` invocation, honouring the jar's mixin configs,
    /// config plugins, and the host's physical side.
    pub(crate) fn from_jar<H: Host>(jar: &NativeJar, host: &H) -> Self {
        Self::from_jar_side(jar, host.mixin_side())
    }

    /// Build state for a real `NativeJar` invocation pinned to an explicit physical side.
    pub(crate) fn from_jar_side(jar: &NativeJar, side: MixinSide) -> Self {
        Self {
            configs: jar.mixin_configs(),
            config_sides: jar.mixin_config_sides(),
            plugins: jar.cached_config_plugins(side).clone(),
            side: Some(side),
            require_config: true,
            index: OnceCell::new(),
        }
    }

    /// Build an unconfigured state for synthetic in-memory unit tests that construct a
    /// `Runtime` directly without a `NativeJar`.
    #[cfg(test)]
    pub(crate) fn unconfigured() -> Self {
        Self {
            configs: Vec::new(),
            config_sides: BTreeMap::new(),
            plugins: ConfigPlugins::default(),
            side: None,
            require_config: false,
            index: OnceCell::new(),
        }
    }

    /// Whether `(mixin, target)` passes the primary jar's Mixin config, physical-side,
    /// and `IMixinConfigPlugin` gate.
    ///
    /// Returns `true` when config gating is disabled (`unconfigured()` in synthetic unit
    /// tests) or when the jar has readable Mixin configs that list `mixin` for the active
    /// side and any config plugin allows `(mixin, target)`. A jar with no readable Mixin
    /// configs never passes this gate.
    fn allows_config_gated(&self, mixin: &str, target: &str) -> bool {
        if !self.require_config {
            return true;
        }
        if self.configs.is_empty() {
            return false;
        }
        let Some(side) = self.side else {
            return false;
        };
        mixin_config_gate_with_plugins(
            &self.configs,
            &self.config_sides,
            &self.plugins,
            mixin,
            target,
            side,
        )
        .is_ok()
    }
}

#[derive(Clone, Copy)]
struct MixinRegistration {
    /// True for primary-jar classes (preserving the existing jar-local and non-host-game
    /// `by_target` scan), false for linked-jar classes.
    is_primary: bool,
    /// True when the primary jar has no readable Mixin config at all.
    primary_has_no_configs: bool,
}

/// Outcome of [`Runtime::game_handle_put_field`].
pub(crate) enum GameFieldWrite {
    /// Stored on the game handle as an active interface-injection mixin's added field.
    Stored,
    /// Counted as an unknown write (or refused on a strict host); do not store a fake field.
    Handled,
    /// Not a game handle or deferred to the caller's existing path.
    Fallthrough,
}

fn is_game_class(owner: &str) -> bool {
    owner.starts_with("net/minecraft/") || owner.starts_with("com/mojang/")
}

fn mixin_annotations(class: &ClassFile) -> Vec<&Annotation> {
    class
        .annotations
        .iter()
        .filter(|a| a.descriptor == crate::MIXIN_ANNOTATION_DESCRIPTOR)
        .collect()
}

fn class_targets(class: &ClassFile) -> Vec<String> {
    let mut targets = Vec::new();
    for annotation in mixin_annotations(class) {
        for owner in mixin_target_owners(annotation) {
            if !targets.contains(&owner) {
                targets.push(owner);
            }
        }
    }
    targets.sort();
    targets
}

fn has_annotation(descriptors: Option<&Vec<String>>, target: &str) -> bool {
    descriptors.is_some_and(|list| list.iter().any(|d| d == target))
}

fn is_planned_method_annotation(descriptor: &str) -> bool {
    PLANNED_INJECTOR_ANNOTATIONS.contains(&descriptor)
        || MixinExtrasKind::from_descriptor(descriptor).is_some()
        || descriptor == crate::MIXIN_ACCESSOR_DESCRIPTOR
        || descriptor == crate::MIXIN_INVOKER_DESCRIPTOR
}

/// Inspect one `@Mixin` class for its targets, interfaces, added members, `@Shadow`
/// members, and `<init>` initialiser properties.
fn inspect_mixin_class<'a>(
    class: &ClassFile,
    lookup: impl Fn(&str) -> Option<&'a ClassFile>,
) -> Option<MixinClassDetails> {
    let targets = class_targets(class);
    if targets.is_empty() {
        return None;
    }
    let is_interface = class_access(class).is_some_and(|access| access & ACC_INTERFACE != 0);
    let interfaces = class.interface_names();
    let (field_anns, method_anns) = member_annotations(class).unwrap_or_default();

    let mut added_fields = Vec::new();
    let mut added_instance_fields = BTreeSet::new();
    let mut shadow_fields = BTreeSet::new();
    let mut static_shadow_fields = BTreeMap::new();
    for field in &class.fields {
        let key = format!("{}|{}", field.name, field.descriptor);
        let anns = field_anns.get(&key);
        let is_shadow = has_annotation(anns, MIXIN_SHADOW_DESCRIPTOR);
        if is_shadow {
            let member = (field.name.clone(), field.descriptor.clone());
            shadow_fields.insert(member.clone());
            if field.is_static() {
                let is_final =
                    (field.access & ACC_FINAL != 0) || has_annotation(anns, MIXIN_FINAL_DESCRIPTOR);
                let is_mutable = has_annotation(anns, MIXIN_MUTABLE_DESCRIPTOR);
                static_shadow_fields.insert(
                    member,
                    StaticShadowField {
                        is_final,
                        is_mutable,
                    },
                );
            }
            continue;
        }
        if field.access & ACC_SYNTHETIC != 0 {
            continue;
        }
        added_fields.push(MethodRef {
            owner: class.name.clone(),
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
        });
        if !field.is_static() {
            added_instance_fields.insert((field.name.clone(), field.descriptor.clone()));
        }
    }
    added_fields.sort_by(|a, b| (&a.name, &a.descriptor).cmp(&(&b.name, &b.descriptor)));

    let mut added_methods = Vec::new();
    let mut added_public_methods = BTreeSet::new();
    let mut shadow_methods = BTreeSet::new();
    for method in &class.methods {
        if method.name == "<init>" || method.name == "<clinit>" {
            continue;
        }
        let key = format!("{}|{}", method.name, method.descriptor);
        let invis = method_anns.get(&key);
        let is_shadow = has_annotation(invis, MIXIN_SHADOW_DESCRIPTOR)
            || method
                .annotations
                .iter()
                .any(|a| a.descriptor == MIXIN_SHADOW_DESCRIPTOR);
        let is_accessor = resolve_mixin_accessor(class, method).is_some()
            || has_annotation(invis, crate::MIXIN_ACCESSOR_DESCRIPTOR)
            || has_annotation(invis, crate::MIXIN_INVOKER_DESCRIPTOR)
            || method.annotations.iter().any(|a| {
                a.descriptor == crate::MIXIN_ACCESSOR_DESCRIPTOR
                    || a.descriptor == crate::MIXIN_INVOKER_DESCRIPTOR
            });
        if is_shadow || (!is_accessor && method.access & ACC_ABSTRACT != 0) {
            shadow_methods.insert((method.name.clone(), method.descriptor.clone()));
            continue;
        }
        if method.is_static() || method.access & ACC_SYNTHETIC != 0 || is_accessor {
            continue;
        }
        let is_injector = method
            .annotations
            .iter()
            .any(|a| is_planned_method_annotation(&a.descriptor))
            || invis.is_some_and(|list| list.iter().any(|d| is_planned_method_annotation(d)));
        if is_injector {
            continue;
        }
        if method.code.is_some() {
            added_methods.push(MethodRef {
                owner: class.name.clone(),
                name: method.name.clone(),
                descriptor: method.descriptor.clone(),
            });
            if method.access & ACC_PRIVATE == 0 {
                added_public_methods.insert((method.name.clone(), method.descriptor.clone()));
            }
        }
    }
    added_methods.sort_by(|a, b| (&a.name, &a.descriptor).cmp(&(&b.name, &b.descriptor)));

    let has_initializer = class.super_name == "java/lang/Object"
        && class
            .method("<init>", "()V")
            .is_some_and(|m| m.code.is_some());
    let unrunnable_initializer = !has_initializer
        && !added_instance_fields.is_empty()
        && init_writes_added_field(class, &added_instance_fields);

    let mut declared_chain_fields = BTreeSet::new();
    let mut exit = class.super_name.clone();
    for field in &class.fields {
        declared_chain_fields.insert((field.name.clone(), field.descriptor.clone()));
    }
    for _ in 0..crate::MAX_SUPERCLASS_WALK {
        let Some(super_file) = lookup(&exit) else {
            break;
        };
        for field in &super_file.fields {
            declared_chain_fields.insert((field.name.clone(), field.descriptor.clone()));
        }
        if super_file.super_name.is_empty() || super_file.super_name == exit {
            exit.clear();
            break;
        }
        exit = super_file.super_name.clone();
    }
    let extends_game_class = lookup(&exit).is_none() && is_game_class(&exit);

    Some(MixinClassDetails {
        mixin: class.name.clone(),
        targets,
        is_interface,
        interfaces,
        added_fields,
        added_instance_fields,
        shadow_fields,
        static_shadow_fields,
        added_methods,
        added_public_methods,
        shadow_methods,
        has_initializer,
        unrunnable_initializer,
        extends_game_class,
        declared_chain_fields,
    })
}

/// Whether any constructor (`<init>`) of `class` executes `putfield` (`0xb5`) targeting
/// one of `added_instance_fields`. Used to refuse non-`Object` mixins whose field
/// initialisers live in a superclass-chaining constructor the runtime cannot run.
fn init_writes_added_field(
    class: &ClassFile,
    added_instance_fields: &BTreeSet<(String, String)>,
) -> bool {
    for method in &class.methods {
        if method.name != "<init>" {
            continue;
        }
        let Some(code) = method.code.as_deref() else {
            continue;
        };
        if bytecode_puts_field(class, code, added_instance_fields) {
            return true;
        }
    }
    false
}

/// Walk JVM bytecode instruction boundaries accurately and report whether any `putfield`
/// (`0xb5`) writes to `(class.name, name, desc)` in `added_instance_fields`.
fn bytecode_puts_field(
    class: &ClassFile,
    code: &[u8],
    added_instance_fields: &BTreeSet<(String, String)>,
) -> bool {
    let mut pc = 0usize;
    while pc < code.len() {
        let op = code[pc];
        if op == 0xb5 {
            if pc + 2 >= code.len() {
                return true;
            }
            let idx = u16::from_be_bytes([code[pc + 1], code[pc + 2]]);
            match class.field_ref(idx) {
                Ok(field) => {
                    if field.owner == class.name
                        && added_instance_fields.contains(&(field.name, field.descriptor))
                    {
                        return true;
                    }
                }
                Err(_) => return true,
            }
            pc += 3;
            continue;
        }
        let Some(next) = next_instruction_pc(code, pc) else {
            // Unrecognised or truncated opcode: assume conservative `true` so we never
            // silently drop an initialiser we could not inspect.
            return true;
        };
        pc = next;
    }
    false
}

/// Compute the next PC after the JVM instruction at `pc`, or `None` on malformed bytecode.
fn next_instruction_pc(code: &[u8], pc: usize) -> Option<usize> {
    let op = *code.get(pc)?;
    let step = match op {
        // 1-byte opcodes (no operands)
        0x00..=0x0f
        | 0x1a..=0x35
        | 0x3b..=0x83
        | 0x85..=0x98
        | 0xac..=0xb1
        | 0xbe
        | 0xbf
        | 0xc2
        | 0xc3 => 1,
        // 2-byte opcodes (1-byte operand)
        0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xa9 | 0xbc => 2,
        // 3-byte opcodes (2-byte operand)
        0x11
        | 0x13
        | 0x14
        | 0x84
        | 0x99..=0xa8
        | 0xb2..=0xb8
        | 0xbb
        | 0xbd
        | 0xc0
        | 0xc1
        | 0xc6
        | 0xc7 => 3,
        // multianewarray: 2-byte class index + 1-byte dimensions
        0xc5 => 4,
        // invokeinterface, invokedynamic, goto_w, jsr_w: 4-byte operand
        0xb9 | 0xba | 0xc8 | 0xc9 => 5,
        // wide
        0xc4 => {
            let sub = *code.get(pc + 1)?;
            if sub == 0x84 { 6 } else { 4 }
        }
        // tableswitch
        0xaa => {
            let mut cursor = (pc + 4) & !3;
            let low = i32::from_be_bytes(code.get(cursor + 4..cursor + 8)?.try_into().ok()?);
            let high = i32::from_be_bytes(code.get(cursor + 8..cursor + 12)?.try_into().ok()?);
            if high < low {
                return None;
            }
            let count = usize::try_from(i64::from(high) - i64::from(low) + 1).ok()?;
            cursor = cursor.checked_add(12)?.checked_add(count.checked_mul(4)?)?;
            return (cursor <= code.len()).then_some(cursor);
        }
        // lookupswitch
        0xab => {
            let mut cursor = (pc + 4) & !3;
            let npairs = i32::from_be_bytes(code.get(cursor + 4..cursor + 8)?.try_into().ok()?);
            if npairs < 0 {
                return None;
            }
            let count = usize::try_from(npairs).ok()?;
            cursor = cursor.checked_add(8)?.checked_add(count.checked_mul(8)?)?;
            return (cursor <= code.len()).then_some(cursor);
        }
        _ => return None,
    };
    let next = pc.checked_add(step)?;
    (next <= code.len()).then_some(next)
}

fn class_ref(owner: &str) -> MethodRef {
    MethodRef {
        owner: owner.to_owned(),
        name: String::new(),
        descriptor: String::new(),
    }
}

impl NativeJar {
    /// Every interface-injection `@Mixin` plan targeting a host-modelled game class that
    /// this jar's classes declare, in deterministic `(target, mixin)` order.
    pub fn mixin_interface_plans(&self) -> Vec<MixinInterfacePlan> {
        let mut plans = Vec::new();
        for class in self.classes.values() {
            let Some(details) = inspect_mixin_class(class, |name| self.classes.get(name)) else {
                continue;
            };
            if details.is_interface
                || details.interfaces.is_empty()
                || details.unrunnable_initializer
            {
                continue;
            }
            for target in &details.targets {
                if !is_host_modelled_game_target(target) {
                    continue;
                }
                plans.push(MixinInterfacePlan {
                    mixin: details.mixin.clone(),
                    target: target.clone(),
                    interfaces: details.interfaces.clone(),
                    added_fields: details.added_fields.clone(),
                    added_methods: details.added_methods.clone(),
                    has_initializer: details.has_initializer,
                });
            }
        }
        plans.sort_by(|a, b| (&a.target, &a.mixin).cmp(&(&b.target, &b.mixin)));
        plans
    }

    /// Plan interface-injection mixins for [`NativeJar::plan_load_time_weave`] /
    /// [`NativeJar::plan_load_time_weave_with_game_hooks`], recording applied plans in
    /// `weave.interface_mixins` and every refusal in `weave.skipped`.
    pub(crate) fn plan_interface_mixins(
        &self,
        side: MixinSide,
        configs: &[MixinConfig],
        config_sides: &BTreeMap<String, MixinSide>,
        plugins: Option<&ConfigPlugins>,
        weave: &mut LoadTimeMixinWeave,
    ) -> InterfaceWeaveSummary {
        let mut summary = InterfaceWeaveSummary::default();
        let mut candidates: Vec<(MixinInterfacePlan, BTreeSet<(String, String)>)> = Vec::new();

        for class in self.classes.values() {
            let Some(details) = inspect_mixin_class(class, |name| self.classes.get(name)) else {
                continue;
            };
            if details.is_interface || details.interfaces.is_empty() {
                continue;
            }
            let pure_interface_shape = details.shadow_fields.is_empty()
                && details.shadow_methods.is_empty()
                && !details.unrunnable_initializer;
            let mut recorded_any = false;
            for target in &details.targets {
                let gate = match plugins {
                    Some(plugins) => self.game_hook_config_gate(
                        configs,
                        config_sides,
                        plugins,
                        &details.mixin,
                        target,
                        side,
                    ),
                    None => mixin_config_gate(configs, config_sides, &details.mixin, side),
                };
                if let Err(reason) = gate {
                    weave.skipped.push(SkippedMixinPlan {
                        target: class_ref(target),
                        callback: class_ref(&details.mixin),
                        at: "@Mixin(implements)".to_owned(),
                        reason,
                    });
                    recorded_any = true;
                    continue;
                }
                if !is_host_modelled_game_target(target) {
                    let reason = if self.classes.contains_key(target) {
                        MixinSkipReason::UnsupportedFamily
                    } else {
                        MixinSkipReason::TargetOutsideJar
                    };
                    weave.skipped.push(SkippedMixinPlan {
                        target: class_ref(target),
                        callback: class_ref(&details.mixin),
                        at: "@Mixin(implements)".to_owned(),
                        reason,
                    });
                    recorded_any = true;
                    continue;
                }
                if details.unrunnable_initializer {
                    weave.skipped.push(SkippedMixinPlan {
                        target: class_ref(target),
                        callback: class_ref(&details.mixin),
                        at: "@Mixin(implements)".to_owned(),
                        reason: MixinSkipReason::UnsupportedFamily,
                    });
                    recorded_any = true;
                    continue;
                }
                candidates.push((
                    MixinInterfacePlan {
                        mixin: details.mixin.clone(),
                        target: target.clone(),
                        interfaces: details.interfaces.clone(),
                        added_fields: details.added_fields.clone(),
                        added_methods: details.added_methods.clone(),
                        has_initializer: details.has_initializer,
                    },
                    details.added_public_methods.clone(),
                ));
                recorded_any = true;
            }
            if recorded_any && pure_interface_shape {
                summary.accounted_mixins.insert(details.mixin);
            }
        }

        // Refuse ambiguous interface-injection mixins that declare the same non-private
        // method on the same target class.
        let mut method_owners: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
        for (plan, public_methods) in &candidates {
            for (name, desc) in public_methods {
                let owners = method_owners
                    .entry((plan.target.clone(), name.clone(), desc.clone()))
                    .or_default();
                if !owners.contains(&plan.mixin) {
                    owners.push(plan.mixin.clone());
                }
            }
        }
        let mut ambiguous: BTreeSet<(String, String)> = BTreeSet::new();
        for ((target, name, desc), owners) in &method_owners {
            if owners.len() > 1 {
                for mixin in owners {
                    ambiguous.insert((mixin.clone(), target.clone()));
                    weave.skipped.push(SkippedMixinPlan {
                        target: MethodRef {
                            owner: target.clone(),
                            name: name.clone(),
                            descriptor: desc.clone(),
                        },
                        callback: MethodRef {
                            owner: mixin.clone(),
                            name: name.clone(),
                            descriptor: desc.clone(),
                        },
                        at: "@Mixin(implements)".to_owned(),
                        reason: MixinSkipReason::AmbiguousTarget,
                    });
                }
            }
        }

        for (plan, _) in candidates {
            if ambiguous.contains(&(plan.mixin.clone(), plan.target.clone())) {
                continue;
            }
            summary.applied_mixins.insert(plan.mixin.clone());
            weave.interface_mixins.push(plan);
        }
        weave
            .interface_mixins
            .sort_by(|a, b| (&a.target, &a.mixin).cmp(&(&b.target, &b.mixin)));
        summary
    }

    /// The first unwoven member of `@Mixin` `class`, ignoring members already handled by
    /// applied interface-injection plans (or by an accounted interface-mixin skip row).
    pub(crate) fn unwoven_member_for_class(
        &self,
        class: &ClassFile,
        summary: &InterfaceWeaveSummary,
    ) -> Option<(String, String)> {
        if summary.accounted_mixins.contains(&class.name) {
            return None;
        }
        let is_applied = summary.applied_mixins.contains(&class.name);
        let (field_anns, method_anns) = if is_applied {
            member_annotations(class).unwrap_or_default()
        } else {
            Default::default()
        };

        let field = class
            .fields
            .iter()
            .find(|field| {
                if field.access & ACC_SYNTHETIC != 0 {
                    return false;
                }
                if !is_applied {
                    return true;
                }
                let key = format!("{}|{}", field.name, field.descriptor);
                has_annotation(field_anns.get(&key), MIXIN_SHADOW_DESCRIPTOR)
            })
            .map(|field| (field.name.clone(), field.descriptor.clone()));

        let method = class
            .methods
            .iter()
            .find(|method| {
                if method.is_static()
                    || method.name == "<init>"
                    || method.access & ACC_SYNTHETIC != 0
                {
                    return false;
                }
                if method
                    .annotations
                    .iter()
                    .any(|annotation| is_planned_method_annotation(&annotation.descriptor))
                {
                    return false;
                }
                if resolve_mixin_accessor(class, method).is_some() {
                    return false;
                }
                if !is_applied {
                    return true;
                }
                let key = format!("{}|{}", method.name, method.descriptor);
                let invis = method_anns.get(&key);
                if invis.is_some_and(|list| list.iter().any(|d| is_planned_method_annotation(d))) {
                    return false;
                }
                let is_shadow = has_annotation(invis, MIXIN_SHADOW_DESCRIPTOR)
                    || method
                        .annotations
                        .iter()
                        .any(|a| a.descriptor == MIXIN_SHADOW_DESCRIPTOR);
                is_shadow || method.code.is_none() || method.access & ACC_ABSTRACT != 0
            })
            .map(|method| (method.name.clone(), method.descriptor.clone()));

        field.or(method)
    }
}

impl<H: Host> Runtime<'_, H> {
    /// Build or fetch the per-runtime index of active `@Mixin` classes across the primary
    /// jar and all linked jars.
    fn runtime_mixin_index(&self) -> &RuntimeMixinIndex {
        self.mixin_index.index.get_or_init(|| {
            let mut index = RuntimeMixinIndex::default();
            let mut seen_mixins = BTreeSet::new();

            // 1. Primary jar classes.
            let primary_has_no_configs = self.mixin_index.configs.is_empty();
            for class in self.classes.values() {
                if !seen_mixins.insert(class.name.clone()) {
                    continue;
                }
                let reg = MixinRegistration {
                    is_primary: true,
                    primary_has_no_configs,
                };
                index.register_class(
                    class,
                    reg,
                    |target| self.mixin_index.allows_config_gated(&class.name, target),
                    |target| self.classes.contains_key(target),
                    |name| self.class(name),
                );
            }

            // 2. Linked jar classes (gated by each linked jar's own mixin configs and plugins).
            if let Some(side) = self.mixin_index.side {
                for set in self.linked.mixin_config_sets() {
                    let plugins = match side {
                        MixinSide::Server => &set.server_plugins,
                        MixinSide::Client => &set.client_plugins,
                    };
                    for class_name in &set.classes {
                        if !seen_mixins.insert(class_name.clone()) {
                            continue;
                        }
                        let Some(class) = self.class(class_name) else {
                            continue;
                        };
                        let reg = MixinRegistration {
                            is_primary: false,
                            primary_has_no_configs: false,
                        };
                        index.register_class(
                            class,
                            reg,
                            |target| {
                                mixin_config_gate_with_plugins(
                                    &set.configs,
                                    &set.config_sides,
                                    plugins,
                                    class_name,
                                    target,
                                    side,
                                )
                                .is_ok()
                            },
                            |target| self.classes.contains_key(target),
                            |name| self.class(name),
                        );
                    }
                }
            }

            // Also record non-config-listed linked mixin classes' shadow metadata so
            // calling/reading them cannot silently bypass `@Shadow` checks.
            for (class_name, class) in self.linked.all_classes() {
                if !seen_mixins.insert(class_name.clone()) {
                    continue;
                }
                if mixin_annotations(class).is_empty() {
                    continue;
                }
                let reg = MixinRegistration {
                    is_primary: false,
                    primary_has_no_configs: false,
                };
                let unconfigured = !self.mixin_index.require_config;
                index.register_class(
                    class,
                    reg,
                    |_target| unconfigured,
                    |target| self.classes.contains_key(target),
                    |name| self.class(name),
                );
            }

            index
        })
    }

    /// Active `@Mixin` classes targeting `class` (internal name), across the primary jar
    /// and linked jars, respecting config gating.
    pub(crate) fn mixins_targeting(&self, class: &str) -> &[String] {
        self.runtime_mixin_index()
            .by_target
            .get(class)
            .map_or(&[], Vec::as_slice)
    }

    /// Declared vanilla classes of a game handle (using host override when running under
    /// `GameHookHost`, falling back to `game_object::declared_types`).
    fn game_handle_types(&self, key: &str) -> Vec<String> {
        if let Some(types) = self.host.game_object_types(key) {
            return types;
        }
        crate::game_object::declared_types(key)
            .iter()
            .map(|ty| (*ty).to_owned())
            .collect()
    }

    /// Run a mixin's no-argument `<init>()V` field initialiser once per `(mixin, handle_key)`
    /// pair before its first method invocation or field access on that game handle.
    pub(crate) fn ensure_game_mixin_initialized(
        &mut self,
        mixin: &str,
        receiver: &Value,
    ) -> Result<(), NativeError> {
        let Some(key) = crate::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(());
        };
        let has_initializer = self
            .runtime_mixin_index()
            .meta
            .get(mixin)
            .is_some_and(|meta| meta.has_initializer);
        if !has_initializer {
            return Ok(());
        }
        if !self.host.mark_game_mixin_initialized(mixin, &key) {
            return Ok(());
        }
        self.invoke(mixin, "<init>", "()V", Some(receiver.clone()), &[])?;
        Ok(())
    }

    /// Dispatch a method call on a game handle through an active interface-injection Mixin
    /// (or redirect a `@Shadow` method on a Mixin class to its target on the host).
    ///
    /// Returns `Ok(None)` when the call does not target an injected interface or a Mixin
    /// class for this receiver.
    pub(crate) fn invoke_interface_mixin(
        &mut self,
        r: &MethodRef,
        op: u8,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let Some(handle_key) = crate::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(None);
        };
        if r.owner == "java/lang/Object" || r.name == "<init>" {
            return Ok(None);
        }

        let declared = self.game_handle_types(&handle_key);

        // 1. Direct call on a `@Mixin` class (e.g. `this.helper()` or `@Shadow` call).
        let direct_action = {
            let index = self.runtime_mixin_index();
            index.meta.get(&r.owner).map(|meta| {
                let is_shadow = meta
                    .shadow_methods
                    .contains(&(r.name.clone(), r.descriptor.clone()));
                let shadow_target = if is_shadow {
                    resolve_shadow_target(&meta.targets, &declared).map(str::to_owned)
                } else {
                    None
                };
                let active_on_handle = declared
                    .iter()
                    .any(|ty| meta.active_interface_targets.contains(ty.as_str()));
                (is_shadow, shadow_target, active_on_handle)
            })
        };
        if let Some((is_shadow, shadow_target, active_on_handle)) = direct_action {
            if is_shadow {
                return match shadow_target {
                    Some(owner) => {
                        let target_ref = MethodRef {
                            owner,
                            name: r.name.clone(),
                            descriptor: r.descriptor.clone(),
                        };
                        self.host
                            .invoke_instance(&target_ref, receiver, args)
                            .map(Some)
                    }
                    // Ambiguous multi-target `@Shadow`: count/refuse loudly rather than
                    // guessing `targets.first()`.
                    None => {
                        self.host.unknown_operand_call(r)?;
                        Ok(Some(Value::default_for(return_desc(&r.descriptor))))
                    }
                };
            }
            if active_on_handle
                && self
                    .class(&r.owner)
                    .and_then(|file| file.method(&r.name, &r.descriptor))
                    .is_some_and(|m| !m.is_static() && m.code.is_some())
            {
                self.ensure_game_mixin_initialized(&r.owner, receiver)?;
                return self
                    .invoke(
                        &r.owner,
                        &r.name,
                        &r.descriptor,
                        Some(receiver.clone()),
                        args,
                    )
                    .map(Some);
            }
            return Ok(None);
        }

        // Non-virtual `invokespecial` on an interface (`I.super.m()`) runs that interface's
        // concrete default directly without overriding back into the mixin.
        if op == 0xb7 {
            if self
                .class(&r.owner)
                .and_then(|file| file.method(&r.name, &r.descriptor))
                .is_some_and(|m| !m.is_static() && m.code.is_some())
            {
                return Ok(None);
            }
            return Ok(None);
        }

        // 2. Virtual/interface call targeting an interface added by an active interface-injection
        //    mixin for one of this handle's declared vanilla types.
        enum Resolution {
            None,
            Mixin(String),
            Ambiguous,
            UnknownTargetHierarchy,
            TargetClassMethod(String),
            InterfaceDefault(String),
            DefaultConflict(Vec<String>),
            UnimplementedInterface,
        }

        let resolution = {
            let index = self.runtime_mixin_index();
            let mut matching_mixins_by_level: Vec<Vec<String>> = Vec::new();
            let mut any_matching_mixin = false;
            let mut all_active_mixins: Vec<String> = Vec::new();

            for target_type in &declared {
                let Some(mixins) = index.interface_by_target.get(target_type.as_str()) else {
                    continue;
                };
                let mut level_impls = Vec::new();
                for active in mixins {
                    if !all_active_mixins.contains(&active.mixin) {
                        all_active_mixins.push(active.mixin.clone());
                    }
                    let implements_owner = active
                        .interfaces
                        .iter()
                        .any(|iface| iface == &r.owner || self.interface_extends(iface, &r.owner));
                    if implements_owner {
                        any_matching_mixin = true;
                    }
                    if active
                        .added_public_methods
                        .contains(&(r.name.clone(), r.descriptor.clone()))
                        && !level_impls.contains(&active.mixin)
                    {
                        level_impls.push(active.mixin.clone());
                    }
                }
                if !level_impls.is_empty() {
                    matching_mixins_by_level.push(level_impls);
                }
            }

            if !any_matching_mixin {
                Resolution::None
            } else if let Some(top_level) = matching_mixins_by_level.into_iter().next() {
                if top_level.len() > 1 {
                    Resolution::Ambiguous
                } else {
                    Resolution::Mixin(top_level.into_iter().next().unwrap_or_default())
                }
            } else {
                // JVMS §5.4.3.3 / §5.4.3.4 ("classes win over interfaces" + maximally-specific
                // superinterface defaults):
                // 1. Any method declared by the target's vanilla/JDK superclass chain wins over
                //    every interface default and dispatches through the host.
                // 2. Any missing edge in the target's hierarchy fails closed (counted/refused).
                // 3. Otherwise combine the target's vanilla/JDK interface defaults with all
                //    injected interfaces' declarations: two unrelated defaults conflict with
                //    `IncompatibleClassChangeError`.
                let table = vanilla_host_methods();
                match table.check_target_methods(&declared, &r.name, &r.descriptor) {
                    VanillaMethodCheck::ClassMethod(target_owner) => {
                        Resolution::TargetClassMethod(target_owner.to_owned())
                    }
                    VanillaMethodCheck::MissingHierarchyEdge(_) => {
                        Resolution::UnknownTargetHierarchy
                    }
                    VanillaMethodCheck::VanillaDefaults(vanilla_defaults) => {
                        // Collect all non-static, non-private declarations across the full
                        // superinterface closure of all active mixins on this target.
                        let mut queue = VecDeque::new();
                        for mixin in &all_active_mixins {
                            if let Some(file) = self.class(mixin) {
                                queue.extend(file.interface_names());
                            }
                        }
                        let mut seen_ifaces = Vec::new();
                        // (owner, has_body, is_vanilla_host_default)
                        let mut all_declaring: Vec<(String, bool, bool)> = vanilla_defaults
                            .into_iter()
                            .map(|owner| (owner.to_owned(), true, true))
                            .collect();
                        while let Some(iface) = queue.pop_front() {
                            if seen_ifaces.len() >= crate::MAX_SUPERCLASS_WALK {
                                break;
                            }
                            if seen_ifaces.contains(&iface) {
                                continue;
                            }
                            if let Some(file) = self.class(&iface) {
                                queue.extend(file.interface_names());
                                if let Some(m) = file.method(&r.name, &r.descriptor)
                                    && !m.is_static()
                                    && m.access & ACC_PRIVATE == 0
                                    && !all_declaring.iter().any(|(o, _, _)| o == &iface)
                                {
                                    all_declaring.push((iface.clone(), m.code.is_some(), false));
                                }
                                seen_ifaces.push(iface);
                            } else if let Some(info) = table.interfaces.get(iface.as_str()) {
                                queue.extend(info.interfaces.iter().map(|s| (*s).to_owned()));
                                if info
                                    .methods
                                    .contains(&(r.name.as_str(), r.descriptor.as_str()))
                                    && !all_declaring.iter().any(|(o, _, _)| o == &iface)
                                {
                                    all_declaring.push((iface.clone(), true, true));
                                }
                                seen_ifaces.push(iface);
                            }
                        }
                        let maximal: Vec<&(String, bool, bool)> = all_declaring
                            .iter()
                            .filter(|(iface, _, _)| {
                                !all_declaring.iter().any(|(other, _, _)| {
                                    other != iface
                                        && iface_extends_with_vanilla(self, table, other, iface)
                                })
                            })
                            .collect();
                        let concrete: Vec<&(String, bool, bool)> = maximal
                            .into_iter()
                            .filter(|(_, has_body, _)| *has_body)
                            .collect();
                        match concrete.as_slice() {
                            [] => Resolution::UnimplementedInterface,
                            [(owner, _, true)] => Resolution::TargetClassMethod(owner.clone()),
                            [(owner, _, false)] => Resolution::InterfaceDefault(owner.clone()),
                            many => Resolution::DefaultConflict(
                                many.iter().map(|(owner, _, _)| owner.clone()).collect(),
                            ),
                        }
                    }
                }
            }
        };

        match resolution {
            Resolution::None => Ok(None),
            Resolution::Mixin(mixin) => {
                self.ensure_game_mixin_initialized(&mixin, receiver)?;
                self.invoke(&mixin, &r.name, &r.descriptor, Some(receiver.clone()), args)
                    .map(Some)
            }
            Resolution::Ambiguous | Resolution::UnknownTargetHierarchy => {
                self.host.unknown_operand_call(r)?;
                Ok(Some(Value::default_for(return_desc(&r.descriptor))))
            }
            Resolution::TargetClassMethod(owner) => {
                let target_ref = MethodRef {
                    owner,
                    name: r.name.clone(),
                    descriptor: r.descriptor.clone(),
                };
                self.host
                    .invoke_instance(&target_ref, receiver, args)
                    .map(Some)
            }
            Resolution::InterfaceDefault(iface) => self
                .invoke(&iface, &r.name, &r.descriptor, Some(receiver.clone()), args)
                .map(Some),
            Resolution::DefaultConflict(owners) => Err(NativeError::UncaughtException {
                class: "java/lang/IncompatibleClassChangeError".into(),
                message: Some(format!(
                    "Conflicting default methods: {}",
                    owners
                        .iter()
                        .map(|owner| format!("{owner}.{}", r.name))
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                frame: None,
            }),
            Resolution::UnimplementedInterface => {
                self.host.invoke_instance(r, receiver, args).map(Some)
            }
        }
    }

    /// Resolve a `getstatic`/`putstatic` targeting a `@Mixin` class's static `@Shadow`
    /// field (or a static field inherited from a game superclass) to its target owner and
    /// Mixin `@Final`/`@Mutable` modifiers, or report ambiguity for multi-target mixins.
    #[inline(never)]
    pub(crate) fn resolve_static_shadow(&self, field: &MethodRef) -> StaticShadowResolution {
        let lookup = {
            let index = self.runtime_mixin_index();
            let Some(meta) = index.meta.get(&field.owner) else {
                return StaticShadowResolution::NotShadow;
            };
            let member = (field.name.clone(), field.descriptor.clone());
            if let Some(&info) = meta.static_shadow_fields.get(&member) {
                let target_owner = resolve_shadow_target(&meta.targets, &[]).map(str::to_owned);
                let target_name = field
                    .name
                    .strip_prefix("shadow$")
                    .unwrap_or(&field.name)
                    .to_owned();
                Some((target_owner, target_name, info.is_final, info.is_mutable))
            } else if meta.extends_game_class && !meta.declared_chain_fields.contains(&member) {
                let target_owner = resolve_shadow_target(&meta.targets, &[]).map(str::to_owned);
                Some((target_owner, field.name.clone(), false, false))
            } else {
                None
            }
        };
        let Some((target_owner, target_name, shadow_final, is_mutable)) = lookup else {
            return StaticShadowResolution::NotShadow;
        };
        let Some(owner) = target_owner else {
            return StaticShadowResolution::Ambiguous;
        };
        let raw_target = MethodRef {
            owner,
            name: target_name,
            descriptor: field.descriptor.clone(),
        };
        let resolved_owner = self.field_owner(&raw_target);
        let target_decl_final = self
            .class(&resolved_owner)
            .and_then(|file| {
                file.fields
                    .iter()
                    .find(|fl| fl.name == raw_target.name && fl.descriptor == raw_target.descriptor)
            })
            .is_some_and(|fl| fl.access & ACC_FINAL != 0);
        StaticShadowResolution::Resolved {
            target: MethodRef {
                owner: resolved_owner,
                ..raw_target
            },
            is_final: shadow_final || target_decl_final,
            is_mutable,
        }
    }

    /// Handle `getfield` on a game handle for `@Shadow` fields (routing to the host so
    /// unmodelled shadows are counted/refused) and for added fields of active
    /// interface-injection mixins (ensuring `<init>()V` ran and returning Java defaults
    /// without counting an unknown host call).
    pub(crate) fn game_handle_get_field(
        &mut self,
        receiver: &Value,
        r: &MethodRef,
    ) -> Result<Option<Value>, NativeError> {
        let Some(handle_key) = crate::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(None);
        };
        let declared = self.game_handle_types(&handle_key);
        enum Action {
            None,
            Shadow(Option<MethodRef>),
            AddedMixinField,
        }
        let action = {
            let index = self.runtime_mixin_index();
            match index.meta.get(&r.owner) {
                Some(meta) => {
                    let member = (r.name.clone(), r.descriptor.clone());
                    if meta.shadow_fields.contains(&member)
                        || (meta.extends_game_class
                            && !meta.declared_chain_fields.contains(&member))
                    {
                        let target_ref =
                            resolve_shadow_target(&meta.targets, &declared).map(|owner| {
                                MethodRef {
                                    owner: owner.to_owned(),
                                    name: r.name.clone(),
                                    descriptor: r.descriptor.clone(),
                                }
                            });
                        Action::Shadow(target_ref)
                    } else if meta.added_instance_fields.contains(&member)
                        && declared
                            .iter()
                            .any(|ty| meta.active_interface_targets.contains(ty.as_str()))
                    {
                        Action::AddedMixinField
                    } else {
                        Action::None
                    }
                }
                None => Action::None,
            }
        };
        match action {
            Action::None => Ok(None),
            Action::Shadow(Some(target_ref)) => {
                // Under `GameHookHost`, pass the original mixin ref so its `@Shadow` error
                // diagnostic names both the mixin field and `@Shadow`.
                let query = if self.host.defers_game_field_write_check() {
                    r
                } else {
                    &target_ref
                };
                self.host.get_field(receiver, query).map(Some)
            }
            Action::Shadow(None) => {
                // Ambiguous multi-target `@Shadow`: count/refuse loudly.
                if self.host.defers_game_field_write_check() {
                    self.host.get_field(receiver, r).map(Some)
                } else {
                    self.host.unknown_operand_call(r)?;
                    Ok(Some(Value::default_for(&r.descriptor)))
                }
            }
            Action::AddedMixinField => {
                self.ensure_game_mixin_initialized(&r.owner, receiver)?;
                let value = instance_field_key(receiver, r)
                    .and_then(|key| self.fields.get(&key).cloned())
                    .unwrap_or_else(|| Value::default_for(&r.descriptor));
                Ok(Some(value))
            }
        }
    }

    /// Handle `putfield` on a game handle for added fields of active interface-injection
    /// mixins (ensuring `<init>()V` ran first) and for unmodelled `@Shadow` / game-class
    /// field writes (counted/refused loudly instead of silently faking a host mutation).
    pub(crate) fn game_handle_put_field(
        &mut self,
        receiver: &Value,
        r: &MethodRef,
        value: &Value,
    ) -> Result<GameFieldWrite, NativeError> {
        let Some(handle_key) = crate::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(GameFieldWrite::Fallthrough);
        };
        let declared = self.game_handle_types(&handle_key);
        enum Action {
            Fallthrough,
            AddedMixinField,
            UnmodelledGameField(MethodRef),
        }
        let action = {
            let index = self.runtime_mixin_index();
            match index.meta.get(&r.owner) {
                Some(meta) => {
                    let member = (r.name.clone(), r.descriptor.clone());
                    let is_shadow_or_inherited_game = meta.shadow_fields.contains(&member)
                        || (meta.extends_game_class
                            && !meta.declared_chain_fields.contains(&member));
                    if is_shadow_or_inherited_game {
                        if self.host.defers_game_field_write_check() {
                            Action::Fallthrough
                        } else {
                            let target_owner = resolve_shadow_target(&meta.targets, &declared)
                                .unwrap_or(r.owner.as_str())
                                .to_owned();
                            Action::UnmodelledGameField(MethodRef {
                                owner: target_owner,
                                name: r.name.clone(),
                                descriptor: r.descriptor.clone(),
                            })
                        }
                    } else if meta.added_instance_fields.contains(&member)
                        && declared
                            .iter()
                            .any(|ty| meta.active_interface_targets.contains(ty.as_str()))
                    {
                        Action::AddedMixinField
                    } else {
                        Action::Fallthrough
                    }
                }
                None if !self.has_class(&r.owner)
                    && is_game_class(&r.owner)
                    && !self.host.defers_game_field_write_check() =>
                {
                    Action::UnmodelledGameField(r.clone())
                }
                None => Action::Fallthrough,
            }
        };
        match action {
            Action::Fallthrough => Ok(GameFieldWrite::Fallthrough),
            Action::AddedMixinField => {
                self.ensure_game_mixin_initialized(&r.owner, receiver)?;
                if let Some(key) = instance_field_key(receiver, r) {
                    self.fields.insert(key, value.clone());
                }
                Ok(GameFieldWrite::Stored)
            }
            Action::UnmodelledGameField(target_ref) => {
                self.host.unknown_operand_call(&target_ref)?;
                Ok(GameFieldWrite::Handled)
            }
        }
    }
}

impl RuntimeMixinIndex {
    fn register_class<'a>(
        &mut self,
        class: &ClassFile,
        reg: MixinRegistration,
        config_active_for_target: impl Fn(&str) -> bool,
        is_jar_local: impl Fn(&str) -> bool,
        lookup: impl Fn(&str) -> Option<&'a ClassFile>,
    ) {
        let Some(details) = inspect_mixin_class(class, lookup) else {
            return;
        };
        let is_interface_injection = !details.is_interface && !details.interfaces.is_empty();
        let mut active_interface_targets = BTreeSet::new();
        for target in &details.targets {
            let is_external_host_game_target =
                !is_jar_local(target) && is_host_modelled_game_target(target);
            let config_active = config_active_for_target(target);
            let (include_in_by_target, include_in_interface_dispatch) = if reg.is_primary {
                if is_external_host_game_target && is_interface_injection {
                    // Host-game-object interface-injection Mixins are always strictly
                    // config-, side-, and plugin-gated and require a runnable no-arg initialiser.
                    let active = config_active && !details.unrunnable_initializer;
                    (active, active)
                } else if is_external_host_game_target {
                    // Accessor/invoker interfaces on host-modelled game targets keep the
                    // config-less probe scan when the jar has no Mixin configs, and obey
                    // the config/plugin gate when it does.
                    (reg.primary_has_no_configs || config_active, false)
                } else {
                    // Jar-local woven classes (e.g. `demo/Woven$Target`) and non-host-game
                    // targets (e.g. `ArrayBackedEvent`) preserve the existing unconditional
                    // primary-jar `by_target` scan and never enter host-object dispatch.
                    (true, false)
                }
            } else {
                // Linked-jar Mixins are always config- and plugin-gated.
                let active = config_active && !details.unrunnable_initializer;
                (
                    active,
                    active && is_external_host_game_target && is_interface_injection,
                )
            };

            if include_in_by_target {
                let mixins = self.by_target.entry(target.clone()).or_default();
                if !mixins.contains(&details.mixin) {
                    mixins.push(details.mixin.clone());
                }
            }
            if include_in_interface_dispatch {
                active_interface_targets.insert(target.clone());
                self.interface_by_target
                    .entry(target.clone())
                    .or_default()
                    .push(ActiveInterfaceMixin {
                        mixin: details.mixin.clone(),
                        interfaces: details.interfaces.clone(),
                        added_public_methods: details.added_public_methods.clone(),
                    });
            }
        }
        self.meta.insert(
            details.mixin,
            MixinClassMeta {
                targets: details.targets,
                active_interface_targets,
                added_instance_fields: details.added_instance_fields,
                shadow_fields: details.shadow_fields,
                static_shadow_fields: details.static_shadow_fields,
                shadow_methods: details.shadow_methods,
                has_initializer: details.has_initializer,
                extends_game_class: details.extends_game_class,
                declared_chain_fields: details.declared_chain_fields,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_modelled_game_targets_are_strictly_sorted() {
        assert!(
            HOST_MODELLED_GAME_TARGETS.windows(2).all(|w| w[0] < w[1]),
            "HOST_MODELLED_GAME_TARGETS must stay sorted for binary_search"
        );
        assert!(is_host_modelled_game_target(
            "net/minecraft/world/entity/player/Player"
        ));
        assert!(is_host_modelled_game_target(
            "net/minecraft/server/level/ServerPlayer"
        ));
        assert!(!is_host_modelled_game_target(
            "net/minecraft/world/level/chunk/LevelChunk"
        ));
    }

    #[test]
    fn vanilla_host_methods_table_is_hierarchy_closed_and_includes_jdk_defaults() {
        let table = vanilla_host_methods();
        for target in HOST_MODELLED_GAME_TARGETS {
            assert!(
                table.classes.contains_key(target) || table.interfaces.contains_key(target),
                "HOST_MODELLED_GAME_TARGETS entry {target} missing from vanilla_host_methods.tsv"
            );
        }
        for (cname, info) in &table.classes {
            if !info.super_name.is_empty() && info.super_name != "java/lang/Object" {
                assert!(
                    table.classes.contains_key(info.super_name),
                    "class {cname} superclass {} missing from vanilla_host_methods.tsv",
                    info.super_name
                );
            }
            for &iface in &info.interfaces {
                assert!(
                    table.interfaces.contains_key(iface),
                    "class {cname} interface {iface} missing from vanilla_host_methods.tsv"
                );
            }
        }
        for (iname, info) in &table.interfaces {
            for &parent in &info.interfaces {
                assert!(
                    table.interfaces.contains_key(parent),
                    "interface {iname} superinterface {parent} missing from vanilla_host_methods.tsv"
                );
            }
        }
        let inv = vec!["net/minecraft/world/entity/player/Inventory".to_owned()];
        assert_eq!(
            table.check_target_methods(&inv, "forEach", "(Ljava/util/function/Consumer;)V"),
            VanillaMethodCheck::VanillaDefaults(vec!["java/lang/Iterable"]),
        );
        assert_eq!(
            table.check_target_methods(&inv, "spliterator", "()Ljava/util/Spliterator;"),
            VanillaMethodCheck::VanillaDefaults(vec!["java/lang/Iterable"]),
        );
        let player = vec![
            "net/minecraft/server/level/ServerPlayer".to_owned(),
            "net/minecraft/world/entity/player/Player".to_owned(),
        ];
        assert_eq!(
            table.check_target_methods(&player, "getId", "()I"),
            VanillaMethodCheck::ClassMethod("net/minecraft/world/entity/Entity"),
        );
        assert_eq!(
            table.check_target_methods(&player, "getScore", "()I"),
            VanillaMethodCheck::ClassMethod("net/minecraft/world/entity/player/Player"),
        );
    }

    #[test]
    fn check_target_methods_fails_closed_on_missing_hierarchy_edge() {
        let synthetic = VanillaHostMethods::parse(
            "C\tdemo/MissingSuper\tdemo/NotInTable\t\n\
             C\tdemo/HasIface\tjava/lang/Object\tdemo/IfaceWithMissingParent\n\
             I\tdemo/IfaceWithMissingParent\t\tdemo/MissingParent\n",
        );
        assert_eq!(
            synthetic.check_target_methods(&["demo/MissingSuper".to_owned()], "foo", "()V"),
            VanillaMethodCheck::MissingHierarchyEdge("demo/NotInTable".to_owned()),
        );
        assert_eq!(
            synthetic.check_target_methods(&["demo/HasIface".to_owned()], "foo", "()V"),
            VanillaMethodCheck::MissingHierarchyEdge("demo/MissingParent".to_owned()),
        );
        assert_eq!(
            synthetic.check_target_methods(&["demo/UnknownStart".to_owned()], "foo", "()V"),
            VanillaMethodCheck::MissingHierarchyEdge("demo/UnknownStart".to_owned()),
        );
    }
}

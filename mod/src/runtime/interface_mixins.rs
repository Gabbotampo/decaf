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
use std::collections::{BTreeMap, BTreeSet};

use crate::runtime::checkcast::class_access;
use crate::runtime::game_hooks::{MIXIN_SHADOW_DESCRIPTOR, member_annotations};
use crate::runtime::jvm_dispatch::DefaultSelection;
use crate::runtime::mixin_plugins::ConfigPlugins;
use crate::runtime::{
    Annotation, ClassFile, Host, LoadTimeMixinWeave, MethodRef, MixinConfig, MixinExtrasKind,
    MixinSide, MixinSkipReason, NativeError, NativeJar, Runtime, SkippedMixinPlan, Value,
    instance_field_key, mixin_config_gate, mixin_target_owners, resolve_mixin_accessor,
    return_desc,
};

const ACC_PRIVATE: u16 = 0x0002;
const ACC_INTERFACE: u16 = 0x0200;
const ACC_ABSTRACT: u16 = 0x0400;
const ACC_SYNTHETIC: u16 = 0x1000;

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

/// Per-`Runtime` mixin configuration gate and lazy index.
#[derive(Debug, Default)]
pub(crate) struct RuntimeMixinState {
    configs: Vec<MixinConfig>,
    config_sides: BTreeMap<String, MixinSide>,
    side: Option<MixinSide>,
    require_config: bool,
    index: OnceCell<RuntimeMixinIndex>,
}

impl RuntimeMixinState {
    /// Build state for a real `NativeJar` invocation, honouring the jar's mixin configs
    /// and the host's physical side.
    pub(crate) fn from_jar<H: Host>(jar: &NativeJar, host: &H) -> Self {
        Self::from_jar_side(jar, host.mixin_side())
    }

    /// Build state for a real `NativeJar` invocation pinned to an explicit physical side.
    pub(crate) fn from_jar_side(jar: &NativeJar, side: MixinSide) -> Self {
        Self {
            configs: jar.mixin_configs(),
            config_sides: jar.mixin_config_sides(),
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
            side: None,
            require_config: false,
            index: OnceCell::new(),
        }
    }

    /// Whether `mixin` passes the primary jar's Mixin config and physical-side gate.
    ///
    /// Returns `true` when config gating is disabled (`unconfigured()` in synthetic unit
    /// tests) or when the jar has readable Mixin configs that list `mixin` for the active
    /// side. A jar with no readable Mixin configs never passes this gate.
    fn allows_config_gated(&self, mixin: &str) -> bool {
        if !self.require_config {
            return true;
        }
        if self.configs.is_empty() {
            return false;
        }
        let Some(side) = self.side else {
            return false;
        };
        mixin_config_gate(&self.configs, &self.config_sides, mixin, side).is_ok()
    }
}

#[derive(Clone, Copy)]
struct MixinRegistration {
    /// True for primary-jar classes (preserving the existing jar-local and non-host-game
    /// `by_target` scan), false for linked-jar classes.
    is_primary: bool,
    /// True when the mixin passes config & side gating (or config gating is disabled).
    config_active: bool,
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
        .filter(|a| a.descriptor == crate::runtime::MIXIN_ANNOTATION_DESCRIPTOR)
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
        || descriptor == crate::runtime::MIXIN_ACCESSOR_DESCRIPTOR
        || descriptor == crate::runtime::MIXIN_INVOKER_DESCRIPTOR
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
    for field in &class.fields {
        let key = format!("{}|{}", field.name, field.descriptor);
        let is_shadow = has_annotation(field_anns.get(&key), MIXIN_SHADOW_DESCRIPTOR);
        if is_shadow {
            shadow_fields.insert((field.name.clone(), field.descriptor.clone()));
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
            || has_annotation(invis, crate::runtime::MIXIN_ACCESSOR_DESCRIPTOR)
            || has_annotation(invis, crate::runtime::MIXIN_INVOKER_DESCRIPTOR)
            || method.annotations.iter().any(|a| {
                a.descriptor == crate::runtime::MIXIN_ACCESSOR_DESCRIPTOR
                    || a.descriptor == crate::runtime::MIXIN_INVOKER_DESCRIPTOR
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
    for _ in 0..crate::runtime::MAX_SUPERCLASS_WALK {
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
                    config_active: self.mixin_index.allows_config_gated(&class.name),
                    primary_has_no_configs,
                };
                index.register_class(
                    class,
                    reg,
                    |target| self.classes.contains_key(target),
                    |name| self.class(name),
                );
            }

            // 2. Linked jar classes (gated by each linked jar's own mixin configs).
            if let Some(side) = self.mixin_index.side {
                for (configs, config_sides, jar_classes) in self.linked.mixin_config_sets() {
                    for class_name in jar_classes {
                        if !seen_mixins.insert(class_name.clone()) {
                            continue;
                        }
                        let Some(class) = self.class(class_name) else {
                            continue;
                        };
                        let reg = MixinRegistration {
                            is_primary: false,
                            config_active: mixin_config_gate(
                                configs,
                                config_sides,
                                class_name,
                                side,
                            )
                            .is_ok(),
                            primary_has_no_configs: false,
                        };
                        index.register_class(
                            class,
                            reg,
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
                    config_active: !self.mixin_index.require_config,
                    primary_has_no_configs: false,
                };
                index.register_class(
                    class,
                    reg,
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
        crate::runtime::game_object::declared_types(key)
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
        let Some(key) = crate::runtime::game_object::handle_key(receiver).map(str::to_owned) else {
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
        let Some(handle_key) = crate::runtime::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(None);
        };
        if r.owner == "java/lang/Object" || r.name == "<init>" {
            return Ok(None);
        }

        // 1. Direct call on a `@Mixin` class (e.g. `this.helper()` or `@Shadow` call).
        let direct_action = {
            let index = self.runtime_mixin_index();
            index.meta.get(&r.owner).map(|meta| {
                let is_shadow = meta
                    .shadow_methods
                    .contains(&(r.name.clone(), r.descriptor.clone()));
                let first_target = meta.targets.first().cloned();
                let active_on_handle = self
                    .game_handle_types(&handle_key)
                    .iter()
                    .any(|ty| meta.active_interface_targets.contains(ty.as_str()));
                (is_shadow, first_target, active_on_handle)
            })
        };
        if let Some((is_shadow, first_target, active_on_handle)) = direct_action {
            if is_shadow {
                let target_ref = MethodRef {
                    owner: first_target.unwrap_or_else(|| r.owner.clone()),
                    name: r.name.clone(),
                    descriptor: r.descriptor.clone(),
                };
                return self
                    .host
                    .invoke_instance(&target_ref, receiver, args)
                    .map(Some);
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
        let declared = self.game_handle_types(&handle_key);
        enum Resolution {
            None,
            Mixin(String),
            Ambiguous,
            InterfaceDefault(String),
            DefaultConflict(Vec<String>),
            UnimplementedInterface,
        }

        let resolution = {
            let index = self.runtime_mixin_index();
            let mut matching_mixins_by_level: Vec<Vec<String>> = Vec::new();
            let mut any_matching_mixin = false;
            let mut all_matching_mixins: Vec<String> = Vec::new();

            for target_type in &declared {
                let Some(mixins) = index.interface_by_target.get(target_type.as_str()) else {
                    continue;
                };
                let mut level_impls = Vec::new();
                for active in mixins {
                    let implements_owner = active
                        .interfaces
                        .iter()
                        .any(|iface| iface == &r.owner || self.interface_extends(iface, &r.owner));
                    if !implements_owner {
                        continue;
                    }
                    any_matching_mixin = true;
                    if !all_matching_mixins.contains(&active.mixin) {
                        all_matching_mixins.push(active.mixin.clone());
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
                // No mixin overrides the method: look for a maximally specific default
                // method on the injected interfaces (and `r.owner`).
                match self.maximally_specific_default(&all_matching_mixins, &r.name, &r.descriptor)
                {
                    DefaultSelection::Selected(iface) => Resolution::InterfaceDefault(iface),
                    DefaultSelection::Conflict(owners) => Resolution::DefaultConflict(owners),
                    DefaultSelection::None | DefaultSelection::Abstract => {
                        if self
                            .class(&r.owner)
                            .and_then(|file| file.method(&r.name, &r.descriptor))
                            .is_some_and(|m| !m.is_static() && m.code.is_some())
                        {
                            Resolution::InterfaceDefault(r.owner.clone())
                        } else {
                            Resolution::UnimplementedInterface
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
            Resolution::Ambiguous => {
                self.host.unknown_operand_call(r)?;
                Ok(Some(Value::default_for(return_desc(&r.descriptor))))
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

    /// Handle `getfield` on a game handle for `@Shadow` fields (routing to the host so
    /// unmodelled shadows are counted/refused) and for added fields of active
    /// interface-injection mixins (ensuring `<init>()V` ran and returning Java defaults
    /// without counting an unknown host call).
    pub(crate) fn game_handle_get_field(
        &mut self,
        receiver: &Value,
        r: &MethodRef,
    ) -> Result<Option<Value>, NativeError> {
        let Some(handle_key) = crate::runtime::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(None);
        };
        enum Action {
            None,
            Shadow(MethodRef),
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
                        let target_owner = meta
                            .targets
                            .first()
                            .cloned()
                            .unwrap_or_else(|| r.owner.clone());
                        Action::Shadow(MethodRef {
                            owner: target_owner,
                            name: r.name.clone(),
                            descriptor: r.descriptor.clone(),
                        })
                    } else if meta.added_instance_fields.contains(&member)
                        && self
                            .game_handle_types(&handle_key)
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
            Action::Shadow(target_ref) => {
                // Under `GameHookHost`, pass the original mixin ref so its `@Shadow` error
                // diagnostic names both the mixin field and `@Shadow`.
                let query = if self.host.defers_game_field_write_check() {
                    r
                } else {
                    &target_ref
                };
                self.host.get_field(receiver, query).map(Some)
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
        let Some(handle_key) = crate::runtime::game_object::handle_key(receiver).map(str::to_owned) else {
            return Ok(GameFieldWrite::Fallthrough);
        };
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
                            let target_owner = meta
                                .targets
                                .first()
                                .cloned()
                                .unwrap_or_else(|| r.owner.clone());
                            Action::UnmodelledGameField(MethodRef {
                                owner: target_owner,
                                name: r.name.clone(),
                                descriptor: r.descriptor.clone(),
                            })
                        }
                    } else if meta.added_instance_fields.contains(&member)
                        && self
                            .game_handle_types(&handle_key)
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
            let (include_in_by_target, include_in_interface_dispatch) = if reg.is_primary {
                if is_external_host_game_target && is_interface_injection {
                    // Host-game-object interface-injection Mixins are always strictly
                    // config-gated and require a runnable no-arg initialiser.
                    let active = reg.config_active && !details.unrunnable_initializer;
                    (active, active)
                } else if is_external_host_game_target {
                    // Accessor/invoker interfaces on host-modelled game targets keep the
                    // config-less probe scan when the jar has no Mixin configs, and obey
                    // the config gate when it does.
                    (reg.primary_has_no_configs || reg.config_active, false)
                } else {
                    // Jar-local woven classes (e.g. `demo/Woven$Target`) and non-host-game
                    // targets (e.g. `ArrayBackedEvent`) preserve the existing unconditional
                    // primary-jar `by_target` scan and never enter host-object dispatch.
                    (true, false)
                }
            } else {
                // Linked-jar Mixins are always config-gated.
                let active = reg.config_active && !details.unrunnable_initializer;
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
}

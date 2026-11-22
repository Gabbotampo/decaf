//! Generic Minecraft 26.3 block-use (`BlockBehaviour.useItemOn`, `useWithoutItem`,
//! `getMenuProvider`), sealed `InteractionResult` hierarchy, `BlockHitResult`, and
//! `ServerPlayer.openMenu(MenuProvider)` bridge.
//!
//! Nothing here knows any specific mod. Every answer matches Java 25 / Minecraft 26.3
//! bytecode (`server-26.3.jar`) or is counted as an unknown host call.

use std::collections::BTreeMap;

use crate::runtime::{
    ExecutionBudget, Host, MethodRef, NativeError, NativeJar, Runtime, StdHost, Value,
    component_value, expect_ref, facade_text, game_object, host_ids, host_optional,
    host_ref::{ClassHierarchy, HostKind, HostRef, Methods, exact_class_rules_out},
    jar_object, jdk_call, menu_models, null_checks, server_values, uuid_value,
};

pub(crate) const INTERACTION_RESULT: &str = "net/minecraft/world/InteractionResult";
pub(crate) const INTERACTION_RESULT_SUCCESS: &str = "net/minecraft/world/InteractionResult$Success";
pub(crate) const INTERACTION_RESULT_FAIL: &str = "net/minecraft/world/InteractionResult$Fail";
pub(crate) const INTERACTION_RESULT_PASS: &str = "net/minecraft/world/InteractionResult$Pass";
pub(crate) const INTERACTION_RESULT_TRY_EMPTY: &str =
    "net/minecraft/world/InteractionResult$TryEmptyHandInteraction";
pub(crate) const INTERACTION_RESULT_SWING_SOURCE: &str =
    "net/minecraft/world/InteractionResult$SwingSource";
pub(crate) const INTERACTION_RESULT_ITEM_CONTEXT: &str =
    "net/minecraft/world/InteractionResult$ItemContext";

pub(crate) const BLOCK_HIT_RESULT: &str = "net/minecraft/world/phys/BlockHitResult";
const HIT_RESULT_TYPE_PREFIX: &str = "enum/net/minecraft/world/phys/HitResult$Type/";
const DIRECTION_ENUM_PREFIX: &str = "enum/net/minecraft/core/Direction/";
const HAND_ENUM_PREFIX: &str = "enum/net/minecraft/world/InteractionHand/";
const SWING_SOURCE_ENUM_PREFIX: &str = "enum/net/minecraft/world/InteractionResult$SwingSource/";

pub(crate) const BLOCK_BEHAVIOUR: &str = "net/minecraft/world/level/block/state/BlockBehaviour";
pub(crate) const BLOCK_STATE_BASE: &str =
    "net/minecraft/world/level/block/state/BlockBehaviour$BlockStateBase";
pub(crate) const BLOCK_STATE: &str = "net/minecraft/world/level/block/state/BlockState";

pub(crate) const USE_ITEM_ON_DESC: &str = "(Lnet/minecraft/world/item/ItemStack;Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/InteractionHand;Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;";
pub(crate) const USE_WITHOUT_ITEM_DESC: &str = "(Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;";
pub(crate) const GET_MENU_PROVIDER_DESC: &str = "(Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/MenuProvider;";

const BLOCK_STATE_USE_ITEM_ON_DESC: &str = "(Lnet/minecraft/world/item/ItemStack;Lnet/minecraft/world/level/Level;Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/InteractionHand;Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;";
const BLOCK_STATE_USE_WITHOUT_ITEM_DESC: &str = "(Lnet/minecraft/world/level/Level;Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;";
const BLOCK_STATE_GET_MENU_PROVIDER_DESC: &str = "(Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/MenuProvider;";
const PLAYER_OPEN_MENU_DESC: &str = "(Lnet/minecraft/world/MenuProvider;)Ljava/util/OptionalInt;";

const STATIC_SUCCESS: &str = "static/SUCCESS";
const STATIC_SUCCESS_SERVER: &str = "static/SUCCESS_SERVER";
const STATIC_CONSUME: &str = "static/CONSUME";
const STATIC_FAIL: &str = "static/FAIL";
const STATIC_PASS: &str = "static/PASS";
const STATIC_TRY_EMPTY: &str = "static/TRY_WITH_EMPTY_HAND";

const STATIC_ITEM_CONTEXT_NONE: &str = "static/NONE";
const STATIC_ITEM_CONTEXT_DEFAULT: &str = "static/DEFAULT";

/// Logical side on which a block-use call runs (`Level.isClientSide()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockUseSide {
    Server,
    Client,
}

/// Exact data of a Minecraft 26.3 `BlockHitResult`.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockHitData {
    pub location: [f64; 3],
    pub direction: String,
    pub pos: [i32; 3],
    pub inside: bool,
    pub miss: bool,
    pub world_border_hit: bool,
}

impl BlockHitData {
    /// A normal non-miss block hit at `pos` on face `direction` (`UP`, `NORTH`, ...).
    pub fn block(
        pos: [i32; 3],
        direction: impl Into<String>,
        location: [f64; 3],
        inside: bool,
    ) -> Self {
        Self {
            location,
            direction: direction.into(),
            pos,
            inside,
            miss: false,
            world_border_hit: false,
        }
    }
}

/// Player snapshot handed into a block-use invocation.
#[derive(Clone, Debug)]
pub struct BlockUsePlayerData {
    pub uuid: u128,
    pub name: String,
    pub entity_id: Option<i32>,
    pub session: u32,
    pub life: u32,
    pub sneaking: bool,
    pub spectator: bool,
    /// Optional held stacks (`ItemStack` host values). When `None`, `getMainHandItem` /
    /// `getOffhandItem` / `getItemInHand` remain counted rather than invented.
    pub main_hand: Option<Value>,
    pub off_hand: Option<Value>,
}

impl Default for BlockUsePlayerData {
    fn default() -> Self {
        Self {
            uuid: 1,
            name: "Player".to_owned(),
            entity_id: Some(1),
            session: 1,
            life: 0,
            sneaking: false,
            spectator: false,
            main_hand: None,
            off_hand: None,
        }
    }
}

/// Full vanilla context for a single `Block.useWithoutItem` or `Block.useItemOn` dispatch.
#[derive(Clone, Debug)]
pub struct BlockUseContext {
    pub side: BlockUseSide,
    pub dimension: String,
    pub block_id: String,
    pub state_properties: BTreeMap<String, String>,
    pub pos: [i32; 3],
    pub hit: BlockHitData,
    pub player: BlockUsePlayerData,
    /// `InteractionHand` enum constant name: `"MAIN_HAND"` or `"OFF_HAND"`.
    pub hand: String,
    /// Held `ItemStack` for `useItemOn` (`None` uses `ItemStack.EMPTY`).
    pub stack: Option<Value>,
}

/// Minecraft 26.3 `InteractionResult$SwingSource`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwingSource {
    None,
    Predicted,
    ServerOnly,
}

impl SwingSource {
    fn enum_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Predicted => "PREDICTED",
            Self::ServerOnly => "SERVER_ONLY",
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        let name = expect_ref(value)?.strip_prefix(SWING_SOURCE_ENUM_PREFIX)?;
        match name {
            "NONE" => Some(Self::None),
            "PREDICTED" => Some(Self::Predicted),
            "SERVER_ONLY" => Some(Self::ServerOnly),
            _ => None,
        }
    }

    fn to_value(self) -> Value {
        Value::Ref(Some(format!(
            "{SWING_SOURCE_ENUM_PREFIX}{}",
            self.enum_name()
        )))
    }
}

/// Minecraft 26.3 `InteractionResult$ItemContext`.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionItemContext {
    pub was_item_interaction: bool,
    pub held_item_transformed_to: Option<Value>,
}

/// Decoded Minecraft 26.3 `InteractionResult` sealed value.
#[derive(Clone, Debug, PartialEq)]
pub enum InteractionResultValue {
    Success {
        swing_source: SwingSource,
        item_context: InteractionItemContext,
    },
    Fail,
    Pass,
    TryEmptyHandInteraction,
}

impl InteractionResultValue {
    /// Vanilla `InteractionResult.consumesAction()`.
    pub fn consumes_action(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Vanilla `InteractionResult$Success.shouldSwing()` (`false` for non-`Success`).
    pub fn should_swing(&self) -> bool {
        matches!(
            self,
            Self::Success {
                swing_source: SwingSource::Predicted | SwingSource::ServerOnly,
                ..
            }
        )
    }

    /// Vanilla `SwingSource` when `Success`, `None` otherwise.
    pub fn swing_source(&self) -> Option<SwingSource> {
        match self {
            Self::Success { swing_source, .. } => Some(*swing_source),
            _ => None,
        }
    }
}

/// Typed effect recorded when server-side Java calls `player.openMenu(MenuProvider)`.
#[derive(Clone, Debug)]
pub struct OpenMenuEffect {
    /// Per-player server container sync id (`1..=100`, matching `ServerPlayer.nextContainerCounter`).
    pub container_id: i32,
    /// Java `MenuType` object returned by `menu.getType()`.
    pub menu_type: Value,
    /// Resolved registry id (e.g. `"example:bench"` or `"minecraft:crafting"`), when known.
    pub menu_type_id: Option<String>,
    /// Java `Component` returned by `provider.getDisplayName()`.
    pub title: Value,
    /// Exact vanilla JSON of `title`, when representable.
    pub title_json: Option<serde_json::Value>,
    /// Block position carried by `ContainerLevelAccess.create(level, pos)`, if any.
    pub access_pos: Option<[i32; 3]>,
    /// Java `MenuConstructor` lambda captured by `SimpleMenuProvider`, kept for live-menu W11.
    pub constructor: Option<Value>,
    /// Java `MenuProvider` passed to `player.openMenu(...)`.
    pub provider: Value,
    /// Live `AbstractContainerMenu` returned by `provider.createMenu(...)`.
    pub menu: Value,
}

/// Result of running a block's `useItemOn` or `useWithoutItem` hook.
#[derive(Clone, Debug)]
pub struct BlockUseOutcome {
    /// Decoded `InteractionResult`, or `None` if Java returned `null` or a non-`InteractionResult` value.
    pub result: Option<InteractionResultValue>,
    /// Raw Java return value.
    pub raw_result: Value,
    /// Typed open-menu effects recorded by `ServerPlayer.openMenu(MenuProvider)`.
    pub open_menus: Vec<OpenMenuEffect>,
    /// Unmodelled host calls recorded during this block-use invocation.
    pub unknown_host_calls: BTreeMap<String, usize>,
    /// Total count of unmodelled host calls during this block-use invocation.
    pub unknown_host_call_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SuccessData {
    pub(crate) swing_source: Value,
    pub(crate) item_context: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct ItemContextData {
    pub(crate) was_item_interaction: bool,
    pub(crate) held_item_transformed_to: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveBlockUse {
    pub(crate) side: BlockUseSide,
    pub(crate) dimension: String,
    pub(crate) level: Value,
    pub(crate) state: Value,
    pub(crate) pos: [i32; 3],
    pub(crate) player: Value,
    pub(crate) player_data: BlockUsePlayerData,
    pub(crate) player_inventory: Value,
    pub(crate) open_menus: Vec<OpenMenuEffect>,
}

/// Host state for block-use, `InteractionResult`, `BlockHitResult`, and `openMenu`.
#[derive(Clone, Debug, Default)]
pub(crate) struct BlockUseHostState {
    pub(crate) interaction_results: BTreeMap<String, SuccessData>,
    pub(crate) interaction_item_contexts: BTreeMap<String, ItemContextData>,
    pub(crate) block_hits: BTreeMap<String, BlockHitData>,
    pub(crate) active_use: Option<ActiveBlockUse>,
    pub(crate) player_container_counters: BTreeMap<u128, i32>,
    pub(crate) next_id: u64,
}

impl BlockUseHostState {
    fn next_serial(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub(crate) fn next_container_counter(&mut self, player_uuid: u128) -> i32 {
        let counter = self
            .player_container_counters
            .entry(player_uuid)
            .or_insert(0);
        *counter = (*counter % 100) + 1;
        *counter
    }

    pub(crate) fn alloc_success(&mut self, swing_source: Value, item_context: Value) -> Value {
        let id = self.next_serial();
        let key = format!("new/Success#ir{id}");
        self.interaction_results.insert(
            key.clone(),
            SuccessData {
                swing_source,
                item_context,
            },
        );
        Value::Host(HostRef::new(HostKind::InteractionResult, key))
    }

    pub(crate) fn alloc_empty_result(&mut self, variant: &str) -> Value {
        let id = self.next_serial();
        let key = format!("new/{variant}#ir{id}");
        Value::Host(HostRef::new(HostKind::InteractionResult, key))
    }

    pub(crate) fn alloc_item_context(
        &mut self,
        was_item_interaction: bool,
        held_item_transformed_to: Value,
    ) -> Value {
        let id = self.next_serial();
        let key = format!("new/ItemContext#ir{id}");
        self.interaction_item_contexts.insert(
            key.clone(),
            ItemContextData {
                was_item_interaction,
                held_item_transformed_to,
            },
        );
        Value::Host(HostRef::new(HostKind::InteractionItemContext, key))
    }

    pub(crate) fn alloc_block_hit(&mut self, data: BlockHitData) -> Value {
        let id = self.next_serial();
        let key = format!("game/block-hit/hit#hit{id}");
        self.block_hits.insert(key.clone(), data);
        Value::Host(HostRef::new(HostKind::GameTransient, key))
    }

    pub(crate) fn root_texts<'a>(&'a self, out: &mut Vec<&'a str>) {
        fn push_val<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
            out.extend(crate::runtime::heap::value_texts(v));
        }
        if let Some(active) = &self.active_use {
            push_val(&active.level, out);
            push_val(&active.state, out);
            push_val(&active.player, out);
            push_val(&active.player_inventory, out);
            if let Some(stack) = &active.player_data.main_hand {
                push_val(stack, out);
            }
            if let Some(stack) = &active.player_data.off_hand {
                push_val(stack, out);
            }
            for effect in &active.open_menus {
                push_val(&effect.menu_type, out);
                push_val(&effect.title, out);
                if let Some(ctor) = &effect.constructor {
                    push_val(ctor, out);
                }
                push_val(&effect.provider, out);
                push_val(&effect.menu, out);
            }
        }
    }

    pub(crate) fn keyed_text(&self, marker: &str) -> Option<String> {
        if let Some(entry) = self.interaction_results.get(marker) {
            return Some(format!("{entry:?}"));
        }
        if let Some(entry) = self.interaction_item_contexts.get(marker) {
            return Some(format!("{entry:?}"));
        }
        None
    }

    pub(crate) fn keyed_table_keys(&self) -> impl Iterator<Item = &str> {
        self.interaction_results
            .keys()
            .chain(self.interaction_item_contexts.keys())
            .chain(self.block_hits.keys())
            .map(String::as_str)
    }

    pub(crate) fn prune_keyed_tables(&mut self, keep: impl Fn(&str) -> bool) {
        self.interaction_results.retain(|k, _| keep(k));
        self.interaction_item_contexts.retain(|k, _| keep(k));
        self.block_hits.retain(|k, _| keep(k));
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.interaction_results.len()
            + self.interaction_item_contexts.len()
            + self.block_hits.len()
    }
}

// ---------------------------------------------------------------------------
// Vanilla MC 26.3 InteractionResult static handles & HostRef kind methods
// ---------------------------------------------------------------------------

pub(crate) fn ir_success() -> Value {
    Value::Host(HostRef::new(HostKind::InteractionResult, STATIC_SUCCESS))
}

pub(crate) fn ir_success_server() -> Value {
    Value::Host(HostRef::new(
        HostKind::InteractionResult,
        STATIC_SUCCESS_SERVER,
    ))
}

pub(crate) fn ir_consume() -> Value {
    Value::Host(HostRef::new(HostKind::InteractionResult, STATIC_CONSUME))
}

pub(crate) fn ir_fail() -> Value {
    Value::Host(HostRef::new(HostKind::InteractionResult, STATIC_FAIL))
}

pub(crate) fn ir_pass() -> Value {
    Value::Host(HostRef::new(HostKind::InteractionResult, STATIC_PASS))
}

pub(crate) fn ir_try_with_empty_hand() -> Value {
    Value::Host(HostRef::new(HostKind::InteractionResult, STATIC_TRY_EMPTY))
}

pub(crate) fn item_context_none() -> Value {
    Value::Host(HostRef::new(
        HostKind::InteractionItemContext,
        STATIC_ITEM_CONTEXT_NONE,
    ))
}

pub(crate) fn item_context_default() -> Value {
    Value::Host(HostRef::new(
        HostKind::InteractionItemContext,
        STATIC_ITEM_CONTEXT_DEFAULT,
    ))
}

pub(crate) fn interaction_result_class(key: &str) -> &'static str {
    match key {
        STATIC_SUCCESS | STATIC_SUCCESS_SERVER | STATIC_CONSUME => INTERACTION_RESULT_SUCCESS,
        STATIC_FAIL => INTERACTION_RESULT_FAIL,
        STATIC_PASS => INTERACTION_RESULT_PASS,
        STATIC_TRY_EMPTY => INTERACTION_RESULT_TRY_EMPTY,
        _ if key.starts_with("new/Success#") => INTERACTION_RESULT_SUCCESS,
        _ if key.starts_with("new/Fail#") => INTERACTION_RESULT_FAIL,
        _ if key.starts_with("new/Pass#") => INTERACTION_RESULT_PASS,
        _ if key.starts_with("new/TryEmptyHandInteraction#") => INTERACTION_RESULT_TRY_EMPTY,
        _ => INTERACTION_RESULT,
    }
}

fn static_success_parts(key: &str) -> Option<(Value, Value)> {
    match key {
        STATIC_SUCCESS => Some((SwingSource::Predicted.to_value(), item_context_default())),
        STATIC_SUCCESS_SERVER => Some((SwingSource::ServerOnly.to_value(), item_context_default())),
        STATIC_CONSUME => Some((SwingSource::None.to_value(), item_context_default())),
        _ => None,
    }
}

fn static_item_context_parts(key: &str) -> Option<(bool, Value)> {
    match key {
        STATIC_ITEM_CONTEXT_NONE => Some((false, Value::Ref(None))),
        STATIC_ITEM_CONTEXT_DEFAULT => Some((true, Value::Ref(None))),
        _ => None,
    }
}

pub(crate) fn success_parts_in_host(host: &StdHost, key: &str) -> Option<(Value, Value)> {
    static_success_parts(key).or_else(|| {
        host.block_use
            .interaction_results
            .get(key)
            .map(|d| (d.swing_source.clone(), d.item_context.clone()))
    })
}

pub(crate) fn item_context_parts_in_host(host: &StdHost, key: &str) -> Option<(bool, Value)> {
    static_item_context_parts(key).or_else(|| {
        host.block_use
            .interaction_item_contexts
            .get(key)
            .map(|d| (d.was_item_interaction, d.held_item_transformed_to.clone()))
    })
}

/// Map a `HostKind::InteractionResult` (or legacy text marker) to its Fabric event name,
/// used by `events.rs` `Combine::Interaction`.
pub(crate) fn events_interaction_name(host: &StdHost, value: &Value) -> Option<String> {
    if let Value::Host(handle) = value
        && handle.kind() == HostKind::InteractionResult
    {
        return Some(match handle.key() {
            STATIC_SUCCESS => "SUCCESS".to_owned(),
            STATIC_SUCCESS_SERVER => "SUCCESS_SERVER".to_owned(),
            STATIC_CONSUME => "CONSUME".to_owned(),
            STATIC_FAIL => "FAIL".to_owned(),
            STATIC_PASS => "PASS".to_owned(),
            STATIC_TRY_EMPTY => "TRY_WITH_EMPTY_HAND".to_owned(),
            key => match interaction_result_class(key) {
                INTERACTION_RESULT_FAIL => "FAIL".to_owned(),
                INTERACTION_RESULT_PASS => "PASS".to_owned(),
                INTERACTION_RESULT_TRY_EMPTY => "TRY_WITH_EMPTY_HAND".to_owned(),
                INTERACTION_RESULT_SUCCESS => match success_parts_in_host(host, key)
                    .and_then(|(s, _)| SwingSource::from_value(&s))
                {
                    Some(SwingSource::Predicted) => "SUCCESS".to_owned(),
                    Some(SwingSource::ServerOnly) => "SUCCESS_SERVER".to_owned(),
                    Some(SwingSource::None) => "CONSUME".to_owned(),
                    None => "SUCCESS".to_owned(),
                },
                _ => return None,
            },
        });
    }
    expect_ref(value)
        .and_then(|text| text.strip_prefix("interaction-result/"))
        .map(str::to_owned)
}

/// Decode a Java `InteractionResult` handle into a typed [`InteractionResultValue`].
pub(crate) fn decode_interaction_result(
    host: &StdHost,
    value: &Value,
) -> Option<InteractionResultValue> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::InteractionResult {
        return None;
    }
    match interaction_result_class(handle.key()) {
        INTERACTION_RESULT_FAIL => Some(InteractionResultValue::Fail),
        INTERACTION_RESULT_PASS => Some(InteractionResultValue::Pass),
        INTERACTION_RESULT_TRY_EMPTY => Some(InteractionResultValue::TryEmptyHandInteraction),
        INTERACTION_RESULT_SUCCESS => {
            let (swing_val, ctx_val) = success_parts_in_host(host, handle.key())?;
            let swing_source = SwingSource::from_value(&swing_val)?;
            let item_context = match &ctx_val {
                Value::Host(ctx) if ctx.kind() == HostKind::InteractionItemContext => {
                    let (was_item, transformed) = item_context_parts_in_host(host, ctx.key())?;
                    InteractionItemContext {
                        was_item_interaction: was_item,
                        held_item_transformed_to: if matches!(transformed, Value::Ref(None)) {
                            None
                        } else {
                            Some(transformed)
                        },
                    }
                }
                _ => return None,
            };
            Some(InteractionResultValue::Success {
                swing_source,
                item_context,
            })
        }
        _ => None,
    }
}

fn interaction_result_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    let exact = interaction_result_class(handle.key());
    if wanted == exact
        || wanted == INTERACTION_RESULT
        || wanted == "java/lang/Record"
        || wanted == "java/lang/Object"
    {
        return Some(true);
    }
    if matches!(
        wanted,
        INTERACTION_RESULT_SUCCESS
            | INTERACTION_RESULT_FAIL
            | INTERACTION_RESULT_PASS
            | INTERACTION_RESULT_TRY_EMPTY
            | INTERACTION_RESULT_ITEM_CONTEXT
            | INTERACTION_RESULT_SWING_SOURCE
            | "java/lang/String"
    ) || wanted.starts_with('[')
        || exact_class_rules_out(exact, wanted)
    {
        return Some(false);
    }
    hierarchy.is_instance(exact, wanted)
}

fn interaction_item_context_instance_of(
    _handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if wanted == INTERACTION_RESULT_ITEM_CONTEXT
        || wanted == "java/lang/Record"
        || wanted == "java/lang/Object"
    {
        return Some(true);
    }
    if matches!(
        wanted,
        INTERACTION_RESULT
            | INTERACTION_RESULT_SUCCESS
            | INTERACTION_RESULT_FAIL
            | INTERACTION_RESULT_PASS
            | INTERACTION_RESULT_TRY_EMPTY
            | INTERACTION_RESULT_SWING_SOURCE
            | "java/lang/String"
    ) || wanted.starts_with('[')
        || exact_class_rules_out(INTERACTION_RESULT_ITEM_CONTEXT, wanted)
    {
        return Some(false);
    }
    hierarchy.is_instance(INTERACTION_RESULT_ITEM_CONTEXT, wanted)
}

pub(crate) const INTERACTION_RESULT_METHODS: Methods = Methods {
    to_string: |handle, nested| {
        let key = handle.key();
        match interaction_result_class(key) {
            INTERACTION_RESULT_FAIL => Ok(Some("Fail[]".to_owned())),
            INTERACTION_RESULT_PASS => Ok(Some("Pass[]".to_owned())),
            INTERACTION_RESULT_TRY_EMPTY => Ok(Some("TryEmptyHandInteraction[]".to_owned())),
            INTERACTION_RESULT_SUCCESS => {
                let Some((swing, ctx)) =
                    static_success_parts(key).or_else(|| nested.interaction_result_success(key))
                else {
                    return Ok(None);
                };
                let swing_str = match &swing {
                    Value::Ref(None) => "null".to_owned(),
                    Value::Ref(Some(text)) => match text.strip_prefix(SWING_SOURCE_ENUM_PREFIX) {
                        Some(name) => name.to_owned(),
                        None => match nested.text(&swing)? {
                            Some(t) => t,
                            None => return Ok(None),
                        },
                    },
                    _ => match nested.text(&swing)? {
                        Some(t) => t,
                        None => return Ok(None),
                    },
                };
                let ctx_str = match &ctx {
                    Value::Ref(None) => "null".to_owned(),
                    _ => match nested.text(&ctx)? {
                        Some(t) => t,
                        None => return Ok(None),
                    },
                };
                Ok(Some(format!(
                    "Success[swingSource={swing_str}, itemContext={ctx_str}]"
                )))
            }
            _ => Ok(None),
        }
    },
    hash_code: |handle, nested| {
        let key = handle.key();
        match interaction_result_class(key) {
            INTERACTION_RESULT_FAIL | INTERACTION_RESULT_PASS | INTERACTION_RESULT_TRY_EMPTY => {
                Ok(Some(0))
            }
            INTERACTION_RESULT_SUCCESS => {
                let Some((swing, ctx)) =
                    static_success_parts(key).or_else(|| nested.interaction_result_success(key))
                else {
                    return Ok(None);
                };
                // Enum.hashCode() is Object.hashCode() (identity hash), so a non-null
                // SwingSource has no reproducible hashCode and must remain counted (None).
                let swing_hash = match &swing {
                    Value::Ref(None) => 0,
                    _ => match nested.hash(&swing)? {
                        Some(h) => h,
                        None => return Ok(None),
                    },
                };
                let ctx_hash = match &ctx {
                    Value::Ref(None) => 0,
                    _ => match nested.hash(&ctx)? {
                        Some(h) => h,
                        None => return Ok(None),
                    },
                };
                Ok(Some(swing_hash.wrapping_mul(31).wrapping_add(ctx_hash)))
            }
            _ => Ok(None),
        }
    },
    equals: |handle, other, nested| {
        let Value::Host(other_handle) = other else {
            return Ok(Some(false));
        };
        if other_handle.kind() != HostKind::InteractionResult {
            return Ok(Some(false));
        }
        if handle == other_handle {
            return Ok(Some(true));
        }
        let class_a = interaction_result_class(handle.key());
        let class_b = interaction_result_class(other_handle.key());
        if class_a != class_b {
            return Ok(Some(false));
        }
        if class_a != INTERACTION_RESULT_SUCCESS {
            return Ok(Some(true));
        }
        let Some((s1, c1)) = static_success_parts(handle.key())
            .or_else(|| nested.interaction_result_success(handle.key()))
        else {
            return Ok(None);
        };
        let Some((s2, c2)) = static_success_parts(other_handle.key())
            .or_else(|| nested.interaction_result_success(other_handle.key()))
        else {
            return Ok(None);
        };
        // Record `equals` compares components last-to-first (`itemContext`, then `swingSource`)
        // or short-circuits cleanly; both are pure (`ItemContext` / enum / null).
        let eq_ctx = match (&c1, &c2) {
            (Value::Ref(None), Value::Ref(None)) => Some(true),
            (Value::Ref(None), _) | (_, Value::Ref(None)) => Some(false),
            _ => nested.equals(&c1, &c2)?,
        };
        let Some(eq_ctx) = eq_ctx else {
            return Ok(None);
        };
        if !eq_ctx {
            return Ok(Some(false));
        }
        let eq_swing = match (&s1, &s2) {
            (Value::Ref(None), Value::Ref(None)) => Some(true),
            (Value::Ref(None), _) | (_, Value::Ref(None)) => Some(false),
            _ => Some(s1 == s2),
        };
        Ok(eq_swing)
    },
    instance_of: interaction_result_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

pub(crate) const INTERACTION_ITEM_CONTEXT_METHODS: Methods = Methods {
    to_string: |handle, nested| {
        let key = handle.key();
        let Some((was_item, stack)) =
            static_item_context_parts(key).or_else(|| nested.interaction_item_context(key))
        else {
            return Ok(None);
        };
        let stack_str = match &stack {
            Value::Ref(None) => "null".to_owned(),
            _ => match nested.text(&stack)? {
                Some(t) => t,
                None => return Ok(None),
            },
        };
        Ok(Some(format!(
            "ItemContext[wasItemInteraction={was_item}, heldItemTransformedTo={stack_str}]"
        )))
    },
    hash_code: |handle, nested| {
        let key = handle.key();
        let Some((was_item, stack)) =
            static_item_context_parts(key).or_else(|| nested.interaction_item_context(key))
        else {
            return Ok(None);
        };
        let b_hash = if was_item { 1231_i32 } else { 1237_i32 };
        let stack_hash = match &stack {
            Value::Ref(None) => 0,
            _ => match nested.hash(&stack)? {
                Some(h) => h,
                None => return Ok(None),
            },
        };
        Ok(Some(b_hash.wrapping_mul(31).wrapping_add(stack_hash)))
    },
    equals: |handle, other, nested| {
        let Value::Host(other_handle) = other else {
            return Ok(Some(false));
        };
        if other_handle.kind() != HostKind::InteractionItemContext {
            return Ok(Some(false));
        }
        if handle == other_handle {
            return Ok(Some(true));
        }
        let Some((b1, s1)) = static_item_context_parts(handle.key())
            .or_else(|| nested.interaction_item_context(handle.key()))
        else {
            return Ok(None);
        };
        let Some((b2, s2)) = static_item_context_parts(other_handle.key())
            .or_else(|| nested.interaction_item_context(other_handle.key()))
        else {
            return Ok(None);
        };
        if b1 != b2 {
            return Ok(Some(false));
        }
        match (&s1, &s2) {
            (Value::Ref(None), Value::Ref(None)) => Ok(Some(true)),
            (Value::Ref(None), _) | (_, Value::Ref(None)) => Ok(Some(false)),
            _ => nested.equals(&s1, &s2),
        }
    },
    instance_of: interaction_item_context_instance_of,
    crosses_jars: |_, _| true,
    compare_to: None,
};

// ---------------------------------------------------------------------------
// Vanilla MC 26.3 MenuType static constants (verified via javap on server-26.3.jar)
// ---------------------------------------------------------------------------

const VANILLA_MENU_TYPES: &[(&str, &str)] = &[
    ("GENERIC_9x1", "minecraft:generic_9x1"),
    ("GENERIC_9x2", "minecraft:generic_9x2"),
    ("GENERIC_9x3", "minecraft:generic_9x3"),
    ("GENERIC_9x4", "minecraft:generic_9x4"),
    ("GENERIC_9x5", "minecraft:generic_9x5"),
    ("GENERIC_9x6", "minecraft:generic_9x6"),
    ("GENERIC_3x3", "minecraft:generic_3x3"),
    ("CRAFTER_3x3", "minecraft:crafter_3x3"),
    ("ANVIL", "minecraft:anvil"),
    ("BEACON", "minecraft:beacon"),
    ("BLAST_FURNACE", "minecraft:blast_furnace"),
    ("BREWING_STAND", "minecraft:brewing_stand"),
    ("CRAFTING", "minecraft:crafting"),
    ("ENCHANTMENT", "minecraft:enchantment"),
    ("FURNACE", "minecraft:furnace"),
    ("GRINDSTONE", "minecraft:grindstone"),
    ("HOPPER", "minecraft:hopper"),
    ("LECTERN", "minecraft:lectern"),
    ("LOOM", "minecraft:loom"),
    ("MERCHANT", "minecraft:merchant"),
    ("SHULKER_BOX", "minecraft:shulker_box"),
    ("SMITHING", "minecraft:smithing"),
    ("SMOKER", "minecraft:smoker"),
    ("CARTOGRAPHY_TABLE", "minecraft:cartography_table"),
    ("STONECUTTER", "minecraft:stonecutter"),
];

/// Resolve a `MenuType` value to its registry id (`namespace:path`).
pub(crate) fn resolve_menu_type_id(host: &StdHost, menu_type: &Value) -> Option<String> {
    if let Some(key) = game_object::handle_key(menu_type)
        && let Some(id) = key.strip_prefix("game/menu-type/")
    {
        return Some(id.to_owned());
    }
    host.registrations()
        .iter()
        .find(|entry| entry.registry == "minecraft:menu" && &entry.value == menu_type)
        .map(|entry| entry.id.clone())
}

// ---------------------------------------------------------------------------
// Vanilla MC 26.3 BlockBehaviour allow-lists (verified via javap on server-26.3.jar)
// ---------------------------------------------------------------------------

/// Vanilla block superclasses whose `useItemOn` inherits `BlockBehaviour.useItemOn`
/// verbatim (`return InteractionResult.TRY_WITH_EMPTY_HAND;`).
pub(crate) fn inherits_block_behaviour_use_item_on(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/world/level/block/state/BlockBehaviour"
            | "net/minecraft/world/level/block/Block"
            | "net/minecraft/world/level/block/HorizontalDirectionalBlock"
            | "net/minecraft/world/level/block/DirectionalBlock"
            | "net/minecraft/world/level/block/RotatedPillarBlock"
            | "net/minecraft/world/level/block/TransparentBlock"
            | "net/minecraft/world/level/block/HalfTransparentBlock"
            | "net/minecraft/world/level/block/FallingBlock"
            | "net/minecraft/world/level/block/ColoredFallingBlock"
            | "net/minecraft/world/level/block/BaseEntityBlock"
            | "net/minecraft/world/level/block/CraftingTableBlock"
            | "net/minecraft/world/level/block/SlabBlock"
            | "net/minecraft/world/level/block/StairBlock"
            | "net/minecraft/world/level/block/WallBlock"
            | "net/minecraft/world/level/block/FenceBlock"
            | "net/minecraft/world/level/block/CrossCollisionBlock"
            | "net/minecraft/world/level/block/IronBarsBlock"
    )
}

/// Vanilla block superclasses whose `useWithoutItem` inherits `BlockBehaviour.useWithoutItem`
/// verbatim (`return InteractionResult.PASS;`).
/// Note: `CraftingTableBlock` and `FenceBlock` override `useWithoutItem`, so they are excluded.
pub(crate) fn inherits_block_behaviour_use_without_item(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/world/level/block/state/BlockBehaviour"
            | "net/minecraft/world/level/block/Block"
            | "net/minecraft/world/level/block/HorizontalDirectionalBlock"
            | "net/minecraft/world/level/block/DirectionalBlock"
            | "net/minecraft/world/level/block/RotatedPillarBlock"
            | "net/minecraft/world/level/block/TransparentBlock"
            | "net/minecraft/world/level/block/HalfTransparentBlock"
            | "net/minecraft/world/level/block/FallingBlock"
            | "net/minecraft/world/level/block/ColoredFallingBlock"
            | "net/minecraft/world/level/block/BaseEntityBlock"
            | "net/minecraft/world/level/block/SlabBlock"
            | "net/minecraft/world/level/block/StairBlock"
            | "net/minecraft/world/level/block/WallBlock"
            | "net/minecraft/world/level/block/CrossCollisionBlock"
            | "net/minecraft/world/level/block/IronBarsBlock"
    )
}

/// Vanilla block superclasses whose `getMenuProvider` inherits `BlockBehaviour.getMenuProvider`
/// verbatim (`return null;`).
/// Note: `CraftingTableBlock` and `BaseEntityBlock` override `getMenuProvider`, so they are excluded.
pub(crate) fn inherits_block_behaviour_get_menu_provider(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/world/level/block/state/BlockBehaviour"
            | "net/minecraft/world/level/block/Block"
            | "net/minecraft/world/level/block/HorizontalDirectionalBlock"
            | "net/minecraft/world/level/block/DirectionalBlock"
            | "net/minecraft/world/level/block/RotatedPillarBlock"
            | "net/minecraft/world/level/block/TransparentBlock"
            | "net/minecraft/world/level/block/HalfTransparentBlock"
            | "net/minecraft/world/level/block/FallingBlock"
            | "net/minecraft/world/level/block/ColoredFallingBlock"
            | "net/minecraft/world/level/block/SlabBlock"
            | "net/minecraft/world/level/block/StairBlock"
            | "net/minecraft/world/level/block/WallBlock"
            | "net/minecraft/world/level/block/FenceBlock"
            | "net/minecraft/world/level/block/CrossCollisionBlock"
            | "net/minecraft/world/level/block/IronBarsBlock"
    )
}

// ---------------------------------------------------------------------------
// StdHost hooks: get_static, random_new, static_call, instance_call
// ---------------------------------------------------------------------------

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner == INTERACTION_RESULT
        && r.descriptor
            .starts_with("Lnet/minecraft/world/InteractionResult")
    {
        return match r.name.as_str() {
            "SUCCESS" => Some(ir_success()),
            "SUCCESS_SERVER" => Some(ir_success_server()),
            "CONSUME" => Some(ir_consume()),
            "FAIL" => Some(ir_fail()),
            "PASS" => Some(ir_pass()),
            "TRY_WITH_EMPTY_HAND" => Some(ir_try_with_empty_hand()),
            _ => None,
        };
    }
    if r.owner == INTERACTION_RESULT_ITEM_CONTEXT
        && r.descriptor == "Lnet/minecraft/world/InteractionResult$ItemContext;"
    {
        return match r.name.as_str() {
            "NONE" => Some(item_context_none()),
            "DEFAULT" => Some(item_context_default()),
            _ => None,
        };
    }
    if r.owner == menu_models::MENU_TYPE
        && r.descriptor == "Lnet/minecraft/world/inventory/MenuType;"
    {
        return VANILLA_MENU_TYPES
            .iter()
            .find(|(field, _)| *field == r.name)
            .map(|(_, id)| game_object::server_value(format!("game/menu-type/{id}")));
    }
    None
}

pub(crate) fn random_new(host: &mut StdHost, class: &str) -> Option<Value> {
    match class {
        INTERACTION_RESULT_SUCCESS => Some(
            host.block_use
                .alloc_success(Value::Ref(None), Value::Ref(None)),
        ),
        INTERACTION_RESULT_FAIL => Some(host.block_use.alloc_empty_result("Fail")),
        INTERACTION_RESULT_PASS => Some(host.block_use.alloc_empty_result("Pass")),
        INTERACTION_RESULT_TRY_EMPTY => {
            Some(host.block_use.alloc_empty_result("TryEmptyHandInteraction"))
        }
        INTERACTION_RESULT_ITEM_CONTEXT => {
            Some(host.block_use.alloc_item_context(false, Value::Ref(None)))
        }
        BLOCK_HIT_RESULT => Some(host.block_use.alloc_block_hit(BlockHitData {
            location: [0.0, 0.0, 0.0],
            direction: "UP".to_owned(),
            pos: [0, 0, 0],
            inside: false,
            miss: false,
            world_border_hit: false,
        })),
        menu_models::SIMPLE_MENU_PROVIDER => Some(menu_models::alloc_simple_menu_provider(
            &mut host.next_menu_object,
        )),
        _ => None,
    }
}

fn parse_vec3(value: &Value) -> Option<[f64; 3]> {
    let key = game_object::handle_key(value)?.strip_prefix(crate::runtime::client_values::VEC3)?;
    let mut parts = key.split(',');
    let x = parts.next()?.parse::<f64>().ok()?;
    let y = parts.next()?.parse::<f64>().ok()?;
    let z = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some([x, y, z])
}

fn vec3_value([x, y, z]: [f64; 3]) -> Value {
    game_object::client_value(format!(
        "{}{},{},{}",
        crate::runtime::client_values::VEC3,
        crate::runtime::client_values::coord_text(x),
        crate::runtime::client_values::coord_text(y),
        crate::runtime::client_values::coord_text(z),
    ))
}

fn parse_direction(value: &Value) -> Option<String> {
    expect_ref(value)?
        .strip_prefix(DIRECTION_ENUM_PREFIX)
        .map(str::to_owned)
}

fn parse_block_pos(value: &Value) -> Option<[i32; 3]> {
    let key = game_object::handle_key(value)?;
    server_values::block_pos(key)
}

fn block_pos_value([x, y, z]: [i32; 3]) -> Value {
    game_object::server_value(format!("block-pos/{x},{y},{z}"))
}

fn uuid_text(uuid: u128) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (uuid >> 96) as u32,
        (uuid >> 80) as u16,
        (uuid >> 64) as u16,
        (uuid >> 48) as u16,
        uuid & 0xffff_ffff_ffff
    )
}

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == BLOCK_HIT_RESULT
        && r.name == "miss"
        && r.descriptor
            == "(Lnet/minecraft/world/phys/Vec3;Lnet/minecraft/core/Direction;Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/phys/BlockHitResult;"
    {
        let [loc, dir, pos] = args else {
            return None;
        };
        let location = parse_vec3(loc)?;
        let direction = parse_direction(dir)?;
        let pos = parse_block_pos(pos)?;
        return Some(Ok(host.block_use.alloc_block_hit(BlockHitData {
            location,
            direction,
            pos,
            inside: false,
            miss: true,
            world_border_hit: false,
        })));
    }
    None
}

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    // 1. InteractionResult & its sealed records
    if let Value::Host(handle) = receiver
        && handle.kind() == HostKind::InteractionResult
    {
        return interaction_result_call(host, r, handle, args);
    }

    // 2. InteractionResult$ItemContext
    if let Value::Host(handle) = receiver
        && handle.kind() == HostKind::InteractionItemContext
    {
        return item_context_call(host, r, handle, args);
    }

    // 3. BlockHitResult / HitResult
    if !host.block_use.block_hits.is_empty()
        && let Some(key) = game_object::handle_key(receiver)
        && host.block_use.block_hits.contains_key(key)
    {
        return block_hit_call(host, r, key.to_owned(), args);
    }

    // 4. Allow-listed vanilla BlockBehaviour defaults (inherited or `super.use*` / `super.getMenuProvider`)
    if matches!(
        r.name.as_str(),
        "useItemOn" | "useWithoutItem" | "getMenuProvider"
    ) && jar_object::object_class(receiver).is_some()
    {
        if r.name == "useItemOn"
            && r.descriptor == USE_ITEM_ON_DESC
            && inherits_block_behaviour_use_item_on(&r.owner)
        {
            return Some(Ok(ir_try_with_empty_hand()));
        }
        if r.name == "useWithoutItem"
            && r.descriptor == USE_WITHOUT_ITEM_DESC
            && inherits_block_behaviour_use_without_item(&r.owner)
        {
            return Some(Ok(ir_pass()));
        }
        if r.name == "getMenuProvider"
            && r.descriptor == GET_MENU_PROVIDER_DESC
            && inherits_block_behaviour_get_menu_provider(&r.owner)
        {
            return Some(Ok(Value::Ref(None)));
        }
    }

    // 5. Active block-use Level & Player objects
    if let Some(active) = host.block_use.active_use.as_ref() {
        let is_level = receiver == &active.level;
        let is_player = !is_level && receiver == &active.player;
        if is_level || is_player {
            let active = active.clone();
            return Some(if is_level {
                active_level_call(host, &active, r, args)
            } else {
                active_player_call(host, &active, r, args)
            });
        }
    }

    None
}

fn interaction_result_call(
    host: &mut StdHost,
    r: &MethodRef,
    handle: &HostRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = handle.key();
    let exact = interaction_result_class(key);
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "<init>",
            "(Lnet/minecraft/world/InteractionResult$SwingSource;Lnet/minecraft/world/InteractionResult$ItemContext;)V",
        ) if exact == INTERACTION_RESULT_SUCCESS && key.starts_with("new/Success#") => {
            let [swing_source, item_context] = args else {
                return None;
            };
            host.block_use.interaction_results.insert(
                key.to_owned(),
                SuccessData {
                    swing_source: swing_source.clone(),
                    item_context: item_context.clone(),
                },
            );
            Some(Ok(Value::Void))
        }
        ("<init>", "()V")
            if matches!(
                exact,
                INTERACTION_RESULT_FAIL | INTERACTION_RESULT_PASS | INTERACTION_RESULT_TRY_EMPTY
            ) && key.starts_with("new/") =>
        {
            Some(Ok(Value::Void))
        }
        ("consumesAction", "()Z") => Some(Ok(Value::Int(i32::from(
            exact == INTERACTION_RESULT_SUCCESS,
        )))),
        ("shouldSwing", "()Z") if exact == INTERACTION_RESULT_SUCCESS => {
            let (swing, _) = success_parts_in_host(host, key)?;
            let is_none = swing == SwingSource::None.to_value();
            Some(Ok(Value::Int(i32::from(!is_none))))
        }
        ("swingSource", "()Lnet/minecraft/world/InteractionResult$SwingSource;")
            if exact == INTERACTION_RESULT_SUCCESS =>
        {
            let (swing, _) = success_parts_in_host(host, key)?;
            Some(Ok(swing))
        }
        ("itemContext", "()Lnet/minecraft/world/InteractionResult$ItemContext;")
            if exact == INTERACTION_RESULT_SUCCESS =>
        {
            let (_, ctx) = success_parts_in_host(host, key)?;
            Some(Ok(ctx))
        }
        ("wasItemInteraction", "()Z") if exact == INTERACTION_RESULT_SUCCESS => {
            let (_, ctx) = success_parts_in_host(host, key)?;
            if ctx == Value::Ref(None) {
                return Some(Err(null_checks::invoke_on_null(&MethodRef {
                    owner: INTERACTION_RESULT_ITEM_CONTEXT.into(),
                    name: "wasItemInteraction".into(),
                    descriptor: "()Z".into(),
                })));
            }
            let Value::Host(ctx_handle) = ctx else {
                return None;
            };
            let (was_item, _) = item_context_parts_in_host(host, ctx_handle.key())?;
            Some(Ok(Value::Int(i32::from(was_item))))
        }
        ("heldItemTransformedTo", "()Lnet/minecraft/world/item/ItemStack;")
            if exact == INTERACTION_RESULT_SUCCESS =>
        {
            let (_, ctx) = success_parts_in_host(host, key)?;
            if ctx == Value::Ref(None) {
                return Some(Err(null_checks::invoke_on_null(&MethodRef {
                    owner: INTERACTION_RESULT_ITEM_CONTEXT.into(),
                    name: "heldItemTransformedTo".into(),
                    descriptor: "()Lnet/minecraft/world/item/ItemStack;".into(),
                })));
            }
            let Value::Host(ctx_handle) = ctx else {
                return None;
            };
            let (_, stack) = item_context_parts_in_host(host, ctx_handle.key())?;
            Some(Ok(stack))
        }
        (
            "heldItemTransformedTo",
            "(Lnet/minecraft/world/item/ItemStack;)Lnet/minecraft/world/InteractionResult$Success;",
        ) if exact == INTERACTION_RESULT_SUCCESS => {
            let [stack] = args else {
                return None;
            };
            let (swing, _) = success_parts_in_host(host, key)?;
            let new_ctx = host.block_use.alloc_item_context(true, stack.clone());
            Some(Ok(host.block_use.alloc_success(swing, new_ctx)))
        }
        ("withoutItem", "()Lnet/minecraft/world/InteractionResult$Success;")
            if exact == INTERACTION_RESULT_SUCCESS =>
        {
            let (swing, _) = success_parts_in_host(host, key)?;
            Some(Ok(host.block_use.alloc_success(swing, item_context_none())))
        }
        _ => None,
    }
}

fn item_context_call(
    host: &mut StdHost,
    r: &MethodRef,
    handle: &HostRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = handle.key();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("<init>", "(ZLnet/minecraft/world/item/ItemStack;)V")
            if key.starts_with("new/ItemContext#") =>
        {
            let [Value::Int(was_item), stack] = args else {
                return None;
            };
            host.block_use.interaction_item_contexts.insert(
                key.to_owned(),
                ItemContextData {
                    was_item_interaction: *was_item != 0,
                    held_item_transformed_to: stack.clone(),
                },
            );
            Some(Ok(Value::Void))
        }
        ("wasItemInteraction", "()Z") => {
            let (was_item, _) = item_context_parts_in_host(host, key)?;
            Some(Ok(Value::Int(i32::from(was_item))))
        }
        ("heldItemTransformedTo", "()Lnet/minecraft/world/item/ItemStack;") => {
            let (_, stack) = item_context_parts_in_host(host, key)?;
            Some(Ok(stack))
        }
        _ => None,
    }
}

fn block_hit_call(
    host: &mut StdHost,
    r: &MethodRef,
    key: String,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "<init>",
            "(Lnet/minecraft/world/phys/Vec3;Lnet/minecraft/core/Direction;Lnet/minecraft/core/BlockPos;Z)V",
        ) => {
            let [loc, dir, pos, Value::Int(inside)] = args else {
                return None;
            };
            let data = BlockHitData {
                location: parse_vec3(loc)?,
                direction: parse_direction(dir)?,
                pos: parse_block_pos(pos)?,
                inside: *inside != 0,
                miss: false,
                world_border_hit: false,
            };
            host.block_use.block_hits.insert(key, data);
            Some(Ok(Value::Void))
        }
        (
            "<init>",
            "(Lnet/minecraft/world/phys/Vec3;Lnet/minecraft/core/Direction;Lnet/minecraft/core/BlockPos;ZZ)V",
        ) => {
            let [
                loc,
                dir,
                pos,
                Value::Int(inside),
                Value::Int(world_border_hit),
            ] = args
            else {
                return None;
            };
            let data = BlockHitData {
                location: parse_vec3(loc)?,
                direction: parse_direction(dir)?,
                pos: parse_block_pos(pos)?,
                inside: *inside != 0,
                miss: false,
                world_border_hit: *world_border_hit != 0,
            };
            host.block_use.block_hits.insert(key, data);
            Some(Ok(Value::Void))
        }
        ("getBlockPos", "()Lnet/minecraft/core/BlockPos;") => {
            let data = host.block_use.block_hits.get(&key)?;
            Some(Ok(block_pos_value(data.pos)))
        }
        ("getDirection", "()Lnet/minecraft/core/Direction;") => {
            let data = host.block_use.block_hits.get(&key)?;
            Some(Ok(Value::Ref(Some(format!(
                "{DIRECTION_ENUM_PREFIX}{}",
                data.direction
            )))))
        }
        ("getLocation", "()Lnet/minecraft/world/phys/Vec3;") => {
            let data = host.block_use.block_hits.get(&key)?;
            Some(Ok(vec3_value(data.location)))
        }
        ("getType", "()Lnet/minecraft/world/phys/HitResult$Type;") => {
            let data = host.block_use.block_hits.get(&key)?;
            let kind = if data.miss { "MISS" } else { "BLOCK" };
            Some(Ok(Value::Ref(Some(format!(
                "{HIT_RESULT_TYPE_PREFIX}{kind}"
            )))))
        }
        ("isInside", "()Z") => {
            let data = host.block_use.block_hits.get(&key)?;
            Some(Ok(Value::Int(i32::from(data.inside))))
        }
        ("isWorldBorderHit", "()Z") => {
            let data = host.block_use.block_hits.get(&key)?;
            Some(Ok(Value::Int(i32::from(data.world_border_hit))))
        }
        (
            "withDirection",
            "(Lnet/minecraft/core/Direction;)Lnet/minecraft/world/phys/BlockHitResult;",
        ) => {
            let dir = parse_direction(args.first()?)?;
            let mut next = host.block_use.block_hits.get(&key)?.clone();
            next.direction = dir;
            Some(Ok(host.block_use.alloc_block_hit(next)))
        }
        (
            "withPosition",
            "(Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/phys/BlockHitResult;",
        ) => {
            let pos = parse_block_pos(args.first()?)?;
            let mut next = host.block_use.block_hits.get(&key)?.clone();
            next.pos = pos;
            Some(Ok(host.block_use.alloc_block_hit(next)))
        }
        ("hitBorder", "()Lnet/minecraft/world/phys/BlockHitResult;") => {
            let mut next = host.block_use.block_hits.get(&key)?.clone();
            next.world_border_hit = true;
            Some(Ok(host.block_use.alloc_block_hit(next)))
        }
        _ => None,
    }
}

fn active_level_call(
    host: &mut StdHost,
    active: &ActiveBlockUse,
    r: &MethodRef,
    args: &[Value],
) -> Result<Value, NativeError> {
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("isClientSide", "()Z") => Ok(Value::Int(i32::from(active.side == BlockUseSide::Client))),
        ("dimension", "()Lnet/minecraft/resources/ResourceKey;") => Ok(
            host_ids::resource_key_named("minecraft:dimension", &active.dimension),
        ),
        (
            "getBlockState",
            "(Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/level/block/state/BlockState;",
        ) => {
            if let Some(pos) = args.first().and_then(parse_block_pos)
                && pos == active.pos
            {
                return Ok(active.state.clone());
            }
            host.unknown_host_call(r)
        }
        ("getServer", "()Lnet/minecraft/server/MinecraftServer;") => {
            if active.side == BlockUseSide::Client {
                Ok(Value::Ref(None))
            } else {
                Ok(game_object::server_value("game/server".into()))
            }
        }
        _ => host.unknown_host_call(r),
    }
}

fn active_player_call(
    host: &mut StdHost,
    active: &ActiveBlockUse,
    r: &MethodRef,
    args: &[Value],
) -> Result<Value, NativeError> {
    let p = &active.player_data;
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("level", "()Lnet/minecraft/world/level/Level;") => Ok(active.level.clone()),
        ("serverLevel", "()Lnet/minecraft/server/level/ServerLevel;")
            if active.side == BlockUseSide::Server =>
        {
            Ok(active.level.clone())
        }
        ("getInventory", "()Lnet/minecraft/world/entity/player/Inventory;") => {
            Ok(active.player_inventory.clone())
        }
        ("getUUID", "()Ljava/util/UUID;") => uuid_value(&uuid_text(p.uuid)),
        ("getStringUUID", "()Ljava/lang/String;") => Ok(Value::Ref(Some(uuid_text(p.uuid)))),
        ("getName" | "getDisplayName", "()Lnet/minecraft/network/chat/Component;") => {
            Ok(component_value("literal", &p.name))
        }
        ("getScoreboardName" | "getPlainTextName", "()Ljava/lang/String;") => {
            Ok(Value::Ref(Some(p.name.clone())))
        }
        ("getId", "()I") => match p.entity_id {
            Some(id) => Ok(Value::Int(id)),
            None => host.unknown_host_call(r),
        },
        ("isShiftKeyDown" | "isCrouching" | "isSecondaryUseActive", "()Z") => {
            Ok(Value::Int(i32::from(p.sneaking)))
        }
        ("isSpectator", "()Z") => Ok(Value::Int(i32::from(p.spectator))),
        ("getMainHandItem", "()Lnet/minecraft/world/item/ItemStack;") => match &p.main_hand {
            Some(stack) => Ok(stack.clone()),
            None => host.unknown_host_call(r),
        },
        ("getOffhandItem", "()Lnet/minecraft/world/item/ItemStack;") => match &p.off_hand {
            Some(stack) => Ok(stack.clone()),
            None => host.unknown_host_call(r),
        },
        (
            "getItemInHand",
            "(Lnet/minecraft/world/InteractionHand;)Lnet/minecraft/world/item/ItemStack;",
        ) => {
            let hand = args
                .first()
                .and_then(expect_ref)
                .and_then(|s| s.strip_prefix(HAND_ENUM_PREFIX));
            let stack = match hand {
                Some("MAIN_HAND") => p.main_hand.as_ref(),
                Some("OFF_HAND") => p.off_hand.as_ref(),
                _ => None,
            };
            match stack {
                Some(stack) => Ok(stack.clone()),
                None => host.unknown_host_call(r),
            }
        }
        _ => host.unknown_host_call(r),
    }
}

// ---------------------------------------------------------------------------
// Out-of-line Runtime helpers (`#[inline(never)]` to preserve invoke_frame stack budget)
// ---------------------------------------------------------------------------

/// Handle `BlockStateBase.{getMenuProvider, useWithoutItem, useItemOn}` and
/// `Player.openMenu(MenuProvider)` from `Runtime::invoke_game_helpers`.
#[inline(never)]
pub(crate) fn invoke_runtime_helper<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(receiver) = receiver else {
        return Ok(None);
    };

    if matches!(r.owner.as_str(), BLOCK_STATE | BLOCK_STATE_BASE) {
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("getMenuProvider", BLOCK_STATE_GET_MENU_PROVIDER_DESC) => {
                return invoke_block_state_get_menu_provider(runtime, receiver, args);
            }
            ("useWithoutItem", BLOCK_STATE_USE_WITHOUT_ITEM_DESC) => {
                return invoke_block_state_use_without_item(runtime, receiver, args);
            }
            ("useItemOn", BLOCK_STATE_USE_ITEM_ON_DESC) => {
                return invoke_block_state_use_item_on(runtime, receiver, args);
            }
            _ => {}
        }
    }

    if r.name == "openMenu"
        && r.descriptor == PLAYER_OPEN_MENU_DESC
        && matches!(
            r.owner.as_str(),
            "net/minecraft/world/entity/player/Player"
                | "net/minecraft/server/level/ServerPlayer"
                | "net/minecraft/client/player/LocalPlayer"
                | "net/minecraft/client/player/AbstractClientPlayer"
                | "net/minecraft/client/player/RemotePlayer"
        )
    {
        return invoke_player_open_menu(runtime, receiver, args);
    }

    Ok(None)
}

#[inline(never)]
fn block_vanilla_superclass<H: Host>(runtime: &Runtime<'_, H>, block: &Value) -> String {
    let Some(class) = jar_object::object_class(block) else {
        return BLOCK_BEHAVIOUR.to_owned();
    };
    let mut cur = class.to_owned();
    for _ in 0..64 {
        let Some(file) = runtime.class(&cur) else {
            return cur;
        };
        if file.super_name.is_empty() {
            return BLOCK_BEHAVIOUR.to_owned();
        }
        cur = file.super_name.clone();
    }
    BLOCK_BEHAVIOUR.to_owned()
}

#[inline(never)]
fn dispatch_block_virtual<H: Host>(
    runtime: &mut Runtime<'_, H>,
    block: &Value,
    name: &str,
    descriptor: &str,
    args: &[Value],
) -> Result<Value, NativeError> {
    let virtual_ref = MethodRef {
        owner: BLOCK_BEHAVIOUR.to_owned(),
        name: name.to_owned(),
        descriptor: descriptor.to_owned(),
    };
    if let Some(jar_owner) = runtime.jvm_selected_jar_body(&virtual_ref, 0xb6, Some(block))? {
        return runtime.invoke(&jar_owner, name, descriptor, Some(block.clone()), args);
    }
    let vanilla_super = block_vanilla_superclass(runtime, block);
    match (name, descriptor) {
        ("useItemOn", USE_ITEM_ON_DESC) if inherits_block_behaviour_use_item_on(&vanilla_super) => {
            Ok(ir_try_with_empty_hand())
        }
        ("useWithoutItem", USE_WITHOUT_ITEM_DESC)
            if inherits_block_behaviour_use_without_item(&vanilla_super) =>
        {
            Ok(ir_pass())
        }
        ("getMenuProvider", GET_MENU_PROVIDER_DESC)
            if inherits_block_behaviour_get_menu_provider(&vanilla_super) =>
        {
            Ok(Value::Ref(None))
        }
        _ => runtime.host.unknown_operand_call(&MethodRef {
            owner: vanilla_super,
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
        }),
    }
}

#[inline(never)]
fn invoke_block_state_get_menu_provider<H: Host>(
    runtime: &mut Runtime<'_, H>,
    state: &Value,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(block) = runtime.host.block_state_block(state) else {
        return Ok(None);
    };
    let [level, pos] = args else {
        return Ok(None);
    };
    dispatch_block_virtual(
        runtime,
        &block,
        "getMenuProvider",
        GET_MENU_PROVIDER_DESC,
        &[state.clone(), level.clone(), pos.clone()],
    )
    .map(Some)
}

#[inline(never)]
fn invoke_block_state_use_without_item<H: Host>(
    runtime: &mut Runtime<'_, H>,
    state: &Value,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(block) = runtime.host.block_state_block(state) else {
        return Ok(None);
    };
    let [level, player, hit] = args else {
        return Ok(None);
    };
    let pos = runtime.call_virtual(
        jdk_call::Site::Virtual,
        BLOCK_HIT_RESULT,
        "getBlockPos",
        "()Lnet/minecraft/core/BlockPos;",
        hit,
        &[],
    )?;
    dispatch_block_virtual(
        runtime,
        &block,
        "useWithoutItem",
        USE_WITHOUT_ITEM_DESC,
        &[
            state.clone(),
            level.clone(),
            pos,
            player.clone(),
            hit.clone(),
        ],
    )
    .map(Some)
}

#[inline(never)]
fn invoke_block_state_use_item_on<H: Host>(
    runtime: &mut Runtime<'_, H>,
    state: &Value,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(block) = runtime.host.block_state_block(state) else {
        return Ok(None);
    };
    let [stack, level, player, hand, hit] = args else {
        return Ok(None);
    };
    let pos = runtime.call_virtual(
        jdk_call::Site::Virtual,
        BLOCK_HIT_RESULT,
        "getBlockPos",
        "()Lnet/minecraft/core/BlockPos;",
        hit,
        &[],
    )?;
    dispatch_block_virtual(
        runtime,
        &block,
        "useItemOn",
        USE_ITEM_ON_DESC,
        &[
            stack.clone(),
            state.clone(),
            level.clone(),
            pos,
            player.clone(),
            hand.clone(),
            hit.clone(),
        ],
    )
    .map(Some)
}

/// Exact MC 26.3 `Player.openMenu(MenuProvider)` vs `ServerPlayer.openMenu(MenuProvider)`:
/// - Client (`LocalPlayer` / `Player`): returns `OptionalInt.empty()`, records nothing.
/// - Server (`ServerPlayer`):
///   - `provider == null` -> `OptionalInt.empty()`
///   - increments `containerCounter = containerCounter % 100 + 1`
///   - calls `provider.createMenu(containerCounter, getInventory(), this)`
///   - if `menu == null` -> `OptionalInt.empty()`
///   - calls `menu.getType()` (throws `UnsupportedOperationException` if `menuType == null`)
///   - calls `provider.getDisplayName()`
///   - records typed [`OpenMenuEffect`] and returns `OptionalInt.of(containerCounter)`
#[inline(never)]
fn invoke_player_open_menu<H: Host>(
    runtime: &mut Runtime<'_, H>,
    receiver: &Value,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(provider) = args.first() else {
        return Ok(None);
    };
    // Client side (`LocalPlayer` inherits `Player.openMenu` -> `OptionalInt.empty()`).
    if game_object::handle_key(receiver) == Some("client/player") {
        return Ok(Some(host_optional::empty_of(HostKind::OptionalInt)));
    }
    let Some(std_host) = runtime.host.as_std_host_mut() else {
        return Ok(None);
    };
    let Some((player_uuid, inventory)) =
        std_host.block_use.active_use.as_ref().and_then(|active| {
            (active.side == BlockUseSide::Server && receiver == &active.player)
                .then(|| (active.player_data.uuid, active.player_inventory.clone()))
        })
    else {
        return Ok(None);
    };
    if *provider == Value::Ref(None) {
        return Ok(Some(host_optional::empty_of(HostKind::OptionalInt)));
    }

    let container_id = std_host.block_use.next_container_counter(player_uuid);

    let menu = runtime.call_virtual(
        jdk_call::Site::Interface,
        menu_models::MENU_PROVIDER,
        "createMenu",
        "(ILnet/minecraft/world/entity/player/Inventory;Lnet/minecraft/world/entity/player/Player;)Lnet/minecraft/world/inventory/AbstractContainerMenu;",
        provider,
        &[Value::Int(container_id), inventory, receiver.clone()],
    )?;
    if menu == Value::Ref(None) {
        return Ok(Some(host_optional::empty_of(HostKind::OptionalInt)));
    }

    let menu_type = runtime.call_virtual(
        jdk_call::Site::Virtual,
        menu_models::ABSTRACT_CONTAINER_MENU,
        "getType",
        "()Lnet/minecraft/world/inventory/MenuType;",
        &menu,
        &[],
    )?;
    let title = runtime.call_virtual(
        jdk_call::Site::Interface,
        menu_models::MENU_PROVIDER,
        "getDisplayName",
        "()Lnet/minecraft/network/chat/Component;",
        provider,
        &[],
    )?;

    // Resolve access_pos: first from ItemCombinerMenu.access (exact, honoring NULL),
    // otherwise from any ContainerLevelAccess instance field stored on a custom menu subclass.
    let mut access_pos = None;
    let mut has_combiner_access = false;
    if let Some(std_host) = runtime.host.as_std_host_mut()
        && let Some(access) = menu_models::menu_combiner_access(&std_host.menu_states, &menu)
    {
        has_combiner_access = true;
        access_pos = menu_models::container_level_access_pos(&std_host.menu_states, &access);
    }
    if !has_combiner_access
        && let Some(menu_key) = jar_object::object_key(&menu)
        && let Some(std_host) = runtime.host.as_std_host_mut()
    {
        let prefix = format!("{menu_key}|");
        for (field_key, field_val) in &runtime.fields {
            if field_key.starts_with(&prefix)
                && let Some(pos) =
                    menu_models::container_level_access_pos(&std_host.menu_states, field_val)
            {
                access_pos = Some(pos);
                break;
            }
        }
    }

    if let Some(std_host) = runtime.host.as_std_host_mut() {
        let menu_type_id = resolve_menu_type_id(std_host, &menu_type);
        let title_json = facade_text::json_now(&title, Some(&std_host.component_states));
        let constructor =
            menu_models::simple_menu_provider_constructor(&std_host.menu_states, provider);
        if let Some(active) = std_host.block_use.active_use.as_mut() {
            active.open_menus.push(OpenMenuEffect {
                container_id,
                menu_type,
                menu_type_id,
                title,
                title_json,
                access_pos,
                constructor,
                provider: provider.clone(),
                menu,
            });
        }
    }

    Ok(Some(host_optional::optional_int(container_id)))
}

// ---------------------------------------------------------------------------
// Public entrypoints on StdHost and NativeJar
// ---------------------------------------------------------------------------

impl StdHost {
    /// Look up the Java `Block` object registered under `minecraft:block` for `block_id`.
    pub fn registered_block(&self, block_id: &str) -> Option<Value> {
        self.registrations()
            .iter()
            .find(|entry| entry.registry == "minecraft:block" && entry.id == block_id)
            .map(|entry| entry.value.clone())
    }

    /// Retain every Java value held by an [`OpenMenuEffect`] across subsequent GC cycles
    /// (for W11 live menu execution). Pair with [`StdHost::release_open_menu_effect`].
    pub fn retain_open_menu_effect(&mut self, effect: &OpenMenuEffect) {
        self.retain(&effect.menu_type);
        self.retain(&effect.title);
        if let Some(ctor) = &effect.constructor {
            self.retain(ctor);
        }
        self.retain(&effect.provider);
        self.retain(&effect.menu);
    }

    /// Release one retain hold acquired by [`StdHost::retain_open_menu_effect`].
    pub fn release_open_menu_effect(&mut self, effect: &OpenMenuEffect) {
        self.release(&effect.menu_type);
        self.release(&effect.title);
        if let Some(ctor) = &effect.constructor {
            self.release(ctor);
        }
        self.release(&effect.provider);
        self.release(&effect.menu);
    }

    /// Build the canonical `BlockState` value for a registered mod block and property overrides.
    pub(crate) fn block_state_for_use(
        &mut self,
        block_id: &str,
        overrides: &BTreeMap<String, String>,
    ) -> Option<(Value, Value)> {
        let block = self.registered_block(block_id)?;
        let block_key = jar_object::object_key(&block)?.to_owned();
        let mut values = BTreeMap::new();
        if let Some(props) = self.block_state_properties.get(&block_key) {
            for (name, info) in props {
                if let Some(first) = info.values.first() {
                    values.insert(name.clone(), first.clone());
                }
            }
        }
        if let Some(defaults) = self.block_default_states.get(&block_key) {
            for (name, value) in defaults {
                values.insert(name.clone(), value.clone());
            }
        }
        for (name, value) in overrides {
            values.insert(name.clone(), value.clone());
        }
        let state = self.canonical_block_state(&block_key, values);
        Some((block, state))
    }
}

#[derive(Clone, Copy)]
enum BlockUseKind {
    WithoutItem,
    ItemOn,
}

impl NativeJar {
    /// Run a registered Java block's `Block.useWithoutItem` override with exact MC 26.3 arguments.
    pub fn block_use_without_item(
        &self,
        host: &mut StdHost,
        context: &BlockUseContext,
    ) -> Result<BlockUseOutcome, NativeError> {
        self.block_use_inner(
            host,
            context,
            BlockUseKind::WithoutItem,
            &ExecutionBudget::UNBOUNDED,
        )
    }

    /// [`NativeJar::block_use_without_item`] bounded by `budget`.
    pub fn block_use_without_item_with_budget(
        &self,
        host: &mut StdHost,
        context: &BlockUseContext,
        budget: &ExecutionBudget,
    ) -> Result<BlockUseOutcome, NativeError> {
        self.block_use_inner(host, context, BlockUseKind::WithoutItem, budget)
    }

    /// Run a registered Java block's `Block.useItemOn` override with exact MC 26.3 arguments.
    pub fn block_use_item_on(
        &self,
        host: &mut StdHost,
        context: &BlockUseContext,
    ) -> Result<BlockUseOutcome, NativeError> {
        self.block_use_inner(
            host,
            context,
            BlockUseKind::ItemOn,
            &ExecutionBudget::UNBOUNDED,
        )
    }

    /// [`NativeJar::block_use_item_on`] bounded by `budget`.
    pub fn block_use_item_on_with_budget(
        &self,
        host: &mut StdHost,
        context: &BlockUseContext,
        budget: &ExecutionBudget,
    ) -> Result<BlockUseOutcome, NativeError> {
        self.block_use_inner(host, context, BlockUseKind::ItemOn, budget)
    }

    fn block_use_inner(
        &self,
        host: &mut StdHost,
        context: &BlockUseContext,
        kind: BlockUseKind,
        budget: &ExecutionBudget,
    ) -> Result<BlockUseOutcome, NativeError> {
        let Some((block, state)) =
            host.block_state_for_use(&context.block_id, &context.state_properties)
        else {
            return Err(NativeError::Metadata(format!(
                "block `{}` is not registered in this jar",
                context.block_id
            )));
        };

        let level = match context.side {
            BlockUseSide::Server => {
                game_object::server_value(format!("game/level/{}", context.dimension))
            }
            BlockUseSide::Client => game_object::client_value("client/level".into()),
        };
        let player = match context.side {
            BlockUseSide::Server => {
                let p = &context.player;
                let key = match p.entity_id {
                    Some(id) => {
                        format!("game/player/{:032x}/{}.{}/{id}", p.uuid, p.session, p.life)
                    }
                    None => format!("game/player/{:032x}/{}.{}", p.uuid, p.session, p.life),
                };
                game_object::server_value(key)
            }
            BlockUseSide::Client => game_object::client_value("client/player".into()),
        };
        let player_inventory = menu_models::allocate_inventory(
            &mut host.menu_states,
            &mut host.next_menu_object,
            player.clone(),
        );
        let pos = block_pos_value(context.pos);
        let hit = host.block_use.alloc_block_hit(context.hit.clone());
        let hand = Value::Ref(Some(format!("{HAND_ENUM_PREFIX}{}", context.hand)));
        let stack = context
            .stack
            .clone()
            .unwrap_or_else(menu_models::empty_stack);

        let unknowns_before: BTreeMap<String, usize> =
            host.unknown_host_calls().into_iter().collect();
        let mark = host.call_log_mark();
        let previous_active = host.block_use.active_use.replace(ActiveBlockUse {
            side: context.side,
            dimension: context.dimension.clone(),
            level: level.clone(),
            state: state.clone(),
            pos: context.pos,
            player: player.clone(),
            player_data: context.player.clone(),
            player_inventory: player_inventory.clone(),
            open_menus: Vec::new(),
        });

        let roots = vec![
            block.clone(),
            state.clone(),
            level.clone(),
            pos.clone(),
            player.clone(),
            player_inventory.clone(),
            hit.clone(),
            stack.clone(),
        ];

        let call_result =
            self.with_persistent_runtime_with_budget(host, &roots, budget, |runtime| match kind {
                BlockUseKind::WithoutItem => dispatch_block_virtual(
                    runtime,
                    &block,
                    "useWithoutItem",
                    USE_WITHOUT_ITEM_DESC,
                    &[
                        state.clone(),
                        level.clone(),
                        pos.clone(),
                        player.clone(),
                        hit.clone(),
                    ],
                ),
                BlockUseKind::ItemOn => dispatch_block_virtual(
                    runtime,
                    &block,
                    "useItemOn",
                    USE_ITEM_ON_DESC,
                    &[
                        stack.clone(),
                        state.clone(),
                        level.clone(),
                        pos.clone(),
                        player.clone(),
                        hand.clone(),
                        hit.clone(),
                    ],
                ),
            });

        let finished_active = host.block_use.active_use.take();
        host.block_use.active_use = previous_active;
        host.rewind_call_log(mark);

        let raw_result = call_result?;
        let open_menus = finished_active.map(|a| a.open_menus).unwrap_or_default();
        let result = decode_interaction_result(host, &raw_result);

        let mut unknown_host_calls = BTreeMap::new();
        let mut unknown_host_call_count = 0usize;
        for (name, count) in host.unknown_host_calls() {
            let prev = unknowns_before.get(&name).copied().unwrap_or(0);
            if count > prev {
                let delta = count - prev;
                unknown_host_calls.insert(name, delta);
                unknown_host_call_count += delta;
            }
        }

        Ok(BlockUseOutcome {
            result,
            raw_result,
            open_menus,
            unknown_host_calls,
            unknown_host_call_count,
        })
    }
}

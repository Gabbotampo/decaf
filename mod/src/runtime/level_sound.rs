//! Minecraft 26.3 `Level.playSound`, `playSeededSound`, `playLocalSound`,
//! `playPlayerSound`, and `Entity.playSound` models.
//!
//! Every overload mirrors the real 26.3 `server-26.3.jar` and client bytecode:
//!
//! - `Level` owns two distinct `RandomSource` fields:
//!   - `random` (`Level.getRandom()`): advanced by `ClientLevel.playLocalSound`
//!     and `ClientLevel.playPlayerSound` (when `minecraft.player != null`), and
//!     NOT advanced by `Level.playSound`.
//!   - `soundSeedGenerator` (`RandomSource.createThreadSafe()`, same 48-bit LCG
//!     as `LegacyRandomSource`): advanced by every non-seeded `Level.playSound`
//!     overload before calling `playSeededSound`, on both `ServerLevel` and
//!     `ClientLevel`.
//! - `playSeededSound` uses the caller-supplied `long seed` and draws neither
//!   random stream.
//! - `BlockPos` overloads convert `(x, y, z)` to `(x + 0.5, y + 0.5, z + 0.5)`
//!   and throw `NullPointerException` on a `null` position before drawing any
//!   random.
//! - `ServerLevel.playSeededSound` maps `except` via
//!   `except instanceof Player p ? p : null` and broadcasts using
//!   `SoundEvent.getRange(volume)`. `ServerLevel.playLocalSound` and
//!   `ServerLevel.playPlayerSound` are no-ops (after reading `BlockPos` for the
//!   `BlockPos` overload) and advance no random stream.
//! - `ClientLevel.playSeededSound` is a no-op unless
//!   `except == minecraft.player` (`if_acmpne`), though calling via
//!   `Level.playSound` still draws `soundSeedGenerator.nextLong()` first.
//!   `ClientLevel.playLocalSound` with `distanceDelay == true` computes delay
//!   ticks as `d2 > 100.0 ? (int)((Math.sqrt(d2) / 40.0) * 20.0) : 0` from the
//!   main camera position.
//! - `Entity.playSound(SoundEvent)` checks `!isSilent()` before calling
//!   `playSound(SoundEvent, 1.0f, 1.0f)` (even on `Player` / `LocalPlayer`,
//!   which inherit the 1-arg overload). `Player.playSound(SoundEvent, float,
//!   float)` and `LocalPlayer.playSound(SoundEvent, float, float)` do not check
//!   `isSilent()`.
//!
//! Whenever a required fact (sound event/holder, sound source, `BlockPos`
//! coordinates, target entity id, entity position/silent/sound-source, camera
//! position for `distanceDelay`, or the needed random stream) is absent or
//! unmodelled, the call is counted as unknown instead of guessing.

use std::collections::BTreeMap;

use crate::runtime::{
    Host, MethodRef, NativeError, StdHost, Value, client_surface, events, game_hooks, game_object,
    host_ref::HostKind, random_source, sound_event,
};

/// Minecraft 26.3 `net.minecraft.sounds.SoundSource`, in declaration (`ordinal`) order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum SoundSource {
    Master = 0,
    Music = 1,
    Records = 2,
    Weather = 3,
    Blocks = 4,
    Hostile = 5,
    Neutral = 6,
    Players = 7,
    Ambient = 8,
    Voice = 9,
    Ui = 10,
}

impl SoundSource {
    /// Every vanilla `SoundSource` in `SoundSource.values()` order.
    pub const ALL: [SoundSource; 11] = [
        Self::Master,
        Self::Music,
        Self::Records,
        Self::Weather,
        Self::Blocks,
        Self::Hostile,
        Self::Neutral,
        Self::Players,
        Self::Ambient,
        Self::Voice,
        Self::Ui,
    ];

    pub const fn ordinal(self) -> usize {
        self as usize
    }

    pub const fn from_ordinal(ordinal: usize) -> Option<Self> {
        if ordinal < Self::ALL.len() {
            Some(Self::ALL[ordinal])
        } else {
            None
        }
    }

    /// The Java enum constant name (`SoundSource.name()`).
    pub const fn java_name(self) -> &'static str {
        match self {
            Self::Master => "MASTER",
            Self::Music => "MUSIC",
            Self::Records => "RECORDS",
            Self::Weather => "WEATHER",
            Self::Blocks => "BLOCKS",
            Self::Hostile => "HOSTILE",
            Self::Neutral => "NEUTRAL",
            Self::Players => "PLAYERS",
            Self::Ambient => "AMBIENT",
            Self::Voice => "VOICE",
            Self::Ui => "UI",
        }
    }

    /// The vanilla wire/category name (`SoundSource.getName()`).
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Records => "record",
            Self::Weather => "weather",
            Self::Blocks => "block",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Players => "player",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
            Self::Ui => "ui",
        }
    }

    pub fn from_java_name(name: &str) -> Option<Self> {
        match name {
            "MASTER" => Some(Self::Master),
            "MUSIC" => Some(Self::Music),
            "RECORDS" => Some(Self::Records),
            "WEATHER" => Some(Self::Weather),
            "BLOCKS" => Some(Self::Blocks),
            "HOSTILE" => Some(Self::Hostile),
            "NEUTRAL" => Some(Self::Neutral),
            "PLAYERS" => Some(Self::Players),
            "AMBIENT" => Some(Self::Ambient),
            "VOICE" => Some(Self::Voice),
            "UI" => Some(Self::Ui),
            _ => None,
        }
    }

    /// Parse a runtime `SoundSource` enum constant marker.
    pub fn from_value(value: &Value) -> Option<Self> {
        let Value::Ref(Some(text)) = value else {
            return None;
        };
        let variant = text.strip_prefix("enum/net/minecraft/sounds/SoundSource/")?;
        Self::from_java_name(variant)
    }

    pub fn to_value(self) -> Value {
        Value::Ref(Some(format!(
            "enum/net/minecraft/sounds/SoundSource/{}",
            self.java_name()
        )))
    }
}

/// How the sound's `Holder<SoundEvent>` was obtained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundHolderKind {
    /// Bound `Holder$Reference` (passed directly or returned by
    /// `BuiltInRegistries.SOUND_EVENT.wrapAsHolder` for a registered `SoundEvent`).
    Reference { registry_id: String },
    /// Direct `Holder$Direct` (passed via `Holder.direct` or returned by
    /// `wrapAsHolder` for an unregistered `SoundEvent`).
    Direct,
}

/// Whether the recorded sound effect came from a `ServerLevel` or `ClientLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundSide {
    Server,
    Client,
}

/// Where a recorded sound plays.
#[derive(Clone, Debug, PartialEq)]
pub enum SoundTarget {
    /// Positional sound at `[x, y, z]` (`BlockPos` is centred with `+0.5`).
    Position([f64; 3]),
    /// Entity-bound sound (`ClientboundSoundEntityPacket` on server,
    /// `EntityBoundSoundInstance` on client).
    Entity {
        entity_id: i32,
        position: Option<[f64; 3]>,
    },
}

/// How the sound's 64-bit seed was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundSeed {
    /// Drawn from `Level.soundSeedGenerator.nextLong()`.
    LevelSeedGenerator(i64),
    /// Drawn from `ClientLevel.random.nextLong()`.
    LevelRandom(i64),
    /// Explicit `long seed` argument to `Level.playSeededSound`.
    Explicit(i64),
    /// Delegated to the live host engine (`LevelRef::play_sound`,
    /// `LevelWriteHost::play_sound`, `HitHost::play_sound_*`, or
    /// `ClientHost::play_local_sound`), which draws from its own sound-seed
    /// stream at execution time.
    HostDeferred,
}

/// One exact level sound effect recorded from a Java `Level` or `Entity` call.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelSoundEffect {
    pub side: SoundSide,
    pub dimension: Option<String>,
    /// Resolved `SoundEvent.location()`.
    pub sound_id: String,
    /// `SoundEvent.fixedRange()`: `None` means variable range
    /// (`volume > 1.0f ? 16.0f * volume : 16.0f`).
    pub fixed_range: Option<f32>,
    /// `Holder<SoundEvent>` kind for `playSound` / `playSeededSound`, or `None`
    /// for `ClientLevel.playLocalSound` / `playPlayerSound` which take
    /// `SoundEvent` directly.
    pub holder: Option<SoundHolderKind>,
    pub target: SoundTarget,
    pub source: SoundSource,
    pub volume: f32,
    pub pitch: f32,
    /// Excluded player UUID for `ServerLevel.playSeededSound`
    /// (`except instanceof Player p ? p : null`). Always `None` on client.
    pub except_player: Option<u128>,
    pub seed: SoundSeed,
    /// `distanceDelay` argument to `ClientLevel.playLocalSound`.
    pub distance_delay: bool,
    /// Exact `SoundManager.playDelayed` tick delay on client (`0` for
    /// immediate `SoundManager.play` and all server sounds).
    pub delay_ticks: u32,
}

impl LevelSoundEffect {
    /// Exact MC 26.3 `SoundEvent.getRange(volume)`:
    /// `fixedRange.orElse(volume > 1.0f ? 16.0f * volume : 16.0f)`.
    pub fn range(&self) -> f32 {
        self.fixed_range.unwrap_or(if self.volume > 1.0 {
            16.0 * self.volume
        } else {
            16.0
        })
    }

    /// Exact 64-bit seed when drawn locally or passed explicitly (`None` when
    /// deferred to a live host).
    pub fn exact_seed(&self) -> Option<i64> {
        match self.seed {
            SoundSeed::LevelSeedGenerator(seed)
            | SoundSeed::LevelRandom(seed)
            | SoundSeed::Explicit(seed) => Some(seed),
            SoundSeed::HostDeferred => None,
        }
    }
}

/// Optional sound-relevant facts for an entity or player marker.
/// Every absent field stays unknown and causes any call needing it to count.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntitySoundFacts {
    pub level: Option<String>,
    pub position: Option<[f64; 3]>,
    pub silent: Option<bool>,
    pub sound_source: Option<SoundSource>,
    pub entity_id: Option<i32>,
    pub player_uuid: Option<u128>,
    pub is_player: bool,
}

/// Per-host level sound state: seeded level randoms, standalone entity/client
/// facts, host-deferred seed flags, and recorded standalone sounds.
#[derive(Clone, Debug, Default)]
pub(crate) struct LevelSoundState {
    pub(crate) level_random_handles: BTreeMap<String, Value>,
    pub(crate) level_sound_seeds: BTreeMap<String, random_source::State>,
    pub(crate) host_deferred_server_seeds: bool,
    pub(crate) host_deferred_client_seeds: bool,
    pub(crate) entity_facts: BTreeMap<String, EntitySoundFacts>,
    pub(crate) standalone_client_camera: Option<[f64; 3]>,
    pub(crate) standalone_client_player: Option<EntitySoundFacts>,
    pub(crate) played_sounds: Vec<LevelSoundEffect>,
}

/// Exact vanilla MC 26.3 `Entity.getSoundSource()` for entity types whose sound
/// source does not depend on instance state.
///
/// Verified against every `net/minecraft/world/entity/**/*.class` in
/// `server-26.3.jar`:
/// - `minecraft:rabbit` depends on `Rabbit.getVariant() == Variant.EVIL`, so
///   it returns `None` here (counted unless an explicit `sound_source` is given).
/// - Modded or unknown entity ids return `None`.
pub fn vanilla_entity_sound_source(type_id: &str) -> Option<SoundSource> {
    if !crate::runtime::vanilla_entity_types::ENTITY_TYPES.contains(&type_id) {
        return None;
    }
    match type_id {
        "minecraft:rabbit" => None,
        "minecraft:player" => Some(SoundSource::Players),
        "minecraft:experience_orb" | "minecraft:item" => Some(SoundSource::Ambient),
        "minecraft:lightning_bolt" => Some(SoundSource::Weather),
        "minecraft:ender_dragon"
        | "minecraft:wither"
        | "minecraft:blaze"
        | "minecraft:creeper"
        | "minecraft:elder_guardian"
        | "minecraft:enderman"
        | "minecraft:endermite"
        | "minecraft:ghast"
        | "minecraft:giant"
        | "minecraft:guardian"
        | "minecraft:phantom"
        | "minecraft:ravager"
        | "minecraft:shulker"
        | "minecraft:silverfish"
        | "minecraft:vex"
        | "minecraft:witch"
        | "minecraft:zoglin"
        | "minecraft:breeze"
        | "minecraft:creaking"
        | "minecraft:magma_cube"
        | "minecraft:slime"
        | "minecraft:hoglin"
        | "minecraft:evoker"
        | "minecraft:illusioner"
        | "minecraft:pillager"
        | "minecraft:vindicator"
        | "minecraft:piglin"
        | "minecraft:piglin_brute"
        | "minecraft:bogged"
        | "minecraft:parched"
        | "minecraft:skeleton"
        | "minecraft:stray"
        | "minecraft:wither_skeleton"
        | "minecraft:cave_spider"
        | "minecraft:spider"
        | "minecraft:warden"
        | "minecraft:drowned"
        | "minecraft:husk"
        | "minecraft:zombie"
        | "minecraft:zombie_villager"
        | "minecraft:zombified_piglin"
        | "minecraft:shulker_bullet" => Some(SoundSource::Hostile),
        _ => Some(SoundSource::Neutral),
    }
}

/// Outcome of a sound-surface call before converting `Unmodelled` into either
/// a lenient counted host answer or a strict façade refusal.
pub(crate) enum SoundCallOutcome {
    Handled(Value),
    Exception(NativeError),
    Unmodelled,
}

#[derive(Clone, Debug)]
struct ResolvedSound {
    sound_id: String,
    fixed_range: Option<f32>,
    holder: Option<SoundHolderKind>,
}

enum BlockPosArg {
    Coords([i32; 3]),
    Null,
    Unmodelled,
}

enum ExceptArg {
    None,
    Player(u128),
    NonPlayer,
    Unmodelled,
}

enum ClientExceptMatch {
    MatchesLocalPlayer,
    DoesNotMatch,
    Unmodelled,
}

enum TargetEntityArg {
    Entity {
        entity_id: i32,
        position: Option<[f64; 3]>,
    },
    Unmodelled,
}

fn npe(message: impl Into<String>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn block_pos_arg(value: &Value) -> BlockPosArg {
    if *value == Value::Ref(None) {
        return BlockPosArg::Null;
    }
    let Value::Host(handle) = value else {
        return BlockPosArg::Unmodelled;
    };
    if handle.kind() != HostKind::GameValue {
        return BlockPosArg::Unmodelled;
    }
    match crate::runtime::server_values::block_pos(handle.key()) {
        Some(coords) => BlockPosArg::Coords(coords),
        None => BlockPosArg::Unmodelled,
    }
}

fn double_arg(args: &[Value], index: usize) -> Option<f64> {
    match args.get(index)? {
        Value::Float(v) => Some(*v),
        Value::Int(v) => Some(f64::from(*v)),
        Value::Long(v) => Some(*v as f64),
        _ => None,
    }
}

fn float_arg(args: &[Value], index: usize) -> Option<f32> {
    match args.get(index)? {
        Value::Float(v) => Some(*v as f32),
        Value::Int(v) => Some(*v as f32),
        Value::Long(v) => Some(*v as f32),
        _ => None,
    }
}

fn long_arg(args: &[Value], index: usize) -> Option<i64> {
    match args.get(index)? {
        Value::Long(v) => Some(*v),
        Value::Int(v) => Some(i64::from(*v)),
        _ => None,
    }
}

fn bool_arg(args: &[Value], index: usize) -> Option<bool> {
    match args.get(index)? {
        Value::Int(v) => Some(*v != 0),
        _ => None,
    }
}

fn resolve_direct_sound_event(value: &Value) -> Option<(String, Option<f32>)> {
    sound_event::from_value(value)
}

fn wrap_sound_event_as_holder<H: Host + ?Sized>(host: &H, value: &Value) -> Option<ResolvedSound> {
    let (sound_id, fixed_range) = resolve_direct_sound_event(value)?;
    let holder = match host.registered_sound_id(value) {
        Some(registry_id) => SoundHolderKind::Reference { registry_id },
        None => SoundHolderKind::Direct,
    };
    Some(ResolvedSound {
        sound_id,
        fixed_range,
        holder: Some(holder),
    })
}

fn resolve_sound_holder<H: Host + ?Sized>(
    host: &mut H,
    value: &Value,
) -> Result<Option<ResolvedSound>, NativeError> {
    let Some(unwrapped) = host.unwrap_sound_holder(value) else {
        return Ok(None);
    };
    let (inner, registry_id) = unwrapped?;
    let Some((sound_id, fixed_range)) = resolve_direct_sound_event(&inner) else {
        return Ok(None);
    };
    let holder = match registry_id {
        Some(registry_id) => SoundHolderKind::Reference { registry_id },
        None => SoundHolderKind::Direct,
    };
    Ok(Some(ResolvedSound {
        sound_id,
        fixed_range,
        holder: Some(holder),
    }))
}

/// Exact MC 26.3 `ClientLevel.playSound(DDD..., boolean distanceDelay, long seed)`
/// delay formula: `d2 > 100.0 ? (int)((Math.sqrt(d2) / 40.0) * 20.0) : 0`.
fn client_delay_ticks(camera: [f64; 3], pos: [f64; 3]) -> u32 {
    let dx = camera[0] - pos[0];
    let dy = camera[1] - pos[1];
    let dz = camera[2] - pos[2];
    let d2 = dx * dx + dy * dy + dz * dz;
    if d2 > 100.0 {
        let seconds = d2.sqrt() / 40.0;
        let ticks = (seconds * 20.0) as i32;
        ticks.max(0) as u32
    } else {
        0
    }
}

fn is_level_sound_method_name(name: &str) -> bool {
    matches!(
        name,
        "playSound" | "playSeededSound" | "playLocalSound" | "playPlayerSound" | "getRandom"
    )
}

fn is_entity_sound_method_name(name: &str) -> bool {
    matches!(name, "playSound" | "getSoundSource" | "isSilent")
}

/// Parsed overload of `Level` / `LevelAccessor` / `ServerLevel` / `ClientLevel`.
enum LevelSoundOp<'a> {
    /// `playSound(Entity except, BlockPos pos, SoundEvent sound, SoundSource source[, float vol, float pitch])`
    BlockPos {
        except: &'a Value,
        pos: &'a Value,
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playSound(Entity except, double x, y, z, SoundEvent sound, SoundSource source[, float vol, float pitch])`
    PosEvent {
        except: &'a Value,
        pos: [f64; 3],
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playSound(Entity except, double x, y, z, Holder<SoundEvent> sound, SoundSource source, float vol, float pitch)`
    PosHolder {
        except: &'a Value,
        pos: [f64; 3],
        holder: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playSound(Entity except, Entity target, SoundEvent sound, SoundSource source, float vol, float pitch)`
    EntityEvent {
        except: &'a Value,
        target: &'a Value,
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playSound(Entity except, Entity target, Holder<SoundEvent> sound, SoundSource source, float vol, float pitch)`
    EntityHolder {
        except: &'a Value,
        target: &'a Value,
        holder: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playSeededSound(Entity except, double x, y, z, Holder<SoundEvent> sound, SoundSource source, float vol, float pitch, long seed)`
    SeededPosHolder {
        except: &'a Value,
        pos: [f64; 3],
        holder: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
        seed: i64,
    },
    /// `playSeededSound(Entity except, double x, y, z, SoundEvent sound, SoundSource source, float vol, float pitch, long seed)`
    SeededPosEvent {
        except: &'a Value,
        pos: [f64; 3],
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
        seed: i64,
    },
    /// `playSeededSound(Entity except, Entity target, Holder<SoundEvent> sound, SoundSource source, float vol, float pitch, long seed)`
    SeededEntityHolder {
        except: &'a Value,
        target: &'a Value,
        holder: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
        seed: i64,
    },
    /// `playLocalSound(BlockPos pos, SoundEvent sound, SoundSource source, float vol, float pitch, boolean distanceDelay)`
    LocalBlockPos {
        pos: &'a Value,
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
        distance_delay: bool,
    },
    /// `playLocalSound(double x, y, z, SoundEvent sound, SoundSource source, float vol, float pitch, boolean distanceDelay)`
    LocalPos {
        pos: [f64; 3],
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
        distance_delay: bool,
    },
    /// `playLocalSound(Entity target, SoundEvent sound, SoundSource source, float vol, float pitch)`
    LocalEntity {
        target: &'a Value,
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
    /// `playPlayerSound(SoundEvent sound, SoundSource source, float vol, float pitch)`
    PlayerSound {
        sound: &'a Value,
        source: &'a Value,
        volume: f32,
        pitch: f32,
    },
}

fn parse_level_op<'a>(r: &MethodRef, args: &'a [Value]) -> Option<LevelSoundOp<'a>> {
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/BlockPos;Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;)V",
        ) => Some(LevelSoundOp::BlockPos {
            except: args.first()?,
            pos: args.get(1)?,
            sound: args.get(2)?,
            source: args.get(3)?,
            volume: 1.0,
            pitch: 1.0,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/BlockPos;Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::BlockPos {
            except: args.first()?,
            pos: args.get(1)?,
            sound: args.get(2)?,
            source: args.get(3)?,
            volume: float_arg(args, 4)?,
            pitch: float_arg(args, 5)?,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;DDDLnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;)V",
        ) => Some(LevelSoundOp::PosEvent {
            except: args.first()?,
            pos: [
                double_arg(args, 1)?,
                double_arg(args, 2)?,
                double_arg(args, 3)?,
            ],
            sound: args.get(4)?,
            source: args.get(5)?,
            volume: 1.0,
            pitch: 1.0,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;DDDLnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::PosEvent {
            except: args.first()?,
            pos: [
                double_arg(args, 1)?,
                double_arg(args, 2)?,
                double_arg(args, 3)?,
            ],
            sound: args.get(4)?,
            source: args.get(5)?,
            volume: float_arg(args, 6)?,
            pitch: float_arg(args, 7)?,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;DDDLnet/minecraft/core/Holder;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::PosHolder {
            except: args.first()?,
            pos: [
                double_arg(args, 1)?,
                double_arg(args, 2)?,
                double_arg(args, 3)?,
            ],
            holder: args.get(4)?,
            source: args.get(5)?,
            volume: float_arg(args, 6)?,
            pitch: float_arg(args, 7)?,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/entity/Entity;Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::EntityEvent {
            except: args.first()?,
            target: args.get(1)?,
            sound: args.get(2)?,
            source: args.get(3)?,
            volume: float_arg(args, 4)?,
            pitch: float_arg(args, 5)?,
        }),
        (
            "playSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/Holder;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::EntityHolder {
            except: args.first()?,
            target: args.get(1)?,
            holder: args.get(2)?,
            source: args.get(3)?,
            volume: float_arg(args, 4)?,
            pitch: float_arg(args, 5)?,
        }),
        (
            "playSeededSound",
            "(Lnet/minecraft/world/entity/Entity;DDDLnet/minecraft/core/Holder;Lnet/minecraft/sounds/SoundSource;FFJ)V",
        ) => Some(LevelSoundOp::SeededPosHolder {
            except: args.first()?,
            pos: [
                double_arg(args, 1)?,
                double_arg(args, 2)?,
                double_arg(args, 3)?,
            ],
            holder: args.get(4)?,
            source: args.get(5)?,
            volume: float_arg(args, 6)?,
            pitch: float_arg(args, 7)?,
            seed: long_arg(args, 8)?,
        }),
        (
            "playSeededSound",
            "(Lnet/minecraft/world/entity/Entity;DDDLnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FFJ)V",
        ) => Some(LevelSoundOp::SeededPosEvent {
            except: args.first()?,
            pos: [
                double_arg(args, 1)?,
                double_arg(args, 2)?,
                double_arg(args, 3)?,
            ],
            sound: args.get(4)?,
            source: args.get(5)?,
            volume: float_arg(args, 6)?,
            pitch: float_arg(args, 7)?,
            seed: long_arg(args, 8)?,
        }),
        (
            "playSeededSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/Holder;Lnet/minecraft/sounds/SoundSource;FFJ)V",
        ) => Some(LevelSoundOp::SeededEntityHolder {
            except: args.first()?,
            target: args.get(1)?,
            holder: args.get(2)?,
            source: args.get(3)?,
            volume: float_arg(args, 4)?,
            pitch: float_arg(args, 5)?,
            seed: long_arg(args, 6)?,
        }),
        (
            "playLocalSound",
            "(Lnet/minecraft/core/BlockPos;Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FFZ)V",
        ) => Some(LevelSoundOp::LocalBlockPos {
            pos: args.first()?,
            sound: args.get(1)?,
            source: args.get(2)?,
            volume: float_arg(args, 3)?,
            pitch: float_arg(args, 4)?,
            distance_delay: bool_arg(args, 5)?,
        }),
        (
            "playLocalSound",
            "(DDDLnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FFZ)V",
        ) => Some(LevelSoundOp::LocalPos {
            pos: [
                double_arg(args, 0)?,
                double_arg(args, 1)?,
                double_arg(args, 2)?,
            ],
            sound: args.get(3)?,
            source: args.get(4)?,
            volume: float_arg(args, 5)?,
            pitch: float_arg(args, 6)?,
            distance_delay: bool_arg(args, 7)?,
        }),
        (
            "playLocalSound",
            "(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::LocalEntity {
            target: args.first()?,
            sound: args.get(1)?,
            source: args.get(2)?,
            volume: float_arg(args, 3)?,
            pitch: float_arg(args, 4)?,
        }),
        (
            "playPlayerSound",
            "(Lnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FF)V",
        ) => Some(LevelSoundOp::PlayerSound {
            sound: args.first()?,
            source: args.get(1)?,
            volume: float_arg(args, 2)?,
            pitch: float_arg(args, 3)?,
        }),
        _ => None,
    }
}

/// Resolve `except` for `ServerLevel.playSeededSound` (`except instanceof Player p ? p : null`).
fn server_except_arg(
    value: &Value,
    context: Option<&game_hooks::GameHookContext>,
    entity_facts: &BTreeMap<String, EntitySoundFacts>,
) -> ExceptArg {
    if *value == Value::Ref(None) {
        return ExceptArg::None;
    }
    let Value::Host(handle) = value else {
        return ExceptArg::Unmodelled;
    };
    if handle.kind() != HostKind::GameEntity {
        return ExceptArg::Unmodelled;
    }
    let key = handle.key();
    if let Some(ctx) = context
        && let Some(obj) = ctx.object(key)
    {
        return match obj.kind {
            game_hooks::GameObjectKind::Player => match obj.uuid {
                Some(uuid) => ExceptArg::Player(uuid),
                None => ExceptArg::Unmodelled,
            },
            game_hooks::GameObjectKind::Living
            | game_hooks::GameObjectKind::Entity
            | game_hooks::GameObjectKind::ItemEntity => ExceptArg::NonPlayer,
            _ => ExceptArg::Unmodelled,
        };
    }
    if let Some(uuid) = game_object::player_uuid(key) {
        return ExceptArg::Player(uuid);
    }
    if key.starts_with("game/entity/") {
        if let Some(facts) = entity_facts.get(key)
            && facts.is_player
        {
            return facts
                .player_uuid
                .map_or(ExceptArg::Unmodelled, ExceptArg::Player);
        }
        return ExceptArg::NonPlayer;
    }
    if let Some(facts) = entity_facts.get(key) {
        if facts.is_player {
            return facts
                .player_uuid
                .map_or(ExceptArg::Unmodelled, ExceptArg::Player);
        }
        return ExceptArg::NonPlayer;
    }
    ExceptArg::Unmodelled
}

/// Resolve `target` for entity-bound `ServerLevel` sounds.
fn server_target_entity(
    value: &Value,
    firing: Option<&events::EventFiring>,
    hook_context: Option<&game_hooks::GameHookContext>,
    host_facts: &BTreeMap<String, EntitySoundFacts>,
) -> TargetEntityArg {
    let Value::Host(handle) = value else {
        return TargetEntityArg::Unmodelled;
    };
    if handle.kind() != HostKind::GameEntity {
        return TargetEntityArg::Unmodelled;
    }
    let key = handle.key();
    if let Some(ctx) = hook_context
        && let Some(obj) = ctx.object(key)
    {
        return match obj.entity_id {
            Some(entity_id) => TargetEntityArg::Entity {
                entity_id,
                position: obj.position,
            },
            None => TargetEntityArg::Unmodelled,
        };
    }
    let extra = firing
        .and_then(|f| f.context.entity_sound_facts.get(key))
        .or_else(|| host_facts.get(key));
    if let Some(entity_id) = game_object::entity_id_of(key) {
        let position = extra.and_then(|f| f.position);
        return TargetEntityArg::Entity {
            entity_id,
            position,
        };
    }
    if let Some(uuid) = game_object::player_uuid(key)
        && let Some(firing) = firing
        && let Some(player) = game_object::facade_player(&firing.facade, uuid)
        && let Some(entity_id) = player.entity_id
    {
        return TargetEntityArg::Entity {
            entity_id,
            position: extra.and_then(|f| f.position),
        };
    }
    if let Some(facts) = extra
        && let Some(entity_id) = facts.entity_id
    {
        return TargetEntityArg::Entity {
            entity_id,
            position: facts.position,
        };
    }
    TargetEntityArg::Unmodelled
}

/// Check `ClientLevel.playSeededSound`'s `except == this.minecraft.player` (`if_acmpne`).
fn client_except_matches(except: &Value, has_local_player: bool) -> ClientExceptMatch {
    if !has_local_player {
        return if *except == Value::Ref(None) {
            ClientExceptMatch::MatchesLocalPlayer
        } else {
            ClientExceptMatch::DoesNotMatch
        };
    }
    if *except == Value::Ref(None) {
        return ClientExceptMatch::DoesNotMatch;
    }
    match except {
        Value::Host(handle)
            if handle.kind() == HostKind::GameEntity && handle.key() == client_surface::PLAYER =>
        {
            ClientExceptMatch::MatchesLocalPlayer
        }
        Value::Host(handle) if handle.kind() == HostKind::GameEntity => {
            ClientExceptMatch::DoesNotMatch
        }
        _ => ClientExceptMatch::Unmodelled,
    }
}

fn client_target_entity(
    value: &Value,
    frame: Option<&client_surface::ClientSnapshot>,
    host: &StdHost,
) -> TargetEntityArg {
    let Value::Host(handle) = value else {
        return TargetEntityArg::Unmodelled;
    };
    if handle.kind() != HostKind::GameEntity {
        return TargetEntityArg::Unmodelled;
    }
    let key = handle.key();
    if key == client_surface::PLAYER {
        if let Some(player) = frame.and_then(|f| f.player.as_ref()) {
            return TargetEntityArg::Entity {
                entity_id: player.entity_id,
                position: Some(player.pos),
            };
        }
        if let Some(facts) = host.level_sound.standalone_client_player.as_ref()
            && let Some(entity_id) = facts.entity_id
        {
            return TargetEntityArg::Entity {
                entity_id,
                position: facts.position,
            };
        }
        return TargetEntityArg::Unmodelled;
    }
    if let Some(entity_id) = game_object::entity_id_of(key) {
        let position = host
            .level_sound
            .entity_facts
            .get(key)
            .and_then(|f| f.position);
        return TargetEntityArg::Entity {
            entity_id,
            position,
        };
    }
    if let Some(facts) = host.level_sound.entity_facts.get(key)
        && let Some(entity_id) = facts.entity_id
    {
        return TargetEntityArg::Entity {
            entity_id,
            position: facts.position,
        };
    }
    TargetEntityArg::Unmodelled
}

/// Core `ServerLevel` sound/random dispatcher shared by standalone `StdHost`,
/// `events::EventFiring`, and `game_hooks::GameHookHost`.
#[allow(clippy::too_many_arguments)]
fn execute_server_level_call<H: Host + ?Sized>(
    host: &mut H,
    level_marker: &str,
    dimension: &str,
    r: &MethodRef,
    args: &[Value],
    firing: Option<&events::EventFiring>,
    hook_context: Option<&game_hooks::GameHookContext>,
    host_facts: &BTreeMap<String, EntitySoundFacts>,
    emit: &mut dyn FnMut(LevelSoundEffect),
) -> SoundCallOutcome {
    if r.name == "getRandom" && r.descriptor == "()Lnet/minecraft/util/RandomSource;" {
        return match host.level_random_handle(level_marker) {
            Some(handle) => SoundCallOutcome::Handled(handle),
            None => SoundCallOutcome::Unmodelled,
        };
    }
    let Some(op) = parse_level_op(r, args) else {
        return SoundCallOutcome::Unmodelled;
    };
    match op {
        // `ServerLevel` inherits `Level.playLocalSound` / `playPlayerSound` as no-ops.
        // Only the `BlockPos` overload reads `pos.getX/Y/Z()` before delegating.
        LevelSoundOp::LocalBlockPos { pos, .. } => match block_pos_arg(pos) {
            BlockPosArg::Coords(_) => SoundCallOutcome::Handled(Value::Void),
            BlockPosArg::Null => SoundCallOutcome::Exception(npe(
                r#"Cannot invoke "net.minecraft.core.BlockPos.getX()" because "pos" is null"#,
            )),
            BlockPosArg::Unmodelled => SoundCallOutcome::Unmodelled,
        },
        LevelSoundOp::LocalPos { .. }
        | LevelSoundOp::LocalEntity { .. }
        | LevelSoundOp::PlayerSound { .. } => SoundCallOutcome::Handled(Value::Void),

        LevelSoundOp::BlockPos {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
        } => {
            let [bx, by, bz] = match block_pos_arg(pos) {
                BlockPosArg::Coords(coords) => coords,
                BlockPosArg::Null => {
                    return SoundCallOutcome::Exception(npe(
                        r#"Cannot invoke "net.minecraft.core.BlockPos.getX()" because "pos" is null"#,
                    ));
                }
                BlockPosArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let centered = [
                f64::from(bx) + 0.5,
                f64::from(by) + 0.5,
                f64::from(bz) + 0.5,
            ];
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let Some(resolved) = wrap_sound_event_as_holder(host, sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(seed) = host.draw_level_sound_seed(level_marker, true) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Position(centered),
                source,
                volume,
                pitch,
                except_player,
                seed,
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::PosEvent {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let Some(resolved) = wrap_sound_event_as_holder(host, sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(seed) = host.draw_level_sound_seed(level_marker, true) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Position(pos),
                source,
                volume,
                pitch,
                except_player,
                seed,
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::PosHolder {
            except,
            pos,
            holder,
            source,
            volume,
            pitch,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let resolved = match resolve_sound_holder(host, holder) {
                Ok(Some(resolved)) => resolved,
                Ok(None) => return SoundCallOutcome::Unmodelled,
                Err(err) => {
                    // `Level.playSound(..., Holder, ...)` draws `soundSeedGenerator` before
                    // `ServerLevel.playSeededSound` calls `holder.value()`.
                    if host.draw_level_sound_seed(level_marker, true).is_none() {
                        return SoundCallOutcome::Unmodelled;
                    }
                    return SoundCallOutcome::Exception(err);
                }
            };
            let Some(seed) = host.draw_level_sound_seed(level_marker, true) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Position(pos),
                source,
                volume,
                pitch,
                except_player,
                seed,
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::EntityEvent {
            except,
            target,
            sound,
            source,
            volume,
            pitch,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let TargetEntityArg::Entity {
                entity_id,
                position,
            } = server_target_entity(target, firing, hook_context, host_facts)
            else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(resolved) = wrap_sound_event_as_holder(host, sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(seed) = host.draw_level_sound_seed(level_marker, true) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Entity {
                    entity_id,
                    position,
                },
                source,
                volume,
                pitch,
                except_player,
                seed,
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::EntityHolder {
            except,
            target,
            holder,
            source,
            volume,
            pitch,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let TargetEntityArg::Entity {
                entity_id,
                position,
            } = server_target_entity(target, firing, hook_context, host_facts)
            else {
                return SoundCallOutcome::Unmodelled;
            };
            let resolved = match resolve_sound_holder(host, holder) {
                Ok(Some(resolved)) => resolved,
                Ok(None) => return SoundCallOutcome::Unmodelled,
                Err(err) => {
                    if host.draw_level_sound_seed(level_marker, true).is_none() {
                        return SoundCallOutcome::Unmodelled;
                    }
                    return SoundCallOutcome::Exception(err);
                }
            };
            let Some(seed) = host.draw_level_sound_seed(level_marker, true) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Entity {
                    entity_id,
                    position,
                },
                source,
                volume,
                pitch,
                except_player,
                seed,
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::SeededPosHolder {
            except,
            pos,
            holder,
            source,
            volume,
            pitch,
            seed,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let resolved = match resolve_sound_holder(host, holder) {
                Ok(Some(resolved)) => resolved,
                Ok(None) => return SoundCallOutcome::Unmodelled,
                Err(err) => return SoundCallOutcome::Exception(err),
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Position(pos),
                source,
                volume,
                pitch,
                except_player,
                seed: SoundSeed::Explicit(seed),
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::SeededPosEvent {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
            seed,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let Some(resolved) = wrap_sound_event_as_holder(host, sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Position(pos),
                source,
                volume,
                pitch,
                except_player,
                seed: SoundSeed::Explicit(seed),
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::SeededEntityHolder {
            except,
            target,
            holder,
            source,
            volume,
            pitch,
            seed,
        } => {
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let except_player = match server_except_arg(except, hook_context, host_facts) {
                ExceptArg::None | ExceptArg::NonPlayer => None,
                ExceptArg::Player(uuid) => Some(uuid),
                ExceptArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let TargetEntityArg::Entity {
                entity_id,
                position,
            } = server_target_entity(target, firing, hook_context, host_facts)
            else {
                return SoundCallOutcome::Unmodelled;
            };
            let resolved = match resolve_sound_holder(host, holder) {
                Ok(Some(resolved)) => resolved,
                Ok(None) => return SoundCallOutcome::Unmodelled,
                Err(err) => return SoundCallOutcome::Exception(err),
            };
            emit(LevelSoundEffect {
                side: SoundSide::Server,
                dimension: Some(dimension.to_owned()),
                sound_id: resolved.sound_id,
                fixed_range: resolved.fixed_range,
                holder: resolved.holder,
                target: SoundTarget::Entity {
                    entity_id,
                    position,
                },
                source,
                volume,
                pitch,
                except_player,
                seed: SoundSeed::Explicit(seed),
                distance_delay: false,
                delay_ticks: 0,
            });
            SoundCallOutcome::Handled(Value::Void)
        }
    }
}

/// Core `ClientLevel` sound/random dispatcher for both `ClientSnapshot` frames
/// and standalone `client/level` handles.
fn execute_client_level_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
    frame: Option<&client_surface::ClientSnapshot>,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    let level_marker = client_surface::LEVEL;
    if r.name == "getRandom" && r.descriptor == "()Lnet/minecraft/util/RandomSource;" {
        return match host.level_random_handle(level_marker) {
            Some(handle) => SoundCallOutcome::Handled(handle),
            None => SoundCallOutcome::Unmodelled,
        };
    }
    let Some(op) = parse_level_op(r, args) else {
        return SoundCallOutcome::Unmodelled;
    };
    let dimension = frame
        .and_then(|f| f.level.as_ref())
        .map(|l| l.dimension.clone())
        .filter(|d| !d.is_empty());
    let has_local_player = frame.map_or_else(
        || host.level_sound.standalone_client_player.is_some(),
        |f| f.player.is_some(),
    );
    let camera_pos = frame
        .and_then(|f| f.camera.as_ref().map(|c| c.pos))
        .or(host.level_sound.standalone_client_camera);

    match op {
        LevelSoundOp::BlockPos {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
        } => {
            let [bx, by, bz] = match block_pos_arg(pos) {
                BlockPosArg::Coords(coords) => coords,
                BlockPosArg::Null => {
                    return SoundCallOutcome::Exception(npe(
                        r#"Cannot invoke "net.minecraft.core.BlockPos.getX()" because "pos" is null"#,
                    ));
                }
                BlockPosArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let centered = [
                f64::from(bx) + 0.5,
                f64::from(by) + 0.5,
                f64::from(bz) + 0.5,
            ];
            client_play_seeded_or_unseeded_pos(
                host,
                dimension,
                has_local_player,
                except,
                centered,
                sound,
                false,
                source,
                volume,
                pitch,
                None,
                emit,
            )
        }

        LevelSoundOp::PosEvent {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
        } => client_play_seeded_or_unseeded_pos(
            host,
            dimension,
            has_local_player,
            except,
            pos,
            sound,
            false,
            source,
            volume,
            pitch,
            None,
            emit,
        ),

        LevelSoundOp::PosHolder {
            except,
            pos,
            holder,
            source,
            volume,
            pitch,
        } => client_play_seeded_or_unseeded_pos(
            host,
            dimension,
            has_local_player,
            except,
            pos,
            holder,
            true,
            source,
            volume,
            pitch,
            None,
            emit,
        ),

        LevelSoundOp::EntityEvent {
            except,
            target,
            sound,
            source,
            volume,
            pitch,
        } => client_play_seeded_or_unseeded_entity(
            host,
            frame,
            dimension,
            has_local_player,
            except,
            target,
            sound,
            false,
            source,
            volume,
            pitch,
            None,
            emit,
        ),

        LevelSoundOp::EntityHolder {
            except,
            target,
            holder,
            source,
            volume,
            pitch,
        } => client_play_seeded_or_unseeded_entity(
            host,
            frame,
            dimension,
            has_local_player,
            except,
            target,
            holder,
            true,
            source,
            volume,
            pitch,
            None,
            emit,
        ),

        LevelSoundOp::SeededPosHolder {
            except,
            pos,
            holder,
            source,
            volume,
            pitch,
            seed,
        } => client_play_seeded_or_unseeded_pos(
            host,
            dimension,
            has_local_player,
            except,
            pos,
            holder,
            true,
            source,
            volume,
            pitch,
            Some(seed),
            emit,
        ),

        LevelSoundOp::SeededPosEvent {
            except,
            pos,
            sound,
            source,
            volume,
            pitch,
            seed,
        } => client_play_seeded_or_unseeded_pos(
            host,
            dimension,
            has_local_player,
            except,
            pos,
            sound,
            false,
            source,
            volume,
            pitch,
            Some(seed),
            emit,
        ),

        LevelSoundOp::SeededEntityHolder {
            except,
            target,
            holder,
            source,
            volume,
            pitch,
            seed,
        } => client_play_seeded_or_unseeded_entity(
            host,
            frame,
            dimension,
            has_local_player,
            except,
            target,
            holder,
            true,
            source,
            volume,
            pitch,
            Some(seed),
            emit,
        ),

        LevelSoundOp::LocalBlockPos {
            pos,
            sound,
            source,
            volume,
            pitch,
            distance_delay,
        } => {
            let [bx, by, bz] = match block_pos_arg(pos) {
                BlockPosArg::Coords(coords) => coords,
                BlockPosArg::Null => {
                    return SoundCallOutcome::Exception(npe(
                        r#"Cannot invoke "net.minecraft.core.BlockPos.getX()" because "pos" is null"#,
                    ));
                }
                BlockPosArg::Unmodelled => return SoundCallOutcome::Unmodelled,
            };
            let centered = [
                f64::from(bx) + 0.5,
                f64::from(by) + 0.5,
                f64::from(bz) + 0.5,
            ];
            client_play_local_pos(
                host,
                dimension,
                camera_pos,
                centered,
                sound,
                source,
                volume,
                pitch,
                distance_delay,
                emit,
            )
        }

        LevelSoundOp::LocalPos {
            pos,
            sound,
            source,
            volume,
            pitch,
            distance_delay,
        } => client_play_local_pos(
            host,
            dimension,
            camera_pos,
            pos,
            sound,
            source,
            volume,
            pitch,
            distance_delay,
            emit,
        ),

        LevelSoundOp::LocalEntity {
            target,
            sound,
            source,
            volume,
            pitch,
        } => {
            let Some((sound_id, fixed_range)) = resolve_direct_sound_event(sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let TargetEntityArg::Entity {
                entity_id,
                position,
            } = client_target_entity(target, frame, host)
            else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(seed) = host.draw_client_local_seed() else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(
                host,
                LevelSoundEffect {
                    side: SoundSide::Client,
                    dimension,
                    sound_id,
                    fixed_range,
                    holder: None,
                    target: SoundTarget::Entity {
                        entity_id,
                        position,
                    },
                    source,
                    volume,
                    pitch,
                    except_player: None,
                    seed,
                    distance_delay: false,
                    delay_ticks: 0,
                },
            );
            SoundCallOutcome::Handled(Value::Void)
        }

        LevelSoundOp::PlayerSound {
            sound,
            source,
            volume,
            pitch,
        } => {
            // `ClientLevel.playPlayerSound`: checks `if (this.minecraft.player != null)` first.
            // When `player == null`, returns without drawing `this.random` or reading sound.
            if !has_local_player {
                return SoundCallOutcome::Handled(Value::Void);
            }
            let Some((sound_id, fixed_range)) = resolve_direct_sound_event(sound) else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(source) = SoundSource::from_value(source) else {
                return SoundCallOutcome::Unmodelled;
            };
            let (entity_id, position) = if let Some(player) = frame.and_then(|f| f.player.as_ref())
            {
                (player.entity_id, Some(player.pos))
            } else if let Some(facts) = host.level_sound.standalone_client_player.as_ref() {
                let Some(entity_id) = facts.entity_id else {
                    return SoundCallOutcome::Unmodelled;
                };
                (entity_id, facts.position)
            } else {
                return SoundCallOutcome::Unmodelled;
            };
            let Some(seed) = host.draw_client_local_seed() else {
                return SoundCallOutcome::Unmodelled;
            };
            emit(
                host,
                LevelSoundEffect {
                    side: SoundSide::Client,
                    dimension,
                    sound_id,
                    fixed_range,
                    holder: None,
                    target: SoundTarget::Entity {
                        entity_id,
                        position,
                    },
                    source,
                    volume,
                    pitch,
                    except_player: None,
                    seed,
                    distance_delay: false,
                    delay_ticks: 0,
                },
            );
            SoundCallOutcome::Handled(Value::Void)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn client_play_seeded_or_unseeded_pos(
    host: &mut StdHost,
    dimension: Option<String>,
    has_local_player: bool,
    except: &Value,
    pos: [f64; 3],
    sound_or_holder: &Value,
    is_holder: bool,
    source: &Value,
    volume: f32,
    pitch: f32,
    explicit_seed: Option<i64>,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    let Some(source) = SoundSource::from_value(source) else {
        return SoundCallOutcome::Unmodelled;
    };
    let matches = client_except_matches(except, has_local_player);
    match matches {
        ClientExceptMatch::Unmodelled => return SoundCallOutcome::Unmodelled,
        ClientExceptMatch::DoesNotMatch => {
            if !is_holder && resolve_direct_sound_event(sound_or_holder).is_none() {
                return SoundCallOutcome::Unmodelled;
            }
            // `Level.playSound` draws `soundSeedGenerator.nextLong()` BEFORE invoking
            // `ClientLevel.playSeededSound`, which then checks `except == minecraft.player`.
            if explicit_seed.is_none()
                && host
                    .draw_level_sound_seed(client_surface::LEVEL, false)
                    .is_none()
            {
                return SoundCallOutcome::Unmodelled;
            }
            return SoundCallOutcome::Handled(Value::Void);
        }
        ClientExceptMatch::MatchesLocalPlayer => {}
    }
    let resolved = if is_holder {
        match resolve_sound_holder(host, sound_or_holder) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => return SoundCallOutcome::Unmodelled,
            Err(err) => {
                if explicit_seed.is_none()
                    && host
                        .draw_level_sound_seed(client_surface::LEVEL, false)
                        .is_none()
                {
                    return SoundCallOutcome::Unmodelled;
                }
                return SoundCallOutcome::Exception(err);
            }
        }
    } else {
        match wrap_sound_event_as_holder(host, sound_or_holder) {
            Some(resolved) => resolved,
            None => return SoundCallOutcome::Unmodelled,
        }
    };
    let seed = match explicit_seed {
        Some(seed) => SoundSeed::Explicit(seed),
        None => match host.draw_level_sound_seed(client_surface::LEVEL, false) {
            Some(seed) => seed,
            None => return SoundCallOutcome::Unmodelled,
        },
    };
    emit(
        host,
        LevelSoundEffect {
            side: SoundSide::Client,
            dimension,
            sound_id: resolved.sound_id,
            fixed_range: resolved.fixed_range,
            holder: resolved.holder,
            target: SoundTarget::Position(pos),
            source,
            volume,
            pitch,
            except_player: None,
            seed,
            distance_delay: false,
            delay_ticks: 0,
        },
    );
    SoundCallOutcome::Handled(Value::Void)
}

#[allow(clippy::too_many_arguments)]
fn client_play_seeded_or_unseeded_entity(
    host: &mut StdHost,
    frame: Option<&client_surface::ClientSnapshot>,
    dimension: Option<String>,
    has_local_player: bool,
    except: &Value,
    target: &Value,
    sound_or_holder: &Value,
    is_holder: bool,
    source: &Value,
    volume: f32,
    pitch: f32,
    explicit_seed: Option<i64>,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    let Some(source) = SoundSource::from_value(source) else {
        return SoundCallOutcome::Unmodelled;
    };
    let matches = client_except_matches(except, has_local_player);
    match matches {
        ClientExceptMatch::Unmodelled => return SoundCallOutcome::Unmodelled,
        ClientExceptMatch::DoesNotMatch => {
            if !is_holder && resolve_direct_sound_event(sound_or_holder).is_none() {
                return SoundCallOutcome::Unmodelled;
            }
            if explicit_seed.is_none()
                && host
                    .draw_level_sound_seed(client_surface::LEVEL, false)
                    .is_none()
            {
                return SoundCallOutcome::Unmodelled;
            }
            return SoundCallOutcome::Handled(Value::Void);
        }
        ClientExceptMatch::MatchesLocalPlayer => {}
    }
    let TargetEntityArg::Entity {
        entity_id,
        position,
    } = client_target_entity(target, frame, host)
    else {
        return SoundCallOutcome::Unmodelled;
    };
    let resolved = if is_holder {
        match resolve_sound_holder(host, sound_or_holder) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => return SoundCallOutcome::Unmodelled,
            Err(err) => {
                if explicit_seed.is_none()
                    && host
                        .draw_level_sound_seed(client_surface::LEVEL, false)
                        .is_none()
                {
                    return SoundCallOutcome::Unmodelled;
                }
                return SoundCallOutcome::Exception(err);
            }
        }
    } else {
        match wrap_sound_event_as_holder(host, sound_or_holder) {
            Some(resolved) => resolved,
            None => return SoundCallOutcome::Unmodelled,
        }
    };
    let seed = match explicit_seed {
        Some(seed) => SoundSeed::Explicit(seed),
        None => match host.draw_level_sound_seed(client_surface::LEVEL, false) {
            Some(seed) => seed,
            None => return SoundCallOutcome::Unmodelled,
        },
    };
    emit(
        host,
        LevelSoundEffect {
            side: SoundSide::Client,
            dimension,
            sound_id: resolved.sound_id,
            fixed_range: resolved.fixed_range,
            holder: resolved.holder,
            target: SoundTarget::Entity {
                entity_id,
                position,
            },
            source,
            volume,
            pitch,
            except_player: None,
            seed,
            distance_delay: false,
            delay_ticks: 0,
        },
    );
    SoundCallOutcome::Handled(Value::Void)
}

#[allow(clippy::too_many_arguments)]
fn client_play_local_pos(
    host: &mut StdHost,
    dimension: Option<String>,
    camera_pos: Option<[f64; 3]>,
    pos: [f64; 3],
    sound: &Value,
    source: &Value,
    volume: f32,
    pitch: f32,
    distance_delay: bool,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    let Some((sound_id, fixed_range)) = resolve_direct_sound_event(sound) else {
        return SoundCallOutcome::Unmodelled;
    };
    let Some(source) = SoundSource::from_value(source) else {
        return SoundCallOutcome::Unmodelled;
    };
    let delay_ticks = if distance_delay {
        let Some(cam) = camera_pos else {
            return SoundCallOutcome::Unmodelled;
        };
        client_delay_ticks(cam, pos)
    } else {
        0
    };
    let Some(seed) = host.draw_client_local_seed() else {
        return SoundCallOutcome::Unmodelled;
    };
    emit(
        host,
        LevelSoundEffect {
            side: SoundSide::Client,
            dimension,
            sound_id,
            fixed_range,
            holder: None,
            target: SoundTarget::Position(pos),
            source,
            volume,
            pitch,
            except_player: None,
            seed,
            distance_delay,
            delay_ticks,
        },
    );
    SoundCallOutcome::Handled(Value::Void)
}

/// Dispatch `Entity.playSound`, `Player.playSound`, `Entity.getSoundSource`, and
/// `Entity.isSilent` for a server-side entity or player.
#[allow(clippy::too_many_arguments)]
fn execute_server_entity_call<H: Host + ?Sized>(
    host: &mut H,
    r: &MethodRef,
    args: &[Value],
    is_player: bool,
    player_uuid: Option<u128>,
    dimension: Option<&str>,
    position: Option<[f64; 3]>,
    silent: Option<bool>,
    sound_source: Option<SoundSource>,
    emit: &mut dyn FnMut(LevelSoundEffect),
) -> SoundCallOutcome {
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("getSoundSource", "()Lnet/minecraft/sounds/SoundSource;") => {
            let source = if is_player {
                Some(SoundSource::Players)
            } else {
                sound_source
            };
            match source {
                Some(source) => SoundCallOutcome::Handled(source.to_value()),
                None => SoundCallOutcome::Unmodelled,
            }
        }
        ("isSilent", "()Z") => match silent {
            Some(silent) => SoundCallOutcome::Handled(Value::Int(i32::from(silent))),
            None => SoundCallOutcome::Unmodelled,
        },
        // `Entity.playSound(SoundEvent)`: checks `!this.isSilent()` BEFORE calling
        // virtual `this.playSound(sound, 1.0f, 1.0f)` (even on `Player`!).
        ("playSound", "(Lnet/minecraft/sounds/SoundEvent;)V") => {
            let Some(is_silent) = silent else {
                return SoundCallOutcome::Unmodelled;
            };
            if is_silent {
                return SoundCallOutcome::Handled(Value::Void);
            }
            let Some(sound) = args.first() else {
                return SoundCallOutcome::Unmodelled;
            };
            server_entity_emit_play_sound(
                host,
                is_player,
                player_uuid,
                dimension,
                position,
                sound_source,
                sound,
                1.0,
                1.0,
                emit,
            )
        }
        // `Player.playSound(SoundEvent, float, float)` does NOT check `isSilent()`;
        // `Entity.playSound(SoundEvent, float, float)` DOES check `!isSilent()`.
        ("playSound", "(Lnet/minecraft/sounds/SoundEvent;FF)V") => {
            if !is_player {
                let Some(is_silent) = silent else {
                    return SoundCallOutcome::Unmodelled;
                };
                if is_silent {
                    return SoundCallOutcome::Handled(Value::Void);
                }
            }
            let (Some(sound), Some(volume), Some(pitch)) =
                (args.first(), float_arg(args, 1), float_arg(args, 2))
            else {
                return SoundCallOutcome::Unmodelled;
            };
            server_entity_emit_play_sound(
                host,
                is_player,
                player_uuid,
                dimension,
                position,
                sound_source,
                sound,
                volume,
                pitch,
                emit,
            )
        }
        _ => SoundCallOutcome::Unmodelled,
    }
}

#[allow(clippy::too_many_arguments)]
fn server_entity_emit_play_sound<H: Host + ?Sized>(
    host: &mut H,
    is_player: bool,
    player_uuid: Option<u128>,
    dimension: Option<&str>,
    position: Option<[f64; 3]>,
    sound_source: Option<SoundSource>,
    sound: &Value,
    volume: f32,
    pitch: f32,
    emit: &mut dyn FnMut(LevelSoundEffect),
) -> SoundCallOutcome {
    let Some(dimension) = dimension else {
        return SoundCallOutcome::Unmodelled;
    };
    let Some(pos) = position else {
        return SoundCallOutcome::Unmodelled;
    };
    let source = if is_player {
        SoundSource::Players
    } else {
        let Some(source) = sound_source else {
            return SoundCallOutcome::Unmodelled;
        };
        source
    };
    let except_player = if is_player {
        let Some(uuid) = player_uuid else {
            return SoundCallOutcome::Unmodelled;
        };
        Some(uuid)
    } else {
        None
    };
    let Some(resolved) = wrap_sound_event_as_holder(host, sound) else {
        return SoundCallOutcome::Unmodelled;
    };
    let level_marker = format!("game/level/{dimension}");
    let Some(seed) = host.draw_level_sound_seed(&level_marker, true) else {
        return SoundCallOutcome::Unmodelled;
    };
    emit(LevelSoundEffect {
        side: SoundSide::Server,
        dimension: Some(dimension.to_owned()),
        sound_id: resolved.sound_id,
        fixed_range: resolved.fixed_range,
        holder: resolved.holder,
        target: SoundTarget::Position(pos),
        source,
        volume,
        pitch,
        except_player,
        seed,
        distance_delay: false,
        delay_ticks: 0,
    });
    SoundCallOutcome::Handled(Value::Void)
}

/// Dispatch `LocalPlayer.playSound`, `getSoundSource`, and `isSilent`.
fn execute_local_player_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
    frame: Option<&client_surface::ClientSnapshot>,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    let (has_level, dimension, pos, silent) = if let Some(frame) = frame {
        let Some(player) = frame.player.as_ref() else {
            return SoundCallOutcome::Unmodelled;
        };
        (
            frame.level.is_some(),
            frame
                .level
                .as_ref()
                .map(|l| l.dimension.clone())
                .filter(|d| !d.is_empty()),
            Some(player.pos),
            player.silent,
        )
    } else if let Some(facts) = host.level_sound.standalone_client_player.as_ref() {
        (
            facts.level.is_some(),
            facts.level.clone(),
            facts.position,
            facts.silent,
        )
    } else {
        return SoundCallOutcome::Unmodelled;
    };
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("getSoundSource", "()Lnet/minecraft/sounds/SoundSource;") => {
            SoundCallOutcome::Handled(SoundSource::Players.to_value())
        }
        ("isSilent", "()Z") => match silent {
            Some(silent) => SoundCallOutcome::Handled(Value::Int(i32::from(silent))),
            None => SoundCallOutcome::Unmodelled,
        },
        // `Entity.playSound(SoundEvent)` checks `!isSilent()`, then calls
        // virtual `LocalPlayer.playSound(sound, 1.0f, 1.0f)`.
        ("playSound", "(Lnet/minecraft/sounds/SoundEvent;)V") => {
            let Some(is_silent) = silent else {
                return SoundCallOutcome::Unmodelled;
            };
            if is_silent {
                return SoundCallOutcome::Handled(Value::Void);
            }
            let Some(sound) = args.first() else {
                return SoundCallOutcome::Unmodelled;
            };
            local_player_emit_play_sound(host, has_level, dimension, pos, sound, 1.0, 1.0, emit)
        }
        // `LocalPlayer.playSound(SoundEvent, float, float)` does NOT check `isSilent()`,
        // and calls `level.playLocalSound(getX(), getY(), getZ(), sound, getSoundSource(), vol, pitch, false)`.
        ("playSound", "(Lnet/minecraft/sounds/SoundEvent;FF)V") => {
            let (Some(sound), Some(volume), Some(pitch)) =
                (args.first(), float_arg(args, 1), float_arg(args, 2))
            else {
                return SoundCallOutcome::Unmodelled;
            };
            local_player_emit_play_sound(
                host, has_level, dimension, pos, sound, volume, pitch, emit,
            )
        }
        _ => SoundCallOutcome::Unmodelled,
    }
}

#[allow(clippy::too_many_arguments)]
fn local_player_emit_play_sound(
    host: &mut StdHost,
    has_level: bool,
    dimension: Option<String>,
    pos: Option<[f64; 3]>,
    sound: &Value,
    volume: f32,
    pitch: f32,
    emit: &mut dyn FnMut(&mut StdHost, LevelSoundEffect),
) -> SoundCallOutcome {
    if !has_level {
        return SoundCallOutcome::Unmodelled;
    }
    let Some(pos) = pos else {
        return SoundCallOutcome::Unmodelled;
    };
    let Some((sound_id, fixed_range)) = resolve_direct_sound_event(sound) else {
        return SoundCallOutcome::Unmodelled;
    };
    let Some(seed) = host.draw_client_local_seed() else {
        return SoundCallOutcome::Unmodelled;
    };
    emit(
        host,
        LevelSoundEffect {
            side: SoundSide::Client,
            dimension,
            sound_id,
            fixed_range,
            holder: None,
            target: SoundTarget::Position(pos),
            source: SoundSource::Players,
            volume,
            pitch,
            except_player: None,
            seed,
            distance_delay: false,
            delay_ticks: 0,
        },
    );
    SoundCallOutcome::Handled(Value::Void)
}

/// Entry point for `client_surface.rs` inside an active `ClientSnapshot` frame.
pub(crate) fn client_frame_call(
    host: &mut StdHost,
    object: &str,
    frame: &client_surface::ClientSnapshot,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let outcome = match object {
        client_surface::LEVEL if is_level_sound_method_name(&r.name) => {
            frame.level.as_ref()?;
            let mut recorded = Vec::new();
            let outcome =
                execute_client_level_call(host, r, args, Some(frame), &mut |_, effect| {
                    recorded.push(effect);
                });
            host.client.sounds.extend(recorded);
            outcome
        }
        client_surface::PLAYER if is_entity_sound_method_name(&r.name) => {
            frame.player.as_ref()?;
            let mut recorded = Vec::new();
            let outcome =
                execute_local_player_call(host, r, args, Some(frame), &mut |_, effect| {
                    recorded.push(effect);
                });
            host.client.sounds.extend(recorded);
            outcome
        }
        _ => return None,
    };
    Some(match outcome {
        SoundCallOutcome::Handled(value) => Ok(value),
        SoundCallOutcome::Exception(err) => Err(err),
        SoundCallOutcome::Unmodelled => Err(host.refuse_client_sound_call(r)),
    })
}

/// Entry point for `events.rs` (both inside an active `EventFiring` and for
/// standalone `game/level/<dim>`, `game/player/...`, `game/entity/...` handles).
pub(crate) fn server_facade_sound_call(
    host: &mut StdHost,
    marker: &str,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let in_firing = host.game_event.is_some();
    if let Some(dimension) = marker.strip_prefix("game/level/") {
        if !is_level_sound_method_name(&r.name) {
            return None;
        }
        let firing_snapshot = host.game_event.as_deref().cloned();
        let host_facts = host.level_sound.entity_facts.clone();
        let mut recorded = Vec::new();
        let outcome = execute_server_level_call(
            host,
            marker,
            dimension,
            r,
            args,
            firing_snapshot.as_ref(),
            None,
            &host_facts,
            &mut |effect| recorded.push(effect),
        );
        if let Some(firing) = host.game_event.as_deref_mut() {
            firing.sounds.extend(recorded);
        } else {
            host.level_sound.played_sounds.extend(recorded);
        }
        return Some(finish_server_outcome(host, in_firing, r, outcome));
    }

    if !is_entity_sound_method_name(&r.name) {
        return None;
    }

    if let Some(uuid) = game_object::player_uuid(marker) {
        let firing_snapshot = host.game_event.as_deref().cloned();
        let extra = firing_snapshot
            .as_ref()
            .and_then(|f| f.context.entity_sound_facts.get(marker))
            .or_else(|| host.level_sound.entity_facts.get(marker))
            .cloned();
        let facade_level = firing_snapshot
            .as_ref()
            .and_then(|f| game_object::facade_player(&f.facade, uuid))
            .and_then(|p| p.level.clone());
        let dimension = extra
            .as_ref()
            .and_then(|e| e.level.clone())
            .or(facade_level);
        let position = extra.as_ref().and_then(|e| e.position);
        let silent = extra.as_ref().and_then(|e| e.silent);
        let mut recorded = Vec::new();
        let outcome = execute_server_entity_call(
            host,
            r,
            args,
            true,
            Some(uuid),
            dimension.as_deref(),
            position,
            silent,
            Some(SoundSource::Players),
            &mut |effect| recorded.push(effect),
        );
        if let Some(firing) = host.game_event.as_deref_mut() {
            firing.sounds.extend(recorded);
        } else {
            host.level_sound.played_sounds.extend(recorded);
        }
        return Some(finish_server_outcome(host, in_firing, r, outcome));
    }

    if let Some(id_str) = marker.strip_prefix("game/entity/")
        && let Ok(entity_id) = id_str.parse::<i32>()
    {
        let firing_snapshot = host.game_event.as_deref().cloned();
        let extra = firing_snapshot
            .as_ref()
            .and_then(|f| f.context.entity_sound_facts.get(marker))
            .or_else(|| host.level_sound.entity_facts.get(marker))
            .cloned();
        let facade_entity = firing_snapshot
            .as_ref()
            .and_then(|f| f.context.entity.as_ref())
            .filter(|e| e.id == entity_id);
        let dimension = extra
            .as_ref()
            .and_then(|e| e.level.clone())
            .or_else(|| facade_entity.and_then(|e| e.level.clone()));
        let position = extra.as_ref().and_then(|e| e.position);
        let silent = extra.as_ref().and_then(|e| e.silent);
        let sound_source = extra
            .as_ref()
            .and_then(|e| e.sound_source)
            .or_else(|| facade_entity.and_then(|e| vanilla_entity_sound_source(&e.kind)));
        let mut recorded = Vec::new();
        let outcome = execute_server_entity_call(
            host,
            r,
            args,
            false,
            None,
            dimension.as_deref(),
            position,
            silent,
            sound_source,
            &mut |effect| recorded.push(effect),
        );
        if let Some(firing) = host.game_event.as_deref_mut() {
            firing.sounds.extend(recorded);
        } else {
            host.level_sound.played_sounds.extend(recorded);
        }
        return Some(finish_server_outcome(host, in_firing, r, outcome));
    }

    None
}

fn finish_server_outcome(
    host: &mut StdHost,
    in_firing: bool,
    r: &MethodRef,
    outcome: SoundCallOutcome,
) -> Result<Value, NativeError> {
    match outcome {
        SoundCallOutcome::Handled(value) => Ok(value),
        SoundCallOutcome::Exception(err) => Err(err),
        SoundCallOutcome::Unmodelled => {
            if in_firing {
                let _ = host.unknown_host_call(r);
                Err(NativeError::UnknownHostCall(r.clone()))
            } else {
                host.unknown_host_call(r)
            }
        }
    }
}

/// Entry point for standalone `client/level` and `client/player` handles outside
/// an active `ClientSnapshot` frame (e.g. direct `NativeJar::invoke_method` fixtures).
pub(crate) fn standalone_client_sound_call(
    host: &mut StdHost,
    receiver: &Value,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if host.client.frame.is_some() {
        return None;
    }
    let Value::Host(handle) = receiver else {
        return None;
    };
    let key = handle.key();
    let outcome = match (handle.kind(), key) {
        (HostKind::GameLevel, client_surface::LEVEL)
            if r.name == "isClientSide" && r.descriptor == "()Z" =>
        {
            SoundCallOutcome::Handled(Value::Int(1))
        }
        (HostKind::GameLevel, client_surface::LEVEL) if is_level_sound_method_name(&r.name) => {
            execute_client_level_call(host, r, args, None, &mut |host, effect| {
                host.level_sound.played_sounds.push(effect);
            })
        }
        (HostKind::GameEntity, client_surface::PLAYER)
            if matches!(r.name.as_str(), "getX" | "getY" | "getZ") && r.descriptor == "()D" =>
        {
            let axis = match r.name.as_str() {
                "getX" => 0,
                "getY" => 1,
                _ => 2,
            };
            match host
                .level_sound
                .standalone_client_player
                .as_ref()
                .and_then(|facts| facts.position)
            {
                Some(pos) => SoundCallOutcome::Handled(Value::Float(pos[axis])),
                None => SoundCallOutcome::Unmodelled,
            }
        }
        (HostKind::GameEntity, client_surface::PLAYER) if is_entity_sound_method_name(&r.name) => {
            execute_local_player_call(host, r, args, None, &mut |host, effect| {
                host.level_sound.played_sounds.push(effect);
            })
        }
        _ => return None,
    };
    Some(match outcome {
        SoundCallOutcome::Handled(value) => Ok(value),
        SoundCallOutcome::Exception(err) => Err(err),
        SoundCallOutcome::Unmodelled => host.unknown_host_call(r),
    })
}

/// Entry point for `game_hooks.rs::GameHookHost::answer`.
pub(crate) fn game_hook_sound_call<H: Host>(
    inner: &mut H,
    context: &mut game_hooks::GameHookContext,
    marker: &str,
    object: &game_hooks::GameObject,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let empty_facts = BTreeMap::new();
    match object.kind {
        game_hooks::GameObjectKind::Level if is_level_sound_method_name(&r.name) => {
            let dimension = object.dimension.as_deref()?;
            let mut recorded = Vec::new();
            let outcome = execute_server_level_call(
                inner,
                marker,
                dimension,
                r,
                args,
                None,
                Some(context),
                &empty_facts,
                &mut |effect| recorded.push(effect),
            );
            for effect in recorded {
                context.push_effect(game_hooks::GameHookEffect::PlaySound(effect));
            }
            Some(finish_game_hook_outcome(inner, marker, r, outcome))
        }
        game_hooks::GameObjectKind::Player
        | game_hooks::GameObjectKind::Living
        | game_hooks::GameObjectKind::Entity
        | game_hooks::GameObjectKind::ItemEntity
            if is_entity_sound_method_name(&r.name) =>
        {
            let is_player = object.kind == game_hooks::GameObjectKind::Player;
            let sound_source = if is_player {
                Some(SoundSource::Players)
            } else if object.kind == game_hooks::GameObjectKind::ItemEntity {
                Some(SoundSource::Ambient)
            } else {
                object.sound_source.or_else(|| {
                    object
                        .type_id
                        .as_deref()
                        .and_then(vanilla_entity_sound_source)
                })
            };
            let mut recorded = Vec::new();
            let outcome = execute_server_entity_call(
                inner,
                r,
                args,
                is_player,
                object.uuid,
                object.dimension.as_deref(),
                object.position,
                object.silent,
                sound_source,
                &mut |effect| recorded.push(effect),
            );
            for effect in recorded {
                context.push_effect(game_hooks::GameHookEffect::PlaySound(effect));
            }
            Some(finish_game_hook_outcome(inner, marker, r, outcome))
        }
        _ => None,
    }
}

fn finish_game_hook_outcome<H: Host>(
    inner: &mut H,
    marker: &str,
    r: &MethodRef,
    outcome: SoundCallOutcome,
) -> Result<Value, NativeError> {
    match outcome {
        SoundCallOutcome::Handled(value) => Ok(value),
        SoundCallOutcome::Exception(err) => Err(err),
        SoundCallOutcome::Unmodelled => {
            let _ = inner.unknown_operand_call(r);
            Err(NativeError::Metadata(format!(
                "game hook object {marker} does not model {}#{}{}",
                r.owner, r.name, r.descriptor
            )))
        }
    }
}

impl StdHost {
    /// Handle for a server level of `dimension` (`game/level/<dimension>`).
    pub fn server_level_handle(dimension: &str) -> Value {
        game_object::server_value(format!("game/level/{dimension}"))
    }

    /// Handle for the client level (`client/level`).
    pub fn client_level_handle() -> Value {
        game_object::client_value(client_surface::LEVEL.to_owned())
    }

    /// Handle for the local client player (`client/player`).
    pub fn client_player_handle() -> Value {
        game_object::client_value(client_surface::PLAYER.to_owned())
    }

    /// Seed `Level.random` (`Level.getRandom()`, and the seed stream used by
    /// `ClientLevel.playLocalSound` / `playPlayerSound`) for `level_marker`.
    pub fn seed_level_random(&mut self, level_marker: &str, seed: i64) {
        let handle = self.make_random_handle(random_source::legacy_state(seed));
        self.level_sound
            .level_random_handles
            .insert(level_marker.to_owned(), handle);
    }

    /// Seed `Level.soundSeedGenerator` (used by non-seeded `Level.playSound`
    /// overloads on both `ServerLevel` and `ClientLevel`) for `level_marker`.
    pub fn seed_level_sound_seeds(&mut self, level_marker: &str, seed: i64) {
        self.level_sound
            .level_sound_seeds
            .insert(level_marker.to_owned(), random_source::legacy_state(seed));
    }

    /// Seed both `random` and `soundSeedGenerator` for a server level dimension.
    pub fn seed_server_level_randoms(
        &mut self,
        dimension: &str,
        random_seed: i64,
        sound_seed: i64,
    ) {
        let marker = format!("game/level/{dimension}");
        self.seed_level_random(&marker, random_seed);
        self.seed_level_sound_seeds(&marker, sound_seed);
    }

    /// Seed `soundSeedGenerator` only for a server level dimension.
    pub fn seed_server_level_sound_seeds(&mut self, dimension: &str, sound_seed: i64) {
        self.seed_level_sound_seeds(&format!("game/level/{dimension}"), sound_seed);
    }

    /// Seed `random` only for a server level dimension.
    pub fn seed_server_level_random(&mut self, dimension: &str, random_seed: i64) {
        self.seed_level_random(&format!("game/level/{dimension}"), random_seed);
    }

    /// Seed both `random` and `soundSeedGenerator` for `ClientLevel`.
    pub fn seed_client_level_randoms(&mut self, random_seed: i64, sound_seed: i64) {
        self.seed_level_random(client_surface::LEVEL, random_seed);
        self.seed_level_sound_seeds(client_surface::LEVEL, sound_seed);
    }

    /// Seed `random` only for `ClientLevel`.
    pub fn seed_client_level_random(&mut self, random_seed: i64) {
        self.seed_level_random(client_surface::LEVEL, random_seed);
    }

    /// Seed `soundSeedGenerator` only for `ClientLevel`.
    pub fn seed_client_level_sound_seeds(&mut self, sound_seed: i64) {
        self.seed_level_sound_seeds(client_surface::LEVEL, sound_seed);
    }

    /// Allow server-side `Level.playSound` to defer its `soundSeedGenerator`
    /// draw to a live host (such as Sunrise's `LevelRef::play_sound`, which
    /// draws from the server's real `sound_seeds` stream). Off by default so
    /// unseeded standalone runs count unreachable random draws.
    pub fn set_host_deferred_server_sound_seeds(&mut self, enabled: bool) {
        self.level_sound.host_deferred_server_seeds = enabled;
    }

    /// Allow client-side sound calls to defer their seed draw to a live client
    /// host (`ClientHost::play_local_sound`). Off by default.
    pub fn set_host_deferred_client_sound_seeds(&mut self, enabled: bool) {
        self.level_sound.host_deferred_client_seeds = enabled;
    }

    /// Record optional sound facts for an entity or player marker (used by
    /// standalone calls and host bridges).
    pub fn set_entity_sound_facts(&mut self, marker: impl Into<String>, facts: EntitySoundFacts) {
        self.level_sound.entity_facts.insert(marker.into(), facts);
    }

    /// Clear all standalone entity sound facts.
    pub fn clear_entity_sound_facts(&mut self) {
        self.level_sound.entity_facts.clear();
    }

    /// Configure standalone `ClientLevel` camera position (outside a `ClientSnapshot` frame).
    pub fn set_standalone_client_camera(&mut self, pos: Option<[f64; 3]>) {
        self.level_sound.standalone_client_camera = pos;
    }

    /// Configure standalone `ClientLevel` local player facts (outside a `ClientSnapshot` frame).
    pub fn set_standalone_client_player(&mut self, player: Option<EntitySoundFacts>) {
        self.level_sound.standalone_client_player = player;
    }

    /// Standalone sounds recorded outside an `EventFiring`, `GameHookContext`,
    /// or `ClientSnapshot` frame.
    pub fn played_sounds(&self) -> &[LevelSoundEffect] {
        &self.level_sound.played_sounds
    }

    /// Drain standalone recorded sounds.
    pub fn take_played_sounds(&mut self) -> Vec<LevelSoundEffect> {
        std::mem::take(&mut self.level_sound.played_sounds)
    }

    pub(crate) fn draw_client_local_seed(&mut self) -> Option<SoundSeed> {
        if let Some(handle) = self
            .level_sound
            .level_random_handles
            .get(client_surface::LEVEL)
            .cloned()
            && let Some(seed) = self.draw_random_handle_long(&handle)
        {
            return Some(SoundSeed::LevelRandom(seed));
        }
        if self.level_sound.host_deferred_client_seeds {
            return Some(SoundSeed::HostDeferred);
        }
        None
    }

    pub(crate) fn draw_level_sound_seed_impl(
        &mut self,
        level_marker: &str,
        server_side: bool,
    ) -> Option<SoundSeed> {
        if let Some(state) = self.level_sound.level_sound_seeds.get_mut(level_marker) {
            return Some(SoundSeed::LevelSeedGenerator(
                random_source::draw_next_long(state),
            ));
        }
        let deferred = if server_side {
            self.level_sound.host_deferred_server_seeds
        } else {
            self.level_sound.host_deferred_client_seeds
        };
        deferred.then_some(SoundSeed::HostDeferred)
    }

    pub(crate) fn level_random_handle_impl(&self, level_marker: &str) -> Option<Value> {
        self.level_sound
            .level_random_handles
            .get(level_marker)
            .cloned()
    }

    pub(crate) fn registered_sound_id_impl(&self, sound_value: &Value) -> Option<String> {
        self.registrations
            .iter()
            .rev()
            .find(|entry| entry.registry == "minecraft:sound_event" && entry.value == *sound_value)
            .map(|entry| entry.id.clone())
    }

    pub(crate) fn refuse_client_sound_call(&mut self, r: &MethodRef) -> NativeError {
        let key = format!("{}#{}{}", r.owner, r.name, r.descriptor);
        *self.unknown_host_calls.entry(key).or_default() += 1;
        NativeError::UnknownHostCall(r.clone())
    }
}

/// Fast check for whether `r` takes a `BlockPos` argument that may be a
/// jar-allocated `BlockPos` / `MutableBlockPos` / `Vec3i` object.
#[inline]
pub(crate) fn needs_block_pos_resolution(r: &MethodRef, args: &[Value]) -> bool {
    let pos_index = if r.name == "playSound"
        && r.descriptor
            .starts_with("(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/BlockPos;")
    {
        1
    } else if r.name == "playLocalSound"
        && r.descriptor.starts_with("(Lnet/minecraft/core/BlockPos;")
    {
        0
    } else {
        return false;
    };
    args.get(pos_index)
        .is_some_and(crate::runtime::jar_object::is_object)
}

/// Resolve a jar-constructed `BlockPos` / `MutableBlockPos` / `Vec3i` argument
/// in `Level.playSound` / `Level.playLocalSound` into a `HostKind::GameValue`
/// `block-pos/x,y,z` handle before forwarding to the host.
///
/// Kept `#[inline(never)]` so `Runtime::invoke_frame` does not grow its stack.
#[inline(never)]
pub(super) fn resolve_block_pos_arg<H: Host>(
    runtime: &crate::runtime::Runtime<'_, H>,
    r: &MethodRef,
    args: &mut [Value],
) {
    let pos_index = if r.name == "playSound"
        && r.descriptor
            .starts_with("(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/core/BlockPos;")
    {
        1
    } else if r.name == "playLocalSound"
        && r.descriptor.starts_with("(Lnet/minecraft/core/BlockPos;")
    {
        0
    } else {
        return;
    };
    let Some(slot) = args.get_mut(pos_index) else {
        return;
    };
    if !crate::runtime::jar_object::is_object(slot) {
        return;
    }
    if let Some(Some(key)) = runtime.jar_server_value_key(slot)
        && key.starts_with(crate::runtime::server_values::BLOCK_POS)
        && let Some(handle) = game_object::value(&key)
    {
        *slot = handle;
    }
}

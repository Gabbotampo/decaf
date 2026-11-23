//! Bridge from the runtime's recorded [`LevelSoundEffect`]s to Sunrise's
//! server, transactional level-write, hit, and client sound host APIs.
//!
//! Soundness rule: every effect whose parameters Sunrise's host trait can
//! express faithfully is delivered in call order; every effect that carries
//! semantics Sunrise cannot represent faithfully (custom fixed range, explicit
//! caller-supplied seed, entity-bound with an excluded player, client delay
//! past 1200 ticks, missing target level/entity/player, or host refusal) is
//! reported to `on_unbridged` so the jar counts it as unknown instead of
//! silently approximating or dropping it.
//!
//! Seed streams: Java draws a sound's seed from its level's stream at the
//! call; Sunrise draws a deferred effect's seed only when the effect reaches
//! it. A refused deferred effect therefore leaves Sunrise's stream one draw
//! behind Java's. Sunrise keeps one such stream for the whole server (Java
//! keeps one per level), so every level's stream is marked unknown for every
//! jar, and every later deferred effect is counted instead of played with the
//! wrong seed ([`deliver_in_order`], [`gather_unknown_sound_streams`]).

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use mod_api::{
    ClientHost, ClientSetupContext, EntityRef, HitHost, LevelWriteHost, ServerHost, SoundPlay,
    SoundRef, SoundSource as ApiSoundSource,
};

use crate::runtime::events::{EventContext, EventFacade};
use crate::runtime::level_sound::{host_sound_streams, sound_stream_known_in};
use crate::runtime::{
    EntitySoundFacts, GameObject, GameObjectKind, LevelSoundEffect, SoundSeed, SoundSide,
    SoundSource as RuntimeSoundSource, SoundTarget,
};

/// Maximum `after` delay accepted by Sunrise's `ClientHost::play_local_sound`.
const MAX_CLIENT_DELAY_TICKS: u32 = 1200;

/// Exact 1:1 mapping from runtime `SoundSource` to Sunrise's `mod_api::SoundSource`.
pub(crate) fn to_api_source(source: RuntimeSoundSource) -> ApiSoundSource {
    match source {
        RuntimeSoundSource::Master => ApiSoundSource::Master,
        RuntimeSoundSource::Music => ApiSoundSource::Music,
        RuntimeSoundSource::Records => ApiSoundSource::Records,
        RuntimeSoundSource::Weather => ApiSoundSource::Weather,
        RuntimeSoundSource::Blocks => ApiSoundSource::Blocks,
        RuntimeSoundSource::Hostile => ApiSoundSource::Hostile,
        RuntimeSoundSource::Neutral => ApiSoundSource::Neutral,
        RuntimeSoundSource::Players => ApiSoundSource::Players,
        RuntimeSoundSource::Ambient => ApiSoundSource::Ambient,
        RuntimeSoundSource::Voice => ApiSoundSource::Voice,
        RuntimeSoundSource::Ui => ApiSoundSource::Ui,
    }
}

/// Exact 1:1 mapping from Sunrise's `mod_api::SoundSource` to runtime `SoundSource`.
pub(crate) fn from_api_source(source: ApiSoundSource) -> RuntimeSoundSource {
    match source {
        ApiSoundSource::Master => RuntimeSoundSource::Master,
        ApiSoundSource::Music => RuntimeSoundSource::Music,
        ApiSoundSource::Records => RuntimeSoundSource::Records,
        ApiSoundSource::Weather => RuntimeSoundSource::Weather,
        ApiSoundSource::Blocks => RuntimeSoundSource::Blocks,
        ApiSoundSource::Hostile => RuntimeSoundSource::Hostile,
        ApiSoundSource::Neutral => RuntimeSoundSource::Neutral,
        ApiSoundSource::Players => RuntimeSoundSource::Players,
        ApiSoundSource::Ambient => RuntimeSoundSource::Ambient,
        ApiSoundSource::Voice => RuntimeSoundSource::Voice,
        ApiSoundSource::Ui => RuntimeSoundSource::Ui,
        other => RuntimeSoundSource::from_ordinal(other.index())
            .expect("known Sunrise SoundSource ordinal"),
    }
}

fn facade_player_marker(facade: &EventFacade, uuid: u128) -> String {
    let hex = format!("{uuid:032x}");
    if let Some(player) = facade.players.iter().find(|p| p.uuid == uuid) {
        match player.entity_id {
            Some(id) => format!("game/player/{hex}/{}.{}/{id}", player.session, player.life),
            None => format!("game/player/{hex}/{}.{}", player.session, player.life),
        }
    } else {
        format!("game/player/{hex}")
    }
}

/// Best-effort read of sound facts for an entity/player from `ServerHost`.
/// Minimal test hosts that omit optional host areas panic via `unprovided`;
/// those reads yield `None` so missing facts stay unpopulated (and counted if
/// Java reads them) rather than fabricated.
fn try_read_entity_facts(
    host: &dyn ServerHost,
    entity: EntityRef,
    is_player: bool,
    player_uuid: Option<u128>,
) -> EntitySoundFacts {
    let position = catch_unwind(AssertUnwindSafe(|| entity.position(host)))
        .ok()
        .flatten();
    let silent = catch_unwind(AssertUnwindSafe(|| entity.is_silent(host)))
        .ok()
        .flatten();
    let sound_source = if is_player {
        Some(RuntimeSoundSource::Players)
    } else {
        catch_unwind(AssertUnwindSafe(|| entity.sound_source(host)))
            .ok()
            .flatten()
            .map(from_api_source)
    };
    let level = catch_unwind(AssertUnwindSafe(|| {
        entity
            .level(host)
            .map(|level| level.dimension_id().to_owned())
    }))
    .ok()
    .flatten();
    EntitySoundFacts {
        entity_id: i32::try_from(entity.id()).ok(),
        level,
        position,
        silent,
        sound_source,
        is_player,
        player_uuid,
    }
}

/// Populate `context.entity_sound_facts` from `host` for the context's player
/// and entity so `Entity.playSound`, `Player.playSound`, `getX/Y/Z`, `isSilent`,
/// and `getSoundSource` observe the real Sunrise entity state.
pub(crate) fn populate_context_sound_facts(
    host: &dyn ServerHost,
    facade: &EventFacade,
    context: &mut EventContext,
) {
    if let Some(uuid) = context.player
        && let Ok(Some(player)) = catch_unwind(AssertUnwindSafe(|| host.player(uuid)))
    {
        let marker = facade_player_marker(facade, uuid);
        let facts = try_read_entity_facts(host, player.entity(), true, Some(uuid));
        context.entity_sound_facts.insert(marker, facts);
    }
    if let Some(entity) = context.entity.as_ref()
        && let Ok(id) = u32::try_from(entity.id)
        && let Ok(Some(handle)) = catch_unwind(AssertUnwindSafe(|| host.entity(id)))
    {
        let marker = format!("game/entity/{}", entity.id);
        let facts = try_read_entity_facts(host, handle, false, None);
        context.entity_sound_facts.insert(marker, facts);
    }
}

/// Populate standalone player sound facts on a jar's [`crate::runtime::StdHost`]
/// for a command invocation's acting player.
pub(crate) fn seed_player_sound_facts(
    host: &dyn ServerHost,
    facade: &EventFacade,
    uuid: u128,
    runtime_host: &mut crate::runtime::StdHost,
) {
    if let Ok(Some(player)) = catch_unwind(AssertUnwindSafe(|| host.player(uuid))) {
        let marker = facade_player_marker(facade, uuid);
        let facts = try_read_entity_facts(host, player.entity(), true, Some(uuid));
        runtime_host.set_entity_sound_facts(marker, facts);
    }
}

/// Enrich a [`GameObject`] with live `position`, `silent`, `sound_source`, and
/// `dimension` from `ServerHost` when not already supplied by the hook.
pub(crate) fn enrich_game_object(host: &dyn ServerHost, mut object: GameObject) -> GameObject {
    let entity = match object.kind {
        GameObjectKind::Player => object
            .uuid
            .and_then(|uuid| {
                catch_unwind(AssertUnwindSafe(|| host.player(uuid)))
                    .ok()
                    .flatten()
            })
            .map(|player| player.entity()),
        GameObjectKind::Living | GameObjectKind::Entity | GameObjectKind::ItemEntity => object
            .entity_id
            .and_then(|id| u32::try_from(id).ok())
            .and_then(|id| {
                catch_unwind(AssertUnwindSafe(|| host.entity(id)))
                    .ok()
                    .flatten()
            }),
        _ => None,
    };
    let Some(entity) = entity else {
        return object;
    };
    let is_player = object.kind == GameObjectKind::Player;
    let facts = try_read_entity_facts(host, entity, is_player, object.uuid);
    if object.position.is_none()
        && let Some(pos) = facts.position
    {
        object = object.with_position(pos);
    }
    if object.silent.is_none()
        && let Some(silent) = facts.silent
    {
        object = object.with_silent(silent);
    }
    if object.sound_source.is_none()
        && let Some(source) = facts.sound_source
    {
        object = object.with_sound_source(source);
    }
    if object.dimension.is_none()
        && let Some(dimension) = facts.level
    {
        object = object.with_dimension(&dimension);
    }
    object
}

/// The unknown level sound streams every jar's runtime host has marked (see
/// the runtime's `level_sound` module docs).
///
/// A level's sound-seed stream is the level's, not a jar's: Java's
/// `Level.soundSeedGenerator` (and Sunrise's stream standing in for it) is
/// shared by every mod and by the game itself. So once any jar's host, or the
/// bridge, finds a stream's position unknown, it is unknown for every jar.
/// Each site that defers seeds to Sunrise gathers this ledger first and shares
/// it with every jar's host ([`share_unknown_sound_streams`]); within a
/// site, a jar's host is lent the ledger as it stands before that jar's Java
/// code runs.
///
/// Each stream gathered widens to every stream Sunrise's one stream for it
/// stands for ([`host_sound_streams`]): Sunrise draws every server sound's seed
/// from one server-wide stream where Java keeps one per level, so a draw it
/// missed on one level puts every level's later deferred seeds out of step.
pub(crate) fn gather_unknown_sound_streams<'a>(
    hosts: impl IntoIterator<Item = &'a crate::runtime::StdHost>,
) -> BTreeSet<String> {
    let mut ledger = BTreeSet::new();
    for host in hosts {
        for stream in host.unknown_sound_streams() {
            note_unknown(&mut ledger, stream);
        }
    }
    ledger
}

/// Add `stream`, widened by [`host_sound_streams`], to `ledger`.
fn note_unknown(ledger: &mut BTreeSet<String>, stream: &str) {
    ledger.extend(host_sound_streams(stream));
}

/// Mark every stream of `streams` unknown on each of `hosts`.
pub(crate) fn share_unknown_sound_streams<'a>(
    hosts: impl IntoIterator<Item = &'a mut crate::runtime::StdHost>,
    streams: &BTreeSet<String>,
) {
    for host in hosts {
        for stream in streams {
            // By key, not by `sound_stream_known`: a key the `game/level/*` wildcard already
            // covers still goes in, so every jar holds the same ledger.
            if !host.unknown_sound_streams().contains(stream) {
                host.mark_sound_stream_unknown(stream);
            }
        }
    }
}

/// [`gather_unknown_sound_streams`] then [`share_unknown_sound_streams`] over
/// the same hosts: afterwards every host holds the whole ledger, which this
/// returns.
pub(crate) fn sync_unknown_sound_streams<J>(
    jars: &mut [J],
    host: impl Fn(&mut J) -> &mut crate::runtime::StdHost,
) -> BTreeSet<String> {
    let mut ledger = BTreeSet::new();
    for jar in jars.iter_mut() {
        for stream in host(jar).unknown_sound_streams() {
            note_unknown(&mut ledger, stream);
        }
    }
    share_unknown_sound_streams(jars.iter_mut().map(host), &ledger);
    ledger
}

/// The Sunrise stream a recorded effect's seed is deferred to: `Some` only
/// for [`SoundSeed::HostDeferred`], whose seed Sunrise draws when (and only
/// when) the effect reaches it.
fn deferred_stream(effect: &LevelSoundEffect) -> Option<String> {
    if effect.seed == SoundSeed::HostDeferred {
        effect.seed_stream()
    } else {
        None
    }
}

/// Deliver `sounds` in call order through `deliver`, which plays one effect
/// or names why Sunrise cannot play it faithfully.
///
/// Java drew a deferred effect's seed when the call ran; Sunrise draws it only
/// if the effect reaches it. So an effect `deliver` refuses leaves Sunrise's
/// stream one draw behind Java's: the stream goes into `unknown`, and every
/// later deferred effect on it, in this batch or a later one, is counted
/// instead of played with a seed Java would not have drawn. The stream widens
/// to every level's ([`note_unknown`]): Sunrise's is one for the server. (A
/// `false` from Sunrise is a refusal too: `LevelRef::play_sound` draws only
/// once its checks pass, but the other host calls do not say.)
/// The pinned SDK has no call that advances a level's sound-seed stream
/// without playing a sound, so the position cannot be put right.
fn deliver_in_order(
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    on_unbridged: &mut dyn FnMut(&str),
    deliver: &mut dyn FnMut(&LevelSoundEffect) -> Result<(), &'static str>,
) {
    for effect in sounds {
        let stream = deferred_stream(effect);
        if let Some(stream) = &stream
            && !sound_stream_known_in(unknown, stream)
        {
            on_unbridged(
                "level sound seed stream position unknown: an earlier seed never reached Sunrise",
            );
            continue;
        }
        if let Err(reason) = deliver(effect) {
            on_unbridged(reason);
            if let Some(stream) = stream {
                note_unknown(unknown, &stream);
            }
        }
    }
}

/// Count recorded sounds that will never be delivered (their callback failed,
/// or there is no Sunrise host to play them on), in call order and exactly as
/// [`deliver_in_order`] counts a refusal: a deferred one leaves its stream
/// unknown.
pub(crate) fn count_undelivered_sounds(
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    mut on_unbridged: impl FnMut(&str),
    reason: &'static str,
) {
    deliver_in_order(sounds, unknown, &mut on_unbridged, &mut |_| Err(reason));
}

/// Deliver recorded server sounds to a Sunrise [`ServerHost`] via
/// `LevelRef::play_sound` and `EntityRef::play_bound_sound`. See
/// [`deliver_in_order`] for `unknown`.
pub(crate) fn apply_server_sounds(
    host: &mut dyn ServerHost,
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    mut on_unbridged: impl FnMut(&str),
) {
    deliver_in_order(sounds, unknown, &mut on_unbridged, &mut |effect| {
        deliver_server_sound(&mut *host, effect)
    });
}

fn deliver_server_sound(
    host: &mut dyn ServerHost,
    effect: &LevelSoundEffect,
) -> Result<(), &'static str> {
    if effect.side != SoundSide::Server {
        return Err("level sound wrong side for server host");
    }
    if effect.fixed_range.is_some() {
        return Err("level sound fixed range unsupported by Sunrise ServerHost");
    }
    if effect.seed != SoundSeed::HostDeferred {
        return Err("level sound explicit seed unsupported by Sunrise ServerHost");
    }
    let source = to_api_source(effect.source);
    match effect.target {
        SoundTarget::Position(pos) => {
            let Some(dimension) = effect.dimension.as_deref() else {
                return Err("level sound missing dimension");
            };
            let Some(level) = host.level(dimension) else {
                return Err("level sound missing Sunrise level");
            };
            let except = match effect.except_player {
                None => None,
                Some(uuid) => match host.player(uuid) {
                    Some(player) => Some(player),
                    None => return Err("level sound missing except player"),
                },
            };
            if !level.play_sound(
                host,
                &effect.sound_id,
                pos,
                source,
                effect.volume,
                effect.pitch,
                except,
            ) {
                return Err("level sound refused by Sunrise LevelRef::play_sound");
            }
        }
        SoundTarget::Entity { entity_id, .. } => {
            if effect.except_player.is_some() {
                return Err(
                    "level sound entity-bound except player unsupported by Sunrise EntityRef::play_bound_sound",
                );
            }
            let Ok(id) = u32::try_from(entity_id) else {
                return Err("level sound invalid entity id");
            };
            let Some(entity) = host.entity(id) else {
                return Err("level sound missing Sunrise entity");
            };
            if let Some(dim) = effect.dimension.as_deref()
                && let Ok(Some(entity_level)) =
                    catch_unwind(AssertUnwindSafe(|| entity.level(host)))
                && entity_level.dimension_id() != dim
            {
                return Err("level sound entity dimension mismatch");
            }
            if !entity.play_bound_sound(host, &effect.sound_id, source, effect.volume, effect.pitch)
            {
                return Err("level sound refused by Sunrise EntityRef::play_bound_sound");
            }
        }
    }
    Ok(())
}

/// Deliver recorded server sounds to a Sunrise [`LevelWriteHost`] (respecting
/// Sunrise's transactional journaling: queued on open transaction, committed
/// at root commit drawing `sound_seeds`, dropped on rollback). See
/// [`deliver_in_order`] for `unknown`.
#[allow(dead_code)]
pub(crate) fn apply_level_write_sounds(
    level: &mut dyn LevelWriteHost,
    expected_dimension: Option<&str>,
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    mut on_unbridged: impl FnMut(&str),
) {
    deliver_in_order(sounds, unknown, &mut on_unbridged, &mut |effect| {
        if effect.side != SoundSide::Server {
            return Err("level write sound wrong side");
        }
        if effect.fixed_range.is_some() {
            return Err("level write sound fixed range unsupported");
        }
        if effect.seed != SoundSeed::HostDeferred {
            return Err("level write sound explicit seed unsupported");
        }
        if let Some(expected) = expected_dimension
            && effect.dimension.as_deref() != Some(expected)
        {
            return Err("level write sound dimension mismatch");
        }
        if effect.except_player.is_some() {
            return Err("level write sound except player unsupported by LevelWriteHost");
        }
        let SoundTarget::Position(pos) = effect.target else {
            return Err("level write sound entity-bound unsupported by LevelWriteHost");
        };
        let source = to_api_source(effect.source).index();
        if !level.play_sound(&effect.sound_id, pos, source, effect.volume, effect.pitch) {
            return Err("level write sound refused by Sunrise LevelWriteHost");
        }
        Ok(())
    });
}

/// Deliver recorded server sounds during a damage hook to a Sunrise [`HitHost`]
/// via `HitHost::play_sound_at` and `HitHost::play_sound_from`. See
/// [`deliver_in_order`] for `unknown`.
pub(crate) fn apply_hit_sounds(
    hit: &mut dyn HitHost,
    lent_entity: Option<(i32, EntityRef)>,
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    mut on_unbridged: impl FnMut(&str),
) {
    deliver_in_order(sounds, unknown, &mut on_unbridged, &mut |effect| {
        if effect.side != SoundSide::Server {
            return Err("hit sound wrong side");
        }
        if effect.fixed_range.is_some() {
            return Err("hit sound fixed range unsupported by Sunrise HitHost");
        }
        if effect.seed != SoundSeed::HostDeferred {
            return Err("hit sound explicit seed unsupported by Sunrise HitHost");
        }
        if effect.except_player.is_some() {
            return Err("hit sound except player unsupported by Sunrise HitHost");
        }
        let source = to_api_source(effect.source);
        match effect.target {
            SoundTarget::Position(pos) => {
                if !hit.play_sound_at(&effect.sound_id, pos, source, effect.volume, effect.pitch) {
                    return Err("hit sound refused by Sunrise HitHost::play_sound_at");
                }
            }
            SoundTarget::Entity { entity_id, .. } => {
                let Some((_, entity)) = lent_entity.filter(|(id, _)| *id == entity_id) else {
                    return Err("hit sound entity not lent by Sunrise HitHost");
                };
                if !hit.play_sound_from(
                    entity,
                    &effect.sound_id,
                    source,
                    effect.volume,
                    effect.pitch,
                ) {
                    return Err("hit sound refused by Sunrise HitHost::play_sound_from");
                }
            }
        }
        Ok(())
    });
}

/// Whatever plays a recorded client sound with vanilla's `SoundEngine.playDelayed`: the world
/// client host (`EffectsClientHost::play_local_sound`), or the setup context `client_setup`
/// lends since d404b5f02f (`ClientSetupContext::play_local_sound`), which has no world.
pub(crate) trait LocalSounds {
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> Option<SoundRef>;
}

impl LocalSounds for dyn ClientHost + '_ {
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> Option<SoundRef> {
        mod_api::prelude::EffectsClientHost::play_local_sound(self, sound, after)
    }
}

impl LocalSounds for dyn ClientSetupContext + '_ {
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> Option<SoundRef> {
        ClientSetupContext::play_local_sound(self, sound, after)
    }
}

/// Deliver recorded client sounds to a Sunrise client host ([`LocalSounds`]) via
/// `play_local_sound`. See [`deliver_in_order`] for `unknown`.
///
/// Only a [`SoundSeed::HostDeferred`] effect can play, and the client owner
/// never turns client deferral on: `play_local_sound` draws its seed from
/// Sunrise's audio random at the next audio update, where Java draws
/// `ClientLevel.random` (or the client `soundSeedGenerator`, for
/// `Level.playSound`) at the call, so the seed and the stream's order would
/// both differ. A recorded client sound (an explicit `playSeededSound` seed,
/// say) therefore drains here and is counted, never dropped.
pub(crate) fn apply_client_sounds<H: LocalSounds + ?Sized>(
    host: &H,
    sounds: &[LevelSoundEffect],
    unknown: &mut BTreeSet<String>,
    mut on_unbridged: impl FnMut(&str),
) {
    deliver_in_order(sounds, unknown, &mut on_unbridged, &mut |effect| {
        if effect.side != SoundSide::Client {
            return Err("client sound wrong side");
        }
        if effect.fixed_range.is_some() {
            return Err("client sound fixed range unsupported by Sunrise play_local_sound");
        }
        if effect.seed != SoundSeed::HostDeferred {
            return Err("client sound explicit seed unsupported by Sunrise play_local_sound");
        }
        if effect.delay_ticks > MAX_CLIENT_DELAY_TICKS {
            return Err("client sound delay exceeds Sunrise 1200-tick limit");
        }
        let SoundTarget::Position(pos) = effect.target else {
            return Err("client sound entity-bound unsupported by Sunrise play_local_sound");
        };
        let play = SoundPlay::at(&effect.sound_id, to_api_source(effect.source), pos)
            .with_volume(effect.volume)
            .with_pitch(effect.pitch);
        if host.play_local_sound(&play, effect.delay_ticks).is_none() {
            return Err("client sound refused by Sunrise ClientHost::play_local_sound");
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use mod_api::__private::game::BackendMutRefOf;
    use mod_api::prelude::{
        EffectsClientHost, EntitiesHost, EntityActionsBackendMut, LevelEffectsBackendMut,
        PlayersHost,
    };
    use mod_api::testing::FakeHost;
    use mod_api::{
        BackendMut, BackendMutRef, ClientHost, DimensionKey, EntityRef, HitHost, Host, LevelHost,
        LevelRandomHost, LevelRef, LevelTicksHost, LevelWriteHost, PlayerRef, ServerHost,
        SoundPlay, SoundRef, WorldEpoch,
    };

    fn server_pos_effect(
        sound_id: &str,
        pos: [f64; 3],
        source: RuntimeSoundSource,
        except_player: Option<u128>,
    ) -> LevelSoundEffect {
        LevelSoundEffect {
            side: SoundSide::Server,
            dimension: Some("minecraft:overworld".into()),
            sound_id: sound_id.into(),
            fixed_range: None,
            holder: None,
            target: SoundTarget::Position(pos),
            source,
            volume: 0.9,
            pitch: 1.1,
            except_player,
            seed: SoundSeed::HostDeferred,
            distance_delay: false,
            delay_ticks: 0,
        }
    }

    #[test]
    fn sound_source_round_trip_matches_mc_ordinals() {
        for source in RuntimeSoundSource::ALL {
            let api = to_api_source(source);
            assert_eq!(api.index(), source.ordinal());
            assert_eq!(from_api_source(api), source);
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum RecordedServerSound {
        Pos {
            dimension: String,
            sound: String,
            at: [f64; 3],
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
            except: Option<u128>,
        },
        Bound {
            entity_id: u32,
            sound: String,
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
        },
    }

    struct SoundRecordingServerHost {
        overworld: LevelRef,
        player: PlayerRef,
        mob: EntityRef,
        recorded: Vec<RecordedServerSound>,
    }

    impl SoundRecordingServerHost {
        fn new() -> Self {
            let epoch = WorldEpoch::new(1);
            let overworld = LevelRef::new(epoch, DimensionKey::OVERWORLD);
            let player = mod_api::__private::game::player_ref(EntityRef::new(epoch, 10), 0xa1);
            let mob = EntityRef::new(epoch, 20);
            Self {
                overworld,
                player,
                mob,
                recorded: Vec::new(),
            }
        }
    }

    impl Host for SoundRecordingServerHost {}
    mod_api::impl_host_areas!(SoundRecordingServerHost, except(entities, players));
    impl EntitiesHost for SoundRecordingServerHost {
        fn entity(&self, id: u32) -> Option<EntityRef> {
            match id {
                10 => Some(self.player.entity()),
                20 => Some(self.mob),
                _ => None,
            }
        }
        fn level(&self, dimension: &str) -> Option<LevelRef> {
            (dimension == "minecraft:overworld").then_some(self.overworld)
        }
    }
    impl PlayersHost for SoundRecordingServerHost {
        fn player(&self, uuid: u128) -> Option<PlayerRef> {
            (uuid == 0xa1).then_some(self.player)
        }
    }
    impl LevelEffectsBackendMut for SoundRecordingServerHost {
        fn level_play_sound(
            &mut self,
            level: LevelRef,
            sound: &str,
            at: [f64; 3],
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
            except: Option<PlayerRef>,
        ) -> bool {
            self.recorded.push(RecordedServerSound::Pos {
                dimension: level.dimension_id().to_owned(),
                sound: sound.to_owned(),
                at,
                source,
                volume,
                pitch,
                except: except.map(PlayerRef::uuid),
            });
            true
        }
    }
    impl EntityActionsBackendMut for SoundRecordingServerHost {
        fn entity_play_bound_sound(
            &mut self,
            entity: EntityRef,
            sound: &str,
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
        ) -> bool {
            self.recorded.push(RecordedServerSound::Bound {
                entity_id: entity.id(),
                sound: sound.to_owned(),
                source,
                volume,
                pitch,
            });
            true
        }
    }
    mod_api::impl_backend_mut_areas!(
        SoundRecordingServerHost,
        except(entity_actions, level_effects)
    );
    impl BackendMut for SoundRecordingServerHost {}
    impl ServerHost for SoundRecordingServerHost {
        fn backend_mut(&mut self) -> Option<BackendMutRef<'_>> {
            Some(BackendMutRef::of(self))
        }
    }
    mod_api::impl_server_host_areas!(SoundRecordingServerHost);

    #[test]
    fn server_host_bridges_positional_except_entity_bound_and_counts_unbridgeable() {
        // 1. Verify directly against Sunrise's own FakeHost.
        let mut fake = FakeHost::new();
        fake.join(0xa1, 10);
        let mut unbridged = Vec::new();
        let pos_effect = server_pos_effect(
            "demo:chime",
            [1.5, 64.5, -2.5],
            RuntimeSoundSource::Blocks,
            Some(0xa1),
        );
        apply_server_sounds(
            &mut fake,
            std::slice::from_ref(&pos_effect),
            &mut BTreeSet::new(),
            |r| unbridged.push(r.to_owned()),
        );
        assert!(unbridged.is_empty());
        assert_eq!(fake.sounds.len(), 1);
        assert_eq!(fake.sounds[0].sound, "demo:chime");
        assert_eq!(fake.sounds[0].at, [1.5, 64.5, -2.5]);
        assert_eq!(fake.sounds[0].source, ApiSoundSource::Blocks);
        assert_eq!(fake.sounds[0].volume, 0.9);
        assert_eq!(fake.sounds[0].pitch, 1.1);

        // 2. Verify except player, entity-bound, and every unbridgeable refusal.
        let mut host = SoundRecordingServerHost::new();
        let bound_ok = LevelSoundEffect {
            target: SoundTarget::Entity {
                entity_id: 20,
                position: None,
            },
            except_player: None,
            ..pos_effect.clone()
        };
        let bound_with_except = LevelSoundEffect {
            except_player: Some(0xa1),
            ..bound_ok.clone()
        };
        let fixed_range = LevelSoundEffect {
            fixed_range: Some(32.0),
            ..pos_effect.clone()
        };
        let explicit_seed = LevelSoundEffect {
            seed: SoundSeed::Explicit(1234),
            ..pos_effect.clone()
        };
        let missing_except = LevelSoundEffect {
            except_player: Some(0xdead),
            ..pos_effect.clone()
        };
        let missing_level = LevelSoundEffect {
            dimension: Some("minecraft:the_nether".into()),
            ..pos_effect.clone()
        };

        let mut refused = Vec::new();
        apply_server_sounds(
            &mut host,
            &[
                pos_effect,
                bound_ok,
                bound_with_except,
                fixed_range,
                explicit_seed,
                missing_except,
                missing_level,
            ],
            &mut BTreeSet::new(),
            |reason| refused.push(reason.to_owned()),
        );
        assert_eq!(
            host.recorded,
            vec![
                RecordedServerSound::Pos {
                    dimension: "minecraft:overworld".into(),
                    sound: "demo:chime".into(),
                    at: [1.5, 64.5, -2.5],
                    source: ApiSoundSource::Blocks,
                    volume: 0.9,
                    pitch: 1.1,
                    except: Some(0xa1),
                },
                RecordedServerSound::Bound {
                    entity_id: 20,
                    sound: "demo:chime".into(),
                    source: ApiSoundSource::Blocks,
                    volume: 0.9,
                    pitch: 1.1,
                },
            ]
        );
        assert_eq!(
            refused.len(),
            5,
            "all 5 unbridgeable effects must count: {refused:?}"
        );
    }

    #[derive(Clone, Debug, PartialEq)]
    struct WriteSoundRecord {
        sound: String,
        at: [f64; 3],
        source: usize,
        volume: f32,
        pitch: f32,
    }

    #[derive(Default)]
    struct TransactionalLevelWriteHost {
        in_tx: bool,
        staged: Vec<WriteSoundRecord>,
        committed: Vec<WriteSoundRecord>,
    }

    impl TransactionalLevelWriteHost {
        fn begin_tx(&mut self) {
            self.in_tx = true;
            self.staged.clear();
        }
        fn commit_tx(&mut self) {
            self.in_tx = false;
            self.committed.append(&mut self.staged);
        }
        fn rollback_tx(&mut self) {
            self.in_tx = false;
            self.staged.clear();
        }
    }

    impl LevelHost for TransactionalLevelWriteHost {
        fn block_entity_data(&self, _: mod_api::BlockEntityRef) -> Option<mod_api::Compound> {
            None
        }
    }
    impl LevelTicksHost for TransactionalLevelWriteHost {}
    impl LevelRandomHost for TransactionalLevelWriteHost {}
    impl LevelWriteHost for TransactionalLevelWriteHost {
        fn play_sound(
            &mut self,
            sound: &str,
            at: [f64; 3],
            source: usize,
            volume: f32,
            pitch: f32,
        ) -> bool {
            let record = WriteSoundRecord {
                sound: sound.to_owned(),
                at,
                source,
                volume,
                pitch,
            };
            if self.in_tx {
                self.staged.push(record);
            } else {
                self.committed.push(record);
            }
            true
        }
    }

    #[test]
    fn level_write_host_respects_transaction_commit_and_rollback() {
        let mut level = TransactionalLevelWriteHost::default();
        let ok = server_pos_effect(
            "demo:place",
            [2.5, 65.5, 3.5],
            RuntimeSoundSource::Blocks,
            None,
        );
        let with_except = server_pos_effect(
            "demo:except",
            [2.5, 65.5, 3.5],
            RuntimeSoundSource::Blocks,
            Some(0xa1),
        );
        let fixed = LevelSoundEffect {
            fixed_range: Some(48.0),
            ..ok.clone()
        };

        // 1. Open transaction -> staged only, rolled back -> nothing committed.
        level.begin_tx();
        let mut refused = Vec::new();
        apply_level_write_sounds(
            &mut level,
            Some("minecraft:overworld"),
            &[ok.clone(), with_except, fixed],
            &mut BTreeSet::new(),
            |reason| refused.push(reason.to_owned()),
        );
        assert_eq!(refused.len(), 2);
        assert!(
            level.committed.is_empty(),
            "sound must not publish before root commit"
        );
        assert_eq!(level.staged.len(), 1);
        level.rollback_tx();
        assert!(
            level.committed.is_empty(),
            "aborted transaction must drop queued sound"
        );

        // 2. Open transaction -> committed -> published with exact source index.
        level.begin_tx();
        refused.clear();
        apply_level_write_sounds(
            &mut level,
            Some("minecraft:overworld"),
            &[ok],
            &mut BTreeSet::new(),
            |reason| refused.push(reason.to_owned()),
        );
        assert!(refused.is_empty());
        assert!(level.committed.is_empty());
        level.commit_tx();
        assert_eq!(
            level.committed,
            vec![WriteSoundRecord {
                sound: "demo:place".into(),
                at: [2.5, 65.5, 3.5],
                source: ApiSoundSource::Blocks.index(),
                volume: 0.9,
                pitch: 1.1,
            }]
        );
    }

    #[derive(Default)]
    struct RecordingHitHost {
        at_calls: Vec<(String, [f64; 3], ApiSoundSource, f32, f32)>,
        from_calls: Vec<(u32, String, ApiSoundSource, f32, f32)>,
    }

    impl LevelHost for RecordingHitHost {
        fn block_entity_data(&self, _: mod_api::BlockEntityRef) -> Option<mod_api::Compound> {
            None
        }
    }
    impl LevelRandomHost for RecordingHitHost {}
    impl HitHost for RecordingHitHost {
        fn play_sound_at(
            &mut self,
            sound: &str,
            at: [f64; 3],
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
        ) -> bool {
            self.at_calls
                .push((sound.to_owned(), at, source, volume, pitch));
            true
        }
        fn play_sound_from(
            &mut self,
            entity: EntityRef,
            sound: &str,
            source: ApiSoundSource,
            volume: f32,
            pitch: f32,
        ) -> bool {
            self.from_calls
                .push((entity.id(), sound.to_owned(), source, volume, pitch));
            true
        }
    }

    #[test]
    fn hit_host_bridges_positional_and_lent_entity_and_counts_unbridgeable() {
        let mut hit = RecordingHitHost::default();
        let lent = EntityRef::new(WorldEpoch::new(1), 55);
        let pos = server_pos_effect(
            "demo:hit",
            [4.0, 65.0, -1.0],
            RuntimeSoundSource::Hostile,
            None,
        );
        let lent_bound = LevelSoundEffect {
            target: SoundTarget::Entity {
                entity_id: 55,
                position: None,
            },
            ..pos.clone()
        };
        let unlent_bound = LevelSoundEffect {
            target: SoundTarget::Entity {
                entity_id: 99,
                position: None,
            },
            ..pos.clone()
        };
        let with_except = LevelSoundEffect {
            except_player: Some(1),
            ..pos.clone()
        };
        let mut refused = Vec::new();
        apply_hit_sounds(
            &mut hit,
            Some((55, lent)),
            &[pos, lent_bound, unlent_bound, with_except],
            &mut BTreeSet::new(),
            |reason| refused.push(reason.to_owned()),
        );
        assert_eq!(
            hit.at_calls,
            vec![(
                "demo:hit".into(),
                [4.0, 65.0, -1.0],
                ApiSoundSource::Hostile,
                0.9,
                1.1
            )]
        );
        assert_eq!(
            hit.from_calls,
            vec![(55, "demo:hit".into(), ApiSoundSource::Hostile, 0.9, 1.1)]
        );
        assert_eq!(refused.len(), 2);
    }

    #[derive(Default)]
    struct RecordingClientHost {
        played: RefCell<Vec<(SoundPlay, u32)>>,
    }

    impl Host for RecordingClientHost {}
    mod_api::impl_host_areas!(RecordingClientHost);
    impl EffectsClientHost for RecordingClientHost {
        fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> Option<SoundRef> {
            self.played.borrow_mut().push((sound.clone(), after));
            Some(mod_api::__private::game::sound_ref(
                self.played.borrow().len() as u64,
            ))
        }
    }
    impl ClientHost for RecordingClientHost {}
    mod_api::impl_client_host_areas!(RecordingClientHost, except(effects));

    #[test]
    fn client_host_bridges_play_local_sound_and_counts_unbridgeable() {
        let host = RecordingClientHost::default();
        let ok = LevelSoundEffect {
            side: SoundSide::Client,
            dimension: Some("minecraft:overworld".into()),
            sound_id: "demo:thunder".into(),
            fixed_range: None,
            holder: None,
            target: SoundTarget::Position([40.0, 64.0, 0.0]),
            source: RuntimeSoundSource::Weather,
            volume: 2.0,
            pitch: 0.8,
            except_player: None,
            seed: SoundSeed::HostDeferred,
            distance_delay: true,
            delay_ticks: 20,
        };
        let too_late = LevelSoundEffect {
            delay_ticks: 1201,
            ..ok.clone()
        };
        let entity_bound = LevelSoundEffect {
            target: SoundTarget::Entity {
                entity_id: 7,
                position: Some([0.0, 0.0, 0.0]),
            },
            ..ok.clone()
        };
        let fixed_range = LevelSoundEffect {
            fixed_range: Some(64.0),
            ..ok.clone()
        };
        let explicit_seed = LevelSoundEffect {
            seed: SoundSeed::Explicit(42),
            ..ok.clone()
        };

        let mut refused = Vec::new();
        apply_client_sounds(
            &host as &dyn ClientHost,
            &[ok, too_late, entity_bound, fixed_range, explicit_seed],
            &mut BTreeSet::new(),
            |reason| refused.push(reason.to_owned()),
        );
        let played = host.played.borrow();
        assert_eq!(played.len(), 1);
        assert_eq!(
            played[0],
            (
                SoundPlay::at("demo:thunder", ApiSoundSource::Weather, [40.0, 64.0, 0.0])
                    .with_volume(2.0)
                    .with_pitch(0.8),
                20,
            )
        );
        assert_eq!(refused.len(), 4);
    }

    #[test]
    fn refused_deferred_sound_leaves_its_stream_unknown_for_later_sounds() {
        let mut host = SoundRecordingServerHost::new();
        let ok = server_pos_effect("demo:a", [1.0, 2.0, 3.0], RuntimeSoundSource::Blocks, None);
        // Sunrise's API has no fixed range, so this one is refused after Java drew its seed.
        let fixed = LevelSoundEffect {
            fixed_range: Some(32.0),
            ..ok.clone()
        };
        // An explicit `playSeededSound` seed drew nothing: refusing it moves no stream.
        let explicit = LevelSoundEffect {
            seed: SoundSeed::Explicit(7),
            ..ok.clone()
        };
        let nether = LevelSoundEffect {
            dimension: Some("minecraft:the_nether".into()),
            ..ok.clone()
        };
        let mut unknown = BTreeSet::new();
        let mut refused = Vec::new();
        apply_server_sounds(
            &mut host,
            &[explicit, ok.clone(), fixed, ok.clone(), nether],
            &mut unknown,
            |reason| refused.push(reason.to_owned()),
        );
        // Only the first deferred sound reached Sunrise. After the fixed-range refusal Sunrise's
        // one server-wide stream is a draw behind, so later deferred sounds on every level are
        // counted, not played.
        assert_eq!(host.recorded.len(), 1, "{:?}", host.recorded);
        assert_eq!(
            refused,
            vec![
                "level sound explicit seed unsupported by Sunrise ServerHost",
                "level sound fixed range unsupported by Sunrise ServerHost",
                "level sound seed stream position unknown: an earlier seed never reached Sunrise",
                "level sound seed stream position unknown: an earlier seed never reached Sunrise",
            ]
        );
        assert_eq!(
            unknown,
            BTreeSet::from([
                "game/level/*".to_owned(),
                "game/level/minecraft:overworld".to_owned(),
            ])
        );
        // A later batch against the same ledger counts the overworld sound as well.
        refused.clear();
        apply_server_sounds(&mut host, &[ok], &mut unknown, |reason| {
            refused.push(reason.to_owned());
        });
        assert_eq!(host.recorded.len(), 1);
        assert_eq!(refused.len(), 1);

        // Client effects name their stream by how they were drawn: a holder means
        // `Level.playSound` (`soundSeedGenerator`), none means `playLocalSound` (`Level.random`).
        let client = RecordingClientHost::default();
        let local = LevelSoundEffect {
            side: SoundSide::Client,
            dimension: Some("minecraft:overworld".into()),
            sound_id: "demo:c".into(),
            fixed_range: None,
            holder: None,
            target: SoundTarget::Position([0.0, 64.0, 0.0]),
            source: RuntimeSoundSource::Blocks,
            volume: 1.0,
            pitch: 1.0,
            except_player: None,
            seed: SoundSeed::HostDeferred,
            distance_delay: false,
            delay_ticks: 0,
        };
        let local_entity_bound = LevelSoundEffect {
            target: SoundTarget::Entity {
                entity_id: 5,
                position: None,
            },
            ..local.clone()
        };
        let level_sound = LevelSoundEffect {
            holder: Some(crate::runtime::SoundHolderKind::Direct),
            ..local.clone()
        };
        let mut client_unknown = BTreeSet::new();
        refused.clear();
        apply_client_sounds(
            &client as &dyn ClientHost,
            &[level_sound.clone(), local_entity_bound, local, level_sound],
            &mut client_unknown,
            |reason| refused.push(reason.to_owned()),
        );
        // Sunrise's `play_local_sound` draws every client mod sound's seed from one audio
        // random, so a refused one leaves both client streams unknown.
        assert_eq!(
            client_unknown,
            BTreeSet::from(["client/level".to_owned(), "client/level#random".to_owned()])
        );
        assert_eq!(refused.len(), 3, "{refused:?}");
        let played = client.played.borrow();
        assert_eq!(played.len(), 1);
        assert_eq!(played[0].0.identifier, "demo:c");
    }

    #[test]
    fn sync_shares_every_jars_unknown_streams() {
        let mut hosts = [
            crate::runtime::StdHost::new(),
            crate::runtime::StdHost::new(),
        ];
        // Marked while no seed was deferred: Java's per-level streams, one level each.
        hosts[0].mark_sound_stream_unknown("game/level/minecraft:overworld");
        hosts[1].mark_sound_stream_unknown("client/level#random");
        assert!(hosts[0].sound_stream_known("game/level/minecraft:the_end"));
        let ledger = sync_unknown_sound_streams(&mut hosts, |host| host);
        // Shared, and widened to what Sunrise's one stream per side stands for.
        assert_eq!(
            ledger,
            BTreeSet::from([
                "client/level".to_owned(),
                "client/level#random".to_owned(),
                "game/level/*".to_owned(),
                "game/level/minecraft:overworld".to_owned(),
            ])
        );
        for host in &hosts {
            assert_eq!(host.unknown_sound_streams(), &ledger);
            assert!(!host.sound_stream_known("game/level/minecraft:the_end"));
            assert!(!host.sound_stream_known("client/level"));
        }
    }

    /// Java stubs for a jar whose `ServerTickEvents.START_LEVEL_TICK` listener plays level
    /// sounds. Only `demo/` classes ship in the jar.
    const TICK_SOUND_STUBS: &[(&str, &str)] = &[
        (
            "net/fabricmc/fabric/api/event/Event.java",
            "package net.fabricmc.fabric.api.event; public abstract class Event<T> { public abstract void register(T listener); }",
        ),
        (
            "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.java",
            "package net.fabricmc.fabric.api.event.lifecycle.v1; \
             import net.fabricmc.fabric.api.event.Event; \
             import net.minecraft.server.level.ServerLevel; \
             public final class ServerTickEvents { \
               public static Event<StartLevelTick> START_LEVEL_TICK; \
               public interface StartLevelTick { void onStartTick(ServerLevel level); } \
             }",
        ),
        (
            "net/minecraft/resources/Identifier.java",
            "package net.minecraft.resources; public final class Identifier { \
               public static Identifier fromNamespaceAndPath(String ns, String p) { return null; } }",
        ),
        (
            "net/minecraft/core/Vec3i.java",
            "package net.minecraft.core; public class Vec3i { \
               private int x, y, z; \
               public Vec3i(int x, int y, int z) { this.x = x; this.y = y; this.z = z; } \
               public int getX() { return x; } \
               public int getY() { return y; } \
               public int getZ() { return z; } }",
        ),
        (
            "net/minecraft/core/BlockPos.java",
            "package net.minecraft.core; public class BlockPos extends Vec3i { \
               public BlockPos(int x, int y, int z) { super(x, y, z); } }",
        ),
        (
            "net/minecraft/sounds/SoundEvent.java",
            "package net.minecraft.sounds; import net.minecraft.resources.Identifier; \
             public final class SoundEvent { \
               public static SoundEvent createVariableRangeEvent(Identifier id) { return null; } \
               public static SoundEvent createFixedRangeEvent(Identifier id, float r) { return null; } }",
        ),
        (
            "net/minecraft/sounds/SoundSource.java",
            "package net.minecraft.sounds; public enum SoundSource { \
               MASTER, MUSIC, RECORDS, WEATHER, BLOCKS, HOSTILE, NEUTRAL, PLAYERS, AMBIENT, VOICE, UI }",
        ),
        (
            "net/minecraft/world/entity/Entity.java",
            "package net.minecraft.world.entity; public class Entity {}",
        ),
        (
            "net/minecraft/world/level/Level.java",
            "package net.minecraft.world.level; \
             import net.minecraft.core.BlockPos; \
             import net.minecraft.sounds.SoundEvent; \
             import net.minecraft.sounds.SoundSource; \
             import net.minecraft.world.entity.Entity; \
             public abstract class Level { \
               public void playSound(Entity e, BlockPos pos, SoundEvent s, SoundSource src, float v, float p) {} \
             }",
        ),
        (
            "net/minecraft/server/level/ServerLevel.java",
            "package net.minecraft.server.level; import net.minecraft.world.level.Level; \
             public class ServerLevel extends Level {}",
        ),
    ];

    /// Compile a Fabric jar `mod_id` whose `START_LEVEL_TICK` listener runs `listener` (Java
    /// statements over `level`, with `ok` the variable-range `demo:bell` and `fixed` a 64-block
    /// fixed-range `demo:far`) and load it as the server owner does.
    fn tick_sound_jar(mod_id: &str, listener: &str) -> crate::LoadedJar {
        use std::io::Write;
        use std::path::PathBuf;
        use std::process::Command;
        use std::sync::Arc;

        use zip::write::SimpleFileOptions;

        let root = std::env::temp_dir().join(format!(
            "horizon-java-mod-sound-{mod_id}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("src");
        let classes = root.join("classes");
        std::fs::create_dir_all(&classes).unwrap();

        let main = format!(
            "package demo; \
             import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents; \
             import net.minecraft.core.BlockPos; \
             import net.minecraft.resources.Identifier; \
             import net.minecraft.sounds.SoundEvent; \
             import net.minecraft.sounds.SoundSource; \
             public final class Main {{ \
               public void onInitialize() {{ \
                 SoundEvent ok = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath(\"demo\", \"bell\")); \
                 SoundEvent fixed = SoundEvent.createFixedRangeEvent(Identifier.fromNamespaceAndPath(\"demo\", \"far\"), 64.0f); \
                 ServerTickEvents.START_LEVEL_TICK.register(level -> {{ {listener} }}); \
               }} \
             }}"
        );
        let mut java_paths = Vec::new();
        for (rel, body) in TICK_SOUND_STUBS
            .iter()
            .copied()
            .chain([("demo/Main.java", main.as_str())])
        {
            let path = src.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            java_paths.push(path);
        }
        let javac = std::env::var_os("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        assert!(
            Command::new(javac)
                .args(["--release", "21", "-nowarn", "-d"])
                .arg(&classes)
                .args(&java_paths)
                .status()
                .expect("javac runs")
                .success()
        );

        let mut jar_bytes = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut jar_bytes));
            let opts = SimpleFileOptions::default();
            zip.start_file("fabric.mod.json", opts).unwrap();
            zip.write_all(
                format!(
                    r#"{{"id":"{mod_id}","version":"1","entrypoints":{{"main":["demo.Main"]}}}}"#
                )
                .as_bytes(),
            )
            .unwrap();
            let class_path = classes.join("demo/Main.class");
            zip.start_file("demo/Main.class", opts).unwrap();
            zip.write_all(&std::fs::read(&class_path).unwrap()).unwrap();
            zip.finish().unwrap();
        }
        let _ = std::fs::remove_dir_all(&root);

        let jar = Arc::new(crate::runtime::NativeJar::from_bytes(&jar_bytes).unwrap());
        let imc = crate::runtime::neoforge_imc::ImcBus::default();
        let Ok(loaded) = crate::load_jar(format!("{mod_id}.jar"), jar, &[], &imc, None) else {
            panic!("{mod_id} did not load");
        };
        loaded
    }

    fn fire_level_tick(java: &crate::HorizonJava, fake: &mut FakeHost) {
        let overworld = fake.level.level();
        let context = crate::runtime::events::EventContext {
            level: Some(overworld.dimension_id().to_owned()),
            has_time: Some(true),
            ..Default::default()
        };
        java.fire_event_with_server_sounds(
            fake,
            crate::runtime::events::GameEvent::LevelTickStart,
            true,
            context,
            |_| true,
        );
    }

    #[test]
    fn fake_host_receives_sound_from_bridged_java_level_tick() {
        let loaded = tick_sound_jar(
            "sounddemo",
            "level.playSound(null, new BlockPos(4, 65, -2), ok, SoundSource.BLOCKS, 0.8f, 1.2f); \
             level.playSound(null, new BlockPos(0, 64, 0), fixed, SoundSource.BLOCKS, 1.0f, 1.0f);",
        );
        let java = crate::HorizonJava::default();
        java.lock().jars.push(loaded);

        let mut fake = FakeHost::new();
        fire_level_tick(&java, &mut fake);

        assert_eq!(
            fake.sounds.len(),
            1,
            "variable-range sound bridged; fixed-range refused"
        );
        assert_eq!(fake.sounds[0].sound, "demo:bell");
        assert_eq!(fake.sounds[0].at, [4.5, 65.5, -1.5]);
        assert_eq!(fake.sounds[0].source, ApiSoundSource::Blocks);
        assert_eq!(fake.sounds[0].volume, 0.8);
        assert_eq!(fake.sounds[0].pitch, 1.2);
        // Fixed-range sound was counted as an unknown game hook on the emitting jar!
        let state = java.lock();
        assert_eq!(state.jars[0].row.callback_unknown_host_calls, 1);
    }

    /// A refused sound's seed was drawn by Java but never by Sunrise, so the level's stream is
    /// out of step for every jar, in this firing and every later one, until the server restarts.
    #[test]
    fn refused_sound_puts_every_jars_later_sounds_on_that_level_out_of_play() {
        // Jar A plays a variable-range sound, then a fixed-range one Sunrise cannot take.
        let first = tick_sound_jar(
            "soundfirst",
            "level.playSound(null, new BlockPos(4, 65, -2), ok, SoundSource.BLOCKS, 0.8f, 1.2f); \
             level.playSound(null, new BlockPos(0, 64, 0), fixed, SoundSource.BLOCKS, 1.0f, 1.0f);",
        );
        // Jar B's listener runs after A's in the same firing: an ordinary sound.
        let second = tick_sound_jar(
            "soundsecond",
            "level.playSound(null, new BlockPos(7, 70, 7), ok, SoundSource.AMBIENT, 1.0f, 1.0f);",
        );
        let java = crate::HorizonJava::default();
        java.lock().jars.extend([first, second]);
        let mut fake = FakeHost::new();
        let overworld = "game/level/minecraft:overworld";

        // Firing 1: A's first sound plays. Its fixed-range sound is refused, so B's sound, which
        // Java would have played with the next seed, is counted instead of played with one
        // Sunrise draws a step early.
        fire_level_tick(&java, &mut fake);
        assert_eq!(fake.sounds.len(), 1);
        assert_eq!(fake.sounds[0].sound, "demo:bell");
        assert_eq!(fake.sounds[0].at, [4.5, 65.5, -1.5]);
        {
            let state = java.lock();
            assert_eq!(state.jars[0].row.callback_unknown_host_calls, 1);
            assert_eq!(state.jars[1].row.callback_unknown_host_calls, 1);
            for jar in &state.jars {
                assert!(!jar.host.sound_stream_known(overworld));
            }
        }

        // Firing 2: every jar's runtime now counts its overworld sound at the call.
        fire_level_tick(&java, &mut fake);
        assert_eq!(fake.sounds.len(), 1, "no sound plays on an unknown stream");
        {
            let state = java.lock();
            assert_eq!(state.jars[0].row.callback_unknown_host_calls, 2);
            assert_eq!(state.jars[1].row.callback_unknown_host_calls, 2);
        }

        // A new server: fresh levels, fresh streams.
        java.lock().reset_server();
        fire_level_tick(&java, &mut fake);
        assert_eq!(fake.sounds.len(), 2);
        assert_eq!(fake.sounds[1].sound, "demo:bell");
        let state = java.lock();
        assert_eq!(state.jars[0].row.callback_unknown_host_calls, 1);
        assert_eq!(state.jars[1].row.callback_unknown_host_calls, 1);
    }
}

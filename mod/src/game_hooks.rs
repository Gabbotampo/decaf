//! Game-class Mixin callbacks at the moments the pinned mod API names (#156 P16e2).
//!
//! A jar's `@Inject` into a Minecraft class cannot be woven: the class is the game's, not the
//! jar's. The runtime keeps such a callback when a game hook site stands for its target method
//! and anchor (`register_game_hooks`), and this module runs it where the game fires a
//! [`mod_api::Hooks`] hook at **exactly** that moment, with the receiver and arguments the target
//! method has, built from what the hook passes (`GameObject`, keyed as the event tier keys the
//! same object: a callback's `this` is that object).
//!
//! A site is bound only where the hook fires at the site's own moment and passes what the
//! callback needs ([`BOUND`]); every other site stays unbound and is listed with why
//! ([`UNBOUND`]), never run at a nearby moment. A Mixin `cancel()` reaches the game only where
//! the hook's cancel does what the Mixin's would; elsewhere a jar with a cancellable callback at
//! the site is not run there.
//!
//! What ran, failed or was refused is counted per jar (`JarRow::game_hooks`), for the report and
//! the in-game survey.

use std::collections::BTreeMap;

use crate::runtime::events::FacadePlayer;
use crate::runtime::{
    ExecutionBudget, GameHookContext, GameHookEffect, GameObject, MixinSide, NativeJar, Value,
};
use crate::{JAR_BUDGET, LOG, LoadedJar, State};

/// The sites this mod runs, and the mod API hook that fires at each one's moment.
pub(crate) const BOUND: &[(&str, &str)] = &[
    (
        "player_logged_in",
        "player_logged_in: PlayerList.placeNewPlayer TAIL",
    ),
    (
        "player_clone",
        "player_clone: ServerPlayer.restoreFrom TAIL",
    ),
    ("player_died", "player_died: ServerPlayer.die HEAD"),
    (
        "living_death",
        "living_death: LivingEntity.die HEAD (mobs; listen only)",
    ),
    ("level_unload", "level_unload: ServerLevel.close TAIL"),
    (
        "server_stopping",
        "server_stopping: MinecraftServer.stopServer HEAD",
    ),
    (
        "java_server_stopped",
        "server_stopped: MinecraftServer.stopServer TAIL",
    ),
    (
        "java_server_tick_head",
        "server_tick ServerTickStart: tickServer HEAD",
    ),
    (
        "java_server_tick_tail",
        "server_tick ServerTickEnd: tickServer TAIL",
    ),
    (
        "java_level_tick_head",
        "level_tick_pre: ServerLevel.tick HEAD",
    ),
    (
        "java_level_tick_tail",
        "level_tick_post: ServerLevel.tick TAIL",
    ),
    (
        "player_start_tracking",
        "player_start_tracking: ServerEntity.addPairing TAIL",
    ),
    ("attack_entity", "attack_entity: Player.attack HEAD"),
];

/// The sites the runtime knows that no pinned hook fires at exactly, and why.
pub(crate) const UNBOUND: &[(&str, &str)] = &[
    (
        "player_tick",
        "no player tick hook at the pin (entity_tick_* skip players)",
    ),
    ("java_player_tick_head", "no player tick hook at the pin"),
    (
        "java_base_player_tick_head",
        "no player tick hook at the pin",
    ),
    (
        "java_base_player_tick_tail",
        "no player tick hook at the pin",
    ),
    (
        "java_player_list_remove",
        "player_logged_out fires after the player left the list; PlayerList.remove HEAD is before",
    ),
    (
        "java_living_hurt",
        "living_damage_pre is actuallyHurt after armor, not hurtServer HEAD",
    ),
    (
        "java_living_died",
        "living_death_post is at die's broadcastEntityEvent, not its TAIL",
    ),
    (
        "java_explosion_explode",
        "explosion_detonate is after toBlow is computed, not explode HEAD",
    ),
    (
        "java_destroy_speed",
        "player_break_speed is not getDestroySpeed's RETURN",
    ),
    (
        "java_drop_death_loot_head",
        "living_drops is after the drops are collected, not dropAllDeathLoot HEAD",
    ),
    (
        "java_drop_death_loot_tail",
        "no hook at dropAllDeathLoot TAIL",
    ),
    (
        "java_block_player_destroy",
        "no hook at Block.playerDestroy HEAD",
    ),
    (
        "java_block_player_destroyed",
        "player_destroyed_block is Block.destroy, not playerDestroy TAIL",
    ),
    (
        "java_grow_crop",
        "crop_grow is the random-tick growth, not BoneMealItem.growCrop",
    ),
    (
        "java_server_constructed",
        "no hook at the server's constructor",
    ),
    (
        "java_dedicated_server_constructed",
        "no hook at the server's constructor",
    ),
    ("java_bootstrap_head", "no hook at Bootstrap.bootStrap"),
    ("java_bootstrap_tail", "no hook at Bootstrap.bootStrap"),
    (
        "item_picked_up",
        "the hook names no item entity id, the callback's `this`",
    ),
    (
        "living_jump",
        "one hook for LivingEntity and MagmaCube targets, and no Host in its loop",
    ),
    ("living_knockback", "no living_knockback hook at the pin"),
    ("entity_mount", "no entity_mount hook at the pin"),
    ("item_damaged", "no item_damaged hook at the pin"),
    (
        "left_click_block",
        "the hook is after the reach and protection checks, START only, not HEAD",
    ),
    (
        "player_container_open",
        "the hook is heard once a tick, not at openHorseInventory's TAIL",
    ),
    (
        "living_entity_use_item_stop",
        "the hook is after the use check, not releaseUsingItem HEAD",
    ),
    (
        "living_entity_use_item_tick",
        "its cancel finishes the use; a Mixin cancel at HEAD skips the tick",
    ),
];

/// The bound and unbound sites, for the report: which game-class Mixin targets run here, and why
/// the others do not.
pub(crate) fn sites_json() -> serde_json::Value {
    serde_json::json!({
        "bound": BOUND.iter().map(|(site, moment)| serde_json::json!({ "site": site, "moment": moment })).collect::<Vec<_>>(),
        "unbound": UNBOUND.iter().map(|(site, why)| serde_json::json!({ "site": site, "why": why })).collect::<Vec<_>>(),
    })
}

/// What one jar's game-class callbacks did, for the report and the survey.
#[derive(Clone, Debug, Default)]
pub(crate) struct Tally {
    /// Callbacks registered at a bound site, and at a site this mod does not run.
    pub bound: usize,
    pub unbound: usize,
    /// Callback runs, firings that failed (an error names the call the model refused) and
    /// firings skipped because a cancellable callback's `cancel()` would mean something else here.
    pub ran: usize,
    pub failed: usize,
    pub skipped: usize,
    /// Failure reasons, first line, with counts (at most 16 kept).
    pub failures: BTreeMap<String, usize>,
}

impl Tally {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "bound": self.bound,
            "unbound": self.unbound,
            "ran": self.ran,
            "failed": self.failed,
            "skipped": self.skipped,
            "failures": self.failures,
        })
    }
}

/// The game-hook registrations of one jar: its `@Inject`s a game hook site stands for.
pub(crate) fn register(
    jar: &NativeJar,
    tally: &mut Tally,
) -> crate::runtime::GameHookRegistrations {
    let weave = jar.plan_load_time_weave_with_game_hooks(MixinSide::Server);
    let registrations = jar.register_game_hooks(&weave).unwrap_or_default();
    for hook in registrations.hooks() {
        let count = registrations.entries(hook).len();
        if BOUND.iter().any(|(site, _)| *site == hook) {
            tally.bound += count;
        } else {
            tally.unbound += count;
        }
    }
    registrations
}

/// One argument of a target method: a game object, or a value (a `boolean`).
pub(crate) enum Arg {
    Object(GameObject),
    Value(Value),
}

/// A player as the bridge counts them: their session, life and entity id (`State::players`),
/// so the handle is the event tier's object for them.
pub(crate) fn player(state: &State, uuid: u128, life_back: u32) -> GameObject {
    let listed: Option<&FacadePlayer> = state.players.iter().find(|p| p.uuid == uuid);
    let mut object = GameObject::player(uuid, listed.map_or("", |p| p.name.as_str()));
    if let Some(listed) = listed {
        if let Some(life) = listed.life.checked_sub(life_back) {
            object = object.with_incarnation(listed.session, life);
        }
        if let Some(id) = listed.entity_id {
            object = object.with_entity_id(id);
        }
        if let Some(level) = &listed.level {
            object = object.with_dimension(level);
        }
    }
    object
}

/// An entity by id: the listed player of that id, else a living entity of `kind`.
pub(crate) fn entity(state: &State, id: i32, kind: &str) -> GameObject {
    match state.players.iter().find(|p| p.entity_id == Some(id)) {
        Some(listed) => player(state, listed.uuid, 0),
        None => GameObject::entity(id, kind, true),
    }
}

/// What a firing of one site did across the jars.
#[derive(Default)]
pub(crate) struct Fired {
    pub cancelled: bool,
    pub effects: Vec<GameHookEffect>,
}

/// Run every jar's callbacks at `site`, against `receiver` and `args` (the target method's).
/// `cancels` says whether the hook's cancel is the Mixin's: where it is not, a jar with a
/// cancellable callback at the site is skipped, so no `cancel()` is dropped or mistranslated.
pub(crate) fn fire(
    state: &mut State,
    site: &str,
    cancels: bool,
    receiver: Option<GameObject>,
    args: Vec<Arg>,
) -> Fired {
    let mut fired = Fired::default();
    let facts = std::sync::Arc::clone(&state.server);
    let dedicated = state.dedicated;
    for loaded in &mut state.jars {
        let LoadedJar {
            jar,
            host,
            hooks,
            hook_state,
            row,
            ..
        } = loaded;
        let entries = hooks.entries(site);
        if entries.is_empty() {
            continue;
        }
        if !cancels && entries.iter().any(|entry| entry.cancellable) {
            row.game_hooks.skipped += 1;
            continue;
        }
        let mut context = GameHookContext::new();
        context.set_server_facts(std::sync::Arc::clone(&facts));
        context.insert(GameObject::server(dedicated));
        let receiver = receiver.clone().map(|object| context.insert(object));
        let values: Vec<Value> = args
            .iter()
            .map(|arg| match arg {
                Arg::Object(object) => context.insert(object.clone()),
                Arg::Value(value) => value.clone(),
            })
            .collect();
        let budget = ExecutionBudget::within(JAR_BUDGET);
        let outcome = crate::caught(&format!("{} game hook {site}", row.id), || {
            jar.run_game_hook_in(
                hooks,
                hook_state,
                &mut context,
                site,
                receiver,
                &values,
                host,
                &budget,
            )
        });
        match outcome {
            Ok(Ok(run)) => {
                row.game_hooks.ran += run.ran;
                fired.cancelled |= run.cancelled;
            }
            Ok(Err(error)) => note_failure(row, site, &error.to_string()),
            Err(panic) => note_failure(row, site, &format!("panicked: {panic}")),
        }
        fired.effects.extend(context.take_effects());
    }
    fired
}

fn note_failure(row: &mut crate::JarRow, site: &str, reason: &str) {
    row.game_hooks.failed += 1;
    let line = format!("{site}: {}", reason.lines().next().unwrap_or_default());
    log::warn!("{LOG}: {}: game hook callback failed: {line}", row.id);
    let failures = &mut row.game_hooks.failures;
    if failures.len() < 16 || failures.contains_key(&line) {
        *failures.entry(line).or_default() += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// Compile-only stubs, and the jar's own classes: a Fabric entrypoint and a `ServerPlayer.die`
    /// HEAD Mixin that keeps a field on the player and prints its UUID.
    const SOURCES: &[(&str, &str)] = &[
        (
            "org/spongepowered/asm/mixin/Mixin.java",
            "package org.spongepowered.asm.mixin; import java.lang.annotation.*; \
             @Retention(RetentionPolicy.RUNTIME) @Target(ElementType.TYPE) \
             public @interface Mixin { Class<?>[] value() default {}; }",
        ),
        (
            "org/spongepowered/asm/mixin/injection/At.java",
            "package org.spongepowered.asm.mixin.injection; import java.lang.annotation.*; \
             @Retention(RetentionPolicy.RUNTIME) @Target({}) public @interface At { String value() default \"\"; }",
        ),
        (
            "org/spongepowered/asm/mixin/injection/Inject.java",
            "package org.spongepowered.asm.mixin.injection; import java.lang.annotation.*; \
             @Retention(RetentionPolicy.RUNTIME) @Target(ElementType.METHOD) \
             public @interface Inject { String[] method(); At[] at(); boolean cancellable() default false; }",
        ),
        (
            "org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java",
            "package org.spongepowered.asm.mixin.injection.callback; public class CallbackInfo {}",
        ),
        (
            "net/minecraft/world/damagesource/DamageSource.java",
            "package net.minecraft.world.damagesource; public class DamageSource {}",
        ),
        (
            "net/minecraft/server/level/ServerPlayer.java",
            "package net.minecraft.server.level; public class ServerPlayer { \
             public java.util.UUID getUUID() { return null; } }",
        ),
        (
            "demo/Main.java",
            "package demo; public final class Main { public void onInitialize() {} }",
        ),
        (
            "demo/DeathMixin.java",
            "package demo; \
             import net.minecraft.server.level.ServerPlayer; \
             import net.minecraft.world.damagesource.DamageSource; \
             import org.spongepowered.asm.mixin.Mixin; \
             import org.spongepowered.asm.mixin.injection.*; \
             import org.spongepowered.asm.mixin.injection.callback.CallbackInfo; \
             @Mixin(ServerPlayer.class) public class DeathMixin { \
               private int demo$deaths; \
               @Inject(method = \"die\", at = @At(\"HEAD\")) \
               private void onDie(DamageSource source, CallbackInfo ci) { \
                 demo$deaths++; \
                 System.out.println(((ServerPlayer) (Object) this).getUUID() + \" \" + demo$deaths); \
               } }",
        ),
    ];

    fn jar_bytes() -> Vec<u8> {
        let root =
            std::env::temp_dir().join(format!("horizon-java-mod-hooks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (src, classes) = (root.join("src"), root.join("classes"));
        let mut paths = Vec::new();
        for (path, body) in SOURCES {
            let file = src.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, body).unwrap();
            paths.push(file);
        }
        std::fs::create_dir_all(&classes).unwrap();
        let javac = std::env::var_os("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let status = std::process::Command::new(javac)
            .args(["--release", "17", "-nowarn", "-d"])
            .arg(&classes)
            .args(&paths)
            .status()
            .expect("javac runs");
        assert!(status.success(), "javac failed");
        let mut bytes = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default();
        let files = [
            (
                "fabric.mod.json",
                r#"{"id":"deathdemo","version":"1","entrypoints":{"main":["demo.Main"]},"mixins":["deathdemo.mixins.json"]}"#,
            ),
            (
                "deathdemo.mixins.json",
                r#"{"package":"demo","mixins":["DeathMixin"]}"#,
            ),
        ];
        for (path, body) in files {
            zip.start_file(path, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        for name in ["Main", "DeathMixin"] {
            let path: &Path = &classes.join(format!("demo/{name}.class"));
            zip.start_file(format!("demo/{name}.class"), options)
                .unwrap();
            zip.write_all(&std::fs::read(path).unwrap()).unwrap();
        }
        zip.finish().unwrap();
        let _ = std::fs::remove_dir_all(&root);
        bytes
    }

    /// The mod runs a jar's `ServerPlayer.die` HEAD Mixin where `player_died` fires: `this` is
    /// the player (its UUID), the field the Mixin adds stays on that player across deaths of one
    /// life, and starts again on the new `ServerPlayer` a respawn builds.
    #[test]
    fn a_bound_mixin_callback_runs_on_the_player_at_its_hook() {
        let jar = Arc::new(NativeJar::from_bytes(&jar_bytes()).unwrap());
        let imc = crate::runtime::neoforge_imc::ImcBus::default();
        let Ok(loaded) = crate::load_jar("deathdemo.jar".into(), jar, &[], &imc, None) else {
            panic!("the jar did not load")
        };
        assert_eq!(
            (loaded.row.game_hooks.bound, loaded.row.game_hooks.unbound),
            (1, 0)
        );
        let uuid = 0x0123_4567_89ab_cdef_0011_2233_4455_6677_u128;
        let mut state = State::default();
        state.jars.push(loaded);
        let die = |state: &mut State, life: u32| {
            state.players = vec![FacadePlayer {
                uuid,
                name: "Alex".into(),
                level: Some("minecraft:overworld".into()),
                entity_id: Some(7),
                session: 1,
                life,
            }];
            let player = player(state, uuid, 0);
            let source =
                GameObject::opaque("net/minecraft/world/damagesource/DamageSource", "lava");
            fire(
                state,
                "player_died",
                true,
                Some(player),
                vec![Arg::Object(source)],
            );
        };
        die(&mut state, 0);
        die(&mut state, 0);
        die(&mut state, 1);
        let loaded = &mut state.jars[0];
        assert_eq!(
            (loaded.row.game_hooks.ran, loaded.row.game_hooks.failed),
            (3, 0)
        );
        assert_eq!(
            loaded.host.take_stdout().lines().collect::<Vec<_>>(),
            [
                "01234567-89ab-cdef-0011-223344556677 1",
                "01234567-89ab-cdef-0011-223344556677 2",
                "01234567-89ab-cdef-0011-223344556677 1",
            ]
        );
    }

    /// Every site the runtime knows is either bound to a pinned hook at its moment or listed as
    /// unbound with why, and never both.
    #[test]
    fn every_site_is_bound_or_listed_unbound() {
        fn named(list: &'static [(&'static str, &'static str)]) -> Vec<&'static str> {
            list.iter().map(|(site, _)| *site).collect()
        }
        let (bound, unbound) = (named(BOUND), named(UNBOUND));
        for site in crate::runtime::every_game_hook_site() {
            let listed = bound.contains(&site.hook) as u8 + unbound.contains(&site.hook) as u8;
            assert_eq!(listed, 1, "{} is listed {listed} times", site.hook);
        }
        for site in bound.iter().chain(&unbound) {
            assert!(
                crate::runtime::every_game_hook_site().any(|known| known.hook == *site),
                "{site} is no runtime site"
            );
        }
    }
}

//! A mod's own `BlockPos`, `MutableBlockPos` and `LevelResource` equal the
//! game's as Java's do (#245): `equals` both ways, `hashCode`, `HashMap`
//! lookups and `HashSet.contains` across the two, `compareTo` and a mixed
//! `TreeSet`, a `MutableBlockPos` changed after it went into a `HashMap`, and
//! a `LevelResource` against the game's constant.
//!
//! `java25.txt` is what `Oracle.java` printed on OpenJDK 25.0.5+8 against the
//! real 26.3 server classes (`Bootstrap.bootStrap()`, two runs alike; kept on
//! elysium under `~/ava/results/rv245/`): it calls the same `Probe.ask` with
//! vanilla's own positions where the runtime hands the game's handles. The
//! game stubs only let `javac` compile the probe and are never packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, Host, StdHost, Value,
    events::{EventContext, EventFacade, EventFiring, GameEvent},
};
use std::sync::Arc;

const ORACLE: &str = include_str!("fixtures/jar_server_values/java25.txt");
const PROBE: &str = include_str!("fixtures/jar_server_values/demo/Probe.java");

const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event; public abstract class Event<T> { public void register(T l) {} }",
    ),
    (
        "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.java",
        "package net.fabricmc.fabric.api.event.lifecycle.v1; public final class ServerTickEvents { \
         public static final net.fabricmc.fabric.api.event.Event<EndTick> END_SERVER_TICK = null; \
         public interface EndTick { void onEndTick(net.minecraft.server.MinecraftServer server); } }",
    ),
    (
        "net/fabricmc/fabric/api/event/player/PlayerBlockBreakEvents.java",
        "package net.fabricmc.fabric.api.event.player; public final class PlayerBlockBreakEvents { \
         public static final net.fabricmc.fabric.api.event.Event<After> AFTER = null; \
         public interface After { void afterBlockBreak(net.minecraft.world.level.Level level, \
         net.minecraft.world.entity.player.Player player, net.minecraft.core.BlockPos pos, \
         net.minecraft.world.level.block.state.BlockState state, \
         net.minecraft.world.level.block.entity.BlockEntity entity); } }",
    ),
    (
        "net/minecraft/server/MinecraftServer.java",
        "package net.minecraft.server; public class MinecraftServer {}",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level; public class Level {}",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player; public class Player {}",
    ),
    (
        "net/minecraft/world/level/block/state/BlockState.java",
        "package net.minecraft.world.level.block.state; public class BlockState {}",
    ),
    (
        "net/minecraft/world/level/block/entity/BlockEntity.java",
        "package net.minecraft.world.level.block.entity; public class BlockEntity {}",
    ),
    (
        "net/minecraft/core/Vec3i.java",
        "package net.minecraft.core; public class Vec3i implements Comparable<Vec3i> { \
         public Vec3i(int x, int y, int z) {} \
         public int getX() { return 0; } public int getY() { return 0; } public int getZ() { return 0; } \
         public boolean equals(Object o) { return false; } public int hashCode() { return 0; } \
         public String toString() { return null; } public int compareTo(Vec3i o) { return 0; } \
         protected Vec3i setZ(int z) { return this; } }",
    ),
    (
        "net/minecraft/core/BlockPos.java",
        "package net.minecraft.core; public class BlockPos extends Vec3i { \
         public BlockPos(int x, int y, int z) { super(x, y, z); } \
         public static class MutableBlockPos extends BlockPos { \
           public MutableBlockPos() { super(0, 0, 0); } \
           public MutableBlockPos(int x, int y, int z) { super(x, y, z); } \
           public MutableBlockPos(double x, double y, double z) { super(0, 0, 0); } \
           public MutableBlockPos set(int x, int y, int z) { return this; } \
           public MutableBlockPos set(long packed) { return this; } \
           public MutableBlockPos setY(int y) { return this; } \
           public MutableBlockPos move(int x, int y, int z) { return this; } \
           public MutableBlockPos move(Direction d) { return this; } \
           public MutableBlockPos move(Direction d, int n) { return this; } \
           public MutableBlockPos setWithOffset(Vec3i v, int x, int y, int z) { return this; } \
           public MutableBlockPos setWithOffset(Vec3i v, Direction d) { return this; } } }",
    ),
    (
        "net/minecraft/core/Direction.java",
        "package net.minecraft.core; public enum Direction { DOWN, UP, NORTH, SOUTH, WEST, EAST }",
    ),
    (
        "net/minecraft/world/level/storage/LevelResource.java",
        "package net.minecraft.world.level.storage; public final class LevelResource { \
         public static final LevelResource DATA = null; \
         public LevelResource(String id) {} public String id() { return null; } }",
    ),
];

fn statics(host: &StdHost, field: &str) -> String {
    match host
        .persistent_class_state()
        .0
        .remove(&format!("demo/Probe#{field}Ljava/lang/String;"))
    {
        Some(Value::Ref(Some(text))) => text,
        other => panic!("{field}: {other:?}"),
    }
}

#[test]
fn a_mods_own_positions_and_resources_meet_the_games_as_in_java() {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    let fixture = javac_fixture::Fixture::build("jar-server-values", &sources);
    let jar = &fixture.jar;
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let listeners = host.game_listeners(jar, true).listeners;
    let fire = |host: &mut StdHost, event: GameEvent, context: EventContext| {
        let mut firing = EventFiring::new(event, Arc::new(EventFacade::default()), context);
        for listener in listeners.iter().filter(|l| l.event == event) {
            jar.run_game_listener(listener, &mut firing, host, &ExecutionBudget::UNBOUNDED)
                .unwrap_or_else(|error| panic!("{event:?} refused: {error}"));
        }
    };
    // The game's positions: two broken blocks.
    for pos in [[1, 2, 3], [1, 2, 4]] {
        let context = EventContext {
            pos: Some(pos),
            block: Some("minecraft:stone".into()),
            ..EventContext::default()
        };
        fire(&mut host, GameEvent::BlockBreakAfter, context);
    }
    fire(&mut host, GameEvent::ServerTickEnd, EventContext::default());

    // Every line is Java's, and none of them was counted.
    assert_eq!(statics(&host, "log"), ORACLE);

    // A subclass overriding `getX`, which `Vec3i.equals` calls, both ways:
    // Java answers `false,false` from the subclass's `getX`; the runtime
    // does not run it there, so both comparisons are counted.
    // Then a subclass that moves itself through `Vec3i`'s protected `setZ`,
    // which the model does not run: Java's `true,3907` reads the new place;
    // the runtime forgets the fields, so both answers after it are counted,
    // never read from the old place (#259's review).
    assert_eq!(statics(&host, "counted"), "false,false;false,0");
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    assert_eq!(
        counted,
        [
            "demo/Probe$Odd#equals(Ljava/lang/Object;)Z",
            "net/minecraft/core/BlockPos#equals(Ljava/lang/Object;)Z",
            "net/minecraft/core/BlockPos#hashCode()I",
            "net/minecraft/core/BlockPos#setZ(I)Lnet/minecraft/core/Vec3i;",
            "net/minecraft/core/Vec3i#equals(Ljava/lang/Object;)Z",
            "net/minecraft/core/Vec3i#hashCode()I",
        ],
        "{counted:?}"
    );
}

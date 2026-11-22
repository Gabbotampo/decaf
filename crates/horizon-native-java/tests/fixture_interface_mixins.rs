//! Interface-injection `@Mixin` fixtures (`@Mixin(Target.class) implements I` +
//! added fields/methods on host-modelled game objects), negative fixtures for
//! unsupported mixin forms, and an opt-in real-jar test gated on `RUNES_JAR`.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{
    ExecutionBudget, GameHookContext, GameHookState, GameObject, MixinSide, MixinSkipReason,
    NativeError, NativeJar, StdHost, Value,
    client_surface::{ClientLevel, ClientPlayer, ClientSnapshot},
    events::{EventContext, EventFacade, EventFiring, FacadeEntity, FacadePlayer, GameEvent},
};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use zip::write::SimpleFileOptions;

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn java_bin() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/java"))
        .unwrap_or_else(|| "java".into())
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn build_jar(
    name: &str,
    sources: BTreeMap<&str, &str>,
    resources: &[(&str, &str)],
    excluded: &[&str],
) -> scratch::Scratch {
    let root = scratch::Scratch::new(&format!("interface-mixin-{name}"));
    let src = root.join("src");
    let classes = root.join("classes");
    fs::create_dir_all(&classes).unwrap();
    let mut paths = Vec::new();
    for (relative, body) in sources {
        let path = src.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, body).unwrap();
        paths.push(path);
    }
    let output = Command::new(javac())
        .arg("--release")
        .arg("17")
        .arg("-d")
        .arg(&classes)
        .args(&paths)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "javac failed for {name}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let jar_path = root.join(format!("{name}.jar"));
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    for (path, body) in resources {
        zip.start_file(*path, SimpleFileOptions::default()).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    for class in files_under(&classes) {
        let entry = class
            .strip_prefix(&classes)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if excluded.contains(&entry.as_str()) {
            continue;
        }
        zip.start_file(entry, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    root.into_path(jar_path)
}

fn mixin_stubs() -> BTreeMap<&'static str, &'static str> {
    let mut sources = BTreeMap::new();
    // Real SpongePowered Mixin uses RetentionPolicy.CLASS for @Mixin, @Shadow, @Unique.
    sources.insert(
        "org/spongepowered/asm/mixin/Mixin.java",
        r#"package org.spongepowered.asm.mixin;
import java.lang.annotation.*;
@Retention(RetentionPolicy.CLASS)
@Target(ElementType.TYPE)
public @interface Mixin {
    Class<?>[] value() default {};
    String[] targets() default {};
}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/Unique.java",
        r#"package org.spongepowered.asm.mixin;
import java.lang.annotation.*;
@Retention(RetentionPolicy.CLASS)
@Target({ElementType.FIELD, ElementType.METHOD})
public @interface Unique {}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/Shadow.java",
        r#"package org.spongepowered.asm.mixin;
import java.lang.annotation.*;
@Retention(RetentionPolicy.CLASS)
@Target({ElementType.FIELD, ElementType.METHOD})
public @interface Shadow {}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/Overwrite.java",
        r#"package org.spongepowered.asm.mixin;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Overwrite {}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/injection/At.java",
        r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target({})
public @interface At {
    String value();
    String target() default "";
}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/injection/Inject.java",
        r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Inject {
    String[] method();
    At[] at();
}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/injection/Redirect.java",
        r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Redirect {
    String[] method();
    At at();
}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java",
        r#"package org.spongepowered.asm.mixin.injection.callback;
public class CallbackInfo {}
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/gen/Accessor.java",
        r#"package org.spongepowered.asm.mixin.gen;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Accessor { String value() default ""; }
"#,
    );
    sources.insert(
        "org/spongepowered/asm/mixin/gen/Invoker.java",
        r#"package org.spongepowered.asm.mixin.gen;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Invoker { String value() default ""; }
"#,
    );
    sources.insert(
        "net/minecraft/world/entity/Entity.java",
        r#"package net.minecraft.world.entity;
public class Entity {}
"#,
    );
    sources.insert(
        "net/minecraft/world/entity/LivingEntity.java",
        r#"package net.minecraft.world.entity;
public class LivingEntity extends Entity {}
"#,
    );
    sources.insert(
        "net/minecraft/world/entity/player/Player.java",
        r#"package net.minecraft.world.entity.player;
import net.minecraft.world.entity.LivingEntity;
public class Player extends LivingEntity {}
"#,
    );
    sources.insert(
        "net/minecraft/server/level/ServerPlayer.java",
        r#"package net.minecraft.server.level;
import net.minecraft.world.entity.player.Player;
public class ServerPlayer extends Player {}
"#,
    );
    sources.insert(
        "net/minecraft/world/level/Level.java",
        r#"package net.minecraft.world.level;
public class Level {}
"#,
    );
    sources.insert(
        "net/minecraft/world/level/chunk/LevelChunk.java",
        r#"package net.minecraft.world.level.chunk;
public class LevelChunk {}
"#,
    );
    sources
}

const COMPILE_ONLY_STUBS: &[&str] = &[
    "org/spongepowered/asm/mixin/Mixin.class",
    "org/spongepowered/asm/mixin/Unique.class",
    "org/spongepowered/asm/mixin/Shadow.class",
    "org/spongepowered/asm/mixin/Overwrite.class",
    "org/spongepowered/asm/mixin/injection/At.class",
    "org/spongepowered/asm/mixin/injection/Inject.class",
    "org/spongepowered/asm/mixin/injection/Redirect.class",
    "org/spongepowered/asm/mixin/injection/callback/CallbackInfo.class",
    "org/spongepowered/asm/mixin/gen/Accessor.class",
    "org/spongepowered/asm/mixin/gen/Invoker.class",
    "net/minecraft/world/entity/Entity.class",
    "net/minecraft/world/entity/LivingEntity.class",
    "net/minecraft/world/entity/player/Player.class",
    "net/minecraft/server/level/ServerPlayer.class",
    "net/minecraft/world/level/Level.class",
    "net/minecraft/world/level/chunk/LevelChunk.class",
];

fn server_player(uuid: u128, name: &str, session: u32, life: u32, id: i32) -> Value {
    GameObject::player(uuid, name)
        .with_incarnation(session, life)
        .with_entity_id(id)
        .value()
}

/// (a) Positive fixture: `@Mixin(Player.class) implements ExtendedCarrier` and
/// `@Mixin(Level.class) implements LevelTag`.
///
/// Proves:
/// - `plan_load_time_weave` records both in `weave.interface_mixins` with zero skipped rows;
/// - `instanceof` and `checkcast` succeed for the direct interface and its superinterface;
/// - `invokeinterface` runs mixin methods, mixin overrides of default methods, default
///   interface methods calling back into mixin methods, and private mixin helpers;
/// - `<init>()V` field initialisers run once per host object identity;
/// - separate players keep independent state;
/// - primitive and jar-object fields survive across calls and forced heap GC (GC roots them);
/// - respawn (new life) and `StdHost::forget_game_object` start fresh;
/// - zero unknown host calls (verified with strict unknown host calls enabled).
#[test]
fn interface_mixin_on_host_objects_casts_dispatches_isolates_and_roots_through_gc() {
    let mut sources = mixin_stubs();
    sources.insert(
        "demo/Box.java",
        r#"package demo;
public final class Box {
    private final int id;
    private final String label;
    public Box(int id, String label) {
        this.id = id;
        this.label = label;
    }
    public String describe() {
        return id + ":" + label;
    }
}
"#,
    );
    sources.insert(
        "demo/CooldownCarrier.java",
        r#"package demo;
public interface CooldownCarrier {
    int getLastTick();
    void setLastTick(int tick);
    default boolean canTrigger(int now) {
        return now > getLastTick() + 20;
    }
    default void recordTrigger(int now) {
        setLastTick(now);
    }
}
"#,
    );
    sources.insert(
        "demo/ExtendedCarrier.java",
        r#"package demo;
public interface ExtendedCarrier extends CooldownCarrier {
    int getSeed();
    String getTag();
    void setTag(String tag);
    Box getBox();
    void setBox(Box box);
    default int overriddenDefault() { return 1; }
    default int inheritedDefault() { return 99; }
    int computeWithHelper(int delta);
}
"#,
    );
    sources.insert(
        "demo/LevelTag.java",
        r#"package demo;
public interface LevelTag {
    int getWorldTag();
    void setWorldTag(int tag);
}
"#,
    );
    sources.insert(
        "demo/PlayerCarrierMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;

@Mixin(Player.class)
public class PlayerCarrierMixin implements ExtendedCarrier {
    private int lastTick;
    @Unique
    private int seed = 7;
    @Unique
    private String tag = "init";
    @Unique
    private Box box;

    @Override
    public int getLastTick() {
        return this.lastTick;
    }

    @Override
    public void setLastTick(int tick) {
        this.lastTick = tick;
    }

    @Override
    public int getSeed() {
        return this.seed;
    }

    @Override
    public String getTag() {
        return this.tag;
    }

    @Override
    public void setTag(String tag) {
        this.tag = tag;
    }

    @Override
    public Box getBox() {
        return this.box;
    }

    @Override
    public void setBox(Box box) {
        this.box = box;
    }

    @Override
    public int overriddenDefault() {
        return 42;
    }

    @Override
    public int computeWithHelper(int delta) {
        return this.helperAdd(delta);
    }

    @Unique
    private int helperAdd(int delta) {
        return this.lastTick + this.seed + delta;
    }
}
"#,
    );
    sources.insert(
        "demo/LevelTagMixin.java",
        r#"package demo;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;

@Mixin(Level.class)
public class LevelTagMixin implements LevelTag {
    @Unique
    private int worldTag = 11;

    @Override
    public int getWorldTag() {
        return this.worldTag;
    }

    @Override
    public void setWorldTag(int tag) {
        this.worldTag = tag;
    }
}
"#,
    );
    sources.insert(
        "demo/Harness.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

public final class Harness {
    public static String inspectAndCast(Object obj) {
        boolean isExt = obj instanceof ExtendedCarrier;
        boolean isSuper = obj instanceof CooldownCarrier;
        boolean clsCheck = CooldownCarrier.class.isInstance(obj);
        ExtendedCarrier ext = (ExtendedCarrier) obj;
        CooldownCarrier base = (CooldownCarrier) obj;
        return isExt + "," + isSuper + "," + clsCheck
            + "," + ext.getSeed()
            + "," + ext.getTag()
            + "," + base.getLastTick()
            + "," + ext.overriddenDefault()
            + "," + ext.inheritedDefault()
            + "," + (ext.getBox() == null);
    }

    public static String exerciseCooldown(Player player, int t1, int t2, int t3, int t4) {
        CooldownCarrier c = (CooldownCarrier) player;
        boolean r1 = c.canTrigger(t1);
        c.recordTrigger(t1);
        boolean r2 = c.canTrigger(t2);
        boolean r3 = c.canTrigger(t3);
        boolean r4 = c.canTrigger(t4);
        return r1 + "," + r2 + "," + r3 + "," + r4 + "," + c.getLastTick();
    }

    public static void storeState(Player player, int tick, String tag, int boxId, String boxLabel) {
        ExtendedCarrier ext = (ExtendedCarrier) player;
        ext.setLastTick(tick);
        ext.setTag(tag);
        ext.setBox(new Box(boxId, boxLabel));
    }

    public static String readState(Player player, int delta) {
        ExtendedCarrier ext = (ExtendedCarrier) player;
        Box b = ext.getBox();
        return ext.getLastTick()
            + "|" + ext.getSeed()
            + "|" + ext.getTag()
            + "|" + (b == null ? "null" : b.describe())
            + "|" + ext.computeWithHelper(delta);
    }

    public static int churnGarbage(int count) {
        int sum = 0;
        for (int i = 0; i < count; i++) {
            Box g = new Box(i, "garbage-" + i);
            sum += g.describe().length();
        }
        return sum;
    }

    public static String exerciseLevel(Level level, int nextTag) {
        boolean ok = level instanceof LevelTag;
        LevelTag tagged = (LevelTag) level;
        int before = tagged.getWorldTag();
        tagged.setWorldTag(nextTag);
        return ok + "," + before + "," + tagged.getWorldTag();
    }
}
"#,
    );

    let jar_path = build_jar(
        "positive",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"demo","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["PlayerCarrierMixin","LevelTagMixin"]}"#,
            ),
        ],
        COMPILE_ONLY_STUBS,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // Verify planning records both interface-injection mixins and skips nothing.
    let weave = jar.plan_load_time_weave(MixinSide::Server);
    assert!(
        weave.skipped.is_empty(),
        "expected zero skipped plans, got: {:#?}",
        weave.skipped
    );
    assert_eq!(
        weave.interface_mixins.len(),
        2,
        "{:#?}",
        weave.interface_mixins
    );
    let player_plan = weave
        .interface_mixins
        .iter()
        .find(|p| p.mixin == "demo/PlayerCarrierMixin")
        .expect("PlayerCarrierMixin planned");
    assert_eq!(
        player_plan.target,
        "net/minecraft/world/entity/player/Player"
    );
    assert_eq!(player_plan.interfaces, vec!["demo/ExtendedCarrier"]);
    assert!(player_plan.has_initializer);
    assert_eq!(player_plan.added_fields.len(), 4);

    let mut host = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();

    let alice = server_player(1, "Alice", 1, 0, 10);
    let bob = server_player(2, "Bob", 1, 0, 20);
    let alice_respawned = server_player(1, "Alice", 1, 1, 10);
    let overworld = GameObject::level("minecraft:overworld").value();

    // 1. instanceof, checkcast, Class.isInstance, <init>()V, default & overridden methods.
    let v = jar
        .invoke_method_with_args(
            "demo.Harness",
            "inspectAndCast",
            "(Ljava/lang/Object;)Ljava/lang/String;",
            None,
            std::slice::from_ref(&alice),
            &mut host,
        )
        .unwrap();
    assert_eq!(
        v,
        Value::Ref(Some("true,true,true,7,init,0,42,99,true".into()))
    );

    // 2. Default interface methods delegating to mixin getter/setter.
    let v = jar
        .invoke_method_with_args(
            "demo.Harness",
            "exerciseCooldown",
            "(Lnet/minecraft/world/entity/player/Player;IIII)Ljava/lang/String;",
            None,
            &[
                alice.clone(),
                Value::Int(100),
                Value::Int(110),
                Value::Int(120),
                Value::Int(121),
            ],
            &mut host,
        )
        .unwrap();
    assert_eq!(v, Value::Ref(Some("true,false,false,true,100".into())));

    // 3. Store primitive + jar object on Alice, different state on Bob.
    jar.invoke_method_with_args(
        "demo.Harness",
        "storeState",
        "(Lnet/minecraft/world/entity/player/Player;ILjava/lang/String;ILjava/lang/String;)V",
        None,
        &[
            alice.clone(),
            Value::Int(100),
            Value::Ref(Some("alice".into())),
            Value::Int(314),
            Value::Ref(Some("alice-box".into())),
        ],
        &mut host,
    )
    .unwrap();

    // Bob starts completely clean and gets his own state.
    let v = jar
        .invoke_method_with_args(
            "demo.Harness",
            "readState",
            "(Lnet/minecraft/world/entity/player/Player;I)Ljava/lang/String;",
            None,
            &[bob.clone(), Value::Int(5)],
            &mut host,
        )
        .unwrap();
    assert_eq!(v, Value::Ref(Some("0|7|init|null|12".into())));

    jar.invoke_method_with_args(
        "demo.Harness",
        "storeState",
        "(Lnet/minecraft/world/entity/player/Player;ILjava/lang/String;ILjava/lang/String;)V",
        None,
        &[
            bob.clone(),
            Value::Int(500),
            Value::Ref(Some("bob".into())),
            Value::Int(271),
            Value::Ref(Some("bob-box".into())),
        ],
        &mut host,
    )
    .unwrap();

    // 4. Non-player host object (Level) also supports interface-injection mixins.
    let v = jar
        .invoke_method_with_args(
            "demo.Harness",
            "exerciseLevel",
            "(Lnet/minecraft/world/level/Level;I)Ljava/lang/String;",
            None,
            &[overworld.clone(), Value::Int(555)],
            &mut host,
        )
        .unwrap();
    assert_eq!(v, Value::Ref(Some("true,11,555".into())));

    // 5. Force multiple GC collections by churning >3,000 short-lived objects across calls.
    for _ in 0..3 {
        jar.invoke_method_with_args(
            "demo.Harness",
            "churnGarbage",
            "(I)I",
            None,
            &[Value::Int(1500)],
            &mut host,
        )
        .unwrap();
    }

    // Both Alice and Bob's primitive fields AND jar objects survived GC intact!
    let alice_after = jar
        .invoke_method_with_args(
            "demo.Harness",
            "readState",
            "(Lnet/minecraft/world/entity/player/Player;I)Ljava/lang/String;",
            None,
            &[alice.clone(), Value::Int(3)],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        alice_after,
        Value::Ref(Some("100|7|alice|314:alice-box|110".into()))
    );

    let bob_after = jar
        .invoke_method_with_args(
            "demo.Harness",
            "readState",
            "(Lnet/minecraft/world/entity/player/Player;I)Ljava/lang/String;",
            None,
            &[bob.clone(), Value::Int(3)],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        bob_after,
        Value::Ref(Some("500|7|bob|271:bob-box|510".into()))
    );

    // Level state also survived GC.
    let level_after = jar
        .invoke_method_with_args(
            "demo.Harness",
            "exerciseLevel",
            "(Lnet/minecraft/world/level/Level;I)Ljava/lang/String;",
            None,
            &[overworld, Value::Int(777)],
            &mut host,
        )
        .unwrap();
    assert_eq!(level_after, Value::Ref(Some("true,555,777".into())));

    // 6. Respawn (new life) starts fresh.
    let respawned = jar
        .invoke_method_with_args(
            "demo.Harness",
            "readState",
            "(Lnet/minecraft/world/entity/player/Player;I)Ljava/lang/String;",
            None,
            &[alice_respawned, Value::Int(1)],
            &mut host,
        )
        .unwrap();
    assert_eq!(respawned, Value::Ref(Some("0|7|init|null|8".into())));

    // 7. Forgetting a player clears its fields and initializer record so it starts fresh.
    let alice_marker = GameObject::player(1, "Alice")
        .with_incarnation(1, 0)
        .with_entity_id(10)
        .marker();
    host.forget_game_object(&alice_marker);
    let alice_forgotten = jar
        .invoke_method_with_args(
            "demo.Harness",
            "readState",
            "(Lnet/minecraft/world/entity/player/Player;I)Ljava/lang/String;",
            None,
            &[alice, Value::Int(1)],
            &mut host,
        )
        .unwrap();
    assert_eq!(alice_forgotten, Value::Ref(Some("0|7|init|null|8".into())));

    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "supported interface-injection mixin ops must count zero unknown host calls: {:?}",
        host.unknown_host_calls()
    );
}

/// Positive linked-jar fixture: a consumer jar casts a host `Player` to an interface
/// injected by a linked library jar's `@Mixin(Player.class)` and invokes its methods.
#[test]
fn linked_jar_interface_mixin_applies_to_host_player() {
    let mut provider_sources = mixin_stubs();
    provider_sources.insert(
        "lib/Counter.java",
        r#"package lib;
public interface Counter {
    int get();
    void set(int v);
    default int bump() {
        int next = get() + 1;
        set(next);
        return next;
    }
}
"#,
    );
    provider_sources.insert(
        "lib/PlayerCounterMixin.java",
        r#"package lib;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class PlayerCounterMixin implements Counter {
    private int count = 10;
    @Override public int get() { return this.count; }
    @Override public void set(int v) { this.count = v; }
}
"#,
    );
    let provider_path = build_jar(
        "linked-provider",
        provider_sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"lib","version":"1.0","mixins":["lib.mixins.json"]}"#,
            ),
            (
                "lib.mixins.json",
                r#"{"required":true,"package":"lib","mixins":["PlayerCounterMixin"]}"#,
            ),
        ],
        COMPILE_ONLY_STUBS,
    );
    let provider = NativeJar::from_path(&provider_path).unwrap();

    let mut consumer_sources = mixin_stubs();
    consumer_sources.insert(
        "lib/Counter.java",
        r#"package lib;
public interface Counter {
    int get();
    void set(int v);
    default int bump() { return 0; }
}
"#,
    );
    consumer_sources.insert(
        "app/UseCounter.java",
        r#"package app;
import lib.Counter;
import net.minecraft.world.entity.player.Player;

public final class UseCounter {
    public static int step(Player player) {
        if (!(player instanceof Counter)) return -1;
        return ((Counter) player).bump();
    }
}
"#,
    );
    let mut consumer_excluded = COMPILE_ONLY_STUBS.to_vec();
    consumer_excluded.push("lib/Counter.class");
    let consumer_path = build_jar(
        "linked-consumer",
        consumer_sources,
        &[("fabric.mod.json", r#"{"id":"app","version":"1.0"}"#)],
        &consumer_excluded,
    );
    let consumer = NativeJar::from_path(&consumer_path).unwrap();

    let mut host = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    host.link_jar("lib", &provider).unwrap();

    let player = server_player(42, "Steve", 1, 0, 7);
    let first = consumer
        .invoke_method_with_args(
            "app.UseCounter",
            "step",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&player),
            &mut host,
        )
        .unwrap();
    let second = consumer
        .invoke_method_with_args(
            "app.UseCounter",
            "step",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&player),
            &mut host,
        )
        .unwrap();
    assert_eq!(first, Value::Int(11));
    assert_eq!(second, Value::Int(12));
    assert_eq!(host.unknown_host_call_count(), 0);
}

/// Positive fixture: `LocalPlayer` (`client/player`) inherits `@Mixin(Player.class)`
/// interfaces on the client, and `StdHost` + `run_game_hook_in` share per-player
/// interface-mixin fields consistently across tiers.
#[test]
fn client_player_and_game_hook_share_interface_mixin_state() {
    let mut sources = mixin_stubs();
    sources.insert(
        "net/minecraft/client/player/AbstractClientPlayer.java",
        r#"package net.minecraft.client.player;
import net.minecraft.world.entity.player.Player;
public class AbstractClientPlayer extends Player {}
"#,
    );
    sources.insert(
        "net/minecraft/client/player/LocalPlayer.java",
        r#"package net.minecraft.client.player;
public class LocalPlayer extends AbstractClientPlayer {}
"#,
    );
    sources.insert(
        "net/minecraft/client/Minecraft.java",
        r#"package net.minecraft.client;
import net.minecraft.client.player.LocalPlayer;
public class Minecraft {
    public LocalPlayer player;
    public static Minecraft getInstance() { return null; }
}
"#,
    );
    sources.insert(
        "demo/PlayerState.java",
        r#"package demo;
public interface PlayerState {
    int getCounter();
    void setCounter(int v);
}
"#,
    );
    sources.insert(
        "demo/PlayerStateMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class PlayerStateMixin implements PlayerState {
    private int counter = 5;
    @Override public int getCounter() { return this.counter; }
    @Override public void setCounter(int v) { this.counter = v; }
}
"#,
    );
    sources.insert(
        "demo/TickHookMixin.java",
        r#"package demo;
import net.minecraft.server.level.ServerPlayer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerPlayer.class)
public abstract class TickHookMixin {
    @Inject(method = "tick()V", at = @At("TAIL"))
    private void onTick(CallbackInfo ci) {
        PlayerState state = (PlayerState) (Object) this;
        state.setCounter(state.getCounter() + 10);
    }
}
"#,
    );
    sources.insert(
        "demo/Bridge.java",
        r#"package demo;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.player.Player;

public final class Bridge {
    public static int bumpClientPlayer() {
        Player p = Minecraft.getInstance().player;
        if (!(p instanceof PlayerState)) return -1;
        PlayerState s = (PlayerState) p;
        int next = s.getCounter() + 1;
        s.setCounter(next);
        return next;
    }

    public static int readServer(Player p) {
        return ((PlayerState) p).getCounter();
    }

    public static void writeServer(Player p, int v) {
        ((PlayerState) p).setCounter(v);
    }
}
"#,
    );

    let mut excluded = COMPILE_ONLY_STUBS.to_vec();
    excluded.extend([
        "net/minecraft/client/player/AbstractClientPlayer.class",
        "net/minecraft/client/player/LocalPlayer.class",
        "net/minecraft/client/Minecraft.class",
    ]);
    let jar_path = build_jar(
        "client-and-hook",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"bridge","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["PlayerStateMixin","TickHookMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // 1. Client LocalPlayer inherits @Mixin(Player.class) interface and keeps per-player state.
    let mut client_host = StdHost::new()
        .with_environment_type("CLIENT")
        .with_strict_unknown_host_calls();
    let snapshot = Arc::new(ClientSnapshot {
        player: Some(ClientPlayer::default()),
        ..ClientSnapshot::default()
    });
    client_host.begin_client_frame(snapshot, &[]);
    let c1 = jar
        .invoke_method(
            "demo.Bridge",
            "bumpClientPlayer",
            "()I",
            None,
            &mut client_host,
        )
        .unwrap();
    let c2 = jar
        .invoke_method(
            "demo.Bridge",
            "bumpClientPlayer",
            "()I",
            None,
            &mut client_host,
        )
        .unwrap();
    client_host.end_client_frame();
    assert_eq!(c1, Value::Int(6));
    assert_eq!(c2, Value::Int(7));
    assert_eq!(client_host.unknown_host_call_count(), 0);

    // 2. Server StdHost + GameHookHost share per-player state seamlessly.
    let weave = jar.plan_load_time_weave_with_game_hooks(MixinSide::Server);
    assert_eq!(weave.interface_mixins.len(), 1);
    assert_eq!(weave.game_hooks.len(), 1);
    let registrations = jar.register_game_hooks(&weave).unwrap();

    let mut host = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    let mut hook_state = GameHookState::default();
    let budget = ExecutionBudget::within(Duration::from_secs(30));

    let player_obj = GameObject::player(77, "Alex")
        .with_incarnation(1, 0)
        .with_entity_id(15);
    let player_val = player_obj.value();

    // Write via StdHost invocation -> 20
    jar.invoke_method_with_args(
        "demo.Bridge",
        "writeServer",
        "(Lnet/minecraft/world/entity/player/Player;I)V",
        None,
        &[player_val.clone(), Value::Int(20)],
        &mut host,
    )
    .unwrap();

    // Fire game-hook tick -> reads 20, adds 10 -> 30
    let mut ctx = GameHookContext::new();
    let recv = ctx.insert(player_obj.clone());
    jar.run_game_hook_in(
        &registrations,
        &mut hook_state,
        &mut ctx,
        "player_tick",
        Some(recv),
        &[],
        &mut host,
        &budget,
    )
    .unwrap();

    // Read back via StdHost -> sees 30!
    let after = jar
        .invoke_method_with_args(
            "demo.Bridge",
            "readServer",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&player_val),
            &mut host,
        )
        .unwrap();
    assert_eq!(after, Value::Int(30));
    assert_eq!(host.unknown_host_call_count(), 0);
}

/// (b) Negative fixtures: unsupported mixin forms are skipped in the weave plan and
/// counted as unknown (or refused loudly on a strict host), never silently applied.
#[test]
fn unsupported_mixin_forms_are_skipped_counted_or_refused() {
    let mut sources = mixin_stubs();
    sources.insert(
        "demo/ShadowDuck.java",
        r#"package demo;
public interface ShadowDuck {
    int readShadow();
    void writeShadow(int v);
    int callShadow();
}
"#,
    );
    sources.insert(
        "demo/UnlistedDuck.java",
        r#"package demo;
public interface UnlistedDuck { int ping(); }
"#,
    );
    sources.insert(
        "demo/ClientOnlyDuck.java",
        r#"package demo;
public interface ClientOnlyDuck { int clientPing(); }
"#,
    );
    sources.insert(
        "demo/ChunkDuck.java",
        r#"package demo;
public interface ChunkDuck { int chunkTag(); }
"#,
    );
    sources.insert(
        "demo/ConflictDuck.java",
        r#"package demo;
public interface ConflictDuck { int clash(); }
"#,
    );
    sources.insert(
        "demo/BadSuperDuck.java",
        r#"package demo;
public interface BadSuperDuck { int getVal(); }
"#,
    );
    // 1. Unsupported injectors into game code: @Inject, @Redirect, @Overwrite.
    sources.insert(
        "demo/GameInjectMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Overwrite;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Player.class)
public abstract class GameInjectMixin {
    @Inject(method = "tick()V", at = @At("HEAD"))
    private void onTick(CallbackInfo ci) {}

    @Redirect(method = "tick()V", at = @At(value = "INVOKE", target = "Lnet/minecraft/world/entity/player/Player;jump()V"))
    private void redirectJump(Player self) {}

    @Overwrite
    public void aiStep() {}
}
"#,
    );
    // 2. Accessor/Invoker on an unmodelled game class (LevelChunk).
    sources.insert(
        "demo/ChunkAccessor.java",
        r#"package demo;
import net.minecraft.world.level.chunk.LevelChunk;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;
import org.spongepowered.asm.mixin.gen.Invoker;

@Mixin(LevelChunk.class)
public interface ChunkAccessor {
    @Accessor("secret") int getSecret();
    @Accessor("secret") void setSecret(int v);
    @Invoker("recalc") int callRecalc();
}
"#,
    );
    // 3. Interface-injection mixin targeting an unmodelled game class (LevelChunk).
    sources.insert(
        "demo/ChunkDuckMixin.java",
        r#"package demo;
import net.minecraft.world.level.chunk.LevelChunk;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(LevelChunk.class)
public class ChunkDuckMixin implements ChunkDuck {
    private int tag = 3;
    @Override public int chunkTag() { return this.tag; }
}
"#,
    );
    // 4. Interface-injection mixin with @Shadow field and @Shadow method.
    sources.insert(
        "demo/ShadowMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

@Mixin(Player.class)
public abstract class ShadowMixin implements ShadowDuck {
    @Shadow private int unmodelledField;
    @Shadow public abstract int unmodelledMethod();

    @Override public int readShadow() { return this.unmodelledField; }
    @Override public void writeShadow(int v) { this.unmodelledField = v; }
    @Override public int callShadow() { return this.unmodelledMethod(); }
}
"#,
    );
    // 5. Unlisted mixin (not in demo.mixins.json) and client-only mixin.
    sources.insert(
        "demo/UnlistedMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class UnlistedMixin implements UnlistedDuck {
    @Override public int ping() { return 77; }
}
"#,
    );
    sources.insert(
        "demo/ClientOnlyMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class ClientOnlyMixin implements ClientOnlyDuck {
    @Override public int clientPing() { return 88; }
}
"#,
    );
    // 6. Conflicting interface-injection mixins on the same target and method.
    sources.insert(
        "demo/ConflictAMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class ConflictAMixin implements ConflictDuck {
    @Override public int clash() { return 1; }
}
"#,
    );
    sources.insert(
        "demo/ConflictBMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class ConflictBMixin implements ConflictDuck {
    @Override public int clash() { return 2; }
}
"#,
    );
    // 7. Mixin extending a game superclass with a field initialiser in a non-Object <init>.
    sources.insert(
        "demo/BadSuperMixin.java",
        r#"package demo;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public abstract class BadSuperMixin extends LivingEntity implements BadSuperDuck {
    private int val = 99;
    @Override public int getVal() { return this.val; }
}
"#,
    );
    sources.insert(
        "demo/NegativeProbe.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.chunk.LevelChunk;

public final class NegativeProbe {
    public static int readShadow(Player p) { return ((ShadowDuck) p).readShadow(); }
    public static void writeShadow(Player p, int v) { ((ShadowDuck) p).writeShadow(v); }
    public static int callShadow(Player p) { return ((ShadowDuck) p).callShadow(); }
    public static boolean checkUnlisted(Object p) { return p instanceof UnlistedDuck; }
    public static boolean checkClientOnly(Object p) { return p instanceof ClientOnlyDuck; }
    public static int callConflict(Player p) { return ((ConflictDuck) p).clash(); }
    public static boolean checkBadSuper(Object p) { return p instanceof BadSuperDuck; }
    public static boolean checkWrongTarget(Level l) { return l instanceof ShadowDuck; }
    public static int readChunkAccessor(Object chunk) {
        ChunkAccessor a = (ChunkAccessor) chunk;
        int got = a.getSecret();
        a.setSecret(5);
        return got + a.callRecalc();
    }
    public static Object makeChunk() { return new LevelChunk(); }
}
"#,
    );

    let jar_path = build_jar(
        "negative",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"neg","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["GameInjectMixin","ChunkAccessor","ChunkDuckMixin","ShadowMixin","ConflictAMixin","ConflictBMixin","BadSuperMixin"],"client":["ClientOnlyMixin"]}"#,
            ),
        ],
        COMPILE_ONLY_STUBS,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let weave = jar.plan_load_time_weave(MixinSide::Server);

    // 1. @Inject, @Redirect, @Overwrite on Player are skipped (TargetOutsideJar).
    for name in ["onTick", "redirectJump", "aiStep"] {
        assert!(
            weave.skipped.iter().any(|row| {
                row.callback.owner == "demo/GameInjectMixin"
                    && row.callback.name == name
                    && row.reason == MixinSkipReason::TargetOutsideJar
            }),
            "expected {name} skipped as TargetOutsideJar; skipped: {:#?}",
            weave.skipped
        );
    }

    // 2. Accessor/Invoker on unmodelled LevelChunk is skipped (TargetOutsideJar), NOT applied.
    assert!(
        weave.accessors.is_empty(),
        "external/unmodelled accessors must not be claimed in weave.accessors: {:#?}",
        weave.accessors
    );
    assert!(
        weave.skipped.iter().any(|row| {
            row.callback.owner == "demo/ChunkAccessor"
                && row.reason == MixinSkipReason::TargetOutsideJar
        }),
        "expected ChunkAccessor skipped as TargetOutsideJar: {:#?}",
        weave.skipped
    );

    // 3. Interface mixin on unmodelled LevelChunk is skipped (TargetOutsideJar).
    assert!(
        weave
            .interface_mixins
            .iter()
            .all(|p| p.mixin != "demo/ChunkDuckMixin"),
        "ChunkDuckMixin must not be applied"
    );
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/ChunkDuckMixin"
            && row.reason == MixinSkipReason::TargetOutsideJar
    }));

    // 4. ShadowMixin's @Shadow field is still reported as unwoven.
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/ShadowMixin" && row.reason == MixinSkipReason::UnsupportedFamily
    }));

    // 5. Unlisted and client-only mixins are skipped on Server.
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/UnlistedMixin" && row.reason == MixinSkipReason::NotInConfig
    }));
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/ClientOnlyMixin" && row.reason == MixinSkipReason::OtherSide
    }));

    // 6. Conflicting interface mixins are skipped with AmbiguousTarget.
    assert!(
        weave
            .interface_mixins
            .iter()
            .all(|p| p.mixin != "demo/ConflictAMixin" && p.mixin != "demo/ConflictBMixin"),
        "conflicting mixins must not be planned as applied: {:#?}",
        weave.interface_mixins
    );
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/ConflictAMixin"
            && row.reason == MixinSkipReason::AmbiguousTarget
    }));

    // 7. Non-Object superclass with field initialiser is skipped as UnsupportedFamily.
    assert!(
        weave
            .interface_mixins
            .iter()
            .all(|p| p.mixin != "demo/BadSuperMixin"),
        "BadSuperMixin must not be applied: {:#?}",
        weave.interface_mixins
    );
    assert!(weave.skipped.iter().any(|row| {
        row.callback.owner == "demo/BadSuperMixin"
            && row.reason == MixinSkipReason::UnsupportedFamily
    }));

    // Runtime checks: every unsupported operation is counted on lenient host AND refused on strict host.
    let player = server_player(9, "Neg", 1, 0, 99);
    let level = GameObject::level("minecraft:overworld").value();

    for (method, desc, args) in [
        (
            "readShadow",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            vec![player.clone()],
        ),
        (
            "writeShadow",
            "(Lnet/minecraft/world/entity/player/Player;I)V",
            vec![player.clone(), Value::Int(5)],
        ),
        (
            "callShadow",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            vec![player.clone()],
        ),
        (
            "checkUnlisted",
            "(Ljava/lang/Object;)Z",
            vec![player.clone()],
        ),
        (
            "checkClientOnly",
            "(Ljava/lang/Object;)Z",
            vec![player.clone()],
        ),
        (
            "callConflict",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            vec![player.clone()],
        ),
        (
            "checkBadSuper",
            "(Ljava/lang/Object;)Z",
            vec![player.clone()],
        ),
        (
            "checkWrongTarget",
            "(Lnet/minecraft/world/level/Level;)Z",
            vec![level],
        ),
    ] {
        let mut lenient = StdHost::new().with_environment_type("SERVER");
        let outcome = jar.invoke_method_with_args(
            "demo.NegativeProbe",
            method,
            desc,
            None,
            &args,
            &mut lenient,
        );
        assert!(
            outcome.is_err() || lenient.unknown_host_call_count() >= 1,
            "{method} must refuse loudly or count at least one unknown host call on lenient host: outcome={outcome:?}, unknowns={:?}",
            lenient.unknown_host_calls()
        );

        let mut strict = StdHost::new()
            .with_environment_type("SERVER")
            .with_strict_unknown_host_calls();
        let err = jar.invoke_method_with_args(
            "demo.NegativeProbe",
            method,
            desc,
            None,
            &args,
            &mut strict,
        );
        assert!(
            err.is_err(),
            "{method} must refuse loudly on strict host, got: {err:?}"
        );
    }

    // Shadow write on lenient host must NOT fake-persist a field that a subsequent read returns.
    let mut shadow_host = StdHost::new().with_environment_type("SERVER");
    jar.invoke_method_with_args(
        "demo.NegativeProbe",
        "writeShadow",
        "(Lnet/minecraft/world/entity/player/Player;I)V",
        None,
        &[player.clone(), Value::Int(1234)],
        &mut shadow_host,
    )
    .unwrap();
    let read_back = jar
        .invoke_method_with_args(
            "demo.NegativeProbe",
            "readShadow",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&player),
            &mut shadow_host,
        )
        .unwrap();
    assert_eq!(
        read_back,
        Value::Int(0),
        "unmodelled @Shadow write must not be silently stored and read back"
    );
    assert_eq!(shadow_host.unknown_host_call_count(), 2);

    // Accessor getter + setter + invoker on unmodelled LevelChunk counts every unmodelled op.
    let mut accessor_host = StdHost::new().with_environment_type("SERVER");
    let chunk = jar
        .invoke_method(
            "demo.NegativeProbe",
            "makeChunk",
            "()Ljava/lang/Object;",
            None,
            &mut accessor_host,
        )
        .unwrap();
    let before = accessor_host.unknown_host_call_count();
    jar.invoke_method_with_args(
        "demo.NegativeProbe",
        "readChunkAccessor",
        "(Ljava/lang/Object;)I",
        None,
        std::slice::from_ref(&chunk),
        &mut accessor_host,
    )
    .unwrap();
    assert!(
        accessor_host.unknown_host_call_count() >= before + 3,
        "accessor getter, setter, and invoker on unmodelled class must all be counted: {:?}",
        accessor_host.unknown_host_calls()
    );
}

/// (c) Real-jar test driven by `RUNES_JAR` (skipped with `eprintln!` when unset):
/// asserts `PlayerEntityMixin` is planned and applied to `Player`, and runs the real
/// `RuneCrafter` + `PlayerEntityMixin` + `RuneCrafting.SOUND_DELAY` bytecode offline on
/// separate players across calls, GC, and respawn with zero unknown host calls.
#[test]
fn runes_jar_plans_and_runs_player_entity_mixin_rune_crafter_state() {
    let Ok(path) = std::env::var("RUNES_JAR") else {
        eprintln!(
            "skipping runes_jar_plans_and_runs_player_entity_mixin_rune_crafter_state: RUNES_JAR not set"
        );
        return;
    };
    if path.trim().is_empty() {
        eprintln!(
            "skipping runes_jar_plans_and_runs_player_entity_mixin_rune_crafter_state: RUNES_JAR is empty"
        );
        return;
    }

    let runes_jar = NativeJar::from_path(Path::new(&path)).expect("load RUNES_JAR");

    // 1. Planning: PlayerEntityMixin is planned as an interface-injection mixin on Player,
    //    with zero skipped rows on both plain and game-hook weaves.
    let discovered = runes_jar.mixin_interface_plans();
    let plan = discovered
        .iter()
        .find(|p| p.mixin == "net/runes/mixin/PlayerEntityMixin")
        .expect("PlayerEntityMixin discovered as interface-injection mixin");
    assert_eq!(plan.target, "net/minecraft/world/entity/player/Player");
    assert_eq!(plan.interfaces, vec!["net/runes/crafting/RuneCrafter"]);
    assert!(plan.has_initializer);
    assert!(
        plan.added_fields
            .iter()
            .any(|f| f.name == "lastRuneCrafted" && f.descriptor == "I"),
        "added_fields: {:?}",
        plan.added_fields
    );
    assert!(
        plan.added_methods
            .iter()
            .any(|m| m.name == "getLastRuneCrafted" && m.descriptor == "()I"),
        "added_methods: {:?}",
        plan.added_methods
    );
    assert!(
        plan.added_methods
            .iter()
            .any(|m| m.name == "setLastRuneCrafted" && m.descriptor == "(I)V"),
        "added_methods: {:?}",
        plan.added_methods
    );

    for weave in [
        runes_jar.plan_load_time_weave(MixinSide::Server),
        runes_jar.plan_load_time_weave_with_game_hooks(MixinSide::Server),
    ] {
        assert_eq!(
            weave.interface_mixins.len(),
            1,
            "{:#?}",
            weave.interface_mixins
        );
        assert_eq!(
            weave.interface_mixins[0].mixin,
            "net/runes/mixin/PlayerEntityMixin"
        );
        assert!(
            weave.skipped.is_empty(),
            "Runes PlayerEntityMixin should have zero skipped plans: {:#?}",
            weave.skipped
        );
    }

    // 2. Drive the real Runes RuneCrafter + PlayerEntityMixin bytecode offline via a tiny
    //    harness jar that links RUNES_JAR, proving instanceof, checkcast, 20-tick cooldown,
    //    per-player isolation, GC survival, and respawn reset with zero unknown host calls.
    let mut harness_sources = BTreeMap::new();
    harness_sources.insert(
        "net/minecraft/world/entity/player/Player.java",
        r#"package net.minecraft.world.entity.player;
public class Player {}
"#,
    );
    harness_sources.insert(
        "net/runes/crafting/RuneCrafter.java",
        r#"package net.runes.crafting;
public interface RuneCrafter {
    void setLastRuneCrafted(int time);
    int getLastRuneCrafted();
    default boolean shouldPlayRuneCraftingSound(int age) { return false; }
    default void onPlayedRuneCraftingSound(int age) {}
}
"#,
    );
    harness_sources.insert(
        "harness/RunesProbe.java",
        r#"package harness;
import net.minecraft.world.entity.player.Player;
import net.runes.crafting.RuneCrafter;

public final class RunesProbe {
    public static String checkSoundCooldown(Player player, int t0, int t1, int t2, int t3) {
        boolean isCrafter = player instanceof RuneCrafter;
        RuneCrafter crafter = (RuneCrafter) player;
        int initial = crafter.getLastRuneCrafted();
        boolean play0 = crafter.shouldPlayRuneCraftingSound(t0);
        crafter.onPlayedRuneCraftingSound(t0);
        boolean play1 = crafter.shouldPlayRuneCraftingSound(t1);
        boolean play2 = crafter.shouldPlayRuneCraftingSound(t2);
        boolean play3 = crafter.shouldPlayRuneCraftingSound(t3);
        return isCrafter + "," + initial + "," + play0 + "," + play1 + "," + play2 + "," + play3
            + "," + crafter.getLastRuneCrafted();
    }

    public static int readLast(Player player) {
        return ((RuneCrafter) player).getLastRuneCrafted();
    }

    public static int churn(int n) {
        int acc = 0;
        for (int i = 0; i < n; i++) {
            Object o = new Object();
            acc += (o != null ? 1 : 0);
        }
        return acc;
    }
}
"#,
    );
    let harness_path = build_jar(
        "runes-harness",
        harness_sources,
        &[("fabric.mod.json", r#"{"id":"harness","version":"1.0"}"#)],
        &[
            "net/minecraft/world/entity/player/Player.class",
            "net/runes/crafting/RuneCrafter.class",
        ],
    );
    let harness = NativeJar::from_path(&harness_path).unwrap();

    let mut host = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    host.link_jar("runes", &runes_jar).unwrap();

    let alice = server_player(101, "Alice", 1, 0, 1);
    let bob = server_player(202, "Bob", 1, 0, 2);
    let alice_respawned = server_player(101, "Alice", 1, 1, 1);

    // Real RuneCrafting.SOUND_DELAY is 20 ticks; shouldPlayRuneCraftingSound checks `age > last + 20`.
    let res = harness
        .invoke_method_with_args(
            "harness.RunesProbe",
            "checkSoundCooldown",
            "(Lnet/minecraft/world/entity/player/Player;IIII)Ljava/lang/String;",
            None,
            &[
                alice.clone(),
                Value::Int(100),
                Value::Int(110),
                Value::Int(120),
                Value::Int(121),
            ],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        res,
        Value::Ref(Some("true,0,true,false,false,true,100".into()))
    );

    // Bob is unaffected by Alice's cooldown.
    let bob_last = harness
        .invoke_method_with_args(
            "harness.RunesProbe",
            "readLast",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&bob),
            &mut host,
        )
        .unwrap();
    assert_eq!(bob_last, Value::Int(0));

    // Churn garbage across calls to trigger GC, then re-read Alice's stored tick.
    for _ in 0..3 {
        harness
            .invoke_method_with_args(
                "harness.RunesProbe",
                "churn",
                "(I)I",
                None,
                &[Value::Int(1500)],
                &mut host,
            )
            .unwrap();
    }
    let alice_after_gc = harness
        .invoke_method_with_args(
            "harness.RunesProbe",
            "readLast",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&alice),
            &mut host,
        )
        .unwrap();
    assert_eq!(alice_after_gc, Value::Int(100));

    // Respawned Alice starts fresh.
    let alice_new_life = harness
        .invoke_method_with_args(
            "harness.RunesProbe",
            "readLast",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&alice_respawned),
            &mut host,
        )
        .unwrap();
    assert_eq!(alice_new_life, Value::Int(0));

    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "Runes RuneCrafter path must count zero unknown host calls: {:?}",
        host.unknown_host_calls()
    );
}

/// Regression test: in a config-less jar (no `mixins.json`), a jar-local `@Mixin` class
/// still adds its implemented interface to the jar-local woven class (matching `main`'s
/// `fixture_class_objects` behaviour), while a `@Mixin(Player.class)` interface-injection
/// class targeting a host-modelled game object stays strictly config-gated.
#[test]
fn configless_jar_preserves_jar_local_woven_interface_but_gates_host_game_object() {
    let mut sources = mixin_stubs();
    sources.insert(
        "demo/LocalDuck.java",
        "package demo;\npublic interface LocalDuck {}\n",
    );
    sources.insert(
        "demo/HostDuck.java",
        "package demo;\npublic interface HostDuck { int value(); }\n",
    );
    sources.insert(
        "demo/LocalTarget.java",
        "package demo;\npublic class LocalTarget {}\n",
    );
    sources.insert(
        "demo/LocalTargetMixin.java",
        r#"package demo;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(LocalTarget.class)
public abstract class LocalTargetMixin implements LocalDuck {}
"#,
    );
    sources.insert(
        "demo/UnconfiguredPlayerMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class UnconfiguredPlayerMixin implements HostDuck {
    @Override public int value() { return 55; }
}
"#,
    );
    sources.insert(
        "demo/ConfiglessProbe.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;

public final class ConfiglessProbe {
    public static String checkLocal() {
        LocalTarget target = new LocalTarget();
        boolean assignable = LocalDuck.class.isAssignableFrom(LocalTarget.class);
        boolean isInst = LocalDuck.class.isInstance(target);
        boolean same = LocalDuck.class.cast(target) == target;
        boolean inIfaces = java.util.Arrays.asList(LocalTarget.class.getInterfaces())
            .contains(LocalDuck.class);
        return assignable + "," + isInst + "," + same + "," + inIfaces;
    }

    public static boolean checkHost(Player p) {
        return p instanceof HostDuck;
    }
}
"#,
    );

    let jar_path = build_jar("configless-local-vs-host", sources, &[], COMPILE_ONLY_STUBS);
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // Jar-local woven class still gains LocalDuck without any mixins.json.
    let mut host = StdHost::new().with_environment_type("SERVER");
    let local = jar
        .invoke_method(
            "demo.ConfiglessProbe",
            "checkLocal",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap();
    assert_eq!(local, Value::Ref(Some("true,true,true,true".into())));
    assert_eq!(host.unknown_host_call_count(), 0);

    // Host-modelled Player does NOT receive HostDuck in a config-less jar.
    let player = server_player(77, "NoCfg", 1, 0, 1);
    let mut strict = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    let err = jar.invoke_method_with_args(
        "demo.ConfiglessProbe",
        "checkHost",
        "(Lnet/minecraft/world/entity/player/Player;)Z",
        None,
        std::slice::from_ref(&player),
        &mut strict,
    );
    assert!(
        err.is_err(),
        "config-less jar must not apply interface-injection mixin onto host Player: {err:?}"
    );
}

/// Review #2 with JDK 25 oracle: an injected interface's `default` method MUST NOT
/// override a concrete method the target class hierarchy already declares.
///
/// - On JDK 25, `ConcretePlayer implements Duck` returns `42` for `getId()` and `123`
///   for `getScore()` (not `Duck`'s default `7`), and `99` for `customDefault()`.
/// - On Decaf, calling `((Duck) player).getId()` dispatches to `Entity#getId()I` (`42`,
///   zero unknown calls); calling `((Duck) level).isClientSide()` dispatches to
///   `Level#isClientSide()Z` (`false`, zero unknown calls); calling `((Duck) player).getScore()`
///   (a real vanilla 26.3 `Player` concrete method that the host does not model) is
///   counted on lenient hosts and refused on strict hosts / `GameHookHost`, never
///   falling back to `Duck`'s default `7`; and calling `((Duck) player).customDefault()`
///   (not declared on `Player`/`Entity`/`Object`) runs `Duck`'s default (`99`, zero unknown calls).
#[test]
fn injected_interface_default_does_not_override_concrete_target_method_matches_jdk25() {
    // 1. Run the real JDK 25 oracle to pin JVM semantics (`ConcretePlayer implements Duck`).
    let oracle_root = scratch::Scratch::new("interface-mixin-jdk25-default-oracle");
    let oracle_src = oracle_root.join("src/demo/JdkOracle.java");
    let oracle_classes = oracle_root.join("classes");
    fs::create_dir_all(oracle_src.parent().unwrap()).unwrap();
    fs::create_dir_all(&oracle_classes).unwrap();
    fs::write(
        &oracle_src,
        r#"package demo;
public final class JdkOracle {
    public interface Duck {
        default int getId() { return 7; }
        default int getScore() { return 7; }
        default int customDefault() { return 99; }
    }
    public static class ConcreteEntity {
        public int getId() { return 42; }
    }
    public static class ConcretePlayer extends ConcreteEntity implements Duck {
        public int getScore() { return 123; }
    }
    public static void main(String[] args) {
        Duck d = new ConcretePlayer();
        System.out.print(d.getId() + "," + d.getScore() + "," + d.customDefault());
    }
}
"#,
    )
    .unwrap();
    let compile = Command::new(javac())
        .arg("--release")
        .arg("17")
        .arg("-d")
        .arg(&oracle_classes)
        .arg(&oracle_src)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "javac oracle failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(java_bin())
        .arg("-cp")
        .arg(&oracle_classes)
        .arg("demo.JdkOracle")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "java oracle failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42,123,99");

    // 2. Build the Decaf fixture jar with `@Mixin(Player.class)` and `@Mixin(Level.class)`
    //    implementing `Duck` without overriding `getId`, `getScore`, `isClientSide`, or `customDefault`.
    let mut sources = mixin_stubs();
    sources.insert(
        "net/minecraft/client/player/AbstractClientPlayer.java",
        "package net.minecraft.client.player;\nimport net.minecraft.world.entity.player.Player;\npublic class AbstractClientPlayer extends Player {}\n",
    );
    sources.insert(
        "net/minecraft/client/player/LocalPlayer.java",
        "package net.minecraft.client.player;\npublic class LocalPlayer extends AbstractClientPlayer {}\n",
    );
    sources.insert(
        "net/minecraft/client/Minecraft.java",
        "package net.minecraft.client;\nimport net.minecraft.client.player.LocalPlayer;\npublic class Minecraft { public LocalPlayer player; public static Minecraft getInstance() { return null; } }\n",
    );
    sources.insert(
        "demo/Duck.java",
        r#"package demo;
public interface Duck {
    default int getId() { return 7; }
    default int getScore() { return 7; }
    default boolean isClientSide() { return true; }
    default int customDefault() { return 99; }
}
"#,
    );
    sources.insert(
        "demo/PlayerDuckMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Player.class)
public class PlayerDuckMixin implements Duck {
    @Inject(method = "tick()V", at = @At("HEAD"))
    private void onTick(CallbackInfo ci) {
        Duck self = (Duck) (Object) this;
        DefaultProbe.hookObserved = self.getId() + "," + self.customDefault();
        if (DefaultProbe.callScoreInHook) {
            self.getScore();
        }
    }
}
"#,
    );
    sources.insert(
        "demo/LevelDuckMixin.java",
        r#"package demo;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Level.class)
public class LevelDuckMixin implements Duck {}
"#,
    );
    sources.insert(
        "demo/DefaultProbe.java",
        r#"package demo;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

public final class DefaultProbe {
    public static String hookObserved = "";
    public static boolean callScoreInHook = false;

    public static void setCallScoreInHook(boolean v) {
        callScoreInHook = v;
    }

    public static String probeModelled(Player p, Level l) {
        Duck dp = (Duck) p;
        Duck dl = (Duck) l;
        return dp.getId() + "," + dl.isClientSide() + "," + dp.customDefault();
    }

    public static int probeUnmodelledScore(Player p) {
        return ((Duck) p).getScore();
    }

    public static String probeClientModelled() {
        Duck dp = (Duck) (Object) Minecraft.getInstance().player;
        return dp.getId() + "," + dp.customDefault();
    }

    public static int probeClientUnmodelledScore() {
        return ((Duck) (Object) Minecraft.getInstance().player).getScore();
    }

    public static String readHookObserved() {
        return hookObserved;
    }
}
"#,
    );
    let mut excluded = COMPILE_ONLY_STUBS.to_vec();
    excluded.extend([
        "net/minecraft/client/player/AbstractClientPlayer.class",
        "net/minecraft/client/player/LocalPlayer.class",
        "net/minecraft/client/Minecraft.class",
    ]);
    let jar_path = build_jar(
        "default-vs-concrete",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"defcheck","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["PlayerDuckMixin","LevelDuckMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // 2a. StdHost inside an active server EventFiring with entity_id=42.
    let mut facade = EventFacade::default();
    facade.players.push(FacadePlayer {
        uuid: 1,
        name: "Alice".into(),
        level: Some("minecraft:overworld".into()),
        entity_id: Some(42),
        session: 1,
        life: 0,
    });
    let player = server_player(1, "Alice", 1, 0, 42);
    let level = GameObject::level("minecraft:overworld").value();

    let mut host = StdHost::new().with_environment_type("SERVER");
    host.set_game_event_firing(Some(EventFiring::new(
        GameEvent::ServerTickEnd,
        Arc::new(facade.clone()),
        EventContext::default(),
    )));
    let modelled = jar
        .invoke_method_with_args(
            "demo.DefaultProbe",
            "probeModelled",
            "(Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/level/Level;)Ljava/lang/String;",
            None,
            &[player.clone(), level],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        modelled,
        Value::Ref(Some("42,false,99".into())),
        "target concrete methods must beat Duck's default methods"
    );
    assert_eq!(host.unknown_host_call_count(), 0);

    // - On a server player handle, StdHost's server façade refuses unmodelled Player#getScore()I loudly.
    let firing_err = jar.invoke_method_with_args(
        "demo.DefaultProbe",
        "probeUnmodelledScore",
        "(Lnet/minecraft/world/entity/player/Player;)I",
        None,
        std::slice::from_ref(&player),
        &mut host,
    );
    assert!(
        firing_err.is_err()
            && firing_err
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("net/minecraft/world/entity/player/Player#getScore()I"),
        "expected unmodelled Player#getScore()I refused on server player handle, got: {firing_err:?}"
    );

    // - Inside a client frame on `client/player`, modelled getId() returns 42 (not 7),
    //   customDefault() returns 99, and unmodelled Player#getScore()I refuses loudly with
    //   UnknownHostCall (matching `client_surface::an_unmodelled_member_refuses_only_inside_a_firing`),
    //   never falling back to Duck's default 7!
    let client_snap = Arc::new(ClientSnapshot {
        level: Some(ClientLevel {
            dimension: "minecraft:overworld".into(),
            ..ClientLevel::default()
        }),
        player: Some(ClientPlayer {
            uuid: 1,
            name: "Alice".into(),
            entity_id: 42,
            ..ClientPlayer::default()
        }),
        ..ClientSnapshot::default()
    });
    let mut client_host = StdHost::new().with_environment_type("CLIENT");
    client_host.begin_client_frame(Arc::clone(&client_snap), &[]);
    let client_mod = jar
        .invoke_method(
            "demo.DefaultProbe",
            "probeClientModelled",
            "()Ljava/lang/String;",
            None,
            &mut client_host,
        )
        .unwrap();
    assert_eq!(client_mod, Value::Ref(Some("42,99".into())));
    assert_eq!(client_host.unknown_host_call_count(), 0);
    let client_score_err = jar.invoke_method(
        "demo.DefaultProbe",
        "probeClientUnmodelledScore",
        "()I",
        None,
        &mut client_host,
    );
    assert!(
        client_score_err.is_err()
            && client_score_err
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("net/minecraft/world/entity/player/Player#getScore()I"),
        "unmodelled concrete target method inside client frame must refuse loudly instead of returning default 7: {client_score_err:?}"
    );
    client_host.end_client_frame();

    // 2b. GameHookHost (`run_game_hook_in`) also resolves `getId()` -> `42` and
    //     `customDefault()` -> `99`, and refuses unmodelled `Player#getScore()I` loudly
    //     rather than falling back to Duck's default 7.
    let weave = jar.plan_load_time_weave_with_game_hooks(MixinSide::Server);
    let registrations = jar.register_game_hooks(&weave).unwrap();
    let mut hook_host = StdHost::new().with_environment_type("SERVER");
    let mut hook_state = GameHookState::default();
    let mut ctx = GameHookContext::new();
    let hook_player = ctx.insert(
        GameObject::player(1, "Alice")
            .with_incarnation(1, 0)
            .with_entity_id(42),
    );
    jar.run_game_hook_in(
        &registrations,
        &mut hook_state,
        &mut ctx,
        "java_base_player_tick_head",
        Some(hook_player),
        &[],
        &mut hook_host,
        &ExecutionBudget::within(Duration::from_secs(5)),
    )
    .unwrap();
    let observed = jar
        .invoke_method(
            "demo.DefaultProbe",
            "readHookObserved",
            "()Ljava/lang/String;",
            None,
            &mut hook_host,
        )
        .unwrap();
    assert_eq!(observed, Value::Ref(Some("42,99".into())));
    assert_eq!(hook_host.unknown_host_call_count(), 0);

    // Calling unmodelled concrete Player#getScore()I inside the game hook refuses loudly.
    jar.invoke_method_with_args(
        "demo.DefaultProbe",
        "setCallScoreInHook",
        "(Z)V",
        None,
        &[Value::Int(1)],
        &mut hook_host,
    )
    .unwrap();
    let mut ctx2 = GameHookContext::new();
    let hook_player2 = ctx2.insert(
        GameObject::player(1, "Alice")
            .with_incarnation(1, 0)
            .with_entity_id(42),
    );
    let hook_score_err = jar.run_game_hook_in(
        &registrations,
        &mut hook_state,
        &mut ctx2,
        "java_base_player_tick_head",
        Some(hook_player2),
        &[],
        &mut hook_host,
        &ExecutionBudget::within(Duration::from_secs(5)),
    );
    assert!(
        hook_score_err.is_err()
            && hook_score_err
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("net/minecraft/world/entity/player/Player#getScore()I"),
        "expected unmodelled Player#getScore()I refused in GameHookHost, got: {hook_score_err:?}"
    );
}

/// Review #3: game-hook dispatch (`MixinModel::build`, `host_instance_of`,
/// `redirect_jar_call`) uses the exact same config-, side-, and plugin-gated index as
/// `RuntimeMixinIndex`, so unlisted, client-only, and plugin-declined mixins are inactive
/// inside a server game hook.
#[test]
fn game_hook_dispatch_gates_unlisted_client_only_and_plugin_declined_mixins() {
    let mut sources = mixin_stubs();
    sources.insert(
        "org/objectweb/asm/tree/ClassNode.java",
        "package org.objectweb.asm.tree;\npublic class ClassNode {}\n",
    );
    sources.insert(
        "org/spongepowered/asm/mixin/extensibility/IMixinInfo.java",
        "package org.spongepowered.asm.mixin.extensibility;\npublic interface IMixinInfo {}\n",
    );
    sources.insert(
        "org/spongepowered/asm/mixin/extensibility/IMixinConfigPlugin.java",
        r#"package org.spongepowered.asm.mixin.extensibility;
import java.util.List;
import java.util.Set;
import org.objectweb.asm.tree.ClassNode;
public interface IMixinConfigPlugin {
    void onLoad(String mixinPackage);
    String getRefMapperConfig();
    boolean shouldApplyMixin(String targetClassName, String mixinClassName);
    void acceptTargets(Set<String> myTargets, Set<String> otherTargets);
    List<String> getMixins();
    void preApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo);
    void postApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo);
}
"#,
    );
    sources.insert(
        "demo/DemoPlugin.java",
        r#"package demo;
import java.util.List;
import java.util.Set;
import org.objectweb.asm.tree.ClassNode;
import org.spongepowered.asm.mixin.extensibility.IMixinConfigPlugin;
import org.spongepowered.asm.mixin.extensibility.IMixinInfo;

public class DemoPlugin implements IMixinConfigPlugin {
    @Override public void onLoad(String mixinPackage) {}
    @Override public String getRefMapperConfig() { return null; }
    @Override public boolean shouldApplyMixin(String targetClassName, String mixinClassName) {
        return !mixinClassName.endsWith("PluginDeclinedMixin");
    }
    @Override public void acceptTargets(Set<String> myTargets, Set<String> otherTargets) {}
    @Override public List<String> getMixins() { return null; }
    @Override public void preApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo) {}
    @Override public void postApply(String targetClassName, ClassNode targetClass, String mixinClassName, IMixinInfo mixinInfo) {}
}
"#,
    );
    sources.insert(
        "demo/ActiveDuck.java",
        "package demo;\npublic interface ActiveDuck { int activeVal(); }\n",
    );
    sources.insert(
        "demo/UnlistedDuck.java",
        "package demo;\npublic interface UnlistedDuck { int val(); }\n",
    );
    sources.insert(
        "demo/ClientOnlyDuck.java",
        "package demo;\npublic interface ClientOnlyDuck { int val(); }\n",
    );
    sources.insert(
        "demo/PluginDeclinedDuck.java",
        "package demo;\npublic interface PluginDeclinedDuck { int val(); }\n",
    );
    sources.insert(
        "demo/ServerActiveMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Player.class)
public class ServerActiveMixin implements ActiveDuck {
    @Override public int activeVal() { return 111; }

    @Inject(method = "tick()V", at = @At("HEAD"))
    private void onTick(CallbackInfo ci) {
        Object self = this;
        boolean a = self instanceof ActiveDuck;
        boolean u = self instanceof UnlistedDuck;
        boolean c = self instanceof ClientOnlyDuck;
        boolean p = self instanceof PluginDeclinedDuck;
        int av = ((ActiveDuck) self).activeVal();
        HookGateProbe.report = a + "," + u + "," + c + "," + p + "," + av;
        if (HookGateProbe.callMode == 1) {
            ((UnlistedDuck) self).val();
        } else if (HookGateProbe.callMode == 2) {
            ((ClientOnlyDuck) self).val();
        } else if (HookGateProbe.callMode == 3) {
            ((PluginDeclinedDuck) self).val();
        }
    }
}
"#,
    );
    sources.insert(
        "demo/UnlistedHookMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class UnlistedHookMixin implements UnlistedDuck {
    @Override public int val() { return 222; }
}
"#,
    );
    sources.insert(
        "demo/ClientOnlyHookMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class ClientOnlyHookMixin implements ClientOnlyDuck {
    @Override public int val() { return 333; }
}
"#,
    );
    sources.insert(
        "demo/PluginDeclinedMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public class PluginDeclinedMixin implements PluginDeclinedDuck {
    @Override public int val() { return 444; }
}
"#,
    );
    sources.insert(
        "demo/HookGateProbe.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;

public final class HookGateProbe {
    public static String report = "";
    public static int callMode = 0;
    public static void setCallMode(int mode) { callMode = mode; }
    public static String readReport() { return report; }
    public static boolean checkPluginDeclined(Player p) { return p instanceof PluginDeclinedDuck; }
    public static int readActive(Player p) { return ((ActiveDuck) p).activeVal(); }
}
"#,
    );

    let mut excluded = COMPILE_ONLY_STUBS.to_vec();
    excluded.extend([
        "org/objectweb/asm/tree/ClassNode.class",
        "org/spongepowered/asm/mixin/extensibility/IMixinInfo.class",
        "org/spongepowered/asm/mixin/extensibility/IMixinConfigPlugin.class",
    ]);
    let jar_path = build_jar(
        "game-hook-gating",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"hookgate","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","plugin":"demo.DemoPlugin","mixins":["ServerActiveMixin","PluginDeclinedMixin"],"client":["ClientOnlyHookMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let weave = jar.plan_load_time_weave_with_game_hooks(MixinSide::Server);
    let registrations = jar.register_game_hooks(&weave).unwrap();
    let mut host = StdHost::new().with_environment_type("SERVER");
    let mut state = GameHookState::default();
    let mut ctx = GameHookContext::new();
    let player_val = ctx.insert(
        GameObject::player(12, "HookTester")
            .with_incarnation(1, 0)
            .with_entity_id(5),
    );
    jar.run_game_hook_in(
        &registrations,
        &mut state,
        &mut ctx,
        "java_base_player_tick_head",
        Some(player_val),
        &[],
        &mut host,
        &ExecutionBudget::within(Duration::from_secs(5)),
    )
    .unwrap();
    let report = jar
        .invoke_method(
            "demo.HookGateProbe",
            "readReport",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap();
    assert_eq!(
        report,
        Value::Ref(Some("true,false,false,false,111".into())),
        "unlisted, client-only, and plugin-declined mixins must be inactive in server game hooks"
    );

    // Calling any gated interface method inside the server hook must refuse loudly rather than
    // dispatching to UnlistedHookMixin, ClientOnlyHookMixin, or PluginDeclinedMixin.
    for (mode, expected_iface) in [
        (1, "demo/UnlistedDuck#val()I"),
        (2, "demo/ClientOnlyDuck#val()I"),
        (3, "demo/PluginDeclinedDuck#val()I"),
    ] {
        jar.invoke_method_with_args(
            "demo.HookGateProbe",
            "setCallMode",
            "(I)V",
            None,
            &[Value::Int(mode)],
            &mut host,
        )
        .unwrap();
        let mut ctx2 = GameHookContext::new();
        let p2 = ctx2.insert(
            GameObject::player(12, "HookTester")
                .with_incarnation(1, 0)
                .with_entity_id(5),
        );
        let err = jar.run_game_hook_in(
            &registrations,
            &mut state,
            &mut ctx2,
            "java_base_player_tick_head",
            Some(p2),
            &[],
            &mut host,
            &ExecutionBudget::within(Duration::from_secs(5)),
        );
        assert!(
            err.is_err()
                && err
                    .as_ref()
                    .unwrap_err()
                    .to_string()
                    .contains(expected_iface),
            "expected gated mixin call mode={mode} to refuse loudly naming {expected_iface}, got: {err:?}"
        );
    }

    // Same plugin gate applies on StdHost outside game hooks too.
    let p = server_player(12, "HookTester", 1, 0, 5);
    let active = jar
        .invoke_method_with_args(
            "demo.HookGateProbe",
            "readActive",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&p),
            &mut host,
        )
        .unwrap();
    assert_eq!(active, Value::Int(111));

    let mut strict = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    let declined_err = jar.invoke_method_with_args(
        "demo.HookGateProbe",
        "checkPluginDeclined",
        "(Lnet/minecraft/world/entity/player/Player;)Z",
        None,
        std::slice::from_ref(&p),
        &mut strict,
    );
    assert!(
        declined_err.is_err(),
        "plugin-declined mixin must not pass instanceof on StdHost either: {declined_err:?}"
    );
}

/// Review #4 & #5:
/// - Non-player entity despawn + respawn with the SAME numeric id resets Mixin fields
///   (both via explicit `forget_game_object_prefix` and via non-reusable incarnation keys
///   `game/entity/<id>/<incarnation>`), while `Entity#getId()` still returns the numeric id.
/// - Client `LocalPlayer` (`client/player`) resets its Mixin fields on world unload / rejoin
///   and on respawn (`forget_game_object`), so a new `LocalPlayer` never inherits old state.
#[test]
fn entity_respawn_same_id_and_client_player_rejoin_reset_mixin_fields() {
    let mut sources = mixin_stubs();
    sources.insert(
        "net/minecraft/client/player/AbstractClientPlayer.java",
        "package net.minecraft.client.player;\nimport net.minecraft.world.entity.player.Player;\npublic class AbstractClientPlayer extends Player {}\n",
    );
    sources.insert(
        "net/minecraft/client/player/LocalPlayer.java",
        "package net.minecraft.client.player;\npublic class LocalPlayer extends AbstractClientPlayer {}\n",
    );
    sources.insert(
        "net/minecraft/client/Minecraft.java",
        "package net.minecraft.client;\nimport net.minecraft.client.player.LocalPlayer;\npublic class Minecraft { public LocalPlayer player; public static Minecraft getInstance() { return null; } }\n",
    );
    sources.insert(
        "demo/EntityTag.java",
        "package demo;\npublic interface EntityTag { int getCounter(); void setCounter(int v); int readId(); }\n",
    );
    sources.insert(
        "demo/PlayerTag.java",
        "package demo;\npublic interface PlayerTag { int getTag(); void setTag(int v); }\n",
    );
    sources.insert(
        "demo/EntityTagMixin.java",
        r#"package demo;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;

@Mixin(Entity.class)
public abstract class EntityTagMixin implements EntityTag {
    @Unique private int counter = 10;
    @Shadow public abstract int getId();

    @Override public int getCounter() { return this.counter; }
    @Override public void setCounter(int v) { this.counter = v; }
    @Override public int readId() { return this.getId(); }
}
"#,
    );
    sources.insert(
        "demo/ClientPlayerTagMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;

@Mixin(Player.class)
public class ClientPlayerTagMixin implements PlayerTag {
    @Unique private int tag = 7;
    @Override public int getTag() { return this.tag; }
    @Override public void setTag(int v) { this.tag = v; }
}
"#,
    );
    sources.insert(
        "demo/LifetimeProbe.java",
        r#"package demo;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.Entity;

public final class LifetimeProbe {
    public static int readEntity(Entity e) { return ((EntityTag) e).getCounter(); }
    public static void writeEntity(Entity e, int v) { ((EntityTag) e).setCounter(v); }
    public static int readEntityId(Entity e) { return ((EntityTag) e).readId(); }
    public static int readLocalPlayer() { return ((PlayerTag) (Object) Minecraft.getInstance().player).getTag(); }
    public static void writeLocalPlayer(int v) { ((PlayerTag) (Object) Minecraft.getInstance().player).setTag(v); }
}
"#,
    );
    let mut excluded = COMPILE_ONLY_STUBS.to_vec();
    excluded.extend([
        "net/minecraft/client/player/AbstractClientPlayer.class",
        "net/minecraft/client/player/LocalPlayer.class",
        "net/minecraft/client/Minecraft.class",
    ]);
    let jar_path = build_jar(
        "entity-and-client-lifetime",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"lifetime","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["EntityTagMixin","ClientPlayerTagMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // 1. Server entity id=42 (incarnation 1) -> write 99 -> despawn + same id=42 (incarnation 2) -> resets to 10.
    let mut host = StdHost::new().with_environment_type("SERVER");
    let mut facade1 = EventFacade::default();
    facade1.entity_incarnations.insert(42, 1);
    host.set_game_event_firing(Some(EventFiring::new(
        GameEvent::EntityLoad,
        Arc::new(facade1),
        EventContext {
            entity: Some(FacadeEntity {
                id: 42,
                kind: "minecraft:zombie".into(),
                level: Some("minecraft:overworld".into()),
            }),
            ..EventContext::default()
        },
    )));
    let zombie1 = GameObject::entity(42, "minecraft:zombie", true)
        .with_entity_incarnation(1)
        .value();
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntity",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie1),
            &mut host,
        )
        .unwrap(),
        Value::Int(10)
    );
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntityId",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie1),
            &mut host,
        )
        .unwrap(),
        Value::Int(42)
    );
    jar.invoke_method_with_args(
        "demo.LifetimeProbe",
        "writeEntity",
        "(Lnet/minecraft/world/entity/Entity;I)V",
        None,
        &[zombie1.clone(), Value::Int(99)],
        &mut host,
    )
    .unwrap();
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntity",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie1),
            &mut host,
        )
        .unwrap(),
        Value::Int(99)
    );

    // Despawn entity 42 (forget prefix) and spawn a fresh entity reusing id=42 (incarnation 2).
    host.forget_game_object_prefix("game/entity/42");
    let mut facade2 = EventFacade::default();
    facade2.entity_incarnations.insert(42, 2);
    host.set_game_event_firing(Some(EventFiring::new(
        GameEvent::EntityLoad,
        Arc::new(facade2),
        EventContext {
            entity: Some(FacadeEntity {
                id: 42,
                kind: "minecraft:zombie".into(),
                level: Some("minecraft:overworld".into()),
            }),
            ..EventContext::default()
        },
    )));
    let zombie2 = GameObject::entity(42, "minecraft:zombie", true)
        .with_entity_incarnation(2)
        .value();
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntity",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie2),
            &mut host,
        )
        .unwrap(),
        Value::Int(10),
        "respawned entity with reused id=42 must start with fresh default fields"
    );
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntityId",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie2),
            &mut host,
        )
        .unwrap(),
        Value::Int(42),
        "incarnated entity marker must still parse numeric Entity#getId()"
    );
    // Even without calling forget, a fresh incarnation 3 on id=42 cannot alias incarnation 2.
    jar.invoke_method_with_args(
        "demo.LifetimeProbe",
        "writeEntity",
        "(Lnet/minecraft/world/entity/Entity;I)V",
        None,
        &[zombie2, Value::Int(123)],
        &mut host,
    )
    .unwrap();
    let zombie3 = GameObject::entity(42, "minecraft:zombie", true)
        .with_entity_incarnation(3)
        .value();
    assert_eq!(
        jar.invoke_method_with_args(
            "demo.LifetimeProbe",
            "readEntity",
            "(Lnet/minecraft/world/entity/Entity;)I",
            None,
            std::slice::from_ref(&zombie3),
            &mut host,
        )
        .unwrap(),
        Value::Int(10)
    );
    assert_eq!(host.unknown_host_call_count(), 0);

    // 2. Client LocalPlayer (`client/player`) resets across level unload / rejoin and respawn.
    let mut client = StdHost::new().with_environment_type("CLIENT");
    let frame_with = |uuid: u128, entity_id: i32| {
        Arc::new(ClientSnapshot {
            level: Some(ClientLevel {
                dimension: "minecraft:overworld".into(),
                ..ClientLevel::default()
            }),
            player: Some(ClientPlayer {
                uuid,
                name: "ClientHero".into(),
                entity_id,
                ..ClientPlayer::default()
            }),
            ..ClientSnapshot::default()
        })
    };
    let empty_frame = Arc::new(ClientSnapshot::default());

    client.begin_client_frame(frame_with(7, 100), &[]);
    assert_eq!(
        jar.invoke_method(
            "demo.LifetimeProbe",
            "readLocalPlayer",
            "()I",
            None,
            &mut client
        )
        .unwrap(),
        Value::Int(7)
    );
    jar.invoke_method_with_args(
        "demo.LifetimeProbe",
        "writeLocalPlayer",
        "(I)V",
        None,
        &[Value::Int(555)],
        &mut client,
    )
    .unwrap();
    client.end_client_frame();

    // Same player in next frame keeps 555.
    client.begin_client_frame(frame_with(7, 100), &[]);
    assert_eq!(
        jar.invoke_method(
            "demo.LifetimeProbe",
            "readLocalPlayer",
            "()I",
            None,
            &mut client
        )
        .unwrap(),
        Value::Int(555)
    );
    client.end_client_frame();

    // Unload level (title screen / disconnect) then rejoin with same uuid & entity_id -> resets to 7!
    client.begin_client_frame(empty_frame, &[]);
    client.end_client_frame();
    client.begin_client_frame(frame_with(7, 100), &[]);
    assert_eq!(
        jar.invoke_method(
            "demo.LifetimeProbe",
            "readLocalPlayer",
            "()I",
            None,
            &mut client
        )
        .unwrap(),
        Value::Int(7),
        "rejoined LocalPlayer must start with default Mixin fields"
    );
    jar.invoke_method_with_args(
        "demo.LifetimeProbe",
        "writeLocalPlayer",
        "(I)V",
        None,
        &[Value::Int(666)],
        &mut client,
    )
    .unwrap();
    client.end_client_frame();

    // New entity_id on rejoin also resets to 7.
    client.begin_client_frame(frame_with(7, 101), &[]);
    assert_eq!(
        jar.invoke_method(
            "demo.LifetimeProbe",
            "readLocalPlayer",
            "()I",
            None,
            &mut client
        )
        .unwrap(),
        Value::Int(7),
        "LocalPlayer with new entity_id must start with default Mixin fields"
    );
    jar.invoke_method_with_args(
        "demo.LifetimeProbe",
        "writeLocalPlayer",
        "(I)V",
        None,
        &[Value::Int(777)],
        &mut client,
    )
    .unwrap();
    client.end_client_frame();

    // Explicit forget (e.g. client_player_clone respawn keeping same entity_id) resets to 7.
    client.forget_game_object("client/player");
    client.begin_client_frame(frame_with(7, 101), &[]);
    assert_eq!(
        jar.invoke_method(
            "demo.LifetimeProbe",
            "readLocalPlayer",
            "()I",
            None,
            &mut client
        )
        .unwrap(),
        Value::Int(7),
        "respawned LocalPlayer after forget_game_object must start with default Mixin fields"
    );
    client.end_client_frame();
    assert_eq!(client.unknown_host_call_count(), 0);
}

/// Review #6: multi-target `@Shadow` resolves against the actual receiver's target class
/// (not `targets.first()`) and counts/refuses when multiple targets match `this`.
#[test]
fn multi_target_shadow_resolves_per_receiver_and_refuses_ambiguous() {
    let mut sources = mixin_stubs();
    sources.insert(
        "demo/MultiDuck.java",
        "package demo;\npublic interface MultiDuck { boolean checkClient(); int checkId(); }\n",
    );
    sources.insert(
        "demo/AmbiguousDuck.java",
        "package demo;\npublic interface AmbiguousDuck { int ambiguousId(); }\n",
    );
    sources.insert(
        "demo/MultiShadowMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin({Player.class, Level.class})
public abstract class MultiShadowMixin implements MultiDuck {
    @Shadow public abstract boolean isClientSide();
    @Shadow public abstract int getId();

    @Override public boolean checkClient() { return this.isClientSide(); }
    @Override public int checkId() { return this.getId(); }

    @Inject(method = "tick()V", at = @At("HEAD"))
    private void onMultiTick(CallbackInfo ci) {
        if (ShadowProbe.hookMode == 1) {
            ShadowProbe.hookResult = this.getId();
        }
    }
}
"#,
    );
    sources.insert(
        "demo/AmbiguousShadowMixin.java",
        r#"package demo;
import java.util.function.IntSupplier;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin({ServerPlayer.class, Player.class})
public abstract class AmbiguousShadowMixin implements AmbiguousDuck {
    @Shadow public abstract int getId();
    @Override public int ambiguousId() { return this.getId(); }

    @Inject(method = "tick()V", at = @At("HEAD"))
    private void onAmbiguousTick(CallbackInfo ci) {
        if (ShadowProbe.hookMode == 2) {
            ShadowProbe.hookResult = this.getId();
        } else if (ShadowProbe.hookMode == 3) {
            IntSupplier ref = this::getId;
            ShadowProbe.hookResult = ref.getAsInt();
        }
    }
}
"#,
    );
    sources.insert(
        "demo/ShadowProbe.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

public final class ShadowProbe {
    public static int hookMode = 0;
    public static int hookResult = -1;
    public static void configureHook(int mode) { hookMode = mode; hookResult = -1; }
    public static int readHookResult() { return hookResult; }
    public static String checkBoth(Player p, Level l) {
        return ((MultiDuck) p).checkId() + "," + ((MultiDuck) l).checkClient();
    }
    public static int checkAmbiguous(Player p) {
        return ((AmbiguousDuck) p).ambiguousId();
    }
}
"#,
    );

    let jar_path = build_jar(
        "multi-target-shadow",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"multishadow","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["MultiShadowMixin","AmbiguousShadowMixin"]}"#,
            ),
        ],
        COMPILE_ONLY_STUBS,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    let mut facade = EventFacade::default();
    facade.players.push(FacadePlayer {
        uuid: 1,
        name: "Multi".into(),
        level: Some("minecraft:overworld".into()),
        entity_id: Some(42),
        session: 1,
        life: 0,
    });
    let player = server_player(1, "Multi", 1, 0, 42);
    let level = GameObject::level("minecraft:overworld").value();

    let mut host = StdHost::new().with_environment_type("SERVER");
    host.set_game_event_firing(Some(EventFiring::new(
        GameEvent::ServerTickEnd,
        Arc::new(facade.clone()),
        EventContext::default(),
    )));
    let both = jar
        .invoke_method_with_args(
            "demo.ShadowProbe",
            "checkBoth",
            "(Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/level/Level;)Ljava/lang/String;",
            None,
            &[player.clone(), level],
            &mut host,
        )
        .unwrap();
    assert_eq!(both, Value::Ref(Some("42,false".into())));
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "multi-target @Shadow must resolve per receiver (Player -> Player#getId, Level -> Level#isClientSide): {:?}",
        host.unknown_host_calls()
    );

    // Ambiguous multi-target `@Mixin({ServerPlayer.class, Player.class})` on a ServerPlayer
    // must count on lenient host and refuse loudly on strict host instead of guessing targets.first().
    let ambig = jar
        .invoke_method_with_args(
            "demo.ShadowProbe",
            "checkAmbiguous",
            "(Lnet/minecraft/world/entity/player/Player;)I",
            None,
            std::slice::from_ref(&player),
            &mut host,
        )
        .unwrap();
    assert_eq!(ambig, Value::Int(0));
    assert_eq!(
        host.unknown_host_call_count(),
        1,
        "ambiguous multi-target @Shadow must be counted: {:?}",
        host.unknown_host_calls()
    );

    let mut strict = StdHost::new()
        .with_environment_type("SERVER")
        .with_strict_unknown_host_calls();
    strict.set_game_event_firing(Some(EventFiring::new(
        GameEvent::ServerTickEnd,
        Arc::new(facade),
        EventContext::default(),
    )));
    let err = jar.invoke_method_with_args(
        "demo.ShadowProbe",
        "checkAmbiguous",
        "(Lnet/minecraft/world/entity/player/Player;)I",
        None,
        std::slice::from_ref(&player),
        &mut strict,
    );
    assert!(
        err.is_err(),
        "ambiguous multi-target @Shadow must refuse loudly on strict host: {err:?}"
    );

    // Round 2 P1 #1: verify the same resolution and ambiguity counting/refusal through
    // `run_game_hook_in` (both direct `this.getId()` and method-reference `this::getId`,
    // which exercises `GameHookHost::redirect_jar_call` directly).
    let weave = jar.plan_load_time_weave_with_game_hooks(MixinSide::Server);
    let registrations = jar.register_game_hooks(&weave).unwrap();

    // Mode 1: non-ambiguous `@Mixin({Player.class, Level.class})` inside `run_game_hook_in` -> 42.
    let mut hook_host = StdHost::new().with_environment_type("SERVER");
    let mut hook_state = GameHookState::default();
    jar.invoke_method_with_args(
        "demo.ShadowProbe",
        "configureHook",
        "(I)V",
        None,
        &[Value::Int(1)],
        &mut hook_host,
    )
    .unwrap();
    let mut ctx1 = GameHookContext::new();
    let hp1 = ctx1.insert(
        GameObject::player(1, "Multi")
            .with_incarnation(1, 0)
            .with_entity_id(42),
    );
    jar.run_game_hook_in(
        &registrations,
        &mut hook_state,
        &mut ctx1,
        "java_base_player_tick_head",
        Some(hp1),
        &[],
        &mut hook_host,
        &ExecutionBudget::within(Duration::from_secs(5)),
    )
    .unwrap();
    assert_eq!(
        jar.invoke_method(
            "demo.ShadowProbe",
            "readHookResult",
            "()I",
            None,
            &mut hook_host,
        )
        .unwrap(),
        Value::Int(42)
    );
    assert_eq!(hook_host.unknown_host_call_count(), 0);

    // Modes 2 & 3: ambiguous `@Mixin({ServerPlayer.class, Player.class})` inside `run_game_hook_in`
    // must count on lenient host (returning 0, never 42) and refuse loudly on strict host.
    for mode in [2, 3] {
        let mut lenient = StdHost::new().with_environment_type("SERVER");
        let mut state_l = GameHookState::default();
        jar.invoke_method_with_args(
            "demo.ShadowProbe",
            "configureHook",
            "(I)V",
            None,
            &[Value::Int(mode)],
            &mut lenient,
        )
        .unwrap();
        let mut ctx_l = GameHookContext::new();
        let hp_l = ctx_l.insert(
            GameObject::player(1, "Multi")
                .with_incarnation(1, 0)
                .with_entity_id(42),
        );
        jar.run_game_hook_in(
            &registrations,
            &mut state_l,
            &mut ctx_l,
            "java_base_player_tick_head",
            Some(hp_l),
            &[],
            &mut lenient,
            &ExecutionBudget::within(Duration::from_secs(5)),
        )
        .unwrap();
        let got = jar
            .invoke_method(
                "demo.ShadowProbe",
                "readHookResult",
                "()I",
                None,
                &mut lenient,
            )
            .unwrap();
        assert_eq!(
            got,
            Value::Int(0),
            "mode={mode}: ambiguous @Shadow in run_game_hook_in must not answer 42 by name"
        );
        assert_eq!(
            lenient.unknown_host_call_count(),
            1,
            "mode={mode}: ambiguous @Shadow in run_game_hook_in must be counted: {:?}",
            lenient.unknown_host_calls()
        );

        let mut strict_hook = StdHost::new()
            .with_environment_type("SERVER")
            .with_strict_unknown_host_calls();
        let mut state_s = GameHookState::default();
        jar.invoke_method_with_args(
            "demo.ShadowProbe",
            "configureHook",
            "(I)V",
            None,
            &[Value::Int(mode)],
            &mut strict_hook,
        )
        .unwrap();
        let mut ctx_s = GameHookContext::new();
        let hp_s = ctx_s.insert(
            GameObject::player(1, "Multi")
                .with_incarnation(1, 0)
                .with_entity_id(42),
        );
        let hook_err = jar.run_game_hook_in(
            &registrations,
            &mut state_s,
            &mut ctx_s,
            "java_base_player_tick_head",
            Some(hp_s),
            &[],
            &mut strict_hook,
            &ExecutionBudget::within(Duration::from_secs(5)),
        );
        assert!(
            hook_err.is_err(),
            "mode={mode}: ambiguous @Shadow in run_game_hook_in must refuse on strict host: {hook_err:?}"
        );
    }
}

/// Round 2 P1 #2 with JDK 25 `javac` + JVM probes:
/// - `vanilla_host_methods.tsv` follows target hierarchies into JDK 25 (`Container extends
///   java/lang/Iterable`), so `Inventory` inherits `Iterable.forEach` and `Iterable.spliterator`.
/// - Adding an unrelated interface `default` for `forEach`/`spliterator` on `Inventory` (or
///   two unrelated interface defaults on `Player`) throws `IncompatibleClassChangeError`
///   ("Conflicting default methods"), matching JDK 25's "inherits unrelated defaults" rule,
///   whereas a strict subinterface of `Iterable` overrides `Iterable.forEach` cleanly.
#[test]
fn jdk_interface_defaults_and_unrelated_default_conflicts_match_jdk25() {
    // 1. Live JDK 25 `javac` + runtime probes pinning the JVM semantics.
    let oracle_root = scratch::Scratch::new("interface-mixin-jdk25-unrelated-defaults");
    let compile_probe = |subdir: &str, files: &[(&str, &str)]| -> std::process::Output {
        let dir = oracle_root.join(subdir);
        let out = dir.join("classes");
        fs::create_dir_all(&out).unwrap();
        let mut src_paths = Vec::new();
        for (rel, body) in files {
            let p = dir.join("src").join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, body).unwrap();
            src_paths.push(p);
        }
        Command::new(javac())
            .arg("-d")
            .arg(&out)
            .args(&src_paths)
            .output()
            .expect("run javac")
    };

    // 1a. Unrelated default `forEach` on `Iterable` subclass -> javac rejects ("unrelated defaults").
    let out_foreach = compile_probe(
        "probe-foreach",
        &[
            (
                "Duck.java",
                "import java.util.function.Consumer; public interface Duck { default void forEach(Consumer<? super String> c) { c.accept(\"duck\"); } }\n",
            ),
            (
                "Inv.java",
                "import java.util.*; public class Inv implements Iterable<String>, Duck { public Iterator<String> iterator() { return List.of(\"inv\").iterator(); } }\n",
            ),
        ],
    );
    let stderr_foreach = String::from_utf8_lossy(&out_foreach.stderr);
    assert!(
        !out_foreach.status.success() && stderr_foreach.contains("unrelated defaults"),
        "expected JDK 25 javac to reject unrelated default forEach:\n{stderr_foreach}"
    );

    // 1b. Unrelated default `spliterator` on `Iterable` subclass -> javac rejects ("unrelated defaults").
    let out_split = compile_probe(
        "probe-spliterator",
        &[
            (
                "Duck.java",
                "import java.util.Spliterator; public interface Duck { default Spliterator<String> spliterator() { return null; } }\n",
            ),
            (
                "Inv.java",
                "import java.util.*; public class Inv implements Iterable<String>, Duck { public Iterator<String> iterator() { return List.of(\"inv\").iterator(); } }\n",
            ),
        ],
    );
    let stderr_split = String::from_utf8_lossy(&out_split.stderr);
    assert!(
        !out_split.status.success() && stderr_split.contains("unrelated defaults"),
        "expected JDK 25 javac to reject unrelated default spliterator:\n{stderr_split}"
    );

    // 1c. Strict subinterface of `Iterable` overriding `default forEach` -> javac accepts & runs subinterface.
    let out_sub = compile_probe(
        "probe-sub",
        &[
            (
                "SubDuck.java",
                "import java.util.function.Consumer; public interface SubDuck extends Iterable<String> { default void forEach(Consumer<? super String> c) { c.accept(\"sub\"); } }\n",
            ),
            (
                "Inv.java",
                "import java.util.*; public class Inv implements Iterable<String>, SubDuck { public Iterator<String> iterator() { return List.of(\"inv\").iterator(); } }\n",
            ),
            (
                "Main.java",
                "public class Main { public static void main(String[] a) { var sb = new StringBuilder(); ((SubDuck) new Inv()).forEach(sb::append); System.out.print(sb); } }\n",
            ),
        ],
    );
    assert!(
        out_sub.status.success(),
        "expected JDK 25 javac to accept strict subinterface default:\n{}",
        String::from_utf8_lossy(&out_sub.stderr)
    );
    let run_sub = Command::new(java_bin())
        .arg("-cp")
        .arg(oracle_root.join("probe-sub/classes"))
        .arg("Main")
        .output()
        .expect("run java");
    assert!(run_sub.status.success());
    assert_eq!(String::from_utf8_lossy(&run_sub.stdout), "sub");

    // 1d. Two unrelated interface defaults `PingA` & `PingB` -> javac rejects ("unrelated defaults").
    let out_pair = compile_probe(
        "probe-pair",
        &[
            (
                "PingA.java",
                "public interface PingA { default int ping() { return 1; } }\n",
            ),
            (
                "PingB.java",
                "public interface PingB { default int ping() { return 2; } }\n",
            ),
            ("C.java", "public class C implements PingA, PingB {}\n"),
        ],
    );
    let stderr_pair = String::from_utf8_lossy(&out_pair.stderr);
    assert!(
        !out_pair.status.success() && stderr_pair.contains("unrelated defaults"),
        "expected JDK 25 javac to reject unrelated defaults PingA/PingB:\n{stderr_pair}"
    );

    // 1e. Separate-compilation runtime probe (exact Mixin bytecode weaving): throws IncompatibleClassChangeError.
    let rt_dir = oracle_root.join("probe-runtime");
    let rt_classes = rt_dir.join("classes");
    fs::create_dir_all(&rt_classes).unwrap();
    let rt_duck = rt_dir.join("Duck.java");
    let rt_inv = rt_dir.join("Inv.java");
    let rt_main = rt_dir.join("Main.java");
    fs::write(&rt_duck, "public interface Duck {}\n").unwrap();
    fs::write(
        &rt_inv,
        "import java.util.*; public class Inv implements Iterable<String>, Duck { public Iterator<String> iterator() { return List.of(\"inv\").iterator(); } }\n",
    )
    .unwrap();
    assert!(
        Command::new(javac())
            .arg("-d")
            .arg(&rt_classes)
            .arg(&rt_duck)
            .arg(&rt_inv)
            .status()
            .unwrap()
            .success()
    );
    fs::write(
        &rt_duck,
        "import java.util.function.Consumer; public interface Duck { default void forEach(Consumer<? super String> c) { c.accept(\"duck\"); } }\n",
    )
    .unwrap();
    fs::write(
        &rt_main,
        "public class Main { public static void main(String[] a) { ((Duck) new Inv()).forEach(x -> {}); } }\n",
    )
    .unwrap();
    assert!(
        Command::new(javac())
            .arg("-cp")
            .arg(&rt_classes)
            .arg("-d")
            .arg(&rt_classes)
            .arg(&rt_duck)
            .arg(&rt_main)
            .status()
            .unwrap()
            .success()
    );
    let run_rt = Command::new(java_bin())
        .arg("-cp")
        .arg(&rt_classes)
        .arg("Main")
        .output()
        .expect("run java");
    let rt_err = String::from_utf8_lossy(&run_rt.stderr);
    assert!(
        !run_rt.status.success()
            && rt_err.contains("java.lang.IncompatibleClassChangeError")
            && rt_err.contains("Conflicting default methods"),
        "expected JDK 25 runtime IncompatibleClassChangeError:\n{rt_err}"
    );

    // 2. Build Decaf test jar exercising Inventory (`Container extends Iterable`) + unrelated
    //    default `forEach`/`spliterator`, strict subinterface default, and two unrelated Player defaults.
    let mut sources = mixin_stubs();
    sources.insert(
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\npublic class ItemStack {}\n",
    );
    sources.insert(
        "net/minecraft/world/Container.java",
        "package net.minecraft.world;\nimport net.minecraft.world.item.ItemStack;\npublic interface Container extends Iterable<ItemStack> {}\n",
    );
    sources.insert(
        "net/minecraft/world/entity/player/Inventory.java",
        "package net.minecraft.world.entity.player;\nimport java.util.Iterator;\nimport net.minecraft.world.Container;\nimport net.minecraft.world.item.ItemStack;\npublic class Inventory implements Container { public Iterator<ItemStack> iterator() { return null; } }\n",
    );
    sources.insert(
        "net/minecraft/client/player/AbstractClientPlayer.java",
        "package net.minecraft.client.player;\nimport net.minecraft.world.entity.player.Player;\npublic class AbstractClientPlayer extends Player {}\n",
    );
    sources.insert(
        "net/minecraft/client/player/LocalPlayer.java",
        "package net.minecraft.client.player;\nimport net.minecraft.world.entity.player.Inventory;\npublic class LocalPlayer extends AbstractClientPlayer { public Inventory getInventory() { return null; } }\n",
    );
    sources.insert(
        "net/minecraft/client/Minecraft.java",
        "package net.minecraft.client;\nimport net.minecraft.client.player.LocalPlayer;\npublic class Minecraft { public LocalPlayer player; public static Minecraft getInstance() { return null; } }\n",
    );
    sources.insert(
        "demo/ForEachDuck.java",
        r#"package demo;
import java.util.Spliterator;
import java.util.function.Consumer;
import net.minecraft.world.item.ItemStack;

public interface ForEachDuck {
    default void forEach(Consumer<? super ItemStack> c) {
        JdkDefaultProbe.ranUnrelatedDuck = true;
    }
    default Spliterator<ItemStack> spliterator() {
        JdkDefaultProbe.ranUnrelatedDuck = true;
        return null;
    }
}
"#,
    );
    sources.insert(
        "demo/SubIterableDuck.java",
        r#"package demo;
import java.util.function.Consumer;
import net.minecraft.world.item.ItemStack;

public interface SubIterableDuck extends Iterable<ItemStack> {
    default void forEach(Consumer<? super ItemStack> c) {
        JdkDefaultProbe.subResult = "sub";
    }
}
"#,
    );
    sources.insert(
        "demo/InvConflictMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Inventory;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Inventory.class)
public abstract class InvConflictMixin implements ForEachDuck {}
"#,
    );
    sources.insert(
        "demo/InvSubMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Inventory;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Inventory.class)
public abstract class InvSubMixin implements SubIterableDuck {}
"#,
    );
    sources.insert(
        "demo/PingA.java",
        "package demo;\npublic interface PingA { default int ping() { return 1; } }\n",
    );
    sources.insert(
        "demo/PingB.java",
        "package demo;\npublic interface PingB { default int ping() { return 2; } }\n",
    );
    sources.insert(
        "demo/PingAMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public abstract class PingAMixin implements PingA {}
"#,
    );
    sources.insert(
        "demo/PingBMixin.java",
        r#"package demo;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(Player.class)
public abstract class PingBMixin implements PingB {}
"#,
    );
    sources.insert(
        "demo/JdkDefaultProbe.java",
        r#"package demo;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.player.Player;

public final class JdkDefaultProbe {
    public static boolean ranUnrelatedDuck = false;
    public static String subResult = "";

    public static void callInvForEach() {
        Object inv = Minecraft.getInstance().player.getInventory();
        ((ForEachDuck) inv).forEach(x -> {});
    }
    public static Object callInvSpliterator() {
        Object inv = Minecraft.getInstance().player.getInventory();
        return ((ForEachDuck) inv).spliterator();
    }
    public static String callInvSubForEach() {
        subResult = "";
        Object inv = Minecraft.getInstance().player.getInventory();
        ((SubIterableDuck) inv).forEach(x -> {});
        return subResult;
    }
    public static boolean didRunUnrelatedDuck() {
        return ranUnrelatedDuck;
    }
    public static int callUnrelatedPlayerDefaults(Player p) {
        return ((PingA) p).ping();
    }
}
"#,
    );

    let mut excluded = COMPILE_ONLY_STUBS.to_vec();
    excluded.extend([
        "net/minecraft/world/item/ItemStack.class",
        "net/minecraft/world/Container.class",
        "net/minecraft/world/entity/player/Inventory.class",
        "net/minecraft/client/player/AbstractClientPlayer.class",
        "net/minecraft/client/player/LocalPlayer.class",
        "net/minecraft/client/Minecraft.class",
    ]);

    // Jar 1: unrelated `ForEachDuck` on Inventory + unrelated `PingA`/`PingB` on Player.
    let jar_conflict_path = build_jar(
        "jdk-default-conflict",
        sources.clone(),
        &[
            (
                "fabric.mod.json",
                r#"{"id":"jdkconflict","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["InvConflictMixin","PingAMixin","PingBMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar_conflict = NativeJar::from_path(&jar_conflict_path).unwrap();

    let mut client = StdHost::new().with_environment_type("CLIENT");
    let frame = Arc::new(ClientSnapshot {
        level: Some(ClientLevel {
            dimension: "minecraft:overworld".into(),
            ..ClientLevel::default()
        }),
        player: Some(ClientPlayer {
            uuid: 7,
            name: "InvHero".into(),
            entity_id: 100,
            ..ClientPlayer::default()
        }),
        ..ClientSnapshot::default()
    });
    client.begin_client_frame(Arc::clone(&frame), &[]);

    // 2a. `ForEachDuck.forEach` conflicts with `java/lang/Iterable.forEach` -> IncompatibleClassChangeError.
    let foreach_err = jar_conflict.invoke_method(
        "demo.JdkDefaultProbe",
        "callInvForEach",
        "()V",
        None,
        &mut client,
    );
    assert!(
        matches!(
            &foreach_err,
            Err(NativeError::UncaughtException { class, message: Some(msg), .. })
                if class == "java/lang/IncompatibleClassChangeError"
                    && msg.contains("java/lang/Iterable.forEach")
                    && msg.contains("demo/ForEachDuck.forEach")
        ),
        "expected IncompatibleClassChangeError between Iterable.forEach and ForEachDuck.forEach, got: {foreach_err:?}"
    );

    // 2b. `ForEachDuck.spliterator` conflicts with `java/lang/Iterable.spliterator` -> IncompatibleClassChangeError.
    let split_err = jar_conflict.invoke_method(
        "demo.JdkDefaultProbe",
        "callInvSpliterator",
        "()Ljava/lang/Object;",
        None,
        &mut client,
    );
    assert!(
        matches!(
            &split_err,
            Err(NativeError::UncaughtException { class, message: Some(msg), .. })
                if class == "java/lang/IncompatibleClassChangeError"
                    && msg.contains("java/lang/Iterable.spliterator")
                    && msg.contains("demo/ForEachDuck.spliterator")
        ),
        "expected IncompatibleClassChangeError between Iterable.spliterator and ForEachDuck.spliterator, got: {split_err:?}"
    );
    assert_eq!(
        jar_conflict
            .invoke_method(
                "demo.JdkDefaultProbe",
                "didRunUnrelatedDuck",
                "()Z",
                None,
                &mut client,
            )
            .unwrap(),
        Value::Int(0),
        "unrelated Duck defaults must never have executed"
    );
    client.end_client_frame();

    // 2c. Two unrelated injected defaults (`PingA.ping` & `PingB.ping`) on `Player` -> IncompatibleClassChangeError.
    let mut server = StdHost::new().with_environment_type("SERVER");
    let player = server_player(1, "ConflictHero", 1, 0, 42);
    let pair_err = jar_conflict.invoke_method_with_args(
        "demo.JdkDefaultProbe",
        "callUnrelatedPlayerDefaults",
        "(Lnet/minecraft/world/entity/player/Player;)I",
        None,
        std::slice::from_ref(&player),
        &mut server,
    );
    assert!(
        matches!(
            &pair_err,
            Err(NativeError::UncaughtException { class, message: Some(msg), .. })
                if class == "java/lang/IncompatibleClassChangeError"
                    && msg.contains("demo/PingA.ping")
                    && msg.contains("demo/PingB.ping")
        ),
        "expected IncompatibleClassChangeError between PingA.ping and PingB.ping, got: {pair_err:?}"
    );

    // 3. Jar 2: strict subinterface `SubIterableDuck extends Iterable` overrides `Iterable.forEach` cleanly!
    let jar_sub_path = build_jar(
        "jdk-default-subinterface",
        sources,
        &[
            (
                "fabric.mod.json",
                r#"{"id":"jdksub","version":"1.0","mixins":["demo.mixins.json"]}"#,
            ),
            (
                "demo.mixins.json",
                r#"{"required":true,"package":"demo","mixins":["InvSubMixin"]}"#,
            ),
        ],
        &excluded,
    );
    let jar_sub = NativeJar::from_path(&jar_sub_path).unwrap();
    let mut sub_client = StdHost::new().with_environment_type("CLIENT");
    sub_client.begin_client_frame(frame, &[]);
    let sub_out = jar_sub
        .invoke_method(
            "demo.JdkDefaultProbe",
            "callInvSubForEach",
            "()Ljava/lang/String;",
            None,
            &mut sub_client,
        )
        .unwrap();
    sub_client.end_client_frame();
    assert_eq!(sub_out, Value::Ref(Some("sub".into())));
    assert_eq!(sub_client.unknown_host_call_count(), 0);
}

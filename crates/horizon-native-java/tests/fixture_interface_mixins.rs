//! Interface-injection `@Mixin` fixtures (`@Mixin(Target.class) implements I` +
//! added fields/methods on host-modelled game objects), negative fixtures for
//! unsupported mixin forms, and an opt-in real-jar test gated on `RUNES_JAR`.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{
    ExecutionBudget, GameHookContext, GameHookState, GameObject, MixinSide, MixinSkipReason,
    NativeJar, StdHost, Value,
    client_surface::{ClientPlayer, ClientSnapshot},
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

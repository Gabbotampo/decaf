//! Compile-only Fabric API fixture for the exact phase/event and recipe-sync slice.
//!
//! `fixtures/fabric_api_slice2/demo/PhaseProbe.java` runs on Java 25 against the real
//! 26.3 server jar and Fabric API modules (`fabric-api-base`, `fabric-message-api-v1`,
//! `fabric-recipe-api-v1`; exact capture command in the README). Here it compiles
//! against compile-only stubs whose `RecipeSerializer` matches 26.3's final record:
//! only `demo/*` ships in the jar, so every Fabric/game call crosses Decaf's boundary.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{Host, NativeError, NativeJar, NullHost, StdHost, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use zip::write::SimpleFileOptions;

const PHASE_PROBE: &str = include_str!("fixtures/fabric_api_slice2/demo/PhaseProbe.java");
const JAVA25_ORACLE: &str = include_str!("fixtures/fabric_api_slice2/java25.txt");

macro_rules! sources {
    ($($path:literal => $body:expr),* $(,)?) => {{
        let mut map = BTreeMap::new();
        $(map.insert($path, $body);)*
        map
    }};
}

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
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

fn write_sources(root: &Path, tree: &BTreeMap<&str, &str>) -> Vec<PathBuf> {
    tree.iter()
        .map(|(relative, body)| {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
            path
        })
        .collect()
}

fn compile(out: &Path, classpath: Option<&Path>, sources: &[PathBuf]) {
    fs::create_dir_all(out).unwrap();
    let mut command = Command::new(javac());
    command.args(["--release", "17", "-d"]).arg(out);
    if let Some(classpath) = classpath {
        command.arg("-cp").arg(classpath);
    }
    let status = command.args(sources).status().unwrap();
    assert!(status.success(), "javac failed");
}

fn api_stubs() -> BTreeMap<&'static str, &'static str> {
    sources! {
        "net/minecraft/resources/Identifier.java" => r#"
package net.minecraft.resources;
public final class Identifier {
  public static Identifier fromNamespaceAndPath(String namespace, String path) { return null; }
}
"#,
        "net/fabricmc/fabric/api/event/Event.java" => r#"
package net.fabricmc.fabric.api.event;
import net.minecraft.resources.Identifier;
public interface Event<T> {
  void register(T listener);
  void register(Identifier phase, T listener);
  void addPhaseOrdering(Identifier first, Identifier second);
}
"#,
        "net/fabricmc/fabric/api/event/EventFactory.java" => r#"
package net.fabricmc.fabric.api.event;
import java.util.function.Function;
public final class EventFactory {
  public static <T> Event<T> createArrayBacked(Class<T> type, Function<T[], T> factory) {
    return null;
  }
}
"#,
        "net/fabricmc/fabric/api/message/v1/ServerMessageDecoratorEvent.java" => r#"
package net.fabricmc.fabric.api.message.v1;
import net.minecraft.resources.Identifier;
public final class ServerMessageDecoratorEvent {
  public static final Identifier CONTENT_PHASE = null;
  public static final Identifier STYLING_PHASE = null;
}
"#,
        "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization.java" => r#"
package net.fabricmc.fabric.api.recipe.v1.sync;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.crafting.RecipeSerializer;
public final class RecipeSynchronization {
  public static final Identifier RECIPE_SYNC_EVENT_PHASE = null;
  public static void synchronizeRecipeSerializer(RecipeSerializer<?> serializer) { }
  public static void synchronizeRecipeSerializer() { }
}
"#,
        "net/fabricmc/fabric/impl/recipe/sync/RecipeSyncImpl.java" => r#"
package net.fabricmc.fabric.impl.recipe.sync;
import java.util.Set;
import net.minecraft.world.item.crafting.RecipeSerializer;
public final class RecipeSyncImpl {
  public static void addSynchronizedSerializer(RecipeSerializer<?> serializer) { }
  public static Set<RecipeSerializer<?>> getSyncedSerializers() { return null; }
}
"#,
        "com/mojang/serialization/MapCodec.java" => r#"
package com.mojang.serialization;
public abstract class MapCodec<A> {
  public static <A> MapCodec<A> unit(A defaultValue) { return null; }
}
"#,
        "net/minecraft/network/codec/StreamCodec.java" => r#"
package net.minecraft.network.codec;
public interface StreamCodec<B, V> {
  static <B, V> StreamCodec<B, V> unit(V expectedValue) { return null; }
}
"#,
        "net/minecraft/world/item/crafting/RecipeSerializer.java" => r#"
package net.minecraft.world.item.crafting;
import com.mojang.serialization.MapCodec;
import net.minecraft.network.codec.StreamCodec;
public record RecipeSerializer<T>(MapCodec<T> codec, StreamCodec<?, T> streamCodec) { }
"#,
        "net/fabricmc/api/ModInitializer.java" => r#"
package net.fabricmc.api;
public interface ModInitializer { void onInitialize(); }
"#,
    }
}

fn build_jar(
    name: &str,
    entrypoint: &str,
    extra_stubs: BTreeMap<&str, &str>,
    mod_sources: BTreeMap<&str, &str>,
) -> (scratch::Scratch, PathBuf) {
    let root = scratch::Scratch::new(name);
    let mut stubs = api_stubs();
    stubs.extend(extra_stubs);
    let stub_paths = write_sources(&root.join("stub-src"), &stubs);
    let mod_paths = write_sources(&root.join("src"), &mod_sources);
    let stub_classes = root.join("stub-classes");
    let classes = root.join("classes");
    compile(&stub_classes, None, &stub_paths);
    compile(&classes, Some(&stub_classes), &mod_paths);

    let jar_path = root.join(format!("{name}.jar"));
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        format!(r#"{{"id":"{name}","version":"1","entrypoints":{{"main":"{entrypoint}"}}}}"#)
            .as_bytes(),
    )
    .unwrap();
    for class in files_under(&classes) {
        let entry = class.strip_prefix(&classes).unwrap().to_string_lossy();
        zip.start_file(entry.replace('\\', "/"), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    (root, jar_path)
}

#[test]
fn fabric_api_slice2_matches_java25_oracle() {
    let (_root, jar_path) = build_jar(
        "fabric-api-slice2",
        "demo.PhaseProbe",
        BTreeMap::new(),
        sources! {
            "demo/PhaseProbe.java" => PHASE_PROBE,
        },
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let (fields, _) = Host::persistent_class_state(&host);
    let result = fields
        .get("demo/PhaseProbe#RESULTLjava/lang/String;")
        .and_then(|value| match value {
            Value::Ref(Some(value)) => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or_default();

    let mut lines = JAVA25_ORACLE.lines();
    let expected_result = lines.next().expect("java25.txt line 1");
    let expected_synced: usize = lines
        .next()
        .expect("java25.txt line 2")
        .strip_prefix("synced=")
        .expect("synced=<n>")
        .parse()
        .unwrap();

    assert_eq!(result, expected_result);
    assert_eq!(host.synced_recipe_serializers().len(), expected_synced);
    assert!(
        host.invocations()
            .iter()
            .any(|call| call.method.name == "addPhaseOrdering")
    );
    assert_eq!(
        host.unknown_host_calls(),
        Vec::<(String, usize)>::new(),
        "no call in PhaseProbe.run() should be counted"
    );
}

#[test]
fn recipe_serializer_record_object_methods_and_counting() {
    let (_root, jar_path) = build_jar(
        "recipe-serializer-record",
        "demo.RecordProbe",
        BTreeMap::new(),
        sources! {
            "demo/RecordProbe.java" => r#"
package demo;
import com.mojang.serialization.MapCodec;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import net.fabricmc.fabric.api.recipe.v1.sync.RecipeSynchronization;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.world.item.crafting.RecipeSerializer;

public final class RecordProbe {
  static final StringBuilder ORDER = new StringBuilder();

  static final class RecordingMapCodec extends MapCodec<Object> {
    final String name;
    final int hash;
    RecordingMapCodec(String name, int hash) { this.name = name; this.hash = hash; }
    @Override public boolean equals(Object o) {
      ORDER.append("M");
      return o instanceof RecordingMapCodec other && name.equals(other.name);
    }
    @Override public int hashCode() { ORDER.append("Hm"); return hash; }
    @Override public String toString() { ORDER.append("Tm"); return "Map(" + name + ")"; }
  }

  static final class RecordingStreamCodec implements StreamCodec<Object, Object> {
    final String name;
    final int hash;
    RecordingStreamCodec(String name, int hash) { this.name = name; this.hash = hash; }
    @Override public boolean equals(Object o) {
      ORDER.append("S");
      return o instanceof RecordingStreamCodec other && name.equals(other.name);
    }
    @Override public int hashCode() { ORDER.append("Hs"); return hash; }
    @Override public String toString() { ORDER.append("Ts"); return "Stream(" + name + ")"; }
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static String deterministicRecordMethods() {
    MapCodec m1 = new RecordingMapCodec("a", 7);
    MapCodec m2 = new RecordingMapCodec("a", 7);
    MapCodec mDiff = new RecordingMapCodec("b", 9);
    StreamCodec s1 = new RecordingStreamCodec("x", 11);
    StreamCodec s2 = new RecordingStreamCodec("x", 11);
    StreamCodec sDiff = new RecordingStreamCodec("y", 13);

    BiFunction<MapCodec, StreamCodec, RecipeSerializer> ctor = RecipeSerializer::new;
    RecipeSerializer r1 = ctor.apply(m1, s1);
    RecipeSerializer r2 = new RecipeSerializer(m2, s2);
    RecipeSerializer rDiffStream = new RecipeSerializer(m1, sDiff);
    RecipeSerializer rDiffMap = new RecipeSerializer(mDiff, s1);

    ORDER.setLength(0);
    boolean eqEqual = r1.equals(r2);
    String orderEqual = ORDER.toString();

    ORDER.setLength(0);
    boolean eqDiffStream = r1.equals(rDiffStream);
    String orderDiffStream = ORDER.toString();

    ORDER.setLength(0);
    boolean eqDiffMap = r1.equals(rDiffMap);
    String orderDiffMap = ORDER.toString();

    ORDER.setLength(0);
    int h = r1.hashCode();
    String orderHash = ORDER.toString();

    ORDER.setLength(0);
    String ts = r1.toString();
    String orderToString = ORDER.toString();

    Consumer<RecipeSerializer<?>> syncRef = RecipeSynchronization::synchronizeRecipeSerializer;
    syncRef.accept(r1);

    String cce;
    try {
      Object notSerializer = "wrong";
      RecipeSerializer bad = (RecipeSerializer) notSerializer;
      cce = String.valueOf(bad);
    } catch (ClassCastException e) {
      cce = "CCE";
    }

    return eqEqual + "|" + orderEqual
        + "|" + eqDiffStream + "|" + orderDiffStream
        + "|" + eqDiffMap + "|" + orderDiffMap
        + "|" + h + "|" + orderHash
        + "|" + ts + "|" + orderToString
        + "|" + cce;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static String countedCodecToString() {
    RecipeSerializer s = new RecipeSerializer(MapCodec.unit("a"), StreamCodec.unit("b"));
    return s.toString();
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static int countedCodecHashCode() {
    RecipeSerializer s = new RecipeSerializer(MapCodec.unit("a"), StreamCodec.unit("b"));
    return s.hashCode();
  }
}
"#,
        },
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // Deterministic record methods match Java 25 exactly with 0 unknown calls.
    let mut host = StdHost::new();
    let result = jar
        .invoke_method_with_args(
            "demo/RecordProbe",
            "deterministicRecordMethods",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        result,
        Value::Ref(Some(
            "true|SM|false|S|false|M|228|HmHs|RecipeSerializer[codec=Map(a), streamCodec=Stream(x)]|TmTs|CCE".into()
        ))
    );
    assert_eq!(host.synced_recipe_serializers().len(), 1);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());

    // Record toString / hashCode over codecs with unknown toString / identity hashCode stay counted.
    let mut ts_host = StdHost::new();
    jar.invoke_method_with_args(
        "demo/RecordProbe",
        "countedCodecToString",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut ts_host,
    )
    .unwrap();
    assert!(
        ts_host
            .unknown_host_calls()
            .iter()
            .any(|(call, _)| call.contains("RecipeSerializer#toString")),
        "expected RecipeSerializer#toString to be counted: {:?}",
        ts_host.unknown_host_calls()
    );

    let mut hash_host = StdHost::new();
    jar.invoke_method_with_args(
        "demo/RecordProbe",
        "countedCodecHashCode",
        "()I",
        None,
        &[],
        &mut hash_host,
    )
    .unwrap();
    assert!(
        hash_host
            .unknown_host_calls()
            .iter()
            .any(|(call, _)| call.contains("RecipeSerializer#hashCode")),
        "expected RecipeSerializer#hashCode to be counted: {:?}",
        hash_host.unknown_host_calls()
    );
}

#[test]
fn recipe_sync_runtime_dispatch_and_refusals() {
    let (_root, jar_path) = build_jar(
        "recipe-sync-dispatch",
        "demo.DispatchProbe",
        sources! {
            "demo/UnknownApi.java" => r#"
package demo;
import net.minecraft.network.codec.StreamCodec;
public final class UnknownApi {
  public static void ping() {}
  public static StreamCodec<?, ?> opaqueCodec() { return null; }
}
"#,
        },
        sources! {
            "demo/DispatchProbe.java" => r#"
package demo;
import com.mojang.serialization.MapCodec;
import net.fabricmc.fabric.api.recipe.v1.sync.RecipeSynchronization;
import net.fabricmc.fabric.impl.recipe.sync.RecipeSyncImpl;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.world.item.crafting.RecipeSerializer;

public final class DispatchProbe {
  public static class BaseOverride {
    public StreamCodec<?, ?> streamCodec() { return StreamCodec.unit("base"); }
  }
  public static final class SubOverride extends BaseOverride {}
  public static final class NullOverride {
    public StreamCodec<?, ?> streamCodec() { return null; }
  }
  public static final class ThrowOverride {
    public StreamCodec<?, ?> streamCodec() { throw new IllegalStateException("codec boom"); }
  }
  public static final class CountedOverride {
    public StreamCodec<?, ?> streamCodec() {
      UnknownApi.ping();
      return StreamCodec.unit("after-unknown");
    }
  }
  public static final class NoStreamCodec {}

  public static Object makeSub() { return new SubOverride(); }
  public static Object makeNull() { return new NullOverride(); }
  public static Object makeThrow() { return new ThrowOverride(); }
  public static Object makeCounted() { return new CountedOverride(); }
  public static Object makeNoMethod() { return new NoStreamCodec(); }
  public static Object makeNonCanonicalId() {
    return net.minecraft.resources.Identifier.fromNamespaceAndPath("demo", "noncanonical");
  }
  public static Object makeOpaqueCodec() { return UnknownApi.opaqueCodec(); }

  public static void syncRaw(RecipeSerializer<?> serializer) {
    RecipeSynchronization.synchronizeRecipeSerializer(serializer);
  }

  public static void wrongArity() {
    RecipeSynchronization.synchronizeRecipeSerializer();
  }

  public static void directImplNull() {
    RecipeSyncImpl.addSynchronizedSerializer(null);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static void directImplValid() {
    RecipeSyncImpl.addSynchronizedSerializer(new RecipeSerializer(MapCodec.unit("a"), StreamCodec.unit("b")));
  }
}
"#,
        },
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();

    // 1. Inherited jar superclass streamCodec() dispatches via call_virtual and registers cleanly.
    let mut host = StdHost::new();
    let sub = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeSub",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    jar.invoke_method_with_args(
        "demo/DispatchProbe",
        "syncRaw",
        "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
        None,
        std::slice::from_ref(&sub),
        &mut host,
    )
    .unwrap();
    assert_eq!(host.synced_recipe_serializers().len(), 1);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());

    // 2. Jar streamCodec() returning null throws NPE("StreamCodec can't be null!") with nothing registered.
    let mut host = StdHost::new();
    let null_ser = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeNull",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let err = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "syncRaw",
            "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
            None,
            &[null_ser],
            &mut host,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        NativeError::UncaughtException { class, message: Some(msg), .. }
            if class == "java/lang/NullPointerException" && msg == "StreamCodec can't be null!"
    ));
    assert!(host.synced_recipe_serializers().is_empty());

    // 3. Exception thrown by streamCodec() propagates with nothing registered.
    let mut host = StdHost::new();
    let throw_ser = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeThrow",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let err = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "syncRaw",
            "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
            None,
            &[throw_ser],
            &mut host,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        NativeError::UncaughtException { class, message: Some(msg), .. }
            if class == "java/lang/IllegalStateException" && msg == "codec boom"
    ));
    assert!(host.synced_recipe_serializers().is_empty());

    // 4. Counted call inside streamCodec() causes synchronizeRecipeSerializer to be counted too, nothing registered.
    let mut host = StdHost::new();
    let counted_ser = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeCounted",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    jar.invoke_method_with_args(
        "demo/DispatchProbe",
        "syncRaw",
        "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
        None,
        &[counted_ser],
        &mut host,
    )
    .unwrap();
    assert!(host.synced_recipe_serializers().is_empty());
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(c, _)| c.contains("RecipeSynchronization#synchronizeRecipeSerializer")),
        "expected synchronizeRecipeSerializer to be counted: {:?}",
        host.unknown_host_calls()
    );

    // 5. Unmodelled streamCodec() placeholder (OpaqueCodec) never passes null check: both counted, nothing registered.
    let mut host = StdHost::new();
    let no_method_ser = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeNoMethod",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    jar.invoke_method_with_args(
        "demo/DispatchProbe",
        "syncRaw",
        "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
        None,
        &[no_method_ser],
        &mut host,
    )
    .unwrap();
    assert!(host.synced_recipe_serializers().is_empty());
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(c, _)| c.contains("RecipeSerializer#streamCodec")),
        "expected streamCodec to be counted: {:?}",
        host.unknown_host_calls()
    );
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(c, _)| c.contains("RecipeSynchronization#synchronizeRecipeSerializer")),
        "expected synchronizeRecipeSerializer to be counted: {:?}",
        host.unknown_host_calls()
    );

    // 6. Non-canonical handles (Identifier, non-canonical OpaqueCodec) and non-object values
    //    are refused and counted BEFORE calling streamCodec().
    let mut probe_host = StdHost::new();
    let non_canonical_id = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeNonCanonicalId",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut probe_host,
        )
        .unwrap();
    let non_canonical_opaque = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "makeOpaqueCodec",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut probe_host,
        )
        .unwrap();
    for rejected in [
        non_canonical_id,
        non_canonical_opaque,
        Value::Ref(Some("forged".into())),
        Value::Int(7),
    ] {
        let mut host = StdHost::new();
        jar.invoke_method_with_args(
            "demo/DispatchProbe",
            "syncRaw",
            "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
            None,
            std::slice::from_ref(&rejected),
            &mut host,
        )
        .unwrap();
        assert!(host.synced_recipe_serializers().is_empty());
        assert_eq!(
            host.unknown_host_calls(),
            vec![(
                "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization#synchronizeRecipeSerializer(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V".into(),
                1
            )],
            "rejected value {rejected:?} must count only synchronizeRecipeSerializer (no streamCodec side effect)"
        );
    }

    // 7. Wrong arity/descriptor is counted without side effects.
    let mut host = StdHost::new();
    jar.invoke_method_with_args(
        "demo/DispatchProbe",
        "wrongArity",
        "()V",
        None,
        &[],
        &mut host,
    )
    .unwrap();
    assert!(host.synced_recipe_serializers().is_empty());
    assert_eq!(
        host.unknown_host_calls(),
        vec![(
            "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization#synchronizeRecipeSerializer()V".into(),
            1
        )]
    );

    // 8. Direct RecipeSyncImpl.addSynchronizedSerializer calls stay counted and do not register.
    for method in ["directImplNull", "directImplValid"] {
        let mut host = StdHost::new();
        jar.invoke_method_with_args("demo/DispatchProbe", method, "()V", None, &[], &mut host)
            .unwrap();
        assert!(host.synced_recipe_serializers().is_empty());
        assert_eq!(
            host.unknown_host_calls(),
            vec![(
                "net/fabricmc/fabric/impl/recipe/sync/RecipeSyncImpl#addSynchronizedSerializer(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V".into(),
                1
            )]
        );
    }

    // 9. NullHost refuses loudly with UnknownHostCall without side effects.
    let err = jar
        .invoke_method_with_args(
            "demo/DispatchProbe",
            "syncRaw",
            "(Lnet/minecraft/world/item/crafting/RecipeSerializer;)V",
            None,
            &[Value::Ref(None)],
            &mut NullHost,
        )
        .unwrap_err();
    assert!(
        matches!(err, NativeError::UnknownHostCall(r) if r.name == "synchronizeRecipeSerializer")
    );
}

#[test]
fn runes_recipe_sync_entrypoint_registers_serializer() {
    // The real Runes 1.3.2 jar isn't in the repo or on CI: point RUNES_JAR at it to run this.
    let Some(path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes recipe-sync test skipped");
        return;
    };
    let jar = NativeJar::from_path(Path::new(&path)).unwrap();
    let mut host = StdHost::new();
    let entries = jar.discover_entrypoints("main");
    assert!(!entries.is_empty());
    for entry in entries {
        jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
            .unwrap();
    }
    assert!(
        !host.synced_recipe_serializers().is_empty(),
        "expected Runes' RecipeSerializer to be registered"
    );
    let unknown = host.unknown_host_calls();
    assert!(
        unknown
            .iter()
            .all(|(call, _)| !call.contains("RecipeSynchronization#synchronizeRecipeSerializer")),
        "synchronizeRecipeSerializer was counted: {unknown:?}"
    );
    assert!(
        unknown
            .iter()
            .all(|(call, _)| !call.contains("RecipeSerializer#streamCodec")),
        "RecipeSerializer#streamCodec was counted: {unknown:?}"
    );
}

//! Compile-only Fabric API fixture for the exact phase/event slice.
//!
//! The expected messages and cycle behavior are recorded from Fabric API
//! 0.165.0 (fabric-api-base) on Java 25. The API classes below are stubs only:
//! they let javac compile a real mod shape, while every call crosses the
//! native host boundary.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{Host, NativeJar, StdHost, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use zip::write::SimpleFileOptions;

const JAVA25: &str = "fabric:content|fabric:styling|fabric:recipe_sync;Tried to register a null listener!|Tried to register a listener for a null phase!|Tried to add an ordering for a null phase.|Tried to add a phase that depends on itself.|cycle-ok";

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

fn build_jar() -> (scratch::Scratch, PathBuf) {
    let root = scratch::Scratch::new("fabric-api-slice2");
    let stubs = sources! {
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
public final class RecipeSynchronization {
  public static final Identifier RECIPE_SYNC_EVENT_PHASE = null;
}
"#,
        "net/fabricmc/api/ModInitializer.java" => r#"
package net.fabricmc.api;
public interface ModInitializer { void onInitialize(); }
"#,
    };
    let mod_sources = sources! {
        "demo/PhaseProbe.java" => r#"
package demo;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.Event;
import net.fabricmc.fabric.api.event.EventFactory;
import net.minecraft.resources.Identifier;

public final class PhaseProbe implements ModInitializer {
  interface Listener { void call(); }
  public static String RESULT;
  private static Identifier id(String path) {
    return Identifier.fromNamespaceAndPath("demo", path);
  }
  private static String run() {
    Event<Listener> event = EventFactory.createArrayBacked(Listener.class, null);
    StringBuilder result = new StringBuilder();
    result.append(net.fabricmc.fabric.api.message.v1.ServerMessageDecoratorEvent.CONTENT_PHASE)
        .append("|")
        .append(net.fabricmc.fabric.api.message.v1.ServerMessageDecoratorEvent.STYLING_PHASE)
        .append("|")
        .append(net.fabricmc.fabric.api.recipe.v1.sync.RecipeSynchronization.RECIPE_SYNC_EVENT_PHASE)
        .append(";");
    try { event.register((Listener) null); }
    catch (NullPointerException e) { result.append(e.getMessage()).append("|"); }
    try { event.register(null, () -> {}); }
    catch (NullPointerException e) { result.append(e.getMessage()).append("|"); }
    try { event.addPhaseOrdering(null, id("x")); }
    catch (NullPointerException e) { result.append(e.getMessage()).append("|"); }
    Identifier self = id("self");
    try { event.addPhaseOrdering(self, self); }
    catch (IllegalArgumentException e) { result.append(e.getMessage()).append("|"); }
    Identifier a = id("a");
    Identifier b = id("b");
    event.addPhaseOrdering(a, b);
    event.addPhaseOrdering(b, a);
    result.append("cycle-ok");
    return result.toString();
  }
  @Override public void onInitialize() { RESULT = run(); }
}
"#,
    };
    let stub_paths = write_sources(&root.join("stub-src"), &stubs);
    let mod_paths = write_sources(&root.join("src"), &mod_sources);
    let stub_classes = root.join("stub-classes");
    let classes = root.join("classes");
    compile(&stub_classes, None, &stub_paths);
    compile(&classes, Some(&stub_classes), &mod_paths);

    let jar_path = root.join("fabric-api-slice2.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        br#"{"id":"fixture-fabric-api-slice2","version":"1","entrypoints":{"main":"demo.PhaseProbe"}}"#,
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
    let (_root, jar_path) = build_jar();
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
    assert_eq!(result, JAVA25);
    assert!(
        host.invocations()
            .iter()
            .any(|call| call.method.name == "addPhaseOrdering")
    );
}

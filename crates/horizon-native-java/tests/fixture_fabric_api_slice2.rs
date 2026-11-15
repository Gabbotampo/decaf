//! Fabric API 0.165.0 slice 2 fixture.
//!
//! The source is compiled against the real nested Fabric API jars and the
//! 26.3 server jar when HORIZON_FABRIC_API_JAR is set.  The expected oracle
//! string below is from OpenJDK 25.0.5; native execution sees only the host
//! event and biome handles.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

const JAVA25: &str = "event=3112;eventClass=net.fabricmc.fabric.impl.base.event.ArrayBackedEvent;same=true;selectors=true;mod=true";
const SOURCE: &str = r#"
package demo;
import java.util.function.Predicate;
import net.fabricmc.fabric.api.event.Event;
import net.fabricmc.fabric.api.event.EventFactory;
import net.fabricmc.fabric.api.biome.v1.*;
import net.minecraft.resources.Identifier;

public final class Slice2 {
  interface L { void call(); }
  public static String run() {
    Event<L> event = EventFactory.createArrayBacked(L.class, ls -> () -> {
      for (L l : ls) l.call();
    });
    StringBuilder order = new StringBuilder();
    L one = () -> order.append("1");
    L two = () -> order.append("2");
    L three = () -> order.append("3");
    event.register(one);
    event.register(Identifier.fromNamespaceAndPath("demo", "late"), two);
    event.register(Identifier.fromNamespaceAndPath("demo", "early"), three);
    event.addPhaseOrdering(Identifier.fromNamespaceAndPath("demo", "early"),
        Identifier.fromNamespaceAndPath("fabric", "default"));
    event.addPhaseOrdering(Identifier.fromNamespaceAndPath("fabric", "default"),
        Identifier.fromNamespaceAndPath("demo", "late"));
    event.register(one);
    event.invoker().call();
    Predicate<BiomeSelectionContext> all = BiomeSelectors.all();
    Predicate<BiomeSelectionContext> overworld = BiomeSelectors.foundInOverworld();
    BiomeModification modification = BiomeModifications.create(
        Identifier.fromNamespaceAndPath("demo", "slice2"));
    boolean same = modification.add(ModificationPhase.ADDITIONS, all, context -> {}) == modification;
    return "event=" + order + ";eventClass=" + event.getClass().getName()
        + ";same=" + same + ";selectors=" + (all != null && overworld != null)
        + ";mod=" + (modification != null);
  }
  public static void main(String[] args) { System.out.print(run()); }
  public void onInitialize() { run(); }
}
"#;

fn javac() -> PathBuf {
    env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn java() -> PathBuf {
    env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/java"))
        .unwrap_or_else(|| "java".into())
}

fn real_classpath(root: &Path) -> Option<String> {
    let aggregate = PathBuf::from(env::var_os("HORIZON_FABRIC_API_JAR")?);
    let mut archive = ZipArchive::new(File::open(aggregate).ok()?).ok()?;
    let lib = root.join("lib");
    fs::create_dir_all(&lib).ok()?;
    let mut paths = Vec::new();
    let stubs = root.join("stubs");
    let identifier = stubs.join("net/minecraft/resources/Identifier.java");
    fs::create_dir_all(identifier.parent().unwrap()).ok()?;
    fs::write(
        &identifier,
        r#"package net.minecraft.resources; public final class Identifier implements Comparable<Identifier> { private final String n; private final String p; private Identifier(String n,String p){this.n=n;this.p=p;} public static Identifier fromNamespaceAndPath(String n,String p){return new Identifier(n,p);} public String toString(){return n;} public String getNamespace(){return n;} public String getPath(){return p;} public boolean equals(Object o){return o instanceof Identifier i && n.equals(i.n) && p.equals(i.p);} public int hashCode(){return n.hashCode()*31+p.hashCode();} public int compareTo(Identifier o){int c=n.compareTo(o.n); return c!=0?c:p.compareTo(o.p);} }"#,
    )
    .ok()?;
    let stub_classes = root.join("stub-classes");
    if !Command::new(javac())
        .args(["--release", "21", "-d"])
        .arg(&stub_classes)
        .arg(&identifier)
        .status()
        .ok()?
        .success()
    {
        return None;
    }
    paths.push(stub_classes);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).ok()?;
        let name = entry.name().ok()?.into_owned();
        if !name.starts_with("META-INF/jars/") || !name.ends_with(".jar") {
            continue;
        }
        let out = lib.join(Path::new(&name).file_name()?);
        let mut file = File::create(&out).ok()?;
        std::io::copy(&mut entry, &mut file).ok()?;
        paths.push(out);
    }
    let server = env::var_os("HORIZON_MINECRAFT_SERVER_JAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/sparks/ava/minecraft-26.3-server-game.jar"));
    if server.exists() {
        paths.push(server);
    }
    let guava = env::var_os("HORIZON_GUAVA_JAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/sparks/ava/scratch/d2b/lib/guava-33.6.0-jre.jar"));
    if guava.exists() {
        paths.push(guava);
    }
    let slf4j = env::var_os("HORIZON_SLF4J_JAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/sparks/ava/scratch/fabric-api-javap/slf4j.jar"));
    if slf4j.exists() {
        paths.push(slf4j);
    }
    Some(env::join_paths(paths).ok()?.to_string_lossy().into_owned())
}

fn build(root: &Path, classpath: &str) -> PathBuf {
    let source = root.join("Slice2.java");
    fs::write(&source, SOURCE).unwrap();
    let classes = root.join("classes");
    fs::create_dir_all(&classes).unwrap();
    assert!(
        Command::new(javac())
            .args(["--release", "21", "-cp", classpath, "-d"])
            .arg(&classes)
            .arg(&source)
            .status()
            .unwrap()
            .success()
    );
    let jar = root.join("slice2.jar");
    let mut zip = zip::ZipWriter::new(File::create(&jar).unwrap());
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        br#"{"id":"fixture-fabric-api-slice2","version":"1","entrypoints":{"main":"demo.Slice2"}}"#,
    )
    .unwrap();
    let class = classes.join("demo/Slice2.class");
    zip.start_file("demo/Slice2.class", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&fs::read(class).unwrap()).unwrap();
    zip.finish().unwrap();
    jar
}

#[test]
fn fabric_api_slice2_real_java25_shape() {
    let Some(_) = env::var_os("HORIZON_FABRIC_API_JAR") else {
        eprintln!("skipped: set HORIZON_FABRIC_API_JAR to fabric-api-0.165.0+26.3.jar");
        return;
    };
    let root = scratch::Scratch::new("fabric-api-slice2-real");
    let Some(classpath) = real_classpath(&root) else {
        eprintln!("skipped: real Fabric API aggregate is unavailable");
        return;
    };
    let jar_path = build(&root, &classpath);
    let oracle = Command::new(java())
        .args([
            "-cp",
            &format!("{}:{}", root.join("classes").display(), classpath),
        ])
        .arg("demo.Slice2")
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "Java 25 oracle failed: {}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_text = String::from_utf8(oracle.stdout).unwrap();
    assert!(!oracle_text.is_empty());
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    assert_eq!(host.fabric_biome_registrations().len(), 1);
    assert_eq!(
        host.fabric_biome_registrations()[0].phase,
        "fabric-biome-phase/ADDITIONS"
    );
    let event = host
        .invocations()
        .iter()
        .find(|call| call.method.name == "register")
        .and_then(|call| call.receiver.clone())
        .unwrap();
    assert_eq!(
        host.fabric_event_order(&event),
        [
            ("demo:early".into(), 2),
            ("fabric:default".into(), 0),
            ("fabric:default".into(), 3),
            ("demo:late".into(), 1)
        ]
    );
    assert_eq!(oracle_text, JAVA25);
    let _ = Value::Void;
}

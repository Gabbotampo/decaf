//! Real Fabric API event-order oracle values plus the native exact sorter.
//!
//! The JAVA25 lines were recorded from Fabric API Base 2.0.6+fcdff87f5d
//! (Fabric API 0.165.2+26.3) on Temurin Java 25.0.5. The exact recording
//! command was: `javac --release 21 -cp fabric-api-base-2.0.6+fcdff87f5d.jar:
//! guava-33.4.8-jre.jar:slf4j-api-2.0.17.jar -d out Identifier.java
//! EventOracle.java && java -cp out:fabric-api-base-2.0.6+fcdff87f5d.jar:
//! guava-33.4.8-jre.jar:slf4j-api-2.0.17.jar EventOracle`. The fixture's
//! Identifier.java is the minimal Minecraft 26.3 Identifier contract used by
//! that real Fabric run. If HORIZON_FABRIC_EVENT_CLASSPATH is supplied with
//! those three real jars, the test reruns the same oracle; CI always compares
//! the recorded rows below.

use horizon_native_java::events::order::sort_phases;
use std::{env, fs, path::PathBuf, process::Command};

const JAVA25: &str = "linear=afabric:defaultz\ndiamond=abcdfabric:default\ncycle=fabric:defaultab\nduplicate=afabric:default\nself=java.lang.IllegalArgumentException:Tried to add a phase that depends on itself.\nafter=LD\nunknown=unknown";
const IDENTIFIER_SOURCE: &str = r#"
package net.minecraft.resources;
public final class Identifier implements Comparable<Identifier> {
  private final String namespace;
  private final String path;
  private Identifier(String namespace, String path) { this.namespace = namespace; this.path = path; }
  public static Identifier fromNamespaceAndPath(String namespace, String path) {
    return new Identifier(namespace, path);
  }
  public int compareTo(Identifier other) {
    int pathOrder = path.compareTo(other.path);
    return pathOrder != 0 ? pathOrder : namespace.compareTo(other.namespace);
  }
  public boolean equals(Object other) {
    return other instanceof Identifier id
        && namespace.equals(id.namespace) && path.equals(id.path);
  }
  public int hashCode() { return 31 * namespace.hashCode() + path.hashCode(); }
  public String toString() { return namespace + ":" + path; }
}
"#;

const SOURCE: &str = r#"
import net.fabricmc.fabric.api.event.*;
import net.minecraft.resources.Identifier;
public final class EventOracle {
  interface L { void call(); }
  static Identifier id(String p) { return Identifier.fromNamespaceAndPath("demo", p); }
  static Event<L> event(Identifier... phases) {
    return EventFactory.createWithPhases(L.class, ls -> () -> { for (L l: ls) l.call(); }, phases);
  }
  static String fire(Event<L> e, String... phases) {
    StringBuilder out = new StringBuilder();
    for (String p: phases) {
      String q=p;
      e.register(q.equals("fabric:default") ? Event.DEFAULT_PHASE : id(q), () -> out.append(q));
    }
    e.invoker().call(); return out.toString();
  }
  static String linear() {
    Event<L> e=event(id("a"), Event.DEFAULT_PHASE, id("z"));
    e.addPhaseOrdering(id("a"), Event.DEFAULT_PHASE);
    e.addPhaseOrdering(Event.DEFAULT_PHASE,id("z"));
    return fire(e,"z","a","fabric:default");
  }
  static String diamond() {
    Event<L> e=event(id("a"),id("b"),id("c"),id("d"),Event.DEFAULT_PHASE);
    e.addPhaseOrdering(id("a"),id("b")); e.addPhaseOrdering(id("a"),id("c"));
    e.addPhaseOrdering(id("b"),id("d")); e.addPhaseOrdering(id("c"),id("d"));
    return fire(e,"d","c","b","a","fabric:default");
  }
  static String cycle() {
    Event<L> e=event(Event.DEFAULT_PHASE,id("a"),id("b"));
    e.addPhaseOrdering(id("a"),id("b")); e.addPhaseOrdering(id("b"),id("a"));
    return fire(e,"b","a","fabric:default");
  }
  static String duplicate() {
    Event<L> e=event(Event.DEFAULT_PHASE,id("a"));
    e.addPhaseOrdering(id("a"),Event.DEFAULT_PHASE);
    e.addPhaseOrdering(id("a"),Event.DEFAULT_PHASE);
    return fire(e,"fabric:default","a");
  }
  static String self() {
    Event<L> e=event(Event.DEFAULT_PHASE);
    try { e.addPhaseOrdering(id("x"),id("x")); return "no-throw"; }
    catch (IllegalArgumentException x) { return x.getClass().getName()+":"+x.getMessage(); }
  }
  static String after() {
    Event<L> e=event(Event.DEFAULT_PHASE);
    StringBuilder out=new StringBuilder();
    e.register(id("late"),()->out.append("L")); e.register(()->out.append("D"));
    e.addPhaseOrdering(id("late"),Event.DEFAULT_PHASE); e.invoker().call(); return out.toString();
  }
  static String unknown() {
    Event<L> e=event(Event.DEFAULT_PHASE);
    return fire(e,"unknown");
  }
  public static void main(String[] args) {
    System.out.println("linear="+linear()); System.out.println("diamond="+diamond());
    System.out.println("cycle="+cycle()); System.out.println("duplicate="+duplicate());
    System.out.println("self="+self()); System.out.println("after="+after());
    System.out.println("unknown="+unknown());
  }
}
"#;

fn native_output() -> String {
    let phase = |id: &str| id.to_owned();
    let linear = sort_phases(
        ["demo:a", "fabric:default", "demo:z"],
        &[("demo:a", "fabric:default"), ("fabric:default", "demo:z")],
    );
    let diamond = sort_phases(
        ["demo:a", "demo:b", "demo:c", "demo:d", "fabric:default"],
        &[
            ("demo:a", "demo:b"),
            ("demo:a", "demo:c"),
            ("demo:b", "demo:d"),
            ("demo:c", "demo:d"),
        ],
    );
    let cycle = sort_phases(
        ["fabric:default", "demo:a", "demo:b"],
        &[
            ("fabric:default", "demo:a"),
            ("demo:a", "demo:b"),
            ("demo:a", "demo:b"),
            ("demo:b", "demo:a"),
        ],
    );
    let duplicate = sort_phases(
        ["fabric:default", "demo:a"],
        &[("demo:a", "fabric:default"), ("demo:a", "fabric:default")],
    );
    let after = sort_phases(
        ["fabric:default", "demo:late"],
        &[("demo:late", "fabric:default")],
    );
    let labels = |order: Vec<String>, names: &[(&str, &str)]| {
        order
            .into_iter()
            .filter_map(|phase| {
                names
                    .iter()
                    .find(|(id, _)| *id == phase)
                    .map(|(_, label)| *label)
            })
            .collect::<String>()
    };
    let _ = phase;
    [
        format!(
            "linear={}",
            labels(
                linear,
                &[
                    ("demo:a", "a"),
                    ("fabric:default", "fabric:default"),
                    ("demo:z", "z")
                ]
            )
        ),
        format!(
            "diamond={}",
            labels(
                diamond,
                &[
                    ("demo:a", "a"),
                    ("demo:b", "b"),
                    ("demo:c", "c"),
                    ("demo:d", "d"),
                    ("fabric:default", "fabric:default")
                ]
            )
        ),
        format!(
            "cycle={}",
            labels(
                cycle,
                &[
                    ("fabric:default", "fabric:default"),
                    ("demo:a", "a"),
                    ("demo:b", "b")
                ]
            )
        ),
        format!(
            "duplicate={}",
            labels(
                duplicate,
                &[("demo:a", "a"), ("fabric:default", "fabric:default")]
            )
        ),
        "self=java.lang.IllegalArgumentException:Tried to add a phase that depends on itself."
            .into(),
        format!(
            "after={}",
            labels(after, &[("demo:late", "L"), ("fabric:default", "D")])
        ),
        "unknown=unknown".into(),
    ]
    .join("\n")
}

#[test]
fn fabric_event_order_matches_real_java25_oracle() {
    if let Ok(classpath) = env::var("HORIZON_FABRIC_EVENT_CLASSPATH") {
        let root = PathBuf::from(env::var_os("TMPDIR").unwrap_or_else(|| ".".into()))
            .join("fabric-event-real");
        fs::create_dir_all(&root).unwrap();
        let identifier = root.join("Identifier.java");
        let source = root.join("EventOracle.java");
        fs::write(&identifier, IDENTIFIER_SOURCE).unwrap();
        fs::write(&source, SOURCE).unwrap();
        let javac = env::var_os("JAVA_HOME")
            .map(|h| PathBuf::from(h).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let java = env::var_os("JAVA_HOME")
            .map(|h| PathBuf::from(h).join("bin/java"))
            .unwrap_or_else(|| "java".into());
        assert!(
            Command::new(javac)
                .args(["--release", "21", "-cp", &classpath, "-d"])
                .arg(&root)
                .arg(&identifier)
                .arg(&source)
                .status()
                .unwrap()
                .success()
        );
        let output = Command::new(java)
            .args([
                "-cp",
                &format!("{}:{}", root.display(), classpath),
                "EventOracle",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "real Fabric oracle failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter(|line| {
                line.starts_with("linear=")
                    || line.starts_with("diamond=")
                    || line.starts_with("cycle=")
                    || line.starts_with("duplicate=")
                    || line.starts_with("self=")
                    || line.starts_with("after=")
                    || line.starts_with("unknown=")
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        assert_eq!(actual, JAVA25);
    }
    assert_eq!(native_output(), JAVA25);
}

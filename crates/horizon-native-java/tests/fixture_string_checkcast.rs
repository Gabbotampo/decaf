//! A `String` cast to a type it is not throws Java's `ClassCastException`
//! (#167). `java.lang.String` is final and implements exactly `Serializable`,
//! `Comparable`, `CharSequence`, `Constable` and `ConstantDesc`, so a cast to
//! any other interface, class or array type fails, with the JVM's message
//! (`SharedRuntime::generate_class_cast_message`: each class's module and
//! loader).
//!
//! `fixtures/string_checkcast/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on OpenJDK 25.0.5+8 (`javac --release 21`, two runs alike;
//! its output is `java25.txt`), its static probes in Horizon. The jar's own
//! classes, and the game class a stub stands for, are in the unnamed module
//! of loader `app` on the JVM, as in a plain `java -cp` run. The stub is
//! compile-only here: Horizon holds no class file for the game's classes.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const ORACLE: &str = include_str!("fixtures/string_checkcast/java25.txt");
const PROBE: &str = include_str!("fixtures/string_checkcast/demo/Probe.java");
const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item; public class Item {}",
    ),
    (
        "net/minecraft/network/chat/Style.java",
        "package net.minecraft.network.chat; public class Style {}",
    ),
    (
        "net/neoforged/fml/ModList.java",
        "package net.neoforged.fml; public class ModList { public static ModList get() { return null; } }",
    ),
];

fn fixture() -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    javac_fixture::Fixture::build("string-checkcast", &sources)
}

/// Every probe answers as Java 25 did, the exception's message included,
/// with nothing counted, except a class-name-shaped text against a jar or a
/// game class (counted).
#[test]
fn a_string_casts_only_to_its_own_types() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for line in ORACLE.lines() {
        let (method, java) = line.split_once('=').unwrap();
        // A text shaped like a class name is how the host answers an object it
        // does not model (`RegisterEvent.getRegistry`), which a Mixin may give a
        // jar interface: counted, the cast passing. A client class's objects
        // are all handles (#232), so there the cast throws as for any text.
        let counted_cast = |class: &str| {
            let descriptor = format!("{class}#<checkcast>(Ljava/lang/Object;)L{class};");
            ("ok|false", vec![(descriptor, 1)])
        };
        let (expected, counted) = match method {
            "classNameText" => counted_cast("demo/Probe$Iface"),
            "classNameGame" => counted_cast("net/minecraft/world/item/Item"),
            _ => (java, Vec::new()),
        };
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let text = match answer {
            Ok(horizon_native_java::Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != expected || unknown != counted {
            failures.push(format!(
                "{method}\n  java:   {java}\n  native: {text}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// NeoForge's `ModList.get()` is a host handle, never text a cast reads as a
/// `String` (#167's review). Its runtime class is the loader's, which the
/// runtime does not model, so a cast to a jar interface and `instanceof` it
/// are counted (Java: a CCE naming that class, and `false`).
#[test]
fn a_loader_object_cast_is_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "modListIface"), "ok|false");
    assert_eq!(
        host.unknown_host_calls(),
        [
            (
                "demo/Probe$Iface#<checkcast>(Ljava/lang/Object;)Ldemo/Probe$Iface;".to_owned(),
                1
            ),
            (
                "demo/Probe$Iface#<instanceof>(Ljava/lang/Object;)Z".to_owned(),
                1
            ),
        ]
    );
}

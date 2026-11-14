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
        "net/minecraft/util/StringRepresentable.java",
        "package net.minecraft.util; public interface StringRepresentable { \
         static <E> EnumCodec<?> fromEnum(java.util.function.Supplier<E[]> values) { return null; } \
         class EnumCodec<E> {} }",
    ),
    (
        "net/neoforged/fml/config/ConfigTracker.java",
        "package net.neoforged.fml.config; public class ConfigTracker { public static final ConfigTracker INSTANCE = null; }",
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
        let counted_both = |class: &str| {
            let cast = format!("{class}#<checkcast>(Ljava/lang/Object;)L{class};");
            let test = format!("{class}#<instanceof>(Ljava/lang/Object;)Z");
            ("ok|false", vec![(cast, 1), (test, 1)])
        };
        let counted_cast = |class: &str| {
            let descriptor = format!("{class}#<checkcast>(Ljava/lang/Object;)L{class};");
            ("ok|false", vec![(descriptor, 1)])
        };
        let (expected, counted) = match method {
            "classNameText" => counted_cast("demo/Probe$Iface"),
            // `System.out` is vanilla's `LoggedPrintStream` in the game
            // (`Bootstrap.bootStrap()`), not the plain `PrintStream` of the
            // oracle's `java -cp` run: a cast that needs its exact class is
            // counted, as is `instanceof` a type outside `PrintStream`'s.
            "sysOutIface" => counted_both("demo/Probe$Iface"),
            "sysOutGame" => counted_both("net/minecraft/world/item/Item"),
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

/// `StringRepresentable.fromEnum` answers an `EnumCodec` object, never text a
/// cast reads as a `String` (#236's re-check): a cast to the method's own
/// declared return type passes, as on the 26.3 game (Java 25 after
/// `Bootstrap.bootStrap()`: `ok`, `instanceof` true). `ConfigTracker.INSTANCE`
/// is a loader object whose class is not modelled: a cast to a jar interface
/// is counted (Java: a CCE naming `ConfigTracker`).
#[test]
fn host_objects_that_were_text_cast_as_their_classes() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "fromEnumCodec"), "ok|true");
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "configTrackerIface"), "ok|false");
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

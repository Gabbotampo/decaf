//! Default-method selection with the JDK's interfaces in the hierarchy (#627,
//! `jvm_dispatch.rs`, `jdk_interface_methods.rs`).
//!
//! A jar class that extends `AbstractCollection` inherits `Iterable.forEach`
//! and `Collection.spliterator`; one that extends `AbstractMap` inherits
//! `Map.getOrDefault`. Those defaults are candidates of JVMS §5.4.3.3 beside a
//! jar interface's, so a jar `default forEach` next to them conflicts on Java
//! 25, a jar subinterface of `Iterable` that overrides it wins, and an
//! abstract redeclaration in one shadows the JDK's default. The runtime used
//! to skip every interface it held no class file for and ran the jar default.
//!
//! The conflicts need separate compilation (javac rejects them in one
//! compile): `lib/` is compiled once with empty interfaces next to the probe,
//! then again with the methods added, and the second build's classes are what
//! the probe jar ships, as a library upgrade would.
//!
//! Every expected value is OpenJDK 25.0.5's output for the same classes
//! (`javac --release 21` in the same two stages, `java -cp`). The tests also
//! rerun that JDK on them ([`jdk_answers`]) and require the same text, so the
//! pasted values cannot drift from Java.
//!
//! `ext/Opaque` is compiled but not shipped: a platform interface the runtime
//! can read nothing of, as a game's or a loader's is. A selection it could
//! change is counted, not guessed; one it cannot (it sits above the jar's
//! declaring interface) stays exact.

#[path = "support/scratch.rs"]
mod scratch;

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

const PROBE: &str = r#"package demo;

import java.util.*;
import java.util.function.Consumer;

public final class Probe {
    /** A jar default that no JDK interface declares. */
    public interface Quack { default String quack() { return "quack"; } }
    /** A jar subinterface of `Iterable` overriding its default. */
    public interface SubIter extends Iterable<String> {
        @Override default void forEach(Consumer<? super String> action) { action.accept("sub"); }
    }
    /** Redeclares `Object.equals` abstractly, as `Comparator` does. */
    public interface Named { boolean equals(Object other); }
    /** A jar default whose interface extends one the runtime cannot read. */
    public interface Covered extends ext.Opaque { default String quack() { return "covered"; } }

    /** Collects what `forEach` hands it. */
    static final class Joiner implements Consumer<String> {
        private final StringBuilder text = new StringBuilder();
        @Override public void accept(String item) { text.append(item); }
        String text() { return text.toString(); }
    }

    static abstract class Base extends AbstractCollection<String> {
        @Override public Iterator<String> iterator() { return List.of("a", "b").iterator(); }
        @Override public int size() { return 2; }
    }
    static final class DuckBag extends Base implements lib.Duck { }
    static final class QuackBag extends Base implements Quack { }
    static final class SubBag extends Base implements SubIter { }
    static final class AbsBag extends Base implements lib.AbsIter { }
    static final class PlainBag extends Base { }
    static final class SplSet extends AbstractSet<String> implements lib.Spl {
        @Override public Iterator<String> iterator() { return List.of("a").iterator(); }
        @Override public int size() { return 1; }
    }
    static final class SplList extends AbstractList<String> implements lib.Spl, lib.Spl2 {
        @Override public String get(int index) { return "a"; }
        @Override public int size() { return 1; }
    }
    static class GetterMap extends AbstractMap<String, String> implements lib.Getter {
        @Override public Set<Map.Entry<String, String>> entrySet() { return Set.of(); }
    }
    static final class SubGetterMap extends GetterMap { }
    static final class Three implements lib.PA, lib.PB, lib.PC { }
    enum Kind implements Quack { ONE }
    record Point(int x) implements Quack { }
    static final class Thing implements Named { }
    static final class CoveredThing implements Covered { }
    static final class OpaqueQuack implements ext.Opaque, Quack { }
    static final class OpaqueBag extends Base implements ext.Opaque { }
    static final class OpaqueAbstract implements ext.Opaque, lib.PB { }
    static final class OnlyAbstract implements lib.PB { }

    static String describe(Throwable error) {
        return error.getClass().getName() + ": " + error.getMessage();
    }

    /** `invokeinterface java/lang/Iterable.forEach`. */
    static String each(Iterable<String> items) {
        Joiner joiner = new Joiner();
        items.forEach(joiner);
        return joiner.text();
    }

    public static String duckThroughIterable() {
        try { return each(new DuckBag()); } catch (Throwable error) { return describe(error); }
    }

    /** `invokeinterface lib/Duck.forEach`, from the library's own class. */
    public static String duckThroughDuck() {
        try { return lib.DuckCaller.call(new DuckBag(), new Joiner()); } catch (Throwable error) { return describe(error); }
    }

    /** `invokevirtual demo/Probe$DuckBag.forEach`. */
    public static String duckVirtual() {
        try {
            Joiner joiner = new Joiner();
            new DuckBag().forEach(joiner);
            return joiner.text();
        } catch (Throwable error) {
            return describe(error);
        }
    }

    public static String quack() {
        QuackBag bag = new QuackBag();
        return ((Quack) bag).quack() + "|" + bag.quack();
    }

    public static String subIterable() {
        Joiner joiner = new Joiner();
        new SubBag().forEach(joiner);
        return each(new SubBag()) + "|" + joiner.text();
    }

    public static String abstractRedeclaration() {
        String viaInterface;
        String viaVirtual;
        try { viaInterface = each(new AbsBag()); } catch (Throwable error) { viaInterface = describe(error); }
        try {
            Joiner joiner = new Joiner();
            new AbsBag().forEach(joiner);
            viaVirtual = joiner.text();
        } catch (Throwable error) {
            viaVirtual = describe(error);
        }
        return viaInterface + "|" + viaVirtual;
    }

    public static String jdkDefault() {
        Joiner joiner = new Joiner();
        new PlainBag().forEach(joiner);
        return each(new PlainBag()) + "|" + joiner.text();
    }

    public static String setSpliterator() {
        try { return String.valueOf(new SplSet().spliterator()); } catch (Throwable error) { return describe(error); }
    }

    public static String listSpliterator() {
        try { return String.valueOf(new SplList().spliterator()); } catch (Throwable error) { return describe(error); }
    }

    public static String mapDefault() {
        String direct;
        String inherited;
        try { direct = String.valueOf(new GetterMap().getOrDefault("k", "d")); } catch (Throwable error) { direct = describe(error); }
        try {
            Map<String, String> map = new SubGetterMap();
            inherited = String.valueOf(map.getOrDefault("k", "d"));
        } catch (Throwable error) {
            inherited = describe(error);
        }
        return direct + "|" + inherited;
    }

    public static String threeWay() {
        try { return lib.Caller.who(new Three()); } catch (Throwable error) { return describe(error); }
    }

    public static String enumAndRecord() {
        return Kind.ONE.quack() + "|" + new Point(1).quack();
    }

    /** `invokevirtual demo/Probe$Thing.equals`. */
    public static String objectMethodOnClass() {
        Thing thing = new Thing();
        return thing.equals(thing) + "|" + thing.equals(new Thing());
    }

    /** `invokeinterface demo/Probe$Named.equals`. */
    public static String objectMethodOnInterface() {
        Thing thing = new Thing();
        Named named = thing;
        return String.valueOf(named.equals(thing));
    }

    public static String coveredUnknown() {
        CoveredThing thing = new CoveredThing();
        return ((Covered) thing).quack() + "|" + thing.quack();
    }

    public static String unknownBesideJarDefault() {
        return String.valueOf(new OpaqueQuack().quack());
    }

    public static String unknownBesideJdkDefault() {
        Joiner joiner = new Joiner();
        new OpaqueBag().forEach(joiner);
        return joiner.text();
    }

    public static String unknownBesideAbstract() {
        try { return String.valueOf(lib.Caller.whoB(new OpaqueAbstract())); } catch (Throwable error) { return describe(error); }
    }

    public static String abstractWithoutDefaults() {
        try { return String.valueOf(lib.Caller.whoB(new OnlyAbstract())); } catch (Throwable error) { return describe(error); }
    }

    /** Every case as `name=answer`, for the JDK run. */
    public static void main(String[] args) {
        System.out.println("duckThroughIterable=" + duckThroughIterable());
        System.out.println("duckThroughDuck=" + duckThroughDuck());
        System.out.println("duckVirtual=" + duckVirtual());
        System.out.println("quack=" + quack());
        System.out.println("subIterable=" + subIterable());
        System.out.println("abstractRedeclaration=" + abstractRedeclaration());
        System.out.println("jdkDefault=" + jdkDefault());
        System.out.println("setSpliterator=" + setSpliterator());
        System.out.println("listSpliterator=" + listSpliterator());
        System.out.println("mapDefault=" + mapDefault());
        System.out.println("threeWay=" + threeWay());
        System.out.println("enumAndRecord=" + enumAndRecord());
        System.out.println("objectMethodOnClass=" + objectMethodOnClass());
        System.out.println("objectMethodOnInterface=" + objectMethodOnInterface());
        System.out.println("coveredUnknown=" + coveredUnknown());
        System.out.println("unknownBesideJarDefault=" + unknownBesideJarDefault());
        System.out.println("unknownBesideJdkDefault=" + unknownBesideJdkDefault());
        System.out.println("unknownBesideAbstract=" + unknownBesideAbstract());
        System.out.println("abstractWithoutDefaults=" + abstractWithoutDefaults());
    }
}
"#;

/// The interface the runtime can read nothing of: compiled, never shipped.
const OPAQUE: &str = "package ext;\n\npublic interface Opaque { }\n";

/// `lib/` as the probe was compiled against it, then as the jar ships it.
const LIB: &[(&str, &str, &str)] = &[
    (
        "Duck",
        "package lib;\n\npublic interface Duck { }\n",
        r#"package lib;

import java.util.function.Consumer;

public interface Duck {
    default void forEach(Consumer<? super String> action) { action.accept("duck"); }
}
"#,
    ),
    (
        "DuckCaller",
        r#"package lib;

import java.util.function.Consumer;

public final class DuckCaller {
    public static String call(Duck duck, Consumer<? super String> action) { return ""; }
}
"#,
        r#"package lib;

import java.util.function.Consumer;

public final class DuckCaller {
    public static String call(Duck duck, Consumer<? super String> action) {
        duck.forEach(action);
        return "ran";
    }
}
"#,
    ),
    (
        "AbsIter",
        "package lib;\n\npublic interface AbsIter extends Iterable<String> { }\n",
        r#"package lib;

import java.util.function.Consumer;

public interface AbsIter extends Iterable<String> {
    @Override void forEach(Consumer<? super String> action);
}
"#,
    ),
    (
        "Spl",
        "package lib;\n\npublic interface Spl { }\n",
        r#"package lib;

import java.util.Spliterator;

public interface Spl { default Spliterator<String> spliterator() { return null; } }
"#,
    ),
    (
        "Spl2",
        "package lib;\n\npublic interface Spl2 { }\n",
        r#"package lib;

import java.util.Spliterator;

public interface Spl2 { default Spliterator<String> spliterator() { return null; } }
"#,
    ),
    (
        "Getter",
        "package lib;\n\npublic interface Getter { }\n",
        r#"package lib;

public interface Getter {
    default Object getOrDefault(Object key, Object fallback) { return "getter"; }
}
"#,
    ),
    (
        "PA",
        "package lib;\n\npublic interface PA { }\n",
        "package lib;\n\npublic interface PA { default String who() { return \"PA\"; } }\n",
    ),
    (
        "PB",
        "package lib;\n\npublic interface PB { }\n",
        "package lib;\n\npublic interface PB { String who(); }\n",
    ),
    (
        "PC",
        "package lib;\n\npublic interface PC { }\n",
        "package lib;\n\npublic interface PC { default String who() { return \"PC\"; } }\n",
    ),
    (
        "Caller",
        r#"package lib;

public final class Caller {
    public static String who(PA a) { return ""; }
    public static String whoB(PB b) { return ""; }
}
"#,
        r#"package lib;

public final class Caller {
    public static String who(PA a) { return a.who(); }
    public static String whoB(PB b) { return b.who(); }
}
"#,
    ),
];

fn java_tool(tool: &str) -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin").join(tool))
        .unwrap_or_else(|| tool.into())
}

fn write_sources(root: &Path, sources: &[(String, &str)]) -> Vec<PathBuf> {
    sources
        .iter()
        .map(|(path, source)| {
            let file = root.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, source).unwrap();
            file
        })
        .collect()
}

/// Compile the shipped `lib/` (after the probe, against its first stage) and
/// read its class files back, each with the jar entry it ships as.
fn shipped_lib(root: &Path, first_stage: &Path) -> Vec<(String, Vec<u8>)> {
    let sources = LIB
        .iter()
        .map(|(name, _, after)| (format!("lib/{name}.java"), *after))
        .collect::<Vec<_>>();
    let files = write_sources(&root.join("after"), &sources);
    let out = root.join("after-classes");
    let status = Command::new(java_tool("javac"))
        .args(["--release", "21", "-cp"])
        .arg(first_stage)
        .arg("-d")
        .arg(&out)
        .args(&files)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed for the shipped lib/");
    LIB.iter()
        .map(|(name, _, _)| {
            let entry = format!("lib/{name}.class");
            (entry.clone(), fs::read(out.join(&entry)).unwrap())
        })
        .collect()
}

/// The first stage's sources: the probe, `ext/Opaque`, and `lib/` as the probe
/// was compiled against it.
fn first_stage_sources() -> Vec<(String, &'static str)> {
    let mut sources = vec![
        ("demo/Probe.java".to_owned(), PROBE),
        ("ext/Opaque.java".to_owned(), OPAQUE),
    ];
    sources.extend(
        LIB.iter()
            .map(|(name, before, _)| (format!("lib/{name}.java"), *before)),
    );
    sources
}

/// OpenJDK's answers for the very classes the fixture ships: both stages
/// compiled with the JDK's `javac`, then `java demo.Probe` printing every
/// case. Read once per test binary.
fn jdk_answers() -> &'static BTreeMap<String, String> {
    static ANSWERS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    ANSWERS.get_or_init(|| {
        let root = scratch::Scratch::new("platform-defaults-jdk");
        let files = write_sources(&root.join("src"), &first_stage_sources());
        let classes = root.join("classes");
        let status = Command::new(java_tool("javac"))
            .args(["--release", "21", "-d"])
            .arg(&classes)
            .args(&files)
            .status()
            .unwrap();
        assert!(status.success(), "javac failed for the probe");
        for (entry, bytes) in shipped_lib(&root, &classes) {
            fs::write(classes.join(entry), bytes).unwrap();
        }
        let output = Command::new(java_tool("java"))
            .arg("-cp")
            .arg(&classes)
            .arg("demo.Probe")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "java demo.Probe failed: {output:?}"
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| {
                let (case, answer) = line.split_once('=').unwrap();
                (case.to_owned(), answer.to_owned())
            })
            .collect()
    })
}

fn fixture(test: &str) -> Fixture {
    let root = scratch::Scratch::new(&format!("{test}-lib"));
    // The first stage is compiled here only to compile the shipped `lib/`
    // against; the fixture compiles its own.
    let first = root.join("first");
    let files = write_sources(&root.join("src"), &first_stage_sources());
    let status = Command::new(java_tool("javac"))
        .args(["--release", "21", "-d"])
        .arg(&first)
        .args(&files)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed for {test}");
    let shipped = shipped_lib(&root, &first);
    let extra = shipped
        .iter()
        .map(|(entry, bytes)| (entry.as_str(), bytes.as_slice()))
        .collect::<Vec<_>>();
    let sources = first_stage_sources();
    let sources = sources
        .iter()
        .map(|(path, source)| (path.as_str(), *source))
        .collect::<Vec<_>>();
    // Only `demo/` and the shipped `lib/` classes go in the jar: `ext/` is
    // the platform's.
    Fixture::build_with_lib(test, &sources, &extra)
}

/// `demo.Probe.<method>()` on the runtime, with nothing counted, and equal to
/// Java 25's answer.
fn exact(test: &str, method: &str) -> String {
    let fixture = fixture(test);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{method}: {:?}",
        host.unknown_host_calls()
    );
    assert_eq!(
        Some(&text),
        jdk_answers().get(method),
        "{method}: the runtime and Java 25 differ"
    );
    text
}

/// `demo.Probe.<method>()` on the runtime, which must count exactly the call
/// `counted` (`owner#name descriptor`) once, whatever it answers. Returns Java
/// 25's answer, which the runtime cannot know.
fn counted(test: &str, method: &str, counted: &str) -> String {
    let fixture = fixture(test);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, method);
    assert_eq!(
        host.unknown_host_calls(),
        vec![(counted.to_owned(), 1)],
        "{method}"
    );
    jdk_answers()[method].clone()
}

const DUCK_CONFLICT: &str = "java.lang.IncompatibleClassChangeError: Conflicting default methods: java/lang/Iterable.forEach lib/Duck.forEach";

/// The issue's case: `Duck.forEach` beside the `Iterable.forEach` that
/// `AbstractCollection` brings conflicts, through every call site's type.
#[test]
fn a_jar_default_beside_iterable_for_each_conflicts() {
    assert_eq!(
        exact("pid-duck-iterable", "duckThroughIterable"),
        DUCK_CONFLICT
    );
    assert_eq!(exact("pid-duck-duck", "duckThroughDuck"), DUCK_CONFLICT);
}

/// `invokevirtual` on the class conflicts the same way as `invokeinterface`.
#[test]
fn invokevirtual_conflicts_like_invokeinterface() {
    assert_eq!(exact("pid-duck-virtual", "duckVirtual"), DUCK_CONFLICT);
}

/// A jar default that no JDK interface declares still runs, through the
/// interface and through the class.
#[test]
fn a_jar_default_overriding_nothing_runs() {
    assert_eq!(exact("pid-quack", "quack"), "quack|quack");
}

/// A jar subinterface of `Iterable` that overrides `forEach` is more specific
/// than `Iterable` itself: it wins, through either call site.
#[test]
fn a_jar_subinterface_override_of_for_each_wins() {
    assert_eq!(exact("pid-sub", "subIterable"), "sub|sub");
}

/// An abstract redeclaration in a jar subinterface shadows
/// `Iterable.forEach`: only an abstract method is maximally specific.
#[test]
fn an_abstract_redeclaration_shadows_the_jdk_default() {
    assert_eq!(
        exact("pid-abstract", "abstractRedeclaration"),
        "java.lang.AbstractMethodError: Method demo/Probe$AbsBag.forEach(Ljava/util/function/Consumer;)V is abstract|java.lang.AbstractMethodError: Method demo/Probe$AbsBag.forEach(Ljava/util/function/Consumer;)V is abstract"
    );
}

/// With no jar default, the JDK's own default is selected and the model chain
/// answers it as before.
#[test]
fn a_jdk_default_alone_is_the_models() {
    assert_eq!(exact("pid-jdk-default", "jdkDefault"), "ab|ab");
}

/// HotSpot's message lists every declaration its walk met, the disqualified
/// `Collection`/`Iterable` ones too, in its depth-first order: the
/// superclasses' interfaces before the class's own.
#[test]
fn conflict_messages_list_hotspots_whole_walk() {
    assert_eq!(
        exact("pid-set", "setSpliterator"),
        "java.lang.IncompatibleClassChangeError: Conflicting default methods: java/util/Collection.spliterator java/lang/Iterable.spliterator java/util/Set.spliterator lib/Spl.spliterator"
    );
    assert_eq!(
        exact("pid-list", "listSpliterator"),
        "java.lang.IncompatibleClassChangeError: Conflicting default methods: java/util/Collection.spliterator java/lang/Iterable.spliterator java/util/List.spliterator lib/Spl.spliterator lib/Spl2.spliterator"
    );
}

/// An abstract declaration beside two defaults is a member of the message
/// too (jar interfaces only).
#[test]
fn conflict_messages_list_abstract_members() {
    assert_eq!(
        exact("pid-three", "threeWay"),
        "java.lang.IncompatibleClassChangeError: Conflicting default methods: lib/PA.who lib/PB.who lib/PC.who"
    );
}

/// `AbstractMap`'s `Map.getOrDefault` conflicts with a jar default, on the
/// class and on a subclass of it.
#[test]
fn a_map_skeletons_default_conflicts() {
    let conflict = "java.lang.IncompatibleClassChangeError: Conflicting default methods: java/util/Map.getOrDefault lib/Getter.getOrDefault";
    assert_eq!(
        exact("pid-map", "mapDefault"),
        format!("{conflict}|{conflict}")
    );
}

/// `Enum`'s and `Record`'s JDK interfaces (`Comparable`, `Constable`,
/// `Serializable`) declare nothing of a jar default's.
#[test]
fn enum_and_record_jar_defaults_run() {
    assert_eq!(exact("pid-enum-record", "enumAndRecord"), "quack|quack");
}

/// An interface's abstract `equals(Object)` (as `Comparator` declares one)
/// leaves `Object.equals` selected: a class's declaration beats every
/// interface's. The runtime used to take `Object` for a class declaring
/// nothing, found the abstract `Named.equals` and threw
/// `AbstractMethodError`.
#[test]
fn an_abstract_object_method_redeclaration_leaves_objects() {
    assert_eq!(exact("pid-object", "objectMethodOnClass"), "true|false");
}

/// Through the interface, the host is asked for an `equals` it does not
/// answer for a jar object yet: counted, no longer an `AbstractMethodError`.
#[test]
fn an_abstract_object_method_redeclaration_through_the_interface_counts() {
    assert_eq!(
        counted(
            "pid-object-interface",
            "objectMethodOnInterface",
            "demo/Probe$Named#equals(Ljava/lang/Object;)Z"
        ),
        "true"
    );
}

/// An unreadable interface above the jar's declaring interface cannot change
/// the selection: exact, nothing counted.
#[test]
fn an_unknown_interface_above_the_declaring_one_stays_exact() {
    assert_eq!(exact("pid-covered", "coveredUnknown"), "covered|covered");
}

/// An unreadable interface beside the jar's default could hold a conflicting
/// default: counted (Java 25 runs `quack` here, but only because `Opaque` is
/// empty, which the runtime cannot see).
#[test]
fn an_unknown_interface_beside_a_jar_default_counts() {
    assert_eq!(
        counted(
            "pid-unknown-jar",
            "unknownBesideJarDefault",
            "demo/Probe$OpaqueQuack#quack()Ljava/lang/String;"
        ),
        "quack"
    );
}

/// Beside a JDK default the same holds: `Iterable.forEach` is selected only
/// if `Opaque` declares no `forEach`.
#[test]
fn an_unknown_interface_beside_a_jdk_default_counts() {
    assert_eq!(
        counted(
            "pid-unknown-jdk",
            "unknownBesideJdkDefault",
            "demo/Probe$OpaqueBag#forEach(Ljava/util/function/Consumer;)V"
        ),
        "ab"
    );
}

/// Beside an abstract-only declaration, `Opaque` could hold the default that
/// runs instead of the `AbstractMethodError`.
#[test]
fn an_unknown_interface_beside_an_abstract_method_counts() {
    assert_eq!(
        counted(
            "pid-unknown-abstract",
            "unknownBesideAbstract",
            "lib/PB#who()Ljava/lang/String;"
        ),
        "java.lang.AbstractMethodError: Receiver class demo.Probe$OpaqueAbstract does not define or inherit an implementation of the resolved method 'abstract java.lang.String who()' of interface lib.PB."
    );
}

/// With no concrete interface method anywhere above the class, HotSpot runs
/// no default-method pass and builds no overpass: the error is the link
/// resolver's, naming the call site's resolved method, which the runtime does
/// not model. Counted, not answered with the overpass text
/// (`Method demo/Probe$OnlyAbstract.who()Ljava/lang/String; is abstract`)
/// it used to give.
#[test]
fn an_abstract_method_without_any_default_counts() {
    assert_eq!(
        counted(
            "pid-abstract-no-defaults",
            "abstractWithoutDefaults",
            "lib/PB#who()Ljava/lang/String;"
        ),
        "java.lang.AbstractMethodError: Receiver class demo.Probe$OnlyAbstract does not define or inherit an implementation of the resolved method 'abstract java.lang.String who()' of interface lib.PB."
    );
}

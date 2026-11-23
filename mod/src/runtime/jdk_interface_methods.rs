//! Java 25's own interfaces as default-method selection reads them (#627).
//!
//! JVMS §5.4.3.3's maximally specific superinterface methods include the JDK's: a jar class
//! that extends `AbstractCollection` and implements a jar interface with
//! `default void forEach(Consumer)` inherits `Iterable.forEach` too, and Java 25 throws
//! `IncompatibleClassChangeError: Conflicting default methods: java/lang/Iterable.forEach
//! lib/Duck.forEach` where the runtime used to run the jar's default. The runtime holds no JDK
//! class files, so `jdk_interface_methods_25.tsv` lists what the selection needs from them:
//!
//! * every public, non-sealed interface of the packages `java.base` exports, and their
//!   superinterfaces transitively, each with its direct superinterfaces in class-file order and
//!   the methods it declares (`abstract`, `default`, `static` or `private`);
//! * the JDK superclasses whose own declarations `jvm_dispatch::jdk_superclass_declares`
//!   knows (`Object`, `Record`, `Enum`, the collection skeletons), with their superclass and
//!   direct interfaces.
//!
//! The file is generated, never edited: `tools/generate_jdk_interface_methods.py` reads the
//! class files of a JDK 25 runtime through `jrt:/`. An interface without a row (a game's, a
//! loader's, another JDK module's) is unknown here, and `jvm_dispatch.rs` counts a selection
//! that depends on it instead of guessing.

use std::collections::HashMap;
use std::sync::OnceLock;

const DATA: &str = include_str!("jdk_interface_methods_25.tsv");

/// How an interface's class file declares a method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `public abstract`: a candidate that runs nothing.
    Abstract,
    /// `public`, with code: a default method.
    Default,
    /// `static` (any access): never a candidate (JVMS §5.4.3.3).
    Static,
    /// `private` instance method: never a candidate.
    Private,
}

/// One JDK interface's row.
pub(crate) struct Interface {
    /// Its direct superinterfaces, in `getInterfaces()` (class-file) order: `HotSpot`'s walk
    /// visits them in this order, and the conflict message lists what it meets in that order.
    pub(crate) superinterfaces: Vec<&'static str>,
    /// `(kind, name + descriptor)` for every method it declares, `<clinit>` and synthetic
    /// lambda bodies left out.
    methods: Vec<(Kind, &'static str)>,
}

impl Interface {
    /// How this interface declares `name descriptor` itself, `None` when it does not.
    pub(crate) fn declares(&self, name: &str, descriptor: &str) -> Option<Kind> {
        self.methods
            .iter()
            .find(|(_, signature)| {
                signature
                    .strip_prefix(name)
                    .is_some_and(|rest| rest == descriptor)
            })
            .map(|(kind, _)| *kind)
    }

    /// Whether it declares a default or a private instance method, of any
    /// name (`HotSpot`'s `declares_nonstatic_concrete_methods`): a class with
    /// such an interface anywhere above it gets `HotSpot`'s default-method
    /// pass, and with it the overpass errors. Synthetic private lambda bodies
    /// are not rows, but an instance one lives only in a concrete method that
    /// is.
    pub(crate) fn has_concrete_instance_method(&self) -> bool {
        self.methods
            .iter()
            .any(|(kind, _)| matches!(kind, Kind::Default | Kind::Private))
    }
}

/// One known JDK superclass's row.
pub(crate) struct Superclass {
    /// Its superclass, `None` for `java/lang/Object`.
    pub(crate) superclass: Option<&'static str>,
    /// Its direct interfaces, in class-file order.
    pub(crate) interfaces: Vec<&'static str>,
}

struct Table {
    /// The generator's first line: the JDK the table was read from.
    #[cfg_attr(not(test), allow(dead_code))]
    identity: &'static str,
    interfaces: HashMap<&'static str, Interface>,
    classes: HashMap<&'static str, Superclass>,
}

fn names(list: &'static str) -> Vec<&'static str> {
    list.split(',').filter(|name| !name.is_empty()).collect()
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut interfaces = HashMap::new();
        let mut classes = HashMap::new();
        for line in DATA.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<&'static str> = line.split('\t').collect();
            match fields.as_slice() {
                ["I", name, superinterfaces, methods] => {
                    let methods = methods
                        .split(' ')
                        .filter(|method| !method.is_empty())
                        .map(|method| {
                            let kind = match method.as_bytes()[0] {
                                b'a' => Kind::Abstract,
                                b'd' => Kind::Default,
                                b's' => Kind::Static,
                                b'p' => Kind::Private,
                                _ => panic!(
                                    "malformed jdk_interface_methods_25.tsv method: {method:?}"
                                ),
                            };
                            (kind, &method[1..])
                        })
                        .collect();
                    interfaces.insert(
                        *name,
                        Interface {
                            superinterfaces: names(superinterfaces),
                            methods,
                        },
                    );
                }
                ["C", name, superclass, implemented] => {
                    classes.insert(
                        *name,
                        Superclass {
                            superclass: (!superclass.is_empty()).then_some(*superclass),
                            interfaces: names(implemented),
                        },
                    );
                }
                _ => panic!("malformed jdk_interface_methods_25.tsv line: {line:?}"),
            }
        }
        Table {
            identity: DATA.lines().next().unwrap_or_default(),
            interfaces,
            classes,
        }
    })
}

/// The row of the JDK interface `name`, `None` when the table does not hold it.
pub(crate) fn interface(name: &str) -> Option<&'static Interface> {
    table().interfaces.get(name)
}

/// The row of the known JDK superclass `name` (`java/util/AbstractList`), `None` for any
/// other class.
pub(crate) fn superclass(name: &str) -> Option<&'static Superclass> {
    table().classes.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows a selection walks are closed: every superinterface and every known
    /// superclass's interface has a row of its own, so the walk never meets an unknown name
    /// inside the JDK's part of a hierarchy.
    #[test]
    fn the_table_is_closed_under_superinterfaces() {
        for (name, row) in &table().interfaces {
            for parent in &row.superinterfaces {
                assert!(
                    interface(parent).is_some(),
                    "{name} extends {parent}, which has no row"
                );
            }
        }
        for (name, row) in &table().classes {
            for parent in &row.interfaces {
                assert!(
                    interface(parent).is_some(),
                    "{name} implements {parent}, which has no row"
                );
            }
            if let Some(parent) = row.superclass {
                assert!(
                    superclass(parent).is_some(),
                    "{name} extends {parent}, which has no row"
                );
            }
        }
    }

    /// Spot checks against `javap` of JDK 25.0.5.
    #[test]
    fn rows_read_java_25() {
        let iterable = interface("java/lang/Iterable").unwrap();
        assert!(iterable.superinterfaces.is_empty());
        assert_eq!(
            iterable.declares("forEach", "(Ljava/util/function/Consumer;)V"),
            Some(Kind::Default)
        );
        assert_eq!(
            iterable.declares("iterator", "()Ljava/util/Iterator;"),
            Some(Kind::Abstract)
        );
        // A name that is a prefix of another (`forEach` / `forEachRemaining`) is not it.
        assert_eq!(iterable.declares("forEach", "()V"), None);
        assert_eq!(
            interface("java/util/List").unwrap().superinterfaces,
            ["java/util/SequencedCollection"]
        );
        assert_eq!(
            interface("java/util/List")
                .unwrap()
                .declares("of", "()Ljava/util/List;"),
            Some(Kind::Static)
        );
        assert_eq!(
            interface("java/util/Map").unwrap().declares(
                "getOrDefault",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"
            ),
            Some(Kind::Default)
        );
        // `Comparator.equals` is an abstract redeclaration of `Object.equals`.
        assert_eq!(
            interface("java/util/Comparator")
                .unwrap()
                .declares("equals", "(Ljava/lang/Object;)Z"),
            Some(Kind::Abstract)
        );
        let enum_row = superclass("java/lang/Enum").unwrap();
        assert_eq!(enum_row.superclass, Some("java/lang/Object"));
        assert_eq!(
            enum_row.interfaces,
            [
                "java/lang/constant/Constable",
                "java/lang/Comparable",
                "java/io/Serializable"
            ]
        );
        assert_eq!(superclass("java/lang/Object").unwrap().superclass, None);
        // Sealed interfaces are not seeds (no class-path class can implement them), and a
        // game's interface is never a row.
        assert!(interface("java/lang/constant/ConstantDesc").is_none());
        assert!(interface("net/minecraft/world/level/ItemLike").is_none());
    }

    /// The superclasses with rows are exactly the ones `jdk_superclass_declares` answers for:
    /// `Object`, `Record`, `Enum` and every `JDK_SKELETONS` class.
    #[test]
    fn superclass_rows_are_the_known_superclasses() {
        let mut expected: Vec<&str> = crate::runtime::jvm_dispatch::JDK_SKELETONS
            .iter()
            .map(|(class, _, _)| *class)
            .chain(["java/lang/Object", "java/lang/Record", "java/lang/Enum"])
            .collect();
        expected.sort_unstable();
        let mut rows: Vec<&str> = table().classes.keys().copied().collect();
        rows.sort_unstable();
        assert_eq!(rows, expected);
        for (class, superclass_name, _) in crate::runtime::jvm_dispatch::JDK_SKELETONS {
            assert_eq!(
                superclass(class).unwrap().superclass,
                Some(*superclass_name)
            );
        }
    }

    /// Rerun the generator when the JDK at hand is the one the table records: its output must
    /// be the committed file, byte for byte.
    #[test]
    fn the_table_is_the_generators_output() {
        let java_home = std::env::var_os("JAVA_HOME").map(std::path::PathBuf::from);
        let java = java_home
            .as_ref()
            .map(|home| home.join("bin/java"))
            .unwrap_or_else(|| "java".into());
        let Ok(version) = std::process::Command::new(&java)
            .args(["-XshowSettings:properties", "-version"])
            .output()
        else {
            eprintln!("skipped: no java to run the generator");
            return;
        };
        let settings = String::from_utf8_lossy(&version.stderr);
        let property = |key: &str| {
            settings
                .lines()
                .find_map(|line| line.trim().strip_prefix(key)?.strip_prefix(" = "))
                .unwrap_or_default()
                .to_owned()
        };
        let here = format!(
            "# {} {} ",
            property("java.vm.vendor"),
            property("java.runtime.version")
        );
        if !table().identity.starts_with(&here) {
            eprintln!(
                "skipped: the table records {:?}, this JDK is {here:?}",
                table().identity
            );
            return;
        }
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let Some(generator) = [
            manifest.join("../../tools/generate_jdk_interface_methods.py"),
            manifest.join("../tools/generate_jdk_interface_methods.py"),
        ]
        .into_iter()
        .find(|path| path.is_file()) else {
            eprintln!("skipped: tools/generate_jdk_interface_methods.py is not beside this crate");
            return;
        };
        let output = std::env::temp_dir().join(format!(
            "jdk_interface_methods_25-{}.tsv",
            std::process::id()
        ));
        let mut command = std::process::Command::new("python3");
        command.arg(&generator).arg("--output").arg(&output);
        if let Some(home) = &java_home {
            command.arg("--jdk").arg(home);
        }
        let Ok(status) = command.status() else {
            eprintln!("skipped: no python3 to run the generator");
            return;
        };
        assert!(status.success(), "the generator failed");
        let generated = std::fs::read_to_string(&output).unwrap();
        let _ = std::fs::remove_file(&output);
        assert!(
            generated == DATA,
            "jdk_interface_methods_25.tsv is not the generator's output"
        );
    }
}

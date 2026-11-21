//! Fabric Loader's `ModOrigin`, compiled from real Java at test time.
//!
//! The stub API classes declare the loader signatures a mod compiles against;
//! the runtime answers them from its own model. The expected values below are
//! Fabric Loader 0.19.5's own `ModOriginImpl` on a real JVM (javac 25,
//! `--release 17`, `fabric-loader-0.19.5.jar` on the class path):
//! The real Java 25 oracle command was javac -cp fabric-loader-0.19.5.jar ModOriginOracle.java followed by java -cp .:fabric-loader-0.19.5.jar ModOriginOracle.
//!
//! ```text
//! enum=[PATH|0|PATH, NESTED|1|NESTED, UNKNOWN|2|UNKNOWN]
//! valueOf=[PATH=true, NESTED=true, UNKNOWN=true]
//! valueOf-bad=[java.lang.IllegalArgumentException|No enum constant net.fabricmc.loader.api.metadata.ModOrigin.Kind.NOPE]
//! valueOf-null=[java.lang.NullPointerException|Name is null]
//! path=[PATH|/horizon/mods/origin-fixture|0]
//! paths=[1|/horizon/mods/origin-fixture]
//! path-parent=[uoe:kind PATH doesn't have a parent mod]
//! path-sub=[uoe:kind PATH doesn't have a parent sub-location]
//! nested=[NESTED|parent-mod:sub.jar|parent-mod|sub.jar]
//! nested-paths=[uoe:kind NESTED doesn't have paths]
//! unknown=[UNKNOWN|unknown]
//! unknown-paths=[uoe:kind UNKNOWN doesn't have paths]
//! ```
//!
//! The path is the runtime's virtual mod root (the same value
//! `ModContainer.getRootPaths()` answers), not the jar's host path; the
//! runtime never exposes host paths.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{NativeError, NativeJar, StdHost, Value};
use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::write::SimpleFileOptions;

const FABRIC_LOADER: &str = r#"package net.fabricmc.loader.api;

public interface FabricLoader {
    static FabricLoader getInstance() { return null; }
    java.util.Optional<ModContainer> getModContainer(String id);
}
"#;

const MOD_CONTAINER: &str = r#"package net.fabricmc.loader.api;

import net.fabricmc.loader.api.metadata.ModOrigin;

public interface ModContainer {
    ModOrigin getOrigin();
}
"#;

const MOD_ORIGIN: &str = r#"package net.fabricmc.loader.api.metadata;

import java.nio.file.Path;
import java.util.List;

public interface ModOrigin {
    Kind getKind();
    List<Path> getPaths();
    String getParentModId();
    String getParentSubLocation();

    enum Kind {
        PATH,
        NESTED,
        UNKNOWN
    }
}
"#;

const PROBE: &str = r#"package demo;

import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;
import net.fabricmc.loader.api.metadata.ModOrigin;

public final class Probe {
    static ModOrigin origin(String id) {
        ModContainer container = FabricLoader.getInstance().getModContainer(id).orElseThrow();
        return container.getOrigin();
    }

    public static String path() {
        ModOrigin origin = origin("origin-fixture");
        return origin.getKind().name() + "|" + origin + "|" + origin.getKind().ordinal();
    }

    public static String paths() {
        ModOrigin origin = origin("origin-fixture");
        return origin.getPaths().size() + "|" + origin.getPaths().get(0);
    }

    public static String parentMod() {
        try {
            return origin("origin-fixture").getParentModId();
        } catch (UnsupportedOperationException error) {
            return "uoe:" + error.getMessage();
        }
    }

    public static String parentSub() {
        try {
            return origin("origin-fixture").getParentSubLocation();
        } catch (UnsupportedOperationException error) {
            return "uoe:" + error.getMessage();
        }
    }

    // Another container's origin is its own object, so the two are not equal,
    // and one container's origin is equal to itself.
    public static String identity() {
        ModOrigin first = origin("origin-fixture");
        ModOrigin again = origin("origin-fixture");
        ModOrigin other = origin("minecraft");
        return first.equals(again) + "|" + first.equals(other) + "|" + other;
    }

    public static String kindEnum() {
        ModOrigin.Kind[] first = ModOrigin.Kind.values();
        ModOrigin.Kind[] second = ModOrigin.Kind.values();
        return first.length + "|" +
            first[0].name() + ":" + first[0].ordinal() + ":" + first[0].toString() + "|" +
            first[1].name() + ":" + first[1].ordinal() + ":" + first[1].toString() + "|" +
            first[2].name() + ":" + first[2].ordinal() + ":" + first[2].toString() + "|" +
            (first != second) + "|" +
            (first[0] == ModOrigin.Kind.PATH) + "|" +
            (first[0] == ModOrigin.Kind.valueOf("PATH")) + "|" +
            (first[1] == ModOrigin.Kind.valueOf("NESTED")) + "|" +
            (first[2] == ModOrigin.Kind.valueOf("UNKNOWN"));
    }

    public static String nested() {
        ModOrigin origin = origin("nested-child");
        return (origin.getKind() == ModOrigin.Kind.NESTED) + "|" +
            origin.getKind().name() + "|" + origin.getParentModId() + "|" +
            origin.getParentSubLocation() + "|" + origin;
    }

    public static String kindDeclaringClass() {
        return ModOrigin.Kind.PATH.getDeclaringClass().getName();
    }

    public static String kindValueOfFailures() {
        String bad;
        try {
            ModOrigin.Kind.valueOf("NOPE");
            bad = "ok";
        } catch (IllegalArgumentException error) {
            bad = "iae:" + error.getMessage();
        }
        String nil;
        try {
            ModOrigin.Kind.valueOf(null);
            nil = "ok";
        } catch (NullPointerException error) {
            nil = "npe:" + error.getMessage();
        }
        return bad + "|" + nil;
    }
}
"#;

fn javac_path() -> PathBuf {
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

struct Fixture {
    root: PathBuf,
    jar: NativeJar,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn build(test: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = scratch::unique_path(&format!(
            "horizon-native-java-mod-origin-{test}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let mut fixture = Self {
            root,
            jar: NativeJar::default(),
        };
        let classes = fixture.root.join("classes");
        let sources = [
            ("net/fabricmc/loader/api/FabricLoader.java", FABRIC_LOADER),
            ("net/fabricmc/loader/api/ModContainer.java", MOD_CONTAINER),
            (
                "net/fabricmc/loader/api/metadata/ModOrigin.java",
                MOD_ORIGIN,
            ),
            ("demo/Probe.java", PROBE),
        ];
        let mut javac = Command::new(javac_path());
        javac.args(["--release", "17", "-d"]).arg(&classes);
        for (path, source) in sources {
            let file = fixture.root.join("src").join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, source).unwrap();
            javac.arg(file);
        }
        let status = javac.status().unwrap();
        assert!(
            status.success(),
            "javac failed for mod origin fixture {test}"
        );

        let jar_path = fixture.root.join("mod-origin.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            // Fabric's ModDiscoverer loads only the nested jars a mod lists in
            // `jars`, so the child is declared exactly as a real mod declares it.
            br#"{"id":"origin-fixture","version":"1.0","entrypoints":{"main":"demo.Probe"},"jars":[{"file":"META-INF/jars/nested-child.jar"}]}"#,
        )
        .unwrap();
        let mut nested_bytes = Cursor::new(Vec::new());
        {
            let mut nested = zip::ZipWriter::new(&mut nested_bytes);
            nested
                .start_file("fabric.mod.json", SimpleFileOptions::default())
                .unwrap();
            nested
                .write_all(br#"{"id":"nested-child","version":"1.0"}"#)
                .unwrap();
            nested.finish().unwrap();
        }
        zip.start_file(
            "META-INF/jars/nested-child.jar",
            SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(nested_bytes.get_ref()).unwrap();
        for class in files_under(&classes) {
            let entry = class
                .strip_prefix(&classes)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            // Only the mod's own classes are packaged: the loader API stubs
            // are the compile-time surface, and the runtime answers the calls
            // from its own model (a packaged stub would shadow it).
            if !entry.starts_with("demo/") {
                continue;
            }
            zip.start_file(entry, SimpleFileOptions::default()).unwrap();
            zip.write_all(&fs::read(class).unwrap()).unwrap();
        }
        zip.finish().unwrap();

        fixture.jar = NativeJar::from_path(&jar_path).unwrap();
        fixture
    }

    fn call(&self, method: &str) -> Result<Value, NativeError> {
        // Seed the jar and link its META-INF/jars child exactly as the loader
        // does before invoking an entrypoint.
        let mut host = self.jar.seed_host(StdHost::new());
        self.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
    }

    fn text(&self, method: &str) -> String {
        match self.call(method).unwrap() {
            Value::Ref(Some(text)) => text,
            other => panic!("{method} did not return a string: {other:?}"),
        }
    }
}

#[test]
fn a_jar_container_origin_is_a_path_origin_over_its_root() {
    let fixture = Fixture::build("path");
    // java: PATH|/horizon/mods/origin-fixture|0
    assert_eq!(fixture.text("path"), "PATH|/horizon/mods/origin-fixture|0");
    // java: 1|/horizon/mods/origin-fixture
    assert_eq!(fixture.text("paths"), "1|/horizon/mods/origin-fixture");
}

#[test]
fn parent_accessors_throw_what_fabric_throws_for_a_path_origin() {
    let fixture = Fixture::build("parent");
    // java: uoe:kind PATH doesn't have a parent mod
    assert_eq!(
        fixture.text("parentMod"),
        "uoe:kind PATH doesn't have a parent mod"
    );
    // java: uoe:kind PATH doesn't have a parent sub-location
    assert_eq!(
        fixture.text("parentSub"),
        "uoe:kind PATH doesn't have a parent sub-location"
    );
}

#[test]
fn mod_origin_kind_is_the_loader_enum_with_fresh_values_arrays() {
    let fixture = Fixture::build("kind-enum");
    assert_eq!(
        fixture.text("kindEnum"),
        "3|PATH:0:PATH|NESTED:1:NESTED|UNKNOWN:2:UNKNOWN|true|true|true|true|true"
    );
    assert_eq!(
        fixture.text("kindValueOfFailures"),
        "iae:No enum constant net.fabricmc.loader.api.metadata.ModOrigin.Kind.NOPE|npe:Name is null"
    );
}

#[test]
fn mod_origin_kind_declaring_class_matches_java() {
    let fixture = Fixture::build("kind-declaring-class");
    assert_eq!(
        fixture.text("kindDeclaringClass"),
        "net.fabricmc.loader.api.metadata.ModOrigin$Kind"
    );
}

#[test]
fn nested_container_origin_is_a_nested_origin_over_its_entry() {
    let fixture = Fixture::build("nested");
    assert_eq!(
        fixture.text("nested"),
        "true|NESTED|origin-fixture|META-INF/jars/nested-child.jar|origin-fixture:META-INF/jars/nested-child.jar"
    );
}

#[test]
fn each_container_has_its_own_origin() {
    let fixture = Fixture::build("identity");
    // java: true|false|/horizon/mods/minecraft
    assert_eq!(
        fixture.text("identity"),
        "true|false|/horizon/mods/minecraft"
    );
}

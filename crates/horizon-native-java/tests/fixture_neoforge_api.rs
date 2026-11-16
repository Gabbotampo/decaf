//! Java 25 oracle for the small NeoForge loader/API slice.
//!
//! The NeoForge classes below are compile-only stubs.  The jar contains only
//! `demo.Probe`, so every API call reaches the native model just as it does
//! when the real 26.3 FML/NeoForge jars are supplied by the game.

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use zip::write::SimpleFileOptions;

#[path = "support/scratch.rs"]
mod scratch;

const STUBS: &[(&str, &str)] = &[
    (
        "net/neoforged/fml/common/Mod.java",
        "package net.neoforged.fml.common; import java.lang.annotation.Retention; import java.lang.annotation.RetentionPolicy; @Retention(RetentionPolicy.RUNTIME) public @interface Mod { String value(); }",
    ),
    (
        "net/neoforged/fml/ModContainer.java",
        "package net.neoforged.fml; import net.neoforged.bus.api.IEventBus; import net.neoforged.neoforgespi.language.IModInfo; public class ModContainer { public String getModId(){return null;} public String getNamespace(){return null;} public IModInfo getModInfo(){return null;} public IEventBus getEventBus(){return null;} }",
    ),
    (
        "net/neoforged/bus/api/IEventBus.java",
        "package net.neoforged.bus.api; public interface IEventBus {}",
    ),
    (
        "net/neoforged/neoforgespi/language/IModInfo.java",
        "package net.neoforged.neoforgespi.language; public interface IModInfo { String getModId(); }",
    ),
    (
        "net/neoforged/fml/ModLoadingContext.java",
        "package net.neoforged.fml; public class ModLoadingContext { public static ModLoadingContext get(){return null;} public ModContainer getActiveContainer(){return null;} public String getActiveNamespace(){return null;} }",
    ),
    (
        "net/neoforged/fml/loading/FMLLoader.java",
        "package net.neoforged.fml.loading; public class FMLLoader { public static boolean isProduction(){return false;} public static java.nio.file.Path getGameDir(){return null;} }",
    ),
    (
        "net/neoforged/neoforge/data/loading/DatagenModLoader.java",
        "package net.neoforged.neoforge.data.loading; public class DatagenModLoader { public static boolean isRunningDataGen(){return true;} }",
    ),
    (
        "net/neoforged/neoforge/network/event/RegisterPayloadHandlersEvent.java",
        "package net.neoforged.neoforge.network.event; import net.neoforged.neoforge.network.registration.PayloadRegistrar; public class RegisterPayloadHandlersEvent { public PayloadRegistrar registrar(String v){return null;} }",
    ),
    (
        "net/neoforged/neoforge/network/registration/PayloadRegistrar.java",
        "package net.neoforged.neoforge.network.registration; public class PayloadRegistrar { public PayloadRegistrar versioned(String v){return null;} public PayloadRegistrar optional(){return null;} }",
    ),
];

const ORACLE: &str = include_str!("fixtures/neoforge_api_slice/java25.txt");

fn expected(name: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("missing oracle row {name}"))
}

const PROBE: &str = r#"package demo;

import net.neoforged.fml.ModContainer;
import net.neoforged.fml.ModLoadingContext;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.data.loading.DatagenModLoader;
import net.neoforged.fml.loading.FMLLoader;
import net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent;
import net.neoforged.neoforge.network.registration.PayloadRegistrar;

@Mod("demo")
public final class Probe {
    private static String active;

    public Probe() {
        ModLoadingContext context = ModLoadingContext.get();
        ModContainer container = context.getActiveContainer();
        PayloadRegistrar registrar = new RegisterPayloadHandlersEvent().registrar("1");
        Object object = registrar;
        PayloadRegistrar refined = registrar.versioned("2").optional();
        active = container.getModId() + "|" + container.getNamespace()
            + "|" + (container.getEventBus() != null)
            + "|" + container.getModInfo().getModId()
            + "|" + context.getActiveNamespace()
            + "|" + FMLLoader.isProduction() + "|" + !DatagenModLoader.isRunningDataGen()
            + "|" + (object instanceof PayloadRegistrar) + "|" + (registrar == refined)
            + "|" + (registrar != refined);
    }

    public static String active() { return active; }

    public static String outside() {
        ModLoadingContext context = ModLoadingContext.get();
        return context.getActiveContainer().getModId() + "|"
            + context.getActiveContainer().getNamespace() + "|"
            + (context.getActiveContainer().getEventBus() == null) + "|"
            + context.getActiveContainer().getModInfo().getModId() + "|"
            + context.getActiveNamespace();
    }

    public static String contextIdentity() {
        return String.valueOf(ModLoadingContext.get() == ModLoadingContext.get());
    }

    public static ModLoadingContext context() { return ModLoadingContext.get(); }

    public static String dataGen() {
        return String.valueOf(DatagenModLoader.isRunningDataGen());
    }

    public static String gameDir() { return FMLLoader.getGameDir().toString(); }
}
"#;

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

fn text(jar: &NativeJar, host: &mut StdHost, method: &str) -> String {
    match jar
        .invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            host,
        )
        .unwrap()
    {
        Value::Ref(Some(value)) => value,
        other => panic!("{method}: expected text, got {other:?}"),
    }
}

#[test]
fn neoforge_loader_and_payload_handles_match_java25_oracle() {
    let root = scratch::Scratch::new("neoforge-api");
    let src = root.join("src");
    let classes = root.join("classes");
    for (relative, body) in STUBS {
        let path = src.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    let probe = src.join("demo/Probe.java");
    fs::create_dir_all(probe.parent().unwrap()).unwrap();
    fs::write(&probe, PROBE).unwrap();
    let sources = files_under(&src);
    fs::create_dir_all(&classes).unwrap();
    let status = Command::new(javac())
        .args(["--release", "21", "-d"])
        .arg(&classes)
        .args(&sources)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed");

    let jar_path = root.join("neoforge-api.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    zip.start_file("META-INF/neoforge.mods.toml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"[[mods]]\nmodId = \"demo\"\nversion = \"1.0\"\n")
        .unwrap();
    for class in files_under(&classes.join("demo")) {
        let name = class
            .strip_prefix(&classes)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();

    let jar = NativeJar::from_path(&jar_path).unwrap();
    let mut host = jar.seed_host(StdHost::new().with_mod("demo", "1.0"));
    jar.execute("demo.Probe", None, "neoforge", &mut host)
        .unwrap();
    assert_eq!(text(&jar, &mut host, "active"), expected("active"));
    assert_eq!(text(&jar, &mut host, "outside"), expected("outside"));
    assert_eq!(
        text(&jar, &mut host, "contextIdentity"),
        expected("contextIdentity")
    );
    assert_eq!(
        text(&jar, &mut host, "dataGen"),
        expected("dataGen(normal)")
    );
    assert_eq!(text(&jar, &mut host, "gameDir"), expected("gameDir"));

    let mut datagen = jar.seed_host(StdHost::new().with_mod("demo", "1.0").with_data_gen(true));
    assert_eq!(
        text(&jar, &mut datagen, "dataGen"),
        expected("dataGen(data-gen)")
    );
    let mut unknown_mode = jar.seed_host(
        StdHost::new()
            .with_mod("demo", "1.0")
            .without_data_gen_mode(),
    );
    assert_eq!(text(&jar, &mut unknown_mode, "dataGen"), "false");
    assert!(
        unknown_mode
            .unknown_host_calls()
            .iter()
            .any(|(call, count)| call.contains("DatagenModLoader#isRunningDataGen") && *count == 1)
    );

    // FML's context is a ThreadLocal: equal reads on one native thread share
    // an object, while a context obtained on another thread is different.
    let jar = Arc::new(jar);
    let first = jar
        .invoke_method(
            "demo/Probe",
            "context",
            "()Lnet/neoforged/fml/ModLoadingContext;",
            None,
            &mut host,
        )
        .unwrap();
    let other_jar = Arc::clone(&jar);
    let other = std::thread::spawn(move || {
        let mut other_host = other_jar.seed_host(StdHost::new().with_mod("demo", "1.0"));
        other_jar
            .invoke_method(
                "demo/Probe",
                "context",
                "()Lnet/neoforged/fml/ModLoadingContext;",
                None,
                &mut other_host,
            )
            .unwrap()
    })
    .join()
    .unwrap();
    assert_ne!(first, other);
    assert!(
        host.unknown_host_calls().is_empty(),
        "modelled calls were counted: {:?}",
        host.unknown_host_calls()
    );
}

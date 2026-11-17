#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{Host, MethodRef, RunLoader, StdHost};
use javac_fixture::Fixture;
const WORLD: &str = r#"package net.minecraft; public interface WorldVersion { String name(); String id(); int protocolVersion(); boolean stable(); }"#;
const SHARED: &str = r#"package net.minecraft; public final class SharedConstants { public static void tryDetectVersion(){} public static WorldVersion getCurrentVersion(){throw new AssertionError();} }"#;
const PROBE: &str = include_str!("fixtures/shared_constants_version/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/shared_constants_version/java25.txt");
#[test]
fn rows_match_java25() {
    let f = Fixture::build(
        "shared-constants-version",
        &[
            ("net/minecraft/WorldVersion.java", WORLD),
            ("net/minecraft/SharedConstants.java", SHARED),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut h = f.jar.seed_host(
        StdHost::new()
            .with_run_loader(RunLoader::NeoForge)
            .with_game_version("26.3"),
    );
    assert_eq!(f.text_on(&mut h, "rows"), ORACLE);
    assert!(h.unknown_host_calls().is_empty());
}
#[test]
fn unmodelled_world_version_method_is_counted() {
    let f = Fixture::build(
        "shared-constants-version-counted",
        &[
            ("net/minecraft/WorldVersion.java", WORLD),
            ("net/minecraft/SharedConstants.java", SHARED),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut h = f.jar.seed_host(
        StdHost::new()
            .with_run_loader(RunLoader::NeoForge)
            .with_game_version("26.3"),
    );
    let v = h
        .invoke(
            &MethodRef {
                owner: "net/minecraft/SharedConstants".into(),
                name: "getCurrentVersion".into(),
                descriptor: "()Lnet/minecraft/WorldVersion;".into(),
            },
            &[],
        )
        .unwrap();
    let _ = h.invoke_instance(
        &MethodRef {
            owner: "net/minecraft/WorldVersion".into(),
            name: "id".into(),
            descriptor: "()Ljava/lang/String;".into(),
        },
        &v,
        &[],
    );
    assert_eq!(h.unknown_host_calls().len(), 1);
}

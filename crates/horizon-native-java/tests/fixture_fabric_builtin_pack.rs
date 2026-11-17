//! Fabric's builtin pack registration is static and delegates to ModNioPackResources.create.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{MethodRef, StdHost, Value};
use javac_fixture::Fixture;
const JAVAP: &str = include_str!("fixtures/fabric_builtin_pack/javap_facts.txt");
const PROBE: &str = include_str!("fixtures/fabric_builtin_pack/demo/Probe.java");
const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/fabric/api/resource/v1/ResourceLoader.java",
        "package net.fabricmc.fabric.api.resource.v1; public interface ResourceLoader { static boolean registerBuiltinPack(net.minecraft.resources.Identifier i, net.fabricmc.loader.api.ModContainer c, net.minecraft.network.chat.Component d, net.fabricmc.fabric.api.resource.v1.pack.PackActivationType a){return false;} }",
    ),
    (
        "net/fabricmc/fabric/api/resource/v1/pack/PackActivationType.java",
        "package net.fabricmc.fabric.api.resource.v1.pack; public class PackActivationType {}",
    ),
    (
        "net/fabricmc/loader/api/FabricLoader.java",
        "package net.fabricmc.loader.api; public class FabricLoader { public static FabricLoader getInstance(){return null;} public java.util.Optional<ModContainer> getModContainer(String id){return null;} }",
    ),
    (
        "net/fabricmc/loader/api/ModContainer.java",
        "package net.fabricmc.loader.api; public interface ModContainer {}",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources; public class Identifier { public static Identifier parse(String s){return null;} public String getNamespace(){return null;} public String getPath(){return null;} }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat; public interface Component {}",
    ),
];
#[test]
fn builtin_pack_registration_matches_javap_facts() {
    assert!(JAVAP.contains("invokestatic registerBuiltinPack:(Identifier,String,ModContainer,Component,PackActivationType)Z"));
    assert!(JAVAP.contains("invokestatic exists:(Path)Z"));
    assert!(JAVAP.contains("iconst_0"));
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    let fixture = Fixture::build_with_lib(
        "fabric-builtin-pack",
        &sources,
        &[("resourcepacks/demo/present/pack.mcmeta", b"{}")],
    );
    let mut host = fixture.jar.seed_host(
        StdHost::new()
            .with_mod("demo", "1")
            .with_resource("resourcepacks/demo/present/pack.mcmeta", "pack"),
    );
    assert_eq!(
        fixture.text_on(&mut host, "rows"),
        "present=true\nabsent=false\n"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
    let method = MethodRef { owner: "net/fabricmc/fabric/api/resource/v1/ResourceLoader".into(), name: "registerBuiltinPack".into(), descriptor: "(Lnet/minecraft/resources/Identifier;Lnet/fabricmc/loader/api/ModContainer;Lnet/minecraft/network/chat/Component;Lnet/fabricmc/fabric/api/resource/v1/pack/PackActivationType;)Z".into() };
    let id = host
        .invoke(
            &MethodRef {
                owner: "net/minecraft/resources/Identifier".into(),
                name: "parse".into(),
                descriptor: "(Ljava/lang/String;)Lnet/minecraft/resources/Identifier;".into(),
            },
            &[Value::Ref(Some("demo:present".into()))],
        )
        .unwrap();
    let _ = host.invoke(
        &method,
        &[
            id,
            Value::Ref(Some("unknown".into())),
            Value::Ref(None),
            Value::Ref(None),
        ],
    );
    assert_eq!(host.unknown_host_calls(), vec![(method.signature(), 1)]);
}

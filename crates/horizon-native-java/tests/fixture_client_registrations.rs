//! Java 25 fixture for client registration-only APIs.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const API_STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources; public class Identifier { public static Identifier withDefaultNamespace(String s) { throw new AssertionError(); } }"#,
    ),
    (
        "net/minecraft/core/particles/ParticleType.java",
        r#"package net.minecraft.core.particles; public class ParticleType<T> {}"#,
    ),
    (
        "net/minecraft/core/particles/SimpleParticleType.java",
        r#"package net.minecraft.core.particles; public class SimpleParticleType extends ParticleType<Object> {}"#,
    ),
    (
        "net/minecraft/client/particle/ParticleProvider.java",
        r#"package net.minecraft.client.particle; public interface ParticleProvider<T> {}"#,
    ),
    (
        "net/fabricmc/fabric/api/particle/v1/FabricParticleTypes.java",
        r#"package net.fabricmc.fabric.api.particle.v1; public final class FabricParticleTypes { public static net.minecraft.core.particles.SimpleParticleType simple() { throw new AssertionError(); } public static <T> net.minecraft.core.particles.ParticleType<T> complex(Object a, Object b) { throw new AssertionError(); } }"#,
    ),
    (
        "net/fabricmc/fabric/api/client/particle/v1/ParticleProviderRegistry.java",
        r#"package net.fabricmc.fabric.api.client.particle.v1; public interface ParticleProviderRegistry { static ParticleProviderRegistry getInstance() { throw new AssertionError(); } void register(net.minecraft.core.particles.ParticleType<?> t, net.minecraft.client.particle.ParticleProvider<?> p); }"#,
    ),
    (
        "net/minecraft/client/gui/components/debug/DebugScreenEntry.java",
        r#"package net.minecraft.client.gui.components.debug; public interface DebugScreenEntry {}"#,
    ),
    (
        "net/minecraft/client/gui/components/debug/DebugScreenEntries.java",
        r#"package net.minecraft.client.gui.components.debug; public class DebugScreenEntries { public static net.minecraft.resources.Identifier register(net.minecraft.resources.Identifier i, DebugScreenEntry e) { throw new AssertionError(); } public static final net.minecraft.resources.Identifier FPS = null; }"#,
    ),
    (
        "net/minecraft/client/gui/components/debug/DebugScreenProfile.java",
        r#"package net.minecraft.client.gui.components.debug; public enum DebugScreenProfile { DEFAULT, PERFORMANCE }"#,
    ),
    (
        "net/minecraft/client/gui/screens/MenuScreens.java",
        r#"package net.minecraft.client.gui.screens; public class MenuScreens { public static void register(Object menu, Object constructor) { throw new AssertionError(); } }"#,
    ),
    (
        "net/fabricmc/fabric/api/client/rendering/v1/hud/VanillaHudElements.java",
        r#"package net.fabricmc.fabric.api.client.rendering.v1.hud; public final class VanillaHudElements { public static final net.minecraft.resources.Identifier CHAT = null; }"#,
    ),
    (
        "net/minecraft/client/model/geom/ModelLayerLocation.java",
        r#"package net.minecraft.client.model.geom; public class ModelLayerLocation { public ModelLayerLocation(Object id, String part) {} }"#,
    ),
    (
        "net/fabricmc/fabric/api/client/rendering/v1/ModelLayerRegistry.java",
        r#"package net.fabricmc.fabric.api.client.rendering.v1; public final class ModelLayerRegistry { public static void registerModelLayer(net.minecraft.client.model.geom.ModelLayerLocation l, Object p) { throw new AssertionError(); } }"#,
    ),
    (
        "net/fabricmc/fabric/api/client/rendering/v1/EntityRendererRegistry.java",
        r#"package net.fabricmc.fabric.api.client.rendering.v1; public final class EntityRendererRegistry { public static void register(Object e, Object p) { throw new AssertionError(); } }"#,
    ),
    (
        "net/fabricmc/fabric/api/client/model/loading/v1/ModelLoadingPlugin.java",
        r#"package net.fabricmc.fabric.api.client.model.loading.v1; public interface ModelLoadingPlugin { static void register(ModelLoadingPlugin p) { throw new AssertionError(); } }"#,
    ),
];

const PROBE: &str = include_str!("fixtures/client_registrations/Probe.java");

#[test]
fn client_registration_surfaces_keep_java25_identity_and_order() {
    let mut sources = API_STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    let fixture = Fixture::build("client-registrations", &sources);
    let mut host = fixture
        .jar
        .seed_host(StdHost::new().with_environment_type("CLIENT"));
    assert_eq!(fixture.text_on(&mut host, "identities"), "false|true|true|true");
    assert_eq!(
        fixture.text_on(&mut host, "duplicates"),
        "IllegalArgumentException"
    );
    assert_eq!(fixture.text_on(&mut host, "fields"), "true|true");
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

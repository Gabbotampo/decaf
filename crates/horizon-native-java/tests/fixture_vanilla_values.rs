//! Vanilla value families behind spell_engine: static tags, MobEffectCategory,
//! and stateful RangedAttribute handles, checked against Java 25 values.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const JAVA25: &[u8] = include_bytes!("fixtures/vanilla_values/java25.txt");

const SOURCES: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources; public final class Identifier { public static Identifier withDefaultNamespace(String x) { return null; } public String toString() { return null; } }"#,
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        r#"package net.minecraft.resources; public class ResourceKey<T> { public Identifier identifier() { return null; } public boolean isFor(ResourceKey<?> x) { return false; } }"#,
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        r#"package net.minecraft.core.registries; import net.minecraft.resources.ResourceKey; public final class Registries { public static final ResourceKey<?> ENTITY_TYPE = null; public static final ResourceKey<?> ITEM = null; }"#,
    ),
    (
        "net/minecraft/tags/TagKey.java",
        r#"package net.minecraft.tags; import net.minecraft.resources.Identifier; import net.minecraft.resources.ResourceKey; public final class TagKey<T> { public Identifier location() { return null; } public ResourceKey<?> registry() { return null; } public boolean isFor(ResourceKey<?> x) { return false; } }"#,
    ),
    (
        "net/minecraft/tags/EntityTypeTags.java",
        r#"package net.minecraft.tags; import net.minecraft.world.entity.EntityType; public interface EntityTypeTags { TagKey<EntityType<?>> FREEZE_IMMUNE_ENTITY_TYPES = null; }"#,
    ),
    (
        "net/minecraft/world/entity/EntityType.java",
        r#"package net.minecraft.world.entity; public class EntityType<T> {}"#,
    ),
    (
        "net/minecraft/ChatFormatting.java",
        r#"package net.minecraft; public enum ChatFormatting { BLUE, RED }"#,
    ),
    (
        "net/minecraft/world/effect/MobEffectCategory.java",
        r#"package net.minecraft.world.effect; import net.minecraft.ChatFormatting; public enum MobEffectCategory { BENEFICIAL, HARMFUL, NEUTRAL; public ChatFormatting getTooltipFormatting() { return null; } }"#,
    ),
    (
        "net/minecraft/world/entity/ai/attributes/Attribute.java",
        r#"package net.minecraft.world.entity.ai.attributes; public class Attribute { protected Attribute(String id, double value) {} public double getDefaultValue() { return 0; } public boolean isClientSyncable() { return false; } public Attribute setSyncable(boolean x) { return this; } public double sanitizeValue(double x) { return x; } public String getDescriptionId() { return null; } }"#,
    ),
    (
        "net/minecraft/world/entity/ai/attributes/RangedAttribute.java",
        r#"package net.minecraft.world.entity.ai.attributes; public class RangedAttribute extends Attribute { public RangedAttribute(String id, double d, double min, double max) { super(id, d); } public double getMinValue() { return 0; } public double getMaxValue() { return 0; } public double sanitizeValue(double x) { return x; } }"#,
    ),
    (
        "demo/Probe.java",
        include_str!("fixtures/vanilla_values/demo/Probe.java"),
    ),
];

fn fixture() -> Fixture {
    Fixture::build("vanilla-values", SOURCES)
}

#[test]
fn vanilla_values_match_java_25_and_forged_marker_stays_string() {
    let fixture = fixture();
    let oracle = std::str::from_utf8(JAVA25).expect("java25.txt is UTF-8");
    for method in ["tags", "categories", "attrs", "invalid", "attrsEdges"] {
        let prefix = format!("{method}=");
        let expected = oracle
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("missing {method} row in java25.txt"));
        let mut host = fixture.jar.seed_host(StdHost::new());
        let got = fixture.text_on(&mut host, method);
        assert_eq!(got, expected, "{method}");
        let unknown = host.unknown_host_calls();
        if method == "tags" {
            assert!(
                unknown
                    .iter()
                    .all(|(call, _)| call.ends_with("#<instanceof>(Ljava/lang/Object;)Z")),
                "{method} counted {unknown:?}"
            );
        } else {
            assert!(unknown.is_empty(), "{method} counted {unknown:?}");
        }
    }
}

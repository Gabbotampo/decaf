//! Vanilla value families behind spell_engine: static tags, MobEffectCategory,
//! and stateful RangedAttribute handles, checked against Java 25 values.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const EXPECTED: &[(&str, &str)] = &[
    (
        "tags",
        "TagKey[minecraft:entity_type / minecraft:freeze_immune_entity_types]|minecraft:freeze_immune_entity_types|ResourceKey[minecraft:root / minecraft:entity_type]|truetrue|true|falsefalsefalse|net.minecraft.tags.TagKey",
    ),
    (
        "categories",
        "BENEFICIALBENEFICIAL0BLUE|1RED|2BLUE|truetrue-2",
    ),
    (
        "attrs",
        "attribute.test|5.0,-2.0,10.0|true|-2.0,-2.0,10.0,2.5|true|true|net.minecraft.world.entity.ai.attributes.RangedAttribute",
    ),
    (
        "invalid",
        "Minimum value cannot be bigger than maximum value!;Default value cannot be lower than minimum value!;Default value cannot be bigger than maximum value!;",
    ),
];

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
        r#"package demo;
import net.minecraft.tags.*; import net.minecraft.world.entity.*; import net.minecraft.world.effect.*; import net.minecraft.world.entity.ai.attributes.*;
public final class Probe {
  public static String tags() { TagKey<EntityType<?>> t=EntityTypeTags.FREEZE_IMMUNE_ENTITY_TYPES; Object s="tag-key/registry/minecraft:entity_type|minecraft:freeze_immune_entity_types"; return t+"|"+t.location()+"|"+t.registry()+"|"+t.isFor(t.registry())+t.equals(EntityTypeTags.FREEZE_IMMUNE_ENTITY_TYPES)+"|"+(t==EntityTypeTags.FREEZE_IMMUNE_ENTITY_TYPES)+"|"+(s instanceof TagKey)+s.equals(t)+t.equals(s)+"|"+t.getClass().getName(); }
  public static String categories() { MobEffectCategory b=MobEffectCategory.BENEFICIAL; return b.name()+b.toString()+b.ordinal()+b.getTooltipFormatting().name()+"|"+MobEffectCategory.HARMFUL.ordinal()+MobEffectCategory.HARMFUL.getTooltipFormatting().name()+"|"+MobEffectCategory.NEUTRAL.ordinal()+MobEffectCategory.NEUTRAL.getTooltipFormatting().name()+"|"+(b==MobEffectCategory.BENEFICIAL)+b.equals(MobEffectCategory.BENEFICIAL)+b.compareTo(MobEffectCategory.NEUTRAL); }
  public static String attrs() { RangedAttribute a=new RangedAttribute("attribute.test",5,-2,10); Attribute same=a.setSyncable(true); return a.getDescriptionId()+"|"+a.getDefaultValue()+","+a.getMinValue()+","+a.getMaxValue()+"|"+a.isClientSyncable()+"|"+a.sanitizeValue(Double.NaN)+","+a.sanitizeValue(-3)+","+a.sanitizeValue(12)+","+a.sanitizeValue(2.5)+"|"+(same==a)+"|"+(a instanceof Attribute)+"|"+a.getClass().getName(); }
  public static String invalid() { StringBuilder o=new StringBuilder(); for(double[] x:new double[][]{{5,2,1},{1,2,3},{4,1,3}}) { try {new RangedAttribute("x",x[0],x[1],x[2]);o.append("ok");} catch(IllegalArgumentException e){o.append(e.getMessage());} o.append(';');} return o.toString(); }
}
"#,
    ),
];

fn fixture() -> Fixture {
    Fixture::build("vanilla-values", SOURCES)
}

#[test]
fn vanilla_values_match_java_25_and_forged_marker_stays_string() {
    let fixture = fixture();
    for (method, expected) in EXPECTED {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let got = fixture.text_on(&mut host, method);
        assert_eq!(got, *expected, "{method}");
        let unknown = host.unknown_host_calls();
        if *method == "tags" {
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

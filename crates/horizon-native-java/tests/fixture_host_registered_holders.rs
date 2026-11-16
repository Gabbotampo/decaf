//! Mod-created holders: the host answers only ids observed in registration calls.
//!
//! `java25.txt` is captured from the real 26.3 server classes after bootstrap;
//! the Rust fixture keeps only the signatures needed to exercise the same calls.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const CAPTURED: &str = include_str!("fixtures/registered_holders/java25.txt");

fn captured_rows() -> Vec<&'static str> {
    CAPTURED
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect()
}

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources;
public final class Identifier {
  public static Identifier fromNamespaceAndPath(String namespace, String path) { return null; }
  public static Identifier parse(String text) { return null; }
}
"#,
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        r#"package net.minecraft.resources;
public class ResourceKey<T> {
  public static <T> ResourceKey<T> create(ResourceKey<?> r, Identifier i) { return null; }
  public Identifier identifier() { return null; }
  public String toString() { return null; }
}
"#,
    ),
    (
        "net/minecraft/core/Holder.java",
        r#"package net.minecraft.core;
import java.util.Optional;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
public interface Holder<T> {
  interface Reference<T> extends Holder<T> { ResourceKey<T> key(); }
  interface Direct<T> extends Holder<T> {}
  T value();
  boolean isBound();
  boolean areComponentsBound();
  boolean is(Identifier id);
  boolean is(ResourceKey<T> key);
  String getRegisteredName();
  Optional<ResourceKey<T>> unwrapKey();
  Kind kind();
  enum Kind { REFERENCE, DIRECT }
  static <T> Holder<T> direct(T value) { return null; }
  static <T> Holder<T> direct(T value, net.minecraft.core.component.DataComponentMap map) { return null; }
}
"#,
    ),
    (
        "net/minecraft/core/component/DataComponentMap.java",
        "package net.minecraft.core.component; public interface DataComponentMap {}\n",
    ),
    (
        "net/minecraft/core/Registry.java",
        r#"package net.minecraft.core;
import java.util.Optional;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
public interface Registry<T> {
  static <T> T register(Registry<? super T> registry, Identifier id, T value) { return value; }
  static <T> Holder.Reference<T> registerForHolder(Registry<? super T> registry, Identifier id, T value) { return null; }
  Holder<T> wrapAsHolder(T value);
  Optional<Holder.Reference<T>> get(Identifier id);
}
"#,
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        r#"package net.minecraft.core.registries;
import net.minecraft.core.Registry;
import net.minecraft.world.effect.MobEffect;
import net.minecraft.world.entity.ai.attributes.Attribute;
public final class BuiltInRegistries {
  public static final Registry<MobEffect> MOB_EFFECT = null;
  public static final Registry<Attribute> ATTRIBUTE = null;
}
"#,
    ),
    (
        "net/minecraft/world/effect/MobEffectCategory.java",
        r#"package net.minecraft.world.effect;
public enum MobEffectCategory { BENEFICIAL, HARMFUL }
"#,
    ),
    (
        "net/minecraft/world/effect/MobEffect.java",
        r#"package net.minecraft.world.effect;
public class MobEffect { protected MobEffect(MobEffectCategory category, int color) {} }
"#,
    ),
    (
        "net/minecraft/world/entity/ai/attributes/Attribute.java",
        "package net.minecraft.world.entity.ai.attributes; public class Attribute {}\n",
    ),
    (
        "net/minecraft/world/entity/ai/attributes/RangedAttribute.java",
        r#"package net.minecraft.world.entity.ai.attributes;
public class RangedAttribute extends Attribute {
  public RangedAttribute(String id, double value, double min, double max) {}
}
"#,
    ),
    (
        "demo/Probe.java",
        r#"package demo;
import net.minecraft.core.Holder;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.effect.MobEffect;
import net.minecraft.world.effect.MobEffectCategory;
import net.minecraft.world.entity.ai.attributes.Attribute;
import net.minecraft.world.entity.ai.attributes.RangedAttribute;
public final class Probe {
  static final class Effect extends MobEffect {
    Effect() { super(MobEffectCategory.BENEFICIAL, 1); }
  }
  static String facts(String label, Holder<?> holder, Identifier id, Object expected) {
    Object key = holder.unwrapKey().orElse(null);
    return label + "|" + holder.getRegisteredName() + "|" + key + "|"
      + (holder.value() == expected) + "|" + holder.isBound() + "|" + holder.kind()
      + "|" + holder.is(id);
  }
  public static String effect() {
    Identifier id = Identifier.fromNamespaceAndPath("probe", "effect");
    MobEffect value = new Effect();
    Holder.Reference<MobEffect> holder = Registry.registerForHolder(BuiltInRegistries.MOB_EFFECT, id, value);
    return facts("effect", holder, id, value);
  }
  static Attribute attributeValue(Identifier id) {
    return Registry.register(BuiltInRegistries.ATTRIBUTE, id,
      new RangedAttribute("attribute", 1.0, 0.0, 2.0));
  }
  public static String attribute() {
    Identifier id = Identifier.fromNamespaceAndPath("probe", "attribute");
    Attribute value = attributeValue(id);
    return facts("direct", Holder.direct(value), id, value);
  }
  public static String wrapped() {
    Identifier id = Identifier.fromNamespaceAndPath("probe", "attribute");
    Attribute value = attributeValue(id);
    return facts("wrapped", BuiltInRegistries.ATTRIBUTE.wrapAsHolder(value), id, value);
  }
}
"#,
    ),
];

fn fixture(test: &str) -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(test, STUBS)
}

#[test]
fn observed_registration_holders_match_java25_oracle() {
    let fixture = fixture("registered-holders");
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "effect"), captured_rows()[0]);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "attribute"), captured_rows()[1]);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

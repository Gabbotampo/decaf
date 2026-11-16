//! Vanilla `Holder.Reference` constants are typed game-owned handles.
//!
//! The expected pure holder facts were recorded from Java 25 against the real
//! 26.3 server classes after `Bootstrap.bootStrap()`; the stubs below only keep
//! the fixture jar small so the interpreter takes the same static-field path.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const CAPTURED: &str = include_str!("fixtures/vanilla_holders/java25.txt");

fn captured_rows() -> Vec<&'static str> {
    CAPTURED
        .lines()
        .filter(|line| !line.starts_with("#"))
        .collect()
}

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources;
public final class Identifier { public static Identifier parse(String s) { return null; } }
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
import net.minecraft.resources.ResourceKey;
public interface Holder<T> {
  interface Reference<T> extends Holder<T> { ResourceKey<T> key(); }
  String getRegisteredName();
  Optional<ResourceKey<T>> unwrapKey();
  boolean isBound();
  boolean is(ResourceKey<T> key);
  T value();
}
"#,
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        r#"package net.minecraft.core.registries;
import net.minecraft.resources.ResourceKey;
public final class Registries {
  public static final ResourceKey<?> MOB_EFFECT = null;
  public static final ResourceKey<?> ATTRIBUTE = null;
}
"#,
    ),
    (
        "net/minecraft/world/effect/MobEffect.java",
        "package net.minecraft.world.effect; public class MobEffect {}\n",
    ),
    (
        "net/minecraft/world/effect/MobEffects.java",
        r#"package net.minecraft.world.effect;
import net.minecraft.core.Holder;
public final class MobEffects { public static final Holder<MobEffect> SPEED = null; }
"#,
    ),
    (
        "net/minecraft/world/entity/ai/attributes/Attribute.java",
        "package net.minecraft.world.entity.ai.attributes; public class Attribute {}\n",
    ),
    (
        "net/minecraft/world/entity/ai/attributes/Attributes.java",
        r#"package net.minecraft.world.entity.ai.attributes;
import net.minecraft.core.Holder;
public final class Attributes { public static final Holder<Attribute> ARMOR = null; public static final double DEFAULT_ATTACK_SPEED = 1.0; }
"#,
    ),
    (
        "demo/Probe.java",
        r#"package demo;
import net.minecraft.core.Holder;
import net.minecraft.world.effect.MobEffects;
public final class Probe {
  public static String speed() {
    Holder h = MobEffects.SPEED;
    Holder.Reference raw = (Holder.Reference) h;
    Object key = h.unwrapKey().get();
    return h.getRegisteredName()+"|"+key+"|"+raw.key()+"|"+h.isBound()+"|"+h.is((net.minecraft.resources.ResourceKey) key)+"|"+(h==MobEffects.SPEED)+"|"+(h.value()==h.value());
  }
  public static String armor() {
    Holder h = net.minecraft.world.entity.ai.attributes.Attributes.ARMOR;
    Holder.Reference raw = (Holder.Reference) h;
    Object key = h.unwrapKey().get();
    return h.getRegisteredName()+"|"+key+"|"+raw.key()+"|"+h.isBound()+"|"+h.is((net.minecraft.resources.ResourceKey) key)+"|"+(h==MobEffects.SPEED)+"|"+(h.value()==h.value());
  }
  public static String forgery() {
    Object s = "client/mob-effect/minecraft:speed";
    return (s instanceof Holder)+"|"+s.equals(MobEffects.SPEED)+"|"+MobEffects.SPEED.equals(s);
  }
}
"#,
    ),
];

fn fixture(test: &str) -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(test, STUBS)
}

#[test]
fn vanilla_holder_facts_match_java25_oracle() {
    let fixture = fixture("vanilla-holders");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, "speed");
    assert_eq!(text, captured_rows()[0]);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "armor"), captured_rows()[1]);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn a_string_spelled_like_the_game_value_is_not_a_holder() {
    let fixture = fixture("vanilla-holders-forgery");
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, "forgery");
    assert_eq!(text, captured_rows()[2]);
}

//! Java 25 oracle fixture for the vanilla `Enchantments` `ResourceKey` constants
//! (`vanilla_registry_keys.rs`; Spell Engine's `RPGSeriesCore.init` puts three in a `Set`).
//!
//! `fixtures/vanilla_damage_types_keys/demo/Probe.java` is one source run two ways: on Java 25
//! against the real 26.3 game jar after bootstrap (`java25.txt`, command in the README), and
//! here, compiled against the stubs below. Only `demo/` ships, so every game call reaches the
//! host model; `rows()` must equal the capture byte for byte and nothing may be counted.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources; public class ResourceKey<T> {}",
    ),
    (
        "net/minecraft/world/damagesource/DamageTypes.java",
        "package net.minecraft.world.damagesource; import net.minecraft.resources.ResourceKey; public interface DamageTypes { public static final ResourceKey PLAYER_ATTACK = null, ARROW = null; }",
    ),
];
const PROBE: &str = include_str!("fixtures/vanilla_damage_types_keys/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/vanilla_damage_types_keys/java25.txt");

#[test]
fn vanilla_enchantment_key_rows_match_java25() {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    let fixture = Fixture::build("vanilla-damage-types-keys", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let rows = fixture.text_on(&mut host, "rows");
    let unknown = host.unknown_host_calls();
    assert_eq!(rows, ORACLE, "unknown calls: {unknown:?}");
    assert!(unknown.is_empty(), "{unknown:?}");
}

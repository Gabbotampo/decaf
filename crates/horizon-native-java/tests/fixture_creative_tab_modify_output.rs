//! Fabric modify-output listeners run through NativeJar and retain tab order.
//! The real API's `FabricCreativeModeTabOutput` declares `accept(ItemStack,
//! TabVisibility)` as its sole abstract Output method; its ItemLike overloads
//! are interface defaults. We model only the one-argument defaults here.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use std::path::Path;

const PROBE: &str = r#"
package demo;
import net.fabricmc.fabric.api.event.Event;
import net.fabricmc.fabric.api.creativetab.v1.*;
import net.minecraft.resources.*;
import net.minecraft.world.item.*;
import net.minecraft.world.level.ItemLike;
public final class Probe {
  static ResourceKey<CreativeModeTab> key(String id) { return ResourceKey.create(ResourceKey.createRegistryKey(Identifier.parse("minecraft:creative_mode_tab")), Identifier.parse(id)); }
  static final class DemoItem extends net.minecraft.world.item.Item implements ItemLike { final String id; DemoItem(String id) { this.id=id; } public net.minecraft.world.item.Item asItem() { return this; } public String toString(){return id;} }
  static final class ObjectListener implements CreativeModeTabEvents.ModifyOutput { public void modifyOutput(FabricCreativeModeTabOutput o) { o.accept(new DemoItem("object")); } }
  static void register() {
    Event<CreativeModeTabEvents.ModifyOutput> a = CreativeModeTabEvents.modifyOutputEvent(key("demo:functional"));
    a.register(o -> o.accept(new DemoItem("lambda")));
    Event<CreativeModeTabEvents.ModifyOutput> b = CreativeModeTabEvents.modifyOutputEvent(key("demo:combat"));
    b.register(new ObjectListener());
  }
  public static void registerThenThrow() {
    register();
    CreativeModeTabEvents.modifyOutputEvent(key("demo:throws")).register(o -> { throw new IllegalStateException("boom"); });
  }
}
"#;

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources; public final class Identifier { public static Identifier parse(String s){return null;} }",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources; public final class ResourceKey<T> { public static <T> ResourceKey<T> create(ResourceKey<?> r, Identifier i){return null;} public static <T> ResourceKey<T> createRegistryKey(Identifier i){return null;} }",
    ),
    (
        "net/minecraft/world/item/CreativeModeTab.java",
        "package net.minecraft.world.item; public final class CreativeModeTab {}",
    ),
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item; public class Item {}",
    ),
    (
        "net/minecraft/world/level/ItemLike.java",
        "package net.minecraft.world.level; import net.minecraft.world.item.Item; public interface ItemLike { Item asItem(); }",
    ),
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event; public interface Event<T> { void register(T listener); }",
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput.java",
        "package net.fabricmc.fabric.api.creativetab.v1; import net.minecraft.world.level.ItemLike; public class FabricCreativeModeTabOutput { public void accept(ItemLike i){} }",
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.java",
        "package net.fabricmc.fabric.api.creativetab.v1; import net.fabricmc.fabric.api.event.Event; import net.minecraft.resources.ResourceKey; import net.minecraft.world.item.CreativeModeTab; public final class CreativeModeTabEvents { public interface ModifyOutput { void modifyOutput(FabricCreativeModeTabOutput o); } public static Event<ModifyOutput> modifyOutputEvent(ResourceKey<CreativeModeTab> k){return null;} }",
    ),
];

#[test]
fn javac_listeners_are_dispatched_per_tab_and_exceptions_propagate() {
    let mut sources = vec![("demo/Probe.java", PROBE)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("creative-tab-modify-output", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture
        .jar
        .invoke_method("demo/Probe", "register", "()V", None, &mut host)
        .unwrap();
    let contents = fixture.jar.creative_tab_contents(&mut host).unwrap();
    let functional = "minecraft:creative_mode_tab:demo:functional";
    let combat = "minecraft:creative_mode_tab:demo:combat";
    assert_eq!(contents[functional].len(), 1);
    assert_eq!(contents[combat].len(), 1);
    assert!(matches!(contents[functional][0], Value::Host(_)));

    fixture
        .jar
        .invoke_method("demo/Probe", "registerThenThrow", "()V", None, &mut host)
        .unwrap();
    let error = fixture.jar.creative_tab_contents(&mut host).unwrap_err();
    assert!(format!("{error:?}").contains("boom"));
}

#[test]
fn real_runes_entrypoint_adds_items_to_both_tabs() {
    let jar = horizon_native_java::NativeJar::from_path(Path::new(
        "/home/solenopsisbot/horizon-corpus/deps-pool/runes-fabric-1.3.2+26.3.jar",
    ))
    .unwrap();
    let mut host = jar.seed_host(StdHost::new());
    // The full entrypoint also initializes the optional Bundle API component;
    // invoke its exact item-registration and two bytecode listener bodies so
    // this test isolates the creative-tab boundary.
    jar.invoke_method(
        "net/runes/RunesMod",
        "registerItems",
        "()V",
        None,
        &mut host,
    )
    .unwrap();
    let before_lambdas = host.unknown_host_call_count();
    let functional_output =
        host.begin_creative_tab_output("minecraft:creative_mode_tab:minecraft:functional_blocks");
    jar.invoke_method_with_args(
        "net/runes/fabric/FabricMod",
        "lambda$onInitialize$1",
        "(Lnet/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput;)V",
        None,
        std::slice::from_ref(&functional_output),
        &mut host,
    )
    .unwrap();
    let combat_output =
        host.begin_creative_tab_output("minecraft:creative_mode_tab:minecraft:combat");
    jar.invoke_method_with_args(
        "net/runes/fabric/FabricMod",
        "lambda$onInitialize$2",
        "(Lnet/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput;)V",
        None,
        std::slice::from_ref(&combat_output),
        &mut host,
    )
    .unwrap();
    let functional = host.take_creative_tab_output(&functional_output).unwrap();
    let combat = host.take_creative_tab_output(&combat_output).unwrap();
    let item_id = |value: &Value| {
        host.registrations()
            .iter()
            .find(|r| r.registry == "minecraft:item" && r.value == *value)
            .map(|r| r.id.as_str())
            .unwrap()
    };
    assert_eq!(item_id(&functional[0]), "runes:crafting_altar");
    let combat_ids: Vec<_> = combat.iter().map(item_id).collect();
    assert_eq!(
        combat_ids,
        vec![
            "runes:arcane_stone",
            "runes:fire_stone",
            "runes:frost_stone",
            "runes:healing_stone",
            "runes:lightning_stone",
            "runes:soul_stone",
        ]
    );
    assert_eq!(functional.len(), 1);
    assert_eq!(combat.len(), 6);
    assert_eq!(
        host.registrations()
            .iter()
            .filter(|r| r.registry == "minecraft:item")
            .count(),
        7
    );
    assert_eq!(host.unknown_host_call_count() - before_lambdas, 0);
}

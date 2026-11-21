//! Real-jar integration test against `runes-fabric-1.3.2+26.3.jar`.
//!
//! Runs Runes' own `RunesMod.registerItems()`, `RunesMod.registerRecipeType()`,
//! `RuneCraftingRecipe.MAP_CODEC`, `RuneCraftingRecipe.matches`, `RuneCraftingRecipe.assemble`,
//! and `RuneCraftingScreenHandler.createResult()` through Decaf's general vanilla recipe and
//! menu models, and checks every result line-for-line against `fixtures/runes_recipes/mc263.txt`
//! captured from a real Java 25 run on Minecraft 26.3 + the same Runes jar.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, NativeJar, StdHost,
    events::{EventContext, EventFacade, EventFiring, FacadeLevel, FacadePlayer, GameEvent},
};
use std::{path::Path, sync::Arc};

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier { public static Identifier parse(String s) { return null; } }",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources;\n\
         public class ResourceKey<T> {\n\
           public static <T> ResourceKey<T> create(ResourceKey<? extends net.minecraft.core.Registry<T>> reg, Identifier id) { return null; }\n\
           public Identifier identifier() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/core/Registry.java",
        "package net.minecraft.core; public interface Registry<T> {}",
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        "package net.minecraft.core.registries;\n\
         import net.minecraft.core.Registry; import net.minecraft.resources.ResourceKey;\n\
         public final class Registries {\n\
           public static final ResourceKey<Registry<net.minecraft.world.item.crafting.Recipe<?>>> RECIPE = null;\n\
         }",
    ),
    (
        "net/minecraft/core/Holder.java",
        "package net.minecraft.core; public interface Holder<T> { String getRegisteredName(); }",
    ),
    (
        "net/minecraft/core/component/DataComponentType.java",
        "package net.minecraft.core.component; public interface DataComponentType<T> {}",
    ),
    (
        "net/minecraft/core/component/DataComponents.java",
        "package net.minecraft.core.component;\n\
         public final class DataComponents { public static final DataComponentType<Integer> REPAIR_COST = null; }",
    ),
    (
        "net/minecraft/world/level/ItemLike.java",
        "package net.minecraft.world.level; public interface ItemLike {}",
    ),
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item; public class Item implements net.minecraft.world.level.ItemLike {}",
    ),
    (
        "net/minecraft/world/item/Items.java",
        "package net.minecraft.world.item;\n\
         public final class Items {\n\
           public static final Item COBBLESTONE = null;\n\
           public static final Item BLACKSTONE = null;\n\
           public static final Item COBBLED_DEEPSLATE = null;\n\
           public static final Item DIRT = null;\n\
           public static final Item AMETHYST_SHARD = null;\n\
           public static final Item ENDER_PEARL = null;\n\
           public static final Item BLAZE_POWDER = null;\n\
           public static final Item COAL = null;\n\
           public static final Item SNOWBALL = null;\n\
           public static final Item ICE = null;\n\
           public static final Item GOLD_NUGGET = null;\n\
           public static final Item HONEYCOMB = null;\n\
         }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\n\
         import net.minecraft.core.component.DataComponentType; import net.minecraft.world.level.ItemLike;\n\
         public final class ItemStack {\n\
           public static final ItemStack EMPTY = null;\n\
           public ItemStack(ItemLike item) {}\n\
           public ItemStack(ItemLike item, int count) {}\n\
           public boolean isEmpty() { return true; }\n\
           public <T> T set(DataComponentType<T> type, T value) { return null; }\n\
           public <T> T get(DataComponentType<? extends T> type) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/ItemStackTemplate.java",
        "package net.minecraft.world.item;\n\
         import net.minecraft.core.Holder;\n\
         public final class ItemStackTemplate { public Holder<Item> item() { return null; } public int count() { return 0; } }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeInput.java",
        "package net.minecraft.world.item.crafting; public interface RecipeInput {}",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeType.java",
        "package net.minecraft.world.item.crafting; public interface RecipeType<T extends Recipe<?>> {}",
    ),
    (
        "net/minecraft/world/item/crafting/Recipe.java",
        "package net.minecraft.world.item.crafting; public interface Recipe<I extends RecipeInput> {}",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeHolder.java",
        "package net.minecraft.world.item.crafting;\n\
         import net.minecraft.resources.ResourceKey;\n\
         public final class RecipeHolder<T extends Recipe<?>> {\n\
           public ResourceKey<Recipe<?>> id() { return null; }\n\
           public T value() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeManager.java",
        "package net.minecraft.world.item.crafting;\n\
         import java.util.Optional; import net.minecraft.resources.ResourceKey; import net.minecraft.world.level.Level;\n\
         public class RecipeManager {\n\
           public <I extends RecipeInput, T extends Recipe<I>> Optional<RecipeHolder<T>> getRecipeFor(RecipeType<T> type, I input, Level level) { return Optional.empty(); }\n\
           public Optional<RecipeHolder<?>> byKey(ResourceKey<Recipe<?>> key) { return Optional.empty(); }\n\
         }",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level; public abstract class Level {}",
    ),
    (
        "net/minecraft/server/level/ServerLevel.java",
        "package net.minecraft.server.level;\n\
         import net.minecraft.world.item.crafting.RecipeManager; import net.minecraft.world.level.Level;\n\
         public class ServerLevel extends Level { public RecipeManager recipeAccess() { return null; } }",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player; public class Player {}",
    ),
    (
        "net/minecraft/world/entity/EntityEquipment.java",
        "package net.minecraft.world.entity; public class EntityEquipment {}",
    ),
    (
        "net/minecraft/world/entity/player/Inventory.java",
        "package net.minecraft.world.entity.player;\n\
         import net.minecraft.world.entity.EntityEquipment;\n\
         public class Inventory { public Inventory(Player p, EntityEquipment e) {} }",
    ),
    (
        "net/minecraft/world/Container.java",
        "package net.minecraft.world; import net.minecraft.world.item.ItemStack;\n\
         public interface Container { ItemStack getItem(int i); void setItem(int i, ItemStack s); }",
    ),
    (
        "net/minecraft/world/inventory/ResultContainer.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container; import net.minecraft.world.item.ItemStack;\n\
         import net.minecraft.world.item.crafting.RecipeHolder;\n\
         public class ResultContainer implements Container {\n\
           public ItemStack getItem(int i) { return null; }\n\
           public void setItem(int i, ItemStack s) {}\n\
           public RecipeHolder<?> getRecipeUsed() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ContainerLevelAccess.java",
        "package net.minecraft.world.inventory;\n\
         public class ContainerLevelAccess { public static final ContainerLevelAccess NULL = null; }",
    ),
    (
        "net/minecraft/world/inventory/Slot.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container;\n\
         public class Slot { public final Container container; public Slot(Container c) { container = c; } }",
    ),
    (
        "net/minecraft/world/inventory/AbstractContainerMenu.java",
        "package net.minecraft.world.inventory;\n\
         public abstract class AbstractContainerMenu { public Slot getSlot(int index) { return null; } }",
    ),
    (
        "net/minecraft/world/inventory/ItemCombinerMenu.java",
        "package net.minecraft.world.inventory;\n\
         public abstract class ItemCombinerMenu extends AbstractContainerMenu { public abstract void createResult(); }",
    ),
    (
        "net/minecraft/world/InteractionHand.java",
        "package net.minecraft.world; public enum InteractionHand { MAIN_HAND, OFF_HAND }",
    ),
    (
        "net/minecraft/world/InteractionResult.java",
        "package net.minecraft.world; public final class InteractionResult { public static final InteractionResult PASS = new InteractionResult(); }",
    ),
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event; public abstract class Event<T> { public void register(T listener) {} }",
    ),
    (
        "net/fabricmc/fabric/api/event/player/UseItemCallback.java",
        "package net.fabricmc.fabric.api.event.player;\n\
         import net.fabricmc.fabric.api.event.Event;\n\
         import net.minecraft.world.*; import net.minecraft.world.entity.player.Player; import net.minecraft.world.level.Level;\n\
         public interface UseItemCallback {\n\
           Event<UseItemCallback> EVENT = null;\n\
           InteractionResult interact(Player player, Level level, InteractionHand hand);\n\
         }",
    ),
    // Compile-only stubs for the real Runes jar classes (not packed into the probe jar;
    // resolved at runtime from the linked real Runes 1.3.2+26.3 jar).
    (
        "net/runes/RunesMod.java",
        "package net.runes;\n\
         public class RunesMod {\n\
           public static void registerItems() {}\n\
           public static void registerRecipeType() {}\n\
         }",
    ),
    (
        "net/runes/crafting/RuneCraftingRecipeInput.java",
        "package net.runes.crafting;\n\
         import net.minecraft.world.item.ItemStack; import net.minecraft.world.item.crafting.RecipeInput;\n\
         public final class RuneCraftingRecipeInput implements RecipeInput {\n\
           public RuneCraftingRecipeInput(ItemStack base, ItemStack addition) {}\n\
         }",
    ),
    (
        "net/runes/crafting/RuneCraftingRecipe.java",
        "package net.runes.crafting;\n\
         import net.minecraft.world.item.*; import net.minecraft.world.item.crafting.*;\n\
         public class RuneCraftingRecipe implements Recipe<RuneCraftingRecipeInput> {\n\
           public static final RecipeType<RuneCraftingRecipe> TYPE = null;\n\
           public boolean showNotification() { return true; }\n\
           public ItemStackTemplate result() { return null; }\n\
           public ItemStack assemble(RuneCraftingRecipeInput input) { return null; }\n\
         }",
    ),
    (
        "net/runes/crafting/RuneCraftingScreenHandler.java",
        "package net.runes.crafting;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.inventory.*;\n\
         public class RuneCraftingScreenHandler extends ItemCombinerMenu {\n\
           public RuneCraftingScreenHandler(int id, Inventory inv, ContainerLevelAccess acc) {}\n\
           public void createResult() {}\n\
         }",
    ),
];

const HARNESS: &str = r#"
package demo;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.*;
import net.minecraft.world.entity.player.*;
import net.minecraft.world.inventory.*;
import net.minecraft.world.item.*;
import net.minecraft.world.item.crafting.*;
import net.runes.RunesMod;
import net.runes.crafting.*;

public final class Probe {
  public static void onInitialize() {
    RunesMod.registerItems();
    RunesMod.registerRecipeType();
    UseItemCallback.EVENT.register((player, level, hand) -> {
      run(player, (ServerLevel) level);
      return InteractionResult.PASS;
    });
  }

  private static void run(Player player, ServerLevel level) {
    RecipeManager mgr = level.recipeAccess();
    String[] altarIds = {
        "runes:arcane_rune_medium_altar",
        "runes:arcane_rune_small_altar",
        "runes:fire_rune_medium_altar",
        "runes:fire_rune_small_altar",
        "runes:frost_rune_medium_altar",
        "runes:frost_rune_small_altar",
        "runes:healing_rune_medium_altar",
        "runes:healing_rune_small_altar",
    };
    int decodedCount = 0;
    for (String id : altarIds) {
      ResourceKey<Recipe<?>> key = ResourceKey.create(Registries.RECIPE, Identifier.parse(id));
      var opt = mgr.byKey(key);
      if (opt.isPresent()) {
        decodedCount++;
        RuneCraftingRecipe rec = (RuneCraftingRecipe) opt.get().value();
        System.out.println("recipe=" + id
            + "|notify=" + rec.showNotification()
            + "|result=" + rec.result().item().getRegisteredName()
            + "|count=" + rec.result().count());
      }
    }
    System.out.println("decodedCount=" + decodedCount);

    RuneCraftingScreenHandler menu = new RuneCraftingScreenHandler(
        1, new Inventory(player, null), ContainerLevelAccess.NULL);
    Container inputs = menu.getSlot(0).container;
    ResultContainer results = (ResultContainer) menu.getSlot(2).container;

    ItemStack cobbleWithRepair = new ItemStack(Items.COBBLESTONE, 1);
    cobbleWithRepair.set(DataComponents.REPAIR_COST, 9);

    String[] labels = {
        "cobble+amethyst_shard",
        "cobble+ender_pearl",
        "blackstone+blaze_powder",
        "cobble+coal",
        "deepslate+snowball",
        "cobble+ice",
        "cobble+gold_nugget",
        "cobble+honeycomb",
        "cobble_repair9+amethyst_shard",
        "dirt+amethyst_shard",
        "empty+empty",
    };
    ItemStack[] bases = {
        new ItemStack(Items.COBBLESTONE),
        new ItemStack(Items.COBBLESTONE),
        new ItemStack(Items.BLACKSTONE),
        new ItemStack(Items.COBBLESTONE),
        new ItemStack(Items.COBBLED_DEEPSLATE),
        new ItemStack(Items.COBBLESTONE),
        new ItemStack(Items.COBBLESTONE),
        new ItemStack(Items.COBBLESTONE),
        cobbleWithRepair,
        new ItemStack(Items.DIRT),
        ItemStack.EMPTY,
    };
    ItemStack[] additions = {
        new ItemStack(Items.AMETHYST_SHARD),
        new ItemStack(Items.ENDER_PEARL),
        new ItemStack(Items.BLAZE_POWDER),
        new ItemStack(Items.COAL),
        new ItemStack(Items.SNOWBALL),
        new ItemStack(Items.ICE),
        new ItemStack(Items.GOLD_NUGGET),
        new ItemStack(Items.HONEYCOMB),
        new ItemStack(Items.AMETHYST_SHARD),
        new ItemStack(Items.AMETHYST_SHARD),
        ItemStack.EMPTY,
    };

    for (int i = 0; i < labels.length; i++) {
      // Setting slot 0 then slot 1 drives ItemCombinerMenu.slotsChanged -> RuneCraftingScreenHandler.createResult().
      inputs.setItem(0, bases[i]);
      inputs.setItem(1, additions[i]);
      RecipeHolder<?> used = results.getRecipeUsed();
      String found = used == null ? "empty" : used.id().identifier().toString();
      ItemStack out = results.getItem(0);
      System.out.println("case:" + labels[i]
          + "|recipe=" + found
          + "|out=" + out.toString()
          + "|empty=" + out.isEmpty()
          + "|repairCost=" + out.get(DataComponents.REPAIR_COST));
    }

    // Also verify direct RecipeManager.getRecipeFor + RuneCraftingRecipe.assemble + Optional.empty().
    RuneCraftingRecipeInput smallIn = new RuneCraftingRecipeInput(
        new ItemStack(Items.COBBLESTONE), new ItemStack(Items.AMETHYST_SHARD));
    var smallOpt = mgr.getRecipeFor(RuneCraftingRecipe.TYPE, smallIn, level);
    if (smallOpt.isEmpty() || !"8 runes:arcane_stone".equals(smallOpt.get().value().assemble(smallIn).toString())) {
      throw new AssertionError("unexpected small altar direct getRecipeFor/assemble result");
    }
    RuneCraftingRecipeInput medIn = new RuneCraftingRecipeInput(
        new ItemStack(Items.COBBLESTONE), new ItemStack(Items.ENDER_PEARL));
    var medOpt = mgr.getRecipeFor(RuneCraftingRecipe.TYPE, medIn, level);
    if (medOpt.isEmpty() || !"16 runes:arcane_stone".equals(medOpt.get().value().assemble(medIn).toString())) {
      throw new AssertionError("unexpected medium altar direct getRecipeFor/assemble result");
    }
    RuneCraftingRecipeInput missIn = new RuneCraftingRecipeInput(
        new ItemStack(Items.DIRT), new ItemStack(Items.AMETHYST_SHARD));
    if (mgr.getRecipeFor(RuneCraftingRecipe.TYPE, missIn, level).isPresent()) {
      throw new AssertionError("expected Optional.empty() for non-matching input");
    }
  }
}
"#;

#[test]
fn real_runes_jar_decodes_altar_recipes_and_runs_create_result() {
    // The real Runes 1.3.2 jar isn't in the repo or on CI: point RUNES_JAR at it to run this.
    let Some(runes_jar_path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes recipe test skipped");
        return;
    };
    let runes_path = Path::new(&runes_jar_path);

    let runes_jar = NativeJar::from_path(runes_path).expect("load real Runes 1.3.2+26.3 jar");
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", HARNESS));
    let fixture = javac_fixture::Fixture::build_with_mod_json(
        "runes-recipes",
        &sources,
        r#"{"id":"runes-harness","version":"1.0","entrypoints":{"main":"demo.Probe"}}"#,
    );

    // Link the real Runes jar directly (without seed_host): recipes and item tags load from
    // the linked jar layer automatically alongside its bytecode classes.
    let mut host = StdHost::new();
    host.link_jar("runes", &runes_jar).expect("link Runes jar");
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap_or_else(|err| {
            panic!(
                "Runes init failed: {err:?}\nunknown={:?}",
                host.unknown_host_calls()
            )
        });

    let listeners = host.game_listeners(&fixture.jar, true);
    assert_eq!(listeners.listeners.len(), 1);
    let player_uuid = 0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111;
    let facade = Arc::new(EventFacade {
        players: vec![FacadePlayer {
            uuid: player_uuid,
            name: "Runecrafter".into(),
            level: Some("minecraft:overworld".into()),
            entity_id: Some(9),
            session: 1,
            life: 0,
        }],
        levels: vec![FacadeLevel {
            id: "minecraft:overworld".into(),
            ..FacadeLevel::default()
        }],
        ..EventFacade::default()
    });
    let mut firing = EventFiring::new(
        GameEvent::UseItem,
        facade,
        EventContext {
            level: Some("minecraft:overworld".into()),
            player: Some(player_uuid),
            off_hand: Some(false),
            ..EventContext::default()
        },
    );
    let unknowns_before = host.unknown_host_calls();
    fixture
        .jar
        .run_game_listener(
            &listeners.listeners[0],
            &mut firing,
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap_or_else(|err| {
            panic!(
                "Runes createResult failed: {err:?}\nunknown={:?}",
                host.unknown_host_calls()
            )
        });

    assert_eq!(
        host.unknown_host_calls(),
        unknowns_before,
        "recipe decoding, getRecipeFor, assemble, and RuneCraftingScreenHandler.createResult must incur zero unknown host calls"
    );
    assert_eq!(
        host.take_stdout(),
        include_str!("fixtures/runes_recipes/mc263.txt")
    );
    assert_eq!(host.loaded_recipe_ids().len(), 8);
    assert!(
        host.loaded_recipe_sources()
            .iter()
            .all(|(_, pack)| pack == "runes"),
        "expected all 8 altar recipes to report source pack \"runes\", got {:?}",
        host.loaded_recipe_sources()
    );
    assert!(
        host.recipe_decode_errors().is_empty(),
        "unexpected decode errors: {:?}",
        host.recipe_decode_errors()
    );
}

#[test]
fn real_runes_jar_skipped_vanilla_serializer_recipe_stays_counted() {
    let Some(runes_jar_path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes skipped-recipe test skipped");
        return;
    };
    let runes_path = Path::new(&runes_jar_path);
    let runes_jar = NativeJar::from_path(runes_path).expect("load real Runes 1.3.2+26.3 jar");
    let mut sources = STUBS.to_vec();
    sources.push((
        "demo/SkippedProbe.java",
        r#"
package demo;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.*;
import net.minecraft.world.item.crafting.*;
import net.runes.RunesMod;

public final class SkippedProbe {
  public static void onInitialize() {
    RunesMod.registerItems();
    RunesMod.registerRecipeType();
    UseItemCallback.EVENT.register((player, level, hand) -> {
      RecipeManager mgr = ((ServerLevel) level).recipeAccess();
      // data/runes/recipe/rune_crafting_altar.json uses vanilla minecraft:crafting_shaped -> must be counted.
      mgr.byKey(ResourceKey.create(Registries.RECIPE, Identifier.parse("runes:rune_crafting_altar")));
      return InteractionResult.PASS;
    });
  }
}
"#,
    ));
    let fixture = javac_fixture::Fixture::build_with_mod_json(
        "runes-recipes-skipped",
        &sources,
        r#"{"id":"runes-skipped","version":"1.0","entrypoints":{"main":"demo.SkippedProbe"}}"#,
    );
    let mut host = StdHost::new();
    host.link_jar("runes", &runes_jar).expect("link Runes jar");
    fixture
        .jar
        .execute("demo.SkippedProbe", None, "main", &mut host)
        .unwrap();
    let listeners = host.game_listeners(&fixture.jar, true);
    let player_uuid = 0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111;
    let facade = Arc::new(EventFacade {
        players: vec![FacadePlayer {
            uuid: player_uuid,
            name: "Runecrafter".into(),
            level: Some("minecraft:overworld".into()),
            entity_id: Some(9),
            session: 1,
            life: 0,
        }],
        levels: vec![FacadeLevel {
            id: "minecraft:overworld".into(),
            ..FacadeLevel::default()
        }],
        ..EventFacade::default()
    });
    let mut firing = EventFiring::new(
        GameEvent::UseItem,
        facade,
        EventContext {
            level: Some("minecraft:overworld".into()),
            player: Some(player_uuid),
            off_hand: Some(false),
            ..EventContext::default()
        },
    );
    fixture
        .jar
        .run_game_listener(
            &listeners.listeners[0],
            &mut firing,
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert!(
        host.unknown_host_calls().iter().any(|(call, _)| {
            call == "net/minecraft/world/item/crafting/RecipeManager#byKey(Lnet/minecraft/resources/ResourceKey;)Ljava/util/Optional;"
        }),
        "expected skipped minecraft:crafting_shaped recipe byKey to be counted, got {:?}",
        host.unknown_host_calls()
    );
}

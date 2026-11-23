//! Live Java menus (`live_menu.rs`): a javac-built two-input combiner
//! (`fixtures/live_menu/demo/*.java`) opened by a block use, then driven step by step through
//! `NativeJar::click_player_menu` / `tick_player_menu` / `close_player_menu_live`, the way an
//! embedder delivers its player's clicks.
//!
//! Every line is compared with `fixtures/live_menu/java25.txt`, captured on Java 25 from the
//! same demo sources and the same `script.txt`, run against the real 26.3 server jar's
//! `AbstractContainerMenu` / `ItemCombinerMenu` / `Slot` / `SimpleContainer` / `Inventory` /
//! `ItemStack` (`oracle/LiveMenuOracle.java`). The mod's own prints come from the interpreter's
//! stdout; the effect lines from the typed `LiveMenuEffect` stream.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use std::collections::BTreeMap;

use horizon_native_java::{
    BlockHitData, BlockUseContext, BlockUsePlayerData, BlockUseSide, ContainerInput,
    ExecutionBudget, LiveMenuEffect, MenuClick, MenuPlayerFacts, PlayerContainer, StackComponents,
    StdHost, Value,
};

const UUID: u128 = 0x5eed_0000_0000_0000_0000_0000_0000_0001;
const POS: [i32; 3] = [3, 64, 5];

const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/api/ModInitializer.java",
        "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier {\n\
           public static Identifier fromNamespaceAndPath(String ns, String p) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/core/Registry.java",
        "package net.minecraft.core;\n\
         import net.minecraft.resources.Identifier;\n\
         public interface Registry<T> {\n\
           static <V, T extends V> T register(Registry<V> reg, Identifier id, T val) { return val; }\n\
         }",
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        "package net.minecraft.core.registries;\n\
         import net.minecraft.core.Registry;\n\
         import net.minecraft.world.inventory.MenuType;\n\
         import net.minecraft.world.level.block.Block;\n\
         public final class BuiltInRegistries {\n\
           public static final Registry<Block> BLOCK = null;\n\
           public static final Registry<MenuType<?>> MENU = null;\n\
         }",
    ),
    (
        "net/minecraft/core/BlockPos.java",
        "package net.minecraft.core; public class BlockPos {}",
    ),
    (
        "net/minecraft/world/phys/BlockHitResult.java",
        "package net.minecraft.world.phys; public class BlockHitResult {}",
    ),
    (
        "net/minecraft/world/InteractionResult.java",
        "package net.minecraft.world;\n\
         public interface InteractionResult {\n\
           InteractionResult SUCCESS = null;\n\
           InteractionResult CONSUME = null;\n\
           InteractionResult PASS = null;\n\
         }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat;\n\
         public interface Component { static MutableComponent translatable(String key) { return null; } }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat; public class MutableComponent implements Component {}",
    ),
    (
        "net/minecraft/world/flag/FeatureFlagSet.java",
        "package net.minecraft.world.flag; public class FeatureFlagSet {}",
    ),
    (
        "net/minecraft/world/flag/FeatureFlags.java",
        "package net.minecraft.world.flag;\n\
         public final class FeatureFlags { public static final FeatureFlagSet VANILLA_SET = null; }",
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
           public static final Item AMETHYST_SHARD = null;\n\
           public static final Item STONE_BRICKS = null;\n\
           public static final Item DIRT = null;\n\
         }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.ItemLike;\n\
         public final class ItemStack {\n\
           public static final ItemStack EMPTY = null;\n\
           public ItemStack(ItemLike item, int count) {}\n\
           public boolean isEmpty() { return true; }\n\
           public int getCount() { return 0; }\n\
           public void shrink(int amount) {}\n\
           public boolean is(Item item) { return false; }\n\
           public void onCraftedBy(Player player, int amount) {}\n\
         }",
    ),
    (
        "net/minecraft/world/Container.java",
        "package net.minecraft.world;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public interface Container {\n\
           ItemStack getItem(int slot);\n\
           void setItem(int slot, ItemStack stack);\n\
         }",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player;\n\
         import java.util.OptionalInt;\n\
         import net.minecraft.world.MenuProvider;\n\
         public abstract class Player {\n\
           public OptionalInt openMenu(MenuProvider provider) { return OptionalInt.empty(); }\n\
         }",
    ),
    (
        "net/minecraft/world/entity/player/Inventory.java",
        "package net.minecraft.world.entity.player; public class Inventory {}",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level;\n\
         public abstract class Level { public boolean isClientSide() { return false; } }",
    ),
    (
        "net/minecraft/world/inventory/Slot.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container;\n\
         public class Slot { public final Container container = null; }",
    ),
    (
        "net/minecraft/world/inventory/ResultContainer.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public class ResultContainer implements Container {\n\
           public ItemStack getItem(int slot) { return null; }\n\
           public void setItem(int slot, ItemStack stack) {}\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/AbstractContainerMenu.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public abstract class AbstractContainerMenu {\n\
           protected AbstractContainerMenu(MenuType<?> type, int id) {}\n\
           public boolean canTakeItemForPickAll(ItemStack stack, Slot slot) { return true; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ContainerLevelAccess.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.level.Level;\n\
         public interface ContainerLevelAccess {\n\
           ContainerLevelAccess NULL = null;\n\
           static ContainerLevelAccess create(Level level, BlockPos pos) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition.java",
        "package net.minecraft.world.inventory;\n\
         import java.util.function.Predicate;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public class ItemCombinerMenuSlotDefinition {\n\
           public static Builder create() { return null; }\n\
           public static class Builder {\n\
             public Builder withSlot(int i, int x, int y, Predicate<ItemStack> p) { return this; }\n\
             public Builder withResultSlot(int i, int x, int y) { return this; }\n\
             public ItemCombinerMenuSlotDefinition build() { return null; }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ItemCombinerMenu.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.item.ItemStack;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         public abstract class ItemCombinerMenu extends AbstractContainerMenu {\n\
           protected final Container inputSlots = null;\n\
           protected final ResultContainer resultSlots = null;\n\
           public ItemCombinerMenu(MenuType<?> type, int id, Inventory inv, ContainerLevelAccess access, ItemCombinerMenuSlotDefinition def) {\n\
             super(type, id);\n\
           }\n\
           protected abstract void onTake(Player player, ItemStack stack);\n\
           protected abstract boolean isValidBlock(BlockState state);\n\
           public abstract void createResult();\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/MenuType.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.flag.FeatureFlagSet;\n\
         public class MenuType<T extends AbstractContainerMenu> {\n\
           public interface MenuSupplier<T extends AbstractContainerMenu> { T create(int id, Inventory inv); }\n\
           public MenuType(MenuSupplier<T> supplier, FeatureFlagSet features) {}\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/MenuConstructor.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         public interface MenuConstructor {\n\
           AbstractContainerMenu createMenu(int id, Inventory inv, Player player);\n\
         }",
    ),
    (
        "net/minecraft/world/MenuProvider.java",
        "package net.minecraft.world;\n\
         import net.minecraft.world.inventory.MenuConstructor;\n\
         public interface MenuProvider extends MenuConstructor {}",
    ),
    (
        "net/minecraft/world/SimpleMenuProvider.java",
        "package net.minecraft.world;\n\
         import net.minecraft.network.chat.Component;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.inventory.AbstractContainerMenu;\n\
         import net.minecraft.world.inventory.MenuConstructor;\n\
         public final class SimpleMenuProvider implements MenuProvider {\n\
           public SimpleMenuProvider(MenuConstructor ctor, Component title) {}\n\
           public AbstractContainerMenu createMenu(int id, Inventory inv, Player p) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        "package net.minecraft.world.level.block.state;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public abstract class BlockBehaviour {\n\
           public static class Properties { public static Properties of() { return new Properties(); } }\n\
           protected BlockBehaviour(Properties props) {}\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             return InteractionResult.PASS;\n\
           }\n\
           public static abstract class BlockStateBase {}\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/state/BlockState.java",
        "package net.minecraft.world.level.block.state;\n\
         public class BlockState extends BlockBehaviour.BlockStateBase {\n\
           public boolean is(Object block) { return false; }\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Block extends BlockBehaviour { public Block(Properties props) { super(props); } }",
    ),
];

fn sources() -> Vec<(&'static str, &'static str)> {
    let mut sources = STUBS.to_vec();
    sources.push((
        "demo/Probe.java",
        include_str!("fixtures/live_menu/demo/Probe.java"),
    ));
    sources.push((
        "demo/Bench.java",
        include_str!("fixtures/live_menu/demo/Bench.java"),
    ));
    sources.push((
        "demo/BenchMenu.java",
        include_str!("fixtures/live_menu/demo/BenchMenu.java"),
    ));
    sources.push((
        "demo/Types.java",
        include_str!("fixtures/live_menu/types/Types.java"),
    ));
    sources
}

/// `ItemStack.toString()`: `count + " " + item`, `0 minecraft:air` for an empty stack.
fn components_text(stack: &StackComponents) -> String {
    if stack.count <= 0 || stack.item_id == "minecraft:air" {
        "0 minecraft:air".to_owned()
    } else {
        format!("{} {}", stack.count, stack.item_id)
    }
}

fn stack_text(host: &StdHost, stack: &Value) -> String {
    components_text(
        &host
            .item_stack_components(stack)
            .unwrap_or_else(|| panic!("unmodelled stack {stack:?}")),
    )
}

/// The oracle's `effect:` lines (`LiveMenuOracle`'s recorders) for Decaf's effects.
fn effect_lines(effects: &[LiveMenuEffect], out: &mut Vec<String>) {
    for effect in effects {
        let line = match effect {
            LiveMenuEffect::InitialData {
                container_id,
                state_id,
                items,
                carried,
            } => {
                let mut line = format!(
                    "initial {container_id} state={} [",
                    state_id.map_or("?".to_owned(), |id| id.to_string())
                );
                for (index, item) in items.iter().enumerate() {
                    if item.count > 0 && item.item_id != "minecraft:air" {
                        line.push_str(&format!(" {index}:{}", components_text(item)));
                    }
                }
                format!("{line} ] carried={}", components_text(carried))
            }
            LiveMenuEffect::SlotChanged {
                container_id,
                slot,
                stack,
                ..
            } => format!("slot {container_id} {slot} {}", components_text(stack)),
            LiveMenuEffect::CarriedChanged { stack } => {
                format!("carried {}", components_text(stack))
            }
            LiveMenuEffect::DataChanged {
                container_id,
                index,
                value,
            } => format!("data {container_id} {index}={value}"),
            LiveMenuEffect::InventoryChangedCriterion { stack } => {
                format!("criterion {}", components_text(stack))
            }
            LiveMenuEffect::Drop {
                stack,
                retain_ownership,
                prediction,
            } => format!(
                "drop {} retain={retain_ownership} {}",
                components_text(stack),
                match prediction {
                    horizon_native_java::Prediction::Predicted => "PREDICTED",
                    horizon_native_java::Prediction::ServerOnly => "SERVER_ONLY",
                }
            ),
            LiveMenuEffect::AwardStat {
                stat_type,
                value,
                amount,
            } => format!("stat {stat_type} {value} {amount}"),
            LiveMenuEffect::RecipeCrafted { recipe, .. } => format!("recipe_crafted {recipe}"),
            LiveMenuEffect::AwardRecipes { recipes } => format!("award_recipes {recipes:?}"),
            LiveMenuEffect::InventorySlotSent { slot, stack } => {
                format!("inventory_sent {slot} {}", components_text(stack))
            }
            LiveMenuEffect::Closed { container_id } => format!("closed {container_id}"),
        };
        out.push(format!("effect: {line}"));
    }
}

fn state_lines(host: &StdHost, carried: &Value, slots: &[Value], out: &mut Vec<String>) {
    out.push(format!("carried={}", stack_text(host, carried)));
    for (index, slot) in slots.iter().enumerate() {
        if host.item_stack_is_empty(slot) == Some(false) {
            out.push(format!("slot {index}={}", stack_text(host, slot)));
        }
    }
}

fn inventory_line(host: &StdHost, inventory: &[Value], out: &mut Vec<String>) {
    let mut line = "inventory:".to_owned();
    for (index, stack) in inventory.iter().enumerate() {
        if host.item_stack_is_empty(stack) == Some(false) {
            line.push_str(&format!(" {index}={}", stack_text(host, stack)));
        }
    }
    out.push(line);
}

fn player() -> BlockUsePlayerData {
    BlockUsePlayerData {
        uuid: UUID,
        name: "Benchwright".to_owned(),
        entity_id: Some(7),
        incarnation: Some((1, 0)),
        container: PlayerContainer::Inventory,
        sneaking: false,
        spectator: false,
        main_hand: None,
        off_hand: None,
    }
}

/// The player's facts: `LiveMenuOracle`'s fake `ServerPlayer` at (3.5, 65, z) with eye height
/// `1.62f` and a 4.5 block interaction range, facing the bench at `POS`.
fn facts(inventory: Vec<Value>, z: f64, creative: bool) -> MenuPlayerFacts {
    MenuPlayerFacts {
        inventory: Some(inventory),
        selected_slot: 0,
        instabuild: Some(creative),
        eye_position: Some([3.5, 65.0 + f64::from(1.62f32), z]),
        block_interaction_range: Some(4.5),
        access_block: Some(("demo:bench".to_owned(), BTreeMap::new())),
        ..MenuPlayerFacts::default()
    }
}

fn input(name: &str) -> ContainerInput {
    match name {
        "PICKUP" => ContainerInput::Pickup,
        "QUICK_MOVE" => ContainerInput::QuickMove,
        "SWAP" => ContainerInput::Swap,
        "CLONE" => ContainerInput::Clone,
        "THROW" => ContainerInput::Throw,
        "QUICK_CRAFT" => ContainerInput::QuickCraft,
        "PICKUP_ALL" => ContainerInput::PickupAll,
        other => panic!("unknown input {other}"),
    }
}

fn run_script() -> Vec<String> {
    let fixture = javac_fixture::Fixture::build("live_menu", &sources());
    let mut host = StdHost::new();
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap();
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
    host.server_player_created(UUID, 1, 0);

    let empty = StdHost::empty_item_stack();
    let mut inventory = vec![empty; 43];
    inventory[0] = host.make_item_stack_for_id("minecraft:cobblestone", 10, false);
    inventory[1] = host.make_item_stack_for_id("minecraft:amethyst_shard", 3, false);
    inventory[2] = host.make_item_stack_for_id("minecraft:dirt", 5, false);
    let mut z = 7.5;
    let mut creative = false;
    let mut window = 0;
    let budget = ExecutionBudget::UNBOUNDED;
    let mut out = Vec::new();

    for raw in include_str!("fixtures/live_menu/script.txt").lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        out.push(format!("== {line}"));
        let parts: Vec<&str> = line.split(' ').collect();
        let before = host.unknown_host_call_count();
        let state: Option<(Value, Vec<Value>)>;
        match parts[0] {
            "open" => {
                host.set_player_menu_facts(UUID, Some(facts(inventory.clone(), z, creative)));
                let context = BlockUseContext {
                    side: BlockUseSide::Server,
                    dimension: "minecraft:overworld".to_owned(),
                    block_id: "demo:bench".to_owned(),
                    state_properties: BTreeMap::new(),
                    pos: POS,
                    hit: BlockHitData::block(POS, "UP", [3.5, 65.0, 5.5], false),
                    player: player(),
                    hand: "MAIN_HAND".to_owned(),
                    stack: None,
                };
                let outcome = fixture
                    .jar
                    .block_use_without_item(&mut host, &context)
                    .unwrap();
                assert_eq!(outcome.failure, None);
                let opened = outcome
                    .open_menus()
                    .next()
                    .expect("the bench opened its menu");
                window = opened.container_id;
                out.extend(host.take_stdout().lines().map(str::to_owned));
                effect_lines(&outcome.live_effects, &mut out);
                inventory = outcome
                    .inventory
                    .clone()
                    .expect("the facts gave the inventory");
                state = host.held_menu_contents(UUID);
            }
            "click" | "mismatch" => {
                let click = MenuClick {
                    container_id: window,
                    slot: parts[1].parse().unwrap(),
                    button: parts[2].parse().unwrap(),
                    input: input(parts[3]),
                    state_id_matches: parts[0] == "click",
                };
                let mut player = player();
                player.container = PlayerContainer::Open(window);
                let outcome = fixture
                    .jar
                    .click_player_menu(
                        &mut host,
                        &player,
                        "minecraft:overworld",
                        &facts(inventory.clone(), z, creative),
                        &click,
                        &budget,
                    )
                    .unwrap()
                    .expect("the menu is held");
                assert_eq!(outcome.failure, None, "{line}");
                assert!(outcome.handled, "{line}");
                out.extend(host.take_stdout().lines().map(str::to_owned));
                effect_lines(&outcome.effects, &mut out);
                inventory = outcome.inventory.clone().unwrap();
                state = outcome.carried.clone().zip(outcome.slots.clone());
            }
            "tick" | "close" => {
                let mut player = player();
                player.container = PlayerContainer::Open(window);
                let facts = facts(inventory.clone(), z, creative);
                let outcome = if parts[0] == "tick" {
                    fixture.jar.tick_player_menu(
                        &mut host,
                        &player,
                        "minecraft:overworld",
                        &facts,
                        window,
                        &budget,
                    )
                } else {
                    fixture.jar.close_player_menu_live(
                        &mut host,
                        &player,
                        "minecraft:overworld",
                        &facts,
                        window,
                        &budget,
                    )
                }
                .unwrap()
                .expect("the menu is held");
                assert_eq!(outcome.failure, None, "{line}");
                out.extend(host.take_stdout().lines().map(str::to_owned));
                effect_lines(&outcome.effects, &mut out);
                inventory = outcome.inventory.clone().unwrap();
                state = outcome.carried.clone().zip(outcome.slots.clone());
            }
            "far" => {
                z = 15.5;
                state = host.held_menu_contents(UUID);
            }
            "creative" => {
                creative = parts[1] == "true";
                state = host.held_menu_contents(UUID);
            }
            other => panic!("unknown step {other}"),
        }
        assert_eq!(
            host.unknown_host_call_count(),
            before,
            "{line} counted {:?}",
            host.unknown_host_calls()
        );
        if let Some((carried, slots)) = state {
            state_lines(&host, &carried, &slots, &mut out);
        }
        inventory_line(&host, &inventory, &mut out);
    }
    out
}

#[test]
fn live_combiner_menu_matches_java25() {
    let lines = std::thread::Builder::new()
        .name("live-menu".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(run_script)
        .unwrap()
        .join()
        .unwrap();
    let expected: Vec<&str> = include_str!("fixtures/live_menu/java25.txt")
        .lines()
        .collect();
    for (index, (got, want)) in lines.iter().zip(&expected).enumerate() {
        assert_eq!(
            got,
            want,
            "line {} differs; Decaf so far:\n{}",
            index + 1,
            lines[..=index].join("\n")
        );
    }
    assert_eq!(lines.len(), expected.len(), "Decaf:\n{}", lines.join("\n"));
}

/// A drag over both combiner inputs: Java places it in `quickcraftSlots`' `HashSet` order,
/// which follows identity hashes no one can reproduce, and each placement runs the mod's
/// `createResult` (which sees the other input before or after its own). Decaf counts the click
/// instead of picking an order. A drag over plain inventory slots (the script above) is exact.
#[test]
fn drag_over_both_inputs_counts_the_unknowable_order() {
    std::thread::Builder::new()
        .name("live-menu-drag".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let fixture = javac_fixture::Fixture::build("live_menu_drag", &sources());
            let mut host = StdHost::new();
            fixture
                .jar
                .execute("demo.Probe", None, "main", &mut host)
                .unwrap();
            host.server_player_created(UUID, 1, 0);
            let mut inventory = vec![StdHost::empty_item_stack(); 43];
            inventory[0] = host.make_item_stack_for_id("minecraft:cobblestone", 10, false);
            host.set_player_menu_facts(UUID, Some(facts(inventory.clone(), 7.5, false)));
            let context = BlockUseContext {
                side: BlockUseSide::Server,
                dimension: "minecraft:overworld".to_owned(),
                block_id: "demo:bench".to_owned(),
                state_properties: BTreeMap::new(),
                pos: POS,
                hit: BlockHitData::block(POS, "UP", [3.5, 65.0, 5.5], false),
                player: player(),
                hand: "MAIN_HAND".to_owned(),
                stack: None,
            };
            let opened = fixture
                .jar
                .block_use_without_item(&mut host, &context)
                .unwrap();
            let mut inventory = opened.inventory.unwrap();
            let mut player = player();
            player.container = PlayerContainer::Open(1);
            let steps: &[(i32, i32, ContainerInput)] = &[
                (30, 0, ContainerInput::Pickup),
                (-999, 0, ContainerInput::QuickCraft),
                (0, 1, ContainerInput::QuickCraft),
                (1, 1, ContainerInput::QuickCraft),
                (-999, 2, ContainerInput::QuickCraft),
            ];
            let mut last = None;
            for (slot, button, input) in steps {
                let outcome = fixture
                    .jar
                    .click_player_menu(
                        &mut host,
                        &player,
                        "minecraft:overworld",
                        &facts(inventory.clone(), 7.5, false),
                        &MenuClick {
                            container_id: 1,
                            slot: *slot,
                            button: *button,
                            input: *input,
                            state_id_matches: true,
                        },
                        &ExecutionBudget::UNBOUNDED,
                    )
                    .unwrap()
                    .unwrap();
                inventory = outcome.inventory.clone().unwrap();
                last = Some(outcome);
            }
            let end = last.unwrap();
            assert_eq!(
                end.unknown_host_calls,
                BTreeMap::from([(
                    "net/minecraft/server/network/ServerGamePacketListenerImpl#handleContainerClick(Lnet/minecraft/network/protocol/game/ServerboundContainerClickPacket;)V"
                        .to_owned(),
                    1
                )]),
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

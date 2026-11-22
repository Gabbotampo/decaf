//! Integration tests for generic block-use (`Block.useItemOn`, `Block.useWithoutItem`,
//! `Block.getMenuProvider`), the MC 26.3 sealed `InteractionResult` hierarchy,
//! `BlockHitResult`, `SimpleMenuProvider`, `ContainerLevelAccess.create`, and
//! `ServerPlayer.openMenu(MenuProvider)` vs client-side `Player.openMenu`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use std::collections::BTreeMap;

use horizon_native_java::{
    BlockHitData, BlockUseContext, BlockUsePlayerData, BlockUseSide, Host, InteractionItemContext,
    InteractionResultValue, StdHost, SwingSource, Value,
};
use serde_json::json;

const MC_STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/api/ModInitializer.java",
        "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier {\n\
           public static Identifier parse(String s) { return null; }\n\
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
        "net/minecraft/core/Direction.java",
        "package net.minecraft.core;\n\
         public enum Direction { DOWN, UP, NORTH, SOUTH, WEST, EAST }",
    ),
    (
        "net/minecraft/core/Vec3i.java",
        "package net.minecraft.core;\n\
         public class Vec3i {\n\
           public int getX() { return 0; }\n\
           public int getY() { return 0; }\n\
           public int getZ() { return 0; }\n\
         }",
    ),
    (
        "net/minecraft/core/BlockPos.java",
        "package net.minecraft.core;\n\
         public class BlockPos extends Vec3i {\n\
           public static final BlockPos ZERO = null;\n\
           public BlockPos(int x, int y, int z) {}\n\
         }",
    ),
    (
        "net/minecraft/world/phys/Vec3.java",
        "package net.minecraft.world.phys;\n\
         public class Vec3 {\n\
           public final double x = 0, y = 0, z = 0;\n\
           public Vec3(double x, double y, double z) {}\n\
           public double x() { return x; }\n\
           public double y() { return y; }\n\
           public double z() { return z; }\n\
         }",
    ),
    (
        "net/minecraft/world/phys/HitResult.java",
        "package net.minecraft.world.phys;\n\
         public abstract class HitResult {\n\
           public enum Type { MISS, BLOCK, ENTITY }\n\
           public Vec3 getLocation() { return null; }\n\
           public abstract Type getType();\n\
         }",
    ),
    (
        "net/minecraft/world/phys/BlockHitResult.java",
        "package net.minecraft.world.phys;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.core.Direction;\n\
         public class BlockHitResult extends HitResult {\n\
           public static BlockHitResult miss(Vec3 loc, Direction dir, BlockPos pos) { return null; }\n\
           public BlockHitResult(Vec3 loc, Direction dir, BlockPos pos, boolean inside) {}\n\
           public BlockHitResult(Vec3 loc, Direction dir, BlockPos pos, boolean inside, boolean border) {}\n\
           public BlockHitResult withDirection(Direction dir) { return null; }\n\
           public BlockHitResult withPosition(BlockPos pos) { return null; }\n\
           public BlockHitResult hitBorder() { return null; }\n\
           public BlockPos getBlockPos() { return null; }\n\
           public Direction getDirection() { return null; }\n\
           public Type getType() { return Type.BLOCK; }\n\
           public boolean isInside() { return false; }\n\
           public boolean isWorldBorderHit() { return false; }\n\
         }",
    ),
    (
        "net/minecraft/world/InteractionHand.java",
        "package net.minecraft.world;\n\
         public enum InteractionHand { MAIN_HAND, OFF_HAND }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\n\
         public final class ItemStack {\n\
           public static final ItemStack EMPTY = null;\n\
           public boolean isEmpty() { return true; }\n\
         }",
    ),
    (
        "net/minecraft/world/InteractionResult.java",
        "package net.minecraft.world;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public sealed interface InteractionResult\n\
             permits InteractionResult.Success, InteractionResult.Fail,\n\
                     InteractionResult.Pass, InteractionResult.TryEmptyHandInteraction {\n\
           Success SUCCESS = null;\n\
           Success SUCCESS_SERVER = null;\n\
           Success CONSUME = null;\n\
           Fail FAIL = null;\n\
           Pass PASS = null;\n\
           TryEmptyHandInteraction TRY_WITH_EMPTY_HAND = null;\n\
           default boolean consumesAction() { return false; }\n\
           enum SwingSource { NONE, PREDICTED, SERVER_ONLY }\n\
           record ItemContext(boolean wasItemInteraction, ItemStack heldItemTransformedTo) {\n\
             public static final ItemContext NONE = null;\n\
             public static final ItemContext DEFAULT = null;\n\
           }\n\
           record Success(SwingSource swingSource, ItemContext itemContext) implements InteractionResult {\n\
             @Override public boolean consumesAction() { return true; }\n\
             public boolean shouldSwing() { return swingSource != SwingSource.NONE; }\n\
             public Success heldItemTransformedTo(ItemStack stack) { return null; }\n\
             public Success withoutItem() { return null; }\n\
             public boolean wasItemInteraction() { return itemContext.wasItemInteraction(); }\n\
             public ItemStack heldItemTransformedTo() { return itemContext.heldItemTransformedTo(); }\n\
           }\n\
           record Fail() implements InteractionResult {}\n\
           record Pass() implements InteractionResult {}\n\
           record TryEmptyHandInteraction() implements InteractionResult {}\n\
         }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat;\n\
         public interface Component {\n\
           static MutableComponent literal(String s) { return null; }\n\
           static MutableComponent translatable(String key) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat;\n\
         public class MutableComponent implements Component {}",
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
        "net/minecraft/world/Container.java",
        "package net.minecraft.world;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public interface Container {\n\
           int getContainerSize();\n\
           ItemStack getItem(int slot);\n\
           void setItem(int slot, ItemStack stack);\n\
         }",
    ),
    (
        "net/minecraft/world/entity/player/Inventory.java",
        "package net.minecraft.world.entity.player;\n\
         import net.minecraft.world.Container;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public class Inventory implements Container {\n\
           public final Player player = null;\n\
           public int getContainerSize() { return 0; }\n\
           public ItemStack getItem(int slot) { return ItemStack.EMPTY; }\n\
           public void setItem(int slot, ItemStack stack) {}\n\
         }",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         public abstract class Level {\n\
           public boolean isClientSide() { return false; }\n\
           public BlockState getBlockState(BlockPos pos) { return null; }\n\
           public void playSound(Object entity, BlockPos pos, Object sound) {}\n\
         }",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player;\n\
         import java.util.OptionalInt;\n\
         import net.minecraft.world.MenuProvider;\n\
         import net.minecraft.world.level.Level;\n\
         public abstract class Player {\n\
           public Level level() { return null; }\n\
           public Inventory getInventory() { return null; }\n\
           public OptionalInt openMenu(MenuProvider provider) { return OptionalInt.empty(); }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/AbstractContainerMenu.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         public abstract class AbstractContainerMenu {\n\
           protected AbstractContainerMenu(MenuType<?> type, int id) {}\n\
           public MenuType<?> getType() { return null; }\n\
           public abstract boolean stillValid(Player player);\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ContainerLevelAccess.java",
        "package net.minecraft.world.inventory;\n\
         import java.util.Optional;\n\
         import java.util.function.BiConsumer;\n\
         import java.util.function.BiFunction;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.level.Level;\n\
         public interface ContainerLevelAccess {\n\
           ContainerLevelAccess NULL = null;\n\
           static ContainerLevelAccess create(Level level, BlockPos pos) { return null; }\n\
           <T> Optional<T> evaluate(BiFunction<Level, BlockPos, T> fn);\n\
           default <T> T evaluate(BiFunction<Level, BlockPos, T> fn, T fallback) { return fallback; }\n\
           default void execute(BiConsumer<Level, BlockPos> c) {}\n\
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
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         public abstract class ItemCombinerMenu extends AbstractContainerMenu {\n\
           protected final ContainerLevelAccess access;\n\
           protected ItemCombinerMenu(MenuType<?> type, int id, Inventory inv, ContainerLevelAccess access, ItemCombinerMenuSlotDefinition def) {\n\
             super(type, id);\n\
             this.access = access;\n\
           }\n\
           protected abstract boolean isValidBlock(BlockState state);\n\
           public abstract void createResult();\n\
           @Override public boolean stillValid(Player player) { return true; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/MenuType.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.flag.FeatureFlagSet;\n\
         public class MenuType<T extends AbstractContainerMenu> {\n\
           public interface MenuSupplier<T extends AbstractContainerMenu> {\n\
             T create(int containerId, Inventory inv);\n\
           }\n\
           public MenuType(MenuSupplier<T> supplier, FeatureFlagSet features) {}\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/MenuConstructor.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.entity.player.Player;\n\
         @FunctionalInterface\n\
         public interface MenuConstructor {\n\
           AbstractContainerMenu createMenu(int containerId, Inventory inv, Player player);\n\
         }",
    ),
    (
        "net/minecraft/world/MenuProvider.java",
        "package net.minecraft.world;\n\
         import net.minecraft.network.chat.Component;\n\
         import net.minecraft.world.inventory.MenuConstructor;\n\
         public interface MenuProvider extends MenuConstructor {\n\
           Component getDisplayName();\n\
         }",
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
           @Override public Component getDisplayName() { return null; }\n\
           @Override public AbstractContainerMenu createMenu(int id, Inventory inv, Player p) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        "package net.minecraft.world.level.block.state;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.InteractionHand;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.MenuProvider;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.item.ItemStack;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public abstract class BlockBehaviour {\n\
           public static class Properties { public static Properties of() { return new Properties(); } }\n\
           protected BlockBehaviour(Properties props) {}\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             return InteractionResult.PASS;\n\
           }\n\
           protected InteractionResult useItemOn(ItemStack stack, BlockState state, Level level, BlockPos pos, Player player, InteractionHand hand, BlockHitResult hit) {\n\
             return InteractionResult.TRY_WITH_EMPTY_HAND;\n\
           }\n\
           protected MenuProvider getMenuProvider(BlockState state, Level level, BlockPos pos) { return null; }\n\
           public static abstract class BlockStateBase {\n\
             public MenuProvider getMenuProvider(Level level, BlockPos pos) { return null; }\n\
             public InteractionResult useWithoutItem(Level level, Player player, BlockHitResult hit) { return null; }\n\
             public InteractionResult useItemOn(ItemStack stack, Level level, Player player, InteractionHand hand, BlockHitResult hit) { return null; }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/state/BlockState.java",
        "package net.minecraft.world.level.block.state;\n\
         public class BlockState extends BlockBehaviour.BlockStateBase {}",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Block extends BlockBehaviour {\n\
           public Block(Properties props) { super(props); }\n\
         }",
    ),
    (
        "net/minecraft/world/level/block/CraftingTableBlock.java",
        "package net.minecraft.world.level.block;\n\
         public class CraftingTableBlock extends Block {\n\
           public CraftingTableBlock(Properties props) { super(props); }\n\
         }",
    ),
];

fn default_context(block_id: &str, side: BlockUseSide, pos: [i32; 3]) -> BlockUseContext {
    BlockUseContext {
        side,
        dimension: "minecraft:overworld".to_owned(),
        block_id: block_id.to_owned(),
        state_properties: BTreeMap::new(),
        pos,
        hit: BlockHitData::block(
            pos,
            "UP",
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 1.0,
                pos[2] as f64 + 0.5,
            ],
            false,
        ),
        player: BlockUsePlayerData::default(),
        hand: "MAIN_HAND".to_owned(),
        stack: None,
    }
}

#[test]
fn block_use_opens_combiner_menu_via_simple_menu_provider() {
    let mut sources = MC_STUBS.to_vec();
    sources.push((
        "demo/DemoCombinerMenu.java",
        "package demo;\n\
         import net.minecraft.world.entity.player.Inventory;\n\
         import net.minecraft.world.flag.FeatureFlags;\n\
         import net.minecraft.world.inventory.ContainerLevelAccess;\n\
         import net.minecraft.world.inventory.ItemCombinerMenu;\n\
         import net.minecraft.world.inventory.ItemCombinerMenuSlotDefinition;\n\
         import net.minecraft.world.inventory.MenuType;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         public class DemoCombinerMenu extends ItemCombinerMenu {\n\
           public static final MenuType<DemoCombinerMenu> TYPE =\n\
             new MenuType<>(DemoCombinerMenu::new, FeatureFlags.VANILLA_SET);\n\
           public static int lastAccessX = -999;\n\
           public static int lastSyncId = -1;\n\
           public final Level world;\n\
           public DemoCombinerMenu(int id, Inventory inv) {\n\
             this(id, inv, ContainerLevelAccess.NULL);\n\
           }\n\
           public DemoCombinerMenu(int id, Inventory inv, ContainerLevelAccess access) {\n\
             super(TYPE, id, inv, access, createSlots());\n\
             this.world = inv.player.level();\n\
             lastSyncId = id;\n\
             lastAccessX = access.evaluate((lvl, pos) -> pos.getX(), -1);\n\
           }\n\
           private static ItemCombinerMenuSlotDefinition createSlots() {\n\
             return ItemCombinerMenuSlotDefinition.create()\n\
               .withSlot(0, 27, 47, s -> true)\n\
               .withSlot(1, 76, 47, s -> true)\n\
               .withResultSlot(2, 134, 47)\n\
               .build();\n\
           }\n\
           @Override protected boolean isValidBlock(BlockState state) { return true; }\n\
           @Override public void createResult() {}\n\
         }",
    ));
    sources.push((
        "demo/AltarBlock.java",
        "package demo;\n\
         import java.util.OptionalInt;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.network.chat.Component;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.MenuProvider;\n\
         import net.minecraft.world.SimpleMenuProvider;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.inventory.ContainerLevelAccess;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.CraftingTableBlock;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public class AltarBlock extends CraftingTableBlock {\n\
           private static final Component TITLE = Component.translatable(\"gui.demo.combiner\");\n\
           public static int lastReturnedContainerId = -1;\n\
           public AltarBlock(Properties props) { super(props); }\n\
           @Override\n\
           protected MenuProvider getMenuProvider(BlockState state, Level level, BlockPos pos) {\n\
             return new SimpleMenuProvider(\n\
               (syncId, inv, p) -> new DemoCombinerMenu(syncId, inv, ContainerLevelAccess.create(level, pos)),\n\
               TITLE\n\
             );\n\
           }\n\
           @Override\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             if (level.isClientSide()) {\n\
               return InteractionResult.SUCCESS;\n\
             }\n\
             OptionalInt id = player.openMenu(state.getMenuProvider(level, pos));\n\
             lastReturnedContainerId = id.orElse(-1);\n\
             return InteractionResult.CONSUME;\n\
           }\n\
         }",
    ));
    sources.push((
        "demo/Probe.java",
        "package demo;\n\
         import net.fabricmc.api.ModInitializer;\n\
         import net.minecraft.core.Registry;\n\
         import net.minecraft.core.registries.BuiltInRegistries;\n\
         import net.minecraft.resources.Identifier;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Probe implements ModInitializer {\n\
           @Override public void onInitialize() {\n\
             Registry.register(BuiltInRegistries.MENU, Identifier.fromNamespaceAndPath(\"demo\", \"combiner\"), DemoCombinerMenu.TYPE);\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"altar\"), new AltarBlock(BlockBehaviour.Properties.of()));\n\
           }\n\
         }",
    ));

    let fixture = javac_fixture::Fixture::build("block_use_combiner", &sources);
    let mut host = StdHost::new();
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), 0);

    // 1. Inherited BlockBehaviour.useItemOn -> TRY_WITH_EMPTY_HAND (0 unknowns).
    let ctx = default_context("demo:altar", BlockUseSide::Server, [10, 64, -5]);
    let item_on = fixture.jar.block_use_item_on(&mut host, &ctx).unwrap();
    assert_eq!(item_on.unknown_host_call_count, 0);
    assert_eq!(
        item_on.result,
        Some(InteractionResultValue::TryEmptyHandInteraction)
    );
    assert!(item_on.open_menus.is_empty());

    // 2. Server-side useWithoutItem -> CONSUME + typed OpenMenuEffect.
    let server = fixture.jar.block_use_without_item(&mut host, &ctx).unwrap();
    assert_eq!(server.unknown_host_call_count, 0);
    assert_eq!(
        server.result,
        Some(InteractionResultValue::Success {
            swing_source: SwingSource::None,
            item_context: InteractionItemContext {
                was_item_interaction: true,
                held_item_transformed_to: None,
            },
        })
    );
    assert_eq!(server.open_menus.len(), 1);
    let effect = &server.open_menus[0];
    assert_eq!(effect.container_id, 1);
    assert_eq!(effect.menu_type_id.as_deref(), Some("demo:combiner"));
    assert_eq!(
        effect.title_json,
        Some(json!({ "translate": "gui.demo.combiner" }))
    );
    assert_eq!(effect.access_pos, Some([10, 64, -5]));
    assert!(effect.constructor.is_some());
    // Retain & release for W11 live-menu lifetime tracking must balance cleanly.
    let retained_before = host.retained_count();
    host.retain_open_menu_effect(effect);
    assert!(host.retained_count() > retained_before);
    host.release_open_menu_effect(effect);
    assert_eq!(host.retained_count(), retained_before);
    let statics = host.persistent_class_state().0;
    assert_eq!(
        statics.get("demo/AltarBlock#lastReturnedContainerIdI"),
        Some(&Value::Int(1))
    );
    assert_eq!(
        statics.get("demo/DemoCombinerMenu#lastAccessXI"),
        Some(&Value::Int(10))
    );
    assert_eq!(
        statics.get("demo/DemoCombinerMenu#lastSyncIdI"),
        Some(&Value::Int(1))
    );

    // Second server click increments per-player containerCounter to 2.
    let second = fixture.jar.block_use_without_item(&mut host, &ctx).unwrap();
    assert_eq!(second.unknown_host_call_count, 0);
    assert_eq!(second.open_menus.len(), 1);
    assert_eq!(second.open_menus[0].container_id, 2);

    // 3. Client-side useWithoutItem -> SUCCESS + 0 open-menu effects.
    let client_ctx = default_context("demo:altar", BlockUseSide::Client, [10, 64, -5]);
    let client = fixture
        .jar
        .block_use_without_item(&mut host, &client_ctx)
        .unwrap();
    assert_eq!(client.unknown_host_call_count, 0);
    assert_eq!(
        client.result,
        Some(InteractionResultValue::Success {
            swing_source: SwingSource::Predicted,
            item_context: InteractionItemContext {
                was_item_interaction: true,
                held_item_transformed_to: None,
            },
        })
    );
    assert!(client.open_menus.is_empty());
}

#[test]
fn block_use_returns_every_interaction_result_kind() {
    let mut sources = MC_STUBS.to_vec();
    sources.push((
        "demo/ResultBlock.java",
        "package demo;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.core.Direction;\n\
         import net.minecraft.world.InteractionHand;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.item.ItemStack;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.Block;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         import net.minecraft.world.phys.HitResult;\n\
         public class ResultBlock extends Block {\n\
           public static int checksPassed = 0;\n\
           public ResultBlock(Properties props) { super(props); }\n\
           private static String classify(InteractionResult r) {\n\
             return switch (r) {\n\
               case InteractionResult.Success s -> \"S:\" + s.shouldSwing() + \":\" + s.wasItemInteraction();\n\
               case InteractionResult.Fail f -> \"F:\" + f.consumesAction();\n\
               case InteractionResult.Pass p -> \"P:\" + p.consumesAction();\n\
               case InteractionResult.TryEmptyHandInteraction t -> \"T:\" + t.consumesAction();\n\
             };\n\
           }\n\
           @Override\n\
           protected InteractionResult useItemOn(ItemStack stack, BlockState state, Level level, BlockPos pos, Player player, InteractionHand hand, BlockHitResult hit) {\n\
             if (hit.getType() != HitResult.Type.BLOCK || hit.getDirection() != Direction.UP || hit.isInside() || hit.isWorldBorderHit()) {\n\
               throw new AssertionError(\"unexpected hit\");\n\
             }\n\
             BlockHitResult shifted = hit.withDirection(Direction.NORTH).hitBorder();\n\
             if (shifted.getDirection() != Direction.NORTH || !shifted.isWorldBorderHit()) {\n\
               throw new AssertionError(\"unexpected shifted hit\");\n\
             }\n\
             if (!classify(InteractionResult.SUCCESS).equals(\"S:true:true\")) throw new AssertionError();\n\
             if (!classify(InteractionResult.CONSUME).equals(\"S:false:true\")) throw new AssertionError();\n\
             if (!classify(InteractionResult.FAIL).equals(\"F:false\")) throw new AssertionError();\n\
             if (!classify(InteractionResult.PASS).equals(\"P:false\")) throw new AssertionError();\n\
             if (!classify(InteractionResult.TRY_WITH_EMPTY_HAND).equals(\"T:false\")) throw new AssertionError();\n\
             InteractionResult.Success fresh = new InteractionResult.Success(InteractionResult.SwingSource.NONE, InteractionResult.ItemContext.NONE);\n\
             if (!fresh.equals(InteractionResult.CONSUME.withoutItem())) throw new AssertionError();\n\
             if (fresh == InteractionResult.CONSUME.withoutItem()) throw new AssertionError();\n\
             if (!InteractionResult.FAIL.toString().equals(\"Fail[]\")) throw new AssertionError();\n\
             if (InteractionResult.PASS.hashCode() != 0) throw new AssertionError();\n\
             if (InteractionResult.ItemContext.DEFAULT.hashCode() != 31 * 1231) throw new AssertionError();\n\
             checksPassed++;\n\
             return switch (pos.getX()) {\n\
               case 0 -> InteractionResult.SUCCESS;\n\
               case 1 -> InteractionResult.SUCCESS_SERVER;\n\
               case 2 -> InteractionResult.CONSUME;\n\
               case 3 -> InteractionResult.FAIL;\n\
               case 4 -> InteractionResult.PASS;\n\
               case 5 -> InteractionResult.TRY_WITH_EMPTY_HAND;\n\
               case 6 -> InteractionResult.SUCCESS.withoutItem();\n\
               case 7 -> InteractionResult.SUCCESS_SERVER.heldItemTransformedTo(stack);\n\
               default -> fresh;\n\
             };\n\
           }\n\
         }",
    ));
    sources.push((
        "demo/Probe.java",
        "package demo;\n\
         import net.fabricmc.api.ModInitializer;\n\
         import net.minecraft.core.Registry;\n\
         import net.minecraft.core.registries.BuiltInRegistries;\n\
         import net.minecraft.resources.Identifier;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Probe implements ModInitializer {\n\
           @Override public void onInitialize() {\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"results\"), new ResultBlock(BlockBehaviour.Properties.of()));\n\
           }\n\
         }",
    ));

    let fixture = javac_fixture::Fixture::build("block_use_results", &sources);
    let mut host = StdHost::new();
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), 0);

    let cases: &[(i32, InteractionResultValue)] = &[
        (
            0,
            InteractionResultValue::Success {
                swing_source: SwingSource::Predicted,
                item_context: InteractionItemContext {
                    was_item_interaction: true,
                    held_item_transformed_to: None,
                },
            },
        ),
        (
            1,
            InteractionResultValue::Success {
                swing_source: SwingSource::ServerOnly,
                item_context: InteractionItemContext {
                    was_item_interaction: true,
                    held_item_transformed_to: None,
                },
            },
        ),
        (
            2,
            InteractionResultValue::Success {
                swing_source: SwingSource::None,
                item_context: InteractionItemContext {
                    was_item_interaction: true,
                    held_item_transformed_to: None,
                },
            },
        ),
        (3, InteractionResultValue::Fail),
        (4, InteractionResultValue::Pass),
        (5, InteractionResultValue::TryEmptyHandInteraction),
        (
            6,
            InteractionResultValue::Success {
                swing_source: SwingSource::Predicted,
                item_context: InteractionItemContext {
                    was_item_interaction: false,
                    held_item_transformed_to: None,
                },
            },
        ),
        (
            8,
            InteractionResultValue::Success {
                swing_source: SwingSource::None,
                item_context: InteractionItemContext {
                    was_item_interaction: false,
                    held_item_transformed_to: None,
                },
            },
        ),
    ];

    for (x, expected) in cases {
        let ctx = default_context("demo:results", BlockUseSide::Server, [*x, 64, 0]);
        let outcome = fixture.jar.block_use_item_on(&mut host, &ctx).unwrap();
        assert_eq!(outcome.unknown_host_call_count, 0, "x={x}");
        assert_eq!(outcome.result.as_ref(), Some(expected), "x={x}");
    }

    // Case 7: heldItemTransformedTo(stack) preserves the transformed ItemStack value.
    let ctx7 = default_context("demo:results", BlockUseSide::Server, [7, 64, 0]);
    let outcome7 = fixture.jar.block_use_item_on(&mut host, &ctx7).unwrap();
    assert_eq!(outcome7.unknown_host_call_count, 0);
    match outcome7.result {
        Some(InteractionResultValue::Success {
            swing_source: SwingSource::ServerOnly,
            item_context:
                InteractionItemContext {
                    was_item_interaction: true,
                    held_item_transformed_to: Some(_),
                },
        }) => {}
        other => panic!("unexpected case 7 result: {other:?}"),
    }
    assert_eq!(
        host.persistent_class_state()
            .0
            .get("demo/ResultBlock#checksPassedI"),
        Some(&Value::Int(9))
    );
}

#[test]
fn block_use_client_side_only_and_client_open_menu_noop() {
    let mut sources = MC_STUBS.to_vec();
    sources.push((
        "demo/ClientOnlyBlock.java",
        "package demo;\n\
         import java.util.OptionalInt;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.network.chat.Component;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.SimpleMenuProvider;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.Block;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public class ClientOnlyBlock extends Block {\n\
           public static int clientOpenEmpty = 0;\n\
           public ClientOnlyBlock(Properties props) { super(props); }\n\
           @Override\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             if (!level.isClientSide()) {\n\
               return InteractionResult.PASS;\n\
             }\n\
             OptionalInt res = player.openMenu(new SimpleMenuProvider((id, inv, p) -> null, Component.literal(\"x\")));\n\
             if (res.isEmpty()) clientOpenEmpty++;\n\
             return InteractionResult.SUCCESS;\n\
           }\n\
         }",
    ));
    sources.push((
        "demo/Probe.java",
        "package demo;\n\
         import net.fabricmc.api.ModInitializer;\n\
         import net.minecraft.core.Registry;\n\
         import net.minecraft.core.registries.BuiltInRegistries;\n\
         import net.minecraft.resources.Identifier;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Probe implements ModInitializer {\n\
           @Override public void onInitialize() {\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"client_only\"), new ClientOnlyBlock(BlockBehaviour.Properties.of()));\n\
           }\n\
         }",
    ));

    let fixture = javac_fixture::Fixture::build("block_use_client_only", &sources);
    let mut host = StdHost::new();
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap();

    let server = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:client_only", BlockUseSide::Server, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(server.unknown_host_call_count, 0);
    assert_eq!(server.result, Some(InteractionResultValue::Pass));
    assert!(server.open_menus.is_empty());

    let client = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:client_only", BlockUseSide::Client, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(client.unknown_host_call_count, 0);
    assert_eq!(
        client.result,
        Some(InteractionResultValue::Success {
            swing_source: SwingSource::Predicted,
            item_context: InteractionItemContext {
                was_item_interaction: true,
                held_item_transformed_to: None,
            },
        })
    );
    assert!(client.open_menus.is_empty());
    assert_eq!(
        host.persistent_class_state()
            .0
            .get("demo/ClientOnlyBlock#clientOpenEmptyI"),
        Some(&Value::Int(1))
    );
}

#[test]
fn block_use_counts_unmodelled_calls_and_unmodelled_superclasses() {
    let mut sources = MC_STUBS.to_vec();
    sources.push((
        "demo/UnmodelledCallBlock.java",
        "package demo;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.Block;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public class UnmodelledCallBlock extends Block {\n\
           public UnmodelledCallBlock(Properties props) { super(props); }\n\
           @Override\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             // Reading the clicked pos is modelled; reading a neighbour pos or playSound must COUNT.\n\
             if (level.getBlockState(pos) == null) throw new AssertionError();\n\
             level.getBlockState(BlockPos.ZERO);\n\
             level.playSound(player, pos, null);\n\
             return InteractionResult.CONSUME;\n\
           }\n\
         }",
    ));
    sources.push((
        "demo/UnoverriddenCraftingBlock.java",
        "package demo;\n\
         import net.minecraft.world.level.block.CraftingTableBlock;\n\
         public class UnoverriddenCraftingBlock extends CraftingTableBlock {\n\
           public UnoverriddenCraftingBlock(Properties props) { super(props); }\n\
         }",
    ));
    sources.push((
        "demo/PlainBlock.java",
        "package demo;\n\
         import net.minecraft.world.level.block.Block;\n\
         public class PlainBlock extends Block {\n\
           public PlainBlock(Properties props) { super(props); }\n\
         }",
    ));
    sources.push((
        "demo/NullMenuBlock.java",
        "package demo;\n\
         import net.minecraft.core.BlockPos;\n\
         import net.minecraft.network.chat.Component;\n\
         import net.minecraft.world.InteractionResult;\n\
         import net.minecraft.world.SimpleMenuProvider;\n\
         import net.minecraft.world.entity.player.Player;\n\
         import net.minecraft.world.level.Level;\n\
         import net.minecraft.world.level.block.Block;\n\
         import net.minecraft.world.level.block.state.BlockState;\n\
         import net.minecraft.world.phys.BlockHitResult;\n\
         public class NullMenuBlock extends Block {\n\
           public NullMenuBlock(Properties props) { super(props); }\n\
           @Override\n\
           protected InteractionResult useWithoutItem(BlockState state, Level level, BlockPos pos, Player player, BlockHitResult hit) {\n\
             if (player.openMenu(null).isPresent()) throw new AssertionError();\n\
             if (player.openMenu(new SimpleMenuProvider((id, inv, p) -> null, Component.literal(\"x\"))).isPresent()) throw new AssertionError();\n\
             return InteractionResult.PASS;\n\
           }\n\
         }",
    ));
    sources.push((
        "demo/Probe.java",
        "package demo;\n\
         import net.fabricmc.api.ModInitializer;\n\
         import net.minecraft.core.Registry;\n\
         import net.minecraft.core.registries.BuiltInRegistries;\n\
         import net.minecraft.resources.Identifier;\n\
         import net.minecraft.world.level.block.state.BlockBehaviour;\n\
         public class Probe implements ModInitializer {\n\
           @Override public void onInitialize() {\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"unknown_call\"), new UnmodelledCallBlock(BlockBehaviour.Properties.of()));\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"crafting_sub\"), new UnoverriddenCraftingBlock(BlockBehaviour.Properties.of()));\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"plain\"), new PlainBlock(BlockBehaviour.Properties.of()));\n\
             Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath(\"demo\", \"null_menu\"), new NullMenuBlock(BlockBehaviour.Properties.of()));\n\
           }\n\
         }",
    ));

    let fixture = javac_fixture::Fixture::build("block_use_unknowns", &sources);
    let mut host = StdHost::new();
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap();

    // 1. Unmodelled calls inside useWithoutItem COUNT.
    let unk = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:unknown_call", BlockUseSide::Server, [5, 70, 5]),
        )
        .unwrap();
    assert_eq!(unk.unknown_host_call_count, 2);
    assert!(
        unk.unknown_host_calls
            .keys()
            .any(|k| k.contains("getBlockState")),
        "expected getBlockState to be counted: {:?}",
        unk.unknown_host_calls
    );
    assert!(
        unk.unknown_host_calls
            .keys()
            .any(|k| k.contains("playSound")),
        "expected playSound to be counted: {:?}",
        unk.unknown_host_calls
    );

    // 2. Inherited CraftingTableBlock.useItemOn is allow-listed (0 unknowns),
    //    but CraftingTableBlock.useWithoutItem is NOT allow-listed -> COUNTED.
    let craft_item = fixture
        .jar
        .block_use_item_on(
            &mut host,
            &default_context("demo:crafting_sub", BlockUseSide::Server, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(craft_item.unknown_host_call_count, 0);
    assert_eq!(
        craft_item.result,
        Some(InteractionResultValue::TryEmptyHandInteraction)
    );
    let craft_empty = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:crafting_sub", BlockUseSide::Server, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(craft_empty.unknown_host_call_count, 1);
    assert!(
        craft_empty
            .unknown_host_calls
            .keys()
            .any(|k| k.contains("CraftingTableBlock#useWithoutItem")),
        "expected CraftingTableBlock#useWithoutItem to be counted: {:?}",
        craft_empty.unknown_host_calls
    );

    // 3. Plain Block subclass inherits BlockBehaviour defaults cleanly with 0 unknowns.
    let plain = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:plain", BlockUseSide::Server, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(plain.unknown_host_call_count, 0);
    assert_eq!(plain.result, Some(InteractionResultValue::Pass));

    // 4. Null provider or provider.createMenu returning null returns OptionalInt.empty() with 0 effects & 0 unknowns.
    let null_menu = fixture
        .jar
        .block_use_without_item(
            &mut host,
            &default_context("demo:null_menu", BlockUseSide::Server, [1, 2, 3]),
        )
        .unwrap();
    assert_eq!(null_menu.unknown_host_call_count, 0);
    assert!(null_menu.open_menus.is_empty());
    assert_eq!(null_menu.result, Some(InteractionResultValue::Pass));
}

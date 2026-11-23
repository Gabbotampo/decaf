//! Fabric creative-tab modify-output callbacks (`host_creative_tab.rs`, `events.rs`).
//!
//! `fixtures/creative_tab_modify_output/java25.txt` is captured by running
//! `oracle/CaptureCreativeTabOutput.java` on Java 25 against the real
//! `fabric-creative-tab-api-v1` (5.0.21), `fabric-api-base` (2.0.6), and Minecraft 26.3
//! server jar. Every row in the javac fixture below is compared against that capture with
//! zero unknown host calls.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{CreativeTabBuildContext, NativeJar, StdHost, Value};
use std::collections::BTreeMap;
use std::path::Path;

const ORACLE: &str = include_str!("fixtures/creative_tab_modify_output/java25.txt");

fn oracle_rows() -> BTreeMap<&'static str, &'static str> {
    ORACLE
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split_once('=')
                .unwrap_or_else(|| panic!("bad oracle line: {line}"))
        })
        .collect()
}

/// Compile-only stubs mirroring the real Minecraft 26.3 + Fabric API 5.0.21 signatures.
/// `javac_fixture::Fixture::build` ships only `demo/` classes into the jar, so every call
/// into `net/minecraft/*` and `net/fabricmc/*` crosses the native host boundary.
const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources;
public final class Identifier {
    public static Identifier parse(String s) { throw new AssertionError(); }
    public static Identifier fromNamespaceAndPath(String ns, String path) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        r#"package net.minecraft.resources;
public final class ResourceKey<T> {
    public static <T> ResourceKey<T> createRegistryKey(Identifier id) { throw new AssertionError(); }
    public static <T> ResourceKey<T> create(ResourceKey<?> registry, Identifier id) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        r#"package net.minecraft.core.registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.block.Block;
public final class Registries {
    public static final ResourceKey<Object> ITEM = null;
    public static final ResourceKey<Object> BLOCK = null;
    public static final ResourceKey<Object> CREATIVE_MODE_TAB = null;
}"#,
    ),
    (
        "net/minecraft/core/Registry.java",
        r#"package net.minecraft.core;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
public interface Registry<T> {
    static <V, T extends V> T register(Registry<V> registry, Identifier id, T value) { throw new AssertionError(); }
    static <V, T extends V> T register(Registry<V> registry, ResourceKey<V> key, T value) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/core/DefaultedRegistry.java",
        r#"package net.minecraft.core;
public interface DefaultedRegistry<T> extends Registry<T> {}"#,
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        r#"package net.minecraft.core.registries;
import net.minecraft.core.DefaultedRegistry;
import net.minecraft.core.Registry;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.block.Block;
public final class BuiltInRegistries {
    public static final DefaultedRegistry<Block> BLOCK = null;
    public static final DefaultedRegistry<Item> ITEM = null;
    public static final Registry<CreativeModeTab> CREATIVE_MODE_TAB = null;
}"#,
    ),
    (
        "net/minecraft/network/chat/Component.java",
        r#"package net.minecraft.network.chat;
public interface Component {
    static MutableComponent literal(String s) { throw new AssertionError(); }
    String getString();
}"#,
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        r#"package net.minecraft.network.chat;
public interface MutableComponent extends Component {}"#,
    ),
    (
        "net/minecraft/world/flag/FeatureFlag.java",
        r#"package net.minecraft.world.flag;
public class FeatureFlag {}"#,
    ),
    (
        "net/minecraft/world/flag/FeatureFlagSet.java",
        r#"package net.minecraft.world.flag;
public final class FeatureFlagSet {
    public static FeatureFlagSet of() { throw new AssertionError(); }
    public static FeatureFlagSet of(FeatureFlag flag) { throw new AssertionError(); }
    public static FeatureFlagSet of(FeatureFlag flag, FeatureFlag... otherFlags) { throw new AssertionError(); }
    public boolean isEmpty() { throw new AssertionError(); }
    public boolean contains(FeatureFlag flag) { throw new AssertionError(); }
    public boolean isSubsetOf(FeatureFlagSet set) { throw new AssertionError(); }
    public boolean intersects(FeatureFlagSet set) { throw new AssertionError(); }
    public FeatureFlagSet join(FeatureFlagSet other) { throw new AssertionError(); }
    public FeatureFlagSet subtract(FeatureFlagSet other) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/flag/FeatureFlags.java",
        r#"package net.minecraft.world.flag;
public final class FeatureFlags {
    public static final FeatureFlag VANILLA = null;
    public static final FeatureFlag TRADE_REBALANCE = null;
    public static final FeatureFlag REDSTONE_EXPERIMENTS = null;
    public static final FeatureFlag MINECART_IMPROVEMENTS = null;
    public static final FeatureFlagSet VANILLA_SET = null;
    public static final FeatureFlagSet DEFAULT_FLAGS = null;
}"#,
    ),
    (
        "net/minecraft/world/flag/FeatureElement.java",
        r#"package net.minecraft.world.flag;
public interface FeatureElement {
    FeatureFlagSet requiredFeatures();
    default boolean isEnabled(FeatureFlagSet enabledFeatures) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/level/ItemLike.java",
        r#"package net.minecraft.world.level;
import net.minecraft.world.item.Item;
public interface ItemLike {
    Item asItem();
}"#,
    ),
    (
        "net/minecraft/world/entity/EntityType.java",
        r#"package net.minecraft.world.entity;
public class EntityType<T> {}"#,
    ),
    (
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        r#"package net.minecraft.world.level.block.state;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.level.block.Block;
public abstract class BlockBehaviour {
    public static class Properties {
        public static Properties of() { throw new AssertionError(); }
        public Properties setId(ResourceKey<Block> id) { throw new AssertionError(); }
    }
}"#,
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        r#"package net.minecraft.world.level.block;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.ItemLike;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class Block extends BlockBehaviour implements ItemLike {
    public Block(BlockBehaviour.Properties properties) {}
    @Override public Item asItem() { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/level/block/Blocks.java",
        r#"package net.minecraft.world.level.block;
public final class Blocks {
    public static final Block STONE = null;
}"#,
    ),
    (
        "net/minecraft/world/item/Item.java",
        r#"package net.minecraft.world.item;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.flag.FeatureElement;
import net.minecraft.world.flag.FeatureFlag;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.level.ItemLike;
public class Item implements ItemLike, FeatureElement {
    public static class Properties {
        public Properties() {}
        public Properties setId(ResourceKey<Item> id) { throw new AssertionError(); }
        public Properties requiredFeatures(FeatureFlag... flags) { throw new AssertionError(); }
        public Properties requiredFeatures(FeatureFlagSet flags) { throw new AssertionError(); }
        public Properties spawnEgg(EntityType<?> entityType) { throw new AssertionError(); }
    }
    public Item(Properties properties) {}
    @Override public Item asItem() { return this; }
    @Override public final FeatureFlagSet requiredFeatures() { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/item/BlockItem.java",
        r#"package net.minecraft.world.item;
import net.minecraft.world.level.block.Block;
public class BlockItem extends Item {
    public BlockItem(Block block, Item.Properties properties) { super(properties); }
}"#,
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        r#"package net.minecraft.world.item;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.level.ItemLike;
public final class ItemStack {
    public static final ItemStack EMPTY = null;
    public ItemStack(ItemLike item) {}
    public ItemStack(ItemLike item, int count) {}
    public boolean isEmpty() { throw new AssertionError(); }
    public int getCount() { throw new AssertionError(); }
    public Item getItem() { throw new AssertionError(); }
    public boolean is(Item item) { throw new AssertionError(); }
    public boolean isItemEnabled(FeatureFlagSet enabledFeatures) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/item/CreativeModeTab.java",
        r#"package net.minecraft.world.item;
import net.minecraft.network.chat.Component;
import net.minecraft.world.level.ItemLike;
public class CreativeModeTab {
    public enum TabVisibility {
        PARENT_AND_SEARCH_TABS,
        PARENT_TAB_ONLY,
        SEARCH_TAB_ONLY
    }
    public interface Output {
        void accept(ItemStack stack, TabVisibility visibility);
        default void accept(ItemStack stack) { accept(stack, TabVisibility.PARENT_AND_SEARCH_TABS); }
        default void accept(ItemLike item, TabVisibility visibility) { accept(new ItemStack(item), visibility); }
        default void accept(ItemLike item) { accept(new ItemStack(item), TabVisibility.PARENT_AND_SEARCH_TABS); }
    }
    public static class Builder {
        public Builder title(Component title) { throw new AssertionError(); }
        public Builder alignedRight() { throw new AssertionError(); }
        public CreativeModeTab build() { throw new AssertionError(); }
    }
    public Component getDisplayName() { throw new AssertionError(); }
    public boolean isAlignedRight() { throw new AssertionError(); }
}"#,
    ),
    (
        "net/minecraft/world/item/CreativeModeTabs.java",
        r#"package net.minecraft.world.item;
import net.minecraft.resources.ResourceKey;
public final class CreativeModeTabs {
    public static final ResourceKey<CreativeModeTab> FUNCTIONAL_BLOCKS = null;
    public static final ResourceKey<CreativeModeTab> COMBAT = null;
    public static final ResourceKey<CreativeModeTab> HOTBAR = null;
    public static final ResourceKey<CreativeModeTab> OP_BLOCKS = null;
}"#,
    ),
    (
        "net/fabricmc/fabric/api/event/Event.java",
        r#"package net.fabricmc.fabric.api.event;
import net.minecraft.resources.Identifier;
public abstract class Event<T> {
    public static final Identifier DEFAULT_PHASE = null;
    public abstract void register(T listener);
    public abstract void register(Identifier phase, T listener);
    public abstract void addPhaseOrdering(Identifier firstPhase, Identifier secondPhase);
}"#,
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTab.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import net.minecraft.world.item.CreativeModeTab;
public final class FabricCreativeModeTab {
    public static CreativeModeTab.Builder builder() { throw new AssertionError(); }
}"#,
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import java.util.Collection;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ItemLike;
public class FabricCreativeModeTabOutput implements CreativeModeTab.Output {
    public FeatureFlagSet getEnabledFeatures() { throw new AssertionError(); }
    public boolean shouldShowOpRestrictedItems() { throw new AssertionError(); }
    @Override public void accept(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void prepend(ItemStack stack) { throw new AssertionError(); }
    public void prepend(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void prepend(ItemLike item) { throw new AssertionError(); }
    public void prepend(ItemLike item, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, ItemStack... stacks) { throw new AssertionError(); }
    public void insertBefore(ItemStack anchor, ItemStack... stacks) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertBefore(ItemStack anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, Collection<ItemStack> stacks) { throw new AssertionError(); }
    public void insertBefore(ItemStack anchor, Collection<ItemStack> stacks) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemStack anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, ItemStack... stacks) { throw new AssertionError(); }
    public void insertAfter(ItemStack anchor, ItemStack... stacks) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertAfter(ItemStack anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, Collection<ItemStack> stacks) { throw new AssertionError(); }
    public void insertAfter(ItemStack anchor, Collection<ItemStack> stacks) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertAfter(ItemStack anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import net.fabricmc.fabric.api.event.Event;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.CreativeModeTab;
public final class CreativeModeTabEvents {
    public static final Event<ModifyOutputAll> MODIFY_OUTPUT_ALL = null;
    public interface ModifyOutput {
        void modifyOutput(FabricCreativeModeTabOutput output);
    }
    public interface ModifyOutputAll {
        void modifyOutput(CreativeModeTab tab, FabricCreativeModeTabOutput output);
    }
    public static Event<ModifyOutput> modifyOutputEvent(ResourceKey<CreativeModeTab> tabKey) { throw new AssertionError(); }
}"#,
    ),
];

const ORACLE_PROBE: &str = r##"
package demo;

import java.util.ArrayList;
import java.util.List;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTabOutput;
import net.fabricmc.fabric.api.event.Event;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.flag.FeatureFlag;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ItemLike;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockBehaviour;

public final class Probe {
    static Item enabledA;
    static Item enabledB;
    static Item enabledC;
    static Block altarBlock;
    static BlockItem altarItem;
    static Item disabledItem;
    static ItemLike customHolder;
    static CreativeModeTab primaryTab;

    static final List<String> order = new ArrayList<>();
    static FabricCreativeModeTabOutput seenOutput;
    static boolean sharedPerTab;
    static boolean sharedAll;
    static boolean tabSame;
    static String tabTitle;
    static boolean dynamicAdded;
    static final String[] errs = new String[7];
    static boolean out1EnabledMatchesVanilla;
    static boolean out1ShowOp;
    static String tradeCount2Err;
    static String emptyFlagsEmptyStack;

    static ResourceKey<CreativeModeTab> tabKey(String id) {
        return ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse(id));
    }

    static String catchErr(Runnable r) {
        try {
            r.run();
            return "NONE";
        } catch (Throwable t) {
            return t.getClass().getName() + ": " + t.getMessage();
        }
    }

    static String catchClass(Runnable r) {
        try {
            r.run();
            return "NONE";
        } catch (Throwable t) {
            return t.getClass().getName();
        }
    }

    static final class CustomHolder implements ItemLike {
        @Override
        public Item asItem() {
            return enabledC;
        }
    }

    public static void setup() {
        ResourceKey<Item> keyA = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_a"));
        ResourceKey<Item> keyB = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_b"));
        ResourceKey<Item> keyC = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_c"));
        ResourceKey<Block> keyAltarBlock = ResourceKey.create(Registries.BLOCK, Identifier.parse("demo:altar"));
        ResourceKey<Item> keyAltarItem = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:altar"));
        ResourceKey<Item> keyDisabled = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:disabled_trade"));

        enabledA = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:enabled_a"),
                new Item(new Item.Properties().setId(keyA)));
        enabledB = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:enabled_b"),
                new Item(new Item.Properties().setId(keyB)));
        enabledC = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:enabled_c"),
                new Item(new Item.Properties().setId(keyC)));
        altarBlock = Registry.register(BuiltInRegistries.BLOCK, Identifier.parse("demo:altar"),
                new Block(BlockBehaviour.Properties.of().setId(keyAltarBlock)));
        altarItem = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:altar"),
                new BlockItem(altarBlock, new Item.Properties().setId(keyAltarItem)));
        disabledItem = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:disabled_trade"),
                new Item(new Item.Properties().setId(keyDisabled).requiredFeatures(FeatureFlags.TRADE_REBALANCE)));
        customHolder = new CustomHolder();

        ResourceKey<CreativeModeTab> primaryKey = tabKey("demo:primary");
        primaryTab = FabricCreativeModeTab.builder().title(Component.literal("Primary Tab")).build();
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB, primaryKey, primaryTab);

        Identifier earlyPhase = Identifier.parse("demo:early");
        Identifier latePhase = Identifier.parse("demo:late");
        Identifier dynamicPhase = Identifier.parse("demo:dynamic");
        Identifier allFromPerTabPhase = Identifier.parse("demo:all_from_per_tab");
        Identifier allLatePhase = Identifier.parse("demo:all_late");

        Event<CreativeModeTabEvents.ModifyOutput> primaryEvent =
                CreativeModeTabEvents.modifyOutputEvent(primaryKey);
        primaryEvent.addPhaseOrdering(earlyPhase, Event.DEFAULT_PHASE);
        primaryEvent.addPhaseOrdering(Event.DEFAULT_PHASE, latePhase);
        primaryEvent.addPhaseOrdering(latePhase, dynamicPhase);

        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.addPhaseOrdering(Event.DEFAULT_PHASE, allFromPerTabPhase);
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.addPhaseOrdering(allFromPerTabPhase, allLatePhase);

        // Register late BEFORE default and early to test phase ordering.
        primaryEvent.register(latePhase, output -> {
            order.add("late");
            sharedPerTab = (output == seenOutput);
            if (!dynamicAdded) {
                dynamicAdded = true;
                primaryEvent.register(dynamicPhase, o2 -> order.add("dynamic"));
                // Registered during per-tab dispatch; runs in this SAME firing because
                // CreativeModeTabMixin.getStacks snapshots MODIFY_OUTPUT_ALL after per-tab.
                CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register(allFromPerTabPhase, (tab, o2) -> {
                    order.add("all_from_per_tab");
                });
            }
        });
        primaryEvent.register(output -> {
            order.add("default1");
            output.accept(enabledA);
            output.accept(Blocks.STONE);
            output.accept(customHolder, CreativeModeTab.TabVisibility.PARENT_TAB_ONLY);
            output.accept(new ItemStack(altarItem), CreativeModeTab.TabVisibility.SEARCH_TAB_ONLY);
            // Disabled item: silently skipped before checkStack and visibility check.
            output.accept(disabledItem);
            output.accept(new ItemStack(disabledItem, 2));
            output.accept(new ItemStack(disabledItem, 2), null);
        });
        primaryEvent.register(output -> {
            order.add("default2");
            output.accept(altarBlock);
            if (errs[0] == null) {
                errs[0] = catchErr(() -> output.accept(new ItemStack(enabledA, 2)));
                errs[1] = catchErr(() -> output.accept(ItemStack.EMPTY));
                errs[2] = catchErr(() -> output.accept(new ItemStack(enabledA, 0)));
                errs[3] = catchErr(() -> output.accept(new ItemStack(enabledA, -1)));
                errs[4] = catchClass(() -> output.accept((ItemStack) null));
                errs[5] = catchClass(() -> output.accept((ItemLike) null));
                errs[6] = catchClass(() -> output.accept(new ItemStack(enabledA), null));
            }
        });
        primaryEvent.register(earlyPhase, output -> {
            order.add("early");
            seenOutput = output;
            out1EnabledMatchesVanilla = output.getEnabledFeatures().equals(FeatureFlags.VANILLA_SET);
            out1ShowOp = output.shouldShowOpRestrictedItems();
            output.accept(new ItemStack(enabledB));
        });

        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register(allLatePhase, (tab, output) -> {
            order.add("all_late");
            sharedAll = (output == seenOutput);
            tabSame = (tab == primaryTab);
            output.accept(enabledC);
        });
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register((tab, output) -> {
            order.add("all_default");
            if (tab == primaryTab) {
                tabTitle = tab.getDisplayName().getString();
            }
        });

        // Trade-enabled tab: only accepts disabledItem and probes count=2 error.
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:trade_tab")).register(output -> {
            output.accept(disabledItem);
            tradeCount2Err = catchErr(() -> output.accept(new ItemStack(disabledItem, 2)));
        });
        // Empty-features tab: accepts ItemStack.EMPTY (must skip because Items.AIR requires VANILLA).
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:empty_flags_tab")).register(output -> {
            emptyFlagsEmptyStack = catchErr(() -> output.accept(ItemStack.EMPTY));
        });
        // Unbuilt tab: never built, so its registration must remain in game_listeners().unsupported.
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:unbuilt")).register(output -> {
            output.accept(enabledA);
        });
    }

    public static void clearOrder() {
        order.clear();
    }

    public static String orderJoined() {
        return String.join(",", order);
    }

    public static String formatStack(ItemStack s) {
        return s.toString() + "#" + s.getCount() + "#" + s.isEmpty() + "#" + s.getItem().toString();
    }

    public static String firing1Summary() {
        return "firing1_order=" + String.join(",", order) + "\n"
                + "firing1_shared=" + (sharedPerTab && sharedAll) + "\n"
                + "firing1_tab_same=" + tabSame + "\n"
                + "firing1_tab_title=" + tabTitle + "\n"
                + "err_count2=" + errs[0] + "\n"
                + "err_empty=" + errs[1] + "\n"
                + "err_zero=" + errs[2] + "\n"
                + "err_neg=" + errs[3] + "\n"
                + "err_null_stack=" + errs[4] + "\n"
                + "err_null_itemlike=" + errs[5] + "\n"
                + "err_null_vis=" + errs[6];
    }

    public static String extraRows() {
        FeatureFlagSet withTrade = FeatureFlagSet.of(FeatureFlags.VANILLA, FeatureFlags.TRADE_REBALANCE);
        ItemStack stackA = new ItemStack(enabledA);
        ItemStack stackZero = new ItemStack(enabledA, 0);
        return "trade_count2_err=" + tradeCount2Err + "\n"
                + "empty_flags_empty_stack=" + emptyFlagsEmptyStack + "\n"
                + "stack_props="
                + stackA.isEmpty() + "|"
                + stackA.getCount() + "|"
                + stackA.is(enabledA) + "|"
                + stackA.is(enabledB) + "|"
                + stackA.toString() + "|"
                + stackZero.isEmpty() + "|"
                + stackZero.getCount() + "|"
                + stackZero.toString() + "\n"
                + "flags_props="
                + (FeatureFlags.VANILLA_SET == FeatureFlags.DEFAULT_FLAGS) + "|"
                + FeatureFlags.VANILLA_SET.equals(FeatureFlagSet.of(FeatureFlags.VANILLA)) + "|"
                + FeatureFlagSet.of().isEmpty() + "|"
                + FeatureFlagSet.of().isSubsetOf(FeatureFlags.VANILLA_SET) + "|"
                + FeatureFlags.VANILLA_SET.isSubsetOf(FeatureFlagSet.of()) + "|"
                + withTrade.contains(FeatureFlags.TRADE_REBALANCE) + "|"
                + disabledItem.isEnabled(FeatureFlags.VANILLA_SET) + "|"
                + disabledItem.isEnabled(withTrade) + "|"
                + out1EnabledMatchesVanilla + "|"
                + out1ShowOp + "\n"
                + "item_enabled_props="
                + ItemStack.EMPTY.isItemEnabled(FeatureFlagSet.of()) + "|"
                + ItemStack.EMPTY.isItemEnabled(null) + "|"
                + stackZero.isItemEnabled(FeatureFlagSet.of()) + "|"
                + new ItemStack(disabledItem, 0).isItemEnabled(FeatureFlagSet.of()) + "|"
                + stackA.isItemEnabled(FeatureFlagSet.of()) + "|"
                + stackA.isItemEnabled(FeatureFlags.VANILLA_SET) + "|"
                + new ItemStack(disabledItem).isItemEnabled(FeatureFlags.VANILLA_SET) + "|"
                + new ItemStack(disabledItem).isItemEnabled(withTrade) + "\n"
                + "npe_of_null_flags="
                + catchErr(() -> FeatureFlagSet.of(FeatureFlags.VANILLA, (FeatureFlag[]) null)) + "\n"
                + "npe_of_null_flag_empty="
                + catchErr(() -> FeatureFlagSet.of(null, new FeatureFlag[0])) + "\n"
                + "npe_of_null_flag_nonempty="
                + catchErr(() -> FeatureFlagSet.of(null, FeatureFlags.VANILLA)) + "\n"
                + "npe_of_null_elem="
                + catchErr(() -> FeatureFlagSet.of(FeatureFlags.VANILLA, (FeatureFlag) null));
    }
}
"##;

const EDGE_PROBE: &str = r#"
package demo;

import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTabOutput;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.CreativeModeTabs;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;

public final class EdgeProbe {
    static boolean alignedRan;
    static boolean hotbarRan;
    static boolean opRan;
    static boolean opShowFlag;
    static boolean objectRan;
    static boolean allObjectRan;

    static final class TaggedItem extends Item {
        final String label;
        TaggedItem(Item.Properties properties, String label) {
            super(properties);
            this.label = label;
        }
        String label() {
            return label;
        }
    }

    static final class ObjectListener implements CreativeModeTabEvents.ModifyOutput {
        final Item item;
        ObjectListener(Item item) { this.item = item; }
        @Override
        public void modifyOutput(FabricCreativeModeTabOutput output) {
            objectRan = true;
            output.accept(item);
        }
    }

    static final class AllObjectListener implements CreativeModeTabEvents.ModifyOutputAll {
        @Override
        public void modifyOutput(CreativeModeTab tab, FabricCreativeModeTabOutput output) {
            allObjectRan = true;
        }
    }

    static ResourceKey<CreativeModeTab> tabKey(String id) {
        return ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse(id));
    }

    public static void setupEdgeCases() {
        ResourceKey<Item> itemKey = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:obj_item"));
        Item objItem = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:obj_item"),
                new Item(new Item.Properties().setId(itemKey)));

        CreativeModeTab rightTab = FabricCreativeModeTab.builder()
                .title(Component.literal("Right"))
                .alignedRight()
                .build();
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB, tabKey("demo:right"), rightTab);

        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:right")).register(o -> alignedRan = true);
        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.HOTBAR).register(o -> hotbarRan = true);
        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.OP_BLOCKS).register(o -> {
            opRan = true;
            opShowFlag = o.shouldShowOpRestrictedItems();
        });
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:obj_tab")).register(new ObjectListener(objItem));
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register(new AllObjectListener());
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:throws")).register(o -> {
            throw new IllegalStateException("boom");
        });
        CreativeModeTabEvents.modifyOutputEvent(tabKey("demo:unknown_features")).register(o -> {
            // `new Item(null)` has unknown `requiredFeatures`, so `accept` must be counted.
            o.accept(new Item(null));
        });
        // GC rooting: listener 1 creates an unrooted jar TaggedItem and accepts it;
        // listener 2 allocates enough temporary objects to force a resident heap collection.
        ResourceKey<CreativeModeTab> gcKey = tabKey("demo:gc_tab");
        CreativeModeTabEvents.modifyOutputEvent(gcKey).register(o -> {
            TaggedItem ephem = new TaggedItem(new Item.Properties(), "survived-gc");
            o.accept(ephem);
        });
        CreativeModeTabEvents.modifyOutputEvent(gcKey).register(o -> {
            for (int i = 0; i < 600; i++) {
                new TaggedItem(new Item.Properties(), "garbage-" + i);
            }
        });
    }

    public static String edgeFlags() {
        return alignedRan + "|" + hotbarRan + "|" + opRan + "|" + opShowFlag + "|" + objectRan + "|" + allObjectRan;
    }

    public static String readTaggedLabel(ItemStack stack) {
        return ((TaggedItem) stack.getItem()).label();
    }
}
"#;

fn format_stacks_via_java(jar: &NativeJar, host: &mut StdHost, stacks: &[Value]) -> String {
    stacks
        .iter()
        .map(|stack| {
            let value = jar
                .invoke_method_with_args(
                    "demo/Probe",
                    "formatStack",
                    "(Lnet/minecraft/world/item/ItemStack;)Ljava/lang/String;",
                    None,
                    std::slice::from_ref(stack),
                    host,
                )
                .unwrap();
            match value {
                Value::Ref(Some(text)) => text,
                other => panic!("unexpected formatStack result: {other:?}"),
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn javac_creative_tab_modify_output_matches_java25_oracle() {
    let mut sources = vec![("demo/Probe.java", ORACLE_PROBE)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("creative-tab-modify-output-oracle", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let expected = oracle_rows();

    fixture
        .jar
        .invoke_method("demo/Probe", "setup", "()V", None, &mut host)
        .unwrap();

    // Before building any tab, every creative-tab registration MUST stay counted in `unsupported`.
    let before = host.game_listeners(&fixture.jar, true);
    assert!(before.listeners.is_empty());
    let primary_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:demo:primary";
    let unbuilt_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:demo:unbuilt";
    let all_source =
        "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.MODIFY_OUTPUT_ALL";
    assert_eq!(before.unsupported.get(primary_source), Some(&4));
    assert_eq!(before.unsupported.get(all_source), Some(&2));
    assert_eq!(before.unsupported.get(unbuilt_source), Some(&1));

    // Firing 1: build `demo:primary` with default vanilla features.
    let contents1 = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:primary")
        .unwrap();
    let display1 = format_stacks_via_java(&fixture.jar, &mut host, &contents1.display_stacks);
    let search1 = format_stacks_via_java(&fixture.jar, &mut host, &contents1.search_tab_stacks);
    assert_eq!(display1, expected["firing1_display"]);
    assert_eq!(search1, expected["firing1_search"]);

    // Verify Rust inspection helpers agree with Java stack values.
    let display_ids: Vec<_> = contents1
        .display_stacks
        .iter()
        .map(|s| {
            (
                host.item_stack_item_id(s).unwrap(),
                host.item_stack_count(s).unwrap(),
                host.item_stack_is_empty(s).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        display_ids,
        vec![
            ("demo:enabled_b".to_owned(), 1, false),
            ("demo:enabled_a".to_owned(), 1, false),
            ("minecraft:stone".to_owned(), 1, false),
            ("demo:enabled_c".to_owned(), 1, false),
            ("demo:altar".to_owned(), 1, false),
            ("demo:enabled_c".to_owned(), 1, false),
        ]
    );

    let summary = fixture.text_on(&mut host, "firing1Summary");
    for line in summary.lines() {
        let (key, actual) = line.split_once('=').unwrap();
        assert_eq!(actual, expected[key], "mismatch for {key}");
    }

    // After firing 1: initial primary + MODIFY_OUTPUT_ALL cleared; dynamic listener registered
    // during firing 1 is still unfired (count 1); unbuilt tab still counted (count 1).
    let after1 = host.game_listeners(&fixture.jar, true);
    assert_eq!(after1.unsupported.get(primary_source), Some(&1));
    assert_eq!(after1.unsupported.get(all_source), None);
    assert_eq!(after1.unsupported.get(unbuilt_source), Some(&1));

    // Firing 2: dynamic listener now runs in phase order.
    fixture
        .jar
        .invoke_method("demo/Probe", "clearOrder", "()V", None, &mut host)
        .unwrap();
    let _contents2 = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:primary")
        .unwrap();
    assert_eq!(
        fixture.text_on(&mut host, "orderJoined"),
        expected["firing2_order"]
    );
    let after2 = host.game_listeners(&fixture.jar, true);
    assert_eq!(after2.unsupported.get(primary_source), None);
    assert_eq!(after2.unsupported.get(unbuilt_source), Some(&1));

    // Firing 3: TRADE_REBALANCE enabled accepts `disabledItem` and throws exact count=2 error.
    let trade_contents = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:trade_tab",
            CreativeTabBuildContext {
                enabled_features: (1 << 0) | (1 << 1),
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    // Note: MODIFY_OUTPUT_ALL also appended `enabledC`; check the first stack is `disabledItem`.
    let trade_first =
        format_stacks_via_java(&fixture.jar, &mut host, &trade_contents.display_stacks[..1]);
    assert_eq!(trade_first, expected["trade_display"]);

    // Firing 4: empty FeatureFlagSet skips `ItemStack.EMPTY` (Items.AIR requires VANILLA).
    let empty_flags_contents = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:empty_flags_tab",
            CreativeTabBuildContext {
                enabled_features: 0,
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    assert!(empty_flags_contents.display_stacks.is_empty());

    let extra = fixture.text_on(&mut host, "extraRows");
    for line in extra.lines() {
        let (key, actual) = line.split_once('=').unwrap();
        assert_eq!(actual, expected[key], "mismatch for {key}");
    }

    // Unbuilt tab is STILL counted in `unsupported`.
    let final_listeners = host.game_listeners(&fixture.jar, true);
    assert_eq!(final_listeners.unsupported.get(unbuilt_source), Some(&1));
    assert_eq!(
        host.unknown_host_calls(),
        Vec::<(String, usize)>::new(),
        "oracle fixture must run with zero counted unknown host calls"
    );
}

#[test]
fn alignment_op_blocks_object_listeners_exceptions_gc_roots_and_unknown_features() {
    let mut sources = vec![("demo/EdgeProbe.java", EDGE_PROBE)];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("creative-tab-modify-output-edges", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());

    fixture
        .jar
        .invoke_method("demo/EdgeProbe", "setupEdgeCases", "()V", None, &mut host)
        .unwrap();

    // Aligned-right custom tab and vanilla `minecraft:hotbar` do not fire listeners.
    assert!(
        fixture
            .jar
            .creative_tab_contents(&mut host, "demo:right")
            .unwrap()
            .display_stacks
            .is_empty()
    );
    assert!(
        fixture
            .jar
            .creative_tab_contents(&mut host, "minecraft:hotbar")
            .unwrap()
            .display_stacks
            .is_empty()
    );
    // `minecraft:op_blocks` does not fire without permissions.
    assert!(
        fixture
            .jar
            .creative_tab_contents(&mut host, "minecraft:op_blocks")
            .unwrap()
            .display_stacks
            .is_empty()
    );
    let edge_flags = |host: &mut StdHost| -> String {
        match fixture
            .jar
            .invoke_method(
                "demo/EdgeProbe",
                "edgeFlags",
                "()Ljava/lang/String;",
                None,
                host,
            )
            .unwrap()
        {
            Value::Ref(Some(text)) => text,
            other => panic!("unexpected edgeFlags result: {other:?}"),
        }
    };
    assert_eq!(edge_flags(&mut host), "false|false|false|false|false|false");

    // `minecraft:op_blocks` fires when `has_permissions: true`.
    let _ = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "minecraft:op_blocks",
            CreativeTabBuildContext {
                has_permissions: true,
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    // Object listener on `demo:obj_tab` fires too.
    let obj_contents = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:obj_tab")
        .unwrap();
    assert_eq!(obj_contents.display_stacks.len(), 1);
    assert_eq!(
        host.item_stack_item_id(&obj_contents.display_stacks[0])
            .as_deref(),
        Some("demo:obj_item")
    );
    assert_eq!(edge_flags(&mut host), "false|false|true|true|true|true");

    // Right-aligned unfired tabs remain counted in `game_listeners().unsupported`.
    let listeners = host.game_listeners(&fixture.jar, true);
    assert_eq!(
        listeners
            .unsupported
            .get("net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:demo:right"),
        Some(&1)
    );
    assert_eq!(
        listeners
            .unsupported
            .get("net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:hotbar"),
        Some(&1)
    );

    // Exceptions propagate and clean up the active collector.
    let error = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:throws")
        .unwrap_err();
    assert!(format!("{error:?}").contains("boom"), "{error:?}");

    // GC roots: unrooted jar Item accepted in listener 1 survives heap collection triggered by
    // listener 2 on the shared collector.
    let gc_contents = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:gc_tab")
        .unwrap();
    assert_eq!(gc_contents.display_stacks.len(), 1);
    let survived = fixture
        .jar
        .invoke_method_with_args(
            "demo/EdgeProbe",
            "readTaggedLabel",
            "(Lnet/minecraft/world/item/ItemStack;)Ljava/lang/String;",
            None,
            std::slice::from_ref(&gc_contents.display_stacks[0]),
            &mut host,
        )
        .unwrap();
    assert_eq!(survived, Value::Ref(Some("survived-gc".into())));

    // Unknown `requiredFeatures` (`new Item(null)`) MUST be counted, never guessed.
    let before_unknown = host.unknown_host_call_count();
    let unknown_contents = fixture
        .jar
        .creative_tab_contents(&mut host, "demo:unknown_features")
        .unwrap();
    assert!(unknown_contents.display_stacks.is_empty());
    assert!(
        host.unknown_host_call_count() > before_unknown,
        "accepting an item with unknown requiredFeatures must be counted"
    );
}

#[test]
fn real_runes_entrypoint_adds_items_to_both_tabs() {
    let Some(jar_path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes test skipped");
        return;
    };
    // Run on a 16 MiB thread so unoptimised debug builds do not hit `frame_bound`'s
    // 2 MiB test-thread stack floor when Runes initializes its nested `bundle-api` jar.
    std::thread::Builder::new()
        .name("real-runes-creative-tab".into())
        .stack_size(16 << 20)
        .spawn(move || run_real_runes_test(Path::new(&jar_path)))
        .unwrap()
        .join()
        .unwrap();
}

fn run_real_runes_test(jar_path: &Path) {
    let jar = NativeJar::from_path(jar_path).unwrap();
    let mut host = jar.seed_host(StdHost::new());

    // Run every real `main` entrypoint declared in `fabric.mod.json` (registration included).
    let entrypoints = jar.discover_entrypoints("main");
    assert!(
        !entrypoints.is_empty(),
        "expected at least one main entrypoint in Runes jar"
    );
    for entry in &entrypoints {
        jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
            .unwrap_or_else(|e| panic!("entrypoint {} failed: {e:?}", entry.class));
    }

    let functional_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:functional_blocks";
    let combat_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:combat";
    let before_build = host.game_listeners(&jar, true);
    assert_eq!(
        before_build.unsupported.get(functional_source),
        Some(&1),
        "functional_blocks listener must stay counted before tab build: {:?}",
        before_build.unsupported
    );
    assert_eq!(
        before_build.unsupported.get(combat_source),
        Some(&1),
        "combat listener must stay counted before tab build: {:?}",
        before_build.unsupported
    );

    let before_tabs = host.unknown_host_call_count();
    let functional = jar
        .creative_tab_contents(&mut host, "minecraft:functional_blocks")
        .unwrap();
    let combat = jar
        .creative_tab_contents(&mut host, "minecraft:combat")
        .unwrap();
    let after_tabs = host.unknown_host_call_count();

    let inspect = |stacks: &[Value]| -> Vec<(String, i32, bool)> {
        stacks
            .iter()
            .map(|stack| {
                (
                    host.item_stack_item_id(stack)
                        .unwrap_or_else(|| panic!("missing item id for {stack:?}")),
                    host.item_stack_count(stack)
                        .unwrap_or_else(|| panic!("missing count for {stack:?}")),
                    host.item_stack_is_empty(stack)
                        .unwrap_or_else(|| panic!("missing isEmpty for {stack:?}")),
                )
            })
            .collect()
    };

    // Runes bundles `META-INF/jars/bundle-api-5.0.0+26.3-fabric.jar`, so `BundleApiCompat`
    // registers and appends the three rune pouches after the six rune stones.
    let expected_functional = vec![("runes:crafting_altar".to_owned(), 1, false)];
    let expected_combat = vec![
        ("runes:arcane_stone".to_owned(), 1, false),
        ("runes:fire_stone".to_owned(), 1, false),
        ("runes:frost_stone".to_owned(), 1, false),
        ("runes:healing_stone".to_owned(), 1, false),
        ("runes:lightning_stone".to_owned(), 1, false),
        ("runes:soul_stone".to_owned(), 1, false),
        ("runes:small_rune_pouch".to_owned(), 1, false),
        ("runes:medium_rune_pouch".to_owned(), 1, false),
        ("runes:large_rune_pouch".to_owned(), 1, false),
    ];

    assert_eq!(inspect(&functional.display_stacks), expected_functional);
    assert_eq!(inspect(&functional.search_tab_stacks), expected_functional);
    assert_eq!(inspect(&combat.display_stacks), expected_combat);
    assert_eq!(inspect(&combat.search_tab_stacks), expected_combat);
    assert_eq!(
        host.registrations()
            .iter()
            .filter(|r| r.registry == "minecraft:item")
            .count(),
        10
    );
    assert_eq!(
        after_tabs - before_tabs,
        0,
        "creative_tab_contents for both Runes tabs must have zero counted calls; unknown calls: {:?}",
        host.unknown_host_calls()
    );

    // After building both tabs, both registrations must be cleared from `unsupported`.
    let after_build = host.game_listeners(&jar, true);
    assert_eq!(after_build.unsupported.get(functional_source), None);
    assert_eq!(after_build.unsupported.get(combat_source), None);
}

const PLACEMENT_PROBE: &str = r##"
package demo;

import java.util.List;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.core.registries.Registries;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.ItemLike;

public final class PlacementProbe {
    public static String errLog = "";

    private static String catchMsg(Runnable r) {
        try {
            r.run();
            return "NONE";
        } catch (Throwable t) {
            return t.getClass().getSimpleName() + ":" + t.getMessage();
        }
    }

    public static void setup() {
        Item a = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:a"), new Item(new Item.Properties()));
        Item b = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:b"), new Item(new Item.Properties()));
        Item c = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:c"), new Item(new Item.Properties()));
        Item d = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:d"), new Item(new Item.Properties()));
        Item e = Registry.register(BuiltInRegistries.ITEM, Identifier.parse("demo:e"), new Item(new Item.Properties()));
        Item disabled = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.parse("demo:disabled"),
            new Item(new Item.Properties().requiredFeatures(FeatureFlags.TRADE_REBALANCE))
        );

        ResourceKey<CreativeModeTab> mainTab = ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse("demo:placement"));
        CreativeModeTabEvents.modifyOutputEvent(mainTab).register(entries -> {
            entries.insertBefore(Items.DIRT, List.of(new ItemStack(a)), CreativeModeTab.TabVisibility.PARENT_TAB_ONLY);
            entries.insertAfter(Items.STONE, List.of(new ItemStack(b)), CreativeModeTab.TabVisibility.SEARCH_TAB_ONLY);
            entries.insertAfter(a, c, disabled);
            entries.insertBefore(Items.GOLD_INGOT, d);
            entries.prepend(e, CreativeModeTab.TabVisibility.PARENT_AND_SEARCH_TABS);
        });

        ResourceKey<CreativeModeTab> dupTab = ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse("demo:dup"));
        CreativeModeTabEvents.modifyOutputEvent(dupTab).register(entries -> {
            entries.insertBefore(Items.STONE, a);
            entries.insertAfter(Items.STONE, b);
        });

        ResourceKey<CreativeModeTab> errTab = ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse("demo:errors"));
        CreativeModeTabEvents.modifyOutputEvent(errTab).register(entries -> {
            StringBuilder sb = new StringBuilder();
            sb.append("null_vis_disabled=").append(catchMsg(() ->
                entries.insertBefore(Items.STONE, List.of(new ItemStack(disabled)), null)
            )).append('\n');
            sb.append("null_anchor_disabled=").append(catchMsg(() ->
                entries.insertBefore((ItemLike) null, List.of(new ItemStack(disabled)))
            )).append('\n');
            sb.append("null_item_anchor=").append(catchMsg(() ->
                entries.insertBefore((ItemLike) null, List.of(new ItemStack(a)))
            )).append('\n');
            sb.append("null_stack_anchor=").append(catchMsg(() ->
                entries.insertBefore((ItemStack) null, List.of(new ItemStack(a)))
            )).append('\n');
            sb.append("null_collection=").append(catchMsg(() ->
                entries.insertBefore(Items.STONE, (List<ItemStack>) null)
            )).append('\n');
            sb.append("null_vis=").append(catchMsg(() ->
                entries.insertBefore(Items.STONE, List.of(new ItemStack(a)), null)
            )).append('\n');
            sb.append("empty_stack=").append(catchMsg(() ->
                entries.insertBefore(Items.STONE, ItemStack.EMPTY)
            )).append('\n');
            sb.append("count2=").append(catchMsg(() ->
                entries.insertBefore(Items.STONE, new ItemStack(a, 2))
            ));
            errLog = sb.toString();
        });

        ResourceKey<CreativeModeTab> stackAnchorTab = ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse("demo:stack_anchor"));
        CreativeModeTabEvents.modifyOutputEvent(stackAnchorTab).register(entries -> {
            entries.insertAfter(new ItemStack(Items.STONE), new ItemStack(a));
        });
    }

    public static String errors() {
        return errLog;
    }
}
"##;

#[test]
fn anchor_placement_and_prepend_match_java25_fabric_api_oracle() {
    let mut sources = vec![
        ("demo/PlacementProbe.java", PLACEMENT_PROBE),
        (
            "net/minecraft/world/item/Items.java",
            "package net.minecraft.world.item; public final class Items { public static final Item STONE = null; public static final Item DIRT = null; public static final Item GOLD_INGOT = null; }",
        ),
    ];
    sources.extend_from_slice(STUBS);
    let fixture = javac_fixture::Fixture::build("creative-tab-placement-oracle", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());

    fixture
        .jar
        .invoke_method("demo/PlacementProbe", "setup", "()V", None, &mut host)
        .unwrap();

    let ids = |host: &StdHost, stacks: &[Value]| -> Vec<String> {
        stacks
            .iter()
            .map(|s| host.item_stack_item_id(s).expect("stack item id"))
            .collect()
    };

    // Case 1: Initial `[stone, dirt]` — exact Java 25 + Fabric API 5.0.21 oracle output.
    let s_stone = host.make_item_stack_for_id("minecraft:stone", 1, false);
    let s_dirt = host.make_item_stack_for_id("minecraft:dirt", 1, false);
    let placed = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:placement",
            CreativeTabBuildContext {
                initial_display_stacks: vec![s_stone.clone(), s_dirt.clone()],
                initial_search_tab_stacks: vec![s_stone.clone(), s_dirt.clone()],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    assert_eq!(
        ids(&host, &placed.display_stacks),
        vec![
            "demo:e",
            "minecraft:stone",
            "demo:a",
            "demo:c",
            "minecraft:dirt",
            "demo:d",
        ]
    );
    assert_eq!(
        ids(&host, &placed.search_tab_stacks),
        vec![
            "demo:e",
            "minecraft:stone",
            "demo:b",
            "minecraft:dirt",
            "demo:c",
            "demo:d",
        ]
    );

    // Case 2: Duplicate anchor `[stone, dirt, stone]` — `insertBefore` uses FIRST match,
    // `insertAfter` uses LAST match.
    let s_stone2 = host.make_item_stack_for_id("minecraft:stone", 1, false);
    let dup = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:dup",
            CreativeTabBuildContext {
                initial_display_stacks: vec![s_stone.clone(), s_dirt.clone(), s_stone2.clone()],
                initial_search_tab_stacks: vec![s_stone.clone(), s_dirt.clone(), s_stone2],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    assert_eq!(
        ids(&host, &dup.display_stacks),
        vec![
            "demo:a",
            "minecraft:stone",
            "minecraft:dirt",
            "minecraft:stone",
            "demo:b",
        ]
    );

    // Case 3: Exact Java 25 / Fabric API 5.0.21 exception messages and evaluation order.
    let _ = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:errors",
            CreativeTabBuildContext {
                initial_display_stacks: vec![s_stone.clone(), s_dirt.clone()],
                initial_search_tab_stacks: vec![s_stone.clone(), s_dirt],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    let err_text = match fixture
        .jar
        .invoke_method(
            "demo/PlacementProbe",
            "errors",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap()
    {
        Value::Ref(Some(text)) => text,
        other => panic!("unexpected errors result: {other:?}"),
    };
    let rows: BTreeMap<&str, &str> = err_text
        .lines()
        .map(|l| l.split_once('=').unwrap())
        .collect();
    assert_eq!(rows["null_vis_disabled"], "NONE");
    assert_eq!(rows["null_anchor_disabled"], "NONE");
    assert_eq!(
        rows["null_item_anchor"],
        "NullPointerException:Cannot invoke \"net.minecraft.world.level.ItemLike.asItem()\" because \"anchor\" is null"
    );
    assert_eq!(
        rows["null_stack_anchor"],
        "NullPointerException:Cannot invoke \"net.minecraft.world.item.ItemStack.is(Object)\" because \"a\" is null"
    );
    assert_eq!(
        rows["null_collection"],
        "NullPointerException:Cannot invoke \"java.util.Collection.stream()\" because \"newStacks\" is null"
    );
    assert_eq!(
        rows["null_vis"],
        "NullPointerException:Cannot invoke \"net.minecraft.world.item.CreativeModeTab$TabVisibility.ordinal()\" because \"visibility\" is null"
    );
    assert_eq!(
        rows["empty_stack"],
        "IllegalArgumentException:Cannot add empty stack"
    );
    assert_eq!(
        rows["count2"],
        "IllegalArgumentException:Stack size must be exactly 1 for stack: 2 demo:a"
    );
    assert_eq!(
        host.unknown_host_calls(),
        Vec::<(String, usize)>::new(),
        "placement oracle must have zero unknown host calls"
    );

    // Case 4: `ItemStack` anchor against a seeded stack with opaque custom components MUST be
    // counted as unknown rather than falsely equating components.
    let opaque_stone = host.make_item_stack_for_id("minecraft:stone", 1, true);
    let before_unknown = host.unknown_host_call_count();
    let _ = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:stack_anchor",
            CreativeTabBuildContext {
                initial_display_stacks: vec![opaque_stone.clone()],
                initial_search_tab_stacks: vec![opaque_stone],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    assert!(
        host.unknown_host_call_count() > before_unknown,
        "ItemStack anchor matching an opaque-component stack must be counted"
    );
}

const COMPONENT_ANCHOR_PROBE: &str = r##"
package demo;

import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.util.Unit;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.component.CustomData;

public final class ComponentAnchorProbe {
    public static void setup() {
        Item marker = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.parse("demo:marker"),
            new Item(new Item.Properties())
        );

        ResourceKey<CreativeModeTab> tab = ResourceKey.create(
            Registries.CREATIVE_MODE_TAB,
            Identifier.parse("demo:comp_anchor")
        );
        CreativeModeTabEvents.modifyOutputEvent(tab).register(entries -> {
            ItemStack damagedAnchor = new ItemStack(Items.DIAMOND_SWORD);
            damagedAnchor.set(DataComponents.DAMAGE, 12);
            CompoundTag tag = new CompoundTag();
            tag.putInt("tier", 3);
            CustomData.set(DataComponents.CUSTOM_DATA, damagedAnchor, tag);
            damagedAnchor.set(DataComponents.UNBREAKABLE, Unit.INSTANCE);
            entries.insertAfter(damagedAnchor, new ItemStack(marker));
        });
    }
}
"##;

#[test]
fn component_stack_anchor_distinguishes_modelled_components_with_zero_unknown_calls() {
    use horizon_native_java::{Nbt, NbtCompound, RuntimeComponentValue};

    let mut sources = vec![
        ("demo/ComponentAnchorProbe.java", COMPONENT_ANCHOR_PROBE),
        (
            "net/minecraft/world/item/Items.java",
            "package net.minecraft.world.item; public final class Items { public static final Item DIAMOND_SWORD = null; }",
        ),
        (
            "net/minecraft/core/component/DataComponentType.java",
            "package net.minecraft.core.component; public interface DataComponentType<T> {}",
        ),
        (
            "net/minecraft/core/component/DataComponents.java",
            r#"package net.minecraft.core.component;
import net.minecraft.util.Unit;
import net.minecraft.world.item.component.CustomData;
public final class DataComponents {
    public static final DataComponentType<Integer> DAMAGE = null;
    public static final DataComponentType<CustomData> CUSTOM_DATA = null;
    public static final DataComponentType<Unit> UNBREAKABLE = null;
}"#,
        ),
        (
            "net/minecraft/util/Unit.java",
            "package net.minecraft.util; public enum Unit { INSTANCE }",
        ),
        (
            "net/minecraft/nbt/CompoundTag.java",
            "package net.minecraft.nbt; public class CompoundTag { public CompoundTag() {} public void putInt(String k, int v) {} }",
        ),
        (
            "net/minecraft/world/item/component/CustomData.java",
            "package net.minecraft.world.item.component; import net.minecraft.core.component.DataComponentType; import net.minecraft.nbt.CompoundTag; import net.minecraft.world.item.ItemStack; public final class CustomData { public static void set(DataComponentType<CustomData> type, ItemStack stack, CompoundTag tag) {} }",
        ),
        (
            "net/minecraft/world/item/ItemStack.java",
            r#"package net.minecraft.world.item;
import net.minecraft.core.component.DataComponentType;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.level.ItemLike;
public final class ItemStack {
    public static final ItemStack EMPTY = null;
    public ItemStack(ItemLike item) {}
    public ItemStack(ItemLike item, int count) {}
    public <T> T set(DataComponentType<? super T> type, T value) { throw new AssertionError(); }
    public boolean isEmpty() { throw new AssertionError(); }
    public int getCount() { throw new AssertionError(); }
    public Item getItem() { throw new AssertionError(); }
    public boolean is(Item item) { throw new AssertionError(); }
    public boolean isItemEnabled(FeatureFlagSet enabledFeatures) { throw new AssertionError(); }
}"#,
        ),
    ];
    for &(path, src) in STUBS {
        if path != "net/minecraft/world/item/ItemStack.java" {
            sources.push((path, src));
        }
    }
    let fixture = javac_fixture::Fixture::build("creative-tab-component-anchor", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture
        .jar
        .invoke_method("demo/ComponentAnchorProbe", "setup", "()V", None, &mut host)
        .unwrap();

    let plain_sword = host.make_item_stack_for_id("minecraft:diamond_sword", 1, false);
    let mut nbt = NbtCompound::new();
    nbt.insert("tier".into(), Nbt::Int(3));
    let matching_sword = host.make_host_item_stack_with_components(
        None,
        "minecraft:diamond_sword",
        1,
        vec![
            (
                "minecraft:damage".into(),
                Some(RuntimeComponentValue::Int(12)),
            ),
            (
                "minecraft:custom_data".into(),
                Some(RuntimeComponentValue::CustomData(nbt)),
            ),
            (
                "minecraft:unbreakable".into(),
                Some(RuntimeComponentValue::Unit),
            ),
        ],
        false,
        Vec::new(),
    );
    let different_sword = host.make_host_item_stack_with_components(
        None,
        "minecraft:diamond_sword",
        1,
        vec![(
            "minecraft:damage".into(),
            Some(RuntimeComponentValue::Int(5)),
        )],
        false,
        Vec::new(),
    );

    let out = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:comp_anchor",
            CreativeTabBuildContext {
                initial_display_stacks: vec![
                    plain_sword.clone(),
                    matching_sword.clone(),
                    different_sword.clone(),
                ],
                initial_search_tab_stacks: vec![plain_sword, matching_sword, different_sword],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();

    let ids: Vec<String> = out
        .display_stacks
        .iter()
        .map(|s| host.item_stack_item_id(s).unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "minecraft:diamond_sword",
            "minecraft:diamond_sword",
            "demo:marker",
            "minecraft:diamond_sword",
        ]
    );
    assert_eq!(
        host.unknown_host_calls(),
        Vec::<(String, usize)>::new(),
        "modelled component ItemStack anchor must have zero unknown host calls"
    );
}

const TOOLTIP_DISPLAY_SET_PROBE: &str = r##"
package demo;

import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.component.TooltipDisplay;

public final class TooltipDisplaySetProbe {
    public static String checkEquality() {
        TooltipDisplay a = TooltipDisplay.DEFAULT
            .withHidden(DataComponents.LORE, true)
            .withHidden(DataComponents.DAMAGE, true);
        TooltipDisplay b = TooltipDisplay.DEFAULT
            .withHidden(DataComponents.DAMAGE, true)
            .withHidden(DataComponents.LORE, true);
        ItemStack stackA = new ItemStack(Items.DIAMOND_SWORD);
        stackA.set(DataComponents.TOOLTIP_DISPLAY, a);
        ItemStack stackB = new ItemStack(Items.DIAMOND_SWORD);
        stackB.set(DataComponents.TOOLTIP_DISPLAY, b);
        ItemStack stackPlain = new ItemStack(Items.DIAMOND_SWORD);
        return a.equals(b) + "|"
            + a.equals(TooltipDisplay.DEFAULT) + "|"
            + ItemStack.isSameItemSameComponents(stackA, stackB) + "|"
            + ItemStack.isSameItemSameComponents(stackA, stackPlain);
    }

    public static void setup() {
        Item marker = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.parse("demo:td_marker"),
            new Item(new Item.Properties())
        );
        ResourceKey<CreativeModeTab> tab = ResourceKey.create(
            Registries.CREATIVE_MODE_TAB,
            Identifier.parse("demo:td_set_anchor")
        );
        CreativeModeTabEvents.modifyOutputEvent(tab).register(entries -> {
            TooltipDisplay b = TooltipDisplay.DEFAULT
                .withHidden(DataComponents.DAMAGE, true)
                .withHidden(DataComponents.LORE, true);
            ItemStack anchorB = new ItemStack(Items.DIAMOND_SWORD);
            anchorB.set(DataComponents.TOOLTIP_DISPLAY, b);
            entries.insertAfter(anchorB, new ItemStack(marker));
        });
    }

    public static int probeHashCode() {
        return TooltipDisplay.DEFAULT.withHidden(DataComponents.LORE, true).hashCode();
    }
}
"##;

#[test]
fn tooltip_display_hidden_components_use_set_equality_and_keep_first_occurrence_order() {
    use horizon_native_java::RuntimeComponentValue;

    let mut sources = vec![
        (
            "demo/TooltipDisplaySetProbe.java",
            TOOLTIP_DISPLAY_SET_PROBE,
        ),
        (
            "net/minecraft/world/item/Items.java",
            "package net.minecraft.world.item; public final class Items { public static final Item DIAMOND_SWORD = null; }",
        ),
        (
            "net/minecraft/core/component/DataComponentType.java",
            "package net.minecraft.core.component; public interface DataComponentType<T> {}",
        ),
        (
            "net/minecraft/core/component/DataComponents.java",
            r#"package net.minecraft.core.component;
import net.minecraft.world.item.component.TooltipDisplay;
public final class DataComponents {
    public static final DataComponentType<Integer> DAMAGE = null;
    public static final DataComponentType<Object> LORE = null;
    public static final DataComponentType<TooltipDisplay> TOOLTIP_DISPLAY = null;
}"#,
        ),
        (
            "net/minecraft/world/item/component/TooltipDisplay.java",
            r#"package net.minecraft.world.item.component;
import net.minecraft.core.component.DataComponentType;
public final class TooltipDisplay {
    public static final TooltipDisplay DEFAULT = null;
    public boolean hideTooltip() { throw new AssertionError(); }
    public boolean shows(DataComponentType<?> type) { throw new AssertionError(); }
    public TooltipDisplay withHidden(DataComponentType<?> type, boolean hidden) { throw new AssertionError(); }
}"#,
        ),
        (
            "net/minecraft/world/item/ItemStack.java",
            r#"package net.minecraft.world.item;
import net.minecraft.core.component.DataComponentType;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.level.ItemLike;
public final class ItemStack {
    public static final ItemStack EMPTY = null;
    public ItemStack(ItemLike item) {}
    public ItemStack(ItemLike item, int count) {}
    public static boolean isSameItemSameComponents(ItemStack a, ItemStack b) { throw new AssertionError(); }
    public <T> T set(DataComponentType<? super T> type, T value) { throw new AssertionError(); }
    public boolean isEmpty() { throw new AssertionError(); }
    public int getCount() { throw new AssertionError(); }
    public Item getItem() { throw new AssertionError(); }
    public boolean is(Item item) { throw new AssertionError(); }
    public boolean isItemEnabled(FeatureFlagSet enabledFeatures) { throw new AssertionError(); }
}"#,
        ),
    ];
    for &(path, src) in STUBS {
        if path != "net/minecraft/world/item/ItemStack.java" {
            sources.push((path, src));
        }
    }
    let fixture = javac_fixture::Fixture::build("creative-tab-tooltip-display-set", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());

    let eq_summary = match fixture
        .jar
        .invoke_method(
            "demo/TooltipDisplaySetProbe",
            "checkEquality",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap()
    {
        Value::Ref(Some(text)) => text,
        other => panic!("unexpected checkEquality result: {other:?}"),
    };
    assert_eq!(eq_summary, "true|false|true|false");

    fixture
        .jar
        .invoke_method(
            "demo/TooltipDisplaySetProbe",
            "setup",
            "()V",
            None,
            &mut host,
        )
        .unwrap();

    // Seed with duplicate hidden_components: alloc_tooltip_display must dedupe preserving
    // first-occurrence order (`["minecraft:lore", "minecraft:damage"]`), and anchor `[damage, lore]`
    // must match it with zero unknown calls.
    let seeded_dup = host.make_host_item_stack_with_components(
        None,
        "minecraft:diamond_sword",
        1,
        vec![(
            "minecraft:tooltip_display".into(),
            Some(RuntimeComponentValue::TooltipDisplay {
                hide_tooltip: false,
                hidden_components: vec![
                    "minecraft:lore".into(),
                    "minecraft:damage".into(),
                    "minecraft:lore".into(),
                ],
                hidden_known: true,
            }),
        )],
        false,
        Vec::new(),
    );
    let extracted = host.item_stack_components(&seeded_dup).unwrap();
    assert_eq!(
        extracted.patch,
        vec![(
            "minecraft:tooltip_display".into(),
            Some(RuntimeComponentValue::TooltipDisplay {
                hide_tooltip: false,
                hidden_components: vec!["minecraft:lore".into(), "minecraft:damage".into()],
                hidden_known: true,
            }),
        )]
    );

    let plain_sword = host.make_item_stack_for_id("minecraft:diamond_sword", 1, false);
    let hidden_sword = host.make_host_item_stack_with_components(
        None,
        "minecraft:diamond_sword",
        1,
        vec![(
            "minecraft:tooltip_display".into(),
            Some(RuntimeComponentValue::TooltipDisplay {
                hide_tooltip: true,
                hidden_components: vec!["minecraft:lore".into(), "minecraft:damage".into()],
                hidden_known: true,
            }),
        )],
        false,
        Vec::new(),
    );
    // `hide_tooltip` mismatch must NOT compare equal (`stack_same_with_hide=false` on JDK 25).
    assert_eq!(
        host.stacks_same_item_same_components(&seeded_dup, &hidden_sword)
            .unwrap(),
        Some(false)
    );

    let out = fixture
        .jar
        .creative_tab_contents_with(
            &mut host,
            "demo:td_set_anchor",
            CreativeTabBuildContext {
                initial_display_stacks: vec![
                    plain_sword.clone(),
                    seeded_dup.clone(),
                    hidden_sword.clone(),
                ],
                initial_search_tab_stacks: vec![plain_sword, seeded_dup, hidden_sword],
                ..CreativeTabBuildContext::default()
            },
        )
        .unwrap();
    let ids: Vec<String> = out
        .display_stacks
        .iter()
        .map(|s| host.item_stack_item_id(s).unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "minecraft:diamond_sword",
            "minecraft:diamond_sword",
            "demo:td_marker",
            "minecraft:diamond_sword",
        ]
    );
    assert_eq!(
        host.unknown_host_calls(),
        Vec::<(String, usize)>::new(),
        "TooltipDisplay set equality and anchor matching must have zero unknown calls"
    );

    // `TooltipDisplay.hashCode()` depends on JVM identity hash codes of `DataComponentType`
    // singletons and must remain counted rather than fabricating an order-dependent value.
    let _ = fixture
        .jar
        .invoke_method(
            "demo/TooltipDisplaySetProbe",
            "probeHashCode",
            "()I",
            None,
            &mut host,
        )
        .unwrap();
    assert!(
        !host.unknown_host_calls().is_empty(),
        "TooltipDisplay.hashCode must stay counted"
    );
}

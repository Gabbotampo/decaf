import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.StringJoiner;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTabOutput;
import net.fabricmc.fabric.api.event.Event;
import net.minecraft.SharedConstants;
import net.minecraft.core.Holder;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponentMap;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ItemLike;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;

/**
 * Real Java 25 oracle against fabric-creative-tab-api-v1 + fabric-api-base + server-26.3.jar.
 * Produces tests/fixtures/creative_tab_modify_output/java25.txt verbatim.
 */
public final class CaptureCreativeTabOutput {
    private static void unfreeze(Registry<?> reg, boolean createIntrusiveMap) throws Exception {
        Field frozen = MappedRegistry.class.getDeclaredField("frozen");
        frozen.setAccessible(true);
        frozen.setBoolean(reg, false);
        if (createIntrusiveMap) {
            Field intrusive = MappedRegistry.class.getDeclaredField("unregisteredIntrusiveHolders");
            intrusive.setAccessible(true);
            if (intrusive.get(reg) == null) {
                intrusive.set(reg, new IdentityHashMap<>());
            }
        }
    }

    private static void bindComponents(Holder.Reference<?> holder) throws Exception {
        Method bind = Holder.Reference.class.getDeclaredMethod("bindComponents", DataComponentMap.class);
        bind.setAccessible(true);
        bind.invoke(holder, DataComponentMap.EMPTY);
    }

    private static Item registerItem(String id, Item item) throws Exception {
        Registry.register(BuiltInRegistries.ITEM, Identifier.parse(id), item);
        bindComponents(item.builtInRegistryHolder());
        return item;
    }

    private static Block registerBlock(String id, Block block) throws Exception {
        Registry.register(BuiltInRegistries.BLOCK, Identifier.parse(id), block);
        return block;
    }

    private static FabricCreativeModeTabOutput newOutput(
            FeatureFlagSet enabledFeatures, boolean hasPermissions) throws Exception {
        Class<?> paramsCls = Class.forName("net.minecraft.world.item.CreativeModeTab$ItemDisplayParameters");
        Constructor<?> paramsCtor = paramsCls.getDeclaredConstructor(
                FeatureFlagSet.class, boolean.class, net.minecraft.core.HolderLookup.Provider.class);
        paramsCtor.setAccessible(true);
        Object params = paramsCtor.newInstance(enabledFeatures, hasPermissions, null);

        Constructor<FabricCreativeModeTabOutput> outCtor = FabricCreativeModeTabOutput.class
                .getDeclaredConstructor(paramsCls, List.class, List.class);
        outCtor.setAccessible(true);
        return outCtor.newInstance(params, new ArrayList<>(), new ArrayList<>());
    }

    private static String catchErr(Runnable r) {
        try {
            r.run();
            return "NONE";
        } catch (Throwable t) {
            return t.getClass().getName() + ": " + t.getMessage();
        }
    }

    private static String catchClass(Runnable r) {
        try {
            r.run();
            return "NONE";
        } catch (Throwable t) {
            return t.getClass().getName();
        }
    }

    private static String catchReflect(ThrowingRunnable r) {
        try {
            r.run();
            return "NONE";
        } catch (InvocationTargetException e) {
            Throwable cause = e.getCause();
            return cause.getClass().getName();
        } catch (Throwable t) {
            return t.getClass().getName();
        }
    }

    @FunctionalInterface
    interface ThrowingRunnable {
        void run() throws Exception;
    }

    private static String formatStacks(List<ItemStack> stacks) {
        StringJoiner j = new StringJoiner(",");
        for (ItemStack s : stacks) {
            j.add(s.toString() + "#" + s.getCount() + "#" + s.isEmpty() + "#" + s.getItem().toString());
        }
        return j.toString();
    }

    private static ResourceKey<CreativeModeTab> tabKey(String id) {
        return ResourceKey.create(Registries.CREATIVE_MODE_TAB, Identifier.parse(id));
    }

    public static void main(String[] args) throws Exception {
        java.io.PrintStream rawOut = System.out;
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        unfreeze(BuiltInRegistries.BLOCK, true);
        unfreeze(BuiltInRegistries.ITEM, true);
        unfreeze(BuiltInRegistries.CREATIVE_MODE_TAB, false);

        Class<?> visCls = Class.forName("net.minecraft.world.item.CreativeModeTab$TabVisibility");
        @SuppressWarnings("unchecked")
        Enum<?>[] visValues = (Enum<?>[]) visCls.getEnumConstants();
        Object PARENT_AND_SEARCH = visValues[0];
        Object PARENT_ONLY = visValues[1];
        Object SEARCH_ONLY = visValues[2];

        Method acceptStackVis =
                FabricCreativeModeTabOutput.class.getMethod("accept", ItemStack.class, visCls);
        Method acceptLikeVis =
                FabricCreativeModeTabOutput.class.getMethod("accept", ItemLike.class, visCls);

        ResourceKey<Item> keyA = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_a"));
        ResourceKey<Item> keyB = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_b"));
        ResourceKey<Item> keyC = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:enabled_c"));
        ResourceKey<Block> keyAltarBlock = ResourceKey.create(Registries.BLOCK, Identifier.parse("demo:altar"));
        ResourceKey<Item> keyAltarItem = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:altar"));
        ResourceKey<Item> keyDisabled = ResourceKey.create(Registries.ITEM, Identifier.parse("demo:disabled_trade"));

        Item enabledA = registerItem("demo:enabled_a", new Item(new Item.Properties().setId(keyA)));
        Item enabledB = registerItem("demo:enabled_b", new Item(new Item.Properties().setId(keyB)));
        Item enabledC = registerItem("demo:enabled_c", new Item(new Item.Properties().setId(keyC)));
        Block altarBlock = registerBlock("demo:altar", new Block(BlockBehaviour.Properties.of().setId(keyAltarBlock)));
        BlockItem altarItem = new BlockItem(altarBlock, new Item.Properties().setId(keyAltarItem));
        Item.BY_BLOCK.put(altarBlock, altarItem);
        registerItem("demo:altar", altarItem);
        Item disabledItem = registerItem(
                "demo:disabled_trade",
                new Item(new Item.Properties().setId(keyDisabled).requiredFeatures(FeatureFlags.TRADE_REBALANCE)));
        ItemLike customHolder = () -> enabledC;

        ResourceKey<CreativeModeTab> primaryKey = tabKey("demo:primary");
        CreativeModeTab primaryTab =
                FabricCreativeModeTab.builder().title(Component.literal("Primary Tab")).build();
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB, primaryKey, primaryTab);

        Identifier earlyPhase = Identifier.parse("demo:early");
        Identifier latePhase = Identifier.parse("demo:late");
        Identifier dynamicPhase = Identifier.parse("demo:dynamic");
        Identifier allLatePhase = Identifier.parse("demo:all_late");

        Event<CreativeModeTabEvents.ModifyOutput> primaryEvent =
                CreativeModeTabEvents.modifyOutputEvent(primaryKey);
        primaryEvent.addPhaseOrdering(earlyPhase, Event.DEFAULT_PHASE);
        primaryEvent.addPhaseOrdering(Event.DEFAULT_PHASE, latePhase);
        primaryEvent.addPhaseOrdering(latePhase, dynamicPhase);

        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.addPhaseOrdering(Event.DEFAULT_PHASE, allLatePhase);

        List<String> order = new ArrayList<>();
        FabricCreativeModeTabOutput[] seenOutput = new FabricCreativeModeTabOutput[1];
        boolean[] sharedPerTab = new boolean[1];
        boolean[] sharedAll = new boolean[1];
        boolean[] tabSame = new boolean[1];
        String[] tabTitle = new String[1];
        boolean[] dynamicAdded = new boolean[1];
        String[] errs = new String[7];

        // Register late BEFORE default and early to prove addPhaseOrdering governs dispatch order.
        primaryEvent.register(latePhase, output -> {
            order.add("late");
            sharedPerTab[0] = (output == seenOutput[0]);
            if (!dynamicAdded[0]) {
                dynamicAdded[0] = true;
                primaryEvent.register(dynamicPhase, o2 -> order.add("dynamic"));
            }
        });
        primaryEvent.register(output -> {
            order.add("default1");
            output.accept(enabledA);
            try {
                acceptLikeVis.invoke(output, customHolder, PARENT_ONLY);
                acceptStackVis.invoke(output, new ItemStack(altarItem), SEARCH_ONLY);
                // Disabled item: silently skipped (including count=2 and null visibility).
                output.accept(disabledItem);
                output.accept(new ItemStack(disabledItem, 2));
                acceptStackVis.invoke(output, new ItemStack(disabledItem, 2), null);
            } catch (Exception e) {
                throw new RuntimeException(e);
            }
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
                errs[6] = catchReflect(() -> acceptStackVis.invoke(output, new ItemStack(enabledA), null));
            }
        });
        primaryEvent.register(earlyPhase, output -> {
            order.add("early");
            seenOutput[0] = output;
            output.accept(new ItemStack(enabledB));
        });

        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register(allLatePhase, (tab, output) -> {
            order.add("all_late");
            sharedAll[0] = (output == seenOutput[0]);
            tabSame[0] = (tab == primaryTab);
            output.accept(enabledC);
        });
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register((tab, output) -> {
            order.add("all_default");
            tabTitle[0] = tab.getDisplayName().getString();
        });

        // Firing 1: default vanilla feature flags, per-tab then MODIFY_OUTPUT_ALL on ONE shared output.
        FabricCreativeModeTabOutput out1 = newOutput(FeatureFlags.VANILLA_SET, false);
        primaryEvent.invoker().modifyOutput(out1);
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.invoker().modifyOutput(primaryTab, out1);

        String firing1Order = String.join(",", order);
        String firing1Display = formatStacks(out1.getDisplayStacks());
        String firing1Search = formatStacks(out1.getSearchTabStacks());

        // Firing 2: dynamic listener registered during firing 1 now runs in phase order.
        order.clear();
        FabricCreativeModeTabOutput out2 = newOutput(FeatureFlags.VANILLA_SET, false);
        primaryEvent.invoker().modifyOutput(out2);
        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.invoker().modifyOutput(primaryTab, out2);
        String firing2Order = String.join(",", order);

        // Firing 3: with TRADE_REBALANCE enabled, disabledItem is accepted and count=2 throws.
        FeatureFlagSet withTrade = FeatureFlagSet.of(FeatureFlags.VANILLA, FeatureFlags.TRADE_REBALANCE);
        FabricCreativeModeTabOutput out3 = newOutput(withTrade, false);
        out3.accept(disabledItem);
        String tradeCount2Err = catchErr(() -> out3.accept(new ItemStack(disabledItem, 2)));
        String tradeDisplay = formatStacks(out3.getDisplayStacks());

        // Firing 4: empty FeatureFlagSet skips ItemStack.EMPTY because Items.AIR requires VANILLA.
        FabricCreativeModeTabOutput out4 = newOutput(FeatureFlagSet.of(), false);
        String emptyFlagsEmptyStack = catchErr(() -> out4.accept(ItemStack.EMPTY));

        // FeatureFlagSet and ItemStack checks.
        ItemStack stackA = new ItemStack(enabledA);
        ItemStack stackZero = new ItemStack(enabledA, 0);
        StringBuilder sb = new StringBuilder();
        sb.append("firing1_order=").append(firing1Order).append('\n');
        sb.append("firing1_shared=").append(sharedPerTab[0] && sharedAll[0]).append('\n');
        sb.append("firing1_tab_same=").append(tabSame[0]).append('\n');
        sb.append("firing1_tab_title=").append(tabTitle[0]).append('\n');
        sb.append("firing1_display=").append(firing1Display).append('\n');
        sb.append("firing1_search=").append(firing1Search).append('\n');
        sb.append("err_count2=").append(errs[0]).append('\n');
        sb.append("err_empty=").append(errs[1]).append('\n');
        sb.append("err_zero=").append(errs[2]).append('\n');
        sb.append("err_neg=").append(errs[3]).append('\n');
        sb.append("err_null_stack=").append(errs[4]).append('\n');
        sb.append("err_null_itemlike=").append(errs[5]).append('\n');
        sb.append("err_null_vis=").append(errs[6]).append('\n');
        sb.append("firing2_order=").append(firing2Order).append('\n');
        sb.append("trade_display=").append(tradeDisplay).append('\n');
        sb.append("trade_count2_err=").append(tradeCount2Err).append('\n');
        sb.append("empty_flags_empty_stack=").append(emptyFlagsEmptyStack).append('\n');
        sb.append("stack_props=")
                .append(stackA.isEmpty()).append('|')
                .append(stackA.getCount()).append('|')
                .append(stackA.is(enabledA)).append('|')
                .append(stackA.is(enabledB)).append('|')
                .append(stackA.toString()).append('|')
                .append(stackZero.isEmpty()).append('|')
                .append(stackZero.getCount()).append('|')
                .append(stackZero.toString()).append('\n');
        sb.append("flags_props=")
                .append(FeatureFlags.VANILLA_SET == FeatureFlags.DEFAULT_FLAGS).append('|')
                .append(FeatureFlags.VANILLA_SET.equals(FeatureFlagSet.of(FeatureFlags.VANILLA))).append('|')
                .append(FeatureFlagSet.of().isEmpty()).append('|')
                .append(FeatureFlagSet.of().isSubsetOf(FeatureFlags.VANILLA_SET)).append('|')
                .append(FeatureFlags.VANILLA_SET.isSubsetOf(FeatureFlagSet.of())).append('|')
                .append(withTrade.contains(FeatureFlags.TRADE_REBALANCE)).append('|')
                .append(disabledItem.isEnabled(FeatureFlags.VANILLA_SET)).append('|')
                .append(disabledItem.isEnabled(withTrade)).append('|')
                .append(out1.getEnabledFeatures().equals(FeatureFlags.VANILLA_SET)).append('|')
                .append(out1.shouldShowOpRestrictedItems()).append('\n');
        rawOut.print(sb);
    }
}

//! General vanilla 26.3 recipe decoding, matching, assembly, TransmuteRecipe, Ingredient,
//! ItemStackTemplate, DataComponentPatch, and ItemCombinerMenu.createResult fixture.
//!
//! Expected outputs in `fixtures/recipes/mc263.txt` were captured from a real Java 25 run
//! against Minecraft 26.3 server classes.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, Host, StdHost,
    events::{EventContext, EventFacade, EventFiring, FacadeLevel, FacadePlayer, GameEvent},
};
use std::sync::Arc;

const STUBS: &[(&str, &str)] = &[
    (
        "com/mojang/datafixers/kinds/App.java",
        "package com.mojang.datafixers.kinds; public interface App<F, A> {}",
    ),
    (
        "com/mojang/datafixers/kinds/Applicative.java",
        "package com.mojang.datafixers.kinds; public interface Applicative<F, Mu> {}",
    ),
    (
        "com/mojang/datafixers/util/Function6.java",
        "package com.mojang.datafixers.util;\n\
         @FunctionalInterface public interface Function6<T1,T2,T3,T4,T5,T6,R> { R apply(T1 a, T2 b, T3 c, T4 d, T5 e, T6 f); }",
    ),
    (
        "com/mojang/datafixers/Products.java",
        "package com.mojang.datafixers;\n\
         import com.mojang.datafixers.kinds.*; import com.mojang.datafixers.util.*;\n\
         public final class Products {\n\
           public static final class P6<F,T1,T2,T3,T4,T5,T6> {\n\
             public <R> App<F,R> apply(Applicative<F,?> app, Function6<T1,T2,T3,T4,T5,T6,R> f) { return null; }\n\
           }\n\
         }",
    ),
    (
        "com/mojang/serialization/Codec.java",
        "package com.mojang.serialization;\n\
         import java.util.Optional; import java.util.function.Function;\n\
         public interface Codec<A> {\n\
           Codec<Integer> INT = null;\n\
           Codec<String> STRING = null;\n\
           MapCodec<A> fieldOf(String name);\n\
           MapCodec<Optional<A>> optionalFieldOf(String name);\n\
           MapCodec<A> optionalFieldOf(String name, A defaultValue);\n\
           <S> Codec<S> xmap(Function<? super A, ? extends S> to, Function<? super S, ? extends A> from);\n\
         }",
    ),
    (
        "com/mojang/serialization/MapCodec.java",
        "package com.mojang.serialization;\n\
         import com.mojang.serialization.codecs.RecordCodecBuilder;\n\
         import java.util.function.Function;\n\
         public abstract class MapCodec<A> {\n\
           public <O> RecordCodecBuilder<O, A> forGetter(Function<O, A> getter) { return null; }\n\
           public <S> MapCodec<S> xmap(Function<? super A, ? extends S> to, Function<? super S, ? extends A> from) { return null; }\n\
           public Codec<A> codec() { return null; }\n\
         }",
    ),
    (
        "com/mojang/serialization/codecs/RecordCodecBuilder.java",
        "package com.mojang.serialization.codecs;\n\
         import com.mojang.datafixers.Products; import com.mojang.datafixers.kinds.*;\n\
         import com.mojang.serialization.*; import java.util.function.Function;\n\
         public final class RecordCodecBuilder<O, P> implements App<RecordCodecBuilder.Mu<O>, P> {\n\
           public static final class Mu<O> {}\n\
           public static final class Instance<O> implements Applicative<Mu<O>, Object> {\n\
             public <T1,T2,T3,T4,T5,T6> Products.P6<Mu<O>,T1,T2,T3,T4,T5,T6> group(\n\
               App<Mu<O>,T1> t1, App<Mu<O>,T2> t2, App<Mu<O>,T3> t3,\n\
               App<Mu<O>,T4> t4, App<Mu<O>,T5> t5, App<Mu<O>,T6> t6) { return null; }\n\
           }\n\
           public static <O> MapCodec<O> mapCodec(Function<Instance<O>, ? extends App<Mu<O>, O>> builder) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/network/codec/StreamCodec.java",
        "package net.minecraft.network.codec; public interface StreamCodec<B, V> {}",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier {\n\
           public static Identifier fromNamespaceAndPath(String ns, String path) { return null; }\n\
           public static Identifier parse(String id) { return null; }\n\
         }",
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
        "package net.minecraft.core;\n\
         import net.minecraft.resources.Identifier;\n\
         public interface Registry<T> {\n\
           static <V, T extends V> T register(Registry<V> reg, Identifier id, T value) { return value; }\n\
         }",
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        "package net.minecraft.core.registries;\n\
         import net.minecraft.resources.ResourceKey; import net.minecraft.core.Registry;\n\
         public final class Registries {\n\
           public static final ResourceKey<Registry<net.minecraft.world.item.Item>> ITEM = null;\n\
           public static final ResourceKey<Registry<net.minecraft.world.item.crafting.Recipe<?>>> RECIPE = null;\n\
         }",
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        "package net.minecraft.core.registries;\n\
         import net.minecraft.core.Registry;\n\
         public final class BuiltInRegistries {\n\
           public static final Registry<net.minecraft.world.item.Item> ITEM = null;\n\
           public static final Registry<net.minecraft.world.item.crafting.RecipeType<?>> RECIPE_TYPE = null;\n\
           public static final Registry<net.minecraft.world.item.crafting.RecipeSerializer<?>> RECIPE_SERIALIZER = null;\n\
         }",
    ),
    (
        "net/minecraft/core/Holder.java",
        "package net.minecraft.core;\n\
         public interface Holder<T> { T value(); String getRegisteredName(); }",
    ),
    (
        "net/minecraft/core/component/DataComponentType.java",
        "package net.minecraft.core.component; public interface DataComponentType<T> {}",
    ),
    (
        "net/minecraft/core/component/DataComponentPatch.java",
        "package net.minecraft.core.component;\n\
         public final class DataComponentPatch {\n\
           public static final DataComponentPatch EMPTY = null;\n\
           public static Builder builder() { return null; }\n\
           public boolean isEmpty() { return true; }\n\
           public int size() { return 0; }\n\
           public static final class Builder {\n\
             public <T> Builder set(DataComponentType<T> t, T v) { return this; }\n\
             public <T> Builder remove(DataComponentType<T> t) { return this; }\n\
             public DataComponentPatch build() { return null; }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/core/component/DataComponents.java",
        "package net.minecraft.core.component;\n\
         import net.minecraft.network.chat.Component;\n\
         public final class DataComponents {\n\
           public static final DataComponentType<Integer> REPAIR_COST = null;\n\
           public static final DataComponentType<Component> CUSTOM_NAME = null;\n\
         }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat;\n\
         public interface Component {\n\
           static MutableComponent literal(String text) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat;\n\
         public class MutableComponent implements Component {}",
    ),
    (
        "net/minecraft/world/level/ItemLike.java",
        "package net.minecraft.world.level;\n\
         import net.minecraft.world.item.Item;\n\
         public interface ItemLike { Item asItem(); }",
    ),
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item;\n\
         import net.minecraft.resources.ResourceKey; import net.minecraft.world.level.ItemLike;\n\
         public class Item implements ItemLike {\n\
           public Item(Properties props) {}\n\
           public Item asItem() { return this; }\n\
           public static class Properties {\n\
             public Properties() {}\n\
             public Properties stacksTo(int n) { return this; }\n\
             public Properties setId(ResourceKey<Item> key) { return this; }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/world/item/Items.java",
        "package net.minecraft.world.item;\n\
         public final class Items {\n\
           public static final Item AIR = null;\n\
           public static final Item STONE = null;\n\
           public static final Item COBBLESTONE = null;\n\
           public static final Item BLACKSTONE = null;\n\
           public static final Item COBBLED_DEEPSLATE = null;\n\
           public static final Item DIRT = null;\n\
           public static final Item IRON_INGOT = null;\n\
           public static final Item GOLD_INGOT = null;\n\
           public static final Item COPPER_INGOT = null;\n\
           public static final Item REDSTONE = null;\n\
           public static final Item AMETHYST_SHARD = null;\n\
           public static final Item COAL = null;\n\
           public static final Item DIAMOND = null;\n\
           public static final Item GOLD_NUGGET = null;\n\
           public static final Item STICK = null;\n\
           public static final Item ENDER_PEARL = null;\n\
         }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\n\
         import net.minecraft.core.component.*; import net.minecraft.world.level.ItemLike;\n\
         public final class ItemStack {\n\
           public static final ItemStack EMPTY = null;\n\
           public ItemStack(ItemLike item) {}\n\
           public ItemStack(ItemLike item, int count) {}\n\
           public boolean isEmpty() { return true; }\n\
           public int getCount() { return 0; }\n\
           public Item getItem() { return null; }\n\
           public boolean is(Item item) { return false; }\n\
           public DataComponentPatch getComponentsPatch() { return null; }\n\
           public <T> T set(DataComponentType<T> type, T value) { return null; }\n\
           public <T> T get(DataComponentType<? extends T> type) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/ItemStackTemplate.java",
        "package net.minecraft.world.item;\n\
         import com.mojang.serialization.Codec; import net.minecraft.core.Holder;\n\
         import net.minecraft.core.component.DataComponentPatch;\n\
         public final class ItemStackTemplate {\n\
           public static final Codec<ItemStackTemplate> CODEC = null;\n\
           public ItemStackTemplate(Item item, int count) {}\n\
           public static ItemStackTemplate fromNonEmptyStack(ItemStack stack) { return null; }\n\
           public Holder<Item> item() { return null; }\n\
           public int count() { return 0; }\n\
           public DataComponentPatch components() { return null; }\n\
           public ItemStack create() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/Ingredient.java",
        "package net.minecraft.world.item.crafting;\n\
         import com.mojang.serialization.Codec; import java.util.Optional;\n\
         import net.minecraft.world.item.ItemStack; import net.minecraft.world.level.ItemLike;\n\
         public final class Ingredient {\n\
           public static final Codec<Ingredient> CODEC = null;\n\
           public static Ingredient of(ItemLike item) { return null; }\n\
           public static boolean testOptionalIngredient(Optional<Ingredient> opt, ItemStack stack) { return false; }\n\
           public boolean test(ItemStack stack) { return false; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeInput.java",
        "package net.minecraft.world.item.crafting;\n\
         import net.minecraft.world.item.ItemStack;\n\
         public interface RecipeInput { ItemStack getItem(int index); int size(); }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeType.java",
        "package net.minecraft.world.item.crafting; public interface RecipeType<T extends Recipe<?>> {}",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeSerializer.java",
        "package net.minecraft.world.item.crafting;\n\
         import com.mojang.serialization.MapCodec; import net.minecraft.network.codec.StreamCodec;\n\
         public final class RecipeSerializer<T extends Recipe<?>> {\n\
           public RecipeSerializer(MapCodec<T> codec, StreamCodec<?, T> streamCodec) {}\n\
           public MapCodec<T> codec() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/Recipe.java",
        "package net.minecraft.world.item.crafting;\n\
         import com.mojang.serialization.MapCodec;\n\
         import net.minecraft.world.item.ItemStack; import net.minecraft.world.level.Level;\n\
         public interface Recipe<I extends RecipeInput> {\n\
           boolean matches(I input, Level level);\n\
           ItemStack assemble(I input);\n\
           RecipeSerializer<? extends Recipe<I>> getSerializer();\n\
           RecipeType<? extends Recipe<I>> getType();\n\
           final class CommonInfo {\n\
             public static final MapCodec<CommonInfo> MAP_CODEC = null;\n\
             public CommonInfo(boolean showNotification) {}\n\
             public boolean showNotification() { return true; }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeHolder.java",
        "package net.minecraft.world.item.crafting;\n\
         import net.minecraft.resources.ResourceKey;\n\
         public final class RecipeHolder<T extends Recipe<?>> {\n\
           public RecipeHolder(ResourceKey<Recipe<?>> id, T value) {}\n\
           public ResourceKey<Recipe<?>> id() { return null; }\n\
           public T value() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/RecipeManager.java",
        "package net.minecraft.world.item.crafting;\n\
         import java.util.Optional; import net.minecraft.resources.ResourceKey;\n\
         import net.minecraft.world.level.Level;\n\
         public class RecipeManager {\n\
           public RecipeManager() {}\n\
           public Object getRecipeFor() { return null; }\n\
           public <I extends RecipeInput, T extends Recipe<I>> Optional<RecipeHolder<T>> getRecipeFor(RecipeType<T> type, I input, Level level) { return Optional.empty(); }\n\
           public <I extends RecipeInput, T extends Recipe<I>> Optional<RecipeHolder<T>> getRecipeFor(RecipeType<T> type, I input, Level level, ResourceKey<Recipe<?>> last) { return Optional.empty(); }\n\
           public <I extends RecipeInput, T extends Recipe<I>> Optional<RecipeHolder<T>> getRecipeFor(RecipeType<T> type, I input, Level level, RecipeHolder<T> last) { return Optional.empty(); }\n\
           public Optional<RecipeHolder<?>> byKey(ResourceKey<Recipe<?>> key) { return Optional.empty(); }\n\
         }",
    ),
    (
        "net/minecraft/world/item/crafting/TransmuteRecipe.java",
        "package net.minecraft.world.item.crafting;\n\
         import net.minecraft.world.item.*;\n\
         public final class TransmuteRecipe {\n\
           public static ItemStack createWithOriginalComponents(ItemStackTemplate tpl, ItemStack base) { return null; }\n\
           public static ItemStack createWithOriginalComponents(ItemStackTemplate tpl, ItemStack base, int extra) { return null; }\n\
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
         public class ServerLevel extends Level {\n\
           public RecipeManager recipeAccess() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player;\n\
         import net.minecraft.world.level.Level;\n\
         public class Player { public Level level() { return null; } }",
    ),
    (
        "net/minecraft/world/entity/EntityEquipment.java",
        "package net.minecraft.world.entity; public class EntityEquipment {}",
    ),
    (
        "net/minecraft/world/entity/player/Inventory.java",
        "package net.minecraft.world.entity.player;\n\
         import net.minecraft.world.entity.EntityEquipment;\n\
         public class Inventory { public final Player player; public Inventory(Player p, EntityEquipment e) { player = p; } }",
    ),
    (
        "net/minecraft/core/NonNullList.java",
        "package net.minecraft.core; import java.util.ArrayList; public class NonNullList<T> extends ArrayList<T> {}",
    ),
    (
        "net/minecraft/world/Container.java",
        "package net.minecraft.world; import net.minecraft.world.item.ItemStack;\n\
         public interface Container { int getContainerSize(); ItemStack getItem(int i); void setItem(int i, ItemStack s); void setChanged(); boolean isEmpty(); }",
    ),
    (
        "net/minecraft/world/SimpleContainer.java",
        "package net.minecraft.world; import net.minecraft.world.item.ItemStack;\n\
         public class SimpleContainer implements Container {\n\
           public SimpleContainer(int size) {}\n\
           public int getContainerSize() { return 0; }\n\
           public ItemStack getItem(int i) { return null; }\n\
           public void setItem(int i, ItemStack s) {}\n\
           public void setChanged() {}\n\
           public boolean isEmpty() { return true; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ResultContainer.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container; import net.minecraft.world.item.ItemStack;\n\
         import net.minecraft.world.item.crafting.RecipeHolder;\n\
         public class ResultContainer implements Container {\n\
           public ResultContainer() {}\n\
           public int getContainerSize() { return 1; }\n\
           public ItemStack getItem(int i) { return null; }\n\
           public void setItem(int i, ItemStack s) {}\n\
           public void setChanged() {}\n\
           public boolean isEmpty() { return true; }\n\
           public void setRecipeUsed(RecipeHolder<?> h) {}\n\
           public RecipeHolder<?> getRecipeUsed() { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/MenuType.java",
        "package net.minecraft.world.inventory; public class MenuType<T> {}",
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
         public class Slot { public final Container container; public Slot(Container c, int i, int x, int y) { container = c; } }",
    ),
    (
        "net/minecraft/world/inventory/AbstractContainerMenu.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.core.NonNullList;\n\
         public abstract class AbstractContainerMenu {\n\
           public final NonNullList<Slot> slots = new NonNullList<>();\n\
           protected AbstractContainerMenu(MenuType<?> t, int id) {}\n\
           public Slot getSlot(int index) { return null; }\n\
         }",
    ),
    (
        "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition.java",
        "package net.minecraft.world.inventory;\n\
         import java.util.function.Predicate; import net.minecraft.world.item.ItemStack;\n\
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
        "net/minecraft/world/level/block/state/BlockState.java",
        "package net.minecraft.world.level.block.state; public class BlockState {}",
    ),
    (
        "net/minecraft/world/inventory/ItemCombinerMenu.java",
        "package net.minecraft.world.inventory;\n\
         import net.minecraft.world.Container; import net.minecraft.world.entity.player.*;\n\
         import net.minecraft.world.item.ItemStack; import net.minecraft.world.level.block.state.BlockState;\n\
         public abstract class ItemCombinerMenu extends AbstractContainerMenu {\n\
           protected final Container inputSlots;\n\
           protected final ResultContainer resultSlots;\n\
           protected ItemCombinerMenu(MenuType<?> t, int id, Inventory inv, ContainerLevelAccess acc, ItemCombinerMenuSlotDefinition def) {\n\
             super(t, id); inputSlots = null; resultSlots = null;\n\
           }\n\
           public abstract void createResult();\n\
           protected abstract void onTake(Player p, ItemStack s);\n\
           protected abstract boolean isValidBlock(BlockState s);\n\
         }",
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
];

const DEMO_RECIPE_INPUT: &str = r#"
package demo;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.crafting.RecipeInput;

public final class AlloyInput implements RecipeInput {
  public final ItemStack base;
  public final ItemStack catalyst;
  public AlloyInput(ItemStack base, ItemStack catalyst) {
    this.base = base;
    this.catalyst = catalyst;
  }
  public ItemStack getItem(int index) { return index == 0 ? base : catalyst; }
  public int size() { return 2; }
}
"#;

const DEMO_RECIPE: &str = r#"
package demo;
import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import java.util.Optional;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.ItemStackTemplate;
import net.minecraft.world.item.crafting.*;
import net.minecraft.world.level.Level;

public final class AlloyRecipe implements Recipe<AlloyInput> {
  public final Recipe.CommonInfo commonInfo;
  public final Ingredient base;
  public final Ingredient catalyst;
  public final ItemStackTemplate result;
  public final int tier;
  public final Optional<String> label;

  public static final RecipeType<AlloyRecipe> TYPE = new RecipeType<>() {};
  public static final MapCodec<AlloyRecipe> MAP_CODEC = RecordCodecBuilder.mapCodec(i -> i.group(
      Recipe.CommonInfo.MAP_CODEC.forGetter(r -> r.commonInfo),
      Ingredient.CODEC.fieldOf("base").forGetter(r -> r.base),
      Ingredient.CODEC.fieldOf("catalyst").forGetter(r -> r.catalyst),
      ItemStackTemplate.CODEC.fieldOf("result").forGetter(r -> r.result),
      Codec.INT.optionalFieldOf("tier", 1).forGetter(r -> r.tier),
      Codec.STRING.optionalFieldOf("label").xmap(
          opt -> opt.map(String::toUpperCase),
          opt -> opt.map(String::toLowerCase)).forGetter(r -> r.label)
  ).apply(i, AlloyRecipe::new));
  public static final RecipeSerializer<AlloyRecipe> SERIALIZER = new RecipeSerializer<>(MAP_CODEC, null);

  public AlloyRecipe(
      Recipe.CommonInfo commonInfo,
      Ingredient base,
      Ingredient catalyst,
      ItemStackTemplate result,
      int tier,
      Optional<String> label) {
    this.commonInfo = commonInfo;
    this.base = base;
    this.catalyst = catalyst;
    this.result = result;
    this.tier = tier;
    this.label = label;
  }

  public boolean matches(AlloyInput input, Level level) {
    return base.test(input.base) && catalyst.test(input.catalyst);
  }

  public ItemStack assemble(AlloyInput input) {
    return TransmuteRecipe.createWithOriginalComponents(result, input.base);
  }

  public RecipeSerializer<AlloyRecipe> getSerializer() { return SERIALIZER; }
  public RecipeType<AlloyRecipe> getType() { return TYPE; }
}
"#;

const DEMO_MENU: &str = r#"
package demo;
import java.util.Optional;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.player.*;
import net.minecraft.world.inventory.*;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.crafting.RecipeHolder;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;

public final class AlloyMenu extends ItemCombinerMenu {
  private final Level world;
  public int createResultCalls;

  public AlloyMenu(int id, Inventory inv) {
    super(null, id, inv, ContainerLevelAccess.NULL, ItemCombinerMenuSlotDefinition.create()
        .withSlot(0, 27, 47, s -> true)
        .withSlot(1, 76, 47, s -> true)
        .withResultSlot(2, 134, 47)
        .build());
    this.world = inv.player.level();
  }

  public void createResult() {
    createResultCalls++;
    AlloyInput input = new AlloyInput(inputSlots.getItem(0), inputSlots.getItem(1));
    Optional<RecipeHolder<AlloyRecipe>> match = (world instanceof ServerLevel sl)
        ? sl.recipeAccess().getRecipeFor(AlloyRecipe.TYPE, input, sl)
        : Optional.empty();
    if (match.isPresent()) {
      RecipeHolder<AlloyRecipe> holder = match.get();
      resultSlots.setRecipeUsed(holder);
      resultSlots.setItem(0, holder.value().assemble(input));
    } else {
      resultSlots.setRecipeUsed(null);
      resultSlots.setItem(0, ItemStack.EMPTY);
    }
  }

  public void setInputs(ItemStack a, ItemStack b) {
    inputSlots.setItem(0, a);
    inputSlots.setItem(1, b);
  }

  public ItemStack resultItem() { return resultSlots.getItem(0); }
  public RecipeHolder<?> usedRecipe() { return resultSlots.getRecipeUsed(); }
  public boolean inputsEmpty() { return inputSlots.isEmpty(); }
  public boolean resultEmpty() { return resultSlots.isEmpty(); }
  protected void onTake(Player p, ItemStack s) {}
  protected boolean isValidBlock(BlockState s) { return true; }
}
"#;

const DEMO_PROBE: &str = r#"
package demo;
import java.util.Optional;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.*;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.*;
import net.minecraft.world.entity.player.*;
import net.minecraft.world.item.*;
import net.minecraft.world.item.crafting.*;
import net.minecraft.world.level.Level;

public final class Probe {
  public static Item ALLOY_ITEM;
  public static Item SMALL_ITEM;
  public static String MENU_SUMMARY;
  public static String OVERLOAD_SUMMARY;

  private static void capture(String name, Runnable action) {
    try {
      action.run();
      System.out.println(name + "=ok");
    } catch (Throwable t) {
      System.out.println(name + "=" + t.getClass().getName() + ":" + t.getMessage());
    }
  }

  public static void onInitialize() {
    Identifier alloyId = Identifier.fromNamespaceAndPath("demo", "alloy_ingot");
    ALLOY_ITEM = Registry.register(
        BuiltInRegistries.ITEM,
        alloyId,
        new Item(new Item.Properties().setId(ResourceKey.create(Registries.ITEM, alloyId))));
    Identifier smallId = Identifier.fromNamespaceAndPath("demo", "small_stack_item");
    SMALL_ITEM = Registry.register(
        BuiltInRegistries.ITEM,
        smallId,
        new Item(new Item.Properties().stacksTo(16).setId(ResourceKey.create(Registries.ITEM, smallId))));
    Identifier typeId = Identifier.fromNamespaceAndPath("demo", "alloy");
    Registry.register(BuiltInRegistries.RECIPE_TYPE, typeId, AlloyRecipe.TYPE);
    Registry.register(BuiltInRegistries.RECIPE_SERIALIZER, typeId, AlloyRecipe.SERIALIZER);

    UseItemCallback.EVENT.register((player, level, hand) -> {
      runAll(player, (ServerLevel) level);
      return InteractionResult.PASS;
    });
  }

  private static void runAll(Player player, ServerLevel level) {
    RecipeManager mgr = level.recipeAccess();
    String[] validIds = {"demo:iron_alloy", "demo:precious_alloy", "demo:stone_alloy", "demo:small_stack"};
    for (String id : validIds) {
      ResourceKey<Recipe<?>> key = ResourceKey.create(Registries.RECIPE, Identifier.parse(id));
      AlloyRecipe rec = (AlloyRecipe) mgr.byKey(key).get().value();
      System.out.println("decoded:" + id
          + "|notify=" + rec.commonInfo.showNotification()
          + "|tier=" + rec.tier
          + "|label=" + rec.label.orElse("none")
          + "|count=" + rec.result.count());
    }

    String[] invalidIds = {
        "demo:bad_air", "demo:bad_empty_list", "demo:bad_missing_tag", "demo:bad_count", "demo:bad_result_air"
    };
    int invalidErrors = 0;
    for (String id : invalidIds) {
      ResourceKey<Recipe<?>> key = ResourceKey.create(Registries.RECIPE, Identifier.parse(id));
      if (mgr.byKey(key).isEmpty()) {
        invalidErrors++;
      }
    }
    System.out.println("invalidErrorCount=" + invalidErrors);

    ItemStack taggedBase = new ItemStack(Items.COBBLESTONE, 3);
    taggedBase.set(DataComponents.REPAIR_COST, 5);
    taggedBase.set(DataComponents.CUSTOM_NAME, Component.literal("Ancient"));

    ItemStack ironWithComp = new ItemStack(Items.IRON_INGOT, 1);
    ironWithComp.set(DataComponents.REPAIR_COST, 9);

    String[] names = {
        "iron+redstone", "gold+amethyst", "copper+amethyst",
        "cobble+coal", "blackstone+coal", "deepslate+coal",
        "diamond+nugget", "dirt+coal", "empty+empty"
    };
    ItemStack[] left = {
        ironWithComp, new ItemStack(Items.GOLD_INGOT), new ItemStack(Items.COPPER_INGOT),
        taggedBase, new ItemStack(Items.BLACKSTONE), new ItemStack(Items.COBBLED_DEEPSLATE),
        new ItemStack(Items.DIAMOND), new ItemStack(Items.DIRT), ItemStack.EMPTY
    };
    ItemStack[] right = {
        new ItemStack(Items.REDSTONE), new ItemStack(Items.AMETHYST_SHARD), new ItemStack(Items.AMETHYST_SHARD),
        new ItemStack(Items.COAL), new ItemStack(Items.COAL), new ItemStack(Items.COAL),
        new ItemStack(Items.GOLD_NUGGET), new ItemStack(Items.COAL), ItemStack.EMPTY
    };

    RecipeHolder<AlloyRecipe> firstHolder = null;
    for (int i = 0; i < names.length; i++) {
      AlloyInput input = new AlloyInput(left[i], right[i]);
      Optional<RecipeHolder<AlloyRecipe>> found = mgr.getRecipeFor(AlloyRecipe.TYPE, input, level);
      String foundId = "empty";
      ItemStack out = ItemStack.EMPTY;
      if (found.isPresent()) {
        RecipeHolder<AlloyRecipe> h = found.get();
        if (firstHolder == null) firstHolder = h;
        foundId = h.id().identifier().toString();
        out = h.value().assemble(input);
      }
      System.out.println("query:" + names[i]
          + "|id=" + foundId
          + "|out=" + out.toString()
          + "|empty=" + out.isEmpty()
          + "|repairCost=" + out.get(DataComponents.REPAIR_COST)
          + "|hasCustomName=" + (out.get(DataComponents.CUSTOM_NAME) != null));
    }

    // 4-arg cached key and cached holder overloads
    AlloyInput ironInput = new AlloyInput(ironWithComp, new ItemStack(Items.REDSTONE));
    Optional<RecipeHolder<AlloyRecipe>> byHintKey =
        mgr.getRecipeFor(AlloyRecipe.TYPE, ironInput, level, firstHolder.id());
    Optional<RecipeHolder<AlloyRecipe>> byHintHolder =
        mgr.getRecipeFor(AlloyRecipe.TYPE, ironInput, level, firstHolder);
    OVERLOAD_SUMMARY = byHintKey.get().id().identifier().toString()
        + "|" + byHintHolder.get().id().identifier().toString();

    ItemStackTemplate pearl20 = new ItemStackTemplate(Items.ENDER_PEARL, 20);
    ItemStack pearlOut = TransmuteRecipe.createWithOriginalComponents(pearl20, new ItemStack(Items.STONE));
    System.out.println("pearl20=" + pearlOut.toString() + "|empty=" + pearlOut.isEmpty());

    ItemStackTemplate small20 = new ItemStackTemplate(SMALL_ITEM, 20);
    ItemStack smallOut20 = TransmuteRecipe.createWithOriginalComponents(small20, new ItemStack(Items.STONE));
    System.out.println("small20=" + smallOut20.toString() + "|empty=" + smallOut20.isEmpty());

    ItemStackTemplate stick2 = new ItemStackTemplate(Items.STICK, 2);
    ItemStack stickExtra = TransmuteRecipe.createWithOriginalComponents(stick2, ironWithComp, 3);
    System.out.println("stickExtra=" + stickExtra.toString() + "|repairCost=" + stickExtra.get(DataComponents.REPAIR_COST));

    Recipe.CommonInfo ciTrue = new Recipe.CommonInfo(true);
    Recipe.CommonInfo ciFalse = new Recipe.CommonInfo(false);
    System.out.println("ci=" + ciTrue + "|" + ciFalse
        + "|eq=" + ciTrue.equals(new Recipe.CommonInfo(true)) + "|" + ciTrue.equals(ciFalse)
        + "|hash=" + ciTrue.hashCode() + "|" + ciFalse.hashCode());
    System.out.println("emptyStack=" + ItemStack.EMPTY.toString()
        + "|isAir=" + ItemStack.EMPTY.is(Items.AIR)
        + "|isStick=" + ItemStack.EMPTY.is(Items.STICK)
        + "|patchEmpty=" + ItemStack.EMPTY.getComponentsPatch().isEmpty()
        + "|patchStr=" + ItemStack.EMPTY.getComponentsPatch().toString());
    System.out.println("optIngr="
        + Ingredient.testOptionalIngredient(Optional.empty(), ItemStack.EMPTY) + "|"
        + Ingredient.testOptionalIngredient(Optional.empty(), new ItemStack(Items.STONE)) + "|"
        + Ingredient.testOptionalIngredient(Optional.of(Ingredient.of(Items.STONE)), new ItemStack(Items.STONE)));

    SimpleContainer box = new SimpleContainer(2);
    ItemStack pearls = new ItemStack(Items.ENDER_PEARL, 30);
    box.setItem(0, pearls);
    System.out.println("boxClamp=" + box.getItem(0).toString() + "|boxEmpty=" + box.isEmpty());

    capture("ingrAir", () -> Ingredient.of(Items.AIR));
    capture("tplZero", () -> new ItemStackTemplate(Items.STONE, 0));
    capture("tplAir", () -> new ItemStackTemplate(Items.AIR, 1));
    capture("fromEmpty", () -> ItemStackTemplate.fromNonEmptyStack(ItemStack.EMPTY));

    ResourceKey<Recipe<?>> rk1 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:one"));
    ResourceKey<Recipe<?>> rk2 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:one"));
    ResourceKey<Recipe<?>> rk3 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:two"));
    RecipeHolder<?> h1 = new RecipeHolder<>(rk1, null);
    RecipeHolder<?> h2 = new RecipeHolder<>(rk2, null);
    RecipeHolder<?> h3 = new RecipeHolder<>(rk3, null);
    System.out.println("holder=" + h1.toString() + "|eqSame=" + h1.equals(h2) + "|eqDiff=" + h1.equals(h3));

    // Drive mod ItemCombinerMenu.createResult via inputSlots.setItem
    AlloyMenu menu = new AlloyMenu(1, new Inventory(player, null));
    menu.setInputs(new ItemStack(Items.COBBLESTONE, 1), new ItemStack(Items.COAL, 1));
    String hitRes = menu.resultItem().toString()
        + "|used=" + menu.usedRecipe().id().identifier().toString()
        + "|inputsEmpty=" + menu.inputsEmpty()
        + "|resultEmpty=" + menu.resultEmpty()
        + "|calls=" + menu.createResultCalls;
    menu.setInputs(new ItemStack(Items.DIRT, 1), new ItemStack(Items.COAL, 1));
    String missRes = menu.resultItem().toString()
        + "|used=" + menu.usedRecipe()
        + "|resultEmpty=" + menu.resultEmpty();
    MENU_SUMMARY = hitRes + "||" + missRes;

    // Bare RecipeManager remains counted.
    new RecipeManager().getRecipeFor();
  }
}
"#;

#[test]
fn general_recipe_codec_matching_transmute_and_menu_match_mc263_oracle() {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/AlloyInput.java", DEMO_RECIPE_INPUT));
    sources.push(("demo/AlloyRecipe.java", DEMO_RECIPE));
    sources.push(("demo/AlloyMenu.java", DEMO_MENU));
    sources.push(("demo/Probe.java", DEMO_PROBE));

    let extra: &[(&str, &[u8])] = &[
        (
            "data/demo/tags/item/reagents.json",
            br##"{"values":["#minecraft:stone_tool_materials"]}"##,
        ),
        (
            "data/demo/recipe/iron_alloy.json",
            br#"{"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:redstone","result":{"id":"demo:alloy_ingot","count":4},"tier":2,"label":"basic"}"#,
        ),
        (
            "data/demo/recipe/precious_alloy.json",
            br#"{"type":"demo:alloy","show_notification":false,"base":["minecraft:gold_ingot","minecraft:copper_ingot"],"catalyst":"minecraft:amethyst_shard","result":"demo:alloy_ingot"}"#,
        ),
        (
            "data/demo/recipe/stone_alloy.json",
            br##"{"type":"demo:alloy","base":"#demo:reagents","catalyst":"minecraft:coal","result":{"id":"minecraft:stick","count":2,"components":{"minecraft:repair_cost":7}}}"##,
        ),
        (
            "data/demo/recipe/small_stack.json",
            br#"{"type":"demo:alloy","base":"minecraft:diamond","catalyst":"minecraft:gold_nugget","result":{"id":"demo:small_stack_item","count":16}}"#,
        ),
        (
            "data/demo/recipe/bad_air.json",
            br#"{"type":"demo:alloy","base":"minecraft:air","catalyst":"minecraft:coal","result":"demo:alloy_ingot"}"#,
        ),
        (
            "data/demo/recipe/bad_empty_list.json",
            br#"{"type":"demo:alloy","base":[],"catalyst":"minecraft:coal","result":"demo:alloy_ingot"}"#,
        ),
        (
            "data/demo/recipe/bad_missing_tag.json",
            br##"{"type":"demo:alloy","base":"#demo:no_such_tag","catalyst":"minecraft:coal","result":"demo:alloy_ingot"}"##,
        ),
        (
            "data/demo/recipe/bad_count.json",
            br#"{"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:coal","result":{"id":"demo:alloy_ingot","count":100}}"#,
        ),
        (
            "data/demo/recipe/bad_result_air.json",
            br#"{"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:coal","result":{"id":"minecraft:air","count":1}}"#,
        ),
        (
            "data/demo/recipe/vanilla_shaped.json",
            br#"{"type":"minecraft:crafting_shaped","pattern":["X"],"key":{"X":"minecraft:stick"},"result":{"id":"minecraft:stick"}}"#,
        ),
    ];

    let fixture = javac_fixture::Fixture::build_with_mod_json_and_extra(
        "recipes",
        &sources,
        r#"{"id":"demo","version":"1.0","entrypoints":{"main":"demo.Probe"}}"#,
        extra,
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture
        .jar
        .execute("demo.Probe", None, "main", &mut host)
        .unwrap_or_else(|err| panic!("onInitialize failed: {err:?}"));

    let listeners = host.game_listeners(&fixture.jar, true);
    assert_eq!(listeners.listeners.len(), 1);
    let player_uuid = 0x1111_2222_3333_4444_5555_6666_7777_8888;
    let facade = Arc::new(EventFacade {
        players: vec![FacadePlayer {
            uuid: player_uuid,
            name: "Tester".into(),
            level: Some("minecraft:overworld".into()),
            entity_id: Some(42),
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
        .unwrap_or_else(|err| panic!("listener failed: {err:?}"));

    assert_eq!(
        host.take_stdout(),
        include_str!("fixtures/recipes/mc263.txt")
    );
    assert_eq!(
        host.loaded_recipe_ids(),
        vec![
            "demo:iron_alloy",
            "demo:precious_alloy",
            "demo:small_stack",
            "demo:stone_alloy",
        ]
    );
    assert_eq!(
        host.recipe_decode_errors().len(),
        5,
        "errors={:?}",
        host.recipe_decode_errors()
    );

    let statics = host.persistent_class_state().0;
    assert_eq!(
        statics.get("demo/Probe#OVERLOAD_SUMMARYLjava/lang/String;"),
        Some(&horizon_native_java::Value::Ref(Some(
            "demo:iron_alloy|demo:iron_alloy".into()
        )))
    );
    assert_eq!(
        statics.get("demo/Probe#MENU_SUMMARYLjava/lang/String;"),
        Some(&horizon_native_java::Value::Ref(Some(
            "2 minecraft:stick|used=demo:stone_alloy|inputsEmpty=false|resultEmpty=false|calls=2||0 minecraft:air|used=null|resultEmpty=true"
                .into()
        )))
    );
    assert!(host.unknown_host_calls().iter().any(|(call, _)| {
        call.starts_with(
            "net/minecraft/world/item/crafting/RecipeManager#getRecipeFor()Ljava/lang/Object;",
        )
    }));
}

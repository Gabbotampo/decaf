import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import com.mojang.serialization.Codec;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.JsonOps;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import java.lang.reflect.Field;
import java.util.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.Holder;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponentPatch;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.RegistryOps;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.tags.TagKey;
import net.minecraft.tags.TagLoader;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.ItemStackTemplate;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.crafting.*;

public final class GeneralRecipesOracle {
  static void unfreezeItemRegistry() throws Exception {
    Class<?> mapped = BuiltInRegistries.ITEM.getClass().getSuperclass();
    Field frozen = mapped.getDeclaredField("frozen");
    frozen.setAccessible(true);
    frozen.setBoolean(BuiltInRegistries.ITEM, false);
    Field intrusive = mapped.getDeclaredField("unregisteredIntrusiveHolders");
    intrusive.setAccessible(true);
    intrusive.set(BuiltInRegistries.ITEM, new IdentityHashMap<>());
  }

  static void refreezeFlag(boolean value) throws Exception {
    Class<?> mapped = BuiltInRegistries.ITEM.getClass().getSuperclass();
    Field frozen = mapped.getDeclaredField("frozen");
    frozen.setAccessible(true);
    frozen.setBoolean(BuiltInRegistries.ITEM, value);
    if (value) {
      Field intrusive = mapped.getDeclaredField("unregisteredIntrusiveHolders");
      intrusive.setAccessible(true);
      intrusive.set(BuiltInRegistries.ITEM, null);
    }
  }

  static void initMc263() throws Exception {
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    HolderLookup.Provider lookup = VanillaRegistries.createWorldLookup();
    BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(lookup).forEach(p -> p.apply());
  }

  static java.io.PrintStream out;

  static void capture(String name, Runnable action) {
    try {
      action.run();
      out.println(name + "=ok");
    } catch (Throwable t) {
      out.println(name + "=" + t.getClass().getName() + ":" + t.getMessage());
    }
  }

  public record AlloyRecipe(
      Recipe.CommonInfo commonInfo,
      Ingredient base,
      Ingredient catalyst,
      ItemStackTemplate result,
      int tier,
      Optional<String> label) {
    static final MapCodec<AlloyRecipe> MAP_CODEC = RecordCodecBuilder.mapCodec(i -> i.group(
        Recipe.CommonInfo.MAP_CODEC.forGetter(AlloyRecipe::commonInfo),
        Ingredient.CODEC.fieldOf("base").forGetter(AlloyRecipe::base),
        Ingredient.CODEC.fieldOf("catalyst").forGetter(AlloyRecipe::catalyst),
        ItemStackTemplate.CODEC.fieldOf("result").forGetter(AlloyRecipe::result),
        Codec.INT.optionalFieldOf("tier", 1).forGetter(AlloyRecipe::tier),
        Codec.STRING.optionalFieldOf("label").xmap(
            opt -> opt.map(String::toUpperCase),
            opt -> opt.map(String::toLowerCase)).forGetter(AlloyRecipe::label)
    ).apply(i, AlloyRecipe::new));

    boolean matches(ItemStack a, ItemStack b) {
      return base.test(a) && catalyst.test(b);
    }

    ItemStack assemble(ItemStack a) {
      return TransmuteRecipe.createWithOriginalComponents(result, a);
    }
  }

  public static void main(String[] args) throws Exception {
    out = System.out;
    initMc263();

    // Register demo items with custom stack sizes
    unfreezeItemRegistry();
    Identifier alloyId = Identifier.fromNamespaceAndPath("demo", "alloy_ingot");
    Item alloyItem = Registry.register(
        BuiltInRegistries.ITEM,
        alloyId,
        new Item(new Item.Properties().setId(ResourceKey.create(Registries.ITEM, alloyId))));
    Identifier smallId = Identifier.fromNamespaceAndPath("demo", "small_stack_item");
    Item smallItem = Registry.register(
        BuiltInRegistries.ITEM,
        smallId,
        new Item(new Item.Properties().stacksTo(16).setId(ResourceKey.create(Registries.ITEM, smallId))));
    BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(VanillaRegistries.createWorldLookup()).forEach(p -> p.apply());

    // Bind vanilla stone_tool_materials + demo:reagents tag
    TagKey<Item> stoneMat = TagKey.create(Registries.ITEM, Identifier.parse("minecraft:stone_tool_materials"));
    TagKey<Item> demoReagents = TagKey.create(Registries.ITEM, Identifier.fromNamespaceAndPath("demo", "reagents"));
    List<Holder<Item>> stoneList = List.of(
        Items.COBBLESTONE.builtInRegistryHolder(),
        Items.BLACKSTONE.builtInRegistryHolder(),
        Items.COBBLED_DEEPSLATE.builtInRegistryHolder());
    Map<TagKey<Item>, List<Holder<Item>>> tags = new LinkedHashMap<>();
    tags.put(stoneMat, stoneList);
    tags.put(demoReagents, stoneList);
    refreezeFlag(true);
    BuiltInRegistries.ITEM.prepareTagReload(new TagLoader.LoadResult<>(Registries.ITEM, tags)).apply();

    RegistryOps<JsonElement> ops = RegistryOps.create(
        JsonOps.INSTANCE, HolderLookup.Provider.create(java.util.stream.Stream.of(BuiltInRegistries.ITEM)));

    // Decode 4 valid recipes
    Map<String, String> validJsons = new LinkedHashMap<>();
    validJsons.put("demo:iron_alloy", """
        {"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:redstone","result":{"id":"demo:alloy_ingot","count":4},"tier":2,"label":"basic"}""");
    validJsons.put("demo:precious_alloy", """
        {"type":"demo:alloy","show_notification":false,"base":["minecraft:gold_ingot","minecraft:copper_ingot"],"catalyst":"minecraft:amethyst_shard","result":"demo:alloy_ingot"}""");
    validJsons.put("demo:stone_alloy", """
        {"type":"demo:alloy","base":"#demo:reagents","catalyst":"minecraft:coal","result":{"id":"minecraft:stick","count":2,"components":{"minecraft:repair_cost":7}}}""");
    validJsons.put("demo:small_stack", """
        {"type":"demo:alloy","base":"minecraft:diamond","catalyst":"minecraft:gold_nugget","result":{"id":"demo:small_stack_item","count":16}}""");

    Map<String, AlloyRecipe> recipes = new LinkedHashMap<>();
    for (var e : validJsons.entrySet()) {
      DataResult<AlloyRecipe> res = AlloyRecipe.MAP_CODEC.codec().parse(ops, JsonParser.parseString(e.getValue()));
      AlloyRecipe rec = res.getOrThrow();
      recipes.put(e.getKey(), rec);
      out.println("decoded:" + e.getKey()
          + "|notify=" + rec.commonInfo().showNotification()
          + "|tier=" + rec.tier()
          + "|label=" + rec.label().orElse("none")
          + "|count=" + rec.result().count());
    }

    // Invalid recipes fail decode cleanly
    String[] invalidJsons = {
        """
        {"type":"demo:alloy","base":"minecraft:air","catalyst":"minecraft:coal","result":"demo:alloy_ingot"}""",
        """
        {"type":"demo:alloy","base":[],"catalyst":"minecraft:coal","result":"demo:alloy_ingot"}""",
        """
        {"type":"demo:alloy","base":"#demo:no_such_tag","catalyst":"minecraft:coal","result":"demo:alloy_ingot"}""",
        """
        {"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:coal","result":{"id":"demo:alloy_ingot","count":100}}""",
        """
        {"type":"demo:alloy","base":"minecraft:iron_ingot","catalyst":"minecraft:coal","result":{"id":"minecraft:air","count":1}}"""
    };
    int invalidErrors = 0;
    for (String bad : invalidJsons) {
      if (AlloyRecipe.MAP_CODEC.codec().parse(ops, JsonParser.parseString(bad)).isError()) {
        invalidErrors++;
      }
    }
    out.println("invalidErrorCount=" + invalidErrors);

    // Matching + assemble tests
    record Pair(String name, ItemStack a, ItemStack b) {}
    ItemStack taggedBase = new ItemStack(Items.COBBLESTONE, 3);
    taggedBase.set(DataComponents.REPAIR_COST, 5);
    taggedBase.set(DataComponents.CUSTOM_NAME, Component.literal("Ancient"));

    ItemStack ironWithComp = new ItemStack(Items.IRON_INGOT, 1);
    ironWithComp.set(DataComponents.REPAIR_COST, 9);

    List<Pair> queries = List.of(
        new Pair("iron+redstone", ironWithComp, new ItemStack(Items.REDSTONE)),
        new Pair("gold+amethyst", new ItemStack(Items.GOLD_INGOT), new ItemStack(Items.AMETHYST_SHARD)),
        new Pair("copper+amethyst", new ItemStack(Items.COPPER_INGOT), new ItemStack(Items.AMETHYST_SHARD)),
        new Pair("cobble+coal", taggedBase, new ItemStack(Items.COAL)),
        new Pair("blackstone+coal", new ItemStack(Items.BLACKSTONE), new ItemStack(Items.COAL)),
        new Pair("deepslate+coal", new ItemStack(Items.COBBLED_DEEPSLATE), new ItemStack(Items.COAL)),
        new Pair("diamond+nugget", new ItemStack(Items.DIAMOND), new ItemStack(Items.GOLD_NUGGET)),
        new Pair("dirt+coal", new ItemStack(Items.DIRT), new ItemStack(Items.COAL)),
        new Pair("empty+empty", ItemStack.EMPTY, ItemStack.EMPTY)
    );
    for (Pair q : queries) {
      String foundId = "empty";
      ItemStack resultStack = ItemStack.EMPTY;
      for (var e : recipes.entrySet()) {
        if (e.getValue().matches(q.a(), q.b())) {
          foundId = e.getKey();
          resultStack = e.getValue().assemble(q.a());
          break;
        }
      }
      out.println("query:" + q.name()
          + "|id=" + foundId
          + "|out=" + resultStack.toString()
          + "|empty=" + resultStack.isEmpty()
          + "|repairCost=" + resultStack.get(DataComponents.REPAIR_COST)
          + "|hasCustomName=" + (resultStack.get(DataComponents.CUSTOM_NAME) != null));
    }

    // TransmuteRecipe extraCount + maxStackSize validation
    ItemStackTemplate pearl20 = new ItemStackTemplate(Items.ENDER_PEARL, 20);
    ItemStack pearlOut = TransmuteRecipe.createWithOriginalComponents(pearl20, new ItemStack(Items.STONE));
    out.println("pearl20=" + pearlOut.toString() + "|empty=" + pearlOut.isEmpty());

    ItemStackTemplate small20 = new ItemStackTemplate(smallItem, 20);
    ItemStack smallOut20 = TransmuteRecipe.createWithOriginalComponents(small20, new ItemStack(Items.STONE));
    out.println("small20=" + smallOut20.toString() + "|empty=" + smallOut20.isEmpty());

    ItemStackTemplate stick2 = new ItemStackTemplate(Items.STICK, 2);
    ItemStack stickExtra = TransmuteRecipe.createWithOriginalComponents(stick2, ironWithComp, 3);
    out.println("stickExtra=" + stickExtra.toString() + "|repairCost=" + stickExtra.get(DataComponents.REPAIR_COST));

    // CommonInfo & RecipeHolder & DataComponentPatch & ItemStack.EMPTY
    Recipe.CommonInfo ciTrue = new Recipe.CommonInfo(true);
    Recipe.CommonInfo ciFalse = new Recipe.CommonInfo(false);
    out.println("ci=" + ciTrue + "|" + ciFalse
        + "|eq=" + ciTrue.equals(new Recipe.CommonInfo(true)) + "|" + ciTrue.equals(ciFalse)
        + "|hash=" + ciTrue.hashCode() + "|" + ciFalse.hashCode());
    out.println("emptyStack=" + ItemStack.EMPTY.toString()
        + "|isAir=" + ItemStack.EMPTY.is(Items.AIR)
        + "|isStick=" + ItemStack.EMPTY.is(Items.STICK)
        + "|patchEmpty=" + ItemStack.EMPTY.getComponentsPatch().isEmpty()
        + "|patchStr=" + ItemStack.EMPTY.getComponentsPatch().toString());
    out.println("optIngr="
        + Ingredient.testOptionalIngredient(Optional.empty(), ItemStack.EMPTY) + "|"
        + Ingredient.testOptionalIngredient(Optional.empty(), new ItemStack(Items.STONE)) + "|"
        + Ingredient.testOptionalIngredient(Optional.of(Ingredient.of(Items.STONE)), new ItemStack(Items.STONE)));

    // SimpleContainer clamping
    SimpleContainer box = new SimpleContainer(2);
    ItemStack pearls = new ItemStack(Items.ENDER_PEARL, 30);
    box.setItem(0, pearls);
    out.println("boxClamp=" + box.getItem(0).toString() + "|boxEmpty=" + box.isEmpty());

    // Exact validation exceptions
    capture("ingrAir", () -> Ingredient.of(Items.AIR));
    capture("tplZero", () -> new ItemStackTemplate(Items.STONE, 0));
    capture("tplAir", () -> new ItemStackTemplate(Items.AIR, 1));
    capture("fromEmpty", () -> ItemStackTemplate.fromNonEmptyStack(ItemStack.EMPTY));

    // RecipeHolder identity/toString
    ResourceKey<Recipe<?>> rk1 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:one"));
    ResourceKey<Recipe<?>> rk2 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:one"));
    ResourceKey<Recipe<?>> rk3 = ResourceKey.create(Registries.RECIPE, Identifier.parse("demo:two"));
    RecipeHolder<?> h1 = new RecipeHolder<>(rk1, null);
    RecipeHolder<?> h2 = new RecipeHolder<>(rk2, null);
    RecipeHolder<?> h3 = new RecipeHolder<>(rk3, null);
    out.println("holder=" + h1.toString() + "|eqSame=" + h1.equals(h2) + "|eqDiff=" + h1.equals(h3));
  }
}

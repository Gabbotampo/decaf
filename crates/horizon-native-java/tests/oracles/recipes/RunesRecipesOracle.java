import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.JsonOps;
import java.io.InputStream;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import net.minecraft.SharedConstants;
import net.minecraft.core.Holder;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.Registry;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.RegistryOps;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.tags.TagKey;
import net.minecraft.tags.TagLoader;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.runes.api.RuneItems;
import net.runes.crafting.RuneCraftingRecipe;
import net.runes.crafting.RuneCraftingRecipeInput;

public final class RunesRecipesOracle {
  static void setFrozen(boolean value) throws Exception {
    Class<?> mapped = BuiltInRegistries.ITEM.getClass().getSuperclass();
    Field frozen = mapped.getDeclaredField("frozen");
    frozen.setAccessible(true);
    frozen.setBoolean(BuiltInRegistries.ITEM, value);
    Field intrusive = mapped.getDeclaredField("unregisteredIntrusiveHolders");
    intrusive.setAccessible(true);
    intrusive.set(BuiltInRegistries.ITEM, value ? null : new IdentityHashMap<>());
  }

  public static void main(String[] args) throws Exception {
    java.io.PrintStream out = System.out;
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    HolderLookup.Provider lookup = VanillaRegistries.createWorldLookup();
    BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(lookup).forEach(p -> p.apply());

    // Register Runes items via Runes' own RuneItems.entries
    setFrozen(false);
    for (RuneItems.Entry entry : RuneItems.entries) {
      Registry.register(BuiltInRegistries.ITEM, entry.id(), entry.item());
    }
    BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(VanillaRegistries.createWorldLookup()).forEach(p -> p.apply());

    String jarPath = args[0];
    Map<String, List<String>> rawTags = new LinkedHashMap<>();
    rawTags.put("minecraft:stone_tool_materials",
        List.of("minecraft:cobblestone", "minecraft:blackstone", "minecraft:cobbled_deepslate"));
    Map<String, String> altarRecipes = new TreeMap<>();

    try (JarFile jf = new JarFile(jarPath)) {
      Enumeration<JarEntry> entries = jf.entries();
      while (entries.hasMoreElements()) {
        JarEntry je = entries.nextElement();
        String name = je.getName();
        if (name.startsWith("data/runes/tags/item/") && name.endsWith(".json")) {
          String tagPath = name.substring("data/runes/tags/item/".length(), name.length() - 5);
          try (InputStream is = jf.getInputStream(je)) {
            var obj = JsonParser.parseString(new String(is.readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject();
            List<String> vals = new ArrayList<>();
            for (var el : obj.getAsJsonArray("values")) {
              if (el.isJsonPrimitive()) {
                vals.add(el.getAsString());
              } else if (el.isJsonObject()) {
                var o = el.getAsJsonObject();
                boolean req = !o.has("required") || o.get("required").getAsBoolean();
                if (req) vals.add(o.get("id").getAsString());
              }
            }
            rawTags.put("runes:" + tagPath, vals);
          }
        } else if (name.startsWith("data/runes/recipe/") && name.endsWith(".json")) {
          String recPath = name.substring("data/runes/recipe/".length(), name.length() - 5);
          try (InputStream is = jf.getInputStream(je)) {
            String raw = new String(is.readAllBytes(), StandardCharsets.UTF_8);
            var parsed = JsonParser.parseString(raw).getAsJsonObject();
            if (parsed.has("type") && "runes:crafting".equals(parsed.get("type").getAsString())) {
              altarRecipes.put("runes:" + recPath, raw);
            }
          }
        }
      }
    }

    Map<TagKey<Item>, List<Holder<Item>>> resolvedTags = new LinkedHashMap<>();
    for (String tagId : rawTags.keySet()) {
      List<Holder<Item>> holders = new ArrayList<>();
      Deque<String> queue = new ArrayDeque<>(rawTags.get(tagId));
      while (!queue.isEmpty()) {
        String s = queue.poll();
        if (s.startsWith("#")) {
          List<String> sub = rawTags.get(s.substring(1));
          if (sub != null) queue.addAll(sub);
        } else {
          Optional<Holder.Reference<Item>> h = BuiltInRegistries.ITEM.get(Identifier.parse(s));
          h.ifPresent(holders::add);
        }
      }
      resolvedTags.put(TagKey.create(Registries.ITEM, Identifier.parse(tagId)), holders);
    }
    setFrozen(true);
    BuiltInRegistries.ITEM.prepareTagReload(new TagLoader.LoadResult<>(Registries.ITEM, resolvedTags)).apply();

    RegistryOps<JsonElement> ops = RegistryOps.create(
        JsonOps.INSTANCE, HolderLookup.Provider.create(java.util.stream.Stream.of(BuiltInRegistries.ITEM)));

    Map<String, RuneCraftingRecipe> decoded = new TreeMap<>();
    for (var e : altarRecipes.entrySet()) {
      DataResult<RuneCraftingRecipe> res = RuneCraftingRecipe.MAP_CODEC.codec().parse(ops, JsonParser.parseString(e.getValue()));
      RuneCraftingRecipe rec = res.getOrThrow();
      decoded.put(e.getKey(), rec);
      out.println("recipe=" + e.getKey()
          + "|notify=" + rec.showNotification()
          + "|result=" + rec.result().item().getRegisteredName()
          + "|count=" + rec.result().count());
    }
    out.println("decodedCount=" + decoded.size());

    record Case(String label, ItemStack base, ItemStack addition) {}
    ItemStack cobbleWithRepair = new ItemStack(Items.COBBLESTONE, 1);
    cobbleWithRepair.set(DataComponents.REPAIR_COST, 9);

    List<Case> cases = List.of(
        new Case("cobble+amethyst_shard", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.AMETHYST_SHARD)),
        new Case("cobble+ender_pearl", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.ENDER_PEARL)),
        new Case("blackstone+blaze_powder", new ItemStack(Items.BLACKSTONE), new ItemStack(Items.BLAZE_POWDER)),
        new Case("cobble+coal", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.COAL)),
        new Case("deepslate+snowball", new ItemStack(Items.COBBLED_DEEPSLATE), new ItemStack(Items.SNOWBALL)),
        new Case("cobble+ice", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.ICE)),
        new Case("cobble+gold_nugget", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.GOLD_NUGGET)),
        new Case("cobble+honeycomb", new ItemStack(Items.COBBLESTONE), new ItemStack(Items.HONEYCOMB)),
        new Case("cobble_repair9+amethyst_shard", cobbleWithRepair, new ItemStack(Items.AMETHYST_SHARD)),
        new Case("dirt+amethyst_shard", new ItemStack(Items.DIRT), new ItemStack(Items.AMETHYST_SHARD)),
        new Case("empty+empty", ItemStack.EMPTY, ItemStack.EMPTY)
    );

    for (Case c : cases) {
      RuneCraftingRecipeInput input = new RuneCraftingRecipeInput(c.base(), c.addition());
      String found = "empty";
      ItemStack resultStack = ItemStack.EMPTY;
      for (var e : decoded.entrySet()) {
        if (e.getValue().matches(input, null)) {
          found = e.getKey();
          resultStack = e.getValue().assemble(input);
          break;
        }
      }
      out.println("case:" + c.label()
          + "|recipe=" + found
          + "|out=" + resultStack.toString()
          + "|empty=" + resultStack.isEmpty()
          + "|repairCost=" + resultStack.get(DataComponents.REPAIR_COST));
    }
  }
}

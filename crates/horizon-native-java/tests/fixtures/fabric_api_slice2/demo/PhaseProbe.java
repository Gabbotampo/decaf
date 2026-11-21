package demo;

import com.mojang.serialization.MapCodec;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.Event;
import net.fabricmc.fabric.api.event.EventFactory;
import net.fabricmc.fabric.api.message.v1.ServerMessageDecoratorEvent;
import net.fabricmc.fabric.api.recipe.v1.sync.RecipeSynchronization;
import net.fabricmc.fabric.impl.recipe.sync.RecipeSyncImpl;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.crafting.RecipeSerializer;

public final class PhaseProbe implements ModInitializer {
  interface Listener {
    void call();
  }

  public static String RESULT;

  private static Identifier id(String path) {
    return Identifier.fromNamespaceAndPath("demo", path);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static String run() {
    Event<Listener> event = EventFactory.createArrayBacked(Listener.class, listeners -> () -> {});
    StringBuilder result = new StringBuilder();
    result
        .append(ServerMessageDecoratorEvent.CONTENT_PHASE)
        .append("|")
        .append(ServerMessageDecoratorEvent.STYLING_PHASE)
        .append("|")
        .append(RecipeSynchronization.RECIPE_SYNC_EVENT_PHASE)
        .append(";");
    try {
      event.register((Listener) null);
    } catch (NullPointerException e) {
      result.append(e.getMessage()).append("|");
    }
    try {
      event.register(null, () -> {});
    } catch (NullPointerException e) {
      result.append(e.getMessage()).append("|");
    }
    try {
      event.addPhaseOrdering(null, id("x"));
    } catch (NullPointerException e) {
      result.append(e.getMessage()).append("|");
    }
    Identifier self = id("self");
    try {
      event.addPhaseOrdering(self, self);
    } catch (IllegalArgumentException e) {
      result.append(e.getMessage()).append("|");
    }
    Identifier a = id("a");
    Identifier b = id("b");
    event.addPhaseOrdering(a, b);
    event.addPhaseOrdering(b, a);

    MapCodec mapCodec = MapCodec.unit("demo");
    StreamCodec streamCodec = StreamCodec.unit("demo");
    RecipeSerializer s1 = new RecipeSerializer(mapCodec, streamCodec);
    RecipeSerializer s2 = new RecipeSerializer(mapCodec, streamCodec);
    RecipeSerializer nullCodec = new RecipeSerializer(mapCodec, null);
    RecipeSerializer bothNull = new RecipeSerializer(null, null);

    result
        .append("codecSame=")
        .append(s1.codec() == mapCodec)
        .append("|streamSame=")
        .append(s1.streamCodec() == streamCodec)
        .append("|nullStream=")
        .append(nullCodec.streamCodec() == null)
        .append("|recEq=")
        .append(s1.equals(s2))
        .append("|sameRef=")
        .append(s1 == s2)
        .append("|neq=")
        .append(s1.equals(nullCodec))
        .append("|isRec=")
        .append(((Object) s1) instanceof Record)
        .append("|nullsStr=")
        .append(bothNull.toString())
        .append("|nullsHash=")
        .append(bothNull.hashCode());

    RecipeSynchronization.synchronizeRecipeSerializer(s1);
    RecipeSynchronization.synchronizeRecipeSerializer(s1);
    RecipeSynchronization.synchronizeRecipeSerializer(s2);
    try {
      RecipeSynchronization.synchronizeRecipeSerializer(null);
    } catch (NullPointerException e) {
      result.append("|").append(e.getMessage());
    }
    try {
      RecipeSynchronization.synchronizeRecipeSerializer(nullCodec);
    } catch (NullPointerException e) {
      result.append("|").append(e.getMessage());
    }
    result.append("|cycle-ok");
    return result.toString();
  }

  @Override
  public void onInitialize() {
    RESULT = run();
  }

  public static void main(String[] args) throws java.io.IOException {
    String out =
        run()
            + "\nsynced="
            + RecipeSyncImpl.getSyncedSerializers().size()
            + "\n";
    if (args.length > 0) {
      java.nio.file.Files.writeString(java.nio.file.Path.of(args[0]), out);
    } else {
      System.out.print(out);
    }
  }
}

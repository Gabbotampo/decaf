package demo;

import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;

/** Registers a bench block whose use opens a two-input combiner menu. */
public final class Probe implements ModInitializer {
  public static Block BENCH;

  @Override
  public void onInitialize() {
    Registry.register(
        BuiltInRegistries.MENU, Identifier.fromNamespaceAndPath("demo", "bench"), BenchMenu.TYPE);
    BENCH = new Bench(BlockBehaviour.Properties.of());
    Registry.register(BuiltInRegistries.BLOCK, Identifier.fromNamespaceAndPath("demo", "bench"), BENCH);
  }
}

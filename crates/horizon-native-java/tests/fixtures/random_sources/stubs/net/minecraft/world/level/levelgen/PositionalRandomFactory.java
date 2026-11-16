package net.minecraft.world.level.levelgen;
import net.minecraft.util.RandomSource;
public interface PositionalRandomFactory {
  RandomSource fromSeed(long seed);
  RandomSource at(int x, int y, int z);
  RandomSource fromHashOf(String name);
}

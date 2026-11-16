package net.minecraft.util;
import net.minecraft.world.level.levelgen.PositionalRandomFactory;
public interface RandomSource {
  static RandomSource create() { return null; }
  static RandomSource createThreadSafe() { return null; }
  static RandomSource create(long seed) { return null; }
  static RandomSource createThreadLocalInstance() { return null; }
  static RandomSource createThreadLocalInstance(long seed) { return null; }
  RandomSource fork();
  PositionalRandomFactory forkPositional();
  void setSeed(long seed);
  int nextInt();
  int nextInt(int bound);
  long nextLong();
  boolean nextBoolean();
  float nextFloat();
  double nextDouble();
  double nextGaussian();
  default int nextIntBetweenInclusive(int min, int max) { return min; }
  default int nextInt(int origin, int bound) { return origin; }
  default void consumeCount(int count) {}
}

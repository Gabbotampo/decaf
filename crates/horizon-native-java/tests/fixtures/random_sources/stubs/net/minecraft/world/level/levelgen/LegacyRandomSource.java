package net.minecraft.world.level.levelgen;
import net.minecraft.util.RandomSource;
public class LegacyRandomSource implements RandomSource {
  public LegacyRandomSource(long seed) {}
  public RandomSource fork(){return null;} public PositionalRandomFactory forkPositional(){return null;}
  public void setSeed(long s){} public int nextInt(){return 0;} public int nextInt(int b){return 0;}
  public long nextLong(){return 0;} public boolean nextBoolean(){return false;} public float nextFloat(){return 0;}
  public double nextDouble(){return 0;} public double nextGaussian(){return 0;}
}

package demo;

import net.minecraft.util.RandomSource;
import net.minecraft.world.level.levelgen.LegacyRandomSource;
import net.minecraft.world.level.levelgen.SingleThreadedRandomSource;
import net.minecraft.world.level.levelgen.XoroshiroRandomSource;
import net.minecraft.world.level.levelgen.PositionalRandomFactory;

public final class RandomSources {
  private static long mix(long h, long value) {
    return (h * 0x9e3779b97f4a7c15L) ^ value;
  }

  private static String ints(RandomSource r) {
    long h = 0x123456789abcdef0L;
    for (int i = 0; i < 100003; i++) h = mix(h, r.nextInt());
    return Long.toString(h);
  }

  private static String bounds(RandomSource r) {
    long h = 0x223456789abcdef0L;
    int[] bounds = {1, 2, 3, 7, 16, 1024, Integer.MAX_VALUE};
    for (int i = 0; i < 100003; i++)
      for (int bound : bounds) h = mix(h, r.nextInt(bound));
    return Long.toString(h);
  }

  private static String longs(RandomSource r) {
    long h = 0x323456789abcdef0L;
    for (int i = 0; i < 100003; i++) h = mix(h, r.nextLong());
    return Long.toString(h);
  }

  private static String floats(RandomSource r) {
    long h = 0x423456789abcdef0L;
    for (int i = 0; i < 100003; i++) h = mix(h, Float.floatToRawIntBits(r.nextFloat()));
    return Long.toString(h);
  }

  private static String doubles(RandomSource r) {
    long h = 0x523456789abcdef0L;
    for (int i = 0; i < 100003; i++) h = mix(h, Double.doubleToRawLongBits(r.nextDouble()));
    return Long.toString(h);
  }

  private static String booleans(RandomSource r) {
    long h = 0x623456789abcdef0L;
    for (int i = 0; i < 100003; i++) h = mix(h, r.nextBoolean() ? 1 : 0);
    return Long.toString(h);
  }

  private static String gaussians(RandomSource r) {
    for (int i = 0; i < 100003; i++) r.nextGaussian();
    return "done";
  }

  private static String consume() {
    RandomSource r = RandomSource.create(123456789L);
    r.consumeCount(100003);
    return ints(r);
  }

  private static String reseed() {
    RandomSource r = RandomSource.create(1L);
    r.nextLong();
    r.setSeed(123456789L);
    return ints(r);
  }

  private static String forks() {
    RandomSource r = RandomSource.create(123456789L);
    return ints(r.fork()) + "|" + ints(r.forkPositional().fromSeed(987654321L));
  }

  private static String positional() {
    PositionalRandomFactory legacy = RandomSource.create(123456789L).forkPositional();
    PositionalRandomFactory xoro = new XoroshiroRandomSource(123456789L).forkPositional();
    return ints(legacy.at(1, 2, 3)) + "|" + ints(xoro.at(1, 2, 3))
        + "|" + ints(legacy.fromSeed(987654321L));
  }

  public static String all() {
    return ints(RandomSource.create(123456789L))
        + "|" + ints(new SingleThreadedRandomSource(123456789L))
        + "|" + ints(new XoroshiroRandomSource(123456789L))
        + "|" + bounds(RandomSource.create(123456789L))
        + "|" + longs(RandomSource.create(123456789L))
        + "|" + floats(RandomSource.create(123456789L))
        + "|" + doubles(RandomSource.create(123456789L))
        + "|" + booleans(RandomSource.create(123456789L))
        + "|" + gaussians(RandomSource.create(123456789L))
        + "|" + consume() + "|" + reseed() + "|" + forks() + "|" + positional();
  }

  public static String bits() { RandomSource r = RandomSource.create(123456789L); return Float.floatToRawIntBits(r.nextFloat()) + "|" + Long.toHexString(Double.doubleToRawLongBits(r.nextDouble())); }

  public static String quick() { RandomSource r = RandomSource.create(1L); return r == null ? "null" : r.getClass().getName() + "|" + r.nextInt(); }

  public static String countedHash() {
    PositionalRandomFactory f = RandomSource.create(123456789L).forkPositional();
    RandomSource r = f.fromHashOf("minecraft:ore");
    return r == null ? "null" : r.getClass().getName();
  }

  public static String errors() {
    StringBuilder out = new StringBuilder();
    for (int bound : new int[] {0, -1, Integer.MIN_VALUE}) {
      try { RandomSource.create(1L).nextInt(bound); }
      catch (Throwable x) { out.append(x.getClass().getName()).append("|").append(x.getMessage()).append(";"); }
    }
    try { RandomSource.create(1L).nextIntBetweenInclusive(5, 4); }
    catch (Throwable x) { out.append(x.getClass().getName()).append("|").append(x.getMessage()); }
    return out.toString();
  }
}

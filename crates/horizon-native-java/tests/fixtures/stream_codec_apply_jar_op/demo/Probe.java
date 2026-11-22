package demo;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;

public final class Probe {
  static StreamCodec<?, ?> base() {
    return StreamCodec.unit("x");
  }

  /** The operation runs once, receives the receiver itself, and its result is returned. */
  public static String identity() {
    StreamCodec base = base();
    int[] calls = {0};
    Object[] received = {null};
    StreamCodec out =
        base.apply(
            c -> {
              calls[0]++;
              received[0] = c;
              return c;
            });
    return "same=" + (out == base) + ",calls=" + calls[0] + ",received=" + (received[0] == base);
  }

  /** The operation's return value is returned unchanged. */
  public static String resultUnchanged() {
    StreamCodec base = base();
    StreamCodec other = StreamCodec.unit("y");
    StreamCodec out = base.apply(c -> other);
    return "outSame=" + (out == other) + ",notReceiver=" + (out != base);
  }

  /** null is returned unchanged (no NPE from the runtime itself). */
  public static String resultNull() {
    StreamCodec base = base();
    StreamCodec out = base.apply(c -> null);
    return "outNull=" + (out == null);
  }

  /** An exception thrown by the operation propagates (class and message). */
  public static String throws_() {
    StreamCodec base = base();
    try {
      base.apply(
          c -> {
            throw new IllegalStateException("boom");
          });
      return "no throw";
    } catch (Throwable t) {
      return t.getClass().getName() + ":" + t.getMessage();
    }
  }

  /** apply(null) throws an NPE with the game's helpful message. */
  public static String nullOp() {
    StreamCodec base = base();
    try {
      @SuppressWarnings("unused")
      StreamCodec out = base.apply(null);
      return "no throw";
    } catch (Throwable t) {
      return t.getClass().getName() + ":" + t.getMessage();
    }
  }

  /** Geckolib shape: a codec.apply(method reference). */
  public static String geckolibShape() {
    StreamCodec base = base();
    StreamCodec optional = base.apply(Probe::optionalLike);
    return "opIsCodec=" + (optional instanceof StreamCodec);
  }

  static StreamCodec<?, ?> optionalLike(StreamCodec c) {
    return c;
  }

  public static void main(String[] x) {
    System.out.print("identity=");
    System.out.println(identity());
    System.out.print("resultUnchanged=");
    System.out.println(resultUnchanged());
    System.out.print("resultNull=");
    System.out.println(resultNull());
    System.out.print("throws=");
    System.out.println(throws_());
    System.out.print("nullOp=");
    System.out.println(nullOp());
    System.out.print("geckolibShape=");
    System.out.println(geckolibShape());
  }
}

package demo;
import java.util.ArrayList;
import java.util.function.BiFunction;
import java.util.function.Function;
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


  static final class Boom extends RuntimeException {
    Boom(String m) {
      super(m);
    }
  }

  static final class Keeper {
    StreamCodec seen;
    int calls;

    StreamCodec keep(StreamCodec c) {
      calls++;
      seen = c;
      return c;
    }
  }

  /** A jar class that is itself a StreamCodec, built by a constructor reference. */
  static final class Wrap implements StreamCodec {
    final StreamCodec inner;

    Wrap(StreamCodec inner) {
      this.inner = inner;
    }

    public void encode(Object b, Object v) {}

    public Object decode(Object b) {
      return null;
    }
  }

  /** A lambda that captures an int and an array: captures come before the receiver. */
  public static String captureInt() {
    StreamCodec base = base();
    int k = 7;
    int[] seen = {0};
    StreamCodec out =
        base.apply(
            c -> {
              seen[0] = k;
              return c;
            });
    return "k=" + seen[0] + ",same=" + (out == base);
  }

  /** The operation ignores its argument and returns a captured codec (here the receiver). */
  public static String captureSelf() {
    StreamCodec base = base();
    StreamCodec out = base.apply(c -> base);
    return "same=" + (out == base);
  }

  /** A bound method reference: the captured object is the receiver of the call. */
  public static String boundRef() {
    StreamCodec base = base();
    Keeper k = new Keeper();
    StreamCodec out = base.apply(k::keep);
    return "calls=" + k.calls + ",seen=" + (k.seen == base) + ",out=" + (out == base);
  }

  /** A constructor reference: the receiver is the constructor's argument. */
  public static String constructorRef() {
    StreamCodec base = base();
    StreamCodec out = base.apply(Wrap::new);
    return "wrap="
        + (out instanceof Wrap)
        + ",inner="
        + (out instanceof Wrap w && w.inner == base)
        + ",notBase="
        + (out != base);
  }

  /** A static reference to a game factory the runtime models: StreamCodec.unit(receiver). */
  public static String staticRefUnit() {
    StreamCodec base = base();
    StreamCodec out = base.apply(StreamCodec::unit);
    return "decoded=" + (out.decode(null) == base) + ",notBase=" + (out != base);
  }

  /** A static reference to ByteBufCodecs.optional on the game's own STRING_UTF8 codec. */
  public static String staticRefOptional() {
    StreamCodec s = ByteBufCodecs.STRING_UTF8;
    StreamCodec out = s.apply(ByteBufCodecs::optional);
    return "codec=" + (out instanceof StreamCodec) + ",notReceiver=" + (out != s);
  }

  /** An exception thrown by the operation is a normal exception: a catch and a finally see it. */
  public static String caughtAndFinally() {
    StreamCodec base = base();
    StringBuilder log = new StringBuilder();
    try {
      try {
        base.apply(
            c -> {
              log.append("in;");
              throw new Boom("b1");
            });
        log.append("after;");
      } finally {
        log.append("finally;");
      }
    } catch (RuntimeException e) {
      log.append(e.getClass().getName()).append(':').append(e.getMessage()).append(';');
    }
    StreamCodec again = base.apply(c -> c);
    log.append("again=").append(again == base);
    return log.toString();
  }

  /** The operation runs apply again on its own argument. */
  public static String nested() {
    StreamCodec base = base();
    StringBuilder log = new StringBuilder();
    StreamCodec out =
        base.apply(
            c -> {
              log.append("outer-in;");
              StreamCodec r =
                  c.apply(
                      d -> {
                        log.append("inner;");
                        return d;
                      });
              log.append("outer-out;");
              return r;
            });
    return log + "same=" + (out == base);
  }

  /** A modelled operation (ByteBufCodecs.list) applied inside an operation. */
  public static String nestedModelled() {
    StreamCodec base = base();
    int[] calls = {0};
    StreamCodec out =
        base.apply(
            c -> {
              calls[0]++;
              return c.apply(ByteBufCodecs.list());
            });
    return "calls=" + calls[0] + ",codec=" + (out instanceof StreamCodec) + ",notBase=" + (out != base);
  }

  /** apply on the result of apply. */
  public static String chained() {
    StreamCodec base = base();
    int[] calls = {0};
    StreamCodec out =
        base.apply(
                c -> {
                  calls[0]++;
                  return c;
                })
            .apply(
                c -> {
                  calls[0]++;
                  return c;
                });
    return "calls=" + calls[0] + ",same=" + (out == base);
  }

  private static String check(String name, StreamCodec codec) {
    int[] calls = {0};
    Object[] seen = {null};
    StreamCodec out =
        codec.apply(
            c -> {
              calls[0]++;
              seen[0] = c;
              return c;
            });
    return name + ":calls=" + calls[0] + ",seen=" + (seen[0] == codec) + ",out=" + (out == codec);
  }

  private static String nullMessage(String name, StreamCodec codec) {
    try {
      codec.apply(null);
      return name + ":no throw";
    } catch (Throwable t) {
      return name + ":" + t.getClass().getName() + ":" + t.getMessage();
    }
  }

  private static StreamCodec composite2() {
    StreamCodec s = ByteBufCodecs.STRING_UTF8;
    Function f = x -> x;
    BiFunction g = (a, b) -> a;
    return StreamCodec.composite(s, f, s, f, g);
  }

  private static StreamCodec of2() {
    return StreamCodec.of((b, v) -> {}, b -> "x");
  }

  private static StreamCodec ofMember2() {
    return StreamCodec.ofMember((v, b) -> {}, b -> "x");
  }

  /** The receiver kinds the runtime builds, and the game's own STRING_UTF8 codec. */
  public static String kinds() {
    StreamCodec s = ByteBufCodecs.STRING_UTF8;
    StreamCodec optional = ByteBufCodecs.optional(s);
    StreamCodec collection = ByteBufCodecs.collection(ArrayList::new, s);
    StreamCodec b = base();
    StreamCodec mapped = b.map(x -> x, x -> x);
    StringBuilder sb = new StringBuilder();
    sb.append(check("unit", base())).append(" | ");
    sb.append(check("of", of2())).append(" | ");
    sb.append(check("ofMember", ofMember2())).append(" | ");
    sb.append(check("composite", composite2())).append(" | ");
    sb.append(check("map", mapped)).append(" | ");
    sb.append(check("stringUtf8", s)).append(" | ");
    sb.append(check("optional", optional)).append(" | ");
    sb.append(check("collection", collection)).append(" | ");
    sb.append(check("cast", b.cast())).append(" | ");
    sb.append(check("applyList", b.apply(ByteBufCodecs.list())));
    return sb.toString();
  }

  /** apply(null) on every receiver kind. */
  public static String nullKinds() {
    StreamCodec s = ByteBufCodecs.STRING_UTF8;
    StringBuilder sb = new StringBuilder();
    sb.append(nullMessage("unit", base())).append(" | ");
    sb.append(nullMessage("of", of2())).append(" | ");
    sb.append(nullMessage("composite", composite2())).append(" | ");
    sb.append(nullMessage("stringUtf8", s)).append(" | ");
    sb.append(nullMessage("optional", ByteBufCodecs.optional(s)));
    return sb.toString();
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
    System.out.print("captureInt=");
    System.out.println(captureInt());
    System.out.print("captureSelf=");
    System.out.println(captureSelf());
    System.out.print("boundRef=");
    System.out.println(boundRef());
    System.out.print("constructorRef=");
    System.out.println(constructorRef());
    System.out.print("staticRefUnit=");
    System.out.println(staticRefUnit());
    System.out.print("staticRefOptional=");
    System.out.println(staticRefOptional());
    System.out.print("caughtAndFinally=");
    System.out.println(caughtAndFinally());
    System.out.print("nested=");
    System.out.println(nested());
    System.out.print("nestedModelled=");
    System.out.println(nestedModelled());
    System.out.print("chained=");
    System.out.println(chained());
    System.out.print("kinds=");
    System.out.println(kinds());
    System.out.print("nullKinds=");
    System.out.println(nullKinds());
  }
}

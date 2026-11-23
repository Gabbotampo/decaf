package demo;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;

/**
 * Operations the runtime cannot answer exactly: `apply` stays counted. No oracle row; the test
 * asserts the call is counted and that the operation ran at most once. This class is compiled
 * against the test's stubs only (`ByteBufCodecs.unmodelled` is not a game method).
 */
public final class Counted {
  static int calls;

  /** The operation calls a game method the runtime does not model (toString on a codec). */
  public static String innerCounted() {
    StreamCodec base = StreamCodec.unit("x");
    calls = 0;
    base.apply(
        c -> {
          calls++;
          c.toString();
          return c;
        });
    return "calls=" + calls;
  }

  /** The operation counts an inner call and then throws. */
  public static String innerCountedThenThrows() {
    StreamCodec base = StreamCodec.unit("x");
    calls = 0;
    try {
      base.apply(
          c -> {
            calls++;
            c.toString();
            throw new IllegalStateException("boom");
          });
    } catch (IllegalStateException e) {
      return "threw,calls=" + calls;
    }
    return "swallowed,calls=" + calls;
  }

  /** A jar class implementing CodecOperation (not a lambda or method reference). */
  static final class Op implements StreamCodec.CodecOperation {
    public StreamCodec apply(StreamCodec c) {
      calls++;
      return c;
    }
  }

  public static String operationObject() {
    StreamCodec base = StreamCodec.unit("x");
    calls = 0;
    base.apply(new Op());
    return "calls=" + calls;
  }

  /** A static reference to a game method the runtime does not model. */
  public static String unmodelledStaticRef() {
    StreamCodec s = ByteBufCodecs.STRING_UTF8;
    s.apply(ByteBufCodecs::unmodelled);
    return "done";
  }

  /** An unbound method reference to a game method: the codec is the call's receiver. */
  public static String unboundGameRef() {
    StreamCodec base = StreamCodec.unit("x");
    StreamCodec out = base.apply(StreamCodec::cast);
    return "done";
  }

  static StreamCodec recurse(StreamCodec c) {
    return c.apply(Counted::recurse);
  }

  /** An operation that applies itself forever. */
  public static String recursion() {
    try {
      recurse(StreamCodec.unit("x"));
      return "no throw";
    } catch (StackOverflowError e) {
      return "StackOverflowError";
    }
  }

  /** A jar class that is itself a StreamCodec is not a modelled receiver. */
  static final class Mine implements StreamCodec {
    public void encode(Object b, Object v) {}

    public Object decode(Object b) {
      return null;
    }
  }

  public static String jarCodecReceiver() {
    StreamCodec mine = new Mine();
    calls = 0;
    mine.apply(
        c -> {
          calls++;
          return c;
        });
    return "calls=" + calls;
  }
}

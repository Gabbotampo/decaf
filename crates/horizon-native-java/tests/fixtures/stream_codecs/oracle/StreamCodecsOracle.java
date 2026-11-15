import demo.StreamCodecs;

public final class StreamCodecsOracle {
  public static void main(String[] args) {
    System.out.println("constructions=" + StreamCodecs.constructions());
    try {
      System.out.println("encodeDecode=" + StreamCodecs.encodeDecode());
    } catch (Throwable x) {
      System.out.println("encodeDecode=" + x.getClass().getName() + "|" + x.getMessage());
    }
  }
}

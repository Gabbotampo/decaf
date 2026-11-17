package demo;
public final class Probe {
  public static String rows() {
    double sub = Double.longBitsToDouble(1L);
    return String.join("|", Double.toHexString(Math.toRadians(0.0)), Double.toHexString(Math.toRadians(-0.0)), Double.toHexString(Math.toRadians(180.0)), Double.toHexString(Math.toRadians(1e308)), Double.toHexString(Math.toRadians(Double.NaN)), Double.toHexString(Math.toRadians(Double.POSITIVE_INFINITY)), Double.toHexString(Math.toRadians(Double.NEGATIVE_INFINITY)), Double.toHexString(Math.toRadians(sub)));
  }
  public static void main(String[] args) { System.out.println(rows()); }
}

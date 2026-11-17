package demo;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
public final class Probe {
  private static final String[] INPUTS = {"a b", "*-._", "~!'()", "é", "😀", "\uD800", "", "a+b=c&d"};
  public static String rows() {
    String[] out = new String[INPUTS.length];
    for (int i = 0; i < INPUTS.length; i++) out[i] = URLEncoder.encode(INPUTS[i], StandardCharsets.UTF_8);
    return String.join("|", out);
  }
  public static void main(String[] args) { System.out.print(rows()); }
}

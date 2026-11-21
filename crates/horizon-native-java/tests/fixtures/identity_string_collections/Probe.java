package demo;
import java.util.IdentityHashMap;
public final class Probe {
  public static String rows() {
    String literal = "x";
    String copy1 = new String("x");
    String copy2 = new String("x");
    IdentityHashMap<String,Integer> map = new IdentityHashMap<>();
    map.put(literal, 1); map.put(copy1, 2); map.put(copy2, 3);
    String lookup = new String("x");
    return map.size() + ":" + map.get(copy1) + ":" + map.get(copy2)
        + ":" + map.get(lookup) + ":" + map.size();
  }
  public static String sameReference() {
    String copy = new String("x");
    IdentityHashMap<String,Integer> map = new IdentityHashMap<>();
    map.put(copy, 7);
    return map.get(copy) + ":" + map.size();
  }
  public static String callbackMutation() {
    String a = new String("x");
    String b = new String("x");
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    m.put("z", 1);
    m.put(a, 2);
    Integer n = m.computeIfAbsent(b, k -> {
      m.remove("z");
      return 3;
    });
    return n + ":" + m.get(a) + ":" + m.get(b) + ":" + m.size();
  }
  public static String stringValueOfIdentity() {
    String a = new String("x");
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    m.put(a, 1);
    return String.valueOf(m.get(String.valueOf(a)));
  }
  public static void main(String[] args) {
    System.out.println(rows());
    System.out.println(sameReference());
    System.out.println(callbackMutation());
    System.out.println(stringValueOfIdentity());
  }
}

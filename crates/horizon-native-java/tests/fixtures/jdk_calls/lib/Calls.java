package lib;
import java.util.*;
import java.util.stream.*;
import com.google.common.collect.*;

public final class Calls {
  enum Mode { ZETA, ALPHA, OMEGA }
  static class EnumField { static Mode mode; }
  static final class EnumEntry<E extends Enum<E>> {
    private final java.lang.reflect.Field field;
    EnumEntry(java.lang.reflect.Field field) { this.field = field; }
    protected String getValueString(E value) { return value.name(); }
    Set<String> getAllValues() {
      return Arrays.stream(field.getType().getEnumConstants())
          .map(e -> (E)e)
          .map(this::getValueString)
          .collect(Collectors.toCollection(HashSet::new));
    }
  }
  public static String enumSet() {
    EnumSet<Mode> s = EnumSet.allOf(Mode.class);
    StringBuilder b = new StringBuilder();
    for (Mode m : s) b.append(m.name()).append(',');
    boolean removed = s.remove(Mode.ALPHA);
    return s.size()+"|"+b+"|"+s+"|"+removed+"|"+s.contains(Mode.ZETA)+"|"+s.toArray().length;
  }
  public static String streamArray() {
    String[] a = Stream.of("a", "bb", "c").map(String::toUpperCase).toArray(String[]::new);
    String mismatch;
    try { Stream.of("x").toArray(Integer[]::new); mismatch = "none"; }
    catch (ArrayStoreException e) { mismatch = e.getClass().getName(); }
    return a.getClass().getName()+"|"+a.length+"|"+String.join(",", a)+"|"+mismatch;
  }
  public static String enumValuesStream() {
    try {
      return new EnumEntry<Mode>(EnumField.class.getDeclaredField("mode")).getAllValues().toString();
    } catch (ReflectiveOperationException e) { throw new AssertionError(e); }
  }
  public static String arrayMap() { return view(ArrayListMultimap.create()); }
  public static String hashMap() { return view(HashMultimap.create()); }
  public static String linkedMap() { return view(LinkedHashMultimap.create()); }
  private static String view(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("b", 2);
    Map<String, Collection<Integer>> view = m.asMap();
    String before = view.size()+"/"+view.get("a")+"/"+view.keySet();
    m.put("a", 3); m.put("c", 4);
    String after = view.size()+"/"+view.get("a")+"/"+view.keySet()+"/"+view.containsKey("c");
    return before+"|"+after;
  }
}

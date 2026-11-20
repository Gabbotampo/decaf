package lib;
import java.util.*;
import java.util.stream.*;
import com.google.common.collect.*;

public final class Calls {
  enum Mode { ZETA, ALPHA, OMEGA }
  enum Other { ONLY }
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

  public static String enumContracts() {
    String all = EnumSet.allOf(Mode.class).toString();
    String nullAdd;
    try { EnumSet.allOf(Mode.class).add(null); nullAdd = "none"; }
    catch (NullPointerException e) { nullAdd = "npe"; }
    String wrongAdd;
    try { ((Collection) EnumSet.allOf(Mode.class)).add(Other.ONLY); wrongAdd = "none"; }
    catch (ClassCastException e) { wrongAdd = "cce"; }
    String nonEnum;
    try { EnumSet.allOf((Class) String.class); nonEnum = "none"; }
    catch (ClassCastException e) { nonEnum = e.getMessage(); }
    return all + "|" + nullAdd + "|" + wrongAdd + "|" + nonEnum;
  }

  static final class Bag extends AbstractCollection<Integer> {
    public int size() { return 2; }
    public Iterator<Integer> iterator() { return List.of(4, 5).iterator(); }
    public Stream<Integer> stream() { return Stream.of(9, 8); }
  }

  static final class Adds extends ArrayList<Integer> {
    static String log = "";
    public boolean add(Integer value) { log += "add"; return super.add(value); }
  }

  public static String jarCollectionStream() {
    Collection<Integer> bag = new Bag();
    return bag.stream().map(Object::toString).collect(Collectors.joining(","));
  }

  public static String toCollectionJarOverride() {
    Adds.log = "";
    List<Integer> result = Stream.of(1, 2).collect(Collectors.toCollection(Adds::new));
    return Adds.log + "|" + result;
  }

  private static String view(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("b", 2);
    Map<String, Collection<Integer>> view = m.asMap();
    String before = view.size()+"/"+view.get("a")+"/"+view.keySet();
    m.put("a", 3); m.put("c", 4);
    String after = view.size()+"/"+view.get("a")+"/"+view.keySet()+"/"+view.containsKey("c");
    return before+"|"+after;
  }
}

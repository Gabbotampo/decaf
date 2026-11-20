package demo;
import com.google.common.collect.*;
import java.util.Collection;
import java.util.Map;
public final class Probe {
  private static String run(Multimap<String,Integer> m) {
    boolean p1=m.put("a",1);
    boolean p2=m.put("a",2);
    boolean p3=m.put("b",3);
    int getSize=m.get("a").size();
    boolean key=m.containsKey("a");
    boolean entry=m.containsEntry("a",2);
    boolean removed=m.remove("a",1);
    int afterOne=m.size();
    boolean removedLast=m.remove("b",3);
    return p1+"|"+p2+"|"+p3+"|"+getSize+"|"+key+"|"+entry+"|"+removed+"|"+afterOne+"|"+removedLast+"|"+m.size()+"|"+m.containsKey("b");
  }
  public static String array(){return run(ArrayListMultimap.create());}
  public static String hash(){return run(HashMultimap.create());}
  public static String linked(){return run(LinkedHashMultimap.create());}
  private static String entryBehavior(Multimap<String,Integer> m) {
    m.put("a", 1);
    Map.Entry<String, Collection<Integer>> e = m.asMap().entrySet().iterator().next();
    String set;
    try { e.setValue(java.util.List.of(9)); set = "no"; }
    catch (UnsupportedOperationException x) { set = "uo"; }
    String before = Integer.toString(e.getValue().size());
    m.put("a", 2);
    String grown = e.getValue().size() + "|" + e.getValue().contains(2);
    m.asMap().remove("a");
    String empty = e.getValue().size() + "|" + e.toString();
    return e.getClass().getName() + "|" + e.getKey() + "|" + before + "|" + grown + "|" + empty
        + "|" + set + "|" + e.equals(java.util.Map.entry("a", e.getValue())) + "|"
        + e.toString();
  }
  public static String entryArray() { return entryBehavior(ArrayListMultimap.create()); }
  public static String entryHash() { return entryBehavior(HashMultimap.create()); }
  public static String entryLinked() { return entryBehavior(LinkedHashMultimap.create()); }

  public static String asMapIdentity() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Map<String, Collection<Integer>> map = m.asMap();
    Collection<Integer> first = map.get("a");
    Collection<Integer> second = map.get("a");
    boolean distinct = first != second && first != m.get("a");
    m.put("a", 2);
    boolean live = first.size() == 2 && first.contains(2);
    map.keySet().remove("a");
    boolean keyWrite = m.size() == 0;
    m.put("b", 3);
    boolean keyLive = map.keySet().contains("b");
    map.entrySet().clear();
    return distinct + "|" + live + "|" + keyWrite + "|" + keyLive + "|" + (m.size() == 0);
  }
  private static String removed(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("a", 2); m.put("b", 3);
    Map<String, Collection<Integer>> map = m.asMap();
    Collection<Integer> held = m.get("a");
    Collection<Integer> detached = map.remove("a");
    return (detached != held) + "|" + detached.size() + "|" + held.size() + "|"
        + m.containsKey("a") + "|" + m.size() + "|" + (map.remove("missing") == null)
        + "|" + (map.remove(Integer.valueOf(7)) == null);
  }
  public static String asMapRemoveArray() { return removed(ArrayListMultimap.create()); }
  public static String asMapRemoveHash() { return removed(HashMultimap.create()); }
  public static String asMapRemoveLinked() { return removed(LinkedHashMultimap.create()); }
  private static String iteratorRemove(Multimap<String,Integer> m, int view) {
    m.put("a", 1); m.put("b", 2);
    Collection<Integer> held = m.get("a");
    java.util.Iterator<?> it = view == 0 ? m.asMap().entrySet().iterator()
        : view == 1 ? m.asMap().keySet().iterator() : m.asMap().values().iterator();
    it.next(); it.remove();
    String out = m.size() + ":" + m.containsKey("a") + ":" + held.size() + ":" + it.hasNext();
    try { it.remove(); out += ":no"; } catch (IllegalStateException e) { out += ":ise"; }
    java.util.Iterator<?> before = view == 0 ? m.asMap().entrySet().iterator()
        : view == 1 ? m.asMap().keySet().iterator() : m.asMap().values().iterator();
    try { before.remove(); out += ":no"; } catch (IllegalStateException e) { out += ":ise"; }
    java.util.Iterator<?> cme = view == 0 ? m.asMap().entrySet().iterator()
        : view == 1 ? m.asMap().keySet().iterator() : m.asMap().values().iterator();
    m.put("c", 3);
    try { cme.next(); out += ":no"; } catch (java.util.ConcurrentModificationException e) { out += ":cme"; }
    return out;
  }
  public static String iteratorRemoveArray() {
    return iteratorRemove(ArrayListMultimap.create(), 0) + "|"
        + iteratorRemove(ArrayListMultimap.create(), 1) + "|"
        + iteratorRemove(ArrayListMultimap.create(), 2);
  }
  public static String iteratorRemoveHash() {
    return iteratorRemove(HashMultimap.create(), 0) + "|"
        + iteratorRemove(HashMultimap.create(), 1) + "|"
        + iteratorRemove(HashMultimap.create(), 2);
  }
  public static String iteratorRemoveLinked() {
    return iteratorRemove(LinkedHashMultimap.create(), 0) + "|"
        + iteratorRemove(LinkedHashMultimap.create(), 1) + "|"
        + iteratorRemove(LinkedHashMultimap.create(), 2);
  }
  public static String sync(){ ListMultimap<String,Integer> base=ArrayListMultimap.create(); return Boolean.toString(Multimaps.synchronizedListMultimap(base) != base); }
  public static String excluded(){
    Multimap<String,Integer> m=ArrayListMultimap.create(); m.put("a",1);
    m.asMap(); m.entries(); m.keySet(); m.keys(); m.values();
    try { m.entries().iterator(); } catch (NullPointerException ignored) {}
    try { m.keySet().iterator(); } catch (NullPointerException ignored) {}
    m.replaceValues("a", java.util.List.of(9));
    m.asMap().remove("a");
    m.equals(m); m.toString();
    m.put(null, 1); m.get(null); m.containsKey(null);
    m.containsEntry(null, 1); m.remove(null, 1);
    try { m.removeAll("a").iterator(); } catch (NullPointerException ignored) {}
    return "done";
  }
}

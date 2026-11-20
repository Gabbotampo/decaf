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
  private static String wrappers(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("a", 2);
    Collection<Integer> direct = m.get("a");
    Collection<Integer> asMap = m.asMap().get("a");
    String classes = direct.getClass().getName() + "|" + asMap.getClass().getName()
        + "|" + m.asMap().entrySet().iterator().next().getValue().getClass().getName();
    if (direct instanceof java.util.List<?> list)
      return classes + "|" + list.get(1);
    @SuppressWarnings("unchecked") java.util.Set<Integer> set = (java.util.Set<Integer>) direct;
    return classes + "|" + set.contains(2) + "|" + set.add(2) + "|" + set.remove(1);
  }
  public static String wrappersArray() { return wrappers(ArrayListMultimap.create()); }
  public static String wrappersHash() { return wrappers(HashMultimap.create()); }
  public static String wrappersLinked() { return wrappers(LinkedHashMultimap.create()); }
  private static String countedWrapperCalls(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("a", 2);
    Collection<Integer> direct = m.get("a");
    StringBuilder out = new StringBuilder();
    try { direct.iterator(); out.append("iterator-ok"); }
    catch (RuntimeException e) { out.append("iterator-").append(e.getClass().getSimpleName()); }
    if (direct instanceof java.util.List<?> list) {
      try { list.listIterator(); out.append("|listIterator-ok"); }
      catch (RuntimeException e) { out.append("|listIterator-").append(e.getClass().getSimpleName()); }
      try { list.subList(0, 1); out.append("|subList-ok"); }
      catch (RuntimeException e) { out.append("|subList-").append(e.getClass().getSimpleName()); }
    }
    return out.toString();
  }
  public static String countedWrappersArray() { return countedWrapperCalls(ArrayListMultimap.create()); }
  public static String countedWrappersHash() { return countedWrapperCalls(HashMultimap.create()); }
  public static String countedWrappersLinked() { return countedWrapperCalls(LinkedHashMultimap.create()); }
  public static String instanceofKinds() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Object direct = m.get("a");
    Object asMap = m.asMap();
    Object asValue = m.asMap().get("a");
    Object values = m.asMap().values();
    Object keys = m.asMap().keySet();
    Object entry = m.asMap().entrySet().iterator().next();
    return ((Object)m instanceof java.util.Collection) + "|" + ((Object)m instanceof java.util.Map) + "|"
        + (direct instanceof java.util.Collection) + "|" + (direct instanceof java.util.List)
        + "|" + (asMap instanceof java.util.Collection) + "|" + (asMap instanceof java.util.Map)
        + "|" + (asValue instanceof java.util.Collection) + "|" + (values instanceof java.util.Collection)
        + "|" + (keys instanceof java.util.Collection) + "|" + (keys instanceof java.util.Set)
        + "|" + (entry instanceof java.util.Map.Entry);
  }
  public static String freshGet() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    return Boolean.toString(m.get("a") != m.get("a"));
  }
  public static String removeAllRefresh() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Collection<Integer> held = m.get("a");
    m.removeAll("a");
    m.put("a", 2);
    return held + "|" + held.size() + "|" + held.contains(1) + "|"
        + held.contains(2) + "|" + m.get("a");
  }
  private static String instanceofAll(Multimap<String,Integer> m) {
    m.put("a", 1);
    Object direct = m.get("a");
    Object asMap = m.asMap();
    Object asValue = m.asMap().get("a");
    Object values = m.asMap().values();
    Object keys = m.asMap().keySet();
    Object entry = m.asMap().entrySet().iterator().next();
    Object iterator = m.asMap().keySet().iterator();
    return ((Object)m instanceof java.util.Collection) + "," + ((Object)m instanceof ListMultimap)
        + "," + ((Object)m instanceof SetMultimap) + "," + ((Object)m instanceof java.util.Map)
        + ";" + (direct instanceof java.util.Collection) + "," + (direct instanceof java.util.List)
        + "," + (direct instanceof java.util.Set) + "," + (direct instanceof java.util.RandomAccess)
        + ";" + (asMap instanceof java.util.Map) + "," + (asMap instanceof java.util.Collection)
        + ";" + (asValue instanceof java.util.Collection) + "," + (asValue instanceof java.util.List)
        + "," + (asValue instanceof java.util.Set) + "," + (asValue instanceof java.util.RandomAccess)
        + ";" + (values instanceof java.util.Collection) + "," + (values instanceof java.util.Set)
        + ";" + (keys instanceof java.util.Collection) + "," + (keys instanceof java.util.Set)
        + ";" + (entry instanceof java.util.Map.Entry) + "," + (entry instanceof java.util.Collection)
        + ";" + (iterator instanceof java.util.Collection) + "," + (iterator instanceof java.util.Map);
  }
  public static String instanceofArrayAll() { return instanceofAll(ArrayListMultimap.create()); }
  public static String instanceofHashAll() { return instanceofAll(HashMultimap.create()); }
  public static String instanceofLinkedAll() { return instanceofAll(LinkedHashMultimap.create()); }
  private static String countedValuesCalls() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Collection<Collection<Integer>> values = m.asMap().values();
    StringBuilder out = new StringBuilder();
    try { values.toString(); out.append("toString-ok"); }
    catch (RuntimeException e) { out.append("toString-").append(e.getClass().getSimpleName()); }
    try { values.contains(java.util.List.of(1)); out.append("|contains-ok"); }
    catch (RuntimeException e) { out.append("|contains-").append(e.getClass().getSimpleName()); }
    try { values.remove(java.util.List.of(1)); out.append("|remove-ok"); }
    catch (RuntimeException e) { out.append("|remove-").append(e.getClass().getSimpleName()); }
    return out.toString();
  }
  public static String countedValues() { return countedValuesCalls(); }
  public static String countedEntrySetToString() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    try { m.asMap().entrySet().toString(); }
    catch (RuntimeException ignored) {}
    return "done";
  }
  public static String taintedRead() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    try { m.get("a").iterator(); } catch (RuntimeException ignored) {}
    try { m.size(); } catch (RuntimeException ignored) {}
    return "done";
  }
  public static String countedEntryTaintsMultimap() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Map.Entry<String, Collection<Integer>> e = m.asMap().entrySet().iterator().next();
    System.identityHashCode(e);
    try { m.size(); } catch (RuntimeException ignored) {}
    return "done";
  }
  public static String entryAfterTaintCounts() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    Map.Entry<String, Collection<Integer>> e = m.asMap().entrySet().iterator().next();
    try { m.entries(); } catch (RuntimeException ignored) {}
    try { e.getKey(); } catch (RuntimeException ignored) {}
    return "done";
  }
  public static String countedReadTaintsMultimap() {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    m.entries();
    try { m.size(); } catch (RuntimeException ignored) {}
    return "done";
  }
  private static void laterReads(Multimap<String,Integer> m) {
    try { m.size(); } catch (RuntimeException ignored) {}
    try { m.get("a").size(); } catch (RuntimeException ignored) {}
    try { m.containsKey("a"); } catch (RuntimeException ignored) {}
    try { m.asMap().entrySet().iterator().hasNext(); } catch (RuntimeException ignored) {}
    try { m.entries().size(); } catch (RuntimeException ignored) {}
  }
  private static String countedMutationReads(int which) {
    ArrayListMultimap<String,Integer> m = ArrayListMultimap.create();
    m.put("a", 1);
    try {
      switch (which) {
        case 0: m.entries(); break;
        case 1: m.keys(); break;
        case 2: m.replaceValues("a", java.util.List.of(2)); break;
        default: m.asMap().values().remove(java.util.List.of(1)); break;
      }
    } catch (RuntimeException ignored) {}
    laterReads(m);
    return "done";
  }
  public static String countedMutationEntries() { return countedMutationReads(0); }
  public static String countedMutationKeys() { return countedMutationReads(1); }
  public static String countedMutationReplaceValues() { return countedMutationReads(2); }
  public static String countedMutationAsMapValuesRemove() { return countedMutationReads(3); }
  private static String entrySetHandles(Multimap<String,Integer> m) {
    m.put("a", 1); m.put("a", 2);
    java.util.Set<Map.Entry<String, Collection<Integer>>> entries = m.asMap().entrySet();
    Map.Entry<String, Collection<Integer>> e = entries.iterator().next();
    Map.Entry<String, Collection<Integer>> same = entries.iterator().next();
    boolean contains = entries.contains(same);
    boolean removed = entries.remove(same);
    return contains + "|" + removed + "|" + entries.size() + "|" + entries.contains(e);
  }
  public static String entrySetHandlesArray() { return entrySetHandles(ArrayListMultimap.create()); }
  public static String entrySetHandlesHash() { return entrySetHandles(HashMultimap.create()); }
  public static String entrySetHandlesLinked() { return entrySetHandles(LinkedHashMultimap.create()); }
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

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
  public static String sync(){ ListMultimap<String,Integer> base=ArrayListMultimap.create(); return Boolean.toString(Multimaps.synchronizedListMultimap(base) != base); }
  public static String excluded(){
    Multimap<String,Integer> m=ArrayListMultimap.create(); m.put("a",1);
    m.asMap(); m.entries(); m.keySet(); m.keys(); m.values();
    try { m.entries().iterator(); } catch (NullPointerException ignored) {}
    try { m.keySet().iterator(); } catch (NullPointerException ignored) {}
    m.replaceValues("a", java.util.List.of(9));
    m.removeAll("a");
    m.equals(m); m.toString();
    m.put(null, 1); m.get(null); m.containsKey(null);
    m.containsEntry(null, 1); m.remove(null, 1);
    try { m.removeAll("a").iterator(); } catch (NullPointerException ignored) {}
    return "done";
  }
}

package demo;
import com.google.common.collect.*;
import java.util.*;
public final class Probe {
  private static String run(Multimap<String,Integer> m) {
    m.put("a",1); m.put("b",2); m.put("a",3);
    boolean x=m.get("a").add(4);
    boolean y=m.putAll("b", List.of(5,6));
    return m+"|"+x+"|"+y+"|"+m.size()+"|"+m.keySet()+"|"+m.values()+"|"+m.entries()+"|"+m.asMap()+"|"+m.containsKey("a")+"|"+m.containsEntry("a",3)+"|"+m.remove("a",3)+"|"+m.removeAll("b")+"|"+m;
  }
  public static String array(){return run(ArrayListMultimap.create());}
  public static String linked(){return run(LinkedHashMultimap.create());}
  public static String sync(){return run(Multimaps.synchronizedListMultimap(ArrayListMultimap.create()));}
  public static String identity(){
    Multimap<String,Integer> m=ArrayListMultimap.create(); m.put("a",1);
    Multimap<String,Integer> n=ArrayListMultimap.create(); n.put("a",1);
    return m.getClass().getName()+"|"+m.get("a").getClass().getName()+"|"+m.keySet().getClass().getName()+"|"+m.values().getClass().getName()+"|"+m.entries().getClass().getName()+"|"+m.asMap().getClass().getName()+"|"+(m.get("a")==m.get("a"))+"|"+(m.keySet()==m.keySet())+"|"+m.equals(n)+"|"+m.hashCode()+"|"+m;
  }
}

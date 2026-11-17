package demo;
import java.util.*; import java.util.stream.*;
public class Probe {
 static String rev(List<Integer> l){ Collections.reverse(l); return l.toString(); }
 public static String rows(){
  StringBuilder s=new StringBuilder();
  for(List<Integer> l: List.of(new ArrayList<>(),new ArrayList<>(List.of(1)),new ArrayList<>(List.of(1,2)),new ArrayList<>(List.of(1,2,3)),new LinkedList<>(List.of(1,2)),new LinkedList<>(List.of(1,2,3)))) s.append(rev(l)).append('|');
  for(List<Integer> l: List.of(List.of(),List.of(1),List.of(1,2))){try{Collections.reverse(l);s.append("ok|");}catch(Exception e){s.append(e.getClass().getSimpleName()).append('|');}}
  List<Integer> m=new ArrayList<>(List.of(1,2,3)); Iterator<Integer> it=m.iterator(); Collections.reverse(m); try{it.next();s.append("next");}catch(Exception e){s.append(e.getClass().getSimpleName());} s.append('|');
  s.append(Stream.of(1,2,3).iterator().next()).append('|');
  return s.toString(); }
 public static void main(String[] a){System.out.print(rows());}
}

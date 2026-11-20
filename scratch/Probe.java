import java.util.*;
public class Probe {
 static String x(Runnable r){try{r.run();return "OK";}catch(Throwable t){return t.getClass().getSimpleName();}}
 public static String run(){
  List<String> a=List.of("a","b","c","d"); List<String> s=a.subList(1,2).subList(0,1);
  List<String> e=Collections.<String>emptyList().subList(0,0);
  List<String> q=Collections.singletonList("a").subList(0,1);
  List<String> qe=Collections.singletonList("a").subList(0,0);
  List<String> n=Collections.nCopies(2,"a").subList(0,2);
  return "null="+x(()->s.indexOf(null))+","+x(()->s.contains(null))+"|emptyAdd="+x(()->e.add("z"))+"|singleSort="+x(()->Collections.singletonList("a").sort(null))+"|single="+x(()->q.set(0,"z"))+","+x(()->q.add("z"))+","+x(()->q.clear())+"|singleEmpty="+x(()->qe.add("z"))+","+x(()->qe.clear())+"|copies="+x(()->n.set(0,"z"))+","+x(()->n.add("z"));
 }
 public static void main(String[]x){System.out.print(run());}
}

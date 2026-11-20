import java.util.*;
public class Sub {
 static void p(String n,Runnable r){try{r.run();System.out.println(n+" OK");}catch(Throwable t){System.out.println(n+" "+t.getClass().getSimpleName());}}
 public static void main(String[]x){
  List<List<String>> ls=List.of(Collections.emptyList(),Collections.singletonList("a"),Collections.nCopies(2,"a"),List.of("a","b"));
  for(List<String> l:ls) {System.out.println("P "+l.getClass()+" "+l); for(int[] q:new int[][]{{0,0},{0,l.size()},{0,Math.min(l.size(),1)}}){ List<String>s=l.subList(q[0],q[1]);System.out.println(" S "+s.getClass()+" "+s); for(String op:new String[]{"set","add","addAll","remove","clear","sort"}){p(op,()->{switch(op){case"set":s.set(0,"z");break;case"add":s.add("z");break;case"addAll":s.addAll(List.of("z"));break;case"remove":s.remove("a");break;case"clear":s.clear();break;default:s.sort(null);}});}}}
 }
}

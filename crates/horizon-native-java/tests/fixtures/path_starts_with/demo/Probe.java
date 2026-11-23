package demo;
import java.nio.file.*;
public final class Probe {
 static final String[] TEXTS = {"", "/", "a", "a/", "a/b", "a/b/c", "/a", "/a/b", "/a/b/", "//a//b", ".", "..", "../a", "a/..", "./a", "a/./b", "../..", "...", "..a", "a..", "/..", "/../a", "a b", " ", "a/ ", "ab", "abc", "a/bc", "b", "abc/d", "a//b", "/a/b/c"};
 static String q(java.util.function.Supplier<Object> s){try{return String.valueOf(s.get());}catch(RuntimeException e){return e.getClass().getSimpleName()+":"+e.getMessage();}}
 public static String rows(){
  StringBuilder x=new StringBuilder();
  for (String p : TEXTS) {
   for (String s : TEXTS) {
    Path path = Path.of(p);
    x.append("[").append(p).append("] [").append(s).append("] str=")
     .append(q(() -> path.startsWith(s))).append(" path=")
     .append(q(() -> path.startsWith(Path.of(s)))).append("\n");
   }
  }
  for (String p : TEXTS) {
   Path n = Paths.get(p).normalize();
   x.append("normalize [").append(p).append("] -> [").append(n).append("] dots=")
    .append(q(() -> n.startsWith(".."))).append(" dotsPath=").append(q(() -> n.startsWith(Path.of("..")))).append("\n");
  }
  Path joined = Path.of("a", "b");
  x.append("joined.str=").append(q(() -> joined.startsWith("a"))).append(" resolved=").append(q(() -> Path.of("/a").resolve("b").startsWith("/a/b"))).append(" parent=").append(q(() -> Path.of("a/b").getParent().startsWith("a"))).append("\n");
  return x.toString();
 }
 public static void main(String[] x){System.out.print(rows());}
}

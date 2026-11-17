package demo;
import java.nio.file.*;
public final class Probe {
 static String q(java.util.function.Supplier<Object> s){try{return String.valueOf(s.get());}catch(RuntimeException e){return e.getClass().getSimpleName()+":"+e.getMessage();}}
 public static void main(String[] x){System.out.print(rows());}
 public static String rows(){ Path a=Path.of("/a/b/../c"); Path e=Path.of(""); return "file="+q(a::getFileName)+"|parent="+q(a::getParent)+"|root="+q(a::getRoot)+"|count="+a.getNameCount()+"|name="+q(()->a.getName(0))+"|oor="+q(()->a.getName(9))+"|sub="+q(()->a.subpath(0,2))+"|abs="+a.isAbsolute()+"|empty="+q(e::getFileName)+"|eq="+a.equals(Path.of("/a/b/../c")); }
}

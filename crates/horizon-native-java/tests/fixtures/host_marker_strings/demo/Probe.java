package demo;
import java.util.*;
public final class Probe {
 static String s(String p,String x){return new StringBuilder().append(p).append(x).toString();}
 static String one(String n,String x){String c=new String(x); return n+"="+x+"|"+x.length()+"|"+(x instanceof String)+"|"+x.equals(c)+"|"+x.toUpperCase()+"|"+x.substring(0,Math.min(3,x.length()));}
 public static String strings(){String[] a={s("abc/","hcb:x"),s("list/","hobj:x"),s("h","cb:"),s("enum/","x"),s("client/","x"),s("game/","x")}; StringBuilder b=new StringBuilder(); for(int i=0;i<a.length;i++){if(i>0)b.append('\n');b.append(one("s"+i,a[i]));} return b.toString();}
 public static String collections(){Runnable r=()->{}; Object[] a={List.of(r),new ArrayList<>(List.of(r)),Map.of("k",r),Optional.of(r)}; StringBuilder b=new StringBuilder(); for(int i=0;i<a.length;i++){if(i>0)b.append('\n');b.append("c"+i+"=").append(String.valueOf(a[i]));} return b.toString();}
 public static void main(String[] x){System.out.println(strings());System.out.println(collections());}
}

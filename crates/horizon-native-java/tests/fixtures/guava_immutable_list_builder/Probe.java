import com.google.common.collect.*; import java.util.*;
public class Probe { static void p(String k,Object v){System.out.println(k+"="+v);} public static void main(String[]a){
 var b=ImmutableList.<String>builder(); p("empty",b.build().size()); p("emptyIdentity",b.build()==ImmutableList.<String>of());
 try{b.add((String)null);p("null","ok");}catch(Exception e){p("null",e.getClass().getName()+":"+e.getMessage());}
 b.add("a").add("b"); p("order",b.build()); var c=b.build(); b.add("c"); p("reuse",c+"/"+b.build());
 var d=ImmutableList.<String>builderWithExpectedSize(3); d.add("x","y"); d.addAll(List.of("z")); p("varargsIterable",d.build());
 var e=ImmutableList.<String>builder(); e.addAll(List.of("i","j").iterator()); p("iterator",e.build());
 }}

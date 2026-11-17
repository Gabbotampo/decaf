package demo;
public final class Probe {
 static String built(String a,String b){return new StringBuilder(a).append(b).toString();}
 static String row(String s){Object o=s; return (o instanceof String)+"|"+s.length()+"|"+s.equals(s);}
 public static String hello(){return row(built("hel","lo"));}
 public static String client(){return row(built("client/","x"));}
 public static String blockPos(){return row(built("block-pos/","x"));}
 public static String typed(){return row(built("typed/","x"));}
 public static String registry(){return row(built("registry/","x"));}
 public static String payload(){return row(built("payload/","x"));}
 public static String payloadContext(){return row(built("payload-context/","x"));}
 public static String iterator(){return row(built("iterator/","x"));}
 public static String enumX(){return row(built("enum/","x"));}
 public static String game(){return row(built("game/","x"));}
 public static String list(){return row(built("list/hobj:","x"));}
 public static String particle(){return row(built("particle/","x"));}
 public static String callback(){return row(built("abc/hcb:","x"));}
 public static String stream(){return row(built("stream/","x"));}
 public static String textures(){return row(built("textures/gui/","a.png"));}
 public static String models(){return row(built("models/item/","x"));}
}

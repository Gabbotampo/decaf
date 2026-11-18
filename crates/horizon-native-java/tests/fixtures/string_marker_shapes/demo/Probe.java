package demo;
public final class Probe {
 static String built(String a,String b){return new StringBuilder(a).append(b).toString();}
 static String row(String s){Object o=s; return (o instanceof String)+"|"+s.length()+"|"+s.equals(s);}
 static String castRow(String s){Object o=s; String cast=(String)o; return (o instanceof String)+"|"+cast.length()+"|"+cast.equals(s);}
 public static String hello(){return row(built("hel","lo"));}
 public static String client(){return castRow(built("client/","x"));}
 public static String blockPos(){return castRow(built("block-pos/","x"));}
 public static String typed(){return castRow(built("typed/","x"));}
 public static String registry(){return castRow(built("registry/","x"));}
 public static String payload(){return castRow(built("payload/","x"));}
 public static String payloadContext(){return castRow(built("payload-context/","x"));}
 public static String iterator(){return castRow(built("iterator/","x"));}
 public static String enumX(){return castRow(built("enum/","x"));}
 public static String game(){return castRow(built("game/","x"));}
 public static String list(){return castRow(built("list/hobj:","x"));}
 public static String particle(){return castRow(built("particle/","x"));}
 public static String callback(){return castRow(built("abc/hcb:","x"));}
 public static String stream(){return row(built("stream/","x"));}
 public static String textures(){return row(built("textures/gui/","a.png"));}
 public static String models(){return row(built("models/item/","x"));}
 public static void main(String[] a){System.out.println("hello="+hello());System.out.println("client="+client());System.out.println("blockPos="+blockPos());System.out.println("typed="+typed());System.out.println("registry="+registry());System.out.println("payload="+payload());System.out.println("payloadContext="+payloadContext());System.out.println("iterator="+iterator());System.out.println("enumX="+enumX());System.out.println("game="+game());System.out.println("list="+list());System.out.println("particle="+particle());System.out.println("callback="+callback());System.out.println("stream="+stream());System.out.println("textures="+textures());System.out.println("models="+models());}
}

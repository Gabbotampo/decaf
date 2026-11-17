package demo;
public final class Probe {
  static String one(String s) { Object o=s; return s+"|"+(o instanceof String)+"|"+o.getClass().getName()+"|"+s.length()+"|"+s.equals(o)+"|"+s.hashCode(); }
  public static void main(String[] a){System.out.print(rows());}
  public static String rows() { StringBuilder b=new StringBuilder(); String[] xs={"hello","abc/hcb:x","enum/x","game/x","list/hobj:x","textures/gui/a.png","models/item/x"}; for(int i=0;i<xs.length;i++){if(i>0)b.append('\n'); b.append(one(new StringBuilder().append(xs[i]).toString()));} return b.toString(); }
}

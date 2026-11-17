package demo;
public final class Probe {
  static String one(String s) { Object o=s; return s+"|"+(o instanceof String)+"|"+o.getClass().getName()+"|"+s.length()+"|"+s.equals(o)+"|"+s.hashCode(); }
  public static String counted(){ try { return one(new StringBuilder().append("abc/hcb:x").toString()); } catch (Throwable t) { return "threw"; } }
  public static void main(String[] a){System.out.print(rows());}
  public static String rows() { StringBuilder b=new StringBuilder(); String[] xs={"hello","game/x","list/hobj:x"}; for(int i=0;i<xs.length;i++){if(i>0)b.append('\n'); b.append(one(new StringBuilder().append(xs[i]).toString()));} return b.toString(); }
}

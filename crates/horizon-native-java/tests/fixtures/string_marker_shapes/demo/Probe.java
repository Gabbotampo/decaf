package demo;
public final class Probe {
  static String one(String s) { Object o=s; return s+"|"+(o instanceof String)+"|"+o.getClass().getName()+"|"+s.length()+"|"+s.equals(o)+"|"+s.hashCode(); }
  static final Object X = new StringBuilder().append("iterator/unknown").toString();
  public static String instanceOfUndecidable(){ try { return String.valueOf(X instanceof String); } catch (Throwable t) { return "threw"; } }
  public static String castUndecidable(){ try { return String.valueOf((String) X); } catch (Throwable t) { return "threw"; } }
  public static String getClassUndecidable(){ try { return X.getClass().getName(); } catch (Throwable t) { return "threw"; } }
  public static void main(String[] a){System.out.print(rows());}
  public static String rows() { StringBuilder b=new StringBuilder(); String[] xs={"hello","game/x","list/hobj:x"}; for(int i=0;i<xs.length;i++){if(i>0)b.append('\n'); b.append(one(new StringBuilder().append(xs[i]).toString()));} return b.toString(); }
}

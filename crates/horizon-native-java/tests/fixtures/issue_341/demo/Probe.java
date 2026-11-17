package demo;
import java.io.*;
public class Probe {
  static String one(String n, Throwing t) { try { return n+"="+t.get(); } catch(Throwable e) { return n+"="+e.getClass().getName()+":"+String.valueOf(e.getMessage()); } }
  interface Throwing { String get() throws Exception; }
  public static String rows() {
    return String.join("\n",
      one("read", () -> { var s=new ByteArrayInputStream(new byte[]{65,(byte)255}); return s.read()+","+s.read()+","+s.read(); }),
      one("readPart", () -> { var s=new ByteArrayInputStream(new byte[]{10,20,30,40},1,2); byte[] b={0,0,0,0,0}; int n=s.read(b,1,2); return n+":"+b[0]+","+b[1]+","+b[2]+","+b[3]+","+b[4]+":"+s.available(); }),
      one("zero", () -> { var s=new ByteArrayInputStream(new byte[]{1}); byte[] b={9}; return s.read(b,0,0)+":"+s.available(); }),
      one("bad", () -> { var s=new ByteArrayInputStream(new byte[]{1}); try { s.read(new byte[2],2,1); return "none"; } catch(IndexOutOfBoundsException e) { return e.getClass().getName(); } }),
      one("negative", () -> { var s=new ByteArrayInputStream(new byte[]{1}); return ""+s.skip(-2); }),
      one("past", () -> { var s=new ByteArrayInputStream(new byte[]{1,2}); return s.skip(99)+":"+s.available(); }),
      one("all", () -> { byte[] b=new ByteArrayInputStream(new byte[]{1,2,3,4},1,2).readAllBytes(); return b.length+":"+b[0]+","+b[1]; } ),
      one("ctor", () -> { var s=new ByteArrayInputStream(new byte[]{9,8,7,6},1,2); return s.available()+":"+s.read()+":"+s.read()+":"+s.read(); })
    );
  }
  public static void main(String[] a) { System.out.print(rows()); }
}

package demo;
import java.io.*; import java.nio.file.*; import java.util.*;
public class Probe {
  interface X { void run() throws Exception; }
  static String err(X r){ try{r.run(); return "ok";}catch(Exception x){return x.getClass().getName()+":"+x.getMessage();} }
  static InputStream open(Path p, boolean channel) throws Exception {
    return channel ? Files.newInputStream(p) : new FileInputStream(p.toFile());
  }
  static String closed(Path p, boolean channel) throws Exception {
    InputStream in=open(p,channel); in.close(); in.close();
    String a=err(()->in.read());
    String b=err(()->in.available());
    String c=Arrays.toString(in.readNBytes(0));
    String d=err(()->in.readNBytes(new byte[2],0,0));
    String e=err(()->in.readNBytes((byte[])null,0,0));
    String f=err(()->in.readNBytes(new byte[2],3,0));
    String g=err(()->in.readNBytes(new byte[2],0,-1));
    byte[] buf=new byte[2];
    String h=err(()->in.readNBytes(buf,0,1));
    return a+"|"+b+"|"+c+"|"+d+"|"+e+"|"+f+"|"+g+"|"+h;
  }
  static String counted(Path p) throws Exception {
    InputStream in=open(p,false); in.close();
    String skip=err(()->in.skip(1));
    String all=err(()->in.readAllBytes());
    String positive=err(()->in.readNBytes(1));
    String negative=err(()->in.skip(-1));
    // Native failures whose text is the OS's: counted, never answered.
    String transfer=err(()->in.transferTo(new ByteArrayOutputStream()));
    InputStream live=open(p,false);
    String openNegative=err(()->live.skip(-1));
    live.close();
    // Opening below a missing directory: the FileNotFoundException's text is the OS's.
    String missingParent=err(()->new FileOutputStream(p.resolveSibling("no-such-dir").resolve("x.bin").toFile()).close());
    return skip+"|"+all+"|"+positive+"|"+negative+"|"+transfer+"|"+openNegative+"|"+missingParent;
  }
  static String nbytes(Path p, boolean channel) throws Exception {
    try (InputStream in=open(p,channel)) {
      String neg=err(()->in.readNBytes(-1));
      String bulkNeg=err(()->in.readNBytes(new byte[2],0,-1));
      String bulkOff=err(()->in.readNBytes(new byte[2],3,0));
      String array=Arrays.toString(in.readNBytes(2));
      byte[] b=new byte[4]; int n=in.readNBytes(b,1,2);
      int eof=in.readNBytes(b,0,4); int eofAgain=in.readNBytes(b,0,4);
      return neg+"|"+bulkNeg+"|"+bulkOff+"|"+array+"|"+n+":"+Arrays.toString(b)+"|"+eof+"|"+eofAgain;
    }
  }
  static String skipping(Path p, boolean channel) throws Exception {
    try (InputStream in=open(p,channel)) {
      String neg=err(()->in.skip(-1));
      long skipped=in.skip(99);
      int available=in.available(); int read=in.read();
      return neg+"|"+skipped+"|"+available+"|"+read;
    }
  }
  public static String rows() {
    try {
      Path d=Path.of("fos-scratch"); Files.createDirectories(d); Path f=d.resolve("x.bin");
      Files.write(f,new byte[]{1,2,3});
      return "fis.closed="+closed(f,false)+";files.closed="+closed(f,true)
        +";fis.nbytes="+nbytes(f,false)+";files.nbytes="+nbytes(f,true)
        +";files.skip="+skipping(f,true);
    } catch(Exception x){return "ERR:"+x;}
  }
  public static String countedRows() {
    try {
      Path f=Path.of("fos-scratch").resolve("x.bin");
      return counted(f);
    } catch(Exception x){return "ERR:"+x;}
  }
  public static void main(String[] a){System.out.println(rows());}
}

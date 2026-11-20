package demo;
import java.io.*; import java.nio.file.*;
public class Probe {
  interface X { void run() throws Exception; }
  static String err(X r){ try{r.run(); return "ok";}catch(Exception x){return x.getClass().getName()+":"+x.getMessage();} }
  static String fis(Path p) throws Exception {
    try (FileInputStream in = new FileInputStream(p.toFile())) {
      return java.util.Arrays.toString(in.readAllBytes());
    }
  }
  static String raw(Path p) throws Exception {
    try (FileInputStream in = new FileInputStream(p.toFile())) {
      int a=in.read(), b=in.read(), c=in.read(), d=in.read();
      return a+","+b+","+c+","+d;
    }
  }
  static String bulk(Path p) throws Exception {
    try (InputStream in = Files.newInputStream(p)) {
      byte[] b=new byte[8]; int n=in.read(b);
      return n+":"+java.util.Arrays.toString(b);
    }
  }
  static String bulkOffset(Path p) throws Exception {
    try (InputStream in = Files.newInputStream(p)) {
      byte[] b={9,9,9,9,9}; int n=in.read(b,1,2);
      return n+":"+java.util.Arrays.toString(b);
    }
  }
  static String positioning(Path p) throws Exception {
    try (InputStream in = Files.newInputStream(p)) {
      int before=in.available(); long skipped=in.skip(2); int after=in.available();
      return before+":"+skipped+":"+after+":"+java.util.Arrays.toString(in.readAllBytes());
    }
  }
  static String all(Path p) throws Exception {
    try (InputStream in = Files.newInputStream(p)) {
      return java.util.Arrays.toString(in.readAllBytes());
    }
  }
  static String buffered(Path p) {
    return err(()->{try (BufferedReader r=Files.newBufferedReader(p)) { r.read(); }});
  }
  static String string(Path p) {
    return err(()->Files.readString(p));
  }
  public static String rows() {
    try {
      Path d=Path.of("fos-scratch"); Files.createDirectories(d); Path f=d.resolve("x.bin"); Files.deleteIfExists(f);
      FileOutputStream o=new FileOutputStream(f.toFile()); byte[] b={9,8,7,6}; o.write(b,1,2); o.write(b,0,0); o.write(257); o.close(); o.close();
      String bytes=java.util.Arrays.toString(Files.readAllBytes(f));
      String after=err(()->{o.write(b,0,1);});
      String closedNull=err(()->{o.write(null,0,1);});
      String closedOff=err(()->{o.write(b,-1,1);});
      String closedLen=err(()->{o.write(b,0,-1);});
      String negOff=err(()->{new FileOutputStream(d.resolve("n").toFile()).write(b,-1,1);});
      String negLen=err(()->{new FileOutputStream(d.resolve("m").toFile()).write(b,0,-1);});
      String over=err(()->{new FileOutputStream(d.resolve("q").toFile()).write(b,3,2);});
      String nul=err(()->{new FileOutputStream(d.resolve("z").toFile()).write(null,0,1);});
      Path live=d.resolve("live.bin"); Files.deleteIfExists(live);
      FileOutputStream liveOut=new FileOutputStream(live.toFile()); liveOut.write(new byte[]{1,2,3});
      String beforeFis=fis(live); String beforeFiles=java.util.Arrays.toString(Files.readAllBytes(live));
      liveOut.close(); String afterFis=fis(live); String afterFiles=java.util.Arrays.toString(Files.readAllBytes(live));
      Path trunc=d.resolve("trunc.bin"); Files.deleteIfExists(trunc);
      try(FileOutputStream first=new FileOutputStream(trunc.toFile())) { first.write(new byte[]{4,5,6,7}); }
      try(FileOutputStream second=new FileOutputStream(trunc.toFile())) { second.write(new byte[]{8,9}); }
      String reopen=fis(trunc);
      Path shared=d.resolve("shared.bin"); Files.deleteIfExists(shared);
      FileOutputStream one=new FileOutputStream(shared.toFile()); FileOutputStream two=new FileOutputStream(shared.toFile());
      one.write('A'); two.write('B'); one.write('C'); one.close(); two.close();
      String twoStreams=fis(shared);
      Path nd=Path.of("missing-parent"); Files.deleteIfExists(nd.resolve("x.zip")); String parent=err(()->{new FileOutputStream(nd.resolve("x.zip").toFile());});
      Path raw=d.resolve("raw.bin"); Files.write(raw,new byte[]{(byte)0xc3,0x28,(byte)0xff,0});
      return "bytes="+bytes+";after="+after+";closedNull="+closedNull+";closedOff="+closedOff+";closedLen="+closedLen+";negOff="+negOff+";negLen="+negLen+";over="+over+";null="+nul+";beforeFis="+beforeFis+";beforeFiles="+beforeFiles+";afterFis="+afterFis+";afterFiles="+afterFiles+";reopen="+reopen+";twoStreams="+twoStreams+";parent="+parent+";raw="+raw(raw)+";bulk="+bulk(raw)+";bulkOffset="+bulkOffset(raw)+";positioning="+positioning(raw)+";all="+all(raw)+";buffered="+buffered(raw)+";string="+string(raw);
    } catch(Exception x){return "ERR:"+x;}
  }
  public static void main(String[] a){System.out.println(rows());}
}

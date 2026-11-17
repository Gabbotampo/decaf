package demo;
import java.util.*; import java.io.*; import java.nio.charset.StandardCharsets;
public class Probe {
 static String esc(String s){return s.replace("\\","\\\\").replace("\n","\\n").replace("\r","\\r").replace("\t","\\t").replace("\f","\\f");}
 static String load(String text, boolean stream) throws Exception { Properties p=new Properties(); if(stream)p.load(new ByteArrayInputStream(text.getBytes("ISO-8859-1"))); else p.load(new StringReader(text)); StringBuilder b=new StringBuilder(); for(String k:new TreeSet<>(p.stringPropertyNames())) b.append(esc(k)).append('=').append(esc(p.getProperty(k))).append(';'); return b.toString(); }
 static String row(String x, boolean s){try{return load(x,s);}catch(Exception e){return e.getClass().getName()+":"+e.getMessage();}}
 public static String rows(){StringBuilder b=new StringBuilder(); String[] a={"# c\n! d\nkey=value\n", "a=b\nc:d\ne f\ng   h\ni= j\n", "cont=one\\\n  two\neven=x\\\\\ny=z\n", "u=\\u00E9\n", "=emptykey\nempty=\n", "dup=one\ndup=two\n", "trail=x\\", "bad=\\u12G4\n"}; for(int i=0;i<a.length;i++){b.append(i).append('R').append('=').append(row(a[i],false)).append('|').append(i).append('I').append('=').append(row(a[i],true)).append('\n');} return b.toString();}
 public static void main(String[]x){System.out.print(rows());}
}

package demo;
import java.io.*; import java.lang.reflect.*; import java.util.*; import net.fabricmc.loader.api.metadata.*;
public class Probe {
 public static String rows() throws Exception {
  String j="{\"schemaVersion\":1,\"id\":\"demo\",\"version\":\"1\",\"custom\":{\"z\":1,\"a\":{\"nested\":true},\"arr\":[false,null,2.5],\"dup\":\"first\",\"mid\":null,\"dup\":\"last\"}}";
  Method m=Class.forName("net.fabricmc.loader.impl.metadata.ModMetadataParser").getDeclaredMethod("readModMetadata",InputStream.class,boolean.class); m.setAccessible(true);
  Object md=m.invoke(null,new ByteArrayInputStream(j.getBytes()),false);
  CustomValue.CvObject o=((net.fabricmc.loader.impl.metadata.LoaderModMetadata)md).getCustomValue("cfg").getAsObject();
  StringBuilder b=new StringBuilder(); for(var e:o){ b.append(e.getKey()).append('=').append(e.getValue().getType()).append(':').append(e.getValue()).append('|'); }
  try { o.iterator().remove(); } catch(Exception e){ b.append("remove=").append(e.getClass().getName()).append('|'); }
  b.append("size=").append(o.size()); return b.toString();
 }
 public static void main(String[] a) throws Exception { System.out.print(rows()); }
}

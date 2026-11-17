package demo;
import java.util.*;
import net.minecraft.network.codec.*;
public final class Probe {
 public static String rows(){
  StreamCodec<?,?> m1=ByteBufCodecs.map(HashMap::new,ByteBufCodecs.VAR_INT,ByteBufCodecs.STRING_UTF8);
  StreamCodec<?,?> m2=ByteBufCodecs.map(HashMap::new,ByteBufCodecs.VAR_INT,ByteBufCodecs.STRING_UTF8);
  StreamCodec<?,?> c1=ByteBufCodecs.collection(ArrayList::new,ByteBufCodecs.VAR_INT);
  StreamCodec<?,?> c2=ByteBufCodecs.collection(ArrayList::new,ByteBufCodecs.VAR_INT);
  StringBuilder s=new StringBuilder(); s.append(m1!=m2).append('|').append(c1!=c2).append('|').append(m1 instanceof StreamCodec).append('|').append(c1 instanceof StreamCodec);
  try { ((StreamCodec)m1).encode(null,null); s.append("|encode-ok"); } catch(Throwable t){ s.append("|encode:").append(t.getClass().getName()); }
  try { ByteBufCodecs.map(null,ByteBufCodecs.VAR_INT,ByteBufCodecs.STRING_UTF8); s.append("|null-ok"); } catch(Throwable t){ s.append("|null:").append(t.getClass().getName()); }
  return s.toString();
 }
 public static void main(String[] a){System.out.print(rows());}
}

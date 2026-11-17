package demo;
import net.minecraft.network.codec.StreamCodec;
public final class Probe {
 public static String rows(){
  StreamCodec<Object,String> base=null;
  StreamCodec<?,?> m1=base.map(x->x,x->x), m2=base.map(x->x,x->x);
  StreamCodec.CodecOperation<Object,String,Integer> op=c->(StreamCodec<Object,Integer>) null;
  StreamCodec<?,?> a=base.apply(op), am=a.map(x->x,x->x);
  return "mapDistinct="+(m1!=m2)+"\nmapCodec="+(m1 instanceof StreamCodec)+"\napplyCodec="+(a instanceof StreamCodec)+"\napplyMapCodec="+(am instanceof StreamCodec)+"\n";
 }
 public static String counted(){
  StreamCodec<Object,String> base=null; StreamCodec<?,?> a=base.apply(c->(StreamCodec<Object,Integer>) null);
  try { ((StreamCodec)a).encode(null,null); } catch(RuntimeException e) { return e.getClass().getName(); }
  return "returned";
 }
 public static void main(String[]x){System.out.print(rows());}
}

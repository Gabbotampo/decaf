package demo;
import net.minecraft.network.chat.*;
import net.minecraft.resources.Identifier;
public final class Probe {
 public static Component a() { return CommonComponents.EMPTY; }
 public static Component b() { return CommonComponents.EMPTY; }
 public static String rows() {
  Component e=CommonComponents.EMPTY;
  String a=e.getString();
  String b=Boolean.toString(CommonComponents.EMPTY==CommonComponents.EMPTY);
  String c=Boolean.toString(e.equals(Component.empty()));
  String d=Boolean.toString(Identifier.CODEC==Identifier.CODEC);
  String e=Boolean.toString(a()==b());
  String f=Integer.toString(CommonComponents.EMPTY.hashCode());
  return String.join("|",a,b,c,d,e,f);
 }
 public static String counted() { return Boolean.toString(Identifier.CODEC.fieldOf("id") != null); }
}

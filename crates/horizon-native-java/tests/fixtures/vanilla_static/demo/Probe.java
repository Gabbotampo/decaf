package demo;
import net.minecraft.network.chat.*;
import net.minecraft.resources.Identifier;
public final class Probe {
 public static String rows() {
  Component e=CommonComponents.EMPTY;
  String a=e.getString();
  String b=Boolean.toString(CommonComponents.EMPTY==CommonComponents.EMPTY);
  String c=Boolean.toString(e.equals(Component.empty()));
  String d=Boolean.toString(Identifier.CODEC==Identifier.CODEC);
  return String.join("|",a,b,c,d);
 }
}

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
 public static String codecEncode() { try { Identifier.CODEC.encodeStart(com.mojang.serialization.JsonOps.INSTANCE, Identifier.parse("minecraft:stone")); return "ok"; } catch (Throwable t) { return t.getClass().getName(); } }
 public static String codecDecode() { try { Identifier.CODEC.parse(com.mojang.serialization.JsonOps.INSTANCE, new com.google.gson.JsonPrimitive("minecraft:stone")); return "ok"; } catch (Throwable t) { return t.getClass().getName(); } }
 public static String codecDecodePair() { try { Identifier.CODEC.decode(com.mojang.serialization.JsonOps.INSTANCE, new com.google.gson.JsonPrimitive("minecraft:stone")); return "ok"; } catch (Throwable t) { return t.getClass().getName(); } }

 public static String counted() { return Boolean.toString(Identifier.CODEC.fieldOf("id") != null); }
}

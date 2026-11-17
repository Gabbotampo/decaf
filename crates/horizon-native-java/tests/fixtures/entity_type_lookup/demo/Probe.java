package demo;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
public final class Probe {
 static Identifier id(String s){ return Identifier.parse(s); }
 public static String rows(){ return String.join("|", Boolean.toString(BuiltInRegistries.ENTITY_TYPE.getOptional(id("minecraft:pig")).isPresent()), Boolean.toString(BuiltInRegistries.ENTITY_TYPE.getOptional(id("minecraft:nope")).isPresent()), Boolean.toString(BuiltInRegistries.ENTITY_TYPE.getOptional(id("minecraft:pig")).get()==BuiltInRegistries.ENTITY_TYPE.getOptional(id("minecraft:pig")).get()), Boolean.toString(BuiltInRegistries.ENTITY_TYPE.getOptional(id("minecraft:zombie")).isPresent())); }
 public static String othermod(){ return Boolean.toString(BuiltInRegistries.ENTITY_TYPE.getOptional(id("othermod:thing")).isPresent()); }
}

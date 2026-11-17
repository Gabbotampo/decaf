package demo;
import net.minecraft.resources.Identifier;
public final class Counted { public static String run() { return Boolean.toString(Identifier.CODEC.fieldOf("id") != null); } }

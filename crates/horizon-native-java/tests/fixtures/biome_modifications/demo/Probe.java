package demo;
import net.fabricmc.fabric.api.biome.v1.*; import net.minecraft.resources.Identifier;
public class Probe { public static void main(String[] x){System.out.print(rows());} public static String rows(){ Identifier id=Identifier.parse("demo:test"); BiomeModification a=BiomeModifications.create(id), b=BiomeModifications.create(id); BiomeModification c=a.add(ModificationPhase.ADDITIONS,s -> true,(s,x)->{}); return (a!=b)+"|"+(c==a)+"|"+(a instanceof BiomeModification); }}

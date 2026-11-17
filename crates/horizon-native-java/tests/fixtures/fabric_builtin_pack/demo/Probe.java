package demo;
import net.fabricmc.fabric.api.resource.v1.ResourceLoader;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;
import net.fabricmc.fabric.api.resource.v1.pack.PackActivationType;
import net.minecraft.resources.Identifier;
public final class Probe {
  public static String rows() {
    ModContainer c = FabricLoader.getInstance().getModContainer("demo").get();
    StringBuilder b = new StringBuilder();
    b.append("present=").append(ResourceLoader.registerBuiltinPack(Identifier.parse("demo:present"), c, null, null)).append('\n');
    b.append("absent=").append(ResourceLoader.registerBuiltinPack(Identifier.parse("demo:absent"), c, null, null)).append('\n');
    return b.toString();
  }
}

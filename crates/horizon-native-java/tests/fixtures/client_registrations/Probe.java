package demo;
import net.minecraft.resources.Identifier;
import net.minecraft.core.particles.*;
import net.fabricmc.fabric.api.particle.v1.FabricParticleTypes;
import net.fabricmc.fabric.api.client.particle.v1.ParticleProviderRegistry;
import net.minecraft.client.gui.components.debug.*;
import net.minecraft.client.gui.screens.MenuScreens;
import net.fabricmc.fabric.api.client.rendering.v1.ModelLayerRegistry;

public class Probe {
  public static String identities() {
    ParticleType<?> a = FabricParticleTypes.simple();
    ParticleType<?> b = FabricParticleTypes.simple();
    ParticleProviderRegistry r1 = ParticleProviderRegistry.getInstance();
    ParticleProviderRegistry r2 = ParticleProviderRegistry.getInstance();
    r1.register(a, null);
    return (a == b) + "|" + (r1 == r2) + "|" + (DebugScreenEntries.FPS == DebugScreenEntries.FPS)
        + "|" + (DebugScreenProfile.DEFAULT == DebugScreenProfile.DEFAULT);
  }
  public static String duplicates() {
    try {
      net.minecraft.client.model.geom.ModelLayerLocation loc = new net.minecraft.client.model.geom.ModelLayerLocation(Identifier.withDefaultNamespace("x"), "main");
      ModelLayerRegistry.registerModelLayer(loc, new Object());
      ModelLayerRegistry.registerModelLayer(loc, new Object());
      return "none";
    } catch (IllegalArgumentException e) { return e.getClass().getSimpleName(); }
  }
  public static String fields() {
    return (DebugScreenEntries.FPS != null) + "|" + (net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements.CHAT != null);
  }
}

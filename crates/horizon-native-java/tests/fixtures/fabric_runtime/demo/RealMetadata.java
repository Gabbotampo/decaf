package demo;
import java.util.*;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.metadata.ModDependency;
import net.fabricmc.loader.api.metadata.ModMetadata;
public final class RealMetadata implements net.fabricmc.api.ModInitializer {
  public void onInitialize() {}
  public static String inspect() {
    ModMetadata m = FabricLoader.getInstance().getModContainer("probe-fixture").get().getMetadata();
    Collection authors=m.getAuthors(), contributors=m.getContributors(), license=m.getLicense(), provides=m.getProvides(), deps=m.getDependencies();
    StringBuilder out=new StringBuilder();
    out.append(authors.getClass().getName()).append('|').append(contributors.getClass().getName()).append('|').append(license.getClass().getName()).append('|').append(provides.getClass().getName()).append('|').append(deps.getClass().getName());
    out.append('|').append(authors==m.getAuthors()).append('|').append(deps==m.getDependencies());
    try { authors.add("x"); out.append("|ok"); } catch (UnsupportedOperationException e) { out.append("|uoe:").append(e.getMessage()); }
    try { deps.iterator().remove(); out.append("|ok"); } catch (UnsupportedOperationException e) { out.append("|iter-uoe:").append(e.getMessage()); }
    out.append('|');
    for (ModDependency d: m.getDependencies()) out.append(d.getKind().name()).append(',');
    return out.toString();
  }
}

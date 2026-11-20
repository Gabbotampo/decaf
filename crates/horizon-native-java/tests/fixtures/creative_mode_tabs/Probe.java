package demo;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.CreativeModeTabs;
import net.minecraft.resources.ResourceKey;
public final class Probe {
  public static String rows() {
    ResourceKey<CreativeModeTab> functional = CreativeModeTabs.FUNCTIONAL_BLOCKS;
    ResourceKey<CreativeModeTab> combat = CreativeModeTabs.COMBAT;
    return "functional=" + functional.identifier() + "\n"
        + "combat=" + combat.identifier() + "\n"
        + "same=" + (functional == CreativeModeTabs.FUNCTIONAL_BLOCKS) + "\n"
        + "different=" + functional.equals(combat) + "\n";
  }
  public static void main(String[] args) { System.out.print(rows()); }
}

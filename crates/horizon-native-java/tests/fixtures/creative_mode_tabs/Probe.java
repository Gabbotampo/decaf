package demo;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.CreativeModeTabs;
import net.minecraft.resources.ResourceKey;
public final class Probe {
  public static String rows() {
    ResourceKey<CreativeModeTab> functional = CreativeModeTabs.FUNCTIONAL_BLOCKS;
    ResourceKey<CreativeModeTab> combat = CreativeModeTabs.COMBAT;
    return "functional=" + functional.location() + "\n"
        + "combat=" + combat.location() + "\n"
        + "same=" + (functional == CreativeModeTabs.FUNCTIONAL_BLOCKS) + "\n"
        + "different=" + functional.equals(combat) + "\n";
  }
  public static void main(String[] args) { System.out.print(rows()); }
}

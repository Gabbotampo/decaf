import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.BundleItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.MapItem;

/**
 * The vanilla items whose class overrides the click-stacking hooks (BundleItem) or
 * onCraftedPostProcess (MapItem) in 26.3: live_menu.rs's BUNDLE_ITEMS and
 * CRAFTED_POST_PROCESS_ITEMS. (javap over the server jar shows no other Item subclass declares
 * overrideStackedOnOther, overrideOtherStackedOnMe, onCraftedBy or onCraftedPostProcess.)
 */
public final class CaptureItemHooks {
  public static void main(String[] args) {
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    for (Item item : BuiltInRegistries.ITEM) {
      if (item instanceof BundleItem) {
        System.out.println("bundle " + BuiltInRegistries.ITEM.getKey(item));
      }
    }
    for (Item item : BuiltInRegistries.ITEM) {
      if (item instanceof MapItem) {
        System.out.println("map " + BuiltInRegistries.ITEM.getKey(item));
      }
    }
  }
}

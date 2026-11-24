package demo;

import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.inventory.MenuType;

/** The bench's MenuType (Fabric's access widener makes this constructor public for mods). */
final class Types {
  static MenuType<BenchMenu> bench() {
    return new MenuType<>(BenchMenu::new, FeatureFlags.VANILLA_SET);
  }
}

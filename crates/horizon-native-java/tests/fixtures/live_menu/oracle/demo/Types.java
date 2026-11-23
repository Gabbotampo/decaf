package demo;

import net.minecraft.world.inventory.MenuType;

/**
 * The oracle's stand-in for types/Types.java: the plain server jar keeps MenuType's
 * constructor private, and nothing the oracle runs reads the menu's type.
 */
final class Types {
  static MenuType<BenchMenu> bench() {
    return null;
  }
}

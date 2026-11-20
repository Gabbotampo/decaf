package oracle;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/**
 * Lists the vanilla 26.3 block classes (directly in net/minecraft/world/level/block) whose state
 * definition is not Block's own empty one: the class or a superclass below Block declares
 * createBlockStateDefinition(StateDefinition.Builder). Prints "Class|DeclaringClass" per row,
 * sorted. A class not listed (Block, CraftingTableBlock, ...) inherits Block's, which adds nothing.
 * Reads class names (internal form) from args[0]; loads without initializing.
 */
public final class CaptureStateDefiners {
  public static void main(String[] args) throws Exception {
    ClassLoader loader = CaptureStateDefiners.class.getClassLoader();
    Class<?> block = Class.forName("net.minecraft.world.level.block.Block", false, loader);
    Class<?> builder = Class.forName("net.minecraft.world.level.block.state.StateDefinition$Builder", false, loader);
    List<String> rows = new ArrayList<>();
    for (String name : Files.readAllLines(Path.of(args[0]))) {
      Class<?> c = Class.forName(name.replace('/', '.'), false, loader);
      if (c.isInterface() || !block.isAssignableFrom(c) || c == block) continue;
      for (Class<?> k = c; k != null && k != block; k = k.getSuperclass()) {
        try {
          k.getDeclaredMethod("createBlockStateDefinition", builder);
          rows.add(name + "|" + k.getName().replace('.', '/'));
          break;
        } catch (NoSuchMethodException none) {
          // keep walking up
        }
      }
    }
    rows.sort(null);
    rows.forEach(System.out::println);
  }
}

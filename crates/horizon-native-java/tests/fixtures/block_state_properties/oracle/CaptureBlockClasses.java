package oracle;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/**
 * Every vanilla 26.3 class (top-level, any package) that is Block or a subclass, with the class
 * below Block that declares createBlockStateDefinition, if any: "Class|Declaring" or "Class|".
 * Reads internal class names from args[0]; loads without initializing.
 */
public final class CaptureBlockClasses {
  public static void main(String[] args) throws Exception {
    ClassLoader loader = CaptureBlockClasses.class.getClassLoader();
    Class<?> block = Class.forName("net.minecraft.world.level.block.Block", false, loader);
    Class<?> builder = Class.forName("net.minecraft.world.level.block.state.StateDefinition$Builder", false, loader);
    List<String> rows = new ArrayList<>();
    for (String name : Files.readAllLines(Path.of(args[0]))) {
      Class<?> c;
      try {
        c = Class.forName(name.replace('/', '.'), false, loader);
      } catch (Throwable unloadable) {
        System.err.println("unloadable " + name + ": " + unloadable);
        continue;
      }
      if (c.isInterface() || !block.isAssignableFrom(c)) continue;
      String declaring = "";
      for (Class<?> k = c; k != null && k != block; k = k.getSuperclass()) {
        try {
          k.getDeclaredMethod("createBlockStateDefinition", builder);
          declaring = k.getName().replace('.', '/');
          break;
        } catch (NoSuchMethodException none) {
          // keep walking up
        }
      }
      rows.add(name + "|" + declaring);
    }
    rows.sort(null);
    rows.forEach(System.out::println);
  }
}

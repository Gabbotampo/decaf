import java.lang.reflect.Field;
public class CaptureTabs { public static void main(String[] a) throws Exception {
  Class<?> tabs = Class.forName("net.minecraft.world.item.CreativeModeTabs");
  Field f = tabs.getDeclaredField("FUNCTIONAL_BLOCKS"); f.setAccessible(true);
  Field c = tabs.getDeclaredField("COMBAT"); c.setAccessible(true);
  Object functional = f.get(null), combat = c.get(null);
  java.lang.reflect.Method id = functional.getClass().getMethod("identifier");
  System.out.print("functional=" + id.invoke(functional) + "\ncombat=" + id.invoke(combat) + "\nsame=" + (functional == f.get(null)) + "\ndifferent=" + functional.equals(combat) + "\n");
}}

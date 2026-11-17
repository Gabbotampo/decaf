package demo;
import java.lang.reflect.*;
import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.Item;
import net.minecraft.core.registries.BuiltInRegistries;
public final class ItemsCapture {
  public static void main(String[] args) throws Exception {
    SharedConstants.tryDetectVersion(); Bootstrap.bootStrap();
    for (Field f : Items.class.getDeclaredFields())
      if (Modifier.isStatic(f.getModifiers()) && f.getType() == Item.class)
        System.out.println(f.getName()+"|"+BuiltInRegistries.ITEM.getKey((Item)f.get(null)));
  }
}

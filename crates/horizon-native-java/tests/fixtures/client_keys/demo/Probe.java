package demo;
import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.KeyMapping;
import net.minecraft.world.item.ItemStack;
import net.minecraft.client.Minecraft;
public final class Probe {
  public static String run() {
    var u=InputConstants.UNKNOWN; var a=InputConstants.Type.KEYBOARD.getOrCreate(4); var b=InputConstants.getKey("key.keyboard.a"); var m=InputConstants.getKey("key.mouse.left");
    var x=new KeyMapping("x", InputConstants.Type.KEYBOARD, 4, KeyMapping.Category.MISC); var y=new KeyMapping("y", InputConstants.Type.KEYBOARD, 4, KeyMapping.Category.MISC);
    return "unknown="+u.getType().name()+"/"+u.getValue()+"/"+u.getName()+"/"+u.toString()+"\n"+
      "a="+a.getType().name()+"/"+a.getValue()+"/"+a.getName()+"/"+a.toString()+"/"+a.equals(b)+"\n"+
      "mouse="+m.getType().name()+"/"+m.getValue()+"/"+m.getName()+"/"+m.toString()+"\n"+
      "mapping="+x.getDefaultKey().getName()+"/"+x.getKey().getName()+"/"+x.same(y)+"\n"+
      "dir="+Minecraft.getInstance().gameDirectory.getPath();
  }
  public static String bad() { try { InputConstants.getKey("key.scancode.4"); return "none"; } catch (RuntimeException e) { return e.getClass().getName()+"|"+e.getMessage(); } }
}

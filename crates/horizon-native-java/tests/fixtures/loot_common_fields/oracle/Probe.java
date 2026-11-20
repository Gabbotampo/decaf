import net.minecraft.world.level.storage.loot.functions.LootItemConditionalFunction;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.storage.loot.LootContext;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import com.mojang.datafixers.Products;
public final class Probe extends LootItemConditionalFunction {
  Probe(){super(java.util.Optional.empty());}
  public com.mojang.serialization.MapCodec<? extends LootItemConditionalFunction> codec(){return null;}
  static String shape() {
    net.minecraft.SharedConstants.tryDetectVersion();
    net.minecraft.server.Bootstrap.bootStrap();
    Products.P1<?,?> p = commonFields(RecordCodecBuilder.instance());
    var c = p.getClass().getRecordComponents();
    return c.length + "|" + c[0].getName() + "|" + (p.t1() != null);
  }
  protected ItemStack run(ItemStack a,LootContext b){return a;}
  public static void main(String[] a){System.out.print(shape());}
}

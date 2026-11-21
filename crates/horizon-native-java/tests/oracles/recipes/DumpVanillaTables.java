import net.minecraft.SharedConstants;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import java.util.*;

public final class DumpVanillaTables {
    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        HolderLookup.Provider vanilla = net.minecraft.data.registries.VanillaRegistries.createWorldLookup();
        for (var pending : BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(vanilla)) {
            pending.apply();
        }
        int total = 0, non64 = 0;
        Map<Integer, Integer> hist = new TreeMap<>();
        for (var holder : BuiltInRegistries.ITEM.listElements().toList()) {
            total++;
            int max = holder.components().getOrDefault(DataComponents.MAX_STACK_SIZE, 64);
            hist.merge(max, 1, Integer::sum);
            if (max != 64) non64++;
        }
        System.out.println("total_items=" + total + " non64=" + non64 + " hist=" + hist);
    }
}

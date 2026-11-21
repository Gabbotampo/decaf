import java.util.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;

public final class DumpVanillaTables {
    public static void main(String[] args) {
        java.io.PrintStream out = System.out;
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        HolderLookup.Provider vanilla = net.minecraft.data.registries.VanillaRegistries.createWorldLookup();
        for (var pending : BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(vanilla)) {
            pending.apply();
        }
        int total = 0, non64 = 0;
        Map<Integer, Integer> hist = new TreeMap<>();
        Map<String, Integer> maxDamages = new TreeMap<>();
        for (var holder : BuiltInRegistries.ITEM.listElements().toList()) {
            total++;
            String id = holder.key().identifier().toString();
            int max = holder.components().getOrDefault(DataComponents.MAX_STACK_SIZE, 64);
            hist.merge(max, 1, Integer::sum);
            if (max != 64) non64++;
            Integer maxDamage = holder.components().get(DataComponents.MAX_DAMAGE);
            if (maxDamage != null) {
                maxDamages.put(id, maxDamage);
            }
        }
        out.println("total_items=" + total + " non64=" + non64 + " hist=" + hist + " maxDamage=" + maxDamages.size());
        if (args.length > 0 && "--rust-extra".equals(args[0])) {
            out.println("const VANILLA_MAX_DAMAGE: &[(&str, i32)] = &[");
            for (var e : maxDamages.entrySet()) {
                out.println("    (\"" + e.getKey() + "\", " + e.getValue() + "),");
            }
            out.println("];");
            out.println("const VANILLA_RECIPE_SERIALIZERS: &[&str] = &[");
            BuiltInRegistries.RECIPE_SERIALIZER.keySet().stream()
                .map(Object::toString).sorted()
                .forEach(s -> out.println("    \"" + s + "\","));
            out.println("];");
            out.println("const VANILLA_RECIPE_TYPES: &[&str] = &[");
            BuiltInRegistries.RECIPE_TYPE.keySet().stream()
                .map(Object::toString).sorted()
                .forEach(s -> out.println("    \"" + s + "\","));
            out.println("];");
        }
    }
}

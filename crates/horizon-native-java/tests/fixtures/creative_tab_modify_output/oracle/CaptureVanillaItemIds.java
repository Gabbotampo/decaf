import java.io.PrintStream;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.Item;

/**
 * Captures the complete sorted `BuiltInRegistries.ITEM.keySet()` from Minecraft 26.3 on Java 25,
 * and asserts that every vanilla item requires `FeatureFlags.VANILLA_SET`.
 * Output is written verbatim to `crates/horizon-native-java/src/vanilla_item_ids_java25.txt`.
 */
public final class CaptureVanillaItemIds {
    public static void main(String[] args) {
        PrintStream rawOut = System.out;
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();

        List<String> ids = new ArrayList<>();
        int nonVanilla = 0;
        for (Identifier id : BuiltInRegistries.ITEM.keySet()) {
            Item item = BuiltInRegistries.ITEM.getValue(id);
            if (!item.requiredFeatures().equals(FeatureFlags.VANILLA_SET)) {
                nonVanilla++;
                System.err.println("Non-VANILLA requiredFeatures on " + id + ": " + item.requiredFeatures());
            }
            ids.add(id.toString());
        }
        if (nonVanilla != 0) {
            throw new AssertionError("Found " + nonVanilla + " non-VANILLA vanilla items");
        }
        Collections.sort(ids);
        StringBuilder sb = new StringBuilder(ids.size() * 32);
        for (String id : ids) {
            sb.append(id).append('\n');
        }
        rawOut.print(sb);
    }
}

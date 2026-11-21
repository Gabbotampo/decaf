package orderprobe;

import java.io.IOException;
import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.impl.resource.pack.ModPackResourcesSorter;
import net.fabricmc.fabric.impl.resource.pack.ModPackResourcesUtil;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.resources.Identifier;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.packs.PackResources;
import net.minecraft.server.packs.PackType;
import net.minecraft.server.packs.resources.Resource;

/**
 * Real Fabric 26.3 (fabric-resource-loader-v1 3.1.0 + server-26.3.jar) oracle for mod data-pack
 * ordering. Booted as a mod on KnotServer with test mods exercising:
 * - {@code fabric:resource_load_order} {@code before} ({@code mod_a} before {@code mod_c})
 * - {@code fabric:resource_load_order} {@code after} ({@code mod_z} after {@code mod_x})
 * - cycle breaking ({@code cyc_a} <-> {@code cyc_b})
 * - malformed {@code fabric:resource_load_order} ({@code bad_form})
 * - nested JiJ mods ({@code nested_zzz} in {@code mod_a}, {@code nested_aaa} in {@code mod_c})
 * - empty {@code data/<ns>/} directory bridge ({@code empty_bridge} with empty {@code data/empty_ns/}
 *   plus {@code before: ["emp_z"], after: ["emp_a"]} flipping {@code emp_z} ahead of {@code emp_a})
 * - invalid/non-directory {@code data/} entries ({@code empty_invalid} with {@code data/Bad_NS/}
 *   and bare file {@code data/plain_file.txt} plus {@code before: ["inv_z"], after: ["inv_a"]},
 *   excluded by {@code ModNioPackResources.readNamespaces} so {@code inv_a} stays before {@code inv_z})
 * - world datapack ({@code file/dp_override})
 * - self-dependency exception in {@link ModPackResourcesSorter#addLoadOrdering}
 *
 * Writes its captured lines to the path in system property {@code orderprobe.out} (or stdout),
 * then halts the server cleanly.
 */
public final class FabricPackOrderOracle implements ModInitializer {
    @Override
    public void onInitialize() {
        ServerLifecycleEvents.SERVER_STARTING.register((MinecraftServer server) -> {
            try {
                List<String> lines = new ArrayList<>();

                var sorted = ModPackResourcesUtil.getModResourcePacks(
                    FabricLoader.getInstance(), PackType.SERVER_DATA, null);
                List<String> sortedIds = sorted.stream()
                    .map(PackResources::packId)
                    .filter(id -> !id.startsWith("fabric-"))
                    .toList();
                List<String> rmPackIds = server.getResourceManager().listPacks()
                    .map(PackResources::packId)
                    .filter(id -> !id.startsWith("fabric-"))
                    .toList();

                String recBefore = server.getResourceManager()
                    .getResource(Identifier.parse("orderprobe:recipe/shared_before.json"))
                    .map(Resource::sourcePackId)
                    .orElse("NONE");
                List<String> tagBefore = server.getResourceManager()
                    .getResourceStack(Identifier.parse("orderprobe:tags/item/shared_before.json"))
                    .stream().map(Resource::sourcePackId).toList();

                String recAfter = server.getResourceManager()
                    .getResource(Identifier.parse("orderprobe:recipe/shared_after.json"))
                    .map(Resource::sourcePackId)
                    .orElse("NONE");
                List<String> tagAfter = server.getResourceManager()
                    .getResourceStack(Identifier.parse("orderprobe:tags/item/shared_after.json"))
                    .stream().map(Resource::sourcePackId).toList();

                String recNested = server.getResourceManager()
                    .getResource(Identifier.parse("orderprobe:recipe/shared_nested.json"))
                    .map(Resource::sourcePackId)
                    .orElse("NONE");
                List<String> tagNested = server.getResourceManager()
                    .getResourceStack(Identifier.parse("orderprobe:tags/item/shared_nested.json"))
                    .stream().map(Resource::sourcePackId).toList();

                String recDp = server.getResourceManager()
                    .getResource(Identifier.parse("orderprobe:recipe/shared_dp.json"))
                    .map(Resource::sourcePackId)
                    .orElse("NONE");
                List<String> tagDp = server.getResourceManager()
                    .getResourceStack(Identifier.parse("orderprobe:tags/item/shared_dp.json"))
                    .stream().map(Resource::sourcePackId).toList();

                String selfDep = "NO_EXCEPTION";
                try {
                    Constructor<ModPackResourcesSorter> ctor =
                        ModPackResourcesSorter.class.getDeclaredConstructor();
                    ctor.setAccessible(true);
                    ModPackResourcesSorter sorter = ctor.newInstance();
                    Method add = ModPackResourcesSorter.class.getMethod(
                        "addLoadOrdering",
                        String.class,
                        String.class,
                        ModPackResourcesUtil.Order.class);
                    add.invoke(sorter, "self_mod", "self_mod", ModPackResourcesUtil.Order.BEFORE);
                } catch (InvocationTargetException ite) {
                    Throwable cause = ite.getCause();
                    selfDep = cause.getClass().getName() + ": " + cause.getMessage();
                }

                List<String> emptyBridgeNs = sorted.stream()
                    .filter(p -> "empty_bridge".equals(p.packId()))
                    .flatMap(p -> p.getNamespaces(PackType.SERVER_DATA).stream())
                    .sorted()
                    .toList();

                lines.add("SORTER_MOD_ORDER=" + sortedIds);
                lines.add("RM_PACK_ORDER=" + rmPackIds);
                lines.add("EMPTY_BRIDGE_NAMESPACES=" + emptyBridgeNs);
                lines.add("BEFORE_RECIPE_WINNER=" + recBefore);
                lines.add("BEFORE_TAG_STACK=" + tagBefore);
                lines.add("AFTER_RECIPE_WINNER=" + recAfter);
                lines.add("AFTER_TAG_STACK=" + tagAfter);
                lines.add("NESTED_RECIPE_WINNER=" + recNested);
                lines.add("NESTED_TAG_STACK=" + tagNested);
                lines.add("DP_RECIPE_WINNER=" + recDp);
                lines.add("DP_TAG_STACK=" + tagDp);
                lines.add("SELF_DEP=" + selfDep);

                String body = String.join("\n", lines) + "\n";
                String outPath = System.getProperty("orderprobe.out");
                if (outPath != null && !outPath.isBlank()) {
                    Files.writeString(Path.of(outPath), body, StandardCharsets.UTF_8);
                } else {
                    System.out.print(body);
                    System.out.flush();
                }
            } catch (Throwable t) {
                t.printStackTrace();
                Runtime.getRuntime().halt(1);
            } finally {
                Runtime.getRuntime().halt(0);
            }
        });
    }
}

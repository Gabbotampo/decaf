package demo;

import java.nio.file.Path;
import java.util.Collection;
import java.util.List;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;

/** A mod container's root paths held across collections (Fabric: the container's own field). */
public final class Roots {
    static List<Path> held;
    static String before;
    static Collection<String> license;

    static ModContainer mod() {
        return FabricLoader.getInstance().getModContainer("moda").get();
    }

    public static String hold() {
        held = mod().getRootPaths();
        before = held.get(0).toString();
        license = mod().getMetadata().getLicense();
        return "ok";
    }

    public static String check() {
        List<Path> now = mod().getRootPaths();
        return (now == held) + "|" + held.get(0) + "|" + before.equals(now.get(0).toString()) + "|"
            + held.size() + "|" + (license == mod().getMetadata().getLicense());
    }
}

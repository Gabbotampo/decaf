package dev.horizonlab.parity;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashSet;
import java.util.Set;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;

/**
 * Exports exact internal-name superclass/interface closures for every 26.3
 * vanilla block. Run this against the player's own 26.3 server jar and its
 * libraries; the output is prepared input, never a repository data dump.
 */
public final class VanillaBlockClassClosureExporter {
    private static void add(Class<?> type, Set<Class<?>> closure) {
        if (type == null || !closure.add(type)) {
            return;
        }
        for (Class<?> iface : type.getInterfaces()) {
            add(iface, closure);
        }
        add(type.getSuperclass(), closure);
    }

    private static JsonArray closure(Block block) {
        Set<Class<?>> classes = new LinkedHashSet<>();
        add(block.getClass(), classes);
        JsonArray result = new JsonArray();
        for (Class<?> type : classes) {
            result.add(type.getName().replace('.', '/'));
        }
        return result;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            throw new IllegalArgumentException("usage: VanillaBlockClassClosureExporter <output.json>");
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        JsonObject blocks = new JsonObject();
        for (Block block : BuiltInRegistries.BLOCK) {
            blocks.add(BuiltInRegistries.BLOCK.getKey(block).toString(), closure(block));
        }
        JsonObject result = new JsonObject();
        result.add("blocks", blocks);
        Files.writeString(Path.of(args[0]), new Gson().toJson(result), StandardCharsets.UTF_8);
    }
}

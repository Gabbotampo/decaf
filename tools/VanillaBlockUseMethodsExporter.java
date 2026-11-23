import java.io.IOException;
import java.lang.classfile.ClassFile;
import java.lang.classfile.ClassModel;
import java.lang.classfile.MethodModel;
import java.lang.reflect.AccessFlag;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.zip.ZipFile;

/**
 * Prints the rows of {@code crates/horizon-native-java/src/vanilla_block_use_methods.rs}:
 * every class of a 26.3 server jar that extends {@code BlockBehaviour} (itself included), its
 * superclass, and which of the three block-use methods it declares itself.
 *
 * <p>Only names and declaration bits are kept, no code. Run it with a JDK 25 on the player's
 * own unpacked server jar ({@code META-INF/versions/26.3/server-26.3.jar}):
 *
 * <pre>java tools/VanillaBlockUseMethodsExporter.java server-26.3.jar</pre>
 *
 * and paste the output between the table's markers. Decaf answers an inherited
 * {@code useItemOn}/{@code useWithoutItem}/{@code getMenuProvider} only when the class that
 * declares it on this chain is {@code BlockBehaviour}, whose bodies it models.
 */
public final class VanillaBlockUseMethodsExporter {
    private static final String BLOCK_BEHAVIOUR =
            "net/minecraft/world/level/block/state/BlockBehaviour";

    /** Bit, method name and exact descriptor, in the order the Rust table spells them. */
    private static final String[][] METHODS = {
        {"1", "useItemOn",
            "(Lnet/minecraft/world/item/ItemStack;Lnet/minecraft/world/level/block/state/BlockState;"
                + "Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;"
                + "Lnet/minecraft/world/entity/player/Player;Lnet/minecraft/world/InteractionHand;"
                + "Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;"},
        {"2", "useWithoutItem",
            "(Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/Level;"
                + "Lnet/minecraft/core/BlockPos;Lnet/minecraft/world/entity/player/Player;"
                + "Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;"},
        {"4", "getMenuProvider",
            "(Lnet/minecraft/world/level/block/state/BlockState;Lnet/minecraft/world/level/Level;"
                + "Lnet/minecraft/core/BlockPos;)Lnet/minecraft/world/MenuProvider;"},
    };

    public static void main(String[] args) throws IOException {
        if (args.length != 1) {
            System.err.println("usage: java VanillaBlockUseMethodsExporter.java <server-26.3.jar>");
            System.exit(2);
        }
        Map<String, String> superOf = new HashMap<>();
        Map<String, Integer> declared = new HashMap<>();
        try (ZipFile zip = new ZipFile(Path.of(args[0]).toFile())) {
            var entries = zip.entries();
            while (entries.hasMoreElements()) {
                var entry = entries.nextElement();
                if (!entry.getName().endsWith(".class")) {
                    continue;
                }
                byte[] bytes;
                try (var in = zip.getInputStream(entry)) {
                    bytes = in.readAllBytes();
                }
                ClassModel model = ClassFile.of().parse(bytes);
                String name = model.thisClass().asInternalName();
                superOf.put(name, model.superclass().map(c -> c.asInternalName()).orElse(""));
                int bits = 0;
                for (MethodModel method : model.methods()) {
                    if (method.flags().has(AccessFlag.STATIC)
                            || method.flags().has(AccessFlag.PRIVATE)) {
                        continue;
                    }
                    for (String[] wanted : METHODS) {
                        if (method.methodName().equalsString(wanted[1])
                                && method.methodType().equalsString(wanted[2])) {
                            bits |= Integer.parseInt(wanted[0]);
                        }
                    }
                }
                declared.put(name, bits);
            }
        }
        TreeMap<String, String> rows = new TreeMap<>();
        for (String name : superOf.keySet()) {
            List<String> chain = new ArrayList<>();
            String current = name;
            while (current != null && !current.isEmpty() && !current.equals(BLOCK_BEHAVIOUR)) {
                chain.add(current);
                current = superOf.get(current);
            }
            if (BLOCK_BEHAVIOUR.equals(current)) {
                rows.put(name, "    (\"" + name + "\", \"" + superOf.get(name) + "\", "
                        + declared.get(name) + "),");
            }
        }
        rows.put(BLOCK_BEHAVIOUR, "    (\"" + BLOCK_BEHAVIOUR + "\", \"\", "
                + declared.get(BLOCK_BEHAVIOUR) + "),");
        // Rust's `binary_search_by` compares `&str` bytewise, as `TreeMap<String>` orders.
        rows.values().forEach(System.out::println);
    }
}

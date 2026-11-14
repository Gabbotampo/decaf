package demo;

import java.util.*;
import net.minecraft.core.registries.Registries;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;

/** A register's getEntries() view held across forced collections (#182, #226). */
@SuppressWarnings({"rawtypes", "unchecked"})
public final class HeapRegister {
    static DeferredRegister register;
    static Collection view;
    static DeferredHolder x;

    public static String setup() {
        register = DeferredRegister.create(Registries.ITEM, "demo");
        view = register.getEntries();
        x = register.register("x", () -> "v");
        register.register("y", () -> "w");
        return "ok";
    }

    /** Lists and maps nothing keeps. */
    public static String garbage() {
        int s = 0;
        for (int i = 0; i < 300; i++) {
            List<Object> l = new ArrayList<>();
            l.add(new Object());
            Map<String, Integer> m = new HashMap<>();
            m.put("k" + i, i);
            s += l.size() + m.size();
        }
        return "g" + (s & 1);
    }

    public static String check() {
        return "" + (view == register.getEntries()) + view.size() + view.contains(x)
            + (view.iterator().next() == x) + view;
    }

    public static void main(String[] args) {
        net.minecraft.SharedConstants.tryDetectVersion();
        net.minecraft.server.Bootstrap.bootStrap();
        System.out.println("setup=" + setup());
        for (int i = 0; i < 40; i++) garbage();
        System.out.println("check=" + check());
    }
}

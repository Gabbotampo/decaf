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
    static Set<Object> t1;
    static Set<Object> t2;
    static Set<Object> u1;

    public static String setup() {
        register = DeferredRegister.create(Registries.ITEM, "demo");
        view = register.getEntries();
        x = register.register("x", () -> "v");
        register.register("y", () -> "w");
        t1 = Collections.unmodifiableSet(new HashSet<>(Set.of("t1")));
        t2 = Collections.unmodifiableSet(new HashSet<>(Set.of("t2")));
        u1 = Collections.unmodifiableSet(new HashSet<>(Set.of("u1")));
        return "ok";
    }

    /** The two temporary views become garbage. */
    public static String drop() {
        t1 = null;
        t2 = null;
        return "ok";
    }

    /** New views and a new register after collections: the held ones read what they did (#243 review B1). */
    public static String after() {
        DeferredRegister r2 = DeferredRegister.create(Registries.ITEM, "demo2");
        r2.register("b", () -> "v");
        r2.register("c", () -> "v");
        Collection v2 = r2.getEntries();
        Set<Object> n1 = Collections.unmodifiableSet(new HashSet<>(Set.of("n1")));
        Set<Object> n2 = Collections.unmodifiableSet(new HashSet<>(Set.of("n2")));
        return "" + view + (view.iterator().next() == x) + (view == register.getEntries()) + u1
            + "|" + v2.size() + n1 + n2;
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
        System.out.println("drop=" + drop());
        for (int i = 0; i < 40; i++) garbage();
        System.out.println("check=" + check());
        System.out.println("after=" + after());
    }
}

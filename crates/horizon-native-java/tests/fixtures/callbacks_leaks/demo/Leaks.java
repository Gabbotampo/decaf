package demo;

import java.util.*;
import java.util.function.*;

/**
 * Every text a jar can make from a callback or a structure holding one (#261 re-review): none
 * may carry the runtime's key text for it. Each case prints a value through every path and
 * answers whether any output names a key ({@code @indy}, {@code @fn}, {@code @cmp}, {@code cb:}).
 */
public final class Leaks {
    static boolean leaks(String s) {
        return s != null && (s.contains("@indy") || s.contains("@fn") || s.contains("@cmp") || s.contains("cb:"));
    }

    static String scan(Object v) {
        List<String> outs = new ArrayList<>();
        outs.add("" + v);
        outs.add(String.valueOf(v));
        outs.add(Objects.toString(v));
        outs.add(new StringBuilder().append(v).toString());
        outs.add(String.format("%s", v));
        outs.add(List.of(v).toString());
        outs.add(new ArrayList<>(List.of(v)).toString());
        outs.add(Map.of("k", v).toString());
        outs.add(Arrays.toString(new Object[] {v}));
        outs.add(Optional.of(v).toString());
        outs.add(String.join(",", String.valueOf(v), "x"));
        for (int i = 0; i < outs.size(); i++) if (leaks(outs.get(i))) return "LEAK" + i;
        return "ok";
    }

    public static String run() {
        String cap = "c";
        Predicate<String> p = s -> s.isEmpty();
        Object[] values = {
            (Runnable) () -> {},
            (Supplier<String>) () -> cap,
            (Function<String, Integer>) String::length,
            p.and(s -> true),
            p.negate(),
            Function.identity(),
            Comparator.comparing((String s) -> s.length()),
            Comparator.comparing((String s) -> s.length()).reversed(),
            Comparator.<String>naturalOrder().thenComparing(Comparator.comparing(String::length)),
            Comparator.naturalOrder(),
            Comparator.reverseOrder(),
            Map.Entry.comparingByKey(),
            Map.entry("k", (Runnable) () -> {}),
            List.of((Runnable) () -> {}),
            Optional.of((Runnable) () -> {}),
            new HashMap<>(Map.of("k", (Runnable) () -> {})),
        };
        StringBuilder out = new StringBuilder();
        for (Object v : values) out.append(scan(v)).append(',');
        // An entry of a map holding a lambda, from entrySet().
        Map<String, Runnable> handlers = new HashMap<>();
        handlers.put("a", () -> {});
        for (Map.Entry<String, Runnable> e : handlers.entrySet()) out.append(scan(e));
        return out.toString();
    }

    public static void main(String[] args) {
        System.out.println(run());
    }
}

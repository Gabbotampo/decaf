package demo;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Collections models make are objects of their own, of the class Java makes, and they live as long as something
 * holds them (#156 P14d2): `Map.ofEntries`, `List.copyOf`/`Set.copyOf`, `Stream.toList()`, `Collectors.toList()`,
 * `Collectors.toUnmodifiableList()`, a stream's iterator. `MapN`/`SetN` iterate in an order Java salts per run, so
 * only their lookups are printed.
 */
public final class Probe {
    static final List<Object> KEPT = new ArrayList<>();
    /**
     * A two-entry `Map.ofEntries` in a field of its own: adding one to a list counts `List.add` (a `MapN`'s
     * salted order, pre-existing on main), which this probe is not about.
     */
    static Map<String, Integer> two;

    static String uoe(Runnable step) {
        try {
            step.run();
            return "ok";
        } catch (UnsupportedOperationException e) {
            return "UOE";
        }
    }

    public static String setup() {
        KEPT.clear();
        two = Map.ofEntries(Map.entry("a", 1), Map.entry("b", 2));
        KEPT.add(Map.ofEntries(Map.entry("c", 3)));
        KEPT.add(List.copyOf(new ArrayList<>(List.of("x", "y", "z"))));
        KEPT.add(Set.copyOf(new ArrayList<>(List.of("p", "q", "p"))));
        KEPT.add(Stream.of(1, 2, 3).toList());
        KEPT.add(Stream.of(4, 5).collect(Collectors.toList()));
        KEPT.add(Stream.of(6).collect(Collectors.toUnmodifiableList()));
        Iterator<Integer> it = Stream.of(7, 8).iterator();
        it.next();
        KEPT.add(it);
        return "ok";
    }

    public static String garbage() {
        int sum = 0;
        for (int i = 0; i < 3000; i++) {
            int[] box = {i};
            sum += box[0];
        }
        return "g" + (sum & 1);
    }

    @SuppressWarnings("unchecked")
    public static String check() {
        Map<String, Integer> one = (Map<String, Integer>) KEPT.get(0);
        List<String> copy = (List<String>) KEPT.get(1);
        Set<String> set = (Set<String>) KEPT.get(2);
        List<Integer> toList = (List<Integer>) KEPT.get(3);
        List<Integer> collected = (List<Integer>) KEPT.get(4);
        List<Integer> unmodifiable = (List<Integer>) KEPT.get(5);
        Iterator<Integer> it = (Iterator<Integer>) KEPT.get(6);
        collected.add(9);
        return two.get("a") + "" + two.get("b") + two.size() + two.getClass().getName() + "|" + one + one.getClass().getName()
            + "|" + copy + copy.getClass().getName() + uoe(() -> copy.add("w")) + "|" + set.size() + set.contains("p")
            + set.contains("q") + set.getClass().getName() + "|" + toList + toList.getClass().getName()
            + uoe(() -> toList.add(4)) + "|" + collected + collected.getClass().getName() + "|" + unmodifiable
            + unmodifiable.getClass().getName() + uoe(() -> unmodifiable.add(1)) + "|" + it.next() + it.hasNext();
    }

    public static void main(String[] args) {
        System.out.println("setup=" + setup());
        for (int i = 0; i < 40; i++) {
            garbage();
        }
        System.out.println("check=" + check());
    }
}

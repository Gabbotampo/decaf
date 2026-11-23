package demo;

import java.util.*;
import java.util.concurrent.Callable;

/** A null key in a TreeMap/TreeSet with natural ordering, empty and holding "a". */
public final class Probe {
    static String of(Callable<?> f) {
        try {
            Object r = f.call();
            return String.valueOf(r);
        } catch (Throwable e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }

    static StringBuilder out = new StringBuilder();

    static void row(String label, Callable<?> f) {
        out.append(label).append('=').append(of(f)).append('\n');
    }

    static TreeMap<String, Integer> tmEmpty() { return new TreeMap<>(); }
    static TreeMap<String, Integer> tmFull() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("a", 1);
        return m;
    }
    static TreeSet<String> tsEmpty() { return new TreeSet<>(); }
    static TreeSet<String> tsFull() {
        TreeSet<String> s = new TreeSet<>();
        s.add("a");
        return s;
    }
    static List<String> nullFirst() { return Arrays.asList(null, "a"); }
    static List<String> nullSecond() { return Arrays.asList("a", null); }
    static Map<String, Integer> hashWithNull() {
        Map<String, Integer> h = new HashMap<>();
        h.put(null, 7);
        return h;
    }

    public static String rows() {
        out.setLength(0);
        // TreeMap, empty (E) and holding {a=1} (F), null key.
        for (int full = 0; full < 2; full++) {
            final boolean f = full == 1;
            String p = f ? "tmFull." : "tmEmpty.";
            row(p + "put", () -> (f ? tmFull() : tmEmpty()).put(null, 2));
            row(p + "putIfAbsent", () -> (f ? tmFull() : tmEmpty()).putIfAbsent(null, 2));
            row(p + "get", () -> (f ? tmFull() : tmEmpty()).get(null));
            row(p + "containsKey", () -> (f ? tmFull() : tmEmpty()).containsKey(null));
            row(p + "getOrDefault", () -> (f ? tmFull() : tmEmpty()).getOrDefault(null, 9));
            row(p + "remove", () -> (f ? tmFull() : tmEmpty()).remove(null));
            row(p + "remove2", () -> (f ? tmFull() : tmEmpty()).remove(null, 1));
            row(p + "replace", () -> (f ? tmFull() : tmEmpty()).replace(null, 5));
            row(p + "replace3", () -> (f ? tmFull() : tmEmpty()).replace(null, 1, 5));
            row(p + "ceilingKey", () -> (f ? tmFull() : tmEmpty()).ceilingKey(null));
            row(p + "floorKey", () -> (f ? tmFull() : tmEmpty()).floorKey(null));
            row(p + "higherKey", () -> (f ? tmFull() : tmEmpty()).higherKey(null));
            row(p + "lowerKey", () -> (f ? tmFull() : tmEmpty()).lowerKey(null));
            row(p + "ceilingEntry", () -> (f ? tmFull() : tmEmpty()).ceilingEntry(null));
            row(p + "floorEntry", () -> (f ? tmFull() : tmEmpty()).floorEntry(null));
            row(p + "higherEntry", () -> (f ? tmFull() : tmEmpty()).higherEntry(null));
            row(p + "lowerEntry", () -> (f ? tmFull() : tmEmpty()).lowerEntry(null));
            row(p + "headMap", () -> (f ? tmFull() : tmEmpty()).headMap(null));
            row(p + "tailMap", () -> (f ? tmFull() : tmEmpty()).tailMap(null));
            row(p + "computeIfAbsentValue", () -> (f ? tmFull() : tmEmpty()).computeIfAbsent(null, k -> 3));
            row(p + "computeIfAbsentNull", () -> (f ? tmFull() : tmEmpty()).computeIfAbsent(null, k -> null));
            row(p + "computeIfPresent", () -> (f ? tmFull() : tmEmpty()).computeIfPresent(null, (k, v) -> 3));
            row(p + "computeValue", () -> (f ? tmFull() : tmEmpty()).compute(null, (k, v) -> 3));
            row(p + "computeNull", () -> (f ? tmFull() : tmEmpty()).compute(null, (k, v) -> null));
            row(p + "merge", () -> (f ? tmFull() : tmEmpty()).merge(null, 3, Integer::sum));
            row(p + "putAll", () -> {
                TreeMap<String, Integer> m = f ? tmFull() : tmEmpty();
                m.putAll(hashWithNull());
                return m;
            });
            row(p + "firstKey", () -> (f ? tmFull() : tmEmpty()).firstKey());
            row(p + "lastKey", () -> (f ? tmFull() : tmEmpty()).lastKey());
            row(p + "pollFirstEntry", () -> (f ? tmFull() : tmEmpty()).pollFirstEntry());
        }
        // TreeSet, empty and holding {a}, null element.
        for (int full = 0; full < 2; full++) {
            final boolean f = full == 1;
            String p = f ? "tsFull." : "tsEmpty.";
            row(p + "add", () -> (f ? tsFull() : tsEmpty()).add(null));
            row(p + "contains", () -> (f ? tsFull() : tsEmpty()).contains(null));
            row(p + "remove", () -> (f ? tsFull() : tsEmpty()).remove(null));
            row(p + "ceiling", () -> (f ? tsFull() : tsEmpty()).ceiling(null));
            row(p + "floor", () -> (f ? tsFull() : tsEmpty()).floor(null));
            row(p + "higher", () -> (f ? tsFull() : tsEmpty()).higher(null));
            row(p + "lower", () -> (f ? tsFull() : tsEmpty()).lower(null));
            row(p + "headSet", () -> (f ? tsFull() : tsEmpty()).headSet(null));
            row(p + "tailSet", () -> (f ? tsFull() : tsEmpty()).tailSet(null));
            row(p + "addAllNullFirst", () -> (f ? tsFull() : tsEmpty()).addAll(nullFirst()));
            row(p + "addAllNullSecond", () -> (f ? tsFull() : tsEmpty()).addAll(nullSecond()));
            row(p + "containsAll", () -> (f ? tsFull() : tsEmpty()).containsAll(nullFirst()));
            row(p + "removeAll", () -> (f ? tsFull() : tsEmpty()).removeAll(nullFirst()));
            row(p + "retainAll", () -> (f ? tsFull() : tsEmpty()).retainAll(nullFirst()));
            row(p + "first", () -> (f ? tsFull() : tsEmpty()).first());
            row(p + "last", () -> (f ? tsFull() : tsEmpty()).last());
            row(p + "pollFirst", () -> (f ? tsFull() : tsEmpty()).pollFirst());
        }
        row("tmCopyConstructor", () -> new TreeMap<String, Integer>(hashWithNull()));
        row("tsCopyNullFirst", () -> new TreeSet<String>(nullFirst()));
        row("tsCopyNullSecond", () -> new TreeSet<String>(nullSecond()));
        // Integer elements: the message names the receiver, not the key type.
        row("tsIntegerCeiling", () -> new TreeSet<>(List.of(1, 2)).ceiling(null));
        row("tmIntegerLowerKey", () -> new TreeMap<>(Map.of(1, "x")).lowerKey(null));
        return out.toString();
    }

    /** Calls the model does not answer: each stays counted. */
    public static String countedSubMap() {
        return of(() -> tmFull().subMap(null, null));
    }
    public static String countedHeadMapInclusive() {
        return of(() -> tmFull().headMap(null, true));
    }
    public static String countedHeadSetInclusive() {
        return of(() -> tsFull().headSet(null, true));
    }
    public static String countedHeadMapComparator() {
        return of(() -> {
            TreeMap<String, Integer> m = new TreeMap<>(String.CASE_INSENSITIVE_ORDER);
            m.put("a", 1);
            return m.headMap(null);
        });
    }

    public static void main(String[] args) {
        System.out.print(rows());
    }
}

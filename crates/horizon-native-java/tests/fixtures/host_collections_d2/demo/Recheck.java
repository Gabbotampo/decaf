package demo;

import com.google.gson.Gson;
import java.util.Collection;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Queue;
import java.util.Set;
import java.util.SortedMap;
import java.util.SortedSet;
import java.util.TreeMap;
import java.util.TreeSet;

/**
 * #253's re-check: `Gson.fromJson` into a collection, map or array of them reads every value as the declared type's
 * adapter does (a raw collection's elements as `ObjectTypeAdapter`: `Double`, `Boolean`, a `LinkedTreeMap` in
 * document order), and a `String` spelled `<collection class>#<word>` is a `String`.
 */
@SuppressWarnings({"rawtypes", "unchecked"})
public final class Recheck {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static String cls(Object o) {
        return o == null ? "null" : o.getClass().getSimpleName();
    }

    public static String gsonMapArray() {
        Map[] m = new Gson().fromJson("[{\"b\":1,\"a\":2}]", Map[].class);
        return m[0].getClass().getName() + m[0] + "|" + m[0].keySet().iterator().next() + cls(m[0].get("b"));
    }

    public static String gsonHashMapArray() {
        HashMap[] m = new Gson().fromJson("[{\"b\":1}]", HashMap[].class);
        return m[0].getClass().getName() + m[0];
    }

    public static String gsonSortedMapArrays() {
        Gson gson = new Gson();
        TreeMap[] t = gson.fromJson("[{\"b\":1,\"a\":2}]", TreeMap[].class);
        SortedMap[] s = gson.fromJson("[{\"b\":1,\"a\":2}]", SortedMap[].class);
        return t[0].getClass().getName() + t[0] + "|" + s[0].getClass().getName() + s[0];
    }

    public static String gsonListArray() {
        List[] l = new Gson().fromJson("[[1,true],[\"x\"]]", List[].class);
        return l[0] + "" + l[1] + "|" + cls(l[0].get(0)) + cls(l[0].get(1)) + l[0].getClass().getName();
    }

    public static String gsonCollectionArray() {
        Collection[] c = new Gson().fromJson("[[1]]", Collection[].class);
        return c[0].getClass().getName() + c[0];
    }

    /** Gson refuses `Iterable` with a `JsonIOException`. */
    public static String gsonIterableArray() {
        Iterable[] i = new Gson().fromJson("[[1]]", Iterable[].class);
        return i[0].getClass().getName();
    }

    public static String gsonSortedSetNumbers() {
        SortedSet s = new Gson().fromJson("[3,1]", SortedSet.class);
        return cls(s.first()) + s;
    }

    public static String gsonMapValues() {
        Map m = new Gson().fromJson("{\"b\":1,\"a\":true,\"n\":{\"y\":1,\"x\":2},\"l\":[2]}", Map.class);
        return m.getClass().getName() + m + "|" + cls(m.get("b")) + cls(m.get("a")) + cls(m.get("n"))
            + cls(m.get("l")) + ((Map) m.get("n")).keySet().iterator().next();
    }

    public static String gsonLinkedHashMap() {
        LinkedHashMap m = new Gson().fromJson("{\"b\":1,\"a\":2}", LinkedHashMap.class);
        return m.toString() + m.keySet().iterator().next();
    }

    static final class Cfg {
        Map<String, Integer> counts;
        List<Double> ratios;
        Set<String> tags;
        int[] nums;
        Map<Integer, String> byId;
        List<List<Long>> nested;
    }

    /** A jar class's fields read as their declared generic types. */
    public static String gsonTypedFields() {
        Cfg c = new Gson().fromJson("{\"counts\":{\"z\":1,\"a\":2},\"ratios\":[1,2.5],\"tags\":[\"t\",\"s\",\"t\"],"
            + "\"nums\":[1,2.0],\"byId\":{\"2\":\"b\",\"1\":\"a\"},\"nested\":[[1],[2,3]]}", Cfg.class);
        return c.counts.getClass().getName() + c.counts + cls(c.counts.get("z")) + "|" + c.ratios
            + cls(c.ratios.get(0)) + "|" + c.tags.getClass().getName() + c.tags + "|" + c.nums[0] + c.nums[1] + "|"
            + c.byId.getClass().getName() + c.byId + cls(c.byId.keySet().iterator().next()) + "|" + c.nested
            + cls(c.nested.get(0).get(0));
    }

    /** `compareTo` between a `String` and a `Double`: Java's `ClassCastException`. */
    public static String gsonTreeSetMixed() {
        TreeSet t = new Gson().fromJson("[\"b\",1]", TreeSet.class);
        return t.toString();
    }

    /** A `Queue` is Gson's `ArrayDeque`. */
    public static String gsonQueue() {
        Queue q = new Gson().fromJson("[1]", Queue.class);
        return q.getClass().getName() + q;
    }

    /** Gson's map adapter refuses a repeated name. */
    public static String gsonDuplicateKey() {
        Map m = new Gson().fromJson("{\"a\":1,\"a\":2}", Map.class);
        return m.toString();
    }

    /** A `String` spelled like a collection's key is a `String`. */
    public static String forgedCollectionText() {
        String[] texts = {
            "net/minecraft/core/" + "NonNullList#x",
            "java/util/" + "ArrayList#x",
            "java/util/" + "HashMap#x",
            "java/util/" + "TreeSet#y",
        };
        StringBuilder out = new StringBuilder();
        for (String text : texts) {
            Object o = new StringBuilder(text).toString();
            out.append(b(o instanceof List)).append(b(o instanceof Iterable)).append(b(o instanceof Map))
                .append(o.getClass().getSimpleName()).append(b(o instanceof String));
            try {
                out.append(((List<?>) o).size());
            } catch (ClassCastException e) {
                out.append("CCE");
            }
            out.append('|');
        }
        return out.toString();
    }

    static String run(java.util.function.Supplier<String> body) {
        try {
            return body.get();
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName();
        }
    }

    public static void main(String[] args) {
        System.out.println("gsonMapArray=" + run(Recheck::gsonMapArray));
        System.out.println("gsonHashMapArray=" + run(Recheck::gsonHashMapArray));
        System.out.println("gsonSortedMapArrays=" + run(Recheck::gsonSortedMapArrays));
        System.out.println("gsonListArray=" + run(Recheck::gsonListArray));
        System.out.println("gsonCollectionArray=" + run(Recheck::gsonCollectionArray));
        System.out.println("gsonIterableArray=" + run(Recheck::gsonIterableArray));
        System.out.println("gsonSortedSetNumbers=" + run(Recheck::gsonSortedSetNumbers));
        System.out.println("gsonMapValues=" + run(Recheck::gsonMapValues));
        System.out.println("gsonLinkedHashMap=" + run(Recheck::gsonLinkedHashMap));
        System.out.println("gsonTypedFields=" + run(Recheck::gsonTypedFields));
        System.out.println("gsonTreeSetMixed=" + run(Recheck::gsonTreeSetMixed));
        System.out.println("gsonQueue=" + run(Recheck::gsonQueue));
        System.out.println("gsonDuplicateKey=" + run(Recheck::gsonDuplicateKey));
        System.out.println("forgedCollectionText=" + run(Recheck::forgedCollectionText));
    }
}

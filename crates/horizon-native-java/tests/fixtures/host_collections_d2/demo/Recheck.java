package demo;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.annotations.SerializedName;
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

    static final class Prims {
        boolean flag = true;
        char letter = 'a';
        float ratio = 0.1f;
        byte small = -2;
        short mid = 300;
    }

    /** A jar object's primitive fields written and read back as Gson's adapters do. */
    public static String gsonPrimitiveFields() {
        Gson gson = new Gson();
        String json = gson.toJson(new Prims());
        Prims back = gson.fromJson(json, Prims.class);
        return json + "|" + back.flag + back.letter + back.ratio + back.small + back.mid;
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

    /**
     * B5: a `fromJson` read's map is Gson's `public final class LinkedTreeMap extends AbstractMap implements
     * Serializable`, to `instanceof`, `isInstance`, `isAssignableFrom`, its `Class` and a pattern switch.
     */
    public static String gsonTreeMapSurface() {
        Object m = new Gson().fromJson("{\"b\":1,\"a\":2}", Map.class);
        Class<?> c = m.getClass();
        String kind = switch (m) {
            case Map<?, ?> mm -> "map" + mm.size();
            default -> "other";
        };
        return b(m instanceof Map) + b(m instanceof java.util.AbstractMap) + b(Map.class.isInstance(m))
            + b(Map.class.isAssignableFrom(c)) + b(m instanceof java.io.Serializable)
            + b(java.util.AbstractMap.class.isAssignableFrom(c)) + b(m instanceof List) + "|" + c.getName() + "|"
            + c.getSuperclass().getName() + "|" + c.getInterfaces().length + c.getInterfaces()[0].getName() + "|"
            + c.getModifiers() + "|" + kind;
    }

    /** B5: the same map as an `Object` read and as a raw list's element. */
    public static String gsonTreeMapNested() {
        Gson gson = new Gson();
        Object o = gson.fromJson("{\"a\":1}", Object.class);
        List l = gson.fromJson("[{\"a\":1}]", List.class);
        return b(o instanceof Map) + b(l.get(0) instanceof Map) + b(l.get(0) instanceof java.util.AbstractMap)
            + o.getClass().getSimpleName();
    }

    /** B5: `LinkedTreeMap.put` refuses a `null` key with its own message; a `String` key is put in order. */
    public static String gsonTreeMapNullKey() {
        Map m = new Gson().fromJson("{\"b\":1}", Map.class);
        m.put("a", 2);
        try {
            m.put(null, 1);
            return "put " + m;
        } catch (NullPointerException e) {
            return "NPE " + e.getMessage() + m;
        }
    }

    enum E {
        A,
        @SerializedName(value = "bee", alternate = {"bea"})
        B,
        C
    }

    static final class Modes {
        E mode;
        E[] modes;
    }

    /**
     * B6: `EnumTypeAdapter` answers the jar's own constant: by `name()`, by `@SerializedName`'s value and
     * alternate, by `toString()` (here `name()`) when the names miss, else `null`; in an array and in fields too.
     */
    public static String gsonEnumRead() {
        Gson gson = new Gson();
        E a = gson.fromJson("\"A\"", E.class);
        E bee = gson.fromJson("\"bee\"", E.class);
        E bea = gson.fromJson("\"bea\"", E.class);
        E byName = gson.fromJson("\"B\"", E.class);
        E z = gson.fromJson("\"Z\"", E.class);
        E number = gson.fromJson("1", E.class);
        E[] arr = gson.fromJson("[\"C\",\"bee\"]", E[].class);
        Modes h = gson.fromJson("{\"mode\":\"bee\",\"modes\":[\"A\"]}", Modes.class);
        String sw = switch (a) {
            case A -> "caseA";
            case B -> "caseB";
            case C -> "caseC";
        };
        return b(a == E.A) + b(a.equals(E.A)) + a.ordinal() + a.name() + sw + "|" + b(bee == E.B) + b(bea == E.B)
            + b(byName == E.B) + "|" + z + number + "|" + arr.length + b(arr[0] == E.C) + b(arr[1] == E.B) + "|"
            + b(h.mode == E.B) + b(h.modes[0] == E.A);
    }

    enum Shown {
        X {
            @Override
            public String toString() {
                return "ex";
            }
        },
        Y
    }

    /** B6: a name still reads its constant when a constant overrides `toString`. */
    public static String gsonEnumNameWithToString() {
        Gson gson = new Gson();
        return b(gson.fromJson("\"Y\"", Shown.class) == Shown.Y) + b(gson.fromJson("\"X\"", Shown.class) == Shown.X);
    }

    /** B6: `toString()`'s text reads its constant on Java; that text is not computed here (counted). */
    public static String gsonEnumToStringText() {
        return String.valueOf(new Gson().fromJson("\"ex\"", Shown.class) == Shown.X);
    }

    static final class Plain {
        int a;
    }

    /** B6: a JSON string for a jar class that is no enum: Gson's `JsonSyntaxException` (counted). */
    public static String gsonStringForPlainClass() {
        Plain p = new Gson().fromJson("\"x\"", Plain.class);
        return String.valueOf(p.a);
    }

    static final class FloatNaN {
        float n = Float.NaN;
    }

    static final class FloatInf {
        float n = Float.POSITIVE_INFINITY;
    }

    static final class FloatNegInf {
        float n = Float.NEGATIVE_INFINITY;
    }

    static final class DoubleNaN {
        double n = Double.NaN;
    }

    static final class BoxNaN {
        Float n = Float.NaN;
    }

    static String json(Gson gson, Object o) {
        try {
            return gson.toJson(o);
        } catch (IllegalArgumentException e) {
            return "IAE " + e.getMessage();
        }
    }

    /** B7: a NaN or infinite `float`/`double`/`Float` field is `checkValidFloatingPoint`'s refusal. */
    public static String gsonNonFiniteFields() {
        Gson gson = new Gson();
        return json(gson, new FloatNaN()) + "|" + json(gson, new FloatInf()) + "|" + json(gson, new FloatNegInf())
            + "|" + json(gson, new DoubleNaN()) + "|" + json(gson, new BoxNaN());
    }

    /** B7: `serializeSpecialFloatingPointValues` writes them as literals. */
    public static String gsonNonFiniteSpecial() {
        Gson gson = new GsonBuilder().serializeSpecialFloatingPointValues().create();
        return json(gson, new FloatNaN()) + "|" + json(gson, new FloatNegInf()) + "|" + json(gson, new DoubleNaN())
            + "|" + json(gson, new BoxNaN());
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
        System.out.println("gsonPrimitiveFields=" + run(Recheck::gsonPrimitiveFields));
        System.out.println("gsonTreeSetMixed=" + run(Recheck::gsonTreeSetMixed));
        System.out.println("gsonQueue=" + run(Recheck::gsonQueue));
        System.out.println("gsonDuplicateKey=" + run(Recheck::gsonDuplicateKey));
        System.out.println("forgedCollectionText=" + run(Recheck::forgedCollectionText));
        System.out.println("gsonTreeMapSurface=" + run(Recheck::gsonTreeMapSurface));
        System.out.println("gsonTreeMapNested=" + run(Recheck::gsonTreeMapNested));
        System.out.println("gsonTreeMapNullKey=" + run(Recheck::gsonTreeMapNullKey));
        System.out.println("gsonEnumRead=" + run(Recheck::gsonEnumRead));
        System.out.println("gsonEnumNameWithToString=" + run(Recheck::gsonEnumNameWithToString));
        System.out.println("gsonEnumToStringText=" + run(Recheck::gsonEnumToStringText));
        System.out.println("gsonStringForPlainClass=" + run(Recheck::gsonStringForPlainClass));
        System.out.println("gsonNonFiniteFields=" + run(Recheck::gsonNonFiniteFields));
        System.out.println("gsonNonFiniteSpecial=" + run(Recheck::gsonNonFiniteSpecial));
    }
}

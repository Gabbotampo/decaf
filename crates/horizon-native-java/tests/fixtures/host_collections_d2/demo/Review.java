package demo;

import com.google.common.collect.HashMultimap;
import com.google.common.collect.ImmutableMap;
import com.google.common.collect.ImmutableSet;
import com.google.common.collect.Multimap;
import com.google.common.collect.Sets;
import com.google.gson.Gson;
import java.util.Map;
import java.util.NavigableSet;
import java.util.SortedMap;
import java.util.SortedSet;
import java.util.concurrent.TimeUnit;
import java.util.Set;
import java.util.TreeSet;
import java.util.stream.Stream;

/**
 * #253's review: Gson's typed arrays read each element as the component type, a `TreeSet` target is sorted, Guava's
 * shared empty map and a multimap's views are one object each, and Guava's immutables have Guava's classes (so
 * `Map.copyOf`/`Set.copyOf` copy them).
 */
public final class Review {
    enum Color { RED, GREEN, BLUE }

    static String b(boolean v) {
        return v ? "T" : "F";
    }

    public static String gsonArrays() {
        Gson gson = new Gson();
        String[] texts = gson.fromJson("[\"a\",1,true]", String[].class);
        int[] ints = gson.fromJson("[1,2]", int[].class);
        Object[] objects = gson.fromJson("[\"x\",2]", Object[].class);
        return texts[0] + texts[1] + texts[1].length() + texts[2] + "|" + ints[0] + ints[1] + "|" + objects[0]
            + texts.getClass().getSimpleName();
    }

    static final class Foo {
        int x;
    }

    /** A jar class's elements need Gson's reflective adapter. */
    public static String gsonForeign() {
        Foo[] foos = new Gson().fromJson("[{\"x\":3}]", Foo[].class);
        return foos[0].getClass().getName() + foos[0].x;
    }

    public static String gsonSets() {
        Gson gson = new Gson();
        @SuppressWarnings("unchecked")
        TreeSet<String> tree = gson.fromJson("[\"c\",\"a\",\"b\",\"a\"]", TreeSet.class);
        @SuppressWarnings("unchecked")
        Set<String> set = gson.fromJson("[\"b\",\"a\",\"b\"]", Set.class);
        return tree.first() + tree.last() + tree.size() + tree + "|" + set.size() + set + set.getClass().getName();
    }

    public static String guavaIdentity() {
        Multimap<String, Integer> m = HashMultimap.create();
        m.put("a", 1);
        Map<String, Integer> empty = Stream.<String>of().collect(ImmutableMap.toImmutableMap(s -> s, s -> 1));
        return b(ImmutableMap.of() == ImmutableMap.of()) + b((Object) empty == ImmutableMap.of()) + b(m.keySet() == m.keySet())
            + b(m.values() == m.values());
    }

    public static String guavaClasses() {
        Map<String, Integer> two = ImmutableMap.of("a", 1, "b", 2);
        Map<String, Integer> one = ImmutableMap.of("a", 1);
        Map<String, Integer> collected = Stream.of("a", "b").collect(ImmutableMap.toImmutableMap(s -> s, s -> 1));
        Set<Color> colors = Sets.immutableEnumSet(Color.BLUE, Color.RED);
        Set<Color> single = Sets.immutableEnumSet(Color.GREEN);
        // Each member's name in iteration order (a jar enum's text in a host collection's `toString` is
        // another matter).
        StringBuilder order = new StringBuilder();
        for (Color color : colors) {
            order.append(color.name()).append(' ');
        }
        return two.getClass().getName() + "," + one.getClass().getName() + "," + collected.getClass().getName() + "|"
            + colors.getClass().getName() + order + "," + single.getClass().getName() + "|" + b(Map.copyOf(two) == two)
            + b(Map.copyOf(collected) == collected) + b(Set.copyOf(colors) == colors) + "|" + two.get("b") + one;
    }

    /** Gson's `SHORT`/`BYTE` narrowing, and a `float` rounded as one. */
    public static String gsonNarrow() {
        Gson gson = new Gson();
        short[] shorts = gson.fromJson("[40000,-1,7]", short[].class);
        byte[] bytes = gson.fromJson("[200,-1]", byte[].class);
        float[] floats = gson.fromJson("[0.1]", float[].class);
        return shorts[0] + "," + shorts[1] + "," + shorts[2] + "|" + bytes[0] + "," + bytes[1] + "|" + b(floats[0] == 0.1f);
    }

    /** `ObjectTypeAdapter`: `Boolean`, `Double`, `String`, `null`, an `ArrayList` of the same. */
    public static String gsonObjects() {
        Object[] objects = new Gson().fromJson("[true,2,\"s\",null,[1,false]]", Object[].class);
        StringBuilder out = new StringBuilder();
        for (Object o : objects) {
            out.append(o == null ? "null" : o.getClass().getSimpleName() + ":" + o).append(' ');
        }
        return out.toString();
    }

    /** `ConstructorConstructor`'s sorted defaults. */
    public static String gsonSorted() {
        Gson gson = new Gson();
        @SuppressWarnings("unchecked")
        SortedMap<String, Object> map = gson.fromJson("{\"b\":1,\"a\":2}", SortedMap.class);
        @SuppressWarnings("unchecked")
        SortedSet<String> set = gson.fromJson("[\"b\",\"a\"]", SortedSet.class);
        @SuppressWarnings("unchecked")
        NavigableSet<String> navigable = gson.fromJson("[\"a\"]", NavigableSet.class);
        return map.getClass().getName() + map.firstKey() + "|" + set.getClass().getName() + set.first() + "|"
            + navigable.getClass().getName();
    }

    /** A Guava map serializes as a JSON object. */
    public static String gsonGuavaJson() {
        Gson gson = new Gson();
        Map<String, Integer> one = Stream.of("x").collect(ImmutableMap.toImmutableMap(s -> s, s -> 1));
        return gson.toJson(ImmutableMap.of("a", 1, "b", 2)) + gson.toJson(ImmutableMap.of("a", 1))
            + gson.toJson(ImmutableMap.of()) + gson.toJson(one);
    }

    /** Guava's classes have Guava's superclasses, and are their interfaces. */
    public static String guavaClassSurface() {
        Class<?> one = ImmutableMap.of("a", 1).getClass();
        Class<?> two = ImmutableMap.of("a", 1, "b", 2).getClass();
        Class<?> none = ImmutableMap.of().getClass();
        Class<?> emptySet = ImmutableSet.of().getClass();
        Class<?> colors = Sets.immutableEnumSet(Color.BLUE, Color.RED).getClass();
        return one.getSuperclass().getName() + "," + two.getSuperclass().getName() + ","
            + none.getSuperclass().getName() + "," + emptySet.getSuperclass().getName() + ","
            + colors.getSuperclass().getName() + "|" + b(Map.class.isAssignableFrom(one))
            + b(Map.class.isAssignableFrom(two)) + b(ImmutableMap.class.isAssignableFrom(one))
            + b(Set.class.isAssignableFrom(colors)) + b(Set.class.isAssignableFrom(emptySet))
            + b(java.util.List.class.isAssignableFrom(two));
    }

    /** Counted where a value here cannot be Java's (exact or counted, never silent). */
    public static String gsonByteOverflow() {
        byte[] bytes = new Gson().fromJson("[300]", byte[].class);
        return "" + bytes[0];
    }

    public static String gsonShortBox() {
        Short[] shorts = new Gson().fromJson("[1]", Short[].class);
        return shorts[0].getClass().getName();
    }

    public static String gsonFloatBox() {
        Float[] floats = new Gson().fromJson("[0.1]", Float[].class);
        return floats[0].getClass().getName();
    }

    public static String guavaJdkEnum() {
        return Sets.immutableEnumSet(TimeUnit.SECONDS, TimeUnit.NANOSECONDS).toString();
    }

    public static void main(String[] args) {
        System.out.println("gsonArrays=" + gsonArrays());
        System.out.println("gsonForeign=" + gsonForeign());
        System.out.println("gsonSets=" + gsonSets());
        System.out.println("guavaIdentity=" + guavaIdentity());
        System.out.println("guavaClasses=" + guavaClasses());
        System.out.println("gsonNarrow=" + gsonNarrow());
        System.out.println("gsonObjects=" + gsonObjects());
        System.out.println("gsonSorted=" + gsonSorted());
        System.out.println("gsonGuavaJson=" + gsonGuavaJson());
        System.out.println("guavaClassSurface=" + guavaClassSurface());
        try {
            System.out.println("gsonByteOverflow=" + gsonByteOverflow());
        } catch (RuntimeException e) {
            System.out.println("gsonByteOverflow=" + e.getClass().getSimpleName());
        }
        System.out.println("gsonShortBox=" + gsonShortBox());
        System.out.println("gsonFloatBox=" + gsonFloatBox());
        System.out.println("guavaJdkEnum=" + guavaJdkEnum());
    }
}

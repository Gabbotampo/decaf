package demo;

import com.google.common.collect.HashMultimap;
import com.google.common.collect.ImmutableMap;
import com.google.common.collect.Multimap;
import com.google.common.collect.Sets;
import com.google.gson.Gson;
import java.util.Map;
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
        return two.getClass().getName() + "," + one.getClass().getName() + "," + collected.getClass().getName() + "|"
            + colors.getClass().getName() + colors + "," + single.getClass().getName() + "|" + b(Map.copyOf(two) == two)
            + b(Map.copyOf(collected) == collected) + b(Set.copyOf(colors) == colors) + "|" + two.get("b") + one;
    }

    public static void main(String[] args) {
        System.out.println("gsonArrays=" + gsonArrays());
        System.out.println("gsonForeign=" + gsonForeign());
        System.out.println("gsonSets=" + gsonSets());
        System.out.println("guavaIdentity=" + guavaIdentity());
        System.out.println("guavaClasses=" + guavaClasses());
    }
}

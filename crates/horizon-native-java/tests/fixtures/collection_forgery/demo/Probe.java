package demo;

import java.util.ArrayList;
import java.util.Collection;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * A `String` spelled like the host's old collection literals (`list/a,b`, `set/x`, `map/k=v`, the bare `list`) is a
 * `String` everywhere (#156 P14d2): its class, `instanceof`, `equals` both ways, hash, a `HashMap` key, a list
 * element, `String.valueOf`, concatenation, and a cast to a collection throws.
 */
public final class Probe {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    /** Built at run time, so none is a constant the compiler folds. */
    static String spelled(String head, String tail) {
        return new StringBuilder(head).append(tail).toString();
    }

    static String one(String text) {
        Object o = text;
        String cast;
        try {
            List<?> list = (List<?>) o;
            cast = "cast:" + list.size();
        } catch (ClassCastException e) {
            cast = "CCE";
        }
        String set;
        try {
            Set<?> s = (Set<?>) o;
            set = "cast:" + s.size();
        } catch (ClassCastException e) {
            set = "CCE";
        }
        String map;
        try {
            Map<?, ?> m = (Map<?, ?>) o;
            map = "cast:" + m.size();
        } catch (ClassCastException e) {
            map = "CCE";
        }
        Map<Object, Integer> keyed = new HashMap<>();
        keyed.put(text, 1);
        List<Object> held = new ArrayList<>();
        held.add(o);
        String copy = new String(text.toCharArray());
        return o.getClass().getName() + "|" + b(o instanceof String) + b(o instanceof CharSequence)
            + b(o instanceof List) + b(o instanceof Set) + b(o instanceof Map) + b(o instanceof Collection)
            + b(o instanceof Iterable) + "|" + b(text.equals(copy)) + b(copy.equals(text)) + b(o.equals(copy))
            + (text.hashCode() == copy.hashCode() ? "H" : "h") + "|" + keyed.get(copy) + b(keyed.containsKey(copy))
            + "|" + held.get(0).getClass().getSimpleName() + b(held.contains(copy)) + "|" + String.valueOf(o) + "|"
            + ("<" + o + ">") + text.length() + "|" + cast + set + map;
    }

    public static String list() {
        return one(spelled("list/", "a,b"));
    }

    public static String set() {
        return one(spelled("set/", "x"));
    }

    public static String map() {
        return one(spelled("map/", "k=v"));
    }

    public static String bare() {
        return one(spelled("li", "st")) + ";" + one(spelled("se", "t")) + ";" + one(spelled("ma", "p"));
    }

    public static void main(String[] args) {
        System.out.println("list=" + list());
        System.out.println("set=" + set());
        System.out.println("map=" + map());
        System.out.println("bare=" + bare());
    }
}

package demo;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import java.util.*;
import java.util.function.Supplier;

/** Collections that contain themselves, and other cycles a jar can build (#250). */
@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    static String t(Supplier<Object> s) {
        try {
            return String.valueOf(s.get());
        } catch (StackOverflowError e) {
            return "SOE";
        }
    }

    /** `AbstractCollection.toString`: an element that is the list prints `(this Collection)`. */
    public static String listSelf() {
        List<Object> l = new ArrayList<>();
        l.add(l);
        l.add(1);
        return l.toString() + "|" + ("" + l) + "|" + String.valueOf(l) + "|" + new StringBuilder().append(l);
    }

    public static String setSelf() {
        Set<Object> s = new LinkedHashSet<>();
        s.add(s);
        s.add("x");
        return s.toString();
    }

    /** `AbstractMap.toString`: a key or value that is the map prints `(this Map)`. */
    public static String mapSelf() {
        Map<Object, Object> m = new LinkedHashMap<>();
        m.put("k", m);
        Map<Object, Object> n = new LinkedHashMap<>();
        n.put(n, 1);
        Map<Object, Object> both = new HashMap<>();
        both.put("v", 2);
        both.put("w", both);
        return m + "|" + n + "|" + both;
    }

    /** A cycle through two collections: Java recurses until `StackOverflowError`. */
    public static String nestedCycle() {
        List<Object> a = new ArrayList<>();
        List<Object> b = new ArrayList<>();
        a.add(b);
        b.add(a);
        Map<Object, Object> m = new HashMap<>();
        List<Object> c = new ArrayList<>();
        m.put("c", c);
        c.add(m);
        return t(() -> a.toString()) + t(() -> "" + b) + t(() -> m.toString());
    }

    /** `hashCode` of a list or map holding itself recurses on the JVM. */
    public static String hashSelf() {
        List<Object> l = new ArrayList<>();
        l.add(l);
        Map<Object, Object> m = new HashMap<>();
        m.put("k", m);
        return t(() -> l.hashCode()) + t(() -> m.hashCode()) + t(() -> Objects.hash(l));
    }

    /** `equals` between two self-containing lists recurses on the JVM. */
    public static String equalsSelf() {
        List<Object> a = new ArrayList<>();
        a.add(a);
        List<Object> b = new ArrayList<>();
        b.add(b);
        List<Object> c = new ArrayList<>(List.of(1));
        return t(() -> a.equals(b)) + t(() -> a.equals(a)) + t(() -> a.equals(c)) + t(() -> List.of(a).contains(b));
    }

    /** `Arrays.deepToString` prints `[...]` for an array that holds itself; the hash recurses. */
    public static String arraysSelf() {
        Object[] arr = new Object[2];
        arr[0] = arr;
        arr[1] = "x";
        return Arrays.deepToString(arr) + t(() -> Arrays.deepHashCode(arr));
    }

    /** Gson recurses through a cycle while it writes. */
    public static String gsonCycle() {
        JsonObject o = new JsonObject();
        o.add("self", o);
        List<Object> l = new ArrayList<>();
        l.add(l);
        return t(() -> o.toString()) + t(() -> new Gson().toJson(l));
    }

    /** A composed comparator: `compare` walks each link; reversing a reversal is the original. */
    public static String comparatorChain() {
        Comparator<Integer> c = Comparator.naturalOrder();
        for (int i = 0; i < 10; i++) {
            c = c.thenComparing(Comparator.naturalOrder());
        }
        Comparator<Integer> chain = c;
        Comparator<Integer> r = Comparator.naturalOrder();
        for (int i = 0; i < 100_001; i++) {
            r = r.reversed();
        }
        return t(() -> chain.compare(1, 2)) + "|" + r.compare(1, 2);
    }

    /** A `compareTo` that recurses through a cycle of jar objects. */
    static final class Node implements Comparable<Node> {
        Node next;
        public int compareTo(Node o) { return next.compareTo(o.next); }
    }

    public static String compareToCycle() {
        Node a = new Node();
        a.next = a;
        return t(() -> a.compareTo(a));
    }

    public static void main(String[] args) throws Exception {
        for (String m : new String[] {"listSelf", "setSelf", "mapSelf", "nestedCycle", "hashSelf", "equalsSelf",
                "arraysSelf", "gsonCycle", "comparatorChain", "compareToCycle"}) {
            System.out.println(m + "=" + Probe.class.getMethod(m).invoke(null));
        }
    }
}

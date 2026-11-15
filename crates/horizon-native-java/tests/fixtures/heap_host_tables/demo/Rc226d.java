package demo;

import java.util.*;
import java.util.function.Supplier;

public final class Rc226d {
    static final class Cell { int v; String s; Cell(int v, String s) { this.v = v; this.s = s; } }
    /** Never written: an object with no heap row. */
    static final class Bare { int count; long stamp; }

    static Bare inStatic;
    static Supplier<Object> lambda;
    static Optional<Object> optional;
    static List<Object> listOfEntry;
    static Object[] array;
    static Map<Object, String> mapKey;
    static List<Object> view;
    static List<Object> singleton;
    static IdentityHashMap<Object, Integer> identity;
    static Supplier<Supplier<Object>> nested;
    static List<Object> uuids;
    static Supplier<Object> uuidLambda;
    static Optional<Object> localeOpt;
    static Map<String, Integer> viewed;
    static Set<String> viewedKeys;

    public static String setup() {
        Properties p = System.getProperties();
        p.put("hj.k", "v");
        p.put("hj.cell", new Cell(7, "seven"));
        p.put("hj.bare", new Bare());
        inStatic = new Bare();
        Bare b1 = new Bare();
        lambda = () -> b1;
        optional = Optional.of(new Bare());
        listOfEntry = List.of(Map.entry(new Bare(), "x"));
        array = new Object[] { new Bare() };
        mapKey = new HashMap<>();
        mapKey.put(new Bare(), "k");
        List<Object> src = new ArrayList<>();
        src.add(new Bare());
        view = Collections.unmodifiableList(src);
        singleton = Collections.singletonList(new Bare());
        identity = new IdentityHashMap<>();
        identity.put(new Bare(), 1);
        Bare b2 = new Bare();
        Supplier<Object> inner = () -> b2;
        nested = () -> inner;
        uuids = List.of(new UUID(1L, 2L));
        UUID u = new UUID(3L, 4L);
        uuidLambda = () -> u;
        localeOpt = Optional.of(new Locale("fr", "ca"));
        viewed = new HashMap<>();
        viewed.put("a", 1);
        viewedKeys = viewed.keySet();
        return "ok";
    }

    public static String garbage() {
        int s = 0;
        for (int i = 0; i < 3000; i++) {
            Cell c = new Cell(i, "g");
            s += c.v;
            Object o = new Object();
        }
        return "g" + (s & 1);
    }

    static String bare(Object o) {
        Bare b = (Bare) o;
        synchronized (b) {
            return Integer.toString(b.count) + b.stamp;
        }
    }

    public static String check() {
        StringBuilder out = new StringBuilder();
        Properties p = System.getProperties();
        Object c = p.get("hj.cell");
        Object b = p.get("hj.bare");
        out.append(System.getProperty("hj.k")).append('|').append(p.get("hj.k")).append('|')
            .append(c == null ? "null" : ((Cell) c).v + ((Cell) c).s).append('|')
            .append(b == null ? "null" : bare(b)).append('|').append(p == System.getProperties()).append(';');
        out.append(bare(inStatic)).append(',');
        out.append(bare(lambda.get())).append(',');
        out.append(bare(optional.get())).append(',');
        out.append(bare(((Map.Entry<?, ?>) listOfEntry.get(0)).getKey())).append(',');
        out.append(bare(array[0])).append(',');
        out.append(bare(mapKey.keySet().iterator().next())).append(',');
        out.append(bare(view.get(0))).append(',');
        out.append(bare(singleton.get(0))).append(',');
        out.append(bare(identity.keySet().iterator().next())).append(',');
        out.append(bare(nested.get().get())).append(';');
        out.append(uuids.get(0)).append(',').append(uuidLambda.get()).append(',').append(localeOpt.get()).append(';');
        out.append(viewedKeys == viewed.keySet()).append(viewedKeys);
        return out.toString();
    }

    /** One fresh map per call, read through its views (a per-tick loop). */
    public static String mapTick() {
        HashMap<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        return Integer.toString(m.keySet().size() + m.values().size() + m.entrySet().size());
    }

    /** The same without views (the control). */
    public static String mapTickPlain() {
        HashMap<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        return Integer.toString(m.size() * 3);
    }

    public static void main(String[] args) {
        System.out.println("setup=" + setup());
        for (int i = 0; i < 40; i++) garbage();
        System.out.println("check=" + check());
        System.out.println("mapTick=" + mapTick() + " plain=" + mapTickPlain());
    }
}

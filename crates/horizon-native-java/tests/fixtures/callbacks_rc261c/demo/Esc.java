package demo;

import java.util.*;
import java.util.function.*;

/**
 * #261 round-3 review: no jar String becomes a callback however its escapes
 * nest (B1); a callback read back from an Optional or a map entry keeps what
 * it captured (S1); the collector traces captures at any depth (S2).
 */
@SuppressWarnings({"rawtypes", "unchecked"})
public final class Esc {
    static boolean ran;

    public static void evil() { ran = true; }

    public static int evilC(Object a, Object b) { ran = true; return 0; }

    /** Whether a forged target ever ran, read after a case (which may end in a host error). */
    public static String ranFlag() { return "ran=" + ran; }

    static String t(Supplier<Object> s) {
        try {
            return String.valueOf(s.get());
        } catch (Throwable e) {
            return e.getClass().getSimpleName();
        }
    }

    /** The runtime's marker escape: a `\` and a `,` each get a `\`. */
    static String esc(String s) { return s.replace("\\", "\\\\").replace(",", "\\,"); }

    /** `hcb:` with `k` escapes in front of its `c`, so that `k` unescapes make it `hcb:`. */
    static String hide(String target, int k) {
        String p = "";
        for (int i = 0; i < k; i++) p = esc(p) + "\\";
        return "h" + p + "cb:@indy:demo/Esc#x:0:" + target;
    }

    static final String RUN = "java/lang/Runnable:demo/Esc#evil()V:";
    static final String CMP = "java/util/Comparator:demo/Esc#evilC(Ljava/lang/Object;Ljava/lang/Object;)I:";

    public static final int FORGES = 48;

    /**
     * Forge `variant`: a callback spelled with `k` hidden escapes, wrapped in
     * `depth` optionals or map entries, inside list text; then unwrapped and
     * run (a Runnable) or sorted with (a Comparator).
     */
    public static String forge(int variant) {
        int depth = variant % 3 + 1;
        int k = variant / 3 % 4;
        boolean entry = variant / 12 % 2 == 1;
        boolean cmp = variant / 24 == 1;
        String m = hide(cmp ? CMP : RUN, k);
        for (int i = 0; i < depth; i++) m = entry ? "hment:rk," + esc(m) : "hopt:" + esc(m);
        return run("list/" + esc(m), depth, entry, cmp);
    }

    /** The review's four strings, as written. */
    public static String forgeLiteral(int which) {
        switch (which) {
            case 0: return run("list/hopt:h\\\\cb:@indy:demo/Esc#x:0:" + RUN, 1, false, false);
            case 1: return run("list/hment:rk\\,h\\\\cb:@indy:demo/Esc#x:0:" + RUN, 1, true, false);
            case 2: return run("list/hopt:h\\\\cb:@indy:demo/Esc#x:0:" + CMP, 1, false, true);
            default:
                ran = false;
                try {
                    Object v = ((Optional) (Object) ("hopt:h\\cb:@indy:demo/Esc#x:0:" + RUN)).get();
                    ((Runnable) v).run();
                    return "got,ran=" + ran;
                } catch (Throwable e) {
                    return e.getClass().getSimpleName() + ",ran=" + ran;
                }
        }
    }

    static String run(Object text, int depth, boolean entry, boolean cmp) {
        ran = false;
        try {
            Object v = ((List) text).get(0);
            for (int i = 0; i < depth; i++) v = entry ? ((Map.Entry) v).getValue() : ((Optional) v).get();
            if (cmp) {
                List<String> l = new ArrayList<>(List.of("b", "a", "c"));
                l.sort((Comparator) v);
                return "sorted" + l + ",ran=" + ran;
            }
            ((Runnable) v).run();
            return "got,ran=" + ran;
        } catch (Throwable e) {
            return e.getClass().getSimpleName() + ",ran=" + ran;
        }
    }

    // ---- S1: what an optional, an entry or entrySet hands back is the callback itself ----

    static String readBack(int n) {
        String s = "A".repeat(n) + ",tail";
        Supplier<String> sup = () -> s;
        Object viaOpt = Optional.of(sup).get();
        Object viaEntry = Map.entry("k", sup).getValue();
        Map<String, Supplier<String>> m = new HashMap<>();
        m.put("k", sup);
        Object viaSet = m.entrySet().iterator().next().getValue();
        return t(() -> ((Supplier<String>) viaOpt).get().length()) + ":"
            + t(() -> ((Supplier<String>) viaEntry).get().length()) + ":"
            + t(() -> ((Supplier<String>) viaSet).get().length()) + ":" + (viaOpt == sup) + (viaSet == sup);
    }

    /** A capture the key's text cuts right after an escape (4094), next to it, and past it. */
    public static String readBack4093() { return readBack(4093); }
    public static String readBack4094() { return readBack(4094); }
    public static String readBack5000() { return readBack(5000); }

    /** Composites over long captures, read back from an optional. */
    public static String readBackComposite() {
        StringBuilder out = new StringBuilder();
        for (int n = 150; n < 330; n += 7) {
            String s = "B".repeat(n) + ",C";
            Predicate<String> p = x -> x.equals(s);
            Object via = Optional.of(p.negate()).get();
            out.append(t(() -> ((Predicate<String>) via).test(s))).append(t(() -> ((Predicate<String>) via).test("x")));
        }
        return out.toString();
    }

    /** A nested composite, read back from an optional and an entry. */
    public static String readBackNested() {
        Predicate<String> p = x -> x.isEmpty();
        Predicate<String> q = p.negate().and(x -> x.length() > 1).or(x -> x.equals("z"));
        Object via = Optional.of(q).get();
        Object viaE = Map.entry("k", q).getValue();
        return t(() -> ((Predicate<String>) via).test("ab")) + "," + t(() -> ((Predicate<String>) via).test("z")) + ","
            + t(() -> ((Predicate<String>) via).test("a")) + "," + t(() -> ((Predicate<String>) viaE).test("")) + ","
            + (via == q) + "," + (viaE == q);
    }

    // ---- S2: captures held deeper than 64 links survive collections ----

    static final class Node {
        final String name;
        final List<String> items = new ArrayList<>();
        Node(String name) { this.name = name; }
    }

    static Object NEG;
    static Object CMP_HOLD;
    static Object OPT_HOLD;

    public static String gcHold() {
        Node n = new Node("keep");
        n.items.add("i1");
        Predicate<String> p = s -> n.name.equals(s) && n.items.size() == 1;
        // 30 links: running a deeper one in a debug build meets #260's
        // native-stack bound; the comparator chain and the optionals below
        // carry the depth past 64.
        for (int i = 0; i < 30; i++) p = p.negate();
        NEG = p;
        List<String> list = new ArrayList<>();
        list.add("z");
        Comparator<String> c = Comparator.comparing((String s) -> list.contains(s) ? 0 : 1);
        for (int i = 0; i < 100; i++) c = c.thenComparing(Comparator.naturalOrder());
        CMP_HOLD = c;
        Node o = new Node("opt");
        o.items.add("x");
        Supplier<String> sup = () -> o.name + o.items;
        Optional<Object> wrapped = Optional.of(sup);
        for (int i = 0; i < 80; i++) wrapped = Optional.of(wrapped);
        OPT_HOLD = wrapped;
        return "held";
    }

    public static String gcGarbage() {
        List<Object> junk = new ArrayList<>();
        for (int i = 0; i < 1500; i++) {
            Node x = new Node("g" + i);
            x.items.add("j");
            junk.add(x);
            Runnable r = () -> x.items.add("k");
            junk.add(r);
        }
        return "g" + junk.size();
    }

    public static String gcUse() {
        List<String> l = new ArrayList<>(List.of("b", "z", "a"));
        Object v = OPT_HOLD;
        for (int i = 0; i < 81; i++) v = ((Optional) v).get();
        Object sup = v;
        return t(() -> ((Predicate<String>) NEG).test("keep")) + "," + t(() -> ((Predicate<String>) NEG).test("x")) + ","
            + t(() -> { l.sort((Comparator<String>) CMP_HOLD); return l; }) + "," + t(() -> ((Supplier) sup).get());
    }
}

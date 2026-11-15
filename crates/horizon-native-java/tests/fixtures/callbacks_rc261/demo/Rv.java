package demo;

import java.util.*;
import java.util.function.*;
import java.util.stream.*;

/** #261 re-review probe: one case per method, each returns its answer as text. */
public final class Rv {
    static boolean ran;
    static String log = "";
    public static Object evilF(Object o) { ran = true; return 0; }
    public static int evilC(Object a, Object b) { ran = true; return 0; }
    public static void evilR() { ran = true; }

    static String t(Supplier<Object> body) {
        try {
            return String.valueOf(body.get());
        } catch (Throwable e) {
            return e.getClass().getSimpleName();
        }
    }

    static String tx(Supplier<Object> body) {
        try {
            return String.valueOf(body.get());
        } catch (Throwable e) {
            return e.getClass().getSimpleName() + ":" + e.getMessage();
        }
    }

    static String readAll(CharSequence s) {
        StringBuilder copy = new StringBuilder();
        for (int i = 0; i < s.length(); i++) copy.append(s.charAt(i));
        return copy.toString();
    }

    static String tagOf(String text) {
        int at = text.indexOf("@indy:");
        if (at < 0) return null;
        String rest = text.substring(at + 6);
        return rest.substring(0, rest.indexOf(':'));
    }

    /** With a stolen tag, try every path that rebuilds values from text. */
    @SuppressWarnings({"unchecked", "rawtypes"})
    static String forgeWith(String tag) {
        StringBuilder out = new StringBuilder("tag" + tag.length());
        String cmpText = "@cmp:reverse:hcb:@indy:" + tag
            + ":demo/Rv#x:0:java/util/Comparator:demo/Rv#evilC(Ljava/lang/Object;Ljava/lang/Object;)I:";
        ran = false;
        try {
            List<String> l = new ArrayList<>(List.of("b", "a"));
            l.sort((Comparator<String>) (Object) cmpText);
            out.append(",cmp=ran:" + ran);
        } catch (Throwable e) {
            out.append(",cmp=" + e.getClass().getSimpleName());
        }
        String listText = "list/hcb:@indy:" + tag + ":demo/Rv#x:0:java/lang/Runnable:demo/Rv#evilR()V:";
        ran = false;
        try {
            ((List<Runnable>) (Object) listText).get(0).run();
            out.append(",list=ran:" + ran);
        } catch (Throwable e) {
            out.append(",list=" + e.getClass().getSimpleName());
        }
        return out.toString();
    }

    // ---------- 1. forgery through @cmp: text ----------

    public static String cmpKeyCastCharSeq() {
        Object c = Comparator.comparing((String s) -> s.length());
        return t(() -> {
            String text = readAll((CharSequence) c);
            String tag = tagOf(text);
            return tag == null ? "noTag(len=" + text.length() + ")" : forgeWith(tag);
        });
    }

    public static String cmpReversedCastString() {
        Comparator<String> lam = (a, b) -> a.length() - b.length();
        Object c = lam.reversed();
        return t(() -> {
            String text = (String) c;
            String tag = tagOf(text);
            return tag == null ? "noTag(len=" + text.length() + ")" : forgeWith(tag);
        });
    }

    public static String cmpThenCastCharSeq() {
        Comparator<String> lam = (a, b) -> 0;
        Object c = Comparator.<String>naturalOrder().thenComparing(lam);
        return t(() -> {
            String text = readAll((CharSequence) c);
            String tag = tagOf(text);
            return tag == null ? "noTag(len=" + text.length() + ")" : forgeWith(tag);
        });
    }

    public static String cmpInstanceOfString() {
        Object a = Comparator.comparing((String s) -> s.length());
        Object b = Comparator.naturalOrder();
        Object c = ((Comparator<String>) (x, y) -> 0).reversed();
        return (a instanceof String) + "," + (b instanceof CharSequence) + "," + (c instanceof String)
            + "," + (a instanceof Comparator) + "," + (c instanceof Comparator);
    }

    public static String cmpConcatLeak() {
        Object c = Comparator.comparing((String s) -> s.length());
        String s1 = "" + c;
        String s2 = String.valueOf(c);
        String s3 = c.toString();
        String s4 = new StringBuilder().append(c).toString();
        String s5 = String.format("%s", c);
        String s6 = List.of(c).toString();
        return leak(s1) + "," + leak(s2) + "," + leak(s3) + "," + leak(s4) + "," + leak(s5) + "," + leak(s6);
    }

    static boolean leak(String s) { return s.contains("@cmp") || s.contains("@indy") || s.contains("cb:"); }

    public static String cmpEqualsText() {
        Object n = Comparator.naturalOrder();
        Object r = Comparator.reverseOrder();
        return n.equals("@cmp:natural") + "," + "@cmp:natural".equals(n) + "," + Objects.equals(r, "@cmp:reverse:r@cmp:natural")
            + "," + "@cmp:natural".contentEquals((CharSequence) "@cmp:natural");
    }

    public static String cmpStringIsString() {
        Object s = "@cmp:natural";
        return (s instanceof String) + "," + s.getClass().getName() + "," + (s instanceof Comparator) + "," + s.hashCode();
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String cmpStringCastSort() {
        Object s = "@cmp:reverse:r@cmp:natural";
        return t(() -> {
            List<String> l = new ArrayList<>(List.of("a", "c", "b"));
            l.sort((Comparator) s);
            return String.join("", l);
        });
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String cmpStringCompare() {
        Object s = "@cmp:natural";
        return t(() -> ((Comparator) s).compare("a", "b"));
    }

    public static final class F implements Function<Object, Object> {
        final String secret;
        F(String secret) { this.secret = secret; }
        public Object apply(Object o) { ran = true; log = secret; return 0; }
    }

    /** The jar never holds `f` again; a String naming its key reaches it through `@cmp:key:`. */
    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String cmpForgeJarObject() {
        new F("hidden");
        ran = false;
        log = "";
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 8; n++) {
            Object s = "@cmp:key:hobj:demo/Rv$F#" + n;
            try {
                List<String> l = new ArrayList<>(List.of("b", "a"));
                l.sort((Comparator) s);
                out.append("sorted,");
            } catch (Throwable e) {
                out.append(e.getClass().getSimpleName()).append(',');
            }
        }
        return out + "ran=" + ran + ",log=" + log;
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String listForgeJarObject() {
        new F("hidden");
        ran = false;
        log = "";
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 8; n++) {
            Object s = "list/hobj:demo/Rv$F#" + n;
            try {
                Object got = ((List) s).get(0);
                ((Function) got).apply("x");
                out.append("got,");
            } catch (Throwable e) {
                out.append(e.getClass().getSimpleName()).append(',');
            }
        }
        return out + "ran=" + ran + ",log=" + log;
    }

    public static String listOfLambdaCast() {
        Runnable r = () -> {};
        Object l = List.of(r);
        return t(() -> {
            String text = readAll((CharSequence) l);
            String tag = tagOf(text);
            return tag == null ? "noTag" : forgeWith(tag);
        });
    }

    public static String entryOfLambdaCast() {
        Runnable r = () -> {};
        Object e = Map.entry("k", r);
        return t(() -> {
            String text = readAll((CharSequence) e);
            String tag = tagOf(text);
            return tag == null ? "noTag" : forgeWith(tag);
        });
    }

    public static String cmpHashCodeText() {
        Object c = Comparator.comparing((String s) -> s.length());
        Object d = Comparator.comparing((String s) -> s.length());
        return t(() -> (c.hashCode() == d.hashCode()) + "," + c.equals(d) + "," + (c == d));
    }

    // ---------- 2. identity ----------

    public static String ncSameSite() {
        Runnable[] a = new Runnable[2];
        for (int i = 0; i < 2; i++) a[i] = () -> {};
        Function<String, Integer>[] f = new Function[2];
        for (int i = 0; i < 2; i++) f[i] = String::length;
        Supplier<Object>[] s = new Supplier[2];
        for (int i = 0; i < 2; i++) s[i] = ArrayList::new;
        return (a[0] == a[1]) + "," + a[0].equals(a[1]) + "," + (f[0] == f[1]) + "," + (s[0] == s[1]);
    }

    static Runnable[] fin = new Runnable[3];
    static void finallySite(int i, int slot) {
        try {
            if (i == 0) return;
        } finally {
            fin[slot] = () -> {};
        }
    }

    public static String finallySites() {
        finallySite(0, 0);
        finallySite(1, 1);
        finallySite(1, 2);
        return (fin[0] == fin[1]) + "," + (fin[1] == fin[2]);
    }

    public static String twoSitesSameBody() {
        Runnable a = () -> {};
        Runnable b = () -> {};
        return (a == b) + "," + a.equals(b);
    }

    public static String capSameSite() {
        int x = 1;
        Runnable[] a = new Runnable[2];
        for (int i = 0; i < 2; i++) a[i] = () -> System.identityHashCode(x);
        return (a[0] == a[1]) + "," + a[0].equals(a[1]);
    }

    public static String capHashSet() {
        int x = 1;
        Set<Runnable> set = new HashSet<>();
        for (int i = 0; i < 2; i++) set.add(() -> System.identityHashCode(x));
        return String.valueOf(set.size());
    }

    public static String capHashSetDiff() {
        Set<Runnable> set = new HashSet<>();
        for (int i = 0; i < 3; i++) {
            int x = i;
            set.add(() -> System.identityHashCode(x));
        }
        return String.valueOf(set.size());
    }

    public static String ncHashSet() {
        Set<Runnable> set = new HashSet<>();
        for (int i = 0; i < 3; i++) set.add(() -> {});
        Runnable again = null;
        for (int i = 0; i < 1; i++) again = () -> {};
        return set.size() + "," + set.contains(again);
    }

    public static String lambdaMapKey() {
        Runnable r1 = () -> {};
        Runnable r2 = () -> {};
        Map<Object, String> m = new HashMap<>();
        m.put(r1, "a");
        m.put(r2, "b");
        return m.get(r1) + "," + m.get(r2) + "," + m.size() + "," + m.containsKey(r1);
    }

    public static String capIdentityMap() {
        int x = 1;
        Map<Object, Integer> m = new IdentityHashMap<>();
        for (int i = 0; i < 2; i++) m.put((Runnable) () -> System.identityHashCode(x), i);
        return String.valueOf(m.size());
    }

    public static String compositeIdentity() {
        Predicate<String> p = s -> true;
        Predicate<String> q = s -> false;
        Predicate<String> a = p.and(q);
        Predicate<String> b = p.and(q);
        Set<Object> set = new HashSet<>();
        set.add(a);
        set.add(b);
        return (a == b) + "," + a.equals(b) + "," + (a == a) + "," + a.equals(a) + "," + set.size();
    }

    public static String negateSetOf() {
        Predicate<String> p = s -> true;
        return t(() -> Set.of(p.negate(), p.negate()).size());
    }

    public static String fnIdentity() {
        Object a = Function.identity();
        Object b = Function.identity();
        Object c = UnaryOperator.identity();
        Object d = UnaryOperator.identity();
        return (a == b) + "," + (c == d) + "," + (a == c) + "," + a.equals(c);
    }

    final class Inner {
        Runnable make() { return () -> System.identityHashCode(this); }
    }

    public static String capThisTwice() {
        Inner in = new Rv().new Inner();
        Runnable a = in.make();
        Runnable b = in.make();
        return (a == b) + "," + a.equals(b);
    }

    public static String heldVsFresh() {
        Runnable a = held();
        Runnable b = held();
        Supplier<Runnable> s = Rv::held;
        return (a == b) + "," + (s.get() == a);
    }
    static Runnable held() { return () -> {}; }

    // ---------- 3. exactness ----------

    public static final class MyEx extends RuntimeException {
        MyEx(String m) { super(m); }
    }

    public static String andShort() {
        log = "";
        Predicate<String> p = s -> { log += "p"; throw new MyEx("pm"); };
        Predicate<String> q = s -> { log += "q"; return true; };
        return tx(() -> p.and(q).test("x")) + "," + log;
    }

    public static String andOrder() {
        log = "";
        Predicate<String> p = s -> { log += "p"; return true; };
        Predicate<String> q = s -> { log += "q"; return false; };
        Predicate<String> f = s -> { log += "f"; return false; };
        boolean a = p.and(q).test("x");
        boolean b = f.and(p).test("x");
        boolean c = p.or(q).test("x");
        boolean d = f.or(q).test("x");
        boolean e = p.negate().or(q.negate()).test("x");
        boolean g = p.and(q).negate().test("x");
        return a + "," + b + "," + c + "," + d + "," + e + "," + g + "," + log;
    }

    public static String orThrowsSecond() {
        log = "";
        Predicate<String> f = s -> { log += "f"; return false; };
        Predicate<String> q = s -> { log += "q"; throw new IllegalStateException("qm"); };
        return tx(() -> f.or(q).test("x")) + "," + log;
    }

    public static String negateThrows() {
        Predicate<String> p = s -> { throw new MyEx("neg"); };
        return tx(() -> p.negate().test("x"));
    }

    public static String consumerThrow() {
        log = "";
        Consumer<String> c1 = s -> { log += "1" + s; throw new MyEx("c1"); };
        Consumer<String> c2 = s -> log += "2" + s;
        String r = tx(() -> { c1.andThen(c2).accept("x"); return "ok"; });
        Consumer<String> c3 = s -> log += "3" + s;
        Consumer<String> c4 = s -> { log += "4" + s; throw new MyEx("c4"); };
        String r2 = tx(() -> { c3.andThen(c4).andThen(c2).accept("y"); return "ok"; });
        return r + "," + r2 + "," + log;
    }

    public static String fnOrder() {
        log = "";
        Function<String, String> f = s -> { log += "f"; return s + "f"; };
        Function<String, String> g = s -> { log += "g"; return s + "g"; };
        Function<String, String> h = s -> { log += "h"; throw new MyEx("h"); };
        String a = f.compose(g).apply("x");
        String b = f.andThen(g).apply("x");
        String c = tx(() -> f.compose(h).apply("x"));
        String d = tx(() -> h.andThen(f).apply("x"));
        String e = f.andThen(g).compose(f).andThen(f).apply("x");
        return a + "," + b + "," + c + "," + d + "," + e + "," + log;
    }

    public static String biOrder() {
        log = "";
        BiFunction<String, String, String> bf = (a, b) -> { log += "b"; return a + b; };
        BiFunction<String, String, String> bt = (a, b) -> { log += "t"; throw new MyEx("bt"); };
        Function<String, String> g = s -> { log += "g"; return s + "g"; };
        BiConsumer<String, String> c1 = (a, b) -> log += "1" + a + b;
        BiConsumer<String, String> c2 = (a, b) -> { log += "2"; throw new MyEx("c2"); };
        String x = bf.andThen(g).apply("p", "q");
        String y = tx(() -> bt.andThen(g).apply("p", "q"));
        String z = tx(() -> { c1.andThen(c2).andThen(c1).accept("u", "v"); return "ok"; });
        return x + "," + y + "," + z + "," + log;
    }

    static <T extends Throwable> void sneaky(Throwable t) throws T { throw (T) t; }

    public static String checkedThroughAnd() {
        Predicate<String> p = s -> { Rv.<RuntimeException>sneaky(new java.io.IOException("io")); return true; };
        try {
            p.and(s -> true).test("x");
            return "none";
        } catch (Throwable e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }

    public static String identityApply() {
        Function<Object, Object> id = Function.identity();
        UnaryOperator<String> u = UnaryOperator.identity();
        Object o = new Object();
        return (id.apply(o) == o) + "," + u.apply("z") + "," + id.andThen(x -> x + "!").apply("a")
            + "," + id.compose((String x) -> x + "?").apply("b");
    }

    // ---------- comparators that are lambdas ----------

    public static String cmpLambdaStable() {
        List<String> l = new ArrayList<>();
        for (int i = 0; i < 45; i++) l.add((char) ('a' + (i * 7) % 26) + "" + i);
        l.sort((a, b) -> Integer.compare(a.length(), b.length()));
        return String.join(",", l);
    }

    public static String cmpLambdaLog() {
        StringBuilder sb = new StringBuilder();
        List<Integer> l = new ArrayList<>(List.of(5, 3, 8, 1, 9, 2, 7));
        l.sort((a, b) -> { sb.append(a).append(b).append(' '); return Integer.compare(a, b); });
        return l + "|" + sb;
    }

    public static String cmpLambdaLogBig() {
        int[] calls = {0};
        long[] sum = {0};
        List<Integer> l = new ArrayList<>();
        for (int i = 0; i < 70; i++) l.add((i * 37 + 11) % 70);
        l.sort((a, b) -> { calls[0]++; sum[0] = sum[0] * 31 + a * 101 + b; return Integer.compare(a, b); });
        return calls[0] + "," + sum[0] + "," + l.get(0) + "," + l.get(69);
    }

    public static String cmpThrowsMidArrayList() {
        int[] n = {0};
        List<Integer> l = new ArrayList<>(List.of(5, 3, 8, 1, 9, 2, 7));
        String r = tx(() -> {
            l.sort((a, b) -> { if (++n[0] == 4) throw new MyEx("cmp4"); return Integer.compare(a, b); });
            return "ok";
        });
        return r + "," + l;
    }

    public static String cmpThrowsMidLinked() {
        int[] n = {0};
        List<Integer> l = new LinkedList<>(List.of(5, 3, 8, 1, 9, 2, 7));
        String r = tx(() -> {
            l.sort((a, b) -> { if (++n[0] == 4) throw new MyEx("cmp4"); return Integer.compare(a, b); });
            return "ok";
        });
        return r + "," + l;
    }

    public static String cmpContract() {
        StringBuilder out = new StringBuilder();
        for (int size : new int[] {50, 100, 500, 1000, 2000}) {
            List<Integer> l = new ArrayList<>();
            for (int i = 0; i < size; i++) l.add((i * 7919 + 13) % size);
            int[] st = {12345};
            int[] k = {0};
            out.append(size).append('=').append(tx(() -> {
                l.sort((a, b) -> { k[0]++; st[0] = st[0] * 1103515245 + 12345; return ((st[0] >>> 16) % 3) - 1; });
                return "ok:" + l.get(0) + ":" + l.get(size - 1);
            })).append(":k").append(k[0]).append(';');
        }
        return out.toString();
    }

    public static String cmpArraysSort() {
        StringBuilder sb = new StringBuilder();
        Integer[] a = {4, 1, 3, 1, 2};
        Arrays.sort(a, (x, y) -> { sb.append(x).append(y).append(' '); return x - y; });
        return Arrays.toString(a) + "|" + sb;
    }

    public static String cmpStreamSorted() {
        StringBuilder sb = new StringBuilder();
        List<String> out = Stream.of("ccc", "a", "bb", "dd", "e")
            .sorted((x, y) -> { sb.append(x).append(y).append(' '); return x.length() - y.length(); })
            .collect(Collectors.toList());
        return out + "|" + sb;
    }

    public static String cmpTreeMap() {
        StringBuilder sb = new StringBuilder();
        TreeMap<String, Integer> m = new TreeMap<>((x, y) -> { sb.append(x).append(y).append(' '); return y.compareTo(x); });
        m.put("b", 1);
        m.put("a", 2);
        m.put("c", 3);
        return m + "|" + sb;
    }

    public static String cmpMax() {
        StringBuilder sb = new StringBuilder();
        List<String> l = List.of("bb", "a", "ccc", "dd");
        String m1 = Collections.max(l, (x, y) -> { sb.append(x).append(y).append(' '); return x.length() - y.length(); });
        String m2 = l.stream().max((x, y) -> x.length() - y.length()).get();
        String m3 = l.stream().min((x, y) -> x.length() - y.length()).get();
        return m1 + "," + m2 + "," + m3 + "|" + sb;
    }

    public static String cmpPriorityQueue() {
        PriorityQueue<Integer> q = new PriorityQueue<>((a, b) -> b - a);
        q.addAll(List.of(3, 1, 4, 1, 5, 9, 2, 6));
        StringBuilder sb = new StringBuilder();
        while (!q.isEmpty()) sb.append(q.poll());
        return sb.toString();
    }

    public static String cmpLambdaCombined() {
        Comparator<String> byLen = (a, b) -> a.length() - b.length();
        List<String> l = new ArrayList<>(List.of("bb", "a", "ccc", "dd", "aa", "c"));
        l.sort(byLen.reversed().thenComparing(Comparator.reverseOrder()));
        List<String> m = new ArrayList<>(l);
        Comparator<Integer> rev = (a, b) -> b - a;
        m.sort(Comparator.comparing(String::length, rev).thenComparing((a, b) -> a.compareTo(b)));
        return l + "," + m;
    }

    public static String cmpReturnsNonInteger() {
        List<String> l = new ArrayList<>(List.of("b", "a", "c"));
        l.sort((a, b) -> a.compareTo(b) * 1000000000);
        return l.toString();
    }

    public static String cmpModifiesList() {
        List<Integer> l = new ArrayList<>(List.of(3, 2, 1));
        return tx(() -> { l.sort((a, b) -> { if (l.size() < 5) l.add(0); return a - b; }); return "ok"; }) + "," + l.size();
    }

    // ---------- 5. collector ----------

    static final class Node {
        final String v;
        Node(String v) { this.v = v; }
    }

    static Object hSup, hPred, hCmp, hNested, hNc;
    static List<Object> hList = new ArrayList<>();
    static Map<String, Object> hMap = new HashMap<>();

    public static String gcHold() {
        Node a = new Node("sup");
        hSup = (Supplier<String>) () -> a.v;
        Node b = new Node("pred");
        hPred = ((Predicate<String>) s -> s.equals(b.v)).and(s -> s.length() == 4);
        Node c = new Node("zz");
        hCmp = Comparator.comparing((String s) -> s.equals(c.v) ? 0 : 1);
        Node d = new Node("list");
        hList.add((Supplier<String>) () -> d.v);
        Node e = new Node("map");
        hMap.put("k", (Function<String, String>) s -> s + e.v);
        Node f = new Node("nested");
        Supplier<String> inner = () -> f.v;
        hNested = (Supplier<Supplier<String>>) () -> inner;
        hNc = ncSite();
        return "held";
    }

    public static String gcGarbage() {
        long sum = 0;
        for (int i = 0; i < 1500; i++) {
            Node g = new Node("g" + i);
            Supplier<String> s = () -> g.v;
            sum += s.get().length();
        }
        return String.valueOf(sum);
    }

    @SuppressWarnings("unchecked")
    public static String gcUse() {
        List<String> sorted = new ArrayList<>(List.of("a", "zz", "b"));
        sorted.sort((Comparator<String>) hCmp);
        return ((Supplier<String>) hSup).get() + "," + ((Predicate<String>) hPred).test("pred") + ","
            + ((Predicate<String>) hPred).test("xxxx") + "," + sorted + ","
            + ((Supplier<String>) hList.get(0)).get() + "," + ((Function<String, String>) hMap.get("k")).apply(">")
            + "," + ((Supplier<Supplier<String>>) hNested).get().get() + "," + (ncSite() == hNc);
    }
    static Runnable ncSite() { return () -> {}; }
}

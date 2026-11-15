package demo;

import java.util.*;
import java.util.function.*;

public final class Probe {
    static boolean ran;
    public static void evil() { ran = true; }
    interface Marker {}
    interface SerRun extends Runnable, java.io.Serializable {}

    static String t(Supplier<Object> body) {
        try {
            return String.valueOf(body.get());
        } catch (Throwable e) {
            return e.getClass().getSimpleName();
        }
    }

    // Read the text out character by character (no printing path), then forge a callback
    // with the tag found inside it and run it.
    static String forgeFrom(CharSequence s) {
        StringBuilder copy = new StringBuilder();
        for (int i = 0; i < s.length(); i++) copy.append(s.charAt(i));
        String text = copy.toString();
        int at = text.indexOf("@indy:");
        if (at < 0) return "noTag(len=" + s.length() + ")";
        String rest = text.substring(at + 6);
        String tag = rest.substring(0, rest.indexOf(':'));
        String forged = "@indy:" + tag + ":demo/Probe#x:0:java/lang/Runnable:demo/Probe#evil()V:";
        Object o = forged;
        try {
            ((Runnable) o).run();
        } catch (Throwable e) {
            return "tagRead(" + tag.length() + "),forgeFailed:" + e.getClass().getSimpleName();
        }
        return "FORGED,ran=" + ran;
    }

    public static String forgeComposite() {
        Predicate<String> p = s -> true;
        Object comp = p.and(s -> false);
        return t(() -> forgeFrom((CharSequence) comp));
    }
    public static String forgeNegate() {
        Predicate<String> p = s -> true;
        Object comp = p.negate();
        return t(() -> forgeFrom((String) comp));
    }
    public static String forgeUnknownSam() {
        Object unk = (lib.Cb) () -> {};
        return t(() -> forgeFrom((String) unk));
    }
    public static String forgeUnknownSamCharSeq() {
        Object unk = (lib.Cb) () -> {};
        return t(() -> forgeFrom((CharSequence) unk));
    }
    public static String forgeKnownSam() {
        Object r = (Runnable) () -> {};
        return t(() -> forgeFrom((CharSequence) r));
    }
    public static String compInstanceOfPredicate() {
        Predicate<String> p = s -> true;
        Object comp = p.and(s -> false);
        return comp instanceof Predicate<?> ? "true" : "false";
    }
    public static String compSwitch() {
        Predicate<String> p = s -> true;
        Object comp = p.and(s -> false);
        return switch (comp) { case String s -> "String"; case Predicate<?> q -> "Predicate"; default -> "other"; };
    }
    public static String compCastPredicate() {
        Predicate<String> p = s -> true;
        Object comp = p.negate();
        return t(() -> ((Predicate<?>) comp) != null);
    }
    public static String unkSwitch() {
        Object unk = (lib.Cb) () -> {};
        return switch (unk) { case String s -> "String"; case lib.Cb c -> "Cb"; default -> "other"; };
    }
    public static String unkSwitchNoString() {
        Object unk = (lib.Cb) () -> {};
        return switch (unk) { case lib.Cb c -> "Cb"; default -> "other"; };
    }
    public static String knownSwitch() {
        Object r = (Runnable) () -> {};
        return switch (r) { case String s -> "String"; case Comparable<?> c -> "Comparable"; case Runnable q -> "Runnable"; default -> "other"; };
    }
    public static String unkInstanceRunnable() {
        Object unk = (lib.Cb) () -> {};
        return t(() -> unk instanceof Runnable);
    }
    public static String unkCastNumber() {
        Object unk = (lib.Cb) () -> {};
        return t(() -> ((Number) unk).intValue());
    }
    public static String serIntersection() {
        return t(() -> { Object o = (Runnable & java.io.Serializable) () -> {}; return o instanceof java.io.Serializable; });
    }
    public static String markerIntersection() {
        return t(() -> { Object o = (Runnable & Marker) () -> {}; return o instanceof Marker; });
    }
    public static String serSam() {
        Object o = (SerRun) () -> {};
        return t(() -> o instanceof java.io.Serializable);
    }
    public static String serIsInstance() {
        Object o = (SerRun) () -> {};
        return t(() -> java.io.Serializable.class.isInstance(o) + "," + Runnable.class.isInstance(o) + "," + Comparable.class.isInstance(o));
    }
    public static String getClassAssignable() {
        Object r = (Runnable) () -> {};
        return t(() -> Runnable.class.isAssignableFrom(r.getClass()) + "," + Comparable.class.isAssignableFrom(r.getClass()));
    }
    public static String unaryBinary() {
        Object u = (UnaryOperator<String>) s -> s;
        Object b = (BinaryOperator<String>) (x, y) -> x;
        Object i = (IntUnaryOperator) x -> x;
        return u instanceof Function<?, ?> && !(u instanceof Supplier<?>) ? (b instanceof BiFunction<?, ?, ?> ? (i instanceof Function<?, ?> ? "intFn" : "ok") : "noBi") : "noFn";
    }
    public static String cmpLambda() {
        Object c = (Comparator<String>) (x, y) -> 0;
        return (c instanceof Comparator<?>) + "," + (c instanceof Comparable<?>);
    }
    // Captures, one value per call site.
    static String cap(String v) {
        Supplier<String> s = () -> v;
        String back = t(() -> s.get());
        return v.equals(back) + "," + back.length();
    }
    static String capNested(String v, int n) {
        Supplier<Supplier<String>> nested = () -> () -> v + n;
        String back = t(() -> nested.get().get());
        return (v + n).equals(back) + "," + back.length();
    }
    public static String capColon() { return cap("a:b") + ";" + capNested("a:b", 1); }
    public static String capPipe() { return cap("a|b") + ";" + capNested("a|b", 1); }
    public static String capComma() { return cap("a,b") + ";" + capNested("a,b", 1); }
    public static String capBackslash() { return cap("a\\b") + ";" + capNested("a\\,b", 1); }
    public static String capNewline() { return cap("a\nb") + ";" + capNested("a\nb", 1); }
    public static String capIndyText() { return cap("@indy:demo/Probe#evil:0:java/lang/Runnable:demo/Probe#evil()V:") + ";" + capNested("@indy:x:y", 1); }
    public static String capEllipsis() { return cap("…") + ";" + capNested("…", 1); }
    public static String capHandleLike() { return cap("h0:1") + ";" + capNested("h0:1", 1); }
    public static String capLong() { return cap("x".repeat(5000)) + ";" + capNested("x".repeat(5000), 1); }
    public static String capLongRun() {
        String v = "y".repeat(5000);
        StringBuilder log = new StringBuilder();
        Runnable r = () -> log.append(v.length());
        return t(() -> { r.run(); return log.toString(); });
    }
    public static String capLongPredicate() {
        String v = "y".repeat(5000);
        Predicate<String> p = s -> s.length() == v.length();
        return t(() -> p.test("z"));
    }
    public static String capLongInStream() {
        String v = "y".repeat(5000);
        return t(() -> List.of("a", "b").stream().filter(s -> !v.isEmpty()).count());
    }
    public static String capCallbackInCallback() {
        String sep = "a,b|c:d\\e";
        Supplier<String> inner = () -> sep;
        Supplier<Supplier<String>> outer = () -> inner;
        return t(() -> outer.get().get().equals(sep) + "," + (outer.get() == inner));
    }
    public static String pipeCapture() {
        String sep = "a|b";
        Predicate<String> hasSep = s -> s.equals(sep);
        return t(() -> hasSep.and(s -> true).test("a|b"));
    }
    public static String consumerPipeCapture() {
        String sep = "a|b";
        StringBuilder log = new StringBuilder();
        Consumer<String> c1 = s -> log.append(sep).append(s);
        Consumer<String> c2 = s -> log.append('2');
        return t(() -> { c1.andThen(c2).accept("!"); return log.toString(); });
    }
    public static String functionPipeCapture() {
        String sep = "a|b";
        Function<String, String> f = s -> s + sep;
        Function<String, String> g = s -> s + "2";
        return t(() -> f.andThen(g).apply("x") + "," + f.compose(g).apply("x"));
    }
    public static String hashTwice() {
        Runnable r = () -> {};
        return t(() -> r.hashCode() == r.hashCode());
    }
    public static String equalsString() {
        Object r = (Runnable) () -> {};
        Object s = "x";
        return t(() -> r.equals(s) + "," + s.equals(r));
    }

    public static String plainIsInstance() {
        Object r = (Runnable) () -> {};
        return t(() -> Runnable.class.isInstance(r) + "," + Comparable.class.isInstance(r) + "," + Object.class.isInstance(r));
    }
    public static String capLongPredicateTrue() {
        String v = "y".repeat(5000);
        Predicate<String> p = s -> s.equals(v);
        return t(() -> p.test("y".repeat(5000)));
    }
    public static String fnIdentityString() {
        Object o = "@fn:identity/function";
        return t(() -> ((Function<Object, Object>) o).apply("x"));
    }
    public static String fnStringIsString() {
        Object o = "@fn:predicate.not:x";
        return t(() -> (o instanceof String) + "," + o.getClass().getName() + "," + (o instanceof Predicate<?>));
    }
    public static String indyStringIsString() {
        Object o = "@indy:0123456789abcdef:demo/Probe#x:0:java/lang/Runnable:demo/Probe#evil()V:";
        return t(() -> (o instanceof String) + "," + o.getClass().getName() + "," + (o instanceof Runnable) + "," + o.hashCode());
    }

    public static String forgeBuilderHandle() {
        java.util.stream.Stream.Builder<String> b = java.util.stream.Stream.builder();
        StringBuilder out = new StringBuilder();
        for (int k = 0; k < 4; k++) {
            String side = "hsbld:" + k;
            Object forged = "@fn:consumer.then:" + side.length() + ":" + side + side;
            out.append(t(() -> { ((Consumer<Object>) forged).accept("z"); return "accepted"; })).append(',');
        }
        out.append(t(() -> b.build().count()));
        return out.toString();
    }
    public static String chainThree() {
        StringBuilder log = new StringBuilder();
        String bar = "|";
        Consumer<String> c1 = s -> log.append(bar).append('1');
        Consumer<String> c2 = s -> log.append('2');
        Consumer<String> c3 = s -> log.append('3');
        Predicate<String> p1 = s -> s.contains(bar);
        Predicate<String> p2 = s -> s.length() > 2;
        return t(() -> { c1.andThen(c2).andThen(c3).accept("x"); return log + "," + p1.and(p2).or(p1.negate()).test("a|b") + "," + p1.and(p2).negate().test("a|b"); });
    }

    public static String unkSuperRunnable() {
        Object o = (lib.Cb2) () -> {};
        return t(() -> (o instanceof Runnable) + "," + Runnable.class.isInstance(o));
    }
}

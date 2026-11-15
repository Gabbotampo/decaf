package demo;

import java.util.*;
import java.util.function.*;

/**
 * Lambdas as objects (#156 P15b): one object per call site for a lambda that captures nothing,
 * a new one per evaluation otherwise (HotSpot 25); method references run by their handle's kind;
 * what a lambda captures stays alive while the lambda does, however long its captures are.
 */
public final class Probe {
    static final class Box {
        int v;
        Box(int v) { this.v = v; }
        int twice() { return v * 2; }
    }

    static IntSupplier held;
    static int garbageRuns;

    static int twiceOf(int x) { return x * 2; }

    public static String identity() {
        Object[] plain = new Object[2], caps = new Object[2], ctor = new Object[2], unbound = new Object[2],
            bound = new Object[2], composed = new Object[2];
        String s = "hello";
        Predicate<String> p = x -> true;
        for (int i = 0; i < 2; i++) {
            String v = "x";
            plain[i] = (Runnable) () -> {};
            caps[i] = (Supplier<String>) () -> v;
            ctor[i] = (Supplier<StringBuilder>) StringBuilder::new;
            unbound[i] = (Function<String, Integer>) String::length;
            bound[i] = (Supplier<Integer>) s::length;
            composed[i] = p.and(p);
        }
        Object a = (Runnable) () -> {};
        Object b = (Runnable) () -> {};
        return "identity=" + (plain[0] == plain[1]) + ',' + (caps[0] == caps[1]) + ','
            + caps[0].equals(caps[1]) + ',' + caps[0].equals(caps[0]) + ',' + (a == b) + ','
            + (ctor[0] == ctor[1]) + ',' + (unbound[0] == unbound[1]) + ',' + (bound[0] == bound[1]) + ','
            + (composed[0] == composed[1]) + ',' + (Function.identity() == Function.identity()) + ','
            + ((Object) UnaryOperator.identity() == (Object) Function.identity());
    }

    public static String collections() {
        Object[] caps = new Object[3];
        Object site = null;
        for (int i = 0; i < 3; i++) {
            int n = i;
            caps[i] = (IntSupplier) () -> n;
            site = (Runnable) () -> {};
        }
        List<Object> list = new ArrayList<>(Arrays.asList(caps));
        IdentityHashMap<Object, String> byIdentity = new IdentityHashMap<>();
        for (Object o : caps) byIdentity.put(o, "v");
        byIdentity.put(caps[0], "again");
        return "collections=" + list.indexOf(caps[2]) + ',' + list.contains(caps[1]) + ','
            + list.contains(site) + ',' + byIdentity.size() + ',' + byIdentity.get(caps[0]);
    }

    public static String dispatch() {
        Function<String, Integer> parse = Integer::parseInt;
        Function<String, Integer> length = String::length;
        String s = "hello";
        Supplier<Integer> bound = s::length;
        Function<String, StringBuilder> make = StringBuilder::new;
        BiFunction<Object, Object, Boolean> equal = Objects::equals;
        IntUnaryOperator jarStatic = Probe::twiceOf;
        Function<Box, Integer> jarUnbound = Box::twice;
        IntFunction<Box> jarNew = Box::new;
        Box box = new Box(21);
        IntSupplier jarBound = box::twice;
        return "dispatch=" + parse.apply("12") + ',' + length.apply("abc") + ',' + bound.get() + ','
            + make.apply("ab").append('c') + ',' + equal.apply("a", "a") + ',' + jarStatic.applyAsInt(4) + ','
            + jarUnbound.apply(box) + ',' + jarNew.apply(7).v + ',' + jarBound.getAsInt();
    }

    /** A lambda capturing a long string, then an object. */
    public static String setup() {
        String big = "y".repeat(5000);
        Box box = new Box(42);
        held = () -> big.length() + box.v;
        return "setup";
    }

    /** Objects nothing keeps: enough of them to make the host collect. */
    public static String garbage() {
        Box[] boxes = new Box[700];
        for (int i = 0; i < boxes.length; i++) boxes[i] = new Box(i);
        garbageRuns++;
        return "garbage";
    }

    public static String check() {
        return "held=" + held.getAsInt();
    }

    public static void main(String[] args) {
        System.out.println(identity());
        System.out.println(collections());
        System.out.println(dispatch());
        setup();
        for (int i = 0; i < 5; i++) garbage();
        System.out.println(check());
    }
}

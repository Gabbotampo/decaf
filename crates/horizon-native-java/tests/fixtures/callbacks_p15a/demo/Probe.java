package demo;

import java.util.function.*;

/**
 * A lambda is what Java 25 makes it, and a String spelled like one is a String (#156 P15a).
 * `exact()` prints Java's own answers; `counted()` the ones whose Java text names a hidden
 * lambda class, which the runtime counts.
 */
public final class Probe {
    static boolean ran;

    public static void evil() { ran = true; }

    interface Op extends Runnable {}

    static String t(Supplier<Object> body) {
        try {
            return String.valueOf(body.get());
        } catch (RuntimeException e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }

    public static String exact() {
        StringBuilder out = new StringBuilder();
        // A String spelled like the runtime's callback text: a String, never run.
        Object forged = "@indy:demo/Probe#exact:0:java/lang/Runnable:demo/Probe#evil()V:";
        String cast = t(() -> { ((Runnable) forged).run(); return "ran"; });
        out.append("forged=").append(forged instanceof Runnable).append(',').append(cast).append(',')
            .append(ran).append(',').append(forged).append(',').append(forged.getClass().getName()).append('\n');
        // A lambda implements its own functional interface and that interface's supertypes only.
        Object r = (Runnable) () -> {};
        Object u = (UnaryOperator<String>) s -> s;
        Object op = (Op) () -> {};
        out.append("instanceof=").append(r instanceof Runnable).append(',').append(r instanceof Comparable)
            .append(',').append(r instanceof java.io.Serializable).append(',').append(u instanceof Function)
            .append(',').append(u instanceof Supplier).append(',').append(op instanceof Runnable)
            .append(',').append(op instanceof Comparable).append('\n');
        // `Objects.requireNonNull(other)` in every combinator.
        Predicate<String> p = s -> true;
        Consumer<String> c = s -> {};
        Function<String, String> f = s -> s;
        out.append("nulls=").append(t(() -> p.and(null))).append(',').append(t(() -> p.or(null))).append(',')
            .append(t(() -> c.andThen(null))).append(',').append(t(() -> f.compose(null))).append(',')
            .append(t(() -> f.andThen(null))).append('\n');
        // Composites run their sides in the combinator's order (#258), `and`/`or`
        // short-circuiting, through left-nested chains and a captured `|`.
        Predicate<Integer> yes = i -> true;
        Predicate<Integer> no = i -> false;
        java.util.List<String> log = new java.util.ArrayList<>();
        Predicate<Integer> loud = i -> log.add("q" + i);
        Consumer<Integer> c1 = i -> log.add("a" + i);
        Consumer<Integer> c2 = i -> log.add("b" + i);
        Consumer<Integer> c3 = i -> log.add("c" + i);
        String bar = "x|y";
        Predicate<String> has = s -> s.contains(bar);
        Function<String, String> f1 = s -> s + bar;
        Function<String, String> f2 = s -> "[" + s + "]";
        BiConsumer<String, String> b1 = (x, y) -> log.add(x + y);
        BiFunction<String, String, String> join = (x, y) -> x + y;
        out.append("composed=").append(yes.and(no).test(1)).append(',').append(no.negate().test(1))
            .append(',').append(no.or(yes).test(1)).append(',').append(yes.and(yes).and(no.negate()).test(1))
            .append(',').append(no.and(loud).test(2)).append(',').append(yes.or(loud).test(3))
            .append(',').append(no.or(loud).test(4)).append(',').append(has.and(has.negate()).test("x|y"))
            .append(',').append(has.or(has).test("x|y")).append('\n');
        c1.andThen(c2).andThen(c3).accept(1);
        c1.andThen(c2.andThen(c3)).accept(2);
        b1.andThen(b1).accept("p", "q");
        out.append("chains=").append(log).append(',').append(f1.andThen(f2).andThen(f2).apply("a"))
            .append(',').append(f2.compose(f1).compose(f2).apply("z")).append(',')
            .append(join.andThen(f2).andThen(f1).apply("m", "n")).append('\n');
        return out.toString();
    }

    public static String counted() {
        Object r = (Runnable) () -> {};
        String cast;
        try {
            Object c = (Comparable<?>) r;
            cast = "cast";
        } catch (ClassCastException e) {
            cast = "ClassCastException";
        }
        String text = "" + r;
        return cast + "," + text.contains("@indy");
    }

    public static void main(String[] args) {
        System.out.print(exact());
        System.out.println("counted=" + counted());
    }
}

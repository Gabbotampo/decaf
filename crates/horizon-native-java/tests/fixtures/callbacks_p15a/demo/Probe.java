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

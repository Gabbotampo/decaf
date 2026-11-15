package demo;
import java.util.*;
public class Probe {
    static List<String> backing(int kind) {
        List<String> values = Arrays.asList("a", "b", "c", "d");
        if (kind == 0) return new ArrayList<>(values);
        if (kind == 1) return new LinkedList<>(values);
        return values;
    }
    public static String probe(int kind, int operation) {
        List<String> source = backing(kind);
        List<String> view = Collections.unmodifiableList(source);
        try {
            if (operation == 0) view.subList(3, 1);
            if (operation == 1) view.subList(-1, 2);
            if (operation == 2) view.subList(0, 9);
            if (operation == 3) view.subList(1, 3).set(0, "x");
            if (operation == 4) {
                List<String> part = view.subList(1, 3).subList(0, 1);
                source.set(1, "changed");
                return part.get(0) + ":" + part.size();
            }
            return "no exception";
        } catch (RuntimeException e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }
    // The wrapper `subList` makes is `new <the receiver's class>(part)`, never
    // `Collections.unmodifiableList(part)`: its class does not follow the
    // part's `RandomAccess`, it is new even when the part already is an
    // unmodifiable list, and a `null` part throws NPE (#251).
    static List<String> last;
    static class Seq extends AbstractList<String> {
        final int mode;
        Seq(int mode) { this.mode = mode; }
        public String get(int i) { return "s" + i; }
        public int size() { return 4; }
        public List<String> subList(int from, int to) {
            if (mode == 0) return last = new ArrayList<>(List.of("x", "y"));
            if (mode == 1) return last = Collections.unmodifiableList(new ArrayList<>(List.of("x")));
            return last = null;
        }
    }
    static class Ra extends Seq implements RandomAccess {
        Ra() { super(0); }
        public List<String> subList(int from, int to) { return last = new LinkedList<>(List.of("x")); }
    }
    public static String wrap(int op) {
        try {
            List<String> view = Collections.unmodifiableList(op == 3 ? new Ra() : new Seq(op));
            List<String> part = view.subList(0, 1);
            return view.getClass().getSimpleName() + "|" + part.getClass().getSimpleName() + "|"
                + (part instanceof RandomAccess) + "|" + (part == last) + "|" + part.size();
        } catch (RuntimeException e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }
    public static void main(String[] args) {
        for (int kind = 0; kind < 3; kind++)
            for (int op = 0; op < 5; op++)
                System.out.println(kind + "." + op + "=" + probe(kind, op));
        for (int op = 0; op < 4; op++)
            System.out.println("w." + op + "=" + wrap(op));
    }
}

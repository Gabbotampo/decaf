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
    public static void main(String[] args) {
        for (int kind = 0; kind < 3; kind++)
            for (int op = 0; op < 5; op++)
                System.out.println(kind + "." + op + "=" + probe(kind, op));
    }
}

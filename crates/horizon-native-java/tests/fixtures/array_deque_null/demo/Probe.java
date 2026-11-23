package demo;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;

/**
 * `ArrayDeque` rejects null elements: every insertion of null on a fresh
 * deque throws `NullPointerException`, `addAll` of a list containing null
 * fails part-way (elements before the null stay), and the collection
 * constructor throws too. `contains(null)`, `remove(null)` and
 * `removeFirstOccurrence(null)` answer `false` without throwing.
 * <p>
 * One `<row>=<value>` line per case. For an exception the value is the
 * exception class, the message (`null` when there is none), then the deque's
 * size and `toString` afterwards; the collection constructor has no live
 * deque afterwards, so it prints only the exception.
 */
public final class Probe {
    interface Throwing {
        Object run();
    }

    public static void main(String[] unused) {
        System.out.println(add());
        System.out.println(addFirst());
        System.out.println(addLast());
        System.out.println(offer());
        System.out.println(offerFirst());
        System.out.println(offerLast());
        System.out.println(push());
        System.out.println(addAll());
        System.out.println(addAllPrefix());
        System.out.println(constructor());
        System.out.println(containsNull());
        System.out.println(removeNull());
        System.out.println(removeFirstOccurrenceNull());
        System.out.println(addValue());
        System.out.println(addAllOk());
    }

    static String row(String name, ArrayDeque<String> deque, Throwing action) {
        try {
            Object result = action.run();
            return name + "=" + String.valueOf(result) + "|" + deque.size() + "|" + deque;
        } catch (RuntimeException exception) {
            return name + "=" + exception.getClass().getName() + " "
                + String.valueOf(exception.getMessage()) + "|" + deque.size() + "|" + deque;
        }
    }

    public static String add() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("add", deque, () -> Boolean.valueOf(deque.add(null)));
    }

    public static String addFirst() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("addFirst", deque, () -> { deque.addFirst(null); return null; });
    }

    public static String addLast() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("addLast", deque, () -> { deque.addLast(null); return null; });
    }

    public static String offer() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("offer", deque, () -> Boolean.valueOf(deque.offer(null)));
    }

    public static String offerFirst() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("offerFirst", deque, () -> { deque.offerFirst(null); return null; });
    }

    public static String offerLast() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("offerLast", deque, () -> { deque.offerLast(null); return null; });
    }

    public static String push() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("push", deque, () -> { deque.push(null); return null; });
    }

    public static String addAll() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        List<String> withNull = new ArrayList<>();
        withNull.add("a");
        withNull.add(null);
        withNull.add("b");
        return row("addAll", deque, () -> Boolean.valueOf(deque.addAll(withNull)));
    }

    public static String addAllPrefix() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.add("x");
        List<String> withNull = new ArrayList<>();
        withNull.add(null);
        withNull.add("y");
        return row("addAllPrefix", deque, () -> Boolean.valueOf(deque.addAll(withNull)));
    }

    public static String constructor() {
        List<String> withNull = new ArrayList<>();
        withNull.add("a");
        withNull.add(null);
        withNull.add("b");
        try {
            new ArrayDeque<>(withNull);
            return "constructor=ok";
        } catch (RuntimeException exception) {
            return "constructor=" + exception.getClass().getName() + " "
                + String.valueOf(exception.getMessage());
        }
    }

    public static String containsNull() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.add("a");
        return row("containsNull", deque, () -> Boolean.valueOf(deque.contains(null)));
    }

    public static String removeNull() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.add("a");
        return row("removeNull", deque, () -> Boolean.valueOf(deque.remove(null)));
    }

    public static String removeFirstOccurrenceNull() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.add("a");
        return row("removeFirstOccurrenceNull", deque,
                () -> Boolean.valueOf(deque.removeFirstOccurrence(null)));
    }

    public static String addValue() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        return row("addValue", deque, () -> Boolean.valueOf(deque.add("a")));
    }

    public static String addAllOk() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        List<String> values = new ArrayList<>();
        values.add("a");
        values.add("b");
        return row("addAllOk", deque, () -> Boolean.valueOf(deque.addAll(values)));
    }

    private Probe() {
    }
}

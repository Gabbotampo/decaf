package demo;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.ConcurrentModificationException;
import java.util.Deque;
import java.util.List;
import java.util.Queue;
import java.util.concurrent.ConcurrentLinkedDeque;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.function.Supplier;

/**
 * `ArrayDeque` (and `ConcurrentLinkedQueue`/`ConcurrentLinkedDeque`) reject
 * null elements: every insertion of null throws `NullPointerException`,
 * whether the call names the class or an interface it implements. `addAll`
 * of a collection holding a null fails part-way on an `ArrayDeque` (what
 * precedes the null stays) and adds nothing on the two concurrent queues; the
 * collection constructor throws too. `contains(null)`, `remove(null)` and
 * `removeFirstOccurrence(null)` answer `false` without throwing.
 * <p>
 * One `<row>=<value>` line per case. For an exception the value is the
 * exception class, the message (`null` when there is none), then the
 * collection's size and `toString` afterwards; a constructor has no live
 * collection afterwards, so it prints only the exception.
 */
public final class Probe {
    private Probe() {
    }

    public static void main(String[] unused) throws Exception {
        List<String> rows = new ArrayList<>();
        for (java.lang.reflect.Method method : Probe.class.getDeclaredMethods()) {
            if (method.getParameterCount() == 0 && method.getReturnType() == String.class
                    && java.lang.reflect.Modifier.isPublic(method.getModifiers())) {
                rows.add((String) method.invoke(null));
            }
        }
        rows.sort(null);
        rows.forEach(System.out::println);
    }

    /** `name=<result or exception>|size|toString`, after running `action`. */
    static String row(String name, Collection<String> collection, Supplier<Object> action) {
        String outcome;
        try {
            outcome = String.valueOf(action.get());
        } catch (RuntimeException exception) {
            outcome = exception.getClass().getName() + " " + exception.getMessage();
        }
        return name + "=" + outcome + "|" + collection.size() + "|" + collection;
    }

    static String constructed(String name, Supplier<Object> action) {
        try {
            action.get();
            return name + "=ok";
        } catch (RuntimeException exception) {
            return name + "=" + exception.getClass().getName() + " " + exception.getMessage();
        }
    }

    static List<String> aNullB() {
        return new ArrayList<>(Arrays.asList("a", null, "b"));
    }

    static List<String> nullY() {
        return new ArrayList<>(Arrays.asList(null, "y"));
    }

    // The class named at the call site: `invokevirtual ArrayDeque.m`.
    public static String add() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("add", d, () -> d.add(null));
    }

    public static String addFirst() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("addFirst", d, () -> { d.addFirst(null); return "void"; });
    }

    public static String addLast() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("addLast", d, () -> { d.addLast(null); return "void"; });
    }

    public static String offer() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("offer", d, () -> d.offer(null));
    }

    public static String offerFirst() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("offerFirst", d, () -> d.offerFirst(null));
    }

    public static String offerLast() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("offerLast", d, () -> d.offerLast(null));
    }

    public static String push() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("push", d, () -> { d.push(null); return "void"; });
    }

    public static String addAll() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("addAll", d, () -> d.addAll(aNullB()));
    }

    public static String addAllPrefix() {
        ArrayDeque<String> d = new ArrayDeque<>(List.of("x"));
        return row("addAllPrefix", d, () -> d.addAll(nullY()));
    }

    public static String constructor() {
        return constructed("constructor", () -> new ArrayDeque<>(aNullB()));
    }

    public static String containsNull() {
        ArrayDeque<String> d = new ArrayDeque<>(List.of("a"));
        return row("containsNull", d, () -> d.contains(null));
    }

    public static String removeNull() {
        ArrayDeque<String> d = new ArrayDeque<>(List.of("a"));
        return row("removeNull", d, () -> d.remove(null));
    }

    public static String removeFirstOccurrenceNull() {
        ArrayDeque<String> d = new ArrayDeque<>(List.of("a"));
        return row("removeFirstOccurrenceNull", d, () -> d.removeFirstOccurrence(null));
    }

    // Non-null neighbours.
    public static String addValue() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("addValue", d, () -> d.add("a"));
    }

    public static String addAllOk() {
        ArrayDeque<String> d = new ArrayDeque<>();
        return row("addAllOk", d, () -> d.addAll(List.of("a", "b")));
    }

    // The interface named at the call site: `invokeinterface Deque.m`.
    public static String dequeAdd() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeAdd", d, () -> d.add(null));
    }

    public static String dequeAddFirst() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeAddFirst", d, () -> { d.addFirst(null); return "void"; });
    }

    public static String dequeAddLast() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeAddLast", d, () -> { d.addLast(null); return "void"; });
    }

    public static String dequeOffer() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeOffer", d, () -> d.offer(null));
    }

    public static String dequeOfferFirst() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeOfferFirst", d, () -> d.offerFirst(null));
    }

    public static String dequeOfferLast() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeOfferLast", d, () -> d.offerLast(null));
    }

    public static String dequePush() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequePush", d, () -> { d.push(null); return "void"; });
    }

    public static String dequeAddAll() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeAddAll", d, () -> d.addAll(aNullB()));
    }

    public static String queueAdd() {
        Queue<String> q = new ArrayDeque<>();
        return row("queueAdd", q, () -> q.add(null));
    }

    public static String queueOffer() {
        Queue<String> q = new ArrayDeque<>();
        return row("queueOffer", q, () -> q.offer(null));
    }

    public static String collectionAdd() {
        Collection<String> c = new ArrayDeque<>();
        return row("collectionAdd", c, () -> c.add(null));
    }

    public static String collectionAddAll() {
        Collection<String> c = new ArrayDeque<>(List.of("x"));
        return row("collectionAddAll", c, () -> c.addAll(aNullB()));
    }

    public static String dequeAddAllOnlyNull() {
        Deque<String> d = new ArrayDeque<>(List.of("x"));
        return row("dequeAddAllOnlyNull", d, () -> d.addAll(Arrays.asList((String) null)));
    }

    // Non-null neighbours through an interface.
    public static String dequeAddValue() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeAddValue", d, () -> d.add("a"));
    }

    public static String dequeOfferValue() {
        Deque<String> d = new ArrayDeque<>();
        return row("dequeOfferValue", d, () -> d.offer("a"));
    }

    public static String collectionAddAllOk() {
        Collection<String> c = new ArrayDeque<>();
        return row("collectionAddAllOk", c, () -> c.addAll(List.of("a", "b")));
    }

    // The two concurrent queues.
    public static String clqAdd() {
        Queue<String> q = new ConcurrentLinkedQueue<>();
        return row("clqAdd", q, () -> q.add(null));
    }

    public static String clqOffer() {
        Queue<String> q = new ConcurrentLinkedQueue<>();
        return row("clqOffer", q, () -> q.offer(null));
    }

    public static String clqAddAll() {
        Queue<String> q = new ConcurrentLinkedQueue<>();
        return row("clqAddAll", q, () -> q.addAll(aNullB()));
    }

    public static String clqConstructor() {
        return constructed("clqConstructor", () -> new ConcurrentLinkedQueue<>(aNullB()));
    }

    public static String cldAdd() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldAdd", d, () -> d.add(null));
    }

    public static String cldAddFirst() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldAddFirst", d, () -> { d.addFirst(null); return "void"; });
    }

    public static String cldAddLast() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldAddLast", d, () -> { d.addLast(null); return "void"; });
    }

    public static String cldOffer() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldOffer", d, () -> d.offer(null));
    }

    public static String cldOfferFirst() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldOfferFirst", d, () -> d.offerFirst(null));
    }

    public static String cldOfferLast() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldOfferLast", d, () -> d.offerLast(null));
    }

    public static String cldPush() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldPush", d, () -> { d.push(null); return "void"; });
    }

    public static String cldAddAll() {
        Deque<String> d = new ConcurrentLinkedDeque<>();
        return row("cldAddAll", d, () -> d.addAll(aNullB()));
    }

    public static String cldConstructor() {
        return constructed("cldConstructor", () -> new ConcurrentLinkedDeque<>(aNullB()));
    }

    // A failed `addAll` has added elements and, here, grown the array first
    // (sliding the deque's first leg, which wraps, to the end of the new
    // array): a `forEach` that reads the live array afterwards must not be
    // answered from a layout that predates it.
    public static String forEachAfterFailedAddAll() {
        ArrayDeque<Integer> d = new ArrayDeque<>();
        for (int i = 1; i <= 8; i++) {
            d.addFirst(i);
        }
        try {
            d.addAll(Arrays.asList(11, 12, 13, 14, 15, 16, 17, 18, 19, null));
        } catch (NullPointerException expected) {
            // the elements before the null stay
        }
        StringBuilder seen = new StringBuilder();
        try {
            d.forEach(x -> {
                seen.append(x == null ? "n" : String.valueOf(x)).append(',');
                if (x != null && x == 8) {
                    d.pollLast();
                }
            });
            seen.append("no");
        } catch (ConcurrentModificationException exception) {
            seen.append("CME");
        }
        return "forEachAfterFailedAddAll=" + seen + d;
    }
}

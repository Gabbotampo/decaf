package demo;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.RandomAccess;

/**
 * `Arrays.asList` is a fixed-size `Arrays$ArrayList` view over its array (#156 P14d2): writes go through both ways,
 * `set`/`sort`/`replaceAll` are allowed, and the structural members throw `UnsupportedOperationException` exactly
 * where JDK 25's `AbstractList`/`AbstractCollection` reach `add`/`remove`.
 */
public final class Probe {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    interface Step {
        Object run();
    }

    static String uoe(Step step) {
        try {
            return String.valueOf(step.run());
        } catch (UnsupportedOperationException e) {
            return e.getMessage() == null ? "UOE" : "UOE:" + e.getMessage();
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName() + (e.getMessage() == null ? "" : ":" + e.getMessage());
        }
    }

    /** Array and list writes show on the other side, and two views of one array share it. */
    public static String writeThrough() {
        String[] array = {"a", "b", "c"};
        List<String> view = Arrays.asList(array);
        List<String> other = Arrays.asList(array);
        array[0] = "x";
        String afterArray = view.get(0) + other.get(0);
        String old = view.set(1, "y");
        String afterSet = array[1] + other.get(1) + old;
        System.arraycopy(new String[] {"p", "q"}, 0, array, 1, 2);
        return afterArray + "|" + afterSet + "|" + view + other + "|" + b(view == other) + b(view.equals(other))
            + view.size();
    }

    /** The structural members throw where `AbstractList`/`AbstractCollection` reach `add` or `remove`. */
    public static String fixedSize() {
        List<String> view = Arrays.asList("a", "b");
        List<String> none = Arrays.asList(new String[0]);
        return uoe(() -> view.add("c")) + "," + uoe(() -> { view.add(0, "c"); return "ok"; }) + ","
            + uoe(() -> view.remove(0)) + "," + uoe(() -> view.remove("z")) + "," + uoe(() -> view.remove("a")) + ","
            + uoe(() -> view.addAll(List.of())) + "," + uoe(() -> view.addAll(List.of("c"))) + ","
            + uoe(() -> view.removeIf(s -> s.equals("z"))) + "," + uoe(() -> view.removeIf(s -> s.equals("b"))) + ","
            + uoe(() -> view.retainAll(List.of("a", "b"))) + "," + uoe(() -> view.removeAll(List.of("a"))) + ","
            + uoe(() -> { none.clear(); return "cleared"; }) + "," + uoe(() -> { view.clear(); return "cleared"; })
            + "," + uoe(() -> none.removeFirst()) + "," + uoe(() -> view.removeLast()) + "|" + view;
    }

    /** `sort` and `set` through the view reach the array, and the array's writes reach the view. */
    public static String reorder() {
        String[] array = {"c", "a", "b"};
        List<String> view = Arrays.asList(array);
        Collections.sort(view);
        String sorted = String.join("", array);
        for (int i = 0; i < view.size(); i++) {
            view.set(i, view.get(i).toUpperCase());
        }
        String upper = String.join("", array);
        for (int i = 0, j = view.size() - 1; i < j; i++, j--) {
            view.set(i, view.set(j, view.get(i)));
        }
        String reversed = String.join("", array);
        String viewed = view.toString();
        for (int i = 0; i < array.length; i++) {
            array[i] = "f";
        }
        return sorted + "," + upper + "," + reversed + "," + viewed + "," + String.join("", view);
    }

    /** Its class, its `List` text and hash, and readers that copy it see the array as it is now. */
    public static String asAList() {
        String[] array = {"a", "b"};
        List<String> view = Arrays.asList(array);
        List<List<String>> outer = List.of(view);
        array[1] = "z";
        List<String> copy = new ArrayList<>(view);
        return view.getClass().getName() + "|" + b(view instanceof RandomAccess) + b(view.equals(List.of("a", "z")))
            + b(List.of("a", "z").equals(view)) + b(view.hashCode() == List.of("a", "z").hashCode()) + "|" + outer
            + copy + "|" + view.indexOf("z") + b(view.contains("a")) + Arrays.asList().size();
    }

    /** `set` checks the value against the array's component type, as `aastore` does. */
    public static String storeCheck() {
        String[] s = {"a"};
        List<Object> l = Arrays.asList((Object[]) s);
        Object[] ints = new Integer[] {2, 1};
        List<Object> li = Arrays.asList(ints);
        return uoe(() -> l.set(0, 1)) + "," + s[0] + l.get(0) + s[0].length() + "," + uoe(() -> li.set(0, "s")) + ","
            + ints[0] + "," + uoe(() -> li.set(1, 7)) + ints[1];
    }

    /** A view's `subList` is a window on the same array: `set` writes through, size changes throw. */
    public static String subLists() {
        String[] a = {"a", "b", "c"};
        List<String> v = Arrays.asList(a);
        List<String> s = v.subList(1, 3);
        String set = uoe(() -> s.set(0, "B")) + a[1] + v.get(1) + s;
        a[2] = "C";
        String seen = s.toString() + s.get(1);
        return set + "," + seen + "," + uoe(() -> { s.clear(); return "cleared"; }) + v + s + ","
            + uoe(() -> v.subList(0, 1).add("z")) + "," + uoe(() -> v.subList(0, 2).removeIf(x -> x.equals("a")))
            + "," + uoe(() -> s.remove(0)) + "," + uoe(() -> { v.subList(0, 0).clear(); return "ok"; }) + ","
            + uoe(() -> s.get(5)) + "," + s.subList(1, 2) + "," + uoe(() -> v.subList(2, 1)) + ","
            + s.getClass().getName() + "," + v + v.size();
    }

    static final class Key {
        final int k;

        Key(int k) {
            this.k = k;
        }

        @Override
        public boolean equals(Object o) {
            return o instanceof Key other && other.k == k;
        }

        @Override
        public int hashCode() {
            return k;
        }
    }

    /** Elements with their own `equals`: the size stays fixed whichever path compares them. */
    public static String jarEquals() {
        List<Key> v = Arrays.asList(new Key(1), new Key(2));
        return uoe(() -> v.remove(new Key(1))) + v.size() + v.get(0).k + "," + uoe(() -> v.remove(new Key(3))) + ","
            + uoe(() -> v.removeAll(List.of(new Key(2)))) + v.size() + ","
            + uoe(() -> v.retainAll(List.of(new Key(1), new Key(2)))) + "," + uoe(() -> v.retainAll(List.of(new Key(1))))
            + v.size() + "," + b(v.contains(new Key(2))) + v.indexOf(new Key(2));
    }

    /** Iteration reads the array live, and out of range is the array's own exception. */
    public static String live() {
        String[] a = {"a", "b", "c"};
        List<String> v = Arrays.asList(a);
        StringBuilder sb = new StringBuilder();
        for (String x : v) {
            a[1] = "q";
            sb.append(x);
        }
        a[1] = "b";
        java.util.Iterator<String> it = v.iterator();
        a[0] = "z";
        sb.append(',').append(it.next()).append(',');
        a[0] = "a";
        for (String x : v) {
            v.set(2, "w");
            sb.append(x);
        }
        a[2] = "c";
        sb.append(',');
        v.forEach(x -> {
            a[2] = "w";
            sb.append(x);
        });
        Object[] r = {"a", "b"};
        List<Object> rv = Arrays.asList(r);
        java.lang.reflect.Array.set(r, 0, "r");
        return sb + "," + rv.get(0) + rv + "," + uoe(() -> v.get(5)) + "," + uoe(() -> v.set(3, "x")) + ","
            + uoe(() -> v.get(-1));
    }

    static List<String> heldView;
    static List<String> heldWindow;

    /** Keep only a view and a window of a fresh array: the array itself is held by nothing else. */
    public static String hold() {
        String[] a = {"h", "i", "j"};
        heldView = Arrays.asList(a);
        heldWindow = heldView.subList(1, 3);
        return "held";
    }

    /** Garbage: enough objects for collections to run between calls. */
    public static String garbage() {
        int sum = 0;
        for (int i = 0; i < 3000; i++) {
            int[] box = {i};
            sum += box[0];
        }
        return "g" + (sum & 1);
    }

    /** The held view and window still share their array: a `set` through one shows in the other. */
    public static String afterCollections() {
        heldWindow.set(0, "I");
        return heldView + "" + heldWindow + heldView.get(1) + uoe(() -> heldView.add("k")) + heldView.getClass().getName();
    }

    public static void main(String[] args) {
        System.out.println("writeThrough=" + writeThrough());
        System.out.println("fixedSize=" + fixedSize());
        System.out.println("reorder=" + reorder());
        System.out.println("asAList=" + asAList());
        System.out.println("storeCheck=" + storeCheck());
        System.out.println("subLists=" + subLists());
        System.out.println("jarEquals=" + jarEquals());
        System.out.println("live=" + live());
        hold();
        for (int i = 0; i < 40; i++) {
            garbage();
        }
        System.out.println("afterCollections=" + afterCollections());
    }
}

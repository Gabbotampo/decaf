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
            return "UOE";
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName();
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

    public static void main(String[] args) {
        System.out.println("writeThrough=" + writeThrough());
        System.out.println("fixedSize=" + fixedSize());
        System.out.println("reorder=" + reorder());
        System.out.println("asAList=" + asAList());
    }
}

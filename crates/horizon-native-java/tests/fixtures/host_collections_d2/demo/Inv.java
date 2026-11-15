package demo;

import java.util.AbstractList;
import java.util.List;
import java.util.RandomAccess;
import net.minecraft.client.Minecraft;
import net.minecraft.core.NonNullList;

/**
 * #253's re-check: the inventory's `NonNullList` (26.3's `Inventory.items`, `NonNullList.withSize(36, ..)`) has the
 * real class's surface, and a `subList` of it, through either owner, keeps `NonNullList`'s rules (fixed size, no
 * `null`) and writes through.
 */
@SuppressWarnings("unchecked")
public final class Inv {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static NonNullList<Object> items() {
        return Minecraft.getInstance().player.getInventory().getNonEquipmentItems();
    }

    public static String surface() {
        Class<?> c = items().getClass();
        return c.getName() + "|" + c.getSuperclass().getName() + "|" + b(List.class.isAssignableFrom(c))
            + b(AbstractList.class.isAssignableFrom(c)) + b(RandomAccess.class.isAssignableFrom(c))
            + c.getInterfaces().length + b(c == NonNullList.class) + b(NonNullList.class.isInstance(items()));
    }

    public static String subListClass() {
        return items().subList(0, 2).getClass().getName();
    }

    public static String subListSet() {
        NonNullList<Object> l = items();
        Object e = l.get(1);
        l.subList(0, 2).set(0, e);
        return b(items().get(0) == e);
    }

    public static String subListAdd() {
        items().subList(0, 2).add(items().get(1));
        return "added";
    }

    public static String subListClear() {
        items().subList(0, 2).clear();
        return "cleared";
    }

    public static String subListRemove() {
        items().subList(0, 2).remove(0);
        return "removed";
    }

    public static String subListSetNull() {
        items().subList(0, 2).set(0, null);
        return "set";
    }

    /** The same through the `List` owner. */
    public static String listSubListSet() {
        List<Object> l = items();
        Object e = l.get(2);
        l.subList(0, 2).set(0, e);
        return b(items().get(0) == e);
    }

    public static String listSubListClear() {
        List<Object> l = items();
        l.subList(0, 2).clear();
        return "cleared";
    }

    static String run(java.util.function.Supplier<String> body) {
        try {
            return body.get();
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName();
        }
    }

    public static void main(String[] args) {
        System.out.println("surface=" + run(Inv::surface));
        System.out.println("subListClass=" + run(Inv::subListClass));
        System.out.println("subListSet=" + run(Inv::subListSet));
        System.out.println("subListAdd=" + run(Inv::subListAdd));
        System.out.println("subListClear=" + run(Inv::subListClear));
        System.out.println("subListRemove=" + run(Inv::subListRemove));
        System.out.println("subListSetNull=" + run(Inv::subListSetNull));
        System.out.println("listSubListSet=" + run(Inv::listSubListSet));
        System.out.println("listSubListClear=" + run(Inv::listSubListClear));
    }
}

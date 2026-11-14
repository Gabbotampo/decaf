package demo;

/**
 * Casts of a `String` (#167): `java.lang.String` is final and implements exactly `Serializable`,
 * `Comparable`, `CharSequence`, `Constable` and `ConstantDesc`, so a cast to any other type
 * throws `ClassCastException`. Each probe casts a computed `String` (no constant the compiler
 * could fold) and answers `ok` or the exception's message, then `|` and `instanceof`. `main`
 * prints every probe on the real JVM; Horizon runs the probes alone.
 */
public final class Probe {
    interface Iface {}

    static class Klass {}

    private static Object text() {
        StringBuilder text = new StringBuilder("hel");
        return text.append("lo").toString();
    }

    private static String cast(Runnable cast, boolean instance) {
        try {
            cast.run();
            return "ok|" + instance;
        } catch (ClassCastException e) {
            return "CCE " + e.getMessage() + "|" + instance;
        }
    }

    public static String runnable() {
        Object s = text();
        return cast(() -> { Runnable c = (Runnable) s; }, s instanceof Runnable);
    }

    public static String list() {
        Object s = text();
        return cast(() -> { java.util.List<?> c = (java.util.List<?>) s; }, s instanceof java.util.List);
    }

    public static String supplier() {
        Object s = text();
        return cast(() -> { java.util.function.Supplier<?> c = (java.util.function.Supplier<?>) s; }, s instanceof java.util.function.Supplier);
    }

    public static String charSequence() {
        Object s = text();
        return cast(() -> { CharSequence c = (CharSequence) s; }, s instanceof CharSequence);
    }

    public static String comparable() {
        Object s = text();
        return cast(() -> { Comparable<?> c = (Comparable<?>) s; }, s instanceof Comparable);
    }

    public static String serializable() {
        Object s = text();
        return cast(() -> { java.io.Serializable c = (java.io.Serializable) s; }, s instanceof java.io.Serializable);
    }

    public static String constable() {
        Object s = text();
        return cast(() -> { java.lang.constant.Constable c = (java.lang.constant.Constable) s; }, s instanceof java.lang.constant.Constable);
    }

    public static String constantDesc() {
        Object s = text();
        return cast(() -> { java.lang.constant.ConstantDesc c = (java.lang.constant.ConstantDesc) s; }, s instanceof java.lang.constant.ConstantDesc);
    }

    public static String integer() {
        Object s = text();
        return cast(() -> { Integer c = (Integer) s; }, s instanceof Integer);
    }

    public static String objectArray() {
        Object s = text();
        return cast(() -> { Object[] c = (Object[]) s; }, s instanceof Object[]);
    }

    public static String stringArray() {
        Object s = text();
        return cast(() -> { String[] c = (String[]) s; }, s instanceof String[]);
    }

    public static String intArray() {
        Object s = text();
        return cast(() -> { int[] c = (int[]) s; }, s instanceof int[]);
    }

    public static String intMatrix() {
        Object s = text();
        return cast(() -> { int[][] c = (int[][]) s; }, s instanceof int[][]);
    }

    public static String jarInterface() {
        Object s = text();
        return cast(() -> { Iface c = (Iface) s; }, s instanceof Iface);
    }

    public static String jarClass() {
        Object s = text();
        return cast(() -> { Klass c = (Klass) s; }, s instanceof Klass);
    }

    public static String jarArray() {
        Object s = text();
        return cast(() -> { Iface[] c = (Iface[]) s; }, s instanceof Iface[]);
    }

    public static String desktop() {
        Object s = text();
        return cast(() -> { java.awt.event.ActionListener c = (java.awt.event.ActionListener) s; }, s instanceof java.awt.event.ActionListener);
    }

    public static String logging() {
        Object s = text();
        return cast(() -> { java.util.logging.Filter c = (java.util.logging.Filter) s; }, s instanceof java.util.logging.Filter);
    }

    public static String sql() {
        Object s = text();
        return cast(() -> { java.sql.Wrapper c = (java.sql.Wrapper) s; }, s instanceof java.sql.Wrapper);
    }

    public static String gameClass() {
        Object s = text();
        return cast(() -> { net.minecraft.world.item.Item c = (net.minecraft.world.item.Item) s; },
            s instanceof net.minecraft.world.item.Item);
    }

    /** Text shaped like a class name: the host's stand-in for objects it does not model. */
    public static String classNameText() {
        StringBuilder name = new StringBuilder("net/minecraft/core/");
        Object s = name.append("Registry").toString();
        return cast(() -> { Iface c = (Iface) s; }, s instanceof Iface);
    }

    /** `System.out` is a `PrintStream`, never a `String` (#167's review). */
    public static String sysOutIface() {
        Object o = System.out;
        return cast(() -> { Iface c = (Iface) o; }, o instanceof Iface);
    }

    public static String sysOutGame() {
        Object o = System.out;
        return cast(() -> { net.minecraft.world.item.Item c = (net.minecraft.world.item.Item) o; },
            o instanceof net.minecraft.world.item.Item);
    }

    public static String sysOutPrintStream() {
        Object o = System.out;
        return cast(() -> { java.io.PrintStream c = (java.io.PrintStream) o; }, o instanceof java.io.PrintStream);
    }

    /** A `String` cast to a client class every object of which is a handle (#232). */
    public static String clientClass() {
        Object s = text();
        return cast(() -> { net.minecraft.network.chat.Style c = (net.minecraft.network.chat.Style) s; },
            s instanceof net.minecraft.network.chat.Style);
    }

    /** Text shaped like a class name, against that client class: still no `Style`. */
    public static String classNameClient() {
        StringBuilder name = new StringBuilder("net/minecraft/network/chat/");
        Object s = name.append("Style").toString();
        return cast(() -> { net.minecraft.network.chat.Style c = (net.minecraft.network.chat.Style) s; },
            s instanceof net.minecraft.network.chat.Style);
    }

    /** Text shaped like a class name, against a game class the host may stand it for. */
    public static String classNameGame() {
        StringBuilder name = new StringBuilder("net/minecraft/world/item/");
        Object s = name.append("Item").toString();
        return cast(() -> { net.minecraft.world.item.Item c = (net.minecraft.world.item.Item) s; },
            s instanceof net.minecraft.world.item.Item);
    }

    /**
     * NeoForge's mod list, a host object (#167's review). Not in `main`: the oracle has no
     * NeoForge, and Horizon counts the cast (the list's runtime class is the loader's).
     */
    public static String modListIface() {
        Object o = net.neoforged.fml.ModList.get();
        return cast(() -> { Iface c = (Iface) o; }, o instanceof Iface);
    }

    /** `StringRepresentable.fromEnum` builds an `EnumCodec` (#236's re-check). Not in `main`. */
    public static String fromEnumCodec() {
        Object o = net.minecraft.util.StringRepresentable.fromEnum(() -> new java.lang.annotation.ElementType[0]);
        return cast(() -> { net.minecraft.util.StringRepresentable.EnumCodec<?> c =
            (net.minecraft.util.StringRepresentable.EnumCodec<?>) o; },
            o instanceof net.minecraft.util.StringRepresentable.EnumCodec);
    }

    /** NeoForge's `ConfigTracker.INSTANCE`, a host object (#236's re-check). Not in `main`. */
    public static String configTrackerIface() {
        Object o = net.neoforged.fml.config.ConfigTracker.INSTANCE;
        return cast(() -> { Iface c = (Iface) o; }, o instanceof Iface);
    }

    public static void main(String[] args) {
        System.out.println("runnable=" + runnable());
        System.out.println("list=" + list());
        System.out.println("supplier=" + supplier());
        System.out.println("charSequence=" + charSequence());
        System.out.println("comparable=" + comparable());
        System.out.println("serializable=" + serializable());
        System.out.println("constable=" + constable());
        System.out.println("constantDesc=" + constantDesc());
        System.out.println("integer=" + integer());
        System.out.println("objectArray=" + objectArray());
        System.out.println("stringArray=" + stringArray());
        System.out.println("intArray=" + intArray());
        System.out.println("intMatrix=" + intMatrix());
        System.out.println("jarInterface=" + jarInterface());
        System.out.println("jarClass=" + jarClass());
        System.out.println("jarArray=" + jarArray());
        System.out.println("desktop=" + desktop());
        System.out.println("logging=" + logging());
        System.out.println("sql=" + sql());
        System.out.println("gameClass=" + gameClass());
        System.out.println("classNameText=" + classNameText());
        System.out.println("sysOutIface=" + sysOutIface());
        System.out.println("sysOutGame=" + sysOutGame());
        System.out.println("sysOutPrintStream=" + sysOutPrintStream());
        System.out.println("clientClass=" + clientClass());
        System.out.println("classNameClient=" + classNameClient());
        System.out.println("classNameGame=" + classNameGame());
    }
}

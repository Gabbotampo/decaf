package demo;
public final class Target {
    public static int number() { Probe.mark("number-target"); return 5; }
    public static String text() { Probe.mark("text-target"); return "actual"; }
    public static int tail() { Probe.mark("tail-target"); return 5; }
    public static Object obj() { Probe.mark("obj-target"); garbage(); return new Cell(5, "actual"); }
    public static Object plain() { garbage(); return new Cell(6, "plain"); }
    public static void garbage() { for (int i = 0; i < 3000; i++) { Cell g = new Cell(i, "g"); g.v++; } }
    public static String read(Object o) { return ((Cell) o).show(); }
}

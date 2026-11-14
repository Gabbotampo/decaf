package demo;
public final class Cell {
    int v; String name;
    public Cell(int v, String name) { this.v = v; this.name = name; }
    public String show() { return v + name; }
}

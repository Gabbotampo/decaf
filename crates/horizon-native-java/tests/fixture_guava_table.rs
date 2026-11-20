//! Guava 33.6.0 Table semantics, checked against a Java 25 run.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const JAVA25: &str = include_str!("fixtures/guava_table_java25.txt");
use javac_fixture::Fixture;

const TABLE_STUB: &str = r#"package com.google.common.collect;
import java.util.*;
public interface Table<R,C,V> {
  V put(R r, C c, V v); V get(Object r, Object c); boolean contains(Object r,Object c);
  boolean containsRow(Object r); boolean containsColumn(Object c); boolean containsValue(Object v);
  V remove(Object r,Object c); Map<C,V> row(R r); Map<R,V> column(C c);
  Map<R,Map<C,V>> rowMap(); Map<C,Map<R,V>> columnMap();
  Set<Cell<R,C,V>> cellSet(); Set<R> rowKeySet(); Set<C> columnKeySet();
  Collection<V> values(); int size(); boolean isEmpty(); void clear();
  interface Cell<R,C,V> { R getRowKey(); C getColumnKey(); V getValue(); }
}
"#;

const HASH_STUB: &str = r#"package com.google.common.collect;
public abstract class HashBasedTable<R,C,V> implements Table<R,C,V> {
  public static <R,C,V> HashBasedTable<R,C,V> create() { throw new AssertionError(); }
  public static <R,C,V> HashBasedTable<R,C,V> create(int r, int c) { throw new AssertionError(); }
  public static <R,C,V> HashBasedTable<R,C,V> create(Table<? extends R, ? extends C, ? extends V> t) { throw new AssertionError(); }
}
"#;

const TREE_STUB: &str = r#"package com.google.common.collect;
import java.util.*;
public abstract class TreeBasedTable<R,C,V> implements Table<R,C,V> {
  public static <R extends Comparable<? super R>,C extends Comparable<? super C>,V> TreeBasedTable<R,C,V> create() { throw new AssertionError(); }
  public static <R,C,V> TreeBasedTable<R,C,V> create(Comparator<? super R> r, Comparator<? super C> c) { throw new AssertionError(); }
  public static <R,C,V> TreeBasedTable<R,C,V> create(TreeBasedTable<R,C,? extends V> t) { throw new AssertionError(); }
}
"#;

const IMMUTABLE_STUB: &str = r#"package com.google.common.collect;
public abstract class ImmutableTable<R,C,V> {
  public static <R,C,V> ImmutableTable<R,C,V> of() { throw new AssertionError(); }
  public static <R,C,V> ImmutableTable<R,C,V> of(R r,C c,V v) { throw new AssertionError(); }
  public static <R,C,V> Builder<R,C,V> builder() { throw new AssertionError(); }
  public static final class Builder<R,C,V> {
    public Builder<R,C,V> put(R r,C c,V v) { throw new AssertionError(); }
    public ImmutableTable<R,C,V> build() { throw new AssertionError(); }
  }
}
"#;

const TABLES_STUB: &str = r#"package com.google.common.collect;
import java.util.*; import java.util.function.Supplier;
public final class Tables {
  public static <R,C,V> Table.Cell<R,C,V> immutableCell(R r, C c, V v) { throw new AssertionError(); }
  public static <R,C,V> Table<R,C,V> newCustomTable(Map<R,Map<C,V>> m, Supplier<Map<C,V>> s) { throw new AssertionError(); }
  public static <R,C,V> Table<R,C,V> synchronizedTable(Table<R,C,V> t) { throw new AssertionError(); }
}
"#;

const PROBE: &str = r#"package demo;
import java.util.*;
import com.google.common.collect.*;
public class Probe {
  public static String core() {
    Table<String,String,Integer> t = HashBasedTable.create(2, 3);
    t.put("b", "y", 2); t.put("a", "x", 1); t.put("b", "z", 3);
    int old = t.put("b", "y", 4);
    Map<String,Integer> row = t.row("a"); row.put("q", 7);
    Map<String,Integer> column = t.column("y"); column.put("c", 8);
    return old + "," + t.get("b", "y") + "," + t.containsRow("c") + ","
      + t.containsColumn("q") + "," + t.size() + "," + t.rowKeySet() + ","
      + t.columnKeySet() + "," + t.values() + "," + t;
  }
  public static String narrow() {
    Table<String,String,Integer> t = HashBasedTable.create();
    t.put("r", "c", 7);
    Map<String,Integer> row = t.row("r");
    Table.Cell<String,String,Integer> cell = t.cellSet().iterator().next();
    return t.get("r", "c") + "," + t.contains("r", "c") + ","
      + row.get("c") + "," + row.size() + "," + t.rowKeySet().size() + ","
      + t.cellSet().size() + "," + cell.getRowKey() + "," + cell.getColumnKey()
      + "," + cell.getValue();
  }
  public static String equalityHashAndCell() {
    Table<String,String,Integer> t = HashBasedTable.create();
    t.put("r", "c", 7);
    Table<String,String,Integer> same = HashBasedTable.create();
    same.put("r", "c", 7);
    Table.Cell<String,String,Integer> cell = t.cellSet().iterator().next();
    Table.Cell<String,String,Integer> equal = Tables.immutableCell("r", "c", 7);
    Table.Cell<String,String,Integer> different = Tables.immutableCell("r", "c", 8);
    return t.toString() + "|" + t.equals(same) + "," + t.equals("x")
      + "|" + (t.hashCode() == t.cellSet().hashCode()) + "|" + cell
      + "|" + cell.equals(equal) + "," + cell.equals(different)
      + "|" + cell.hashCode() + "," + equal.hashCode()
      + "|" + t.cellSet().contains(equal) + "," + t.cellSet().contains(different);
  }

  public static String liveAndFailFast() {
    Table<String,String,Integer> t = HashBasedTable.create();
    t.put("r", "a", 1);
    Set<String> rows = t.rowKeySet();
    Map<String,Integer> row = t.row("r");
    t.put("s", "b", 2);
    boolean live = rows.size() == 2 && row.get("a") == 1;
    Iterator<String> it = rows.iterator();
    t.put("t", "c", 3);
    boolean cme;
    try { it.next(); cme = false; }
    catch (ConcurrentModificationException expected) { cme = true; }
    return live + "," + cme;
  }
  public static String nulls() {
    Table<String,String,Integer> t = HashBasedTable.create();
    try { t.put(null, "c", 1); return "no-throw"; }
    catch (NullPointerException e) { return "npe"; }
  }
  public static String copyAndCapacity() {
    Table<String,String,Integer> source = HashBasedTable.create();
    source.put("r", "c", 7); source.put("s", "d", 8);
    Table<String,String,Integer> copy = HashBasedTable.create(source);
    return copy + "/" + copy.equals(source) + "/" + (copy == source) + "|"
      + capacity(0, 1) + "," + capacity(-1, 1) + "," + capacity(1, 0) + "," + capacity(1, -1);
  }
  private static String capacity(int rows, int cells) {
    try { HashBasedTable.create(rows, cells); return "ok"; }
    catch (Throwable t) { return t.getClass().getName() + ":" + t.getMessage(); }
  }
  public static String standaloneCell() {
    Table.Cell<String,String,Integer> cell = Tables.immutableCell("r", "c", 7);
    return cell.getRowKey() + "," + cell.getColumnKey() + "," + cell.getValue()
      + "|" + cell + "|" + cell.hashCode() + "|" + cell.equals(Tables.immutableCell("r", "c", 7));
  }
  public static String customKey() {
    class K { int x; K(int x) { this.x = x; }
      public boolean equals(Object o) { return o instanceof K k && k.x == x; }
      public int hashCode() { return x; }
    }
    K first = new K(1), second = new K(1);
    Table<K,String,Integer> table = HashBasedTable.create(); table.put(first, "c", 1);
    return table.get(second, "c") + "," + table.rowKeySet().contains(second);
  }
  public static String userCell() {
    Table.Cell<String,String,Integer> cell = Tables.immutableCell("r", "c", 7);
    Table.Cell<String,String,Integer> user = new Table.Cell<>() {
      public String getRowKey() { return "r"; }
      public String getColumnKey() { return "c"; }
      public Integer getValue() { return 7; }
      public boolean equals(Object o) { return o instanceof Table.Cell<?,?,?> c
        && Objects.equals(getRowKey(), c.getRowKey()) && Objects.equals(getColumnKey(), c.getColumnKey())
        && Objects.equals(getValue(), c.getValue()); }
      public int hashCode() { return 142421; }
      public String toString() { return "usercell"; }
    };
    return cell.equals(user) + "," + user.equals(cell) + "," + cell.equals(new Object());
  }
}
"#;

fn fixture() -> Fixture {
    Fixture::build(
        "guava-table",
        &[
            ("com/google/common/collect/Table.java", TABLE_STUB),
            ("com/google/common/collect/HashBasedTable.java", HASH_STUB),
            ("com/google/common/collect/TreeBasedTable.java", TREE_STUB),
            (
                "com/google/common/collect/ImmutableTable.java",
                IMMUTABLE_STUB,
            ),
            ("com/google/common/collect/Tables.java", TABLES_STUB),
            ("demo/Probe.java", PROBE),
        ],
    )
}

#[test]
fn table_surface_counts_unmodelled_members() {
    // Captured with OpenJDK 25.0.5+8 and Guava 33.6.0-jre; these rows are
    // the independent Java oracle for every probe method.
    assert!(JAVA25.contains("narrow=7,true,7,1,1,1,r,c,7"));
    assert!(JAVA25.contains(
        "equalityHashAndCell={r={c=7}}|true,false|true|(r,c)=7|true,false|142421,142421|true,false"
    ));
    assert!(JAVA25.contains("liveAndFailFast=true,true"));
    assert!(JAVA25.contains("nulls=npe"));
    assert!(JAVA25.contains("copyAndCapacity={r={c=7}, s={d=8}}/true/false|ok,java.lang.IllegalArgumentException:expectedSize cannot be negative but was: -1,ok,java.lang.IllegalArgumentException:expectedCellsPerRow cannot be negative but was: -1"));
    assert!(JAVA25.contains("standaloneCell=r,c,7|(r,c)=7|142421|true"));
    assert!(JAVA25.contains("customKey=1,true"));
    assert!(JAVA25.contains("userCell=true,true,false"));
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "core"),
        "2,4,false,false,4,{b={y=4, z=3}, a={x=1, q=7}},[],[],{b={y=4, z=3}, a={x=1, q=7}}"
    );
    let calls = host.unknown_host_calls();
    for expected in [
        "com/google/common/collect/Table#column(Ljava/lang/Object;)Ljava/util/Map;",
        "com/google/common/collect/Table#columnKeySet()Ljava/util/Set;",
        "com/google/common/collect/Table#containsColumn(Ljava/lang/Object;)Z",
        "com/google/common/collect/Table#containsRow(Ljava/lang/Object;)Z",
        "com/google/common/collect/Table#values()Ljava/util/Collection;",
    ] {
        assert!(
            calls.iter().any(|(call, _)| call == expected),
            "{expected}: {calls:?}"
        );
    }
    assert_eq!(fixture.text_on(&mut host, "narrow"), "7,true,7,1,1,1,r,c,7");
    assert_eq!(
        fixture.text_on(&mut host, "equalityHashAndCell"),
        "{r={c=7}}|true,false|true|(r,c)=7|true,false|142421,142421|true,false"
    );
    assert_eq!(fixture.text_on(&mut host, "liveAndFailFast"), "true,true");
    assert_eq!(fixture.text_on(&mut host, "nulls"), "npe");
    assert_eq!(
        fixture.text_on(&mut host, "copyAndCapacity"),
        "{r={c=7}, s={d=8}}/true/false|ok,java.lang.IllegalArgumentException:expectedSize cannot be negative but was: -1,ok,java.lang.IllegalArgumentException:expectedCellsPerRow cannot be negative but was: -1"
    );
    assert_eq!(
        fixture.text_on(&mut host, "standaloneCell"),
        "r,c,7|(r,c)=7|142421|true"
    );
    assert_eq!(fixture.text_on(&mut host, "customKey"), "null,false");
    assert_eq!(fixture.text_on(&mut host, "userCell"), "false,true,false");
    let calls = host.unknown_host_calls();
    assert!(
        calls.iter().any(|(call, _)| call.contains("Table#get")),
        "{calls:?}"
    );
    assert!(
        calls
            .iter()
            .any(|(call, _)| call == "java/lang/Object#equals(Ljava/lang/Object;)Z"),
        "{calls:?}"
    );
}

//! Java 25 oracle for `Collections.newSetFromMap` and its live map-backed set.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const PROBE: &str = r#"package demo;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentSkipListMap;
public final class Probe {
  enum Key { A }
  static String ex(Runnable r) { try { r.run(); return "ok"; } catch (Throwable t) { return t.getClass().getSimpleName()+"|"+t.getMessage(); } }
  public static String basic() {
    Map<String,Boolean> m = new LinkedHashMap<>(); Set<String> s = Collections.newSetFromMap(m);
    boolean a=s.add("a"), b=s.add("a"); m.put("b", true);
    String before=s.getClass().getName()+"|"+a+"|"+b+"|"+s.contains("a")+"|"+s.size()+"|"+m.size()+"|"+s+"|"+s.hashCode()+"|"+s.equals(Set.of("a","b"));
    boolean rem=s.remove("a"); m.remove("b");
    return before+"|"+rem+"|"+s.isEmpty()+"|"+m.isEmpty()+"|"+s;
  }
  static String differential(boolean linked) {
    Map<String,Boolean> m = linked ? new LinkedHashMap<>() : new HashMap<>();
    Set<String> s = Collections.newSetFromMap(m);
    s.add("b"); s.add("a");
    String iter = new ArrayList<>(s).toString();
    String before = s.getClass().getSimpleName()+"|"+iter+"|"+s.toString()+"|"+s.hashCode()+"|"+s.equals(new HashSet<>(List.of("a","b")))+"|"+m.size();
    boolean retained = s.retainAll(List.of("a"));
    boolean retainedWrites = m.size()==1 && m.containsKey("a") && !m.containsKey("b");
    boolean removed = s.removeIf("a"::equals);
    boolean removedWrites = m.isEmpty() && s.isEmpty();
    m.put("x", true);
    Iterator<String> mapIt = s.iterator();
    m.put("y", true);
    String cmeMap = ex(mapIt::next);
    Iterator<String> setIt = s.iterator();
    s.add("z");
    String cmeSet = ex(setIt::next);
    String nonEmpty = ex(() -> Collections.newSetFromMap(m));
    return before+"|"+retained+"|"+retainedWrites+"|"+removed+"|"+removedWrites+"|"+cmeMap+"|"+cmeSet+"|"+nonEmpty+"|"+s+"|"+m;
  }
  public static String hashMapDifferential() { return differential(false); }
  public static String linkedMapDifferential() { return differential(true); }
  public static String duplicate() { Map<String,Boolean> m=new HashMap<>(); m.put("x",true); return ex(()->Collections.newSetFromMap(m)); }
  public static String nullMap() { return ex(()->Collections.newSetFromMap(null)); }
  public static String chmExact() { Set<String> s=Collections.newSetFromMap(new ConcurrentHashMap<>()); boolean a=s.add("a"), d=s.add("a"), c=s.contains("a"), r=s.remove("a"), n=s.remove("z"); String e=ex(()->s.add(null)); return a+"|"+d+"|"+c+"|"+r+"|"+n+"|"+s.size()+"|"+s.isEmpty()+"|"+e; }
  public static String chmIterate() { Set<String> s=Collections.newSetFromMap(new ConcurrentHashMap<>()); s.add("b"); s.add("a"); return s.toString(); }
  public static String chmToArray() { Set<String> s=Collections.newSetFromMap(new ConcurrentHashMap<>()); s.add("a"); return java.util.Arrays.toString(s.toArray()); }
  public static String chmStream() { Set<String> s=Collections.newSetFromMap(new ConcurrentHashMap<>()); s.add("a"); return String.valueOf(s.stream().findFirst()); }
  public static String identity() { return ex(()->Collections.newSetFromMap(new IdentityHashMap<>())); }
  public static String weak() { return ex(()->Collections.newSetFromMap(new WeakHashMap<>())); }
  public static String concurrent() { return ex(()->Collections.newSetFromMap(new ConcurrentHashMap<>())); }
  public static String skipList() { return ex(()->Collections.newSetFromMap(new ConcurrentSkipListMap<>())); }
  public static String tree() { return ex(()->Collections.newSetFromMap(new TreeMap<>())); }
  public static String enumMap() { return ex(()->Collections.newSetFromMap(new EnumMap<Key,Boolean>(Key.class))); }
  public static String hashtable() { return ex(()->Collections.newSetFromMap(new Hashtable<>())); }
}
"#;

const JAVA25: &str = include_str!("fixtures/new_set_from_map/java25.txt");

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build("new-set-from-map", &[("demo/Probe.java", PROBE)])
}

fn expected(name: &str) -> &str {
    JAVA25
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("missing oracle line {name}"))
}

#[test]
fn linked_map_view_is_live_and_matches_set_contracts() {
    let fixture = fixture();
    for method in [
        "basic",
        "duplicate",
        "nullMap",
        "hashMapDifferential",
        "linkedMapDifferential",
        "chmExact",
    ] {
        assert_eq!(fixture.text(method), expected(method), "{method}");
    }
}

#[test]
fn concurrent_hash_map_order_dependent_operations_are_counted() {
    let fixture = fixture();
    for method in ["chmIterate", "chmToArray", "chmStream"] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap_or_else(|error| panic!("{method} failed: {error}"));
        println!("{method}: {:?}", host.unknown_host_calls());
        assert_eq!(
            host.unknown_host_calls()
                .iter()
                .map(|(_, n)| *n)
                .sum::<usize>(),
            1,
            "{method}: {:?}",
            host.unknown_host_calls()
        );
    }
}

#[test]
fn unsupported_map_implementations_are_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    for method in [
        "identity",
        "weak",
        "concurrent",
        "skipList",
        "tree",
        "enumMap",
        "hashtable",
    ] {
        fixture.text_on(&mut host, method);
    }
    let factory = "java/util/Collections#newSetFromMap(Ljava/util/Map;)Ljava/util/Set;";
    assert_eq!(
        host.unknown_host_calls()
            .iter()
            .find_map(|(call, count)| (call == factory).then_some(*count)),
        Some(6),
        "unsupported map implementations must remain counted: {:?}",
        host.unknown_host_calls()
    );
}

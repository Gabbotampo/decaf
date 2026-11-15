//! Java 25 oracle for `Collections.newSetFromMap` and its live map-backed set.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const PROBE: &str = r#"package demo;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
public final class Probe {
  static String ex(Runnable r) { try { r.run(); return "ok"; } catch (Throwable t) { return t.getClass().getSimpleName()+"|"+t.getMessage(); } }
  public static String basic() {
    Map<String,Boolean> m = new LinkedHashMap<>(); Set<String> s = Collections.newSetFromMap(m);
    boolean a=s.add("a"), b=s.add("a"); m.put("b", true);
    String before=s.getClass().getName()+"|"+a+"|"+b+"|"+s.contains("a")+"|"+s.size()+"|"+m.size()+"|"+s+"|"+s.hashCode()+"|"+s.equals(Set.of("a","b"));
    boolean rem=s.remove("a"); m.remove("b");
    return before+"|"+rem+"|"+s.isEmpty()+"|"+m.isEmpty()+"|"+s;
  }
  public static String identity() {
    Map<Object,Boolean> m = new IdentityHashMap<>(); Set<Object> s = Collections.newSetFromMap(m);
    Object x=new Object(), y=new Object(); s.add(x); s.add(y);
    return s.size()+"|"+s.contains(x)+"|"+s.contains(new Object())+"|"+m.size()+"|"+s.equals(Set.of(x))+"|"+s.toString().startsWith("[");
  }
  public static String duplicate() { Map<String,Boolean> m=new HashMap<>(); m.put("x",true); return ex(()->Collections.newSetFromMap(m)); }
  public static String nullMap() { return ex(()->Collections.newSetFromMap(null)); }
  public static String weak() { return Collections.newSetFromMap(new WeakHashMap<>()).getClass().getName(); }
  public static String concurrent() { return Collections.newSetFromMap(new ConcurrentHashMap<>()).getClass().getName(); }
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
    for method in ["basic", "identity", "duplicate", "nullMap"] {
        assert_eq!(fixture.text(method), expected(method), "{method}");
    }
}

#[test]
fn unsupported_map_implementations_are_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.text_on(&mut host, "weak");
    fixture.text_on(&mut host, "concurrent");
    assert!(host.unknown_host_calls().iter().any(|(call, _)| {
        call == "java/util/Collections#newSetFromMap(Ljava/util/Map;)Ljava/util/Set;"
    }));
}

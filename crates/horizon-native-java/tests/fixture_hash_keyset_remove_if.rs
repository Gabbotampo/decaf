//! Java 25 oracle fixture for `removeIf` on a `HashMap`'s or `LinkedHashMap`'s `keySet()`.
//!
//! `fixtures/hash_keyset_remove_if/demo/Probe.java` is one source run two ways: on Java 25
//! (`java25.txt`, command in the README) and here. Only `demo/` ships, so every collection call
//! reaches the host's model. `rows()` must equal the capture byte for byte, and the run must
//! leave no call counted: every row has an exact answer.
//!
//! What is not shown exact stays counted (`counted_key_sets_stay_counted`).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/hash_keyset_remove_if/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/hash_keyset_remove_if/java25.txt");

/// The probe nests jar frames (`rows` -> `run` -> a lambda -> `removeIf` -> the predicate). A debug
/// build's interpreter frame is large, and the runtime's frame bound (`frame_bound.rs`) throws
/// `StackOverflowError` once the native stack left runs low: the 2 MiB default test thread sits at
/// that bound, so the test runs on a roomy thread, as `fixture_callbacks_rc261` does.
#[test]
fn key_set_remove_if_matches_java25() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(every_row)
        .unwrap()
        .join()
        .unwrap();
}

fn every_row() {
    let fixture = Fixture::build("hash-keyset-remove-if", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let rows = fixture.text_on(&mut host, "rows");
    // Both at once, so a failure shows which calls were counted beside the wrong rows.
    let unknown = format!("{:?}", host.unknown_host_calls());
    // Row by row first: the first row that differs is the one to read.
    for (got, want) in rows.split('\n').zip(ORACLE.split('\n')) {
        assert_eq!(got, want);
    }
    assert_eq!((rows.as_str(), unknown.as_str()), (ORACLE, "[]"));
}

/// Cases the oracle does not show: a `TreeMap`'s key set, a hash map whose key order is unknown
/// (identity hashes), and an access-ordered `LinkedHashMap`. The call is counted, not answered.
const COUNTED: &str = r#"package demo;
import java.util.*;
public class Probe {
  public static String treeMap() {
    Map<String, Integer> m = new TreeMap<>();
    m.put("a", 1); m.put("b", 2);
    return String.valueOf(m.keySet().removeIf(k -> k.equals("a")));
  }
  public static String identityKeys() {
    Map<Object, Integer> m = new HashMap<>();
    m.put(new Object(), 1); m.put(new Object(), 2);
    return String.valueOf(m.keySet().removeIf(k -> true));
  }
  public static String accessOrder() {
    Map<String, Integer> m = new LinkedHashMap<>(16, 0.75f, true);
    m.put("a", 1); m.put("b", 2);
    return String.valueOf(m.keySet().removeIf(k -> k.equals("a")));
  }
}
"#;

#[test]
fn counted_key_sets_stay_counted() {
    let fixture = Fixture::build(
        "hash-keyset-remove-if-counted",
        &[("demo/Probe.java", COUNTED)],
    );
    for method in ["treeMap", "identityKeys", "accessOrder"] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        fixture.text_on(&mut host, method);
        let counted = host.unknown_host_calls();
        assert!(
            counted.iter().any(|(call, n)| call
                == "java/util/Set#removeIf(Ljava/util/function/Predicate;)Z"
                && *n == 1),
            "{method}: {counted:?}"
        );
    }
}

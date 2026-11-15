//! #254: adding a `MapN` stores one element and does not inspect its entries.
//! The expected strings come from Java 25 (Temurin 25.0.5, `javac --release 21`).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = r#"package demo;
import java.util.*;
public class Probe {
    public static String mapOf() {
        List<Object> values = new ArrayList<>();
        values.add(Map.of("a", 1, "b", 2));
        return values.size() + ":" + ((Map<?, ?>) values.get(0)).get("a");
    }
    public static String mapOfEntries() {
        List<Object> values = new ArrayList<>();
        values.add(Map.ofEntries(Map.entry("a", 1), Map.entry("b", 2)));
        return values.size() + ":" + ((Map<?, ?>) values.get(0)).get("b");
    }
}
"#;

#[test]
fn mapn_is_stored_as_one_list_element() {
    let fixture =
        javac_fixture::Fixture::build("map-of-entries-add", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let oracle = include_str!("fixtures/map_of_entries_add/java25.txt");
    for (method, key) in [("mapOf", "mapOf"), ("mapOfEntries", "mapOfEntries")] {
        let expected = oracle
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap();
        let result = fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap();
        assert_eq!(result, Value::Ref(Some(expected.into())), "{method}");
    }
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

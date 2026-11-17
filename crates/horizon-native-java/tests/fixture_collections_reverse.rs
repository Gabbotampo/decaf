//! `Collections.reverse(List)` and `Stream.iterator()` (#491).
//!
//! `fixtures/collections_reverse/demo/Probe.java` runs on Java 25 (command in the README) and
//! here; only `demo/` ships, so every JDK call reaches the host. `rows()` covers `ArrayList` and
//! `LinkedList` of sizes 0–3, an iterator made before the reverse, `List.of` (sizes 0/1 succeed,
//! 2+ throw `UnsupportedOperationException`), `Arrays.asList` (fixed-size, but `set` works),
//! a `subList` view writing through to its parent, `Collections.unmodifiableList`, and
//! `Stream.iterator()` with its `remove()` (`UnsupportedOperationException: remove`). Every
//! case is answered: `rows()` must equal `java25.txt` with nothing counted.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;

const PROBE: &str = include_str!("fixtures/collections_reverse/demo/Probe.java");
const JAVA25_CAPTURE: &str = include_str!("fixtures/collections_reverse/java25.txt");

#[test]
fn collections_reverse_and_stream_iterator_match_java25() {
    let fixture =
        javac_fixture::Fixture::build("collections-reverse", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let rows = fixture.text_on(&mut host, "rows");
    let unknown = host.unknown_host_calls();
    assert_eq!(
        rows,
        JAVA25_CAPTURE.trim_end(),
        "unknown calls: {unknown:?}"
    );
    assert_eq!(unknown, Vec::<(String, usize)>::new());
}

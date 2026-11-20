#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/issue_423/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/issue_423/java25.txt");

#[test]
fn string_constructors_match_java25() {
    let fixture = Fixture::build("issue-423", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let got = fixture.text_on(&mut host, "rows");
    assert_eq!(got, ORACLE);
}

/// `new String(bytes, UTF_16)` is a constructor Decaf counts. Its result must
/// count on every later use, so `s == s` records an `if_acmp` unknown call
/// rather than being answered silently (Java 25 prints `true`).
#[test]
fn counted_string_identity_is_counted() {
    let fixture = Fixture::build("issue-423-counted", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, "countedSelf");
    let calls = host.unknown_host_calls();
    assert!(
        calls
            .iter()
            .any(|(call, _)| call
                == "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z"),
        "{calls:?}"
    );
}

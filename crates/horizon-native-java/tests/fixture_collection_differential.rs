//! Differential coverage for the JDK 25 collection matrix used by #558.
//! `java25.txt` is captured from the pinned JDK 25.0.5 run; unknown native
//! calls are counted and therefore are not allowed to masquerade as answers.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};

#[test]
#[ignore = "full JDK matrix is the explicit review oracle"]
fn collection_matrix_matches_jdk_25_for_modelled_calls() {
    let fixture = javac_fixture::Fixture::build(
        "collection-differential",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/collection_differential/Probe.java"),
        )],
    );
    let mut exact = 0usize;
    let mut counted = 0usize;
    let mut mismatches = Vec::new();
    for line in include_str!("fixtures/collection_differential/java25.txt").lines() {
        let Some((case, expected)) = line.split_once('=') else {
            continue;
        };
        let Some((r, op)) = case.split_once(',') else {
            continue;
        };
        let (r, op) = (r.parse::<i32>().unwrap(), op.parse::<i32>().unwrap());
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            "probe",
            "(II)Ljava/lang/String;",
            None,
            &[Value::Int(r), Value::Int(op)],
            &mut host,
        );
        let unknown = host.unknown_host_calls();
        if !unknown.is_empty() {
            counted += 1;
            continue;
        }
        match answer {
            Ok(Value::Ref(Some(text))) if text == expected => exact += 1,
            other => mismatches.push(format!("{case}: expected {expected:?}, got {other:?}")),
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} modelled calls differ (exact={exact}, counted={counted}):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    assert_eq!(exact + counted, 39 * 26);
}

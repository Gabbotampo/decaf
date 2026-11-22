//! `ScheduledThreadPoolExecutor`'s policy setters and getters
//! (`setRemoveOnCancelPolicy`, `setExecuteExistingDelayedTasksAfterShutdownPolicy`),
//! against a Java 25 oracle (`fixtures/stpe_policies/java25.txt`, README.md).
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const ORACLE: &str = include_str!("fixtures/stpe_policies/java25.txt");
const PROBE: &str = include_str!("fixtures/stpe_policies/demo/Probe.java");
#[test]
fn stpe_policies_match_java25() {
    let fixture = javac_fixture::Fixture::build("stpe_policies", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let text = match answer {
        horizon_native_java::Value::Ref(Some(text)) => text,
        other => panic!("{other:?}"),
    };
    assert_eq!(text, ORACLE);
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// Not shown exact, so still counted: a pool `Executors` made (cast to the
/// class) and a jar subclass keep no flags, and the setter reaches the
/// unknown-call list instead of being answered.
const COUNTED: &str = r#"package demo;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledThreadPoolExecutor;
public class Counted {
    static final class Sub extends ScheduledThreadPoolExecutor {
        Sub() { super(1); }
    }
    public static String run() {
        ScheduledThreadPoolExecutor made = (ScheduledThreadPoolExecutor) Executors.newScheduledThreadPool(1);
        made.setRemoveOnCancelPolicy(true);
        Sub sub = new Sub();
        sub.setRemoveOnCancelPolicy(true);
        return "ran";
    }
}
"#;

#[test]
fn pools_the_oracle_does_not_cover_stay_counted() {
    let fixture =
        javac_fixture::Fixture::build("stpe_policies_counted", &[("demo/Counted.java", COUNTED)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let _ = fixture.jar.invoke_method_with_args(
        "demo/Counted",
        "run",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let counted = format!("{:?}", host.unknown_host_calls());
    assert!(
        counted.contains("ScheduledThreadPoolExecutor#setRemoveOnCancelPolicy(Z)V\", 2)"),
        "{counted}"
    );
}

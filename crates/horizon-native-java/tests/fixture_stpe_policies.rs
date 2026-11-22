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

/// A pool whose policy a jar set relies on a delayed task being pending (the
/// JEI delayed executor drops it on cancel or at `shutdown`); the runtime has
/// run it at schedule time, so a delayed `schedule` on such a pool is counted,
/// where a zero delay (the oracle's `jei` row) is still answered.
const DELAYED: &str = r#"package demo;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
public class Delayed {
    public static String zero() {
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1);
        pool.setRemoveOnCancelPolicy(true);
        ScheduledExecutorService service = pool;
        service.schedule(() -> { }, 0, TimeUnit.NANOSECONDS);
        return "zero";
    }
    public static String delayed() {
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1);
        pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        ScheduledExecutorService service = pool;
        service.schedule(() -> { }, 1, TimeUnit.HOURS);
        return "delayed";
    }
    public static String delayedCallable() {
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1);
        pool.setRemoveOnCancelPolicy(true);
        ScheduledExecutorService service = pool;
        service.schedule(() -> "x", 5, TimeUnit.SECONDS);
        return "delayedCallable";
    }
}
"#;

#[test]
fn delayed_schedule_on_a_policy_pool_is_counted() {
    let fixture =
        javac_fixture::Fixture::build("stpe_policies_delayed", &[("demo/Delayed.java", DELAYED)]);
    let counted_after = |method: &str| {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let _ = fixture.jar.invoke_method_with_args(
            "demo/Delayed",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        format!("{:?}", host.unknown_host_calls())
    };
    let zero = counted_after("zero");
    assert!(!zero.contains("schedule"), "{zero}");
    for method in ["delayed", "delayedCallable"] {
        let counted = counted_after(method);
        assert!(counted.contains("schedule"), "{method}: {counted}");
    }
}

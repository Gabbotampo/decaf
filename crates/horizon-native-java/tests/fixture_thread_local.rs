#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.util.function.Supplier;

public final class Probe {
  private static int calls;

  private static String supplied() {
    ThreadLocal<String> local = ThreadLocal.withInitial(() -> "v" + (++calls));
    String first = local.get();
    String second = local.get();
    local.set("set");
    String third = local.get();
    local.remove();
    String fourth = local.get();
    return first + "|" + second + "|" + third + "|" + fourth + "|" + calls;
  }

  private static String nullSupplier() {
    try {
      ThreadLocal.withInitial((Supplier<Object>) null);
      return "no-error";
    } catch (Throwable error) {
      return error.getClass().getSimpleName() + ":" + error.getMessage();
    }
  }

  public static String values() {
    return supplied() + "|" + nullSupplier();
  }
}
"#;

#[test]
fn thread_local_values_match_java_25() {
    let fixture = Fixture::build("thread-local", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture.text_on(&mut host, "values");
    assert_eq!(value, "v1|v1|set|v2|2|NullPointerException:supplier");
    assert!(
        host.unknown_host_calls().is_empty(),
        "counted: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn a_forged_thread_local_string_stays_unmodelled() {
    let mut host = StdHost::new();
    let call = horizon_native_java::MethodRef {
        owner: "java/lang/ThreadLocal".into(),
        name: "get".into(),
        descriptor: "()Ljava/lang/Object;".into(),
    };
    let _ = horizon_native_java::Host::invoke_instance(
        &mut host,
        &call,
        &horizon_native_java::Value::Ref(Some("hthrl:forged".into())),
        &[],
    );
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call.contains("java/lang/ThreadLocal#get")),
        "forged text must stay counted: {:?}",
        host.unknown_host_calls()
    );
}

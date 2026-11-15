#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::Value;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;
import java.util.*;
import java.util.function.Supplier;
public final class Probe {
 static String t(Supplier<Object> s) { try { return String.valueOf(s.get()); } catch (StackOverflowError e) { return "SOE"; } }
 static String deepOptional(int n) { Object x=Optional.empty(); for(int i=0;i<n;i++) x=Optional.of(x); Object y=Optional.empty(); for(int i=0;i<n;i++) y=Optional.of(y); OY=y; Object fx=x; return t(()->fx.toString().length())+","+t(()->fx.hashCode())+","+t(()->fx.equals(OY)); }
 static Object OY;
 public static String optional1000() { return deepOptional(1000); }
 public static String optional100000() { return deepOptional(100000); }
 static <T> T helper(Supplier<T> s) { return s.get(); }
 public static String capturingSupplier() { String a="left", b="right"; return helper(() -> a+":"+b); }
 public static String capturingInts() { int a=4, b=5; return helper(() -> ""+(a+b)); }
 public static String capturingLargeSupplier() { Object a0=Optional.empty(); for(int i=0;i<1000;i++) a0=Optional.of(a0); final Object a=a0; final String b="x"; return helper(() -> String.valueOf(a).length()+b); }
 public static String entrySetValueMap() { Map<String,Object> m=new HashMap<>(); m.put("k",null); Map.Entry<String,Object> e=m.entrySet().iterator().next(); e.setValue(m); return e.toString(); }
}
"#;
#[test]
fn silent_292_rows_match_java_25() {
    // Oracle values were produced by Java 25.0.5 with javac --release 21.
    let fixture = Fixture::build("silent-292", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("optional1000", "10014,0,true", false),
        ("optional100000", "SOE,SOE,SOE", true),
        ("capturingSupplier", "left:right", false),
        ("capturingInts", "9", false),
        ("capturingLargeSupplier", "10014x", false),
        ("entrySetValueMap", "k={k=(this Map)}", false),
    ];
    for (method, expected, counted) in cases {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        assert!(
            matches!(&answer, Ok(Value::Ref(Some(text))) if text == expected),
            "{method}: {answer:?}"
        );
        if counted {
            assert!(
                host.unknown_host_calls()
                    .iter()
                    .any(|(call, _)| call.starts_with("java/lang/StackOverflowError#")),
                "{method}: missing counted bound: {:?}",
                host.unknown_host_calls()
            );
        } else {
            assert!(
                host.unknown_host_calls().is_empty(),
                "{method}: unexpected counts: {:?}",
                host.unknown_host_calls()
            );
        }
    }
}

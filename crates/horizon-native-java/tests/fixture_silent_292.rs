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
 public static String optionalStringForgery() { Object x="deep-empty:1000"; try { return ((Optional<?>) x).toString(); } catch (ClassCastException e) { return e.getClass().getSimpleName(); } }
}
"#;
#[test]
fn silent_292_rows_match_java_25() {
    // Oracle values were produced by Java 25.0.5 with javac --release 21.
    let fixture = Fixture::build("silent-292", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("optional1000", "10014,0,true", false),
        ("optional100000", "SOE,SOE,SOE", true),
        ("optionalStringForgery", "ClassCastException", false),
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

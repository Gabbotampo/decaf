#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::Value;
use javac_fixture::Fixture;

const PROBE: &str = r#"
package demo;
import java.util.function.Supplier;
public final class Probe {
  enum E { A, B { int marker() { return 1; } } }
  enum F { A }
  static final class C implements Comparable<C> {
    final int n;
    C(int n) { this.n = n; }
    public int compareTo(C other) { return Integer.compare(n, other.n); }
  }
  static String caught(Supplier<?> s) {
    try { return String.valueOf(s.get()); }
    catch (Throwable e) { return e.getClass().getSimpleName(); }
  }
  static String caughtDetailed(Supplier<?> s) {
    try { return String.valueOf(s.get()); }
    catch (Throwable e) { return e.getClass().getSimpleName() + ":" + e.getMessage(); }
  }
  public static String strings() {
    return "" + "a\u0000".compareTo("a\u0001") + ","
      + "a\uD83D\uDE00".compareTo("a\uD83D\uDE01") + "," + "ab".compareTo("abcd");
  }
  public static String boxed() {
    return Byte.valueOf((byte) 1).compareTo(Byte.valueOf((byte) 3)) + ","
      + Short.valueOf((short) -100).compareTo(Short.valueOf((short) 3)) + ","
      + Integer.valueOf(-1).compareTo(Integer.valueOf(3)) + ","
      + Long.valueOf(Long.MIN_VALUE).compareTo(Long.valueOf(Long.MAX_VALUE)) + ","
      + Double.valueOf(-0.0).compareTo(Double.valueOf(0.0)) + ","
      + Double.valueOf(Double.NaN).compareTo(Double.valueOf(Double.POSITIVE_INFINITY));
  }
  public static String characterBoolean() {
    return Character.valueOf((char) 97).compareTo(Character.valueOf((char) 122)) + "|"
      + Boolean.FALSE.compareTo(Boolean.TRUE);
  }
  public static String enums() {
    return E.B.compareTo(E.A) + "|" + E.A.compareTo(E.B) + "|"
      + caughtDetailed(() -> ((Comparable) E.A).compareTo(F.A));
  }
  public static String mixed() {
    return caughtDetailed(() -> ((Comparable) Integer.valueOf(1)).compareTo(Long.valueOf(1))) + "|"
      + caughtDetailed(() -> ((Comparable) "x").compareTo(Integer.valueOf(1)));
  }
  public static String nulls() {
    return caughtDetailed(() -> Byte.valueOf((byte) 1).compareTo(null)) + "|"
      + caughtDetailed(() -> Short.valueOf((short) 1).compareTo(null)) + "|"
      + caughtDetailed(() -> Integer.valueOf(1).compareTo(null)) + "|"
      + caughtDetailed(() -> Long.valueOf(1).compareTo(null)) + "|"
      + caughtDetailed(() -> Float.valueOf(1).compareTo(null)) + "|"
      + caughtDetailed(() -> Double.valueOf(1).compareTo(null)) + "|"
      + caughtDetailed(() -> Character.valueOf('a').compareTo(null)) + "|"
      + caughtDetailed(() -> Boolean.FALSE.compareTo(null)) + "|"
      + caughtDetailed(() -> "x".compareTo(null));
  }
  public static String jarAndForgery() {
    return new C(1).compareTo(new C(3)) + "|"
      + "identifier/a:b".compareTo("identifier/a:c") + "|"
      + "enum/demo/E/A".compareTo("enum/demo/E/B");
  }
}
"#;

#[test]
fn java25_compare_to_receivers_are_exact_or_throw() {
    let fixture = Fixture::build("compare-to", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("strings", "-1,-1,-2"),
        ("boxed", "-2,-103,-1,-1,-1,1"),
        ("characterBoolean", "-25|-1"),
        ("enums", "1|-1|ClassCastException:null"),
        ("mixed", "ClassCastException:class java.lang.Long cannot be cast to class java.lang.Integer (java.lang.Long and java.lang.Integer are in module java.base of loader 'bootstrap')|ClassCastException:class java.lang.Integer cannot be cast to class java.lang.String (java.lang.Integer and java.lang.String are in module java.base of loader 'bootstrap')"),
        ("nulls", "NullPointerException:Cannot read field \"value\" because \"anotherByte\" is null|NullPointerException:Cannot read field \"value\" because \"anotherShort\" is null|NullPointerException:Cannot read field \"value\" because \"anotherInteger\" is null|NullPointerException:Cannot read field \"value\" because \"anotherLong\" is null|NullPointerException:Cannot read field \"value\" because \"anotherFloat\" is null|NullPointerException:Cannot read field \"value\" because \"anotherDouble\" is null|NullPointerException:Cannot read field \"value\" because \"anotherCharacter\" is null|NullPointerException:Cannot read field \"value\" because \"b\" is null|NullPointerException:Cannot read field \"value\" because \"anotherString\" is null"),
        ("jarAndForgery", "-1|-1|-1"),
    ];
    for (method, expected) in cases {
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
        assert!(
            host.unknown_host_calls().is_empty(),
            "{method}: unexpected counts: {:?}",
            host.unknown_host_calls()
        );
    }
}

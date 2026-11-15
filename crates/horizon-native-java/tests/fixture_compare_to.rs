#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::Value;
use javac_fixture::Fixture;

const PROBE: &str = r#"
package demo;
import java.util.function.Supplier;
public final class Probe {
  enum E { A, B }
  enum F { A }
  static final class C implements Comparable<C> {
    final int n;
    C(int n) { this.n = n; }
    public int compareTo(C other) { return Integer.compare(n, other.n); }
  }
  static String caught(Supplier<Integer> s) {
    try { return String.valueOf(s.get()); }
    catch (Throwable e) { return e.getClass().getSimpleName(); }
  }
  public static String strings() {
    return "" + "a\u0000".compareTo("a\u0001") + ","
      + "a\uD83D\uDE00".compareTo("a\uD83D\uDE01") + "," + "ab".compareTo("abcd");
  }
  public static String boxed() {
    return Integer.valueOf(-1).compareTo(Integer.valueOf(3)) + ","
      + Long.valueOf(Long.MIN_VALUE).compareTo(Long.valueOf(Long.MAX_VALUE)) + ","
      + Double.valueOf(-0.0).compareTo(Double.valueOf(0.0)) + ","
      + Double.valueOf(Double.NaN).compareTo(Double.valueOf(Double.POSITIVE_INFINITY));
  }
  public static String characterBoolean() {
    return Character.valueOf((char) 97).compareTo(Character.valueOf((char) 122)) + "|"
      + Boolean.FALSE.compareTo(Boolean.TRUE);
  }
  public static String enums() {
    return E.B.compareTo(E.A) + "|" + caught(() -> ((Comparable) E.A).compareTo(F.A));
  }
  public static String mixed() {
    return caught(() -> ((Comparable) Integer.valueOf(1)).compareTo(Long.valueOf(1))) + "|"
      + caught(() -> ((Comparable) "x").compareTo(Integer.valueOf(1)));
  }
  public static String jarAndForgery() {
    return new C(1).compareTo(new C(3)) + "|"
      + "identifier/a:b".compareTo("identifier/a:c");
  }
}
"#;

#[test]
fn java25_compare_to_receivers_are_exact_or_throw() {
    let fixture = Fixture::build("compare-to", &[("demo/Probe.java", PROBE)]);
    let cases = [
        ("strings", "-1,-1,-2"),
        ("boxed", "-1,-1,-1,1"),
        ("characterBoolean", "-25|-1"),
        ("enums", "1|ClassCastException"),
        ("mixed", "ClassCastException|ClassCastException"),
        ("jarAndForgery", "-1|-1"),
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

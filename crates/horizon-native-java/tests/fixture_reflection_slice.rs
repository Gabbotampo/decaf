//! Java 25 oracle for the exact reflection slice: `Class.isEnum` and `Field.getInt`.
//!
//! The strings are recorded from Java 25.0.5 with `javac --release 21`; the
//! interpreter must never treat old marker-shaped Strings as Class or Field handles.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.lang.reflect.Field;

public final class Probe {
  enum Plain { A, B }
  enum Body {
    A { @Override public String toString() { return "A"; } },
    B
  }

  public static class Holder {
    public byte b = -7;
    public short s = -300;
    public char c = 'A';
    public int i = 70000;
    public long l = 5000000000L;
    public float f = 1.5f;
    public double d = 2.25;
    public boolean z = true;
    public Object ref;
    public static int STATIC = 13;
    private int hidden = 42;
  }

  public static class Other { public int i = 1; }

  static String readInt(Field field, Object target) {
    try { return "ok:" + field.getInt(target); }
    catch (Throwable error) { return error.getClass().getSimpleName() + ":" + error.getMessage(); }
  }

  static String cast(Class<?> type, Object value) {
    try { type.cast(value); return "ok"; }
    catch (Throwable error) { return error.getClass().getSimpleName(); }
  }

  public static String enums() {
    Object classText = "demo/Probe$Plain";
    Object fieldText = "reflect-field/demo/Probe$Holder#i";
    return Plain.class.isEnum() + "|" + Body.class.isEnum() + "|"
        + Body.A.getClass().isEnum() + "|" + String.class.isEnum() + "|"
        + Thread.State.class.isEnum() + "|" + java.time.DayOfWeek.class.isEnum() + "|"
        + Class.class.isInstance(classText) + "|" + cast(Class.class, classText) + "|"
        + Field.class.isInstance(fieldText) + "|" + cast(Field.class, fieldText);
  }

  public static String fields() throws Exception {
    Holder holder = new Holder();
    String out = readInt(Holder.class.getField("b"), holder) + "|"
        + readInt(Holder.class.getField("s"), holder) + "|"
        + readInt(Holder.class.getField("c"), holder) + "|"
        + readInt(Holder.class.getField("i"), holder) + "|"
        + readInt(Holder.class.getField("l"), holder) + "|"
        + readInt(Holder.class.getField("f"), holder) + "|"
        + readInt(Holder.class.getField("d"), holder) + "|"
        + readInt(Holder.class.getField("z"), holder) + "|"
        + readInt(Holder.class.getField("ref"), holder) + "|"
        + readInt(Holder.class.getField("STATIC"), null) + "|"
        + readInt(Holder.class.getField("STATIC"), new Other()) + "|"
        + readInt(Holder.class.getField("i"), null) + "|"
        + readInt(Holder.class.getField("i"), new Other());
    Field hidden = Holder.class.getDeclaredField("hidden");
    hidden.setAccessible(true);
    return out + "|" + readInt(hidden, holder);
  }
}
"#;

fn fixture() -> Fixture {
    Fixture::build("reflection-slice", &[("demo/Probe.java", PROBE)])
}

fn run(method: &str) -> (String, Vec<(String, usize)>) {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture.text_on(&mut host, method);
    (value, host.unknown_host_calls())
}

#[test]
fn class_is_enum_uses_held_jdk_and_jar_models_and_rejects_forged_strings() {
    let (value, unknown) = run("enums");
    assert_eq!(
        value,
        "true|true|false|false|true|true|false|ClassCastException|false|ClassCastException"
    );
    assert!(unknown.is_empty(), "counted: {unknown:?}");
}

#[test]
fn field_get_int_widens_and_checks_receivers_and_access() {
    let (value, unknown) = run("fields");
    assert_eq!(
        value,
        "ok:-7|ok:-300|ok:65|ok:70000|IllegalArgumentException:Attempt to get long field \"demo.Probe$Holder.l\" with illegal data type conversion to int|IllegalArgumentException:Attempt to get float field \"demo.Probe$Holder.f\" with illegal data type conversion to int|IllegalArgumentException:Attempt to get double field \"demo.Probe$Holder.d\" with illegal data type conversion to int|IllegalArgumentException:Attempt to get boolean field \"demo.Probe$Holder.z\" with illegal data type conversion to int|IllegalArgumentException:Attempt to get java.lang.Object field \"demo.Probe$Holder.ref\" with illegal data type conversion to int|ok:13|ok:13|NullPointerException:Cannot invoke \"Object.getClass()\" because \"obj\" is null|IllegalArgumentException:Can not get int field demo.Probe$Holder.i on demo.Probe$Other|ok:42"
    );
    assert!(unknown.is_empty(), "counted: {unknown:?}");
}

#[test]
fn field_text_does_not_become_a_field_handle() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture.text_on(&mut host, "enums");
    assert!(value.contains("|false|ClassCastException"));
}

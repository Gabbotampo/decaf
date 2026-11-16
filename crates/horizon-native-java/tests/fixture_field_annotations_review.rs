#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.lang.annotation.Annotation;
import java.lang.annotation.Inherited;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.reflect.Method;

@Retention(RetentionPolicy.RUNTIME)
@interface N { int x(); }

enum E { A, B }

@Retention(RetentionPolicy.RUNTIME)
@interface A {
  byte b(); char c(); double d(); float f(); int i(); E en(); Class<?> cls(); long l();
  String[] sa(); N n(); N[] na(); String s(); short sh(); int[] ia(); boolean z(); E[] ea(); Class<?>[] ca();
}

@Retention(RetentionPolicy.RUNTIME)
@Inherited
@interface I { String value(); }

@I("parent")
class Parent {}

final class Child extends Parent {}

@A(b=(byte)3, c='x', d=2.5, f=1.5f, i=5, en=E.B, cls=String.class, l=6L,
   sa={"x", "y"}, n=@N(x=4), na={@N(x=4), @N(x=8)}, s="hi", sh=(short)4,
   ia={1, 2}, z=true, ea={E.A, E.B}, ca={String.class, int.class})
public final class Probe {
  public static String objects() {
    A a = Probe.class.getAnnotation(A.class);
    return a.toString() + "|" + a.equals(a);
  }

  public static String inherited() {
    I value = Child.class.getAnnotation(I.class);
    return value.value() + "|" + Child.class.getDeclaredAnnotations().length + "|"
        + Child.class.isAnnotationPresent(I.class);
  }

  public static String unheld() throws Exception {
    Class<?> type = String.class;
    Method method = String.class.getDeclaredMethod("length");
    return (type.getAnnotation(A.class) == null) + ":" + (method.getAnnotation(A.class) == null);
  }
}
"#;

#[test]
fn annotation_objects_match_java_25_oracle() {
    let fixture = Fixture::build("field-annotations-review", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let objects = fixture.text_on(&mut host, "objects");
    assert_eq!(
        objects,
        "@A(b=(byte)0x03, c='x', d=2.5, f=1.5f, i=5, en=B, cls=java.lang.String.class, l=6L, sa={\"x\", \"y\"}, n=@N(x=4), na={@N(x=4), @N(x=8)}, s=\"hi\", sh=4, ia={1, 2}, z=true, ea={A, B}, ca={java.lang.String.class, int.class})|true"
    );
}

#[test]
fn inherited_class_annotations_match_java_25() {
    let fixture = Fixture::build("field-annotations-inherited", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "inherited"), "parent|0|true");
}

#[test]
fn unheld_class_and_method_queries_are_counted() {
    let fixture = Fixture::build("field-annotations-unheld", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "unheld"), "true:true");
    let unknown = host.unknown_host_calls();
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("java/lang/Class#getAnnotation")),
        "{unknown:?}"
    );
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("java/lang/reflect/Method#getAnnotation")),
        "{unknown:?}"
    );
}

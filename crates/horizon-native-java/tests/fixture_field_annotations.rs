#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.lang.annotation.Annotation;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Repeatable;
import java.lang.reflect.Field;
import java.lang.reflect.Method;

@Retention(RetentionPolicy.RUNTIME)
@Repeatable(Tags.class)
@interface Tag {
  String value() default "default";
  int n() default 7;
}

@Retention(RetentionPolicy.RUNTIME)
@interface Tags { Tag[] value(); }

@Retention(RetentionPolicy.RUNTIME)
@interface Single { String value() default "single-default"; }

@Retention(RetentionPolicy.CLASS)
@interface Invisible {}

@Tag("class-a")
@Tag(value = "class-b", n = 8)
@Single
@Invisible
public final class Probe {
  @Tag("field-a")
  @Tag(value = "field-b", n = 9)
  @Invisible
  public int field;

  @Tag("method-a")
  @Invisible
  public void run() {}

  @Single
  public void single() {}

  private static String tags(Tag[] values) {
    return values.length + ":" + values[0].value() + ":" + values[0].n()
        + ":" + values[1].value() + ":" + values[1].n();
  }

  public static String fields() throws Exception {
    Field field = Probe.class.getDeclaredField("field");
    Tag[] first = field.getAnnotationsByType(Tag.class);
    Tag[] second = field.getAnnotationsByType(Tag.class);
    Tags container = field.getAnnotation(Tags.class);
    return tags(first) + "|" + first.getClass().getComponentType().getName()
        + "|" + field.getDeclaredAnnotations().length + "|"
        + field.getDeclaredAnnotations()[0].annotationType().getName()
        + "|" + (first != second) + "|" + (field.getAnnotation(Tag.class) == null)
        + "|" + container.value().length + ":" + container.value()[1].value();
  }

  public static String methods() throws Exception {
    Method run = Probe.class.getDeclaredMethod("run");
    Method single = Probe.class.getDeclaredMethod("single");
    Tag[] values = run.getDeclaredAnnotationsByType(Tag.class);
    Single annotation = single.getAnnotation(Single.class);
    return values.length + ":" + values[0].value() + ":" + values[0].n()
        + "|" + run.getDeclaredAnnotations().length + "|"
        + (run.getAnnotation(Tag.class) == null) + "|" + annotation.value()
        + "|" + annotation.annotationType().getName();
  }

  public static String classes() {
    Tag[] values = Probe.class.getDeclaredAnnotationsByType(Tag.class);
    Single single = Probe.class.getAnnotation(Single.class);
    Annotation[] declared = Probe.class.getDeclaredAnnotations();
    return tags(values) + "|" + declared.length + "|"
        + declared[0].annotationType().getName() + "|" + single.value()
        + "|" + (Probe.class.getAnnotation(Tags.class) != null)
        + "|" + (Probe.class.getAnnotation(Tag.class) == null);
  }

  public static String forged() {
    Object text = "annotation/Ldemo/Tag;/0";
    return Tag.class.isInstance(text) + "|" + Tag.class.isAssignableFrom(String.class);
  }
}
"#;

fn fixture() -> Fixture {
    Fixture::build("field-annotations", &[("demo/Probe.java", PROBE)])
}

fn run(method: &str) -> (String, Vec<(String, usize)>) {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture.text_on(&mut host, method);
    (value, host.unknown_host_calls())
}

#[test]
fn field_annotations_are_java_25_exact() {
    let (value, unknown) = run("fields");
    assert_eq!(
        value,
        "2:field-a:7:field-b:9|demo.Tag|1|demo.Tags|true|true|2:field-b"
    );
    assert!(unknown.is_empty(), "counted: {unknown:?}");
}

#[test]
fn method_and_class_annotations_are_java_25_exact() {
    let (methods, unknown_methods) = run("methods");
    assert_eq!(methods, "1:method-a:7|1|false|single-default|demo.Single");
    assert!(unknown_methods.is_empty(), "counted: {unknown_methods:?}");
    let (classes, unknown_classes) = run("classes");
    assert_eq!(
        classes,
        "2:class-a:7:class-b:8|2|demo.Tags|single-default|true|true"
    );
    assert!(unknown_classes.is_empty(), "counted: {unknown_classes:?}");
}

#[test]
fn forged_annotation_text_is_not_a_proxy() {
    let (value, unknown) = run("forged");
    assert_eq!(value, "false|false");
    assert!(unknown.is_empty(), "counted: {unknown:?}");
}

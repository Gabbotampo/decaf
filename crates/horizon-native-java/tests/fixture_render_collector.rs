#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};
const PROBE: &str = include_str!("fixtures/render_collector/demo/Probe.java");
const STUBS: &[(&str, &str)] = &[
    (
        "com/mojang/blaze3d/vertex/PoseStack.java",
        r#"package com.mojang.blaze3d.vertex; public class PoseStack { public PoseStack(){} public Pose last(){return null;} public static class Pose { public org.joml.Matrix4f pose(){return null;} } }"#,
    ),
    (
        "org/joml/Matrix4f.java",
        r#"package org.joml; public class Matrix4f { public Matrix4f(){} public float m00(){return 0;} }"#,
    ),
];
fn fixture() -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    javac_fixture::Fixture::build("render-collector", &sources)
}
#[test]
fn returned_render_handles_survive_java_callbacks_and_are_collected() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());
    let stack = f
        .jar
        .invoke_method_with_args(
            "demo.Probe",
            "build",
            "()Lcom/mojang/blaze3d/vertex/PoseStack;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    host.retain(&stack);
    let matrix = f
        .jar
        .invoke_method_with_args(
            "demo.Probe",
            "matrix",
            "(Lcom/mojang/blaze3d/vertex/PoseStack;)Lorg/joml/Matrix4f;",
            None,
            std::slice::from_ref(&stack),
            &mut host,
        )
        .unwrap();
    host.retain(&matrix);
    let used = f
        .jar
        .invoke_method_with_args(
            "demo.Probe",
            "useMatrix",
            "(Lorg/joml/Matrix4f;)Ljava/lang/String;",
            None,
            std::slice::from_ref(&matrix),
            &mut host,
        )
        .unwrap();
    assert!(matches!(used, Value::Ref(Some(text)) if text == "1.0"));
    f.jar
        .invoke_method_with_args("demo.Probe", "churn", "()V", None, &[], &mut host)
        .unwrap();
    let used = f
        .jar
        .invoke_method_with_args(
            "demo.Probe",
            "useMatrix",
            "(Lorg/joml/Matrix4f;)Ljava/lang/String;",
            None,
            std::slice::from_ref(&matrix),
            &mut host,
        )
        .unwrap();
    assert!(matches!(used, Value::Ref(Some(text)) if text == "1.0"));
    host.release(&stack);
    host.release(&matrix);
    f.jar
        .invoke_method_with_args("demo.Probe", "churn", "()V", None, &[], &mut host)
        .unwrap();
    assert_eq!(host.retained_count(), 0);
    assert!(host.freed_object_ranges() < 64);
}

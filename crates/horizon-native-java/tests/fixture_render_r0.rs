#[path = "support/javac_fixture.rs"]
mod javac_fixture;
const ORACLE: &str = include_str!("fixtures/render_r0/java25.txt");
const TRIG_AARCH64: &str = include_str!("fixtures/render_r0/trig_aarch64.txt");
const TRIG_X86_64: &str = include_str!("fixtures/render_r0/trig_x86_64.txt");
const NORMAL_ORACLE: &str = include_str!("fixtures/render_r0/normal_java25.txt");
const PROBE: &str = include_str!("fixtures/render_r0/demo/Probe.java");
const STUBS: &[(&str, &str)] = &[
    (
        "com/mojang/blaze3d/vertex/PoseStack.java",
        r#"package com.mojang.blaze3d.vertex; public class PoseStack { public PoseStack(){} public void translate(float x,float y,float z){} public void scale(float x,float y,float z){} public void rotateDegrees(com.mojang.math.Axis a,float d){} public void pushPose(){} public void popPose(){} public Pose last(){return null;} public static class Pose { public org.joml.Matrix4f pose(){return null;} } }"#,
    ),
    (
        "com/mojang/math/Axis.java",
        r#"package com.mojang.math; public interface Axis { Axis XP=null, XN=null, YP=null, YN=null, ZP=null, ZN=null; org.joml.Quaternionf rotation(float a); }"#,
    ),
    (
        "org/joml/Matrix4f.java",
        r#"package org.joml; public class Matrix4f { public Matrix4f(){} public float m00(){return 0;} public float m11(){return 0;} public float m22(){return 0;} public float m30(){return 0;} public float m31(){return 0;} public float m32(){return 0;} }"#,
    ),
    (
        "org/joml/Quaternionf.java",
        r#"package org.joml; public class Quaternionf { public Quaternionf(){} }"#,
    ),
    (
        "org/joml/Vector3f.java",
        r#"package org.joml; public class Vector3f { public Vector3f(){} }"#,
    ),
];
#[test]
fn pose_stack_matches_java25_oracle() {
    let mut src = STUBS.to_vec();
    src.push(("demo/Probe.java", PROBE));
    let f = javac_fixture::Fixture::build("render-r0", &src);
    let mut h = f.jar.seed_host(horizon_native_java::StdHost::new());
    let v = f
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "run",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut h,
        )
        .unwrap();
    let got = match v {
        horizon_native_java::Value::Ref(Some(s)) => s,
        x => format!("{x:?}"),
    };
    assert_eq!(got, ORACLE.trim());
    assert!(
        h.unknown_host_calls().is_empty(),
        "{:?}",
        h.unknown_host_calls()
    );
}

#[test]
fn captured_architecture_trig_oracles_are_kept_as_counted_rows() {
    assert_eq!(TRIG_AARCH64.lines().count(), 5);
    assert_eq!(TRIG_X86_64.lines().count(), 5);
    assert_eq!(NORMAL_ORACLE.lines().count(), 5);
}

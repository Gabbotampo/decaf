//! Adversarial identity provenance fixture: every fresh allocation gets one exact identity token,
//! and every ambiguous case (`String.intern()`, loop allocation, whole-string `substring`, Mixin
//! `@ModifyArg` / `@ModifyVariable` reference swaps) is counted instead of guessed.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{MethodRef, StdHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/adversarial_identity/Probe.java");

#[test]
fn adversarial_identity_review() {
    let fixture = Fixture::build("adversarial-identity-review", &[("demo/Probe.java", PROBE)]);
    // Every expected string is captured from JDK 25.0.5 on `fixtures/adversarial_identity/Probe.java`.
    let cases = [
        ("sameClassCallsOut", "2:-1"),
        ("computeCallsOut", "2:-1"),
        ("classCallsOut", "2"),
        ("recursionOut", "2:-1"),
        ("overloadCallsOut", "2"),
        ("staticInitOut", "2:-1"),
        ("fieldCaseOut", "1:2:2"),
        ("listCaseOut", "1:2:2"),
        ("lambdaCaseOut", "2:-1"),
        ("identityCmpHelperOut", "false"),
        ("lambdaIdentityCmpOut", "false"),
        ("recCmpOut", "false"),
        ("loopAllocOut", "2:-1"),
        ("stringValueOfOut", "1"),
        ("wholeSubstringOut", "true:1"),
        ("callbackMutationOut", "3:2:3:2"),
        ("computeValueOverwriteOut", "false"),
        (
            "dynamicInternOut",
            "A intern self=true|B lookup=1 same=true",
        ),
        // Integer > 127 caching depends on -XX:AutoBoxCacheMax /
        // -Djava.lang.Integer.IntegerCache.high (forwarded via JAVA_TOOL_OPTIONS /
        // _JAVA_OPTIONS / JDK_JAVA_OPTIONS):
        //   JDK 25.0.5 default:               integerAbove127EqOut="false", integerAbove127MapOut="2:null"
        //   JDK 25.0.5 -XX:AutoBoxCacheMax=1000 (and -Djava.lang.Integer.IntegerCache.high=1000):
        //                                     integerAbove127EqOut="true",  integerAbove127MapOut="1:b"
        // Because the JVM flag changes the answer, Decaf counts Integer > 127.
        ("integerAbove127EqOut", "false"),
        ("integerAbove127MapOut", "2:null"),
        // Live Map.Entry.setValue, entrySet().toArray() detached SimpleEntry,
        // replaceAll, and entrySet().iterator().remove() on IdentityHashMap:
        ("entrySetValueOut", "false:v"),
        ("entrySetValueOpaqueOut", "false:v"),
        ("entryToArraySetValueOut", "true:null"),
        ("replaceAllValueOut", "false:v"),
        ("entryIterRemoveOut", "0:null"),
    ];
    let exact = [
        "sameClassCallsOut",
        "computeCallsOut",
        "classCallsOut",
        "recursionOut",
        "overloadCallsOut",
        "staticInitOut",
        "lambdaCaseOut",
        "stringValueOfOut",
        "callbackMutationOut",
        "entrySetValueOut",
        "entryToArraySetValueOut",
        "entryIterRemoveOut",
    ];
    let must_count = [
        "loopAllocOut",
        "wholeSubstringOut",
        "computeValueOverwriteOut",
        "dynamicInternOut",
        "integerAbove127EqOut",
        "integerAbove127MapOut",
        "entrySetValueOpaqueOut",
        "replaceAllValueOut",
    ];
    for (method, expected) in cases {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let value = fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap();
        let actual = match value {
            Value::Ref(Some(s)) => s,
            other => panic!("{method}: {other:?}"),
        };
        let counted = host.unknown_host_call_count();
        if exact.contains(&method) {
            assert_eq!(
                counted,
                0,
                "{method}: counted {:?}",
                host.unknown_host_calls()
            );
            assert_eq!(actual, expected, "{method}");
        } else if must_count.contains(&method) {
            assert!(
                counted > 0,
                "{method} answered {actual:?} (JDK 25: {expected:?}) without counting"
            );
            if method == "dynamicInternOut" {
                let calls = host.unknown_host_calls();
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/util/IdentityHashMap#get(Ljava/lang/Object;)Ljava/lang/Object;"),
                    "dynamicInternOut must count IdentityHashMap#get: {calls:?}"
                );
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/lang/String#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z"),
                    "dynamicInternOut must count String#if_acmp: {calls:?}"
                );
            }
            if method == "integerAbove127EqOut" {
                let calls = host.unknown_host_calls();
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z"),
                    "integerAbove127EqOut must count Object#if_acmp: {calls:?}"
                );
            }
            if method == "integerAbove127MapOut" {
                let calls = host.unknown_host_calls();
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/util/IdentityHashMap#put(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                    "integerAbove127MapOut must count IdentityHashMap#put: {calls:?}"
                );
            }
            if method == "entrySetValueOpaqueOut" {
                let calls = host.unknown_host_calls();
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/util/IdentityHashMap#remove(Ljava/lang/Object;Ljava/lang/Object;)Z"),
                    "entrySetValueOpaqueOut must count IdentityHashMap#remove(k, v): {calls:?}"
                );
            }
            if method == "replaceAllValueOut" {
                let calls = host.unknown_host_calls();
                assert!(
                    calls.iter().any(|(call, _)| call
                        == "java/util/IdentityHashMap#replaceAll(Ljava/util/function/BiFunction;)V"),
                    "replaceAllValueOut must count IdentityHashMap#replaceAll: {calls:?}"
                );
            }
        } else if counted == 0 {
            assert_eq!(actual, expected, "{method}");
        } else {
            assert!(
                counted > 0,
                "{method}: identity answer was neither exact nor counted"
            );
        }
    }
}

#[test]
fn dynamic_intern_aliasing_in_identity_set_is_counted() {
    let sets_stub = r#"package com.google.common.collect;
public final class Sets {
  public static <E> java.util.Set<E> newIdentityHashSet() { throw new AssertionError(); }
}"#;
    let probe = r#"package demo;
import com.google.common.collect.Sets;
import java.util.Set;
public final class Probe {
  public static String run() {
    Set<String> set = Sets.newIdentityHashSet();
    String s = new StringBuilder("xy").append("zq").toString();
    s.intern();
    set.add(s);
    return String.valueOf(set.contains("xyzq"));
  }
}"#;
    let fixture = Fixture::build(
        "dynamic-intern-identity-set",
        &[
            ("com/google/common/collect/Sets.java", sets_stub),
            ("demo/Probe.java", probe),
        ],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.text_on(&mut host, "run");
    let calls = host.unknown_host_calls();
    assert!(
        calls
            .iter()
            .any(|(call, _)| call == "java/util/Set#contains(Ljava/lang/Object;)Z"),
        "interned fresh key vs literal in IdentityHashSet must count Set#contains: {calls:?}"
    );
}

#[test]
fn mixin_modified_identity_arguments_are_counted() {
    let mixin_ann = r#"package org.spongepowered.asm.mixin;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.TYPE)
public @interface Mixin {
    Class<?>[] value() default {};
    String[] targets() default {};
}"#;
    let at_ann = r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target({})
public @interface At {
    String value() default "";
    String target() default "";
    int ordinal() default -1;
}"#;
    let modify_arg_ann = r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface ModifyArg {
    String[] method();
    At at();
    int index() default 0;
    boolean cancellable() default false;
}"#;
    let modify_var_ann = r#"package org.spongepowered.asm.mixin.injection;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface ModifyVariable {
    String[] method();
    At at();
    int index() default -1;
    int ordinal() default -1;
    String[] name() default {};
    boolean argsOnly() default false;
    boolean print() default false;
}"#;
    let target = r#"package demo;
import java.util.IdentityHashMap;
public final class Target {
    public static String B;
    public static int unmodified() {
        String a = new String("x");
        String b = new String("x");
        IdentityHashMap<String, Integer> m = new IdentityHashMap<>();
        m.put(a, 1);
        m.put(b, 2);
        Integer v = m.get(a);
        return v == null ? -1 : v;
    }
    public static int lookupWithModifyArg() {
        String a = new String("x");
        String b = new String("x");
        B = b;
        IdentityHashMap<String, Integer> m = new IdentityHashMap<>();
        m.put(a, 1);
        m.put(b, 2);
        Integer v = m.get(a);
        return v == null ? -1 : v;
    }
    public static int lookupWithModifyVarLoad() {
        String a = new String("x");
        String b = new String("x");
        B = b;
        IdentityHashMap<String, Integer> m = new IdentityHashMap<>();
        m.put(a, 1);
        m.put(b, 2);
        String k = a;
        Integer v = m.get(k);
        return v == null ? -1 : v;
    }
}"#;
    let mixin = r#"package demo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyArg;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

@Mixin(targets = {"Ldemo/Target;"})
public abstract class IdentityMixin {
    @ModifyArg(
        method = "lookupWithModifyArg()I",
        at = @At(value = "INVOKE", target = "Ljava/util/IdentityHashMap;get(Ljava/lang/Object;)Ljava/lang/Object;"),
        index = 0
    )
    private static Object swapArgToB(Object arg) {
        return Target.B;
    }

    @ModifyVariable(
        method = "lookupWithModifyVarLoad()I",
        at = @At("LOAD"),
        index = 3
    )
    private static String swapLoadedLocalToB(String value) {
        return Target.B;
    }
}"#;
    let fixture = Fixture::build(
        "mixin-identity-modify",
        &[
            ("org/spongepowered/asm/mixin/Mixin.java", mixin_ann),
            ("org/spongepowered/asm/mixin/injection/At.java", at_ann),
            (
                "org/spongepowered/asm/mixin/injection/ModifyArg.java",
                modify_arg_ann,
            ),
            (
                "org/spongepowered/asm/mixin/injection/ModifyVariable.java",
                modify_var_ann,
            ),
            ("demo/Target.java", target),
            ("demo/IdentityMixin.java", mixin),
        ],
    );

    // Baseline without modifier: exact `1`, zero counted calls.
    let mut host = fixture.jar.seed_host(StdHost::new());
    let baseline = fixture
        .jar
        .invoke_method_with_args("demo/Target", "unmodified", "()I", None, &[], &mut host)
        .unwrap();
    assert_eq!(baseline, Value::Int(1));
    assert_eq!(host.unknown_host_call_count(), 0);

    // `@ModifyArg` swaps `m.get(a)` to `m.get(b)` (JDK 25: `2`): must drop static provenance
    // and count `IdentityHashMap#get` rather than answering `1` from pre-modifier `a`.
    let mut host = fixture.jar.seed_host(StdHost::new());
    let report = fixture
        .jar
        .execute_discovered_mixins(
            &MethodRef {
                owner: "demo/Target".into(),
                name: "lookupWithModifyArg".into(),
                descriptor: "()I".into(),
            },
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert!(
        !report.callback_results.is_empty(),
        "expected @ModifyArg to run: {report:?}"
    );
    let calls = host.unknown_host_calls();
    assert!(
        calls.iter().any(|(call, _)| call
            == "java/util/IdentityHashMap#get(Ljava/lang/Object;)Ljava/lang/Object;"),
        "@ModifyArg swapping equal-text String must count IdentityHashMap#get: {calls:?}"
    );

    // `@ModifyVariable(LOAD)` swaps `aload_3` (`k = a`) to `b` (JDK 25: `2`): must drop static
    // provenance and count `IdentityHashMap#get`.
    let mut host = fixture.jar.seed_host(StdHost::new());
    let report = fixture
        .jar
        .execute_discovered_mixins(
            &MethodRef {
                owner: "demo/Target".into(),
                name: "lookupWithModifyVarLoad".into(),
                descriptor: "()I".into(),
            },
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert!(
        !report.callback_results.is_empty(),
        "expected @ModifyVariable(LOAD) to run: {report:?}"
    );
    let calls = host.unknown_host_calls();
    assert!(
        calls.iter().any(|(call, _)| call
            == "java/util/IdentityHashMap#get(Ljava/lang/Object;)Ljava/lang/Object;"),
        "@ModifyVariable(LOAD) swapping equal-text String must count IdentityHashMap#get: {calls:?}"
    );
}

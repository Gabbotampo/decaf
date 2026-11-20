//! `LootItemConditionalFunction.commonFields(instance)`, as a loot function subclass calls it
//! unqualified (so its `invokestatic` names the subclass), answers the modelled `Products.P1`. A jar
//! class that declares its own `commonFields` keeps its own body: the model never replaces jar code
//! (#410 review). Only `demo/` ships; the game and DFU classes are compile-only stubs.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const SOURCES: &[(&str, &str)] = &[
    (
        "com/mojang/datafixers/Products.java",
        "package com.mojang.datafixers; public interface Products { final class P1<F, T1> { } }",
    ),
    (
        "com/mojang/serialization/codecs/RecordCodecBuilder.java",
        "package com.mojang.serialization.codecs; public final class RecordCodecBuilder<O, F> { public static final class Instance<O> { } public static final class Mu<O> { } }",
    ),
    (
        "net/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction.java",
        r#"package net.minecraft.world.level.storage.loot.functions;
import com.mojang.datafixers.Products;
import com.mojang.serialization.codecs.RecordCodecBuilder;
public abstract class LootItemConditionalFunction {
  protected static <T extends LootItemConditionalFunction> Products.P1<RecordCodecBuilder.Mu<T>, Object> commonFields(RecordCodecBuilder.Instance<T> i) { return null; }
}"#,
    ),
    (
        "demo/Inherits.java",
        r#"package demo;
import net.minecraft.world.level.storage.loot.functions.LootItemConditionalFunction;
public final class Inherits extends LootItemConditionalFunction {
  public static String fields() { return String.valueOf(commonFields(null) != null); }
}"#,
    ),
    (
        "demo/Declares.java",
        r#"package demo;
import com.mojang.datafixers.Products;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import net.minecraft.world.level.storage.loot.functions.LootItemConditionalFunction;
public final class Declares extends LootItemConditionalFunction {
  static boolean ran;
  protected static <T extends LootItemConditionalFunction> Products.P1<RecordCodecBuilder.Mu<T>, Object> commonFields(RecordCodecBuilder.Instance<T> i) { ran = true; return null; }
  public static String fields() { Object p = commonFields(null); return ran + "|" + (p != null); }
}"#,
    ),
];

fn fields(fixture: &Fixture, class: &str) -> (String, usize) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture
        .jar
        .invoke_method(class, "fields", "()Ljava/lang/String;", None, &mut host)
        .unwrap();
    let Value::Ref(Some(text)) = value else {
        panic!("{class}: {value:?}");
    };
    (text, host.unknown_host_call_count())
}

#[test]
fn inherited_common_fields_is_modelled_and_a_jar_override_runs_its_own_body() {
    let fixture = Fixture::build("loot-common-fields", SOURCES);
    // Inherited from the vanilla class: the modelled P1, nothing counted.
    assert_eq!(fields(&fixture, "demo/Inherits"), ("true".into(), 0));
    // Declared by the jar: its own body runs (it answers null), never the model.
    assert_eq!(fields(&fixture, "demo/Declares"), ("true|false".into(), 0));
}

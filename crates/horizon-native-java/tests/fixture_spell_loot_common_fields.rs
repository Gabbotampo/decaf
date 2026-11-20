//! The static codec shape used by loot-function subclasses: `commonFields(i).and(i.group(...))`.
//! The fixture is compiled with tiny vanilla/DFU stubs and checks that a bounded refusal remains
//! counted and non-null instead of becoming an impossible NPE at `Products.P1.and`.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const SOURCES: &[(&str, &str)] = &[
    (
        "com/mojang/datafixers/kinds/App.java",
        "package com.mojang.datafixers.kinds; public interface App<F,A> {}",
    ),
    (
        "com/mojang/datafixers/Products.java",
        r#"package com.mojang.datafixers;
public interface Products {
  final class P1<F,T> implements com.mojang.datafixers.kinds.App<F,T> { public P4 and(P3<?,?,?,?> p) { return null; } }
  final class P3<F,A,B,C> implements com.mojang.datafixers.kinds.App<F,java.lang.Object> { }
  final class P4<F,A,B,C,D> implements com.mojang.datafixers.kinds.App<F,java.lang.Object> { }
}"#,
    ),
    (
        "com/mojang/serialization/Codec.java",
        r#"package com.mojang.serialization;
import com.mojang.serialization.codecs.RecordCodecBuilder;
public class Codec<A> {
  public static final Codec<String> STRING = null;
  public MapCodec<A> optionalFieldOf(String name) { return null; }
  public MapCodec<A> fieldOf(String name) { return null; }
}"#,
    ),
    (
        "com/mojang/serialization/MapCodec.java",
        r#"package com.mojang.serialization;
import com.mojang.serialization.codecs.RecordCodecBuilder;
public class MapCodec<A> {
  public <T> RecordCodecBuilder<?,?> forGetter(java.util.function.Function<T,A> getter) { return null; }
}"#,
    ),
    (
        "com/mojang/serialization/codecs/RecordCodecBuilder.java",
        r#"package com.mojang.serialization.codecs;
import com.mojang.datafixers.Products;
import com.mojang.datafixers.kinds.App;
import com.mojang.serialization.MapCodec;
public final class RecordCodecBuilder<O,F> implements App<RecordCodecBuilder.Mu<O>,F> {
  public static <O> Instance<O> instance() { return null; }
  public static <O> MapCodec<O> mapCodec(java.util.function.Function<Instance<O>,App<?,?>> f) { return null; }
  public static final class Instance<O> {
    public Products.P3<?,?,?,?> group(App<?,?> a, App<?,?> b, App<?,?> c) { return null; }
  }
  public static final class Mu<O> { }
}"#,
    ),
    (
        "net/minecraft/world/level/storage/loot/functions/LootItemConditionalFunction.java",
        r#"package net.minecraft.world.level.storage.loot.functions;
import com.mojang.datafixers.Products;
import com.mojang.serialization.codecs.RecordCodecBuilder;
public abstract class LootItemConditionalFunction {
  protected LootItemConditionalFunction(java.util.Optional<?> condition) {}
  protected static <T extends LootItemConditionalFunction> Products.P1<RecordCodecBuilder.Mu<T>,Object> commonFields(RecordCodecBuilder.Instance<T> i) { return null; }
}"#,
    ),
    (
        "demo/SpellShape.java",
        r#"package demo;
import com.mojang.datafixers.Products;
import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import net.minecraft.world.level.storage.loot.functions.LootItemConditionalFunction;
public final class SpellShape extends LootItemConditionalFunction {
  private SpellShape() { super(java.util.Optional.empty()); }
  public static final MapCodec<SpellShape> CODEC = RecordCodecBuilder.mapCodec(i ->
    commonFields(i).and(i.group(
      Codec.STRING.fieldOf("pool").forGetter(x -> "pool"),
      Codec.STRING.fieldOf("tier").forGetter(x -> "tier"),
      Codec.STRING.fieldOf("count").forGetter(x -> "count"))));
  public static String probe() { return CODEC == null ? "null" : "ok"; }
}"#,
    ),
];

#[test]
fn static_common_fields_and_group_stay_non_null_when_counted() {
    let fixture = Fixture::build("spell-loot-common-fields", SOURCES);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _value = fixture
        .jar
        .invoke_method(
            "demo/SpellShape",
            "probe",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .expect("the static codec lambda must not throw");
    assert_eq!(
        host.unknown_host_calls(),
        &[ (
            "com/mojang/serialization/codecs/RecordCodecBuilder#mapCodec(Ljava/util/function/Function;)Lcom/mojang/serialization/MapCodec;".into(),
            1,
        ) ],
        "the stubbed mapCodec call is the only unknown operation; construction must complete without an invented exception"
    );
}

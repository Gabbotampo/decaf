//! Java 25 oracle comparison and real-jar regression tests for Bundle API's
//! host-side surface (`Fraction`, `ARGB`, `Mth.mulAndTruncate`, `SimpleMutableContainer` /
//! `GrowableMutableContainer`, DFU `RecordCodecBuilder` / `ItemPredicate.CODEC` /
//! `CollectionPredicate.codec`, and `ByteBufCodecs.list` / `StreamCodec.apply` round trips).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{DfuCodecResult, NativeError, NativeJar, StdHost, Value};
use std::path::Path;

const PROBE: &str = include_str!("fixtures/bundle_api/demo/BundleApiProbe.java");
const ORACLE_FRACTION: &str = include_str!("fixtures/bundle_api/fraction.txt");
const ORACLE_ARGB: &str = include_str!("fixtures/bundle_api/argb.txt");
const ORACLE_RECORD_CODEC: &str = include_str!("fixtures/bundle_api/record_codec.txt");
const ORACLE_STREAM_CODEC: &str = include_str!("fixtures/bundle_api/stream_codec_list.txt");

const STUBS: &[(&str, &str)] = &[
    (
        "com/mojang/datafixers/kinds/App.java",
        "package com.mojang.datafixers.kinds; public interface App<F, A> {}",
    ),
    (
        "com/mojang/datafixers/kinds/Applicative.java",
        "package com.mojang.datafixers.kinds; public interface Applicative<F, Mu> {}",
    ),
    (
        "com/mojang/datafixers/Products.java",
        "package com.mojang.datafixers;\n\
         import com.mojang.datafixers.kinds.*; import java.util.function.*;\n\
         public interface Products {\n\
           final class P1<F, T1> {\n\
             public <R> App<F, R> apply(Applicative<F, ?> a, Function<T1, R> f) { throw new AssertionError(); }\n\
           }\n\
           final class P2<F, T1, T2> {\n\
             public <R> App<F, R> apply(Applicative<F, ?> a, BiFunction<T1, T2, R> f) { throw new AssertionError(); }\n\
           }\n\
         }",
    ),
    (
        "com/mojang/serialization/MapCodec.java",
        "package com.mojang.serialization;\n\
         import com.mojang.serialization.codecs.RecordCodecBuilder; import java.util.function.Function;\n\
         public abstract class MapCodec<A> {\n\
           public <O> RecordCodecBuilder<O, A> forGetter(Function<O, A> getter) { throw new AssertionError(); }\n\
           public Codec<A> codec() { throw new AssertionError(); }\n\
         }",
    ),
    (
        "com/mojang/serialization/Codec.java",
        "package com.mojang.serialization;\n\
         import com.mojang.serialization.codecs.PrimitiveCodec;\n\
         import java.util.List; import java.util.Optional; import java.util.function.Function;\n\
         public interface Codec<A> {\n\
           PrimitiveCodec<Integer> INT = null;\n\
           PrimitiveCodec<String> STRING = null;\n\
           MapCodec<A> fieldOf(String name);\n\
           MapCodec<Optional<A>> optionalFieldOf(String name);\n\
           MapCodec<A> optionalFieldOf(String name, A defaultValue);\n\
           Codec<List<A>> listOf();\n\
           <S> Codec<S> xmap(Function<? super A, ? extends S> to, Function<? super S, ? extends A> from);\n\
         }",
    ),
    (
        "com/mojang/serialization/codecs/PrimitiveCodec.java",
        "package com.mojang.serialization.codecs;\n\
         import com.mojang.serialization.Codec;\n\
         public interface PrimitiveCodec<A> extends Codec<A> {}",
    ),
    (
        "com/mojang/serialization/codecs/RecordCodecBuilder.java",
        "package com.mojang.serialization.codecs;\n\
         import com.mojang.datafixers.Products; import com.mojang.datafixers.kinds.*;\n\
         import com.mojang.serialization.Codec; import java.util.function.Function;\n\
         public final class RecordCodecBuilder<O, F> implements App<RecordCodecBuilder.Mu<O>, F> {\n\
           public static final class Mu<O> {}\n\
           public static <O> Codec<O> create(Function<Instance<O>, ? extends App<Mu<O>, O>> builder) { throw new AssertionError(); }\n\
           public static final class Instance<O> implements Applicative<Mu<O>, Instance.Mu<O>> {\n\
             public static final class Mu<O> {}\n\
             public <T1> Products.P1<RecordCodecBuilder.Mu<O>, T1> group(App<RecordCodecBuilder.Mu<O>, T1> t1) { throw new AssertionError(); }\n\
             public <T1, T2> Products.P2<RecordCodecBuilder.Mu<O>, T1, T2> group(App<RecordCodecBuilder.Mu<O>, T1> t1, App<RecordCodecBuilder.Mu<O>, T2> t2) { throw new AssertionError(); }\n\
           }\n\
         }",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         import com.mojang.serialization.Codec;\n\
         public final class Identifier { public static final Codec<Identifier> CODEC = null; }",
    ),
    (
        "net/minecraft/world/item/ItemInstance.java",
        "package net.minecraft.world.item; public interface ItemInstance {}",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item;\n\
         public final class ItemStack implements ItemInstance { public static final ItemStack EMPTY = null; }",
    ),
    (
        "net/minecraft/world/item/ItemStackTemplate.java",
        "package net.minecraft.world.item;\n\
         import com.mojang.serialization.Codec;\n\
         public final class ItemStackTemplate implements ItemInstance {\n\
           public static final Codec<ItemStackTemplate> CODEC = null;\n\
         }",
    ),
    (
        "net/minecraft/world/item/component/SimpleMutableContainer.java",
        "package net.minecraft.world.item.component;\n\
         import java.util.List; import net.minecraft.world.item.ItemStack;\n\
         public abstract class SimpleMutableContainer<T> {\n\
           protected List<ItemStack> items;\n\
           public SimpleMutableContainer(List<ItemStack> items) { this.items = items; }\n\
           public int size() { throw new AssertionError(); }\n\
           public abstract T toImmutable();\n\
         }",
    ),
    (
        "net/minecraft/world/item/component/GrowableMutableContainer.java",
        "package net.minecraft.world.item.component;\n\
         import java.util.List; import net.minecraft.world.item.ItemStack;\n\
         public abstract class GrowableMutableContainer<T> extends SimpleMutableContainer<T> {\n\
           public GrowableMutableContainer(List<ItemStack> items) { super(items); }\n\
           public abstract boolean canInsertNewSlots();\n\
         }",
    ),
    (
        "net/minecraft/advancements/predicates/ItemPredicate.java",
        "package net.minecraft.advancements.predicates;\n\
         import com.mojang.serialization.Codec; import java.util.function.Predicate;\n\
         import net.minecraft.world.item.ItemInstance;\n\
         public final class ItemPredicate implements Predicate<ItemInstance> {\n\
           public static final Codec<ItemPredicate> CODEC = null;\n\
           @Override public boolean test(ItemInstance item) { return false; }\n\
         }",
    ),
    (
        "net/minecraft/advancements/predicates/CollectionPredicate.java",
        "package net.minecraft.advancements.predicates;\n\
         import com.mojang.serialization.Codec; import java.util.function.Predicate;\n\
         public final class CollectionPredicate<T, P extends Predicate<T>> {\n\
           public static <T, P extends Predicate<T>> Codec<CollectionPredicate<T, P>> codec(Codec<P> elementCodec) { throw new AssertionError(); }\n\
         }",
    ),
    (
        "net/minecraft/network/codec/StreamCodec.java",
        "package net.minecraft.network.codec;\n\
         import java.util.function.*;\n\
         public interface StreamCodec<B, V> {\n\
           void encode(B buf, V value);\n\
           V decode(B buf);\n\
           @FunctionalInterface interface CodecOperation<B, S, T> { StreamCodec<B, T> apply(StreamCodec<B, S> codec); }\n\
           default <O> StreamCodec<B, O> apply(CodecOperation<B, V, O> op) { throw new AssertionError(); }\n\
           default <O> StreamCodec<B, O> map(Function<? super V, ? extends O> to, Function<? super O, ? extends V> from) { throw new AssertionError(); }\n\
           static <B, C, T1, T2> StreamCodec<B, C> composite(StreamCodec<? super B, T1> c1, Function<C, T1> g1, StreamCodec<? super B, T2> c2, Function<C, T2> g2, BiFunction<T1, T2, C> ctor) { throw new AssertionError(); }\n\
         }",
    ),
    (
        "net/minecraft/network/codec/ByteBufCodecs.java",
        "package net.minecraft.network.codec;\n\
         import java.util.*; import java.util.function.IntFunction;\n\
         public interface ByteBufCodecs {\n\
           StreamCodec<Object, Integer> VAR_INT = null;\n\
           StreamCodec<Object, String> STRING_UTF8 = null;\n\
           static <B, V> StreamCodec.CodecOperation<B, V, List<V>> list() { throw new AssertionError(); }\n\
           static <B, V> StreamCodec.CodecOperation<B, V, List<V>> list(int maxSize) { throw new AssertionError(); }\n\
           static <B, V> StreamCodec.CodecOperation<B, V, List<V>> fixedSizeList(int size) { throw new AssertionError(); }\n\
           static <B, V, C extends Collection<V>> StreamCodec.CodecOperation<B, V, C> collection(IntFunction<C> ctor, int maxSize) { throw new AssertionError(); }\n\
           static <B, V> StreamCodec<B, Optional<V>> optional(StreamCodec<? super B, V> codec) { throw new AssertionError(); }\n\
         }",
    ),
    (
        "net/minecraft/util/ARGB.java",
        "package net.minecraft.util;\n\
         public class ARGB {\n\
           public static int alpha(int c) { throw new AssertionError(); }\n\
           public static int red(int c) { throw new AssertionError(); }\n\
           public static int green(int c) { throw new AssertionError(); }\n\
           public static int blue(int c) { throw new AssertionError(); }\n\
           public static int color(int a, int r, int g, int b) { throw new AssertionError(); }\n\
           public static int color(int r, int g, int b) { throw new AssertionError(); }\n\
           public static int colorFromFloat(float a, float r, float g, float b) { throw new AssertionError(); }\n\
           public static int as8BitChannel(float v) { throw new AssertionError(); }\n\
           public static int gray(float v) { throw new AssertionError(); }\n\
           public static int multiply(int a, int b) { throw new AssertionError(); }\n\
           public static int average(int a, int b) { throw new AssertionError(); }\n\
           public static float alphaFloat(int c) { throw new AssertionError(); }\n\
           public static float redFloat(int c) { throw new AssertionError(); }\n\
           public static float greenFloat(int c) { throw new AssertionError(); }\n\
           public static float blueFloat(int c) { throw new AssertionError(); }\n\
         }",
    ),
    (
        "net/minecraft/util/Mth.java",
        "package net.minecraft.util;\n\
         import org.apache.commons.lang3.math.Fraction;\n\
         public class Mth {\n\
           public static int mulAndTruncate(Fraction f, int factor) { throw new AssertionError(); }\n\
         }",
    ),
    (
        "org/apache/commons/lang3/math/Fraction.java",
        "package org.apache.commons.lang3.math;\n\
         public final class Fraction extends Number implements Comparable<Fraction> {\n\
           public static final Fraction ZERO = null;\n\
           public static final Fraction ONE = null;\n\
           public static final Fraction ONE_HALF = null;\n\
           public static final Fraction ONE_THIRD = null;\n\
           public static final Fraction TWO_THIRDS = null;\n\
           public static final Fraction ONE_QUARTER = null;\n\
           public static final Fraction TWO_QUARTERS = null;\n\
           public static final Fraction THREE_QUARTERS = null;\n\
           public static final Fraction ONE_FIFTH = null;\n\
           public static final Fraction TWO_FIFTHS = null;\n\
           public static final Fraction THREE_FIFTHS = null;\n\
           public static final Fraction FOUR_FIFTHS = null;\n\
           public static Fraction getFraction(int n, int d) { throw new AssertionError(); }\n\
           public static Fraction getFraction(int w, int n, int d) { throw new AssertionError(); }\n\
           public static Fraction getFraction(double v) { throw new AssertionError(); }\n\
           public static Fraction getFraction(String s) { throw new AssertionError(); }\n\
           public static Fraction getReducedFraction(int n, int d) { throw new AssertionError(); }\n\
           public int getNumerator() { throw new AssertionError(); }\n\
           public int getDenominator() { throw new AssertionError(); }\n\
           public int getProperWhole() { throw new AssertionError(); }\n\
           public int getProperNumerator() { throw new AssertionError(); }\n\
           public Fraction add(Fraction f) { throw new AssertionError(); }\n\
           public Fraction subtract(Fraction f) { throw new AssertionError(); }\n\
           public Fraction multiplyBy(Fraction f) { throw new AssertionError(); }\n\
           public Fraction divideBy(Fraction f) { throw new AssertionError(); }\n\
           public Fraction negate() { throw new AssertionError(); }\n\
           public Fraction abs() { throw new AssertionError(); }\n\
           public Fraction invert() { throw new AssertionError(); }\n\
           public Fraction reduce() { throw new AssertionError(); }\n\
           public Fraction pow(int p) { throw new AssertionError(); }\n\
           public String toProperString() { throw new AssertionError(); }\n\
           @Override public int intValue() { throw new AssertionError(); }\n\
           @Override public long longValue() { throw new AssertionError(); }\n\
           @Override public float floatValue() { throw new AssertionError(); }\n\
           @Override public double doubleValue() { throw new AssertionError(); }\n\
           @Override public int compareTo(Fraction other) { throw new AssertionError(); }\n\
         }",
    ),
];

fn fixture() -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/BundleApiProbe.java", PROBE));
    javac_fixture::Fixture::build("bundle_api", &sources)
}

fn probe_text(f: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            host,
        )
        .unwrap_or_else(|e| panic!("BundleApiProbe.{method} failed: {e:?}"))
    {
        Value::Ref(Some(s)) => s,
        other => panic!("BundleApiProbe.{method} returned {other:?}"),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

#[test]
fn fraction_matches_java25_oracle() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());
    let actual = probe_text(&f, &mut host, "fractionRows");
    assert_eq!(actual, ORACLE_FRACTION);
    assert!(
        host.unknown_host_calls().is_empty(),
        "unexpected unknowns: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn argb_matches_java25_oracle() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());
    let actual = probe_text(&f, &mut host, "argbRows");
    assert_eq!(actual, ORACLE_ARGB);
    assert!(
        host.unknown_host_calls().is_empty(),
        "unexpected unknowns: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn record_codec_round_trips_match_java25_oracle() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());

    let pred_ok = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "predicateCodecNonNull",
            "()Z",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(pred_ok, Value::Int(1));

    let codec = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "bundleCodec",
            "()Lcom/mojang/serialization/Codec;",
            None,
            &[],
            &mut host,
        )
        .unwrap();

    let inputs: &[(&str, &str)] = &[
        ("01_empty_default", r#"{"content":[]}"#),
        (
            "02_shorthand_mult4",
            r#"{"content":["minecraft:apple"],"size_multiplier":4}"#,
        ),
        (
            "03_explicit_default_mult",
            r#"{"content":[{"id":"minecraft:diamond","count":16},{"id":"minecraft:iron_sword","components":{"minecraft:damage":5,"minecraft:custom_name":"Blade"}}],"size_multiplier":1}"#,
        ),
        ("04_missing_content", r#"{"size_multiplier":2}"#),
        (
            "05_bad_count_zero",
            r#"{"content":[{"id":"minecraft:apple","count":0}]}"#,
        ),
        (
            "06_bad_count_100",
            r#"{"content":[{"id":"minecraft:apple","count":100}]}"#,
        ),
        ("07_air_item", r#"{"content":["minecraft:air"]}"#),
        (
            "08_bad_mult_type",
            r#"{"content":[],"size_multiplier":"four"}"#,
        ),
    ];

    let mut expected_rows = std::collections::BTreeMap::new();
    for line in ORACLE_RECORD_CODEC.lines() {
        if let Some((k, v)) = line.split_once('=') {
            expected_rows.insert(k, v);
        }
    }
    assert_eq!(expected_rows.get("predicate.codec_nonnull"), Some(&"true"));

    for (name, raw) in inputs {
        let key = format!("case.{name}");
        let expected = expected_rows
            .get(key.as_str())
            .unwrap_or_else(|| panic!("missing oracle row {key}"));
        let json: serde_json::Value = serde_json::from_str(raw).unwrap();
        let decoded = f
            .jar
            .decode_dfu_codec_json(&codec, &json, &mut host)
            .unwrap();
        if *expected == "ERROR" {
            assert!(
                matches!(decoded, DfuCodecResult::Error(_)),
                "case {name}: expected Error, got {decoded:?}"
            );
            continue;
        }
        let DfuCodecResult::Ok(comp) = decoded else {
            panic!("case {name}: expected Ok, got {decoded:?}");
        };
        let desc = match f
            .jar
            .invoke_method_with_args(
                "demo/BundleApiProbe",
                "describeBundleComponent",
                "(Ldemo/BundleApiProbe$BundleLikeComponent;)Ljava/lang/String;",
                None,
                std::slice::from_ref(&comp),
                &mut host,
            )
            .unwrap()
        {
            Value::Ref(Some(s)) => s,
            other => panic!("unexpected description: {other:?}"),
        };
        let encoded = match f
            .jar
            .encode_dfu_codec_json(&codec, &comp, &mut host)
            .unwrap()
        {
            DfuCodecResult::Ok(j) => j,
            other => panic!("case {name}: encode failed: {other:?}"),
        };
        let (exp_prefix, exp_json_str) = expected.split_once("|json=").unwrap();
        let exp_json: serde_json::Value = serde_json::from_str(exp_json_str).unwrap();
        assert_eq!(format!("OK|{desc}"), exp_prefix, "case {name} header");
        assert_eq!(encoded, exp_json, "case {name} encoded JSON");
    }

    assert!(
        host.unknown_host_calls().is_empty(),
        "unexpected unknowns: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn stream_codec_list_round_trips_match_java25_oracle() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());

    let mut actual = probe_text(&f, &mut host, "streamClassRows");

    let run_rt = |host: &mut StdHost,
                  label: &str,
                  codec_method: &str,
                  val_method: &str,
                  val_desc: &str|
     -> String {
        let codec = f
            .jar
            .invoke_method_with_args(
                "demo/BundleApiProbe",
                codec_method,
                "()Lnet/minecraft/network/codec/StreamCodec;",
                None,
                &[],
                host,
            )
            .unwrap();
        let value = f
            .jar
            .invoke_method_with_args("demo/BundleApiProbe", val_method, val_desc, None, &[], host)
            .unwrap();
        let bytes = f
            .jar
            .encode_stream_codec_bytes(&codec, &value, host)
            .unwrap()
            .unwrap_or_else(|| panic!("{label}: encode returned None"));
        let (decoded, consumed) = f
            .jar
            .decode_stream_codec_bytes(&codec, &bytes, host)
            .unwrap()
            .unwrap_or_else(|| panic!("{label}: decode returned None"));
        let eq_and_val = match f
            .jar
            .invoke_method_with_args(
                "demo/BundleApiProbe",
                "formatEqualityAndValue",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/String;",
                None,
                &[value, decoded],
                host,
            )
            .unwrap()
        {
            Value::Ref(Some(s)) => s,
            other => panic!("{label}: unexpected format result: {other:?}"),
        };
        let remaining = bytes.len() - consumed;
        format!(
            "rt.{label}=hex={}|remaining={remaining}|{eq_and_val}\n",
            hex_encode(&bytes)
        )
    };

    actual.push_str(&run_rt(
        &mut host,
        "int_empty",
        "intListCodec",
        "emptyIntList",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "int_vals",
        "intListCodec",
        "sampleIntList",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "str_empty",
        "strList3Codec",
        "emptyStrList",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "str_vals",
        "strList3Codec",
        "sampleStrList",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "wire_content",
        "wireContentCodec",
        "sampleWireContent",
        "()Ldemo/BundleApiProbe$WireContent;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "fixed2",
        "fixed2Codec",
        "fixed2Values",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "opt_list",
        "optListCodec",
        "sampleOptList",
        "()Ljava/util/List;",
    ));
    actual.push_str(&run_rt(
        &mut host,
        "linked",
        "linkedListCodec",
        "sampleLinkedList",
        "()Ljava/util/LinkedList;",
    ));

    let str_list3 = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "strList3Codec",
            "()Lnet/minecraft/network/codec/StreamCodec;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let str_list_unbounded = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "strListUnboundedCodec",
            "()Lnet/minecraft/network/codec/StreamCodec;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let too_big = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "tooBigStrList",
            "()Ljava/util/List;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    fn fmt_ex<T: std::fmt::Debug>(res: Result<T, NativeError>) -> String {
        match res {
            Err(NativeError::UncaughtException { class, message, .. }) => format!(
                "EX:{}:{}",
                class.replace('/', "."),
                message.unwrap_or_else(|| "null".into())
            ),
            other => panic!("expected UncaughtException, got {other:?}"),
        }
    }
    actual.push_str(&format!(
        "ex.encode_max={}\n",
        fmt_ex(
            f.jar
                .encode_stream_codec_bytes(&str_list3, &too_big, &mut host)
        )
    ));
    let four_bytes = f
        .jar
        .encode_stream_codec_bytes(&str_list_unbounded, &too_big, &mut host)
        .unwrap()
        .unwrap();
    actual.push_str(&format!(
        "ex.decode_max={}\n",
        fmt_ex(
            f.jar
                .decode_stream_codec_bytes(&str_list3, &four_bytes, &mut host)
        )
    ));
    let fixed2 = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "fixed2Codec",
            "()Lnet/minecraft/network/codec/StreamCodec;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let short_fixed = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "fixed2ShortValues",
            "()Ljava/util/List;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    actual.push_str(&format!(
        "ex.fixed_wrong_size={}\n",
        fmt_ex(
            f.jar
                .encode_stream_codec_bytes(&fixed2, &short_fixed, &mut host)
        )
    ));

    assert_eq!(actual, ORACLE_STREAM_CODEC);
    assert!(
        host.unknown_host_calls().is_empty(),
        "unexpected unknowns: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn growable_mutable_container_inherits_items_and_counts_final_write() {
    let f = fixture();
    let mut host = f.jar.seed_host(StdHost::new());
    assert_eq!(
        probe_text(&f, &mut host, "testGrowableContainer"),
        "same=true|size=2"
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "unexpected unknowns: {:?}",
        host.unknown_host_calls()
    );

    let mut host_write = f.jar.seed_host(StdHost::new());
    f.jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "overwriteFinalItems",
            "()V",
            None,
            &[],
            &mut host_write,
        )
        .unwrap();
    assert!(
        host_write.unknown_host_calls().iter().any(|(call, _)| {
            call == "net/minecraft/world/item/component/SimpleMutableContainer#itemsLjava/util/List;"
        }),
        "expected putfield on final SimpleMutableContainer#items to be counted, got {:?}",
        host_write.unknown_host_calls()
    );
}

#[test]
fn counted_placeholder_traps_stay_counted() {
    let f = fixture();

    // 1. DFU xmap callback invoking an unmodelled host call (`System.gc`) must return Unmodelled
    //    and leave the inner call counted, never fabricating a decoded or encoded result.
    let mut host = f.jar.seed_host(StdHost::new());
    let counted_codec = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "countedXmapCodec",
            "()Lcom/mojang/serialization/Codec;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    let decoded = f
        .jar
        .decode_dfu_codec_json(&counted_codec, &serde_json::json!(41), &mut host)
        .unwrap();
    assert!(
        matches!(decoded, DfuCodecResult::Unmodelled(_)),
        "expected Unmodelled for DFU callback with unknown call, got {decoded:?}"
    );
    assert!(host.unknown_host_call_count() > 0);

    // 2. StreamCodec.map callback invoking an unmodelled host call must return None and keep count.
    let mut host_sc = f.jar.seed_host(StdHost::new());
    let counted_sc = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "countedStreamCodec",
            "()Lnet/minecraft/network/codec/StreamCodec;",
            None,
            &[],
            &mut host_sc,
        )
        .unwrap();
    let encoded = f
        .jar
        .encode_stream_codec_bytes(&counted_sc, &Value::Int(7), &mut host_sc)
        .unwrap();
    assert_eq!(encoded, None);
    assert!(host_sc.unknown_host_call_count() > 0);

    // 3. CollectionPredicate.codec(opaqueCodec) must stay counted (never upgrade opaque -> exact).
    let mut host_op = f.jar.seed_host(StdHost::new());
    let _ = f
        .jar
        .invoke_method_with_args(
            "demo/BundleApiProbe",
            "collectionPredicateOfOpaque",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host_op,
        )
        .unwrap();
    assert!(
        host_op
            .unknown_host_calls()
            .iter()
            .any(|(call, _)| { call.contains("CollectionPredicate#codec") }),
        "expected opaque CollectionPredicate.codec to stay counted, got {:?}",
        host_op.unknown_host_calls()
    );

    // 4. CollectionPredicate.codec(null) throws NullPointerException.
    let mut host_null = f.jar.seed_host(StdHost::new());
    let res = probe_text(&f, &mut host_null, "collectionPredicateOfNull");
    assert!(
        res.starts_with("EX:java.lang.NullPointerException:"),
        "expected NPE for CollectionPredicate.codec(null), got {res}"
    );
    assert!(host_null.unknown_host_calls().is_empty());
}

#[test]
fn runes_jar_has_zero_bundle_api_unknowns() {
    let Ok(jar_path) = std::env::var("RUNES_JAR") else {
        eprintln!("skipping runes_jar_has_zero_bundle_api_unknowns: RUNES_JAR not set");
        return;
    };
    // Unoptimised test builds use ~150 KiB per jar frame, which exceeds Rust's default 2 MiB
    // test worker stack when Runes initialises nested Bundle API classes (see `frame_bound.rs`).
    // Match the CLI's stack headroom on a dedicated thread.
    std::thread::Builder::new()
        .name("runes-bundle-api".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let jar = NativeJar::from_path(Path::new(&jar_path)).expect("load RUNES_JAR");
            let mut host = jar.seed_host(StdHost::new().with_game_version("26.3"));

            let entrypoints = jar.discover_entrypoints("main");
            assert!(
                !entrypoints.is_empty(),
                "expected main entrypoints in RUNES_JAR"
            );
            for ep in &entrypoints {
                jar.execute(&ep.class, ep.method.as_deref(), "main", &mut host)
                    .unwrap_or_else(|e| panic!("entrypoint {} failed: {e:?}", ep.class));
            }

            let forbidden_bundle_api_unknowns = [
                "CustomBundleContentsComponent$Builder#items",
                "ItemPredicate#CODEC",
                "CollectionPredicate#codec",
                "Codec#optionalFieldOf",
                "MapCodec#forGetter",
                "RecordCodecBuilder$Instance#group",
                "Products$P1#apply",
                "RecordCodecBuilder#create",
                "ByteBufCodecs#list",
                "StreamCodec#apply",
                "ARGB#colorFromFloat",
                "Fraction#ZERO",
                "Fraction#getFraction",
            ];
            for (call, count) in host.unknown_host_calls() {
                for forbidden in &forbidden_bundle_api_unknowns {
                    assert!(
                        !call.contains(forbidden),
                        "Bundle API unknown call still present: {call} (x{count})"
                    );
                }
            }
            for (call, count) in host.unknown_host_calls() {
                assert_eq!(
                    call, "Block#getStateForPlacement (placement hook)",
                    "unexpected non-placement unknown call: {call} (x{count})"
                );
            }
            assert!(
                host.unknown_host_call_count() <= 1,
                "expected at most the 1 placement-hook unknown, got {}: {:?}",
                host.unknown_host_call_count(),
                host.unknown_host_calls()
            );

            // Confirm Runes actually registered its Bundle-API-backed pouches.
            let registered_items: Vec<&str> = host
                .registrations()
                .iter()
                .filter(|r| r.registry == "minecraft:item")
                .map(|r| r.id.as_str())
                .collect();
            for pouch in [
                "runes:small_rune_pouch",
                "runes:medium_rune_pouch",
                "runes:large_rune_pouch",
            ] {
                assert!(
                    registered_items.contains(&pouch),
                    "expected {pouch} to be registered, got {registered_items:?}"
                );
            }
        })
        .expect("spawn runes-bundle-api thread")
        .join()
        .unwrap();
}

use horizon_native_java::StdHost;
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/stream_codec_apply_jar_op/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/stream_codec_apply_jar_op/java25.txt");
/// The oracle row printed by Probe.main as `<label>=<value>`.
fn row(label: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|l| l.strip_prefix(label)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle row {label}"))
}
const STREAM_CODEC_STUB: &str = r#"package net.minecraft.network.codec; public interface StreamCodec<B,V>{ void encode(B b,V v); V decode(B b); static <B,V> StreamCodec<B,V> of(StreamEncoder<B,V> e,StreamDecoder<B,V> d){throw new AssertionError();} static <B,V> StreamCodec<B,V> ofMember(StreamMemberEncoder<B,V> e,StreamDecoder<B,V> d){throw new AssertionError();} static <B,V> StreamCodec<B,V> unit(V v){throw new AssertionError();} static <B,C,T1,T2> StreamCodec<B,C> composite(StreamCodec<? super B,T1> a,java.util.function.Function<C,T1> f,StreamCodec<? super B,T2> b,java.util.function.Function<C,T2> g,java.util.function.BiFunction<T1,T2,C> h){throw new AssertionError();} default <O> StreamCodec<B,O> apply(CodecOperation<B,V,O> o){throw new AssertionError();} default <O> StreamCodec<B,O> map(java.util.function.Function<? super V,? extends O> a,java.util.function.Function<? super O,? extends V> b){throw new AssertionError();} default <S extends B> StreamCodec<S,V> cast(){throw new AssertionError();} interface CodecOperation<B,S,T>{ StreamCodec<B,T> apply(StreamCodec<B,S> c); } }"#;
const STREAM_ENCODER_STUB: &str = r#"package net.minecraft.network.codec; public interface StreamEncoder<B,T>{ void encode(B b,T t); }"#;
const STREAM_DECODER_STUB: &str =
    r#"package net.minecraft.network.codec; public interface StreamDecoder<B,T>{ T decode(B b); }"#;
const STREAM_MEMBER_ENCODER_STUB: &str = r#"package net.minecraft.network.codec; public interface StreamMemberEncoder<B,T>{ void encode(T t,B b); }"#;
const BYTE_BUF_CODECS_STUB: &str = r#"package net.minecraft.network.codec; public final class ByteBufCodecs { public static final StreamCodec<Object,String> STRING_UTF8=null; public static <B,V> StreamCodec<B,java.util.Optional<V>> optional(StreamCodec<? super B,V> c){throw new AssertionError();} public static <B,V> StreamCodec.CodecOperation<B,V,java.util.List<V>> list(){throw new AssertionError();} public static <B,V,C extends java.util.Collection<V>> StreamCodec<B,C> collection(java.util.function.IntFunction<C> f,StreamCodec<? super B,V> c){throw new AssertionError();} public static <B,V> StreamCodec<B,V> unmodelled(StreamCodec<? super B,V> c){throw new AssertionError();} }"#;
const COUNTED: &str = include_str!("fixtures/stream_codec_apply_jar_op/demo/Counted.java");
fn fixture() -> Fixture {
    Fixture::build(
        "stream-codec-apply-jar-op",
        &[
            (
                "net/minecraft/network/codec/StreamCodec.java",
                STREAM_CODEC_STUB,
            ),
            (
                "net/minecraft/network/codec/StreamEncoder.java",
                STREAM_ENCODER_STUB,
            ),
            (
                "net/minecraft/network/codec/StreamDecoder.java",
                STREAM_DECODER_STUB,
            ),
            (
                "net/minecraft/network/codec/StreamMemberEncoder.java",
                STREAM_MEMBER_ENCODER_STUB,
            ),
            (
                "net/minecraft/network/codec/ByteBufCodecs.java",
                BYTE_BUF_CODECS_STUB,
            ),
            ("demo/Probe.java", PROBE),
            ("demo/Counted.java", COUNTED),
        ],
    )
}
/// Run `method` of `demo/Counted` and return its text and the host's counted calls.
fn counted(method: &str) -> (String, Vec<(String, usize)>) {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    let text = match f
        .jar
        .invoke_method_with_args(
            "demo/Counted",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut h,
        )
        .unwrap_or_else(|error| panic!("{method} failed: {error}"))
    {
        horizon_native_java::Value::Ref(Some(text)) => text,
        other => panic!("{method} did not return a string: {other:?}"),
    };
    (text, h.unknown_host_calls())
}
const APPLY: &str = "net/minecraft/network/codec/StreamCodec#apply(Lnet/minecraft/network/codec/StreamCodec$CodecOperation;)Lnet/minecraft/network/codec/StreamCodec;";
/// The Probe method `method` matches the oracle row `label` and nothing is counted.
fn exact(method: &str, label: &str) {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    let text = f.text_on(&mut h, method);
    let unknown = h.unknown_host_calls();
    assert_eq!(
        unknown,
        Vec::<(String, usize)>::new(),
        "{label}: counted (answer {text:?})"
    );
    assert_eq!(text, row(label), "{label}");
}
#[test]
fn rows_match_java() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "identity"), row("identity"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn result_unchanged_matches_java() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "resultUnchanged"), row("resultUnchanged"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn result_null_matches_java() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "resultNull"), row("resultNull"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn thrown_exception_propagates() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "throws_"), row("throws"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn null_operation_throws_npe() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "nullOp"), row("nullOp"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn geckolib_shape_apply_is_not_counted() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "geckolibShape"), row("geckolibShape"));
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn capture_int_matches_java() {
    exact("captureInt", "captureInt");
}
#[test]
fn capture_self_matches_java() {
    exact("captureSelf", "captureSelf");
}
#[test]
fn bound_method_ref_matches_java() {
    exact("boundRef", "boundRef");
}
#[test]
fn constructor_ref_matches_java() {
    exact("constructorRef", "constructorRef");
}
#[test]
fn static_ref_to_modelled_factory_matches_java() {
    exact("staticRefUnit", "staticRefUnit");
    exact("staticRefOptional", "staticRefOptional");
}
#[test]
fn exception_is_catchable_and_runs_finally() {
    exact("caughtAndFinally", "caughtAndFinally");
}
#[test]
fn nested_apply_matches_java() {
    exact("nested", "nested");
    exact("nestedModelled", "nestedModelled");
    exact("chained", "chained");
}
#[test]
fn every_modelled_receiver_kind_matches_java() {
    exact("kinds", "kinds");
}
#[test]
fn null_operation_on_every_receiver_kind_matches_java() {
    exact("nullKinds", "nullKinds");
}
#[test]
fn inner_counted_call_keeps_apply_counted() {
    let (text, unknown) = counted("innerCounted");
    assert_eq!(text, "calls=1");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}
#[test]
fn inner_counted_call_then_throw_keeps_apply_counted() {
    let (text, unknown) = counted("innerCountedThenThrows");
    assert_eq!(text, "swallowed,calls=1");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}
#[test]
fn operation_object_stays_counted() {
    let (text, unknown) = counted("operationObject");
    assert_eq!(text, "calls=0");
    assert_eq!(unknown, vec![(APPLY.to_owned(), 1)]);
}
#[test]
fn unmodelled_static_ref_stays_counted() {
    let (text, unknown) = counted("unmodelledStaticRef");
    assert_eq!(text, "done");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}
#[test]
fn unbound_game_method_ref_stays_counted() {
    // `StreamCodec::cast` as a callback is not answered by the runtime (the same
    // call made directly is), so the operation counts `cast` and `apply` stays counted.
    let (text, unknown) = counted("unboundGameRef");
    assert_eq!(text, "done");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}
#[test]
fn self_recursive_operation_stays_counted() {
    // Java ends in a StackOverflowError; the runtime's bounded walk counts it instead
    // of answering.
    let (text, unknown) = counted("recursion");
    assert_eq!(text, "no throw");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}
#[test]
fn jar_codec_receiver_stays_counted() {
    let (_, unknown) = counted("jarCodecReceiver");
    assert!(unknown.iter().any(|(name, _)| name == APPLY), "{unknown:?}");
}

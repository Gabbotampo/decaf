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
const STREAM_CODEC_STUB: &str = r#"package net.minecraft.network.codec; public interface StreamCodec<B,V>{ void encode(B b,V v); V decode(B b); static <B,V> StreamCodec<B,V> unit(V v){throw new AssertionError();} default <O> StreamCodec<B,O> apply(CodecOperation<B,V,O> o){throw new AssertionError();} default <O> StreamCodec<B,O> map(java.util.function.Function<? super V,? extends O> a,java.util.function.Function<? super O,? extends V> b){throw new AssertionError();} interface CodecOperation<B,S,T>{ StreamCodec<B,T> apply(StreamCodec<B,S> c); } }"#;
const BYTE_BUF_CODECS_STUB: &str = r#"package net.minecraft.network.codec; public final class ByteBufCodecs { public static final StreamCodec<Object,String> STRING_UTF8=null; public static <V> StreamCodec<Object,java.util.Optional<V>> optional(StreamCodec<? super Object,V> c){throw new AssertionError();} }"#;
fn fixture() -> Fixture {
    Fixture::build(
        "stream-codec-apply-jar-op",
        &[
            (
                "net/minecraft/network/codec/StreamCodec.java",
                STREAM_CODEC_STUB,
            ),
            (
                "net/minecraft/network/codec/ByteBufCodecs.java",
                BYTE_BUF_CODECS_STUB,
            ),
            ("demo/Probe.java", PROBE),
        ],
    )
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

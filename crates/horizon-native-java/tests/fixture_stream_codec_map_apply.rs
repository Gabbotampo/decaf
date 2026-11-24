use horizon_native_java::StdHost;
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use javac_fixture::Fixture;
const PROBE: &str = include_str!("fixtures/stream_codec_map_apply/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/stream_codec_map_apply/java25.txt");
const STUB: &str = r#"package net.minecraft.network.codec; public interface StreamCodec<B,V>{ void encode(B b,V v); V decode(B b); interface CodecOperation<B,S,T>{ StreamCodec<B,T> apply(StreamCodec<B,S> c); } static <B,V> StreamCodec<B,V> unit(V v){throw new AssertionError();} default <O> StreamCodec<B,O> apply(CodecOperation<B,V,O> o){throw new AssertionError();} default <O> StreamCodec<B,O> map(java.util.function.Function<? super V,? extends O> a,java.util.function.Function<? super O,? extends V> b){throw new AssertionError();}}"#;
fn fixture() -> Fixture {
    Fixture::build(
        "stream-codec-map-apply",
        &[
            ("net/minecraft/network/codec/StreamCodec.java", STUB),
            (
                "net/minecraft/network/codec/ByteBufCodecs.java",
                "package net.minecraft.network.codec; public final class ByteBufCodecs { public static StreamCodec<Object,Integer> VAR_INT; }",
            ),
            ("demo/Probe.java", PROBE),
        ],
    )
}
#[test]
fn rows_match_java() {
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "rows"), ORACLE);
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}
#[test]
fn encode_not_counted() {
    // `apply` with a jar-supplied operation now runs `operation.apply(this)`
    // through the interpreter. The lambda body calls `StreamCodec.map`, which
    // is modelled, so nothing inside is counted and `apply` itself answers
    // exactly. The oracle (Java 25) confirms the operation runs once with the
    // receiver and returns its result unchanged; nothing is counted.
    let f = fixture();
    let mut h = f.jar.seed_host(StdHost::new());
    f.text_on(&mut h, "counted");
    assert_eq!(h.unknown_host_calls(), Vec::<(String, usize)>::new());
}

use horizon_native_java::StdHost;
#[path = "support/javac_fixture.rs"] mod javac_fixture;
use javac_fixture::Fixture;
const PROBE:&str=include_str!("fixtures/stream_codec_map_apply/demo/Probe.java");
const ORACLE:&str=include_str!("fixtures/stream_codec_map_apply/java25.txt");
const STUB:&str=r#"package net.minecraft.network.codec; public interface StreamCodec<B,V>{ void encode(B b,V v); V decode(B b); interface CodecOperation<B,V,O>{ StreamCodec<B,O> apply(StreamCodec<B,V> c); } default <O> StreamCodec<B,O> apply(CodecOperation<B,V,O> o){throw new AssertionError();} default <O> StreamCodec<B,O> map(java.util.function.Function<? super V,? extends O> a,java.util.function.Function<? super O,? extends V> b){throw new AssertionError();}}"#;
fn fixture()->Fixture{Fixture::build("stream-codec-map-apply",&[("net/minecraft/network/codec/StreamCodec.java",STUB),("demo/Probe.java",PROBE)])}
#[test] fn rows_match_java(){let f=fixture();let mut h=f.jar.seed_host(StdHost::new());assert_eq!(f.text_on(&mut h,"rows"),ORACLE);assert_eq!(h.unknown_host_calls(),vec![("net/minecraft/network/codec/StreamCodec#apply(Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;".into(),1)]);}
#[test] fn encode_counted(){let f=fixture();let mut h=f.jar.seed_host(StdHost::new());f.text_on(&mut h,"counted");assert_eq!(h.unknown_host_calls(),vec![("net/minecraft/network/codec/StreamCodec#apply(Lnet/minecraft/network/codec/StreamCodec;)Lnet/minecraft/network/codec/StreamCodec;".into(),1),("net/minecraft/network/codec/StreamCodec#encode(Ljava/lang/Object;Ljava/lang/Object;)V".into(),1)]);}

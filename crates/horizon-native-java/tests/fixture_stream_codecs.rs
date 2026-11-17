use horizon_native_java::StdHost;
#[path = "support/javac_fixture.rs"] mod javac_fixture;
use javac_fixture::Fixture;
const PROBE:&str=include_str!("fixtures/stream_codecs/demo/Probe.java");
const ORACLE:&str=include_str!("fixtures/stream_codecs/java25.txt");
const STUB:&str=r#"package net.minecraft.network.codec; public interface StreamCodec<B,V>{ void encode(B b,V v); V decode(B b); }"#;
const CODECS:&str=r#"package net.minecraft.network.codec; import java.util.function.*; public final class ByteBufCodecs { public static final StreamCodec<Object,Integer> VAR_INT=null; public static final StreamCodec<Object,String> STRING_UTF8=null; public static <K,V> StreamCodec<Object,java.util.Map<K,V>> map(IntFunction<java.util.Map<K,V>> f,StreamCodec<Object,K> k,StreamCodec<Object,V> v){throw new AssertionError();} public static <E> StreamCodec<Object,java.util.Collection<E>> collection(IntFunction<java.util.Collection<E>> f,StreamCodec<Object,E> e){throw new AssertionError();} }"#;
#[test] fn stream_codecs_match_java25(){ let f=Fixture::build("stream-codecs",&[("net/minecraft/network/codec/StreamCodec.java",STUB),("net/minecraft/network/codec/ByteBufCodecs.java",CODECS),("demo/Probe.java",PROBE)]); let mut h=f.jar.seed_host(StdHost::new()); assert_eq!(f.text_on(&mut h,"rows"),ORACLE.trim_end()); assert_eq!(h.unknown_host_calls(), vec![("net/minecraft/network/codec/StreamCodec#encode(Ljava/lang/Object;Ljava/lang/Object;)V".into(),1)]); }

package com.mojang.serialization;

public interface Codec<A> {
    com.mojang.serialization.codecs.PrimitiveCodec<String> STRING = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Integer> INT = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Boolean> BOOL = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Byte> BYTE = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Short> SHORT = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Long> LONG = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Float> FLOAT = null;
    com.mojang.serialization.codecs.PrimitiveCodec<Double> DOUBLE = null;
    com.mojang.serialization.codecs.PrimitiveCodec<java.nio.ByteBuffer> BYTE_BUFFER = null;
    com.mojang.serialization.codecs.PrimitiveCodec<java.util.stream.IntStream> INT_STREAM = null;
    com.mojang.serialization.codecs.PrimitiveCodec<java.util.stream.LongStream> LONG_STREAM = null;
    Codec<Object> PASSTHROUGH = null;
    com.mojang.serialization.MapCodec<Object> EMPTY = null;
    <S> Codec<S> xmap(java.util.function.Function<? super A, ? extends S> to,
                      java.util.function.Function<? super S, ? extends A> from);
    com.mojang.serialization.MapCodec<A> fieldOf(String name);
    com.mojang.serialization.MapCodec<java.util.Optional<A>> optionalFieldOf(String name);
    com.mojang.serialization.MapCodec<A> optionalFieldOf(String name, A defaultValue);
    static <O> Codec<O> create(java.util.function.Function<com.mojang.serialization.codecs.RecordCodecBuilder.Instance<O>, com.mojang.datafixers.kinds.App<?, O>> f) { return null; }
}

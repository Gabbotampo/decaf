package com.mojang.serialization;

public interface MapCodec<A> {
    <O> com.mojang.serialization.codecs.RecordCodecBuilder<O, A> forGetter(java.util.function.Function<O, A> getter);
    <S> MapCodec<S> xmap(java.util.function.Function<? super A, ? extends S> to,
                         java.util.function.Function<? super S, ? extends A> from);
}

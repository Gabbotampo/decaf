package com.mojang.datafixers;

import com.mojang.datafixers.kinds.App;
import com.mojang.serialization.codecs.RecordCodecBuilder;

public final class Products {
    public static final class P1<O, A> implements App<RecordCodecBuilder.Mu<O>, A> {
        public <R> App<RecordCodecBuilder.Mu<O>, R> apply(RecordCodecBuilder.Instance<O> instance, java.util.function.Function<A, R> constructor) { return null; }
    }
}

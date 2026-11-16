package com.mojang.serialization.codecs;

import com.mojang.datafixers.kinds.App;
import com.mojang.datafixers.Products;
import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;

public final class RecordCodecBuilder<O, F> implements App<RecordCodecBuilder.Mu<O>, F> {
    public static final class Mu<O> {}
    public static final class Instance<O> {
        public <A> Products.P1<O, A> group(App<RecordCodecBuilder.Mu<O>, A> a) { return null; }
    }
    public static <O> Instance<O> instance() { return null; }
    public static <O> Codec<O> create(java.util.function.Function<Instance<O>, App<Mu<O>, O>> f) { return null; }
}

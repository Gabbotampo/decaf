package com.google.common.collect;
public abstract class ImmutableMap<K, V> implements java.util.Map<K, V> {
    public static <K, V> ImmutableMap<K, V> of() { return null; }
    public static <K, V> ImmutableMap<K, V> of(K k1, V v1) { return null; }
    public static <K, V> ImmutableMap<K, V> of(K k1, V v1, K k2, V v2) { return null; }
    public static <T, K, V> java.util.stream.Collector<T, ?, ImmutableMap<K, V>> toImmutableMap(
        java.util.function.Function<? super T, ? extends K> k, java.util.function.Function<? super T, ? extends V> v) { return null; }
}

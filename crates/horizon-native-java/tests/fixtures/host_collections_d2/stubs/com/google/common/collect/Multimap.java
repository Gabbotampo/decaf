package com.google.common.collect;
public interface Multimap<K, V> {
    boolean put(K key, V value);
    java.util.Set<K> keySet();
    java.util.Collection<V> values();
}

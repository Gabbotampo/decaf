# fastutil sorted entry set

`Object2BooleanLinkedOpenHashMap.object2BooleanEntrySet()` is an
`Object2BooleanSortedMap.FastSortedEntrySet`; this fixture is the oracle for its `iterator()` and
for the `ObjectBidirectionalIterator` it returns (`hasNext`, `next`, `remove`).

## What the runtime answers

Exactly the rows of `java25.txt` (`demo.Probe.rows()`, zero unknown host calls), under these
conditions, each of which the runtime checks and otherwise counts:

- the map is an `Object2BooleanLinkedOpenHashMap` itself (not a subclass, not the
  `Object2BooleanLinkedOpenCustomHashMap`, whose `Hash.Strategy` the host does not model) made by
  the no-arg constructor (`new Object2BooleanLinkedOpenHashMap<>()`, the only one whose Java map
  the host's table mirrors);
- every key in it is a `String`, and no call on the map ever carried a text holding U+FFFD: the
  host stores a lone surrogate as U+FFFD, so two Java keys can be one host key and a key put and
  removed again leaves no trace (a genuine U+FFFD is counted too). A map stops being exact for
  good at such a call, at a `put` of a null value (a `NullPointerException` in Java, stored as
  `false` by the host), at any call on it other than `put`/`remove`/`clear`/`get`/`getOrDefault`/
  `containsKey`/`containsValue`/`size`/`isEmpty`/`object2BooleanEntrySet`/`defaultReturnValue`/
  `hashCode`/`toString`/`getClass` and their primitive forms (`putAll`, `computeIfAbsent`, ...),
  and when it is handed to `Collections.*` or a fastutil static (`synchronizedMap`, `Object2BooleanMaps`);
- the entry set `iterator()` is called on is the map's view now: the host's entry set is a copy
  made when the program asked for it, so it must hold the map's keys in the map's order and every
  entry in it must still read (an entry reads only while the map is unchanged since it was
  made). A set kept across a put, a remove, a clear, an iterator's `remove()`, a put and remove
  that restore the keys, or a put of a present key is counted;
- the iterator's `hasNext`/`next`/`remove`, typed as `Iterator` or as
  `ObjectBidirectionalIterator`, are called while the map's key sequence is the one the iterator
  last saw (its own `remove` moves that sequence on).

Rows: `iterator()` order (including the empty-string key, a re-put of a present key and keys of
any length and code point that are no lone surrogate), an entry set kept while the map does not
change and a fresh entry set per use after a put, typed
use through `getKey()` and `getBooleanValue()`, independent iterators, entries kept after the
iteration, the bare `NoSuchElementException` past the end and on an empty map, the bare
`IllegalStateException` for a `remove` with nothing to remove, and `remove` in the middle, at the
end and for every entry.

## What stays counted

`java25_counted.txt` (`demo.Probe.countedRows()`), each case its own method so the test runs it
on a host of its own and checks it counts a call:

- `fastIterator()`: one entry object reused by every `next()` (`cherry,cherry,cherry`);
- a map changed under a live iterator (a put or remove), untyped or typed as the fastutil
  iterator: fastutil's iterator reads its map live and never throws
  `ConcurrentModificationException`;
- an entry read after the iterator's `remove()` or after a put of its key;
- `setValue` of an entry and `hasPrevious`/`previous` of the iterator (the host models neither);
- `toString()` of a fastutil entry: fastutil prints `key=>value`, `java.util` prints `key=value`;
- an entry set kept across a change of the map (a live view in Java): a put, a remove, a clear,
  an iterator's `remove()`, a put and remove that restore the keys, a put of a present key, a set
  made on an empty map;
- keys the host cannot tell apart (two lone surrogates, a lone surrogate and U+FFFD, a lone
  surrogate removed again, a genuine U+FFFD), a null value put through `Map.put`, and writes the
  runtime does not follow (`putAll`, `computeIfAbsent`, a `Collections.synchronizedMap` wrapper, a
  `map::put` method reference handed to library code);
- `Object2BooleanLinkedOpenCustomHashMap`; maps built by a constructor with arguments (copy
  constructors, arrays, capacity); keys that are not `String`s (a jar class with its own `equals`,
  boxes);
- the tree maps (`Object2BooleanAVLTreeMap`, `Object2BooleanRBTreeMap`): their entry set is an
  unrelated `ObjectSortedSet` in natural key order.

Known and not fixed here (the same at the base commit, outside this change): the entry set typed
as `Set`/`Collection`/`Iterable`/`ObjectSortedSet` goes through the host's generic path (its
iteration, `size`, `isEmpty`, `stream`, `contains` read the host's copy, not the live view); the
host's put through `Map.put(k, null)` stores `false` and `put`/`size`/`containsKey` merge lone
surrogate keys; a fastutil `BasicEntry` is not `equals` to a host entry.

Not run, not claimed: `fastIterator(entry)`, `forEachRemaining` on the iterator, the entry set's
`size`/`contains`/`remove`, `iterator()` of any other fastutil class.

An independent oracle (`notes/scratch-1122/fanout/slices/E3-fastutil-edges/probe`, raw JDK 25
output) agrees with the rows here for the order (empty-string key included), the second `remove()`
(`IllegalStateException`), `next()` past the end, the empty map, the one entry `fastIterator()`
reuses, and no `ConcurrentModificationException` for a put during iteration.

Both files are the raw output of the probe, never edited by hand.

```sh
export FASTUTIL_JAR="$HOME/.local/share/PrismLauncher/libraries/it/unimi/dsi/fastutil/8.5.18/fastutil-8.5.18.jar"
/usr/lib/jvm/java-25-openjdk/bin/javac --release 21 -cp "$FASTUTIL_JAR" -d out demo/Probe.java
/usr/lib/jvm/java-25-openjdk/bin/java -cp "$FASTUTIL_JAR:out" demo.Probe > java25.txt
/usr/lib/jvm/java-25-openjdk/bin/java -cp "$FASTUTIL_JAR:out" demo.Probe counted > java25_counted.txt
```

`java -version`: `openjdk version "25.0.5.1" 2026-11-17`, OpenJDK 64-Bit Server VM (build 25.0.5.1).
`fastutil-8.5.18.jar` SHA-256: `9094ae67d01d0ad246f886f11ad557fc2e79c72cbf3feed83e1512a8ae90a74a`.

The test compiles the probe against that jar (`FASTUTIL_JAR`, or `~/ava/fastutil-8.5.18.jar`, or
the PrismLauncher copy) and is skipped where the jar is absent.

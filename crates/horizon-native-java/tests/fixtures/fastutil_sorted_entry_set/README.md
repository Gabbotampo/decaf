# fastutil sorted entry set

`Object2BooleanLinkedOpenHashMap.object2BooleanEntrySet()` is an
`Object2BooleanSortedMap.FastSortedEntrySet`; this fixture is the oracle for its `iterator()` and
for the `ObjectBidirectionalIterator` it returns (`hasNext`, `next`, `remove`).

- `java25.txt`: `demo.Probe.rows()`, every case the runtime answers (zero unknown host calls).
- `java25_counted.txt`: `demo.Probe.countedRows()`, the cases that stay counted and why:
  `fastIterator()` reuses one entry object (`cherry,cherry,cherry`), fastutil's iterator reads its
  map live (a key put under it is visited, a map changed under it is never a
  `ConcurrentModificationException`), an entry returned before `remove()` reads the map as it is
  afterwards, and the tree maps iterate in natural key order through an unrelated
  `ObjectSortedSet`.

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

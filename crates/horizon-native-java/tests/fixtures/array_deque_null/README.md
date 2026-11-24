# Null elements in `ArrayDeque` and the two concurrent linked queues

`java.util.ArrayDeque`, `ConcurrentLinkedQueue` and `ConcurrentLinkedDeque` reject null elements. Every insertion of
null throws `NullPointerException` (no message, `new NullPointerException()`), whether the call names the class
(`invokevirtual ArrayDeque.add`) or an interface (`invokeinterface Deque.add`, `Queue.offer`, `Collection.addAll`):

- `add`, `offer`, `offerFirst`, `offerLast`, `addFirst`, `addLast`, `push` leave the collection unchanged.
- `ArrayDeque.addAll` adds one element at a time, so what precedes the null stays (`[a, null, b]` leaves `[a]`);
  `ConcurrentLinkedQueue.addAll` and `ConcurrentLinkedDeque.addAll` build a chain first and add nothing.
- A constructor given a collection holding a null throws; no collection exists afterwards.
- `contains(null)`, `remove(null)` and `removeFirstOccurrence(null)` on an `ArrayDeque` answer `false`.

`demo/Probe.java` is the only class the test ships; it needs nothing but the JDK. Each public no-argument `String`
method is one row, `name=<result or exception class and message>|<size>|<toString>` (a constructor row has no
collection afterwards and prints only the exception). `main` runs every row, sorted, so `java25.txt` is the raw
output, unedited. The row `forEachAfterFailedAddAll` is Java's answer for a `forEach` over a deque after a
failed `addAll` that grew its array; Horizon does not answer it (the test asserts the `forEach` stays counted).

```sh
cd crates/horizon-native-java/tests/fixtures/array_deque_null
/usr/lib/jvm/java-25-openjdk/bin/java -version
/usr/lib/jvm/java-25-openjdk/bin/javac --release 21 -d classes demo/Probe.java
/usr/lib/jvm/java-25-openjdk/bin/java -XX:ActiveProcessorCount=1 -Xmx512m -cp classes demo.Probe > java25.txt
rm -r classes
```

`java -version` (captured 2026-11-23):

```
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
```

No library or game jar is involved, so there is no jar checksum.

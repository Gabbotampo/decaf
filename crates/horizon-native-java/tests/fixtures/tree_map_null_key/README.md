# TreeMap/TreeSet with a null key and natural ordering, Java 25

`demo/Probe.java` calls each `TreeMap`/`TreeSet` member that takes a key with `null`, on an empty tree and on a tree
holding `"a"`, and prints one `label=result` line per call (`result` is the return value, or `class:message` of the
exception). `java25.txt` is the unedited output of `Probe.main` on Java 25.

```sh
J=/usr/lib/jvm/java-25-openjdk/bin
$J/javac --release 21 -d out demo/Probe.java
$J/java -XX:ActiveProcessorCount=1 -Xmx512m -cp out demo.Probe > java25.txt
```

`java -version`:

```
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
```

What the rows show:

- `compare(k1, k2)` calls `((Comparable) k1).compareTo(k2)`, so a null `k1` that reaches a comparison throws
  `NullPointerException: Cannot invoke "java.lang.Comparable.compareTo(Object)" because "k1" is null`. That happens
  for `put`/`putIfAbsent`/`add`/`merge`/`compute*`/`putAll`/`addAll` and the copy constructors on an EMPTY tree
  (`addEntryToEmptyMap` compares the key with itself), for `ceiling*`/`floor*`/`higher*`/`lower*` on a NON-EMPTY tree,
  and for `headMap`/`tailMap`/`headSet`/`tailSet` with one argument on either (the sub-map constructor compares the
  bound with itself).
- `get`/`containsKey`/`remove`/`replace`/`getOrDefault`/`contains`, `remove(key, value)`, `replace(key, old, new)`,
  and every insertion into a NON-EMPTY tree fail `Objects.requireNonNull(key)`: `NullPointerException:null`.
- `ceiling*`/`floor*`/`higher*`/`lower*` on an EMPTY tree walk nothing and return `null`.
- `computeIfAbsent`/`compute` with a mapping function that returns `null` on an EMPTY tree return `null`: the
  function runs first and no entry is added, so no comparison happens.

`Probe.counted*` are not in `java25.txt`: they are the calls the model does not answer (`subMap`, the two-argument
`headMap`/`headSet`, and `headMap(null)` with a comparator), and the test only checks that they stay counted.

# Java 25 `Pattern.asPredicate()` oracle

`java25.txt` is the raw output of `demo.Probe` (`rows`) on JDK 25, never edited.
`java25_counted.txt` is the same probe's `counted` mode: what the JDK prints for the
cases the runtime counts instead of answering (a `Pattern` with flags, a supplementary
character, a `test` argument that is not a `String`). It is not an oracle the runtime
claims to match; the tests only assert those calls are counted.

```sh
java -version
# openjdk version "25.0.5.1" 2026-11-17
# OpenJDK Runtime Environment (build 25.0.5.1)
# OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
mkdir out && /usr/lib/jvm/java-25-openjdk/bin/javac -d out demo/Probe.java
java -cp out demo.Probe > java25.txt
java -cp out demo.Probe counted > java25_counted.txt
```

No jar is involved: the probe only uses JDK classes.

Answered, with the oracle rows: `Pattern.compile(String).asPredicate()` and `test` on a
`String` (`matcher(s).find()` on a fresh matcher), including the Predicate handed to
`Stream.filter`, `List.removeIf` and `negate`/`and`/`or`, and a jar's own lambda,
method reference, anonymous and named `Predicate` going through the same call site.
Counted, never guessed: `Pattern.compile(String, int)` (flags), a `test` on `null`, a
`StringBuilder` or any non-`String`, an input with a supplementary character, and
`toString`/`hashCode` of the predicate.

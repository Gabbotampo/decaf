# `ArrayDeque` null elements

`java.util.ArrayDeque` rejects null elements: every insertion of null throws
`NullPointerException` (message `null`, i.e. `new NullPointerException()`),
`addAll` of a list containing null fails part-way and keeps the elements
added before the null, and `new ArrayDeque<>(collection)` with a null inside
throws too. `contains(null)`, `remove(null)` and `removeFirstOccurrence(null)`
answer `false` without throwing.

Captured with Java 25.0.4.1 (Temurin). Each line is one `case=answer` row:
the exception class and message (or the result), then the deque's size and
`toString()` after the call; the constructor row has no live deque
afterwards, so it prints only the exception.

```sh
java -version
openjdk version "25.0.4.1" 2026-08-18 LTS
OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS)
OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
javac --release 21 -d classes demo/Probe.java
java -cp classes demo.Probe > java25.txt
```

Rows:

- `add`, `addFirst`, `addLast`, `offer`, `offerFirst`, `offerLast`, `push` —
  inserting null on a fresh deque throws `java.lang.NullPointerException` with
  no message; the deque stays empty (`0|[]`).
- `addAll` — fresh deque, `addAll([a, null, b])` throws the same NPE after
  adding `a` (`1|[a]`); `addAllPrefix` starts from `[x]` and `addAll([null, y])`
  throws before adding anything (`1|[x]`).
- `constructor` — `new ArrayDeque<>([a, null, b])` throws the NPE; no deque
  exists afterwards.
- `containsNull`, `removeNull`, `removeFirstOccurrenceNull` — on `[a]`, each
  answers `false` and leaves the deque (`1|[a]`).
- `addValue`, `addAllOk` — neighbours showing non-null insertion still works
  (`true|1|[a]`, `true|2|[a, b]`).

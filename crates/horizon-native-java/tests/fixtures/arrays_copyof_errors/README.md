# `Arrays.copyOf` / `Arrays.copyOfRange` argument errors

`demo/Probe.java` is the only class the test ships; it needs nothing but the JDK (no game or library jar, so there is
no jar hash to record). Every row prints `label=value`: the copy as `Arrays.toString` (`deepToString` for `int[][]`)
shows it, or `class:message` of the exception the call threw. `rows()` has the 256 rows the runtime answers, on every
array kind (`int`, `long`, `double`, `float`, `short`, `byte`, `char`, `boolean`, `String`, `Integer[]`, `int[][]`, and
`new T[0]` for `int` and `String`) and on the `Class<? extends T[]>` overloads:

- `copyOf`: a negative length (`-1`, `Integer.MIN_VALUE`), zero, truncation, the same length, padding;
- `copyOfRange`: `from > to`, `from == to`, a negative `from`, `from` past the end by one and by more, `from == length`
  with padding, `to` past the end, a negative `to`, both negative, the wrapped `to - from` (`(-5, Integer.MAX_VALUE)` is
  `from > to` to Java; `(Integer.MAX_VALUE, Integer.MIN_VALUE)` is not), full and middle ranges.

`countedRows()` has three calls on `Object.class.getDeclaredFields()`: the runtime answers an unmodelled call's array as
a shared empty array of its declared type, whose exact class it does not know, so these stay counted
(`java25-counted.txt`).

Not in the probe, because Java's answer depends on the heap or the copy is not modelled: a copy longer than the
runtime's array bound (`Arrays.copyOf(a, Integer.MAX_VALUE)` is `OutOfMemoryError` on Java, an internal error in the
runtime, as before), and a `null` original (the exception depends on the overload; unchanged).

`java25.txt` and `java25-counted.txt` are the unedited output of:

```sh
J=/usr/lib/jvm/java-25-openjdk/bin
$J/javac --release 21 -d out demo/Probe.java
$J/java -XX:ActiveProcessorCount=1 -Xmx512m -cp out demo.Probe > java25.txt
$J/java -XX:ActiveProcessorCount=1 -Xmx512m -cp out demo.Probe counted > java25-counted.txt
```

`java -version`:

```
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
```

(`lib/modules` SHA-256 `2bffefbbdb53eb2c283925185a79dca3da792bbdcd849605409286f023f9d9a0`.) Captured 2026-11-23.

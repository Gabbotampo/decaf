# String.indexOf / lastIndexOf with an int code point: oracle

Captured with Java 25 (`openjdk version "25.0.5.1" 2026-11-17`), no library jar.

```sh
mkdir classes
$JAVA_HOME/bin/javac --release 21 -d classes demo/Probe.java
$JAVA_HOME/bin/java -cp classes demo.Probe > java25.txt
```

Each line is `<method>|<code point>|[<fromIndex>|]<receiver>=<result>` (`I` indexOf(int),
`LI` lastIndexOf(int), `II`/`LII` the `(int,int)` forms, `IIP`/`LIIP` the `(int,int)` forms on
`ab😀cd`, where the receiver holds a surrogate pair). Rows cover:

- needles outside `0..0x10FFFF` (`-1`, `Integer.MIN_VALUE`, `0x110000`, `Integer.MAX_VALUE`):
  never match;
- `0`, `0x61`, `0xFFFF`, `0x10FFFF`, and the supplementary `0x1F600`, which matches its pair;
- the lone surrogates `0xD83D` and `0xDE00`, which match one unit of a pair;
- `fromIndex` from below zero to past the end, `Integer.MIN_VALUE` and `Integer.MAX_VALUE`,
  including every index inside and around the pair;
- the empty receiver.

Raw Java 25 output is in `java25.txt` (never edited). Receivers the runtime cannot hold
exactly (a jar-object marker, text containing U+FFFD that may stand for a lost lone surrogate)
are not in the oracle: the runtime counts those calls.

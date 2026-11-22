# `String.strip`/`isBlank` use Java 25's whitespace set

`demo/Probe.java` is the only class the test ships; it needs nothing but the JDK. `rows()` prints one
`<row>=<value>` line per case and `main` prints them all, so `java25.txt` is its raw stdout on the JDK:

```sh
$J/javac --release 21 -d out demo/Probe.java
$J/java -Xmx512m -cp out demo.Probe | head -c 2000000 > java25.txt
```

`java -version`:
```
openjdk version "25.0.4.1" 2026-08-18 LTS
OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS)
OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)
```

Row kinds (each code point `cp` appears as six uppercase hex digits `XXXXXX`):

- `wsXXXXXX=1` — `Character.isWhitespace(cp)` is true. The full set, straight from the oracle: U+0009-U+000D, U+001C-U+001F, U+0020, U+1680, U+2000-U+2006, U+2008-U+200A, U+2028, U+2029, U+205F, U+3000. Notably absent: U+00A0, U+2007, U+202F.
- `spXXXXXX=1` — `Character.isSpaceChar(cp)` is true.
- `aXXXXXX.<op>` — `"a" + cp + "b"`; `bXXXXXX.<op>` — `cp + "a" + cp`; `zXXXXXX.<op>` — `cp` alone.
  `<op>` is `strip`, `stripLeading`, `stripTrailing` or `trim`, whose value is
  `len=N units=HHHHHH...` (result length in UTF-16 units and their hex), or
  `isBlank`, whose value is `true`/`false`.

The specials U+00A0, U+2007, U+202F (kept by strip), U+200B, U+FEFF, U+0085 and U+180E get the same subjects although they are not whitespace. A row the runtime cannot answer stays counted: the test asserts `unknown_host_calls()` is empty for the answered rows.

Re-run with `openjdk version "25.0.5.1" 2026-11-17` after the probe built its strings with
`Character.toString(int)`: byte-identical output.

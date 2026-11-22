# `i2c` keeps 16 bits (fixture `i2c_char_cast`)

`i2c` (0x92) truncates an int to `char` and zero-extends it (JVMS 6.5): the low 16 bits, unsigned. The runtime had
`(value as u8) as i32`, so every `(char)` cast, `char` compound assignment (`c += 300`) and `char` increment of a
value above 255 came out wrong (`(char) 256` gave 0, `(char) -1` gave 255). `i2b` and `i2s` were already right.

`demo/Probe.java` is the only class the test ships; it needs nothing but the JDK. Its inputs come from a static
array, so javac emits a real `i2c`/`i2b`/`i2s` for each cast (checked with `javap -c`: 61 `i2c`, 17 `i2b`, 16 `i2s`).
Values: 0, 65, 127, 128, 255, 256, 32767, 32768, 0x4e2d, 0xd7ff, 0xd800, 0xdfff, 0xffff, 0x10000, 0x10041, -1, -128,
-129, -32768, -32769, Integer.MIN_VALUE, Integer.MAX_VALUE, 0x12345678, 0x7fff8000. Methods (one `name=value` line
each in `java25.txt`): `charCast`, `byteCast`, `shortCast` (each value printed as `(int)` and as hex),
`charCastInline`, `charFromWide` (long, float and double sources), `chained` (char/byte/short in sequence),
`charArith` (`+= -= *= <<= >>= >>>= ^= |= &=`, `++ --`, `-c`, `~c` on a char), `byteShortArith`, `charFields`
(static and instance char fields), `charArrays` (stores and compound assignment on `char[]`), `narrowArrays`
(`byte[]`, `short[]`), `charStrings`, `charCalls`.

`charStrings` leaves lone surrogates out: a `String` here is Rust text, so the runtime counts an `append` of one
(`jdk_string_builder.rs`) and does not answer it. That stays counted.

`Run.java` prints each method's result. `java25.txt` was captured on 2026-11-22:

```sh
export JAVA_HOME=/usr/lib/jvm/java-25-openjdk
$JAVA_HOME/bin/javac --release 21 -d out demo/Probe.java
$JAVA_HOME/bin/javac -cp out -d out Run.java
$JAVA_HOME/bin/java -XX:ActiveProcessorCount=1 -Xmx512m -cp out Run > java25.txt
```

`java -version`: openjdk version "25.0.5.1" 2026-11-17, OpenJDK Runtime Environment (build 25.0.5.1). `java25.txt`
SHA-256: `ba632d06f8b4bb12c83aed6e14bccb34907132718c2134e803d1aa52ec705d99`. No library jar is involved.

`cargo test -p horizon-native-java --test fixture_i2c_char_cast`: before the fix nine methods differ (`charCast`,
`charCastInline`, `charFromWide`, `chained`, `charArith`, `charFields`, `charArrays`, `charStrings`, `charCalls`);
`byteCast`, `shortCast`, `byteShortArith` and `narrowArrays` matched already.

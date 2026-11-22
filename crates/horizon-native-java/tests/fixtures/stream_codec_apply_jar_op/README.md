# StreamCodec apply with a jar-supplied operation oracle

Captured with Java 25.0.5.1 against the real 26.3 game jar (`server-26.3.jar`)
and its bundled libraries, on the class path.

```sh
CP=<server-26.3.jar>:<every jar under the server's libraries>
mkdir classes
$JAVA_HOME/bin/javac --release 21 -cp "$CP" -d classes demo/Probe.java
$JAVA_HOME/bin/java -cp "classes:$CP" demo.Probe > java25.txt
```

`java -version`: `openjdk version "25.0.5.1" 2026-11-17`

`server-26.3.jar` SHA-256:
`a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6`

`Probe.main` prints one `<row>=<value>` line per method (package `demo`, class `Probe`):

- `identity`: `apply(op)` on `StreamCodec.unit("x")` calls the operation exactly once, passes it
  the receiver codec itself, and returns the operation's result (here the receiver).
- `resultUnchanged`: an operation returning another codec (`StreamCodec.unit("y")`) makes `apply`
  return that codec, not the receiver.
- `resultNull`: an operation returning `null` makes `apply` return `null`.
- `throws`: an `IllegalStateException("boom")` thrown by the operation propagates (class and
  message).
- `nullOp`: `apply(null)` throws a `NullPointerException` with the message
  `Cannot invoke "net.minecraft.network.codec.StreamCodec$CodecOperation.apply(net.minecraft.network.codec.StreamCodec)" because "operation" is null`.
- `geckolibShape`: `apply` with a method reference (`Probe::optionalLike`, the shape of geckolib's
  `STRING_UTF8.apply(ByteBufCodecs::optional)`) returns a `StreamCodec`. The codec that
  `ByteBufCodecs.optional` builds, and its encode/decode, are not covered by this oracle.

Raw Java 25 output is in `java25.txt` (never edited).

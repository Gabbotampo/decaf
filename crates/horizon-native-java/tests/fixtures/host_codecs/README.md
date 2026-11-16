# DataFixerUpper `Pair` and codecs as host objects (#156 P5c)

`demo/HostCodecs.java` makes pairs through the real API and reads an unmodelled codec constant.
`host_codec_tests.rs` compiles it against the compile-only stubs in `stubs/` and ships only
`demo/*`, so every call reaches the runtime's model.

The expected answers are `oracle/HostCodecsOracle.java` on OpenJDK 25.0.5+8 against the 26.3
server's `datafixerupper-10.0.21.jar`, `guava-33.6.0-jre.jar`, `failureaccess-1.0.3.jar` and
`fastutil`, two runs alike except an identity hash. The libraries are the player's own and are
never committed.

## Re-capturing `oracle/java25.txt`

On arcweld, with Java 25.0.5+8 selected by `~/ava/env.sh`, the committed rows are captured
with this exact command (the temporary Netty source is only a compile-time stub for the real
26.3 server classes):

```sh
set -o pipefail
. ~/ava/env.sh
ORACLE=/tmp/host-oracle345
rm -rf "$ORACLE/classes" && mkdir -p "$ORACLE/classes"
"$JAVA_HOME/bin/javac" --release 25 \
  -cp /tmp/datafixerupper-10.0.21.jar:$HOME/ava/minecraft-26.3-server-game.jar:$HOME/ava/fastutil-8.5.18.jar:/tmp/guava-33.6.0-jre.jar:/tmp/failureaccess.jar \
  -d "$ORACLE/classes" \
  "$ORACLE/src/io/netty/buffer/ByteBuf.java" \
  crates/horizon-native-java/tests/fixtures/host_codecs/demo/HostCodecs.java \
  crates/horizon-native-java/tests/fixtures/host_codecs/oracle/HostCodecsOracle.java
"$JAVA_HOME/bin/java" \
  -cp "$ORACLE/classes:/tmp/datafixerupper-10.0.21.jar:$HOME/ava/minecraft-26.3-server-game.jar:$HOME/ava/fastutil-8.5.18.jar:/tmp/guava-33.6.0-jre.jar:/tmp/failureaccess.jar" \
  HostCodecsOracle > "$ORACLE/java25-rerun.txt"
cmp -s "$ORACLE/java25-rerun.txt" crates/horizon-native-java/tests/fixtures/host_codecs/oracle/java25.txt
```

The final `cmp` must succeed; the output is never hand-edited.

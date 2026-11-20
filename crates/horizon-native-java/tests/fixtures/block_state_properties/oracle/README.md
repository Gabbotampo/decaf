# BlockStateProperties capture

`CaptureBlockStateProperties.java` prints the table in `src/vanilla_block_state_properties.rs`. Run it with
JDK 25 against the real Minecraft 26.3 server classes (never committed):

```sh
CP="<server-26.3.jar>:<every jar under the server's META-INF/libraries>"
javac -cp "$CP" -d out CaptureBlockStateProperties.java
java -cp "out:$CP" oracle.CaptureBlockStateProperties \
  | sed 's/^\[[^]]*\] \[[^]]*\]: \[STDOUT\]: //' | grep -v '^#'
```

The game's bootstrap routes stdout through its logger, so the `sed` strips the log prefix.

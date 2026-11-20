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

`CaptureStateDefiners.java` prints the table in `src/vanilla_block_state_definers.rs`: the vanilla
block classes whose state definition is not `Block`'s empty one. It takes the class names to check
(every `net/minecraft/world/level/block/*.class` of the server jar, internal form, one per line):

```sh
unzip -Z1 <server-26.3.jar> | grep -E '^net/minecraft/world/level/block/[A-Za-z0-9]+\.class$' \
  | sed 's/\.class$//' > names.txt
javac -cp "$CP" -d out CaptureStateDefiners.java
java -cp "out:$CP" oracle.CaptureStateDefiners names.txt
```

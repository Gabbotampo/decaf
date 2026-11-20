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

`CaptureBlockClasses.java` prints the table in `src/vanilla_block_state_definers.rs`: every vanilla
`Block` class in any package, with the class below `Block` that declares `createBlockStateDefinition`
(empty when it is `Block`'s own, a bare `return`). It takes the top-level class names to check
(every `net/minecraft/**.class` without `$`, internal form, one per line):

```sh
unzip -Z1 <server-26.3.jar> | grep -E '^net/minecraft/.*\.class$' | grep -v '\$' \
  | sed 's/\.class$//' > names.txt
javac -cp "$CP" -d out CaptureBlockClasses.java
java -cp "out:$CP" oracle.CaptureBlockClasses names.txt
```

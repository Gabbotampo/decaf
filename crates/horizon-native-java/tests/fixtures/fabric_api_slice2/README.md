Java 25 oracle for `fixture_fabric_api_slice2.rs` (`fabric_api_slice2.rs`, `recipe_serializer.rs`).

`java25.txt` is the direct output written by `demo/PhaseProbe.java`'s `main` running on
OpenJDK 25 (`jdk-25.0.5+8`) against the real 26.3 server jar, its libraries,
Fabric Loader, and the real Fabric API modules (`fabric-api-base` 2.0.6,
`fabric-message-api-v1` 7.0.10, `fabric-recipe-api-v1` 10.0.9). Line 1 is
`PhaseProbe.run()`, and line 2 is `synced=<RecipeSyncImpl.getSyncedSerializers().size()>`.

```sh
JDK="$JAVA_HOME"
SERVER="$MC263_SERVER_JAR"
LIBS="$MC263_LIBRARIES_DIR"
MODS="$FABRIC_PROCESSED_MODS_DIR"
LOADER="$FABRIC_LOADER_JAR"
CP="$SERVER:$LOADER:$MODS/fabric-api-base-2.0.6+fcdff87f5d-911714d21467d390.jar:$MODS/fabric-message-api-v1-7.0.10+3434d6d95d-2bd0c2513363578e.jar:$MODS/fabric-recipe-api-v1-10.0.9+b9b88df45d-703f6354514767fd.jar:$(find "$LIBS" -name '*.jar' | sort | tr '\n' ':')"
"$JDK/bin/javac" -cp "$CP" -d out crates/horizon-native-java/tests/fixtures/fabric_api_slice2/demo/PhaseProbe.java
"$JDK/bin/java" -cp "out:$CP" demo.PhaseProbe crates/horizon-native-java/tests/fixtures/fabric_api_slice2/java25.txt
```

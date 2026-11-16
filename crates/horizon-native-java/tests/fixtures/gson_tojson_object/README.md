# Gson `toJson(Object)` oracle

The expected bytes in `oracle/java25.txt` were captured from Java 25.0.5 and the real Gson 2.14.0 jar.

Jar SHA-256:

`2cbd119bf1961c28788310963dc80ba65f58cdeec1dd139c8bdb1240faa2c36f`

Capture command (from the repository root, with `JAVA_HOME` set to the Java 25.0.5 JDK):

```sh
GSON_JAR=~/ava/scratch/p14e/kit/lib/gson-2.14.0.jar
javac --release 25 -cp "$GSON_JAR" -d ~/ava/scratch/gson-tojson-object/oracle-classes \
  crates/horizon-native-java/tests/fixtures/gson_tojson_object/oracle/GsonToJsonObjectOracle.java
java -cp ~/ava/scratch/gson-tojson-object/oracle-classes:"$GSON_JAR" \
  GsonToJsonObjectOracle > crates/horizon-native-java/tests/fixtures/gson_tojson_object/oracle/java25.txt
```

The Rust fixture appends its returned string terminator and compares the resulting bytes directly with `java25.txt`.

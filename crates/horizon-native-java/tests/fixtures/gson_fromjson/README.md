# Gson fromJson oracle

`oracle/GsonFromJsonOracle.java` is compiled and run with the real Gson 2.14.0
jar from Minecraft 26.3 on Temurin Java 25.0.5. The Rust integration fixture
uses the same `demo` shapes against the bounded interpreter. `oracle/java25.txt`
is the captured stdout from the command below.

The jar is pinned by SHA-256:

```text
2cbd119bf1961c28788310963dc80ba65f58cdeec1dd139c8bdb1240faa2c36f  gson-2.14.0.jar
```

On arcweld:

```sh
source ~/ava/env.sh
test "$("$JAVA_HOME/bin/java" -version 2>&1 | sed -n '1s/.*version "\([^"]*\)".*/\1/p')" = 25.0.5
G="$HOME/ava/scratch/vanilla-values/oracle/libraries/server/com/google/code/gson/gson/2.14.0/gson-2.14.0.jar"
test "$(shasum -a 256 "$G" | awk '{print $1}')" = 2cbd119bf1961c28788310963dc80ba65f58cdeec1dd139c8bdb1240faa2c36f
rm -rf /tmp/gson-fromjson-oracle && mkdir -p /tmp/gson-fromjson-oracle
"$JAVA_HOME/bin/javac" -cp "$G" -d /tmp/gson-fromjson-oracle oracle/GsonFromJsonOracle.java
"$JAVA_HOME/bin/java" -cp "/tmp/gson-fromjson-oracle:$G" demo.GsonFromJsonOracle > oracle/java25.txt
```

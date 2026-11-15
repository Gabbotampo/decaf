# Gson fromJson oracle

`oracle/GsonFromJsonOracle.java` is compiled and run with the real gson 2.14.0
jar from Minecraft 26.3 on Java 25. The Rust integration fixture uses the
same `demo` shapes against the bounded interpreter; the oracle is intentionally
kept as source so its values can be regenerated on a clean machine.

On arcweld:

```sh
G=$HOME/ava/scratch/game-pure-values/mc/libraries/server/com/google/code/gson/gson/2.14.0/gson-2.14.0.jar
mkdir -p /tmp/gson-oracle && javac --release 21 -cp "$G" -d /tmp/gson-oracle oracle/GsonFromJsonOracle.java
java -cp "/tmp/gson-oracle:$G" GsonFromJsonOracle
```

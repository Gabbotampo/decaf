# Fabric Loader SemanticVersion Java 25 capture

`java25.txt` is the raw output of `oracle/OracleMain.java` run on a real JDK 25 against the real
Fabric Loader 0.19.3 jar (Apache-2.0), for each text: `Version.parse(text)` and
`SemanticVersion.parse(text)`, then `demo/Probe.java` (`describe`) on what `Version.parse`
returned.

```
java -version
  openjdk version "25.0.5.1" 2026-11-17
  OpenJDK Runtime Environment (build 25.0.5.1)
JAR=fabric-loader-0.19.3.jar   # SHA-256 73eed8c34bbad0320a2a3cba5346351e822f74f82b3f3c060574068474132958
javac -cp $JAR -d out demo/Probe.java oracle/OracleMain.java
java -cp out:$JAR oracle.OracleMain > java25.txt
```

`stubs/` are compile-only stand-ins for the loader API the Decaf side of the test compiles
`demo/Probe.java` against (never run on Java). Only the texts the oracle parses to a
`SemanticVersion` are answered by the runtime (`src/fabric_version.rs`); every other text and every
call without a row (`compareTo`, `equals`, `hashCode`, `Version.parse`) stays counted.

# Log4j getLogger(name, MessageFactory) oracle

Captured with Java 25 and Log4j API/core 2.26.0 shipped with the game:

```sh
J=/Users/sparks/ava/jdk/jdk-25.0.5+8/Contents/Home/bin
A=/Users/sparks/ava/scratch/vanilla-values/oracle/libraries/server/org/apache/logging/log4j/log4j-api/2.26.0/log4j-api-2.26.0.jar
C=/Users/sparks/ava/scratch/vanilla-values/oracle/libraries/server/org/apache/logging/log4j/log4j-core/2.26.0/log4j-core-2.26.0.jar
$J/javac -cp "$A:$C" -d . demo/Probe.java
$J/java -cp "$A:$C:." demo.Probe
```

# SharedConstants current version

Java 25 capture of the real 26.3 server jar. The same `demo/Probe.java` source is used by
`java25.txt` and the Decaf fixture.

```sh
. ~/ava/env.sh
mkdir -p /tmp/shared-constants-version-capture
javac -cp ~/ava/minecraft-26.3-server-game.jar -d /tmp/shared-constants-version-capture oracle/Capture.java
CP=/Users/sparks/ava/minecraft-26.3-server-game.jar; for j in ; do CP=:; done; java -cp /tmp/shared-constants-version-capture: Capture
```

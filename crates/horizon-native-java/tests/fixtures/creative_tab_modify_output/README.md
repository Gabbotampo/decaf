# Fabric creative-tab modify-output capture

Captured with Java 25.0.5+8 against the real Fabric Creative Tab API (`fabric-creative-tab-api-v1`
5.0.21), `fabric-api-base` (2.0.6), and the Minecraft 26.3 server jar and libraries.

```sh
SERVER_JAR="<path-to-server-26.3.jar>"
SERVER_LIBS="<path-to-server-26.3-libraries>"
FAPI_BASE="<path-to-fabric-api-base-2.0.6.jar>"
FAPI_CT="<path-to-fabric-creative-tab-api-v1-5.0.21.jar>"
CP="$SERVER_JAR:$(find "$SERVER_LIBS" -name '*.jar' | tr '\n' ':'):$FAPI_BASE:$FAPI_CT"
javac -cp "$CP" -d out oracle/CaptureCreativeTabOutput.java oracle/CaptureVanillaItemIds.java
java -cp "$CP:out" CaptureCreativeTabOutput > java25.txt
java -cp "$CP:out" CaptureVanillaItemIds > ../../../src/vanilla_item_ids_java25.txt
```

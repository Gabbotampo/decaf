# Creative-tab keys: Java 25 capture

`../creative_mode_tabs_java25.txt` is the output of `oracle/CaptureTabs.java` on OpenJDK 25.0.5 against the
real 26.3 server classes. In the server jar `CreativeModeTabs.FUNCTIONAL_BLOCKS`/`COMBAT` are private
(Fabric's access widener opens them for mods), so the capture reads them by reflection. `demo/Probe.java`
asks the runtime the same questions through the public names a Fabric mod compiles against.

With `JAVA_HOME` at a JDK 25.0.5 and `SERVER` at a folder holding the 26.3 server bundler jar's
`META-INF/` contents (never commit them):

```sh
CP=$SERVER/versions/26.3/server-26.3.jar:$(find "$SERVER/libraries" -name '*.jar' | tr '\n' ':')
mkdir out
"$JAVA_HOME/bin/javac" -d out oracle/CaptureTabs.java
"$JAVA_HOME/bin/java" -cp "out:$CP" CaptureTabs > ../creative_mode_tabs_java25.txt
```

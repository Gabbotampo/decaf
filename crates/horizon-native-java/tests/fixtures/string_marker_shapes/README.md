# Java 25 capture

`java25.txt` is the output of `demo.Probe` on OpenJDK 25.0.5. To recapture, from this folder with
`JAVA_HOME` pointing at a JDK 25.0.5:

```sh
mkdir classes
"$JAVA_HOME/bin/javac" -d classes demo/Probe.java
"$JAVA_HOME/bin/java" -cp classes demo.Probe > java25.txt
```

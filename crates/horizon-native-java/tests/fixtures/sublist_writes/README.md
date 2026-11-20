# `List.subList` writes (#549)

`java25.txt` is the output of `demo.Probe` on OpenJDK 25.0.5. To recapture, from this folder with
`JAVA_HOME` pointing at a JDK 25.0.5:

```sh
mkdir out
"$JAVA_HOME/bin/javac" -d out demo/Probe.java
"$JAVA_HOME/bin/java" -cp out demo.Probe > java25.txt
```

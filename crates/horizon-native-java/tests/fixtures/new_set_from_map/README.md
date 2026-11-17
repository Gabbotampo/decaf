```sh
J="$HOME/ava/jdk/jdk-25.0.5+8/Contents/Home/bin"
mkdir -p out
"$J/javac" -d out demo/Probe.java
"$J/java" -cp out demo.Probe
```

# Java 25 capture

Run from this fixture folder:

```sh
J=$HOME/ava/jdk/jdk-25.0.5+8/Contents/Home/bin
rm -rf classes && mkdir classes
$J/javac -d classes demo/Probe.java
$J/java -cp classes demo.Probe
```

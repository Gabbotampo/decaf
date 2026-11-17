# vanilla_static Java 25 capture

From `~/ava/wt/issue-483`:

```sh
J=~/ava/jdk/jdk-25.0.5+8/Contents/Home/bin
L=~/ava/scratch/vanilla-values/oracle/libraries/server
CP=~/ava/minecraft-26.3-server-game.jar:$(find "$L" -name '*.jar' | tr '\n' ':')
mkdir -p /tmp/vanilla-static-classes
$J/javac -cp "$CP" -d /tmp/vanilla-static-classes crates/horizon-native-java/tests/fixtures/vanilla_static/demo/Probe.java /tmp/Capture483.java
$J/java -cp "/tmp/vanilla-static-classes:$CP" Capture483
```

The first field is `CommonComponents.EMPTY.getString()`, whose Java 25 value is the empty string; therefore the capture begins with `|`.

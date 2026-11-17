# Vanilla Items capture

Run from the repository worktree with Java 25 and the 26.3 server game jar:

```sh
J=$HOME/ava/jdk/jdk-25.0.5+8/Contents/Home/bin
CP=$HOME/ava/minecraft-26.3-server-game.jar:$(find $HOME/ava/scratch/vanilla-values/oracle/libraries/server -name '*.jar' | paste -sd: -)
$J/javac -cp "$CP" crates/horizon-native-java/tests/fixtures/vanilla_items/demo/ItemsCapture.java
$J/java -cp "crates/horizon-native-java/tests/fixtures/vanilla_items:$CP" demo.ItemsCapture
```

The probe bootstraps vanilla with `SharedConstants.tryDetectVersion(); Bootstrap.bootStrap();` and prints one `FIELD|registry id` row per static `Items` field.

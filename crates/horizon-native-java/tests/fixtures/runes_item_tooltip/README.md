# Item tooltip fixture captures

`java25.txt` holds the exact Minecraft 26.3 `ComponentSerialization.CODEC` + `JsonOps.INSTANCE`
output for every component shape tested in `tests/fixture_runes_item_tooltip.rs`, captured on
OpenJDK 25.0.5+8 against the real Minecraft 26.3 server jar and its bundled libraries. The CI
fixture compiles its probe classes against compile-only stubs (shipping only `demo/` and `lib/`
classes) and compares `StdHost::component_json` against `java25.txt`.

To regenerate `java25.txt` (with the unpacked Minecraft 26.3 server `META-INF` directory and
OpenJDK 25):

```sh
MC_META=/path/to/mc263-server/META-INF
CP="$MC_META/versions/26.3/server-26.3.jar:$(find "$MC_META/libraries" -name '*.jar' | tr '\n' ':')"
"$JAVA25/bin/javac" --release 25 -d out -cp "$CP" \
  crates/horizon-native-java/tests/fixtures/runes_item_tooltip/TooltipComponentsOracle.java
"$JAVA25/bin/java" -cp "out:$CP" TooltipComponentsOracle \
  | sed -n 's/^.*CASE://p' \
  > crates/horizon-native-java/tests/fixtures/runes_item_tooltip/java25.txt
```

The real Runes jar test is opt-in through `RUNES_JAR` so repository tests never depend on a
machine-local corpus path.

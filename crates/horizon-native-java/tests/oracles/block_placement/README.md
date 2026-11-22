# Block Placement Java 25 Oracle

Generates the ground-truth capture under
`crates/horizon-native-java/tests/fixtures/block_placement/placement_java25.txt`
by running `BlockPlacementOracle.java` on real JDK 25 against Minecraft 26.3's
server jar (`server-26.3.jar`) and its bundled runtime libraries.

Every row is produced directly by vanilla's `Direction.orderedByNearest`,
`UseOnContext`, `BlockPlaceContext`, and `DirectionalPlaceContext` methods; no
placement math is reimplemented in the oracle.

## Regenerate captures

Set `JDK_HOME`, `MC_SERVER_JAR`, and `MC_SERVER_LIBS` to your local JDK 25 and
extracted Minecraft 26.3 server directories:

```bash
CP="$MC_SERVER_JAR:$(find "$MC_SERVER_LIBS" -name '*.jar' | tr '\n' ':')"

mkdir -p scratch/oracle-placement-classes
"$JDK_HOME/bin/javac" --release 25 -cp "$CP" -d scratch/oracle-placement-classes \
  crates/horizon-native-java/tests/oracles/block_placement/BlockPlacementOracle.java
"$JDK_HOME/bin/java" -cp "scratch/oracle-placement-classes:$CP" \
  BlockPlacementOracle crates/horizon-native-java/tests/fixtures/block_placement
```

## Verified inputs (SHA-256)

| Artifact | SHA-256 |
| --- | --- |
| `server-26.3.jar` | `a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6` |

## Capture format (`placement_java25.txt`)

- `PLAYER|<yaw_hex>|<pitch_hex>|<horiz>|<rot_hex>|<nearest>|<nearest_vert>|<ordered_csv>`:
  `BlockPlaceContext` with a live `ServerPlayer` across 17 yaws (including tie
  and wrap cases `45`, `135`, `-45`, `180`, `-180`, `360`, `-0.0`, `89.99`,
  `-157.3`, `720`) × 9 pitches (`-90`, `-45`, `-22.5`, `-0.0`, `0`, `13.7`,
  `45`, `89.99`, `90`).
- `PLACE_DIRS|<yaw_hex>|<pitch_hex>|<face>|<replaceClicked>|<dirs_csv>`:
  `BlockPlaceContext.getNearestLookingDirections()` across all 17 yaws × 9
  pitches × 6 clicked faces × `replaceClicked` (`false`/`true`).
- `NULL_PLAYER|<face>|<replaceClicked>|<player_null>|<horiz>|<rot_hex>|<secondary>|<nearest>|<nearest_vert>|<dirs>`:
  plain `BlockPlaceContext` with `player == null` across 6 faces ×
  `replaceClicked`.
- `DIRECTIONAL|<direction>|<face>|<player_null>|<horiz>|<rot_hex>|<secondary>|<nearest>|<nearest_vert>|<dirs>`:
  `DirectionalPlaceContext` across all 6 dispenser directions × 6 clicked faces.

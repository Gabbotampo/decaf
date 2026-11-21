# Block shape oracles (#609 polish: getShape)

Captured with Java 25.0.5+8 against the Minecraft 26.3 server classes (server jar SHA-256 `a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6`) and its
bundled libraries (`META-INF/libraries/**`). No game files are in the repo.

| Harness | Output |
|---|---|
| `ShapeOracle.java` | the `VoxelShape` values in `tests/fixture_block_shapes.rs`: Runes' altar boxes rebuilt from its `<clinit>` constants (`Block.box` + `Shapes.or`), the javac fixture's shapes, `Shapes.join` with each `BooleanOp`, `move`, the epsilon edge, `toString`, and the exception messages |
| `CaptureAllOracle.java` | `src/vanilla_block_shapes.rs`: for each of the 298 vanilla block classes, whether it inherits `BlockBehaviour.getShape` (full cube) or overrides it |

```sh
SERVER=<extracted server jar dir>/META-INF/versions/26.3/server-26.3.jar
CP="$SERVER:$(find <extracted server jar dir>/META-INF/libraries -name '*.jar' | tr '\n' ':')"
javac -cp "$CP" -d out ShapeOracle.java CaptureAllOracle.java
java -cp "$CP:out" ShapeOracle "$SERVER"
java -cp "$CP:out" CaptureAllOracle "$SERVER" vanilla_block_shapes.txt
```

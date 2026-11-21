# Recipe oracles (#609 W9)

Captured with Java 25.0.5+8 against the Minecraft 26.3 server classes (server jar SHA-256 `a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6`) and
the libraries the server jar bundles (`META-INF/libraries/**`). No game files are in the repo.

| Harness | Output |
|---|---|
| `DumpVanillaTables.java` | the vanilla item, max-stack-size, item-tag and `DataComponents` tables in `src/vanilla_item_data.rs` (after `SharedConstants.tryDetectVersion()`, `Bootstrap.bootStrap()`, `DataComponentInitializers.build()` and the vanilla item tags from the server jar) |
| `GeneralRecipesOracle.java` | `tests/fixtures/recipes/mc263.txt` |
| `RunesRecipesOracle.java` | `tests/fixtures/runes_recipes/mc263.txt`, run with the Runes 1.3.2 jar (`runes-fabric-1.3.2+26.3.jar`, SHA-256 `8515d3c5eb4d691655fa1663538cbfb6fbcc42001a0c9a73e28df7e990aea6bb`) as its argument |

```sh
SERVER=<extracted server jar dir>/META-INF/versions/26.3/server-26.3.jar
CP="$SERVER:$(find <extracted server jar dir>/META-INF/libraries -name '*.jar' | tr '\n' ':')"
javac -cp "$CP" -d out DumpVanillaTables.java GeneralRecipesOracle.java
java -cp "$CP:out" GeneralRecipesOracle > ../../fixtures/recipes/mc263.txt
javac -cp "$CP:$RUNES_JAR" -d out RunesRecipesOracle.java
java -cp "$CP:$RUNES_JAR:out" RunesRecipesOracle "$RUNES_JAR" > ../../fixtures/runes_recipes/mc263.txt
```

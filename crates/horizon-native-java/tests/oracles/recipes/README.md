# Recipe oracles (#609 W9)

Captured with Java 25.0.5+8 against the Minecraft 26.3 server classes (server jar SHA-256 `a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6`) and
the libraries the server jar bundles (`META-INF/libraries/**`). No game files are in the repo.

| Harness | Output |
|---|---|
| `DumpVanillaTables.java` | the vanilla item, non-64 max-stack-size, max-damage, item-tag, `DataComponents`, vanilla `RecipeSerializer`, and vanilla `RecipeType` tables in `src/vanilla_item_data.rs` (after `SharedConstants.tryDetectVersion()`, `Bootstrap.bootStrap()`, `DataComponentInitializers.build()` and the vanilla item tags from the server jar; `--rust-extra` prints the `VANILLA_MAX_DAMAGE`, `VANILLA_RECIPE_SERIALIZERS`, and `VANILLA_RECIPE_TYPES` tables) |
| `GeneralRecipesOracle.java` | `tests/fixtures/recipes/mc263.txt` (DFU `optionalFieldOf` JSON `null` handling, `Codec.INT`/`ExtraCodecs.intRange` fractional truncation, `Identifier.CODEC` whitespace rejection, `RecordCodecBuilder` full-field evaluation and error accumulation, `PatchedDataComponentMap` prototype/redundancy/canonical overlay semantics, `ItemStackTemplate` negative counts and `ItemStack.validateStrict`, empty-stack `getMaxStackSize() == 1`, `SimpleContainer` default max 99 and virtual `getMaxStackSize()` override, `RecipeManager`/`RecipeMap` `RecipeInput.isEmpty()` precheck and `ResourceKey` registry checks) |
| `RunesRecipesOracle.java` | `tests/fixtures/runes_recipes/mc263.txt`, run with the Runes 1.3.2 jar (`runes-fabric-1.3.2+26.3.jar`, SHA-256 `8515d3c5eb4d691655fa1663538cbfb6fbcc42001a0c9a73e28df7e990aea6bb`) as its argument |
| `FabricPackOrderOracle.java` | `tests/fixtures/recipes/fabric263_pack_order.txt`, captured by booting a live Fabric 26.3 server (`KnotServer`, `fabric-api-0.166.2+26.3.jar` SHA-256 `e8acacc7836f39237c83c2a664b039647ee36f93822797acc40cda44666002ea`, `fabric-resource-loader-v1-3.1.0+5af7744b5d.jar` SHA-256 `f7b76f4512714c1a82f9838b09016188b1d5b6c72d5bfd2928908cfbc7b6aaff`, `fabric-api-base-2.0.6+fcdff87f5d.jar` SHA-256 `e3b64039e8f261cc24a5013cb8b442cf368f08afa28d01ecdf97f3ccdeb7b63c`) with test mods exercising `fabric:resource_load_order` `before`/`after`, cycles, malformed values, nested JiJ mods (`nested_zzz` in `mod_a`, `nested_aaa` in `mod_c`), world datapack `file/dp_override`, and `ModPackResourcesSorter.addLoadOrdering` self-dependency |

```sh
SERVER=<extracted server jar dir>/META-INF/versions/26.3/server-26.3.jar
CP="$SERVER:$(find <extracted server jar dir>/META-INF/libraries -name '*.jar' | tr '\n' ':')"
javac -cp "$CP" -d out DumpVanillaTables.java GeneralRecipesOracle.java
java -cp "$CP:out" DumpVanillaTables --rust-extra
java -cp "$CP:out" GeneralRecipesOracle ../../fixtures/recipes/mc263.txt
javac -cp "$CP:$RUNES_JAR" -d out RunesRecipesOracle.java
java -cp "$CP:$RUNES_JAR:out" RunesRecipesOracle "$RUNES_JAR" > ../../fixtures/runes_recipes/mc263.txt
# FabricPackOrderOracle.java is packaged as a Fabric mod jar alongside fabric-api-0.166.2+26.3.jar
# and the test mod jars + world/datapacks/dp_override, then launched on KnotServer with
# -Dfabric.gameJarPath="$SERVER" -Dorderprobe.out=../../fixtures/recipes/fabric263_pack_order.txt
```

# Bundle API Java 25 Oracle

Generates ground-truth captures under `crates/horizon-native-java/tests/fixtures/bundle_api/`
by executing `BundleApiOracle.java` on real JDK 25 against Minecraft 26.3's server jar and
bundled runtime libraries (Apache Commons Lang 3, DataFixerUpper, Netty).

## Regenerate captures

```bash
JDK=/path/to/jdk-25.0.5+8
MC=/path/to/mc263-server/META-INF/versions/26.3/server-26.3.jar
LIBS=/path/to/mc263-server/META-INF/libraries
CP="$MC:$(find "$LIBS" -name '*.jar' | tr '\n' ':')"

mkdir -p scratch/oracle-classes
"$JDK/bin/javac" --release 25 -cp "$CP" -d scratch/oracle-classes \
  crates/horizon-native-java/tests/oracles/bundle_api/BundleApiOracle.java
"$JDK/bin/java" -cp "scratch/oracle-classes:$CP" \
  BundleApiOracle crates/horizon-native-java/tests/fixtures/bundle_api
```

## Verified inputs (SHA-256)

| Artifact | SHA-256 |
| --- | --- |
| `server-26.3.jar` | `a362163eec5d1612d520772bc16e5b39c09e3b234fdc045f56bf544284ee8ae6` |
| `commons-lang3-3.20.0.jar` | `69e5c9fa35da7a51a5fd2099dfe56a2d8d32cf233e2f6d770e796146440263f4` |
| `datafixerupper-10.0.21.jar` | `3547cb1da8993fb7ebd42494d0e78997d2cfbe90c421f5efd022863e6909941e` |
| `netty-buffer-4.2.16.Final.jar` | `cc36ae9fbd0b03fe755eb4eb4424ca53b59cfd297d7fd47d49e5b1059beded6c` |
| `netty-codec-base-4.2.16.Final.jar` | `feb410225938d9970de6b624a0c031d079804fa5cc5e1ec6e9298f16db5998a7` |
| `runes-fabric-1.3.2+26.3.jar` | `8515d3c5eb4d691655fa1663538cbfb6fbcc42001a0c9a73e28df7e990aea6bb` |

## Captures

- `fraction.txt`: all 12 `Fraction` constants, identity (`==`) vs `.equals()` vs `.compareTo()`,
  constructors (`(II)`, `(III)`, `(D)`, `(String)`, `getReducedFraction`), arithmetic
  (`add`, `subtract`, `multiplyBy`, `divideBy`, `negate`, `abs`, `invert`, `reduce`, `pow`),
  overflow/error exception classes and messages, and `Mth.mulAndTruncate(Fraction, int)`.
- `argb.txt`: `ARGB.colorFromFloat(FFFF)` normal + out-of-range + NaN/Inf edge cases,
  `as8BitChannel`, channel extractors, `color`, `gray`, `multiply`, `average`, float accessors.
- `record_codec.txt`: `RecordCodecBuilder.create`, `Products$P1` / `P2`, `optionalFieldOf`,
  `ItemPredicate.CODEC`, `CollectionPredicate.codec`, and `ItemStackTemplate.CODEC.listOf().xmap(...)`
  JSON decode and encode round trips (including default omission and error cases).
- `stream_codec_list.txt`: `ByteBufCodecs.list()`, `list(maxSize)`, `fixedSizeList(n)`,
  `collection(ctor, maxSize)`, `optional(...)`, `StreamCodec.apply(CodecOperation)`,
  `StreamCodec.map(...)`, and `StreamCodec.composite(...)` wire hex bytes, decoded equality,
  implementation class names, and `EncoderException`/`DecoderException` size-limit errors.

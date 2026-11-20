Java 25 oracle for `LootItemConditionalFunction.commonFields` on the real 26.3 server and DFU jars.
`oracle/Probe.java` boots the game, calls the inherited helper with a real
`RecordCodecBuilder.Instance`, and prints the P1 record arity, its Java record
component name, and the component's class/value shape. `java25.txt` is captured
from OpenJDK 25.0.5+8 against the real jars; the identity suffix is the JDK's
actual run output and is retained as the oracle artifact.

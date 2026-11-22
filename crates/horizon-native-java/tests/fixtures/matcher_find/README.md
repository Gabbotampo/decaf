# Java 25 Matcher.find oracle

`demo/Probe.java` runs `java.util.regex.Matcher#find()` and the calls that read
what it found (`group`, `start`, `end`, `groupCount`), plus `find(int)`,
`reset()` and what `matches()` leaves behind for them, on the patterns
fabric-carpet compiles (`#([0-9a-fA-F]{6})` in `Messenger`, `\[[^\[]*]$` in
`NBTSerializableValue`) and on small patterns that pin where the next search
starts (empty matches, a miss that follows one, a lookahead that moves
`Matcher.last`). `java25.txt` is the probe's raw output on the real JDK 25; the
rows are one `name<TAB>result` line each, an exception printed as its class and
message.

Captured on 2026-11-22 with (the probe uses only the JDK; no library jar is on
the class path):

```sh
java -version
# openjdk version "25.0.5.1" 2026-11-17
# OpenJDK Runtime Environment (build 25.0.5.1)
# OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)

mkdir -p /tmp/matcher-find-classes
nice -n 19 ionice -c3 /usr/lib/jvm/java-25-openjdk/bin/javac \
    -d /tmp/matcher-find-classes crates/horizon-native-java/tests/fixtures/matcher_find/demo/Probe.java
nice -n 19 ionice -c3 /usr/lib/jvm/java-25-openjdk/bin/java -XX:ActiveProcessorCount=1 -Xmx512m \
    -cp /tmp/matcher-find-classes demo.Probe \
    > crates/horizon-native-java/tests/fixtures/matcher_find/java25.txt
```

The patterns carpet uses were read from the jar's own constants (`javap -c -p`
and `strings` on `carpet/utils/Messenger.class` and
`carpet/script/value/NBTSerializableValue.class`, MIT); fabric-carpet
26.3+v260915 has SHA-256
`7c40b93e807ed914064176898037117097446c6cc76e6a80a856c8bf7f7659f9`.

## What is answered, what is counted

Answered exactly (every row above): `find()`, `find(int)`, `matches()`,
`reset()`, `group()`, `group(int)`, `start()`, `start(int)`, `end()`, `end(int)`
and `groupCount()` on a matcher over a `String`, for a pattern and input the
`java_regex` engine decides, with Java's `IllegalStateException` ("No match
found") and `IndexOutOfBoundsException` ("No group n", "Illegal start index").

Counted, never guessed (no row, because the runtime does not answer them):
`find()` after an empty match at the end of the input (the `false` is answered,
the matcher is not followed after it), a `find()` that fails after a lookaround
moved `last`, a matcher over a `StringBuilder`'s text, a group whose value the
engine does not promise, a pattern or input outside the engine's subset, and
every other `Matcher` method (`lookingAt`, `region`, `usePattern`,
`reset(CharSequence)`, `replaceAll`, `appendReplacement`, `results`,
`group(String)`...). Calling one of the state-changing ones counts every later
call on that matcher too. `regex_matcher.rs` has unit tests for these.

# Java 25 Matcher.find oracle

`demo/Probe.java` runs `java.util.regex.Matcher#find()` and the calls that read
what it found (`group`, `start`, `end`, `groupCount`), plus `find(int)`,
`reset()` and what `matches()` leaves behind for them, on the patterns
fabric-carpet compiles (`#([0-9a-fA-F]{6})` in `Messenger`, `\[[^\[]*]$` in
`NBTSerializableValue`) and on small patterns that pin where the next search
starts (empty matches, a miss that follows one, a lookahead that moves
`Matcher.last`), and call sequences (`seq[pattern|input|ops]` rows) that run
after an attempt that failed over a quantified pattern. `java25.txt` is the probe's raw output on the real JDK 25; the
rows are one `name<TAB>result` line each, an exception printed as its class and
message. The `rows()` part is what the runtime answers; the
`counted()` part after the `--counted--` line (37 lines) holds Java's answer to
calls the runtime must not answer, and the test only checks they are counted.
A `seq[...]` row runs its `ops` on a fresh matcher: `f` find(), `m` matches(),
`g` group(), `h` group(1), `s` start(), `e` end(), `r` reset(), `c`
groupCount(), a digit `d` find(d); a call that throws prints `op!class:message`.

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

**Answered exactly**, each shown by the `rows()` lines above: `find()`,
`find(int)`, `matches()`, `reset()`, `group()`, `group(int)`, `start()`,
`start(int)`, `end()`, `end(int)` and `groupCount()` on a matcher over a
`String`, for a pattern and input the `java_regex` engine decides, with Java's
`IllegalStateException` ("No match found") and `IndexOutOfBoundsException`
("No group n", "Illegal start index"), except `group()` and `group(int)` over
an input that may hold U+FFFD (see **Not claimed**). The rows pin, among the
rest:

- the empty-match rule (`find()` resumes one `char` after an empty match) and
  what a failed `matches()` leaves for the next `find()`, **for patterns with
  no quantifier** (`a`, `(?=a)`, `(\w+)-(\d+)`...);
- after a failed `matches()` or `find()` over a pattern **with** a quantifier
  (`a?b`, `a{2}`, `.?`, `#([0-9a-fA-F]{6})`, `a.*?b`, `a*+b`, `(?:ab)*c`,
  `(a{2})+b`...): `group`, `start`, `end` (which throw), `matches()`,
  `find(int)` and `reset()` followed by `find()`; none of them reads `last`.

**Counted, never guessed:**

- A `find()` after an attempt that failed over a pattern with a quantifier on
  a character or a group (`?`, `*`, `+`, `{n,m}`, lazy and possessive forms),
  and the calls after it, whether the attempt was a `find()`, a `find(int)` or
  a `matches()`. Java's repeated atoms set `Matcher.last` on every
  iteration, including in an attempt that fails, so the next `find()` resumes
  from a place the runtime does not know; the 37 `counted()` rows hold Java's
  answers (a `find()` that is `true` here, `false` there) and the test checks
  the runtime answers none of those `find()`s or the `group()`s after them. The
  `false` of the failed attempt itself is answered. A successful attempt sets
  `last` to the match's end, so only misses lose it, and a later successful
  `matches()`, `find(int)` or `reset()` finds it again.
- A `find()` after a `find()` that failed once a lookahead or an atomic group
  had run (`(?=a)b`, `(?=a)(?=a)b`, `(?>a)b`, `(?!b)c`; `FAILED_LOOK`): Java
  left `last` at that sub-pattern's end, wherever the start positions it tried
  stopped. Counted like the quantified case, and the calls that do not read
  `last` are answered after it (`FAILED_LOOK` rows in `rows()`). After a failed
  lookbehind (`FAILED_BEHIND`) the runtime follows the matcher and answers the
  second `find()` too; these rows show Java's answers (`false`), but they
  cannot tell whether Java's `last` moved there, since a search that failed
  from the start fails again from anywhere after it.
- `find()` after an empty match at the end of the input (the `false` is
  answered, the matcher is not followed after it).
- A matcher over a `StringBuilder`'s text, a group whose value the engine does
  not promise, a pattern or input outside the engine's subset, and every other
  `Matcher` method (`lookingAt`, `region`, `usePattern`, `reset(CharSequence)`,
  `replaceAll`, `appendReplacement`, `results`, `group(String)`...). Calling a
  state-changing one counts every later call on that matcher too, `matches()`
  included: Java answers it over the region, pattern or input the call left,
  not over the ones the handle holds (the `changed()` rows: `region(0,1)` then
  `matches()` is `true` for `a` over `ab`, `usePattern(b)` then `matches()` is
  `false` for `a` over `a`). A matcher whose `find()` was counted, or that
  lost its position (the `false` past an empty match at the end of the input),
  has kept its pattern, input and region, so `matches()` there is answered
  (the `lost()` rows and the `x*`/`a*` rows at the end of `rows()`). A matcher
  the table of followed matchers forgot (past 16,384 matchers) counts
  everything, `matches()` included, since a changing call may have come before.
  `regex_matcher.rs` has unit tests for these.
- `matches()` over a matcher on a `StringBuilder`'s text is answered from the
  text copied when the matcher was made (existing behaviour, not changed
  here): a later append shows in Java's matcher and not here.

**Not claimed:** the oracle has no row with a lone surrogate. A `String`
holding one (`"\ud800"`) is already U+FFFD inside the runtime, before any
`Matcher` is involved (an existing departure, not changed here), so
`group()` on an input that held a lone surrogate returns U+FFFD where Java
returns the surrogate, and the runtime cannot tell that text from a real
U+FFFD. `group()` over an input that may hold U+FFFD is therefore not shown
exact by these rows.

**Differential fuzz (not committed, not a gate).** The runtime and a JDK 25.0.5.1
`Matcher` were run on random and exhaustive call sequences (`find`, `matches`,
`group`, `group(1)`, `start`, `end`, `reset`, `groupCount`, `find(int)` with
fresh matchers) over 92 patterns and 38 inputs: 39,968 cases (seed 1) and
47,968 cases (seed 7). Every call the runtime answered equalled Java's:
120,313 and 153,367 answered calls, no difference; 38,571 and 41,776 calls were
counted. This is evidence, not an oracle row: only the rows above are in the
tree.

**Cost.** The compiled pattern and the input read as `char`s are kept per
matcher (256 matchers and 4 Mi `char`s of input at most, then emptied), so a
`while (m.find())` loop pays for each search, not for the length of the input
and the pattern's compile on every call. No row measures it. Measured on
2026-11-22 (fast profile, one machine, `while (m.find())` over `"ab"` repeated
n times, wall clock): before this change (e20997b8) 0.80 s for n = 20,000 and
10.85 s for n = 80,000; after, 0.12 s and 0.80 s, and 10.3 s for n = 320,000.
The search itself is linear now (`regex_matcher::tests::a_long_search_compiles_and_reads_the_input_once`
runs 40,000 `char`s; a throwaway run of the same loop for n = 5,000 / 20,000 /
80,000 took 8 / 31 / 128 ms in a debug build), but the whole loop through the
interpreter is still more than linear at n = 320,000; where that cost sits was
not found (the module's own part is the linear one above).

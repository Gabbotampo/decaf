# `string_case_mapping` fixture

Oracle probe: `demo/Probe.java` (package `demo`, class `Probe`), compiled with
`javac --release 21` and run on Java 25 (this machine: OpenJDK 25.0.5.1).

## Commands

```
javac --release 21 -d classes demo/Probe.java
timeout 120 java -Xmx512m -cp classes demo.Probe | head -c 2000000 > java25.txt
```

`java -version`:

```
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
```

The first 62 rows were first recorded on Temurin 25.0.4.1 and were
byte-identical on 25.0.5.1; the two null-`Locale` rows were added on 25.0.5.1.

The no-argument forms use the default locale. The oracle was run with the
default locale `it_IT`; under a `tr`/`az`/`lt` default, `"I".toLowerCase()` and
`"i".toUpperCase()` differ (run with `-Duser.language=tr`: `turkishIDotLower`
changes from `0069 0307` to `0069`). The runtime answers an ASCII receiver on
the no-argument forms as a non-tailored default locale would, as it did before
this change; see the PR description.

`java25.txt` is the RAW output of that final probe, one `<row>=<value>` line
per case. `<value>` is the result's length in UTF-16 units followed by each
unit as uppercase hexadecimal digits (e.g. `l1SharpUpper=2 0053 0053` for
`"ß".toUpperCase()` == `"SS"`).

## What each row covers

Answered exactly by the runtime (every answered row matches its oracle line
and records no unknown host call):

- `emptyUpper`, `emptyLower` — the empty string.
- `asciiLowerUpper`, `asciiLowerLower`, `asciiUpperUpper`, `asciiUpperLower`
  — pure ASCII (`"abc"`, `"ABC"`), both directions.
- `asciiRootLower`, `asciiRootUpper` — ASCII with `Locale.ROOT`.
- `charUpperSharp`, `charLowerSharp`, `charUpperE`, `charLowerE`,
  `charUpperAMacron`, `charLowerAMacron`, `charLowerSigma`, `charUpperSigma`,
  `charUpperIDotless`, `charLowerIDot`, `charUpperIDot`, `charLowerIDotless`,
  `charLowerKelvin`, `charUpperRoman`, `charLowerRoman` —
  `Character.toUpperCase`/`toLowerCase(char)` for `\u00DF`, `\u00E9`,
  `\u0100`, `\u03A3`/`\u03C3`, `\u0131`/`\u0130`, `\u212A`, `\u2160` (values are
  Java 25's own simple mappings, e.g. `charLowerKelvin=1 006B`).
- `intLowerKelvin`, `intUpperDeseret`, `intLowerDeseret`, `intUpperKelvin`,
  `intLowerRoman`, `intUpperRoman` — the same for `Character` of an `int`
  (`0x212A`, `0x10400`, `0x2160`).

Stayed counted (the runtime records the `String#toUpperCase`/`toLowerCase`
call as an unknown host call and never answers them):

- `l1EUpper`, `l1ELower`, `l1YenUpper`, `l1YenLower`, `l1SharpUpper`,
  `l1SharpLower`, `l1MicroUpper`, `l1MicroLower`, `leAMacronUpper`,
  `leAMacronLower` — Latin-1 (`\u00E9`, `\u00FF`, `\u00DF`, `\u00B5`) and Latin
  Extended (`\u0100`).
- `greekSigmaLower`, `greekSigmaUpper` — context-sensitive final sigma
  (`"A\u03A3".toLowerCase()` == `"a\u03C2`).
- `turkishIDotUpper`, `turkishIDotLower`, `turkishIDotlessUpper`,
  `turkishIDotlessLower` — Turkish dotted/dotless I with the no-arg method
  (default locale).
- `kelvinUpper`, `kelvinLower` — Kelvin sign `\u212A`.
- `romanUpper`, `romanLower` — Roman numeral `\u2160`.
- `deseretUpper`, `deseretLower` — supplementary `\uD800\uDC00` (U+10400).
- `loneLowSurrogateUpper`, `loneLowSurrogateLower`, `loneHighSurrogateUpper`,
  `loneHighSurrogateLower` — a lone surrogate receiver.
- `asciiTrLower`, `asciiTrUpper` — ASCII with the tailored `tr` locale
  (counted with all tailored locales).
- `idotRootLower`, `idotTrLower`, `idotlessRootUpper`, `idotlessTrUpper`,
  `sharpRootUpper` — non-ASCII receivers with `Locale.ROOT` or `tr`.
- `asciiNullLocaleUpper`, `asciiNullLocaleLower` — a null `Locale`; Java throws
  `NullPointerException` (the oracle line is `NullPointerException`; `main`
  catches it so the other rows still print). Only a `Locale` the runtime holds
  is answered; a null or unknown value is counted.

Every oracle line (64 rows) is covered by one of the two groups above; the
fixture asserts that.

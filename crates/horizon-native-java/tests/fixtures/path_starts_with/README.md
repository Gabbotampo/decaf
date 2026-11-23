Java 25 `java.nio.file.Path.startsWith(String)` / `startsWith(Path)` on Unix paths: every pair of 32 path
texts (relative, absolute, empty, root, repeated/trailing separators, `.`/`..` names, whole-name vs
partial-name prefixes), plus `normalize()` results tested against `".."` (the way mezz config's
`ConfigProvider` validates a mod directory), plus joined/resolved/parent paths. Only the exact rows are
modelled; a null argument, text with NUL, non-ASCII text and non-Path arguments stay counted.
Command: cd tests/fixtures/path_starts_with && J=/usr/lib/jvm/java-25-openjdk/bin && $J/javac --release 21 -d /tmp/psw demo/Probe.java && $J/java -cp /tmp/psw demo.Probe > java25.txt
Version is in version.txt (java25.txt is the raw output, never edited).

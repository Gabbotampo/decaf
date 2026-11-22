# ScheduledThreadPoolExecutor policy flags, Java 25 oracle

`java25.txt` is the raw output of `demo/Probe.java` on a real JDK 25 (never edited).
JDK only, no library jar. Probe by a JDK-only program (java.util.concurrent).

```sh
J=/usr/lib/jvm/java-25-openjdk/bin
$J/javac -d classes demo/Probe.java
$J/java -cp classes demo.Probe > java25.txt
```

```
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
```

Every row is deterministic on a real JVM: the only task is scheduled with a
zero delay and read through `Future.get()`, and `shutdown` runs with nothing
queued. The probe says nothing about a task still pending at `shutdown`, where
the flags decide whether it runs; the runtime runs a task when it is scheduled,
so that case has no oracle row and no modelled answer.

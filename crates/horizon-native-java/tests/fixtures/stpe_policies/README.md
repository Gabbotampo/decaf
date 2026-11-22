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

Every row is deterministic on a real JVM: the constructors' refusals are read
from the exception class and message, and the only task is scheduled with a zero
delay and read through `Future.get()`, and `shutdown` runs with nothing queued.
The probe says nothing about a task still pending at `shutdown` or at `cancel`,
where the flags decide whether it runs or leaves the queue; the runtime runs a
task when it is scheduled. So a `schedule` with a non-zero delay on a pool whose
policy a jar set is counted (an unknown host call), not answered: no oracle row
backs it (`fixture_stpe_policies.rs`, `delayed_schedule_on_a_policy_pool_is_counted`).
A pool no setter touched keeps the older answer (the task runs at once), which is
not shown exact either.

A pool only answers a setter or getter once its constructor passed the checks
the oracle shows (`ctor-*` rows: size first, then the factory, then the handler);
a constructor that refused leaves nothing to answer from.

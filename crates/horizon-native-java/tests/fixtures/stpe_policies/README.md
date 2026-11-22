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

Two probes, both JDK-only (java.util.concurrent), both committed:

```sh
$J/javac -d classes demo/Probe.java demo/Pending.java
$J/java -cp classes demo.Probe > java25.txt
$J/java -cp classes demo.Pending > java25-counted.txt
```

`java25.txt` (`Probe`) is what the runtime answers, and the test compares the
runtime's output to it byte for byte with an empty unknown-call list. Every row is
deterministic on a real JVM: the constructors' refusals are read from the
exception class and message, the policies from the getters, and `shutdown` runs
with nothing queued. It holds no task at all.

`java25-counted.txt` (`Pending`) is what the runtime does NOT answer. It records
Java 25's answers for a task run by a pool whose policy a jar set or read
(thread factory called, its thread name, a throwing task or factory, a null unit,
a task still pending at `shutdown`), and for a pool built with a handler after
`shutdown` (a discarding or CallerRuns handler gives no exception; Java calls the
handler, the runtime's pool never does). The runtime runs a task when it is
scheduled, in the caller's thread, and rejects after `shutdown` without a handler,
so it differs from these rows. Every `execute`, `submit` and `schedule` on a pool a
jar called a policy setter or getter on, and every one on a handler-built pool after
`shutdown`, is therefore an unknown host call, as the setter was upstream
(`fixture_stpe_policies.rs`, `tasks_on_a_policy_or_handler_pool_are_counted`: each
row must be counted; the runtime's own answer is not compared). A pool no policy
call touched keeps the older answers (the task runs at once), which are not shown
exact either.

A pool only answers a setter or getter once its constructor passed the checks
the oracle shows (`ctor-*` rows: size first, then the factory, then the handler);
a constructor that refused leaves nothing to answer from.

# `java.util.Objects` null-check messages and `checkIndex` family (JDK 25)

`demo/Probe.java` is the only class the test ships. Each `case_*` method
calls one `Objects` method with one set of operands and returns either the
result as text or `<exception simple name>:<message>`. `java25.txt` is the raw,
unedited output of `demo.Probe.main` on JDK 25, one `<label>=<text>` row per
method; the test reads each row's label and expected text from it.

Covered: `requireNonNull(Object)`, `(Object, String)`, `(Object, Supplier)`;
`requireNonNullElse`; `requireNonNullElseGet`; `checkIndex`,
`checkFromToIndex`, `checkFromIndexSize` (all `int` and `long` overloads),
including negative lengths and operands that would overflow `int` or `long`
arithmetic.

Commands, run in this directory with JDK 25:

```sh
$ java -version
openjdk version "25.0.5.1" 2026-11-17
OpenJDK Runtime Environment (build 25.0.5.1)
OpenJDK 64-Bit Server VM (build 25.0.5.1, mixed mode, sharing)
$ javac --release 21 -d . demo/Probe.java
$ java -cp . demo.Probe > java25.txt
```

Rows the runtime answers (text equal to `java25.txt`, nothing counted): every
row except the four below.

Rows that stay counted (unknown host call, count 1, no answer claimed). Each
needs a live `Supplier.get()` that only the interpreter can run:

| row | counted call |
|-----|--------------|
| `case_rn_supplier_null`, `case_rn_supplier_null_return` | `Objects#requireNonNull(Object, Supplier)` with a null value and a live supplier |
| `case_rneg_null_supplier`, `case_rneg_null_null_return` | `Objects#requireNonNullElseGet(Object, Supplier)` with a null value and a live supplier |

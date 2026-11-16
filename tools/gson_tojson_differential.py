#!/usr/bin/env python3
"""Manual Gson 2.14 Object-to-String differential over generated config objects."""
from __future__ import annotations

import argparse
import hashlib
import os
import subprocess
import tempfile
import zipfile
from pathlib import Path

STEPS = 1_000
SEQUENCE = r'''
  static long state = 0x243f6a8885a308d3L;
  static int next(int n) { state ^= state << 13; state ^= state >>> 7; state ^= state << 17; return (int)((state & Long.MAX_VALUE) % n); }
  static final class Nested {
    String label; int amount;
    Nested(String label, int amount) { this.label = label; this.amount = amount; }
  }
  static final class Config {
    String name; int count; boolean enabled; double ratio; String nullable;
    java.util.List<String> tags; java.util.Map<String,Integer> values; Nested nested;
    Config(int i) {
      name = "cfg-" + i; count = next(10000) - 5000; enabled = (i & 1) == 0;
      ratio = (next(2000) - 1000) / 10.0; nullable = (i % 3 == 0) ? null : "v" + next(64);
      tags = new java.util.ArrayList<>(); tags.add("a" + next(32)); tags.add("b" + next(32));
      values = new java.util.LinkedHashMap<>(); values.put("x", next(100)); values.put("y", next(100));
      nested = new Nested("n" + next(32), next(100));
    }
  }
  static void record(StringBuilder out, String text) { out.append(text).append("\n---\n"); }
  static String trace() {
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < __STEPS__; i++) {
      Config value = new Config(i);
      Gson[] gs = {
        new Gson(),
        new GsonBuilder().setPrettyPrinting().create(),
        new GsonBuilder().setPrettyPrinting().serializeNulls().create(),
        new GsonBuilder().setPrettyPrinting().serializeNulls().disableHtmlEscaping().create()
      };
      for (Gson gson : gs) record(out, i + ":" + gson.toJson(value) + "\n");
    }
    return out.toString();
  }
'''
SEQUENCE = SEQUENCE.replace("__STEPS__", str(STEPS))
GSON = """package com.google.gson;
public class Gson {
  public Gson() {}
  public String toJson(Object src) { return null; }
}
"""
BUILDER = """package com.google.gson;
public class GsonBuilder {
  public GsonBuilder() {}
  public GsonBuilder setPrettyPrinting() { return this; }
  public GsonBuilder serializeNulls() { return this; }
  public GsonBuilder disableHtmlEscaping() { return this; }
  public Gson create() { return new Gson(); }
}
"""
PROBE = """package demo;
import com.google.gson.*;
public final class Probe {
%s
}
""" % SEQUENCE
ORACLE = """import com.google.gson.*;
public final class GsonToJsonDifferential {
%s
  public static void main(String[] args) { System.out.print(trace()); }
}
""" % SEQUENCE

def javac_path() -> Path:
    javac = Path(os.environ.get("JAVA_HOME", "")) / "bin" / "javac"
    return javac if javac.exists() else Path("javac")

def compile_java(root: Path, sources: dict[str, str], classpath: str | None = None) -> None:
    src = root / "src"; classes = root / "classes"; classes.mkdir(parents=True)
    paths = []
    for name, text in sources.items():
        path = src / name; path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text); paths.append(path)
    cmd = [str(javac_path()), "--release", "25"]
    if classpath: cmd += ["-cp", classpath]
    subprocess.run(cmd + ["-d", str(classes), *map(str, paths)], check=True)

def make_probe_jar(root: Path) -> Path:
    compile_java(root, {"com/google/gson/Gson.java": GSON,
                        "com/google/gson/GsonBuilder.java": BUILDER,
                        "demo/Probe.java": PROBE})
    jar = root / "probe.jar"
    with zipfile.ZipFile(jar, "w", compression=zipfile.ZIP_STORED) as archive:
        for path in (root / "classes").rglob("*.class"):
            archive.write(path, path.relative_to(root / "classes").as_posix())
    return jar

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gson-jar", required=True, type=Path)
    parser.add_argument("--native-binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="gson-tojson-diff-") as tmp:
        root = Path(tmp)
        oracle_root = root / "oracle"
        compile_java(oracle_root, {"GsonToJsonDifferential.java": ORACLE}, str(args.gson_jar))
        oracle = subprocess.run(
            ["java", "-cp", f"{oracle_root / 'classes'}:{args.gson_jar}",
             "GsonToJsonDifferential"], check=True, text=True, capture_output=True
        ).stdout
        native = None
        if args.native_binary:
            native = subprocess.run(
                [str(args.native_binary), str(make_probe_jar(root / "native"))],
                check=True, text=True, capture_output=True
            ).stdout
    print(f"oracle objects={STEPS} sha256={hashlib.sha256(oracle.encode()).hexdigest()}")
    if native is not None:
        print(f"native objects={STEPS} sha256={hashlib.sha256(native.encode()).hexdigest()}")
        if native != oracle:
            print("DIFF: traces differ")
            for index, (left, right) in enumerate(zip(oracle.splitlines(), native.splitlines())):
                if left != right:
                    print(f"first mismatch line={index}")
                    print("oracle:", repr(left))
                    print("native:", repr(right))
                    break
            return 1
        print("MATCH")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

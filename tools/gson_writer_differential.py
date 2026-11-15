#!/usr/bin/env python3
"""Run a 50,000-step JsonWriter differential against Gson 2.14 and native.

This is a manual tool, deliberately outside CI. It always runs the real Gson
oracle. With --native-binary it also compiles a matching probe jar and invokes
the supplied native trace driver, comparing every trace line and exception.
"""
from __future__ import annotations
import argparse
import hashlib
import os
import subprocess
import tempfile
from pathlib import Path
import zipfile

STEPS = 50_000
SEQUENCE = r'''  static long state = 0x6a09e667f3bcc909L;
  static long digest;
  static int next(int n) { state ^= state << 13; state ^= state >>> 7; state ^= state << 17; return (int)((state & Long.MAX_VALUE) % n); }
  static void reset(StringWriter[] out, JsonWriter[] writer) { out[0] = new StringWriter(); writer[0] = new JsonWriter(out[0]); }
  static void record(String line) { digest = digest * 1315423911L + line.hashCode(); }
  public static String trace() throws Exception {
    StringWriter[] out = {new StringWriter()}; JsonWriter[] writer = {new JsonWriter(out[0])}; digest = 0;
    for (int i = 0; i < 50000; i++) {
      if (i != 0 && (i & 15) == 0) reset(out, writer);
      int op = next(16);
      try {
        switch (op) {
          case 0 -> writer[0].beginObject(); case 1 -> writer[0].endObject(); case 2 -> writer[0].beginArray(); case 3 -> writer[0].endArray();
          case 4 -> writer[0].name("k" + next(32)); case 5 -> writer[0].value(next(2) == 0); case 6 -> writer[0].value((long)next(1000) - 500);
          case 7 -> writer[0].value((double)next(1000) / 10.0 - 50.0); case 8 -> writer[0].value("v" + next(64)); case 9 -> writer[0].nullValue();
          case 10 -> writer[0].jsonValue(next(2) == 0 ? "[1,2]" : "null"); case 11 -> writer[0].setIndent(next(3) == 0 ? "" : (next(2) == 0 ? "  " : "\t"));
          case 12 -> writer[0].setSerializeNulls(next(2) == 0); case 13 -> writer[0].setHtmlSafe(next(2) == 0); case 14 -> writer[0].setLenient(next(2) == 0); default -> writer[0].flush();
        }
        record(i + ":" + op + ":" + out[0].toString() + "\n");
      } catch (Throwable t) {
        record(i + ":" + op + ":E:" + t.getClass().getName() + ":" + t.getMessage() + "\n"); reset(out, writer);
      }
    }
    return Long.toString(digest);
  }
'''
ORACLE = """import com.google.gson.stream.JsonWriter;
import java.io.StringWriter;
public final class GsonWriterDifferential {
%s  public static void main(String[] args) throws Exception { System.out.print(trace()); }
}
""" % SEQUENCE
NATIVE_WRITER = """package com.google.gson.stream;
public class JsonWriter {
  public JsonWriter(java.io.Writer out) {}
  public JsonWriter beginObject() { return this; }
  public JsonWriter endObject() { return this; }
  public JsonWriter beginArray() { return this; }
  public JsonWriter endArray() { return this; }
  public JsonWriter name(String name) { return this; }
  public JsonWriter value(String value) { return this; }
  public JsonWriter value(boolean value) { return this; }
  public JsonWriter value(double value) { return this; }
  public JsonWriter value(long value) { return this; }
  public JsonWriter nullValue() { return this; }
  public JsonWriter jsonValue(String value) { return this; }
  public void setIndent(String indent) {}
  public void setLenient(boolean value) {}
  public void setSerializeNulls(boolean value) {}
  public void setHtmlSafe(boolean value) {}
  public void flush() {}
}
"""
NATIVE_PROBE = """package demo;
import com.google.gson.stream.JsonWriter;
import java.io.StringWriter;
public final class Probe {
%s}
""" % SEQUENCE

def javac_path() -> Path:
    javac = Path(os.environ.get("JAVA_HOME", "")) / "bin" / "javac"
    return javac if javac.exists() else Path("javac")

def compile_java(root: Path, sources: dict[str, str], classpath: str | None = None) -> None:
    src = root / "src"; classes = root / "classes"; classes.mkdir(parents=True)
    files = []
    for name, text in sources.items():
        path = src / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(text); files.append(path)
    cmd = [str(javac_path()), "--release", "25"]
    if classpath: cmd += ["-cp", classpath]
    subprocess.run(cmd + ["-d", str(classes), *map(str, files)], check=True)

def make_probe_jar(root: Path) -> Path:
    compile_java(root, {"com/google/gson/stream/JsonWriter.java": NATIVE_WRITER, "demo/Probe.java": NATIVE_PROBE})
    jar = root / "native-probe.jar"
    with zipfile.ZipFile(jar, "w", compression=zipfile.ZIP_STORED) as archive:
        for path in (root / "classes").rglob("*.class"):
            archive.write(path, path.relative_to(root / "classes").as_posix())
    return jar

def digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gson-jar", required=True, type=Path)
    parser.add_argument("--native-binary", type=Path)
    parser.add_argument("--native-trace", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="gson-writer-diff-") as tmp:
        root = Path(tmp)
        oracle_root = root / "oracle"
        compile_java(oracle_root, {"GsonWriterDifferential.java": ORACLE}, str(args.gson_jar))
        result = subprocess.run(["java", "-cp", f"{oracle_root / 'classes'}:{args.gson_jar}", "GsonWriterDifferential"], check=True, text=True, capture_output=True)
        oracle = result.stdout
        native = None
        if args.native_binary:
            jar = make_probe_jar(root / "native")
            native = subprocess.run([str(args.native_binary), str(jar)], check=True, text=True, capture_output=True).stdout
        elif args.native_trace:
            native = args.native_trace.read_text()
    print(f"oracle steps={STEPS} sha256={digest(oracle)}")
    if native is not None:
        print(f"native steps={STEPS} sha256={digest(native)}")
        if native != oracle:
            print("DIFF: traces differ")
            return 1
        print("MATCH")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Run a 50,000-step JsonWriter trace against real Gson 2.14.0.

This is a manual differential tool, deliberately outside CI. Give it the
real Gson jar and optionally a native trace file produced by a matching probe.
"""
from __future__ import annotations
import argparse
import hashlib
import os
import subprocess
import tempfile
from pathlib import Path
STEPS = 50_000
JAVA = r'''import com.google.gson.stream.JsonWriter;
import java.io.StringWriter;
public final class GsonWriterDifferential {
  static long state = 0x6a09e667f3bcc909L;
  static int next(int n) { state ^= state << 13; state ^= state >>> 7; state ^= state << 17; return (int)((state & Long.MAX_VALUE) % n); }
  static void reset(StringWriter[] out, JsonWriter[] writer) { out[0] = new StringWriter(); writer[0] = new JsonWriter(out[0]); }
  public static void main(String[] args) throws Exception {
    StringWriter[] out = {new StringWriter()}; JsonWriter[] writer = {new JsonWriter(out[0])}; StringBuilder trace = new StringBuilder();
    for (int i = 0; i < 50000; i++) {
      int op = next(16);
      try {
        switch (op) {
          case 0 -> writer[0].beginObject(); case 1 -> writer[0].endObject(); case 2 -> writer[0].beginArray(); case 3 -> writer[0].endArray();
          case 4 -> writer[0].name("k" + next(32)); case 5 -> writer[0].value(next(2) == 0); case 6 -> writer[0].value((long)next(1000) - 500);
          case 7 -> writer[0].value((double)next(1000) / 10.0 - 50.0); case 8 -> writer[0].value("v" + next(64)); case 9 -> writer[0].nullValue();
          case 10 -> writer[0].jsonValue(next(2) == 0 ? "[1,2]" : "null"); case 11 -> writer[0].setIndent(next(3) == 0 ? "" : (next(2) == 0 ? "  " : "\t"));
          case 12 -> writer[0].setSerializeNulls(next(2) == 0); case 13 -> writer[0].setHtmlSafe(next(2) == 0); case 14 -> writer[0].setLenient(next(2) == 0); default -> writer[0].flush();
        }
        trace.append(i).append(':').append(op).append(':').append(out[0].toString()).append('\n');
      } catch (Throwable t) {
        trace.append(i).append(':').append(op).append(":E:").append(t.getClass().getName()).append(':').append(t.getMessage()).append('\n'); reset(out, writer);
      }
    }
    System.out.print(trace);
  }
}
'''
def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gson-jar", required=True, type=Path)
    parser.add_argument("--native-trace", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="gson-writer-diff-") as tmp:
        root = Path(tmp); source = root / "GsonWriterDifferential.java"; source.write_text(JAVA)
        javac = Path(os.environ.get("JAVA_HOME", "")) / "bin" / "javac"
        if not javac.exists(): javac = Path("javac")
        subprocess.run([str(javac), "--release", "25", "-cp", str(args.gson_jar), "-d", str(root), str(source)], check=True)
        result = subprocess.run(["java", "-cp", f"{root}:{args.gson_jar}", "GsonWriterDifferential"], check=True, text=True, capture_output=True)
        trace = result.stdout
    digest = hashlib.sha256(trace.encode()).hexdigest(); print(f"oracle steps={STEPS} sha256={digest}")
    if args.native_trace:
        native = args.native_trace.read_text(); print(f"native sha256={hashlib.sha256(native.encode()).hexdigest()}")
        if native != trace: print("DIFF: traces differ"); return 1
        print("MATCH")
    return 0
if __name__ == "__main__": raise SystemExit(main())

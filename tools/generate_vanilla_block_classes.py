#!/usr/bin/env python3
"""Generate the vanilla block closure table from a player's own 26.3 jars.

No generated JSON is checked into horizon-java.  The resulting file is passed
to the native host/census with --game-block-classes.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server-jar", type=Path, required=True)
    parser.add_argument("--libraries-dir", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    java = shutil.which("java")
    javac = shutil.which("javac")
    if not java or not javac:
        parser.error("Java 25 java/javac are required")
    libraries = sorted(
        jar for directory in args.libraries_dir
        for jar in directory.rglob("*.jar")
        if jar.is_file()
    )
    classpath = ":".join(str(path) for path in [args.server_jar, *libraries])
    source = Path(__file__).with_name("VanillaBlockClassClosureExporter.java")
    with tempfile.TemporaryDirectory(prefix="vanilla-block-classes-") as temporary:
        classes = Path(temporary) / "classes"
        classes.mkdir()
        subprocess.run(
            [javac, "--release", "25", "-cp", classpath, "-d", str(classes), str(source)],
            check=True,
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            [java, "-cp", ":".join((classpath, str(classes))),
             "dev.horizonlab.parity.VanillaBlockClassClosureExporter", str(args.output)],
            check=True,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

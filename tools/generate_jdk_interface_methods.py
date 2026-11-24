#!/usr/bin/env python3
"""Generate `crates/horizon-native-java/src/jdk_interface_methods_25.tsv` from a JDK 25 runtime.

The table is what `jvm_dispatch.rs` needs to run JVMS §5.4.3.3 / §5.4.6 default-method
selection over the JDK interfaces a jar class reaches (#627): every interface's direct
superinterfaces, in declaration order, and the methods it declares with their kind
(`abstract`, `default`, `static`, `private`); and, for the JDK superclasses whose own
declarations `jvm_dispatch::jdk_superclass_declares` knows, their superclass and direct
interfaces. Only names, descriptors and flags are recorded: no code, no values.

Which interfaces
----------------
Every public, non-sealed interface (`ACC_PUBLIC | ACC_INTERFACE` on the class file, no
`PermittedSubclasses`; nested ones included) of every package `java.base` exports to
everyone (`--module` adds modules). A class-path class can implement only an exported
package's public interfaces, never a sealed one that does not permit it, and
`java.base` is the module every class reads; its packages hold the interfaces mods
implement (`java.util`, `java.util.function`, `java.lang`, `java.io`). Then their
superinterfaces, transitively, wherever they live, so every row's parents have rows.
A platform interface without a row (a game or loader interface, which no jar ships, or
one of another JDK module such as `java.awt.event.MouseListener`) is unknown to the
runtime, and a selection that depends on it is counted, never guessed.

`HotSpot`'s order matters: the conflict message lists the members in the order its
depth-first walk meets them (`defaultMethods.cpp`), so superinterfaces stay in
`getInterfaces()` order, which is the class file's.

Which methods
-------------
Every method an interface's class file declares but `<clinit>` and the synthetic
private/static ones (lambda bodies, `$deserializeLambda$`). Synthetic public methods
(bridges) stay: a bridge default takes part in selection like any default.

Reproducing the committed TSV byte-for-byte
-------------------------------------------
Pass the JDK the table records (Eclipse Temurin 25.0.5+8)::

    python3 tools/generate_jdk_interface_methods.py \\
        --jdk <JDK_25_HOME> \\
        --output crates/horizon-native-java/src/jdk_interface_methods_25.tsv

Then re-vendor the copy in `mod/src/runtime/`::

    python3 tools/vendor.py && python3 tools/vendor.py --check

`tools/generate_vanilla_host_methods.py` (#622) reads JDK interfaces the same way for
another table; the two generators are deliberately separate files.
"""

from __future__ import annotations

import argparse
import os
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

ACC_PUBLIC = 0x0001
ACC_PRIVATE = 0x0002
ACC_STATIC = 0x0008
ACC_SYNTHETIC = 0x1000
ACC_INTERFACE = 0x0200
ACC_ABSTRACT = 0x0400

# The JDK superclasses whose own declarations `jvm_dispatch::jdk_superclass_declares`
# answers: `Object`, `Record`, `Enum` and `jvm_dispatch::JDK_SKELETONS`. A unit test in
# `jvm_dispatch.rs` keeps the two lists equal.
KNOWN_SUPERCLASSES = [
    "java/lang/Enum",
    "java/lang/Object",
    "java/lang/Record",
    "java/util/AbstractCollection",
    "java/util/AbstractList",
    "java/util/AbstractMap",
    "java/util/AbstractMap$SimpleEntry",
    "java/util/AbstractMap$SimpleImmutableEntry",
    "java/util/AbstractQueue",
    "java/util/AbstractSequentialList",
    "java/util/AbstractSet",
]

# Writes the JDK's identity, then every class file of the packages the modules named in
# its arguments export to everyone (internal name and bytes), then the bytes of each class
# named on stdin (a superinterface in a package those modules do not export).
JRT_READER_JAVA = r"""import java.io.*;
import java.lang.module.*;
import java.net.URI;
import java.nio.file.*;
import java.util.*;

public class JdkInterfaceReader {
    public static void main(String[] args) throws Exception {
        FileSystem fs = FileSystems.getFileSystem(URI.create("jrt:/"));
        DataOutputStream out = new DataOutputStream(new BufferedOutputStream(System.out));
        out.writeUTF(System.getProperty("java.vm.vendor") + " " + System.getProperty("java.runtime.version"));
        Set<String> wanted = new HashSet<>(Arrays.asList(args));
        List<String> exported = new ArrayList<>();
        for (ModuleReference ref : ModuleFinder.ofSystem().findAll()) {
            ModuleDescriptor descriptor = ref.descriptor();
            if (!wanted.contains(descriptor.name())) continue;
            for (ModuleDescriptor.Exports exports : descriptor.exports()) {
                if (!exports.isQualified()) {
                    exported.add(descriptor.name() + "/" + exports.source().replace('.', '/'));
                }
            }
        }
        Collections.sort(exported);
        List<String> listed = new ArrayList<>();
        for (String entry : exported) {
            Path dir = fs.getPath("/modules", entry);
            if (!Files.isDirectory(dir)) continue;
            try (var files = Files.list(dir)) {
                files.map(Path::toString)
                    .filter(name -> name.endsWith(".class"))
                    .forEach(listed::add);
            }
        }
        Collections.sort(listed);
        out.writeInt(listed.size());
        for (String name : listed) {
            // "/modules/<module>/<internal name>.class" -> the class bytes.
            byte[] bytes = Files.readAllBytes(fs.getPath(name));
            String rest = name.substring("/modules/".length());
            out.writeUTF(rest.substring(rest.indexOf('/') + 1, rest.length() - ".class".length()));
            out.writeInt(bytes.length);
            out.write(bytes);
        }
        // Named classes outside the exported packages (a superinterface in a hidden package).
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        List<Path> modules = new ArrayList<>();
        try (var s = Files.list(fs.getPath("/modules"))) {
            s.forEach(modules::add);
        }
        modules.sort(Comparator.comparing(Path::toString));
        for (String name; (name = in.readLine()) != null && !name.isEmpty(); ) {
            byte[] found = null;
            for (Path module : modules) {
                Path p = module.resolve(name + ".class");
                if (Files.isRegularFile(p)) {
                    found = Files.readAllBytes(p);
                    break;
                }
            }
            out.writeUTF(name);
            out.writeInt(found == null ? -1 : found.length);
            if (found != null) out.write(found);
        }
        out.flush();
    }
}
"""


class ClassInfo:
    """The parts of one class file the table records."""

    def __init__(self, access: int, name: str, super_name: str, interfaces: list[str],
                 methods: list[tuple[int, str, str]], sealed: bool):
        self.access = access
        self.name = name
        self.super_name = super_name
        self.interfaces = interfaces
        self.methods = methods
        # A `PermittedSubclasses` attribute: only the classes it lists may implement it.
        self.sealed = sealed

    @property
    def is_interface(self) -> bool:
        return bool(self.access & ACC_INTERFACE)


def parse_class(data: bytes) -> ClassInfo:
    """Read access flags, names, interfaces and methods (`(access, name, descriptor)`, in
    class-file order) from a class file."""
    if struct.unpack_from(">I", data, 0)[0] != 0xCAFEBABE:
        raise ValueError("not a class file")
    off = 8
    cp_count = struct.unpack_from(">H", data, off)[0]
    off += 2
    cp: list[object] = [None] * cp_count
    i = 1
    while i < cp_count:
        tag = data[off]
        off += 1
        if tag == 1:
            length = struct.unpack_from(">H", data, off)[0]
            off += 2
            # Modified UTF-8; the JDK's names here are plain.
            cp[i] = data[off : off + length].decode("utf-8", "replace")
            off += length
        elif tag in (3, 4):
            off += 4
        elif tag in (5, 6):
            off += 8
            i += 1
        elif tag in (7, 8, 16, 19, 20):
            cp[i] = struct.unpack_from(">H", data, off)[0]
            off += 2
        elif tag in (9, 10, 11, 12, 17, 18):
            off += 4
        elif tag == 15:
            off += 3
        else:
            raise ValueError(f"unsupported constant pool tag {tag}")
        i += 1

    def class_name(index: int) -> str:
        return str(cp[int(cp[index])]) if index else ""

    access, this_c, super_c = struct.unpack_from(">HHH", data, off)
    off += 6
    count = struct.unpack_from(">H", data, off)[0]
    off += 2
    interfaces = []
    for _ in range(count):
        interfaces.append(class_name(struct.unpack_from(">H", data, off)[0]))
        off += 2

    def read_members() -> list[tuple[int, str, str]]:
        nonlocal off
        members = []
        member_count = struct.unpack_from(">H", data, off)[0]
        off += 2
        for _ in range(member_count):
            m_access, m_name, m_desc, attributes = struct.unpack_from(">HHHH", data, off)
            off += 8
            for _ in range(attributes):
                off += 2
                off += 4 + struct.unpack_from(">I", data, off)[0]
            members.append((m_access, str(cp[m_name]), str(cp[m_desc])))
        return members

    read_members()  # fields
    methods = read_members()
    sealed = False
    for _ in range(struct.unpack_from(">H", data, off)[0]):
        name_index, length = struct.unpack_from(">HI", data, off + 2)
        off += 6 + length
        sealed = sealed or cp[name_index] == "PermittedSubclasses"
    return ClassInfo(access, class_name(this_c), class_name(super_c), interfaces, methods, sealed)


def method_kind(access: int) -> str | None:
    """The table's kind for an interface method, `None` to leave it out."""
    if access & ACC_SYNTHETIC and access & (ACC_PRIVATE | ACC_STATIC):
        return None
    if access & ACC_STATIC:
        return "static"
    if access & ACC_PRIVATE:
        return "private"
    if access & ACC_ABSTRACT:
        return "abstract"
    return "default"


def find_java_bin(jdk: Path | None) -> Path:
    if jdk is not None:
        candidate = jdk / "bin" / "java"
        if candidate.is_file():
            return candidate
        raise SystemExit(f"--jdk does not contain bin/java: {jdk}")
    java_home = os.environ.get("JAVA_HOME")
    if java_home and (Path(java_home) / "bin" / "java").is_file():
        return Path(java_home) / "bin" / "java"
    which = shutil.which("java")
    if which:
        return Path(which)
    raise SystemExit("could not locate a JDK `java` binary; pass --jdk <path>")


def read_jdk(
    java_bin: Path, modules: list[str], wanted: list[str]
) -> tuple[str, dict[str, bytes], dict[str, bytes]]:
    """Run the reader: the JDK's identity, the bytes of every class `modules` export, and
    the bytes of `wanted` (classes asked for by name)."""
    with tempfile.TemporaryDirectory() as td:
        source = Path(td) / "JdkInterfaceReader.java"
        source.write_text(JRT_READER_JAVA, encoding="utf-8")
        proc = subprocess.run(
            [str(java_bin), str(source), *modules],
            input=("\n".join(wanted) + "\n\n").encode(),
            check=True,
            capture_output=True,
        )
    buf = proc.stdout
    off = 0

    def utf() -> str:
        nonlocal off
        length = struct.unpack_from(">H", buf, off)[0]
        off += 2
        text = buf[off : off + length].decode("utf-8")
        off += length
        return text

    def blob() -> bytes | None:
        nonlocal off
        length = struct.unpack_from(">i", buf, off)[0]
        off += 4
        if length < 0:
            return None
        data = buf[off : off + length]
        off += length
        return data

    identity = utf()
    exported: dict[str, bytes] = {}
    count = struct.unpack_from(">i", buf, off)[0]
    off += 4
    for _ in range(count):
        name = utf()
        data = blob()
        assert data is not None
        exported[name] = data
    named: dict[str, bytes] = {}
    while off < len(buf):
        name = utf()
        data = blob()
        if data is not None:
            named[name] = data
    return identity, exported, named


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jdk", type=Path, default=None, help="JDK 25 home, read via jrt:/")
    parser.add_argument(
        "--module",
        action="append",
        default=None,
        help="a system module whose exported interfaces seed the table (default: java.base)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("crates/horizon-native-java/src/jdk_interface_methods_25.tsv"),
    )
    args = parser.parse_args()
    java_bin = find_java_bin(args.jdk)

    modules = sorted(set(args.module or ["java.base"]))
    parsed: dict[str, ClassInfo] = {}
    identity, exported, _ = read_jdk(java_bin, modules, [])
    for name, data in exported.items():
        parsed[name] = parse_class(data)
    seeds = sorted(
        name
        for name, info in parsed.items()
        if info.is_interface and info.access & ACC_PUBLIC and not info.sealed
    )
    missing = [name for name in KNOWN_SUPERCLASSES if name not in parsed]
    if missing:
        raise SystemExit(f"known superclasses missing from the exported packages: {missing}")

    # The closure: every superinterface of a seed or of a known superclass, wherever it
    # lives. A hidden package's class is read by name in a second pass.
    rows: dict[str, ClassInfo] = {}
    pending = list(seeds) + [
        iface for name in KNOWN_SUPERCLASSES for iface in parsed[name].interfaces
    ]
    while pending:
        unread = sorted({name for name in pending if name not in parsed})
        if unread:
            _, _, named = read_jdk(java_bin, modules, unread)
            for name in unread:
                if name not in named:
                    raise SystemExit(f"superinterface not in the JDK: {name}")
                parsed[name] = parse_class(named[name])
        next_pending = []
        for name in pending:
            if name in rows:
                continue
            info = parsed[name]
            if not info.is_interface:
                raise SystemExit(f"{name} is listed as an interface but is a class")
            rows[name] = info
            next_pending.extend(info.interfaces)
        pending = next_pending

    kind_letter = {"abstract": "a", "default": "d", "static": "s", "private": "p"}
    lines = [
        f"# {identity} interfaces of {', '.join(modules)} and their superinterfaces, for"
        " default-method selection (#627). Names and flags only; no code.",
        "# Generated by tools/generate_jdk_interface_methods.py; never edit by hand.",
        "# C<tab>class<tab>superclass<tab>interface,...   (a known JDK superclass; getInterfaces() order)",
        "# I<tab>interface<tab>superinterface,...<tab>method method ...   (getInterfaces() order)",
        "#   a method is <kind><name><descriptor>, kind a(bstract) d(efault) s(tatic) p(rivate),",
        "#   in class-file order",
    ]
    for name in KNOWN_SUPERCLASSES:
        info = parsed[name]
        lines.append(f"C\t{name}\t{info.super_name}\t{','.join(info.interfaces)}")
    methods = 0
    for name in sorted(rows):
        declared = []
        for access, method, descriptor in rows[name].methods:
            kind = method_kind(access)
            if method == "<clinit>" or kind is None:
                continue
            declared.append(f"{kind_letter[kind]}{method}{descriptor}")
        methods += len(declared)
        lines.append(f"I\t{name}\t{','.join(rows[name].interfaces)}\t{' '.join(declared)}")
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {args.output}: {len(rows)} interfaces, {methods} methods ({identity})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

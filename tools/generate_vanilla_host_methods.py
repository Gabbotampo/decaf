#!/usr/bin/env python3
"""Generate `crates/horizon-native-java/src/vanilla_host_methods.tsv` from 26.3 jars.

Only class/interface names, hierarchy edges, non-private non-static method
`(name, descriptor)` signatures and every declared field's `(name, descriptor)`
are recorded — no code, no values. The runtime uses this table to enforce JVMS
§5.4.3.3 when an interface-injection Mixin adds an interface with a `default`
method onto a host-modelled vanilla class: any method already declared by the
target class hierarchy (or a vanilla interface default on it) wins over the
injected interface's `default` method and routes through host dispatch
(answering when modelled, or counting/refusing when not). The field rows let it
tell when a mixin's own static field, which Mixin merges onto the game class,
collides with a field that class already declares (Mixin renames, discards or
rejects it then, which the runtime counts instead of modelling).

Reproducing the committed TSV byte-for-byte
-------------------------------------------
Pass ONLY the unmodified Mojang vanilla artifacts (not a Loom/Fabric-woven jar
with injected interfaces like `FabricRenderPipeline`) plus a JDK 25 runtime:

1. Mojang vanilla Minecraft 26.3 client jar (`26.3.jar`, protocol 777,
   `version.json` id `"26.3"` — Mojang's official unobfuscated client jar,
   which includes `net/minecraft/*` and `com/mojang/renderpearl/*`).
2. `com.mojang:authlib:10.0.77` (`authlib-10.0.77.jar` — from Minecraft 26.3's
   official library manifest, supplying `com/mojang/authlib/GameProfile`).
3. JDK 25 runtime (`--jdk <JDK_25_HOME>`, e.g. Eclipse Temurin `25.0.5+8`;
   read via `jrt:/`, supplying `java/lang/AutoCloseable`,
   `java/lang/Comparable`, `java/lang/Iterable`, `java/lang/Record`, and
   `java/util/concurrent/Executor`).

Command::

    python3 tools/generate_vanilla_host_methods.py \\
        --jar <MINECRAFT_DIR>/versions/26.3/26.3.jar \\
        --jar <MINECRAFT_DIR>/libraries/com/mojang/authlib/10.0.77/authlib-10.0.77.jar \\
        --jdk <JDK_25_HOME> \\
        --output crates/horizon-native-java/src/vanilla_host_methods.tsv

Then re-vendor the copy in `mod/src/runtime/`::

    python3 tools/vendor.py && python3 tools/vendor.py --check
"""

from __future__ import annotations

import argparse
import os
import shutil
import struct
import subprocess
import tempfile
import zipfile
from pathlib import Path

HOST_MODELLED_GAME_TARGETS = [
    "com/mojang/authlib/GameProfile",
    "com/mojang/blaze3d/platform/InputConstants$Key",
    "com/mojang/renderpearl/api/pipeline/RenderPipeline",
    "net/minecraft/client/KeyMapping$Category",
    "net/minecraft/client/Minecraft",
    "net/minecraft/client/Options",
    "net/minecraft/client/gui/Font",
    "net/minecraft/client/gui/Gui",
    "net/minecraft/client/multiplayer/ClientLevel",
    "net/minecraft/client/player/AbstractClientPlayer",
    "net/minecraft/client/player/LocalPlayer",
    "net/minecraft/commands/Commands",
    "net/minecraft/core/BlockPos",
    "net/minecraft/core/component/DataComponentGetter",
    "net/minecraft/core/component/DataComponentHolder",
    "net/minecraft/core/component/DataComponentType",
    "net/minecraft/network/chat/Style",
    "net/minecraft/network/chat/TextColor",
    "net/minecraft/server/MinecraftServer",
    "net/minecraft/server/dedicated/DedicatedServer",
    "net/minecraft/server/level/ServerLevel",
    "net/minecraft/server/level/ServerPlayer",
    "net/minecraft/server/players/PlayerList",
    "net/minecraft/sounds/SoundEvent",
    "net/minecraft/world/Container",
    "net/minecraft/world/Nameable",
    "net/minecraft/world/damagesource/DamageSource",
    "net/minecraft/world/effect/MobEffect",
    "net/minecraft/world/entity/Avatar",
    "net/minecraft/world/entity/Entity",
    "net/minecraft/world/entity/LivingEntity",
    "net/minecraft/world/entity/ai/attributes/Attribute",
    "net/minecraft/world/entity/item/ItemEntity",
    "net/minecraft/world/entity/player/Inventory",
    "net/minecraft/world/entity/player/Player",
    "net/minecraft/world/item/Item",
    "net/minecraft/world/item/ItemInstance",
    "net/minecraft/world/item/ItemStack",
    "net/minecraft/world/item/crafting/RecipeAccess",
    "net/minecraft/world/item/crafting/RecipeManager",
    "net/minecraft/world/level/ItemLike",
    "net/minecraft/world/level/Level",
    "net/minecraft/world/level/block/Block",
    "net/minecraft/world/level/block/entity/BlockEntityType",
    "net/minecraft/world/level/block/state/BlockState",
    "net/minecraft/world/level/border/WorldBorder",
    "net/minecraft/world/level/gamerules/GameRule",
    "net/minecraft/world/level/storage/LevelResource",
    "net/minecraft/world/phys/Vec3",
]

ACC_PRIVATE = 0x0002
ACC_STATIC = 0x0008
ACC_INTERFACE = 0x0200
ACC_ABSTRACT = 0x0400


def parse_class(
    data: bytes,
) -> tuple[bool, str, list[str], list[tuple[str, str]], list[tuple[str, str]]]:
    magic = struct.unpack_from(">I", data, 0)[0]
    if magic != 0xCAFEBABE:
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
            ln = struct.unpack_from(">H", data, off)[0]
            off += 2
            cp[i] = data[off : off + ln].decode("utf-8", "replace")
            off += ln
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
    access, _this_c, super_c = struct.unpack_from(">HHH", data, off)
    off += 6
    super_name = str(cp[int(cp[super_c])]) if super_c else ""
    ifaces_n = struct.unpack_from(">H", data, off)[0]
    off += 2
    ifaces: list[str] = []
    for _ in range(ifaces_n):
        ic = struct.unpack_from(">H", data, off)[0]
        off += 2
        ifaces.append(str(cp[int(cp[ic])]))
    fcount = struct.unpack_from(">H", data, off)[0]
    off += 2
    fields: set[tuple[str, str]] = set()
    for _ in range(fcount):
        _fa, fn, fd = struct.unpack_from(">HHH", data, off)
        off += 6
        fields.add((str(cp[fn]), str(cp[fd])))
        ac = struct.unpack_from(">H", data, off)[0]
        off += 2
        for _ in range(ac):
            off += 2
            al = struct.unpack_from(">I", data, off)[0]
            off += 4 + al
    mcount = struct.unpack_from(">H", data, off)[0]
    off += 2
    is_iface = bool(access & ACC_INTERFACE)
    methods: set[tuple[str, str]] = set()
    for _ in range(mcount):
        ma, mn, md, ac = struct.unpack_from(">HHHH", data, off)
        off += 8
        for _ in range(ac):
            off += 2
            al = struct.unpack_from(">I", data, off)[0]
            off += 4 + al
        name = str(cp[mn])
        desc = str(cp[md])
        if name.startswith("<") or (ma & (ACC_PRIVATE | ACC_STATIC)):
            continue
        # On a class: any non-private, non-static method (concrete or abstract) blocks
        # inheriting an interface default under JVMS §5.4.3.3.
        # On an interface: only concrete (`default`) methods can shadow/conflict with one.
        if not is_iface or not (ma & ACC_ABSTRACT):
            methods.add((name, desc))
    return is_iface, super_name, ifaces, sorted(methods), sorted(fields)


JRT_READER_JAVA = """import java.io.*;
import java.net.URI;
import java.nio.file.*;
import java.util.*;

public class JrtClassReader {
    public static void main(String[] args) throws Exception {
        FileSystem fs = FileSystems.getFileSystem(URI.create("jrt:/"));
        List<Path> modDirs = new ArrayList<>();
        try (var s = Files.list(fs.getPath("/modules"))) {
            s.forEach(modDirs::add);
        }
        modDirs.sort(Comparator.comparing(Path::toString));
        DataOutputStream out = new DataOutputStream(new BufferedOutputStream(System.out));
        for (String cname : args) {
            byte[] found = null;
            for (Path mod : modDirs) {
                Path p = mod.resolve(cname + ".class");
                if (Files.isRegularFile(p)) {
                    found = Files.readAllBytes(p);
                    break;
                }
            }
            out.writeUTF(cname);
            if (found == null) {
                out.writeInt(-1);
            } else {
                out.writeInt(found.length);
                out.write(found);
            }
        }
        out.flush();
    }
}
"""


def find_java_bin(jdk: Path | None) -> Path:
    if jdk is not None:
        candidate = jdk / "bin" / "java"
        if candidate.is_file():
            return candidate
        raise SystemExit(f"--jdk does not contain bin/java: {jdk}")
    java_home = os.environ.get("JAVA_HOME")
    if java_home:
        candidate = Path(java_home) / "bin" / "java"
        if candidate.is_file():
            return candidate
    which = shutil.which("java")
    if which:
        return Path(which)
    raise SystemExit("could not locate JDK `java` binary; pass --jdk <path>")


def read_jrt_classes(java_bin: Path, names: list[str]) -> dict[str, bytes]:
    if not names:
        return {}
    with tempfile.TemporaryDirectory() as td:
        src = Path(td) / "JrtClassReader.java"
        src.write_text(JRT_READER_JAVA, encoding="utf-8")
        proc = subprocess.run(
            [str(java_bin), str(src), *names],
            check=True,
            capture_output=True,
        )
    buf = proc.stdout
    off = 0
    found: dict[str, bytes] = {}
    while off < len(buf):
        nlen = struct.unpack_from(">H", buf, off)[0]
        off += 2
        cname = buf[off : off + nlen].decode("utf-8")
        off += nlen
        clen = struct.unpack_from(">i", buf, off)[0]
        off += 4
        if clen >= 0:
            found[cname] = buf[off : off + clen]
            off += clen
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jar", type=Path, action="append", required=True)
    parser.add_argument(
        "--jdk",
        type=Path,
        default=None,
        help="Path to the JDK 25 home used to resolve platform superclasses/interfaces via jrt:/",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("crates/horizon-native-java/src/vanilla_host_methods.tsv"),
    )
    args = parser.parse_args()
    java_bin = find_java_bin(args.jdk)

    zips = [zipfile.ZipFile(path) for path in args.jar]
    try:
        entries: dict[str, tuple[zipfile.ZipFile, str]] = {}
        for z in zips:
            for name in z.namelist():
                if name.endswith(".class") and name not in entries:
                    entries[name] = (z, name)

        parsed: dict[
            str,
            tuple[bool, str, list[str], list[tuple[str, str]], list[tuple[str, str]]],
        ] = {}
        pending: set[str] = set()
        unresolved: set[str] = set()

        def enqueue(cname: str) -> None:
            if not cname or cname == "java/lang/Object" or cname in parsed or cname in unresolved:
                return
            pending.add(cname)

        for target in HOST_MODELLED_GAME_TARGETS:
            enqueue(target)

        while pending:
            batch = sorted(pending)
            pending.clear()
            jrt_needed: list[str] = []
            for cname in batch:
                hit = entries.get(cname + ".class")
                if hit is not None:
                    z, entry = hit
                    parsed[cname] = parse_class(z.read(entry))
                    sup, ifaces = parsed[cname][1], parsed[cname][2]
                    enqueue(sup)
                    for iface in ifaces:
                        enqueue(iface)
                else:
                    jrt_needed.append(cname)
            if jrt_needed:
                jrt_bytes = read_jrt_classes(java_bin, jrt_needed)
                for cname in jrt_needed:
                    data = jrt_bytes.get(cname)
                    if data is None:
                        unresolved.add(cname)
                        continue
                    parsed[cname] = parse_class(data)
                    sup, ifaces = parsed[cname][1], parsed[cname][2]
                    enqueue(sup)
                    for iface in ifaces:
                        enqueue(iface)

        if unresolved:
            raise SystemExit(
                f"unresolved hierarchy types (add missing jars or check --jdk): {sorted(unresolved)}"
            )

        missing_targets = [t for t in HOST_MODELLED_GAME_TARGETS if t not in parsed]
        if missing_targets:
            raise SystemExit(f"missing HOST_MODELLED_GAME_TARGETS: {missing_targets}")

        lines = [
            "# Vanilla 26.3 + authlib 10.0.77 + JDK 25 target hierarchy and method signatures (names only; no code).",
            "# Generated by tools/generate_vanilla_host_methods.py.",
            "# C<tab>class<tab>super<tab>iface1,iface2,...",
            "# I<tab>interface<tab><tab>super_iface1,...",
            "# M<tab>owner<tab>name<tab>descriptor",
            "# F<tab>owner<tab>name<tab>descriptor (every declared field)",
        ]
        for cname in sorted(parsed):
            is_i, sup, ifaces, _methods, _fields = parsed[cname]
            if is_i:
                lines.append(f"I\t{cname}\t\t{','.join(ifaces)}")
            else:
                lines.append(f"C\t{cname}\t{sup}\t{','.join(ifaces)}")
        for cname in sorted(parsed):
            _is_i, _sup, _ifaces, methods, _fields = parsed[cname]
            for name, desc in methods:
                lines.append(f"M\t{cname}\t{name}\t{desc}")
        for cname in sorted(parsed):
            for name, desc in parsed[cname][4]:
                lines.append(f"F\t{cname}\t{name}\t{desc}")

        args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(
            f"wrote {args.output}: {len(parsed)} types, "
            f"{sum(len(v[3]) for v in parsed.values())} method signatures, "
            f"{sum(len(v[4]) for v in parsed.values())} fields"
        )
        return 0
    finally:
        for z in zips:
            z.close()


if __name__ == "__main__":
    raise SystemExit(main())

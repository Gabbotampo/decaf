#!/usr/bin/env python3
"""Generate `crates/horizon-native-java/src/vanilla_host_methods.tsv` from 26.3 jars.

Only class/interface names, hierarchy edges, and non-private non-static method
`(name, descriptor)` signatures are recorded — no code. The runtime uses this
table to enforce JVMS §5.4.3.3 when an interface-injection Mixin adds an
interface with a `default` method onto a host-modelled vanilla class: any
method already declared by the target class hierarchy (or a vanilla interface
default on it) wins over the injected interface's `default` method and routes
through host dispatch (answering when modelled, or counting/refusing when not).
"""

from __future__ import annotations

import argparse
import struct
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


def parse_class(data: bytes) -> tuple[bool, str, list[str], list[tuple[str, str]]]:
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
    for _ in range(fcount):
        off += 6
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
    return is_iface, super_name, ifaces, sorted(methods)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jar", type=Path, action="append", required=True)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("crates/horizon-native-java/src/vanilla_host_methods.tsv"),
    )
    args = parser.parse_args()

    zips = [zipfile.ZipFile(path) for path in args.jar]
    try:
        entries: dict[str, tuple[zipfile.ZipFile, str]] = {}
        for z in zips:
            for name in z.namelist():
                if name.endswith(".class") and name not in entries:
                    entries[name] = (z, name)

        parsed: dict[str, tuple[bool, str, list[str], list[tuple[str, str]]]] = {}

        def load(cname: str) -> None:
            if not cname or cname in parsed or cname == "java/lang/Object":
                return
            hit = entries.get(cname + ".class")
            if hit is None:
                return
            z, entry = hit
            is_i, sup, ifaces, methods = parse_class(z.read(entry))
            parsed[cname] = (is_i, sup, ifaces, methods)
            load(sup)
            for iface in ifaces:
                load(iface)

        missing = []
        for target in HOST_MODELLED_GAME_TARGETS:
            if (target + ".class") not in entries:
                missing.append(target)
            else:
                load(target)
        if missing:
            raise SystemExit(f"missing HOST_MODELLED_GAME_TARGETS in jars: {missing}")

        lines = [
            "# Vanilla 26.3 + authlib 10.0.77 target hierarchy and method signatures (names only; no code).",
            "# Generated by tools/generate_vanilla_host_methods.py.",
            "# C<tab>class<tab>super<tab>iface1,iface2,...",
            "# I<tab>interface<tab><tab>super_iface1,...",
            "# M<tab>owner<tab>name<tab>descriptor",
        ]
        for cname in sorted(parsed):
            is_i, sup, ifaces, _methods = parsed[cname]
            if is_i:
                lines.append(f"I\t{cname}\t\t{','.join(ifaces)}")
            else:
                lines.append(f"C\t{cname}\t{sup}\t{','.join(ifaces)}")
        for cname in sorted(parsed):
            _is_i, _sup, _ifaces, methods = parsed[cname]
            for name, desc in methods:
                lines.append(f"M\t{cname}\t{name}\t{desc}")

        args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(
            f"wrote {args.output}: {len(parsed)} types, "
            f"{sum(len(v[3]) for v in parsed.values())} method signatures"
        )
        return 0
    finally:
        for z in zips:
            z.close()


if __name__ == "__main__":
    raise SystemExit(main())

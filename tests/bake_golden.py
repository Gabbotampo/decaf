"""The golden synthetic input of the bake's default (Sunrise) target, and its recorded output.

`tests/data/bake_sunrise_golden.json` holds what `tools/bake.py` at commit 91040874 (before the
horizon target existed) wrote for this input: its exit status, stdout, stderr and the sha256 of
every file it left in the game folder. `tests.test_bake` bakes the same input with today's tool
and compares, so any change to the Sunrise output is caught. The input carries everything the
horizon target treats differently (executable names and contents, `.`/`..`/trailing-dot paths,
non-strict JSON, bad tags, registry data kinds, `c`/`horizon`/`horizon_java` content, block
templates and numbers it does not take, a block without its item next to an item of its id, a
quote and a backslash in a jar name), so none of those rules may leak into the default target.

Regenerate only from the original code (never from the code under test):

    git show 91040874:tools/bake.py > /tmp/bake_91040874.py
    python3 tests/bake_golden.py /tmp/bake_91040874.py > tests/data/bake_sunrise_golden.json

(The original reads no table from `crates/`, so it runs from any folder.)
"""

from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
import sys
import tempfile
import zipfile
from pathlib import Path

GOLDEN = Path(__file__).resolve().parent / "data" / "bake_sunrise_golden.json"

# A probe that prints the declarations stored next to each jar as `<jar>.decl.json`.
PROBE = """#!/usr/bin/env python3
import json, sys
with open(sys.argv[2] + ".decl.json") as f:
    content = json.load(f)
print(json.dumps({"status": "ok", "entrypoints": ["x.Y"], "content": content}))
"""

PNG = bytes.fromhex(
    "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4"
    "890000000d49444154789c6360f8cfc000000301010018dd8db10000000049454e44ae426082"
)
STONE = {
    "template": "minecraft:stone",
    "properties": {"hardness": 3.0, "requires_tool": True},
    "state_properties": {},
    "default_state": {},
    "item": True,
}


def _block(**fields) -> dict:
    file = json.loads(json.dumps(STONE))
    file.update(fields)
    return file


def _jar(path: Path, entries: dict[str, bytes], blocks: list, items: list) -> None:
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in entries.items():
            archive.writestr(zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0)), data)
    declarations = {
        "blocks": [{"id": i, "file": f} for i, f in blocks],
        "items": [{"id": i, "file": f} for i, f in items],
    }
    # The probe may print NaN (`json.dumps` writes it); Python's reader takes it.
    Path(str(path) + ".decl.json").write_text(json.dumps(declarations), encoding="utf-8")


def build(root: Path) -> tuple[Path, Path]:
    """Writes the golden input under `root`; returns (game folder, probe)."""
    game = root / "game"
    mods = game / "config" / "horizon_java" / "java-mods"
    mods.mkdir(parents=True)
    probe = root / "probe.py"
    probe.write_text(PROBE, encoding="utf-8")
    probe.chmod(0o755)
    (game / "sunrise-mods.txt").write_bytes(b"mod.weird:true\r\nmod.ruby_mod:false\r\n")

    comment = b"// a comment\n"
    entries = {
        "fabric.mod.json": b'{"id":"ruby_mod"}',
        "ruby_mod/Entry.class": b"\xca\xfe\xba\xbe",
        "assets/ruby_mod/textures/item/ruby.png": PNG,
        "assets/ruby_mod/textures/a.com": b"MZ is a word",
        "assets/ruby_mod/textures/run.ps1": b"plain text\n",
        "assets/ruby_mod/textures/elf.png": b"\x7fELF\x02\x01",
        "assets/ruby_mod/textures/script.sh": b"echo\n",
        "assets/ruby_mod/textures/pack.zip": b"PK\x05\x06",
        "assets/ruby_mod/textures/tool.py": b"print()\n",
        "assets/ruby_mod/textures/a./b.png": PNG,
        "assets/ruby_mod/lang/en_us.json": comment + b'{"item.ruby_mod.ruby":"Ruby"}',
        "assets/ruby_mod/lang/x.json\n": b"{}",
        "assets/ruby_mod/models/item/ruby.json": b'{"parent":"minecraft:item/generated"}',
        "data/ruby_mod/loot_table/blocks/ruby_block.json": b'{"pools":[]}',
        "data/ruby_mod/loot_table/bom.json": b'\xef\xbb\xbf{"pools":[]}',
        "data/ruby_mod/recipe/hand.json": b'{"type":"minecraft:crafting_shapeless"}',
        "data/ruby_mod/recipe/altar.json": b'{"type":"ruby_mod:altar"}',
        "data/ruby_mod/tags/block/empty.json": b"{}",
        "data/ruby_mod/tags/block/int.json": b'{"values":[1]}',
        "data/ruby_mod/tags/block/note.txt": b"x",
        "data/ruby_mod/tags/item/gems.json": b'{"values":["ruby_mod:ruby","ruby_mod:missing"]}',
        "data/ruby_mod/damage_type/x.json": b'{"message_id":"x"}',
        "data/ruby_mod/predicate/x.json": b'{"condition":"minecraft:bogus"}',
        "data/ruby_mod/item_modifier/x.json": b'{"function":"minecraft:bogus"}',
        "data/ruby_mod/worldgen/configured_feature/x.json": b"{}",
        "data/ruby_mod/advancement/a.json": comment + b"{}",
        "data/ruby_mod/structure/s.nbt": b"\x0a\x00\x00",
        "data/minecraft/tags/block/mineable/pickaxe.json": b'{"values":["ruby_mod:ruby_block"]}',
        "data/minecraft/recipe/x.json": b"{}",
        "data/c/tags/item/gems.json": b'{"values":["ruby_mod:ruby"]}',
        "data/c/recipe/x.json": b"{}",
        "data/horizon/tags/block/villager_secondary_poi/farmer.json": b'{"values":[]}',
        "data/horizon/recipe/x.json": b"{}",
        "data/horizon_java/loot_table/x.json": b'{"pools":[]}',
        "data/ruby_mod/sunrise/block/other.json": b"{}",
    }
    blocks = [
        ("ruby_mod:ruby_block", STONE),
        ("ruby_mod:ghost", _block(item=False, block_item="minecraft:diamond")),
        ("ruby_mod:furnace", _block(template="minecraft:furnace",
                                    state_properties={"lit": {"type": "boolean"}})),
        ("ruby_mod:slick", _block(properties={"friction": 5.0, "hardness": -0.5})),
        ("ruby_mod:huge", _block(properties={"hardness": 10**400})),
        ("ruby_mod:nan", _block(properties={"hardness": float("nan")})),
        ("ruby_mod:copper", _block(template="minecraft:copper_block",
                                   properties={"sounds": "minecraft:copper_block"})),
        ("ruby_mod:twice", STONE),
        ("ruby_mod:twice", _block(item=False)),
        ("ruby_mod:a/./b", STONE),
        ("ruby_mod:a/../b", STONE),
        ("ruby_mod:a./b", STONE),
        ("c:gem_block", STONE),
        ("horizon:gem_block", STONE),
        ("horizon_java:proof_block", STONE),
        ("minecraft:stone2", STONE),
    ]
    items = [
        ("ruby_mod:ruby", {"template": "minecraft:stick", "max_stack_size": 16, "rarity": "rare"}),
        ("ruby_mod:ghost", {"template": "minecraft:stick"}),
        ("ruby_mod:ruby_block", {"template": "minecraft:stick"}),
        ("ruby_mod:sword", {"template": "minecraft:diamond_sword", "max_stack_size": 16}),
        ("ruby_mod:odd", {"template": "minecraft:stick", "rarity": ["rare"]}),
        ("ruby_mod:rest", {"template": "minecraft:stick", "crafting_remainder": "other_ns:gem"}),
        ("other_ns:gem", {"template": "minecraft:stick"}),
    ]
    _jar(mods / "ruby_mod.jar", entries, blocks, items)
    _jar(
        mods / 'we"ird\\name.jar',
        {"assets/weird/lang/en_us.json": b"{}"},
        [("weird:b", STONE)],
        [],
    )
    _jar(mods / "evil.jar", {"assets/evil/lang/en_us.json": b"{}", "../x.json": b"{}"}, [], [])
    _jar(mods / "ruby-dash.jar", {"assets/ruby-dash/lang/en_us.json": b"{}"}, [],
         [("ruby-dash:gem", {"template": "minecraft:stick"})])
    return game, probe


def run(bake, root: Path, *arguments: str) -> dict:
    """Bakes the golden input with the `bake` module's `main`; returns what it did."""
    game, probe = build(root)
    stdout, stderr = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
        status = bake.main(["--game-dir", str(game), "--probe", str(probe), *arguments])
    inputs = game / "config" / "horizon_java" / "java-mods"
    files = {
        path.relative_to(game).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(game.rglob("*"))
        if path.is_file() and inputs not in path.parents
    }
    return {
        "status": status,
        "stdout": stdout.getvalue().replace(str(root), "<root>"),
        "stderr": stderr.getvalue().replace(str(root), "<root>"),
        "files": files,
    }


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("bake_under_golden", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


if __name__ == "__main__":
    original = load_module(Path(sys.argv[1]))
    with tempfile.TemporaryDirectory() as scratch:
        result = run(original, Path(scratch))
    result = {"generated_by": "tools/bake.py at 91040874", **result}
    print(json.dumps(result, indent=2, sort_keys=True))

#!/usr/bin/env python3
"""Generate the reviewed Lane 1 Sunrise listener manifest.

The mapping is the review authority. `functions.json` and `points.json` are build artifacts from
one pinned Sunrise checkout; a point row is emitted only when both indexes contain its exact Rust
path and the point has no API refusal. Hook rows retain the reviewed difference/evidence text.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


def _items(value: Any, keys: tuple[str, ...]) -> list[dict[str, Any]]:
    if isinstance(value, list):
        return [item for item in value if isinstance(item, dict)]
    if isinstance(value, dict):
        for key in keys:
            if isinstance(value.get(key), list):
                return [item for item in value[key] if isinstance(item, dict)]
        return [dict(item, path=key) for key, item in value.items() if isinstance(item, dict)]
    return []


def _path(item: dict[str, Any]) -> str | None:
    for key in ("path", "target", "function", "at", "name"):
        value = item.get(key)
        if isinstance(value, str) and value:
            return value
    return None


def _index(value: Any, keys: tuple[str, ...]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for item in _items(value, keys):
        path = _path(item)
        if path:
            result[path] = item
    return result


def _phase(anchor: str) -> str | None:
    value = anchor.split(";", 1)[0].strip().upper()
    if value == "HEAD":
        return "head"
    if value == "RETURN":
        return "return"
    if value == "TAIL":
        return "tail"
    if value in {"HEAD/RETURN", "RETURN/HEAD", "HEAD/TAIL", "RETURN/TAIL"}:
        return "replace"
    return None


def _slug(vanilla: str, phase: str) -> str:
    value = re.sub(r"[^A-Za-z0-9]+", "_", vanilla).strip("_").lower()
    return f"java_{value}_{phase}"


def _method(row: dict[str, Any]) -> tuple[str, str]:
    vanilla = str(row["vanilla"])
    owner, name = vanilla.split("#", 1)
    return f"{owner}#{name}", str(row["descriptor"])


def _fit(row: dict[str, Any]) -> str:
    return str(row.get("fit", "")).upper()


def _is_no_fit(row: dict[str, Any]) -> bool:
    return "NO FIT" in _fit(row)


def _point_candidate(row: dict[str, Any]) -> bool:
    return "::" in str(row.get("sunrise", "")) and _phase(str(row.get("anchor", ""))) is not None


def _candidate_order(row: dict[str, Any]) -> tuple[int, int]:
    # Prefer a validated API point over a coarse existing hook, then exact review fits.
    return (0 if _point_candidate(row) else 1, 0 if _fit(row) == "EXACT FIT" else 1)


def generate(mapping: Any, functions: Any, points: Any) -> dict[str, Any]:
    rows = mapping.get("rows", []) if isinstance(mapping, dict) else mapping
    function_index = _index(functions, ("functions", "items", "entries"))
    point_index = _index(points, ("points", "items", "entries"))
    groups: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in rows if isinstance(rows, list) else []:
        if not isinstance(row, dict) or _is_no_fit(row):
            continue
        groups.setdefault(_method(row), []).append(row)

    inject: list[dict[str, Any]] = []
    hooks: list[dict[str, Any]] = []
    errors: list[str] = []
    for candidates in groups.values():
        selected = None
        for row in sorted(candidates, key=_candidate_order):
            target = str(row.get("sunrise", ""))
            phase = _phase(str(row.get("anchor", "")))
            if not _point_candidate(row):
                selected = ("hook", row, None)
                break
            if target not in function_index:
                errors.append(f"missing function target: {target}")
                continue
            point = point_index.get(target)
            if point is None:
                errors.append(f"missing point target: {target}")
                continue
            refusal = point.get("api_refusal")
            if refusal not in (None, ""):
                errors.append(f"refused point {target}: {refusal}")
                continue
            selected = ("inject", row, point)
            break
        if selected is None:
            continue
        kind, row, point = selected
        method, descriptor = _method(row)
        common = {
            "id": _slug(row["vanilla"], _phase(str(row.get("anchor", ""))) or "hook"),
            "method": method,
            "descriptor": descriptor,
            "fit": _fit(row),
            "evidence": str(row.get("evidence", "")),
        }
        if kind == "inject":
            phase = _phase(str(row["anchor"]))
            entry = {
                **common,
                "at": str(row["sunrise"]),
                "kind": phase,
                "with": common["id"],
            }
            signature = point.get("signature_hash") or point.get("hash")
            if signature:
                entry["hash"] = signature
            if common["fit"] != "EXACT FIT":
                entry["note"] = common["evidence"]
            inject.append(entry)
        else:
            hooks.append({**common, "target": str(row["sunrise"]), "note": common["evidence"]})

    inject.sort(key=lambda item: item["id"])
    hooks.sort(key=lambda item: item["id"])
    return {"inject": inject, "hooks": hooks, "errors": sorted(set(errors))}


def _toml_string(value: Any) -> str:
    return json.dumps(str(value), ensure_ascii=False)


def render(plan: dict[str, Any], source: str) -> str:
    lines = [
        "# Generated by tools/java_hook_generator.py; do not edit.",
        f"source = {_toml_string(source)}",
        f"inject_count = {len(plan['inject'])}",
        f"hook_count = {len(plan['hooks'])}",
    ]
    for section, entries in (("inject", plan["inject"]), ("hook", plan["hooks"])):
        for entry in entries:
            lines.append(f"\n[[{section}]]")
            for key, value in entry.items():
                lines.append(f"{key} = {_toml_string(value)}")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mapping", type=Path)
    parser.add_argument("--functions", required=True, type=Path)
    parser.add_argument("--points", required=True, type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    plan = generate(json.loads(args.mapping.read_text()), json.loads(args.functions.read_text()), json.loads(args.points.read_text()))
    if plan["errors"]:
        for error in plan["errors"]:
            print(f"java-hook-generator: {error}", file=sys.stderr)
        return 2
    text = render(plan, args.mapping.name)
    if args.output:
        args.output.write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

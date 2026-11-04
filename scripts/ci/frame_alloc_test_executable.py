#!/usr/bin/env python3
"""Resolve the frame_alloc integration test binary from `cargo test --no-run --message-format=json`."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def executable_from_cargo_json(path: Path) -> str:
    exe = ""
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        target = msg.get("target") or {}
        kinds = target.get("kind") or []
        if target.get("name") == "frame_alloc" and "test" in kinds and msg.get("executable"):
            exe = msg["executable"]
    if not exe:
        raise SystemExit("frame_alloc executable missing from cargo json")
    return exe


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {argv[0]} <cargo-json-log>", file=sys.stderr)
        return 2
    print(executable_from_cargo_json(Path(argv[1])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

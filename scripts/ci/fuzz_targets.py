#!/usr/bin/env python3
"""Print every cargo-fuzz [[bin]] as a JSON matrix for fuzz-nightly.yml.

Commented-out bins are skipped. A new fuzz_targets file is not a target until
its fuzz/Cargo.toml lists it under [[bin]].
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def bins_in(cargo_text: str) -> list[str]:
    names: list[str] = []
    in_bin = False
    for raw in cargo_text.splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped == "[[bin]]":
            in_bin = True
            continue
        if stripped.startswith("["):
            in_bin = False
            continue
        if in_bin and stripped.startswith("name"):
            value = stripped.split("=", 1)[1].strip().strip('"').strip("'")
            if value:
                names.append(value)
            in_bin = False
    return names


def matrix(root: Path = ROOT) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for cargo in sorted(root.glob("**/fuzz/Cargo.toml")):
        if "target" in cargo.parts:
            continue
        rel = cargo.parent.relative_to(root).as_posix()
        for name in bins_in(cargo.read_text(encoding="utf-8")):
            rows.append({"dir": rel, "target": name})
    rows.sort(key=lambda row: (row["dir"], row["target"]))
    return rows


def main() -> None:
    json.dump(matrix(), sys.stdout, separators=(",", ":"))
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()

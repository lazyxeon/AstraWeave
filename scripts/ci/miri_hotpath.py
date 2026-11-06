#!/usr/bin/env python3
"""Decide whether the Miri ECS hot-path job runs.

`miri.yml` starts on `astraweave-ecs/**` so a rename whose destination stays
in the crate still selects the workflow. GitHub's path list can omit a
rename's old path (#589 item 4, #557 class). This script reads
`git diff --no-renames` (via `affected_crates.diff_files`), so the old path
counts. Miri runs only when a watched path changed. An empty diff runs Miri.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import affected_crates  # noqa: E402

# Exact paths. `astraweave-ecs/**` in the workflow is only the trigger.
WATCHED = (
    "astraweave-ecs/src/blob_vec.rs",
    "astraweave-ecs/src/blob_vec_kani.rs",
    "astraweave-ecs/src/sparse_set.rs",
    "astraweave-ecs/src/archetype.rs",
    "astraweave-ecs/src/events.rs",
    ".github/workflows/miri.yml",
    ".github/rust-nightly-channel",
)
WATCHED_PREFIXES = (".github/actions/rust-nightly/",)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def selects(paths: list[str]) -> bool:
    for path in paths:
        if path in WATCHED:
            return True
        if any(path.startswith(prefix) for prefix in WATCHED_PREFIXES):
            return True
    return False


def classify(paths: list[str]) -> dict[str, str]:
    if not paths:
        return {"run": "true", "reason": "empty-diff"}
    if selects(paths):
        return {"run": "true", "reason": "watched"}
    return {"run": "false", "reason": "not-hot-path"}


def write_output(result: dict[str, str], github_output: str | None) -> None:
    text = "".join(f"{key}={value}\n" for key, value in result.items())
    if github_output:
        with open(github_output, "a", encoding="utf-8") as handle:
            handle.write(text)
    else:
        sys.stdout.write(text)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="Base git SHA (three-dot diff)")
    parser.add_argument("--head", help="Head git SHA")
    parser.add_argument(
        "--files",
        help="Comma-separated paths, instead of git diff (tests)",
    )
    parser.add_argument("--github-output", help="Append KEY=VALUE lines to this file")
    args = parser.parse_args(argv)
    if args.files is not None:
        files = [item for item in args.files.split(",") if item]
    elif args.base and args.head:
        files = affected_crates.diff_files(repo_root(), args.base, args.head)
    else:
        parser.error("pass --base and --head, or --files")
        return 2
    write_output(classify(files), args.github_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

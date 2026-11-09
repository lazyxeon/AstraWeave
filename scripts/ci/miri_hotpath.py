#!/usr/bin/env python3
"""Decide whether the Miri ECS hot-path job runs.

`miri.yml` does not filter `pull_request` on paths. GitHub lists a rename's
destination and can omit the source, so a crate glob would miss a move of
`blob_vec.rs` out of `astraweave-ecs/`. This script reads
`git diff --no-renames` (via `affected_crates.diff_files`), so the old path
counts. Miri runs when a watched path changed. An empty diff runs Miri.

`SOURCE_GLOBS` and `DEST_GLOBS` are the crate globs for the two sides of a
rename. They are not a workflow path filter: a destination outside the crate
does not match them, and a file such as `rng.rs` matches them without being
a watched path.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import affected_crates  # noqa: E402

# Crate globs for a rename's source and destination. Not a run trigger:
# `selects` still requires a watched path. `rng.rs` matches the glob and
# stays `not-hot-path`.
SOURCE_GLOBS = ("astraweave-ecs/**",)
DEST_GLOBS = ("astraweave-ecs/**",)

# Exact paths. The workflow trigger is every pull request, not these globs.
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


def glob_match(pattern: str, path: str) -> bool:
    """Match one GitHub Actions path glob. `*` does not cross `/`. `**` does."""
    regex: list[str] = ["^"]
    index = 0
    while index < len(pattern):
        if pattern.startswith("**", index):
            regex.append(".*")
            index += 2
        elif pattern[index] == "*":
            regex.append("[^/]*")
            index += 1
        else:
            regex.append(re.escape(pattern[index]))
            index += 1
    regex.append("$")
    return re.fullmatch("".join(regex), path) is not None


def matches_crate_glob(path: str) -> bool:
    """True when `path` is under a source or destination crate glob."""
    return any(glob_match(pattern, path) for pattern in SOURCE_GLOBS + DEST_GLOBS)


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

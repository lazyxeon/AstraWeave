#!/usr/bin/env bash
# `cargo fmt --all` puts every crate file on one rustfmt command line. On
# Windows that exceeds 8191 characters (os error 206). Format each cargo
# target root instead. rustfmt follows `mod` from that root, which is the
# same set `cargo fmt` checks, and each command stays short.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
rustfmt_bin="$(rustc --print sysroot)/bin/rustfmt"
if [[ ! -x "$rustfmt_bin" ]]; then
  echo "rustfmt is not installed in $(rustc --print sysroot)" >&2
  exit 1
fi

python3 - "$rustfmt_bin" <<'PY'
import json, subprocess, sys
rustfmt = sys.argv[1]
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"], text=True
))
roots = []
for pkg in meta["packages"]:
    edition = pkg.get("edition") or "2021"
    for target in pkg.get("targets", []):
        src = target.get("src_path")
        if src and src.endswith(".rs"):
            roots.append((edition, src))
# Stable order, drop duplicates (a path can be both lib and test).
seen = set()
ordered = []
for edition, src in roots:
    if src in seen:
        continue
    seen.add(src)
    ordered.append((edition, src))

def flush(edition, batch):
    if not batch:
        return
    proc = subprocess.run(
        [rustfmt, "--check", "--edition", edition, *batch],
        text=True,
    )
    if proc.returncode != 0:
        sys.exit(proc.returncode)

# Keep each rustfmt command line under Windows' 8191 limit.
limit = 6000
batch = []
chars = 0
edition_now = None
for edition, src in ordered:
    n = len(src) + 1
    if batch and (edition != edition_now or chars + n > limit):
        flush(edition_now, batch)
        batch, chars = [], 0
    edition_now = edition
    batch.append(src)
    chars += n
flush(edition_now, batch)
print(f"rustfmt --check passed for {len(ordered)} cargo target roots")
PY

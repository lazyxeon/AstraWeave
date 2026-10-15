#!/usr/bin/env bash
# 11 ms frame budget for a GTX 1660 Ti-class GPU.
# GitHub-hosted runners have no such GPU. Allocation Measurement runs this
# only for workflow_dispatch on a self-hosted runner labeled `gpu`.
set -euo pipefail

limit_ms="11"
src="${AW_FRAME_TIMES_MS:-}"
if [ -z "$src" ] && [ -f frame-times.txt ]; then
  src="$(paste -sd, frame-times.txt)"
fi
if [ -z "$src" ]; then
  echo "No frame sample. On the self-hosted runner, export AW_FRAME_TIMES_MS" >&2
  echo "as comma-separated milliseconds, or write frame-times.txt (one sample per line)." >&2
  exit 1
fi

AW_FRAME_SRC="$src" AW_FRAME_LIMIT="$limit_ms" python3 - << 'PY'
import os
import statistics
import sys

raw = os.environ["AW_FRAME_SRC"]
limit = float(os.environ["AW_FRAME_LIMIT"])
vals = [float(part) for part in raw.split(",") if part.strip()]
if not vals:
    print("AW_FRAME_TIMES_MS was empty", file=sys.stderr)
    sys.exit(1)
median = statistics.median(vals)
print(f"[frame-budget] samples={len(vals)} median_ms={median:.3f} limit_ms={limit:.1f}")
if median > limit:
    print(
        f"median frame {median:.3f} ms is over the {limit:.1f} ms GTX 1660 Ti budget",
        file=sys.stderr,
    )
    sys.exit(1)
PY

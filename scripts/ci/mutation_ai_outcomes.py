#!/usr/bin/env python3
"""Score cargo-mutants outcomes.json for astraweave-ai CI shards."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

# ulimit -v caps virtual memory; allocator aborts look like test failure (exit 101)
# but are not test kills. Exclude them from the caught count used for the 70% gate.
CAP_OOM_MARKERS = (
    "memory allocation of",
    "Cannot allocate memory",
)


@dataclass(frozen=True)
class OutcomeCounts:
    caught: int
    missed: int
    timeout: int
    cap_oom: int

    @property
    def decided(self) -> int:
        return self.caught + self.missed

    @property
    def score_percent(self) -> int:
        if self.decided <= 0:
            return 0
        return self.caught * 100 // self.decided


def _log_is_cap_oom(mutants_out_dir: Path, log_path: str | None) -> bool:
    if not log_path:
        return False
    log_file = mutants_out_dir / log_path
    if not log_file.is_file():
        return False
    text = log_file.read_text(encoding="utf-8", errors="replace")
    return any(marker in text for marker in CAP_OOM_MARKERS)


def count_outcomes(outcomes_path: Path) -> OutcomeCounts:
    data = json.loads(outcomes_path.read_text(encoding="utf-8"))
    mutants_out_dir = outcomes_path.parent
    caught = 0
    missed = 0
    timeout = 0
    cap_oom = 0
    for entry in data.get("outcomes", []):
        if entry.get("scenario") == "Baseline":
            continue
        summary = entry.get("summary")
        if summary == "MissedMutant":
            missed += 1
        elif summary == "Timeout":
            timeout += 1
        elif summary == "CaughtMutant":
            if _log_is_cap_oom(mutants_out_dir, entry.get("log_path")):
                cap_oom += 1
            else:
                caught += 1
    return OutcomeCounts(caught=caught, missed=missed, timeout=timeout, cap_oom=cap_oom)


def verify_shard_outcomes(outcomes_path: Path) -> list[str]:
    errors: list[str] = []
    if not outcomes_path.is_file():
        return [f"missing outcomes file: {outcomes_path}"]

    data = json.loads(outcomes_path.read_text(encoding="utf-8"))
    outcomes = data.get("outcomes", [])
    baseline_ok = any(
        o.get("scenario") == "Baseline" and o.get("summary") == "Success" for o in outcomes
    )
    if not baseline_ok:
        errors.append("baseline did not complete successfully")

    non_baseline = len([o for o in outcomes if o.get("scenario") != "Baseline"])
    if non_baseline <= 0:
        errors.append("no mutant outcomes recorded after baseline")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "outcomes",
        nargs="+",
        type=Path,
        help="paths to outcomes.json (one shard or many for aggregate scoring)",
    )
    parser.add_argument(
        "--verify-shard",
        action="store_true",
        help="require exactly one outcomes file and a completed shard",
    )
    parser.add_argument(
        "--format",
        choices=("shell", "json"),
        default="shell",
        help="output format for aggregate counts",
    )
    args = parser.parse_args(argv)

    if args.verify_shard:
        if len(args.outcomes) != 1:
            print("verify-shard expects exactly one outcomes path", file=sys.stderr)
            return 2
        errors = verify_shard_outcomes(args.outcomes[0])
        for err in errors:
            print(f"::error::{err}", file=sys.stderr)
        return 1 if errors else 0

    total = OutcomeCounts(0, 0, 0, 0)
    for path in args.outcomes:
        if not path.is_file():
            print(f"::error::missing outcomes file: {path}", file=sys.stderr)
            return 1
        shard_errors = verify_shard_outcomes(path)
        for err in shard_errors:
            print(f"::error::{path}: {err}", file=sys.stderr)
            return 1
        part = count_outcomes(path)
        total = OutcomeCounts(
            caught=total.caught + part.caught,
            missed=total.missed + part.missed,
            timeout=total.timeout + part.timeout,
            cap_oom=total.cap_oom + part.cap_oom,
        )

    if args.format == "json":
        print(
            json.dumps(
                {
                    "caught": total.caught,
                    "missed": total.missed,
                    "timeout": total.timeout,
                    "cap_oom": total.cap_oom,
                    "decided": total.decided,
                    "score_percent": total.score_percent,
                }
            )
        )
    else:
        print(f"CAUGHT={total.caught}")
        print(f"MISSED={total.missed}")
        print(f"TIMEOUT={total.timeout}")
        print(f"CAP_OOM={total.cap_oom}")
        print(f"DECIDED={total.decided}")
        print(f"SCORE={total.score_percent}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

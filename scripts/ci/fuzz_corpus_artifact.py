#!/usr/bin/env python3
"""Print the newest previous fuzz-corpus workflow run id.

stdin is the GitHub Actions artifacts list for one artifact name. `gh api
--paginate` may emit one JSON object per page. argv[1] is this run's id.
Expired artifacts and this run are skipped. Prints nothing when no earlier
run has a corpus.
"""

from __future__ import annotations

import json
import sys
from typing import Any


def load_pages(raw: str) -> list[Any]:
    decoder = json.JSONDecoder()
    pages: list[Any] = []
    idx = 0
    length = len(raw)
    while idx < length:
        while idx < length and raw[idx].isspace():
            idx += 1
        if idx >= length:
            break
        obj, end = decoder.raw_decode(raw, idx)
        pages.append(obj)
        idx = end
    return pages


def select_corpus_run(pages: list[Any], this_run: str) -> str | None:
    this = str(this_run)
    best: tuple[str, int] | None = None
    best_id: str | None = None
    for page in pages:
        artifacts = page.get("artifacts") if isinstance(page, dict) else None
        if not isinstance(artifacts, list):
            continue
        for art in artifacts:
            if not isinstance(art, dict) or art.get("expired") is True:
                continue
            run = art.get("workflow_run")
            if not isinstance(run, dict) or run.get("id") is None:
                continue
            run_id = str(run["id"])
            if run_id == this or not run_id.isdigit():
                continue
            created = art.get("created_at")
            if not isinstance(created, str):
                created = ""
            rank = (created, int(run_id))
            if best is None or rank > best:
                best = rank
                best_id = run_id
    return best_id


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 1:
        print("usage: fuzz_corpus_artifact.py <this-run-id>", file=sys.stderr)
        return 2
    chosen = select_corpus_run(load_pages(sys.stdin.read()), args[0])
    if chosen is not None:
        print(chosen)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Print the workflow run id of the corpus artifact to restore.

stdin is the GitHub Actions artifacts list for one artifact name. `gh api
--paginate` may emit one JSON object per page. argv is this run's id, the
default branch, the base repository (`owner/name`), and a file of workflow
runs for `.github/workflows/fuzz-nightly.yml`.

A candidate must be a successful run of that workflow, on the default
branch, whose head repository is the base repository. Fork pull requests,
other branches, other workflows, and failed runs are skipped, as are this
run and expired artifacts. Prints nothing when no corpus qualifies.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

WORKFLOW_PATH = ".github/workflows/fuzz-nightly.yml"


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


def _run_id(value: Any) -> str | None:
    if value is None or isinstance(value, bool):
        return None
    text = str(value)
    if not text.isdigit():
        return None
    return text


def _full_name(value: Any) -> str | None:
    if not isinstance(value, dict):
        return None
    name = value.get("full_name")
    if not isinstance(name, str) or not name:
        return None
    return name


def run_allowed(
    run: dict[str, Any],
    *,
    this_run: str,
    branch: str,
    repository: str,
) -> bool:
    run_id = _run_id(run.get("id"))
    if run_id is None or run_id == this_run:
        return False
    if run.get("conclusion") != "success":
        return False
    status = run.get("status")
    if status is not None and status != "completed":
        return False
    if run.get("head_branch") != branch:
        return False
    if run.get("path") != WORKFLOW_PATH:
        return False
    if run.get("event") == "pull_request":
        return False
    if _full_name(run.get("head_repository")) != repository:
        return False
    home = _full_name(run.get("repository"))
    if home is not None and home != repository:
        return False
    return True


def allowed_run_ids(
    run_pages: list[Any],
    *,
    this_run: str,
    branch: str,
    repository: str,
) -> set[str]:
    if not branch or not repository:
        return set()
    allowed: set[str] = set()
    for page in run_pages:
        runs = page.get("workflow_runs") if isinstance(page, dict) else None
        if not isinstance(runs, list):
            continue
        for run in runs:
            if isinstance(run, dict) and run_allowed(
                run, this_run=this_run, branch=branch, repository=repository
            ):
                run_id = _run_id(run.get("id"))
                if run_id is not None:
                    allowed.add(run_id)
    return allowed


def artifact_matches(run: dict[str, Any], branch: str) -> bool:
    head_branch = run.get("head_branch")
    if isinstance(head_branch, str) and head_branch != branch:
        return False
    repository_id = run.get("repository_id")
    head_repository_id = run.get("head_repository_id")
    if (
        repository_id is not None
        and head_repository_id is not None
        and repository_id != head_repository_id
    ):
        return False
    return True


def select_corpus_run(
    pages: list[Any],
    this_run: str,
    *,
    branch: str,
    repository: str,
    run_pages: list[Any],
) -> str | None:
    this = str(this_run)
    allowed = allowed_run_ids(
        run_pages, this_run=this, branch=branch, repository=repository
    )
    if not allowed:
        return None
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
            if not isinstance(run, dict):
                continue
            run_id = _run_id(run.get("id"))
            if run_id is None or run_id not in allowed:
                continue
            if not artifact_matches(run, branch):
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
    if len(args) != 4:
        print(
            "usage: fuzz_corpus_artifact.py <this-run-id> <branch> "
            "<repository> <runs-json>",
            file=sys.stderr,
        )
        return 2
    this_run, branch, repository, runs_path = args
    run_pages = load_pages(Path(runs_path).read_text(encoding="utf-8"))
    chosen = select_corpus_run(
        load_pages(sys.stdin.read()),
        this_run,
        branch=branch,
        repository=repository,
        run_pages=run_pages,
    )
    if chosen is not None:
        print(chosen)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

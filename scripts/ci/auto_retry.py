#!/usr/bin/env python3
"""Decide and perform a scoped rerun of allowlisted flaky jobs.

The allowlist is .github/flaky-jobs.json. The denylist below is code.
cargo deny, cargo audit, Security Audit, replay hash, determinism-replay,
compare, workflow policy, and Gate never retry, even when the file names them.
A run retries only when every failed job is allowlisted and none is denied.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MAX_ATTEMPT = 3
LABEL = "auto-retry"
TRACKING_TITLE = "Auto-retry record"
TRACKING_MARKER = "Posted by `.github/workflows/auto-retry.yml`."

_REPO_RE = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
_TOKEN_RE = re.compile(r"[A-Za-z0-9_-]+")
_HASH_JOB_RE = re.compile(r"hash( \(.*\))?")


@dataclass(frozen=True)
class AllowJob:
    workflow: str
    job: str
    evidence: str


@dataclass(frozen=True)
class Decision:
    retry: bool
    reason: str


def normalize(value: str) -> str:
    return " ".join(value.casefold().split())


def _contains_any(text: str, needles: tuple[str, ...]) -> bool:
    return any(needle in text for needle in needles)


def job_denied(workflow: str, job: str) -> bool:
    workflow_cf = normalize(workflow)
    job_cf = normalize(job)
    blob = workflow_cf + "\n" + job_cf
    if _contains_any(blob, ("determinism-replay",)):
        return True  # deny:determinism-replay
    if _contains_any(blob, ("security audit",)):
        return True  # deny:security-audit
    if _contains_any(job_cf, ("cargo deny", "cargo-deny", "cargo_deny")):
        return True  # deny:cargo-deny
    if _contains_any(job_cf, ("cargo audit", "cargo-audit", "cargo_audit")):
        return True  # deny:cargo-audit
    if _contains_any(job_cf, ("replay hash",)):
        return True  # deny:replay-hash
    if _HASH_JOB_RE.fullmatch(job_cf):
        return True  # deny:hash-job
    if job_cf == "compare":
        return True  # deny:compare
    if _contains_any(job_cf, ("workflow policy",)):
        return True  # deny:workflow-policy
    if job_cf == "gate" or job_cf.startswith("gate (") or job_cf.startswith("gate "):
        return True  # deny:gate
    return False


def decide(
    workflow: str,
    failed_jobs: list[str],
    allow: list[AllowJob],
    attempt: int,
    conclusion: str,
) -> Decision:
    if conclusion != "failure":  # failure-only
        return Decision(False, "not-failure")
    if attempt >= MAX_ATTEMPT:  # attempt-cap
        return Decision(False, "attempt-cap")
    if attempt < 1:
        return Decision(False, "attempt-cap")
    if not failed_jobs:  # empty-failed
        return Decision(False, "no-failed-jobs")
    allowed = {(item.workflow, item.job) for item in allow}
    denied = False
    unlisted = False
    for job in failed_jobs:
        if job_denied(workflow, job):
            denied = True  # deny-wins
        if (workflow, job) not in allowed:  # allowlist-only
            unlisted = True
    if denied:
        return Decision(False, "denylist")
    if unlisted:
        return Decision(False, "not-allowlisted")
    return Decision(True, "retry")


def load_allowlist(path: Path) -> list[AllowJob]:
    data = json.loads(path.read_text(encoding="utf-8"))
    jobs = data.get("jobs")
    if not isinstance(jobs, list):
        raise ValueError("allowlist jobs must be a list")
    loaded: list[AllowJob] = []
    for entry in jobs:
        if not isinstance(entry, dict):
            raise ValueError("allowlist entry must be an object")
        workflow = entry.get("workflow")
        job = entry.get("job")
        evidence = entry.get("evidence")
        if not isinstance(workflow, str) or not workflow.strip():
            raise ValueError("allowlist workflow is empty")
        if not isinstance(job, str) or not job.strip():
            raise ValueError("allowlist job is empty")
        if not isinstance(evidence, str) or "actions/runs/" not in evidence:
            raise ValueError("allowlist evidence must cite an actions run")
        loaded.append(AllowJob(workflow.strip(), job.strip(), evidence.strip()))
    return loaded


def flatten_pages(payload: object, key: str | None) -> list[Any]:
    pages = payload if isinstance(payload, list) else [payload]
    items: list[Any] = []
    for page in pages:
        if isinstance(page, list):
            items.extend(page)
        elif isinstance(page, dict) and key and isinstance(page.get(key), list):
            items.extend(page[key])
    return items


def failed_job_names(payload: object) -> list[str]:
    names: list[str] = []
    for job in flatten_pages(payload, "jobs"):
        if not isinstance(job, dict) or job.get("conclusion") != "failure":
            continue
        name = job.get("name")
        if isinstance(name, str) and name.strip():
            names.append(name.strip())
    return names


def build_record(
    run_url: str, attempt: int, failed_jobs: list[str], conclusion: str
) -> str:
    names = ", ".join(" ".join(job.split()) for job in failed_jobs) or "(none)"
    return "\n".join(
        [
            "Auto-retry record.",
            "",
            f"- Run: {run_url}",
            f"- Attempt: {attempt}",
            f"- Failed jobs: {names}",
            f"- Conclusion: {conclusion}",
            "",
            "The first failure stays on this record.",
            TRACKING_MARKER,
        ]
    )


def record_destination(event: str, head_branch: str, default_branch: str) -> str:
    if event == "schedule" or head_branch == default_branch:
        return "issue"
    return "pr"


def select_tracking_issue(issues: list[dict[str, Any]]) -> int | None:
    numbers: list[int] = []
    for issue in issues:
        if not isinstance(issue, dict) or issue.get("pull_request"):
            continue
        if issue.get("title") != TRACKING_TITLE:
            continue
        body = issue.get("body") or ""
        if TRACKING_MARKER not in body:
            continue
        number = issue.get("number")
        if isinstance(number, int):
            numbers.append(number)
    if not numbers:
        return None
    return min(numbers)


def run_retry(decision: Decision, record: str, transport: Any) -> None:
    if not decision.retry:
        transport.skip(decision.reason)
        return
    transport.post_record(record)  # record-before-rerun
    transport.rerun()


def default_branch_raw_url(repo: str, branch: str, path: str) -> str:
    if not _REPO_RE.fullmatch(repo):
        raise ValueError("repo")
    if not re.fullmatch(r"[A-Za-z0-9._/-]+", branch) or ".." in branch.split("/"):
        raise ValueError("branch")
    if not re.fullmatch(r"[A-Za-z0-9._/-]+", path) or ".." in path.split("/"):
        raise ValueError("path")
    return f"https://raw.githubusercontent.com/{repo}/{branch}/{path}"


def _gh(args: list[str], body: dict[str, Any] | None = None) -> tuple[int, str, str]:
    proc = subprocess.run(
        ["gh", "api", *args],
        input=None if body is None else json.dumps(body),
        text=True,
        capture_output=True,
        check=False,
    )
    return proc.returncode, proc.stdout, proc.stderr


def _gh_json(args: list[str], body: dict[str, Any] | None = None) -> Any:
    code, out, err = _gh(args, body)
    if code != 0:
        detail = (err or out).strip()
        raise RuntimeError(detail or f"gh api failed ({code})")
    if not out.strip():
        return None
    return json.loads(out)


def _require_repo(value: str) -> str:
    if not _REPO_RE.fullmatch(value):
        raise ValueError("repo")
    return value


def _require_run_id(value: str) -> str:
    if not re.fullmatch(r"[0-9]+", value):
        raise ValueError("run id")
    return value


def _require_attempt(value: str) -> int:
    if not re.fullmatch(r"[0-9]+", value):
        raise ValueError("attempt")
    return int(value)


def _require_token(value: str, label: str) -> str:
    if not _TOKEN_RE.fullmatch(value):
        raise ValueError(label)
    return value


def _require_branch(value: str) -> str:
    if not isinstance(value, str) or not value or len(value) > 256:
        raise ValueError("branch")
    if any(ord(char) < 32 for char in value) or ".." in value.split("/"):
        raise ValueError("branch")
    return value


def _require_sha(value: str) -> str:
    if not value:
        return ""
    if not re.fullmatch(r"[0-9a-fA-F]{40}", value):
        raise ValueError("sha")
    return value


def _require_run_url(url: str, repo: str) -> str:
    prefix = f"https://github.com/{repo}/actions/runs/"
    if not url.startswith(prefix):
        raise ValueError("run url")
    return url


def _require_text(value: str, label: str, limit: int) -> str:
    if not isinstance(value, str):
        raise ValueError(label)
    cleaned = " ".join(value.split())
    if not cleaned or len(cleaned) > limit:
        raise ValueError(label)
    return cleaned


def list_failed_jobs(repo: str, run_id: str) -> list[str]:
    payload = _gh_json(
        [
            "--paginate",
            "--slurp",
            f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100",
        ]
    )
    pages = payload if isinstance(payload, list) else [payload]
    return failed_job_names(pages)


def _ensure_label(repo: str) -> bool:
    code, out, err = _gh(
        ["--method", "POST", f"repos/{repo}/labels", "--input", "-"],
        {
            "name": LABEL,
            "color": "5319e7",
            "description": "Record of scoped auto-retries for allowlisted flaky jobs",
        },
    )
    if code == 0 or "already_exists" in (out + err):
        return True
    if "403" in (out + err) or "Resource not accessible" in (out + err):
        return False
    detail = (err or out).strip()
    raise RuntimeError(detail or "label create failed")


def _list_open_issues(repo: str, label_ready: bool) -> list[dict[str, Any]]:
    endpoint = f"repos/{repo}/issues?state=open&per_page=100"
    if label_ready:
        endpoint += f"&labels={LABEL}"
    payload = _gh_json(["--paginate", "--slurp", endpoint])
    pages = payload if isinstance(payload, list) else [payload]
    return [item for item in flatten_pages(pages, None) if isinstance(item, dict)]


def _comment(repo: str, number: int, body: str) -> None:
    _gh_json(
        ["--method", "POST", f"repos/{repo}/issues/{number}/comments", "--input", "-"],
        {"body": body},
    )


def _create_issue(repo: str, label_ready: bool) -> int:
    payload: dict[str, Any] = {
        "title": TRACKING_TITLE,
        "body": (
            "One issue for scoped auto-retry records. Each retry is a comment "
            "with the run URL, attempt, failed job names, and conclusion.\n\n"
            f"{TRACKING_MARKER}"
        ),
    }
    if label_ready:
        payload["labels"] = [LABEL]
    created = _gh_json(
        ["--method", "POST", f"repos/{repo}/issues", "--input", "-"],
        payload,
    )
    if not isinstance(created, dict) or not isinstance(created.get("number"), int):
        raise RuntimeError("issue create returned no number")
    return created["number"]


def _close_duplicate(repo: str, number: int, keeper: int) -> None:
    _comment(repo, number, f"Duplicate of #{keeper}.")
    _gh_json(
        ["--method", "PATCH", f"repos/{repo}/issues/{number}", "--input", "-"],
        {"state": "closed", "state_reason": "not_planned"},
    )


def find_or_create_tracking_issue(repo: str, record: str) -> int:
    label_ready = _ensure_label(repo)
    existing = select_tracking_issue(_list_open_issues(repo, label_ready))
    if existing is not None:
        _comment(repo, existing, record)
        return existing
    created = _create_issue(repo, label_ready)
    issues = _list_open_issues(repo, label_ready)
    keeper = select_tracking_issue(issues)
    if keeper is None:
        keeper = created
    if keeper != created:
        _comment(repo, keeper, record)
        _close_duplicate(repo, created, keeper)
        return keeper
    _comment(repo, created, record)
    return created


def find_open_pr(repo: str, head_sha: str, head_branch: str) -> int | None:
    numbers: list[int] = []
    if head_sha:
        payload = _gh_json([f"repos/{repo}/commits/{head_sha}/pulls"])
        if isinstance(payload, list):
            for pull in payload:
                if (
                    isinstance(pull, dict)
                    and pull.get("state") == "open"
                    and isinstance(pull.get("number"), int)
                ):
                    numbers.append(pull["number"])
    if numbers:
        return min(numbers)
    owner = repo.split("/", 1)[0]
    payload = _gh_json(
        [
            "--paginate",
            "--slurp",
            "--method",
            "GET",
            f"repos/{repo}/pulls",
            "-f",
            "state=open",
            "-f",
            "per_page=100",
            "-f",
            f"head={owner}:{head_branch}",
        ]
    )
    pages = payload if isinstance(payload, list) else [payload]
    for pull in flatten_pages(pages, None):
        if (
            isinstance(pull, dict)
            and pull.get("state") == "open"
            and isinstance(pull.get("number"), int)
        ):
            numbers.append(pull["number"])
    if not numbers:
        return None
    return min(numbers)


def rerun_failed_jobs(repo: str, run_id: str) -> None:
    _gh_json(
        ["--method", "POST", f"repos/{repo}/actions/runs/{run_id}/rerun-failed-jobs"]
    )


class GitHubTransport:
    def __init__(
        self,
        repo: str,
        event: str,
        head_branch: str,
        default_branch: str,
        head_sha: str,
        run_id: str,
    ) -> None:
        self.repo = repo
        self.event = event
        self.head_branch = head_branch
        self.default_branch = default_branch
        self.head_sha = head_sha
        self.run_id = run_id

    def skip(self, reason: str) -> None:
        print(f"auto-retry: not retrying ({reason})")

    def post_record(self, record: str) -> None:
        destination = record_destination(
            self.event, self.head_branch, self.default_branch
        )
        if destination == "pr":
            number = find_open_pr(self.repo, self.head_sha, self.head_branch)
            if number is not None:
                _comment(self.repo, number, record)
                print(f"auto-retry: commented on pull request #{number}")
                return
            print("auto-retry: no open pull request; using the tracking issue")
        number = find_or_create_tracking_issue(self.repo, record)
        print(f"auto-retry: recorded on issue #{number}")

    def rerun(self) -> None:
        rerun_failed_jobs(self.repo, self.run_id)
        print(f"auto-retry: rerunning failed jobs on run {self.run_id}")


def main() -> int:
    repo = _require_repo(os.environ["REPO"])
    attempt = _require_attempt(os.environ["UPSTREAM_ATTEMPT"])
    conclusion = _require_token(os.environ["UPSTREAM_CONCLUSION"], "conclusion")
    workflow = _require_text(os.environ["UPSTREAM_WORKFLOW_NAME"], "workflow", 200)
    run_id = _require_run_id(os.environ["UPSTREAM_RUN_ID"])
    run_url = _require_run_url(os.environ["UPSTREAM_HTML_URL"], repo)
    head_branch = _require_branch(os.environ["UPSTREAM_HEAD_BRANCH"])
    event = _require_token(os.environ["UPSTREAM_EVENT"], "event")
    head_sha = _require_sha(os.environ.get("UPSTREAM_HEAD_SHA", ""))
    default_branch = _require_branch(os.environ["DEFAULT_BRANCH"])
    allow_path = Path(os.environ.get("AUTO_RETRY_ALLOWLIST", ".github/flaky-jobs.json"))
    allow = load_allowlist(allow_path)
    failed = list_failed_jobs(repo, run_id)
    decision = decide(workflow, failed, allow, attempt, conclusion)
    record = build_record(run_url, attempt, failed, conclusion)
    transport = GitHubTransport(
        repo, event, head_branch, default_branch, head_sha, run_id
    )
    run_retry(decision, record, transport)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"auto-retry: {exc}", file=sys.stderr)
        sys.exit(1)

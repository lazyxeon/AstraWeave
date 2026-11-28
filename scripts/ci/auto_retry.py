#!/usr/bin/env python3
"""Decide and perform a scoped rerun of allowlisted flaky jobs.

The allowlist is .github/flaky-jobs.json. The denylist below is code.
cargo deny, cargo audit, Security Audit, replay hash, determinism-replay,
compare, workflow policy, and Gate never retry, even when the file names them.
A job retries only when it is allowlisted and its log matches a known
infrastructure signature: a lost runner or exit 143, a crates.io or network
timeout, or a failed artifact or cache download. A run retries only when
every candidate job qualifies and none is denied.

`--prune-older-shard-attempts <dir>` keeps the newest attempt of each
benchmark shard so a rerun does not double-count an artifact.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MAX_ATTEMPT = 3
LABEL = "auto-retry"
TRACKING_TITLE = "Auto-retry record"
TRACKING_MARKER = "Posted by `.github/workflows/auto-retry.yml`."
BOT_MARKER = "<!-- aw-bot: auto-retry -->"
CANDIDATE_CONCLUSIONS = frozenset({"failure", "timed_out"})

_REPO_RE = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
_TOKEN_RE = re.compile(r"[A-Za-z0-9_-]+")
_HASH_JOB_RE = re.compile(r"hash( \(.*\))?")
_GATE_RE = re.compile(r"gate(?:$|[^a-z0-9])")
_SHARD_DIR_RE = re.compile(r"-shard-(\d+)(?:-attempt-(\d+))?$")
_TIMESTAMP_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\S+Z\s*")
_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
_EXIT_143_RE = re.compile(r"exit code 143(?:\D|$)")
_SHARD_LINE_RE = re.compile(r"shard-\d+")
_SIGNATURE_RE = re.compile(r"[a-z0-9-]{1,40}")

# Lowercase lookalikes folded after NFKC and casefold. Fullwidth letters are
# already ASCII by then. Cyrillic а in "Gаte" is the case the denylist covers.
_HOMOGLYPHS = str.maketrans(
    {
        "а": "a",
        "е": "e",
        "о": "o",
        "р": "p",
        "с": "c",
        "у": "y",
        "х": "x",
        "і": "i",
        "ѕ": "s",
        "ј": "j",
        "һ": "h",
        "к": "k",
        "м": "m",
        "т": "t",
        "в": "b",
        "н": "h",
        "ο": "o",
        "α": "a",
        "ε": "e",
        "ι": "i",
        "ν": "v",
        "ρ": "p",
        "τ": "t",
        "υ": "u",
        "χ": "x",
        "ɡ": "g",
    }
)

_CRATES_FAIL = (
    "failed",
    "error",
    "timed out",
    "unreachable",
    "could not resolve",
    "connection reset",
    "429",
    "502",
    "503",
    "504",
)
_NETWORK_TIMEOUT = (
    "operation timed out",
    "connection timed out",
    "i/o timeout",
    "tls handshake timeout",
    "curl: (28)",
    "spurious network error (",
    "could not resolve host",
    "temporary failure in name resolution",
    "network is unreachable",
    "httpclient.timeout",
)
_CACHE_FAIL = (
    "failed to restore cache",
    "failed to download cache",
    "cache download failed",
    "failed to read cache",
)
_CACHE_RESTORE_NET = ("etimedout", "connection timed out", "502", "503", "504")


@dataclass(frozen=True)
class AllowJob:
    workflow: str
    job: str
    evidence: str


@dataclass(frozen=True)
class ObservedJob:
    name: str
    log: str = ""
    steps: tuple[tuple[str, str], ...] = ()
    conclusion: str = "failure"
    job_id: int = 0


@dataclass(frozen=True)
class Decision:
    retry: bool
    reason: str
    signatures: tuple[str, ...] = ()


def normalize(value: str) -> str:
    text = unicodedata.normalize("NFKC", value)  # norm:nfkc
    text = "".join(ch for ch in text if unicodedata.category(ch) != "Cf")  # norm:zero-width
    text = text.casefold()
    text = text.translate(_HOMOGLYPHS)  # norm:homoglyph
    text = text.replace("-", " ").replace("_", " ")  # norm:separators
    return " ".join(text.split())


def _contains_any(text: str, needles: tuple[str, ...]) -> bool:
    return any(needle in text for needle in needles)


def job_denied(workflow: str, job: str) -> bool:
    workflow_cf = normalize(workflow)
    job_cf = normalize(job)
    blob = workflow_cf + "\n" + job_cf
    if _contains_any(blob, ("determinism replay",)):
        return True  # deny:determinism-replay
    if "determinism validation" in blob:
        return True  # deny:determinism-validation
    if _contains_any(blob, ("security audit",)):
        return True  # deny:security-audit
    if "cargo deny" in job_cf:
        return True  # deny:cargo-deny
    if "cargo audit" in job_cf:
        return True  # deny:cargo-audit
    if _contains_any(job_cf, ("replay hash",)):
        return True  # deny:replay-hash
    if _HASH_JOB_RE.fullmatch(job_cf):
        return True  # deny:hash-job
    if job_cf == "compare":
        return True  # deny:compare
    if _contains_any(job_cf, ("workflow policy",)):
        return True  # deny:workflow-policy
    if _GATE_RE.match(job_cf):
        return True  # deny:gate
    if "pr rust" in job_cf:
        return True  # deny:pr-rust
    if "detect affected" in job_cf:
        return True  # deny:detect-affected
    if "minimum supported rust" in job_cf:
        return True  # deny:msrv
    if job_cf.startswith("test ("):
        return True  # deny:required-test
    return False


def log_lines(log: str) -> list[str]:
    lines: list[str] = []
    for raw in log.splitlines():
        line = _ANSI_RE.sub("", raw)
        line = _TIMESTAMP_RE.sub("", line).strip()
        lines.append(line)
    return lines


def infra_signature(log: str) -> str | None:
    """Return the infrastructure signature id, or None when the log is a real failure."""
    lines = log_lines(log)
    for index, line in enumerate(lines):
        low = line.casefold()
        if _EXIT_143_RE.search(low):
            return "exit-143"
        if (
            "runner has received a shutdown signal" in low
            or "lost communication with the" in low
            or "we stopped hearing from the runner" in low
        ):
            return "lost-runner"
        if "artifact download failed" in low or "unable to download artifact" in low:
            return "artifact-download"
        if line == "Missing shard artifacts:":
            nxt = lines[index + 1] if index + 1 < len(lines) else ""
            if _SHARD_LINE_RE.fullmatch(nxt):
                return "missing-shard-artifact"
        if "crates.io" in low and any(token in low for token in _CRATES_FAIL):
            return "crates-io"
        if any(phrase in low for phrase in _NETWORK_TIMEOUT):
            return "network-timeout"
        if any(phrase in low for phrase in _CACHE_FAIL):
            return "cache-download"
        if "failed to restore:" in low and any(token in low for token in _CACHE_RESTORE_NET):
            return "cache-download"
    return None


def strict_step_failed(steps: tuple[tuple[str, str], ...]) -> bool:
    for name, conclusion in steps:
        if "strict threshold" in normalize(name) and conclusion == "failure":
            return True
    return False


def decide(
    workflow: str,
    failed_jobs: list[ObservedJob],
    allow: list[AllowJob],
    attempt: int,
    conclusion: str,
) -> Decision:
    if conclusion not in CANDIDATE_CONCLUSIONS:  # failure-only
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
        if job_denied(workflow, job.name):
            denied = True  # deny-wins
        if (workflow, job.name) not in allowed:  # allowlist-only
            unlisted = True
    if denied:
        return Decision(False, "denylist")
    if unlisted:
        return Decision(False, "not-allowlisted")
    if any(strict_step_failed(job.steps) for job in failed_jobs):  # strict-exclusion
        return Decision(False, "strict-threshold")
    if any(infra_signature(job.log) is None for job in failed_jobs):  # infra-signature
        return Decision(False, "no-signature")
    signatures = tuple(infra_signature(job.log) or "" for job in failed_jobs)
    return Decision(True, "retry", signatures)


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


def candidate_jobs(payload: object) -> list[ObservedJob]:
    found: list[ObservedJob] = []
    for job in flatten_pages(payload, "jobs"):
        if not isinstance(job, dict) or job.get("conclusion") not in CANDIDATE_CONCLUSIONS:
            continue
        name = job.get("name")
        if not isinstance(name, str) or not name.strip():
            continue
        steps: list[tuple[str, str]] = []
        steps_raw = job.get("steps") or []
        if isinstance(steps_raw, list):
            for step in steps_raw:
                if not isinstance(step, dict):
                    continue
                step_name = step.get("name")
                if not isinstance(step_name, str):
                    continue
                step_conclusion = step.get("conclusion") or ""
                steps.append((step_name, str(step_conclusion)))
        job_id = job.get("id")
        found.append(
            ObservedJob(
                name.strip(),
                "",
                tuple(steps),
                str(job.get("conclusion")),
                job_id if isinstance(job_id, int) and not isinstance(job_id, bool) else 0,
            )
        )
    return found


def failed_job_names(payload: object) -> list[str]:
    return [job.name for job in candidate_jobs(payload) if job.conclusion == "failure"]


def build_record(
    run_url: str,
    attempt: int,
    failed_jobs: list[str],
    conclusion: str,
    signatures: tuple[str, ...] | list[str] = (),
) -> str:
    names = ", ".join(" ".join(job.split()) for job in failed_jobs) or "(none)"
    safe_signatures = [sig for sig in signatures if _SIGNATURE_RE.fullmatch(sig)]
    signature_text = ", ".join(safe_signatures) or "(none)"
    return "\n".join(
        [
            BOT_MARKER,
            "Auto-retry record.",
            "",
            f"- Run: {run_url}",
            f"- Attempt: {attempt}",
            f"- Failed jobs: {names}",
            f"- Conclusion: {conclusion}",
            f"- Signatures: {signature_text}",
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


def tracking_issue_body() -> str:
    return (
        f"{BOT_MARKER}\n"
        "One issue for scoped auto-retry records. Each retry is a comment "
        "with the run URL, attempt, failed job names, and conclusion.\n\n"
        f"{TRACKING_MARKER}"
    )


def duplicate_comment(keeper: int) -> str:
    return f"{BOT_MARKER}\nDuplicate of #{keeper}."


def run_retry(decision: Decision, record: str, transport: Any) -> None:
    if not decision.retry:
        transport.skip(decision.reason)
        return
    transport.post_record(record)  # record-before-rerun
    transport.rerun()


def repository_matches(head_repository: str, repo: str) -> bool:
    return bool(head_repository) and head_repository == repo


def select_open_pr(pulls: object, head_branch: str, repo: str) -> int | None:
    numbers: list[int] = []
    items = pulls if isinstance(pulls, list) else []
    for pull in items:
        if not isinstance(pull, dict) or pull.get("state") != "open":
            continue
        number = pull.get("number")
        if not isinstance(number, int) or isinstance(number, bool):
            continue
        head = pull.get("head")
        if not isinstance(head, dict) or head.get("ref") != head_branch:
            continue
        head_repo = head.get("repo")
        if not isinstance(head_repo, dict) or head_repo.get("full_name") != repo:
            continue
        numbers.append(number)
    if not numbers:
        return None
    return min(numbers)


def parse_run(payload: object) -> tuple[int, str, str]:
    if not isinstance(payload, dict):
        raise ValueError("run")
    attempt = payload.get("run_attempt")
    status = payload.get("status")
    conclusion = payload.get("conclusion") or ""
    if isinstance(attempt, bool) or not isinstance(attempt, int):
        raise ValueError("run attempt")
    if not isinstance(status, str) or not status:
        raise ValueError("run status")
    if not isinstance(conclusion, str):
        raise ValueError("run conclusion")
    return attempt, status, conclusion


def live_run_guard(event_attempt: int, live_attempt: int, live_status: str) -> str | None:
    if live_status != "completed":
        return "run-in-progress"
    if live_attempt != event_attempt:
        return "stale-attempt"
    if live_attempt < 1 or live_attempt >= MAX_ATTEMPT:
        return "attempt-cap"
    return None


def jobs_path(repo: str, run_id: str) -> str:
    return f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100&filter=latest"  # latest-attempt


def run_path(repo: str, run_id: str) -> str:
    return f"repos/{repo}/actions/runs/{run_id}"


def prune_older_shard_attempts(root: Path) -> list[str]:
    """Drop older attempt directories so each shard is counted once.

    Legacy directories with no attempt suffix count as attempt 1. The newest
    attempt is the one a rerun just uploaded. Earlier uploads stay in the
    Actions artifact store; this only prunes the workspace copy.
    """
    if not root.exists():
        return []
    if not root.is_dir():
        raise ValueError("shard root")
    groups: dict[int, list[tuple[int, Path]]] = {}
    for child in root.iterdir():
        if child.is_symlink() or not child.is_dir():
            continue
        match = _SHARD_DIR_RE.search(child.name)
        if not match:
            continue
        shard = int(match.group(1))
        attempt = int(match.group(2) or "1")
        groups.setdefault(shard, []).append((attempt, child))
    removed: list[str] = []
    for items in groups.values():
        items.sort(key=lambda item: (item[0], item[1].name))
        for _attempt, path in items[:-1]:
            shutil.rmtree(path)
            removed.append(path.name)
    removed.sort()
    return removed


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
        "body": tracking_issue_body(),
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
    _comment(repo, number, duplicate_comment(keeper))
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
    if head_sha:
        payload = _gh_json([f"repos/{repo}/commits/{head_sha}/pulls"])
        chosen = select_open_pr(payload if isinstance(payload, list) else [], head_branch, repo)
        if chosen is not None:
            return chosen
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
    flat = [item for item in flatten_pages(pages, None) if isinstance(item, dict)]
    return select_open_pr(flat, head_branch, repo)


def fetch_job_log(repo: str, job_id: int) -> str:
    if job_id <= 0:
        return ""
    code, out, err = _gh([f"repos/{repo}/actions/jobs/{job_id}/logs"])
    if code != 0:
        detail = err or out
        if "404" in detail:
            return ""
        raise RuntimeError(detail.strip() or "job log failed")
    return out


def rerun_failed_jobs(repo: str, run_id: str, event_attempt: int) -> None:
    payload = _gh_json([run_path(repo, run_id)])
    attempt, status, _conclusion = parse_run(payload)
    reason = live_run_guard(event_attempt, attempt, status)
    if reason:
        raise RuntimeError(reason)
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
        event_attempt: int,
    ) -> None:
        self.repo = repo
        self.event = event
        self.head_branch = head_branch
        self.default_branch = default_branch
        self.head_sha = head_sha
        self.run_id = run_id
        self.event_attempt = event_attempt

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
        rerun_failed_jobs(self.repo, self.run_id, self.event_attempt)
        print(f"auto-retry: rerunning failed jobs on run {self.run_id}")


def _with_log(job: ObservedJob, log: str) -> ObservedJob:
    return ObservedJob(job.name, log, job.steps, job.conclusion, job.job_id)


def _skip(reason: str) -> int:
    print(f"auto-retry: not retrying ({reason})")
    return 0


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if args and args[0] == "--prune-older-shard-attempts":
        if len(args) != 2:
            raise ValueError("prune path")
        prune_older_shard_attempts(Path(args[1]))
        return 0
    if args:
        raise ValueError("arguments")
    repo = _require_repo(os.environ["REPO"])
    attempt = _require_attempt(os.environ["UPSTREAM_ATTEMPT"])
    workflow = _require_text(os.environ["UPSTREAM_WORKFLOW_NAME"], "workflow", 200)
    run_id = _require_run_id(os.environ["UPSTREAM_RUN_ID"])
    run_url = _require_run_url(os.environ["UPSTREAM_HTML_URL"], repo)
    head_branch = _require_branch(os.environ["UPSTREAM_HEAD_BRANCH"])
    event = _require_token(os.environ["UPSTREAM_EVENT"], "event")
    head_sha = _require_sha(os.environ.get("UPSTREAM_HEAD_SHA", ""))
    head_repository = _require_repo(os.environ["UPSTREAM_HEAD_REPOSITORY"])
    default_branch = _require_branch(os.environ["DEFAULT_BRANCH"])
    if not repository_matches(head_repository, repo):
        return _skip("fork-run")
    live = _gh_json([run_path(repo, run_id)])
    live_attempt, live_status, live_conclusion = parse_run(live)
    guard = live_run_guard(attempt, live_attempt, live_status)
    if guard:
        return _skip(guard)
    allow_path = Path(os.environ.get("AUTO_RETRY_ALLOWLIST", ".github/flaky-jobs.json"))
    allow = load_allowlist(allow_path)
    payload = _gh_json(["--paginate", "--slurp", jobs_path(repo, run_id)])
    pages = payload if isinstance(payload, list) else [payload]
    observed = [
        _with_log(job, fetch_job_log(repo, job.job_id)) for job in candidate_jobs(pages)
    ]
    decision = decide(workflow, observed, allow, live_attempt, live_conclusion)
    record = build_record(
        run_url,
        live_attempt,
        [job.name for job in observed],
        live_conclusion,
        decision.signatures,
    )
    transport = GitHubTransport(
        repo,
        event,
        head_branch,
        default_branch,
        head_sha,
        run_id,
        live_attempt,
    )
    run_retry(decision, record, transport)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"auto-retry: {exc}", file=sys.stderr)
        sys.exit(1)

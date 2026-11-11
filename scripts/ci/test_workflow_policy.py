#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

import fnmatch
import gzip
import hashlib
import io
import http.server
import inspect
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import threading
import unittest

import yaml
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
sys.path.insert(0, str(ROOT / "scripts" / "ci"))
import mutation_ai_outcomes  # noqa: E402

CONCURRENCY_GROUP = "group: ${{ github.workflow }}-${{ github.ref }}"
CONCURRENCY_CANCEL = "cancel-in-progress: ${{ github.event_name == 'pull_request' }}"

POLICY_ACTIONS = (
    "actions/checkout",
    "actions/upload-artifact",
    "actions/download-artifact",
    "actions/cache",
    "actions/github-script",
)
EXPECTED_POLICY_ACTIONS = frozenset(
    {
        "actions/checkout",
        "actions/upload-artifact",
        "actions/download-artifact",
        "actions/cache",
        "actions/github-script",
    }
)
ACTION_MAJOR_FLOORS: dict[str, int] = {
    "actions/checkout": 7,
    "actions/upload-artifact": 4,
    "actions/download-artifact": 4,
    "actions/cache": 4,
    "actions/github-script": 7,
}
PINNED_ACTION_MAJOR_FLOORS: dict[str, int] = {
    "actions/checkout": 7,
    "actions/upload-artifact": 4,
    "actions/download-artifact": 4,
    "actions/cache": 4,
    "actions/github-script": 7,
}
# Hardcoded refs one major below each pinned floor (not derived from ACTION_MAJOR_FLOORS).
BELOW_FLOOR_MAJOR_TAG_BY_ACTION: dict[str, str] = {
    "actions/checkout": "v6",
    "actions/upload-artifact": "v3",
    "actions/download-artifact": "v3",
    "actions/cache": "v3",
    "actions/github-script": "v6",
}
AT_FLOOR_MAJOR_TAG_BY_ACTION: dict[str, str] = {
    "actions/checkout": "v7",
    "actions/upload-artifact": "v4",
    "actions/download-artifact": "v4",
    "actions/cache": "v4",
    "actions/github-script": "v7",
}
_POLICY_ACTION_PATTERN = "|".join(re.escape(action) for action in POLICY_ACTIONS)
ACTION_USES_RE = re.compile(
    rf"uses:\s*(?P<action>{_POLICY_ACTION_PATTERN})@(?P<ref>[^\s#]+)"
)
UNPINNED_BRANCH_REFS = frozenset({"main", "master", "HEAD", "latest", "release"})
UNPINNED_REF_MSG = "unpinned action ref"
NON_SHA_TAG_MSG = "must be a 40-char SHA or major tag vN"
BELOW_FLOOR_MSG = "below major floor"


def action_ref_violation(action: str, ref: str) -> str | None:
    if action not in ACTION_MAJOR_FLOORS:
        return None
    if (
        not ref
        or ref in UNPINNED_BRANCH_REFS
        or "/" in ref
        or ref.startswith("refs/")
        or ref.startswith(".")
    ):
        return f"{UNPINNED_REF_MSG} {action}@{ref!r}"
    if re.fullmatch(r"[0-9a-f]{40}", ref):
        return None
    major = re.fullmatch(r"v([0-9]+)", ref)
    if major:
        floor = ACTION_MAJOR_FLOORS[action]
        if int(major.group(1)) < floor:
            return f"{action}@{ref} {BELOW_FLOOR_MSG} v{floor}"
        return None
    return f"{action}@{ref} {NON_SHA_TAG_MSG}"


def workflow_action_pin_violations(text: str, *, path: str = "") -> list[str]:
    violations: list[str] = []
    prefix = f"{path}: " if path else ""
    for line_no, line in enumerate(text.splitlines(), start=1):
        match = ACTION_USES_RE.search(line)
        if not match:
            continue
        violation = action_ref_violation(match.group("action"), match.group("ref"))
        if violation:
            violations.append(f"{prefix}{line_no}: {violation}")
    return violations


def find_action_uses_pos(block: str, action: str) -> int:
    pos = -1
    for match in ACTION_USES_RE.finditer(block):
        if match.group("action") == action:
            pos = match.start()
    return pos


def summary_checkout_step_line(text: str) -> str:
    summary = mutation_summary_job(text)
    for line in summary.splitlines():
        if "uses: actions/checkout@" in line:
            return line
    raise AssertionError("mutants-summary missing checkout step")


def _exec_action_ref_violation_from_source(source: str):
    namespace: dict[str, object] = {
        "re": re,
        "ACTION_MAJOR_FLOORS": ACTION_MAJOR_FLOORS,
        "UNPINNED_BRANCH_REFS": UNPINNED_BRANCH_REFS,
        "UNPINNED_REF_MSG": UNPINNED_REF_MSG,
        "NON_SHA_TAG_MSG": NON_SHA_TAG_MSG,
        "BELOW_FLOOR_MSG": BELOW_FLOOR_MSG,
    }
    exec(compile(source, "<action_ref_violation>", "exec"), namespace)
    fn = namespace.get("action_ref_violation")
    if not callable(fn):
        raise RuntimeError("action_ref_violation missing after exec")
    return fn


def _broken_action_ref_violation(old: str, new: str):
    source = inspect.getsource(action_ref_violation).replace(old, new, 1)
    return _exec_action_ref_violation_from_source(source)


def workflow_texts() -> dict[str, str]:
    return {path.name: path.read_text(encoding="utf-8") for path in sorted(WORKFLOWS.glob("*.yml"))}


def workflow_display_names() -> set[str]:
    names: set[str] = set()
    name_re = re.compile(r"^name:\s*(.+?)\s*$")
    for text in workflow_texts().values():
        for line in text.splitlines():
            match = name_re.match(line)
            if not match:
                continue
            value = match.group(1).strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
                value = value[1:-1]
            names.add(value)
            break
    return names


def nightly_failure_watched_workflows(text: str) -> list[str]:
    watched: list[str] = []
    in_list = False
    for line in text.splitlines():
        if re.match(r"^\s+workflows:\s*$", line):
            in_list = True
            continue
        if not in_list:
            continue
        item = re.match(r"^\s+-\s+(.+)\s*$", line)
        if item:
            watched.append(item.group(1).strip())
            continue
        if line.strip():
            break
    return watched


def has_pull_request(text: str) -> bool:
    # Quoted "on" (scorecard) and bare on: both count.
    in_on = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped in {"on:", '"on":'}:
            in_on = True
            continue
        if in_on and line and not line.startswith(" ") and not line.startswith("#"):
            in_on = False
        if in_on and stripped.startswith("pull_request:"):
            return True
    return False


def jobs_missing_timeout(text: str) -> list[str]:
    lines = text.splitlines()
    in_jobs = False
    current = None
    has_timeout = False
    missing: list[str] = []

    def flush() -> None:
        nonlocal current, has_timeout
        if current and not has_timeout:
            missing.append(current)
        current = None
        has_timeout = False

    for line in lines:
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        if line and not line.startswith(" ") and not line.startswith("#"):
            flush()
            break
        if (
            line.startswith("  ")
            and not line.startswith("   ")
            and line.rstrip().endswith(":")
            and not line.strip().startswith("#")
        ):
            flush()
            current = line.strip()[:-1]
            has_timeout = False
            continue
        if current and "timeout-minutes:" in line:
            has_timeout = True
    flush()
    return missing


MUTATION_TESTING = WORKFLOWS / "mutation-testing.yml"
VERIFY_BASELINE_ERROR = "baseline did not complete successfully"
VERIFY_NO_MUTANTS_ERROR = "no mutant outcomes recorded after baseline"
VERIFY_END_TIME_ERROR = "cargo-mutants did not finish (end_time is null)"
VERIFY_INCOMPLETE_ERROR = "incomplete shard:"


def mutation_testing_text() -> str:
    return MUTATION_TESTING.read_text(encoding="utf-8")


def mutation_ai_job(text: str) -> str:
    return text.split("  mutants-ai:\n", 1)[1].split("\n  mutants-behavior:\n", 1)[0]


def mutation_summary_job(text: str) -> str:
    return text.split("  mutants-summary:\n", 1)[1]


def job_step_block(job: str, step_name: str) -> str:
    marker = f"- name: {step_name}"
    start = job.find(marker)
    if start < 0:
        raise AssertionError(f"missing step {step_name!r}")
    rest = job[start + len(marker) :]
    next_step = rest.find("\n      - ")
    if next_step < 0:
        return rest
    return rest[:next_step]


def _run_script(step_block: str) -> str:
    if "run:" not in step_block:
        return ""
    return step_block.split("run:", 1)[1]


def _python_run_lines(run: str) -> list[str]:
    return [
        line
        for line in run.splitlines()
        if "python3 scripts/ci/mutation_ai_outcomes.py" in line
    ]


# actions/runner ScriptHandler.FixUpScriptContents for `shell: pwsh`, then
# `pwsh -command ". '{0}'"`. Issue #693: a bare dot-source is not that wrapper.
_RUNNER_PWSH_PREAMBLE = "$ErrorActionPreference = 'stop'\n"
_RUNNER_PWSH_EPILOGUE = (
    "if ((Test-Path -LiteralPath variable:\\LASTEXITCODE)) { exit $LASTEXITCODE }\n"
)
_STATUS_FN_RE = re.compile(r"\b(?:success|always|cancelled|failure)\s*\(")
_STATUS_GUARD_RE = re.compile(
    r"(?:!\s*cancelled|always|failure|success|cancelled)\s*\(\s*\)\s*(?:&&|\|\|)\s*"
)
_STEP_OUTPUT_RE = re.compile(
    r"steps\.([A-Za-z_][A-Za-z0-9_]*)\.outputs\.([A-Za-z_][A-Za-z0-9_]*)"
)
_ALERT_ISSUE_STEPS = (
    "Create GitHub issue for regression",
    "Upload regression report",
)


def pwsh_run_body(step_block: str) -> str:
    if "run: |" not in step_block:
        raise AssertionError("step has no literal run block")
    lines = step_block.split("run: |", 1)[1].splitlines()
    while lines and not lines[0].strip():
        lines.pop(0)
    indents = [len(line) - len(line.lstrip(" ")) for line in lines if line.strip()]
    if not indents:
        raise AssertionError("empty pwsh run block")
    cut = min(indents)
    body = "\n".join(line[cut:] if line.strip() else "" for line in lines)
    return body.rstrip() + "\n"


def step_if_expression(step_block: str) -> str:
    for line in step_block.splitlines():
        stripped = line.strip()
        if not stripped.startswith("if:"):
            continue
        value = stripped[3:].strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        return value
    raise AssertionError("step is missing if:")


def github_step_if_runs(
    expr: str,
    *,
    failed: bool,
    cancelled: bool,
    outputs: dict[tuple[str, str], str],
) -> bool:
    """Evaluate a step `if:` the way Actions does, including implicit success()."""
    raw = expr.strip()
    if raw.startswith("${{") and raw.endswith("}}"):
        raw = raw[3:-2].strip()
    if _STATUS_FN_RE.search(raw) is None:
        raw = f"success() && ({raw})"

    def lookup(match: re.Match[str]) -> str:
        return repr(outputs.get((match.group(1), match.group(2)), ""))

    translated = _STEP_OUTPUT_RE.sub(lookup, raw)
    translated = translated.replace("&&", " and ").replace("||", " or ")
    translated = re.sub(r"!(?!=)", " not ", translated)
    translated = re.sub(r"\bcancelled\s*\(", "cancelled_fn(", translated)
    if not re.fullmatch(r"[A-Za-z0-9_ \t\(\)\'\"=<>!]+", translated):
        raise AssertionError(f"unsupported if expression: {expr!r} -> {translated!r}")

    def success() -> bool:
        return (not failed) and (not cancelled)

    def always() -> bool:
        return True

    def cancelled_fn() -> bool:
        return cancelled

    def failure() -> bool:
        return failed

    try:
        value = eval(  # noqa: S307 — workflow if, builtins removed
            translated,
            {"__builtins__": {}},
            {
                "success": success,
                "always": always,
                "cancelled_fn": cancelled_fn,
                "failure": failure,
                "True": True,
                "False": False,
            },
        )
    except Exception as exc:
        raise AssertionError(f"could not evaluate {expr!r}: {exc}") from exc
    if not isinstance(value, bool):
        raise AssertionError(f"if expression did not return a bool: {expr!r}")
    return value


def without_status_guard(expr: str) -> str:
    stripped = expr
    for _ in range(4):
        updated = _STATUS_GUARD_RE.sub("", stripped, count=1)
        if updated == stripped:
            break
        stripped = updated
    if stripped == expr:
        raise AssertionError(f"status guard was not removed from {expr!r}")
    return stripped


def assert_alert_followup_runs(text: str) -> None:
    """Issue and upload steps run after a failed check that set has_regressions."""
    outputs = {("regression_check", "has_regressions"): "true"}
    for name in _ALERT_ISSUE_STEPS:
        block = job_step_block(text, name)
        if "continue-on-error" in block:
            raise AssertionError(f"{name} must not use continue-on-error")
        expr = step_if_expression(block)
        runs = github_step_if_runs(
            expr, failed=True, cancelled=False, outputs=outputs
        )
        if not runs:
            raise AssertionError(
                f"{name} if-guard does not run after regression_check fails "
                "with has_regressions=true"
            )
        skipped = github_step_if_runs(
            expr,
            failed=True,
            cancelled=False,
            outputs={("regression_check", "has_regressions"): "false"},
        )
        if skipped:
            raise AssertionError(f"{name} runs when has_regressions is false")


def run_alert_check_in_runner(body: str, mode: str) -> tuple[int, str, bool]:
    """Run the check step body inside the real runner pwsh wrapper."""
    if shutil.which("pwsh") is None:
        raise AssertionError("pwsh is required to run the Actions runner wrapper")
    with tempfile.TemporaryDirectory(prefix="benchmark-alert-") as raw:
        workspace = Path(raw)
        scripts = workspace / "scripts"
        scripts.mkdir()
        shutil.copy(
            ROOT / "scripts" / "check_benchmark_thresholds.ps1",
            scripts / "check_benchmark_thresholds.ps1",
        )
        github = workspace / ".github"
        github.mkdir()
        (github / "benchmark_thresholds.json").write_text(
            json.dumps(
                {
                    "benchmarks": {
                        "sample": {
                            "baseline": 100,
                            "max_allowed": 150,
                            "unit": "ns",
                        }
                    }
                }
            ),
            encoding="utf-8",
        )
        results = workspace / "benchmark_results"
        results.mkdir()
        if mode == "regression":
            payload = [{"name": "sample", "value": 200, "unit": "ns"}]
        elif mode == "clean":
            payload = [{"name": "sample", "value": 100, "unit": "ns"}]
        elif mode == "checker_error":
            payload = None
        else:
            raise AssertionError(f"unknown alert fixture {mode}")
        if payload is not None:
            (results / "benchmarks.json").write_text(
                json.dumps(payload), encoding="utf-8"
            )
        script_path = workspace / "runner-step.ps1"
        contents = _RUNNER_PWSH_PREAMBLE + body + _RUNNER_PWSH_EPILOGUE
        script_path.write_text(contents, encoding="utf-8")
        github_output = workspace / "github_output.txt"
        github_output.write_text("", encoding="utf-8")
        env = os.environ.copy()
        env["GITHUB_OUTPUT"] = str(github_output)
        env["GITHUB_REF_NAME"] = "main"
        command = f". '{script_path}'"
        proc = subprocess.run(
            ["pwsh", "-command", command],
            cwd=workspace,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        output = github_output.read_text(encoding="utf-8")
        report = (workspace / "regression_report.md").is_file()
        if mode == "clean" and proc.returncode != 0:
            detail = (proc.stdout + "\n" + proc.stderr)[-2000:]
            raise AssertionError(f"clean check exited {proc.returncode}: {detail}")
        return proc.returncode, output, report


def assert_mutation_summary_requires_ai_success(text: str) -> None:
    summary = mutation_summary_job(text)
    report = job_step_block(summary, "Generate summary report")
    run = _run_script(report)
    if 'needs.mutants-ai.result }}" != "success"' not in run:
        raise AssertionError(
            "mutants-summary must fail when needs.mutants-ai.result is not success"
        )


def assert_mutation_ai_outcome_steps_not_softened(text: str) -> None:
    ai = mutation_ai_job(text)
    for step_name in ("Verify astraweave-ai shard outcomes", "Generate mutation report"):
        block = job_step_block(ai, step_name)
        if "continue-on-error: true" in block:
            raise AssertionError(f"{step_name} must not use continue-on-error")
        for line in _python_run_lines(_run_script(block)):
            if "|| true" in line:
                raise AssertionError(f"{step_name} must not mask failures with || true")
    summary = mutation_summary_job(text)
    report = job_step_block(summary, "Generate summary report")
    if "continue-on-error: true" in report:
        raise AssertionError("summary report must not use continue-on-error")
    for line in _python_run_lines(_run_script(report)):
        if "|| true" in line:
            raise AssertionError("summary report must not mask python failures with || true")


def assert_mutation_summary_checks_out_before_scoring(text: str) -> None:
    summary = mutation_summary_job(text)
    checkout = find_action_uses_pos(summary, "actions/checkout")
    scoring = summary.find("mutation_ai_outcomes.py")
    if checkout < 0:
        raise AssertionError("mutants-summary must checkout the repository")
    if scoring < 0:
        raise AssertionError("mutants-summary must score astraweave-ai outcomes")
    if checkout > scoring:
        raise AssertionError("checkout must precede mutation_ai_outcomes.py in mutants-summary")


PINNED_SCCACHE_SHA256 = {
    "sccache-v0.19.0-x86_64-unknown-linux-musl.tar.gz":
        "aa73c7270c997894038c0d8f0d297865f57569e1687883322bc431713d4bb9dd",
    "sccache-v0.19.0-aarch64-unknown-linux-musl.tar.gz":
        "3e47fc4a71b1ddd1b4294a0de9d34498922fdf41e3cd0bda5df3954e3d6deb9d",
    "sccache-v0.19.0-armv7-unknown-linux-musleabi.tar.gz":
        "2a478c2e5006af97b7cfe08b3b4cb02a3770b06aade91f8c03fc99b05edba041",
    "sccache-v0.19.0-x86_64-apple-darwin.tar.gz":
        "2db7344bb96de350df79cd9f0124dd4e9baa47b3de2fac1e63e1346b66b20684",
    "sccache-v0.19.0-aarch64-apple-darwin.tar.gz":
        "a35a0b7d622618774bc1bb8102d72562dfb21e619f88b153b370d7f89bc286af",
    "sccache-v0.19.0-x86_64-pc-windows-msvc.tar.gz":
        "130e4f8eb672e76590e9993c2ab890c4763b88238577bb11d8062158c7698353",
    "sccache-v0.19.0-aarch64-pc-windows-msvc.tar.gz":
        "a680e8e6058b033cd6ff56d56b2219ca13f1c1a45f50fb3b04f4ed80851f5c2b",
}


def assert_mutation_ai_scoring_uses_strict_eval(text: str) -> None:
    ai = mutation_ai_job(text)
    report = job_step_block(ai, "Generate mutation report")
    run = report.split("run:", 1)[1]
    if 'eval "$(python3 scripts/ci/mutation_ai_outcomes.py' in run:
        raise AssertionError("shard report must not use unchecked eval $(python3 …)")
    if "counts=$(python3 scripts/ci/mutation_ai_outcomes.py" not in run:
        raise AssertionError("shard report must capture python output before eval")
    if ': "${DECIDED:?}"' not in run or '"${SCORE:?}"' not in run:
        raise AssertionError("shard report must require DECIDED and SCORE")
    summary = mutation_summary_job(text)
    report = job_step_block(summary, "Generate summary report")
    run = report.split("run:", 1)[1]
    if 'eval "$(python3 scripts/ci/mutation_ai_outcomes.py' in run:
        raise AssertionError("summary must not use unchecked eval $(python3 …)")
    if "counts=$(python3 scripts/ci/mutation_ai_outcomes.py" not in run:
        raise AssertionError("summary must capture python output before eval")
    if ': "${DECIDED:?}"' not in run or '"${SCORE:?}"' not in run:
        raise AssertionError("summary must require DECIDED and SCORE")


class _FixedBodyHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        server = self.server
        assert isinstance(server, http.server.ThreadingHTTPServer)
        hits = getattr(server, "hits")
        hits.append(self.path)
        if self.path.endswith(".tar.gz"):
            body = getattr(server, "body")
            code = getattr(server, "status_code")
        else:
            body, code = b"missing", 404
        self.send_response(code)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args: object) -> None:
        return


@contextmanager
def _fixed_body_server(body: bytes, status: int = 200):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _FixedBodyHandler)
    setattr(server, "hits", [])
    setattr(server, "body", body)
    setattr(server, "status_code", status)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server
    finally:
        server.shutdown()
        server.server_close()


def _host_sccache_filename() -> tuple[str, str]:
    arch = subprocess.check_output(["node", "-p", "process.arch"], text=True).strip()
    platform = subprocess.check_output(["node", "-p", "process.platform"], text=True).strip()
    asset_arch = {"x64": "x86_64", "arm64": "aarch64", "arm": "armv7"}[arch]
    if platform == "darwin":
        asset_os = "apple-darwin"
    elif platform == "win32":
        asset_os = "pc-windows-msvc"
    elif arch == "arm":
        asset_os = "unknown-linux-musleabi"
    else:
        asset_os = "unknown-linux-musl"
    return f"sccache-v0.19.0-{asset_arch}-{asset_os}.tar.gz", arch


def _write_exe(directory: Path, name: str, body: str) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / name
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)


def _octal_field(value: int, length: int) -> bytes:
    text = f"{value:o}".encode()
    if len(text) >= length:
        raise AssertionError(f"octal field overflow {value}")
    return text.rjust(length - 1, b"0") + b"\0"


def _ustar_header(
    name: bytes,
    typ: int,
    size: int,
    linkname: bytes = b"",
    *,
    signed: bool = False,
    prefix: bytes = b"",
    size_field: bytes | None = None,
) -> bytes:
    header = bytearray(512)
    header[0 : len(name)] = name
    header[100:108] = _octal_field(0o755, 8)
    header[108:116] = _octal_field(0, 8)
    header[116:124] = _octal_field(0, 8)
    if size_field is None:
        header[124:136] = _octal_field(size, 12)
    else:
        if len(size_field) != 12:
            raise AssertionError(len(size_field))
        header[124:136] = size_field
    header[136:148] = _octal_field(0, 12)
    header[148:156] = b"        "
    header[156] = typ
    header[157 : 157 + len(linkname)] = linkname
    header[257:262] = b"ustar"
    header[263:265] = b"00"
    if prefix:
        if len(prefix) > 155:
            raise AssertionError(len(prefix))
        header[345 : 345 + len(prefix)] = prefix
    total = 0
    for index, byte in enumerate(header):
        value = 32 if 148 <= index < 156 else byte
        if signed and value >= 128:
            value -= 256
        total += value
    if signed:
        total &= 0xFFFFFFFF
    header[148:156] = f"{total:06o}".encode() + b"\0 "
    return bytes(header)


def _gzip_tar(parts: list[bytes]) -> bytes:
    return gzip.compress(b"".join(parts) + (b"\0" * 1024))


def _pad_block(payload: bytes) -> bytes:
    extra = (-len(payload)) % 512
    return payload + (b"\0" * extra)


_MATERIALIZING_TAR = """#!/bin/sh
real=/usr/bin/tar
extract=0
for arg in "$@"; do
  case "$arg" in
    -*x*) extract=1 ;;
  esac
done
if [ "$extract" -eq 0 ]; then
  exec "$real" "$@"
fi
"$real" "$@" || exit $?
dest=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-C" ]; then
    dest=$arg
  fi
  prev=$arg
done
if [ -n "$dest" ]; then
  find "$dest" \\( -type l -o -type p -o -type b -o -type c \\) -print | while IFS= read -r entry; do
    rm -f "$entry"
    cp "$AW_OUTSIDE" "$entry" 2>/dev/null || printf '%s\\n' 'materialized' > "$entry"
  done
fi
exit 0
"""


def _valid_sccache_archive(directory: Path) -> bytes:
    filename, _arch = _host_sccache_filename()
    member = filename[: -len(".tar.gz")] + "/sccache"
    payload = b"#!/bin/sh\necho sccache 0.19.0\n"
    source = directory / "sccache-payload"
    source.write_bytes(payload)
    archive_path = directory / filename
    with tarfile.open(archive_path, "w:gz") as archive:
        info = tarfile.TarInfo(name=member)
        info.size = len(payload)
        info.mode = 0o755
        with source.open("rb") as handle:
            archive.addfile(info, handle)
    return archive_path.read_bytes()


def _non_regular_sccache_archive(directory: Path, kind: str) -> bytes:
    filename, _arch = _host_sccache_filename()
    root = filename[: -len(".tar.gz")]
    member = f"{root}/sccache"
    payload = b"#!/bin/sh\necho sccache 0.19.0\n"
    archive_path = directory / f"{kind}-{filename}"
    with tarfile.open(archive_path, "w:gz") as archive:
        if kind == "symlink":
            regular = tarfile.TarInfo(name=f"{root}/regular-file")
            regular.size = len(payload)
            regular.mode = 0o755
            archive.addfile(regular, io.BytesIO(payload))
            link = tarfile.TarInfo(name=member)
            link.type = tarfile.SYMTYPE
            link.linkname = "regular-file"
            archive.addfile(link)
        elif kind == "hardlink":
            regular = tarfile.TarInfo(name=f"{root}/regular-file")
            regular.size = len(payload)
            regular.mode = 0o755
            archive.addfile(regular, io.BytesIO(payload))
            link = tarfile.TarInfo(name=member)
            link.type = tarfile.LNKTYPE
            link.linkname = f"{root}/regular-file"
            archive.addfile(link)
        elif kind == "directory":
            info = tarfile.TarInfo(name=member)
            info.type = tarfile.DIRTYPE
            info.mode = 0o755
            archive.addfile(info)
        else:
            raise AssertionError(kind)
    return archive_path.read_bytes()


def _write_rm_that_refuses_tree_wipe(directory: Path) -> None:
    _write_exe(
        directory,
        "rm",
        """#!/bin/sh
if [ "$1" = "-rf" ] && [ "$#" -eq 2 ]; then
  case "$2" in
    */extract|*/extract/)
      exec /bin/rm "$@"
      ;;
  esac
  exit 1
fi
exec /bin/rm "$@"
""",
    )


def _plant_decoy(directory: Path, name: str, log: Path) -> None:
    quoted_name = shlex.quote(name)
    quoted_log = shlex.quote(str(log))
    _write_exe(
        directory,
        name,
        "#!/bin/sh\n"
        f"printf '%s\\n' {quoted_name} >> {quoted_log}\n"
        "exit 1\n",
    )


def _force_missing_linux_tool(script: Path, name: str) -> None:
    text = script.read_text(encoding="utf-8")
    token = f'fail_closed "{name} is missing"'
    index = text.rfind(token)
    if index < 0:
        raise AssertionError(name)
    line_start = text.rfind("\n", 0, index) + 1
    prefix, _rest = text[line_start:index].split("$(pick_tool ", 1)
    missing = script.parent / f"missing-{name}"
    replacement = f"{prefix}$(pick_tool {missing})\" || "
    script.write_text(text[:line_start] + replacement + text[index:], encoding="utf-8")


def _bind_tool(script: Path, literal: str, replacement: Path) -> None:
    text = script.read_text(encoding="utf-8")
    index = text.rfind(literal)
    if index < 0:
        raise AssertionError(literal)
    script.write_text(
        text[:index] + str(replacement) + text[index + len(literal) :],
        encoding="utf-8",
    )


def _script_copy(directory: Path) -> Path:
    source = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"
    dest = directory / "fetch.sh"
    dest.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    return dest


def _script_pinned_to(directory: Path, digest: str) -> Path:
    filename, _arch = _host_sccache_filename()
    return _script_pinned_named(directory, filename, digest)


def _script_pinned_named(directory: Path, filename: str, digest: str) -> Path:
    pinned = PINNED_SCCACHE_SHA256[filename]
    text = (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh").read_text(encoding="utf-8")
    patched = directory / "fetch.sh"
    patched.write_text(text.replace(pinned, digest, 1), encoding="utf-8")
    return patched


def _packed_archive(
    directory: Path,
    members: list[tuple[str, str, object]],
    *,
    filename: str | None = None,
) -> bytes:
    """Gzip of (path, kind, payload) members under one top-level directory.

    kind is file, dir, symlink, fifo, or link-top. link-top makes the top-level
    name itself a symlink and ignores path. payload is bytes for a file and a
    link target string otherwise.
    """
    filename = filename or _host_sccache_filename()[0]
    root = filename[: -len(".tar.gz")]
    archive_path = directory / filename
    with tarfile.open(archive_path, "w:gz") as archive:
        for rel, kind, payload in members:
            if kind == "link-top":
                info = tarfile.TarInfo(name=root)
                info.type = tarfile.SYMTYPE
                info.linkname = str(payload)
                archive.addfile(info)
                continue
            info = tarfile.TarInfo(name=f"{root}/{rel}")
            if kind == "file":
                data = payload if isinstance(payload, bytes) else str(payload).encode()
                info.size = len(data)
                info.mode = 0o755
                archive.addfile(info, io.BytesIO(data))
            elif kind == "dir":
                info.type = tarfile.DIRTYPE
                info.mode = 0o755
                archive.addfile(info)
            elif kind == "symlink":
                info.type = tarfile.SYMTYPE
                info.linkname = str(payload)
                archive.addfile(info)
            elif kind == "fifo":
                info.type = tarfile.FIFOTYPE
                info.mode = 0o644
                archive.addfile(info)
            else:
                raise AssertionError(kind)
    return archive_path.read_bytes()


def _assert_prefetch_fell_back(
    test: unittest.TestCase,
    result: subprocess.CompletedProcess[str],
    output: str,
    work: Path,
    arch: str,
    *,
    message: str,
) -> None:
    marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
    dest = work / "tool-cache" / "sccache" / "0.19.0" / arch
    test.assertEqual(result.returncode, 0, result.stderr)
    test.assertEqual(output, "available=false\n")
    test.assertNotIn("available=true", output)
    test.assertIn(message, result.stderr)
    test.assertNotIn("checksum mismatch", result.stderr)
    test.assertFalse(marker.exists())
    test.assertFalse(dest.exists())


def _reader_status(work: Path, blob: bytes) -> int:
    archive = work / "reader.tar.gz"
    archive.write_bytes(blob)
    script = _script_copy(work)
    env = os.environ.copy()
    env["SCCACHE_READER_CHECK"] = str(archive)
    env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
    env["GITHUB_OUTPUT"] = str(work / "output")
    result = subprocess.run(
        ["bash", str(script)],
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    return result.returncode


def _fetch_blob(
    work: Path,
    blob: bytes,
    *,
    path_prefix: Path | None = None,
    extra_env: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], str]:
    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
    with _fixed_body_server(blob, status=200) as server:
        port = server.server_address[1]
        return _run_fetch(
            work,
            script=script,
            base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
            path_prefix=path_prefix,
            extra_env=extra_env,
        )


def _disable_incremental_steps(text: str) -> list[str]:
    marker = "- name: Disable incremental compilation\n"
    steps: list[str] = []
    rest = text
    while marker in rest:
        rest = rest.split(marker, 1)[1]
        lines = rest.splitlines(keepends=True)
        body: list[str] = []
        index = 0
        for index, line in enumerate(lines):
            if line.strip().startswith("- name:"):
                break
        else:
            index = len(lines)
        body = lines[:index]
        rest = "".join(lines[index:])
        steps.append("".join(body))
    return steps


def _assert_incremental_steps_have_no_if(text: str) -> None:
    steps = _disable_incremental_steps(text)
    if not steps:
        raise AssertionError("missing Disable incremental compilation step")
    for step in steps:
        if "CARGO_INCREMENTAL=0" not in step:
            raise AssertionError("incremental step must write CARGO_INCREMENTAL=0")
        if re.search(r"(?m)^\s*if:", step):
            raise AssertionError("incremental step must not have an if")


def _run_fetch(
    work: Path,
    *,
    script: Path | None = None,
    base: str | None = None,
    attempts: str = "1",
    delay: str = "0",
    path_prefix: Path | None = None,
    tool_cache: Path | None = None,
    extra_env: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], str]:
    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)
    env["GITHUB_OUTPUT"] = str(work / "output")
    env["GITHUB_ENV"] = str(work / "env")
    env["RUNNER_TOOL_CACHE"] = str(tool_cache or (work / "tool-cache"))
    env["SCCACHE_FETCH_ATTEMPTS"] = attempts
    env["SCCACHE_FETCH_DELAY_SECONDS"] = delay
    if path_prefix is not None:
        env["PATH"] = f"{path_prefix}{os.pathsep}{env.get('PATH', '')}"
    (work / "env").write_text("", encoding="utf-8")
    command = ["bash", str(script or (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"))]
    if base is not None:
        command.extend(["--release-base", base])
    result = subprocess.run(command, env=env, text=True, capture_output=True, check=False)
    output_path = work / "output"
    output = output_path.read_text(encoding="utf-8") if output_path.is_file() else ""
    return result, output


SCCACHE_FETCH_COMMAND = (
    'if [ "$(/usr/bin/uname -s)" = Darwin ]; then '
    'exec /usr/bin/env -i HOME="$HOME" GITHUB_OUTPUT="$GITHUB_OUTPUT" '
    'RUNNER_TOOL_CACHE="$RUNNER_TOOL_CACHE" RUNNER_OS="$RUNNER_OS" '
    "/bin/bash --noprofile --norc scripts/ci/fetch_sccache_release.sh; fi; "
    'exec /usr/bin/env -i HOME="$HOME" GITHUB_OUTPUT="$GITHUB_OUTPUT" '
    'RUNNER_TOOL_CACHE="$RUNNER_TOOL_CACHE" RUNNER_OS="$RUNNER_OS" '
    "/usr/bin/bash --noprofile --norc scripts/ci/fetch_sccache_release.sh"
)
_FETCH_RUN_LINE = f"run: {SCCACHE_FETCH_COMMAND}"
# Exact run lines that may name scripts/ci. Anything else that names that
# tree, or that executes by a path the scanner cannot classify, is refused.
_ALLOWED_SCRIPTS_CI_LINES = frozenset(
    {
        _FETCH_RUN_LINE,
        "run: bash scripts/ci/gpu_frame_budget.sh",
        'run: bash scripts/ci/deepen_for_merge_base.sh "${{ github.base_ref }}"',
        "python3 scripts/ci/affected_crates.py \\",
        "run: bash scripts/ci/player_facing_names.sh",
        "run: python3 scripts/ci/test_workflow_policy.py",
        "run: bash scripts/ci/run-gate.sh",
        "run: bash scripts/ci/run-tests.sh",
        "run: bash scripts/ci/check-gpu-skip-log.sh",
        "bin=$(python3 scripts/ci/frame_alloc_test_executable.py target/frame-alloc-build.json)",
        'python3 scripts/ci/mutation_ai_outcomes.py --verify-shard "$outcomes"',
        'counts=$(python3 scripts/ci/mutation_ai_outcomes.py "$outcomes") || exit 1',
        'counts=$(python3 scripts/ci/mutation_ai_outcomes.py "${ai_files[@]}") || exit 1',
    }
)
_ASSIGN_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$")


def _plain_assignment(word: str) -> bool:
    match = _ASSIGN_RE.fullmatch(word)
    if match is None:
        return False
    value = match.group(2)
    return not any(mark in value for mark in ("$(", "`", "\n", ";"))
_FETCH_NAME_RE = re.compile(r"(?<![A-Za-z0-9_])fetch_sccache")
_BRACED_VAR_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")
_SIMPLE_VAR_RE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)")
_INTERPRETERS = frozenset({"bash", "sh", "python", "python3", "node", "source", "."})


class _PolicyScanError(Exception):
    pass


def _sccache_policy_texts() -> list[tuple[str, str]]:
    paths = [ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"]
    paths.extend(sorted((ROOT / ".github" / "workflows").glob("*.yml")))
    return [
        (str(path.relative_to(ROOT)), path.read_text(encoding="utf-8")) for path in paths
    ]


def _allowlisted_command(command: str) -> bool:
    stripped = command.strip()
    if not stripped:
        return False
    if stripped in _ALLOWED_SCRIPTS_CI_LINES or f"run: {stripped}" in _ALLOWED_SCRIPTS_CI_LINES:
        return True
    for item in _ALLOWED_SCRIPTS_CI_LINES:
        if not item.rstrip().endswith("\\"):
            continue
        prefix = item[4:].strip() if item.startswith("run: ") else item
        prefix = prefix.rstrip("\\").strip()
        if stripped == prefix or stripped.startswith(prefix + " "):
            return True
    return False


def _drop_quotes_and_escapes(text: str) -> str:
    """Join quote glue and keep the character after a backslash."""
    out: list[str] = []
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote == "'":
            if char == "'":
                quote = ""
            else:
                out.append(char)
            index += 1
            continue
        if char == "\\" and index + 1 < len(text):
            out.append(text[index + 1])
            index += 2
            continue
        if char in "\"'":
            if quote == char:
                quote = ""
            elif not quote:
                quote = char
            else:
                out.append(char)
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def _fold_path_separators(text: str) -> str:
    """Treat a backslash between path parts as a slash, outside quotes."""
    out: list[str] = []
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            out.append(char)
            if char == quote:
                quote = ""
            index += 1
            continue
        if char in "\"'":
            quote = char
            out.append(char)
            index += 1
            continue
        if char == "\\":
            out.append("/")
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def _expand_vars(text: str, env: dict[str, str]) -> str:
    def braced(match: re.Match[str]) -> str:
        return env.get(match.group(1), match.group(0))

    def simple(match: re.Match[str]) -> str:
        return env.get(match.group(1), match.group(0))

    text = _BRACED_VAR_RE.sub(braced, text)
    return _SIMPLE_VAR_RE.sub(simple, text)


def _path_names_scripts_ci(token: str) -> bool:
    parts = [part for part in token.split("/") if part not in {"", "."}]
    for index in range(len(parts) - 1):
        if fnmatch.fnmatchcase("scripts", parts[index]) and fnmatch.fnmatchcase(
            "ci", parts[index + 1]
        ):
            return True
    return False


def _names_scripts_ci(text: str) -> bool:
    if "scripts/ci/" in text:
        return True
    if _FETCH_NAME_RE.search(text):
        return True
    for match in re.finditer(r"\S+", text):
        if _path_names_scripts_ci(match.group(0)):
            return True
    return False


def _strip_shell_comment(text: str) -> str:
    quote = ""
    for index, char in enumerate(text):
        if quote:
            if char == quote:
                quote = ""
            continue
        if char in "\"'":
            quote = char
            continue
        if char == "#":
            return text[:index]
    return text


def _split_commands(text: str) -> list[str]:
    parts: list[str] = []
    buf: list[str] = []
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            buf.append(char)
            if char == quote and text[index - 1] != "\\":
                quote = ""
            index += 1
            continue
        if char in "\"'":
            quote = char
            buf.append(char)
            index += 1
            continue
        if char in ";\n":
            parts.append("".join(buf))
            buf = []
            index += 1
            continue
        if text.startswith("&&", index) or text.startswith("||", index):
            parts.append("".join(buf))
            buf = []
            index += 2
            continue
        buf.append(char)
        index += 1
    parts.append("".join(buf))
    return [part.strip() for part in parts if part.strip()]


def _raw_words(command: str) -> list[str]:
    words: list[str] = []
    buf: list[str] = []
    quote = ""
    for char in command:
        if quote:
            buf.append(char)
            if char == quote:
                quote = ""
            continue
        if char in "\"'":
            quote = char
            buf.append(char)
            continue
        if char.isspace():
            if buf:
                words.append("".join(buf))
                buf = []
            continue
        buf.append(char)
    if buf:
        words.append("".join(buf))
    return words


_ARRAY_COMMAND_RE = re.compile(r"""^["']?\$\{[A-Za-z_][A-Za-z0-9_]*\[@\]\}["']?$""")
_HEREDOC_RE = re.compile(r"<<\s*-?\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)['\"]?")


def _has_backslash_path(token: str) -> bool:
    index = 0
    while index < len(token):
        if token[index] != "\\":
            index += 1
            continue
        nxt = token[index + 1] if index + 1 < len(token) else ""
        if nxt in {"n", "t", "r", "\\", '"', "'"}:
            index += 2
            continue
        return True
    return False


def _script_arg_nonliteral(token: str) -> bool:
    if any(mark in token for mark in ("$", "`", "*", "?", "[")):
        return True
    return _has_backslash_path(token)


def _argv0_nonliteral(token: str) -> bool:
    if token in {"[", "[[", "]", "]]"}:
        return False
    if _ARRAY_COMMAND_RE.fullmatch(token):
        return False
    if token.startswith("$") or token.startswith("`"):
        return True
    if _has_backslash_path(token):
        return True
    if any(mark in token for mark in ("*", "?")) and (
        "/" in token or token.startswith(".")
    ):
        return True
    return False


def _unclosed_quote(text: str) -> bool:
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            if char == quote:
                quote = ""
            index += 1
            continue
        if char == "\\" and index + 1 < len(text):
            index += 2
            continue
        if char in "\"'":
            quote = char
        index += 1
    return bool(quote)


def _logical_shell_lines(body: str) -> list[str]:
    """Join continuations and quoted newlines, and drop heredoc bodies."""
    lines: list[str] = []
    buf = ""
    pending = False
    heredoc: str | None = None
    for raw in body.splitlines():
        if heredoc is not None:
            if raw.strip() == heredoc:
                heredoc = None
            continue
        if not _unclosed_quote(buf):
            match = _HEREDOC_RE.search(raw)
            if match is not None:
                heredoc = match.group(1)
        stripped = raw.rstrip()
        continued = (
            not _unclosed_quote(buf + stripped)
            and stripped.endswith("\\")
            and not stripped.endswith("\\\\")
        )
        piece = (stripped[:-1] + " ") if continued else raw
        if pending or (buf and _unclosed_quote(buf)):
            buf += piece if pending else "\n" + piece
        else:
            buf = piece
        pending = continued
        if continued or _unclosed_quote(buf):
            continue
        lines.append(buf)
        buf = ""
        pending = False
    if buf:
        lines.append(buf)
    return lines


def _nonliteral_execution(words: list[str]) -> bool:
    if not words:
        return False
    if _argv0_nonliteral(words[0]):
        return True
    base = words[0].rsplit("/", 1)[-1]
    if base in _INTERPRETERS or base.endswith("bash"):
        for word in words[1:]:
            if word.startswith("-") and word != "-":
                continue
            return _script_arg_nonliteral(word)
    return False


def _cwd_parts(path: str) -> list[str]:
    return [part for part in path.split("/") if part not in {"", "."}]


def _apply_cd(parts: list[str], dest: str) -> list[str]:
    if dest.startswith("/"):
        out: list[str] = []
    else:
        out = list(parts)
    for part in dest.split("/"):
        if part in {"", "."}:
            continue
        if part == "..":
            if out:
                out.pop()
            continue
        out.append(part)
    return out


def _cwd_text(parts: list[str]) -> str:
    return "/".join(parts)


def _script_token(words: list[str]) -> str | None:
    if not words:
        return None
    base = words[0].rsplit("/", 1)[-1]
    if base in _INTERPRETERS or base.endswith("bash"):
        for word in words[1:]:
            if word.startswith("-") and word != "-":
                continue
            return word
        return None
    return words[0]


def _relative_under_scripts_ci(parts: list[str], words: list[str]) -> bool:
    if not _path_names_scripts_ci(_cwd_text(parts)):
        return False
    token = _script_token(words)
    if token is None or token.startswith("/"):
        return False
    return token.startswith("./") or token.startswith("../") or "/" in token


def _cwd_unclassified(parts: list[str], words: list[str]) -> bool:
    if not any(any(mark in part for mark in ("$", "`", "\\")) for part in parts):
        return False
    token = _script_token(words)
    if token is None:
        return False
    return token.startswith("./") or token.startswith("../") or (
        "/" in token and not token.startswith("/")
    )


def _command_names_scripts_ci(
    command: str, env: dict[str, str], cwd: list[str]
) -> tuple[bool, dict[str, str], list[str]]:
    folded = _fold_path_separators(command)
    normalized = _expand_vars(_drop_quotes_and_escapes(folded), env)
    try:
        words = shlex.split(folded, posix=True)
    except ValueError:
        return _names_scripts_ci(normalized), env, cwd
    raw = _raw_words(command)
    local = dict(env)
    fresh: dict[str, str] = {}
    index = 0
    while index < len(words) and _plain_assignment(words[index]):
        name, value = words[index].split("=", 1)
        fresh[name] = _drop_quotes_and_escapes(_fold_path_separators(value))
        local[name] = fresh[name]
        index += 1
    if index == len(words):
        env.update(fresh)
        return any(_names_scripts_ci(value) for value in fresh.values()), env, cwd
    if words[index] == "cd" and index + 1 < len(words):
        dest = _expand_vars(_drop_quotes_and_escapes(words[index + 1]), local)
        return _names_scripts_ci(dest), env, _apply_cd(cwd, dest)
    expanded_words = [
        _expand_vars(_drop_quotes_and_escapes(word), local) for word in words[index:]
    ]
    expanded = " ".join(expanded_words)
    named = (
        _names_scripts_ci(normalized)
        or _names_scripts_ci(expanded)
        or _relative_under_scripts_ci(cwd, expanded_words)
    )
    nonliteral = _nonliteral_execution(raw[index:])
    if _cwd_unclassified(cwd, raw[index:]):
        nonliteral = True
    return named or nonliteral, env, cwd


def _run_body_violations(
    label: str,
    body: str,
    env: dict[str, str],
    workdir: str,
    shell: str | None = None,
) -> list[str]:
    violations: list[str] = []
    if shell and shell.split()[0].lower() in {"pwsh", "powershell", "pwsh.exe"}:
        if _names_scripts_ci(body):
            violations.append(f"{label} is not an allowlisted scripts/ci command")
        return violations
    cwd = _cwd_parts(workdir)
    local = dict(env)
    for raw_line in _logical_shell_lines(body):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if _allowlisted_command(stripped):
            continue
        command = _strip_shell_comment(stripped)
        if not command.strip():
            continue
        if _allowlisted_command(command):
            continue
        for part in _split_commands(command):
            if _allowlisted_command(part):
                continue
            named, local, cwd = _command_names_scripts_ci(part, local, cwd)
            if named:
                violations.append(f"{label} is not an allowlisted scripts/ci command")
                return violations
    return violations


def _string_env(value: object, where: str) -> dict[str, str]:
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise _PolicyScanError(f"{where} env")
    out: dict[str, str] = {}
    for key, item in value.items():
        if not isinstance(key, str) or isinstance(item, (dict, list)):
            raise _PolicyScanError(f"{where} env")
        out[key] = "" if item is None else str(item)
    return out


def _optional_string(value: object, where: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise _PolicyScanError(where)
    return value


def _classify_defaults(value: object, where: str) -> tuple[str | None, list[str]]:
    """Absent, or exactly run.shell bash. Any other shape fails closed.

    A shell-only string other than bash is classified so the fetch step can
    report it. A working-directory, a non-mapping, a string run, or a shell
    key that is not under run cannot be classified.
    """
    if value is None:
        return None, []
    if not isinstance(value, dict) or set(value) != {"run"}:
        raise _PolicyScanError(f"{where} defaults")
    run = value["run"]
    if not isinstance(run, dict) or set(run) != {"shell"}:
        raise _PolicyScanError(f"{where} defaults")
    shell = run["shell"]
    if not isinstance(shell, str):
        raise _PolicyScanError(f"{where} defaults")
    if shell != "bash":
        return shell, [f"{where} shell is not bash"]
    return shell, []


_FETCH_SHELL_ENV_KEYS = frozenset({"BASH_ENV", "ENV"})


def _fetch_shell_env_violations(where: str, env: dict[str, str]) -> list[str]:
    return [
        f"{where} sets {key}"
        for key in sorted(key for key in env if key in _FETCH_SHELL_ENV_KEYS)
    ]


def _with_violations(label: str, value: object) -> list[str]:
    if not isinstance(value, dict):
        raise _PolicyScanError(f"{label} with")
    violations: list[str] = []
    for item in value.values():
        if isinstance(item, str):
            normalized = _expand_vars(
                _drop_quotes_and_escapes(_fold_path_separators(item)), {}
            )
            if "scripts/ci/" in normalized or _FETCH_NAME_RE.search(normalized):
                violations.append(f"{label} is not an allowlisted scripts/ci command")
            continue
        if item is None or isinstance(item, (bool, int, float)):
            continue
        raise _PolicyScanError(f"{label} with")
    return violations


def _env_names_scripts_ci(where: str, env: dict[str, str]) -> list[str]:
    violations: list[str] = []
    for value in env.values():
        if _names_scripts_ci(value):
            violations.append(f"{where} is not an allowlisted scripts/ci command")
            break
    return violations


def _scan_step(
    label: str,
    step: object,
    job_env: dict[str, str],
    workflow_env: dict[str, str],
    default_shell: str | None,
    default_wd: str | None,
) -> tuple[list[str], int]:
    if not isinstance(step, dict):
        raise _PolicyScanError(f"{label} step")
    violations: list[str] = []
    if "env" in step:
        step_env = _string_env(step.get("env"), label)
    else:
        step_env = {}
    env = dict(workflow_env)
    env.update(job_env)
    env.update(step_env)
    violations.extend(_env_names_scripts_ci(label, step_env))
    if "with" in step:
        violations.extend(_with_violations(label, step.get("with")))
    shell = default_shell
    if "shell" in step:
        shell = _optional_string(step.get("shell"), f"{label} shell")
    workdir = default_wd or ""
    if "working-directory" in step:
        parsed = _optional_string(step.get("working-directory"), f"{label} working-directory")
        workdir = parsed or ""
    if shell and (_names_scripts_ci(shell) or _script_arg_nonliteral(shell)):
        violations.append(f"{label} shell is not an allowlisted scripts/ci command")
    found = 0
    if "run" not in step:
        return violations, found
    run = step.get("run")
    if run is None:
        raise _PolicyScanError(f"{label} run")
    if not isinstance(run, str):
        raise _PolicyScanError(f"{label} run")
    if run.strip() == SCCACHE_FETCH_COMMAND:
        found = 1
        if "env" in step:
            violations.append(f"{label} fetch step sets env")
        if "working-directory" in step:
            violations.append(f"{label} fetch step sets working-directory")
        own_shell = step.get("shell") if "shell" in step else None
        if "shell" in step and own_shell != "bash":
            violations.append(f"{label} fetch step shell is not bash")
        elif "shell" not in step and default_shell not in (None, "bash"):
            violations.append(f"{label} fetch step shell is not bash")
    violations.extend(_run_body_violations(label, run, env, workdir, shell))
    return violations, found


def _scan_document(label: str, data: object) -> tuple[list[str], int]:
    if not isinstance(data, dict):
        raise _PolicyScanError("root")
    violations: list[str] = []
    found = 0
    workflow_env = _string_env(data.get("env"), label)
    violations.extend(_env_names_scripts_ci(label, workflow_env))
    default_shell, default_violations = _classify_defaults(data.get("defaults"), label)
    violations.extend(default_violations)
    if default_shell and (
        _names_scripts_ci(default_shell) or _script_arg_nonliteral(default_shell)
    ):
        violations.append(f"{label} shell is not an allowlisted scripts/ci command")
    runs = data.get("runs")
    indexed: list[tuple[str, object, dict[str, str], str | None, str]] = []
    job_envs: dict[str, dict[str, str]] = {}
    if isinstance(runs, dict) and "steps" in runs:
        raw = runs.get("steps")
        if not isinstance(raw, list):
            raise _PolicyScanError("runs.steps")
        job_envs["runs"] = {}
        for index, step in enumerate(raw):
            indexed.append((f"{label}:runs:{index}", step, {}, default_shell, "runs"))
    else:
        jobs = data.get("jobs")
        if jobs is None:
            return violations, found
        if not isinstance(jobs, dict):
            raise _PolicyScanError("jobs")
        for name, job in jobs.items():
            if not isinstance(job, dict):
                raise _PolicyScanError(f"job {name}")
            job_where = f"{label}:{name}"
            if "defaults" in job and job.get("defaults") is not None:
                job_shell, job_default_violations = _classify_defaults(
                    job.get("defaults"), job_where
                )
                violations.extend(job_default_violations)
            else:
                job_shell = default_shell
            job_env = _string_env(job.get("env"), job_where)
            job_envs[str(name)] = job_env
            violations.extend(_env_names_scripts_ci(job_where, job_env))
            if "with" in job:
                violations.extend(_with_violations(job_where, job.get("with")))
            raw = job.get("steps")
            if raw is None:
                continue
            if not isinstance(raw, list):
                raise _PolicyScanError(f"job {name} steps")
            for index, step in enumerate(raw):
                indexed.append((f"{job_where}:{index}", step, job_env, job_shell, str(name)))
    fetch_jobs: set[str] = set()
    for step_label, step, job_env, job_shell, job_id in indexed:
        step_violations, step_found = _scan_step(
            step_label, step, job_env, workflow_env, job_shell, None
        )
        violations.extend(step_violations)
        found += step_found
        if step_found:
            fetch_jobs.add(job_id)
    if fetch_jobs:
        violations.extend(_fetch_shell_env_violations(f"{label} workflow", workflow_env))
        for job_id in sorted(fetch_jobs):
            violations.extend(
                _fetch_shell_env_violations(f"{label}:{job_id}", job_envs.get(job_id, {}))
            )
        for step_label, step, _job_env, _job_shell, job_id in indexed:
            if job_id not in fetch_jobs or not isinstance(step, dict) or "env" not in step:
                continue
            step_env = _string_env(step.get("env"), step_label)
            violations.extend(_fetch_shell_env_violations(step_label, step_env))
    return violations, found


def sccache_fetch_run_violations(
    texts: list[tuple[str, str]] | None = None,
    *,
    expected_fetch_count: int | None = None,
) -> list[str]:
    if texts is None:
        texts = _sccache_policy_texts()
        if expected_fetch_count is None:
            expected_fetch_count = 10
    violations: list[str] = []
    found = 0
    for label, text in texts:
        try:
            data = yaml.safe_load(text)
        except yaml.YAMLError:
            violations.append(f"{label}: could not classify the workflow")
            continue
        if data is None:
            continue
        try:
            file_violations, file_found = _scan_document(label, data)
        except _PolicyScanError:
            violations.append(f"{label}: could not classify the workflow")
            continue
        violations.extend(file_violations)
        found += file_found
    if expected_fetch_count is not None and found != expected_fetch_count:
        violations.append(f"expected {expected_fetch_count} fetch steps, found {found}")
    return violations


def _probe_workflow(run: str) -> str:
    if "\n" in run:
        body = "".join(f"          {line}\n" for line in run.splitlines())
        step = "        run: |\n" + body
    else:
        step = f"        run: {run}\n"
    return (
        "name: probe\n"
        "on: push\n"
        "jobs:\n"
        "  build:\n"
        "    runs-on: ubuntu-latest\n"
        "    steps:\n"
        "      - name: Probe\n"
        + step
    )


def _workflow_fetch_argv(script: Path, base: str, env: dict[str, str]) -> list[str]:
    replacement = shlex.quote(str(script)) + " --release-base " + shlex.quote(base)
    command = SCCACHE_FETCH_COMMAND.replace(
        "scripts/ci/fetch_sccache_release.sh", replacement
    )
    argv = ["/usr/bin/env", "-i"]
    for key in ("PATH", "HOME", "GITHUB_OUTPUT", "RUNNER_TOOL_CACHE", "RUNNER_OS"):
        argv.append(f"{key}={env.get(key, '')}")
    argv.extend(["/usr/bin/bash", "--noprofile", "--norc", "-c", command])
    return argv

def _run_workflow_fetch(
    work: Path,
    *,
    script: Path | None = None,
    base: str,
    hostile: dict[str, str] | None = None,
    tool_cache: Path | None = None,
) -> tuple[subprocess.CompletedProcess[str], str]:
    env = os.environ.copy()
    if hostile:
        env.update(hostile)
    env["GITHUB_OUTPUT"] = str(work / "output")
    env["RUNNER_TOOL_CACHE"] = str(tool_cache or (work / "tool-cache"))
    output_path = work / "output"
    output_path.unlink(missing_ok=True)
    argv = _workflow_fetch_argv(
        script or (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"),
        base,
        env,
    )
    result = subprocess.run(argv, env=env, text=True, capture_output=True, check=False)
    output = output_path.read_text(encoding="utf-8") if output_path.is_file() else ""
    return result, output


def _sccache_enable_run_bodies() -> list[tuple[str, str]]:
    paths = [ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"]
    paths.extend(sorted(WORKFLOWS.glob("*.yml")))
    found: list[tuple[str, str]] = []
    for path in paths:
        text = path.read_text(encoding="utf-8")
        if "SCCACHE_GHA_ENABLED=true" not in text:
            continue
        for part in re.split(r"(?m)^[ ]*- name:", text)[1:]:
            if "SCCACHE_GHA_ENABLED=true" not in part:
                continue
            if "AW_GITHUB_REF: ${{ github.ref }}" not in part:
                raise AssertionError(f"{path.name} does not pass the ref into the enable step")
            if "AW_GITHUB_EVENT_NAME: ${{ github.event_name }}" not in part:
                raise AssertionError(f"{path.name} does not pass the event name into the enable step")
            if "run: |" not in part:
                raise AssertionError(f"{path.name} enable step has no run block")
            body = pwsh_run_body("run: |" + part.split("run: |", 1)[1])
            found.append((path.name, body))
    return found


def _run_bash_env(body: str, ref: str, event: str) -> str:
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        envfile = work / "github-env"
        script = work / "step.sh"
        script.write_text(body, encoding="utf-8")
        env = os.environ.copy()
        env["GITHUB_ENV"] = str(envfile)
        env["AW_GITHUB_REF"] = ref
        env["AW_GITHUB_EVENT_NAME"] = event
        result = subprocess.run(
            ["bash", str(script)],
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(result.stderr or result.stdout)
        if not envfile.is_file():
            return ""
        return envfile.read_text(encoding="utf-8")


class WorkflowPolicyTest(unittest.TestCase):
    def test_pr_workflows_cancel_only_pull_requests(self) -> None:
        offenders = []
        for name, text in workflow_texts().items():
            if not has_pull_request(text):
                continue
            if CONCURRENCY_GROUP not in text or CONCURRENCY_CANCEL not in text:
                offenders.append(name)
            if "cancel-in-progress: true" in text:
                offenders.append(f"{name}:cancel-true")
        self.assertEqual(offenders, [])

    def test_every_job_has_a_timeout(self) -> None:
        missing = {
            name: jobs
            for name, text in workflow_texts().items()
            if (jobs := jobs_missing_timeout(text))
        }
        self.assertEqual(missing, {})

    def test_determinism_replay_is_not_path_filtered(self) -> None:
        text = (WORKFLOWS / "determinism-replay.yml").read_text(encoding="utf-8")
        self.assertNotIn("paths-ignore", text)
        self.assertNotIn("paths:", text)
        self.assertIn("ubuntu-latest", text)
        self.assertIn("windows-latest", text)
        self.assertIn("macos-latest", text)
        self.assertNotIn("cargo nextest", text)
        self.assertIn("no-retry", text)
        self.assertEqual(text.count("Fetch Aravath nature models"), 2)
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', text)
        self.assertIn("cargo test -p astraweave-core --test replay_hash", text)
        self.assertIn('echo "RUSTFLAGS=-C target-cpu=x86-64" >> "$GITHUB_ENV"', text)

    def test_benchmarks_do_not_run_on_every_pull_request(self) -> None:
        text = (WORKFLOWS / "benchmark.yml").read_text(encoding="utf-8")
        self.assertIn("types: [labeled]", text)
        self.assertIn("run-benchmarks", text)
        self.assertIn('branches: ["main", "develop"]', text)
        self.assertIn("workflow_dispatch:", text)
        # Push already runs the shards. The 05:17 cron was a second run.
        self.assertNotIn("schedule:", text)
        self.assertNotIn("17 5 * * *", text)
        self.assertNotIn("pull_request:\n    branches:", text)

    def test_duplicate_workflows_are_gone(self) -> None:
        self.assertFalse((WORKFLOWS / "rust-cache-optimized.yml").exists())
        self.assertFalse((WORKFLOWS / "clippy-unwrap-prevention.yml").exists())

    def test_heavy_tier_is_not_on_pull_request(self) -> None:
        nightly_only = [
            "sanitizers.yml",
            "coverage.yml",
            "codeql-analysis.yml",
            "toolchain-management.yml",
            "integration-tests.yml",
            "sdk-ci.yml",
            "pbr-pipeline-ci.yml",
            "editor-ci.yml",
            "aw-editor-tests.yml",
            "allocation-measurement.yml",
        ]
        for name in nightly_only:
            text = (WORKFLOWS / name).read_text(encoding="utf-8")
            self.assertFalse(has_pull_request(text), name)

    def test_ci_gate_is_required_before_heavy_jobs(self) -> None:
        text = (WORKFLOWS / "ci.yml").read_text(encoding="utf-8")
        self.assertIn("name: Gate (fmt, clippy, check)", text)
        self.assertIn("needs: [changes, gate]", text)
        script = (ROOT / "scripts" / "ci" / "run-tests.sh").read_text(encoding="utf-8")
        self.assertIn("--partition", script)
        self.assertIn("cargo nextest run", script)
        self.assertIn("cargo nextest archive", script)
        nextest = (ROOT / ".config" / "nextest.toml").read_text(encoding="utf-8")
        self.assertIn("retries = 0", nextest)
        self.assertIn("[profile.ci-pr]", nextest)
        self.assertIn("[profile.ci-pr-archive]", nextest)
        self.assertIn("[profile.ci-pr-linux-archive]", nextest)
        self.assertIn("default-filter", nextest)
        self.assertIn("family1_client_binary", text)
        self.assertIn("ci-pr-archive", text)
        self.assertIn("ci-pr-linux-archive", text)
        self.assertIn("timeout-minutes: 75", text)
        self.assertIn("Test (windows, build archive)", text)
        self.assertIn("Test (ubuntu, build archive)", text)
        self.assertIn("needs: [changes, gate, test-windows-build]", text)
        self.assertIn("needs: [changes, gate, test-linux-build]", text)
        # Nav stress ceilings run with the PR partitions. An unset
        # AW_PERF_ASSERTS skips them, which drops the old wall-clock gate.
        linux_job = text.split("\n  test-linux:\n", 1)[1].split("\n  test-windows-build:\n", 1)[0]
        windows_job = text.split("\n  test-windows:\n", 1)[1].split("\n  msrv:\n", 1)[0]
        self.assertIn("github.event_name == 'pull_request'", linux_job)
        self.assertIn("github.event_name == 'pull_request'", windows_job)
        self.assertIn('AW_PERF_ASSERTS: "1"', linux_job)
        self.assertIn('AW_PERF_ASSERTS: "1"', windows_job)
        linux = nextest.split("[profile.ci-pr-linux-archive]", 1)[1].split("\n[", 1)[0]
        self.assertNotIn("test_renderer_bake_environment_wave16", linux)
        self.assertNotIn("gpu_containment_invariant_120_frames", linux)
        self.assertNotIn("gpu_settling_envelope_300_frames", linux)
        for line in (ROOT / "scripts" / "ci" / "linux_archive_skips.txt").read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            _kind, package, binary = stripped.split(" ", 2)
            self.assertIn(f"package(={package})", linux)
            self.assertIn(f"binary(={binary})", linux)

    def test_deferred_pr_filter_matches_the_list_and_skips_replay(self) -> None:
        listed = []
        for line in (ROOT / "scripts" / "ci" / "deferred_pr_tests.txt").read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            package, binary, test = stripped.split(" ", 2)
            listed.append((package, binary, test))
        self.assertGreaterEqual(len(listed), 1)
        profile = (ROOT / ".config" / "nextest.toml").read_text(encoding="utf-8")
        for package, binary, test in listed:
            self.assertIn(f"package(={package})", profile)
            self.assertIn(f"binary(={binary})", profile)
            self.assertIn(f"test(={test})", profile)
            if binary == package:
                self.fail(f"lib binary should be the underscore target name, got {binary}")
        for replay in (
            "planner_pure",
            "perception_world",
            "gameplay_rng",
            "replay_hash",
            "gameplay_qa_aravath_slice",
        ):
            self.assertNotIn(replay, profile)
            self.assertNotIn(replay, (ROOT / "scripts" / "ci" / "deferred_pr_tests.txt").read_text(encoding="utf-8"))

    def test_aravath_supply_chain_uses_prebuilt_tools(self) -> None:
        text = (WORKFLOWS / "aravath-windows.yml").read_text(encoding="utf-8")
        self.assertNotIn("cargo install cargo-audit", text)
        self.assertNotIn("cargo install cargo-deny", text)
        self.assertNotIn("cargo install cargo-cyclonedx", text)
        script = (ROOT / ".github" / "scripts" / "install-prebuilt-cargo-tool.sh").read_text(encoding="utf-8")
        for tool in ("cargo-audit", "cargo-deny", "cargo-cyclonedx"):
            self.assertIn(f"install-prebuilt-cargo-tool.sh {tool}", text)
            self.assertIn(f"  {tool})", script)
        self.assertIn("cargo audit", text)
        self.assertIn("cargo deny check", text)
        self.assertIn("cargo cyclonedx", text)
        # Versions stay the ones the job used to cargo-install.
        self.assertIn("0.22.2", script)
        self.assertIn("0.20.2", script)
        self.assertIn("0.5.9", script)
        self.assertIn("9bd3e599314f50810c9d98b8b68a617ff9d3cc20873968d90b29d121f6b226ff", script)
        self.assertIn('"$tool" cyclonedx --version', script)

    def test_sccache_pull_requests_are_read_only(self) -> None:
        action = (ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml").read_text(encoding="utf-8")
        self.assertIn("CARGO_INCREMENTAL=0", action)
        self.assertIn("SCCACHE_GHA_RW_MODE=READ_ONLY", action)
        self.assertIn("SCCACHE_GHA_RW_MODE=READ_WRITE", action)
        self.assertIn("refs/heads/main", action)
        # RUSTFLAGS replaces every rustflags entry. The pin is in cargo config.
        self.assertNotIn("RUSTFLAGS=", action)
        config = (ROOT / ".cargo" / "config.toml").read_text(encoding="utf-8")
        self.assertIn("[target.x86_64-unknown-linux-gnu]", config)
        self.assertIn('"-C", "target-cpu=x86-64"', config)
        self.assertIn("[target.x86_64-pc-windows-msvc]", config)
        self.assertIn("link-args=/STACK:16777216", config)
        self.assertIn("[target.x86_64-pc-windows-gnu]", config)
        self.assertIn("link-args=-Wl,--stack,16777216", config)
        replay = (WORKFLOWS / "determinism-replay.yml").read_text(encoding="utf-8")
        self.assertIn('echo "RUSTFLAGS=-C target-cpu=x86-64" >> "$GITHUB_ENV"', replay)
        self.assertIn("target-cpu=native is forbidden", replay)
        aravath = (WORKFLOWS / "aravath-determinism.yml").read_text(encoding="utf-8")
        self.assertIn("-C target-feature=-fma", aravath)

    def test_scoped_workflows_keep_their_checks_and_fail_closed(self) -> None:
        scoped = {
            "aravath-windows.yml": "aravath",
            "aravath-determinism.yml": "aravath-determinism",
            "render-device-validation.yml": "render-device",
            "net-tests.yml": "net",
        }
        texts = workflow_texts()
        for name, scope in scoped.items():
            text = texts[name]
            self.assertIn(f"--scope {scope}", text)
            self.assertIn("Fail closed when scope detection fails", text)
            if name == "aravath-determinism.yml":
                # The matrix job always starts so each OS check name exists.
                # The first step fails the job when classify did not succeed.
                self.assertIn("needs.scope.result != 'success'", text)
            else:
                self.assertIn("needs.scope.result == 'failure'", text)
            self.assertNotIn("paths-ignore:", text)
            self.assertNotIn("\n    paths:", text)
        # The matrix must keep expanding so the per-OS check names exist.
        det = texts["aravath-determinism.yml"]
        self.assertIn("name: Replay hash (${{ matrix.os }})", det)
        self.assertIn("if: ${{ !cancelled() }}", det)
        self.assertIn("replay_matches_golden_hash", det)
        self.assertNotIn("needs.scope.outputs.run == 'true'\n    runs-on:", det)
        windows = texts["aravath-windows.yml"]
        self.assertIn("name: cargo-audit, cargo-deny, SBOM", windows)
        self.assertIn("name: Volumetrics validation on lavapipe", windows)
        self.assertIn("name: Build, test, and upload greybox", windows)
        self.assertIn("cargo audit", windows)
        self.assertIn("cargo deny check", windows)
        self.assertIn("cargo cyclonedx", windows)
        render = texts["render-device-validation.yml"]
        self.assertIn("name: Startup passes (lavapipe)", render)
        self.assertIn("name: Startup passes (Windows)", render)
        self.assertIn("created_pass_validation", render)
        self.assertIn("frame_alloc", render)
        self.assertIn("stream_fallback", render)
        self.assertIn("gpu_timestamp", render)
        net = texts["net-tests.yml"]
        self.assertIn("name: net-tests", net)
        self.assertIn("cargo test -p astraweave-net --all-features --verbose", net)
        replay = texts["determinism-replay.yml"]
        self.assertNotIn("--scope", replay)
        self.assertNotIn("Affected scope", replay)

    def test_detect_checks_out_merge_parents_only(self) -> None:
        text = (WORKFLOWS / "ci.yml").read_text(encoding="utf-8")
        changes = text.split("name: Detect affected crates", 1)[1].split("\n  gate:", 1)[0]
        self.assertIn("fetch-depth: 2", changes)
        self.assertNotIn("fetch-depth: 0", changes)
        self.assertIn("fetch-tags: false", changes)
        self.assertNotIn("git fetch", changes)
        self.assertIn('--base "HEAD^1"', changes)
        self.assertIn("--head HEAD", changes)
        self.assertIn("reason=missing-merge-parent", changes)

    def test_nightly_failure_issue_serializes_per_upstream_run(self) -> None:
        text = (WORKFLOWS / "nightly-failure-issue.yml").read_text(encoding="utf-8")
        self.assertIn(
            "group: ${{ github.workflow }}-${{ github.event.workflow_run.id }}",
            text,
        )
        self.assertIn("cancel-in-progress: false", text)
        self.assertNotIn("group: ${{ github.workflow }}-${{ github.ref }}", text)

    def test_nightly_failure_issue_watched_names_match_workflows(self) -> None:
        text = (WORKFLOWS / "nightly-failure-issue.yml").read_text(encoding="utf-8")
        watched = nightly_failure_watched_workflows(text)
        known = workflow_display_names()
        self.assertEqual(len(watched), 20)
        missing = [name for name in watched if name not in known]
        self.assertEqual(missing, [])
        self.assertIn("determinism-replay", watched)
        self.assertNotIn("Determinism Replay", watched)

    def test_nightly_failure_issue_job_skips_fork_and_pull_request_runs(self) -> None:
        text = (WORKFLOWS / "nightly-failure-issue.yml").read_text(encoding="utf-8")
        self.assertIn("head_repository.full_name == github.repository", text)
        self.assertIn("github.event.workflow_run.event == 'schedule'", text)
        self.assertIn("github.event.workflow_run.event == 'push'", text)
        self.assertIn("github.event.workflow_run.event == 'workflow_dispatch'", text)
        self.assertNotIn("github.event.workflow_run.event == 'pull_request'", text)
        self.assertNotIn(
            "github.event.workflow_run.event == 'schedule' ||\n"
            "       github.event.workflow_run.head_branch == 'main')",
            text,
        )

    def test_one_off_replay_workflows_stay_removed(self) -> None:
        for name in (
            "ai-depth-replay.yml",
            "combat-replay.yml",
            "determinism-replay-mod.yml",
        ):
            self.assertFalse(
                (WORKFLOWS / name).exists(),
                f"{name} must stay removed; cross-OS replay lives in determinism-replay.yml",
            )

    def test_frame_alloc_sites_is_dispatch_only(self) -> None:
        text = (WORKFLOWS / "frame-alloc-sites.yml").read_text(encoding="utf-8")
        on_block = text.split("on:", 1)[1].split("\npermissions:", 1)[0]
        self.assertIn("workflow_dispatch:", on_block)
        self.assertNotIn("push:", on_block)
        self.assertIn("frame_alloc_test_executable.py", text)

    def test_assets_prefetch_uses_polyhaven_provider_filter(self) -> None:
        text = (WORKFLOWS / "assets-prefetch.yml").read_text(encoding="utf-8")
        self.assertIn("--manifest assets/asset_manifest.toml", text)
        self.assertIn("--provider polyhaven", text)
        self.assertNotIn("astraweave-assets -- check", text)

    def test_allocation_measurement_is_a_real_gate_off_prs(self) -> None:
        text = (WORKFLOWS / "allocation-measurement.yml").read_text(encoding="utf-8")
        self.assertIn("schedule:", text)
        self.assertIn("branches: [main]", text)
        self.assertIn("gpu_frame_budget", text)
        self.assertIn("zero_alloc_tick", text)
        self.assertIn("zero_alloc_step", text)
        self.assertIn("[self-hosted, gpu]", text)
        self.assertNotIn("continue-on-error: true", text)
        self.assertFalse(has_pull_request(text))

    def test_pinned_nightly_steps_follow_the_experimental_flag(self) -> None:
        text = (WORKFLOWS / "toolchain-management.yml").read_text(encoding="utf-8")
        self.assertIn("toolchain: pinned-nightly", text)
        self.assertNotIn("toolchain: nightly\n", text)
        fmt = text.split("name: Check code formatting compatibility", 1)[1].split(
            "name: Run clippy with toolchain", 1
        )[0]
        self.assertIn("if: ${{ !matrix.experimental }}", fmt)
        self.assertNotIn("'nightly'", fmt)
        nightly = text.split("name: Test nightly features (if nightly)", 1)[1].split(
            "\n  msrv-check:", 1
        )[0]
        self.assertIn("if: matrix.experimental", nightly)
        self.assertNotIn("'nightly'", nightly)

    def test_mutation_ai_caps_memory_and_uploads_on_failure(self) -> None:
        text = mutation_testing_text()
        ai = mutation_ai_job(text)
        self.assertIn("ulimit -v 6291456", ai)
        self.assertIn("--in-place", ai)
        self.assertIn("--shard ${{ matrix.shard }}/8", ai)
        self.assertNotIn("--jobs 1", ai)
        self.assertIn("Verify astraweave-ai shard outcomes", ai)
        self.assertIn("mutation_ai_outcomes.py", ai)
        report = ai.split("Generate mutation report", 1)[1].split("Upload mutation results", 1)[0]
        upload = ai.split("Upload mutation results", 1)[1]
        self.assertIn("if: always()", report)
        self.assertIn("if: always()", upload)

    def test_mutation_summary_requires_ai_matrix_success(self) -> None:
        text = mutation_testing_text()
        assert_mutation_summary_requires_ai_success(text)
        without = text.replace(
            'elif [ "${{ needs.mutants-ai.result }}" != "success" ]; then',
            'elif false; then',
            1,
        )
        with self.assertRaises(AssertionError):
            assert_mutation_summary_requires_ai_success(without)

    def test_mutation_ai_outcome_steps_are_not_softened(self) -> None:
        text = mutation_testing_text()
        assert_mutation_ai_outcome_steps_not_softened(text)
        softened = text.replace(
            "python3 scripts/ci/mutation_ai_outcomes.py --verify-shard",
            "python3 scripts/ci/mutation_ai_outcomes.py --verify-shard || true",
            1,
        )
        with self.assertRaises(AssertionError):
            assert_mutation_ai_outcome_steps_not_softened(softened)
        softened = text.replace(
            "      - name: Verify astraweave-ai shard outcomes\n        if: always()\n        run: |",
            "      - name: Verify astraweave-ai shard outcomes\n        if: always()\n        continue-on-error: true\n        run: |",
            1,
        )
        with self.assertRaises(AssertionError):
            assert_mutation_ai_outcome_steps_not_softened(softened)

    def test_mutation_summary_checks_out_before_scoring(self) -> None:
        text = mutation_testing_text()
        assert_mutation_summary_checks_out_before_scoring(text)
        checkout_line = summary_checkout_step_line(text)
        without_checkout = text.replace(f"{checkout_line}\n\n", "", 1)
        with self.assertRaises(AssertionError):
            assert_mutation_summary_checks_out_before_scoring(without_checkout)
        source = inspect.getsource(assert_mutation_summary_checks_out_before_scoring)
        self.assertIn("if checkout > scoring:", source)
        stripped = source.replace("if checkout > scoring:", "if False and checkout > scoring:", 1)
        with self.assertRaises(AssertionError):
            self.assertIn("if checkout > scoring:", stripped)
        reordered = without_checkout.replace(
            'Detailed results available in workflow artifacts." >> "$GITHUB_STEP_SUMMARY"',
            f'Detailed results available in workflow artifacts." >> "$GITHUB_STEP_SUMMARY"\n\n      {checkout_line.strip()}',
            1,
        )
        with self.assertRaises(AssertionError) as ctx:
            assert_mutation_summary_checks_out_before_scoring(reordered)
        self.assertIn("must precede", str(ctx.exception))

    def test_workflow_action_pins(self) -> None:
        violations: list[str] = []
        for name, text in workflow_texts().items():
            violations.extend(workflow_action_pin_violations(text, path=name))
        self.assertEqual(violations, [])

    def test_policy_actions_matches_expected_set(self) -> None:
        self.assertEqual(set(POLICY_ACTIONS), EXPECTED_POLICY_ACTIONS)

    def test_policy_actions_have_floors_and_uses_regex(self) -> None:
        self.assertEqual(ACTION_MAJOR_FLOORS, PINNED_ACTION_MAJOR_FLOORS)
        self.assertEqual(set(ACTION_MAJOR_FLOORS), set(POLICY_ACTIONS))
        for action in POLICY_ACTIONS:
            violations = workflow_action_pin_violations(f"uses: {action}@main")
            self.assertEqual(len(violations), 1, action)
            self.assertIn(UNPINNED_REF_MSG, violations[0])
            match = ACTION_USES_RE.search(f"uses: {action}@v99")
            self.assertIsNotNone(match, action)
            self.assertEqual(match.group("action"), action)
            below = BELOW_FLOOR_MAJOR_TAG_BY_ACTION[action]
            at_floor = AT_FLOOR_MAJOR_TAG_BY_ACTION[action]
            self.assertIn(BELOW_FLOOR_MSG, action_ref_violation(action, below) or "", action)
            self.assertIsNone(action_ref_violation(action, at_floor), action)

    def test_action_ref_rejects_unpinned_branch(self) -> None:
        self.assertIn(UNPINNED_REF_MSG, action_ref_violation("actions/checkout", "main") or "")
        source = inspect.getsource(action_ref_violation)
        self.assertIn("UNPINNED_BRANCH_REFS", source)
        stripped = source.replace("or ref in UNPINNED_BRANCH_REFS", "", 1)
        with self.assertRaises(AssertionError):
            self.assertIn("UNPINNED_BRANCH_REFS", stripped)

    def test_action_ref_rejects_non_sha_semver_tag(self) -> None:
        self.assertIn(NON_SHA_TAG_MSG, action_ref_violation("actions/upload-artifact", "v4.3.0") or "")
        source = inspect.getsource(action_ref_violation)
        self.assertIn("NON_SHA_TAG_MSG", source)
        stripped = source.replace('return f"{action}@{ref} {NON_SHA_TAG_MSG}"', "", 1)
        with self.assertRaises(AssertionError):
            self.assertIn("NON_SHA_TAG_MSG", stripped)
        broken_fn = _broken_action_ref_violation(
            'return f"{action}@{ref} {NON_SHA_TAG_MSG}"',
            "return None",
        )
        self.assertIsNone(broken_fn("actions/upload-artifact", "v4.3.0"))

    def test_action_ref_rejects_major_below_floor(self) -> None:
        self.assertIn(BELOW_FLOOR_MSG, action_ref_violation("actions/checkout", "v4") or "")
        self.assertIn(BELOW_FLOOR_MSG, action_ref_violation("actions/checkout", "v5") or "")
        self.assertIn(BELOW_FLOOR_MSG, action_ref_violation("actions/checkout", "v6") or "")
        for action, below in BELOW_FLOOR_MAJOR_TAG_BY_ACTION.items():
            self.assertIn(BELOW_FLOOR_MSG, action_ref_violation(action, below) or "", action)
        source = inspect.getsource(action_ref_violation)
        self.assertIn("if int(major.group(1)) < floor:", source)
        stripped = source.replace("if int(major.group(1)) < floor:", "if False and int(major.group(1)) < floor:", 1)
        with self.assertRaises(AssertionError):
            self.assertIn("if int(major.group(1)) < floor:", stripped)
        broken_fn = _broken_action_ref_violation(
            "if int(major.group(1)) < floor:",
            "if False and int(major.group(1)) < floor:",
        )
        self.assertIsNone(broken_fn("actions/checkout", "v4"))

    def test_action_ref_accepts_sha_and_major_tags(self) -> None:
        self.assertIsNone(
            action_ref_violation(
                "actions/checkout",
                "3d3c42e5aac5ba805825da76410c181273ba90b1",
            )
        )
        self.assertIsNone(action_ref_violation("actions/checkout", "v7"))
        self.assertIsNone(action_ref_violation("actions/github-script", "v8"))

    def test_action_ref_rejects_malformed_sha(self) -> None:
        bad_refs = (
            "a" * 39,
            "a" * 41,
            "3d3c42e",
            "ABCDEF0123456789ABCDEF0123456789ABCDEF01",
            "0" * 39 + "g",
        )
        for ref in bad_refs:
            msg = action_ref_violation("actions/checkout", ref)
            self.assertIsNotNone(msg, ref)
            self.assertIn(NON_SHA_TAG_MSG, msg or "")
        broken_plus = _broken_action_ref_violation(
            r're.fullmatch(r"[0-9a-f]{40}", ref)',
            r're.fullmatch(r"[0-9a-f]+", ref)',
        )
        self.assertIsNone(broken_plus("actions/checkout", "a" * 39))
        self.assertIsNotNone(action_ref_violation("actions/checkout", "a" * 39))
        broken_dot = _broken_action_ref_violation(
            r're.fullmatch(r"[0-9a-f]{40}", ref)',
            r're.fullmatch(r".{40}", ref)',
        )
        self.assertIsNone(broken_dot("actions/checkout", "G" * 40))
        self.assertIsNotNone(action_ref_violation("actions/checkout", "G" * 40))

    def test_mutation_ai_scoring_uses_strict_eval(self) -> None:
        text = mutation_testing_text()
        assert_mutation_ai_scoring_uses_strict_eval(text)
        loose = text.replace(
            'counts=$(python3 scripts/ci/mutation_ai_outcomes.py "${ai_files[@]}") || exit 1',
            'eval "$(python3 scripts/ci/mutation_ai_outcomes.py "${ai_files[@]}")"',
            1,
        )
        with self.assertRaises(AssertionError):
            assert_mutation_ai_scoring_uses_strict_eval(loose)

    def test_mutation_ai_outcomes_excludes_cap_oom_from_caught(self) -> None:
        root = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_cap_oom"
        counts = mutation_ai_outcomes.count_outcomes(root / "outcomes.json")
        self.assertEqual(counts.cap_oom, 1)
        self.assertEqual(counts.caught, 0)
        self.assertEqual(counts.decided, 0)

    def test_mutation_ai_outcomes_counts_real_caught(self) -> None:
        root = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_real_caught"
        counts = mutation_ai_outcomes.count_outcomes(root / "outcomes.json")
        self.assertEqual(counts.caught, 1)
        self.assertEqual(counts.cap_oom, 0)
        self.assertEqual(counts.score_percent, 100)

    def test_mutation_ai_outcomes_verify_shard_requires_baseline_and_mutants(self) -> None:
        root = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_real_caught"
        self.assertEqual(mutation_ai_outcomes.verify_shard_outcomes(root / "outcomes.json"), [])

        missing = mutation_ai_outcomes.verify_shard_outcomes(root / "missing.json")
        self.assertTrue(missing)
        self.assertIn("missing outcomes", missing[0])

        baseline_failure = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_baseline_failure"
        errors = mutation_ai_outcomes.verify_shard_outcomes(baseline_failure / "outcomes.json")
        self.assertIn(VERIFY_BASELINE_ERROR, errors)

        baseline_only = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_baseline_only"
        errors = mutation_ai_outcomes.verify_shard_outcomes(baseline_only / "outcomes.json")
        self.assertIn(VERIFY_NO_MUTANTS_ERROR, errors)

    def test_mutation_ai_outcomes_verify_shard_rejects_unfinished(self) -> None:
        root = ROOT / "scripts" / "ci" / "testdata" / "mutation_ai_unfinished"
        errors = mutation_ai_outcomes.verify_shard_outcomes(root / "outcomes.json")
        self.assertTrue(any(VERIFY_END_TIME_ERROR in err for err in errors))
        self.assertTrue(any(VERIFY_INCOMPLETE_ERROR in err for err in errors))
        exit_code = mutation_ai_outcomes.main(
            ["--verify-shard", str(root / "outcomes.json")]
        )
        self.assertEqual(exit_code, 1)

    def test_mutation_ai_verify_errors_are_required(self) -> None:
        source = (ROOT / "scripts" / "ci" / "mutation_ai_outcomes.py").read_text(encoding="utf-8")
        self.assertIn(VERIFY_BASELINE_ERROR, source)
        self.assertIn(VERIFY_NO_MUTANTS_ERROR, source)
        self.assertIn(VERIFY_END_TIME_ERROR, source)
        self.assertIn(VERIFY_INCOMPLETE_ERROR, source)
        stripped = source.replace(f'errors.append("{VERIFY_BASELINE_ERROR}")', "", 1)
        with self.assertRaises(AssertionError):
            self.assertIn(VERIFY_BASELINE_ERROR, stripped)
        stripped = source.replace(f'errors.append("{VERIFY_NO_MUTANTS_ERROR}")', "", 1)
        with self.assertRaises(AssertionError):
            self.assertIn(VERIFY_NO_MUTANTS_ERROR, stripped)

    def test_benchmark_alert_issue_runs_after_failed_regression_check(self) -> None:
        alert = (WORKFLOWS / "benchmark_alert.yml").read_text(encoding="utf-8")
        check = job_step_block(alert, "Check for regressions")
        if "continue-on-error" in check:
            raise AssertionError("regression_check must fail the job")
        if "shell: pwsh" not in check:
            raise AssertionError("regression_check must stay shell: pwsh")
        body = pwsh_run_body(check)
        if "-Strict" not in body:
            raise AssertionError("benchmark_alert regression check lost -Strict")
        if "exit 0" in body:
            raise AssertionError("regression_check must stay red; do not exit 0")

        strict = job_step_block(
            (WORKFLOWS / "benchmark.yml").read_text(encoding="utf-8"),
            "Strict threshold validation (main branch only)",
        )
        if "-Strict" not in strict:
            raise AssertionError("benchmark.yml -Strict was changed")
        if "continue-on-error" in strict:
            raise AssertionError("benchmark.yml strict step must not continue on error")

        code, output, report = run_alert_check_in_runner(body, "regression")
        self.assertNotEqual(code, 0)
        self.assertIn("has_regressions=true", output)
        self.assertTrue(report)

        code, output, report = run_alert_check_in_runner(body, "clean")
        self.assertEqual(code, 0)
        self.assertIn("has_regressions=false", output)
        self.assertFalse(report)

        code, _output, _report = run_alert_check_in_runner(body, "checker_error")
        self.assertNotEqual(code, 0)

        assert_alert_followup_runs(alert)
        mutant = alert
        for name in _ALERT_ISSUE_STEPS:
            expr = step_if_expression(job_step_block(mutant, name))
            stripped = without_status_guard(expr)
            self.assertNotEqual(stripped, expr)
            self.assertFalse(
                github_step_if_runs(
                    stripped,
                    failed=True,
                    cancelled=False,
                    outputs={("regression_check", "has_regressions"): "true"},
                )
            )
            mutant = mutant.replace(expr, stripped, 1)
        with self.assertRaises(AssertionError):
            assert_alert_followup_runs(mutant)

    def test_sccache_prefetch_retries_checksum_and_leaves_rustc_wrapper_alone(self) -> None:
        script = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"
        arch = subprocess.check_output(["node", "-p", "process.arch"], text=True).strip()
        platform = subprocess.check_output(["node", "-p", "process.platform"], text=True).strip()
        asset_arch = {"x64": "x86_64", "arm64": "aarch64", "arm": "armv7"}[arch]
        if platform == "darwin":
            asset_os = "apple-darwin"
        elif platform == "win32":
            asset_os = "pc-windows-msvc"
        elif arch == "arm":
            asset_os = "unknown-linux-musleabi"
        else:
            asset_os = "unknown-linux-musl"
        filename = f"sccache-v0.19.0-{asset_arch}-{asset_os}.tar.gz"

        with tempfile.TemporaryDirectory() as tmp:
            archive_path = Path(tmp) / filename
            with tarfile.open(archive_path, "w:gz") as archive:
                payload = Path(tmp) / "sccache"
                payload.write_text("#!/bin/sh\necho sccache 0.19.0\n", encoding="utf-8")
                payload.chmod(0o755)
                info = tarfile.TarInfo(name=f"sccache-v0.19.0-{asset_arch}-{asset_os}/sccache")
                info.size = payload.stat().st_size
                info.mode = 0o755
                with payload.open("rb") as handle:
                    archive.addfile(info, handle)
            blob = archive_path.read_bytes()
            pinned = PINNED_SCCACHE_SHA256[filename]
            script_text = script.read_text(encoding="utf-8")
            self.assertIn(pinned, script_text)
            patched = Path(tmp) / "fetch-ok.sh"
            patched.write_text(script_text.replace(pinned, hashlib.sha256(blob).hexdigest(), 1), encoding="utf-8")

            class Handler(http.server.BaseHTTPRequestHandler):
                def do_GET(self) -> None:  # noqa: N802
                    server = self.server
                    assert isinstance(server, http.server.ThreadingHTTPServer)
                    hits = getattr(server, "hits")
                    mode = getattr(server, "mode")
                    hits.append(self.path)
                    if self.path.endswith(".tar.gz"):
                        if mode == "504":
                            body, code = b"gateway", 504
                        elif mode == "badsum":
                            body, code = blob, 200
                        else:
                            body, code = blob, 200
                    else:
                        body, code = b"missing", 404
                    self.send_response(code)
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)

                def log_message(self, fmt: str, *args: object) -> None:
                    return

            server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            setattr(server, "hits", [])
            setattr(server, "mode", "ok")
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            port = server.server_address[1]
            try:
                def run(
                    mode: str, script_path: Path
                ) -> tuple[subprocess.CompletedProcess[str], list[str], Path]:
                    setattr(server, "mode", mode)
                    hits = getattr(server, "hits")
                    hits.clear()
                    work = Path(tmp) / mode
                    work.mkdir()
                    env = os.environ.copy()
                    env["GITHUB_OUTPUT"] = str(work / "output")
                    env["GITHUB_ENV"] = str(work / "env")
                    env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                    # Workflows cannot set this. The script must ignore it.
                    env["SCCACHE_RELEASE_BASE"] = "http://127.0.0.1:1/ignored"
                    env["SCCACHE_FETCH_ATTEMPTS"] = "2"
                    env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                    (work / "env").write_text("", encoding="utf-8")
                    result = subprocess.run(
                        [
                            "bash",
                            str(script_path),
                            "--release-base",
                            f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        ],
                        env=env,
                        text=True,
                        capture_output=True,
                        check=False,
                    )
                    return result, list(hits), work

                ok, ok_hits, ok_work = run("ok", patched)
                self.assertEqual(ok.returncode, 0, ok.stderr)
                self.assertEqual((ok_work / "output").read_text(encoding="utf-8"), "available=true\n")
                self.assertEqual((ok_work / "env").read_text(encoding="utf-8"), "")
                self.assertEqual(ok_hits, [f"/releases/download/v0.19.0/{filename}"], ok_hits)
                cached = ok_work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                self.assertTrue(cached.is_file(), cached)
                self.assertTrue((ok_work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete").is_file())
                self.assertEqual(
                    subprocess.check_output([str(cached)], text=True).strip(),
                    "sccache 0.19.0",
                )

                down, down_hits, down_work = run("504", script)
                self.assertEqual(down.returncode, 0, down.stderr)
                self.assertEqual((down_work / "output").read_text(encoding="utf-8"), "available=false\n")
                self.assertEqual((down_work / "env").read_text(encoding="utf-8"), "")
                self.assertEqual(len(down_hits), 2, down_hits)
                self.assertTrue(all(hit.endswith(".tar.gz") for hit in down_hits), down_hits)
                self.assertIn("retrying in 0s", down.stderr)
                self.assertNotIn("RUSTC_WRAPPER", down.stdout + down.stderr)

                bad, bad_hits, bad_work = run("badsum", script)
                self.assertNotEqual(bad.returncode, 0, bad.stderr)
                bad_output = (bad_work / "output").read_text(encoding="utf-8") if (bad_work / "output").exists() else ""
                self.assertNotIn("available=false", bad_output)
                self.assertEqual((bad_work / "env").read_text(encoding="utf-8"), "")
                self.assertEqual(bad_hits, [f"/releases/download/v0.19.0/{filename}"], bad_hits)
                self.assertIn("checksum mismatch", bad.stderr)
                self.assertNotIn("retrying", bad.stderr)
                self.assertFalse((bad_work / "tool-cache").exists())
            finally:
                server.shutdown()
                server.server_close()

    def test_sccache_prefetch_accepts_zero_padded_attempts(self) -> None:
        filename, _arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            with _fixed_body_server(b"no", status=504) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    attempts="08",
                    delay="0",
                )
                hits = list(getattr(server, "hits"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertEqual(len(hits), 8, hits)
        self.assertIn("attempt 1/8 failed", result.stderr)
        self.assertNotIn("attempt 1/08 failed", result.stderr)
        self.assertTrue(all(hit.endswith(filename) for hit in hits), hits)

    def test_sccache_prefetch_accepts_zero_padded_delay(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "sleep", "#!/bin/sh\nexit 0\n")
            _bind_tool(script, "/usr/bin/sleep", work / "bin" / "sleep")
            with _fixed_body_server(b"no", status=504) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    attempts="2",
                    delay="08",
                )
                hits = list(getattr(server, "hits"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertEqual(len(hits), 2, hits)
        self.assertIn("retrying in 8s", result.stderr)
        self.assertNotIn("retrying in 08s", result.stderr)

    def test_sccache_prefetch_falls_back_when_node_query_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "node", "#!/bin/sh\nexit 1\n")
            _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
            result, output = _run_fetch(work, script=script)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not read the runner arch", result.stderr)

    def test_sccache_prefetch_falls_back_when_mktemp_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "mktemp", "#!/bin/sh\nexit 1\n")
            _bind_tool(script, "/usr/bin/mktemp", work / "bin" / "mktemp")
            result, output = _run_fetch(work, script=script)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not create a temp directory", result.stderr)

    def test_sccache_prefetch_falls_back_when_the_asset_has_no_pin(self) -> None:
        node = """#!/bin/sh
if [ "$1" = "-p" ] && [ "$2" = "process.arch" ]; then
  printf '%s\\n' arm
  exit 0
fi
if [ "$1" = "-p" ] && [ "$2" = "process.platform" ]; then
  printf '%s\\n' darwin
  exit 0
fi
exit 1
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "node", node)
            _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
            with _fixed_body_server(b"not-the-archive", status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                hits = list(getattr(server, "hits"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertEqual(hits, [])
        self.assertIn("no pinned checksum", result.stderr)
        self.assertNotIn("checksum mismatch", result.stderr)

    def test_sccache_prefetch_falls_back_when_hashing_fails(self) -> None:
        node = """#!/bin/sh
if [ "$1" = "-p" ] && [ "$2" = "process.arch" ]; then
  printf '%s\\n' x64
  exit 0
fi
if [ "$1" = "-p" ] && [ "$2" = "process.platform" ]; then
  printf '%s\\n' linux
  exit 0
fi
exit 1
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "node", node)
            _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
            with _fixed_body_server(b"archive-bytes", status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                tool_cache_exists = (work / "tool-cache").exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("unexpected error", result.stderr)
        self.assertFalse(tool_cache_exists)
        self.assertNotIn("checksum mismatch", result.stderr)

    def test_sccache_prefetch_falls_back_when_extract_fails(self) -> None:
        body = b"this is not a gzip archive"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_pinned_to(work, hashlib.sha256(body).hexdigest())
            with _fixed_body_server(body, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                tool_cache_exists = (work / "tool-cache").exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not extract the archive", result.stderr)
        self.assertFalse(tool_cache_exists)

    def test_sccache_prefetch_falls_back_when_the_tool_cache_is_read_only(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            cache = work / "tool-cache"
            cache.mkdir()
            cache.chmod(0o555)
            try:
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        tool_cache=cache,
                    )
                    installed = (cache / "sccache" / "0.19.0" / arch / "sccache").exists()
            finally:
                cache.chmod(0o755)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not install sccache into the tool cache", result.stderr)
        self.assertFalse(installed)

    def test_sccache_prefetch_falls_back_when_the_tool_cache_is_a_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            cache = work / "tool-cache-file"
            cache.write_text("not a directory", encoding="utf-8")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    tool_cache=cache,
                )
            still = cache.read_text(encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not install sccache into the tool cache", result.stderr)
        self.assertNotIn("checksum mismatch", result.stderr)
        self.assertEqual(still, "not a directory")

    def test_sccache_prefetch_removes_a_partial_tool_cache_install(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\nprintf leaked > \"$3/leaked\"\nexit 1\n",
            )
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                dest = work / "tool-cache" / "sccache" / "0.19.0" / arch
                dest_exists = dest.exists()
                marker_exists = (dest.parent / f"{arch}.complete").exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not install sccache into the tool cache", result.stderr)
        self.assertFalse(dest_exists, result.stderr)
        self.assertFalse(marker_exists)

    def test_sccache_prefetch_falls_back_when_step_output_is_unwritable(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            output_path = work / "output"
            output_path.write_bytes(b"")
            output_path.chmod(0)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                env = os.environ.copy()
                env["GITHUB_OUTPUT"] = str(output_path)
                env["GITHUB_ENV"] = str(work / "env")
                env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                env["SCCACHE_FETCH_ATTEMPTS"] = "1"
                env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                result = subprocess.run(
                    [
                        "bash",
                        str(script),
                        "--release-base",
                        f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    ],
                    env=env,
                    text=True,
                    capture_output=True,
                    check=False,
                )
            output_path.chmod(0o644)
            output = output_path.read_text(encoding="utf-8")
            installed = (work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache").exists()
            marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
            marker_exists = marker.exists()
            dest_exists = (work / "tool-cache" / "sccache" / "0.19.0" / arch).exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("available=true", output)
        self.assertNotIn("available=false", output)
        self.assertIn("unexpected error", result.stderr)
        self.assertNotIn("checksum mismatch", result.stderr)
        self.assertFalse(installed, result.stderr)
        self.assertFalse(marker_exists)
        self.assertFalse(dest_exists)

    def test_sccache_prefetch_falls_back_when_temp_cleanup_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_rm_that_refuses_tree_wipe(work / "bin")
            _bind_tool(script, "/usr/bin/rm", work / "bin" / "rm")
            result, output = _run_fetch(
                work,
                script=script,
                base="http://127.0.0.1:1/releases/download/v0.19.0",
                attempts="1",
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertNotIn("available=true", output)
        self.assertIn("release download failed", result.stderr)
        self.assertNotIn("checksum mismatch", result.stderr)

    def test_sccache_prefetch_checksum_mismatch_stays_fatal_when_cleanup_fails(self) -> None:
        filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_copy(work)
            _write_rm_that_refuses_tree_wipe(work / "bin")
            _bind_tool(script, "/usr/bin/rm", work / "bin" / "rm")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                marker_exists = marker.exists()
                cached_exists = cached.exists()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertIn(filename, result.stderr)
        self.assertNotIn("available=false", output)
        self.assertNotIn("available=true", output)
        self.assertNotIn("unexpected error", result.stderr)
        self.assertFalse(marker_exists)
        self.assertFalse(cached_exists)

    def test_sccache_prefetch_rejects_a_non_regular_binary_entry(self) -> None:
        _filename, arch = _host_sccache_filename()
        for kind in ("symlink", "hardlink", "directory"):
            with self.subTest(kind=kind):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    blob = _non_regular_sccache_archive(work, kind)
                    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                    with _fixed_body_server(blob, status=200) as server:
                        port = server.server_address[1]
                        result, output = _run_fetch(
                            work,
                            script=script,
                            base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        )
                        marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                        cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                        marker_exists = marker.exists()
                        cached_exists = cached.exists()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, "available=false\n")
                if kind == "directory":
                    self.assertIn("archive has no sccache binary", result.stderr)
                else:
                    self.assertIn("archive has a symlink or non-regular entry", result.stderr)
                self.assertNotIn("checksum mismatch", result.stderr)
                self.assertFalse(marker_exists)
                self.assertFalse(cached_exists)

        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\nln -s /etc/hostname \"$2\"\nexit 0\n",
            )
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                marker_exists = marker.exists()
                cached_is_link = cached.is_symlink()
                dest_exists = cached.parent.exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("tool cache is missing the sccache binary", result.stderr)
        self.assertFalse(marker_exists)
        self.assertFalse(cached_is_link)
        self.assertFalse(dest_exists)

    def test_sccache_prefetch_binds_one_platform_binary_and_rejects_links(self) -> None:
        _filename, arch = _host_sccache_filename()
        script_text = (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh").read_text(
            encoding="utf-8"
        )
        pre = script_text.split('fail_closed "archive has no sccache binary"', 1)[0]
        pre_check = [line for line in pre.splitlines() if "plain_binary" in line][-1]
        self.assertEqual(pre_check.strip(), 'if ! plain_binary "$inner/$binary_name"; then')
        post = script_text.split(
            'fail_closed "tool cache is missing the sccache binary"', 1
        )[0]
        post_check = [line for line in post.splitlines() if "plain_binary" in line][-1]
        self.assertIn('plain_binary "$dest/$binary_name"', post_check)
        self.assertNotIn("sccache.exe", post_check)
        self.assertIn("binary_name=sccache.exe", script_text)
        self.assertNotIn("cp -R", script_text)
        self.assertNotIn("cp -L", script_text)
        self.assertIn(
            '"$CP_BIN" "${inner}/${binary_name}" "${dest}/${binary_name}"',
            script_text,
        )
        self.assertIn("pick_tool /usr/bin/cp /bin/cp", script_text)
        self.assertNotIn("for candidate in", script_text)
        self.assertIn('if ! plain_directory "$inner"; then', script_text)
        self.assertLess(
            script_text.find("archive_plain_members"),
            script_text.find('"$TAR_BIN" -xzf'),
        )
        plain_binary = script_text.split("plain_binary() {", 1)[1].split("plain_directory() {", 1)[0]
        self.assertIn("fs.lstatSync", plain_binary)
        self.assertNotIn("fs.statSync", plain_binary)
        branch = script_text.split('if [ "${#want}" -ne 64 ]', 1)[1].split("\nfi", 1)[0]
        self.assertLess(branch.find("trap - ERR"), branch.find("builtin printf"))
        self.assertNotIn("|| true", branch)
        echo_at = script_text.rfind('echo "sccache ${VERSION} cached at ${dest}" || true')
        available_at = script_text.rfind('write_output "available=true"')
        marker_token = "printf '' > \"${dest}.complete\""
        self.assertEqual(script_text.count(marker_token), 1)
        marker_at = script_text.find(marker_token)
        self.assertLess(
            script_text.find('archive_plain_members "$tmp/pkg.tar.gz"'),
            script_text.find('"$TAR_BIN" -xzf'),
        )
        self.assertGreater(echo_at, 0)
        self.assertLess(echo_at, available_at)
        self.assertLess(available_at, marker_at)
        self.assertIn('echo "sccache prefetch failed: $*" >&2 || true', script_text)

        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        rejected = (
            (
                "mixed-symlink",
                [
                    ("sccache", "symlink", "/etc/hostname"),
                    ("sccache.exe", "file", payload),
                ],
                "archive has a symlink or non-regular entry",
            ),
            (
                "mixed-directory",
                [
                    ("sccache", "dir", ""),
                    ("sccache.exe", "file", payload),
                ],
                "archive has no sccache binary",
            ),
            (
                "exe-only",
                [("sccache.exe", "file", payload)],
                "archive has no sccache binary",
            ),
            (
                "nested-symlink",
                [
                    ("sccache", "file", payload),
                    ("LICENSE", "file", b"license\n"),
                    ("nested/cargo", "symlink", "/etc/hostname"),
                ],
                "archive has a symlink or non-regular entry",
            ),
            (
                "fifo",
                [
                    ("sccache", "file", payload),
                    ("fifo", "fifo", ""),
                ],
                "archive has a symlink or non-regular entry",
            ),
        )
        for name, members, message in rejected:
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    blob = _packed_archive(work, members)
                    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                    with _fixed_body_server(blob, status=200) as server:
                        port = server.server_address[1]
                        result, output = _run_fetch(
                            work,
                            script=script,
                            base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        )
                    _assert_prefetch_fell_back(
                        self, result, output, work, arch, message=message
                    )

        with self.subTest(name="top-level-symlink"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                outside = work / "outside"
                outside.mkdir()
                planted = outside / "sccache"
                planted.write_bytes(payload)
                planted.chmod(0o755)
                blob = _packed_archive(work, [("", "link-top", str(outside))])
                script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    )
                _assert_prefetch_fell_back(
                    self,
                    result,
                    output,
                    work,
                    arch,
                    message="archive has a symlink or non-regular entry",
                )

        with self.subTest(name="sibling-link-in-the-tool-cache"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                blob = _packed_archive(
                    work,
                    [
                        ("sccache", "file", payload),
                        ("README.md", "file", b"readme\n"),
                    ],
                )
                script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                _write_exe(
                    work / "bin",
                    "cp",
                    "#!/bin/sh\n/bin/cp \"$1\" \"$2\"\n"
                    "ln -s /etc/hostname \"$(dirname \"$2\")/cargo\"\nexit 0\n",
                )
                _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    )
                _assert_prefetch_fell_back(
                    self,
                    result,
                    output,
                    work,
                    arch,
                    message="tool cache is missing the sccache binary",
                )

        with self.subTest(name="regular-files-install"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                blob = _packed_archive(
                    work,
                    [
                        ("sccache", "file", payload),
                        ("LICENSE", "file", b"license\n"),
                        ("README.md", "file", b"readme\n"),
                    ],
                )
                script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    )
                    cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                    marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                    ran = subprocess.check_output([str(cached)], text=True).strip()
                    cached_is_file = cached.is_file()
                    cached_is_link = cached.is_symlink()
                    marker_is_file = marker.is_file()
                    names = sorted(path.name for path in cached.parent.iterdir())
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=true\n")
            self.assertNotIn("available=false", output)
            self.assertTrue(cached_is_file)
            self.assertFalse(cached_is_link)
            self.assertTrue(marker_is_file)
            self.assertEqual(names, ["sccache"])
            self.assertEqual(ran, "sccache 0.19.0")
            self.assertIn("cached at", result.stdout)

    def test_sccache_prefetch_uses_sccache_exe_on_win32(self) -> None:
        real_node = shutil.which("node")
        self.assertIsNotNone(real_node)
        windows = "sccache-v0.19.0-x86_64-pc-windows-msvc.tar.gz"
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        node = f"""#!/bin/sh
if [ "$1" = "-p" ] && [ "$2" = "process.platform" ]; then
  printf '%s\\n' win32
  exit 0
fi
if [ "$1" = "-p" ] && [ "$2" = "process.arch" ]; then
  printf '%s\\n' x64
  exit 0
fi
exec {real_node} "$@"
"""
        with self.subTest(name="exe-installs"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                _write_exe(work / "bin", "node", node)
                blob = _packed_archive(
                    work,
                    [("sccache.exe", "file", payload)],
                    filename=windows,
                )
                script = _script_pinned_named(work, windows, hashlib.sha256(blob).hexdigest())
                _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    )
                    cached = work / "tool-cache" / "sccache" / "0.19.0" / "x64" / "sccache.exe"
                    marker = work / "tool-cache" / "sccache" / "0.19.0" / "x64.complete"
                    linux_name = work / "tool-cache" / "sccache" / "0.19.0" / "x64" / "sccache"
                    cached_is_file = cached.is_file()
                    cached_is_link = cached.is_symlink()
                    marker_is_file = marker.is_file()
                    linux_name_exists = linux_name.exists()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=true\n")
            self.assertTrue(cached_is_file)
            self.assertFalse(cached_is_link)
            self.assertTrue(marker_is_file)
            self.assertFalse(linux_name_exists)

        with self.subTest(name="linux-name-falls-back"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                _write_exe(work / "bin", "node", node)
                blob = _packed_archive(
                    work,
                    [("sccache", "file", payload)],
                    filename=windows,
                )
                script = _script_pinned_named(work, windows, hashlib.sha256(blob).hexdigest())
                _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    )
                _assert_prefetch_fell_back(
                    self,
                    result,
                    output,
                    work,
                    "x64",
                    message="archive has no sccache binary",
                )

    def test_sccache_prefetch_log_faults_keep_the_recorded_exit(self) -> None:
        _filename, arch = _host_sccache_filename()
        script = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"

        with self.subTest(name="stderr-fallback"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                env = os.environ.copy()
                env.pop("BASH_ENV", None)
                env.pop("ENV", None)
                env["GITHUB_OUTPUT"] = str(work / "output")
                env["GITHUB_ENV"] = str(work / "env")
                env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                env["SCCACHE_FETCH_ATTEMPTS"] = "1"
                env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                (work / "env").write_text("", encoding="utf-8")
                with open("/dev/full", "w", encoding="utf-8") as full:
                    result = subprocess.run(
                        [
                            "bash",
                            str(script),
                            "--release-base",
                            "http://127.0.0.1:1/releases/download/v0.19.0",
                        ],
                        env=env,
                        stdout=subprocess.PIPE,
                        stderr=full,
                        text=True,
                        check=False,
                    )
                output = (work / "output").read_text(encoding="utf-8")
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual(output, "available=false\n")
            self.assertNotIn("available=true", output)

        with self.subTest(name="stdout-success"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                blob = _valid_sccache_archive(work)
                patched = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    env = os.environ.copy()
                    env.pop("BASH_ENV", None)
                    env.pop("ENV", None)
                    env["GITHUB_OUTPUT"] = str(work / "output")
                    env["GITHUB_ENV"] = str(work / "env")
                    env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                    env["SCCACHE_FETCH_ATTEMPTS"] = "1"
                    env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                    (work / "env").write_text("", encoding="utf-8")
                    with open("/dev/full", "w", encoding="utf-8") as full:
                        result = subprocess.run(
                            [
                                "bash",
                                str(patched),
                                "--release-base",
                                f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                            ],
                            env=env,
                            stdout=full,
                            stderr=subprocess.PIPE,
                            text=True,
                            check=False,
                        )
                    output = (work / "output").read_text(encoding="utf-8")
                    cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                    marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                    cached_is_file = cached.is_file()
                    marker_is_file = marker.is_file()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=true\n")
            self.assertNotIn("available=false", output)
            self.assertTrue(cached_is_file)
            self.assertTrue(marker_is_file)

        with self.subTest(name="missing-release-base"):
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                result = subprocess.run(
                    ["bash", str(script), "--release-base"],
                    env={
                        "PATH": os.environ["PATH"],
                        "HOME": os.environ.get("HOME", "/tmp"),
                        "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
                        "GITHUB_OUTPUT": str(work / "output"),
                        "GITHUB_ENV": str(work / "env"),
                        "RUNNER_TOOL_CACHE": str(work / "tool-cache"),
                    },
                    text=True,
                    capture_output=True,
                    check=False,
                )
                output = (work / "output").read_text(encoding="utf-8")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=false\n")
            self.assertIn("--release-base needs a URL", result.stderr)

    def test_sccache_prefetch_checksum_mismatch_survives_a_log_fault(self) -> None:
        filename, arch = _host_sccache_filename()
        script = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                base = f"http://127.0.0.1:{port}/releases/download/v0.19.0"
                clean = {
                    "PATH": os.environ["PATH"],
                    "HOME": os.environ.get("HOME", "/tmp"),
                    "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
                    "GITHUB_OUTPUT": str(work / "output"),
                    "GITHUB_ENV": str(work / "env"),
                    "RUNNER_TOOL_CACHE": str(work / "tool-cache"),
                    "SCCACHE_FETCH_ATTEMPTS": "1",
                    "SCCACHE_FETCH_DELAY_SECONDS": "0",
                }
                self.assertNotIn("BASH_ENV", clean)
                self.assertNotIn("ENV", clean)
                (work / "env").write_text("", encoding="utf-8")
                with open("/dev/full", "w", encoding="utf-8") as full:
                    blocked = subprocess.run(
                        [
                            "env",
                            "-i",
                            *[f"{key}={value}" for key, value in clean.items()],
                            "bash",
                            str(script),
                            "--release-base",
                            base,
                        ],
                        stdout=subprocess.PIPE,
                        stderr=full,
                        text=True,
                        check=False,
                    )
                blocked_output = (
                    (work / "output").read_text(encoding="utf-8")
                    if (work / "output").is_file()
                    else ""
                )
                (work / "output").unlink(missing_ok=True)
                hostile = work / "hostile-printf.sh"
                hostile.write_text(
                    "printf() {\n"
                    '  case "$1" in\n'
                    "    *checksum\\ mismatch*) return 1 ;;\n"
                    "  esac\n"
                    '  builtin printf "$@"\n'
                    "}\n"
                    "echo() {\n"
                    '  case "$*" in\n'
                    "    *checksum\\ mismatch*) return 1 ;;\n"
                    "  esac\n"
                    '  builtin echo "$@"\n'
                    "}\n",
                    encoding="utf-8",
                )
                shadowed = os.environ.copy()
                shadowed["BASH_ENV"] = str(hostile)
                shadowed["ENV"] = str(hostile)
                shadowed["GITHUB_OUTPUT"] = str(work / "output")
                shadowed["GITHUB_ENV"] = str(work / "env")
                shadowed["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                shadowed["SCCACHE_FETCH_ATTEMPTS"] = "1"
                shadowed["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                shadowed_run = subprocess.run(
                    ["bash", str(script), "--release-base", base],
                    env=shadowed,
                    text=True,
                    capture_output=True,
                    check=False,
                )
                shadowed_output = (
                    (work / "output").read_text(encoding="utf-8")
                    if (work / "output").is_file()
                    else ""
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                marker_exists = marker.exists()
                cached_exists = cached.exists()
        self.assertEqual(blocked.returncode, 1, blocked.stdout)
        self.assertNotIn("available=", blocked_output)
        self.assertEqual(shadowed_run.returncode, 1, shadowed_run.stderr)
        self.assertIn("checksum mismatch", shadowed_run.stderr)
        self.assertIn(filename, shadowed_run.stderr)
        self.assertNotIn("available=", shadowed_output)
        self.assertNotIn("unexpected error", shadowed_run.stderr)
        self.assertFalse(marker_exists)
        self.assertFalse(cached_exists)

    def test_sccache_fetch_steps_use_a_clean_environment(self) -> None:
        violations = sccache_fetch_run_violations()
        self.assertEqual(violations, [])
        action = (ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml").read_text(
            encoding="utf-8"
        )
        for old, new in (
            (SCCACHE_FETCH_COMMAND, "bash scripts/ci/fetch_sccache_release.sh"),
            (' RUNNER_TOOL_CACHE="$RUNNER_TOOL_CACHE"', ""),
            ("bash --noprofile --norc ", "bash "),
        ):
            mutant = action.replace(old, new, 1)
            self.assertNotEqual(mutant, action)
            self.assertTrue(sccache_fetch_run_violations([("action.yml", mutant)]))
        self.assertTrue(
            SCCACHE_FETCH_COMMAND.startswith(
                'if [ "$(/usr/bin/uname -s)" = Darwin ]; then exec /usr/bin/env -i '
            )
        )
        self.assertIn("/bin/bash --noprofile --norc ", SCCACHE_FETCH_COMMAND)
        self.assertIn("/usr/bin/bash --noprofile --norc ", SCCACHE_FETCH_COMMAND)
        self.assertNotIn(" bash --noprofile", SCCACHE_FETCH_COMMAND)
        self.assertNotIn("PATH=", SCCACHE_FETCH_COMMAND)
        self.assertEqual(SCCACHE_FETCH_COMMAND.count('RUNNER_OS="$RUNNER_OS"'), 2)

    def test_sccache_fetch_rejects_unpinned_env_forms(self) -> None:
        pinned = SCCACHE_FETCH_COMMAND
        forms = {
            "no-dash-i": pinned.replace("/usr/bin/env -i ", "/usr/bin/env ", 1),
            "extra-flag": pinned.replace(
                "bash --noprofile --norc",
                "bash --noprofile --norc --rcfile",
                1,
            ),
            "dash-c": pinned.replace("bash --noprofile --norc", "bash -c", 1),
            "bash-env": pinned.replace(
                "/usr/bin/env -i ",
                "/usr/bin/env -i " + "BASH_" + "ENV=/tmp/hook.sh ",
                1,
            ),
            "ld-preload": pinned.replace(
                "/usr/bin/env -i ",
                "/usr/bin/env -i " + "LD_" + "PRELOAD=/tmp/hook.so ",
                1,
            ),
            "path-value": pinned.replace(
                "/usr/bin/env -i ",
                '/usr/bin/env -i PATH="/tmp/hook" ',
                1,
            ),
        }
        clean = sccache_fetch_run_violations([("clean.yml", _probe_workflow(pinned))])
        self.assertEqual(clean, [])
        for label, run in forms.items():
            found = sccache_fetch_run_violations([(f"{label}.yml", _probe_workflow(run))])
            self.assertTrue(any("allowlisted" in item for item in found), (label, found))

    def test_sccache_fetch_count_rejects_extra_and_removed_sites(self) -> None:
        texts = _sccache_policy_texts()
        label, body = next(item for item in texts if item[0].endswith("benchmark_alert.yml"))
        self.assertIn(SCCACHE_FETCH_COMMAND, body)
        extra_job = (
            "\n  aw_fetch_probe:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: Fetch\n"
            "        shell: bash\n"
            "        run: |\n"
            f"          {SCCACHE_FETCH_COMMAND}\n"
        )
        extra = [(name, text if name != label else body + extra_job) for name, text in texts]
        added = sccache_fetch_run_violations(extra, expected_fetch_count=10)
        self.assertTrue(any("expected 10 fetch steps, found 11" in item for item in added), added)
        removed_body = body.replace(SCCACHE_FETCH_COMMAND, "echo kept", 1)
        self.assertNotEqual(removed_body, body)
        removed = [(name, text if name != label else removed_body) for name, text in texts]
        missing = sccache_fetch_run_violations(removed, expected_fetch_count=10)
        self.assertTrue(
            any("expected 10 fetch steps, found 9" in item for item in missing),
            missing,
        )

    def test_sccache_fetch_rejects_indirect_script_names(self) -> None:
        tree = "scripts" + "/ci/"
        glue = "bash " + "scr" + "''" + "ipts/" + "c" + "''" + "i/tool.sh"
        globbed = "bash " + tree + "fetch_sccache_rel" + "*.sh"
        star_dir = "bash " + "scripts" + "/*/tool.sh"
        escaped = "bash '" + "scri" + "\\pts/" + "ci/tool.sh'"
        open_quote = 'bash "' + tree + "tool.sh"
        variable = "\n".join(
            [
                "root=scripts",
                "leaf=" + "ci/tool.sh",
                'bash "${root}/$leaf"',
            ]
        )
        same_line = "root=scripts leaf=ci/tool.sh bash " + '"$root/$leaf"'
        unused = "keeper=" + tree + "tool.sh\necho hello"
        extra = "bash " + tree + "not_pinned.sh"
        trailing = extra + " # note"
        comment = "# " + tree + "tool.sh"
        noted = "echo hello # " + tree + "tool.sh"
        stem = "bash " + "fetch_sccache_release.sh"
        for label, run in (
            ("glue", glue),
            ("glob", globbed),
            ("star-dir", star_dir),
            ("escaped", escaped),
            ("open-quote", open_quote),
            ("variable", variable),
            ("same-line", same_line),
            ("unused", unused),
            ("extra", extra),
            ("trailing", trailing),
            ("stem", stem),
            ("dot-segment", "bash scripts/./ci/tool.sh"),
            ("alias", "alias f=scripts/ci/tool.sh; f"),
            ("alias-quoted", "alias f='scripts/ci/tool.sh'; f"),
            ("alias-next", "alias f=scripts/ci/tool.sh\nf"),
            ("glued-assign", "true f=scripts/ci/tool.sh"),
        ):
            found = sccache_fetch_run_violations([(f"{label}.yml", _probe_workflow(run))])
            self.assertTrue(any("allowlisted" in item for item in found), (label, found))
        quiet = sccache_fetch_run_violations(
            [
                ("echo.yml", _probe_workflow("echo hello")),
                ("other.yml", _probe_workflow("bash scripts/other/tool.sh")),
                ("city.yml", _probe_workflow("bash scripts/city/tool.sh")),
                ("note.yml", _probe_workflow(comment + "\ntrue")),
                ("noted.yml", _probe_workflow(noted)),
            ]
        )
        self.assertEqual(quiet, [])

    def _mismatch_via_workflow(
        self,
        work: Path,
        hostile: dict[str, str] | None = None,
    ) -> tuple[subprocess.CompletedProcess[str], str, str]:
        filename, arch = _host_sccache_filename()
        blob = _valid_sccache_archive(work)
        with _fixed_body_server(blob, status=200) as server:
            port = server.server_address[1]
            result, output = _run_workflow_fetch(
                work,
                base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                hostile=hostile,
            )
        return result, output, arch

    def test_sccache_fetch_ignores_node_options(self) -> None:
        filename, _arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            hook = work / "hook.js"
            hook.write_text(
                "const crypto = require('crypto');\n"
                f"const forced = {PINNED_SCCACHE_SHA256[filename]!r};\n"
                "const orig = crypto.createHash;\n"
                "crypto.createHash = function (algo) {\n"
                "  const hash = orig.call(crypto, algo);\n"
                "  if (algo !== 'sha256') return hash;\n"
                "  const digest = hash.digest.bind(hash);\n"
                "  hash.digest = function (enc) {\n"
                "    if (enc === 'hex') return forced;\n"
                "    return digest(enc);\n"
                "  };\n"
                "  return hash;\n"
                "};\n",
                encoding="utf-8",
            )
            result, output, arch = self._mismatch_via_workflow(
                work,
                {"NODE_OPTIONS": f"--require {hook}"},
            )
            marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
            self.assertFalse(marker.exists())
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=", output)

    def test_sccache_fetch_ignores_exported_shell_function(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output, _arch = self._mismatch_via_workflow(
                work,
                {"BASH_FUNC_exit%%": "() {  command exit 0\n}"},
            )
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=", output)

    def test_sccache_fetch_ignores_bash_env(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            startup = work / "startup.sh"
            startup.write_text("exit() { command exit 0; }\n", encoding="utf-8")
            result, output, _arch = self._mismatch_via_workflow(
                work,
                {"BASH_ENV": str(startup), "ENV": str(startup)},
            )
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=", output)

    def test_sccache_fetch_ignores_shell_options(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output, _arch = self._mismatch_via_workflow(
                work,
                {"SHELLOPTS": "noexec", "BASHOPTS": "extdebug"},
            )
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=", output)

    def test_sccache_fetch_ignores_ld_preload(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            marker = work / "preload-hit"
            source = work / "preload.c"
            source.write_text(
                "#include <stdio.h>\n"
                "#include <string.h>\n"
                "#include <unistd.h>\n"
                "__attribute__((constructor)) static void aw_preload(void) {\n"
                "  char comm[64];\n"
                "  FILE *handle = fopen(\"/proc/self/comm\", \"r\");\n"
                "  if (!handle) return;\n"
                "  if (fgets(comm, sizeof comm, handle) == NULL) {\n"
                "    fclose(handle);\n"
                "    return;\n"
                "  }\n"
                "  fclose(handle);\n"
                "  if (strncmp(comm, \"bash\", 4) != 0) return;\n"
                f"  handle = fopen(\"{marker}\", \"w\");\n"
                "  if (handle) { fputs(\"loaded\\n\", handle); fclose(handle); }\n"
                "  _exit(0);\n"
                "}\n",
                encoding="utf-8",
            )
            library = work / "preload.so"
            built = subprocess.run(
                ["gcc", "-shared", "-fPIC", "-o", str(library), str(source)],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(built.returncode, 0, built.stderr)
            result, output, _arch = self._mismatch_via_workflow(
                work,
                {"LD_PRELOAD": str(library)},
            )
            self.assertFalse(marker.exists())
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=", output)

    def test_sccache_fetch_ignores_tmpdir(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            hostile_tmp = work / "hostile-tmp"
            hostile_tmp.mkdir()
            hostile_tmp.chmod(0)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            try:
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_workflow_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        hostile={"TMPDIR": str(hostile_tmp)},
                    )
                    marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                    marker_exists = marker.is_file()
            finally:
                hostile_tmp.chmod(0o755)
            left = list(hostile_tmp.iterdir())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(marker_exists)
        self.assertEqual(left, [])

    def test_sccache_fetch_ignores_curlrc(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            home = work / "home"
            home.mkdir()
            (home / ".curlrc").write_text('proxy = "http://127.0.0.1:9"\n', encoding="utf-8")
            curl_home = work / "curl-home"
            curl_home.mkdir()
            (curl_home / ".curlrc").write_text('proxy = "http://127.0.0.1:9"\n', encoding="utf-8")
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_workflow_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    hostile={"HOME": str(home), "CURL_HOME": str(curl_home)},
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                ran = subprocess.check_output([str(cached)], text=True).strip()
                marker_is_file = marker.is_file()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(marker_is_file)
        self.assertEqual(ran, "sccache 0.19.0")

    def test_sccache_fetch_keeps_archive_bytes_when_cp_is_replaced(self) -> None:
        _filename, arch = _host_sccache_filename()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\nprintf '%s\\n' '#!/bin/sh' 'echo SWAPPED' > \"$2\"\nchmod 755 \"$2\"\nexit 0\n",
            )
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_workflow_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    hostile={"PATH": f"{work / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}"},
                )
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                ran = subprocess.check_output([str(cached)], text=True).strip()
                marker_is_file = marker.is_file()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(marker_is_file)
        self.assertEqual(ran, "sccache 0.19.0")
        self.assertNotIn("SWAPPED", ran)

        both = (
            "#!/bin/sh\n"
            "printf '%s\\n' '#!/bin/sh' 'echo SWAPPED' > \"$1\"\n"
            "printf '%s\\n' '#!/bin/sh' 'echo SWAPPED' > \"$2\"\n"
            "chmod 755 \"$1\" \"$2\"\n"
            "exit 0\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(work / "bin", "cp", both)
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            _assert_prefetch_fell_back(
                self,
                result,
                output,
                work,
                arch,
                message="tool cache is missing the sccache binary",
            )
            self.assertNotIn("SWAPPED", result.stdout)

    def test_sccache_prefetch_requires_the_release_root(self) -> None:
        filename, arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")]
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        decoy = b"#!/bin/sh\necho decoy\n"

        def pack(work: Path, members: list[tuple[str, bytes]]) -> bytes:
            archive_path = work / filename
            with tarfile.open(archive_path, "w:gz") as archive:
                for name, data in members:
                    info = tarfile.TarInfo(name=name)
                    info.size = len(data)
                    info.mode = 0o755
                    archive.addfile(info, io.BytesIO(data))
            return archive_path.read_bytes()

        cases = (
            (
                "decoy-sorts-first",
                [
                    (f"aaa/sccache", decoy),
                    (f"{root}/sccache", payload),
                ],
                "archive does not contain exactly one release root",
            ),
            (
                "sibling-file",
                [
                    (f"{root}/sccache", payload),
                    ("notes.txt", b"notes\n"),
                ],
                "archive does not contain exactly one release root",
            ),
            (
                "wrong-root-name",
                [("aaa/sccache", payload)],
                "archive release root does not match the asset",
            ),
            (
                "root-is-a-file",
                [(root, payload)],
                "archive release root is not a directory",
            ),
        )
        for name, members, message in cases:
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    blob = pack(work, members)
                    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                    with _fixed_body_server(blob, status=200) as server:
                        port = server.server_address[1]
                        result, output = _run_fetch(
                            work,
                            script=script,
                            base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        )
                    _assert_prefetch_fell_back(
                        self, result, output, work, arch, message=message
                    )
                    cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                    self.assertFalse(cached.exists())

    def test_sccache_prefetch_refuses_special_members_before_extract(self) -> None:
        filename, arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")]
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        outside_bytes = b"outside-the-archive\n"

        def special(kind: str, directory: Path) -> bytes:
            archive_path = directory / filename
            with tarfile.open(archive_path, "w:gz") as archive:
                if kind == "symlink":
                    link = tarfile.TarInfo(name=f"{root}/sccache")
                    link.type = tarfile.SYMTYPE
                    link.linkname = str(directory / "outside")
                    archive.addfile(link)
                elif kind == "fifo":
                    info = tarfile.TarInfo(name=f"{root}/sccache")
                    info.size = len(payload)
                    info.mode = 0o755
                    archive.addfile(info, io.BytesIO(payload))
                    fifo = tarfile.TarInfo(name=f"{root}/fifo")
                    fifo.type = tarfile.FIFOTYPE
                    archive.addfile(fifo)
                elif kind == "device":
                    info = tarfile.TarInfo(name=f"{root}/sccache")
                    info.size = len(payload)
                    info.mode = 0o755
                    archive.addfile(info, io.BytesIO(payload))
                    device = tarfile.TarInfo(name=f"{root}/device")
                    device.type = tarfile.CHRTYPE
                    device.devmajor = 1
                    device.devminor = 3
                    archive.addfile(device)
                elif kind == "block":
                    info = tarfile.TarInfo(name=f"{root}/sccache")
                    info.size = len(payload)
                    info.mode = 0o755
                    archive.addfile(info, io.BytesIO(payload))
                    device = tarfile.TarInfo(name=f"{root}/device")
                    device.type = tarfile.BLKTYPE
                    device.devmajor = 1
                    device.devminor = 3
                    archive.addfile(device)
                else:
                    raise AssertionError(kind)
            return archive_path.read_bytes()

        materializing_tar = """#!/bin/sh
real=/usr/bin/tar
extract=0
for arg in "$@"; do
  case "$arg" in
    -*x*) extract=1 ;;
  esac
done
if [ "$extract" -eq 0 ]; then
  exec "$real" "$@"
fi
"$real" "$@" || exit $?
dest=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-C" ]; then
    dest=$arg
  fi
  prev=$arg
done
if [ -n "$dest" ]; then
  find "$dest" \\( -type l -o -type p -o -type b -o -type c \\) -print | while IFS= read -r entry; do
    rm -f "$entry"
    cp "$AW_OUTSIDE" "$entry" 2>/dev/null || printf '%s\\n' 'materialized' > "$entry"
  done
fi
exit 0
"""
        for kind in ("symlink", "fifo", "device", "block"):
            with self.subTest(kind=kind):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    outside = work / "outside"
                    outside.write_bytes(outside_bytes)
                    blob = special(kind, work)
                    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                    _write_exe(work / "bin", "tar", materializing_tar)
                    env_path = f"{work / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}"
                    with _fixed_body_server(blob, status=200) as server:
                        port = server.server_address[1]
                        env = os.environ.copy()
                        env["PATH"] = env_path
                        env["AW_OUTSIDE"] = str(outside)
                        env["GITHUB_OUTPUT"] = str(work / "output")
                        env["GITHUB_ENV"] = str(work / "env")
                        env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                        env["SCCACHE_FETCH_ATTEMPTS"] = "1"
                        env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                        result = subprocess.run(
                            [
                                "bash",
                                str(script),
                                "--release-base",
                                f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                            ],
                            env=env,
                            text=True,
                            capture_output=True,
                            check=False,
                        )
                        output = (work / "output").read_text(encoding="utf-8")
                    _assert_prefetch_fell_back(
                        self,
                        result,
                        output,
                        work,
                        arch,
                        message="archive has a symlink or non-regular entry",
                    )
                    self.assertNotIn("outside-the-archive", output)
                    self.assertNotIn("outside-the-archive", result.stdout)

    def _release_root(self) -> tuple[str, str, str]:
        filename, arch = _host_sccache_filename()
        return filename, arch, filename[: -len(".tar.gz")]

    def _materializing_prefix(self, work: Path, outside: Path) -> Path:
        outside.write_bytes(b"outside-the-archive\n")
        _write_exe(work / "bin", "tar", _MATERIALIZING_TAR)
        return work / "bin"

    def test_sccache_prefetch_refuses_a_nonzero_directory_size(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        hidden = _ustar_header(f"{root}/hidden".encode(), 50, 0, b"/etc/hostname")
        member = _ustar_header(f"{root}/sccache".encode(), 48, len(payload))
        padded = _pad_block(payload)
        archives = {
            "typeflag-5": _ustar_header(f"{root}/".encode(), 53, 512),
            "trailing-slash": _ustar_header(f"{root}/".encode(), 0, 512),
        }
        for name, directory in archives.items():
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    outside = work / "outside-file"
                    prefix = self._materializing_prefix(work, outside)
                    blob = _gzip_tar([directory, hidden, member, padded])
                    result, output = _fetch_blob(
                        work,
                        blob,
                        path_prefix=prefix,
                        extra_env={"AW_OUTSIDE": str(outside)},
                    )
                    _assert_prefetch_fell_back(
                        self,
                        result,
                        output,
                        work,
                        arch,
                        message="archive has a symlink or non-regular entry",
                    )
                    self.assertNotIn("outside-the-archive", output)
        for name, typ in (
            ("typeflag-5-size", 53),
            ("trailing-slash-size", 0),
            ("zero-typeflag-size", 48),
        ):
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    directory = _ustar_header(f"{root}/".encode(), typ, 512)
                    blob = _gzip_tar([directory, b"\0" * 512, member, padded])
                    result, output = _fetch_blob(work, blob)
                    _assert_prefetch_fell_back(
                        self,
                        result,
                        output,
                        work,
                        arch,
                        message="archive has a symlink or non-regular entry",
                    )

    def test_sccache_prefetch_refuses_a_signed_checksum_link(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        directory = _ustar_header(f"{root}/".encode(), 53, 0)
        hidden = _ustar_header(
            f"{root}/".encode() + b"\x80hidden",
            50,
            0,
            b"/etc/hostname",
            signed=True,
        )
        member = _ustar_header(f"{root}/sccache".encode(), 48, len(payload))
        blob = _gzip_tar([directory, hidden, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            outside = work / "outside-file"
            prefix = self._materializing_prefix(work, outside)
            result, output = _fetch_blob(
                work,
                blob,
                path_prefix=prefix,
                extra_env={"AW_OUTSIDE": str(outside)},
            )
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="could not extract the archive",
        )
        self.assertNotIn("symlink or non-regular", result.stderr)
        self.assertNotIn("outside-the-archive", output)

    def test_sccache_prefetch_refuses_a_non_octal_size(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"Z"
        header = bytearray(_ustar_header(f"{root}/sccache".encode(), 48, len(payload)))
        header[124:136] = b"00000000008\0"
        header[148:156] = b"        "
        total = sum(32 if 148 <= index < 156 else byte for index, byte in enumerate(header))
        header[148:156] = f"{total:06o}".encode() + b"\0 "
        directory = _ustar_header(f"{root}/".encode(), 53, 0)
        blob = _gzip_tar([directory, bytes(header), _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="could not extract the archive",
        )
        self.assertNotIn("off_t", result.stderr)

    def test_sccache_prefetch_refuses_a_missing_end_block(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        directory = _ustar_header(f"{root}/".encode(), 53, 0)
        member = _ustar_header(f"{root}/sccache".encode(), 48, len(payload))
        blob = gzip.compress(directory + member + _pad_block(payload))
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="could not extract the archive",
        )
        self.assertNotIn("available=true", output)

    def test_sccache_prefetch_installs_a_nul_typeflag_file(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        directory = _ustar_header(f"{root}/".encode(), 53, 0)
        padded = _pad_block(payload)
        for name, typ in (("nul", 0), ("zero", 48)):
            with self.subTest(name=name):
                member = _ustar_header(f"{root}/sccache".encode(), typ, len(payload))
                blob = _gzip_tar([directory, member, padded])
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    result, output = _fetch_blob(work, blob)
                    marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(output, "available=true\n")
                    self.assertTrue(marker.is_file())

    def test_sccache_cleanup_does_not_chmod_a_link(self) -> None:
        _filename, arch = _host_sccache_filename()
        planting = """#!/bin/sh
real=/usr/bin/tar
"$real" "$@" || exit $?
dest=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-C" ]; then
    dest=$arg
  fi
  prev=$arg
done
if [ -n "$dest" ]; then
  ln -s "$AW_OUTSIDE" "$dest/planted"
fi
exit 0
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            outside = work / "outside-file"
            outside.write_bytes(b"keep\n")
            outside.chmod(0o400)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(work / "bin", "tar", planting)
            _bind_tool(script, "/usr/bin/tar", work / "bin" / "tar")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_OUTSIDE": str(outside)},
                )
            mode = outside.stat().st_mode & 0o777
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="archive has a symlink or non-regular entry",
        )
        self.assertEqual(mode, 0o400)

    def test_sccache_prefetch_refuses_a_link_added_at_extract(self) -> None:
        _filename, arch = _host_sccache_filename()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        planting_tar = """#!/bin/sh
real=/usr/bin/tar
"$real" "$@" || exit $?
dest=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-C" ]; then
    dest=$arg
  fi
  prev=$arg
done
if [ -n "$dest" ]; then
  find "$dest" -type f -name README.md -print | while IFS= read -r file; do
    rm -f "$file"
    ln -s /etc/hostname "$file"
  done
fi
exit 0
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _packed_archive(
                work,
                [
                    ("sccache", "file", payload),
                    ("README.md", "file", b"readme\n"),
                ],
            )
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(work / "bin", "tar", planting_tar)
            _bind_tool(script, "/usr/bin/tar", work / "bin" / "tar")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            _assert_prefetch_fell_back(
                self,
                result,
                output,
                work,
                arch,
                message="archive has a symlink or non-regular entry",
            )

    def test_sccache_prefetch_refuses_an_unreadable_member_directory(self) -> None:
        filename, arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")]
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"

        def limited(directory: Path, mode: int, with_child: bool) -> bytes:
            archive_path = directory / filename
            with tarfile.open(archive_path, "w:gz") as archive:
                info = tarfile.TarInfo(name=f"{root}/sccache")
                info.size = len(payload)
                info.mode = 0o755
                archive.addfile(info, io.BytesIO(payload))
                sub = tarfile.TarInfo(name=f"{root}/sub")
                sub.type = tarfile.DIRTYPE
                sub.mode = mode
                archive.addfile(sub)
                if with_child:
                    leaf = tarfile.TarInfo(name=f"{root}/sub/leaf")
                    leaf.size = 1
                    leaf.mode = 0o644
                    archive.addfile(leaf, io.BytesIO(b"z"))
            return archive_path.read_bytes()

        for name, mode, with_child in (
            ("unreadable", 0o000, False),
            ("unsearchable", 0o444, True),
        ):
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    script_tmp = work / "script-tmp"
                    script_tmp.mkdir()
                    blob = limited(work, mode, with_child)
                    script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                    with _fixed_body_server(blob, status=200) as server:
                        port = server.server_address[1]
                        env = os.environ.copy()
                        env["TMPDIR"] = str(script_tmp)
                        env["GITHUB_OUTPUT"] = str(work / "output")
                        env["GITHUB_ENV"] = str(work / "env")
                        env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                        env["SCCACHE_FETCH_ATTEMPTS"] = "1"
                        env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
                        result = subprocess.run(
                            [
                                "bash",
                                str(script),
                                "--release-base",
                                f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                            ],
                            env=env,
                            text=True,
                            capture_output=True,
                            check=False,
                        )
                        output = (work / "output").read_text(encoding="utf-8")
                    _assert_prefetch_fell_back(
                        self,
                        result,
                        output,
                        work,
                        arch,
                        message="archive has a symlink or non-regular entry",
                    )
                    self.assertEqual(list(script_tmp.iterdir()), [])

    def test_sccache_prefetch_rejects_an_extra_tool_cache_file(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _packed_archive(
                work,
                [
                    ("sccache", "file", b"#!/bin/sh\necho sccache 0.19.0\n"),
                    ("README.md", "file", b"readme\n"),
                ],
            )
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\n/bin/cp \"$1\" \"$2\"\nprintf 'extra\\n' > \"$(dirname \"$2\")/cargo\"\nexit 0\n",
            )
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            _assert_prefetch_fell_back(
                self,
                result,
                output,
                work,
                arch,
                message="tool cache is missing the sccache binary",
            )

    def test_sccache_prefetch_replaces_a_preseeded_tool_cache(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            dest = work / "tool-cache" / "sccache" / "0.19.0" / arch
            dest.mkdir(parents=True)
            (dest / "cargo").write_text("stale\n", encoding="utf-8")
            (dest.parent / f"{arch}.complete").write_text("stale\n", encoding="utf-8")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                names = sorted(path.name for path in dest.iterdir())
                marker = (dest.parent / f"{arch}.complete").read_text(encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertEqual(names, ["sccache"])
        self.assertEqual(marker, "")

    def test_sccache_prefetch_uses_sccache_on_darwin(self) -> None:
        real_node = shutil.which("node")
        self.assertIsNotNone(real_node)
        darwin = "sccache-v0.19.0-x86_64-apple-darwin.tar.gz"
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        node = f"""#!/bin/sh
if [ "$1" = "-p" ] && [ "$2" = "process.platform" ]; then
  printf '%s\\n' darwin
  exit 0
fi
if [ "$1" = "-p" ] && [ "$2" = "process.arch" ]; then
  printf '%s\\n' x64
  exit 0
fi
exec {real_node} "$@"
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            _write_exe(work / "bin", "node", node)
            blob = _packed_archive(
                work,
                [("sccache", "file", payload)],
                filename=darwin,
            )
            script = _script_pinned_named(work, darwin, hashlib.sha256(blob).hexdigest())
            _bind_tool(script, "/usr/local/bin/node", work / "bin" / "node")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                cached = work / "tool-cache" / "sccache" / "0.19.0" / "x64" / "sccache"
                exe = work / "tool-cache" / "sccache" / "0.19.0" / "x64" / "sccache.exe"
                cached_is_file = cached.is_file()
                exe_exists = exe.exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(cached_is_file)
        self.assertFalse(exe_exists)

    def test_sccache_prefetch_closed_pipe_leaves_no_marker(self) -> None:
        _filename, arch = _host_sccache_filename()
        script = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            patched = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                read_fd, write_fd = os.pipe()
                os.close(read_fd)
                try:
                    result = subprocess.run(
                        [
                            "bash",
                            str(patched),
                            "--release-base",
                            f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        ],
                        env={
                            "PATH": os.environ["PATH"],
                            "HOME": os.environ.get("HOME", "/tmp"),
                            "GITHUB_OUTPUT": str(work / "output"),
                            "RUNNER_TOOL_CACHE": str(work / "tool-cache"),
                            "SCCACHE_FETCH_ATTEMPTS": "1",
                            "SCCACHE_FETCH_DELAY_SECONDS": "0",
                        },
                        stdout=write_fd,
                        stderr=subprocess.PIPE,
                        text=True,
                        check=False,
                    )
                finally:
                    os.close(write_fd)
                output = (
                    (work / "output").read_text(encoding="utf-8")
                    if (work / "output").is_file()
                    else ""
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                dest = work / "tool-cache" / "sccache" / "0.19.0" / arch
                marker_exists = marker.exists()
                dest_exists = dest.exists()
        self.assertEqual(result.returncode, 141, result.stderr)
        self.assertNotIn("available=true", output)
        self.assertFalse(marker_exists)
        self.assertFalse(dest_exists)

        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            read_fd, write_fd = os.pipe()
            os.close(read_fd)
            try:
                result = subprocess.run(
                    [
                        "bash",
                        str(script),
                        "--release-base",
                        "http://127.0.0.1:1/releases/download/v0.19.0",
                    ],
                    env={
                        "PATH": os.environ["PATH"],
                        "HOME": os.environ.get("HOME", "/tmp"),
                        "GITHUB_OUTPUT": str(work / "output"),
                        "RUNNER_TOOL_CACHE": str(work / "tool-cache"),
                        "SCCACHE_FETCH_ATTEMPTS": "1",
                        "SCCACHE_FETCH_DELAY_SECONDS": "0",
                    },
                    stdout=subprocess.PIPE,
                    stderr=write_fd,
                    text=True,
                    check=False,
                )
            finally:
                os.close(write_fd)
            output = (
                (work / "output").read_text(encoding="utf-8")
                if (work / "output").is_file()
                else ""
            )
            marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
        self.assertEqual(result.returncode, 141, result.stdout)
        self.assertNotIn("available=true", output)
        self.assertFalse(marker.exists())

    def test_sccache_prefetch_keeps_success_when_temp_cleanup_fails(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_rm_that_refuses_tree_wipe(work / "bin")
            _bind_tool(script, "/usr/bin/rm", work / "bin" / "rm")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                marker_exists = marker.is_file()
                cached_exists = cached.is_file()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(marker_exists)
        self.assertTrue(cached_exists)

    def test_sccache_prefetch_marker_failure_clears_the_install(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\n"
                '/bin/cp "$1" "$2"\n'
                'mkdir "$(dirname "$2").complete"\n'
                "exit 0\n",
            )
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
                dest = work / "tool-cache" / "sccache" / "0.19.0" / arch
                marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
                dest_exists = dest.exists()
                marker_exists = marker.exists()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertNotIn("available=true", output)
        self.assertFalse(dest_exists)
        self.assertFalse(marker_exists)

    def test_sccache_prefetch_rejects_a_hardlinked_binary_after_extract(self) -> None:
        _filename, arch = _host_sccache_filename()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        planting_tar = """#!/bin/sh
real=/usr/bin/tar
"$real" "$@" || exit $?
dest=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-C" ]; then
    dest=$arg
  fi
  prev=$arg
done
if [ -n "$dest" ]; then
  find "$dest" -type f -name sccache -print | while IFS= read -r file; do
    ln "$file" "$(dirname "$file")/sccache-alias"
  done
fi
exit 0
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _packed_archive(work, [("sccache", "file", payload)])
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(work / "bin", "tar", planting_tar)
            _bind_tool(script, "/usr/bin/tar", work / "bin" / "tar")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            _assert_prefetch_fell_back(
                self,
                result,
                output,
                work,
                arch,
                message="archive has no sccache binary",
            )

    def test_sccache_prefetch_refuses_an_unclassified_header(self) -> None:
        _filename, arch, root = self._release_root()
        root_b = root.encode()
        link = _ustar_header(root_b + b"/sccache", 50, 0, b"/etc/hostname")
        cases = {
            "extractor": _gzip_tar(
                [_ustar_header(b"", 48, 512, prefix=root_b + b"/"), link]
            ),
            "joined": _gzip_tar(
                [
                    _ustar_header(b"sccache", 48, 30, prefix=root_b),
                    _pad_block(b"#!/bin/sh\necho sccache 0.19.0\n"),
                ]
            ),
            "blank": _gzip_tar([_ustar_header(b"", 48, 512), link]),
        }
        for name, blob in cases.items():
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    outside = work / "outside-file"
                    prefix = self._materializing_prefix(work, outside)
                    outside.chmod(0o400)
                    result, output = _fetch_blob(
                        work,
                        blob,
                        path_prefix=prefix,
                        extra_env={"AW_OUTSIDE": str(outside)},
                    )
                    _assert_prefetch_fell_back(
                        self,
                        result,
                        output,
                        work,
                        arch,
                        message="archive has a symlink or non-regular entry",
                    )
                    self.assertEqual(outside.stat().st_mode & 0o777, 0o400)
                    self.assertNotIn("outside-the-archive", output)

    def test_sccache_prefetch_refuses_a_nul_led_size(self) -> None:
        _filename, arch, root = self._release_root()
        root_b = root.encode()
        first = _ustar_header(
            root_b + b"/decoy",
            48,
            0,
            size_field=b"\0" + b"00000001000",
        )
        second = _ustar_header(root_b + b"/data", 48, 512)
        link = _ustar_header(root_b + b"/sccache", 50, 0, b"/etc/hostname")
        blob = _gzip_tar([first, second, link])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )
        self.assertNotIn("symlink or non-regular", result.stderr)

    def test_sccache_prefetch_refuses_a_hundred_byte_name(self) -> None:
        _filename, arch, root = self._release_root()
        name = b"a" * 99 + b"/"
        self.assertEqual(len(name), 100)
        first = _ustar_header(name, 48, 512)
        link = _ustar_header(root.encode() + b"/sccache", 50, 0, b"/etc/hostname")
        blob = _gzip_tar([first, link])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 1)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="archive has a symlink or non-regular entry",
        )

    def test_sccache_prefetch_reader_rejects_a_base256_size(self) -> None:
        _filename, arch, root = self._release_root()
        raw = (512).to_bytes(12, "big")
        field = bytes([raw[0] | 0x80]) + raw[1:]
        first = _ustar_header(root.encode() + b"/sccache", 48, 0, size_field=field)
        link = _ustar_header(root.encode() + b"/hidden", 50, 0, b"/etc/hostname")
        blob = _gzip_tar([first, link])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )
        self.assertNotIn("symlink or non-regular", result.stderr)

    def test_sccache_prefetch_installs_a_space_padded_size(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        field = f"{len(payload):o}".encode().rjust(11, b" ") + b"\0"
        directory = _ustar_header(root.encode() + b"/", 53, 0)
        member = _ustar_header(
            root.encode() + b"/sccache", 48, len(payload), size_field=field
        )
        blob = _gzip_tar([directory, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output = _fetch_blob(work, blob)
            marker = work / "tool-cache" / "sccache" / "0.19.0" / f"{arch}.complete"
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=true\n")
            self.assertTrue(marker.is_file())
            self.assertEqual(cached.read_bytes(), payload)

    def test_sccache_prefetch_reader_accepts_a_blank_size(self) -> None:
        _filename, _arch, root = self._release_root()
        member = _ustar_header(
            root.encode() + b"/sccache", 48, 0, size_field=b" " * 12
        )
        blob = _gzip_tar([member])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 0)

    def test_sccache_prefetch_reader_rejects_a_trailing_size_byte(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        field = f"{len(payload):o}".encode().rjust(10, b"0") + b"\0X"
        self.assertEqual(len(field), 12)
        directory = _ustar_header(root.encode() + b"/", 53, 0)
        member = _ustar_header(
            root.encode() + b"/sccache", 48, len(payload), size_field=field
        )
        blob = _gzip_tar([directory, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )

    def test_sccache_prefetch_restores_modes_before_cleanup(self) -> None:
        filename, _arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")]
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            archive_path = work / filename
            with tarfile.open(archive_path, "w:gz") as archive:
                info = tarfile.TarInfo(name=f"{root}/sccache")
                info.size = len(payload)
                info.mode = 0
                archive.addfile(info, io.BytesIO(payload))
            blob = archive_path.read_bytes()
            log = work / "mode-log"
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "rm",
                "#!/bin/sh\n"
                'if [ "$1" = "-rf" ] && [ -d "$2" ]; then\n'
                '  find "$2" -type f -perm 000 -print >> "$AW_MODE_LOG"\n'
                "fi\n"
                "exec /bin/rm \"$@\"\n",
            )
            _bind_tool(script, "/usr/bin/rm", work / "bin" / "rm")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_MODE_LOG": str(log)},
                )
            logged = log.read_text(encoding="utf-8") if log.exists() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("available=true", output)
        self.assertEqual(logged, "")

    def test_sccache_fetch_default_count_rejects_a_changed_repo(self) -> None:
        global _sccache_policy_texts
        saved = _sccache_policy_texts
        texts = saved()
        label, body = next(item for item in texts if item[0].endswith("benchmark_alert.yml"))
        nine = [
            (name, text if name != label else body.replace(SCCACHE_FETCH_COMMAND, "echo kept", 1))
            for name, text in texts
        ]
        extra_job = (
            "\n  aw_fetch_probe:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - shell: bash\n"
            "        run: |\n"
            f"          {SCCACHE_FETCH_COMMAND}\n"
        )
        eleven = [(name, text if name != label else body + extra_job) for name, text in texts]

        def nine_texts() -> list[tuple[str, str]]:
            return nine

        def eleven_texts() -> list[tuple[str, str]]:
            return eleven

        try:
            _sccache_policy_texts = nine_texts
            found = sccache_fetch_run_violations()
            self.assertTrue(any("found 9" in item for item in found), found)
            _sccache_policy_texts = eleven_texts
            found = sccache_fetch_run_violations()
            self.assertTrue(any("found 11" in item for item in found), found)
        finally:
            _sccache_policy_texts = saved

    def test_sccache_prefetch_closes_any_reader_failure(self) -> None:
        real_node = "/usr/local/bin/node"
        self.assertTrue(Path(real_node).exists())
        shim = (
            "#!/bin/sh\n"
            "for arg in \"$@\"; do\n"
            "  case \"$arg\" in\n"
            "    *zlib.gunzipSync*) exit 3 ;;\n"
            "  esac\ndone\n"
            f"exec {real_node} \"$@\"\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(work / "bin", "node", shim)
            _bind_tool(script, real_node, work / "bin" / "node")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            _filename, arch = _host_sccache_filename()
            _assert_prefetch_fell_back(
                self, result, output, work, arch, message="could not extract the archive"
            )

    def test_sccache_prefetch_installs_the_named_member(self) -> None:
        _filename, arch = _host_sccache_filename()
        decoy = b"DECOY"
        real = b"#!/bin/sh\necho sccache 0.19.0\n"
        self.assertNotEqual(decoy, real)
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _packed_archive(
                work,
                [("README", "file", decoy), ("sccache", "file", real)],
            )
            result, output = _fetch_blob(work, blob)
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            installed = cached.read_bytes() if cached.is_file() else b""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertEqual(installed, real)

    def test_sccache_prefetch_calls_cygpath_only_when_runner_os_is_windows(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            decoy_log = work / "decoy.log"
            called = work / "cygpath.log"
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cygpath-abs",
                "#!/bin/sh\n"
                f"printf '%s\\n' absolute >> {shlex.quote(str(called))}\n"
                "printf '%s\\n' \"$2\"\n",
            )
            _bind_tool(script, "/usr/bin/cygpath", work / "bin" / "cygpath-abs")
            _plant_decoy(work / "pathbin", "cygpath", decoy_log)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "pathbin",
                    extra_env={"RUNNER_OS": "Windows"},
                )
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            logged = decoy_log.read_text(encoding="utf-8") if decoy_log.is_file() else ""
            absolute = called.read_text(encoding="utf-8") if called.is_file() else ""
            cached_is_file = cached.is_file()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertTrue(cached_is_file)
        self.assertEqual(logged, "")
        self.assertEqual(absolute, "absolute\n")

        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            decoy_log = work / "decoy.log"
            script = _script_copy(work)
            _plant_decoy(work / "pathbin", "cygpath", decoy_log)
            result, output = _run_fetch(
                work,
                script=script,
                path_prefix=work / "pathbin",
                extra_env={"RUNNER_OS": "Windows"},
            )
            logged = decoy_log.read_text(encoding="utf-8") if decoy_log.is_file() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("cygpath is missing", result.stderr)
        self.assertEqual(logged, "")

    def test_sccache_prefetch_never_runs_a_path_tool(self) -> None:
        tools = ("curl", "mktemp", "rm", "chmod", "mkdir", "cygpath", "tar", "cp", "node")
        _filename, arch = _host_sccache_filename()
        for name in tools:
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    log = work / "decoy.log"
                    blob = _valid_sccache_archive(work)
                    _plant_decoy(work / "bin", name, log)
                    result, output = _fetch_blob(work, blob, path_prefix=work / "bin")
                    cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
                    logged = log.read_text(encoding="utf-8") if log.is_file() else ""
                    cached_is_file = cached.is_file()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, "available=true\n")
                self.assertTrue(cached_is_file)
                self.assertEqual(logged, "")

    def test_sccache_prefetch_never_runs_a_path_sleep(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            log = work / "decoy.log"
            _plant_decoy(work / "bin", "sleep", log)
            result, output = _run_fetch(
                work,
                base="http://127.0.0.1:1/releases/download/v0.19.0",
                attempts="2",
                delay="0",
                path_prefix=work / "bin",
            )
            logged = log.read_text(encoding="utf-8") if log.is_file() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("retrying in 0s", result.stderr)
        self.assertEqual(logged, "")

    def test_sccache_fetch_never_runs_a_path_call_site_tool(self) -> None:
        fetch = ROOT / "scripts" / "ci" / "fetch_sccache_release.sh"
        for name in ("uname", "env", "bash"):
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    log = work / "decoy.log"
                    _plant_decoy(work / "bin", name, log)
                    env = os.environ.copy()
                    env["PATH"] = f"{work / 'bin'}{os.pathsep}{env.get('PATH', '')}"
                    env["GITHUB_OUTPUT"] = str(work / "output")
                    env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
                    command = SCCACHE_FETCH_COMMAND.replace(
                        "/usr/bin/env -i ",
                        "/usr/bin/env -i SCCACHE_FETCH_ATTEMPTS=1 SCCACHE_FETCH_DELAY_SECONDS=0 ",
                    ).replace(
                        "scripts/ci/fetch_sccache_release.sh",
                        shlex.quote(str(fetch))
                        + " --release-base "
                        + shlex.quote("http://127.0.0.1:1/releases/download/v0.19.0"),
                    )
                    result = subprocess.run(
                        ["/usr/bin/bash", "--noprofile", "--norc", "-c", command],
                        env=env,
                        text=True,
                        capture_output=True,
                        check=False,
                    )
                    output = (
                        (work / "output").read_text(encoding="utf-8")
                        if (work / "output").is_file()
                        else ""
                    )
                    logged = log.read_text(encoding="utf-8") if log.is_file() else ""
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, "available=false\n")
                self.assertIn("release download failed", result.stderr)
                self.assertEqual(logged, "")

    def test_sccache_prefetch_falls_back_when_a_resolved_tool_is_missing(self) -> None:
        for name in ("curl", "mktemp", "rm", "chmod", "mkdir", "sleep", "tar", "cp"):
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    work = Path(tmp)
                    script = _script_copy(work)
                    _force_missing_linux_tool(script, name)
                    result, output = _run_fetch(work, script=script)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, "available=false\n")
                self.assertIn(f"{name} is missing", result.stderr)
                self.assertNotIn("checksum mismatch", result.stderr)

    def test_sccache_prefetch_ignores_a_path_tar(self) -> None:
        _filename, arch = _host_sccache_filename()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        swap = (
            "#!/bin/sh\n"
            "real=/usr/bin/tar\n"
            '"$real" "$@" || exit $?\n'
            "dest=\n"
            "prev=\n"
            "for arg in \"$@\"; do\n"
            "  if [ \"$prev\" = \"-C\" ]; then dest=$arg; fi\n"
            "  prev=$arg\n"
            "done\n"
            "if [ -n \"$dest\" ]; then\n"
            "  find \"$dest\" -type f -name sccache -exec sh -c "
            "'printf \"%s\\n\" \"#!/bin/sh\" \"echo SWAPPED\" > \"$1\"' sh {} \\;\n"
            "fi\n"
            "exit 0\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            _write_exe(work / "bin", "tar", swap)
            result, output = _fetch_blob(work, blob, path_prefix=work / "bin")
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            ran = subprocess.check_output([str(cached)], text=True).strip()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertEqual(ran, "sccache 0.19.0")

    def test_sccache_fetch_rejects_unclassified_steps(self) -> None:
        text = textwrap.dedent(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - just-a-string
            """
        ).lstrip()
        found = sccache_fetch_run_violations([("probe.yml", text)])
        self.assertTrue(any("could not classify" in item for item in found), found)

    def test_sccache_fetch_rejects_yaml_spellings(self) -> None:
        forms = {
            "list": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash scripts/ci/fetch_sccache_release.sh
            """,
            "flow": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - {name: p, run: bash scripts/ci/fetch_sccache_release.sh}
            """,
            "quoted": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - name: p
                        "run": bash scripts/ci/fetch_sccache_release.sh
            """,
            "indicator": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: |2
                          bash scripts/ci/fetch_sccache_release.sh
            """,
            "comment-header": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: | # note
                          bash scripts/ci/fetch_sccache_release.sh
            """,
            "plain": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash
                          scripts/ci/fetch_sccache_release.sh
            """,
            "working-directory": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - working-directory: scripts/ci
                        run: bash ./fetch_s*.sh
            """,
            "working-directory-literal": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - working-directory: scripts/ci
                        run: bash ./fetch_sccache_release.sh
            """,
            "job-env": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    env:
                      F: scripts/ci/fetch_sccache_release.sh
                    steps:
                      - run: bash "$F"
            """,
            "step-env": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - env:
                          SCRIPT: scripts/ci/fetch_sccache_release.sh
                        run: bash $SCRIPT
            """,
            "expression": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash ${{ env.SCRIPT }}
            """,
            "input-expression": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash ${{ inputs.script }}
            """,
            "shell": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash scripts/ci/fetch_sccache_release.sh {0}
                        run: echo hello
            """,
            "cd": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: |
                          cd scripts
                          cd ci
                          bash ./fetch_s*.sh
            """,
            "assignments": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: s=scripts; c=ci; bash "$s/$c/tool.sh"
            """,
            "split-stem": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: s=scripts; c=ci; p1=fetch_scc; p2=ache_release.sh; bash "$s/$c/$p1$p2"
            """,
            "ansi": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash $'scripts/ci/tool.sh'
            """,
            "ansi-hex": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash $'\\x73cripts/ci/tool.sh'
            """,
            "backslash": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash scripts\\ci\\fetch_sccache_release.sh
            """,
            "command-substitution": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash $(echo c2NyaXB0cy9jaS90b29sLnNo | base64 -d)
            """,
            "glob": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - working-directory: scripts/ci
                        run: bash *
            """,
            "glob-sh": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - working-directory: scripts/ci
                        run: bash *.sh
            """,
            "folded": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: >-
                          bash scripts/ci/tool.sh
            """,
            "folded-strip": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: |-
                          bash scripts/ci/tool.sh
            """,
            "folded-clip": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: >
                          bash scripts/ci/tool.sh
            """,
            "and-or": """
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: echo hello && bash scripts/ci/tool.sh
            """,
            "workflow-env": """
                name: probe
                on: push
                env:
                  SCRIPT: scripts/ci/fetch_sccache_release.sh
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - run: bash $SCRIPT
            """,
        }
        for label, body in forms.items():
            with self.subTest(label=label):
                text = textwrap.dedent(body).lstrip()
                found = sccache_fetch_run_violations([(f"{label}.yml", text)])
                self.assertTrue(found, (label, found))
                self.assertFalse(
                    any("could not classify" in item for item in found),
                    (label, found),
                )
        defaults_wd = textwrap.dedent(
            """
            name: probe
            on: push
            defaults:
              run:
                working-directory: scripts/ci
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run: bash ./tool.sh
            """
        ).lstrip()
        wd_found = sccache_fetch_run_violations([("defaults-wd.yml", defaults_wd)])
        self.assertTrue(any("could not classify" in item for item in wd_found), wd_found)

    def test_sccache_fetch_pins_the_step_shell_and_env(self) -> None:
        pin = SCCACHE_FETCH_COMMAND
        env_step = textwrap.dedent(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    env:
                      BASH_ENV: /tmp/x
                    run: {pin}
            """
        ).lstrip()
        shell_step = textwrap.dedent(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: pwsh
                    run: {pin}
            """
        ).lstrip()
        env_found = sccache_fetch_run_violations([("env.yml", env_step)])
        shell_found = sccache_fetch_run_violations([("shell.yml", shell_step)])
        self.assertTrue(any("fetch step sets env" in item for item in env_found), env_found)
        self.assertTrue(
            any("fetch step shell is not bash" in item for item in shell_found),
            shell_found,
        )
        default_shell = textwrap.dedent(
            f"""
            name: probe
            on: push
            defaults:
              run:
                shell: pwsh
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run: {pin}
            """
        ).lstrip()
        default_found = sccache_fetch_run_violations([("default.yml", default_shell)])
        self.assertTrue(
            any("fetch step shell is not bash" in item for item in default_found),
            default_found,
        )

    def test_sccache_fetch_refuses_unclassified_job_defaults(self) -> None:
        pin = SCCACHE_FETCH_COMMAND
        probes = {
            "job-pwsh": f"""
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    defaults:
                      run:
                        shell: pwsh
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "job-workdir": f"""
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    defaults:
                      run:
                        shell: bash
                        working-directory: scripts
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "matrix-shell": f"""
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    defaults:
                      run:
                        shell: ${{{{ matrix.shell }}}}
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "defaults-true": f"""
                name: probe
                on: push
                defaults: true
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "run-bash": f"""
                name: probe
                on: push
                defaults:
                  run: bash
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "root-shell": f"""
                name: probe
                on: push
                defaults:
                  shell: pwsh
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash
                        run: {pin}
            """,
            "step-workdir": f"""
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash
                        working-directory: /tmp/planted
                        run: {pin}
            """,
        }
        for label, text in probes.items():
            found = sccache_fetch_run_violations(
                [(f"{label}.yml", textwrap.dedent(text).lstrip())]
            )
            self.assertTrue(found, (label, found))

    def test_sccache_fetch_refuses_a_null_run(self) -> None:
        text = textwrap.dedent(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run:
            """
        ).lstrip()
        found = sccache_fetch_run_violations([("null.yml", text)])
        self.assertTrue(any("could not classify" in item for item in found), found)

    def test_sccache_fetch_scans_reusable_workflow_inputs(self) -> None:
        named = textwrap.dedent(
            """
            name: probe
            on: push
            jobs:
              called:
                uses: ./.github/workflows/called.yml
                with:
                  script: scripts/ci/fetch_sccache_release.sh
            """
        ).lstrip()
        opaque = textwrap.dedent(
            """
            name: probe
            on: push
            jobs:
              called:
                uses: ./.github/workflows/called.yml
                with: true
            """
        ).lstrip()
        nested = textwrap.dedent(
            """
            name: probe
            on: push
            jobs:
              called:
                uses: ./.github/workflows/called.yml
                with:
                  script:
                    path: scripts/ci/fetch_sccache_release.sh
            """
        ).lstrip()
        named_found = sccache_fetch_run_violations([("named.yml", named)])
        opaque_found = sccache_fetch_run_violations([("opaque.yml", opaque)])
        nested_found = sccache_fetch_run_violations([("nested.yml", nested)])
        self.assertTrue(any("allowlisted" in item for item in named_found), named_found)
        self.assertTrue(any("could not classify" in item for item in opaque_found), opaque_found)
        self.assertTrue(any("could not classify" in item for item in nested_found), nested_found)

    def test_sccache_fetch_refuses_bash_env_on_a_fetch_job(self) -> None:
        pin = SCCACHE_FETCH_COMMAND

        def probe(body: str) -> list[str]:
            return sccache_fetch_run_violations(
                [("probe.yml", textwrap.dedent(body).lstrip())]
            )

        workflow_env = probe(
            f"""
            name: probe
            on: push
            env:
              BASH_ENV: /tmp/hook.sh
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        job_env = probe(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                env:
                  ENV: /tmp/hook.sh
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        later_step = probe(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: {pin}
                  - shell: bash
                    env:
                      BASH_ENV: /tmp/hook.sh
                    run: echo hello
            """
        )
        sibling = probe(
            f"""
            name: probe
            on: push
            jobs:
              other:
                runs-on: ubuntu-latest
                env:
                  BASH_ENV: /tmp/hook.sh
                steps:
                  - run: echo hello
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        unrelated = probe(
            """
            name: probe
            on: push
            env:
              BASH_ENV: /tmp/hook.sh
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run: echo hello
            """
        )
        self.assertTrue(any("sets BASH_ENV" in item for item in workflow_env), workflow_env)
        self.assertTrue(any("sets ENV" in item for item in job_env), job_env)
        self.assertTrue(any("sets BASH_ENV" in item for item in later_step), later_step)
        self.assertEqual(sibling, [])
        self.assertEqual(unrelated, [])

    def test_cargo_incremental_step_has_no_if(self) -> None:
        paths = [ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"]
        paths.extend(sorted(WORKFLOWS.glob("*.yml")))
        checked = 0
        for path in paths:
            text = path.read_text(encoding="utf-8")
            if "- name: Disable incremental compilation\n" not in text:
                continue
            checked += 1
            _assert_incremental_steps_have_no_if(text)
        self.assertGreaterEqual(checked, 5)
        action_path = ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"
        action = action_path.read_text(encoding="utf-8")
        mutant = action.replace(
            "    - name: Disable incremental compilation\n      shell: bash\n",
            "    - name: Disable incremental compilation\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      shell: bash\n",
            1,
        )
        self.assertNotEqual(mutant, action)
        with self.assertRaises(AssertionError):
            _assert_incremental_steps_have_no_if(mutant)

    def test_enable_sccache_reads_ref_from_the_environment(self) -> None:
        action = (ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml").read_text(
            encoding="utf-8"
        )
        step = action.split("- name: Enable sccache\n", 1)[1].split("\n    - name:", 1)[0]
        run = step.split("run:", 1)[1]
        self.assertIn("AW_GITHUB_REF: ${{ github.ref }}", step)
        self.assertIn("AW_GITHUB_EVENT_NAME: ${{ github.event_name }}", step)
        self.assertNotIn("${{ github.ref }}", run)
        self.assertNotIn("${{ github.event_name }}", run)
        self.assertIn('[ "$AW_GITHUB_REF" = "refs/heads/main" ]', run)
        self.assertIn('[ "$AW_GITHUB_EVENT_NAME" != "pull_request" ]', run)

    def test_configure_sccache_quotes_the_env_file(self) -> None:
        quoted_wrapper = 'echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"'
        quoted_enabled = 'echo "SCCACHE_GHA_ENABLED=true" >> "$GITHUB_ENV"'
        bare_wrapper = 'echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV'
        bare_enabled = 'echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV'
        tool = (WORKFLOWS / "toolchain-management.yml").read_text(encoding="utf-8")
        dep = (WORKFLOWS / "dependency-management.yml").read_text(encoding="utf-8")
        self.assertEqual(tool.count(quoted_wrapper), 2)
        self.assertEqual(tool.count(quoted_enabled), 2)
        self.assertNotIn(bare_wrapper, tool)
        self.assertNotIn(bare_enabled, tool)
        self.assertEqual(dep.count(quoted_wrapper), 1)
        self.assertNotIn(bare_wrapper, dep)

    def test_sccache_rw_mode_is_explicit_on_a_non_main_ref(self) -> None:
        bodies = _sccache_enable_run_bodies()
        self.assertGreaterEqual(len(bodies), 8, [name for name, _body in bodies])
        for name, body in bodies:
            for ref, event, mode in (
                ("refs/heads/develop", "push", "READ_ONLY"),
                ("refs/heads/main", "push", "READ_WRITE"),
                ("refs/heads/main", "schedule", "READ_WRITE"),
                ("refs/heads/main", "workflow_dispatch", "READ_WRITE"),
                ("refs/heads/main", "pull_request", "READ_ONLY"),
            ):
                written = _run_bash_env(body, ref, event)
                self.assertIn(
                    f"SCCACHE_GHA_RW_MODE={mode}",
                    written,
                    f"{name} ref={ref} event={event}\n{written}",
                )
                self.assertNotIn(
                    "SCCACHE_GHA_RW_MODE=READ_WRITE"
                    if mode == "READ_ONLY"
                    else "SCCACHE_GHA_RW_MODE=READ_ONLY",
                    written,
                    name,
                )


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

import hashlib
import io
import http.server
import inspect
import json
import os
import re
import yaml
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import threading
import unittest
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


# v0.0.11 peeled commit from `git ls-remote --tags ... 'v0.0.11^{}'`.
# fd02668681acd5f960e1372061bee5e3e987195c is the annotated tag object.
SCCACHE_ACTION_COMMIT = "fc920bf0ec8de6ee65d409111f7ec508035751ba"
SCCACHE_ACTION_TAG_OBJECT = "fd02668681acd5f960e1372061bee5e3e987195c"
SCCACHE_VERSION = "v0.19.0"
SCCACHE_UNAVAILABLE_WARNING = "::warning::sccache unavailable, building without cache"
SCCACHE_PREFETCH_RUN = "run: bash scripts/ci/fetch_sccache_release.sh"
# Prefetch failure exits 0 with available=false. Anything after this check on
# the install `if` (`|| true`, `|| success()`) runs sccache-action, which then
# calls downloadSCCache on the same release (#721).
SCCACHE_PREFETCH_IF = "steps.sccache_fetch.outputs.available == 'true'"
SCCACHE_FALLBACK_IF = "steps.sccache_fetch.outputs.available != 'true'"
# Live `sccache --show-stats` conditions. Equality, not a substring.
SCCACHE_STATS_IFS = frozenset(
    {
        "steps.sccache_fetch.outputs.available == 'true'",
        "always() && steps.sccache_fetch.outputs.available == 'true'",
        "always() && steps.rust_ci.outputs.sccache-available == 'true'",
        "always() && needs.changes.outputs.docs_only != 'true' && steps.rust_ci.outputs.sccache-available == 'true'",
    }
)
_WRAPPER_ENV_KEYS = frozenset(
    {
        "RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    }
)
# Dotted `cargo --config` keys, plus the bare keys used in a Cargo config file.
_WRAPPER_CONFIG_KEYS = frozenset(
    {
        "build.rustc-wrapper",
        "build.rustc-workspace-wrapper",
    }
)
_WRAPPER_BARE_KEYS = frozenset(
    {
        "rustc-wrapper",
        "rustc-workspace-wrapper",
    }
)
_WRAPPER_KEY_RE = re.compile(
    r"(?i)(?<![A-Za-z0-9_.-])(?:"
    + "|".join(
        re.escape(key)
        for key in sorted(
            set(_WRAPPER_ENV_KEYS) | set(_WRAPPER_CONFIG_KEYS) | set(_WRAPPER_BARE_KEYS),
            key=len,
            reverse=True,
        )
    )
    + r")(?![A-Za-z0-9_.-])"
)
# The only run lines that may name a wrapper key. Anything else fails closed.
_SANCTIONED_GATE_LINES = frozenset(
    {
        'echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"',
        'echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV',
    }
)
_SANCTIONED_FALLBACK_LINE = 'echo "RUSTC_WRAPPER=" >> "$GITHUB_ENV"'
_WORKFLOW_GLOBS = ("*.yml", "*.yaml")
_ACTION_FILE_GLOBS = ("action.yml", "action.yaml")
CARGO_INCREMENTAL_ECHO = 'echo "CARGO_INCREMENTAL=0" >> "$GITHUB_ENV"'
# sha256 of the v0.19.0 archives fetch_sccache_release.sh can install.
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
EXPECTED_SCCACHE_ACTION_USES = {
    ".github/actions/rust-ci-setup/action.yml": 1,
    ".github/workflows/benchmark_alert.yml": 1,
    ".github/workflows/dependency-management.yml": 1,
    ".github/workflows/integration-tests.yml": 5,
    ".github/workflows/toolchain-management.yml": 2,
}
_ENV_ASSIGN_RE = re.compile(
    r"""echo\s+["']([A-Za-z_][A-Za-z0-9_]*=[^"']*)["']\s*>>\s*["']?\$GITHUB_ENV["']?"""
)


def _is_cargo_config(path: str) -> bool:
    parts = [part for part in path.replace("\\", "/").split("/") if part]
    return len(parts) >= 2 and parts[-2] == ".cargo" and parts[-1] in ("config", "config.toml")


def _is_cargo_target_dir(path: Path) -> bool:
    """A Cargo build directory, not a source directory whose name is target."""
    return path.name == "target" and (path / "CACHEDIR.TAG").is_file()


def _iter_cargo_configs() -> list[Path]:
    found: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [
            name
            for name in dirnames
            if name != ".git" and not _is_cargo_target_dir(Path(dirpath) / name)
        ]
        if Path(dirpath).name != ".cargo":
            continue
        for name in ("config.toml", "config"):
            if name in filenames:
                found.append(Path(dirpath) / name)
    return sorted(found)


def sccache_policy_texts() -> dict[str, str]:
    texts: dict[str, str] = {}
    for pattern in _WORKFLOW_GLOBS:
        for path in sorted(WORKFLOWS.glob(pattern)):
            texts[path.relative_to(ROOT).as_posix()] = path.read_text(encoding="utf-8")
    actions = ROOT / ".github" / "actions"
    for pattern in _ACTION_FILE_GLOBS:
        for path in sorted(actions.rglob(pattern)):
            texts[path.relative_to(ROOT).as_posix()] = path.read_text(encoding="utf-8")
    for path in _iter_cargo_configs():
        texts[path.relative_to(ROOT).as_posix()] = path.read_text(encoding="utf-8")
    return texts


def workflow_job_chunks(text: str) -> list[str]:
    match = re.search(r"(?m)^jobs:\s*$", text)
    if match is None:
        raise AssertionError("workflow is missing jobs:")
    pieces = re.split(r"(?m)^(  [A-Za-z0-9_-]+:\s*)$", text[match.end() :])
    chunks: list[str] = []
    index = 1
    while index + 1 < len(pieces):
        chunks.append(pieces[index] + pieces[index + 1])
        index += 2
    if not chunks:
        raise AssertionError("jobs: block has no jobs")
    return chunks


def step_blocks(text: str) -> list[str]:
    matches = list(re.finditer(r"(?m)^(?P<indent> +)- name:", text))
    blocks: list[str] = []
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        lines = text[match.start() : end].splitlines()
        header_indent = len(match.group("indent"))
        kept = [lines[0]]
        for line in lines[1:]:
            if not line.strip():
                kept.append(line)
                continue
            indent = len(line) - len(line.lstrip(" "))
            if indent <= header_indent:
                break
            kept.append(line)
        blocks.append("\n".join(kept))
    return blocks


def _has_continue_on_error(step: str) -> bool:
    return re.search(r"(?m)^\s*continue-on-error\s*:", step) is not None


def _version_value(step: str) -> str | None:
    match = re.search(r"(?m)^\s*version:\s*(.*?)\s*$", step)
    if match is None:
        return None
    value = match.group(1).strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        value = value[1:-1]
    return value


def _env_writes(step: str) -> list[str]:
    return _ENV_ASSIGN_RE.findall(step)


def _last_index_before(steps: list, end: int, needle: str) -> int:
    found = -1
    for index, step in enumerate(steps[:end]):
        haystack = step if isinstance(step, str) else yaml.safe_dump(step, sort_keys=False)
        if needle in haystack:
            found = index
    return found


def _last_prefetch_index_before(steps: list[dict], end: int) -> int:
    found = -1
    for index, step in enumerate(steps[:end]):
        if "fetch_sccache_release.sh" in _step_run(step):
            found = index
    return found


def _last_fallback_index_before(steps: list[dict], end: int) -> int:
    found = -1
    for index, step in enumerate(steps[:end]):
        if _step_looks_like_fallback(step):
            found = index
    return found


def _yaml_if_value(step: str) -> str | None:
    match = re.search(r"(?m)^\s*if:\s*(.*?)\s*$", step)
    if match is None:
        return None
    return match.group(1).strip()


def _step_if(step: dict) -> str | None:
    if "if" not in step:
        return None
    value = step["if"]
    if value is None:
        return None
    return str(value).strip()


def _step_run(step: dict | str) -> str:
    if isinstance(step, str):
        match = re.search(r"(?m)^\s*run:\s*(.*)$", step)
        if match is None:
            return step if "CARGO_INCREMENTAL" in step else ""
        return match.group(1).strip()
    run = step.get("run", "")
    if run is None:
        return ""
    if not isinstance(run, str):
        return str(run)
    return run


def _step_continue_on_error(step: dict) -> bool:
    if "continue-on-error" not in step:
        return False
    value = step.get("continue-on-error")
    if value is False:
        return False
    if isinstance(value, str) and value.strip().lower() == "false":
        return False
    return True


def _sccache_action_ref(uses: str) -> str | None:
    uses = str(uses).strip()
    if "#" in uses:
        uses = uses.split("#", 1)[0].strip()
    match = re.fullmatch(r"mozilla-actions/sccache-action@(.+)", uses, flags=re.IGNORECASE)
    if match is None:
        return None
    return match.group(1).strip()


def _is_sccache_action_step(step: dict) -> bool:
    uses = step.get("uses")
    if uses is None:
        return False
    return _sccache_action_ref(str(uses)) is not None


def _version_from_with(step: dict) -> str | None:
    with_block = step.get("with")
    if not isinstance(with_block, dict):
        return None
    version = with_block.get("version")
    if version is None:
        return None
    return str(version).strip()


def _quote_bang_if_lines(text: str) -> str:
    out: list[str] = []
    for line in text.splitlines():
        match = re.match(r"^(\s*if:\s*)(\S.*)$", line)
        if match:
            prefix, expr = match.groups()
            if expr and expr[0] not in "\"'" and "!" in expr:
                line = f"{prefix}{json.dumps(expr)}"
        out.append(line)
    return "\n".join(out)


def _load_workflow_yaml(text: str) -> dict | None:
    try:
        doc = yaml.safe_load(text)
    except yaml.YAMLError:
        try:
            doc = yaml.safe_load(_quote_bang_if_lines(text))
        except yaml.YAMLError:
            return None
    return doc if isinstance(doc, dict) else None


def _is_composite_action(path: str, doc: dict | None = None) -> bool:
    """A composite is a `runs.using: composite` document, or `.github/actions/**/action.y*ml`.

    Everything under `.github/workflows/` is a workflow, including a file
    whose name ends in `action.yml`, unless the document itself is composite.
    """
    if isinstance(doc, dict):
        runs = doc.get("runs")
        if isinstance(runs, dict) and str(runs.get("using", "")).lower() == "composite":
            return True
    normalized = path.replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    parts = [part for part in normalized.split("/") if part]
    if len(parts) < 3 or parts[0] != ".github" or parts[1] != "actions":
        return False
    return parts[-1] in ("action.yml", "action.yaml")


def _iter_sccache_step_lists(doc: dict, path: str) -> list[tuple[list[dict], dict | None]]:
    lists: list[tuple[list[dict], dict | None]] = []
    if _is_composite_action(path, doc):
        runs = doc.get("runs")
        if isinstance(runs, dict):
            steps = runs.get("steps")
            if isinstance(steps, list):
                lists.append(([step for step in steps if isinstance(step, dict)], None))
        return lists
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return lists
    for job in jobs.values():
        if not isinstance(job, dict):
            continue
        steps = job.get("steps")
        if isinstance(steps, list):
            lists.append(([step for step in steps if isinstance(step, dict)], job))
    return lists


_GITHUB_ENV_ASSIGN_RE = re.compile(
    r"""^\s*(?:echo|printf)\s+(?:"([^"]*)"|'([^']*)'|(\S+))\s*>>\s*["']?\$GITHUB_ENV["']?\s*$"""
)


def _is_wrapper_env_key(key: object) -> bool:
    text = str(key)
    if text.upper() in _WRAPPER_ENV_KEYS:
        return True
    lowered = text.lower()
    return lowered in _WRAPPER_CONFIG_KEYS or lowered in _WRAPPER_BARE_KEYS


def _is_sanctioned_wrapper_line(line: str, if_value: str | None) -> bool:
    if if_value == SCCACHE_PREFETCH_IF and line in _SANCTIONED_GATE_LINES:
        return True
    return if_value == SCCACHE_FALLBACK_IF and line == _SANCTIONED_FALLBACK_LINE


def _env_is_non_mapping(owner: object, field: str = "env") -> bool:
    return isinstance(owner, dict) and field in owner and not isinstance(owner.get(field), dict)


def _if_reads_sccache_gate(if_value: str) -> bool:
    lowered = if_value.lower()
    if "sccache-available" in lowered or "sccache_fetch.outputs.available" in lowered:
        return True
    # steps['sccache_fetch'].outputs.available keeps a quote between the id and `.outputs`.
    return re.search(r"sccache_fetch['\"]\s*\]\s*\.outputs\.available", lowered) is not None


_STEP_OUTPUT_ID_RE = re.compile(
    r"steps\[\s*['\"]([A-Za-z_][A-Za-z0-9_-]*)['\"]\s*\]\.outputs"
    r"|steps\.([A-Za-z_][A-Za-z0-9_-]*)\.outputs"
)
_PINNED_PREFETCH_RUN = "bash scripts/ci/fetch_sccache_release.sh"
_RUST_CI_SETUP_USES = "./.github/actions/rust-ci-setup"
_STATS_RUN = "sccache --show-stats"


def _step_output_refs(if_value: str) -> list[str]:
    refs: list[str] = []
    for match in _STEP_OUTPUT_ID_RE.finditer(if_value):
        ref = match.group(1) or match.group(2)
        if ref:
            refs.append(ref)
    return refs


class _CyclicStructure(Exception):
    """A YAML anchor points at a value that contains itself."""


def _string_values(value: object, seen: set[int] | None = None) -> list[str]:
    if isinstance(value, str):
        return [value]
    if not isinstance(value, (dict, list)):
        return []
    if seen is None:
        seen = set()
    marker = id(value)
    if marker in seen:
        raise _CyclicStructure()
    seen.add(marker)
    try:
        found: list[str] = []
        items = value.values() if isinstance(value, dict) else value
        for item in items:
            found.extend(_string_values(item, seen))
        return found
    finally:
        seen.remove(marker)


_ADJACENT_LITERAL_RE = re.compile(r"""(?P<q>["'])(?P<body>(?:\\.|(?!(?P=q)).)*)(?P=q)""")


def _joined_adjacent_literals(text: str) -> str:
    """Join quoted fragments that sit next to each other or are added together."""
    matches = list(_ADJACENT_LITERAL_RE.finditer(text))
    if len(matches) < 2:
        return text
    groups: list[list[re.Match[str]]] = [[matches[0]]]
    for previous, nxt in zip(matches, matches[1:]):
        gap = text[previous.end() : nxt.start()]
        if re.fullmatch(r"\s*\+\s*|\s*", gap):
            groups[-1].append(nxt)
        else:
            groups.append([nxt])
    rebuilt: list[str] = []
    last = 0
    for group in groups:
        rebuilt.append(text[last : group[0].start()])
        rebuilt.append("".join(match.group("body") for match in group))
        last = group[-1].end()
    rebuilt.append(text[last:])
    return "".join(rebuilt)


_ANSI_C_RE = re.compile(r"\$'((?:\\'|[^'])*)'")
_UNICODE_ESCAPE_RE = re.compile(r"\\u([0-9A-Fa-f]{4})|\\U([0-9A-Fa-f]{8})")
_HEX_ESCAPE_RE = re.compile(r"\\x([0-9A-Fa-f]{2})")


def _decode_ansi_c_body(body: str) -> str:
    def uni(match: re.Match[str]) -> str:
        return chr(int(match.group(1) or match.group(2), 16))

    body = _UNICODE_ESCAPE_RE.sub(uni, body)
    body = _HEX_ESCAPE_RE.sub(lambda match: chr(int(match.group(1), 16)), body)
    body = re.sub(r"\\([0-7]{1,3})", lambda match: chr(int(match.group(1), 8)), body)
    simple = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", "'": "'", '"': '"'}

    def slash(match: re.Match[str]) -> str:
        return simple.get(match.group(1), match.group(1))

    return re.sub(r"\\(.)", slash, body)


def _decode_wrapper_spellings(text: str) -> str:
    """Decode shell and TOML escapes so a wrapper key is visible as one word."""
    text = _ANSI_C_RE.sub(lambda match: _decode_ansi_c_body(match.group(1)), text)

    def uni(match: re.Match[str]) -> str:
        return chr(int(match.group(1) or match.group(2), 16))

    text = _UNICODE_ESCAPE_RE.sub(uni, text)
    text = _HEX_ESCAPE_RE.sub(lambda match: chr(int(match.group(1), 16)), text)
    return re.sub(r"\\(.)", r"\1", text)


def _expand_constructed_keys(text: str) -> str:
    """Expand format() and template literals whose pieces are all string literals."""

    def template(match: re.Match[str]) -> str:
        def slot(slot_match: re.Match[str]) -> str:
            inner = slot_match.group(1).strip()
            lit = re.fullmatch(r"""(?P<q>["'])(?P<body>(?:\\.|(?!(?P=q)).)*)(?P=q)""", inner)
            return lit.group("body") if lit else ""

        return re.sub(r"\$\{([^{}]*)\}", slot, match.group(1))

    text = re.sub(r"`([^`]*)`", template, text)

    def fmt(match: re.Match[str]) -> str:
        pattern = match.group("fmt")
        args = re.findall(r"""["']([^"']*)["']""", match.group("args") or "")
        pieces: list[str] = []
        index = 0
        cursor = 0
        while cursor < len(pattern):
            if pattern.startswith("%s", cursor) or pattern.startswith("{}", cursor):
                pieces.append(args[index] if index < len(args) else "")
                index += 1
                cursor += 2
                continue
            pieces.append(pattern[cursor])
            cursor += 1
        if index == 0:
            pieces.extend(args)
        return "".join(pieces)

    return re.sub(
        r"""\bformat!?\s*\(\s*(?P<q>["'])(?P<fmt>(?:\\.|(?!(?P=q)).)*)(?P=q)"""
        r"""(?P<args>(?:\s*,\s*["'][^"']*["'])*)\s*\)""",
        fmt,
        text,
    )


def _names_wrapper_key(text: str) -> bool:
    joined = _joined_adjacent_literals(text)
    candidates = (
        text,
        joined,
        _decode_wrapper_spellings(text),
        _decode_wrapper_spellings(joined),
        _expand_constructed_keys(text),
        _expand_constructed_keys(joined),
    )
    return any(_WRAPPER_KEY_RE.search(candidate) for candidate in candidates)


def _join_continuations(run: str) -> str:
    """Join a trailing backslash with the next line, as the shell does."""
    joined: list[str] = []
    pending = ""
    continued = False
    for line in run.splitlines():
        pending = pending + line if continued else line
        if pending.endswith("\\"):
            pending = pending[:-1]
            continued = True
            continue
        continued = False
        joined.append(pending)
        pending = ""
    if pending:
        joined.append(pending)
    return "\n".join(joined)


_ASSIGNMENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=.*$")
_PREFETCH_SCRIPT = "scripts/ci/fetch_sccache_release.sh"
_BASH_STARTUP_FLAGS = frozenset({"--noprofile", "--norc"})
_SCCACHE_COMMAND_RE = re.compile(r"(?i)(?<![A-Za-z0-9_])sccache(?:\.exe)?(?![A-Za-z0-9_])")
_FALLBACK_WARNING_LINE = 'echo "::warning::sccache unavailable, building without cache"'


def _shell_words(text: str) -> list[str] | None:
    words: list[str] = []
    buf: list[str] = []
    quote: str | None = None
    escaped = False
    for char in text:
        if escaped:
            buf.append(char)
            escaped = False
            continue
        if char == "\\" and quote != "'":
            escaped = True
            continue
        if quote is not None:
            if char == quote:
                quote = None
            else:
                buf.append(char)
            continue
        if char in {"'", '"'}:
            quote = char
            continue
        if char.isspace():
            if buf:
                words.append("".join(buf))
                buf = []
            continue
        buf.append(char)
    if quote is not None or escaped:
        return None
    if buf:
        words.append("".join(buf))
    return words


def _is_pinned_prefetch_run(run: str) -> bool:
    """The pinned prefetch script, including a clean-env bash and one release root.

    Today's step is `bash <script>`. The same step may be
    `env -i [NAME=value ...] bash --noprofile --norc <script> [root]`.
    A flag, a second argument, or a wrapper assignment is not that step.
    """
    words = _shell_words(run.strip())
    if not words:
        return False
    index = 0
    if words[0] == "env":
        if len(words) < 3 or words[1] != "-i":
            return False
        index = 2
        while index < len(words) and _ASSIGNMENT_RE.fullmatch(words[index]):
            key = words[index].split("=", 1)[0]
            if _is_wrapper_env_key(key):
                return False
            index += 1
        if index < len(words) and words[index] == "--":
            index += 1
    if index >= len(words) or words[index] != "bash":
        return False
    index += 1
    while index < len(words) and words[index] in _BASH_STARTUP_FLAGS:
        index += 1
    if index >= len(words) or words[index] != _PREFETCH_SCRIPT:
        return False
    index += 1
    if index == len(words):
        return True
    if index != len(words) - 1 or words[index].startswith("-"):
        return False
    return True


def _without_full_line_comments(run: str) -> str:
    return "\n".join(line for line in run.splitlines() if not line.strip().startswith("#"))


def _strip_sanctioned_lines(run: str) -> str:
    text = run
    for line in (*_SANCTIONED_GATE_LINES, _SANCTIONED_FALLBACK_LINE, _FALLBACK_WARNING_LINE):
        text = text.replace(line, "")
    return text


def _invokes_sccache_command(run: str) -> bool:
    """True when a run names sccache outside the pinned prefetch and the exact fallback.

    The stats check then requires the exact stats command. Runners, builtins,
    quoted commands, and a command split across a continued line all count.
    """
    if _is_pinned_prefetch_run(run.strip()):
        return False
    text = _decode_wrapper_spellings(_join_continuations(run))
    text = _strip_sanctioned_lines(_without_full_line_comments(text))
    if _is_pinned_prefetch_run(text.strip()):
        return False
    return _SCCACHE_COMMAND_RE.search(text) is not None


def _github_env_assignments(run: str) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    if not run:
        return found
    for line in run.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        match = _GITHUB_ENV_ASSIGN_RE.match(line)
        if match is None:
            continue
        body = next(group for group in match.groups() if group is not None)
        if "=" not in body:
            continue
        key, value = body.split("=", 1)
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            found.append((key, value))
    return found


def _step_wrapper_assignments(step: dict) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    env = step.get("env")
    if isinstance(env, dict):
        for key, value in env.items():
            if _is_wrapper_env_key(key):
                found.append((str(key), "" if value is None else str(value)))
    for key, value in _github_env_assignments(_step_run(step)):
        if _is_wrapper_env_key(key):
            found.append((key, value))
    return found


def _step_looks_like_fallback(step: dict) -> bool:
    run = _step_run(step)
    if SCCACHE_UNAVAILABLE_WARNING in run:
        return True
    return any(
        key == "RUSTC_WRAPPER" and value == "" for key, value in _github_env_assignments(run)
    )


def _is_exact_fallback_clear(assignments: list[tuple[str, str]]) -> bool:
    if not assignments:
        return False
    return all(key == "RUSTC_WRAPPER" and value == "" for key, value in assignments)


def _env_mapping_wrapper_keys(env: object) -> list[str]:
    if not isinstance(env, dict):
        return []
    return [str(key) for key in env if _is_wrapper_env_key(key)]


def _sccache_env_violations(path: str, doc: dict) -> list[str]:
    violations: list[str] = []
    if _env_is_non_mapping(doc):
        violations.append(f"{path}: workflow env must be a mapping")
    for key in _env_mapping_wrapper_keys(doc.get("env")):
        violations.append(f"{path}: workflow env must not set {key}")
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return violations
    for job_id, job in jobs.items():
        if not isinstance(job, dict):
            continue
        if _env_is_non_mapping(job):
            violations.append(f"{path} job {job_id}: job env must be a mapping")
        for key in _env_mapping_wrapper_keys(job.get("env")):
            violations.append(f"{path} job {job_id}: job env must not set {key}")
        container = job.get("container")
        if isinstance(container, dict):
            if _env_is_non_mapping(container):
                violations.append(f"{path} job {job_id}: container env must be a mapping")
            for key in _env_mapping_wrapper_keys(container.get("env")):
                violations.append(f"{path} job {job_id}: container env must not set {key}")
            try:
                expressed = any("${{" in text for text in _string_values(container))
            except _CyclicStructure:
                violations.append(f"{path} job {job_id}: cyclic anchor")
                expressed = False
            if expressed:
                violations.append(f"{path} job {job_id}: container expression is not checked")
        elif isinstance(container, str) and "${{" in container:
            violations.append(f"{path} job {job_id}: container expression is not checked")
    return violations


def _defaults_shell(owner: object) -> str | None:
    if not isinstance(owner, dict):
        return None
    defaults = owner.get("defaults")
    if not isinstance(defaults, dict):
        return None
    run = defaults.get("run")
    if not isinstance(run, dict):
        return None
    shell = run.get("shell")
    if not isinstance(shell, str) or not shell.strip():
        return None
    return shell.strip()


def _resolved_shell(step: dict, job: dict | None, doc: dict | None) -> str | None:
    shell = step.get("shell")
    if isinstance(shell, str) and shell.strip():
        return shell.strip()
    return _defaults_shell(job) or _defaults_shell(doc)


def _sccache_wrapper_violations(
    path: str, steps: list[dict], job: dict | None = None, doc: dict | None = None
) -> list[str]:
    violations: list[str] = []
    for step in steps:
        if _env_is_non_mapping(step):
            violations.append(f"{path}: step env must be a mapping")
        for text in _string_values(step.get("with")):
            if _names_wrapper_key(text):
                violations.append(f"{path}: with: names a wrapper key")
                break
        assignments = _step_wrapper_assignments(step)
        if_value = _step_if(step)
        if assignments:
            allowed = if_value == SCCACHE_PREFETCH_IF or (
                if_value == SCCACHE_FALLBACK_IF and _is_exact_fallback_clear(assignments)
            )
            if not allowed:
                violations.append(
                    f"{path}: RUSTC_WRAPPER=sccache if must be exactly {SCCACHE_PREFETCH_IF!r}, "
                    f"found {if_value!r}"
                )
            if if_value == SCCACHE_PREFETCH_IF:
                for key, value in assignments:
                    if value != "sccache":
                        violations.append(
                            f"{path}: gated wrapper value must be sccache, found {key}={value!r}"
                        )
            # `|| success()` is true after a prefetch that exits 0. The wrapper is
            # then set with no sccache binary, and cargo exits 101.
            if if_value is not None and "||" in if_value:
                violations.append(f"{path}: sccache enable if must not use ||, found {if_value!r}")
            if if_value is not None and "|| true" in if_value:
                violations.append(f"{path}: sccache enable if must not ignore failure with || true")
            if _step_continue_on_error(step):
                violations.append(f"{path}: continue-on-error is set on the RUSTC_WRAPPER step")
        for raw_line in _join_continuations(_step_run(step)).splitlines():
            line = raw_line.strip()
            if not line or not _names_wrapper_key(line):
                continue
            if _is_sanctioned_wrapper_line(line, if_value):
                if _resolved_shell(step, job, doc) != "bash":
                    violations.append(f"{path}: sanctioned wrapper line must run under bash")
                continue
            violations.append(f"{path}: wrapper run line is not the gated form: {line}")
    return violations


def _uses_is_fixed(uses: object) -> bool:
    if not isinstance(uses, str):
        return False
    text = uses.strip()
    return bool(text) and "${{" not in text


def _fixed_uses_violations(path: str, steps: list[dict]) -> list[str]:
    violations: list[str] = []
    for step in steps:
        if "uses" not in step:
            continue
        uses = step.get("uses")
        if _uses_is_fixed(uses):
            continue
        violations.append(f"{path}: uses must be a fixed action reference, found {uses!r}")
    return violations


def _unconditional_cargo_incremental(step: str | dict) -> bool:
    if isinstance(step, dict):
        if _step_if(step) is not None:
            return False
        if _step_continue_on_error(step):
            return False
        for line in _step_run(step).splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if CARGO_INCREMENTAL_ECHO in stripped:
                return True
        return False
    if re.search(r"(?m)^\s*if\s*:", step):
        return False
    if _has_continue_on_error(step):
        return False
    for line in step.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if CARGO_INCREMENTAL_ECHO in stripped or ("run: " + CARGO_INCREMENTAL_ECHO) in stripped:
            return True
    return False


def sccache_checksum_source_violations(script: str) -> list[str]:
    """The prefetch script verifies pinned sha256 values and fails closed on mismatch."""
    violations: list[str] = []
    if ".sha256" in script:
        violations.append("checksum verification reads a remote .sha256")
    if "SCCACHE_RELEASE_BASE" in script:
        violations.append("SCCACHE_RELEASE_BASE can retarget the download")
    for name, digest in PINNED_SCCACHE_SHA256.items():
        if name not in script or digest not in script:
            violations.append(f"missing in-repo sha256 pin for {name}")
    lines = _joined_adjacent_literals(script).splitlines()
    found = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("#") or "checksum mismatch for" not in line:
            continue
        found = True
        for earlier in lines[:index]:
            earlier_text = earlier.strip()
            if not earlier_text or earlier_text.startswith("#"):
                continue
            if "available=true" in earlier_text:
                violations.append("prefetch reports success before the checksum")
                break
        condition = _enclosing_if_condition(lines, index)
        if condition is None or "got" not in condition or "want" not in condition:
            violations.append("checksum mismatch must exit 1 immediately, found ''")
        nxt = ""
        for later in lines[index + 1 : index + 4]:
            if later.strip() and not later.strip().startswith("#"):
                nxt = later.strip()
                break
        if nxt != "exit 1":
            violations.append(f"checksum mismatch must exit 1 immediately, found {nxt!r}")
        window = "\n".join(lines[max(0, index - 3) : index + 4])
        if "fail_closed" in window or "available=false" in window:
            violations.append("checksum mismatch must not fall back")
    if not found:
        violations.append("missing checksum mismatch failure")
    commands = [
        line.strip() for line in lines if line.strip() and not line.strip().startswith("#")
    ]
    success_count = sum("available=true" in line for line in commands)
    if not commands or success_count != 1 or "available=true" not in commands[-1]:
        violations.append("available=true must be the last command")
    return violations


def _enclosing_if_condition(lines: list[str], index: int) -> str | None:
    depth = 0
    for cursor in range(index, -1, -1):
        stripped = lines[cursor].strip()
        if stripped.startswith("#") or not stripped:
            continue
        if re.match(r"^fi\b", stripped):
            depth += 1
            continue
        if re.match(r"^if\b", stripped):
            if depth == 0:
                return stripped
            depth -= 1
    return None


def _sccache_stats_violations(path: str, step: dict) -> list[str]:
    violations: list[str] = []
    run = _join_continuations(_step_run(step)).strip()
    if run != _STATS_RUN:
        violations.append(
            f"{path}: sccache stats run must be exactly {_STATS_RUN!r}, found {run!r}"
        )
    if_value = _step_if(step) or ""
    if "available == 'true'" not in if_value:
        violations.append(f"{path}: sccache --show-stats must run only when sccache is available")
    if if_value not in SCCACHE_STATS_IFS:
        violations.append(
            f"{path}: sccache --show-stats if must be an exact availability check, found {if_value!r}"
        )
    if "|| true" in if_value:
        violations.append(f"{path}: sccache --show-stats must not ignore failure")
    if _step_continue_on_error(step):
        violations.append(f"{path}: continue-on-error is set on sccache --show-stats")
    if step.get("shell") != "bash":
        violations.append(f"{path}: stats step must run under bash")
    return violations


def _gated_prefetch_id_violations(path: str, steps: list[dict]) -> list[str]:
    violations: list[str] = []
    for index, step in enumerate(steps):
        if_value = _step_if(step) or ""
        if not _if_reads_sccache_gate(if_value):
            continue
        refs = _step_output_refs(if_value)
        if not refs:
            violations.append(f"{path}: gated if does not reference a step id")
            continue
        for ref in refs:
            earlier = None
            for prev in steps[:index]:
                if prev.get("id") == ref:
                    earlier = prev
            if ref.lower() == "sccache_fetch":
                run = _step_run(earlier).strip() if earlier is not None else ""
                if ref != "sccache_fetch" or not _is_pinned_prefetch_run(run):
                    violations.append(f"{path}: sccache_fetch must be an earlier pinned prefetch")
            elif ref.lower() == "rust_ci":
                uses = ""
                if earlier is not None and isinstance(earlier.get("uses"), str):
                    uses = str(earlier.get("uses")).strip()
                if ref != "rust_ci" or uses != _RUST_CI_SETUP_USES:
                    if earlier is None:
                        violations.append(f"{path}: gated if references missing step id {ref!r}")
                    else:
                        violations.append(f"{path}: rust_ci must be an earlier rust-ci-setup step")
            elif earlier is None:
                violations.append(f"{path}: gated if references missing step id {ref!r}")
    return violations


def _sccache_action_step_violations(path: str, steps: list[dict], index: int) -> list[str]:
    violations: list[str] = []
    step = steps[index]
    prefix = f"{path}: sccache-action"
    uses = step.get("uses")
    ref = _sccache_action_ref(str(uses)) if uses is not None else ""
    if not ref or not re.fullmatch(r"[0-9a-f]{40}", ref):
        violations.append(f"{prefix}: ref {ref!r} is not a 40-char commit SHA")
    elif ref != SCCACHE_ACTION_COMMIT:
        violations.append(f"{prefix}: commit {ref} is not {SCCACHE_ACTION_COMMIT} (v0.0.11)")
    version = _version_from_with(step)
    if version != SCCACHE_VERSION:
        violations.append(f"{prefix}: unpinned version {version!r}; want explicit {SCCACHE_VERSION}")
    if_value = _step_if(step)
    if if_value != SCCACHE_PREFETCH_IF:
        violations.append(
            f"{prefix}: install if must be exactly {SCCACHE_PREFETCH_IF!r}, found {if_value!r}"
        )
    if _step_continue_on_error(step):
        violations.append(f"{prefix}: continue-on-error is set")

    fetch_at = _last_prefetch_index_before(steps, index)
    if fetch_at < 0:
        violations.append(f"{prefix}: missing prefetch step")
    else:
        fetch = steps[fetch_at]
        if fetch.get("id") != "sccache_fetch":
            violations.append(f"{prefix}: prefetch step is missing id sccache_fetch")
        if _step_continue_on_error(fetch):
            violations.append(f"{prefix}: continue-on-error is set on the prefetch")
        run = _step_run(fetch)
        if_value = _step_if(fetch) or ""
        if not _is_pinned_prefetch_run(run.strip()):
            violations.append(f"{prefix}: prefetch run must be exactly the pinned script")
        if "|| true" in if_value or "|| true" in run:
            violations.append(f"{prefix}: prefetch must not ignore failure with || true")

    fallback_at = _last_fallback_index_before(steps, index)
    if fallback_at < 0:
        violations.append(f"{prefix}: missing fallback step")
    else:
        fallback = steps[fallback_at]
        run = _step_run(fallback)
        if SCCACHE_UNAVAILABLE_WARNING not in run:
            violations.append(f"{prefix}: fallback does not emit {SCCACHE_UNAVAILABLE_WARNING}")
        writes = _env_writes(run)
        env_lines = [line for line in run.splitlines() if "GITHUB_ENV" in line]
        if writes != ["RUSTC_WRAPPER="] or len(env_lines) != 1:
            violations.append(f"{prefix}: fallback must only touch RUSTC_WRAPPER, wrote {writes or env_lines}")
        if _step_if(fallback) != SCCACHE_FALLBACK_IF:
            violations.append(f"{prefix}: fallback if must be the failed prefetch")
        if _step_continue_on_error(fallback):
            violations.append(f"{prefix}: continue-on-error is set on the fallback")
    return violations


def _include_strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [item for item in value if isinstance(item, str)]
    return []


def _toml_include_paths(value: object) -> list[str]:
    found: list[str] = []
    if isinstance(value, dict):
        for key, item in value.items():
            key_text = str(key)
            if key_text == "include":
                found.extend(_include_strings(item))
            elif key_text == "include_if" and isinstance(item, list):
                for entry in item:
                    if isinstance(entry, dict):
                        found.extend(_include_strings(entry.get("include")))
            else:
                found.extend(_toml_include_paths(item))
    elif isinstance(value, list):
        for item in value:
            found.extend(_toml_include_paths(item))
    return found


def _toml_names_wrapper(value: object, prefix: str = "") -> bool:
    if isinstance(value, dict):
        for key, item in value.items():
            key_text = str(key)
            dotted = f"{prefix}.{key_text}" if prefix else key_text
            if _is_wrapper_env_key(key_text) or _is_wrapper_env_key(dotted):
                return True
            if _toml_names_wrapper(item, dotted):
                return True
        return False
    if isinstance(value, list):
        return any(_toml_names_wrapper(item, prefix) for item in value)
    if isinstance(value, str):
        return _names_wrapper_key(value)
    return False


def _resolve_included_config(parent: str, include: str) -> str | None:
    raw = include.strip()
    if not raw or raw.startswith("/") or "${{" in raw or ".." in Path(raw).parts:
        return None
    parent_dir = Path(parent).parent
    relative = (parent_dir / raw).as_posix()
    if relative.startswith("../") or relative.startswith("/"):
        return None
    return relative


def _cargo_config_violations(
    path: str,
    text: str,
    texts: dict[str, str] | None = None,
    seen: set[str] | None = None,
) -> list[str]:
    if seen is None:
        seen = set()
    if path in seen:
        return [f"{path}: cargo config include cycle"]
    seen.add(path)
    try:
        parsed = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        parsed = None
    if parsed is None:
        for line in _decode_wrapper_spellings(text).splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if _names_wrapper_key(stripped):
                return [f"{path}: cargo config sets a wrapper key"]
        return []
    violations: list[str] = []
    if _toml_names_wrapper(parsed):
        violations.append(f"{path}: cargo config sets a wrapper key")
    for include in _toml_include_paths(parsed):
        relative = _resolve_included_config(path, include)
        if relative is None:
            violations.append(f"{path}: cargo config include could not be read")
            continue
        if texts is not None and relative in texts:
            body = texts[relative]
        else:
            file = ROOT / relative
            if not file.is_file():
                violations.append(f"{path}: cargo config include could not be read")
                continue
            body = file.read_text(encoding="utf-8")
        violations.extend(_cargo_config_violations(relative, body, texts, seen))
    return violations


def _referenced_cargo_configs(text: str) -> tuple[list[str], list[str]]:
    """Checked-in Cargo configs a workflow can load, and paths that cannot be read."""
    paths: list[str] = []
    missing: list[str] = []
    for match in re.finditer(r"(?m)^[ \t]*CARGO_HOME:[ \t]*['\"]?([^'\"\n#]+)", text):
        raw = match.group(1).strip().strip("'\"")
        if not raw or "${{" in raw or raw.startswith("$"):
            missing.append(raw or "CARGO_HOME")
            continue
        home = raw.rstrip("/")
        paths.append(f"{home}/config.toml")
        paths.append(f"{home}/config")
    for match in re.finditer(r"--config(?:[=\s]+)(\S+)", text):
        token = match.group(1).strip("'\"")
        if not token or token.startswith("-") or "=" in token or "${{" in token:
            continue
        paths.append(token)
    for match in re.finditer(r"\b(?:cp|mv|install)\s+(\S+)\s+(\S+)", text):
        source = match.group(1).strip("'\"")
        dest = match.group(2).strip("'\"")
        if ".cargo" in dest or "CARGO_HOME" in dest:
            paths.append(source)
    return paths, missing


def _resolve_local_action(target: str) -> Path | None:
    relative = target.strip()
    while relative.startswith("./"):
        relative = relative[2:]
    candidate = (ROOT / relative).resolve()
    try:
        candidate.relative_to(ROOT.resolve())
    except ValueError:
        return None
    if candidate.is_file() and candidate.suffix in {".yml", ".yaml"}:
        return candidate
    if candidate.is_dir():
        for name in ("action.yml", "action.yaml"):
            action = candidate / name
            if action.is_file():
                return action
    return None


def _iter_uses(node: object, seen: set[int] | None = None):
    if seen is None:
        seen = set()
    if not isinstance(node, (dict, list)):
        return
    marker = id(node)
    if marker in seen:
        raise _CyclicStructure()
    seen.add(marker)
    try:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "uses":
                    yield value
                yield from _iter_uses(value, seen)
        else:
            for item in node:
                yield from _iter_uses(item, seen)
    finally:
        seen.remove(marker)


def _expand_local_actions(texts: dict[str, str]) -> tuple[dict[str, str], list[str]]:
    """Load every local `uses: ./…` target. An unreadable target is a violation."""
    expanded = dict(texts)
    violations: list[str] = []
    pending = [(path, text) for path, text in texts.items() if not _is_cargo_config(path)]
    seen = set(expanded)
    while pending:
        path, text = pending.pop()
        doc = _load_workflow_yaml(text)
        if not isinstance(doc, dict):
            continue
        try:
            uses_values = list(_iter_uses(doc))
        except _CyclicStructure:
            violations.append(f"{path}: cyclic anchor")
            continue
        for uses in uses_values:
            if not isinstance(uses, str):
                continue
            target = uses.strip()
            if "#" in target:
                target = target.split("#", 1)[0].strip()
            if not target.startswith("./"):
                continue
            target = target.split("@", 1)[0]
            resolved = _resolve_local_action(target)
            if resolved is None:
                violations.append(f"{path}: local action could not be read: {target}")
                continue
            rel = resolved.relative_to(ROOT).as_posix()
            if rel in seen:
                continue
            seen.add(rel)
            body = resolved.read_text(encoding="utf-8")
            expanded[rel] = body
            pending.append((rel, body))
    return expanded, violations


def _other_field_wrapper_violations(
    path: str, node: object, seen: set[int] | None = None
) -> list[str]:
    """Flag a wrapper key in any string that is not a run body or an env key."""
    if seen is None:
        seen = set()
    violations: list[str] = []
    if isinstance(node, (dict, list)):
        marker = id(node)
        if marker in seen:
            raise _CyclicStructure()
        seen.add(marker)
        try:
            if isinstance(node, dict):
                for key, value in node.items():
                    key_text = str(key)
                    if key_text == "env" and isinstance(value, dict):
                        for env_value in value.values():
                            violations.extend(_other_field_wrapper_violations(path, env_value, seen))
                        continue
                    if key_text == "run" and isinstance(value, str):
                        continue
                    if _names_wrapper_key(key_text):
                        violations.append(f"{path}: wrapper key outside a sanctioned line")
                    violations.extend(_other_field_wrapper_violations(path, value, seen))
            else:
                for item in node:
                    violations.extend(_other_field_wrapper_violations(path, item, seen))
        finally:
            seen.remove(marker)
        return violations
    if isinstance(node, str) and _names_wrapper_key(node):
        violations.append(f"{path}: wrapper key outside a sanctioned line")
    return violations


_PREFETCH_FIELDS = frozenset({"name", "id", "shell", "run"})
_PATH_ENV_KEYS = frozenset({"PATH", "GITHUB_PATH"})
_PATH_ASSIGN_RE = re.compile(r"(?:^|&&|\|\||[;&|])\s*(?:export\s+)?PATH\s*=")


def _is_prefetch_step(step: dict) -> bool:
    if step.get("id") == "sccache_fetch":
        return True
    return _is_pinned_prefetch_run(_step_run(step).strip())


def _defaults_working_directory(owner: object) -> bool:
    if not isinstance(owner, dict):
        return False
    defaults = owner.get("defaults")
    if not isinstance(defaults, dict):
        return False
    run = defaults.get("run")
    return isinstance(run, dict) and "working-directory" in run


def _mapping_changes_path(env: object) -> bool:
    if not isinstance(env, dict):
        return False
    return any(str(key).upper() in _PATH_ENV_KEYS for key in env)


_PREFETCH_ENV_ALLOWLIST = frozenset(
    {
        "CARGO_TERM_COLOR",
        "CARGO_INCREMENTAL",
        "CARGO_NET_RETRY",
        "RUST_BACKTRACE",
        "RUST_LIB_BACKTRACE",
        "EXCLUDED_PACKAGES",
        "SCCACHE_CACHE_SIZE",
        "SCCACHE_IDLE_TIMEOUT",
    }
)


def _step_changes_path(step: dict) -> bool:
    if _mapping_changes_path(step.get("env")):
        return True
    run = _step_run(step)
    if "GITHUB_PATH" in run or _PATH_ASSIGN_RE.search(run):
        return True
    if "GITHUB_ENV" in run and re.search(r"(?i)(?<![A-Za-z0-9_])PATH\s*=", run):
        return True
    with_block = step.get("with")
    if isinstance(with_block, dict):
        try:
            script_text = "\n".join(_string_values(with_block))
        except _CyclicStructure:
            script_text = ""
        if "addPath" in script_text:
            return True
    return False


def _step_rewrites_prefetch_script(step: dict) -> bool:
    run = _decode_wrapper_spellings(_joined_adjacent_literals(_step_run(step)))
    if _is_pinned_prefetch_run(run.strip()):
        return False
    if "fetch_sccache_release.sh" in run:
        return True
    return re.search(r"\b(?:cp|mv|install|tee)\b", run) is not None and "scripts/ci" in run


def _prefetch_step_shape_violations(path: str, step: dict) -> list[str]:
    violations: list[str] = []
    extra = sorted(str(key) for key in step if key not in _PREFETCH_FIELDS)
    if extra:
        violations.append(f"{path}: prefetch step has unexpected fields {extra}")
    if step.get("shell") != "bash":
        violations.append(f"{path}: prefetch shell must be bash")
    if not _is_pinned_prefetch_run(_step_run(step).strip()):
        violations.append(f"{path}: sccache_fetch must be an earlier pinned prefetch")
    if _step_continue_on_error(step):
        violations.append(f"{path}: continue-on-error is set on the prefetch")
    return violations


def _earlier_env_file_unlisted(step: dict) -> bool:
    run = _step_run(step)
    if "GITHUB_ENV" not in run:
        return False
    for line in run.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "GITHUB_ENV" not in line:
            continue
        match = re.search(r"""(?:echo|printf)\s+(?:"([^"]*)"|'([^']*)'|(\S+))""", line)
        if match is None:
            return True
        body = next(group for group in match.groups() if group is not None)
        if "=" not in body:
            return True
        key = body.split("=", 1)[0]
        if str(key).upper() not in _PREFETCH_ENV_ALLOWLIST:
            return True
    return False


def _prefetch_context_violations(
    path: str, earlier: list[dict], job: dict | None, doc: dict | None
) -> list[str]:
    violations: list[str] = []
    if _defaults_working_directory(job) or _defaults_working_directory(doc):
        violations.append(f"{path}: prefetch working directory is not the default")
    for owner in (job, doc):
        if not isinstance(owner, dict):
            continue
        env = owner.get("env")
        if _mapping_changes_path(env):
            violations.append(f"{path}: prefetch PATH is changed")
        elif isinstance(env, dict):
            for key in env:
                if str(key).upper() not in _PREFETCH_ENV_ALLOWLIST:
                    violations.append(f"{path}: prefetch environment is not allowlisted")
                    break
    for step in earlier:
        if "working-directory" in step:
            violations.append(f"{path}: prefetch working directory is not the default")
        if _step_changes_path(step):
            violations.append(f"{path}: prefetch PATH is changed")
        if _step_rewrites_prefetch_script(step):
            violations.append(f"{path}: prefetch script is rewritten by an earlier step")
        if _earlier_env_file_unlisted(step):
            violations.append(f"{path}: prefetch env file is written by an earlier step")
    return violations


def _prefetch_pin_violations(
    path: str, steps: list[dict], job: dict | None, doc: dict | None
) -> list[str]:
    violations: list[str] = []
    for index, step in enumerate(steps):
        if not _is_prefetch_step(step):
            continue
        violations.extend(_prefetch_step_shape_violations(path, step))
        violations.extend(_prefetch_context_violations(path, steps[:index], job, doc))
    return list(dict.fromkeys(violations))


def _uses_rust_ci_setup(step: dict) -> bool:
    uses = step.get("uses")
    if not isinstance(uses, str):
        return False
    target = uses.split("#", 1)[0].strip().split("@", 1)[0]
    return target == _RUST_CI_SETUP_USES


def _composite_caller_violations(
    path: str, steps: list[dict], job: dict | None, doc: dict | None
) -> list[str]:
    violations: list[str] = []
    for index, step in enumerate(steps):
        if not _uses_rust_ci_setup(step):
            continue
        violations.extend(_prefetch_context_violations(path, steps[:index], job, doc))
    return list(dict.fromkeys(violations))


def _job_runs_sccache_policy_steps(steps: list[dict]) -> bool:
    for step in steps:
        if _is_sccache_action_step(step) or _uses_rust_ci_setup(step) or _is_prefetch_step(step):
            return True
        if _step_looks_like_fallback(step) or _step_wrapper_assignments(step):
            return True
        if _invokes_sccache_command(_step_run(step)):
            return True
    return False


def _unconditional_continue_on_error(value: object) -> bool:
    if value is True:
        return True
    if not isinstance(value, str):
        return False
    text = value.strip()
    return text.lower() == "true" or "${{" in text


def _load_referenced_cargo_configs(
    expanded: dict[str, str],
) -> tuple[set[str], list[str]]:
    cargo_paths = {path for path in expanded if _is_cargo_config(path)}
    violations: list[str] = []
    homes: list[tuple[str, str]] = []
    required: list[tuple[str, str]] = []
    for path, text in expanded.items():
        if path in cargo_paths:
            continue
        for match in re.finditer(r"(?m)^[ \t]*CARGO_HOME:[ \t]*['\"]?([^'\"\n#]+)", text):
            raw = match.group(1).strip().strip("'\"")
            if not raw or "${{" in raw or raw.startswith("$"):
                violations.append(f"{path}: cargo config could not be read")
                continue
            homes.append((path, raw.rstrip("/")))
        for match in re.finditer(r"(?m)^(?!.*#).*\bcargo\b.*--config(?:[=\s]+)(\S+)", text):
            token = match.group(1).strip("'\"")
            if not token or token.startswith("-") or "=" in token or "${{" in token:
                continue
            required.append((path, token))
        for match in re.finditer(r"\b(?:cp|mv|install)\s+(\S+)\s+(\S+)", text):
            source = match.group(1).strip("'\"")
            dest = match.group(2).strip("'\"")
            if ".cargo" in dest or "CARGO_HOME" in dest:
                required.append((path, source))
    for path, home in homes:
        candidates = [f"{home}/config.toml", f"{home}/config"]
        found = False
        for rel in candidates:
            file = ROOT / rel
            if rel in expanded or file.is_file():
                found = True
                if rel not in expanded and file.is_file():
                    expanded[rel] = file.read_text(encoding="utf-8")
                cargo_paths.add(rel)
        if not found:
            violations.append(f"{path}: cargo config could not be read")
    for path, rel in required:
        file = ROOT / rel
        if rel not in expanded and not file.is_file():
            violations.append(f"{path}: cargo config could not be read")
            continue
        if rel not in expanded:
            expanded[rel] = file.read_text(encoding="utf-8")
        cargo_paths.add(rel)
    return cargo_paths, violations


def sccache_install_violations(texts: dict[str, str]) -> list[str]:
    expanded, violations = _expand_local_actions(texts)
    cargo_paths, cargo_missing = _load_referenced_cargo_configs(expanded)
    violations.extend(cargo_missing)
    uses_per_file: dict[str, int] = {}
    for path, text in expanded.items():
        if path in cargo_paths or _is_cargo_config(path):
            violations.extend(_cargo_config_violations(path, text, expanded))
            continue
        if SCCACHE_ACTION_TAG_OBJECT in text:
            violations.append(f"{path}: sccache-action is pinned to the v0.0.11 tag object")
        if "SCCACHE_RELEASE_BASE" in text or "--release-base" in text:
            violations.append(f"{path}: release URL override is test-only")
        try:
            doc = _load_workflow_yaml(text)
            if doc is None:
                violations.append(f"{path}: workflow YAML did not parse")
                continue
            violations.extend(_other_field_wrapper_violations(path, doc))
            step_lists = _iter_sccache_step_lists(doc, path)
            file_uses = 0
            for steps, job in step_lists:
                violations.extend(_fixed_uses_violations(path, steps))
                action_indexes = [
                    index for index, step in enumerate(steps) if _is_sccache_action_step(step)
                ]
                file_uses += len(action_indexes)
                violations.extend(_sccache_wrapper_violations(path, steps, job, doc))
                violations.extend(_gated_prefetch_id_violations(path, steps))
                violations.extend(_prefetch_pin_violations(path, steps, job, doc))
                violations.extend(_composite_caller_violations(path, steps, job, doc))
                if job is not None and _unconditional_continue_on_error(job.get("continue-on-error")):
                    if _job_runs_sccache_policy_steps(steps):
                        violations.append(
                            f"{path}: continue-on-error is set on a job that runs sccache"
                        )
                incremental_steps = [step for step in steps if _unconditional_cargo_incremental(step)]
                if action_indexes and not incremental_steps:
                    violations.append(
                        f"{path}: CARGO_INCREMENTAL=0 must be set by a step with no if:"
                    )
                for step in incremental_steps:
                    if step.get("shell") != "bash":
                        violations.append(f"{path}: incremental step must run under bash")
                for index in action_indexes:
                    violations.extend(_sccache_action_step_violations(path, steps, index))
                for step in steps:
                    if _invokes_sccache_command(_step_run(step)):
                        violations.extend(_sccache_stats_violations(path, step))
            if file_uses:
                uses_per_file[path] = file_uses
            violations.extend(_sccache_env_violations(path, doc))
        except _CyclicStructure:
            violations.append(f"{path}: cyclic anchor")
    for path, count in EXPECTED_SCCACHE_ACTION_USES.items():
        found = uses_per_file.get(path, 0)
        if found != count:
            violations.append(f"{path}: expected {count} sccache-action uses, found {found}")
    for path in sorted(set(uses_per_file) - set(EXPECTED_SCCACHE_ACTION_USES)):
        violations.append(f"{path}: unexpected sccache-action use")
    return violations


def _prefetch_run_line(text: str) -> str:
    """The live prefetch `run:` line, found by the script name rather than one spelling."""
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if stripped.startswith("run:") and _PREFETCH_SCRIPT in stripped:
            return line if line.endswith("\n") else line + "\n"
    raise AssertionError("prefetch run line missing")


def _replace_policy_once(texts: dict[str, str], path: str, old: str, new: str) -> dict[str, str]:
    if old not in texts[path]:
        raise AssertionError(f"mutant anchor missing in {path}")
    copied = dict(texts)
    copied[path] = texts[path].replace(old, new, 1)
    if copied[path] == texts[path]:
        raise AssertionError(f"mutant did not change {path}")
    return copied


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
) -> tuple[subprocess.CompletedProcess[str], str]:
    env = os.environ.copy()
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
        incremental_steps = [
            step for step in step_blocks(action) if "Disable incremental compilation" in step
        ]
        self.assertEqual(len(incremental_steps), 1, incremental_steps)
        self.assertTrue(_unconditional_cargo_incremental(incremental_steps[0]), incremental_steps[0])
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

    def test_sccache_action_is_pinned_and_falls_back_without_continue_on_error(self) -> None:
        texts = sccache_policy_texts()
        self.assertEqual(sccache_install_violations(texts), [])
        script = (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh").read_text(encoding="utf-8")
        self.assertIn("VERSION=v0.19.0", script)
        self.assertIn('ATTEMPTS="${SCCACHE_FETCH_ATTEMPTS:-5}"', script)
        self.assertIn("createHash(\"sha256\")", script)
        self.assertIn('sleep "$delay"', script)
        self.assertIn("available=false", script)
        self.assertIn("available=true", script)
        self.assertIn(".complete", script)
        for token in (
            "x86_64",
            "aarch64",
            "armv7",
            "apple-darwin",
            "pc-windows-msvc",
            "unknown-linux-musl",
            "unknown-linux-musleabi",
        ):
            self.assertIn(token, script)
        self.assertNotIn("RUSTC_WRAPPER", script)
        self.assertNotIn("GITHUB_ENV", script)
        self.assertNotIn("continue-on-error", script)
        for line in script.splitlines():
            stripped = line.strip()
            if stripped.startswith("#") or "|| true" not in line:
                continue
            self.assertTrue(
                stripped.startswith(("echo ", "rm ", "write_output ", "builtin printf ", "printf ", "trap ")),
                line,
            )
        self.assertNotIn(".sha256", script)
        self.assertNotIn("SCCACHE_RELEASE_BASE", script)
        self.assertIn('--release-base', script)
        self.assertEqual(sccache_checksum_source_violations(script), [])

        silent = _replace_policy_once(
            texts,
            ".github/workflows/integration-tests.yml",
            'echo "::warning::sccache unavailable, building without cache"\n',
            "\n",
        )
        silent_violations = sccache_install_violations(silent)
        self.assertTrue(
            any("does not emit" in item for item in silent_violations),
            silent_violations,
        )

        softened = _replace_policy_once(
            texts,
            ".github/workflows/toolchain-management.yml",
            "        uses: mozilla-actions/sccache-action@" + SCCACHE_ACTION_COMMIT,
            "        continue-on-error: true\n"
            "        uses: mozilla-actions/sccache-action@" + SCCACHE_ACTION_COMMIT,
        )
        soft_violations = sccache_install_violations(softened)
        self.assertTrue(
            any("continue-on-error is set" in item for item in soft_violations),
            soft_violations,
        )

        unpinned = _replace_policy_once(
            texts,
            ".github/workflows/benchmark_alert.yml",
            "          version: v0.19.0\n",
            '          version: ""\n',
        )
        unpinned_violations = sccache_install_violations(unpinned)
        self.assertTrue(
            any("unpinned version" in item for item in unpinned_violations),
            unpinned_violations,
        )

        tag_object = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            SCCACHE_ACTION_COMMIT,
            SCCACHE_ACTION_TAG_OBJECT,
        )
        tag_violations = sccache_install_violations(tag_object)
        self.assertTrue(tag_violations)
        self.assertTrue(
            any(SCCACHE_ACTION_TAG_OBJECT in item or "tag object" in item for item in tag_violations),
            tag_violations,
        )

        dropped_incremental = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            CARGO_INCREMENTAL_ECHO + "\n",
            "\n",
        )
        dropped_violations = sccache_install_violations(dropped_incremental)
        self.assertTrue(
            any("CARGO_INCREMENTAL=0 must be set" in item for item in dropped_violations),
            dropped_violations,
        )

        prefetch_line = _prefetch_run_line(texts[".github/actions/rust-ci-setup/action.yml"])
        ignored = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            prefetch_line,
            prefetch_line.rstrip("\n") + " || true\n",
        )
        ignored_violations = sccache_install_violations(ignored)
        self.assertTrue(
            any("|| true" in item for item in ignored_violations),
            ignored_violations,
        )

        enable_if = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Enable sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - name: Enable sccache\n      if: steps.sccache_fetch.outputs.available == 'true' || true\n",
        )
        enable_if_violations = sccache_install_violations(enable_if)
        self.assertTrue(
            any("RUSTC_WRAPPER=sccache if must be exactly" in item for item in enable_if_violations),
            enable_if_violations,
        )

        enable_negated = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Enable sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - name: Enable sccache\n      if: !(steps.sccache_fetch.outputs.available == 'true')\n",
        )
        enable_negated_violations = sccache_install_violations(enable_negated)
        self.assertTrue(
            any("RUSTC_WRAPPER=sccache if must be exactly" in item for item in enable_negated_violations),
            enable_negated_violations,
        )

        configure_not_equal = _replace_policy_once(
            texts,
            ".github/workflows/toolchain-management.yml",
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available == 'true'\n",
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available != 'true'\n",
        )
        configure_not_equal_violations = sccache_install_violations(configure_not_equal)
        self.assertTrue(
            any("RUSTC_WRAPPER=sccache if must be exactly" in item for item in configure_not_equal_violations),
            configure_not_equal_violations,
        )

        enable_success = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Enable sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - name: Enable sccache\n      if: steps.sccache_fetch.outputs.available == 'true' || success()\n",
        )
        enable_success_violations = sccache_install_violations(enable_success)
        self.assertTrue(
            any("sccache enable if must not use ||" in item for item in enable_success_violations),
            enable_success_violations,
        )

        configure_success = _replace_policy_once(
            texts,
            ".github/workflows/toolchain-management.yml",
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available == 'true'\n",
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available == 'true' || success()\n",
        )
        configure_success_violations = sccache_install_violations(configure_success)
        self.assertTrue(
            any("sccache enable if must not use ||" in item for item in configure_success_violations),
            configure_success_violations,
        )

        install_or_true = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true' || true\n",
        )
        install_or_true_violations = sccache_install_violations(install_or_true)
        self.assertTrue(
            any("install if must be exactly" in item for item in install_or_true_violations),
            install_or_true_violations,
        )

        install_or_success = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true' || success()\n",
        )
        install_or_success_violations = sccache_install_violations(install_or_success)
        self.assertTrue(
            any("install if must be exactly" in item for item in install_or_success_violations),
            install_or_success_violations,
        )

        workflow_install = _replace_policy_once(
            texts,
            ".github/workflows/toolchain-management.yml",
            "      - name: Install sccache\n        if: steps.sccache_fetch.outputs.available == 'true'\n",
            "      - name: Install sccache\n        if: steps.sccache_fetch.outputs.available == 'true' || success()\n",
        )
        workflow_install_violations = sccache_install_violations(workflow_install)
        self.assertTrue(
            any("install if must be exactly" in item for item in workflow_install_violations),
            workflow_install_violations,
        )

        prefetch_line = _prefetch_run_line(texts[".github/actions/rust-ci-setup/action.yml"])
        overridden = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            prefetch_line,
            "      env:\n        SCCACHE_RELEASE_BASE: http://example.invalid\n" + prefetch_line,
        )
        override_violations = sccache_install_violations(overridden)
        self.assertTrue(
            any("release URL override is test-only" in item for item in override_violations),
            override_violations,
        )

        remote = script.replace(
            'want="$(pinned_sha256 "$filename")"\n',
            'curl -fsSL -o "$tmp/published" "${url}.sha256"\n'
            'want="$(cut -d" " -f1 < "$tmp/published")"\n',
            1,
        )
        self.assertNotEqual(remote, script)
        remote_violations = sccache_checksum_source_violations(remote)
        self.assertNotEqual(remote_violations, [])
        self.assertTrue(
            any("remote .sha256" in item for item in remote_violations),
            remote_violations,
        )

        mismatch_fallback = script.replace(
            "  builtin printf 'sccache checksum mismatch for %s\\n' \"$filename\" >&2\n  exit 1\n",
            "  builtin printf 'sccache checksum mismatch for %s\\n' \"$filename\" >&2\n"
            '  fail_closed "checksum mismatch"\n',
        )
        self.assertNotEqual(mismatch_fallback, script)
        fallback_violations = sccache_checksum_source_violations(mismatch_fallback)
        self.assertTrue(
            any("exit 1" in item or "must not fall back" in item for item in fallback_violations),
            fallback_violations,
        )

        bare_uses = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n",
            "    - uses: mozilla-actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + "\n      with:\n        version: v0.19.0\n    - name: Install sccache\n",
        )
        bare_uses_violations = sccache_install_violations(bare_uses)
        self.assertTrue(
            any("expected 1 sccache-action uses, found 2" in item for item in bare_uses_violations),
            bare_uses_violations,
        )

        quoted_extra = _replace_policy_once(
            texts,
            ".github/workflows/integration-tests.yml",
            "      - name: Install sccache\n",
            "      - uses: 'mozilla-actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + "'\n        with:\n          version: v0.19.0\n      - name: Install sccache\n",
        )
        quoted_extra_violations = sccache_install_violations(quoted_extra)
        self.assertTrue(
            any("expected 5 sccache-action uses, found 6" in item for item in quoted_extra_violations),
            quoted_extra_violations,
        )

        flow_map = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
            "    - {uses: mozilla-actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + ", with: {version: v0.19.0}}\n"
            "    - name: Install sccache\n      if: steps.sccache_fetch.outputs.available == 'true'\n",
        )
        flow_map_violations = sccache_install_violations(flow_map)
        self.assertTrue(
            any("install if must be exactly" in item for item in flow_map_violations),
            flow_map_violations,
        )

        mixed_case_extra = _replace_policy_once(
            texts,
            ".github/workflows/dependency-management.yml",
            "      - name: Setup sccache\n",
            "      - uses: Mozilla-Actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + "\n        with:\n          version: v0.19.0\n      - name: Setup sccache\n",
        )
        mixed_case_violations = sccache_install_violations(mixed_case_extra)
        self.assertTrue(
            any("expected 1 sccache-action uses, found 2" in item for item in mixed_case_violations),
            mixed_case_violations,
        )

        env_wrapper = _replace_policy_once(
            texts,
            ".github/workflows/toolchain-management.yml",
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available == 'true'\n",
            "      - env:\n          RUSTC_WRAPPER: sccache\n        run: echo SCCACHE_GHA_ENABLED=true >> $GITHUB_ENV\n"
            "      - name: Configure sccache\n        if: steps.sccache_fetch.outputs.available == 'true'\n",
        )
        env_wrapper_violations = sccache_install_violations(env_wrapper)
        self.assertTrue(
            any("RUSTC_WRAPPER=sccache if must be exactly" in item for item in env_wrapper_violations),
            env_wrapper_violations,
        )

    def test_sccache_policy_checker_mutants(self) -> None:
        texts = sccache_policy_texts()

        def flagged(path: str, text: str, needle: str) -> None:
            found = sccache_install_violations({path: text})
            self.assertTrue(any(needle in item for item in found), found)

        flagged(
            ".github/workflows/caller.yml",
            "name: caller\n"
            "on: push\n"
            "env:\n"
            "  RUSTC_WRAPPER: sccache\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./.github/actions/rust-ci-setup\n",
            "workflow env must not set RUSTC_WRAPPER",
        )
        flagged(
            ".github/workflows/called.yml",
            "name: called\n"
            "on:\n"
            "  workflow_call:\n"
            "env:\n"
            "  RUSTC_WRAPPER: ${{ inputs.wrapper }}\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: cargo test\n",
            "workflow env must not set RUSTC_WRAPPER",
        )
        flagged(
            ".github/workflows/second-key.yml",
            "name: second\n"
            "on: push\n"
            "env:\n"
            "  cargo_build_rustc_wrapper: /usr/bin/sccache\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: cargo test\n",
            "workflow env must not set cargo_build_rustc_wrapper",
        )
        flagged(
            ".github/workflows/sibling.yml",
            "name: sibling\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./.github/actions/rust-ci-setup\n"
            "  report:\n"
            "    runs-on: ubuntu-latest\n"
            "    env:\n"
            "      RUSTC_WRAPPER: sccache\n"
            "    steps:\n"
            "      - run: echo report\n",
            "job report: job env must not set RUSTC_WRAPPER",
        )
        flagged(
            ".github/workflows/step-env.yml",
            "name: step-env\n"
            "on: push\n"
            "jobs:\n"
            "  bench:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./.github/actions/rust-ci-setup\n"
            "      - name: Build\n"
            "        env:\n"
            "          RUSTC_WRAPPER: sccache\n"
            "        run: cargo bench\n",
            "RUSTC_WRAPPER=sccache if must be exactly",
        )
        flagged(
            ".github/workflows/step-key.yml",
            "name: step-key\n"
            "on: push\n"
            "jobs:\n"
            "  bench:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: Build\n"
            "        env:\n"
            "          rustc_wrapper: /opt/cache\n"
            "        run: cargo bench\n",
            "RUSTC_WRAPPER=sccache if must be exactly",
        )
        flagged(
            ".github/workflows/env-write.yml",
            "name: env-write\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: |\n"
            '          echo "rustc_wrapper=${{ env.WRAP }}" >> "$GITHUB_ENV"\n',
            "RUSTC_WRAPPER=sccache if must be exactly",
        )
        flagged(
            ".github/workflows/release-action.yml",
            "name: release\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: Wrap\n"
            "        run: |\n"
            '          echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n',
            "RUSTC_WRAPPER=sccache if must be exactly",
        )
        flagged(
            ".github/actions/custom/action.yaml",
            "name: custom\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - shell: bash\n"
            "      run: |\n"
            '        echo "CARGO_BUILD_RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n',
            "RUSTC_WRAPPER=sccache if must be exactly",
        )
        flagged(
            ".github/workflows/expr-uses.yml",
            "name: expr-uses\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ${{ matrix.action }}\n",
            "uses must be a fixed action reference",
        )
        flagged(
            ".github/workflows/broken.yml",
            "jobs: [\n",
            "workflow YAML did not parse",
        )
        flagged(
            ".github/workflows/list.yml",
            "[]\n",
            "workflow YAML did not parse",
        )
        flagged(
            ".github/workflows/not-listed.yml",
            "name: extra\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: mozilla-actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + "\n"
            "        with:\n"
            "          version: v0.19.0\n",
            "unexpected sccache-action use",
        )
        flagged(
            ".github/workflows/action.yml",
            "name: not-a-composite\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: mozilla-actions/sccache-action@"
            + SCCACHE_ACTION_COMMIT
            + "\n"
            "        with:\n"
            "          version: v0.19.0\n",
            "unexpected sccache-action use",
        )
        flagged(
            ".github/workflows/uses-int.yml",
            "name: uses-int\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: 1\n",
            "uses must be a fixed action reference",
        )

        def run_job(script: str) -> str:
            body = "".join(f"          {line}\n" for line in script.splitlines())
            return (
                "name: run-line\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - run: |\n" + body
            )

        for name, script in (
            ("brace-env", 'echo "RUSTC_WRAPPER=sccache" >> "${GITHUB_ENV}"'),
            ("printf-args", "printf '%s\\n' 'RUSTC_WRAPPER=sccache' >> \"$GITHUB_ENV\""),
            ("echo-flag", 'echo -n "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"'),
            ("trailing-comment", 'echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV" # cache'),
            ("piped", 'echo "RUSTC_WRAPPER=sccache" | tee -a "$GITHUB_ENV"'),
            ("chained", 'echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV" && true'),
            (
                "heredoc",
                "cat >> \"$GITHUB_ENV\" <<'EOF'\nRUSTC_WRAPPER=sccache\nEOF",
            ),
            (
                "pwsh-append",
                'Add-Content -Path $env:GITHUB_ENV -Value "RUSTC_WRAPPER=sccache"',
            ),
            (
                "workspace-key",
                'echo "Rustc_Workspace_Wrapper=sccache" >> "${GITHUB_ENV}"',
            ),
        ):
            flagged(
                f".github/workflows/{name}.yml",
                run_job(script),
                "wrapper run line is not the gated form",
            )
        flagged(
            ".github/workflows/workspace-env.yml",
            "name: workspace-env\n"
            "on: push\n"
            "env:\n"
            "  rustc_workspace_wrapper: sccache\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: cargo test\n",
            "workflow env must not set rustc_workspace_wrapper",
        )
        flagged(
            ".github/workflows/container-key.yml",
            "name: container-key\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    container:\n"
            "      image: ubuntu:22.04\n"
            "      env:\n"
            "        RUSTC_WRAPPER: sccache\n"
            "    steps:\n"
            "      - run: cargo test\n",
            "container env must not set RUSTC_WRAPPER",
        )
        flagged(
            ".github/workflows/workflow-env-expr.yml",
            "name: workflow-env-expr\n"
            "on: push\n"
            "env: ${{ fromJSON(vars.ENV) }}\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "workflow env must be a mapping",
        )
        flagged(
            ".github/workflows/job-env-expr.yml",
            "name: job-env-expr\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    env: ${{ fromJSON(vars.ENV) }}\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "job env must be a mapping",
        )
        flagged(
            ".github/workflows/step-env-expr.yml",
            "name: step-env-expr\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - env: ${{ fromJSON(vars.ENV) }}\n"
            "        run: echo ok\n",
            "step env must be a mapping",
        )
        flagged(
            ".github/workflows/container-env-expr.yml",
            "name: container-env-expr\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    container:\n"
            "      image: ubuntu:22.04\n"
            "      env: ${{ fromJSON(vars.ENV) }}\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "container env must be a mapping",
        )
        flagged(
            ".github/actions/expr-env/action.yml",
            "name: expr-env\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - shell: bash\n"
            "      env: ${{ fromJSON(inputs.env) }}\n"
            "      run: echo ok\n",
            "step env must be a mapping",
        )
        flagged(
            ".github/workflows/missing-prefetch-id.yml",
            "name: missing-id\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./.github/actions/rust-ci-setup\n"
            "      - name: stats\n"
            "        if: always() && steps.rust_ci.outputs.sccache-available == 'true'\n"
            "        run: sccache --show-stats\n",
            "gated if references missing step id 'rust_ci'",
        )
        flagged(
            ".github/workflows/arbitrary-wrapper.yml",
            "name: arbitrary\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - id: sccache_fetch\n"
            "        run: bash scripts/ci/fetch_sccache_release.sh\n"
            "      - name: Enable\n"
            "        if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        env:\n"
            "          RUSTC_WRAPPER: /tmp/missing\n"
            "        run: cargo test\n",
            "gated wrapper value must be sccache",
        )

        def quiet(path: str, text: str) -> None:
            copied = dict(texts)
            copied[path] = text
            found = [
                item for item in sccache_install_violations(copied) if item.startswith(path)
            ]
            self.assertEqual(found, [], found)

        prefetch = (
            "      - id: sccache_fetch\n"
            "        shell: bash\n"
            "        run: bash scripts/ci/fetch_sccache_release.sh\n"
        )
        for key in (
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
            "build.rustc-wrapper",
            "build.rustc-workspace-wrapper",
        ):
            flagged(
                f".github/workflows/wf-env-{key}.yml",
                "name: k\n"
                "on: push\n"
                "env:\n"
                f"  {key}: sccache\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - run: echo ok\n",
                f"workflow env must not set {key}",
            )
            flagged(
                f".github/workflows/job-env-{key}.yml",
                "name: k\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    env:\n"
                f"      {key}: sccache\n"
                "    steps:\n"
                "      - run: echo ok\n",
                f"job env must not set {key}",
            )
            flagged(
                f".github/workflows/step-env-{key}.yml",
                "name: k\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - env:\n"
                f"          {key}: sccache\n"
                "        run: echo ok\n",
                "RUSTC_WRAPPER=sccache if must be exactly",
            )
            flagged(
                f".github/workflows/container-env-{key}.yml",
                "name: k\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    container:\n"
                "      image: ubuntu:22.04\n"
                "      env:\n"
                f"        {key}: sccache\n"
                "    steps:\n"
                "      - run: echo ok\n",
                f"container env must not set {key}",
            )
            script = (
                f"cargo --config {key}=sccache"
                if "." in key
                else f'echo "{key}=sccache"'
            )
            flagged(
                f".github/workflows/run-key-{key}.yml",
                "name: k\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                f"      - run: {script}\n",
                "wrapper run line is not the gated form",
            )
        flagged(
            ".github/workflows/with-export.yml",
            "name: with-export\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: actions/github-script@v7\n"
            "        with:\n"
            '          script: "core.exportVariable(\'RUSTC_WRAPPER\', \'sccache\')"\n',
            "with: names a wrapper key",
        )
        flagged(
            ".github/workflows/with-export-gated.yml",
            "name: with-export-gated\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - uses: actions/github-script@v7\n"
            "        if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        with:\n"
            "          script: \"core.exportVariable('CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER', 'sccache')\"\n",
            "with: names a wrapper key",
        )
        flagged(
            ".github/workflows/container-expr.yml",
            "name: container-expr\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            '    container: "${{ fromJSON(vars.CONTAINER) }}"\n'
            "    steps:\n"
            "      - run: echo ok\n",
            "container expression is not checked",
        )
        quiet(
            ".github/workflows/container-image.yml",
            "name: container-image\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    container: ubuntu:22.04\n"
            "    steps:\n"
            "      - run: echo ok\n",
        )
        flagged(
            ".github/workflows/padded-wrapper.yml",
            "name: padded\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        env:\n"
            '          RUSTC_WRAPPER: " sccache"\n'
            "        run: cargo test\n",
            "gated wrapper value must be sccache",
        )
        flagged(
            ".github/workflows/prefixed-wrapper.yml",
            "name: prefixed\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        env:\n"
            "          RUSTC_WRAPPER: sccache-bin\n"
            "        run: cargo test\n",
            "gated wrapper value must be sccache",
        )
        flagged(
            ".github/workflows/no-step-ref.yml",
            "name: no-step-ref\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - if: sccache_fetch.outputs.available == 'true'\n"
            "        run: echo ok\n",
            "gated if does not reference a step id",
        )
        flagged(
            ".github/workflows/missing-sccache-fetch.yml",
            "name: missing-fetch\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        run: echo ok\n",
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/later-fetch.yml",
            "name: later-fetch\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        run: echo ok\n"
            + prefetch,
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/forged-fetch.yml",
            "name: forged-fetch\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - id: sccache_fetch\n"
            '        run: echo "available=true" >> "$GITHUB_OUTPUT"\n'
            "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            '        run: echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n',
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/bracket-fetch.yml",
            "name: bracket-fetch\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - id: sccache_fetch\n"
            '        run: echo "available=true" >> "$GITHUB_OUTPUT"\n'
            "      - if: \"steps['sccache_fetch'].outputs.available == 'true'\"\n"
            "        run: echo ok\n",
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/mixed-case-fetch.yml",
            "name: mixed-case-fetch\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - if: steps.SCCACHE_FETCH.outputs.available == 'true'\n"
            "        run: echo ok\n",
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/wrong-rust-ci.yml",
            "name: wrong-rust-ci\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - id: rust_ci\n"
            "        uses: actions/checkout@v7\n"
            "      - if: steps.rust_ci.outputs.sccache-available == 'true'\n"
            "        run: echo ok\n",
            "rust_ci must be an earlier rust-ci-setup step",
        )
        quiet(
            ".github/workflows/bracket-ok.yml",
            "name: bracket-ok\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: \"steps['sccache_fetch'].outputs.available == 'true'\"\n"
            "        run: echo ok\n",
        )
        flagged(
            ".github/workflows/unsanctioned-gate.yml",
            "name: unsanctioned-gate\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        run: 'echo \"RUSTC_WRAPPER=sccache\" >> \"$GITHUB_ENV\" # cache'\n",
            "wrapper run line is not the gated form",
        )
        flagged(
            ".github/workflows/unsanctioned-fallback.yml",
            "name: unsanctioned-fallback\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: steps.sccache_fetch.outputs.available != 'true'\n"
            "        run: 'echo \"RUSTC_WRAPPER=keep\" >> \"$GITHUB_ENV\" # cache'\n",
            "wrapper run line is not the gated form",
        )
        quiet(
            ".github/workflows/longer-token.yml",
            "name: longer-token\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo NOT_RUSTC_WRAPPER=1\n"
            "      - run: echo RUSTC_WRAPPER_EXTRA=1\n",
        )
        flagged(
            ".github/workflows/stats-short.yml",
            "name: stats-short\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - if: always()\n"
            "        run: sccache -s\n",
            "sccache --show-stats if must be an exact availability check",
        )
        flagged(
            ".github/workflows/stats-spaces.yml",
            "name: stats-spaces\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            + prefetch
            + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
            "        run: sccache  --show-stats\n",
            "sccache stats run must be exactly",
        )

        renamed = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "      id: sccache_fetch\n",
            "      id: other_fetch\n",
        )
        renamed_violations = sccache_install_violations(renamed)
        self.assertTrue(
            any("missing id sccache_fetch" in item for item in renamed_violations),
            renamed_violations,
        )

        widened = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "      if: steps.sccache_fetch.outputs.available != 'true'\n",
            "      if: steps.sccache_fetch.outputs.available != 'true' || false\n",
        )
        widened_violations = sccache_install_violations(widened)
        self.assertTrue(
            any("fallback if must be the failed prefetch" in item for item in widened_violations),
            widened_violations,
        )

        quoted_continue = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      uses: mozilla-actions/sccache-action@",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            '      continue-on-error: "true"\n'
            "      uses: mozilla-actions/sccache-action@",
        )
        quoted_violations = sccache_install_violations(quoted_continue)
        self.assertTrue(
            any("continue-on-error is set" in item for item in quoted_violations),
            quoted_violations,
        )

        expr_continue = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      uses: mozilla-actions/sccache-action@",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      continue-on-error: ${{ true }}\n"
            "      uses: mozilla-actions/sccache-action@",
        )
        expr_violations = sccache_install_violations(expr_continue)
        self.assertTrue(
            any("continue-on-error is set" in item for item in expr_violations),
            expr_violations,
        )

        numeric_continue = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      uses: mozilla-actions/sccache-action@",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      continue-on-error: 1\n"
            "      uses: mozilla-actions/sccache-action@",
        )
        numeric_violations = sccache_install_violations(numeric_continue)
        self.assertTrue(
            any("continue-on-error is set" in item for item in numeric_violations),
            numeric_violations,
        )

        mapping_continue = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      uses: mozilla-actions/sccache-action@",
            "    - name: Install sccache\n"
            "      if: steps.sccache_fetch.outputs.available == 'true'\n"
            "      continue-on-error:\n"
            "        x: true\n"
            "      uses: mozilla-actions/sccache-action@",
        )
        mapping_violations = sccache_install_violations(mapping_continue)
        self.assertTrue(
            any("continue-on-error is set" in item for item in mapping_violations),
            mapping_violations,
        )

        widened_stats = _replace_policy_once(
            texts,
            ".github/workflows/ci.yml",
            "        if: always() && needs.changes.outputs.docs_only != 'true' && steps.rust_ci.outputs.sccache-available == 'true'\n",
            "        if: always() && needs.changes.outputs.docs_only != 'true' && steps.rust_ci.outputs.sccache-available == 'true' || success()\n",
        )
        widened_stats_violations = sccache_install_violations(widened_stats)
        self.assertTrue(
            any(
                "sccache --show-stats if must be an exact availability check" in item
                for item in widened_stats_violations
            ),
            widened_stats_violations,
        )

        continued_stats = _replace_policy_once(
            texts,
            ".github/workflows/ci.yml",
            "      - name: sccache stats\n"
            "        if: always() && needs.changes.outputs.docs_only != 'true' && steps.rust_ci.outputs.sccache-available == 'true'\n"
            "        shell: bash\n"
            "        run: sccache --show-stats\n",
            "      - name: sccache stats\n"
            "        if: always() && needs.changes.outputs.docs_only != 'true' && steps.rust_ci.outputs.sccache-available == 'true' || success()\n"
            "        shell: bash\n"
            "        run: |\n"
            "          sccache \\\n"
            "            --show-stats\n",
        )
        continued_violations = sccache_install_violations(continued_stats)
        self.assertTrue(
            any(
                "sccache --show-stats if must be an exact availability check" in item
                for item in continued_violations
            ),
            continued_violations,
        )

        ignored_stats = _replace_policy_once(
            texts,
            ".github/workflows/ci.yml",
            "        run: sccache --show-stats\n",
            "        run: sccache --show-stats || true\n",
        )
        ignored_stats_violations = sccache_install_violations(ignored_stats)
        self.assertTrue(
            any("sccache stats run must be exactly" in item for item in ignored_stats_violations),
            ignored_stats_violations,
        )

        workflow_probe = WORKFLOWS / "policy-probe.yaml"
        action_probe = ROOT / ".github" / "actions" / "policy-probe"
        workflow_probe.write_text(
            "name: probe\n"
            "on: push\n"
            "env:\n"
            "  RUSTC_WRAPPER: probe\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            encoding="utf-8",
        )
        action_probe.mkdir(parents=True)
        (action_probe / "action.yaml").write_text(
            "name: probe\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - shell: bash\n"
            "      run: |\n"
            '        echo "RUSTC_WRAPPER=probe" >> "$GITHUB_ENV"\n',
            encoding="utf-8",
        )
        try:
            loaded = sccache_policy_texts()
            self.assertIn(".github/workflows/policy-probe.yaml", loaded)
            self.assertIn(".github/actions/policy-probe/action.yaml", loaded)
            probe_violations = sccache_install_violations(loaded)
            self.assertTrue(
                any(".github/workflows/policy-probe.yaml" in item for item in probe_violations),
                probe_violations,
            )
            self.assertTrue(
                any(".github/actions/policy-probe/action.yaml" in item for item in probe_violations),
                probe_violations,
            )
        finally:
            workflow_probe.unlink(missing_ok=True)
            shutil.rmtree(action_probe, ignore_errors=True)

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
                            body, code = b"not-the-archive", 200
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
            _write_exe(work / "bin", "sleep", "#!/bin/sh\nexit 0\n")
            with _fixed_body_server(b"no", status=504) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    attempts="2",
                    delay="08",
                    path_prefix=work / "bin",
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
            _write_exe(work / "bin", "node", "#!/bin/sh\nexit 1\n")
            result, output = _run_fetch(work, path_prefix=work / "bin")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("could not read the runner arch", result.stderr)

    def test_sccache_prefetch_falls_back_when_mktemp_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            _write_exe(work / "bin", "mktemp", "#!/bin/sh\nexit 1\n")
            result, output = _run_fetch(work, path_prefix=work / "bin")
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
            _write_exe(work / "bin", "node", node)
            with _fixed_body_server(b"not-the-archive", status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "bin",
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
            _write_exe(work / "bin", "node", node)
            with _fixed_body_server(b"archive-bytes", status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "bin",
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
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "bin",
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
            _write_rm_that_refuses_tree_wipe(work / "bin")
            result, output = _run_fetch(
                work,
                base="http://127.0.0.1:1/releases/download/v0.19.0",
                attempts="1",
                path_prefix=work / "bin",
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
            _write_rm_that_refuses_tree_wipe(work / "bin")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "bin",
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
                if kind == "symlink":
                    self.assertIn("archive has a symlink or non-regular entry", result.stderr)
                else:
                    self.assertIn("archive has no sccache binary", result.stderr)
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
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    path_prefix=work / "bin",
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
        self.assertIn('if plain_directory "$candidate"; then', script_text)
        plain_binary = script_text.split("plain_binary() {", 1)[1].split("plain_directory() {", 1)[0]
        self.assertIn("fs.lstatSync", plain_binary)
        self.assertNotIn("fs.statSync", plain_binary)
        branch = script_text.split('if [ "${#want}" -ne 64 ]', 1)[1].split("\nfi", 1)[0]
        self.assertLess(branch.find("trap - ERR"), branch.find("builtin printf"))
        self.assertNotIn("|| true", branch)
        ending = [line for line in script_text.splitlines() if line.strip()][-2:]
        self.assertEqual(ending[0], 'echo "sccache ${VERSION} cached at ${dest}" || true')
        self.assertEqual(ending[1], 'write_output "available=true"')
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
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        path_prefix=work / "bin",
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
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output, "available=true\n")
            self.assertNotIn("available=false", output)
            self.assertTrue(cached_is_file)
            self.assertFalse(cached_is_link)
            self.assertTrue(marker_is_file)
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
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        path_prefix=work / "bin",
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
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        path_prefix=work / "bin",
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


    def test_sccache_policy_gap_mutants(self) -> None:
        texts = sccache_policy_texts()
        self.assertIn(".cargo/config.toml", texts)
        self.assertIn("examples/fluids_demo/.cargo/config.toml", texts)

        def flagged(path: str, text: str, needle: str) -> None:
            found = sccache_install_violations({path: text})
            self.assertTrue(any(needle in item for item in found), found)

        def quiet(path: str, text: str) -> None:
            found = [
                item
                for item in sccache_install_violations({path: text})
                if item.startswith(path) or item.startswith("tools/")
            ]
            self.assertEqual(found, [], found)

        fetch = (
            "      - name: Fetch\n"
            "        id: sccache_fetch\n"
            "        shell: bash\n"
            "        run: bash scripts/ci/fetch_sccache_release.sh\n"
        )

        def job(body: str, job_header: str = "") -> str:
            return (
                "name: gap\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                f"{job_header}"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                + body
            )

        for spelling in (
            'build.rustc-wrapper = "sccache"',
            'build.rustc-workspace-wrapper = "sccache"',
            '[build]\nrustc-wrapper = "sccache"',
            '[build]\nrustc-workspace-wrapper = "sccache"',
            'build = { rustc-wrapper = "sccache" }',
            '"rustc-wrapper" = "sccache"',
        ):
            flagged(
                ".cargo/config.toml",
                spelling + "\n",
                "cargo config sets a wrapper key",
            )
            flagged(
                ".github/workflows/toml-write.yml",
                job("      - run: |\n" + "".join(f"          {line}\n" for line in spelling.splitlines())),
                "wrapper run line is not the gated form",
            )

        flagged(
            ".github/workflows/local-missing.yml",
            job("      - uses: ./tools/missing_policy_probe\n"),
            "local action could not be read",
        )
        probe = ROOT / "tools" / "policy_probe_local"
        probe.mkdir(parents=True)
        (probe / "action.yml").write_text(
            "name: probe\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - shell: bash\n"
            "      run: echo RUSTC_WRAPPER=probe\n",
            encoding="utf-8",
        )
        try:
            flagged(
                ".github/workflows/local-uses.yml",
                job("      - uses: ./tools/policy_probe_local\n"),
                "wrapper run line is not the gated form",
            )
        finally:
            shutil.rmtree(probe)

        flagged(
            ".github/workflows/with-built.yml",
            job(
                "      - uses: actions/github-script@v7\n"
                "        with:\n"
                "          script: \"core.exportVariable('RUSTC_' + 'WRAPPER', 'x')\"\n"
            ),
            "with: names a wrapper key",
        )

        for label, run in (
            ("path", "/usr/bin/sccache --show-stats"),
            ("assign", "FOO=1 sccache --show-stats"),
            ("env", "env sccache --show-stats"),
            ("command", "command sccache --show-stats"),
            ("exec", "exec sccache --show-stats"),
            ("sudo", "sudo sccache --show-stats"),
            ("xargs", "xargs sccache --show-stats"),
            ("keyword", "then sccache --show-stats"),
        ):
            flagged(
                f".github/workflows/invoke-{label}.yml",
                job(
                    fetch
                    + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                    + f"        run: {run}\n"
                ),
                "sccache stats run must be exactly",
            )

        quiet(
            ".github/workflows/joined-stats.yml",
            job(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        shell: bash\n"
                + '        run: "sccache \\\n--show-stats"\n'
            ),
        )
        for label, run in (
            ("and", "echo ok && sccache --show-stats"),
            ("or", "echo ok || sccache --show-stats"),
            ("semi", "echo ok; sccache --show-stats"),
            ("pipe", "echo ok | sccache --show-stats"),
        ):
            flagged(
                f".github/workflows/sep-{label}.yml",
                job(
                    fetch
                    + "      - if: always() && steps.sccache_fetch.outputs.available == 'true'\n"
                    + f"        run: {run}\n"
                ),
                "sccache stats run must be exactly",
            )

        flagged(
            ".github/workflows/prefetch-extra.yml",
            job(
                "      - id: sccache_fetch\n"
                "        shell: bash\n"
                "        run: bash scripts/ci/fetch_sccache_release.sh --extra\n"
                "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                "        run: echo ok\n"
            ),
            "sccache_fetch must be an earlier pinned prefetch",
        )
        for field, extra in (
            ("env", "        env:\n          FOO: bar\n"),
            ("working-directory", "        working-directory: decoy\n"),
            ("if", "        if: success()\n"),
            ("with", "        with:\n          script: echo ok\n"),
        ):
            flagged(
                f".github/workflows/prefetch-{field}.yml",
                job(
                    "      - id: sccache_fetch\n"
                    "        shell: bash\n"
                    + extra
                    + "        run: bash scripts/ci/fetch_sccache_release.sh\n"
                ),
                "prefetch step has unexpected fields",
            )
        flagged(
            ".github/workflows/prefetch-continue.yml",
            job(
                "      - id: sccache_fetch\n"
                "        shell: bash\n"
                "        continue-on-error: true\n"
                "        run: bash scripts/ci/fetch_sccache_release.sh\n"
            ),
            "continue-on-error is set on the prefetch",
        )
        flagged(
            ".github/workflows/prefetch-pwsh.yml",
            job(
                "      - id: sccache_fetch\n"
                "        shell: pwsh\n"
                "        run: bash scripts/ci/fetch_sccache_release.sh\n"
            ),
            "prefetch shell must be bash",
        )
        flagged(
            ".github/workflows/prefetch-shell-template.yml",
            job(
                "      - id: sccache_fetch\n"
                "        shell: bash -c 'echo available=true >> \"$GITHUB_OUTPUT\"; exit 0' {0}\n"
                "        run: bash scripts/ci/fetch_sccache_release.sh\n"
            ),
            "prefetch shell must be bash",
        )
        flagged(
            ".github/workflows/prefetch-defaults-dir.yml",
            (
                "name: gap\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    defaults:\n"
                "      run:\n"
                "        working-directory: decoy\n"
                "    steps:\n"
                + fetch
            ),
            "prefetch working directory is not the default",
        )
        flagged(
            ".github/workflows/prefetch-rewrite.yml",
            job(
                "      - run: echo replaced > scripts/ci/fetch_sccache_release.sh\n"
                + fetch
            ),
            "prefetch script is rewritten by an earlier step",
        )
        flagged(
            ".github/workflows/prefetch-path.yml",
            job(
                "      - run: echo /tmp/decoy >> \"$GITHUB_PATH\"\n"
                + fetch
            ),
            "prefetch PATH is changed",
        )

        flagged(
            ".github/workflows/rust-ci-case.yml",
            job(
                "      - id: Rust_CI\n"
                "        uses: actions/checkout@v7\n"
                "      - if: steps.Rust_CI.outputs.sccache-available == 'true'\n"
                "        run: echo ok\n"
            ),
            "rust_ci must be an earlier rust-ci-setup step",
        )
        flagged(
            ".github/workflows/rust-ci-prefix.yml",
            job(
                "      - id: rust_ci\n"
                "        uses: ./.github/actions/rust-ci-setup-extra\n"
                "      - if: steps.rust_ci.outputs.sccache-available == 'true'\n"
                "        run: echo ok\n"
            ),
            "rust_ci must be an earlier rust-ci-setup step",
        )
        flagged(
            ".github/workflows/other-id.yml",
            job(
                "      - if: steps.other_step.outputs.sccache-available == 'true'\n"
                "        run: echo ok\n"
            ),
            "gated if references missing step id 'other_step'",
        )
        flagged(
            ".github/workflows/wrapper-continue.yml",
            job(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        shell: bash\n"
                + "        continue-on-error: true\n"
                + "        run: echo \"RUSTC_WRAPPER=sccache\" >> \"$GITHUB_ENV\"\n"
            ),
            "continue-on-error is set on the RUSTC_WRAPPER step",
        )
        flagged(
            ".github/workflows/stats-continue.yml",
            job(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        continue-on-error: true\n"
                + "        run: sccache --show-stats\n"
            ),
            "continue-on-error is set on sccache --show-stats",
        )
        flagged(
            ".github/workflows/job-continue.yml",
            job(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        run: sccache --show-stats\n",
                job_header="    continue-on-error: true\n",
            ),
            "continue-on-error is set on a job that runs sccache",
        )

        for label, snippet in (
            ("step-name", "      - name: RUSTC_WRAPPER\n        run: echo ok\n"),
            ("step-id", "      - id: RUSTC_WRAPPER\n        run: echo ok\n"),
            ("step-shell", "      - shell: RUSTC_WRAPPER\n        run: echo ok\n"),
            ("step-dir", "      - working-directory: RUSTC_WRAPPER\n        run: echo ok\n"),
            ("step-uses", "      - uses: RUSTC_WRAPPER\n"),
            ("step-if", "      - if: contains(RUSTC_WRAPPER, 'x')\n        run: echo ok\n"),
            ("step-timeout", "      - timeout-minutes: RUSTC_WRAPPER\n        run: echo ok\n"),
            (
                "step-env-value",
                "      - env:\n          FOO: RUSTC_WRAPPER\n        run: echo ok\n",
            ),
            (
                "job-name",
                None,
            ),
        ):
            if snippet is None:
                continue
            flagged(
                f".github/workflows/field-{label}.yml",
                job(snippet),
                "wrapper key outside a sanctioned line",
            )
        flagged(
            ".github/workflows/field-job-name.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    name: RUSTC_WRAPPER\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-runs-on.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: RUSTC_WRAPPER\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-job-if.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    if: RUSTC_WRAPPER == 'x'\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-environment.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    environment: RUSTC_WRAPPER\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-defaults.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    defaults:\n"
            "      run:\n"
            "        shell: RUSTC_WRAPPER\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-service.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    services:\n"
            "      db:\n"
            "        image: RUSTC_WRAPPER\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/field-workflow-name.yml",
            "name: RUSTC_WRAPPER\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "wrapper key outside a sanctioned line",
        )
        flagged(
            ".github/workflows/container-image-expr.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    container:\n"
            "      image: ${{ inputs.image }}\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "container expression is not checked",
        )
        flagged(
            ".github/workflows/container-options-expr.yml",
            "name: gap\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    container:\n"
            "      image: ubuntu:22.04\n"
            "      options: ${{ inputs.options }}\n"
            "    steps:\n"
            "      - run: echo ok\n",
            "container expression is not checked",
        )
        cycle = sccache_install_violations(
            {
                ".github/workflows/cycle.yml": (
                    "name: cycle\n"
                    "on: push\n"
                    "jobs:\n"
                    "  build:\n"
                    "    runs-on: ubuntu-latest\n"
                    "    steps:\n"
                    "      - uses: actions/checkout@v7\n"
                    "        with: &cycle\n"
                    "          script: *cycle\n"
                )
            }
        )
        self.assertTrue(any("cyclic anchor" in item for item in cycle), cycle)

        flagged(
            ".github/workflows/release-flag.yml",
            job("      - run: bash scripts/ci/fetch_sccache_release.sh --release-base http://example.invalid\n"),
            "release URL override is test-only",
        )

        commented = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            'echo "CARGO_INCREMENTAL=0" >> "$GITHUB_ENV"',
            '|\n        # echo "CARGO_INCREMENTAL=0" >> "$GITHUB_ENV"',
        )
        commented_found = sccache_install_violations(commented)
        self.assertTrue(
            any("CARGO_INCREMENTAL=0 must be set" in item for item in commented_found),
            commented_found,
        )

        script = (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh").read_text(encoding="utf-8")
        early = script.replace(
            "set -eEuo pipefail\n",
            'set -eEuo pipefail\nwrite_output "available=true"\nexit 0\n',
            1,
        )
        early_found = sccache_checksum_source_violations(early)
        self.assertTrue(
            any("before the checksum" in item for item in early_found),
            early_found,
        )

        dropped_shell = _replace_policy_once(
            texts,
            ".github/actions/rust-ci-setup/action.yml",
            "      shell: bash\n      env:\n        AW_GITHUB_REF:",
            "      env:\n        AW_GITHUB_REF:",
        )
        shell_found = sccache_install_violations(dropped_shell)
        self.assertTrue(
            any("must run under bash" in item for item in shell_found),
            shell_found,
        )

    def test_prefetch_ignores_release_base_environment(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            record = work / "urls"
            _write_exe(
                work / "bin",
                "curl",
                "#!/bin/sh\n"
                "last=\n"
                "for arg in \"$@\"; do last=$arg; done\n"
                f"printf '%s\\n' \"$last\" >> '{record}'\n"
                "exit 1\n",
            )
            env = os.environ.copy()
            env["PATH"] = f"{work / 'bin'}{os.pathsep}{env.get('PATH', '')}"
            env["GITHUB_OUTPUT"] = str(work / "output")
            env["GITHUB_ENV"] = str(work / "env")
            env["RUNNER_TOOL_CACHE"] = str(work / "tool-cache")
            env["SCCACHE_FETCH_ATTEMPTS"] = "1"
            env["SCCACHE_FETCH_DELAY_SECONDS"] = "0"
            env["SCCACHE_RELEASE_BASE"] = "http://127.0.0.1:9/from-env"
            (work / "env").write_text("", encoding="utf-8")
            result = subprocess.run(
                ["bash", str(ROOT / "scripts" / "ci" / "fetch_sccache_release.sh")],
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )
            seen = record.read_text(encoding="utf-8") if record.is_file() else ""
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("127.0.0.1:9", seen)
            self.assertIn("github.com/mozilla/sccache/releases/download/v0.19.0/", seen)

    def test_sccache_policy_closed_gaps(self) -> None:
        """Killer cases for escaped keys, unread configs, fail-closed stats, and prefetch context."""

        def flagged(path: str, text: str, needle: str, texts: dict[str, str] | None = None) -> None:
            body = dict(texts or {})
            body[path] = text
            found = sccache_install_violations(body)
            self.assertTrue(any(needle in item for item in found), found)

        def quiet(path: str, text: str, texts: dict[str, str] | None = None) -> None:
            body = dict(texts or {})
            body[path] = text
            found = [
                item
                for item in sccache_install_violations(body)
                if item.startswith(path) or item.startswith("ci/") or item.startswith("tools/")
            ]
            self.assertEqual(found, [], found)

        def workflow(steps: str, job_header: str = "") -> str:
            return (
                "name: gap\n"
                "on: push\n"
                "jobs:\n"
                "  build:\n"
                f"{job_header}"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                + steps
            )

        fetch = (
            "      - id: sccache_fetch\n"
            "        shell: bash\n"
            "        run: bash scripts/ci/fetch_sccache_release.sh\n"
        )
        clean = (
            "      - id: sccache_fetch\n"
            "        shell: bash\n"
            "        run: env -i PATH=\"$PATH\" bash --noprofile --norc "
            "scripts/ci/fetch_sccache_release.sh\n"
        )
        rooted = (
            "      - id: sccache_fetch\n"
            "        shell: bash\n"
            "        run: env -i PATH=\"$PATH\" -- bash --noprofile --norc "
            "scripts/ci/fetch_sccache_release.sh /opt/sccache-root\n"
        )
        for label, step in (("clean-env", clean), ("release-root", rooted), ("plain", fetch)):
            quiet(f".github/workflows/prefetch-{label}.yml", workflow(step))

        flagged(
            ".github/workflows/prefetch-two-args.yml",
            workflow(
                "      - id: sccache_fetch\n"
                "        shell: bash\n"
                "        run: env -i PATH=\"$PATH\" bash --noprofile --norc "
                "scripts/ci/fetch_sccache_release.sh /opt/root extra\n"
            ),
            "sccache_fetch must be an earlier pinned prefetch",
        )
        flagged(
            ".github/workflows/prefetch-wrapper-env.yml",
            workflow(
                "      - id: sccache_fetch\n"
                "        shell: bash\n"
                "        run: env -i RUSTC_WRAPPER=sccache bash --noprofile --norc "
                "scripts/ci/fetch_sccache_release.sh\n"
            ),
            "sccache_fetch must be an earlier pinned prefetch",
        )

        flagged(
            ".cargo/config.toml",
            '[build]\n"rustc\\u002dwrapper" = "sccache"\n',
            "cargo config sets a wrapper key",
        )
        flagged(
            ".github/workflows/escaped-env.yml",
            workflow('      - run: echo "RUSTC_\\WRAPPER=sccache" >> "$GITHUB_ENV"\n'),
            "wrapper run line is not the gated form",
        )
        flagged(
            ".github/workflows/ansi-c-env.yml",
            workflow("      - run: |\n          echo $'RUSTC_WRAPPER=sccache' >> \"$GITHUB_ENV\"\n"),
            "wrapper run line is not the gated form",
        )
        flagged(
            ".github/workflows/escaped-config-write.yml",
            workflow(
                "      - run: printf '%s\\n' '[build]' '\"rustc\\u002dwrapper\" = \"sccache\"' "
                "> .cargo/config.toml\n"
            ),
            "wrapper run line is not the gated form",
        )
        flagged(
            ".github/workflows/format-key.yml",
            workflow(
                "      - uses: actions/github-script@v7\n"
                "        with:\n"
                "          script: \"core.exportVariable(format('RUSTC_%s', 'WRAPPER'), 'x')\"\n"
            ),
            "with: names a wrapper key",
        )
        flagged(
            ".github/workflows/template-key.yml",
            workflow(
                "      - uses: actions/github-script@v7\n"
                "        with:\n"
                "          script: \"core.exportVariable(`RUSTC_${'WRAPPER'}`, 'x')\"\n"
            ),
            "with: names a wrapper key",
        )

        flagged(
            "ci/cargohome/config.toml",
            'rustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
            {
                ".github/workflows/cargo-home.yml": workflow(
                    "      - run: echo ok\n"
                ).replace(
                    "name: gap\n",
                    "name: gap\nenv:\n  CARGO_HOME: ci/cargohome\n",
                    1,
                ),
                "ci/cargohome/config.toml": 'rustc-wrapper = "sccache"\n',
            },
        )
        flagged(
            "ci/wrap.toml",
            'rustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
            {
                ".github/workflows/cargo-config-arg.yml": workflow(
                    "      - run: cargo build --config ci/wrap.toml\n"
                ),
                "ci/wrap.toml": 'rustc-wrapper = "sccache"\n',
            },
        )
        flagged(
            "tools/target/.cargo/config.toml",
            'rustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
        )
        flagged(
            ".cargo/config",
            'rustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
        )
        flagged(
            "ci/copied.toml",
            'rustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
            {
                ".github/workflows/copy-cargo.yml": workflow(
                    "      - run: cp ci/copied.toml .cargo/config.toml\n"
                ),
                "ci/copied.toml": 'rustc-wrapper = "sccache"\n',
            },
        )
        flagged(
            ".cargo/nested.toml",
            '[build]\nrustc-wrapper = "sccache"\n',
            "cargo config sets a wrapper key",
            {
                ".cargo/config.toml": 'include = "nested.toml"\n',
                ".cargo/nested.toml": '[build]\nrustc-wrapper = "sccache"\n',
            },
        )
        flagged(
            ".github/workflows/missing-config.yml",
            workflow("      - run: cargo build --config ci/missing-policy.toml\n"),
            "cargo config could not be read",
        )
        quiet(
            ".github/workflows/cmake-config.yml",
            workflow("      - run: cmake --build cmake_build --config Release\n"),
        )

        for label, run in (
            ("timeout", "timeout 10 sccache --show-stats"),
            ("nice", "nice sccache --show-stats"),
            ("env-u", "env -u FOO sccache --show-stats"),
            ("sudo-u", "sudo -u nobody sccache --show-stats"),
            ("bash-c", "bash -c 'sccache --show-stats'"),
            ("variable", 'cmd=sccache; "$cmd" --show-stats'),
        ):
            flagged(
                f".github/workflows/stats-{label}.yml",
                workflow(f"      - shell: bash\n        run: {run}\n"),
                "sccache stats run must be exactly",
            )
        flagged(
            ".github/workflows/stats-shell-template.yml",
            workflow(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        shell: bash -c 'echo skipped' {0}\n"
                + "        run: sccache --show-stats\n"
            ),
            "stats step must run under bash",
        )
        flagged(
            ".github/workflows/incremental-pwsh.yml",
            workflow(
                '      - shell: pwsh\n        run: echo "CARGO_INCREMENTAL=0" >> "$GITHUB_ENV"\n'
            ),
            "incremental step must run under bash",
        )

        flagged(
            ".github/workflows/second-prefetch-path.yml",
            workflow(
                fetch
                + '      - run: echo /tmp/decoy >> "$GITHUB_PATH"\n'
                + fetch
            ),
            "prefetch PATH is changed",
        )
        flagged(
            ".github/workflows/env-file-path.yml",
            workflow('      - run: echo "PATH=/tmp/decoy" >> "$GITHUB_ENV"\n' + fetch),
            "prefetch PATH is changed",
        )
        flagged(
            ".github/workflows/env-file-other.yml",
            workflow('      - run: echo "FOO=1" >> "$GITHUB_ENV"\n' + fetch),
            "prefetch env file is written by an earlier step",
        )
        flagged(
            ".github/workflows/caller-path.yml",
            workflow(
                '      - run: echo /tmp/decoy >> "$GITHUB_PATH"\n'
                "      - uses: ./.github/actions/rust-ci-setup\n"
            ),
            "prefetch PATH is changed",
        )
        flagged(
            ".github/workflows/caller-copy.yml",
            workflow(
                "      - run: cp tools/x.sh scripts/ci/decoy.sh\n"
                "      - uses: ./.github/actions/rust-ci-setup\n"
            ),
            "prefetch script is rewritten by an earlier step",
        )
        flagged(
            ".github/actions/rewritten-prefetch/action.yml",
            "name: rewritten\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - shell: bash\n"
            "      run: cp tools/x.sh scripts/ci/decoy.sh\n"
            "    - id: sccache_fetch\n"
            "      shell: bash\n"
            "      run: bash scripts/ci/fetch_sccache_release.sh\n",
            "prefetch script is rewritten by an earlier step",
        )
        flagged(
            ".github/workflows/add-path.yml",
            workflow(
                "      - uses: actions/github-script@v7\n"
                "        with:\n"
                "          script: core.addPath('/tmp/decoy')\n"
                + fetch
            ),
            "prefetch PATH is changed",
        )
        flagged(
            ".github/workflows/joined-rewrite.yml",
            workflow(
                '      - run: echo x > "scripts/ci/" "fetch_sccache_release.sh"\n' + fetch
            ),
            "prefetch script is rewritten by an earlier step",
        )
        flagged(
            ".github/workflows/job-env-extra.yml",
            workflow(fetch, job_header="    env:\n      FOO: bar\n"),
            "prefetch environment is not allowlisted",
        )
        for header in (
            "    continue-on-error: true\n",
            "    continue-on-error: ${{ true }}\n",
            '    continue-on-error: "true"\n',
        ):
            flagged(
                ".github/workflows/composite-continue.yml",
                workflow("      - uses: ./.github/actions/rust-ci-setup\n", job_header=header),
                "continue-on-error is set on a job that runs sccache",
            )
        flagged(
            ".github/workflows/second-wrapper.yml",
            workflow(
                fetch
                + "      - if: steps.sccache_fetch.outputs.available == 'true'\n"
                + "        shell: bash\n"
                + "        run: |\n"
                + '          echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n'
                + '          echo "RUSTC_WRAPPER=/tmp/other" >> "$GITHUB_ENV"\n'
            ),
            "wrapper run line is not the gated form",
        )

        quiet(
            ".github/workflows/reused-anchor.yml",
            "name: reuse\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    env: &shared\n"
            "      FOO: bar\n"
            "    steps:\n"
            "      - env: *shared\n"
            "        run: echo ok\n",
        )
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "target"
            target.mkdir()
            self.assertFalse(_is_cargo_target_dir(target))
            (target / "CACHEDIR.TAG").write_text("Signature: 8a477f597d28d172789f06886806bc55\n", encoding="utf-8")
            self.assertTrue(_is_cargo_target_dir(target))

        script = (ROOT / "scripts" / "ci" / "fetch_sccache_release.sh").read_text(encoding="utf-8")
        dead_if = script.replace(
            'if [ "${#want}" -ne 64 ] || [ "$got" != "$want" ]; then',
            "if false; then",
            1,
        )
        self.assertNotEqual(dead_if, script)
        dead_found = sccache_checksum_source_violations(dead_if)
        self.assertTrue(any("checksum mismatch must exit 1" in item for item in dead_found), dead_found)
        split = script.replace(
            "set -eEuo pipefail\n",
            'set -eEuo pipefail\nwrite_output "available=" "true"\n',
            1,
        )
        split_found = sccache_checksum_source_violations(split)
        self.assertTrue(any("before the checksum" in item for item in split_found), split_found)


if __name__ == "__main__":
    unittest.main()

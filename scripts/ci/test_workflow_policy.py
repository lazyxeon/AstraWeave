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


class _UniqueKeyLoader(yaml.SafeLoader):
    """Reject a repeated mapping key. The safe loader would keep the last one."""


def _construct_unique_mapping(loader: yaml.SafeLoader, node: yaml.Node, deep: bool = False) -> dict:
    mapping: dict = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise yaml.constructor.ConstructorError(
                "while constructing a mapping",
                node.start_mark,
                "found a duplicate key",
                key_node.start_mark,
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


_UniqueKeyLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _construct_unique_mapping,
)


def _load_policy_yaml(text: str) -> object:
    return yaml.load(text, Loader=_UniqueKeyLoader)  # noqa: S506

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
    mode: int = 0o755,
    mode_field: bytes | None = None,
) -> bytes:
    header = bytearray(512)
    header[0 : len(name)] = name
    if mode_field is None:
        header[100:108] = _octal_field(mode, 8)
    else:
        if len(mode_field) != 8:
            raise AssertionError(len(mode_field))
        header[100:108] = mode_field
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


def _fixture_node() -> str:
    """A node binary the shim can exec. The prefetch script does not search PATH."""
    for candidate in (
        "/opt/homebrew/bin/node",
        "/usr/local/bin/node",
        "/usr/bin/node",
    ):
        if os.access(candidate, os.X_OK):
            return candidate
    found = shutil.which("node")
    if not found:
        raise AssertionError("node fixture is missing")
    return found


def _bind_fetch_node(script: Path, replacement: Path) -> None:
    _bind_tool(script, _NODE_BIND, replacement)


def _bind_node_preload(script: Path, work: Path, preload: str) -> None:
    """Run the real node, with a preload on -e so a test can hook fs."""
    patch = work / "bin" / "preload.js"
    patch.parent.mkdir(parents=True, exist_ok=True)
    patch.write_text(preload, encoding="utf-8")
    real = shlex.quote(_fixture_node())
    quoted = shlex.quote(str(patch))
    _write_exe(
        work / "bin",
        "node",
        "#!/bin/sh\n"
        f"if [ \"$1\" = -e ]; then exec {real} -r {quoted} \"$@\"; fi\n"
        f"exec {real} \"$@\"\n",
    )
    _bind_fetch_node(script, work / "bin" / "node")


_FS_PLANT = r"""
const fs = require("fs");
const path = require("path");
const plant = process.env.AW_PLANT || "";
const outside = process.env.AW_OUTSIDE || "";
const log = process.env.AW_CHMOD_LOG || "";
if (log) {
  const origChmod = fs.chmodSync;
  fs.chmodSync = function (p, mode) {
    fs.appendFileSync(log, "chmod " + String(mode) + "\n");
    return origChmod.call(fs, p, mode);
  };
  const origClose = fs.closeSync;
  fs.closeSync = function (fd) {
    let mode = "";
    try { mode = String(fs.fstatSync(fd).mode & 511); } catch (e) { mode = "err"; }
    const result = origClose.call(fs, fd);
    fs.appendFileSync(log, "mode " + mode + "\n");
    return result;
  };
}
if (plant === "file" || plant === "symlink" || plant === "rootlink") {
  const origMk = fs.mkdirSync;
  fs.mkdirSync = function (p, opts) {
    const result = origMk.call(fs, p, opts);
    if (plant === "rootlink") {
      fs.rmSync(p, { recursive: true, force: true });
      fs.symlinkSync(outside, p);
    } else if (plant === "file") {
      fs.writeFileSync(path.join(p, "sccache"), "PLANTED");
    } else if (plant === "symlink") {
      fs.symlinkSync(outside, path.join(p, "sccache"));
    }
    return result;
  };
}
if (plant === "hardlink") {
  const origOpen = fs.openSync;
  let opened = "";
  fs.openSync = function (p, flags, mode) {
    opened = String(p);
    return origOpen.call(fs, p, flags, mode);
  };
  const origClose = fs.closeSync;
  fs.closeSync = function (fd) {
    const result = origClose.call(fs, fd);
    if (opened) {
      try { fs.linkSync(opened, opened + ".alias"); } catch (e) {}
    }
    return result;
  };
}
if (plant === "realpath") {
  const orig = fs.realpathSync;
  fs.realpathSync = function (p) {
    const resolved = orig.call(fs, p);
    if (path.basename(String(p)).startsWith("sccache-v")) return "/outside/not-contained";
    return resolved;
  };
}
"""


def _write_stage_planter(work: Path, plant: str) -> None:
    """Node shim that runs the real interpreter, then edits the staged tree."""
    real = shlex.quote(_fixture_node())
    shim = (
        "#!/bin/sh\n"
        f"real={real}\n"
        "mode=\n"
        "for arg in \"$@\"; do\n"
        "  case \"$arg\" in\n"
        "    stage) mode=stage ;;\n"
        "  esac\n"
        "done\n"
        "if [ \"$mode\" = stage ]; then\n"
        "  out=\"$(\"$real\" \"$@\")\" || exit $?\n"
        "  printf '%s\\n' \"$out\"\n"
        "  dest=$(printf '%s\\n' \"$out\" | /usr/bin/tail -n 1)\n"
        f"  {plant}\n"
        "  exit 0\n"
        "fi\n"
        "exec \"$real\" \"$@\"\n"
    )
    _write_exe(work / "bin", "node", shim)


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
    '/bin/bash --noprofile --norc "$GITHUB_WORKSPACE/scripts/ci/fetch_sccache_release.sh"; fi; '
    'exec /usr/bin/env -i HOME="$HOME" GITHUB_OUTPUT="$GITHUB_OUTPUT" '
    'RUNNER_TOOL_CACHE="$RUNNER_TOOL_CACHE" RUNNER_OS="$RUNNER_OS" '
    '/usr/bin/bash --noprofile --norc "$GITHUB_WORKSPACE/scripts/ci/fetch_sccache_release.sh"'
)
_FETCH_RUN_LINE = f"run: {SCCACHE_FETCH_COMMAND}"
# Exact run lines that may name scripts/ci. Anything else that names that
# tree, or that executes by a path the scanner cannot classify, is refused.
_ALLOWED_SCRIPTS_CI_LINES = frozenset(
    {
        _FETCH_RUN_LINE,
        "run: bash scripts/ci/gpu_frame_budget.sh",
        'run: bash scripts/ci/deepen_for_merge_base.sh "${{ github.base_ref }}"',
        'python3 scripts/ci/affected_crates.py --base "HEAD^1" --head HEAD --github-output "$GITHUB_OUTPUT"',
        'python3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope net --github-output "$GITHUB_OUTPUT"',
        'python3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope render-device --github-output "$GITHUB_OUTPUT"',
        'python3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope aravath-determinism --github-output "$GITHUB_OUTPUT"',
        'python3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope aravath --github-output "$GITHUB_OUTPUT"',
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
# The first node candidate in the prefetch. Tests bind a shim over this path
# so the script never depends on which absolute node the host has.
_NODE_BIND = "/usr/local/bin/node"
_BARE_ARGV0 = re.compile(r"^[A-Za-z0-9_./+\[\]-]+$")
# Ordinary argv0 values. eval, command, sudo, timeout, and the old launcher
# names are absent: a step that uses one must match an exact body.
_ARGV0_ALLOW = frozenset(
    {
        ":",
        "[",
        "[[",
        "awk",
        "basename",
        "bc",
        "brew",
        "cargo",
        "cat",
        "cbindgen",
        "cc",
        "cd",
        "chmod",
        "cmake",
        "cmp",
        "cp",
        "curl",
        "cut",
        "date",
        "diff",
        "dirname",
        "echo",
        "exit",
        "export",
        "false",
        "file",
        "find",
        "gh",
        "git",
        "grep",
        "head",
        "heaptrack",
        "hostname",
        "id",
        "install",
        "jq",
        "kill",
        "ln",
        "local",
        "ls",
        "lychee",
        "mapfile",
        "mdbook",
        "mdbook-mermaid",
        "mkdir",
        "mv",
        "node",
        "nproc",
        "printf",
        "pwd",
        "python",
        "python3",
        "read",
        "rm",
        "rustc",
        "rustup",
        "sccache",
        "sed",
        "set",
        "sha256sum",
        "shopt",
        "sleep",
        "sort",
        "tail",
        "tar",
        "tee",
        "test",
        "touch",
        "tr",
        "true",
        "ulimit",
        "umask",
        "uname",
        "unset",
        "wait",
        "wc",
        "zip",
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
_INTERPRETERS = frozenset({"bash", "sh", "python", "python3", "node"})


class _PolicyScanError(Exception):
    pass


def _policy_names(names: list[str] | None = None) -> list[str]:
    """Workflow YAML plus every composite action, from git ls-files."""
    if names is None:
        raw = subprocess.check_output(["git", "ls-files", "-z", ".github"], cwd=ROOT)
        names = [item for item in raw.decode().split("\0") if item]
    selected: list[str] = []
    for name in names:
        if name.startswith(".github/workflows/") and name.endswith((".yml", ".yaml")):
            selected.append(name)
        elif name.startswith(".github/actions/") and name.endswith(
            ("/action.yml", "/action.yaml")
        ):
            selected.append(name)
    return selected


def _sccache_policy_texts(names: list[str] | None = None) -> list[tuple[str, str]]:
    texts: list[tuple[str, str]] = []
    for name in _policy_names(names):
        path = ROOT / name
        if path.is_file():
            texts.append((name, path.read_text(encoding="utf-8")))
    return texts


def _normalized_command(command: str) -> str:
    return " ".join(command.split())


def _allowlisted_command(command: str) -> bool:
    stripped = _normalized_command(command)
    if not stripped:
        return False
    return (
        stripped in _ALLOWED_SCRIPTS_CI_LINES
        or f"run: {stripped}" in _ALLOWED_SCRIPTS_CI_LINES
    )


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


def _component_matches(literal: str, part: str) -> bool:
    if part == "**":
        return False
    return fnmatch.fnmatchcase(literal, part)


def _path_names_scripts_ci(token: str) -> bool:
    parts = [part for part in token.split("/") if part not in {"", "."}]
    for index, part in enumerate(parts):
        if not _component_matches("scripts", part):
            continue
        nxt = index + 1
        while nxt < len(parts) and parts[nxt] == "**":
            nxt += 1
        if nxt < len(parts) and _component_matches("ci", parts[nxt]):
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


def _bash_unescape(text: str) -> str:
    """Drop the backslash bash drops, and keep the one double quotes keep."""
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
            nxt = text[index + 1]
            if quote == '"':
                if nxt in "$`\"\\":
                    out.append(nxt)
                    index += 2
                    continue
                out.append(char)
                index += 1
                continue
            if not quote:
                out.append(nxt)
                index += 2
                continue
        if char in "\"'" and not quote:
            quote = char
            index += 1
            continue
        if char == quote:
            quote = ""
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def _expand_braces(text: str) -> list[str]:
    match = re.search(r"(?<!\$)\{([A-Za-z0-9_./+-]*,[A-Za-z0-9_./+-]*)\}", text)
    if match is None:
        return [text]
    prefix = text[: match.start()]
    suffix = text[match.end() :]
    found: list[str] = []
    for alt in match.group(1).split(","):
        found.extend(_expand_braces(prefix + alt + suffix))
    return found


def _mentions_scripts_ci(text: str) -> bool:
    forms = [text, _fold_path_separators(text), _bash_unescape(text)]
    for form in list(forms):
        forms.extend(_expand_braces(form))
    return any(_names_scripts_ci(form) for form in forms)


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
        if char == ";":
            parts.append("".join(buf))
            buf = []
            index += 1
            continue
        if text.startswith("&&", index) or text.startswith("||", index):
            parts.append("".join(buf))
            buf = []
            index += 2
            continue
        if char == "|":
            parts.append("".join(buf))
            buf = []
            index += 1
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


def _logical_shell_lines(body: str, *, keep_heredoc: bool = False) -> list[str]:
    """Join continuations and quoted newlines. Heredoc bodies stay when asked."""
    lines: list[str] = []
    buf = ""
    pending = False
    heredoc: str | None = None
    for raw in body.splitlines():
        if heredoc is not None:
            if raw.strip() == heredoc:
                heredoc = None
            elif keep_heredoc:
                lines.append(raw)
            continue
        visible = raw
        if not pending and not _unclosed_quote(buf):
            visible = _strip_shell_comment(raw)
            match = _HEREDOC_RE.search(visible)
            if match is not None:
                heredoc = match.group(1)
        stripped = visible.rstrip()
        continued = (
            not _unclosed_quote(buf + stripped)
            and stripped.endswith("\\")
            and not stripped.endswith("\\\\")
        )
        piece = (stripped[:-1] + " ") if continued else visible
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


_CWD_TAINT = "\0"


def _apply_cd(parts: list[str], dest: str) -> list[str]:
    if dest.startswith("/"):
        if _path_names_scripts_ci(_cwd_text(parts)):
            return [_CWD_TAINT]
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
    unescaped = _bash_unescape(command)
    normalized = _expand_vars(unescaped, env)
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
    tainted = cwd == [_CWD_TAINT]
    token = _script_token(expanded_words) or ""
    relative = token.startswith("./") or token.startswith("../") or (
        "/" in token and not token.startswith("/")
    )
    named = (
        _mentions_scripts_ci(command)
        or _names_scripts_ci(unescaped)
        or _names_scripts_ci(_expand_vars(unescaped, local))
        or _relative_under_scripts_ci(cwd, expanded_words)
        or (tainted and relative)
    )
    nonliteral = _nonliteral_execution(raw[index:])
    if _cwd_unclassified(cwd, raw[index:]):
        nonliteral = True
    return named or nonliteral, env, cwd


_ASSIGN_PREFIX = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_SHELL_KEYWORDS = frozenset(
    {
        "!",
        "{",
        "}",
        "if",
        "then",
        "else",
        "elif",
        "fi",
        "do",
        "done",
        "while",
        "for",
        "until",
        "case",
        "esac",
        "in",
        "time",
    }
)


_ARRAY_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*\+=")


def _command_v_literal(command: str) -> bool:
    try:
        words = shlex.split(command, posix=True)
    except ValueError:
        return False
    index = 0
    while index < len(words) and (
        words[index] in _SHELL_KEYWORDS or _ASSIGN_PREFIX.match(words[index])
    ):
        index += 1
    rest = words[index:]
    if len(rest) < 3 or rest[0] != "command" or rest[1] != "-v":
        return False
    literal = rest[2]
    return _BARE_ARGV0.fullmatch(literal) is not None and not any(
        mark in literal for mark in ("$", "`", "*", "?", "[", "{")
    )


_DENIED_ARGV0 = frozenset(
    {
        ".",
        "alias",
        "builtin",
        "coproc",
        "env",
        "eval",
        "exec",
        "lua",
        "nice",
        "nohup",
        "perl",
        "php",
        "popd",
        "pushd",
        "ruby",
        "source",
        "sudo",
        "time",
        "timeout",
        "xargs",
    }
)


def _skip_shell_value(text: str, index: int) -> int:
    depth = 0
    quote = ""
    while index < len(text):
        char = text[index]
        if quote:
            if char == quote and text[index - 1] != "\\":
                quote = ""
            index += 1
            continue
        if char in "\"'":
            quote = char
            index += 1
            continue
        if text.startswith("$(", index) or char == "(":
            depth += 1
            index += 2 if text.startswith("$(", index) else 1
            continue
        if char == ")":
            if depth:
                depth -= 1
            index += 1
            continue
        if depth == 0 and char.isspace():
            return index
        index += 1
    return index


def _first_argv0(command: str) -> str | None:
    index = 0
    length = len(command)
    while index < length and command[index].isspace():
        index += 1
    while index < length:
        if command.startswith("$(", index):
            index = _skip_shell_value(command, index)
        else:
            matched = _ASSIGN_PREFIX.match(command, index)
            if matched is not None and matched.end() > index:
                index = _skip_shell_value(command, matched.end())
            else:
                break
        while index < length and command[index].isspace():
            index += 1
    if index >= length:
        return None
    word = _raw_words(command[index:])
    return word[0] if word else None


def _compound_hostile(command: str) -> bool:
    if any(mark in command for mark in ("<<", "<(", "<<<", "eval ", "sudo ", "timeout ")):
        return True
    if re.search(r"(^|[^<])<\s", command) or re.search(r"(^|[^<])<[\"']", command):
        return True
    pattern = (
        r"(?:^|\b(?:do|then|elif|else|if|while|until)\b|[;&|])\s*"
        r"(\"[^\"]*\$|\$[A-Za-z_{]|`)"
    )
    return re.search(pattern, command) is not None


def _command_indirection(command: str) -> bool:
    """Fail closed unless argv0 is on the allowlist.

    An exact body never reaches this check. A path argv0 is refused.
    command is allowed only as command -v of a literal. source, a dot
    command, time, eval, sudo, timeout, a bare interpreter, and an
    input redirect are refused.
    """
    if re.search(r"(^|\s)CDPATH=", command):
        return True
    if "<<" in command or "<(" in command or "<<<" in command:
        return True
    if re.search(r"(^|[;&|\s])[A-Za-z_][A-Za-z0-9_]*\(\)", command):
        return True
    if re.search(r"(?<!\$)\{[A-Za-z0-9_./+-]*,", command):
        return True
    if re.search(r"(^|[^<])<\s", command) or re.search(r"(^|[^<])<[\"']", command):
        return True
    if re.search(
        r"(?:^|[\s;|&])[A-Za-z_][A-Za-z0-9_]*=[\"']?(?:\$\(|`)",
        command,
    ):
        return True
    try:
        words = shlex.split(command, posix=True)
    except ValueError:
        return True
    if "-exec" in words:
        return True
    argv0 = _first_argv0(command)
    if argv0 is None:
        return False
    if argv0 == "time":
        return True
    if argv0 in _SHELL_KEYWORDS or argv0 == "{":
        return _compound_hostile(command)
    base = argv0.rsplit("/", 1)[-1]
    if base == "command":
        return not _command_v_literal(command)
    if base in _DENIED_ARGV0 or argv0 in _DENIED_ARGV0:
        return True
    if _BARE_ARGV0.fullmatch(argv0) is None:
        return True
    if "/" in argv0 or argv0.startswith("."):
        return True
    if (
        base not in _ARGV0_ALLOW
        and base not in _INTERPRETERS
        and not base.endswith("bash")
    ):
        return True
    if base in _INTERPRETERS or base.endswith("bash"):
        operand = None
        for word in words[1:]:
            if word.startswith("--"):
                return True
            if word.startswith("-") and word != "-":
                if any(flag in word[1:] for flag in ("c", "e")):
                    return True
                continue
            operand = word
            break
        if operand is None or _script_arg_nonliteral(operand):
            return True
    return False


def _body_has_fetch_pin(body: str) -> bool:
    for raw_line in _logical_shell_lines(body):
        command = _strip_shell_comment(raw_line).strip()
        if _normalized_command(command) == SCCACHE_FETCH_COMMAND:
            return True
    return False


def _fetch_changes_directory(body: str) -> bool:
    for raw_line in _logical_shell_lines(body):
        command = _strip_shell_comment(raw_line).strip()
        if not command:
            continue
        if _normalized_command(command) == SCCACHE_FETCH_COMMAND:
            return False
        for part in _split_commands(command):
            try:
                words = shlex.split(part, posix=True)
            except ValueError:
                continue
            index = 0
            while index < len(words) and (
                words[index] in _SHELL_KEYWORDS or _ASSIGN_PREFIX.match(words[index])
            ):
                index += 1
            if index < len(words) and words[index] == "cd":
                return True
    return False


def _normalized_body(body: str) -> str:
    lines: list[str] = []
    for raw_line in _logical_shell_lines(body, keep_heredoc=True):
        command = _strip_shell_comment(raw_line).strip()
        if command:
            lines.append(_normalized_command(command))
    return "\n".join(lines)


def _exact_entry(entry: str) -> str:
    text = entry[5:] if entry.startswith("run: ") else entry
    return _normalized_command(text)


_REVIEWED_EXACT_BODIES: frozenset[str] = frozenset((
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools',
    'channel=$(grep -v \'^[[:space:]]*#\' .github/rust-nightly-channel | tr -d \'[:space:]\')\nif [ -z "$channel" ]; then\necho "rust-nightly-channel is empty"\nexit 1\nfi\necho "channel=$channel" >> "$GITHUB_OUTPUT"\necho "AW_NIGHTLY=$channel" >> "$GITHUB_ENV"\necho "Pinned nightly: $channel"',
    'set -euo pipefail\nif [ "${{ github.event_name }}" != "pull_request" ]; then\n{\necho "run=true"\necho "reason=non-pull-request"\necho "full=true"\necho "packages="\n} >> "$GITHUB_OUTPUT"\nelse\npython3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope aravath-determinism --github-output "$GITHUB_OUTPUT"\nfi\ncat "$GITHUB_OUTPUT"',
    'sudo apt-get update && sudo apt-get install -y pkg-config libudev-dev libasound2-dev',
    'host=$(rustc -vV | sed -n \'s/^host: //p\')\ncase "$host" in\nx86_64-*)\necho "RUSTFLAGS=-C target-cpu=x86-64 -C target-feature=-fma" >> "$GITHUB_ENV"\n;;\naarch64-*)\necho "aarch64 uses the default baseline CPU (target-cpu=native is forbidden)."\n;;\n*)\necho "unsupported host $host" >&2\nexit 1\n;;\nesac',
    'set -euo pipefail\nif [ "${{ github.event_name }}" != "pull_request" ]; then\n{\necho "run=true"\necho "reason=non-pull-request"\necho "full=true"\necho "packages="\n} >> "$GITHUB_OUTPUT"\nelse\npython3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope aravath --github-output "$GITHUB_OUTPUT"\nfi\ncat "$GITHUB_OUTPUT"',
    'sudo apt-get update && sudo apt-get install -y --no-install-recommends pkg-config libudev-dev libasound2-dev mesa-vulkan-drivers libvulkan1',
    'mkdir -p dist/aravath_greybox/assets\ncp target/dist/aravath_greybox.exe dist/aravath_greybox/\ncp -a assets/aravath dist/aravath_greybox/assets/\ncp docs/current/ARAVATH_WINDOWS_README.txt dist/aravath_greybox/README.txt\ncp LICENSE dist/aravath_greybox/LICENSE\ntest -f dist/aravath_greybox/aravath_greybox.exe\ntest -f dist/aravath_greybox/README.txt\ntest -f dist/aravath_greybox/LICENSE\ntest -f dist/aravath_greybox/assets/aravath/models/AnimationLibrary_Godot_Standard.gltf\ntest -f dist/aravath_greybox/assets/aravath/textures/grass_diff_1k.jpg\ntest -f dist/aravath_greybox/assets/aravath/models/kaykit/knight/Knight.glb\nsig=$(head -c 4 dist/aravath_greybox/assets/aravath/models/kaykit/knight/Knight.glb)\ntest "$sig" = "glTF"\nfind dist/aravath_greybox -maxdepth 3 -type d -o -type f | sort',
    './aravath_greybox.exe --smoke-test --no-volumetrics',
    './aravath_greybox.exe --smoke-test --volumetrics',
    'set -euo pipefail\ntest -f aravath_greybox/aravath_greybox.exe\ntest -f aravath_greybox/SHA256SUMS.txt\ntest -f aravath_greybox/aravath_greybox.cdx.json\nzip -r aravath_greybox-windows.zip aravath_greybox\nnotes=$(cat <<EOF\nRolling Windows build of the Aravath greybox from \\`${GITHUB_SHA}\\`.\nExtract the whole zip and start \\`aravath_greybox.exe\\` from the extracted folder. Saves and logs go to \\`%LOCALAPPDATA%\\\\Aravath\\`. \\`SHA256SUMS.txt\\` covers the files in this zip. \\`aravath_greybox.cdx.json\\` is the CycloneDX SBOM.\nThis release is a prerelease. Chrome and SmartScreen can still warn until the file is signed with a publicly trusted certificate and has download reputation. See \\`docs/current/ARAVATH_DISTRIBUTION.md\\`.\n)\nif gh release view aravath-greybox >/dev/null 2>&1; then\ngh release upload aravath-greybox aravath_greybox-windows.zip --clobber\ngh release edit aravath-greybox --prerelease --target "$GITHUB_SHA" --notes "$notes"\nelse\ngh release create aravath-greybox aravath_greybox-windows.zip --prerelease --target "$GITHUB_SHA" --title "Aravath greybox" --notes "$notes"\nfi',
    'set -euo pipefail\ncargo run -p astraweave-assets -- fetch --quiet --json --manifest assets/asset_manifest.toml --provider polyhaven > assets_summary.json\nFAILED=$(jq -r \'.failed\' assets_summary.json)\nif [ "$FAILED" != "0" ]; then\necho "Asset fetch reported $FAILED failures" >&2\njq \'.assets[] | select(.status == "failed")\' assets_summary.json >&2\nexit 1\nfi',
    'sudo apt-get update && sudo apt-get install -y pkg-config libudev-dev',
    'sudo apt-get update\nsudo apt-get install -y pkg-config libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev libwayland-dev libudev-dev libasound2-dev',
    'sudo apt-get update\nsudo apt-get install -y pkg-config libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev libudev-dev libasound2-dev',
    "set -euo pipefail\nif command -v pwsh >/dev/null; then\npwsh -NoLogo -Command '$PSVersionTable.PSVersion.ToString()'\nexit 0\nfi\nsudo apt-get update\nsudo apt-get install -y powershell\ncommand -v pwsh\npwsh -NoLogo -Command '$PSVersionTable.PSVersion.ToString()'",
    'pwsh -File scripts/export_benchmark_jsonl.ps1 -Verbose',
    'sudo apt-get update\nsudo apt-get install -y python3 python3-pip\npython3 -m pip install --upgrade pip\npython3 -m pip install pandas matplotlib seaborn',
    'sudo apt-get update\nsudo apt-get install -y jq\nbash scripts/export_benchmark_history.sh || true',
    'pwsh -File scripts/check_benchmark_thresholds.ps1 -ShowDetails',
    'echo "=== gh-pages contents ==="\nfind gh-pages -type f | head -30\necho ""\nif [ -d gh-pages/graphs ]; then\nCOUNT=$(ls gh-pages/graphs | wc -l)\nif [ "$COUNT" -gt 0 ]; then\necho "Found $COUNT graph(s) to be deployed";\nelse\necho "Warning: No graphs were generated";\nfi\nelse\necho "Warning: gh-pages/graphs directory not found";\nfi\nif [ -f gh-pages/benchmark-data/history.jsonl ]; then\nLINES=$(wc -l < gh-pages/benchmark-data/history.jsonl)\necho "benchmark-data/history.jsonl: $LINES entries"\nelse\necho "Warning: No benchmark-data/history.jsonl"\nfi\nif [ -f gh-pages/static-data/history.jsonl ]; then\necho "static-data/history.jsonl: present"\nelse\necho "Warning: No static-data/history.jsonl"\nfi\ncat > gh-pages/README.md << \'EOF\'\nLive performance telemetry dashboard for the AstraWeave AI-native game engine.\n**View Dashboard**: https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/\n- 30-day performance trends\n- System filtering (ECS, AI, Physics, Terrain, Input)\n- Interactive d3.js charts\n- Regression detection\nUpdated nightly via GitHub Actions from Criterion benchmark results.\nLast Updated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")',
    'echo "### Benchmark Dashboard Updated ✅" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "**Dashboard URL**: https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\nif [ -f target/benchmark-data/history.jsonl ]; then\nENTRY_COUNT=$(wc -l < target/benchmark-data/history.jsonl)\necho "**Benchmark Entries**: $ENTRY_COUNT" >> $GITHUB_STEP_SUMMARY\nfi\nif [ -d gh-pages/graphs ]; then\nGRAPH_COUNT=$(ls gh-pages/graphs | wc -l)\necho "**Static Graphs Generated**: $GRAPH_COUNT" >> $GITHUB_STEP_SUMMARY\nfi\necho "" >> $GITHUB_STEP_SUMMARY\necho "**Last Updated**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> $GITHUB_STEP_SUMMARY',
    'sudo apt-get update -q\nsudo apt-get install -y -q mesa-vulkan-drivers vulkan-tools bc jq coreutils',
    'export BENCHMARK_RESULTS_DIR="benchmark_results/shard-${{ matrix.shard_index }}"\nexport VERBOSE="true"\n./.github/scripts/benchmark-runner.sh --shard-index "${{ matrix.shard_index }}" --shard-count 16',
    'set -euo pipefail\nmkdir -p benchmark_results\nresult_files=()\nstatus_files=()\ntiming_files=()\nif [ -d benchmark-shards ]; then\nmapfile -d \'\' result_files < <(find benchmark-shards -name benchmarks.json -print0 | sort -z)\nmapfile -d \'\' status_files < <(find benchmark-shards -name package-status.json -print0 | sort -z)\nmapfile -d \'\' timing_files < <(find benchmark-shards -name timings.tsv -print0 | sort -z)\nfi\nif [ "${#result_files[@]}" -gt 0 ]; then\njq -s \'add\' "${result_files[@]}" > benchmark_results/benchmarks.json\nelse\necho \'[]\' > benchmark_results/benchmarks.json\nfi\nif [ "${#status_files[@]}" -gt 0 ]; then\njq -s \'add\' "${status_files[@]}" > benchmark_results/package-status.json\nelse\necho \'[]\' > benchmark_results/package-status.json\nfi\nprintf \'package\\telapsed_seconds\\texecution_status\\tcollection_status\\tbenchmark_count\\n\' > benchmark_results/timings.tsv\nfor timing_file in "${timing_files[@]}"; do\ntail -n +2 "$timing_file" >> benchmark_results/timings.tsv\ndone\n: > benchmark_results/missing-shards.txt\nfor shard_index in {0..15}; do\nif ! find benchmark-shards -mindepth 1 -maxdepth 1 -type d -name "*-shard-${shard_index}" -print -quit 2>/dev/null |\ngrep -q .; then\necho "shard-${shard_index}" >> benchmark_results/missing-shards.txt\nfi\ndone\npackage_count=$(jq \'length\' benchmark_results/package-status.json)\nunique_package_count=$(jq \'[.[].package] | unique | length\' benchmark_results/package-status.json)\ncollected_package_count=$(jq \'[.[] | select(.collection_status == "collected")] | length\' benchmark_results/package-status.json)\nexception_count=$(jq \'[.[] | select(.collection_status == "exception")] | length\' benchmark_results/package-status.json)\nno_output_failure_count=$(jq \'[.[] | select( .execution_status != "success" and .benchmark_count == 0 )] | length\' benchmark_results/package-status.json)\nfailed_package_count=$(jq \'[.[] | select( .execution_status != "success" or (.collection_status != "collected" and .collection_status != "exception") )] | length\' benchmark_results/package-status.json)\nbenchmark_count=$(jq \'length\' benchmark_results/benchmarks.json)\ncollection_accounted_count=$((collected_package_count + exception_count))\nnamed_accounting_exception_count=$((exception_count + no_output_failure_count))\noutcome_accounted_count=$((collected_package_count + named_accounting_exception_count))\nrecord_accounted_count=$unique_package_count\njq -r \'.[] | select( .execution_status != "success" or (.collection_status != "collected" and .collection_status != "exception") ) | "\\(.package): execution=\\(.execution_status), collection=\\(.collection_status), results=\\(.benchmark_count), elapsed=\\(.elapsed_seconds)s"\' benchmark_results/package-status.json > benchmark_results/failed-packages.txt\ncomplete=false\nif [ "${#result_files[@]}" -eq 16 ] &&\n[ "${#status_files[@]}" -eq 16 ] &&\n[ "${#timing_files[@]}" -eq 16 ] &&\n[ "$package_count" -eq 53 ] &&\n[ "$unique_package_count" -eq 53 ]; then\ncomplete=true\nfi\nhealthy=false\nif [ "$complete" = true ] &&\n[ "$failed_package_count" -eq 0 ] &&\n[ "$collection_accounted_count" -eq 53 ]; then\nhealthy=true\nfi\n{\necho "=== Benchmark Aggregation Summary ==="\necho "Shard result files: ${#result_files[@]}/16"\necho "Shard status files: ${#status_files[@]}/16"\necho "Shard timing files: ${#timing_files[@]}/16"\necho "Package records: $package_count (unique: $unique_package_count; expected: 53)"\necho "Package records accounted for: $record_accounted_count/53"\necho "Packages with collected results: $collected_package_count"\necho "Named collection exceptions: $exception_count"\necho "Named no-output execution failures: $no_output_failure_count"\necho "Packages with Criterion output or collection exception: $collection_accounted_count/53"\necho "Outcome accounting: $collected_package_count collected + $named_accounting_exception_count named accounting exceptions = $outcome_accounted_count/53"\necho "Failed packages: $failed_package_count"\necho "Benchmark results collected: $benchmark_count"\necho "Complete: $complete"\necho "Healthy: $healthy"\nif [ -s benchmark_results/missing-shards.txt ]; then\necho ""\necho "Missing shard artifacts:"\ncat benchmark_results/missing-shards.txt\nfi\nif [ -s benchmark_results/failed-packages.txt ]; then\necho ""\necho "Named failures:"\ncat benchmark_results/failed-packages.txt\nfi\n} | tee benchmark_results/aggregation-summary.txt\necho "complete=$complete" >> "$GITHUB_OUTPUT"\necho "healthy=$healthy" >> "$GITHUB_OUTPUT"\necho "package_count=$package_count" >> "$GITHUB_OUTPUT"\necho "record_accounted_count=$record_accounted_count" >> "$GITHUB_OUTPUT"\necho "collection_accounted_count=$collection_accounted_count" >> "$GITHUB_OUTPUT"\necho "failed_package_count=$failed_package_count" >> "$GITHUB_OUTPUT"\necho "benchmark_count=$benchmark_count" >> "$GITHUB_OUTPUT"',
    'echo "=== Benchmark Validation ==="\nif [ ! -f "benchmark_results/benchmarks.json" ]; then\necho "❌ No benchmark results file found"\necho "Contents of benchmark_results directory:"\nls -la benchmark_results/ || echo "Directory doesn\'t exist"\nexit 1\nfi\nif ! jq empty benchmark_results/benchmarks.json 2>/dev/null; then\necho "❌ Invalid JSON in benchmark results"\necho "JSON content:"\ncat benchmark_results/benchmarks.json\nexit 1\nfi\nresult_count=$(jq \'. | length\' benchmark_results/benchmarks.json)\nif [ "$result_count" -eq 0 ]; then\necho "❌ No benchmark results collected"\necho "JSON content:"\ncat benchmark_results/benchmarks.json\nexit 1\nfi\nif ! jq -e \'.[0] | has("name") and has("value") and has("unit")\' benchmark_results/benchmarks.json >/dev/null 2>&1; then\necho "❌ Benchmark results missing required fields (name, value, unit)"\njq \'.[0]\' benchmark_results/benchmarks.json\nexit 1\nfi\necho "✅ Validation passed: $result_count benchmark results collected"\necho ""\necho "=== Complete Results ==="\njq -r \'.[] | "\\(.name): \\(.value) \\(.unit)"\' benchmark_results/benchmarks.json\necho ""\necho "=== Performance Analysis ==="\nmin_time=$(jq \'[.[].value] | min\' benchmark_results/benchmarks.json)\nmax_time=$(jq \'[.[].value] | max\' benchmark_results/benchmarks.json)\navg_time=$(jq \'[.[].value] | add / length\' benchmark_results/benchmarks.json)\necho "Fastest operation: ${min_time} ns"\necho "Slowest operation: ${max_time} ns"\necho "Average time: ${avg_time} ns"',
    'if (!(Get-Command pwsh -ErrorAction SilentlyContinue)) {\nWrite-Host "Installing PowerShell..."\nsudo apt-get update\nsudo apt-get install -y powershell\n}\nWrite-Host "=== Running Threshold Validation ===" -ForegroundColor Cyan\n& ./scripts/check_benchmark_thresholds.ps1 `\n-BenchmarkJsonPath "benchmark_results/benchmarks.json" `\n-ThresholdsJsonPath ".github/benchmark_thresholds.json" `\n-ShowDetails',
    'Write-Host "=== Strict Threshold Validation (Main Branch) ===" -ForegroundColor Red\n& ./scripts/check_benchmark_thresholds.ps1 `\n-BenchmarkJsonPath "benchmark_results/benchmarks.json" `\n-ThresholdsJsonPath ".github/benchmark_thresholds.json" `\n-Strict `\n-ShowDetails',
    'sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools bc jq coreutils',
    'Write-Host "=== Checking for Performance Regressions ===" -ForegroundColor Cyan\n& ./scripts/check_benchmark_thresholds.ps1 `\n-BenchmarkJsonPath "benchmark_results/benchmarks.json" `\n-ThresholdsJsonPath ".github/benchmark_thresholds.json" `\n-Strict `\n-ShowDetails\n$exitCode = $LASTEXITCODE\nif ($exitCode -ne 0) {\nWrite-Host "Regressions detected (exit code $exitCode)" -ForegroundColor Red\necho "has_regressions=true" >> $env:GITHUB_OUTPUT\n$results = Get-Content "benchmark_results/benchmarks.json" | ConvertFrom-Json\n$thresholds = Get-Content ".github/benchmark_thresholds.json" | ConvertFrom-Json\n$regressions = @()\nforeach ($result in $results) {\n$threshold = $thresholds.benchmarks.($result.name)\nif ($threshold -and $result.value -gt $threshold.max_allowed) {\n$overPercent = [math]::Round((($result.value - $threshold.max_allowed) / $threshold.max_allowed) * 100, 1)\n$regressions += "- **$($result.name)**: $($result.value) ns (baseline: $($threshold.baseline) ns, max: $($threshold.max_allowed) ns, **+$overPercent% over limit**)"\n}\n}\n$regressionList = $regressions -join "`n"\n$lines = @(\n\'## Performance Regression Detected\'\n\'\'\n\'Automated benchmark validation found performance regressions in the following benchmarks:\'\n\'\'\n$regressionList\n\'\'\n\'**Commit**: ${{ github.sha }}\'\n(\'**Branch**: \' + $env:GITHUB_REF_NAME)\n\'**Workflow**: ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}\'\n\'\'\n\'### Action Required\'\n\'\'\n\'1. Review the regression in the workflow run linked above.\'\n\'2. Investigate the cause.\'\n\'3. Fix the regression, or update the threshold with a written reason.\'\n\'\'\n\'Default max regression is 50% slower than baseline.\'\n\'\'\n\'This issue was created by the Benchmark Regression Alerts workflow.\'\n)\nSet-Content -Path regression_report.md -Value $lines -Encoding utf8\n} else {\nWrite-Host "No regressions detected" -ForegroundColor Green\necho "has_regressions=false" >> $env:GITHUB_OUTPUT\n}',
    'bash scripts/export_benchmark_history.sh\nmkdir -p docs/benchmark_data\nif [ -f "docs/benchmark_data/benchmark_history.jsonl" ]; then\necho "✅ Benchmark history exported successfully"\necho " Lines: $(wc -l < docs/benchmark_data/benchmark_history.jsonl)"\nelse\necho "⚠️ Export script completed but no history file generated"\necho " This is normal if no gh-pages branch exists yet"\nfi',
    'set -euo pipefail\nif [ "${{ github.event_name }}" != "pull_request" ]; then\n{\necho "docs_only=false"\necho "full=true"\necho "packages="\necho "reason=non-pull-request"\n} >> "$GITHUB_OUTPUT"\nelif ! git rev-parse --verify --quiet \'HEAD^1\' >/dev/null; then\n{\necho "docs_only=false"\necho "full=true"\necho "packages="\necho "reason=missing-merge-parent"\n} >> "$GITHUB_OUTPUT"\nelse\npython3 scripts/ci/affected_crates.py --base "HEAD^1" --head HEAD --github-output "$GITHUB_OUTPUT"\nfi',
    'set -euo pipefail\nversion=$(grep -m1 \'rust-version\' Cargo.toml | sed \'s/.*"\\(.*\\)".*/\\1/\')\nif [ -z "$version" ]; then\necho "rust-version is missing from the workspace Cargo.toml" >&2\nexit 1\nfi\necho "version=$version" >> "$GITHUB_OUTPUT"\necho "MSRV $version"',
    'set -euo pipefail\nif [ "$AW_CI_FULL" = "true" ]; then\ncargo check --locked --workspace ${AW_CI_EXCLUDES}\nelse\nIFS=\',\' read -ra pkgs <<< "$AW_CI_PACKAGES"\nargs=(cargo check --locked)\nfor pkg in "${pkgs[@]}"; do\nif [ -n "$pkg" ]; then\nargs+=(-p "$pkg")\nfi\ndone\n"${args[@]}"\nfi',
    'cargo build --locked -p veilweaver_slice_loader\ntimeout 120s make smoke-veilweaver',
    'echo "Testing hello_companion demo..."\ntimeout 10s cargo run --release -p hello_companion || echo "Demo completed or timed out as expected"',
    'echo "Testing IPC loopback example..."\ncargo run --release -p ipc_loopback --bin server &\nSERVER_PID=$!\nsleep 2\ntimeout 5s cargo run --release -p ipc_loopback --bin client || echo "IPC test completed"\nkill $SERVER_PID 2>/dev/null || true',
    'sudo apt-get update && sudo apt-get install -y libasound2-dev libudev-dev',
    'cd tools/astraweave-assets\necho "## Code Coverage Report" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\nif [ -f coverage/cobertura.xml ]; then\nCOVERAGE=$(grep -oP \'line-rate="\\K[0-9.]+\' coverage/cobertura.xml | head -1 | awk \'{printf "%.1f", $1*100}\')\necho "**Overall Coverage:** ${COVERAGE}%" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "Target: 80%+ (Production Ready)" >> $GITHUB_STEP_SUMMARY\nif (( $(echo "$COVERAGE >= 80" | bc -l) )); then\necho "✅ **Status:** PASS (meets target)" >> $GITHUB_STEP_SUMMARY\nelse\necho "⚠️ **Status:** Below target (needs improvement)" >> $GITHUB_STEP_SUMMARY\nfi\nelse\necho "⚠️ Coverage data not generated" >> $GITHUB_STEP_SUMMARY\nfi',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev',
    'sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools time',
    'echo "## Build Performance Profile" > performance.md\necho "Generated on: $(date)" >> performance.md\necho "" >> performance.md\necho "### Clean Build Timing (Core Components)" >> performance.md\necho \'```\' >> performance.md\ncargo clean\n/usr/bin/time -v cargo build -p astraweave-core -p astraweave-ai -p astraweave-physics -p hello_companion 2>&1 | tee build-timing.txt\ncat build-timing.txt >> performance.md\necho \'```\' >> performance.md\necho "" >> performance.md\necho "### Release Build Timing (Core Components)" >> performance.md\necho \'```\' >> performance.md\ncargo clean\n/usr/bin/time -v cargo build --release -p astraweave-core -p astraweave-ai -p astraweave-physics -p hello_companion 2>&1 | tee release-timing.txt\ncat release-timing.txt >> performance.md\necho \'```\' >> performance.md',
    'host=$(rustc -vV | sed -n \'s/^host: //p\')\ncase "$host" in\nx86_64-*)\necho "RUSTFLAGS=-C target-cpu=x86-64" >> "$GITHUB_ENV"\n;;\naarch64-*)\necho "aarch64 uses the default baseline CPU (target-cpu=native is forbidden)."\n;;\n*)\necho "unsupported host $host" >&2\nexit 1\n;;\nesac',
    'node <<\'JS\'\nconst fs = require("fs");\nconst path = require("path");\nconst root = process.env.CARGO_TARGET_DIR || "target";\nconst core = JSON.parse(fs.readFileSync(path.join(root, "replay-hash.json"), "utf8"));\nconst combat = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-combat.json"), "utf8"));\nconst gameplay = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-gameplay-qa.json"), "utf8"));\nconst causal = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-causal.json"), "utf8"));\nconst traversal = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-traversal.json"), "utf8"));\nconst localsim = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-localsim.json"), "utf8"));\nconst greybox = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-greybox.json"), "utf8"));\nconst quests = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-quests.json"), "utf8"));\nconst persist = JSON.parse(fs.readFileSync(path.join(root, "replay-hash-persist.json"), "utf8"));\nconst seen = new Set(core.map((row) => row.case));\nconst merged = core.slice();\nfor (const row of combat.concat(gameplay, causal, traversal, localsim, greybox, quests, persist)) {\nif (seen.has(row.case)) {\nconsole.error(`duplicate replay case ${row.case}`);\nprocess.exit(1);\n}\nseen.add(row.case);\nmerged.push(row);\n}\nconst required = ["planner_pure", "perception_world", "gameplay_rng", "fixed64", "combat_resolve", "destruction", "gameplay_qa_aravath_slice", "nav_reroute", "causal_bridge", "scripted_system", "weaving", "traversal", "traversal_deformed", "worldgen_columns", "deformed_chunk", "local_sim", "greybox_checkpoint", "quest_ticks", "persist_replay"];\nconst names = new Set(merged.map((row) => row.case));\nfor (const name of required) {\nif (!names.has(name)) {\nconsole.error(`missing required case ${name}`);\nprocess.exit(1);\n}\n}\nfs.writeFileSync(path.join(root, "replay-hash.json"), JSON.stringify(merged, null, 2) + "\\n");\nconsole.log("merged", merged.map((row) => `${row.case} ${row.sha256}`).join("\\n"));',
    'python3 - <<\'PY\'\nimport json\nimport sys\nfrom pathlib import Path\nrequired = {"planner_pure", "perception_world", "gameplay_rng", "fixed64", "combat_resolve", "destruction", "gameplay_qa_aravath_slice", "nav_reroute", "causal_bridge", "scripted_system", "weaving", "traversal", "traversal_deformed", "worldgen_columns", "deformed_chunk", "local_sim", "greybox_checkpoint", "quest_ticks", "persist_replay"}\nroot = Path("replay-artifacts")\nfiles = sorted(root.glob("*/replay-hash.json"))\nprint(f"artifacts: {[str(p) for p in files]}")\nif len(files) != 3:\nprint(f"expected 3 replay-hash artifacts, found {len(files)}")\nsys.exit(1)\nby_os = {}\nfor path in files:\nos_name = path.parent.name.removeprefix("replay-hash-")\nrows = json.loads(path.read_text(encoding="utf-8"))\nby_os[os_name] = {row["case"]: row["sha256"] for row in rows}\ncases = set()\nfor digests in by_os.values():\ncases.update(digests)\nfailed = False\nfor case in sorted(cases | required):\ndigests = {os_name: mapping.get(case) for os_name, mapping in sorted(by_os.items())}\npresent = {value for value in digests.values() if value is not None}\nmissing = [os_name for os_name, value in digests.items() if value is None]\nif missing or len(present) != 1:\nprint(f"MISMATCH {case}: {digests}")\nif case in required:\nfailed = True\nelse:\nprint(f"{case} differs and is not a required case")\nelse:\ndigest = next(iter(present))\nprint(f"OK {case} {digest}")\nif failed:\nsys.exit(1)',
    'curl -sSL "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/mdbook-v${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz" | tar -xz\nsudo mv mdbook /usr/local/bin/\nmdbook --version',
    'curl -sSL "https://github.com/badboy/mdbook-mermaid/releases/download/v0.17.0/mdbook-mermaid-v0.17.0-x86_64-unknown-linux-gnu.tar.gz" | tar -xz && sudo mv mdbook-mermaid /usr/local/bin/ && mdbook-mermaid --version',
    'chmod +x docs/scripts/generate-sitemap.sh || true\ndocs/scripts/generate-sitemap.sh docs/book || echo "Sitemap generation had issues"',
    'curl -sSL https://github.com/lycheeverse/lychee/releases/download/v0.15.1/lychee-v0.15.1-x86_64-unknown-linux-gnu.tar.gz | tar -xz\n./lychee docs/book/**/*.html --exclude localhost --exclude 127.0.0.1 --exclude "file://" --format markdown --output link-report.md || true\nif [ -f link-report.md ]; then\necho "### Link Check Report" >> $GITHUB_STEP_SUMMARY\ncat link-report.md >> $GITHUB_STEP_SUMMARY\nfi',
    'python - <<\'PY\'\nimport json, pathlib, sys\nsummary_path = pathlib.Path("coverage/summary.json")\nif not summary_path.exists():\nsys.exit("coverage/summary.json not found")\ndata = json.loads(summary_path.read_text())\nfiles = [\nentry for entry in data["data"][0]["files"]\nif "tools\\\\aw_editor" in entry["filename"]\nor "tools/aw_editor" in entry["filename"]\n]\nif not files:\nsys.exit("coverage summary missing aw_editor entries")\ndef collect(metric):\ncovered = sum(f["summary"][metric]["covered"] for f in files)\ntotal = sum(f["summary"][metric]["count"] for f in files)\npercent = (covered / total * 100) if total else 0.0\nreturn covered, total, percent\nline_cov, line_total, line_pct = collect("lines")\nfn_cov, fn_total, fn_pct = collect("functions")\nregion_cov, region_total, region_pct = collect("regions")\nwith open("${{ github.step_summary }}", "a", encoding="utf-8") as fh:\nfh.write("## aw_editor Coverage\\n\\n")\nfh.write(f"- Lines: {line_pct:.2f}% ({line_cov}/{line_total})\\n")\nfh.write(f"- Functions: {fn_pct:.2f}% ({fn_cov}/{fn_total})\\n")\nfh.write(f"- Regions: {region_pct:.2f}% ({region_cov}/{region_total})\\n")',
    'sudo apt-get update\nsudo apt-get install -y heaptrack mesa-vulkan-drivers libvulkan1 pkg-config libudev-dev',
    'echo "RUST_BACKTRACE=${RUST_BACKTRACE:-unset}"\necho "RUST_LIB_BACKTRACE=${RUST_LIB_BACKTRACE:-unset}"\nunset VK_ADD_DRIVER_FILES\nif [ -f /usr/share/vulkan/icd.d/lvp_icd.x86_64.json ]; then\nicd=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json\nelif [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then\nicd=/usr/share/vulkan/icd.d/lvp_icd.json\nelse\necho "lavapipe ICD is not installed" >&2\nexit 1\nfi\nexport VK_ICD_FILENAMES="$icd"\nexport VK_DRIVER_FILES="$icd"\nmkdir -p target\ncargo test -p astraweave-render --all-features --test frame_alloc --no-run --message-format=json > target/frame-alloc-build.json\nbin=$(python3 scripts/ci/frame_alloc_test_executable.py target/frame-alloc-build.json)\ntest -x "$bin"\necho "heaptrack binary=$bin"\nheaptrack -o target/frame-alloc-heaptrack "$bin" --exact draw_into_allocates_nothing_after_warmup --nocapture',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake libudev-dev',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake',
    'echo "::group::Parsing validation results"\ncheck_validation() {\nlocal file=$1\nlocal name=$2\nif [ -f "$file" ]; then\npassed=$(jq -r \'.results[0].passed // false\' "$file")\nerror_count=$(jq -r \'.results[0].errors | length\' "$file")\nwarning_count=$(jq -r \'.results[0].warnings | length\' "$file")\necho "${name}_passed=${passed}" >> $GITHUB_OUTPUT\necho "${name}_errors=${error_count}" >> $GITHUB_OUTPUT\necho "${name}_warnings=${warning_count}" >> $GITHUB_OUTPUT\nif [ "$passed" = "true" ]; then\necho "✅ ${name}: PASS (${warning_count} warnings)"\nelse\necho "❌ ${name}: FAIL (${error_count} errors, ${warning_count} warnings)"\nfi\nelse\necho "⚠️ ${name}: No validation results found"\necho "${name}_passed=false" >> $GITHUB_OUTPUT\nfi\n}\ncheck_validation "validation-grassland.json" "grassland"\ncheck_validation "validation-mountain.json" "mountain"\ncheck_validation "validation-desert.json" "desert"\ncheck_validation "validation-all.json" "all_materials"\necho "::endgroup::"',
    'echo "## 🎨 Material Validation Summary" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "| Material | Status | Errors | Warnings |" >> $GITHUB_STEP_SUMMARY\necho "|----------|--------|--------|----------|" >> $GITHUB_STEP_SUMMARY\nadd_summary_row() {\nlocal file=$1\nlocal name=$2\nif [ -f "$file" ]; then\npassed=$(jq -r \'.results[0].passed // false\' "$file")\nerror_count=$(jq -r \'.results[0].errors | length\' "$file")\nwarning_count=$(jq -r \'.results[0].warnings | length\' "$file")\nif [ "$passed" = "true" ]; then\nstatus="✅ PASS"\nelse\nstatus="❌ FAIL"\nfi\necho "| ${name} | ${status} | ${error_count} | ${warning_count} |" >> $GITHUB_STEP_SUMMARY\nelse\necho "| ${name} | ⚠️ NO RESULTS | - | - |" >> $GITHUB_STEP_SUMMARY\nfi\n}\nadd_summary_row "validation-grassland.json" "Grassland Demo"\nadd_summary_row "validation-mountain.json" "Mountain Demo"\nadd_summary_row "validation-desert.json" "Desert Demo"\nadd_summary_row "validation-all.json" "All Materials"\necho "" >> $GITHUB_STEP_SUMMARY\necho "### 📊 Details" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "See artifacts below for full validation reports (JSON format)." >> $GITHUB_STEP_SUMMARY',
    'echo "::group::Checking overall validation status"\nfailed=false\nfor file in validation-*.json; do\nif [ -f "$file" ]; then\npassed=$(jq -r \'.results[0].passed // false\' "$file")\nif [ "$passed" != "true" ]; then\nfailed=true\necho "❌ Validation failed: $file"\njq -r \'.results[0].errors[]\' "$file" || true\nfi\nfi\ndone\nif [ "$failed" = "true" ]; then\necho "::error::Material validation failed. See artifacts for details."\nexit 1\nelse\necho "✅ All material validations passed!"\nfi\necho "::endgroup::"',
    'if [ -f mutants-ecs-results/mutants.json ]; then\necho "## Mutation Testing: astraweave-ecs" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\nTOTAL=$(jq \'.total_mutants\' mutants-ecs-results/mutants.json)\nKILLED=$(jq \'.killed\' mutants-ecs-results/mutants.json)\nSURVIVED=$(jq \'.survived\' mutants-ecs-results/mutants.json)\nTIMEOUT=$(jq \'.timeout\' mutants-ecs-results/mutants.json)\nif [ "$TOTAL" -gt 0 ]; then\nSCORE=$((KILLED * 100 / TOTAL))\necho "| Metric | Count |" >> $GITHUB_STEP_SUMMARY\necho "|--------|-------|" >> $GITHUB_STEP_SUMMARY\necho "| Total Mutants | $TOTAL |" >> $GITHUB_STEP_SUMMARY\necho "| Killed | $KILLED |" >> $GITHUB_STEP_SUMMARY\necho "| Survived | $SURVIVED |" >> $GITHUB_STEP_SUMMARY\necho "| Timeout | $TIMEOUT |" >> $GITHUB_STEP_SUMMARY\necho "| **Mutation Score** | **${SCORE}%** |" >> $GITHUB_STEP_SUMMARY\nfi\nfi',
    'outcomes=mutants-ai-results/mutants.out/outcomes.json\npython3 scripts/ci/mutation_ai_outcomes.py --verify-shard "$outcomes"',
    'outcomes=mutants-ai-results/mutants.out/outcomes.json\nif [ -f "$outcomes" ]; then\necho "## Mutation Testing: astraweave-ai" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\ncounts=$(python3 scripts/ci/mutation_ai_outcomes.py "$outcomes") || exit 1\neval "$counts"\n: "${DECIDED:?}" "${SCORE:?}"\necho "| Metric | Count |" >> $GITHUB_STEP_SUMMARY\necho "|--------|-------|" >> $GITHUB_STEP_SUMMARY\necho "| Caught | $CAUGHT |" >> $GITHUB_STEP_SUMMARY\necho "| Missed | $MISSED |" >> $GITHUB_STEP_SUMMARY\necho "| Timeout | $TIMEOUT |" >> $GITHUB_STEP_SUMMARY\necho "| Cap OOM (excluded) | $CAP_OOM |" >> $GITHUB_STEP_SUMMARY\nif [ "$DECIDED" -gt 0 ]; then\necho "| **Shard score** | **${SCORE}%** (${CAUGHT}/${DECIDED}, target 70%) |" >> $GITHUB_STEP_SUMMARY\nelse\necho "::error::astraweave-ai shard has no caught or missed mutants after cap-OOM exclusion (timeouts ${TIMEOUT})"\nexit 1\nfi\nelse\necho "::error::missing astraweave-ai outcomes at $outcomes"\nexit 1\nfi',
    'echo "## 🧬 Mutation Testing Summary" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "| Crate | Status | Notes |" >> $GITHUB_STEP_SUMMARY\necho "|-------|--------|-------|" >> $GITHUB_STEP_SUMMARY\necho "| astraweave-ecs | ${{ needs.mutants-ecs.result == \'success\' && \'✅\' || \'⚠️\' }} | P0 Core |" >> $GITHUB_STEP_SUMMARY\necho "| astraweave-core | ${{ needs.mutants-core.result == \'success\' && \'✅\' || \'⚠️\' }} | P0 Core |" >> $GITHUB_STEP_SUMMARY\necho "| astraweave-behavior | ${{ needs.mutants-behavior.result == \'success\' && \'✅\' || \'⚠️\' }} | P1 AI |" >> $GITHUB_STEP_SUMMARY\necho "| astraweave-nav | ${{ needs.mutants-nav.result == \'success\' && \'✅\' || \'⚠️\' }} | P1 Infra |" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\nif [ "${{ needs.mutants-ai.result }}" = "skipped" ]; then\necho "| astraweave-ai | skipped | P1 AI not selected |" >> "$GITHUB_STEP_SUMMARY"\nelif [ "${{ needs.mutants-ai.result }}" != "success" ]; then\necho "::error::astraweave-ai matrix did not succeed (result=${{ needs.mutants-ai.result }}); partial shards must not pass the gate"\nexit 1\nelse\nmapfile -t ai_files < <(find all-results -type f -name outcomes.json | sort | grep \'/mutants-ai-results-\' || true)\necho "astraweave-ai shard files: ${#ai_files[@]}"\nprintf \'%s\\n\' "${ai_files[@]}"\nif [ "${#ai_files[@]}" -ne 8 ]; then\necho "::error::Expected 8 astraweave-ai shard outcomes, found ${#ai_files[@]}"\nexit 1\nfi\ncounts=$(python3 scripts/ci/mutation_ai_outcomes.py "${ai_files[@]}") || exit 1\neval "$counts"\n: "${DECIDED:?}" "${SCORE:?}"\nif [ "$DECIDED" -le 0 ]; then\necho "::error::astraweave-ai has no caught or missed mutants after cap-OOM exclusion (timeouts ${TIMEOUT})"\nexit 1\nfi\necho "| astraweave-ai | ${SCORE}% | P1 AI, 8 shards, caught ${CAUGHT}, missed ${MISSED}, timeout ${TIMEOUT}, cap-OOM ${CAP_OOM}, target 70% |" >> "$GITHUB_STEP_SUMMARY"\necho "" >> "$GITHUB_STEP_SUMMARY"\necho "astraweave-ai combined score: ${SCORE}% (target 70%). Caught ${CAUGHT}, missed ${MISSED}, timeout ${TIMEOUT}, cap-OOM excluded ${CAP_OOM}." >> "$GITHUB_STEP_SUMMARY"\nif [ "$SCORE" -lt 70 ]; then\necho "::error::astraweave-ai mutation score ${SCORE}% is below the 70% target"\nexit 1\nfi\nfi\necho "" >> "$GITHUB_STEP_SUMMARY"\necho "**Target**: 80%+ mutation score on P0, 70%+ on P1" >> "$GITHUB_STEP_SUMMARY"\necho "" >> "$GITHUB_STEP_SUMMARY"\necho "Detailed results available in workflow artifacts." >> "$GITHUB_STEP_SUMMARY"',
    'set -euo pipefail\nif [ "${{ github.event_name }}" != "pull_request" ]; then\n{\necho "run=true"\necho "reason=non-pull-request"\necho "full=true"\necho "packages="\n} >> "$GITHUB_OUTPUT"\nelse\npython3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope net --github-output "$GITHUB_OUTPUT"\nfi\ncat "$GITHUB_OUTPUT"',
    'sudo apt-get update\nsudo apt-get install -y libx11-dev libxcursor-dev libxrandr-dev libxi-dev libasound2-dev mesa-vulkan-drivers libvulkan1 libudev-dev pkg-config',
    'mark() {\nif [ "$1" = "success" ]; then\nprintf \'%s\' "✅"\nelse\nprintf \'%s\' "❌"\nfi\n}\n{\necho "## PBR Test Summary"\necho\necho "### Tested Components:"\necho "- $(mark "${{ steps.quiet.outcome }}") CountingAlloc (alloc, alloc_zeroed, realloc)"\necho "- $(mark "${{ steps.floor.outcome }}") draw_into allocation floor (lavapipe)"\necho "- $(mark "${{ steps.fallback.outcome }}") streamed fallback floor (lavapipe)"\necho "- $(mark "${{ steps.render.outcome }}") astraweave-render (core PBR)"\necho "- $(mark "${{ steps.terrain.outcome }}") Terrain material system (Phase PBR-F)"\necho "- $(mark "${{ steps.advanced.outcome }}") Advanced materials (Phase PBR-E)"\necho\necho "See job logs for detailed test results."\n} >> "$GITHUB_STEP_SUMMARY"',
    'echo "## 📜 WGSL Shaders" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "### Shader Files:" >> $GITHUB_STEP_SUMMARY\nfor shader in shaders/*.wgsl; do\nlines=$(wc -l < "$shader" 2>/dev/null || echo "?")\necho "- \\`$(basename $shader)\\` (${lines} lines)" >> $GITHUB_STEP_SUMMARY\ndone',
    'sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools',
    'set -euo pipefail\nif [ "${{ github.event_name }}" != "pull_request" ]; then\n{\necho "run=true"\necho "reason=non-pull-request"\necho "full=true"\necho "packages="\n} >> "$GITHUB_OUTPUT"\nelse\npython3 scripts/ci/affected_crates.py --base "origin/${{ github.base_ref }}" --head HEAD --scope render-device --github-output "$GITHUB_OUTPUT"\nfi\ncat "$GITHUB_OUTPUT"',
    'sudo apt-get update && sudo apt-get install -y mesa-vulkan-drivers libvulkan1 pkg-config libudev-dev',
    'sudo apt-get update\nsudo apt-get install -y pkg-config libudev-dev',
    'set +e\nP0_CRATES=(\nastraweave-ecs\nastraweave-physics\nastraweave-core\nastraweave-ai\nastraweave-nav\n)\nP1_CRATES=(\nastraweave-net\nastraweave-asset\nastraweave-memory\nastraweave-context\n)\nP2_CRATES=(\nastraweave-behavior\nastraweave-prompts\nastraweave-embeddings\n)\necho "=== Running ASan on ${#P0_CRATES[@]}+${#P1_CRATES[@]}+${#P2_CRATES[@]} crates ==="\nP0_FAILED=0\nP0_PASSED=0\nADVISORY_FAILED=0\nADVISORY_PASSED=0\nFAILED_LIST=""\nfor crate in "${P0_CRATES[@]}"; do\necho ""\necho "::group::[P0 CRITICAL] Testing $crate with ASan"\nif RUSTFLAGS="-Zsanitizer=address" cargo +$AW_NIGHTLY test -p "$crate" --target x86_64-unknown-linux-gnu --lib -- --test-threads=1; then\necho "✅ $crate passed ASan"\nP0_PASSED=$((P0_PASSED + 1))\nelse\necho "❌ $crate FAILED ASan (P0 critical — will fail job)"\nP0_FAILED=$((P0_FAILED + 1))\nFAILED_LIST="$FAILED_LIST - ❌ **$crate** (P0 Critical)\\n"\nfi\necho "::endgroup::"\ndone\nfor crate in "${P1_CRATES[@]}" "${P2_CRATES[@]}"; do\necho ""\necho "::group::[Advisory] Testing $crate with ASan"\nif RUSTFLAGS="-Zsanitizer=address" cargo +$AW_NIGHTLY test -p "$crate" --target x86_64-unknown-linux-gnu --lib -- --test-threads=1; then\necho "✅ $crate passed ASan"\nADVISORY_PASSED=$((ADVISORY_PASSED + 1))\nelse\necho "⚠️ $crate had ASan failures (advisory — not blocking)"\nADVISORY_FAILED=$((ADVISORY_FAILED + 1))\nFAILED_LIST="$FAILED_LIST - ⚠️ $crate (Advisory)\\n"\nfi\necho "::endgroup::"\ndone\necho "## ASan Full Suite Results" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "| Tier | Passed | Failed |" >> $GITHUB_STEP_SUMMARY\necho "|------|--------|--------|" >> $GITHUB_STEP_SUMMARY\necho "| P0 Critical | $P0_PASSED | $P0_FAILED |" >> $GITHUB_STEP_SUMMARY\necho "| Advisory (P1+P2) | $ADVISORY_PASSED | $ADVISORY_FAILED |" >> $GITHUB_STEP_SUMMARY\nif [ -n "$FAILED_LIST" ]; then\necho "" >> $GITHUB_STEP_SUMMARY\necho "### Failed Crates" >> $GITHUB_STEP_SUMMARY\necho -e "$FAILED_LIST" >> $GITHUB_STEP_SUMMARY\nfi\necho ""\necho "=== ASan Full Suite Summary ==="\necho "P0 crates: ${#P0_CRATES[@]} (passed: $P0_PASSED, failed: $P0_FAILED)"\necho "Advisory crates: $(( ${#P1_CRATES[@]} + ${#P2_CRATES[@]} )) (passed: $ADVISORY_PASSED, failed: $ADVISORY_FAILED)"\nif [ "$P0_FAILED" -gt 0 ]; then\necho "❌ P0 critical crate(s) failed ASan — failing job"\nexit 1\nfi\necho "✅ All P0 crates passed ASan"',
    'if [ ! -f scripts/tsan_suppressions.txt ]; then\necho "::error::scripts/tsan_suppressions.txt is missing — commit it to the repo"\nexit 1\nfi\necho "✅ TSan suppressions file found ($(wc -l < scripts/tsan_suppressions.txt) lines)"',
    'echo "Checking for non-deterministic HashMap iteration in source code..."\necho "(HashMap iteration order is non-deterministic — use BTreeMap instead)"\necho ""\nSEARCH_DIRS=""\nfor d in astraweave-ecs/src astraweave-physics/src astraweave-weaving/src astraweave-terrain/src astraweave-ai/src astraweave-core/src astraweave-nav/src astraweave-gameplay/src; do\n[ -d "$d" ] && SEARCH_DIRS="$SEARCH_DIRS $d"\ndone\nif [ -z "$SEARCH_DIRS" ]; then\necho "⚠️ No source directories found to scan"\nexit 0\nfi\nFOUND=$(grep -rn "use std::collections::HashMap" --include="*.rs" $SEARCH_DIRS 2>/dev/null | head -30 || true)\nif [ -n "$FOUND" ]; then\necho "⚠️ HashMap imports found in determinism-critical code:"\necho "$FOUND"\necho ""\necho "Consider migrating to BTreeMap for deterministic iteration."\necho "## Determinism: HashMap Audit" >> $GITHUB_STEP_SUMMARY\necho \'```\' >> $GITHUB_STEP_SUMMARY\necho "$FOUND" >> $GITHUB_STEP_SUMMARY\necho \'```\' >> $GITHUB_STEP_SUMMARY\nelse\necho "✅ No HashMap imports found in critical paths"\nfi',
    'echo "=== Sanitizer Workflow Summary ==="\necho ""\necho "source-integrity: ${{ needs.source-integrity.result }}"\necho "asan-critical: ${{ needs.asan-critical.result }}"\necho "asan-full: ${{ needs.asan-full.result }}"\necho "leak-sanitizer: ${{ needs.leak-sanitizer.result }}"\necho "thread-sanitizer: ${{ needs.thread-sanitizer.result }}"\necho "determinism: ${{ needs.determinism.result }}"\necho ""\necho "## Sanitizer Workflow Summary" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "| Job | Result | Blocking? |" >> $GITHUB_STEP_SUMMARY\necho "|-----|--------|-----------|" >> $GITHUB_STEP_SUMMARY\nresult_icon() {\ncase "$1" in\nsuccess) echo "✅ Pass" ;;\nfailure) echo "❌ Fail" ;;\nskipped) echo "⏭️ Skipped" ;;\ncancelled) echo "🚫 Cancelled" ;;\n*) echo "❓ $1" ;;\nesac\n}\necho "| Source Integrity | $(result_icon "${{ needs.source-integrity.result }}") | Yes |" >> $GITHUB_STEP_SUMMARY\necho "| ASan (Critical) | $(result_icon "${{ needs.asan-critical.result }}") | Yes |" >> $GITHUB_STEP_SUMMARY\necho "| ASan (Full) | $(result_icon "${{ needs.asan-full.result }}") | Yes |" >> $GITHUB_STEP_SUMMARY\necho "| LeakSanitizer | $(result_icon "${{ needs.leak-sanitizer.result }}") | Yes |" >> $GITHUB_STEP_SUMMARY\necho "| ThreadSanitizer | $(result_icon "${{ needs.thread-sanitizer.result }}") | No (advisory) |" >> $GITHUB_STEP_SUMMARY\necho "| Determinism | $(result_icon "${{ needs.determinism.result }}") | Yes |" >> $GITHUB_STEP_SUMMARY\necho "" >> $GITHUB_STEP_SUMMARY\necho "_Run triggered by: \\`${{ github.event_name }}\\` at $(date -u)_" >> $GITHUB_STEP_SUMMARY\nFAILED=0\nif [ "${{ needs.source-integrity.result }}" = "failure" ]; then\necho "❌ Source integrity check FAILED (cargo-mutants artifacts detected)"\nFAILED=1\nfi\nif [ "${{ needs.asan-critical.result }}" = "failure" ]; then\necho "❌ ASan critical crates FAILED"\nFAILED=1\nfi\nif [ "${{ needs.asan-full.result }}" = "failure" ]; then\necho "❌ ASan full suite FAILED"\nFAILED=1\nfi\nif [ "${{ needs.leak-sanitizer.result }}" = "failure" ]; then\necho "❌ LeakSanitizer FAILED"\nFAILED=1\nfi\nif [ "${{ needs.determinism.result }}" = "failure" ]; then\necho "❌ Determinism validation FAILED"\nFAILED=1\nfi\nif [ "${{ needs.thread-sanitizer.result }}" = "failure" ]; then\necho "⚠️ ThreadSanitizer had failures (advisory — not blocking)"\nfi\nif [ "$FAILED" -eq 1 ]; then\necho ""\necho "❌ One or more critical sanitizer jobs failed"\nexit 1\nfi\necho ""\necho "✅ All critical sanitizer checks passed"',
    'cat > c_harness.c << \'EOF\'\nstatic void cb(const char* s){ if(s) fprintf(stderr, "SNAP:%s\\n", s); }\nint main(){\nstruct AWVersion v = aw_version();\nprintf("%u.%u.%u\\n", v.major, v.minor, v.patch);\n// lightly exercise world and callback\nstruct AWWorld w = aw_world_create();\naw_world_set_snapshot_callback(w, cb);\naw_world_tick(w, 0.016f);\naw_world_destroy(w);\nreturn 0;\n}\ncc -I. -L target/release -Wl,-rpath,\'$ORIGIN/target/release\' -o c_harness c_harness.c -lastraweave_sdk -lpthread -ldl -lm\nLD_LIBRARY_PATH=target/release ./c_harness | tee harness_out.txt\nVER=$(awk \' /^\\[package\\]/ { in_pkg=1; next } /^\\[/ { in_pkg=0 } in_pkg && /^version[[:space:]]*=[[:space:]]*"/ { sub(/^version[[:space:]]*=[[:space:]]*"/, "") sub(/".*/, "") print exit } \' astraweave-sdk/Cargo.toml)\nOUT=$(head -n1 harness_out.txt)\necho "SDK Crate Version: $VER; Runtime: $OUT"\ntest "$OUT" = "$VER"',
    "mkdir -p cmake_example\ncat > cmake_example/CMakeLists.txt << 'EOF'\ncmake_minimum_required(VERSION 3.15)\nproject(aw_cmake_example C)\nadd_executable(aw_cmake_example main.c)\ntarget_include_directories(aw_cmake_example PRIVATE ${CMAKE_SOURCE_DIR}/..)\ntarget_link_directories(aw_cmake_example PRIVATE ${CMAKE_SOURCE_DIR}/../target/release)\nif(UNIX)\ntarget_link_libraries(aw_cmake_example PRIVATE astraweave_sdk pthread dl m)\nelse()\ntarget_link_libraries(aw_cmake_example PRIVATE astraweave_sdk)\nendif()\ncat > cmake_example/main.c << 'EOF'\nint main(){ struct AWWorld w = aw_world_create(); aw_world_tick(w, 0.016f); aw_world_destroy(w); return 0; }\ncmake -S cmake_example -B cmake_build\ncmake --build cmake_build --config Release\nLD_LIBRARY_PATH=target/release ./cmake_build/aw_cmake_example",
    'sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libasound2-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools',
    'log="target/gpu-skip-reasons.txt"\nif [ -s "$log" ]; then\n{\necho "## GPU tests skipped"\necho\necho "These tests did not execute. A line here is a skip, not a pass."\necho\n} >> "$GITHUB_STEP_SUMMARY"\nwhile IFS= read -r line; do\necho "- ${line}" >> "$GITHUB_STEP_SUMMARY"\necho "::warning title=GPU test skipped::$line"\necho "$line"\ndone < "$log"\nfi',
    'if grep -q "rust-version" Cargo.toml; then\nMSRV=$(grep "rust-version" Cargo.toml | cut -d\'"\' -f2)\nelse\nMSRV=$(grep "channel" rust-toolchain.toml | cut -d\'"\' -f2)\nfi\necho "MSRV=$MSRV" >> $GITHUB_ENV\necho "Detected MSRV: $MSRV"',
    'sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libasound2-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools time',
    'echo "=== Compilation Performance for ${{ matrix.toolchain }} ===" > perf-${{ matrix.toolchain }}.txt\necho "" >> perf-${{ matrix.toolchain }}.txt\ncargo clean\necho "Clean build:" >> perf-${{ matrix.toolchain }}.txt\n/usr/bin/time -f "Real: %E, User: %U, System: %S, Memory: %M KB" cargo build --release -p astraweave-core -p astraweave-ai -p astraweave-physics -p hello_companion 2>&1 | tail -1 >> perf-${{ matrix.toolchain }}.txt\necho "Incremental build:" >> perf-${{ matrix.toolchain }}.txt\n/usr/bin/time -f "Real: %E, User: %U, System: %S, Memory: %M KB" cargo build --release -p astraweave-core -p astraweave-ai -p astraweave-physics -p hello_companion 2>&1 | tail -1 >> perf-${{ matrix.toolchain }}.txt',
))

_EXACT_BODIES = (
    frozenset(_exact_entry(item) for item in _ALLOWED_SCRIPTS_CI_LINES)
    | _REVIEWED_EXACT_BODIES
)


def _pwsh_indirect(body: str) -> bool:
    if re.search(r"(^|[\n;])\s*&(\s|$)", body) is not None:
        return True
    return re.search(r"(?i)(?:^|[\s;])(?:Invoke-Expression|iex)(?:\s|$)", body) is not None


def _run_body_violations(
    label: str,
    body: str,
    env: dict[str, str],
    workdir: str,
    shell: str | None = None,
) -> list[str]:
    violations: list[str] = []
    normalized = _normalized_body(body)
    if shell == "pwsh":
        if normalized in _EXACT_BODIES:
            return []
        if _mentions_scripts_ci(body):
            violations.append(f"{label} is not an allowlisted scripts/ci command")
        elif _pwsh_indirect(body):
            violations.append(f"{label} could not classify the command")
        return violations
    if shell is not None and shell != "bash" and not _body_has_fetch_pin(body):
        violations.append(f"{label} shell is not bash or pwsh")
        return violations
    directory_changed = _body_has_fetch_pin(body) and _fetch_changes_directory(body)
    if normalized in _EXACT_BODIES and directory_changed:
        violations.append(f"{label} fetch step sets working-directory")
        return violations
    if normalized in _EXACT_BODIES and (not workdir or (normalized, workdir) in _EXACT_WORKDIRS):
        return []
    visible = "\n".join(
        _strip_shell_comment(line).strip() for line in _logical_shell_lines(body)
    )
    if _mentions_scripts_ci(visible):
        violations.append(f"{label} is not an allowlisted scripts/ci command")
        return violations
    cwd = _cwd_parts(workdir)
    local = dict(env)
    for raw_line in _logical_shell_lines(body):
        command = _strip_shell_comment(raw_line).strip()
        if not command:
            continue
        for part in _split_commands(command):
            named, local, cwd = _command_names_scripts_ci(part, local, cwd)
            if named:
                violations.append(f"{label} is not an allowlisted scripts/ci command")
                return violations
            if _command_indirection(part):
                violations.append(f"{label} could not classify the command")
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


_FETCH_JOB_ENV_ALLOW = frozenset(
    {
        "AW_GITHUB_EVENT_NAME",
        "AW_GITHUB_REF",
        "CARGO_INCREMENTAL",
        "CARGO_NET_RETRY",
        "CARGO_TERM_COLOR",
        "EXCLUDED_PACKAGES",
        "RUST_BACKTRACE",
        "SCCACHE_CACHE_SIZE",
        "SCCACHE_IDLE_TIMEOUT",
    }
)

# Keys a fetch-job run may append to GITHUB_ENV. Startup keys are never here.
_GITHUB_ENV_KEYS = frozenset(
    {
        "AW_GPU_SKIP_IF_UNAVAILABLE",
        "AW_GPU_SKIP_LOG",
        "AW_NIGHTLY",
        "CARGO_INCREMENTAL",
        "MSRV",
        "RUSTC_WRAPPER",
        "RUSTFLAGS",
        "SCCACHE_CACHE_SIZE",
        "SCCACHE_GHA_ENABLED",
        "SCCACHE_GHA_RW_MODE",
        "SCCACHE_IDLE_TIMEOUT",
    }
)
_STARTUP_ENV_KEY = re.compile(
    r"^(?:BASH_ENV|ENV|SHELLOPTS|BASHOPTS|BASH_FUNC_.*|LD_.*|NODE_OPTIONS|PATH|"
    r"GCONV_PATH|IFS|CDPATH|PYTHONPATH|PERL5OPT|RUBYOPT|DYLD_.*)$"
)
_ENV_REDIRECT = re.compile(
    r"(?:>>|>)\s*['\"]?\$\{?GITHUB_(ENV|PATH)\b|"
    r"[|]\s*tee(?:\s+-a)?\s+['\"]?\$\{?GITHUB_(ENV|PATH)\b"
)
_EXACT_WORKDIRS: frozenset[tuple[str, str]] = frozenset(
    (
        (
            "./aravath_greybox.exe --smoke-test --no-volumetrics",
            "dist/aravath_greybox",
        ),
        (
            "./aravath_greybox.exe --smoke-test --volumetrics",
            "dist/aravath_greybox",
        ),
    )
)
_PRE_FETCH_STEPS: frozenset[str] = frozenset((
    'uses:dtolnay/rust-toolchain@master{components=${{ inputs.components }},toolchain=${{ inputs.toolchain }}}',
    'run:bash::sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools',
    'run:bash::if ! command -v cmake >/dev/null 2>&1 || ! command -v pkg-config >/dev/null 2>&1; then\nbrew install pkg-config cmake\nfi',
    'uses:actions/checkout@v7{fetch-depth=0}',
    'uses:dtolnay/rust-toolchain@1.89.0',
    'run:::sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools bc jq coreutils',
    'uses:actions/checkout@v7',
    'run:::sudo apt-get update -q\nsudo apt-get install -y -q build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev libasound2-dev libpulse-dev libudev-dev',
    'run:::brew install pkg-config cmake',
    'run:::echo "Windows dependencies managed via vcpkg"',
    'uses:actions/checkout@v7{lfs=True}',
    'uses:./.github/actions/rust-nightly{components=rustfmt, clippy, rust-src, rust-analyzer}',
    'uses:dtolnay/rust-toolchain@master{components=rustfmt, clippy, rust-src, rust-analyzer,toolchain=${{ github.event.inputs.toolchain_version || matrix.toolchain }}}',
    'run:::sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libasound2-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools',
    'uses:dtolnay/rust-toolchain@master{toolchain=${{ matrix.toolchain }}}',
    'run:::sudo apt-get update\nsudo apt-get install -y build-essential pkg-config cmake ninja-build libx11-dev libxi-dev libxcursor-dev libxrandr-dev libasound2-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools time',
))
_REMOTE_USES_RE = re.compile(r"^[^./\s][^/\s]*(?:/[^/\s@]+)+@\S+$")


def _env_file_write_violations(label: str, body: str) -> list[str]:
    violations: list[str] = []
    for line in body.splitlines():
        match = _ENV_REDIRECT.search(line)
        if match is None:
            continue
        kind = match.group(1) or match.group(2)
        if kind == "PATH":
            violations.append(f"{label} writes GITHUB_PATH")
            continue
        keys = re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)=", line)
        if not keys:
            violations.append(f"{label} writes GITHUB_ENV")
            continue
        for key in keys:
            if _STARTUP_ENV_KEY.fullmatch(key) or key not in _GITHUB_ENV_KEYS:
                violations.append(f"{label} writes {key}")
    return violations


def _pre_fetch_key(step: dict) -> str:
    if "uses" in step and "run" in step:
        return "mixed"
    if "uses" in step:
        uses = step.get("uses")
        if not isinstance(uses, str):
            return "uses:invalid"
        target = uses.strip().split()[0]
        if "with" not in step:
            return "uses:" + target
        with_value = step.get("with")
        if not isinstance(with_value, dict):
            return "uses:bad-with"
        parts = [f"{key}={with_value[key]}" for key in sorted(with_value)]
        return "uses:" + target + "{" + ",".join(parts) + "}"
    run = step.get("run")
    if not isinstance(run, str):
        return "unclassified"
    shell = step.get("shell") if isinstance(step.get("shell"), str) else ""
    workdir = (
        step.get("working-directory")
        if isinstance(step.get("working-directory"), str)
        else ""
    )
    return "run:" + shell + ":" + workdir + ":" + _normalized_body(run)


def _classify_uses(label: str, uses: object, known_actions: frozenset[str]) -> None:
    if not isinstance(uses, str) or not uses.strip():
        raise _PolicyScanError(f"{label} uses")
    text = uses.strip().split()[0]
    local = text.startswith("./") or text.startswith(".github/")
    if (
        text.startswith("/")
        or text.startswith("../")
        or (not local and not _REMOTE_USES_RE.match(text))
    ):
        raise _PolicyScanError(f"{label} uses")
    if not local:
        return
    path = text[2:] if text.startswith("./") else text
    if (
        not path
        or "://" in path
        or path.startswith("../")
        or "/../" in f"/{path}"
        or path.endswith("/..")
    ):
        raise _PolicyScanError(f"{label} uses")
    if path.endswith(("/action.yml", "/action.yaml")):
        candidate = path
    else:
        candidate = path.rstrip("/") + "/action.yml"
    alt = ""
    if candidate.endswith(".yml"):
        alt = candidate[:-4] + "yaml"
    elif candidate.endswith(".yaml"):
        alt = candidate[:-5] + "yml"
    if candidate not in known_actions and alt not in known_actions:
        raise _PolicyScanError(f"{label} uses")


def _fetch_shell_env_violations(where: str, env: dict[str, str]) -> list[str]:
    return [
        f"{where} sets {key}"
        for key in sorted(key for key in env if key not in _FETCH_JOB_ENV_ALLOW)
    ]


def _with_violations(label: str, value: object) -> list[str]:
    if not isinstance(value, dict):
        raise _PolicyScanError(f"{label} with")
    violations: list[str] = []
    for item in value.values():
        if isinstance(item, str):
            if _mentions_scripts_ci(item):
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
    known_actions: frozenset[str],
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
    if "uses" in step:
        _classify_uses(label, step.get("uses"), known_actions)
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
    if _body_has_fetch_pin(run):
        found = 1
        if "env" in step:
            violations.append(f"{label} fetch step sets env")
        if "working-directory" in step or _fetch_changes_directory(run):
            violations.append(f"{label} fetch step sets working-directory")
        if "shell" in step and step.get("shell") != "bash":
            violations.append(f"{label} fetch step shell is not bash")
        elif "shell" not in step and default_shell not in (None, "bash"):
            violations.append(f"{label} fetch step shell is not bash")
    violations.extend(_run_body_violations(label, run, env, workdir, shell))
    return violations, found


def _scan_document(
    label: str, data: object, known_actions: frozenset[str] | None = None
) -> tuple[list[str], int]:
    if not isinstance(data, dict):
        raise _PolicyScanError("root")
    if known_actions is None:
        known_actions = frozenset(_policy_names())
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
            step_label, step, job_env, workflow_env, job_shell, None, known_actions
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
        seen_fetch: set[str] = set()
        for step_label, step, _job_env, _job_shell, job_id in indexed:
            if job_id not in fetch_jobs or not isinstance(step, dict):
                continue
            run = step.get("run")
            if isinstance(run, str):
                violations.extend(_env_file_write_violations(step_label, run))
            if job_id in seen_fetch:
                continue
            if isinstance(run, str) and _body_has_fetch_pin(run):
                seen_fetch.add(job_id)
                continue
            if _pre_fetch_key(step) not in _PRE_FETCH_STEPS:
                violations.append(f"{step_label} is not an allowlisted pre-fetch step")
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
            data = _load_policy_yaml(text)
        except yaml.YAMLError:
            violations.append(f"{label}: could not classify the workflow")
            continue
        if data is None:
            violations.append(f"{label}: could not classify the workflow")
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
        '"$GITHUB_WORKSPACE/scripts/ci/fetch_sccache_release.sh"', replacement
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
            _bind_fetch_node(script, work / "bin" / "node")
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
            _bind_fetch_node(script, work / "bin" / "node")
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
        body = b"archive-bytes"
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_pinned_to(work, hashlib.sha256(body).hexdigest())
            _write_exe(work / "bin", "node", node)
            _bind_fetch_node(script, work / "bin" / "node")
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
        self.assertIn("archive has a symlink or non-regular entry", result.stderr)
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
            created = work / "mktemp.log"
            _write_rm_that_refuses_tree_wipe(work / "bin")
            _write_exe(
                work / "bin",
                "mktemp",
                "#!/bin/sh\n"
                "out=$(/usr/bin/mktemp \"$@\") || exit $?\n"
                f"printf '%s\\n' \"$out\" >> {shlex.quote(str(created))}\n"
                "printf '%s\\n' \"$out\"\n",
            )
            _bind_tool(script, "/usr/bin/rm", work / "bin" / "rm")
            _bind_tool(script, "/usr/bin/mktemp", work / "bin" / "mktemp")
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
                staged = []
                if created.is_file():
                    for line in created.read_text(encoding="utf-8").splitlines():
                        path = Path(line)
                        if path.is_dir():
                            staged.extend(
                                str(item) for item in path.rglob("*") if item.is_file()
                            )
                        shutil.rmtree(path, ignore_errors=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertIn(filename, result.stderr)
        self.assertNotIn("available=false", output)
        self.assertNotIn("available=true", output)
        self.assertNotIn("unexpected error", result.stderr)
        self.assertFalse(marker_exists)
        self.assertFalse(cached_exists)
        self.assertFalse(any(Path(path).name == "sccache" for path in staged), staged)

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
        self.assertEqual(script_text.count("fs.readFileSync(archive)"), 1)
        self.assertIn('read_archive "$tmp/pkg.tar.gz" stage', script_text)
        self.assertNotIn('"$TAR_BIN"', script_text)
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
        self.assertEqual(script_text.count('read_archive "$tmp/pkg.tar.gz" stage'), 1)
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
        real_node = _fixture_node()
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
                _bind_fetch_node(script, work / "bin" / "node")
                _bind_tool(script, "/c/Windows/System32/curl.exe", Path("/usr/bin/curl"))
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
                _bind_fetch_node(script, work / "bin" / "node")
                _bind_tool(script, "/c/Windows/System32/curl.exe", Path("/usr/bin/curl"))
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
            _write_stage_planter(work, 'ln -s "$AW_OUTSIDE" "$dest/planted"')
            _bind_fetch_node(script, work / "bin" / "node")
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
            _write_stage_planter(
                work,
                "find \"$dest\" -type f -name sccache -exec sh -c "
                "'rm -f \"$1\"; ln -s /etc/hostname \"$1\"' sh {} \\;",
            )
            _bind_fetch_node(script, work / "bin" / "node")
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
                        message="archive does not contain exactly one release root",
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
        real_node = _fixture_node()
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
            _bind_fetch_node(script, work / "bin" / "node")
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
            _write_stage_planter(
                work,
                "find \"$dest\" -type f -name sccache -exec sh -c "
                "'ln \"$1\" \"$(dirname \"$1\")/sccache-alias\"' sh {} \\;",
            )
            _bind_fetch_node(script, work / "bin" / "node")
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

    def test_sccache_prefetch_rejects_a_space_padded_size(self) -> None:
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
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )

    def test_sccache_prefetch_reader_rejects_a_blank_size(self) -> None:
        _filename, _arch, root = self._release_root()
        member = _ustar_header(
            root.encode() + b"/sccache", 48, 0, size_field=b" " * 12
        )
        blob = _gzip_tar([member])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)

    def test_sccache_prefetch_reader_accepts_a_nul_padded_size(self) -> None:
        _filename, _arch, root = self._release_root()
        member = _ustar_header(
            root.encode() + b"/sccache", 48, 0, size_field=b"\0" * 12
        )
        blob = _gzip_tar([member, b"\0" * 1024])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 0)

    def test_sccache_prefetch_reader_rejects_a_high_bit_size(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"H" * 80
        field = bytes([0x80]) + b"\0" * 11
        directory = _ustar_header(root.encode() + b"/", 53, 0)
        member = _ustar_header(
            root.encode() + b"/sccache", 48, 80, size_field=field
        )
        blob = _gzip_tar([directory, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )

    def test_sccache_prefetch_reader_rejects_a_non_octal_checksum(self) -> None:
        _filename, _arch, root = self._release_root()
        header = bytearray(_ustar_header(root.encode() + b"/sccache", 48, 0))
        header[148:156] = b"xxxxxxxx"
        blob = _gzip_tar([bytes(header)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)

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
        real_node = shlex.quote(_fixture_node())
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
            _bind_fetch_node(script, work / "bin" / "node")
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
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("archive does not contain exactly one release root", result.stderr)
        self.assertFalse(cached.exists())

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

        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            called = work / "cygpath.log"
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cygpath-abs",
                "#!/bin/sh\n"
                f"printf '%s\\n' called >> {shlex.quote(str(called))}\n"
                "exit 1\n",
            )
            _bind_tool(script, "/usr/bin/cygpath", work / "bin" / "cygpath-abs")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"RUNNER_OS": "Linux"},
                )
            absolute = called.read_text(encoding="utf-8") if called.is_file() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertEqual(absolute, "")

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
                        '"$GITHUB_WORKSPACE/scripts/ci/fetch_sccache_release.sh"',
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
        for name in ("curl", "mktemp", "rm", "chmod", "mkdir", "sleep", "cp"):
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
        shellopts = probe(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                env:
                  SHELLOPTS: braceexpand
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        self.assertTrue(any("sets BASH_ENV" in item for item in workflow_env), workflow_env)
        self.assertTrue(any("sets ENV" in item for item in job_env), job_env)
        self.assertTrue(any("sets BASH_ENV" in item for item in later_step), later_step)
        self.assertTrue(any("sets SHELLOPTS" in item for item in shellopts), shellopts)
        self.assertEqual(sibling, [])
        self.assertEqual(unrelated, [])

    def test_sccache_fetch_fail_closed_arms(self) -> None:
        pin = SCCACHE_FETCH_COMMAND

        def probe(body: str) -> list[str]:
            return sccache_fetch_run_violations(
                [("probe.yml", textwrap.dedent(body).lstrip())]
            )

        def job(run: str, *, shell: str = "bash", workdir: str | None = None) -> list[str]:
            lines = [
                "name: probe",
                "on: push",
                "jobs:",
                "  build:",
                "    runs-on: ubuntu-latest",
                "    steps:",
                f'      - shell: "{shell}"',
            ]
            if workdir is not None:
                lines.append(f"        working-directory: {workdir}")
            lines.append("        run: |")
            lines.extend(f"          {line}" for line in run.splitlines())
            return probe("\n".join(lines) + "\n")

        parsed = sccache_fetch_run_violations([("probe.yml", "jobs: [\n")])
        self.assertTrue(any("could not classify" in item for item in parsed), parsed)
        mapped = probe(
            """
            name: probe
            on: push
            env: nope
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run: echo hi
            """
        )
        self.assertTrue(any("could not classify" in item for item in mapped), mapped)
        listed = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run:
                      - echo
                      - hi
            """
        )
        self.assertTrue(any("could not classify" in item for item in listed), listed)
        numbered = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - run: 1
            """
        )
        self.assertTrue(any("could not classify" in item for item in numbered), numbered)
        directory = job(pin, workdir="/tmp")
        self.assertTrue(any("working-directory" in item for item in directory), directory)
        changed = job(f"cd /tmp\n{pin}")
        self.assertTrue(any("working-directory" in item for item in changed), changed)
        unclassified = job('cd "$D"\nbash ./x.sh')
        self.assertTrue(any("allowlisted" in item for item in unclassified), unclassified)
        powershell = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: pwsh
                    run: echo scripts/ci/other.sh
            """
        )
        self.assertTrue(any("allowlisted" in item for item in powershell), powershell)
        parent = job("cd scripts/x/../ci\nbash ./other.sh")
        self.assertTrue(any("allowlisted" in item for item in parent), parent)
        option = job('bash -x "$F"')
        self.assertTrue(any("allowlisted" in item for item in option), option)
        boundary = job("python3 scripts/ci/affected_crates.pyX")
        self.assertTrue(any("allowlisted" in item for item in boundary), boundary)
        for shell in ("sh", "bash {0}", "bash -x {0}"):
            with self.subTest(shell=shell):
                found = probe(
                    f"""
                    name: probe
                    on: push
                    jobs:
                      build:
                        runs-on: ubuntu-latest
                        steps:
                          - shell: {shell}
                            run: {pin}
                    """
                )
                self.assertTrue(any("shell is not bash" in item for item in found), found)
        globbed = job("timeout 60 scr?pts/ci/other.sh")
        self.assertTrue(any("allowlisted" in item for item in globbed), globbed)
        chained = job("echo hi && X=scripts && bash $X/ci/other.sh")
        self.assertTrue(any("allowlisted" in item for item in chained), chained)
        assigned = job("X=scripts echo $X/ci/other.sh")
        self.assertTrue(any("allowlisted" in item for item in assigned), assigned)
        for label, body in (
            (
                "step",
                """
                name: probe
                on: push
                jobs:
                  other:
                    runs-on: ubuntu-latest
                    steps:
                      - run: echo hi
                        env:
                          BASH_ENV: scripts/ci/other.sh
                """,
            ),
            (
                "job",
                """
                name: probe
                on: push
                jobs:
                  other:
                    runs-on: ubuntu-latest
                    env:
                      ENV: scripts/ci/other.sh
                    steps:
                      - run: echo hi
                """,
            ),
            (
                "workflow",
                """
                name: probe
                on: push
                env:
                  SHELLOPTS: scripts/ci/other.sh
                jobs:
                  other:
                    runs-on: ubuntu-latest
                    steps:
                      - run: echo hi
                """,
            ),
        ):
            with self.subTest(env=label):
                found = probe(body)
                self.assertTrue(any("allowlisted" in item for item in found), found)
        classes = {
            "exec": "exec echo hi",
            "env": "env echo hi",
            "xargs": "xargs echo",
            "find-exec": "find . -exec echo {} +",
            "pushd": "pushd /tmp",
            "popd": "popd",
            "cdpath": "CDPATH=/tmp echo hi",
            "dash-c": "bash -c 'echo hi'",
            "argv0": '"$D"/echo hi',
        }
        for name, command in classes.items():
            with self.subTest(command=name):
                found = job(command)
                self.assertTrue(any("could not classify" in item for item in found), found)
        names = _policy_names(
            [
                ".github/workflows/probe.yaml",
                ".github/actions/rust-nightly/action.yml",
                ".github/workflows/notes.md",
            ]
        )
        self.assertEqual(
            names,
            [
                ".github/workflows/probe.yaml",
                ".github/actions/rust-nightly/action.yml",
            ],
        )
        live = _policy_names()
        self.assertIn(".github/actions/rust-nightly/action.yml", live)
        self.assertIn(".github/actions/rust-ci-setup/action.yml", live)
        missing = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - uses: ./.github/actions/not-a-real-action
            """
        )
        self.assertTrue(any("could not classify" in item for item in missing), missing)
        noted = probe(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: |
                      {pin} # kept
            """
        )
        self.assertEqual(noted, [])

    def test_sccache_prefetch_refuses_a_nonzero_prefix(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        header = bytearray(
            _ustar_header(f"{root}/sccache".encode(), 48, len(payload), prefix=b"pre")
        )
        header[257:262] = b"\0\0\0\0\0"
        header[148:156] = b"        "
        total = sum(32 if 148 <= index < 156 else byte for index, byte in enumerate(header))
        header[148:156] = f"{total:06o}".encode() + b"\0 "
        blob = _gzip_tar([bytes(header), _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self,
            result,
            output,
            work,
            arch,
            message="archive has a symlink or non-regular entry",
        )

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

    def test_sccache_policy_refuses_the_review_command_classes(self) -> None:
        pin = SCCACHE_FETCH_COMMAND

        def probe(body: str) -> list[str]:
            return sccache_fetch_run_violations(
                [("probe.yml", textwrap.dedent(body).lstrip())]
            )

        def job(run: str) -> list[str]:
            return probe(
                f"""
                name: probe
                on: push
                jobs:
                  build:
                    runs-on: ubuntu-latest
                    steps:
                      - shell: bash
                        run: {run}
                """
            )

        # echo, not bash: a glob script argument is refused before fnmatch runs.
        # c* matches ci only when the name is the first fnmatch argument.
        for command in (
            "echo */ci/tool.sh",
            "echo scripts/*/tool.sh",
            "echo scripts/c*/tool.sh",
            "echo scripts/**/ci/tool.sh",
        ):
            with self.subTest(command=command):
                found = job(command)
                self.assertTrue(any("allowlisted" in item for item in found), found)
        self.assertEqual(job("echo dist/**/**/*.zip"), [])
        bashopts = probe(
            f"""
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                env:
                  BASHOPTS: extdebug
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        self.assertTrue(any("sets BASHOPTS" in item for item in bashopts), bashopts)
        func = probe(
            f"""
            name: probe
            on: push
            env:
              BASH_FUNC_echo%%: "() {{ :; }}"
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: {pin}
            """
        )
        self.assertTrue(any("sets BASH_FUNC_echo%%" in item for item in func), func)
        other_shell = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: python {0}
                    run: print(1)
            """
        )
        self.assertTrue(any("shell is not bash" in item for item in other_shell), other_shell)
        relative = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - uses: ./tools/act
            """
        )
        self.assertTrue(any("could not classify" in item for item in relative), relative)
        for label, text in (("empty", ""), ("null", "null\n")):
            with self.subTest(document=label):
                found = sccache_fetch_run_violations([(f"{label}.yml", text)])
                self.assertTrue(any("could not classify" in item for item in found), found)
        self.assertTrue(
            any("could not classify" in item for item in job("eval echo hi")),
            job("eval echo hi"),
        )
        self.assertTrue(
            any("could not classify" in item for item in job("/bin/eval echo hi")),
            job("/bin/eval echo hi"),
        )
        self.assertTrue(
            any("could not classify" in item for item in job("/usr/bin/sudo true")),
            job("/usr/bin/sudo true"),
        )
        self.assertTrue(
            any("could not classify" in item for item in job("command echo hi")),
            job("command echo hi"),
        )
        self.assertEqual(job("command -v cmake >/dev/null"), [])
        chained = job("echo hi && bash scripts/ci/run-gate.sh")
        self.assertTrue(any("allowlisted" in item for item in chained), chained)
        quoted = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: |
                      echo "scripts/c\\i/x"
            """
        )
        self.assertEqual(quoted, [])
        escaped = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - shell: bash
                    run: |
                      bash scripts/c\\i/other.sh
            """
        )
        self.assertTrue(any("allowlisted" in item for item in escaped), escaped)
        tainted = probe(
            """
            name: probe
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                  - working-directory: scripts/ci
                    shell: bash
                    run: |
                      cd /tmp
                      bash ./tool.sh
            """
        )
        self.assertTrue(any("allowlisted" in item for item in tainted), tainted)
        almost = job("echo " + pin)
        self.assertTrue(any("allowlisted" in item for item in almost), almost)
        self.assertFalse(any("expected" in item and "found 1" in item for item in almost))
        action = sccache_fetch_run_violations(
            [
                (
                    ".github/actions/demo/action.yaml",
                    "name: demo\nruns:\n  using: composite\n  steps:\n"
                    "    - shell: bash\n      run: bash scripts/ci/other.sh\n",
                )
            ],
            expected_fetch_count=0,
        )
        self.assertTrue(any("allowlisted" in item for item in action), action)

    def test_sccache_prefetch_uses_one_node_path(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            real = shlex.quote(_fixture_node())
            _write_exe(work / "bin", "good-node", f"#!/bin/sh\nexec {real} \"$@\"\n")
            _write_exe(work / "bin", "bad-node", "#!/bin/sh\nexit 1\n")
            text = script.read_text(encoding="utf-8")
            for literal in (
                "/opt/homebrew/bin/node",
                "/usr/bin/node",
                "/c/Program Files/nodejs/node.exe",
            ):
                text = text.replace(literal, str(work / "bin" / "bad-node"))
            text = text.replace("/usr/local/bin/node", str(work / "bin" / "good-node"))
            script.write_text(text, encoding="utf-8")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")

    def test_sccache_prefetch_win32_curl_is_system32(self) -> None:
        node = """#!/bin/sh
if [ "$1" = "-p" ] && [ "$2" = "process.platform" ]; then
  printf '%s\\n' win32
  exit 0
fi
if [ "$1" = "-p" ] && [ "$2" = "process.arch" ]; then
  printf '%s\\n' x64
  exit 0
fi
exit 1
"""
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            _write_exe(work / "bin", "node", node)
            _bind_fetch_node(script, work / "bin" / "node")
            result, output = _run_fetch(work, script=script)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=false\n")
        self.assertIn("curl is missing", result.stderr)
        self.assertTrue(Path("/usr/bin/curl").is_file())

    def test_sccache_prefetch_does_not_chmod_a_temp_symlink(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            log = work / "chmod.log"
            real_dir = work / "real-tmp"
            link = work / "link-tmp"
            real_dir.mkdir()
            link.symlink_to(real_dir, target_is_directory=True)
            _write_exe(
                work / "bin",
                "mktemp",
                "#!/bin/sh\n"
                "if [ \"$1\" = -d ] && [ -n \"$2\" ]; then\n"
                "  exec /usr/bin/mktemp \"$@\"\n"
                "fi\n"
                f"printf '%s\\n' {shlex.quote(str(link))}\n",
            )
            _write_exe(
                work / "bin",
                "chmod",
                "#!/bin/sh\n"
                f"printf '%s\\n' \"$*\" >> {shlex.quote(str(log))}\n"
                "exec /bin/chmod \"$@\"\n",
            )
            _bind_tool(script, "/usr/bin/mktemp", work / "bin" / "mktemp")
            _bind_tool(script, "/usr/bin/chmod", work / "bin" / "chmod")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            logged = log.read_text(encoding="utf-8") if log.is_file() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertNotIn(str(link), logged)

    def test_sccache_prefetch_writes_nothing_for_a_hostile_member(self) -> None:
        filename, arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")].encode()
        payload = b"ESCAPED-BYTES"
        cases = {
            "dotdot-file": (root + b"/../../ESCAPED", 48, payload),
            "dotdot-dir": (root + b"/../../ESCAPED", 53, b""),
            "absolute": (b"/tmp/aw-pr729-ESCAPED", 48, payload),
            "nul": (root + b"/sccache\0junk", 48, payload),
            "extra": (root + b"/notes.txt", 48, payload),
        }
        for name, (member, typ, data) in cases.items():
            for pinned in (False, True):
                with self.subTest(name=name, pinned=pinned):
                    with tempfile.TemporaryDirectory() as tmp:
                        work = Path(tmp)
                        outside = work / "ESCAPED"
                        header = _ustar_header(member, typ, 0 if typ == 53 else len(data))
                        parts = [header]
                        if typ != 53:
                            parts.append(_pad_block(data))
                        if name == "extra":
                            good = b"#!/bin/sh\necho sccache 0.19.0\n"
                            binary = _ustar_header(root + b"/sccache", 48, len(good))
                            parts = [binary, _pad_block(good), header, _pad_block(data)]
                        blob = _gzip_tar(parts)
                        if pinned:
                            result, output = _fetch_blob(work, blob)
                            self.assertEqual(output, "available=false\n")
                            self.assertEqual(result.returncode, 0, result.stderr)
                            if name == "extra":
                                self.assertIn(
                                    "exactly one release root", result.stderr
                                )
                            else:
                                self.assertIn(
                                    "symlink or non-regular", result.stderr
                                )
                        else:
                            script = _script_copy(work)
                            with _fixed_body_server(blob, status=200) as server:
                                port = server.server_address[1]
                                result, output = _run_fetch(
                                    work,
                                    script=script,
                                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                                )
                            self.assertEqual(result.returncode, 1, result.stderr)
                            self.assertIn("checksum mismatch", result.stderr)
                        self.assertFalse(outside.exists())
                        self.assertFalse((work / "tool-cache").exists())
                        self.assertFalse(Path("/tmp/aw-pr729-ESCAPED").exists())

    def test_sccache_prefetch_masks_setuid_mode_bits(self) -> None:
        filename, arch = _host_sccache_filename()
        root = filename[: -len(".tar.gz")]
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        directory = _ustar_header(f"{root}/".encode(), 53, 0, mode=0o4755)
        member = _ustar_header(f"{root}/sccache".encode(), 48, len(payload), mode=0o4755)
        blob = _gzip_tar([directory, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            log = work / "mode"
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _write_exe(
                work / "bin",
                "cp",
                "#!/bin/sh\n"
                "python3 -c 'import os,sys; f=os.stat(sys.argv[1]); d=os.stat(os.path.dirname(sys.argv[1])); open(sys.argv[2],\"w\").write(str(f.st_mode & 0o7777)+\" \"+str(d.st_mode & 0o7777))' "
                f"\"$1\" {shlex.quote(str(log))}\n"
                "exec /usr/bin/cp \"$@\"\n",
            )
            _bind_tool(script, "/usr/bin/cp", work / "bin" / "cp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            installed = cached.stat().st_mode & 0o7777 if cached.is_file() else 0
            staged = log.read_text(encoding="utf-8") if log.is_file() else ""
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertEqual(staged, "493 493")
        self.assertEqual(installed, 0o755)

    def test_sccache_prefetch_hashes_the_archive_once(self) -> None:
        real = shlex.quote(_fixture_node())
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            patch = work / "reread.js"
            patch.write_text(
                "const fs = require('fs');\n"
                "const real = fs.readFileSync;\n"
                "let seen = 0;\n"
                "fs.readFileSync = function () {\n"
                "  const buf = real.apply(fs, arguments);\n"
                "  if (String(arguments[0]).endsWith('pkg.tar.gz')) {\n"
                "    seen += 1;\n"
                "    if (seen > 1) return Buffer.from('second-read');\n"
                "  }\n"
                "  return buf;\n"
                "};\n",
                encoding="utf-8",
            )
            _write_exe(
                work / "bin",
                "node",
                "#!/bin/sh\n"
                f"if [ \"$1\" = -e ]; then exec {real} -r {shlex.quote(str(patch))} \"$@\"; fi\n"
                f"exec {real} \"$@\"\n",
            )
            _bind_fetch_node(script, work / "bin" / "node")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")

    def test_sccache_prefetch_shell_hash_rejects_a_lying_node(self) -> None:
        filename, arch = _host_sccache_filename()
        pin = PINNED_SCCACHE_SHA256[filename]
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = b"not-the-pinned-release"
            staged = work / "staged"
            binary = staged / filename[: -len(".tar.gz")]
            binary.mkdir(parents=True)
            planted = binary / "sccache"
            planted.write_text("#!/bin/sh\necho SWAPPED\n", encoding="utf-8")
            planted.chmod(0o755)
            member = hashlib.sha256(planted.read_bytes()).hexdigest()
            node = (
                "#!/bin/sh\n"
                'if [ "$1" = -p ] && [ "$2" = process.arch ]; then printf \'%s\\n\' x64; exit 0; fi\n'
                'if [ "$1" = -p ] && [ "$2" = process.platform ]; then printf \'%s\\n\' linux; exit 0; fi\n'
                "for arg in \"$@\"; do\n"
                "  if [ \"$arg\" = stage ]; then\n"
                f"    printf '%s\\n' {shlex.quote(pin)} {shlex.quote(member)} {shlex.quote(str(staged))}\n"
                "    exit 0\n"
                "  fi\n"
                "done\n"
                "exit 1\n"
            )
            _write_exe(work / "bin", "node", node)
            script = _script_copy(work)
            _bind_fetch_node(script, work / "bin" / "node")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    attempts="1",
                    delay="0",
                )
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertNotIn("available=true", output)
        self.assertFalse(cached.exists())

    def test_sccache_prefetch_mismatched_malformed_archive_exits_1(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            script = _script_copy(work)
            with _fixed_body_server(b"not-a-gzip-archive", status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    attempts="1",
                    delay="0",
                )
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertEqual(output, "")

    def test_sccache_prefetch_refuses_a_leading_backslash_name(self) -> None:
        _filename, arch, root = self._release_root()
        member = _ustar_header(b"\\evil", 48, 0)
        blob = _gzip_tar([member])
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

    def test_sccache_prefetch_refuses_a_non_octal_mode(self) -> None:
        _filename, arch, root = self._release_root()
        payload = b"#!/bin/sh\necho sccache 0.19.0\n"
        directory = _ustar_header(root.encode() + b"/", 53, 0)
        member = _ustar_header(
            root.encode() + b"/sccache",
            48,
            len(payload),
            mode_field=b"xxxxxxxx",
        )
        blob = _gzip_tar([directory, member, _pad_block(payload)])
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            self.assertEqual(_reader_status(work, blob), 2)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="could not extract the archive"
        )

    def test_sccache_prefetch_refuses_a_duplicate_binary_member(self) -> None:
        _filename, arch, root = self._release_root()
        first = b"#!/bin/sh\necho one\n"
        second = b"#!/bin/sh\necho two\n"
        directory = _ustar_header(root.encode() + b"/", 53, 0)
        one = _ustar_header(root.encode() + b"/sccache", 48, len(first))
        two = _ustar_header(root.encode() + b"/sccache", 48, len(second))
        blob = _gzip_tar(
            [directory, one, _pad_block(first), two, _pad_block(second)]
        )
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            result, output = _fetch_blob(work, blob)
        _assert_prefetch_fell_back(
            self, result, output, work, arch, message="archive has no sccache binary"
        )

    def test_sccache_prefetch_write_flags_refuse_a_planted_file(self) -> None:
        filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            outside = work / "outside-target"
            outside.write_bytes(b"OUTSIDE\n")
            _bind_node_preload(script, work, _FS_PLANT)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_PLANT": "file", "AW_OUTSIDE": str(outside)},
                )
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            self.assertNotEqual(output, "available=true\n")
            self.assertFalse(cached.exists())
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_sccache_prefetch_write_flags_refuse_a_planted_symlink(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            outside = work / "outside-target"
            outside.write_bytes(b"OUTSIDE\n")
            _bind_node_preload(script, work, _FS_PLANT)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_PLANT": "symlink", "AW_OUTSIDE": str(outside)},
                )
            self.assertEqual(outside.read_bytes(), b"OUTSIDE\n")
            self.assertNotEqual(output, "available=true\n")
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_sccache_prefetch_refuses_a_hardlinked_write(self) -> None:
        _filename, arch = _host_sccache_filename()
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _bind_node_preload(script, work, _FS_PLANT)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_PLANT": "hardlink"},
                )
            cached = work / "tool-cache" / "sccache" / "0.19.0" / arch / "sccache"
            self.assertNotEqual(output, "available=true\n")
            self.assertFalse(cached.exists())
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("symlink or non-regular", result.stderr)

    def test_sccache_prefetch_refuses_a_linked_release_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            outside = work / "outside-dir"
            outside.mkdir()
            _bind_node_preload(script, work, _FS_PLANT)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_PLANT": "rootlink", "AW_OUTSIDE": str(outside)},
                )
            self.assertEqual(list(outside.iterdir()), [])
            self.assertNotEqual(output, "available=true\n")
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_sccache_prefetch_refuses_a_realpath_escape(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            _bind_node_preload(script, work, _FS_PLANT)
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                    extra_env={"AW_PLANT": "realpath"},
                )
        self.assertNotEqual(output, "available=true\n")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_sccache_prefetch_fchmod_keeps_the_member_mode(self) -> None:
        old = os.umask(0o077)
        try:
            with tempfile.TemporaryDirectory() as tmp:
                work = Path(tmp)
                blob = _valid_sccache_archive(work)
                script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
                log = work / "modes"
                _bind_node_preload(script, work, _FS_PLANT)
                with _fixed_body_server(blob, status=200) as server:
                    port = server.server_address[1]
                    result, output = _run_fetch(
                        work,
                        script=script,
                        base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                        extra_env={"AW_CHMOD_LOG": str(log)},
                    )
                logged = log.read_text(encoding="utf-8") if log.exists() else ""
        finally:
            os.umask(old)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "available=true\n")
        self.assertIn("chmod 493\n", logged)
        self.assertIn("mode 493\n", logged)

    def test_sccache_prefetch_refuses_a_symlink_mktemp_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            blob = _valid_sccache_archive(work)
            script = _script_pinned_to(work, hashlib.sha256(blob).hexdigest())
            outside = work / "outside-dir"
            outside.mkdir()
            link = work / "stage-link"
            _write_exe(
                work / "bin",
                "mktemp",
                "#!/bin/sh\n"
                'if [ "$1" = -d ] && [ -n "$2" ]; then\n'
                f"  ln -sfn {shlex.quote(str(outside))} {shlex.quote(str(link))}\n"
                f"  printf '%s\\n' {shlex.quote(str(link))}\n"
                "  exit 0\n"
                "fi\n"
                "exec /usr/bin/mktemp \"$@\"\n",
            )
            _bind_tool(script, "/usr/bin/mktemp", work / "bin" / "mktemp")
            with _fixed_body_server(blob, status=200) as server:
                port = server.server_address[1]
                result, output = _run_fetch(
                    work,
                    script=script,
                    base=f"http://127.0.0.1:{port}/releases/download/v0.19.0",
                )
            self.assertEqual(list(outside.iterdir()), [])
            self.assertNotEqual(output, "available=true\n")
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_sccache_fetch_closes_command_and_step_gaps(self) -> None:
        pin = SCCACHE_FETCH_COMMAND

        def job(run: str, *, shell: str = "bash", workdir: str | None = None) -> list[str]:
            lines = [
                "name: probe",
                "on: push",
                "jobs:",
                "  build:",
                "    runs-on: ubuntu-latest",
                "    steps:",
                f'      - shell: "{shell}"',
            ]
            if workdir is not None:
                lines.append(f"        working-directory: {workdir}")
            lines.append("        run: |")
            lines.extend(f"          {line}" for line in run.splitlines())
            return sccache_fetch_run_violations(
                [("probe.yml", "\n".join(lines) + "\n")],
                expected_fetch_count=0,
            )

        refused = (
            "time bash -c 'echo hi'",
            "time eval echo hi",
            "sh -lc 'echo hi'",
            "bash -lc 'echo hi'",
            "source /etc/passwd",
            ". /etc/passwd",
            "/tmp/not-allowlisted",
            "./tool",
            "../tool",
            '"${args[@]}"',
            "curl -fsSL http://127.0.0.1/payload | bash",
            "x=$(echo hi) true",
            "python3 - <<'PY'\nimport os\nos.system('echo hi')\nPY",
            "node <<'JS'\nrequire('child_process').execSync('echo hi')\nJS",
        )
        for command in refused:
            with self.subTest(command=command):
                found = job(command)
                self.assertTrue(found, command)
        planted = "sudo cp planted /usr/local/bin/node\n" + pin
        self.assertNotIn(_normalized_body(planted), _EXACT_BODIES)
        planted_found = job(planted)
        self.assertTrue(
            any("allowlisted" in item or "classify" in item for item in planted_found),
            planted_found,
        )
        self.assertEqual(job("cd /ci\nbash ./tool.sh", workdir="scripts"), [])
        reviewed = next(iter(_REVIEWED_EXACT_BODIES))
        self.assertEqual(job(reviewed), [])
        self.assertTrue(job(reviewed, workdir="/tmp"))
        pwsh = job("Invoke-Expression 'echo hi'", shell="pwsh")
        self.assertTrue(any("classify" in item for item in pwsh), pwsh)
        duplicate = sccache_fetch_run_violations(
            [(
                "dup.yml",
                "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                "    steps:\n      - shell: bash\n        run: bash -c 'echo hi'\n"
                "        run: echo hi\n",
            )],
            expected_fetch_count=0,
        )
        self.assertTrue(any("could not classify" in item for item in duplicate), duplicate)
        for uses in ("../tools/act", "/tmp/act", "tools/act"):
            with self.subTest(uses=uses):
                found = sccache_fetch_run_violations(
                    [(
                        "uses.yml",
                        "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                        "    steps:\n"
                        f"      - uses: {uses}\n",
                    )],
                    expected_fetch_count=0,
                )
                self.assertTrue(any("could not classify" in item for item in found), found)
        local = sccache_fetch_run_violations(
            [(
                "local.yml",
                "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - uses: .github/actions/rust-ci-setup\n",
            )],
            expected_fetch_count=0,
        )
        self.assertEqual(local, [])

        def before(run: str, *, workdir: str | None = None) -> list[str]:
            wd = f"        working-directory: {workdir}\n" if workdir else ""
            body = "".join(f"          {line}\n" for line in run.splitlines())
            text = (
                "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - shell: bash\n"
                f"{wd}"
                "        run: |\n"
                f"{body}"
                "      - shell: bash\n"
                f"        run: {pin}\n"
            )
            return sccache_fetch_run_violations([("prior.yml", text)], expected_fetch_count=1)

        priors = (
            "python3 tools/plant.py",
            "printf '%s\n' '#!/bin/sh' 'echo SWAPPED' > /usr/local/bin/node",
            "cp \"$GITHUB_WORKSPACE/planted.sh\" \"$(printf 'fetch_\\163ccache_release.sh')\"",
            "cd scripts\ncd ci\ncp \"$GITHUB_WORKSPACE/planted.sh\" \"$(printf 'fetch_\\163ccache_release.sh')\"",
            "cp planted.sh $'fetch_\\163ccache_release.sh'",
            "printf '%s\n' 'BASH_ENV=/tmp/hook.sh' >> \"$GITHUB_ENV\"",
            "echo BASH_ENV=/tmp/payload.sh >> \"$GITHUB_ENV\"",
            "echo LD_PRELOAD=/tmp/x.so >> \"$GITHUB_ENV\"",
            "echo PATH=/tmp/hook >> \"$GITHUB_PATH\"",
        )
        for command in priors:
            with self.subTest(prior=command):
                found = before(command)
                self.assertTrue(found, (command, found))
        found = before(
            "cp \"$GITHUB_WORKSPACE/planted.sh\" \"$(printf 'fetch_\\163ccache_release.sh')\"",
            workdir="scripts/ci",
        )
        self.assertTrue(any("pre-fetch" in item for item in found), found)
        allowed = before("echo RUSTC_WRAPPER=sccache >> \"$GITHUB_ENV\"")
        # The echo is not an enumerated pre-fetch step, so it is refused for that
        # reason. The key itself is not a startup-key violation when it is the
        # only assignment on a later step.
        later = sccache_fetch_run_violations(
            [(
                "later.yml",
                "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - shell: bash\n"
                f"        run: {pin}\n"
                "      - shell: bash\n"
                "        run: echo RUSTC_WRAPPER=sccache >> \"$GITHUB_ENV\"\n",
            )],
            expected_fetch_count=1,
        )
        self.assertEqual(later, [])
        self.assertTrue(allowed)

    def test_sccache_policy_kills_the_remaining_one_edit_arms(self) -> None:
        """One fixture per arm a one-edit sweep can drop and still pass the rest."""
        pin = SCCACHE_FETCH_COMMAND

        def run(body: str, *, fetch: int = 0) -> list[str]:
            text = (
                "name: probe\non: push\njobs:\n  build:\n"
                "    runs-on: ubuntu-latest\n    steps:\n"
                "      - shell: bash\n        run: |\n"
            )
            text += "".join(f"          {line}\n" for line in body.splitlines())
            return sccache_fetch_run_violations(
                [("probe.yml", text)], expected_fetch_count=fetch
            )

        def document(text: str) -> list[str]:
            return sccache_fetch_run_violations(
                [("probe.yml", text)], expected_fetch_count=0
            )

        refused = {
            "exact-prefix": "channel=$(grep -v '^[[:space:]]*#' .github/rust-nightly-channel | tr -d '[:space:]')",
            "unclosed-quote": 'echo "unterminated',
            "dollar-argv0": "$TOOL",
            "folded-backslash": "echo scripts\\ci/tool.sh",
            "keyword-eval": "if true; then eval echo hi; fi",
            "cd-destination": "cd scripts/ci\necho hi",
            "unknown-argv0": "wget http://127.0.0.1/x",
            "heredoc": "cat <<EOF\nhi\nEOF",
            "function": "echo() { echo hi; }",
            "brace": "echo {a,b}",
            "redirect": "cat < /etc/passwd",
            "interpreter-e": "python3 -e 'print(1)'",
            "path-argv0": "/bin/echo hi",
            "unescaped-name": "echo scr\\ipts/ci/tool.sh",
            "brace-name": "echo {scripts,other}/ci/tool.sh",
            "command-v-space": "command -v 'has space'",
            "command-v-bracket": "command -v 'a[b'",
            "glob-argv0": "./bin*/echo hi",
        }
        for name, body in refused.items():
            with self.subTest(arm=name):
                found = run(body)
                self.assertTrue(found, name)
                if name in {"dollar-argv0", "glob-argv0"}:
                    self.assertTrue(any("allowlisted" in item for item in found), found)
        self.assertEqual(run("printf '%s\\n' ok"), [])
        self.assertEqual(run("python3 ok\\n.py"), [])
        self.assertEqual(run("echo **/ci/tool.sh"), [])
        echoed = run("echo " + pin)
        self.assertTrue(any("allowlisted" in item for item in echoed), echoed)
        self.assertFalse(any("found 1" in item for item in echoed), echoed)
        self.assertEqual(_string_env({"A": None}, "probe")["A"], "")
        self.assertIn(
            ".github/actions/demo/action.yaml",
            _policy_names(
                [
                    ".github/actions/demo/action.yaml",
                    ".github/workflows/probe.yml",
                ]
            ),
        )
        expanded = document(
            "name: probe\non: push\njobs:\n  build:\n"
            "    runs-on: ubuntu-latest\n    env:\n"
            "      FOO: scripts\n    steps:\n"
            "      - shell: bash\n        run: echo ${FOO}/ci/tool.sh\n"
        )
        self.assertTrue(any("allowlisted" in item for item in expanded), expanded)
        nested = document(
            "name: probe\non: push\njobs:\n  build:\n"
            "    runs-on: ubuntu-latest\n    env:\n      X:\n        - a\n"
            "    steps:\n      - shell: bash\n        run: echo hi\n"
        )
        self.assertTrue(any("could not classify" in item for item in nested), nested)
        extra_defaults = document(
            "name: probe\non: push\ndefaults:\n  run:\n    shell: bash\n"
            "  extra: 1\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            "    steps:\n      - run: echo hi\n"
        )
        self.assertTrue(
            any("could not classify" in item for item in extra_defaults), extra_defaults
        )
        with_path = document(
            "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            "    steps:\n      - uses: actions/checkout@v7\n        with:\n"
            '          path: "{scripts,x}/ci/tool.sh"\n'
        )
        with_escape = document(
            "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            "    steps:\n      - uses: actions/checkout@v7\n        with:\n"
            '          path: "scr\\\\ipts/ci/tool.sh"\n'
        )
        self.assertTrue(any("allowlisted" in item for item in with_escape), with_escape)
        cd_var = document(
            "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            "    steps:\n      - shell: bash\n        run: |\n"
            "          FOO=scripts\n          cd $FOO/ci\n"
        )
        self.assertTrue(any("allowlisted" in item for item in cd_var), cd_var)
        self.assertTrue(any("allowlisted" in item for item in with_path), with_path)
        for label, text in (
            ("steps", "name: probe\non: push\nruns:\n  using: composite\n  steps: echo\n"),
            ("jobs", "name: probe\non: push\njobs: []\n"),
            (
                "shell",
                "name: probe\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                "    steps:\n      - shell: 1\n        run: echo hi\n",
            ),
            ("root", "- a\n"),
        ):
            with self.subTest(shape=label):
                found = document(text)
                self.assertTrue(any("could not classify" in item for item in found), found)
        moved = run(f"FOO=bar cd /tmp\n{pin}", fetch=1)
        self.assertTrue(any("working-directory" in item for item in moved), moved)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

import difflib
import errno
import inspect
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
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


def _normalize_line_endings(text: str) -> str:
    """Replace CRLF and CR with LF. No other change."""
    return text.replace("\r\n", "\n").replace("\r", "\n")


_GPU_FRAME_BUDGET_GOLDEN = (
    Path(__file__).resolve().parent / "gpu_frame_budget_workflow.golden.yml"
)
_GPU_FRAME_BUDGET_MISMATCH = "gpu-frame-budget.yml is not the canonical workflow text"


def _read_regular_utf8(path: Path) -> str:
    """Read UTF-8 bytes from `path`.

    Open with `O_NOFOLLOW`. `ELOOP` raises `AssertionError`.
    Any other `OSError` propagates. Decode the bytes as UTF-8.
    """
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as exc:
        if exc.errno == errno.ELOOP:
            raise AssertionError(f"{path.name} path is a symlink") from exc
        raise
    try:
        data = os.read(fd, os.fstat(fd).st_size)
    finally:
        os.close(fd)
    return data.decode("utf-8")


def _canonical_gpu_frame_budget_text() -> str:
    raw = _read_regular_utf8(_GPU_FRAME_BUDGET_GOLDEN)
    return _normalize_line_endings(raw)


def _gpu_frame_budget_workflow_text() -> str:
    """Read `.github/workflows/gpu-frame-budget.yml` without following a symlink."""
    return _read_regular_utf8(WORKFLOWS / "gpu-frame-budget.yml")


_GPU_FRAME_BUDGET_CHECKOUT = (
    "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
)
_GPU_FRAME_BUDGET_TOOLCHAIN = (
    "      - uses: dtolnay/rust-toolchain@df2a88e9fe4098bad09cb42c3dec7187f15861fc # master\n"
)
_GPU_FRAME_BUDGET_RUNS_ON = "    runs-on: [self-hosted, Windows, gpu]\n"
_GPU_FRAME_BUDGET_ENV = "          AW_GLB: ${{ inputs.glb }}\n"
_GPU_FRAME_BUDGET_RUN = "        run: |\n"
_GPU_FRAME_BUDGET_CARGO = (
    "          cargo run -p astraweave-render --release "
    "--features gpu-frame-budget --bin gpu_frame_budget -- "
    '--scene heavy --glb "$env:AW_GLB"\n'
)
_GPU_FRAME_BUDGET_JOBS = (
    "jobs:\n",
    "  capture:\n",
    "    name: gpu_frame_budget\n",
    _GPU_FRAME_BUDGET_RUNS_ON,
    "    timeout-minutes: 60\n",
    "    steps:\n",
    _GPU_FRAME_BUDGET_CHECKOUT,
    "        with:\n",
    "          persist-credentials: false\n",
    _GPU_FRAME_BUDGET_TOOLCHAIN,
    "        with:\n",
    '          toolchain: "1.89.0"\n',
    "      - name: Median draw_into\n",
    "        shell: pwsh\n",
    "        env:\n",
    _GPU_FRAME_BUDGET_ENV,
    _GPU_FRAME_BUDGET_RUN,
    _GPU_FRAME_BUDGET_CARGO,
)
_GPU_FRAME_BUDGET_JOBS_LINE_FORMS = (
    "permissions_space_before_colon",
    "explicit_permissions_key",
    "quoted_permissions_write",
    "single_quoted_permissions_write",
    "plain_run_space_before_colon",
    "explicit_plain_run",
    "uses_space_persist_credentials",
    "relay_secrets_inherit",
    "other_job_space_keys",
    "other_job_quoted_keys",
    "path_beside_aw_glb",
    "working_directory_step",
)


def _top_level_key_indexes(lines: list[str]) -> list[int]:
    indexes = []
    for index, line in enumerate(lines):
        if not line.strip() or line[0] in " \t#":
            continue
        indexes.append(index)
    return indexes


def _top_level_block(lines: list[str], key: str) -> list[str]:
    """Return the lines of the first top-level `key`, or `[]` when it is absent.

    A top-level line is non-blank and does not start with a space, tab,
    or `#`. The block runs from that key's line up to the next top-level
    key, or through the end of `lines` when the key is last.
    """
    indexes = _top_level_key_indexes(lines)
    for position, index in enumerate(indexes):
        if lines[index].split(":", 1)[0] != key:
            continue
        end = indexes[position + 1] if position + 1 < len(indexes) else len(lines)
        return lines[index:end]
    return []


def _gpu_frame_budget_guard_failures(text: str) -> list[str]:
    """Return the canonical-property messages `text` fails, in check order.

    A top-level line is non-blank and does not start with a space, tab,
    or `#`. The keys message is `gpu-frame-budget canonical keys` when
    those keys are not `name`, `on`, `permissions`, and `jobs`.
    The `on` block is `_top_level_block` of `on`. Its message is
    `gpu-frame-budget on is not workflow_dispatch only` when the first
    two lines are not `on:` and `  workflow_dispatch:`, or a later
    non-blank line is not indented at least four spaces, or the lines
    indented exactly four spaces are not `    inputs:`.
    The permissions message is
    `gpu-frame-budget permissions are not contents: read` when the
    non-blank lines of the `permissions` block are not `permissions:`
    and `  contents: read`.
    The jobs message is `gpu-frame-budget jobs block` when
    `_top_level_block` of `jobs` is not `_GPU_FRAME_BUDGET_JOBS`.
    The expression message is `gpu-frame-budget expression` when a line
    contains `${{` and is not `_GPU_FRAME_BUDGET_ENV`.
    Each message appears at most once.
    """
    failures: list[str] = []
    lines = text.splitlines(keepends=True)
    indexes = _top_level_key_indexes(lines)
    keys = [lines[index].split(":", 1)[0] for index in indexes]
    if keys != ["name", "on", "permissions", "jobs"]:
        failures.append("gpu-frame-budget canonical keys")
    on_lines = _top_level_block(lines, "on")
    on_message = "gpu-frame-budget on is not workflow_dispatch only"
    if on_lines[:2] != ["on:\n", "  workflow_dispatch:\n"]:
        failures.append(on_message)
    elif any(
        line.strip() and not line.startswith("    ") for line in on_lines[2:]
    ):
        failures.append(on_message)
    elif [
        line
        for line in on_lines[2:]
        if line.startswith("    ") and not line.startswith("     ")
    ] != ["    inputs:\n"]:
        failures.append(on_message)
    perm_lines = [
        line for line in _top_level_block(lines, "permissions") if line.strip()
    ]
    if perm_lines != ["permissions:\n", "  contents: read\n"]:
        failures.append("gpu-frame-budget permissions are not contents: read")
    if tuple(_top_level_block(lines, "jobs")) != _GPU_FRAME_BUDGET_JOBS:
        failures.append("gpu-frame-budget jobs block")
    for line in lines:
        if "${{" in line and line != _GPU_FRAME_BUDGET_ENV:
            failures.append("gpu-frame-budget expression")
            break
    return failures


def _assert_gpu_frame_budget_canonical_properties(text: str) -> None:
    """Raise the first message from `_gpu_frame_budget_guard_failures`.

    An empty failure list passes.
    """
    failures = _gpu_frame_budget_guard_failures(text)
    if failures:
        raise AssertionError(failures[0])


def assert_gpu_frame_budget_dispatch_input_is_env(text: str) -> None:
    """Accept `.github/workflows/gpu-frame-budget.yml` only in canonical form.

    Read `scripts/ci/gpu_frame_budget_workflow.golden.yml` with
    `_read_regular_utf8`. Normalize CRLF and CR to LF. That golden text must
    pass `_assert_gpu_frame_budget_canonical_properties`. `text` must equal
    that golden text after the same normalization, or this raises
    `gpu-frame-budget.yml is not the canonical workflow text`.
    """
    golden = _canonical_gpu_frame_budget_text()
    _assert_gpu_frame_budget_canonical_properties(golden)
    if _normalize_line_endings(text) != golden:
        raise AssertionError(_GPU_FRAME_BUDGET_MISMATCH)


def _replace_once(text: str, old: str, new: str) -> str:
    count = text.count(old)
    if count == 0:
        raise AssertionError("policy fixture anchor missing")
    if count != 1:
        raise AssertionError("policy fixture anchor matched more than once")
    return text.replace(old, new, 1)


def gpu_frame_budget_policy_cases(text: str) -> list[tuple[str, str]]:
    """One full-file variant of `.github/workflows/gpu-frame-budget.yml` per form.

    Each text differs from `text`. A missing anchor fails closed.
    """
    on_anchor = "on:\n  workflow_dispatch:\n"
    perm_anchor = "permissions:\n  contents: read\n"
    cred_anchor = "          persist-credentials: false\n"
    with_anchor = "        with:\n          persist-credentials: false\n"
    env_anchor = "        env:\n          AW_GLB: ${{ inputs.glb }}\n"
    median_anchor = "      - name: Median draw_into\n"
    shell_anchor = "        shell: pwsh\n"
    run_anchor = "        run: |\n"
    cargo_anchor = (
        "          cargo run -p astraweave-render --release "
        "--features gpu-frame-budget --bin gpu_frame_budget -- "
        '--scene heavy --glb "$env:AW_GLB"\n'
    )
    cargo_without_glb = (
        "          cargo run -p astraweave-render --release "
        "--features gpu-frame-budget --bin gpu_frame_budget -- "
        "--scene heavy\n"
    )
    toolchain_anchor = (
        "      - uses: dtolnay/rust-toolchain@"
        "df2a88e9fe4098bad09cb42c3dec7187f15861fc # master\n"
    )
    capture_anchor = "  capture:\n    name: gpu_frame_budget\n"
    expression_line = '          echo "${{ inputs.glb }}"\n'

    def before_median(block: str) -> str:
        return _replace_once(text, median_anchor, block + median_anchor)

    def on_insert(line: str) -> str:
        return _replace_once(text, on_anchor, "on:\n" + line + "  workflow_dispatch:\n")

    def run_header(header: str) -> str:
        return _replace_once(text, run_anchor, header + expression_line)

    cases = [
        (
            "quoted_run_key",
            before_median(
                '      - name: extra\n        "run": echo "${{ inputs.glb }}"\n'
            ),
        ),
        (
            "flow_step",
            before_median(
                "      - {name: extra, shell: pwsh, run: 'echo ${{ inputs.glb }}'}\n"
            ),
        ),
        (
            "run_space_before_colon",
            before_median(
                '      - name: extra\n        run : echo "${{ inputs.glb }}"\n'
            ),
        ),
        (
            "empty_run_indented",
            before_median(
                "      - name: extra\n        run:\n          echo ${{ inputs.glb }}\n"
            ),
        ),
        (
            "double_quoted_split",
            before_median(
                '      - name: extra\n'
                '        run: "echo\n'
                '          ${{ inputs.glb }}"\n'
            ),
        ),
        (
            "single_quoted_split",
            before_median(
                "      - name: extra\n"
                "        run: 'echo\n"
                "          ${{ inputs.glb }}'\n"
            ),
        ),
        (
            "plain_continuation",
            before_median(
                "      - name: extra\n        run: echo\n          ${{ inputs.glb }}\n"
            ),
        ),
        (
            "hash_without_space",
            before_median("      - run: echo x#${{ inputs.glb }}\n"),
        ),
        (
            "explicit_run_key",
            before_median(
                "      - name: extra\n"
                "        ? run\n"
                '        : echo "${{ inputs.glb }}"\n'
            ),
        ),
        (
            "alias_run_value",
            before_median(
                "      - name: extra\n"
                "        env:\n"
                "          PAYLOAD: &payload echo ${{ inputs.glb }}\n"
                "        run: *payload\n"
            ),
        ),
        (
            "alias_step",
            _replace_once(
                _replace_once(
                    text,
                    "jobs:\n",
                    "x-step: &extra {name: extra, shell: pwsh, "
                    "run: 'echo ${{ inputs.glb }}'}\njobs:\n",
                ),
                median_anchor,
                "      - *extra\n" + median_anchor,
            ),
        ),
        (
            "double_quoted_hex",
            before_median(
                '      - name: extra\n'
                '        run: "echo \\x24\\x7B\\x7B inputs.glb }}"\n'
            ),
        ),
        (
            "double_quoted_unicode",
            before_median(
                '      - name: extra\n'
                '        run: "echo \\u0024\\u007B\\u007B inputs.glb }}"\n'
            ),
        ),
        (
            "escaped_line_break",
            before_median(
                '      - name: extra\n        run: "echo $\\\n'
                '          {{ inputs.glb }}"\n'
            ),
        ),
        (
            "quoted_pull_request_target",
            on_insert('  "pull_request_target":\n'),
        ),
        (
            "single_quoted_pull_request",
            on_insert("  'pull_request':\n"),
        ),
        (
            "single_quoted_pull_request_target",
            on_insert("  'pull_request_target':\n"),
        ),
        (
            "quoted_workflow_run",
            on_insert('  "workflow_run":\n'),
        ),
        (
            "quoted_id_token",
            _replace_once(
                text,
                perm_anchor,
                'permissions:\n  contents: read\n  "id-token": write\n',
            ),
        ),
        (
            "job_write_all",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n    permissions: write-all\n    name: gpu_frame_budget\n",
            ),
        ),
        (
            "job_id_token_write",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n"
                "    permissions:\n"
                "      contents: write\n"
                "      id-token: write\n"
                "    name: gpu_frame_budget\n",
            ),
        ),
        (
            "indented_trigger",
            _replace_once(
                text,
                on_anchor,
                "on:\n   pull_request_target:\n  workflow_dispatch:\n",
            ),
        ),
        (
            "env_persist_credentials",
            _replace_once(
                text,
                with_anchor,
                "        with:\n"
                "          persist-credentials: true\n"
                "        env:\n"
                "          persist-credentials: false\n",
            ),
        ),
        (
            "second_checkout_true",
            _replace_once(
                text,
                toolchain_anchor,
                "      - uses: actions/checkout@"
                "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
                "        with:\n"
                "          persist-credentials: true\n"
                + toolchain_anchor,
            ),
        ),
        (
            "sibling_aw_glb",
            _replace_once(
                text,
                env_anchor,
                "        env:\n"
                "          AW_GLB: ${{ github.event.pull_request.title }}\n"
                "        with:\n"
                "          AW_GLB: ${{ inputs.glb }}\n",
            ),
        ),
        (
            "duplicate_shell",
            _replace_once(
                text,
                shell_anchor,
                "        shell: pwsh\n        shell: bash\n",
            ),
        ),
        (
            "pwsh_comment_glb",
            _replace_once(
                text,
                cargo_anchor,
                '          # --glb "$env:AW_GLB"\n' + cargo_without_glb,
            ),
        ),
        (
            "echo_glb_token",
            _replace_once(
                text,
                cargo_anchor,
                "          echo '--glb \"$env:AW_GLB\"'\n" + cargo_without_glb,
            ),
        ),
        (
            "local_composite_uses",
            before_median(
                "      - uses: ./.github/actions/gpu-capture\n"
                "        with:\n"
                "          glb: ${{ inputs.glb }}\n"
            ),
        ),
        (
            "job_level_uses",
            _replace_once(
                text,
                "jobs:\n  capture:\n",
                "jobs:\n"
                "  relay:\n"
                "    uses: ./.github/workflows/reusable-capture.yml\n"
                "    with:\n"
                "      glb: ${{ inputs.glb }}\n"
                "  capture:\n",
            ),
        ),
        (
            "shell_bash",
            _replace_once(text, shell_anchor, "        shell: bash\n"),
        ),
        (
            "zero_run_bodies",
            _replace_once(text, run_anchor + cargo_anchor, ""),
        ),
        (
            "unnamed_run",
            before_median('      - run: echo "${{ inputs.glb }}"\n'),
        ),
        (
            "unnamed_run_block",
            before_median("      - run: |\n" + expression_line),
        ),
        (
            "commented_out_with",
            _replace_once(
                text,
                with_anchor,
                "        # with:\n        #   persist-credentials: false\n",
            ),
        ),
        (
            "inlined_expression",
            _replace_once(text, '--glb "$env:AW_GLB"', '--glb "${{ inputs.glb }}"'),
        ),
        (
            "unquoted_glb",
            _replace_once(text, '--glb "$env:AW_GLB"', "--glb $env:AW_GLB"),
        ),
        (
            "dropped_credentials",
            _replace_once(text, cred_anchor, ""),
        ),
        (
            "block_header_2",
            run_header("        run: |2\n"),
        ),
        (
            "block_header_strip_2",
            run_header("        run: |-2\n"),
        ),
        (
            "block_header_2_strip",
            run_header("        run: |2-\n"),
        ),
        (
            "block_header_comment",
            run_header("        run: | # c\n"),
        ),
        (
            "folded_gt",
            run_header("        run: >\n"),
        ),
        (
            "kept_block_header",
            run_header("        run: |+\n"),
        ),
        (
            "comment_expression",
            _replace_once(
                _replace_once(
                    text,
                    run_anchor,
                    "        run: | # ${{ inputs.glb }}\n",
                ),
                "permissions:\n",
                "# ${{ inputs.glb }}\npermissions:\n",
            ),
        ),
        (
            "comment_credentials",
            _replace_once(
                text,
                cred_anchor,
                "          persist-credentials: true\n"
                "          # persist-credentials: false\n",
            ),
        ),
        (
            "comment_aw_glb",
            _replace_once(
                text,
                "          AW_GLB: ${{ inputs.glb }}\n",
                "          # AW_GLB: ${{ inputs.glb }}\n"
                "          AW_GLB: ${{ github.event.pull_request.title }}\n",
            ),
        ),
        (
            "comment_trigger",
            _replace_once(
                text,
                on_anchor,
                "on:\n  workflow_dispatch:\n  # pull_request_target:\n",
            ),
        ),
        (
            "comment_permission",
            _replace_once(
                text,
                perm_anchor,
                "permissions:\n  contents: read\n  # id-token: write\n",
            ),
        ),
        (
            "pull_request_target",
            on_insert("  pull_request_target:\n"),
        ),
        (
            "pull_request",
            on_insert("  pull_request:\n"),
        ),
        (
            "workflow_run",
            on_insert("  workflow_run:\n"),
        ),
        (
            "list_form_on",
            _replace_once(text, on_anchor, "on: [workflow_dispatch, pull_request]\n"),
        ),
        (
            "quoted_on_key",
            _replace_once(text, "on:\n", '"on":\n'),
        ),
        (
            "id_token_write",
            _replace_once(
                text,
                perm_anchor,
                "permissions:\n  contents: read\n  id-token: write\n",
            ),
        ),
        (
            "top_level_write_all",
            _replace_once(text, perm_anchor, "permissions: write-all\n"),
        ),
        (
            "flow_permissions",
            _replace_once(
                text,
                perm_anchor,
                "permissions: {contents: read, id-token: write}\n",
            ),
        ),
        (
            "flow_with",
            _replace_once(
                text,
                with_anchor,
                "        with: {persist-credentials: true}\n",
            ),
        ),
        (
            "script_line_hash",
            _replace_once(
                text,
                cargo_anchor,
                "          # ${{ inputs.glb }}\n" + cargo_anchor,
            ),
        ),
        (
            "second_block_line",
            _replace_once(text, cargo_anchor, cargo_anchor + expression_line),
        ),
        (
            "tag_run",
            before_median(
                '      - name: extra\n        run: !!str "${{ inputs.glb }}"\n'
            ),
        ),
        (
            "second_job_expression",
            text
            + "  other:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            '      - run: echo "${{ inputs.glb }}"\n',
        ),
        (
            "trailing_space",
            _replace_once(text, "  contents: read\n", "  contents: read \n"),
        ),
        (
            "permissions_space_before_colon",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n"
                "    name: gpu_frame_budget\n"
                "    permissions : write-all\n",
            ),
        ),
        (
            "explicit_permissions_key",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n"
                "    name: gpu_frame_budget\n"
                "    ? permissions\n"
                "    : write-all\n",
            ),
        ),
        (
            "quoted_permissions_write",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n"
                "    name: gpu_frame_budget\n"
                '    "permissions":\n'
                "      contents: write\n"
                "      id-token: write\n",
            ),
        ),
        (
            "single_quoted_permissions_write",
            _replace_once(
                text,
                capture_anchor,
                "  capture:\n"
                "    name: gpu_frame_budget\n"
                "    'permissions':\n"
                "      contents: write\n"
                "      id-token: write\n",
            ),
        ),
        (
            "plain_run_space_before_colon",
            before_median(
                "      - name: extra\n        run : Write-Output pwned\n"
            ),
        ),
        (
            "explicit_plain_run",
            before_median(
                "      - name: extra\n"
                "        ? run\n"
                "        : Write-Output pwned\n"
            ),
        ),
        (
            "uses_space_persist_credentials",
            _replace_once(
                text,
                toolchain_anchor,
                "      - uses : actions/checkout@"
                "3d3c42e5aac5ba805825da76410c181273ba90b1\n"
                "        with:\n"
                "          persist-credentials : true\n"
                + toolchain_anchor,
            ),
        ),
        (
            "relay_secrets_inherit",
            _replace_once(
                text,
                "jobs:\n  capture:\n",
                "jobs:\n"
                "  relay:\n"
                "    uses : ./.github/workflows/reusable-capture.yml\n"
                "    secrets : inherit\n"
                "  capture:\n",
            ),
        ),
        (
            "other_job_space_keys",
            _replace_once(
                text,
                "jobs:\n  capture:\n",
                "jobs:\n"
                "  other:\n"
                "    runs-on : ubuntu-latest\n"
                "    permissions : write-all\n"
                "    steps:\n"
                "      - run : echo pwned\n"
                "  capture:\n",
            ),
        ),
        (
            "other_job_quoted_keys",
            _replace_once(
                text,
                "jobs:\n  capture:\n",
                "jobs:\n"
                "  other:\n"
                '    "runs-on": ubuntu-latest\n'
                '    "permissions": write-all\n'
                "    steps:\n"
                '      - "run": echo pwned\n'
                "  capture:\n",
            ),
        ),
        (
            "path_beside_aw_glb",
            _replace_once(
                text,
                env_anchor,
                "        env:\n          AW_GLB: ${{ inputs.glb }}\n"
                "          PATH: C:\\evil\n",
            ),
        ),
        (
            "working_directory_step",
            _replace_once(
                text,
                shell_anchor,
                "        shell: pwsh\n        working-directory: C:\\evil\n",
            ),
        ),
    ]
    _reject_gpu_frame_budget_fixture_list(cases, _normalize_line_endings(text))
    return cases


def _reject_gpu_frame_budget_fixture_list(
    cases: list[tuple[str, str]], canonical: str
) -> None:
    """Reject a duplicate fixture name or a fixture equal to `canonical`.

    The duplicate message is `duplicate policy fixture name`.
    A fixture equal to `canonical` raises `{name} matches the canonical text`.
    """
    names = [name for name, _mutant in cases]
    if len(names) != len(set(names)):
        raise AssertionError("duplicate policy fixture name")
    for name, mutant in cases:
        if _normalize_line_endings(mutant) == canonical:
            raise AssertionError(f"{name} matches the canonical text")


def _line_diff_spans(canonical: str, mutant: str) -> tuple[tuple[str, str], ...]:
    """Return each non-equal line span as `(deleted, inserted)`."""
    left = canonical.splitlines(keepends=True)
    right = mutant.splitlines(keepends=True)
    spans: list[tuple[str, str]] = []
    matcher = difflib.SequenceMatcher(a=left, b=right, autojunk=False)
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            continue
        spans.append(("".join(left[i1:i2]), "".join(right[j1:j2])))
    return tuple(spans)


# Named form of each gpu_frame_budget_policy_cases entry: the line diff
# against the canonical workflow text, as (deleted, inserted) spans.
_GPU_FRAME_BUDGET_NAMED_FORMS: dict[str, tuple[tuple[str, str], ...]] = {
    'quoted_run_key': (
        ('', '      - name: extra\n        "run": echo "${{ inputs.glb }}"\n'),
    ),
    'flow_step': (
        ('', "      - {name: extra, shell: pwsh, run: 'echo ${{ inputs.glb }}'}\n"),
    ),
    'run_space_before_colon': (
        ('', '      - name: extra\n        run : echo "${{ inputs.glb }}"\n'),
    ),
    'empty_run_indented': (
        ('', '      - name: extra\n        run:\n          echo ${{ inputs.glb }}\n'),
    ),
    'double_quoted_split': (
        ('', '      - name: extra\n        run: "echo\n          ${{ inputs.glb }}"\n'),
    ),
    'single_quoted_split': (
        ('', "      - name: extra\n        run: 'echo\n          ${{ inputs.glb }}'\n"),
    ),
    'plain_continuation': (
        ('', '      - name: extra\n        run: echo\n          ${{ inputs.glb }}\n'),
    ),
    'hash_without_space': (
        ('', '      - run: echo x#${{ inputs.glb }}\n'),
    ),
    'explicit_run_key': (
        ('', '      - name: extra\n        ? run\n        : echo "${{ inputs.glb }}"\n'),
    ),
    'alias_run_value': (
        ('', '      - name: extra\n        env:\n          PAYLOAD: &payload echo ${{ inputs.glb }}\n        run: *payload\n'),
    ),
    'alias_step': (
        ('', "x-step: &extra {name: extra, shell: pwsh, run: 'echo ${{ inputs.glb }}'}\n"),
        ('', '      - *extra\n'),
    ),
    'double_quoted_hex': (
        ('', '      - name: extra\n        run: "echo \\x24\\x7B\\x7B inputs.glb }}"\n'),
    ),
    'double_quoted_unicode': (
        ('', '      - name: extra\n        run: "echo \\u0024\\u007B\\u007B inputs.glb }}"\n'),
    ),
    'escaped_line_break': (
        ('', '      - name: extra\n        run: "echo $\\\n          {{ inputs.glb }}"\n'),
    ),
    'quoted_pull_request_target': (
        ('', '  "pull_request_target":\n'),
    ),
    'single_quoted_pull_request': (
        ('', "  'pull_request':\n"),
    ),
    'single_quoted_pull_request_target': (
        ('', "  'pull_request_target':\n"),
    ),
    'quoted_workflow_run': (
        ('', '  "workflow_run":\n'),
    ),
    'quoted_id_token': (
        ('', '  "id-token": write\n'),
    ),
    'job_write_all': (
        ('', '    permissions: write-all\n'),
    ),
    'job_id_token_write': (
        ('', '    permissions:\n      contents: write\n      id-token: write\n'),
    ),
    'indented_trigger': (
        ('', '   pull_request_target:\n'),
    ),
    'env_persist_credentials': (
        ('', '          persist-credentials: true\n        env:\n'),
    ),
    'second_checkout_true': (
        ('', '      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n        with:\n          persist-credentials: true\n'),
    ),
    'sibling_aw_glb': (
        ('', '          AW_GLB: ${{ github.event.pull_request.title }}\n        with:\n'),
    ),
    'duplicate_shell': (
        ('', '        shell: bash\n'),
    ),
    'pwsh_comment_glb': (
        ('          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "$env:AW_GLB"\n', '          # --glb "$env:AW_GLB"\n          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy\n'),
    ),
    'echo_glb_token': (
        ('          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "$env:AW_GLB"\n', '          echo \'--glb "$env:AW_GLB"\'\n          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy\n'),
    ),
    'local_composite_uses': (
        ('', '      - uses: ./.github/actions/gpu-capture\n        with:\n          glb: ${{ inputs.glb }}\n'),
    ),
    'job_level_uses': (
        ('', '  relay:\n    uses: ./.github/workflows/reusable-capture.yml\n    with:\n      glb: ${{ inputs.glb }}\n'),
    ),
    'shell_bash': (
        ('        shell: pwsh\n', '        shell: bash\n'),
    ),
    'zero_run_bodies': (
        ('        run: |\n          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "$env:AW_GLB"\n', ''),
    ),
    'unnamed_run': (
        ('', '      - run: echo "${{ inputs.glb }}"\n'),
    ),
    'unnamed_run_block': (
        ('', '      - run: |\n          echo "${{ inputs.glb }}"\n'),
    ),
    'commented_out_with': (
        ('        with:\n          persist-credentials: false\n', '        # with:\n        #   persist-credentials: false\n'),
    ),
    'inlined_expression': (
        ('          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "$env:AW_GLB"\n', '          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "${{ inputs.glb }}"\n'),
    ),
    'unquoted_glb': (
        ('          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "$env:AW_GLB"\n', '          cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb $env:AW_GLB\n'),
    ),
    'dropped_credentials': (
        ('          persist-credentials: false\n', ''),
    ),
    'block_header_2': (
        ('        run: |\n', '        run: |2\n          echo "${{ inputs.glb }}"\n'),
    ),
    'block_header_strip_2': (
        ('        run: |\n', '        run: |-2\n          echo "${{ inputs.glb }}"\n'),
    ),
    'block_header_2_strip': (
        ('        run: |\n', '        run: |2-\n          echo "${{ inputs.glb }}"\n'),
    ),
    'block_header_comment': (
        ('        run: |\n', '        run: | # c\n          echo "${{ inputs.glb }}"\n'),
    ),
    'folded_gt': (
        ('        run: |\n', '        run: >\n          echo "${{ inputs.glb }}"\n'),
    ),
    'kept_block_header': (
        ('        run: |\n', '        run: |+\n          echo "${{ inputs.glb }}"\n'),
    ),
    'comment_expression': (
        ('', '# ${{ inputs.glb }}\n'),
        ('        run: |\n', '        run: | # ${{ inputs.glb }}\n'),
    ),
    'comment_credentials': (
        ('          persist-credentials: false\n', '          persist-credentials: true\n          # persist-credentials: false\n'),
    ),
    'comment_aw_glb': (
        ('          AW_GLB: ${{ inputs.glb }}\n', '          # AW_GLB: ${{ inputs.glb }}\n          AW_GLB: ${{ github.event.pull_request.title }}\n'),
    ),
    'comment_trigger': (
        ('', '  # pull_request_target:\n'),
    ),
    'comment_permission': (
        ('', '  # id-token: write\n'),
    ),
    'pull_request_target': (
        ('', '  pull_request_target:\n'),
    ),
    'pull_request': (
        ('', '  pull_request:\n'),
    ),
    'workflow_run': (
        ('', '  workflow_run:\n'),
    ),
    'list_form_on': (
        ('on:\n  workflow_dispatch:\n', 'on: [workflow_dispatch, pull_request]\n'),
    ),
    'quoted_on_key': (
        ('on:\n', '"on":\n'),
    ),
    'id_token_write': (
        ('', '  id-token: write\n'),
    ),
    'top_level_write_all': (
        ('permissions:\n  contents: read\n', 'permissions: write-all\n'),
    ),
    'flow_permissions': (
        ('permissions:\n  contents: read\n', 'permissions: {contents: read, id-token: write}\n'),
    ),
    'flow_with': (
        ('        with:\n          persist-credentials: false\n', '        with: {persist-credentials: true}\n'),
    ),
    'script_line_hash': (
        ('', '          # ${{ inputs.glb }}\n'),
    ),
    'second_block_line': (
        ('', '          echo "${{ inputs.glb }}"\n'),
    ),
    'tag_run': (
        ('', '      - name: extra\n        run: !!str "${{ inputs.glb }}"\n'),
    ),
    'second_job_expression': (
        ('', '  other:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo "${{ inputs.glb }}"\n'),
    ),
    'trailing_space': (
        ('  contents: read\n', '  contents: read \n'),
    ),
    'permissions_space_before_colon': (
        ('', '    permissions : write-all\n'),
    ),
    'explicit_permissions_key': (
        ('', '    ? permissions\n    : write-all\n'),
    ),
    'quoted_permissions_write': (
        ('', '    "permissions":\n      contents: write\n      id-token: write\n'),
    ),
    'single_quoted_permissions_write': (
        ('', "    'permissions':\n      contents: write\n      id-token: write\n"),
    ),
    'plain_run_space_before_colon': (
        ('', '      - name: extra\n        run : Write-Output pwned\n'),
    ),
    'explicit_plain_run': (
        ('', '      - name: extra\n        ? run\n        : Write-Output pwned\n'),
    ),
    'uses_space_persist_credentials': (
        ('', '      - uses : actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1\n        with:\n          persist-credentials : true\n'),
    ),
    'relay_secrets_inherit': (
        ('', '  relay:\n    uses : ./.github/workflows/reusable-capture.yml\n    secrets : inherit\n'),
    ),
    'other_job_space_keys': (
        ('', '  other:\n    runs-on : ubuntu-latest\n    permissions : write-all\n    steps:\n      - run : echo pwned\n'),
    ),
    'other_job_quoted_keys': (
        ('', '  other:\n    "runs-on": ubuntu-latest\n    "permissions": write-all\n    steps:\n      - "run": echo pwned\n'),
    ),
    'path_beside_aw_glb': (
        ('', '          PATH: C:\\evil\n'),
    ),
    'working_directory_step': (
        ('', '        working-directory: C:\\evil\n'),
    ),
}


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


def _gpu_frame_budget_property_samples(golden: str) -> list[tuple[str, str]]:
    """Return `(message, text)` pairs for `WorkflowPolicyTest` property checks.

    The first 15 texts are `golden` plus one edit. The other 12 are the
    `gpu_frame_budget_policy_cases` texts named by
    `_GPU_FRAME_BUDGET_JOBS_LINE_FORMS`, in that order. 27 texts.
    The second text prefixes `golden` with a top-level `Xevil: write-all`
    line and fails only `gpu-frame-budget canonical keys`.
    """
    samples = [
        ("gpu-frame-budget canonical keys", golden + "env:\n  FOO: 1\n"),
        (
            "gpu-frame-budget canonical keys",
            "Xevil: write-all\n" + golden,
        ),
        (
            "gpu-frame-budget on is not workflow_dispatch only",
            _replace_once(golden, "on:\n", "on: [workflow_dispatch]\n"),
        ),
        (
            "gpu-frame-budget on is not workflow_dispatch only",
            _replace_once(
                golden,
                "  workflow_dispatch:\n",
                "  workflow_dispatch:\n  pull_request_target:\n",
            ),
        ),
        (
            "gpu-frame-budget on is not workflow_dispatch only",
            _replace_once(
                golden,
                "    inputs:\n",
                "    inputs:\n    pull_request:\n",
            ),
        ),
        (
            "gpu-frame-budget permissions are not contents: read",
            _replace_once(
                golden,
                "  contents: read\n",
                "  contents: read\n  id-token: write\n",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                "  capture:\n",
                "  capture:\n    permissions: write-all\n",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                _GPU_FRAME_BUDGET_RUNS_ON,
                "    runs-on: ubuntu-latest\n",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                "3d3c42e5aac5ba805825da76410c181273ba90b1",
                "3d3c42e5aac5ba805825da76410c181273ba90b0",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                "          persist-credentials: false\n",
                "          persist-credentials: true\n",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(golden, "        shell: pwsh\n", "        shell: bash\n"),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(golden, _GPU_FRAME_BUDGET_ENV, "          AW_GLB: literal\n"),
        ),
        ("gpu-frame-budget expression", "# ${{ inputs.glb }}\n" + golden),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                "      - name: Median draw_into\n",
                "      - run: echo x\n      - name: Median draw_into\n",
            ),
        ),
        (
            "gpu-frame-budget jobs block",
            _replace_once(
                golden,
                _GPU_FRAME_BUDGET_CARGO,
                _GPU_FRAME_BUDGET_CARGO.replace("--scene heavy", "--scene light"),
            ),
        ),
    ]
    by_name = dict(gpu_frame_budget_policy_cases(golden))
    for name in _GPU_FRAME_BUDGET_JOBS_LINE_FORMS:
        samples.append(("gpu-frame-budget jobs block", by_name[name]))
    return samples


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

    def test_gpu_frame_budget_dispatch_input_is_env(self) -> None:
        """The checked-in workflow matches the golden file after normalization.

        `_gpu_frame_budget_workflow_text` reads the workflow without following
        a symlink. `assert_gpu_frame_budget_dispatch_input_is_env` reads the
        golden the same way, requires
        `_assert_gpu_frame_budget_canonical_properties`, and compares the
        whole text after CRLF and CR become LF. The same text with CRLF or
        CR line endings passes. Every case from
        `gpu_frame_budget_policy_cases` fails
        `_assert_gpu_frame_budget_canonical_properties` and fails
        `assert_gpu_frame_budget_dispatch_input_is_env` with
        `gpu-frame-budget.yml is not the canonical workflow text`.
        """
        raw = _gpu_frame_budget_workflow_text()
        normalized = _normalize_line_endings(raw)
        assert_gpu_frame_budget_dispatch_input_is_env(raw)
        assert_gpu_frame_budget_dispatch_input_is_env(normalized.replace("\n", "\r\n"))
        assert_gpu_frame_budget_dispatch_input_is_env(normalized.replace("\n", "\r"))
        for name, mutant in gpu_frame_budget_policy_cases(raw):
            self.assertNotEqual(_normalize_line_endings(mutant), normalized, name)
            with self.assertRaises(AssertionError, msg=name):
                _assert_gpu_frame_budget_canonical_properties(
                    _normalize_line_endings(mutant)
                )
            with self.assertRaises(AssertionError, msg=name) as caught:
                assert_gpu_frame_budget_dispatch_input_is_env(mutant)
            self.assertEqual(
                str(caught.exception),
                "gpu-frame-budget.yml is not the canonical workflow text",
                name,
            )

    def test_gpu_frame_budget_canonical_properties(self) -> None:
        """Each of the 27 samples fails only the guard named by its message.

        The canonical text passes. `_assert_gpu_frame_budget_canonical_properties`
        raises the first message from `_gpu_frame_budget_guard_failures`.
        `name: x` fails keys, then `on`, then permissions, then the jobs
        block. Both canonical-keys samples and the expression sample keep
        `_GPU_FRAME_BUDGET_JOBS`. The second sample is the canonical text
        prefixed with a top-level `Xevil: write-all` line. Two `${{` lines
        before the canonical text produce one expression message. A line
        whose text is `{{` has no `$` and adds no
        `gpu-frame-budget expression` message. Nine samples are pinned to
        the `_replace_once` text that inserts `pull_request_target`,
        `pull_request`, `id-token: write`, `permissions: write-all`,
        `runs-on: ubuntu-latest`, `persist-credentials: true`,
        `shell: bash`, `AW_GLB: literal`, or `run: echo x`. The first
        `on` key is the block `_top_level_block` returns.
        """
        golden = _canonical_gpu_frame_budget_text()
        _assert_gpu_frame_budget_canonical_properties(golden)
        samples = _gpu_frame_budget_property_samples(golden)
        self.assertEqual(len(samples), 27)
        self.assertEqual(len(_GPU_FRAME_BUDGET_JOBS_LINE_FORMS), 12)
        keys_message, keys_sample = samples[0]
        self.assertEqual(keys_message, "gpu-frame-budget canonical keys")
        self.assertEqual(keys_sample, golden + "env:\n  FOO: 1\n")
        self.assertEqual(
            tuple(_top_level_block(keys_sample.splitlines(keepends=True), "jobs")),
            _GPU_FRAME_BUDGET_JOBS,
        )
        prefix_message, prefix_sample = samples[1]
        self.assertEqual(prefix_message, "gpu-frame-budget canonical keys")
        self.assertEqual(prefix_sample, "Xevil: write-all\n" + golden)
        self.assertEqual(
            tuple(_top_level_block(prefix_sample.splitlines(keepends=True), "jobs")),
            _GPU_FRAME_BUDGET_JOBS,
        )
        pinned = [text for _message, text in samples if "ba90b0" in text]
        self.assertEqual(pinned, [
            _replace_once(
                golden,
                "3d3c42e5aac5ba805825da76410c181273ba90b1",
                "3d3c42e5aac5ba805825da76410c181273ba90b0",
            )
        ])
        lit = [text for _message, text in samples if "--scene light" in text]
        self.assertEqual(lit, [
            _replace_once(
                golden,
                _GPU_FRAME_BUDGET_CARGO,
                _GPU_FRAME_BUDGET_CARGO.replace("--scene heavy", "--scene light"),
            )
        ])
        insertions = (
            (
                "  workflow_dispatch:\n",
                "  workflow_dispatch:\n  pull_request_target:\n",
            ),
            (
                "    inputs:\n",
                "    inputs:\n    pull_request:\n",
            ),
            (
                "  contents: read\n",
                "  contents: read\n  id-token: write\n",
            ),
            (
                "  capture:\n",
                "  capture:\n    permissions: write-all\n",
            ),
            (
                _GPU_FRAME_BUDGET_RUNS_ON,
                "    runs-on: ubuntu-latest\n",
            ),
            (
                "          persist-credentials: false\n",
                "          persist-credentials: true\n",
            ),
            (
                "        shell: pwsh\n",
                "        shell: bash\n",
            ),
            (
                _GPU_FRAME_BUDGET_ENV,
                "          AW_GLB: literal\n",
            ),
            (
                "      - name: Median draw_into\n",
                "      - run: echo x\n      - name: Median draw_into\n",
            ),
        )
        for old, new in insertions:
            self.assertEqual(
                [text for _message, text in samples if new in text],
                [_replace_once(golden, old, new)],
                new,
            )
        self.assertNotIn("$", "{{\n")
        self.assertEqual(
            _gpu_frame_budget_guard_failures("{{\n" + golden),
            ["gpu-frame-budget canonical keys"],
        )
        expression = [
            text
            for message, text in samples
            if message == "gpu-frame-budget expression"
        ]
        self.assertEqual(len(expression), 1)
        self.assertEqual(
            tuple(_top_level_block(expression[0].splitlines(keepends=True), "jobs")),
            _GPU_FRAME_BUDGET_JOBS,
        )
        self.assertEqual(
            _gpu_frame_budget_guard_failures("# ${{ a }}\n# ${{ b }}\n" + golden),
            ["gpu-frame-budget expression"],
        )
        self.assertEqual(
            _top_level_block(["on:\n", "  a:\n", "on:\n", "  b:\n"], "on"),
            ["on:\n", "  a:\n"],
        )
        self.assertEqual(_top_level_block(["name: x\n"], "jobs"), [])
        bare = "name: x\n"
        self.assertEqual(
            _gpu_frame_budget_guard_failures(bare),
            [
                "gpu-frame-budget canonical keys",
                "gpu-frame-budget on is not workflow_dispatch only",
                "gpu-frame-budget permissions are not contents: read",
                "gpu-frame-budget jobs block",
            ],
        )
        with self.assertRaises(AssertionError) as caught:
            _assert_gpu_frame_budget_canonical_properties(bare)
        self.assertEqual(str(caught.exception), "gpu-frame-budget canonical keys")
        for message, sample in samples:
            self.assertEqual(
                _gpu_frame_budget_guard_failures(sample), [message], message
            )
            with self.assertRaises(AssertionError, msg=message) as caught:
                _assert_gpu_frame_budget_canonical_properties(sample)
            self.assertEqual(str(caught.exception), message, message)

    def test_gpu_frame_budget_paired_golden_fails_properties(self) -> None:
        """Each property sample fails when it is both the golden and the workflow.

        `_gpu_frame_budget_property_samples` supplies the 27 texts. The
        golden path is that text. `assert_gpu_frame_budget_dispatch_input_is_env`
        receives the same text and raises that sample's message.
        """
        global _GPU_FRAME_BUDGET_GOLDEN
        golden = _canonical_gpu_frame_budget_text()
        samples = _gpu_frame_budget_property_samples(golden)
        self.assertEqual(len(samples), 27)
        previous = _GPU_FRAME_BUDGET_GOLDEN
        try:
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "gpu_frame_budget_workflow.golden.yml"
                _GPU_FRAME_BUDGET_GOLDEN = path
                for message, sample in samples:
                    path.write_text(sample, encoding="utf-8")
                    with self.assertRaises(AssertionError, msg=message) as caught:
                        assert_gpu_frame_budget_dispatch_input_is_env(sample)
                    self.assertEqual(str(caught.exception), message, message)
        finally:
            _GPU_FRAME_BUDGET_GOLDEN = previous

    def test_gpu_frame_budget_symlink_paths_fail_closed(self) -> None:
        """A symlink at the workflow path or the golden path fails closed.

        `_read_regular_utf8` keeps a leading BOM. A missing path raises
        `FileNotFoundError`.
        """
        global WORKFLOWS, _GPU_FRAME_BUDGET_GOLDEN
        real_workflow = (WORKFLOWS / "gpu-frame-budget.yml").read_bytes()
        real_golden = _GPU_FRAME_BUDGET_GOLDEN.read_bytes()
        previous_workflows = WORKFLOWS
        previous_golden = _GPU_FRAME_BUDGET_GOLDEN
        try:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                workflow_target = root / "workflow-target.yml"
                workflow_target.write_bytes(real_workflow)
                workflow_link = root / "gpu-frame-budget.yml"
                workflow_link.symlink_to(workflow_target)
                WORKFLOWS = root
                with self.assertRaises(AssertionError) as caught:
                    _gpu_frame_budget_workflow_text()
                self.assertEqual(
                    str(caught.exception),
                    "gpu-frame-budget.yml path is a symlink",
                )
                golden_target = root / "golden-target.yml"
                golden_target.write_bytes(real_golden)
                golden_link = root / "gpu_frame_budget_workflow.golden.yml"
                golden_link.symlink_to(golden_target)
                _GPU_FRAME_BUDGET_GOLDEN = golden_link
                with self.assertRaises(AssertionError) as caught:
                    assert_gpu_frame_budget_dispatch_input_is_env(
                        real_golden.decode("utf-8")
                    )
                self.assertEqual(
                    str(caught.exception),
                    "gpu_frame_budget_workflow.golden.yml path is a symlink",
                )
                kept = root / "kept.yml"
                kept.write_bytes(b"\xef\xbb\xbfname: x\n")
                self.assertTrue(_read_regular_utf8(kept).startswith("\ufeff"))
                missing = root / "missing.yml"
                with self.assertRaises(FileNotFoundError):
                    _read_regular_utf8(missing)
        finally:
            WORKFLOWS = previous_workflows
            _GPU_FRAME_BUDGET_GOLDEN = previous_golden

    def test_gpu_frame_budget_fixtures_match_named_forms(self) -> None:
        """Each policy fixture's line diff equals the form stored under its name.

        The case names and order equal `_GPU_FRAME_BUDGET_NAMED_FORMS`.
        A fixture that differs from the canonical text in any other line fails.
        `_GPU_FRAME_BUDGET_JOBS_LINE_FORMS` is 12 names in that list, and each
        of those texts fails with `gpu-frame-budget jobs block` only.
        """
        raw = _gpu_frame_budget_workflow_text()
        normalized = _normalize_line_endings(raw)
        cases = gpu_frame_budget_policy_cases(raw)
        self.assertEqual(
            [name for name, _mutant in cases],
            list(_GPU_FRAME_BUDGET_NAMED_FORMS),
        )
        for name, mutant in cases:
            self.assertEqual(
                _line_diff_spans(normalized, _normalize_line_endings(mutant)),
                _GPU_FRAME_BUDGET_NAMED_FORMS[name],
                name,
            )
        self.assertEqual(
            [name for name in _GPU_FRAME_BUDGET_JOBS_LINE_FORMS],
            [
                "permissions_space_before_colon",
                "explicit_permissions_key",
                "quoted_permissions_write",
                "single_quoted_permissions_write",
                "plain_run_space_before_colon",
                "explicit_plain_run",
                "uses_space_persist_credentials",
                "relay_secrets_inherit",
                "other_job_space_keys",
                "other_job_quoted_keys",
                "path_beside_aw_glb",
                "working_directory_step",
            ],
        )
        for name in _GPU_FRAME_BUDGET_JOBS_LINE_FORMS:
            self.assertEqual(
                _gpu_frame_budget_guard_failures(
                    _normalize_line_endings(dict(cases)[name])
                ),
                ["gpu-frame-budget jobs block"],
                name,
            )

    def test_gpu_frame_budget_fixture_list_rejects_duplicates_and_canonical(
        self,
    ) -> None:
        """`_reject_gpu_frame_budget_fixture_list` rejects both bad lists.

        A repeated name raises `duplicate policy fixture name`.
        A fixture equal to the canonical text raises
        `{name} matches the canonical text`.
        """
        with self.assertRaises(AssertionError) as duplicate:
            _reject_gpu_frame_budget_fixture_list([("a", "x"), ("a", "y")], "z")
        self.assertEqual(str(duplicate.exception), "duplicate policy fixture name")
        with self.assertRaises(AssertionError) as same:
            _reject_gpu_frame_budget_fixture_list([("kept", "z")], "z")
        self.assertEqual(str(same.exception), "kept matches the canonical text")

    def test_replace_once_rejects_zero_and_two_anchors(self) -> None:
        """`_replace_once` raises when the anchor matches 0 times or 2 times.

        Zero matches raise `policy fixture anchor missing`.
        Two matches raise `policy fixture anchor matched more than once`.
        """
        with self.assertRaises(AssertionError) as zero:
            _replace_once("alpha", "missing", "x")
        self.assertEqual(str(zero.exception), "policy fixture anchor missing")
        with self.assertRaises(AssertionError) as two:
            _replace_once("alpha alpha", "alpha", "beta")
        self.assertEqual(
            str(two.exception), "policy fixture anchor matched more than once"
        )
        self.assertEqual(_replace_once("alpha", "alpha", "beta"), "beta")


if __name__ == "__main__":
    unittest.main()

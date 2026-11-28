#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

import inspect
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

import yaml

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


BENCHMARK_ALERT_PUBLISH_DIR = "./docs/benchmark_data"
BENCHMARK_ALERT_DESTINATION_DIR = "dashboard/benchmark_data"
BENCHMARK_ALERT_GH_PAGES_STEP = "Deploy dashboard to GitHub Pages"
BENCHMARK_ALERT_PUBLISH_BRANCH = "gh-pages"
PEACEIRIS_GH_PAGES = "peaceiris/actions-gh-pages"
ARCHIVE_BENCHMARK_DASHBOARD = "docs/archive/benchmark_dashboard"
BOOK_DASHBOARD_DIR = "docs/book/dashboard"
BOOK_BENCHMARK_DASHBOARD = "docs/book/dashboard/benchmark_dashboard"
_PAGES_USE_MARKERS = ("gh-pages", "github-pages")
_COPY_COMMANDS = ("cp", "mv", "rsync")
_BRANCH_WORDS = ("if", "elif", "then", "else", "fi")
_WORKSPACE_PREFIXES = (
    "${GITHUB_WORKSPACE}/",
    "$GITHUB_WORKSPACE/",
    "GITHUB_WORKSPACE/",
)
_GITHUB_WORKSPACE_EXPR = re.compile(
    r"\$\{\{\s*github\.workspace\s*\}\}",
    re.IGNORECASE,
)
_ECHO_LITERAL = re.compile(r"\$\(\s*echo\s+(?:-n\s+)?(.+?)\s*\)\Z")
_ECHO_BACKTICK = re.compile(r"`\s*echo\s+(?:-n\s+)?(.+?)\s*`\Z")
_ECHO_METACHARACTERS = frozenset("$`;&|<>(){}\\")
_TARGET_DIRECTORY_PREFIXES = ("--target-directory=", "--target=")
_ASSIGNMENT_WORD = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$")
_JAMESIVES_DEPLOY = "github-pages-deploy-action"
_JAMESIVES_FOLDER = "docs/benchmark_data"
_JAMESIVES_TARGET = "dashboard/benchmark_data"
_LIVE_DASHBOARD_DEST = "dashboard/benchmark_dashboard"
_SHELL_STATE_CAP = 32
_REDIRECT_WORDS = frozenset({">", ">>", "<", "<<", "<>", ">&", "<&", ">|", "&>", "&>>"})
_REDIRECT_COMBINED = re.compile(
    r"^(?:&>>|&>|\d*>>|\d*>&|\d*<&|\d*>\||\d*<>|\d*>|\d*<)\S+"
)
_REDIRECT_OP = re.compile(r"^(?:&>>|&>|\d*>>|\d*>&|\d*<&|\d*>\||\d*<>|\d*>|\d*<)$")


def _workflow_document(text: str, *, path: str) -> dict:
    try:
        data = yaml.safe_load(text)
    except yaml.YAMLError as exc:
        raise AssertionError(f"{path}: workflow YAML is invalid: {exc}") from exc
    if not isinstance(data, dict):
        raise AssertionError(f"{path}: workflow root is not a mapping")
    return data


def _require_jobs(data: dict, *, path: str) -> dict:
    jobs = data.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        raise AssertionError(f"{path}: missing jobs")
    return jobs


def _workflow_jobs(text: str, *, path: str) -> dict:
    return _require_jobs(_workflow_document(text, path=path), path=path)


def _step_run_text(step: dict) -> str:
    run = step.get("run", "")
    if isinstance(run, list):
        return "\n".join(str(line) for line in run)
    if isinstance(run, str):
        return run
    if run is None:
        return ""
    return str(run)


def _read_policy_fixture(directory: str, filename: str) -> str:
    fixture = ROOT / "scripts" / "ci" / "testdata" / directory / filename
    try:
        return fixture.read_text(encoding="utf-8")
    except (LookupError, UnicodeError, OSError) as exc:
        raise AssertionError(f"could not read policy fixture {fixture}: {exc}") from exc


def _posix_norm(path: str) -> str:
    parts: list[str] = []
    for part in str(path).replace("\\", "/").split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            if parts:
                parts.pop()
            continue
        parts.append(part)
    return "/".join(parts)


def _strip_workspace(token: str) -> str:
    """Drop a workspace root prefix so the remainder is workspace-relative."""
    stripped = _GITHUB_WORKSPACE_EXPR.sub("", token)
    for prefix in _WORKSPACE_PREFIXES:
        if stripped.startswith(prefix):
            # The remainder is workspace-root absolute, under any working directory.
            return "/" + stripped[len(prefix) :]
    return stripped


def _echo_inner(word: str) -> str | None:
    """Inner text of echo or echo -n when $(...) or backticks are the whole word."""
    match = _ECHO_LITERAL.fullmatch(word)
    if match is None:
        match = _ECHO_BACKTICK.fullmatch(word)
    if match is None:
        return None
    return match.group(1).strip()


def _unwrap_echo_literal(word: str) -> str:
    """Reduce echo of one literal path. Leave every other substitution unchanged."""
    inner = _echo_inner(word)
    if inner is None:
        return word
    if len(inner) >= 2 and inner[0] in "'\"" and inner[-1] == inner[0]:
        inner = inner[1:-1]
    if inner == "" or any(char.isspace() for char in inner):
        return word
    if any(char in _ECHO_METACHARACTERS for char in inner):
        return word
    return inner


def _resolve_run_path(token: str, cwd: str) -> str:
    """Resolve one shell word against cwd.

    A relative word is joined onto cwd. A word that begins with ``/`` is
    resolved from the workspace root. A ``$GITHUB_WORKSPACE``,
    ``${GITHUB_WORKSPACE}``, or ``GITHUB_WORKSPACE`` prefix is that root too.
    """
    token = _unwrap_echo_literal(token.strip())
    token = _strip_workspace(token.strip())
    if token.endswith("/*"):
        token = token[:-2]
    slash = token.replace("\\", "/")
    if slash.startswith("/"):
        return _posix_norm(slash)
    cwd_norm = _posix_norm(cwd)
    if not cwd_norm:
        return _posix_norm(slash)
    return _posix_norm(f"{cwd_norm}/{slash}")


def _glob_escape(char: str) -> str:
    if char in "*?[":
        return "[" + char + "]"
    return char


def _closing_paren(text: str, start: int) -> int:
    """Index of the ``)`` matching the ``(`` at start. Last index when unclosed."""
    depth = 0
    quote: str | None = None
    index = start
    limit = len(text)
    previous = start - 1
    while index < limit:
        if index <= previous:
            raise AssertionError("paren scan did not advance")
        previous = index
        char = text[index]
        if quote is not None:
            if quote == '"' and char == "\\" and index + 1 < limit:
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in "'\"":
            quote = char
            index += 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return max(limit - 1, 0)


def _closing_backtick(text: str, start: int) -> int:
    index = start + 1
    limit = len(text)
    previous = start
    while index < limit:
        if index <= previous:
            raise AssertionError("backtick scan did not advance")
        previous = index
        if text[index] == "\\" and index + 1 < limit:
            index += 2
            continue
        if text[index] == "`":
            return index
        index += 1
    return max(limit - 1, 0)


def _shell_words(command: str) -> list[tuple[str, bool, str | None]]:
    """Words of one command.

    The bool is true when any character was quoted. The third value is a glob
    pattern when an unquoted ``*``, ``?``, or ``[`` remains, and None otherwise.
    A quoted metacharacter stays literal. ``$(...)`` and backticks stay one word.
    A quoted ``if``, ``elif``, ``then``, ``else``, or ``fi`` is not a reserved word.
    """
    words: list[tuple[str, bool, str | None]] = []
    index = 0
    limit = len(command)
    previous = -1
    while index < limit:
        if index <= previous:
            raise AssertionError("shell word scan did not advance")
        previous = index
        space = index
        while index < limit and command[index].isspace():
            if index < space:
                raise AssertionError("shell word scan did not advance")
            space = index + 1
            index += 1
        if index >= limit:
            break
        buf: list[str] = []
        glob_buf: list[str] = []
        quoted_any = False
        unquoted_glob = False
        quote: str | None = None
        word_at = index
        while index < limit:
            if index < word_at:
                raise AssertionError("shell word scan did not advance")
            word_at = index + 1
            char = command[index]
            if quote is not None:
                if char == quote:
                    quote = None
                else:
                    buf.append(char)
                    glob_buf.append(_glob_escape(char))
                    quoted_any = True
                index += 1
                continue
            if char in "'\"":
                quote = char
                index += 1
                continue
            if char.isspace():
                break
            if char == "$" and index + 1 < limit and command[index + 1] == "(":
                end = _closing_paren(command, index + 1)
                chunk = command[index : end + 1]
                buf.append(chunk)
                glob_buf.extend(_glob_escape(item) for item in chunk)
                index = end + 1
                continue
            if char == "`":
                end = _closing_backtick(command, index)
                chunk = command[index : end + 1]
                buf.append(chunk)
                glob_buf.extend(_glob_escape(item) for item in chunk)
                index = end + 1
                continue
            buf.append(char)
            glob_buf.append(char)
            if char in "*?[":
                unquoted_glob = True
            index += 1
        if not buf and not quoted_any:
            continue
        pattern = "".join(glob_buf) if unquoted_glob else None
        words.append(("".join(buf), quoted_any, pattern))
    return words


def _redirect_operator_end(text: str, index: int, current: str) -> int:
    """End index of a redirection operator that contains ``&`` or ``|``.

    ``index`` points at that ``&`` or ``|``. The return is ``index`` when the
    character is a command separator instead.
    """
    if index >= len(text) or text[index] not in "&|":
        return index
    nxt = text[index + 1] if index + 1 < len(text) else ""
    if text[index] == "&" and nxt == "&":
        return index
    if text[index] == "|" and nxt == "|":
        return index
    stripped = current.rstrip(" \t")
    space = max(stripped.rfind(" "), stripped.rfind("\t"), stripped.rfind("\n"))
    tail = stripped[space + 1 :] if stripped else ""
    starts = False
    if text[index] == "&" and (tail.endswith(">") or nxt == ">"):
        starts = True
    elif text[index] == "|" and tail.endswith(">"):
        starts = True
    elif text[index] == "|" and nxt == "&":
        return index
    if not starts:
        return index
    end = index + 1
    if text[index] == "&" and nxt == ">":
        end = index + 2
        if end < len(text) and text[end] == ">":
            end += 1
    previous_end = index
    while end < len(text) and text[end] not in " \t\n;&|":
        if end <= previous_end:
            raise AssertionError("redirect scan did not advance")
        previous_end = end
        end += 1
    return end


def _split_shell_commands(run_text: str) -> list[tuple[str, bool, bool, bool]]:
    """Split on newline, `;`, `&&`, `||`, `|`, and a single `&`.

    A redirection operator that contains ``&`` or ``|`` is not a separator.

    The second bool is true when a directory change in that command stays in
    effect for the next command. Newline, `;`, `&&`, and `||` carry it. A
    single `|` and a single `&` do not. `&&` and `||` are one operator even
    when the next command is flush against them. The third bool is true when
    this command sits before `||`, or before `&&` and another command follows,
    so a `cd` or `pushd` in it may leave the earlier directory in place. The
    fourth bool is true when this command follows `&&` or `||`, so it may not
    run. A backslash immediately before a newline continues the command. The
    scan raises if the index does not advance.
    """
    pieces: list[tuple[str, bool, str]] = []
    current: list[str] = []
    quote: str | None = None
    index = 0
    previous = -1
    while index < len(run_text):
        if index <= previous:
            raise AssertionError("shell split did not advance")
        previous = index
        char = run_text[index]
        if quote is not None:
            if (
                quote == '"'
                and char == "\\"
                and index + 1 < len(run_text)
                and run_text[index + 1] == "\n"
            ):
                index += 2
                continue
            current.append(char)
            if char == quote:
                quote = None
            index += 1
            continue
        if char in ("'", '"'):
            quote = char
            current.append(char)
            index += 1
            continue
        if char == "\\" and index + 1 < len(run_text) and run_text[index + 1] == "\n":
            index += 2
            continue
        if char in ("|", "&"):
            redirect_end = _redirect_operator_end(run_text, index, "".join(current))
            if redirect_end > index:
                current.append(run_text[index:redirect_end])
                index = redirect_end
                continue
            doubled = index + 1 < len(run_text) and run_text[index + 1] == char
            # && and || run in the current shell. A single | is a pipeline
            # and a single & backgrounds the command. One advance consumes the
            # operator, so the next character stays in the following command
            # when there is no space.
            carry = doubled
            separator = "or" if char == "|" and doubled else "and" if doubled else "break"
            index += 2 if doubled else 1
            pieces.append(("".join(current), carry, separator))
            current = []
            continue
        if char in ("\n", ";"):
            pieces.append(("".join(current), True, "seq"))
            current = []
            index += 1
            continue
        current.append(char)
        index += 1
    pieces.append(("".join(current), True, "end"))
    kept = [
        (command.strip(), carry, separator)
        for command, carry, separator in pieces
        if command.strip()
    ]
    commands: list[tuple[str, bool, bool, bool]] = []
    follows = False
    for position, (command, carry, separator) in enumerate(kept):
        later = position + 1 < len(kept)
        may_fail = separator == "or" or (separator == "and" and later)
        commands.append((command, carry, may_fail, follows))
        follows = separator in ("and", "or")
    return commands


def _leading_redirect_span(words: list[str], index: int) -> int:
    """Words used by a leading redirection at index. Zero when there is none.

    A file-descriptor operator such as ``2>`` takes the next word as its target.
    """
    word = words[index]
    if word in _REDIRECT_WORDS or _REDIRECT_OP.fullmatch(word):
        return 2 if index + 1 < len(words) else 1
    if _REDIRECT_COMBINED.fullmatch(word):
        return 1
    return 0


def _command_body(words: list[str], quoted: list[bool] | None = None) -> list[str]:
    """Drop leading branch words, `{` group tokens, redirections, and `command`.

    `if`, `elif`, `then`, `else`, and `fi` are skipped when the word is
    unquoted, so a copy or directory change in that branch is read, including
    when it shares the command with `then` or `else`. A quoted reserved word
    stays in place. A leading redirection is skipped before `cd`, `pushd`,
    `popd`, and a copy. `command` or `builtin` is skipped even when the word is
    quoted. `command` followed by an option stays in place. Brace
    expansion is one word and stays in place. A parenthesized subshell is not
    removed. The scan raises if the index does not advance.
    """
    if quoted is None:
        quoted = [False] * len(words)
    index = 0
    limit = len(words)
    previous = -1
    while index < limit:
        if index <= previous:
            raise AssertionError("command scan did not advance")
        previous = index
        if not quoted[index] and (words[index] in _BRANCH_WORDS or words[index] == "{"):
            index += 1
            continue
        if not quoted[index]:
            span = _leading_redirect_span(words, index)
            if span:
                index += span
                continue
        if (
            words[index] in ("command", "builtin")
            and index + 1 < limit
            and not words[index + 1].startswith("-")
        ):
            index += 1
            continue
        break
    return words[index:]


def _directory_update(words: list[str], current: str, stack: list[str]) -> str | None:
    """Return the cwd after cd, pushd, or popd. None when the command is not one of those.

    cd and pushd use the second word. An option in the pushd word is not a
    directory. popd pops the directory stack and does not read a directory word.
    """
    command = words[0]
    if command == "popd":
        if stack:
            return stack.pop()
        return current
    if command == "cd":
        if len(words) > 1:
            return _resolve_run_path(words[1], current)
        return current
    if command == "pushd":
        if len(words) > 1 and not words[1].startswith("-"):
            stack.append(current)
            return _resolve_run_path(words[1], current)
        return current
    return None


def _has_glob(word: str) -> bool:
    return any(char in word for char in "*?[")


def _glob_directories(word: str, current: str) -> list[str]:
    """Directories under the repo that match one cd or pushd word."""
    logical = _resolve_run_path(word, current)
    if not logical or logical.startswith("/"):
        return []
    found: list[str] = []
    for path in sorted(ROOT.glob(logical)):
        if path.is_dir():
            found.append(path.relative_to(ROOT).as_posix())
    return found


def _repo_dir(logical: str) -> bool:
    if not logical or logical.startswith("/"):
        return False
    candidate = (ROOT / logical).resolve()
    try:
        candidate.relative_to(ROOT.resolve())
    except ValueError:
        return False
    return candidate.is_dir()


def _cd_uses_cdpath(word: str, env: dict[str, str]) -> bool:
    if "CDPATH" not in env or not word or word == "-":
        return False
    if word.startswith("/") or word.startswith(".") or "/" in word:
        return False
    return True


def _cdpath_directory(operand: str, cwd: str, cdpath: str) -> str | None:
    """Directory bash selects for a slash-free cd operand when CDPATH is set."""
    for entry in cdpath.split(":"):
        base = cwd if entry == "" else _resolve_run_path(entry, cwd)
        candidate = _posix_norm(f"{base}/{operand}" if base else operand)
        if _repo_dir(candidate):
            return candidate
    local = _resolve_run_path(operand, cwd)
    if _repo_dir(local):
        return local
    return None


def _directory_outcomes(
    rest: list[str],
    quoted: list[bool],
    globs: list[str | None],
    state: _ShellState,
) -> list[tuple[str, tuple[str, ...]]] | None:
    """Directories one cd, pushd, or popd can leave in place.

    A word with an unquoted ``*``, ``?``, or ``[`` that matches exactly one
    directory uses that directory. A quoted metacharacter stays literal. More
    than one match returns None. The caller keeps the earlier state. While
    errexit is on, an unconditional failure runs no later command in that state.
    ``$(echo ...)`` and backtick echo of one literal path are reduced first
    when that substitution is the whole word.
    """
    words = list(rest)
    pattern = globs[1] if len(globs) > 1 else None
    env = dict(state.env)
    if len(words) > 1:
        expanded = _unwrap_echo_literal(
            _expand_assigned_word(words[1], env, state.positionals)
        )
        word_quoted = quoted[1] if len(quoted) > 1 else False
        if expanded != words[1]:
            words[1] = expanded
            if word_quoted or not _has_glob(expanded):
                pattern = None
            else:
                pattern = expanded
    word = words[1] if len(words) > 1 else ""
    command = words[0] if words else ""
    if command == "cd" and word == "-":
        if not state.oldpwd:
            return None
        return [(state.oldpwd, tuple(state.stack))]
    if command == "cd" and _cd_uses_cdpath(word, env):
        found = _cdpath_directory(word, state.cwd, env["CDPATH"])
        if found is None:
            return None
        return [(found, tuple(state.stack))]
    if (
        command in ("cd", "pushd")
        and pattern
        and not (command == "pushd" and word.startswith("-"))
    ):
        matches = _glob_directories(pattern, state.cwd)
        if len(matches) > 1:
            return None
        if len(matches) == 1:
            stack = list(state.stack)
            if command == "pushd":
                stack.append(state.cwd)
            return [(matches[0], tuple(stack))]
    stack = list(state.stack)
    updated = _directory_update(words, state.cwd, stack)
    cwd = state.cwd if updated is None else updated
    return [(cwd, tuple(stack))]


def _next_errexit(words: list[str], quoted: list[bool], enabled: bool) -> bool:
    """Errexit after a set command. Other commands leave it unchanged.

    ``+e``, ``-e``, ``+o errexit``, and ``-o errexit`` apply when the option
    word is quoted. ``set --`` ends option scanning. An assignment,
    ``builtin``, ``command``, or ``eval`` may stand in front of ``set``. A
    following word that itself starts with ``+`` or ``-`` is not the name of
    ``-o``.
    """
    if not words:
        return enabled
    index = 0
    previous = -1
    limit = len(words)
    while index < limit:
        if index <= previous:
            raise AssertionError("errexit scan did not advance")
        previous = index
        if _ASSIGNMENT_WORD.match(words[index]):
            index += 1
            continue
        if (
            words[index] in ("builtin", "command", "eval")
            and index + 1 < limit
            and not words[index + 1].startswith("-")
        ):
            index += 1
            continue
        break
    if index >= limit or words[index] != "set":
        return enabled
    index += 1
    while index < limit:
        if index <= previous:
            raise AssertionError("errexit scan did not advance")
        previous = index
        arg = words[index]
        if arg == "--":
            break
        if arg in ("+o", "-o"):
            nxt = words[index + 1] if index + 1 < limit else ""
            if nxt and nxt[0] not in "+-":
                if nxt == "errexit":
                    enabled = arg == "-o"
                index += 2
                continue
            index += 1
            continue
        if len(arg) >= 2 and arg[0] in "+-" and arg[1:].isalpha() and "e" in arg[1:]:
            enabled = arg[0] == "-"
        index += 1
    return enabled


def _command_basename(word: str) -> str:
    return word.replace("\\", "/").rsplit("/", 1)[-1]


def _recordable_assignment(value: str) -> bool:
    return "$" not in value and "`" not in value


def _lookup_parameter(
    name: str, env: dict[str, str], positionals: tuple[str, ...]
) -> str | None:
    """Value of one parameter. None leaves the original token in the word.

    ``$GITHUB_WORKSPACE`` with no assignment is the workspace root. An unset
    positional stays written out, so a word the shell did not set is unchanged.
    """
    if name.isdigit():
        number = int(name)
        if number <= 0 or number > len(positionals):
            return None
        return positionals[number - 1]
    if name == "GITHUB_WORKSPACE" and name not in env:
        return ""
    if name in env:
        return env[name]
    return None


def _expand_parameters(
    word: str,
    env: dict[str, str],
    positionals: tuple[str, ...] = (),
    depth: int = 0,
) -> str:
    """Expand ``$name``, ``${name}``, ``$n``, and ``${n}`` in one word.

    ``$10`` is ``$1`` followed by ``0``. ``${10}`` is positional 10. More text
    after the parameter stays in the word. ``$(...)`` and backticks stay in
    place. A value that still holds a parameter is expanded again. The scan
    raises if its index does not advance.
    """
    if depth > 8 or "$" not in word:
        return word
    out: list[str] = []
    index = 0
    limit = len(word)
    changed = False
    previous = -1
    steps = 0
    while index < limit:
        steps += 1
        if steps > limit + 1 or index <= previous:
            raise AssertionError("parameter scan did not advance")
        previous = index
        char = word[index]
        if char == "$" and index + 1 < limit and word[index + 1] == "(":
            end = _closing_paren(word, index + 1)
            out.append(word[index : end + 1])
            index = end + 1
            continue
        if char == "`":
            end = _closing_backtick(word, index)
            out.append(word[index : end + 1])
            index = end + 1
            continue
        if char == "$" and index + 1 < limit:
            nxt = word[index + 1]
            if nxt == "{":
                close = word.find("}", index + 2)
                name = word[index + 2 : close] if close > index + 2 else ""
                if name.isidentifier() or name.isdigit():
                    value = _lookup_parameter(name, env, positionals)
                    if value is None:
                        out.append(word[index : close + 1])
                    else:
                        out.append(value)
                        changed = True
                    index = close + 1
                    continue
            if nxt.isdigit():
                value = _lookup_parameter(nxt, env, positionals)
                if value is None:
                    out.append(word[index : index + 2])
                else:
                    out.append(value)
                    changed = True
                index += 2
                continue
            if nxt.isalpha() or nxt == "_":
                end = index + 2
                previous_end = index + 1
                while end < limit and (word[end].isalnum() or word[end] == "_"):
                    if end <= previous_end:
                        raise AssertionError("parameter scan did not advance")
                    previous_end = end
                    end += 1
                name = word[index + 1 : end]
                value = _lookup_parameter(name, env, positionals)
                if value is None:
                    out.append(word[index:end])
                else:
                    out.append(value)
                    changed = True
                index = end
                continue
        out.append(char)
        index += 1
    expanded = "".join(out)
    if changed and expanded != word:
        return _expand_parameters(expanded, env, positionals, depth + 1)
    return expanded


def _expand_assignment_value(
    value: str, env: dict[str, str], positionals: tuple[str, ...]
) -> str:
    """Expand parameters and a workspace prefix in an assignment value.

    A colon separates ``CDPATH`` entries. Each entry is expanded on its own.
    """
    expanded = _expand_parameters(value, env, positionals)
    parts = [
        _strip_workspace(part) if part else "" for part in expanded.split(":")
    ]
    return ":".join(parts)


def _expand_assigned_word(
    word: str, env: dict[str, str], positionals: tuple[str, ...] = ()
) -> str:
    """Expand parameters in a directory or copy word, including a stored value.

    A whole-word ``$name`` or ``${name}`` is included, and so is a parameter
    with more text in the same word. A stored value that holds a parameter is
    expanded again.
    """
    return _expand_parameters(word, env, positionals)


def _path_from_copy_word(command: str, word: str) -> str | None:
    """Path argument of a copy command, including cp/mv --target-directory= and --target=."""
    if command in ("cp", "mv"):
        for prefix in _TARGET_DIRECTORY_PREFIXES:
            if word.startswith(prefix):
                return word[len(prefix) :]
    if word.startswith("-"):
        return None
    return word


def _run_paths(run_text: str, cwd: str) -> list[str]:
    """Paths copied by cp, mv, or rsync, after cd, pushd, popd, and working-directory.

    The copy command is the basename of the command word. A leading command
    or builtin word is skipped before cd, pushd, popd, and a copy, including
    when that word is quoted, and so is a leading
    { group token. A leading redirection is skipped the same way, including a
    file-descriptor redirection and more than one redirection. A leading
    if, elif, then, else, or fi is skipped when the word is unquoted, so a
    copy or directory change in that branch is read, including when it shares
    the command with then or else. A quoted reserved word stays in place. A
    cd, pushd, popd, or assignment-only command inside an if compound is kept
    together with the state from before that command. A cd or pushd before ||,
    or before && when another command follows, keeps the earlier directory
    beside the new one. A command after && or ||, including an assignment-only
    command, may not run, so the earlier state is kept beside the new one.
    set +e, set "+e", set +o errexit, and set -o errexit change errexit,
    including when the option word is quoted. An assignment, builtin, command,
    or eval prefix may stand in front of set. A set inside an if compound or
    after && or || keeps the earlier errexit beside the new one. set +e does
    the same for a later cd or pushd until set -e. A later copy is checked
    against each of those states. An unconditional cd, pushd, popd, or
    assignment still replaces the state. A relative word is joined onto the
    current directory. A word that begins with / is resolved from the workspace
    root. A cd or pushd directory word with an unquoted *, ?, or [ that matches
    exactly one directory uses that directory. A quoted metacharacter stays
    literal. More than one match fails that directory change and keeps earlier
    copies. While errexit is on, later commands in that state do not run.
    Distinct states
    are capped at 32, and passing that cap raises. A leading command or builtin
    word is skipped even when it is quoted. command followed by an option is
    left in place. A leading NAME=value is skipped. An assignment-only command
    expands a workspace prefix and a stored parameter in the value, then stores
    it when no $ or backtick remains. A later copy word that is $name, ${name},
    $n, or ${n} uses that value, including a one-character name and more text
    in the same word.
    echo, or echo -n, of one literal path is reduced when $(...) or backticks
    are the whole word. The assignment scan raises if its index does not advance.
    """
    paths: list[str] = []
    for state in _shell_states(run_text, cwd):
        paths.extend(state.paths)
    return paths


class _ShellState:
    """Cwd, directory stack, assignments, copy paths, and errexit still in play."""

    def __init__(
        self,
        cwd: str,
        stack: tuple[str, ...],
        env: tuple[tuple[str, str], ...],
        paths: list[str],
        errexit: bool = True,
        oldpwd: str = "",
        positionals: tuple[str, ...] = (),
        expand_aliases: bool = False,
        aliases: tuple[tuple[str, str], ...] = (),
        exit_trap: str | None = None,
        stopped: bool = False,
    ) -> None:
        self.cwd = cwd
        self.stack = stack
        self.env = env
        self.paths = paths
        self.errexit = errexit
        self.oldpwd = oldpwd
        self.positionals = positionals
        self.expand_aliases = expand_aliases
        self.aliases = aliases
        self.exit_trap = exit_trap
        self.stopped = stopped


def _clone_state(
    state: _ShellState,
    *,
    cwd: str | None = None,
    stack: tuple[str, ...] | None = None,
    env: tuple[tuple[str, str], ...] | None = None,
    paths: list[str] | None = None,
    errexit: bool | None = None,
    oldpwd: str | None = None,
    positionals: tuple[str, ...] | None = None,
    expand_aliases: bool | None = None,
    aliases: tuple[tuple[str, str], ...] | None = None,
    exit_trap: str | None = None,
    exit_trap_set: bool = False,
    stopped: bool | None = None,
) -> _ShellState:
    return _ShellState(
        state.cwd if cwd is None else cwd,
        state.stack if stack is None else stack,
        state.env if env is None else env,
        list(state.paths) if paths is None else list(paths),
        state.errexit if errexit is None else errexit,
        state.oldpwd if oldpwd is None else oldpwd,
        state.positionals if positionals is None else positionals,
        state.expand_aliases if expand_aliases is None else expand_aliases,
        state.aliases if aliases is None else aliases,
        state.exit_trap if not exit_trap_set else exit_trap,
        state.stopped if stopped is None else stopped,
    )


def _opening_depth(
    words: list[str],
    body: list[str],
    depth: int,
    quoted: list[bool] | None = None,
) -> int:
    """Count unquoted if and fi in the prefix _command_body removed.

    A quoted if or fi is not counted. A body that is not that prefix's suffix
    is treated as conditional.
    """
    if quoted is None:
        quoted = [False] * len(words)
    if words[len(words) - len(body) :] == body:
        cut = len(words) - len(body)
        prefix = words[:cut]
        prefix_quoted = quoted[:cut]
    else:
        prefix = words
        prefix_quoted = quoted
        depth += 1
    for word, is_quoted in zip(prefix, prefix_quoted):
        if is_quoted:
            continue
        if word == "if":
            depth += 1
        elif word == "fi" and depth > 0:
            depth -= 1
    return depth


def _stored_env(
    env: tuple[tuple[str, str], ...],
    assignments: list[tuple[str, str]],
    positionals: tuple[str, ...] = (),
) -> tuple[tuple[str, str], ...]:
    updated = dict(env)
    for name, value in assignments:
        expanded = _expand_assignment_value(value, updated, positionals)
        if _recordable_assignment(expanded):
            updated[name] = expanded
        else:
            updated.pop(name, None)
    return tuple(sorted(updated.items()))


def _record_copy(state: _ShellState, rest: list[str]) -> None:
    command_name = _command_basename(rest[0])
    if command_name not in _COPY_COMMANDS:
        return
    env = dict(state.env)
    for word in rest[1:]:
        expanded = _expand_assigned_word(word, env, state.positionals)
        path_word = _path_from_copy_word(command_name, expanded)
        if path_word is None:
            continue
        state.paths.append(_resolve_run_path(path_word, state.cwd))


def _dedupe_states(states: list[_ShellState]) -> list[_ShellState]:
    seen: set[tuple[object, ...]] = set()
    kept: list[_ShellState] = []
    for state in states:
        key = (
            state.cwd,
            state.stack,
            state.env,
            tuple(state.paths),
            state.errexit,
            state.oldpwd,
            state.positionals,
            state.expand_aliases,
            state.aliases,
            state.exit_trap,
            state.stopped,
        )
        if key in seen:
            continue
        seen.add(key)
        kept.append(state)
    if len(kept) > _SHELL_STATE_CAP:
        raise AssertionError(f"shell state cap exceeded: {len(kept)}")
    return kept


def _positional_words(words: list[str]) -> list[str] | None:
    """Words after ``set --``. None when this command does not assign them."""
    index = 0
    limit = len(words)
    previous = -1
    while index < limit:
        if index <= previous:
            raise AssertionError("errexit scan did not advance")
        previous = index
        if _ASSIGNMENT_WORD.match(words[index]):
            index += 1
            continue
        if (
            words[index] in ("builtin", "command", "eval")
            and index + 1 < limit
            and not words[index + 1].startswith("-")
        ):
            index += 1
            continue
        break
    if index >= limit or words[index] != "set":
        return None
    rest = words[index + 1 :]
    if "--" not in rest:
        return None
    return rest[rest.index("--") + 1 :]


def _apply_errexit(
    states: list[_ShellState],
    words: list[str],
    quoted: list[bool],
    uncertain: bool,
) -> list[_ShellState]:
    """Apply set to each state. An uncertain set keeps the earlier flag too.

    ``set --`` stores the words after the option end as the positional
    parameters. An uncertain set keeps the earlier parameters too.
    """
    produced: list[_ShellState] = []
    positional = _positional_words(words)
    for state in states:
        new_flag = _next_errexit(words, quoted, state.errexit)
        new_pos = state.positionals
        if positional is not None:
            env = dict(state.env)
            new_pos = tuple(
                _expand_parameters(word, env, state.positionals) for word in positional
            )
        changed = new_flag != state.errexit or new_pos != state.positionals
        if uncertain and changed:
            produced.append(_clone_state(state))
            produced.append(
                _clone_state(state, errexit=new_flag, positionals=new_pos)
            )
        else:
            state.errexit = new_flag
            state.positionals = new_pos
            produced.append(state)
    return _dedupe_states(produced)


def _apply_assignment(
    states: list[_ShellState],
    words: list[str],
    quoted: list[bool],
    assignments: list[tuple[str, str]],
    carry: bool,
    uncertain: bool,
) -> list[_ShellState]:
    produced: list[_ShellState] = []
    for state in states:
        new_env = (
            _stored_env(state.env, assignments, state.positionals)
            if carry
            else state.env
        )
        new_flag = _next_errexit(words, quoted, state.errexit)
        new_old = state.oldpwd
        if carry and any(name == "OLDPWD" for name, _ in assignments):
            raw = dict(new_env).get("OLDPWD", "")
            new_old = _resolve_run_path(raw, state.cwd) if raw else ""
        changed = (
            new_env != state.env
            or new_flag != state.errexit
            or new_old != state.oldpwd
        )
        if uncertain and changed:
            produced.append(_clone_state(state))
        produced.append(
            _clone_state(state, env=new_env, errexit=new_flag, oldpwd=new_old)
        )
    return _dedupe_states(produced)


_FUNC_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_SHELL_CALL_DEPTH = 8


def _word_edge(text: str, index: int) -> bool:
    return index >= len(text) or not (text[index].isalnum() or text[index] == "_")


def _skip_horizontal_space(text: str, index: int) -> int:
    previous = index - 1
    while index < len(text) and text[index] in " \t":
        if index <= previous:
            raise AssertionError("space scan did not advance")
        previous = index
        index += 1
    return index


def _matching_brace(text: str, open_at: int) -> int | None:
    depth = 0
    quote: str | None = None
    index = open_at
    limit = len(text)
    previous = open_at - 1
    while index < limit:
        if index <= previous:
            raise AssertionError("brace scan did not advance")
        previous = index
        char = text[index]
        if quote is not None:
            if quote == '"' and char == "\\" and index + 1 < limit:
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in "'\"":
            quote = char
            index += 1
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return None


def _match_function(text: str, index: int) -> tuple[str, str, int] | None:
    if index > 0 and (text[index - 1].isalnum() or text[index - 1] == "_"):
        return None
    cursor = index
    if text.startswith("function", cursor) and _word_edge(text, cursor + 8):
        cursor = _skip_horizontal_space(text, cursor + 8)
        matched = _FUNC_NAME.match(text, cursor)
        if matched is None:
            return None
        name = matched.group(0)
        cursor = _skip_horizontal_space(text, matched.end())
        if text.startswith("()", cursor):
            cursor = _skip_horizontal_space(text, cursor + 2)
    else:
        matched = _FUNC_NAME.match(text, cursor)
        if matched is None:
            return None
        name = matched.group(0)
        cursor = _skip_horizontal_space(text, matched.end())
        if not text.startswith("()", cursor):
            return None
        cursor = _skip_horizontal_space(text, cursor + 2)
    if cursor >= len(text) or text[cursor] != "{":
        return None
    end = _matching_brace(text, cursor)
    if end is None:
        return None
    return name, text[cursor + 1 : end], end + 1


def _extract_functions(text: str) -> tuple[str, dict[str, str]]:
    """Lift ``name() { body }`` and ``function name { body }`` out of a step."""
    functions: dict[str, str] = {}
    out: list[str] = []
    index = 0
    limit = len(text)
    quote: str | None = None
    subst = 0
    previous = -1
    steps = 0
    while index < limit:
        steps += 1
        if steps > limit + 1 or index <= previous:
            raise AssertionError("function scan did not advance")
        previous = index
        char = text[index]
        if quote is not None:
            out.append(char)
            if quote == '"' and char == "\\" and index + 1 < limit:
                out.append(text[index + 1])
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in "'\"":
            quote = char
            out.append(char)
            index += 1
            continue
        if char == "$" and index + 1 < limit and text[index + 1] == "(":
            subst += 1
        elif char == ")" and subst:
            subst -= 1
        if subst == 0:
            matched = _match_function(text, index)
            if matched is not None:
                name, body, end = matched
                functions[name] = body
                out.append(" ")
                index = end
                continue
        out.append(char)
        index += 1
    return "".join(out), functions


def _read_workspace_file(logical: str) -> str | None:
    if not logical or logical.startswith("/") or ".." in logical.split("/"):
        return None
    root = ROOT.resolve()
    candidate = (ROOT / logical).resolve()
    try:
        candidate.relative_to(root)
    except ValueError:
        return None
    if not candidate.is_file() or candidate.stat().st_size > 1_000_000:
        return None
    try:
        return candidate.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return None


def _pad_to_length(items: list, size: int, fill: object, label: str) -> None:
    """Append until items reaches size. Raise if that pad does not advance."""
    steps = 0
    bound = size + 1
    while len(items) < size:
        steps += 1
        if steps > bound:
            raise AssertionError(f"{label} did not advance")
        before = len(items)
        items.append(fill)
        if len(items) <= before:
            raise AssertionError(f"{label} did not advance")


def _strip_trailing_redirects(
    words: list[str],
    quoted: list[bool],
    globs: list[str | None],
) -> tuple[list[str], list[bool], list[str | None]]:
    words = list(words)
    quoted = list(quoted)
    globs = list(globs)
    _pad_to_length(quoted, len(words), False, "quoted pad")
    _pad_to_length(globs, len(words), None, "glob pad")

    def _operator(word: str) -> bool:
        return bool(
            word in _REDIRECT_WORDS
            or _REDIRECT_OP.fullmatch(word)
            or _REDIRECT_COMBINED.fullmatch(word)
        )

    previous_len = len(words) + 1
    while words:
        if len(words) >= previous_len:
            raise AssertionError("redirect strip did not advance")
        previous_len = len(words)
        if quoted[-1]:
            break
        if _operator(words[-1]):
            words.pop()
            quoted.pop()
            globs.pop()
            continue
        if (
            len(words) >= 2
            and not quoted[-2]
            and _operator(words[-2])
            and not _REDIRECT_COMBINED.fullmatch(words[-2])
        ):
            del words[-2:]
            del quoted[-2:]
            del globs[-2:]
            continue
        break
    return words, quoted, globs


def _alias_binding(words: list[str]) -> tuple[str, str] | None:
    if len(words) < 2 or "=" not in words[1]:
        return None
    name, value = words[1].split("=", 1)
    if not name.isidentifier():
        return None
    return name, value


def _expand_one_alias(command: str, state: _ShellState) -> str:
    if not state.expand_aliases or not state.aliases:
        return command
    words = _shell_words(command)
    if not words or words[0][1]:
        return command
    value = dict(state.aliases).get(words[0][0])
    if value is None:
        return command
    tail = " ".join(text for text, _quoted, _glob in words[1:])
    return value if not tail else f"{value} {tail}"


def _read_shopt(args: list[str], errexit: bool, expand: bool) -> tuple[bool, bool]:
    """Apply ``shopt`` ``-o errexit`` and ``expand_aliases``."""
    mode_set = False
    mode_unset = False
    option_mode = False
    names: list[str] = []
    index = 0
    previous = -1
    while index < len(args):
        if index <= previous:
            raise AssertionError("shopt scan did not advance")
        previous = index
        arg = args[index]
        if arg == "--":
            names.extend(args[index + 1 :])
            break
        if arg.startswith("-") and arg not in ("-",) and not arg.startswith("--"):
            flags = arg[1:]
            if "s" in flags:
                mode_set = True
            if "u" in flags:
                mode_unset = True
            if "o" in flags:
                option_mode = True
            index += 1
            continue
        names.append(arg)
        index += 1
    if mode_set and mode_unset:
        return errexit, expand
    if option_mode and "errexit" in names:
        if mode_unset:
            errexit = False
        elif mode_set:
            errexit = True
    if "expand_aliases" in names:
        if mode_set:
            expand = True
        elif mode_unset:
            expand = False
    return errexit, expand


def _keep_and_run(
    states: list[_ShellState],
    uncertain: bool,
    follows: bool,
    runner,
) -> list[_ShellState]:
    produced: list[_ShellState] = []
    for state in states:
        if uncertain or follows:
            produced.append(_clone_state(state))
        produced.extend(runner(_clone_state(state)))
    return _dedupe_states(produced)


def _shell_states(run_text: str, cwd: str, *, errexit: bool = True) -> list[_ShellState]:
    """Shell states after one step.

    A conditional cd or assignment keeps the earlier state. A command after
    && or || may not run, so the earlier state is kept beside the new one. A
    cd or pushd before ||, before && when another command follows, or while
    errexit is off, keeps the earlier directory beside the new one. A failed
    directory change keeps the state. While errexit is on and the command is
    unconditional, later commands in that state do not run. A set inside an
    if compound or after && or || keeps the earlier errexit beside the new
    one. Distinct states are capped at 32.
    """
    text, functions = _extract_functions(run_text)
    initial = _ShellState(_posix_norm(cwd), (), (), [], errexit)
    return _interpret([initial], text, functions, 0)


def _interpret(
    states: list[_ShellState],
    text: str,
    functions: dict[str, str],
    depth: int,
) -> list[_ShellState]:
    if depth > _SHELL_CALL_DEPTH:
        return states
    text, found = _extract_functions(text)
    if found:
        functions.update(found)
    compound = 0
    for command, carry, may_fail, follows in _split_shell_commands(text):
        parked = [state for state in states if state.stopped]
        live = [state for state in states if not state.stopped]
        if not live:
            states = parked
            break
        live, compound = _apply_command(
            live, command, carry, may_fail, follows, functions, depth, compound
        )
        states = _dedupe_states(parked + live)
    if depth == 0:
        states = _run_exit_traps(states, functions)
    return states


def _run_exit_traps(
    states: list[_ShellState], functions: dict[str, str]
) -> list[_ShellState]:
    produced: list[_ShellState] = []
    for state in states:
        if not state.exit_trap:
            produced.append(state)
            continue
        cleared = _clone_state(state, exit_trap=None, exit_trap_set=True)
        produced.extend(_interpret([cleared], state.exit_trap, functions, 1))
    return _dedupe_states(produced)


def _apply_command(
    states: list[_ShellState],
    command: str,
    carry: bool,
    may_fail: bool,
    follows: bool,
    functions: dict[str, str],
    depth: int,
    compound: int,
) -> tuple[list[_ShellState], int]:
    if depth <= _SHELL_CALL_DEPTH and any(
        state.expand_aliases and state.aliases for state in states
    ):
        groups: dict[str, list[_ShellState]] = {}
        order: list[str] = []
        for state in states:
            expanded = _expand_one_alias(command, state)
            if expanded not in groups:
                groups[expanded] = []
                order.append(expanded)
            groups[expanded].append(state)
        if order != [command]:
            produced: list[_ShellState] = []
            for expanded in order:
                group = groups[expanded]
                if expanded == command:
                    applied, compound = _apply_command(
                        group,
                        command,
                        carry,
                        may_fail,
                        follows,
                        functions,
                        depth,
                        compound,
                    )
                    produced.extend(applied)
                    continue
                pieces = _split_shell_commands(expanded)
                if len(pieces) == 1:
                    applied, compound = _apply_command(
                        group,
                        pieces[0][0],
                        carry,
                        may_fail,
                        follows,
                        functions,
                        depth + 1,
                        compound,
                    )
                    produced.extend(applied)
                    continue
                produced.extend(
                    _keep_and_run(
                        group,
                        False,
                        follows,
                        lambda state, body=expanded: _interpret(
                            [state], body, functions, depth + 1
                        ),
                    )
                )
            return _dedupe_states(produced), compound
    triples = _shell_words(command)
    raw = [text for text, _quoted, _glob in triples]
    raw_quoted = [is_quoted for _text, is_quoted, _glob in triples]
    raw_globs = [glob for _text, _quoted, glob in triples]
    body = _command_body(raw, raw_quoted)
    compound = _opening_depth(raw, body, compound, raw_quoted)
    uncertain = compound > 0 or follows
    if not body:
        return states, compound
    body, body_quoted, body_globs = _strip_trailing_redirects(
        body,
        raw_quoted[len(raw) - len(body) :],
        raw_globs[len(raw) - len(body) :],
    )
    if not body:
        return states, compound
    assignments: list[tuple[str, str]] = []
    index = 0
    previous = -1
    while index < len(body):
        if index <= previous:
            raise AssertionError("assignment scan did not advance")
        previous = index
        match = _ASSIGNMENT_WORD.match(body[index])
        if match is None:
            break
        assignments.append((match.group(1), match.group(2)))
        index += 1
    rest = body[index:]
    rest_quoted = body_quoted[index:]
    rest_globs = body_globs[index:]
    if rest and rest[0] in functions and not (rest_quoted and rest_quoted[0]):
        arg_words = rest[1:]
        body_text = functions[rest[0]]

        def _call(
            state: _ShellState,
            args: list[str] = arg_words,
            inner: str = body_text,
        ) -> list[_ShellState]:
            env = dict(state.env)
            bound_args = tuple(
                _expand_parameters(word, env, state.positionals) for word in args
            )
            bound = _clone_state(state, positionals=bound_args)
            result = _interpret([bound], inner, functions, depth + 1)
            return [
                _clone_state(item, positionals=state.positionals) for item in result
            ]

        return _keep_and_run(states, uncertain, follows, _call), compound
    if rest and rest[0] == "eval" and len(rest) >= 2:
        arg_words = rest[1:]

        def _eval(
            state: _ShellState, args: list[str] = arg_words
        ) -> list[_ShellState]:
            env = dict(state.env)
            parts = [
                _expand_parameters(word, env, state.positionals) for word in args
            ]
            return _interpret([state], " ".join(parts), functions, depth + 1)

        return _keep_and_run(states, uncertain, follows, _eval), compound
    if rest and rest[0] in (".", "source") and len(rest) >= 2:
        def _source(state: _ShellState, word: str = rest[1]) -> list[_ShellState]:
            expanded = _expand_assigned_word(word, dict(state.env), state.positionals)
            script = _read_workspace_file(_resolve_run_path(expanded, state.cwd))
            if script is None:
                return [state]
            return _interpret([state], script, functions, depth + 1)

        return _keep_and_run(states, uncertain, follows, _source), compound
    if rest and rest[0] == "trap":
        trap_args = list(rest[1:])
        if trap_args and trap_args[0] == "--":
            trap_args = trap_args[1:]
        if (
            len(trap_args) >= 2
            and not trap_args[0].startswith("-")
            and any(signal in ("EXIT", "exit", "0") for signal in trap_args[1:])
        ):
            trap_body = trap_args[0]

            def _trap(
                state: _ShellState, body_text: str = trap_body
            ) -> list[_ShellState]:
                return [_clone_state(state, exit_trap=body_text, exit_trap_set=True)]

            return _keep_and_run(states, uncertain, follows, _trap), compound
    if rest and rest[0] == "alias":
        binding = _alias_binding(rest)

        def _alias(state: _ShellState, found: tuple[str, str] | None = binding) -> list[_ShellState]:
            if found is None:
                return [state]
            updated = dict(state.aliases)
            updated[found[0]] = found[1]
            return [_clone_state(state, aliases=tuple(sorted(updated.items())))]

        return _keep_and_run(states, uncertain, follows, _alias), compound
    if rest and rest[0] == "shopt":
        args = rest[1:]

        def _shopt(state: _ShellState, shopt_args: list[str] = args) -> list[_ShellState]:
            errexit, expand = _read_shopt(shopt_args, state.errexit, state.expand_aliases)
            return [_clone_state(state, errexit=errexit, expand_aliases=expand)]

        return _keep_and_run(states, uncertain, follows, _shopt), compound
    if not rest:
        return (
            _apply_assignment(states, body, body_quoted, assignments, carry, uncertain),
            compound,
        )
    if _directory_update(rest, "", []) is None:
        for state in states:
            _record_copy(state, rest)
        return _apply_errexit(states, body, body_quoted, uncertain), compound
    if not carry:
        return _apply_errexit(states, body, body_quoted, uncertain), compound
    produced: list[_ShellState] = []
    for state in states:
        keep_old = uncertain or may_fail or not state.errexit
        outcomes = _directory_outcomes(rest, rest_quoted, rest_globs, state)
        if outcomes is None:
            cloned = _clone_state(state)
            if not keep_old:
                cloned.stopped = True
            produced.append(cloned)
            continue
        if keep_old:
            produced.append(_clone_state(state))
        for updated_cwd, stack in outcomes:
            new_old = state.cwd if updated_cwd != state.cwd else state.oldpwd
            produced.append(
                _clone_state(
                    state,
                    cwd=updated_cwd,
                    stack=stack,
                    paths=list(state.paths),
                    oldpwd=new_old,
                )
            )
    return (
        _apply_errexit(_dedupe_states(produced), body, body_quoted, uncertain),
        compound,
    )


def _archive_copied_onto_book(run_text: str, cwd: str, *, errexit: bool = True) -> bool:
    """True when one kept shell state copies the archive onto the book dashboard."""
    for state in _shell_states(run_text, cwd, errexit=errexit):
        archive = any(_is_archive_path(item) for item in state.paths)
        book = any(_is_book_dashboard_target(item) for item in state.paths)
        if archive and book:
            return True
    return False


def _is_archive_path(path: str) -> bool:
    if path == ARCHIVE_BENCHMARK_DASHBOARD or path.startswith(
        ARCHIVE_BENCHMARK_DASHBOARD + "/"
    ):
        return True
    return ARCHIVE_BENCHMARK_DASHBOARD.startswith(path + "/")


def _is_book_dashboard_target(path: str) -> bool:
    return (
        path == BOOK_DASHBOARD_DIR
        or path == BOOK_BENCHMARK_DASHBOARD
        or path.startswith(BOOK_BENCHMARK_DASHBOARD + "/")
    )


def _folded_action_inputs(with_block: dict) -> dict[str, object]:
    """Map with-keys onto action input names.

    A lone key matches ignoring case, and spaces in the key become underscores.
    Actions rejects a second key that repeats an earlier key ignoring case, so
    publish_dir together with Publish_Dir never runs. Keeping the later value is
    defensive only. Keys that are not equal ignoring case, such as publish_dir
    and "Publish Dir", both load and share one input name, and the later value
    is the one recorded here.
    """
    folded: dict[str, object] = {}
    for key, value in with_block.items():
        folded[str(key).replace(" ", "_").lower()] = value
    return folded


def _known_false(value: object) -> bool:
    """Unset and false are the only allowed force_orphan and single-commit values."""
    if value is None or value is False:
        return True
    if isinstance(value, str):
        return value.strip().upper() == "FALSE"
    return False


def _known_true(value: object) -> bool:
    """True when an Actions boolean input is the YAML bool or the string TRUE."""
    if value is True:
        return True
    if isinstance(value, str):
        return value.strip().upper() == "TRUE"
    return False


def _is_pages_publisher(step: dict) -> bool:
    uses = str(step.get("uses") or "").lower()
    return any(marker in uses for marker in _PAGES_USE_MARKERS)


def _normalize_cwd(raw: object) -> str:
    if not isinstance(raw, str) or not raw.strip():
        return ""
    return _posix_norm(_strip_workspace(raw.strip()))


def _defaults_cwd(defaults: object) -> str:
    if not isinstance(defaults, dict):
        return ""
    run = defaults.get("run")
    if not isinstance(run, dict):
        return ""
    return _normalize_cwd(run.get("working-directory"))


def _job_cwd(data: dict, job: dict) -> str:
    workflow_cwd = _defaults_cwd(data.get("defaults"))
    job_cwd = _defaults_cwd(job.get("defaults"))
    if job_cwd:
        return job_cwd
    return workflow_cwd


def _step_cwd(step: dict, job_cwd: str) -> str:
    raw = step.get("working-directory")
    if not isinstance(raw, str) or not raw.strip():
        return job_cwd
    return _normalize_cwd(raw)


def _defaults_shell(defaults: object) -> str:
    if not isinstance(defaults, dict):
        return ""
    run = defaults.get("run")
    if not isinstance(run, dict):
        return ""
    shell = run.get("shell")
    if isinstance(shell, str) and shell.strip():
        return shell.strip()
    return ""


def _step_shell(step: dict, job: dict, data: dict) -> str:
    """Shell template from the step, then the job defaults, then the workflow."""
    raw = step.get("shell")
    if isinstance(raw, str) and raw.strip():
        return raw.strip()
    job_shell = _defaults_shell(job.get("defaults"))
    if job_shell:
        return job_shell
    return _defaults_shell(data.get("defaults"))


def _template_errexit(shell: str) -> bool:
    """Errexit at the start of a step.

    An unset shell is the GitHub bash template and starts on. ``bash {0}``
    does not pass ``-e`` and starts off. Words after ``{0}`` are script
    arguments. A template whose program is not bash, sh, or dash starts off.
    """
    if not shell.strip():
        return True
    parts = shell.split()
    if "{0}" in parts:
        # Words after the script placeholder are arguments to the script.
        words = parts[: parts.index("{0}")]
    else:
        words = parts
    if not words:
        return True
    base = words[0].replace("\\", "/").rsplit("/", 1)[-1]
    if base not in ("bash", "sh", "dash"):
        return False
    enabled = False
    index = 1
    previous = 0
    while index < len(words):
        if index <= previous:
            raise AssertionError("shell template scan did not advance")
        previous = index
        arg = words[index]
        if arg == "--":
            break
        if arg in ("+o", "-o"):
            nxt = words[index + 1] if index + 1 < len(words) else ""
            if nxt and nxt[0] not in "+-":
                if nxt == "errexit":
                    enabled = arg == "-o"
                index += 2
                continue
            index += 1
            continue
        if len(arg) >= 2 and arg[0] in "+-" and arg[1:].isalpha() and "e" in arg[1:]:
            enabled = arg[0] == "-"
        index += 1
    return enabled


def _quoted_literal(inner: str) -> str | None:
    text = inner.strip()
    if len(text) < 2:
        return None
    quote = text[0]
    if quote not in "'\"" or text[-1] != quote:
        return None
    body = text[1:-1]
    if "\\" in body or quote in body:
        return None
    return body


def _reduce_literal_expressions(value: str) -> str | None:
    """Concatenate literal text and quoted-literal expressions.

    Returns None when an expression is not a quoted literal, or is unclosed.
    The scan raises if its index does not advance.
    """
    if "${{" not in value:
        return value
    parts: list[str] = []
    index = 0
    previous = -1
    steps = 0
    bound = len(value) + 1
    while True:
        steps += 1
        if steps > bound or index <= previous:
            raise AssertionError("expression scan did not advance")
        previous = index
        start = value.find("${{", index)
        if start < 0:
            parts.append(value[index:])
            return "".join(parts)
        parts.append(value[index:start])
        end = value.find("}}", start + 3)
        if end < 0:
            return None
        literal = _quoted_literal(value[start + 3 : end])
        if literal is None:
            return None
        parts.append(literal)
        next_index = end + 2
        if next_index <= index:
            raise AssertionError("expression scan did not advance")
        index = next_index


def _pages_input_path(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    reduced = _reduce_literal_expressions(value)
    if reduced is None:
        return None
    return _posix_norm(reduced.strip())


def _docs_destination_forbidden(norm: str) -> bool:
    if norm == "dashboard":
        return True
    return norm == _LIVE_DASHBOARD_DEST or norm.startswith(_LIVE_DASHBOARD_DEST + "/")


def _publish_dir_hits_archive(norm: str) -> bool:
    return norm == ARCHIVE_BENCHMARK_DASHBOARD or norm.startswith(
        ARCHIVE_BENCHMARK_DASHBOARD + "/"
    )


def _same_repo_path(value: object, expected: str) -> bool:
    """True when value is the expected workspace path.

    A value whose first non-space character is `/` is a different directory.
    `./` and a trailing slash still match.
    """
    if not isinstance(value, str):
        return False
    text = value.strip()
    if text.startswith("/"):
        return False
    return _posix_norm(text) == _posix_norm(expected)


def _explicit_false(value: object) -> bool:
    """YAML false or the string false. Unset is not false."""
    if value is False:
        return True
    if isinstance(value, str):
        return value.strip().upper() == "FALSE"
    return False


def _jamesives_branch_ok(value: object) -> bool:
    """Unset, blank, or gh-pages. Any other branch name is not."""
    if value is None:
        return True
    if isinstance(value, str):
        return value.strip() in ("", "gh-pages")
    return False


def _repository_name_unset(value: object) -> bool:
    return value is None or value == ""


def _is_jamesives_deploy(step: dict) -> bool:
    uses = str(step.get("uses") or "").lower()
    return _JAMESIVES_DEPLOY in uses


def _jamesives_target_root(value: object) -> bool:
    """True when target-folder deploys onto the worktree root.

    A missing value is the action's empty target. `.`, `/`, and a path that
    normalizes to empty are that same root. A non-string, and an expression
    that is not a quoted literal, are not judged here.
    """
    if value is None:
        return True
    return _pages_input_path(value) == ""


def _jamesives_alert_data(norm: str) -> bool:
    return norm == BENCHMARK_ALERT_DESTINATION_DIR or norm.startswith(
        BENCHMARK_ALERT_DESTINATION_DIR + "/"
    )


def _assert_docs_jamesives_publish(
    inputs: dict[str, object], *, path: str, step_name: str
) -> None:
    """folder, target-folder, single-commit, branch, and force are what this action deploys.

    target-folder must name a directory other than the worktree root, the
    published dashboard, and the alert data directory. single-commit must be
    unset or a known false. branch must be unset, blank, or gh-pages. force
    must be an explicit false. Other inputs on a documentation step are not
    read.
    """
    folder = _pages_input_path(inputs.get("folder"))
    if folder is not None and _publish_dir_hits_archive(folder):
        raise AssertionError(
            f"{path}: {step_name} folder must stay off {ARCHIVE_BENCHMARK_DASHBOARD}"
        )
    raw_target = inputs.get("target-folder")
    if _jamesives_target_root(raw_target):
        raise AssertionError(
            f"{path}: {step_name} target-folder must not be the worktree root"
        )
    target = _pages_input_path(raw_target)
    if target is not None and _docs_destination_forbidden(target):
        raise AssertionError(
            f"{path}: {step_name} target-folder must not replace the published dashboard"
        )
    if target is not None and _jamesives_alert_data(target):
        raise AssertionError(
            f"{path}: {step_name} target-folder must not replace "
            f"{BENCHMARK_ALERT_DESTINATION_DIR}"
        )
    if not _jamesives_branch_ok(inputs.get("branch")):
        raise AssertionError(
            f"{path}: {step_name} branch must stay unset or "
            f"{BENCHMARK_ALERT_PUBLISH_BRANCH!r}, got {inputs.get('branch')!r}"
        )
    if not _known_false(inputs.get("single-commit")):
        raise AssertionError(
            f"{path}: {step_name} single-commit must stay unset or false"
        )
    if not _explicit_false(inputs.get("force")):
        raise AssertionError(f"{path}: {step_name} force must be false")


def _assert_docs_pages_publish(step: dict, *, path: str) -> None:
    step_name = str(step.get("name") or "<unnamed>")
    with_block = step.get("with")
    if not isinstance(with_block, dict):
        raise AssertionError(f"{path}: {step_name} missing with:")
    inputs = _folded_action_inputs(with_block)
    if _is_jamesives_deploy(step):
        _assert_docs_jamesives_publish(inputs, path=path, step_name=step_name)
        return
    if not _known_false(inputs.get("force_orphan")):
        raise AssertionError(
            f"{path}: {step_name} force_orphan must stay unset or false"
        )
    destination = inputs.get("destination_dir")
    if destination is not None:
        reduced = _pages_input_path(destination)
        if reduced is not None and _docs_destination_forbidden(reduced):
            raise AssertionError(
                f"{path}: {step_name} destination_dir must not replace the published dashboard"
            )
    publish_dir = _pages_input_path(inputs.get("publish_dir"))
    if publish_dir is not None and _publish_dir_hits_archive(publish_dir):
        raise AssertionError(
            f"{path}: {step_name} publish_dir must stay off {ARCHIVE_BENCHMARK_DASHBOARD}"
        )
    if not _known_true(inputs.get("keep_files")):
        raise AssertionError(f"{path}: {step_name} keep_files must stay true")


def _assert_jamesives_alert_publish(
    inputs: dict[str, object], *, path: str, step_name: str
) -> None:
    """Keys github-pages-deploy-action reads for the alert publish.

    folder and target-folder must be the benchmark_data paths. A leading `/`
    does not match. single-commit must be unset or a known false. branch must
    be unset, blank, or gh-pages. repository-name must be unset or the empty
    string. A value that is only space does not pass. force must be
    an explicit false; unset is not false.
    """
    folder = inputs.get("folder")
    target = inputs.get("target-folder")
    if not _same_repo_path(folder, _JAMESIVES_FOLDER):
        raise AssertionError(
            f"{path}: {step_name} folder must be {_JAMESIVES_FOLDER}, got {folder!r}"
        )
    if not _same_repo_path(target, _JAMESIVES_TARGET):
        raise AssertionError(
            f"{path}: {step_name} target-folder must be {_JAMESIVES_TARGET}, "
            f"got {target!r}"
        )
    if not _known_false(inputs.get("single-commit")):
        raise AssertionError(
            f"{path}: {step_name} single-commit must stay unset or false"
        )
    if not _jamesives_branch_ok(inputs.get("branch")):
        raise AssertionError(
            f"{path}: {step_name} branch must stay unset or {BENCHMARK_ALERT_PUBLISH_BRANCH!r}, "
            f"got {inputs.get('branch')!r}"
        )
    if not _repository_name_unset(inputs.get("repository-name")):
        raise AssertionError(f"{path}: {step_name} repository-name must stay unset")
    if not _explicit_false(inputs.get("force")):
        raise AssertionError(f"{path}: {step_name} force must be false")


def assert_benchmark_alert_gh_pages_publish(
    text: str, *, path: str = "benchmark_alert.yml"
) -> None:
    """A step whose uses contains gh-pages or github-pages is checked.

    When that uses also contains github-pages-deploy-action, the step is held to folder,
    target-folder, single-commit, branch, repository-name, and force. folder
    and target-folder must be the benchmark_data paths, and a leading slash
    does not match. single-commit must be unset or a known false. branch must
    be unset, blank, or gh-pages. repository-name must be unset or the empty
    string. A value that is only space does not pass. force must be
    an explicit false. The peaceiris input names on that step are not read.

    Otherwise the step is read as peaceiris. publish_dir must be
    ./docs/benchmark_data, destination_dir must be dashboard/benchmark_data,
    and publish_branch must be gh-pages. external_repository must be unset or
    the empty string. force_orphan must be unset or a known false. Known false
    is YAML false, or a string that is false after surrounding space is removed
    and the letters are uppercased.
    """
    jobs = _workflow_jobs(text, path=path)
    gh_pages_steps: list[dict] = []
    for job_name, job in jobs.items():
        if not isinstance(job, dict):
            raise AssertionError(f"{path}: job {job_name!r} is not a mapping")
        steps = job.get("steps")
        if not isinstance(steps, list):
            raise AssertionError(f"{path}: job {job_name!r} missing steps")
        for step in steps:
            if isinstance(step, dict) and _is_pages_publisher(step):
                gh_pages_steps.append(step)
    if not gh_pages_steps:
        raise AssertionError(
            f"{path}: missing {PEACEIRIS_GH_PAGES} publish step "
            f"({BENCHMARK_ALERT_GH_PAGES_STEP!r})"
        )
    if not any(
        BENCHMARK_ALERT_GH_PAGES_STEP in str(step.get("name") or "")
        for step in gh_pages_steps
    ):
        raise AssertionError(
            f"{path}: missing step named {BENCHMARK_ALERT_GH_PAGES_STEP!r}"
        )
    for step in gh_pages_steps:
        step_name = str(step.get("name") or "<unnamed>")
        with_block = step.get("with")
        if not isinstance(with_block, dict):
            raise AssertionError(f"{path}: {step_name} missing with:")
        inputs = _folded_action_inputs(with_block)
        if _is_jamesives_deploy(step):
            _assert_jamesives_alert_publish(inputs, path=path, step_name=step_name)
            continue
        publish_dir = inputs.get("publish_dir")
        destination_dir = inputs.get("destination_dir")
        if publish_dir in ("./docs", "./docs/"):
            raise AssertionError(
                f"{path}: publish_dir must not be ./docs (would publish the whole tree)"
            )
        if destination_dir in ("dashboard",):
            raise AssertionError(
                f"{path}: destination_dir must not be bare dashboard "
                "(overwrites doc subtree)"
            )
        if publish_dir != BENCHMARK_ALERT_PUBLISH_DIR:
            raise AssertionError(
                f"{path}: expected publish_dir {BENCHMARK_ALERT_PUBLISH_DIR!r}, "
                f"got {publish_dir!r}"
            )
        if destination_dir != BENCHMARK_ALERT_DESTINATION_DIR:
            raise AssertionError(
                f"{path}: expected destination_dir {BENCHMARK_ALERT_DESTINATION_DIR!r}, "
                f"got {destination_dir!r}"
            )
        if inputs.get("publish_branch") != BENCHMARK_ALERT_PUBLISH_BRANCH:
            raise AssertionError(
                f"{path}: publish_branch must be {BENCHMARK_ALERT_PUBLISH_BRANCH!r}, "
                f"got {inputs.get('publish_branch')!r}"
            )
        external = inputs.get("external_repository")
        if external not in (None, ""):
            raise AssertionError(f"{path}: external_repository must stay unset")
        if not _known_false(inputs.get("force_orphan")):
            raise AssertionError(f"{path}: force-push input must stay unset or false")


def assert_docs_no_archive_benchmark_dashboard_copy(
    text: str, *, path: str = "docs.yml"
) -> None:
    """Documentation must not copy the archived dashboard onto the book dashboard directory.

    A step whose uses contains gh-pages or github-pages is checked. When that
    uses also contains github-pages-deploy-action, the step is held to folder,
    target-folder, single-commit, branch, and force. folder and target-folder use the same
    normalization as publish_dir, including a dropped leading slash. folder
    must stay off docs/archive/benchmark_dashboard and files under it.
    Ancestors are not that check. A missing folder does not raise.
    target-folder must be present. A missing value, a blank value, and a path
    that normalizes to empty are the worktree root, and that root raises.
    ".", "/", and dashboard/benchmark_dashboard/../.. are that root.
    target-folder must not be dashboard or dashboard/benchmark_dashboard, or a
    path under that directory, and must not be dashboard/benchmark_data or a
    path under it. single-commit must be unset or a known false. branch must
    be unset, blank, or gh-pages. force must be YAML false or the false
    string. Unset force is not false. Other inputs on that step are not read.

    Otherwise the step is read as peaceiris. force_orphan must stay unset or a
    known false. Known false is YAML false, or a string that is false after
    surrounding space is removed and the letters are uppercased. publish_dir
    must stay off the archive directory and files under it. Ancestors such as
    docs and docs/archive are not that check. destination_dir must not be
    dashboard or dashboard/benchmark_dashboard, or a path under that directory.
    A quoted literal expression is reduced before those path checks. Another
    destination stays allowed while keep_files is true. keep_files must stay
    true.

    working-directory is read from the step, then the job defaults, then the
    workflow defaults. A github.workspace expression and the GITHUB_WORKSPACE
    prefixes are stripped before that directory is used. cd and pushd use the
    second word. An option in the pushd word is not a directory. popd pops the
    directory stack and does not read a directory word. A directory change
    before a newline or ; replaces the directory for later commands. A
    directory change before ||, or before && when another command follows,
    keeps the earlier directory beside the new one, including when the next
    command is flush against && or ||. A command after && or ||, including an
    assignment-only command, may not run, so the earlier state is kept beside
    the new one. A directory change before a single |
    or a single & does not apply. set +e, set "+e", set +o errexit, and
    set -o errexit change errexit, including when the option word is quoted.
    An assignment, builtin, command, or eval prefix may stand in front of set.
    A set inside an if compound or after && or || keeps the earlier errexit
    beside the new one. set +e keeps the earlier directory beside a
    later cd or pushd until set -e. The shell template is read from the step,
    then the job defaults.run, then the workflow defaults.run. An unset shell
    starts with errexit on. A bash, sh, or dash template starts with errexit
    on only when it passes -e or -o errexit. bash {0} starts with errexit off.
    Words after {0} are script arguments and do not change errexit. set --
    ends option scanning, so a later -e or +e is not an option. The words
    after -- are the positional parameters. shopt -u -o
    errexit and shopt -uo errexit turn errexit off, and shopt -s -o errexit
    turns it on. shopt -s expand_aliases makes a later alias run. An eval
    argument is shell text, quoted or unquoted, including a stored word and a
    function definition in that text. A redirection operator that contains & or |
    is not a command separator. A function body runs when that function is
    called, and the call binds the function's arguments. cd - uses the
    previous directory. Distinct previous directories stay in their states.
    An OLDPWD assignment sets that directory. CDPATH is searched for a cd
    operand with no slash. A workspace prefix in that value is resolved from
    the workspace root. An EXIT trap runs at the end of the step. A leading
    -- is an option end. The trap is recorded when EXIT, exit, or 0 is one of
    the signal words. A sourced
    file under the workspace is read, including a function defined in that
    file. A stored variable in a directory word is
    expanded, including when the word is quoted, when the value holds a
    parameter, and when more text follows the parameter in the word. An
    unquoted glob in that value is matched. A $GITHUB_WORKSPACE, ${GITHUB_WORKSPACE}, or
    GITHUB_WORKSPACE prefix is resolved from the workspace root.
    The errexit scan, the paren scan, the backtick scan, the shell word scan,
    and the shell template scan raise if the index does not advance. The shell scan raises if its index does not
    advance. A backslash immediately before a newline continues the command.
    A quoted if, elif, then, else, or fi is not a reserved word. A relative
    word is joined onto the current directory. A word that begins with / is
    resolved from the workspace root. A cd or pushd directory word with an
    unquoted *, ?, or [ that matches exactly one directory uses that directory.
    A quoted metacharacter stays literal. More than one match fails that
    directory change and keeps earlier copies. While errexit is on, later
    commands in that state do not run. Distinct shell states are capped at 32, and passing that
    cap raises. 32 states do not raise. 33 states raise. A quoted literal expression scan raises if its index does not
    advance. The
    copy command is cp, mv, or rsync, matched by basename. A backslash in that
    word is a separator. A leading command or builtin word is skipped before
    cd, pushd, popd, and a copy, including when that word is quoted. A leading
    { group token is skipped the same way. A leading redirection is skipped
    the same way, including a file-descriptor redirection and more than one
    redirection. A
    leading if, elif, then, else, or fi is skipped when the word is unquoted,
    so a copy or directory change in that branch is read, including when it
    shares the command with then or else. A cd, pushd, popd, or
    assignment-only command inside an if compound is kept together with the
    state from before that command. A later copy is checked against each of
    those states. An unconditional cd, pushd, popd, or assignment still
    replaces the state. That scan raises if its index does not advance.
    command
    followed by an option is left in place. A leading NAME=value is skipped.
    An assignment-only command expands a workspace prefix and a stored
    parameter in the value, then stores it when no $ or backtick remains.
    A later copy word that is $name, ${name}, $n, or ${n} uses it, including
    a one-character name and more text in the same word. cp and mv contribute a
    --target-directory= or --target= value. echo, or echo -n, of one literal
    path is reduced when $(...) or backticks are the whole word. A trailing /*
    is removed. The check reads one step at a time.

    Untested classes: a copy split across multiple steps; a copy performed by a
    uses: action; a command word other than cp, mv, or rsync, with reserved
    words and subshells listed separately; the word command followed by an
    option; brace expansion; variable command words; escape splitting; a |, &,
    or ; inside a command substitution; actions/deploy-pages;
    actions/upload-pages-artifact; a destination_dir, publish_dir, folder, or
    target-folder expression that is not a quoted literal; a command
    substitution other than echo or echo -n of one literal path; a cd whose
    directory is not the second word, including cd -- and cd -P; a pushd
    directory that is not the second word; a glued -t destination; a copy word
    containing *, ?, or [ other than a trailing /*; a working-directory
    expression other than github.workspace; any input other than folder,
    target-folder, single-commit, branch, and force on a documentation step
    whose uses contains github-pages-deploy-action; a folder or target-folder
    that begins with a backslash; a ( ... ) subshell; for, while, until, case
    compound commands and a leading !; a copy run through sudo, exec, time,
    env, xargs, nohup, or another wrapper command; an assignment prefix in
    front of command cd; a parameter expansion other than $name, ${name}, $n,
    and ${n}; an export or declare assignment; a cd of $RUNNER_TEMP; a typeset or readonly
    assignment.
    """
    data = _workflow_document(text, path=path)
    jobs = _require_jobs(data, path=path)
    for job_name, job in jobs.items():
        if not isinstance(job, dict):
            raise AssertionError(f"{path}: job {job_name!r} is not a mapping")
        steps = job.get("steps")
        if not isinstance(steps, list):
            raise AssertionError(f"{path}: job {job_name!r} missing steps")
        job_cwd = _job_cwd(data, job)
        for step in steps:
            if not isinstance(step, dict):
                continue
            if _is_pages_publisher(step):
                _assert_docs_pages_publish(step, path=path)
            run_text = _step_run_text(step)
            if not run_text.strip():
                continue
            if _archive_copied_onto_book(
                run_text,
                _step_cwd(step, job_cwd),
                errexit=_template_errexit(_step_shell(step, job, data)),
            ):
                step_name = step.get("name", "<unnamed>")
                raise AssertionError(
                    f"{path}: step {step_name!r} copies {ARCHIVE_BENCHMARK_DASHBOARD} "
                    f"onto {BOOK_DASHBOARD_DIR}"
                )

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


def _call_checker(fn, *args, **kwargs):
    """Turn a non-assertion exception from the checker into AssertionError."""
    try:
        return fn(*args, **kwargs)
    except AssertionError:
        raise
    except Exception as exc:
        raise AssertionError(
            f"{getattr(fn, '__name__', fn)} raised {type(exc).__name__}: {exc}"
        ) from exc


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

    def test_pages_publish_path_helpers(self) -> None:
        try:
            self._check_pages_publish_path_helpers()
        except AssertionError:
            raise
        except Exception as exc:
            raise AssertionError(
                f"path helper raised {type(exc).__name__}: {exc}"
            ) from exc

    def _check_pages_publish_path_helpers(self) -> None:
        self.assertEqual(
            _posix_norm("docs//archive/./benchmark_dashboard"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _posix_norm("docs/archive/../archive/benchmark_dashboard"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _posix_norm("docs\\archive\\benchmark_dashboard"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(_posix_norm("./docs/archive/benchmark_dashboard"), "docs/archive/benchmark_dashboard")
        self.assertEqual(_posix_norm("docs/../archive"), "archive")
        self.assertEqual(_posix_norm(".."), "")
        self.assertEqual(_posix_norm(""), "")
        self.assertEqual(
            _resolve_run_path("  docs/archive/benchmark_dashboard  ", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("Zdocs/archive/benchmark_dashboard", ""),
            "Zdocs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("../benchmark_dashboard", "docs/archive/extra"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("docs/archive/benchmark_dashboard", "docs/"),
            "docs/docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("archive/benchmark_dashboard", "docs"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("docsEXTRA/file", "docs"),
            "docs/docsEXTRA/file",
        )
        self.assertEqual(
            _resolve_run_path("./docs/book/dashboard/*", ""),
            "docs/book/dashboard",
        )
        self.assertEqual(
            _resolve_run_path("$GITHUB_WORKSPACE/docs/archive/benchmark_dashboard", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("${GITHUB_WORKSPACE}/docs/book/dashboard/", ""),
            "docs/book/dashboard",
        )
        self.assertEqual(
            _resolve_run_path("GITHUB_WORKSPACE/docs/archive/benchmark_dashboard", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(_step_cwd({"working-directory": " docs/ "}, ""), "docs")
        self.assertEqual(_step_cwd({"working-directory": "   "}, "docs"), "docs")
        self.assertEqual(_step_cwd({"working-directory": 1}, "docs"), "docs")
        self.assertEqual(
            _defaults_cwd({"run": {"working-directory": " docs "}}),
            "docs",
        )
        self.assertEqual(_defaults_cwd({"run": "bash"}), "")
        self.assertEqual(_defaults_cwd({"shell": "bash"}), "")
        self.assertEqual(
            _resolve_run_path("..\\benchmark_dashboard", "docs/archive/extra"),
            "docs/archive/benchmark_dashboard",
        )
        quoted = (
            'cp -r docs/archive/benchmark_dashboard /tmp/out '
            '"&& cp -r docs/archive/benchmark_dashboard docs/book/dashboard/"'
        )
        quoted_paths = _run_paths(quoted, "")
        self.assertIn("docs/archive/benchmark_dashboard", quoted_paths)
        self.assertNotIn("docs/book/dashboard", quoted_paths)
        single_quoted = (
            "cp -r docs/archive/benchmark_dashboard /tmp/out "
            "'&& cp -r docs/archive/benchmark_dashboard docs/book/dashboard/'"
        )
        single_paths = _run_paths(single_quoted, "")
        self.assertNotIn("docs/book/dashboard", single_paths)
        ampersand_paths = _run_paths(
            "cd docs&extra\ncp -r archive/benchmark_dashboard book/dashboard/",
            "",
        )
        self.assertNotIn("docs/archive/benchmark_dashboard", ampersand_paths)
        self.assertNotIn("docs/book/dashboard", ampersand_paths)
        self.assertEqual(_run_paths("echo ok\n& echo still\n", ""), [])
        bare_cd = _run_paths(
            "cd\ncp -r docs/archive/benchmark_dashboard docs/book/dashboard/",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", bare_cd)
        self.assertIn("docs/book/dashboard", bare_cd)
        self.assertEqual(
            _run_paths(
                "echo docs/archive/benchmark_dashboard docs/book/dashboard/benchmark_dashboard",
                "",
            ),
            [],
        )
        semicolon_paths = _run_paths(
            "echo ok; cp -r docs/archive/benchmark_dashboard docs/book/dashboard/",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", semicolon_paths)
        self.assertIn("docs/book/dashboard", semicolon_paths)
        cd_paths = _run_paths(
            "cd docs && cp -r archive/benchmark_dashboard book/dashboard/",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", cd_paths)
        self.assertIn("docs/book/dashboard", cd_paths)
        self.assertEqual(
            _step_run_text({"run": ["cp -r docs/archive/benchmark_dashboard", "docs/book/dashboard/"]}),
            "cp -r docs/archive/benchmark_dashboard\ndocs/book/dashboard/",
        )
        self.assertEqual(_step_run_text({"run": True}), "True")
        self.assertEqual(_step_run_text({}), "")
        self.assertTrue(_is_archive_path("docs/archive/benchmark_dashboard"))
        self.assertTrue(_is_archive_path("docs/archive/benchmark_dashboard/index.html"))
        self.assertTrue(_is_archive_path("docs/archive"))
        self.assertTrue(_is_archive_path("docs"))
        self.assertFalse(_is_archive_path("docs/archive/benchmark_dashboardEXTRA"))
        self.assertFalse(_is_archive_path("docs/archive/benchmark"))
        self.assertFalse(_is_archive_path("docs/src"))
        self.assertFalse(_is_archive_path(""))
        self.assertTrue(_is_book_dashboard_target("docs/book/dashboard"))
        self.assertTrue(_is_book_dashboard_target("docs/book/dashboard/benchmark_dashboard"))
        self.assertTrue(
            _is_book_dashboard_target("docs/book/dashboard/benchmark_dashboard/index.html")
        )
        self.assertFalse(_is_book_dashboard_target("docs/book/dashboard/other"))
        self.assertFalse(_is_book_dashboard_target("docs/book/dashboard/benchmark_dashboardEXTRA"))
        self.assertFalse(_is_book_dashboard_target("docs/book"))
        self.assertTrue(_known_false(None))
        self.assertTrue(_known_false(False))
        self.assertTrue(_known_false("false"))
        self.assertTrue(_known_false(" FALSE "))
        self.assertFalse(_known_false(True))
        self.assertFalse(_known_false("true"))
        self.assertFalse(_known_false(" TRUE "))
        self.assertFalse(_known_false("${{ true }}"))
        self.assertFalse(_known_false(1))
        self.assertTrue(_known_true(True))
        self.assertTrue(_known_true("true"))
        self.assertTrue(_known_true(" TRUE "))
        self.assertFalse(_known_true(False))
        self.assertFalse(_known_true(None))
        self.assertFalse(_known_true("false"))
        self.assertFalse(_known_true("${{ true }}"))
        self.assertEqual(
            _folded_action_inputs({"Publish_Dir": "./docs"})["publish_dir"],
            "./docs",
        )
        folded = _folded_action_inputs(
            {"publish_dir": "./docs/benchmark_data", "Publish Dir": "./docs"}
        )
        self.assertEqual(folded["publish_dir"], "./docs")
        self.assertEqual(
            _folded_action_inputs({"Keep_Files": True})["keep_files"],
            True,
        )
        self.assertEqual(
            _step_cwd({"working-directory": "${{ github.workspace }}/docs"}, ""),
            "docs",
        )
        self.assertEqual(
            _step_cwd({"working-directory": "${{github.workspace}}/docs"}, "other"),
            "docs",
        )
        self.assertEqual(
            _step_cwd({"working-directory": "${{ GitHub.Workspace }}/docs"}, ""),
            "docs",
        )
        self.assertEqual(
            _step_cwd({"working-directory": "$GITHUB_WORKSPACE/docs"}, "other"),
            "docs",
        )
        self.assertEqual(
            _step_cwd({"working-directory": "$github_workspace/docs"}, ""),
            "$github_workspace/docs",
        )
        self.assertEqual(
            _step_cwd({"working-directory": "$GITHUB_WORKSPACE_EXTRA/docs"}, ""),
            "$GITHUB_WORKSPACE_EXTRA/docs",
        )
        self.assertEqual(
            _defaults_cwd(
                {"run": {"working-directory": "${{ github.workspace }}/docs"}}
            ),
            "docs",
        )
        self.assertEqual(
            _resolve_run_path("${{ github.workspace }}/docs/archive/benchmark_dashboard", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("$(echo docs/archive/benchmark_dashboard)", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("$(echo 'docs/archive/benchmark_dashboardEXTRA')", ""),
            "docs/archive/benchmark_dashboardEXTRA",
        )
        self.assertEqual(
            _resolve_run_path("$(echo docs/archive/benchmark_dashboard; true)", ""),
            "$(echo docs/archive/benchmark_dashboard; true)",
        )
        targeted = _run_paths(
            "cp --target-directory=docs/book/dashboard/benchmark_dashboard "
            "docs/archive/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/book/dashboard/benchmark_dashboard", targeted)
        self.assertIn("docs/archive/benchmark_dashboard", targeted)
        self.assertNotIn(
            "docs/book/dashboard/benchmark_dashboard",
            _run_paths(
                "cp --target-directory=/tmp/out docs/archive/benchmark_dashboard",
                "",
            ),
        )
        echoed = _run_paths(
            'cp "$(echo docs/archive/benchmark_dashboard)" '
            "docs/book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", echoed)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", echoed)
        extra_echo = _run_paths(
            'cp "$(echo docs/archive/benchmark_dashboardEXTRA)" '
            "docs/book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboardEXTRA", extra_echo)
        self.assertNotIn("docs/archive/benchmark_dashboard", extra_echo)
        pushed = _run_paths(
            "pushd docs\ncp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", pushed)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", pushed)
        popped = _run_paths(
            "pushd /tmp\npopd\ncp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", popped)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", popped)
        stayed = _run_paths(
            "popd\ncp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
            "docs",
        )
        self.assertIn("docs/archive/benchmark_dashboard", stayed)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", stayed)
        self.assertEqual(_pages_input_path("${{ 'dashboard' }}"), "dashboard")
        self.assertEqual(_pages_input_path("${{ 'dashboard/' }}"), "dashboard")
        self.assertEqual(_pages_input_path('  ${{ "dashboard" }}  '), "dashboard")
        self.assertEqual(_pages_input_path("${{ 'dash' }}${{ 'board' }}"), "dashboard")
        self.assertEqual(
            _pages_input_path("./docs/archive/benchmark_dashboard"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertIsNone(_pages_input_path("${{ github.ref_name }}"))
        self.assertIsNone(_pages_input_path("${{ 'dash"))
        self.assertIsNone(_pages_input_path(None))
        self.assertIsNone(_pages_input_path(True))
        self.assertTrue(_docs_destination_forbidden("dashboard"))
        self.assertTrue(_docs_destination_forbidden("dashboard/benchmark_dashboard"))
        self.assertTrue(
            _docs_destination_forbidden("dashboard/benchmark_dashboard/index.html")
        )
        self.assertFalse(_docs_destination_forbidden("dashboard/benchmark_dashboardEXTRA"))
        self.assertFalse(_docs_destination_forbidden("dashboard-notes"))
        self.assertFalse(_docs_destination_forbidden("dashboard/benchmark_data"))
        self.assertFalse(_docs_destination_forbidden("api"))
        self.assertTrue(_publish_dir_hits_archive("docs/archive/benchmark_dashboard"))
        self.assertTrue(
            _publish_dir_hits_archive("docs/archive/benchmark_dashboard/index.html")
        )
        self.assertFalse(
            _publish_dir_hits_archive("docs/archive/benchmark_dashboardEXTRA")
        )
        self.assertFalse(_publish_dir_hits_archive("docs"))
        self.assertFalse(_publish_dir_hits_archive("docs/archive"))
        self.assertFalse(_publish_dir_hits_archive("docs/book"))
        self.assertTrue(_same_repo_path("docs/benchmark_data/", "docs/benchmark_data"))
        self.assertTrue(_same_repo_path("./docs/benchmark_data", "docs/benchmark_data"))
        self.assertFalse(_same_repo_path("docs/benchmark_dataEXTRA", "docs/benchmark_data"))
        self.assertFalse(
            _same_repo_path("docs/benchmark_data/extra", "docs/benchmark_data")
        )
        self.assertFalse(_same_repo_path("docs", "docs/benchmark_data"))
        self.assertFalse(_same_repo_path(None, "docs/benchmark_data"))
        self.assertFalse(_same_repo_path(True, "docs/benchmark_data"))
        self.assertTrue(
            _is_jamesives_deploy(
                {"uses": "JamesIves/GitHub-Pages-Deploy-Action@v4"}
            )
        )
        self.assertTrue(
            _is_jamesives_deploy({"uses": "JamesIves/github-pages-deploy-action@v4"})
        )
        self.assertFalse(_is_jamesives_deploy({"uses": "peaceiris/actions-gh-pages@v4"}))
        self.assertFalse(_is_jamesives_deploy({}))
        self.assertEqual(_reduce_literal_expressions("plain"), "plain")
        self.assertEqual(
            _reduce_literal_expressions("${{ 'a' }}${{ 'b' }}"),
            "ab",
        )
        self.assertEqual(_reduce_literal_expressions("${{ 'lit' }}"), "lit")
        self.assertEqual(
            _reduce_literal_expressions("pre${{ 'mid' }}post"),
            "premidpost",
        )
        self.assertEqual(
            _reduce_literal_expressions("${{ 'a' }}-${{ 'b' }}"),
            "a-b",
        )
        self.assertEqual(_pages_input_path("${{'dashboard'}}"), "dashboard")
        self.assertIsNone(_reduce_literal_expressions("${{ github.workspace }}"))
        self.assertIsNone(_reduce_literal_expressions("${{ 'dashboard'"))
        self.assertIsNone(_reduce_literal_expressions("${{ 'docs\\\\archive\\\\benchmark_dashboard' }}"))
        self.assertIsNone(_quoted_literal(""))
        self.assertIsNone(_quoted_literal(" "))
        self.assertIsNone(_quoted_literal("'"))
        self.assertIsNone(_quoted_literal('"'))
        self.assertEqual(_quoted_literal("''"), "")
        self.assertEqual(_quoted_literal("'dashboard'"), "dashboard")
        self.assertIsNone(_quoted_literal("'dashboard\""))
        self.assertIsNone(_quoted_literal("XdashboardX"))
        self.assertIsNone(_quoted_literal("'da'shboard'"))
        self.assertIsNone(_quoted_literal("'a\\b'"))
        self.assertEqual(
            _resolve_run_path("  $(echo docs/archive/benchmark_dashboard)  ", ""),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _unwrap_echo_literal("$(echo '')"),
            "$(echo '')",
        )
        self.assertEqual(
            _unwrap_echo_literal("$(echo 'a b')"),
            "$(echo 'a b')",
        )
        self.assertEqual(
            _unwrap_echo_literal("$(echo 'docs/archive/benchmark_dashboard\")"),
            "'docs/archive/benchmark_dashboard\"",
        )
        self.assertEqual(_unwrap_echo_literal("$(echo ')"), "'")
        self.assertEqual(_unwrap_echo_literal("$(echo a;b)"), "$(echo a;b)")
        self.assertIsNone(_reduce_literal_expressions("${{ 'x' y"))
        self.assertEqual(_run_paths('""', ""), [])
        self.assertEqual(
            _run_paths(
                "pushd\ncp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
                "",
            ),
            [
                "archive/benchmark_dashboard",
                "book/dashboard/benchmark_dashboard",
            ],
        )
        self.assertIn(
            "docs/archive/benchmark_dashboard",
            _run_paths(
                "pushd -n\ncp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
                "docs",
            ),
        )
        self.assertIn(
            "docs/book/dashboard/benchmark_dashboard",
            _run_paths(
                "mv --target-directory=docs/book/dashboard/benchmark_dashboard "
                "docs/archive/benchmark_dashboard",
                "",
            ),
        )
        self.assertNotIn(
            "docs/book/dashboard/benchmark_dashboard",
            _run_paths(
                "rsync --target-directory=docs/book/dashboard/benchmark_dashboard "
                "docs/archive/benchmark_dashboard /tmp/out",
                "",
            ),
        )

        class _SamePath:
            def __str__(self) -> str:
                return "docs/benchmark_data"

        self.assertFalse(_same_repo_path(_SamePath(), "docs/benchmark_data"))
        self.assertFalse(_same_repo_path("/docs/benchmark_data", "docs/benchmark_data"))
        self.assertFalse(_same_repo_path(" /docs/benchmark_data", "docs/benchmark_data"))
        self.assertTrue(_same_repo_path(" docs/benchmark_data", "docs/benchmark_data"))
        self.assertTrue(_same_repo_path("docs/benchmark_data ", "docs/benchmark_data"))
        self.assertEqual(_normalize_cwd("   "), "")
        self.assertEqual(_normalize_cwd(1), "")
        self.assertEqual(_normalize_cwd(None), "")
        self.assertFalse(_explicit_false(None))
        self.assertTrue(_explicit_false(False))
        self.assertTrue(_explicit_false("false"))
        self.assertTrue(_explicit_false(" FALSE "))
        self.assertFalse(_explicit_false(True))
        self.assertFalse(_explicit_false("true"))
        self.assertFalse(_explicit_false("${{ false }}"))
        self.assertFalse(_explicit_false(0))
        self.assertTrue(_jamesives_branch_ok(None))
        self.assertTrue(_jamesives_branch_ok(""))
        self.assertTrue(_jamesives_branch_ok("  "))
        self.assertTrue(_jamesives_branch_ok("gh-pages"))
        self.assertFalse(_jamesives_branch_ok("main"))
        self.assertFalse(_jamesives_branch_ok("GH-pages"))
        self.assertFalse(_jamesives_branch_ok(False))
        self.assertTrue(_repository_name_unset(None))
        self.assertTrue(_repository_name_unset(""))
        self.assertFalse(_repository_name_unset("other/repo"))
        self.assertFalse(_repository_name_unset(" "))
        piped_cd = _run_paths(
            "cd docs | cp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("archive/benchmark_dashboard", piped_cd)
        self.assertIn("book/dashboard/benchmark_dashboard", piped_cd)
        self.assertNotIn("docs/archive/benchmark_dashboard", piped_cd)
        background_cd = _run_paths(
            "cd docs & cp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("archive/benchmark_dashboard", background_cd)
        self.assertNotIn("docs/archive/benchmark_dashboard", background_cd)
        self.assertEqual(_split_shell_commands("  cp a b  "), [("cp a b", True, False, False)])
        self.assertEqual(
            _split_shell_commands("cp a b;   ; cp c d"),
            [("cp a b", True, False, False), ("cp c d", True, False, False)],
        )
        self.assertEqual(
            _split_shell_commands("cd nope || true"),
            [("cd nope", True, True, False), ("true", True, False, True)],
        )
        self.assertEqual(
            _split_shell_commands("cd nope && true"),
            [("cd nope", True, True, False), ("true", True, False, True)],
        )
        self.assertEqual(_split_shell_commands("cd nope &&"), [("cd nope", True, False, False)])
        self.assertEqual(_split_shell_commands("cd nope ||"), [("cd nope", True, True, False)])
        self.assertEqual(
            _split_shell_commands("cp a\\\nb"),
            [("cp ab", True, False, False)],
        )
        self.assertEqual(_split_shell_commands("cp a\\\n"), [("cp a", True, False, False)])
        self.assertEqual(_split_shell_commands("cp a\\"), [("cp a\\", True, False, False)])
        self.assertEqual(
            _split_shell_commands('"a\\\nb"'),
            [('"ab"', True, False, False)],
        )
        self.assertEqual(_split_shell_commands('"a\\\n"'), [('"a"', True, False, False)])
        self.assertEqual(_split_shell_commands('"a\\\n'), [('"a', True, False, False)])
        self.assertEqual(_split_shell_commands('"a\\'), [('"a\\', True, False, False)])
        self.assertEqual(
            _split_shell_commands("'a\\\nb'"),
            [("'a\\\nb'", True, False, False)],
        )
        self.assertFalse(_next_errexit(["set", "+e"], [False, False], True))
        self.assertTrue(_next_errexit(["set", "-e"], [False, False], False))
        self.assertTrue(_next_errexit(["set", "+x"], [False, False], True))
        self.assertFalse(
            _next_errexit(["set", "+o", "errexit"], [False, False, False], True)
        )
        self.assertTrue(
            _next_errexit(["set", "-o", "errexit"], [False, False, False], False)
        )
        self.assertFalse(_next_errexit(["set", "+e"], [True, False], True))
        self.assertFalse(_next_errexit(["set", "+e"], [False, True], True))
        self.assertTrue(_next_errexit(["set", "+o"], [False, False], True))
        self.assertFalse(
            _next_errexit(["set", "+o", "errexit"], [False, False, True], True)
        )
        self.assertTrue(
            _next_errexit(["set", "+o", "-e"], [False, False, False], False)
        )
        self.assertTrue(_next_errexit(["set", "Ze"], [False, False], True))
        self.assertTrue(
            _next_errexit(["set", "+e", "-e"], [False, True, False], False)
        )
        self.assertTrue(
            _next_errexit(["set", "+x", "-e"], [False, False, False], False)
        )
        self.assertFalse(_next_errexit(["set", "+ex"], [False, False], True))
        self.assertTrue(
            _next_errexit(
                ["set", "+o", "errexit", "-e"],
                [False, False, False, False],
                False,
            )
        )
        self.assertEqual(_leading_redirect_span([">", "/dev/null", "cp"], 0), 2)
        self.assertEqual(_leading_redirect_span([">", "/dev/null"], 0), 2)
        self.assertEqual(_leading_redirect_span([">"], 0), 1)
        self.assertEqual(_leading_redirect_span(["cp"], 0), 0)
        self.assertEqual(_leading_redirect_span([">/dev/null", "cp"], 0), 1)
        self.assertEqual(
            _command_body([">", "/dev/null", "cp", "-r", "a", "b"]),
            ["cp", "-r", "a", "b"],
        )
        self.assertEqual(
            _command_body([">", "/dev/null", "command", "cp"]),
            ["cp"],
        )
        self.assertEqual(
            _command_body([">", "/tmp/a", "2>", "/tmp/b", "cp", "a", "b"]),
            ["cp", "a", "b"],
        )
        self.assertEqual(
            _command_body(['command', "cp"], [True, False]),
            ["cp"],
        )
        self.assertEqual(_leading_redirect_span(["2>", "/tmp/b", "cp"], 0), 2)
        self.assertTrue(_template_errexit(""))
        self.assertTrue(
            _template_errexit("bash --noprofile --norc -eo pipefail {0}")
        )
        self.assertFalse(_template_errexit("bash {0}"))
        self.assertTrue(_template_errexit("bash -e {0}"))
        self.assertFalse(_template_errexit("pwsh {0}"))
        self.assertEqual(
            _step_shell({"shell": " bash {0} "}, {"defaults": {"run": {"shell": "bash -e {0}"}}}, {}),
            "bash {0}",
        )
        self.assertEqual(
            _step_shell({}, {"defaults": {"run": {"shell": "bash {0}"}}}, {"defaults": {"run": {"shell": "bash -e {0}"}}}),
            "bash {0}",
        )
        for word in (">", ">>", "<", "<<", "<>", ">&", "<&", ">|", "&>", "&>>"):
            self.assertEqual(
                _command_body([word, "/dev/null", "cp", "a", "b"]),
                ["cp", "a", "b"],
                word,
            )
        self.assertTrue(
            _archive_copied_onto_book(
                "pushd docs/archiv?\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(_shell_words('"x" if')[1][1])
        self.assertEqual(_shell_words('"x" cd docs/archiv?')[2][2], "docs/archiv?")
        failed_glob = _shell_states("cd docs/arch*\ntrue", "")
        self.assertEqual(len(failed_glob), 1)
        self.assertEqual(failed_glob[0].cwd, "")
        self.assertEqual(failed_glob[0].paths, [])
        self.assertTrue(failed_glob[0].stopped)
        self.assertIs(
            _archive_copied_onto_book(
                "cp -r docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard\n"
                "cd docs/arch*",
                "",
            ),
            True,
        )
        self.assertTrue(
            _archive_copied_onto_book(
                'cp "" -r docs/archive/benchmark_dashboard '
                "docs/book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertTrue(
            _archive_copied_onto_book(
                'cp -r $(echo "docs/archive/benchmark_dashboard") '
                "docs/book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertTrue(_shell_words('cp $(echo "a\\b") dest')[1][0].startswith("$("))
        self.assertTrue(_shell_words("`echo a\\b`")[0][0].startswith("`"))
        self.assertTrue(_template_errexit("bash +o -e {0}"))
        self.assertTrue(_template_errexit("bash -o errexit {0}"))
        self.assertFalse(_template_errexit("bash +o errexit {0}"))
        self.assertTrue(_template_errexit("sh -e {0}"))
        self.assertFalse(_template_errexit("dash {0}"))
        self.assertEqual(_glob_escape("*"), "[*]")
        self.assertEqual(_glob_escape("a"), "a")
        self.assertFalse(_has_glob("docs"))
        self.assertTrue(_has_glob("docs/archiv?"))
        self.assertEqual(_command_body(["if", "cp", "a", "b"]), ["cp", "a", "b"])
        self.assertFalse(
            _next_errexit(["command", "set", "+e"], [False, False, False], True)
        )
        self.assertFalse(
            _next_errexit(["eval", "set", "+e"], [False, False, False], True)
        )
        self.assertFalse(
            _next_errexit(["FOO=1", "builtin", "set", "+e"], [False, False, False, False], True)
        )
        self.assertTrue(
            _archive_copied_onto_book(
                "cd $(echo archive)\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "docs",
            )
        )
        self.assertTrue(
            _archive_copied_onto_book(
                "cd `echo archive`\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "docs",
            )
        )
        stacked = _shell_states("pushd docs/book\ncd docs/archiv?\npopd\ntrue", "")
        self.assertEqual(stacked[0].cwd, "")
        popped = _shell_states("pushd docs/book\npopd docs/archiv?\ntrue", "")
        self.assertEqual(popped[0].cwd, "")
        self.assertTrue(
            _archive_copied_onto_book(
                "> /dev/null cp -r docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(
            _archive_copied_onto_book(
                "cp docs/arch* /tmp/out\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(
            _archive_copied_onto_book(
                "pushd docs/arch*\n"
                "popd\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(
            _archive_copied_onto_book(
                "cd 'docs/arch*'\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertTrue(
            _archive_copied_onto_book(
                "cd\n"
                "cp -r docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(
            _archive_copied_onto_book(
                "cd /tmp/no-such*\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertTrue(
            _archive_copied_onto_book(
                "cd docs/a[r]chiv?*\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertFalse(
            _archive_copied_onto_book(
                "cd /*\n"
                "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard",
                "",
            )
        )
        self.assertRaises(
            AssertionError,
            assert_docs_no_archive_benchmark_dashboard_copy,
            "name: Documentation\n"
            "on: [push]\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - echo only\n"
            "      - name: Copy\n"
            "        run: cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard\n",
        )
        self.assertEqual(_run_paths("command", ""), [])
        self.assertEqual(
            _command_body(["command", "-v", "cp"]),
            ["command", "-v", "cp"],
        )
        self.assertEqual(_command_body(["fi"]), [])
        self.assertEqual(_command_body(["{"]), [])
        self.assertEqual(
            _command_body(["if", "elif", "then", "else", "fi"]),
            [],
        )
        self.assertEqual(_command_body(["then", "cd"]), ["cd"])
        self.assertEqual(_command_body(["else", "cp"]), ["cp"])
        self.assertEqual(_command_body(["builtin", "cd"]), ["cd"])
        self.assertEqual(_command_body(["command", "cd"]), ["cd"])
        self.assertEqual(_command_body(["command", "command", "cd"]), ["cd"])
        self.assertEqual(_command_body(["command", "then", "cp"]), ["cp"])
        self.assertEqual(_run_paths("true &", ""), [])
        joined = _run_paths(
            "cd docs|& cp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("archive/benchmark_dashboard", joined)
        self.assertNotIn("docs/archive/benchmark_dashboard", joined)
        self.assertEqual(_expand_assigned_word("${dest", {"des": "BOOK"}), "${dest")
        self.assertEqual(_expand_assigned_word("xxdest}", {"dest": "BOOK"}), "xxdest}")
        self.assertEqual(_expand_assigned_word("xdest", {"dest": "BOOK"}), "xdest")
        self.assertEqual(
            _expand_assigned_word("$dest-x", {"dest-x": "BOOK"}),
            "$dest-x",
        )
        self.assertEqual(_expand_assigned_word("dest", {"dest": "BOOK"}), "dest")
        assigned = _run_paths(
            "dest=docs/book/dashboard/benchmark_dashboard; "
            "cp docs/archive/benchmark_dashboard ${dest}",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", assigned)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", assigned)
        unassigned = _run_paths(
            "dest=$other; cp docs/archive/benchmark_dashboard $dest",
            "",
        )
        self.assertIn("$dest", unassigned)
        self.assertNotIn("docs/book/dashboard/benchmark_dashboard", unassigned)
        backtick_assign = _run_paths(
            "dest=`other`; cp docs/archive/benchmark_dashboard $dest",
            "",
        )
        self.assertIn("$dest", backtick_assign)
        self.assertIn(
            "docs/archive/benchmark_dashboard",
            _run_paths(
                "C:\\bin\\cp docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard",
                "",
            ),
        )
        background_assign = _run_paths(
            "dest=docs/book/dashboard/benchmark_dashboard & "
            "cp docs/archive/benchmark_dashboard $dest",
            "",
        )
        self.assertIn("$dest", background_assign)
        self.assertNotIn("docs/book/dashboard/benchmark_dashboard", background_assign)
        self.assertEqual(
            _unwrap_echo_literal("$(echo -n docs/archive/benchmark_dashboard/index.html)"),
            "docs/archive/benchmark_dashboard/index.html",
        )
        self.assertEqual(
            _unwrap_echo_literal("$(echo Zdocs/archive/benchmark_dashboardZ)"),
            "Zdocs/archive/benchmark_dashboardZ",
        )
        self.assertIsNone(_quoted_literal("ZdashboardZ"))
        self.assertIsNone(_pages_input_path("${{ ZdashboardZ }}"))
        self.assertEqual(
            _pages_input_path("docs/archive/benchmark_dashboard"),
            "docs/archive/benchmark_dashboard",
        )
        masked_cd = _run_paths(
            "if [ -d nope ]; then cd nope; fi\n"
            "cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", masked_cd)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", masked_cd)
        masked_assign = _run_paths(
            "SRC=docs/archive/benchmark_dashboard\n"
            "if false; then SRC=nope; fi\n"
            'cp -r "$SRC" docs/book/dashboard/benchmark_dashboard',
            "",
        )
        self.assertIn("docs/archive/benchmark_dashboard", masked_assign)
        self.assertIn("docs/book/dashboard/benchmark_dashboard", masked_assign)
        unconditional_cd = _run_paths(
            "cd nope\n"
            "cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard",
            "",
        )
        self.assertNotIn("docs/archive/benchmark_dashboard", unconditional_cd)
        self.assertIn("nope/docs/archive/benchmark_dashboard", unconditional_cd)
        self.assertFalse(_next_errexit(["set", "--", "-e"], [False, False, False], False))
        self.assertTrue(_next_errexit(["set", "--", "+e"], [False, False, False], True))
        self.assertTrue(_next_errexit(["set", "-e", "--", "+e"], [False, False, False, False], False))
        self.assertEqual(_read_shopt(["-uo", "errexit"], True, False), (False, False))
        self.assertEqual(_read_shopt(["-u", "-o", "errexit"], True, False), (False, False))
        self.assertEqual(_read_shopt(["-s", "-o", "errexit"], False, False), (True, False))
        self.assertEqual(_read_shopt(["-s", "expand_aliases"], True, False), (True, True))
        self.assertFalse(_template_errexit("bash {0} -e"))
        self.assertFalse(_template_errexit("bash {0} -o errexit"))
        self.assertTrue(_template_errexit("bash -e {0} -x"))
        self.assertIs(_template_errexit("pwsh {0}"), False)
        self.assertIs(_template_errexit("dash {0}"), False)
        self.assertEqual(
            _step_shell({}, {}, {"defaults": {"run": {"shell": "bash {0}"}}}),
            "bash {0}",
        )
        self.assertEqual(_defaults_shell(None), "")
        self.assertEqual(_defaults_shell([]), "")
        self.assertEqual(_defaults_shell({"run": {"shell": "  "}}), "")
        self.assertEqual(_defaults_shell({"run": "bash"}), "")
        self.assertEqual(
            _resolve_run_path("${GITHUB_WORKSPACE}/docs/archive/benchmark_dashboard", "docs/book"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(
            _resolve_run_path("GITHUB_WORKSPACE/docs/archive/benchmark_dashboard", "other"),
            "docs/archive/benchmark_dashboard",
        )
        self.assertEqual(_resolve_run_path("docs/*", ""), "docs")
        self.assertEqual(
            _resolve_run_path("docs/book/dashboard/*", ""),
            "docs/book/dashboard",
        )
        self.assertEqual(
            _resolve_run_path("./docs/book/dashboard/*", ""),
            "docs/book/dashboard",
        )
        self.assertEqual(_closing_paren("(')')", 0), 4)
        self.assertEqual(_closing_paren('("\\")")', 0), 6)
        self.assertEqual(_closing_paren("(", 0), 0)
        self.assertEqual(_closing_paren("(abc", 0), 3)
        self.assertEqual(_closing_paren('("a")', 0), 4)
        self.assertEqual(_closing_backtick("`a`", 0), 2)
        self.assertEqual(_closing_backtick("`\\``", 0), 3)
        self.assertEqual(_closing_backtick("`", 0), 0)
        self.assertIsNone(_shell_words("cp")[0][2])
        self.assertEqual(len(_shell_words("$(echo a b)")), 1)
        self.assertEqual(_shell_words('""'), [])
        self.assertEqual(_shell_words("a*")[0][2], "a*")
        self.assertIsNone(_shell_words("'a*'")[0][2])
        self.assertEqual(_opening_depth(["if", "cp"], ["cp"], 0, [False, False]), 1)
        self.assertEqual(_opening_depth(["if", "cp"], ["cp"], 0, [True, False]), 0)
        self.assertEqual(_opening_depth(["fi", "cp"], ["cp"], 1, [False, False]), 0)
        self.assertEqual(_opening_depth(["cp"], ["nope"], 0), 1)
        self.assertEqual(_glob_directories("docs/archiv?", ""), ["docs/archive"])
        self.assertGreater(len(_glob_directories("docs/arch*", "")), 1)
        self.assertIs(_archive_copied_onto_book("echo ok", ""), False)
        self.assertIs(
            _archive_copied_onto_book(
                "Z x cp -r docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard",
                "",
            ),
            False,
        )
        self.assertIs(
            _archive_copied_onto_book(
                "cp -r docs/archive/benchmark_dashboard "
                "docs/book/dashboard/benchmark_dashboard\n"
                "pushd docs/arch*",
                "",
            ),
            True,
        )
        one = (True, False, False)
        self.assertEqual(_split_shell_commands("2>&1 cp a b"), [("2>&1 cp a b", *one)])
        self.assertEqual(
            _split_shell_commands("&>/dev/null cp a b"),
            [("&>/dev/null cp a b", *one)],
        )
        self.assertEqual(
            _split_shell_commands("&>>/dev/null cp a b"),
            [("&>>/dev/null cp a b", *one)],
        )
        self.assertEqual(
            _split_shell_commands(">| /dev/null cp a b"),
            [(">| /dev/null cp a b", *one)],
        )
        self.assertEqual(
            _split_shell_commands("echo foo>&2"),
            [("echo foo>&2", *one)],
        )
        self.assertEqual(
            _split_shell_commands("echo foo>|bar"),
            [("echo foo>|bar", *one)],
        )
        self.assertEqual(
            _split_shell_commands("echo foo&>bar"),
            [("echo foo&>bar", *one)],
        )
        self.assertEqual(
            _split_shell_commands("a>&&b"),
            [("a>", True, True, False), ("b", True, False, True)],
        )
        self.assertEqual(
            _split_shell_commands("a>||b"),
            [("a>", True, True, False), ("b", True, False, True)],
        )
        self.assertEqual(
            _split_shell_commands("a|&b"),
            [("a", False, False, False), ("b", True, False, False)],
        )
        self.assertEqual(
            _split_shell_commands("cd docs&extra"),
            [("cd docs", False, False, False), ("extra", True, False, False)],
        )
        self.assertEqual(
            _split_shell_commands("2>&1&&cp a b"),
            [("2>&1", True, True, False), ("cp a b", True, False, True)],
        )
        self.assertEqual(
            _split_shell_commands("a>&2&&b"),
            [("a>&2", True, True, False), ("b", True, False, True)],
        )
        self.assertEqual(_redirect_operator_end("a>&&b", 2, "a>"), 2)
        self.assertEqual(_redirect_operator_end("echo>&2", 5, "echo>"), 7)
        self.assertEqual(_redirect_operator_end("2>&1 cp", 2, "2>"), 4)
        self.assertEqual(_redirect_operator_end("echo>|f", 5, "echo>"), 7)
        self.assertEqual(_redirect_operator_end("2>&-", 2, "2>"), 4)
        self.assertEqual(
            _strip_trailing_redirects(
                ["cp", "a", "b", ">", "f"],
                [False, False, False, False, False],
                [None, None, None, None, None],
            )[0],
            ["cp", "a", "b"],
        )
        self.assertEqual(
            _strip_trailing_redirects(
                ["cp", "a", "b", "2>&1"],
                [False, False, False, False],
                [None, None, None, None],
            )[0],
            ["cp", "a", "b"],
        )
        kept, funcs = _extract_functions("publish() { cp a b; }\npublish")
        self.assertEqual(funcs, {"publish": " cp a b; "})
        self.assertIn("publish", kept)
        kept, funcs = _extract_functions("function publish { cp a b; }\npublish")
        self.assertEqual(funcs, {"publish": " cp a b; "})
        kept, funcs = _extract_functions("function publish() { cp a b; }\npublish")
        self.assertEqual(funcs, {"publish": " cp a b; "})
        kept, funcs = _extract_functions("publish() { echo '}'; }\npublish")
        self.assertEqual(funcs, {"publish": " echo '}'; "})
        kept, funcs = _extract_functions("pre publish() { cp a b; } post")
        self.assertEqual(funcs, {"publish": " cp a b; "})
        self.assertIn("pre", kept)
        self.assertIn("post", kept)
        kept, funcs = _extract_functions("publish () { cp a b; }")
        self.assertEqual(list(funcs), ["publish"])
        text, funcs = _extract_functions("Date().toISOString()")
        self.assertEqual(funcs, {})
        self.assertEqual(text, "Date().toISOString()")
        text, funcs = _extract_functions("'publish() { cp a b; }'")
        self.assertEqual(funcs, {})
        text, funcs = _extract_functions("$(publish() { cp a b; })")
        self.assertEqual(funcs, {})
        self.assertEqual(
            _match_function("publish() { cp a b; }", 0),
            ("publish", " cp a b; ", 21),
        )
        self.assertEqual(
            _match_function("function publish { cp a b; }", 0),
            ("publish", " cp a b; ", 28),
        )
        self.assertEqual(
            _match_function("function publish() { cp a b; }", 0),
            ("publish", " cp a b; ", 30),
        )
        self.assertEqual(_match_function("publish () { cp; }", 0)[0], "publish")
        self.assertIsNone(_match_function("Date().toISOString()", 0))
        self.assertIsNone(_match_function("not a function", 0))
        self.assertIsNone(_match_function("xpublish() { }", 1))
        self.assertEqual(_matching_brace("{a}", 0), 2)
        self.assertEqual(_matching_brace("{a{b}c}", 0), 6)
        self.assertEqual(_matching_brace("{a'}'b}", 0), 6)
        self.assertEqual(_matching_brace('{a"}"b}', 0), 6)
        self.assertEqual(_matching_brace("{a\\}b}", 0), 3)
        self.assertEqual(_read_shopt(["-u", "expand_aliases"], True, True), (True, False))
        self.assertEqual(_read_shopt(["--", "-s", "errexit"], True, False), (True, False))
        self.assertEqual(_read_shopt(["-s", "-u", "-o", "errexit"], True, False), (True, False))
        self.assertIs(_template_errexit("bash -e {0}"), True)
        self.assertIs(_template_errexit("bash -- -e {0}"), False)
        self.assertIs(_template_errexit("bash -o errexit {0}"), True)
        self.assertEqual(_skip_horizontal_space("  a", 0), 2)
        self.assertTrue(_word_edge("abc", 3))
        self.assertFalse(_word_edge("abc", 1))
        self.assertEqual(_alias_binding(["alias", "enter=cd"]), ("enter", "cd"))
        self.assertIsNone(_alias_binding(["alias", "1bad=cd"]))
        self.assertFalse(_cd_uses_cdpath(".archive", {"CDPATH": "docs"}))
        self.assertFalse(_cd_uses_cdpath("/tmp", {"CDPATH": "docs"}))
        self.assertTrue(_cd_uses_cdpath("archive", {"CDPATH": "docs"}))
        self.assertEqual(_cdpath_directory("archive", "", "docs"), "docs/archive")
        self.assertIsNone(_cdpath_directory("nope", "", "docs"))
        rel_copy = "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard"
        book_copy = (
            "cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard"
        )
        self.assertIs(
            _archive_copied_onto_book(
                "CDPATH=missing:docs\ncd archive\n" + rel_copy, ""
            ),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("CDPATH=missing\ncd archive\n" + rel_copy, ""),
            False,
        )
        self.assertIs(
            _archive_copied_onto_book("CDPATH=docs:\ncd archive\n" + rel_copy, ""),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("trap '" + book_copy + "' 0\ntrue", ""),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("trap '" + book_copy + "' exit\ntrue", ""),
            True,
        )
        nested = "inner() { " + book_copy + "; }\nouter() { inner; }\nouter"
        self.assertIs(_archive_copied_onto_book(nested, ""), True)
        chain = ["f0() { " + book_copy + "; }"]
        for index in range(1, 9):
            chain.append(f"f{index}() {{ f{index - 1}; }}")
        chain.append("f8")
        self.assertIs(_archive_copied_onto_book("\n".join(chain), ""), False)
        big = ROOT / "scripts/ci/testdata/docs_source_copy/big.sh"
        try:
            big.write_bytes(b"a" * 1_000_001)
            self.assertIsNone(
                _read_workspace_file("scripts/ci/testdata/docs_source_copy/big.sh")
            )
        finally:
            big.unlink(missing_ok=True)
        self._pages_form_oracle()


    def _pages_form_oracle(self) -> None:
        """Pin shell-form results the pages suite reads."""
        expected = json.loads(
"{\"split:2>&1 cp a b\":[\"ok\",\"[('2>&1 cp a b', True, False, False)]\"],\"split:&>/dev/null cp a b\":[\"ok\",\"[('&>/dev/null cp a b', True, False, False)]\"],\"split:&>>/dev/null cp a b\":[\"ok\",\"[('&>>/dev/null cp a b', True, False, False)]\"],\"split:>| /dev/null cp a b\":[\"ok\",\"[('>| /dev/null cp a b', True, False, False)]\"],\"split:2>&- cp a b\":[\"ok\",\"[('2>&- cp a b', True, False, False)]\"],\"split:>&2 echo hi\":[\"ok\",\"[('>&2 echo hi', True, False, False)]\"],\"split:<&0 cat\":[\"ok\",\"[('<', False, False, False), ('0 cat', True, False, False)]\"],\"split:cd docs/archive 2>&1\":[\"ok\",\"[('cd docs/archive 2>&1', True, False, False)]\"],\"split:SRC=docs/archive 2>&1\":[\"ok\",\"[('SRC=docs/archive 2>&1', True, False, False)]\"],\"split:a&&b\":[\"ok\",\"[('a', True, True, False), ('b', True, False, True)]\"],\"split:a||b\":[\"ok\",\"[('a', True, True, False), ('b', True, False, True)]\"],\"split:a|b\":[\"ok\",\"[('a', False, False, False), ('b', True, False, False)]\"],\"split:a&b\":[\"ok\",\"[('a', False, False, False), ('b', True, False, False)]\"],\"split:a|&b\":[\"ok\",\"[('a', False, False, False), ('b', True, False, False)]\"],\"split:cd docs&extra\":[\"ok\",\"[('cd docs', False, False, False), ('extra', True, False, False)]\"],\"split:a>&&b\":[\"ok\",\"[('a>', True, True, False), ('b', True, False, True)]\"],\"split:a>||b\":[\"ok\",\"[('a>', True, True, False), ('b', True, False, True)]\"],\"split:a>&2&&b\":[\"ok\",\"[('a>&2', True, True, False), ('b', True, False, True)]\"],\"split:a&>b&&c\":[\"ok\",\"[('a&>b', True, True, False), ('c', True, False, True)]\"],\"split:echo Z && cp a b\":[\"ok\",\"[('echo Z', True, True, False), ('cp a b', True, False, True)]\"],\"split:true & Z\":[\"ok\",\"[('true', False, False, False), ('Z', True, False, False)]\"],\"split:cmd>file\":[\"ok\",\"[('cmd>file', True, False, False)]\"],\"split:cmd>>file\":[\"ok\",\"[('cmd>>file', True, False, False)]\"],\"split:a\\nb\":[\"ok\",\"[('a', True, False, False), ('b', True, False, False)]\"],\"split:a;b\":[\"ok\",\"[('a', True, False, False), ('b', True, False, False)]\"],\"split:'a&&b'\":[\"ok\",\"[(\\\"'a&&b'\\\", True, False, False)]\"],\"split:\\\"a&&b\\\"\":[\"ok\",\"[('\\\"a&&b\\\"', True, False, False)]\"],\"split:a\\\\\\nb\":[\"ok\",\"[('ab', True, False, False)]\"],\"split:2>&1&&cp a b\":[\"ok\",\"[('2>&1', True, True, False), ('cp a b', True, False, True)]\"],\"split:echo foo>&2\":[\"ok\",\"[('echo foo>&2', True, False, False)]\"],\"split:echo foo>|bar\":[\"ok\",\"[('echo foo>|bar', True, False, False)]\"],\"split:echo foo&>bar\":[\"ok\",\"[('echo foo&>bar', True, False, False)]\"],\"split:echo foo&>>bar\":[\"ok\",\"[('echo foo&>>bar', True, False, False)]\"],\"split:echo foo<&0\":[\"ok\",\"[('echo foo<', False, False, False), ('0', True, False, False)]\"],\"split:echo foo 2>&1 | cat\":[\"ok\",\"[('echo foo 2>&1', False, False, False), ('cat', True, False, False)]\"],\"end:2>&1 cp\":[\"ok\",\"4\"],\"end:a&&b\":[\"ok\",\"1\"],\"end:a||b\":[\"ok\",\"1\"],\"end:a|&b\":[\"ok\",\"1\"],\"end:a&b\":[\"ok\",\"1\"],\"end:echo>&2\":[\"ok\",\"7\"],\"end:echo>|f\":[\"ok\",\"7\"],\"end:echo&>f\":[\"ok\",\"5\"],\"end:echo&>>f\":[\"ok\",\"5\"],\"end:a>&&b\":[\"ok\",\"2\"],\"end:Z>&1\":[\"ok\",\"1\"],\"end:2>&-\":[\"ok\",\"4\"],\"end:<&0\":[\"ok\",\"1\"],\"fn:publish() { cp a b; }\\npublish\":[\"ok\",\"(' \\\\npublish', {'publish': ' cp a b; '})\"],\"fn:function publish { cp a b; }\\np\":[\"ok\",\"(' \\\\npublish', {'publish': ' cp a b; '})\"],\"fn:function publish() { cp a b; }\":[\"ok\",\"(' \\\\npublish', {'publish': ' cp a b; '})\"],\"fn:Date().toISOString()\":[\"ok\",\"('Date().toISOString()', {})\"],\"fn:'publish() { cp a b; }'\":[\"ok\",\"(\\\"'publish() { cp a b; }'\\\", {})\"],\"fn:$(publish() { cp a b; })\":[\"ok\",\"('$(publish() { cp a b; })', {})\"],\"fn:publish() { echo '}'; }\\npublis\":[\"ok\",\"(' \\\\npublish', {'publish': \\\" echo '}'; \\\"})\"],\"fn:pre publish() { cp a b; } post\":[\"ok\",\"('pre   post', {'publish': ' cp a b; '})\"],\"fn:a() { b; }\\nZ() { c; }\":[\"ok\",\"(' \\\\n ', {'a': ' b; ', 'Z': ' c; '})\"],\"brace:{a}\":[\"ok\",\"2\"],\"brace:{a{b}c}\":[\"ok\",\"6\"],\"brace:{a'}'b}\":[\"ok\",\"6\"],\"brace:{a\\\"}\\\"b}\":[\"ok\",\"6\"],\"brace:{a\\\\}b}\":[\"ok\",\"3\"],\"brace:{\":[\"ok\",\"None\"],\"brace:a{b\":[\"ok\",\"None\"],\"match:publish() { cp a b; }\":[\"ok\",\"('publish', ' cp a b; ', 21)\"],\"match:function publish { cp a b; }\":[\"ok\",\"('publish', ' cp a b; ', 28)\"],\"match:function publish() { cp a b; }\":[\"ok\",\"('publish', ' cp a b; ', 30)\"],\"match:not a function\":[\"ok\",\"None\"],\"match:xpublish() { }\":[\"ok\",\"('xpublish', ' ', 14)\"],\"match:publish () { cp; }\":[\"ok\",\"('publish', ' cp; ', 18)\"],\"match:Date().toISOString()\":[\"ok\",\"None\"],\"strip:cp a b 2>&1\":[\"ok\",\"(['cp', 'a', 'b'], [False, False, False], [None, None, None])\"],\"strip:cp a b > f\":[\"ok\",\"(['cp', 'a', 'b'], [False, False, False], [None, None, None])\"],\"strip:2>&1 cp a b\":[\"ok\",\"(['2>&1', 'cp', 'a', 'b'], [False, False, False, False], [None, None, None, None])\"],\"strip:cp a b\":[\"ok\",\"(['cp', 'a', 'b'], [False, False, False], [None, None, None])\"],\"strip:cp a b >&2\":[\"ok\",\"(['cp', 'a', 'b'], [False, False, False], [None, None, None])\"],\"strip:cp \\\"2>&1\\\"\":[\"ok\",\"(['cp', '\\\"2>&1\\\"'], [False, True], [None, None])\"],\"shopt:-uo errexit\":[\"ok\",\"(False, False)\"],\"shopt:-u -o errexit\":[\"ok\",\"(False, False)\"],\"shopt:-s -o errexit\":[\"ok\",\"(True, False)\"],\"shopt:-s expand_aliases\":[\"ok\",\"(False, True)\"],\"shopt:-u expand_aliases\":[\"ok\",\"(True, False)\"],\"shopt:-s -u -o errexit\":[\"ok\",\"(True, False)\"],\"shopt:-s globstar nullglob\":[\"ok\",\"(True, False)\"],\"shopt:-- -s errexit\":[\"ok\",\"(True, False)\"],\"shopt:-o\":[\"ok\",\"(True, False)\"],\"shopt:-Z errexit\":[\"ok\",\"(True, False)\"],\"tmpl:bash {0} -e\":[\"ok\",\"False\"],\"tmpl:bash -e {0}\":[\"ok\",\"True\"],\"tmpl:bash {0} -o errexit\":[\"ok\",\"False\"],\"tmpl:bash -o errexit {0}\":[\"ok\",\"True\"],\"tmpl:bash -- -e {0}\":[\"ok\",\"False\"],\"tmpl:pwsh\":[\"ok\",\"False\"],\"tmpl:dash {0}\":[\"ok\",\"False\"],\"tmpl:sh {0}\":[\"ok\",\"False\"],\"tmpl:\":[\"ok\",\"True\"],\"tmpl:bash {0}\":[\"ok\",\"False\"],\"tmpl:Z {0} -e\":[\"ok\",\"False\"],\"next:set -- -e\":[\"ok\",\"False\"],\"next:set -- +e\":[\"ok\",\"True\"],\"next:set -e\":[\"ok\",\"True\"],\"next:set +e\":[\"ok\",\"False\"],\"next:set -e -- +e\":[\"ok\",\"True\"],\"next:set +o -e\":[\"ok\",\"True\"],\"next:set +o errexit\":[\"ok\",\"False\"],\"next:true\":[\"ok\",\"True\"],\"next:set -Z\":[\"ok\",\"True\"],\"h:paren\":[\"ok\",\"4\"],\"h:paren2\":[\"ok\",\"6\"],\"h:tick\":[\"ok\",\"2\"],\"h:tick2\":[\"ok\",\"3\"],\"h:words\":[\"ok\",\"[]\"],\"h:words2\":[\"ok\",\"[('a*', False, 'a*')]\"],\"h:words3\":[\"ok\",\"[('a*', True, None)]\"],\"h:words4\":[\"ok\",\"[('cp', False, None)]\"],\"h:words5\":[\"ok\",\"[('$(echo a b)', False, None)]\"],\"h:wordsZ\":[\"ok\",\"[('Z', False, None)]\"],\"h:glob\":[\"ok\",\"['docs/archive']\"],\"h:glob2\":[\"ok\",\"['docs/architecture', 'docs/archive']\"],\"h:ws\":[\"ok\",\"'docs/archive/x'\"],\"h:ws2\":[\"ok\",\"'docs/a'\"],\"h:star\":[\"ok\",\"'docs/book/dashboard'\"],\"h:cdpath\":[\"ok\",\"'docs/archive'\"],\"h:cdpath2\":[\"ok\",\"None\"],\"h:uses\":[\"ok\",\"True\"],\"h:uses2\":[\"ok\",\"False\"],\"h:uses3\":[\"ok\",\"False\"],\"h:repo\":[\"ok\",\"True\"],\"h:read\":[\"ok\",\"True\"],\"h:read2\":[\"ok\",\"None\"],\"h:edge\":[\"ok\",\"True\"],\"h:edge2\":[\"ok\",\"False\"],\"h:skip\":[\"ok\",\"2\"],\"h:alias\":[\"ok\",\"('enter', 'cd')\"],\"h:alias2\":[\"ok\",\"None\"],\"st:cp -r docs/archive/benchmark_dashboard d\":[\"ok\",\"(('', True, True, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:cp -r docs/archive/benchmark_dashboard d\":[\"ok\",\"True\"],\"st:2>&1 cp -r docs/archive/benchmark_dashbo\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:2>&1 cp -r docs/archive/benchmark_dashbo\":[\"ok\",\"True\"],\"st:&>/dev/null cp -r docs/archive/benchmark\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:&>/dev/null cp -r docs/archive/benchmark\":[\"ok\",\"True\"],\"st:>| /dev/null cp -r docs/archive/benchmar\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:>| /dev/null cp -r docs/archive/benchmar\":[\"ok\",\"True\"],\"st:cd docs/archive 2>&1|cp -r benchmark_das\":[\"ok\",\"(('docs/archive', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:cd docs/archive 2>&1|cp -r benchmark_das\":[\"ok\",\"True\"],\"st:publish() { cp -r docs/archive/benchmark\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:publish() { cp -r docs/archive/benchmark\":[\"ok\",\"True\"],\"st:function relax { set +e; }|relax|cd nope\":[\"ok\",\"(('docs/archive', False, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''), ('docs/archive/nope', False, False, ('docs/archive/nope/benchmark_dashboard', 'docs/archive/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'))\"],\"cp:function relax { set +e; }|relax|cd nope\":[\"ok\",\"True\"],\"st:shopt -s expand_aliases|alias enter='cd \":[\"ok\",\"(('docs/archive', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), True, (('enter', 'cd docs/archive'),), None, ''),)\"],\"cp:shopt -s expand_aliases|alias enter='cd \":[\"ok\",\"True\"],\"st:cd docs|cd archive|cd -|cp -r archive/be\":[\"ok\",\"(('docs', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'),)\"],\"cp:cd docs|cd archive|cd -|cp -r archive/be\":[\"ok\",\"True\"],\"st:CDPATH=docs|cd archive|cp -r benchmark_d\":[\"ok\",\"(('docs/archive', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:CDPATH=docs|cd archive|cp -r benchmark_d\":[\"ok\",\"True\"],\"st:trap 'cp -r docs/archive/benchmark_dashb\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:trap 'cp -r docs/archive/benchmark_dashb\":[\"ok\",\"True\"],\"st:. scripts/ci/testdata/docs_source_copy/p\":[\"ok\",\"(('', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:. scripts/ci/testdata/docs_source_copy/p\":[\"ok\",\"True\"],\"st:D=docs/archive|cd \\\"$D\\\"|cp -r benchmark_d\":[\"ok\",\"(('docs/archive', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:D=docs/archive|cd \\\"$D\\\"|cp -r benchmark_d\":[\"ok\",\"True\"],\"st:P=docs/archiv?|cd $P|cp -r benchmark_das\":[\"ok\",\"(('docs/archive', True, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''),)\"],\"cp:P=docs/archiv?|cd $P|cp -r benchmark_das\":[\"ok\",\"True\"],\"st:set +e|set -- -e|cd nope|cp -r benchmark\":[\"ok\",\"(('docs/archive', False, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''), ('docs/archive/nope', False, False, ('docs/archive/nope/benchmark_dashboard', 'docs/archive/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'))\"],\"cp:set +e|set -- -e|cd nope|cp -r benchmark\":[\"ok\",\"True\"],\"st:set -- +e|cd nope|cp -r benchmark_dashbo\":[\"ok\",\"(('docs/archive/nope', True, False, ('docs/archive/nope/benchmark_dashboard', 'docs/archive/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'),)\"],\"cp:set -- +e|cd nope|cp -r benchmark_dashbo\":[\"ok\",\"False\"],\"st:shopt -uo errexit|cd nope|cp -r benchmar\":[\"ok\",\"(('docs/archive', False, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''), ('docs/archive/nope', False, False, ('docs/archive/nope/benchmark_dashboard', 'docs/archive/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'))\"],\"cp:shopt -uo errexit|cd nope|cp -r benchmar\":[\"ok\",\"True\"],\"st:eval 'set +e'|cd nope|cp -r benchmark_da\":[\"ok\",\"(('docs/archive', False, False, ('docs/archive/benchmark_dashboard', 'docs/book/dashboard/benchmark_dashboard'), False, (), None, ''), ('docs/archive/nope', False, False, ('docs/archive/nope/benchmark_dashboard', 'docs/archive/book/dashboard/benchmark_dashboard'), False, (), None, 'docs/archive'))\"],\"cp:eval 'set +e'|cd nope|cp -r benchmark_da\":[\"ok\",\"True\"],\"st:cd docs/arch*|true\":[\"ok\",\"(('', True, True, (), False, (), None, ''),)\"],\"cp:cd docs/arch*|true\":[\"ok\",\"False\"],\"st:Z x cp -r docs/archive/benchmark_dashboa\":[\"ok\",\"(('', True, False, (), False, (), None, ''),)\"],\"cp:Z x cp -r docs/archive/benchmark_dashboa\":[\"ok\",\"False\"],\"st:cd -\":[\"ok\",\"(('', True, True, (), False, (), None, ''),)\"],\"cp:cd -\":[\"ok\",\"False\"]}"
        )
        got: dict[str, list[str]] = {}

        def add(name: str, fn) -> None:
            try:
                got[name] = ["ok", repr(fn())]
            except Exception as exc:
                got[name] = ["err", type(exc).__name__]

        copy = (
            "cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard"
        )
        rel = "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard"
        splits = [
            "2>&1 cp a b", "&>/dev/null cp a b", "&>>/dev/null cp a b",
            ">| /dev/null cp a b", "2>&- cp a b", ">&2 echo hi", "<&0 cat",
            "cd docs/archive 2>&1", "SRC=docs/archive 2>&1", "a&&b", "a||b",
            "a|b", "a&b", "a|&b", "cd docs&extra", "a>&&b", "a>||b",
            "a>&2&&b", "a&>b&&c", "echo Z && cp a b", "true & Z", "cmd>file",
            "cmd>>file", "a\nb", "a;b", "'a&&b'", '"a&&b"', "a\\\nb",
            "2>&1&&cp a b", "echo foo>&2", "echo foo>|bar", "echo foo&>bar",
            "echo foo&>>bar", "echo foo<&0", "echo foo 2>&1 | cat",
        ]
        for text in splits:
            add("split:" + text, lambda text=text: _split_shell_commands(text))
        ends = [
            ("2>&1 cp", 2, "2>"), ("a&&b", 1, "a"), ("a||b", 1, "a"),
            ("a|&b", 1, "a"), ("a&b", 1, "a"), ("echo>&2", 5, "echo>"),
            ("echo>|f", 5, "echo>"), ("echo&>f", 5, "echo"),
            ("echo&>>f", 5, "echo"), ("a>&&b", 2, "a>"), ("Z>&1", 1, "Z>"),
            ("2>&-", 2, "2>"), ("<&0", 1, "<"),
        ]
        for text, index, current in ends:
            add(
                "end:" + text,
                lambda text=text, index=index, current=current: _redirect_operator_end(
                    text, index, current
                ),
            )
        funcs = [
            "publish() { cp a b; }\npublish",
            "function publish { cp a b; }\npublish",
            "function publish() { cp a b; }\npublish",
            "Date().toISOString()",
            "'publish() { cp a b; }'",
            "$(publish() { cp a b; })",
            "publish() { echo '}'; }\npublish",
            "pre publish() { cp a b; } post",
            "a() { b; }\nZ() { c; }",
        ]
        for text in funcs:
            add("fn:" + text[:30], lambda text=text: _extract_functions(text))
        for text in ["{a}", "{a{b}c}", "{a'}'b}", '{a"}"b}', "{a\\}b}", "{", "a{b"]:
            add(
                "brace:" + text,
                lambda text=text: _matching_brace(text, text.find("{") if "{" in text else 0),
            )
        for text in [
            "publish() { cp a b; }",
            "function publish { cp a b; }",
            "function publish() { cp a b; }",
            "not a function",
            "xpublish() { }",
            "publish () { cp; }",
            "Date().toISOString()",
        ]:
            add("match:" + text[:30], lambda text=text: _match_function(text, 0))
        strips = [
            (["cp", "a", "b", "2>&1"], [False] * 4, [None] * 4),
            (["cp", "a", "b", ">", "f"], [False] * 5, [None] * 5),
            (["2>&1", "cp", "a", "b"], [False] * 4, [None] * 4),
            (["cp", "a", "b"], [False] * 3, [None] * 3),
            (["cp", "a", "b", ">&2"], [False] * 4, [None] * 4),
            (["cp", '"2>&1"'], [False, True], [None, None]),
        ]
        for words, quoted, globs in strips:
            add(
                "strip:" + " ".join(words),
                lambda words=words, quoted=quoted, globs=globs: _strip_trailing_redirects(
                    words, quoted, globs
                ),
            )
        shopts = [
            (["-uo", "errexit"], True, False),
            (["-u", "-o", "errexit"], True, False),
            (["-s", "-o", "errexit"], False, False),
            (["-s", "expand_aliases"], False, False),
            (["-u", "expand_aliases"], True, True),
            (["-s", "-u", "-o", "errexit"], True, False),
            (["-s", "globstar", "nullglob"], True, False),
            (["--", "-s", "errexit"], True, False),
            (["-o"], True, False),
            (["-Z", "errexit"], True, False),
        ]
        for args, errexit, expand in shopts:
            add(
                "shopt:" + " ".join(args),
                lambda args=args, errexit=errexit, expand=expand: _read_shopt(
                    args, errexit, expand
                ),
            )
        for text in [
            "bash {0} -e", "bash -e {0}", "bash {0} -o errexit",
            "bash -o errexit {0}", "bash -- -e {0}", "pwsh", "dash {0}",
            "sh {0}", "", "bash {0}", "Z {0} -e",
        ]:
            add("tmpl:" + text, lambda text=text: _template_errexit(text))
        for text, flag in [
            ("set -- -e", False), ("set -- +e", True), ("set -e", False),
            ("set +e", True), ("set -e -- +e", True), ("set +o -e", False),
            ("set +o errexit", True), ("true", True), ("set -Z", True),
        ]:
            add(
                "next:" + text,
                lambda text=text, flag=flag: _next_errexit(
                    text.split(), [False] * len(text.split()), flag
                ),
            )
        helpers = {
            "paren": lambda: _closing_paren("(')')", 0),
            "paren2": lambda: _closing_paren('("\\")")', 0),
            "tick": lambda: _closing_backtick("`a`", 0),
            "tick2": lambda: _closing_backtick("`\\``", 0),
            "words": lambda: _shell_words('""'),
            "words2": lambda: _shell_words("a*"),
            "words3": lambda: _shell_words("'a*'"),
            "words4": lambda: _shell_words("cp"),
            "words5": lambda: _shell_words("$(echo a b)"),
            "wordsZ": lambda: _shell_words("Z"),
            "glob": lambda: _glob_directories("docs/archiv?", ""),
            "glob2": lambda: _glob_directories("docs/arch*", ""),
            "ws": lambda: _resolve_run_path(
                "$GITHUB_WORKSPACE/docs/archive/x", "docs/book"
            ),
            "ws2": lambda: _resolve_run_path("${GITHUB_WORKSPACE}/docs/a", ""),
            "star": lambda: _resolve_run_path("docs/book/dashboard/*", ""),
            "cdpath": lambda: _cdpath_directory("archive", "", "docs"),
            "cdpath2": lambda: _cdpath_directory("nope", "", "docs"),
            "uses": lambda: _cd_uses_cdpath("archive", {"CDPATH": "docs"}),
            "uses2": lambda: _cd_uses_cdpath("/tmp", {"CDPATH": "docs"}),
            "uses3": lambda: _cd_uses_cdpath(".archive", {"CDPATH": "docs"}),
            "repo": lambda: _repo_dir("docs/archive"),
            "read": lambda: _read_workspace_file(
                "scripts/ci/testdata/docs_source_copy/payload.sh"
            ) is not None,
            "read2": lambda: _read_workspace_file("../secrets"),
            "edge": lambda: _word_edge("abc", 3),
            "edge2": lambda: _word_edge("abc", 1),
            "skip": lambda: _skip_horizontal_space("  a", 0),
            "alias": lambda: _alias_binding(["alias", "enter=cd"]),
            "alias2": lambda: _alias_binding(["alias", "1bad=cd"]),
        }
        for name, fn in helpers.items():
            add("h:" + name, fn)
        scripts = [
            (copy, ""),
            ("2>&1 " + copy, ""),
            ("&>/dev/null " + copy, ""),
            (">| /dev/null " + copy, ""),
            ("cd docs/archive 2>&1\n" + rel, ""),
            ("publish() { " + copy + "\n}\npublish", ""),
            ("function relax { set +e; }\nrelax\ncd nope\n" + rel, "docs/archive"),
            (
                "shopt -s expand_aliases\nalias enter='cd docs/archive'\nenter\n" + rel,
                "",
            ),
            (
                "cd docs\ncd archive\ncd -\n"
                "cp -r archive/benchmark_dashboard book/dashboard/benchmark_dashboard",
                "",
            ),
            ("CDPATH=docs\ncd archive\n" + rel, ""),
            ("trap '" + copy + "' EXIT\ntrue", ""),
            (". scripts/ci/testdata/docs_source_copy/payload.sh", ""),
            ('D=docs/archive\ncd "$D"\n' + rel, ""),
            ("P=docs/archiv?\ncd $P\n" + rel, ""),
            ("set +e\nset -- -e\ncd nope\n" + rel, "docs/archive"),
            ("set -- +e\ncd nope\n" + rel, "docs/archive"),
            ("shopt -uo errexit\ncd nope\n" + rel, "docs/archive"),
            ("eval 'set +e'\ncd nope\n" + rel, "docs/archive"),
            ("cd docs/arch*\ntrue", ""),
            (copy + "\ncd docs/arch*", ""),
            ("Z x " + copy, ""),
            ("cd -", ""),
        ]

        def state_sig(script: str, cwd: str):
            got_states = _shell_states(script, cwd)
            return tuple(
                (
                    state.cwd,
                    state.errexit,
                    state.stopped,
                    tuple(state.paths),
                    state.expand_aliases,
                    tuple(state.aliases),
                    state.exit_trap,
                    state.oldpwd,
                )
                for state in got_states
            )

        for script, cwd in scripts:
            label = script.replace("\n", "|")[:40]
            add("st:" + label, lambda script=script, cwd=cwd: state_sig(script, cwd))
            add(
                "cp:" + label,
                lambda script=script, cwd=cwd: bool(
                    _archive_copied_onto_book(script, cwd)
                ),
            )
        self.assertEqual(set(got), set(expected))
        for name, value in expected.items():
            self.assertEqual(got[name], value, name)

    def test_benchmark_alert_gh_pages_publish_paths(self) -> None:
        alert = (WORKFLOWS / "benchmark_alert.yml").read_text(encoding="utf-8")
        _call_checker(assert_benchmark_alert_gh_pages_publish, alert)
        honoured = alert.replace("publish_dir:", "Publish_Dir:", 1).replace(
            "destination_dir:", "Destination_Dir:", 1
        )
        _call_checker(assert_benchmark_alert_gh_pages_publish, honoured)
        for force_value in ("false", "'false'", "' false '"):
            safe_force = alert.replace(
                "destination_dir: dashboard/benchmark_data\n",
                "destination_dir: dashboard/benchmark_data\n"
                f"          force_orphan: {force_value}\n",
                1,
            )
            _call_checker(assert_benchmark_alert_gh_pages_publish, safe_force)
        empty_external = alert.replace(
            "destination_dir: dashboard/benchmark_data\n",
            "destination_dir: dashboard/benchmark_data\n"
            "          external_repository: ''\n",
            1,
        )
        _call_checker(assert_benchmark_alert_gh_pages_publish, empty_external)
        for directory in (
            "benchmark_alert_scalar_step",
            "benchmark_alert_two_publishers_one_name",
            "benchmark_alert_jamesives_allowed",
            "benchmark_alert_peaceiris_unused_folder",
        ):
            fixture = _call_checker(_read_policy_fixture, directory, "benchmark_alert.yml")
            _call_checker(assert_benchmark_alert_gh_pages_publish, fixture, path=directory)
        allowed = _call_checker(
            _read_policy_fixture, "benchmark_alert_jamesives_allowed", "benchmark_alert.yml"
        )
        for force_value in ("false", "'false'", "' FALSE '"):
            explicit = allowed.replace("force: false\n", f"force: {force_value}\n", 1)
            self.assertIn("force:", explicit)
            _call_checker(
                assert_benchmark_alert_gh_pages_publish, explicit, path="jamesives-force"
            )
        folded_force = allowed.replace("force: false\n", "Force: false\n", 1)
        _call_checker(
            assert_benchmark_alert_gh_pages_publish, folded_force, path="jamesives-force-case"
        )
        for addition in (
            "          single-commit: false\n",
            "          single-commit: ' false '\n",
            "          branch: gh-pages\n",
            "          branch: ''\n",
            "          repository-name: ''\n",
        ):
            varied = allowed.replace("force: false\n", "force: false\n" + addition, 1)
            self.assertIn(addition.strip(), varied)
            _call_checker(
                assert_benchmark_alert_gh_pages_publish, varied, path="jamesives-allowed-variant"
            )
        snippets = {
            "benchmark_alert_bad_pages_publish": "publish_dir",
            "benchmark_alert_second_publisher": "publish_dir",
            "benchmark_alert_force_push_input": "force-push",
            "benchmark_alert_case_variant_keys": "publish_dir",
            "benchmark_alert_case_variant_action": "publish_dir",
            "benchmark_alert_wrong_step_name": "missing step named",
            "benchmark_alert_missing_destination": "destination_dir",
            "benchmark_alert_root_destination": "destination_dir",
            "benchmark_alert_other_publish_dir": "publish_dir",
            "benchmark_alert_force_expression": "force-push",
            "benchmark_alert_force_padded_true": "force-push",
            "benchmark_alert_force_lowercase_true": "force-push",
            "benchmark_alert_empty_jobs": "missing jobs",
            "benchmark_alert_jobs_scalar": "missing jobs",
            "benchmark_alert_jamesives_publisher": "folder",
            "benchmark_alert_jamesives_decoy_paths": "folder",
            "benchmark_alert_jamesives_second_step": "folder",
            "benchmark_alert_jamesives_folder_suffix": "folder",
            "benchmark_alert_jamesives_nested_folder": "folder",
            "benchmark_alert_jamesives_target_suffix": "target-folder",
            "benchmark_alert_publish_branch_main": "publish_branch",
            "benchmark_alert_external_repository": "external_repository",
            "benchmark_alert_spaced_publish_dir": "publish_dir",
            "benchmark_alert_jamesives_single_commit": "single-commit",
            "benchmark_alert_jamesives_branch_main": "branch",
            "benchmark_alert_jamesives_repository_name": "repository-name",
            "benchmark_alert_jamesives_force_true": "force",
            "benchmark_alert_jamesives_force_omitted": "force",
            "benchmark_alert_jamesives_leading_slash": "folder",
        }
        for directory, snippet in snippets.items():
            fixture = _call_checker(_read_policy_fixture, directory, "benchmark_alert.yml")
            with self.assertRaises(AssertionError) as caught:
                _call_checker(
                    assert_benchmark_alert_gh_pages_publish, fixture, path=directory
                )
            self.assertIn(snippet, str(caught.exception), directory)
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_benchmark_alert_gh_pages_publish, "[]\n", path="list-root"
            )
        self.assertIn("not a mapping", str(caught.exception))
        whole_tree = """
name: Alert
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy dashboard to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          publish_dir: ./docs/
          destination_dir: dashboard/benchmark_data
          publish_branch: gh-pages
"""
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_benchmark_alert_gh_pages_publish, whole_tree, path="whole-tree"
            )
        self.assertIn("whole tree", str(caught.exception))
        bare = whole_tree.replace("publish_dir: ./docs/\n", "publish_dir: ./docs/benchmark_data\n")
        bare = bare.replace(
            "destination_dir: dashboard/benchmark_data\n",
            "destination_dir: dashboard\n",
        )
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_benchmark_alert_gh_pages_publish, bare, path="bare-dashboard"
            )
        self.assertIn("bare dashboard", str(caught.exception))
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_benchmark_alert_gh_pages_publish,
                "name: n\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps: []\n",
                path="no-publish",
            )
        self.assertIn(PEACEIRIS_GH_PAGES, str(caught.exception))
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_benchmark_alert_gh_pages_publish,
                "name: n\non: [push]\njobs:\n  build: []\n",
                path="job-list",
            )
        self.assertIn("not a mapping", str(caught.exception))

    def test_docs_no_archive_benchmark_dashboard_copy(self) -> None:
        docs = (WORKFLOWS / "docs.yml").read_text(encoding="utf-8")
        _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, docs)
        dashboard = (WORKFLOWS / "benchmark-dashboard.yml").read_text(encoding="utf-8")
        dashboard_data = _workflow_document(dashboard, path="benchmark-dashboard.yml")
        seen_prepare = False
        for job in _require_jobs(dashboard_data, path="benchmark-dashboard.yml").values():
            if not isinstance(job, dict):
                continue
            job_cwd = _job_cwd(dashboard_data, job)
            steps = job.get("steps")
            if not isinstance(steps, list):
                continue
            for step in steps:
                if not isinstance(step, dict):
                    continue
                run_text = _step_run_text(step)
                if not isinstance(run_text, str) or not run_text.strip():
                    continue
                paths = _run_paths(run_text, _step_cwd(step, job_cwd))
                self.assertFalse(
                    _archive_copied_onto_book(run_text, _step_cwd(step, job_cwd)),
                    step.get("name"),
                )
                if step.get("name") != "Prepare Dashboard Assets":
                    continue
                self.assertIn("tools/benchmark-dashboard/static-data", paths)
                self.assertIn("gh-pages/static-data", paths)
                self.assertIn("tools/benchmark-dashboard/graphs", paths)
                self.assertIn("gh-pages/graphs", paths)
                self.assertIn("target/benchmark-data/history.jsonl", paths)
                self.assertIn("gh-pages/benchmark-data", paths)
                self.assertIn(
                    "tools/benchmark-dashboard/benchmark-data/history.jsonl", paths
                )
                self.assertIn(
                    "tools/benchmark-dashboard/benchmark-data/metadata.json", paths
                )
                seen_prepare = True
        self.assertTrue(seen_prepare)
        for replacement in ('keep_files: "true"', 'keep_files: " TRUE "'):
            kept = docs.replace("keep_files: true", replacement, 1)
            _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, kept)
        for force_value in ("false", "'false'", "' false '"):
            safe_force = docs.replace(
                "keep_files: true\n",
                "keep_files: true\n          force_orphan: " + force_value + "\n",
                1,
            )
            self.assertIn("force_orphan", safe_force)
            _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, safe_force)
        for directory in (
            "docs_book_path_only",
            "docs_archive_to_other",
            "docs_list_split_harmless",
            "docs_bool_run",
            "docs_dash_flag_not_a_path",
            "docs_other_destination",
            "docs_echo_both_paths",
            "docs_quoted_separator",
            "docs_pages_publish_near_miss",
            "docs_cwd_not_workspace",
            "docs_copy_near_miss",
            "docs_other_quote_delimiter",
            "docs_jamesives_book",
            "docs_pipe_cd",
            "docs_pipe_assign",
            "docs_background_cd",
            "docs_brace_only",
            "docs_command_only",
            "docs_trailing_pipe",
            "docs_single_quoted_separator",
            "docs_command_popd",
            "docs_assign_plain",
            "docs_if_elif_dashboard_safe",
            "docs_unconditional_cd_safe",
            "docs_if_closed_assign_safe",
            "docs_reassign_unrecordable_safe",
            "docs_jamesives_alert_data_sibling",
            "docs_jamesives_target_expr",
            "docs_null_run",
            "docs_path_equals_cwd",
            "docs_trailing_slash_cwd",
            "docs_cd_glob_many",
            "docs_set_positional_safe",
            "docs_set_o_flag_safe",
            "docs_set_double_dash_safe",
        ):
            fixture = _call_checker(_read_policy_fixture, directory, "docs.yml")
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy, fixture, path=directory
            )
        book = _call_checker(
            _read_policy_fixture, "docs_jamesives_book", "docs.yml"
        )
        for addition in (
            "          branch: gh-pages\n",
            "          branch: ''\n",
            "          branch: ' gh-pages '\n",
            "          single-commit: false\n",
            "          single-commit: 'false'\n",
            "          single-commit: ' FALSE '\n",
        ):
            varied = book.replace("force: false\n", "force: false\n" + addition, 1)
            self.assertIn(addition.strip(), varied)
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                varied,
                path="jamesives-docs-branch",
            )
        for bad_commit in ("true", "'true'", "' TRUE '"):
            varied = book.replace(
                "force: false\n",
                f"force: false\n          single-commit: {bad_commit}\n",
                1,
            )
            with self.assertRaises(AssertionError) as caught:
                _call_checker(
                    assert_docs_no_archive_benchmark_dashboard_copy,
                    varied,
                    path="jamesives-docs-single-commit",
                )
            self.assertIn("single-commit", str(caught.exception))
        for force_value in ("false", "'false'", "' FALSE '"):
            varied = book.replace("force: false\n", f"force: {force_value}\n", 1)
            self.assertIn("force:", varied)
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                varied,
                path="jamesives-docs-force",
            )
        folded = book.replace("force: false\n", "Force: false\n", 1).replace(
            "target-folder: api\n", "Target-Folder: api\n", 1
        )
        _call_checker(
            assert_docs_no_archive_benchmark_dashboard_copy,
            folded,
            path="jamesives-docs-folded",
        )
        leading_api = book.replace("target-folder: api\n", "target-folder: /api\n", 1)
        _call_checker(
            assert_docs_no_archive_benchmark_dashboard_copy,
            leading_api,
            path="jamesives-docs-leading-api",
        )
        omitted_folder = """
name: Documentation
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to GitHub Pages
        uses: JamesIves/github-pages-deploy-action@v4
        with:
          target-folder: api
          force: false
"""
        _call_checker(
            assert_docs_no_archive_benchmark_dashboard_copy,
            omitted_folder,
            path="jamesives-omitted-folder",
        )
        omitted_target = omitted_folder.replace(
            "          target-folder: api\n",
            "          folder: docs/book\n",
        )
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                omitted_target,
                path="jamesives-omitted-target",
            )
        self.assertIn("worktree root", str(caught.exception))
        copy_dirs = (
            "docs_bad_benchmark_dashboard_copy",
            "docs_parent_directory_copy",
            "docs_parent_directory_copy_relative",
            "docs_cd_in_step",
            "docs_cd_under_step_cwd",
            "docs_job_default_cwd",
            "docs_workflow_default_cwd",
            "docs_job_cwd_overrides_workflow",
            "docs_step_overrides_job",
            "docs_double_slash",
            "docs_workspace_prefix",
            "docs_parent_dotdot",
            "docs_dot_slash_prefix",
            "docs_glob_destination",
            "docs_backslash_paths",
            "docs_archive_parent_star",
            "docs_archive_parent_dot",
            "docs_archive_parent_rsync",
            "docs_mv_archive_copy",
            "docs_bare_word_from_archive",
            "docs_bare_word_from_book",
            "docs_child_file_copy",
            "docs_list_run",
            "docs_defaults_without_workdir",
            "docs_blank_step_cwd",
            "docs_semicolon_copy",
            "docs_numeric_step_cwd",
            "docs_defaults_shell_only",
            "docs_quote_then_copy",
            "docs_single_quoted_paths",
            "docs_cwd_expr_workspace",
            "docs_cwd_env_workspace",
            "docs_cwd_expr_nospace",
            "docs_cwd_expr_case",
            "docs_job_cwd_expr_workspace",
            "docs_target_directory_copy",
            "docs_echo_literal_copy",
            "docs_pushd_copy",
            "docs_pushd_popd_copy",
            "docs_pushd_option",
            "docs_mv_target_directory",
            "docs_pipe_copy",
            "docs_or_copy",
            "docs_background_copy",
            "docs_bin_cp",
            "docs_command_cp",
            "docs_env_cp",
            "docs_echo_n_copy",
            "docs_target_short_copy",
            "docs_assigned_dest_copy",
            "docs_or_cd",
            "docs_or_assign",
            "docs_command_cd",
            "docs_brace_group_copy",
            "docs_brace_cd",
            "docs_or_tight",
            "docs_pipe_tight",
            "docs_and_tight",
            "docs_assign_one",
            "docs_assign_one_brace",
            "docs_if_elif_dashboard_copy",
            "docs_if_else_dashboard_copy",
            "docs_if_then_cd_copy",
            "docs_if_condition_copy",
            "docs_elif_condition_copy",
            "docs_fi_glued_copy",
            "docs_if_same_line_cd_mask",
            "docs_if_same_line_assign_mask",
            "docs_if_else_cd_mask",
            "docs_if_condition_cd_mask",
            "docs_if_multiline_assign_mask",
            "docs_if_multiline_cd_mask",
            "docs_quoted_fi_mask",
            "docs_quoted_fi_single_mask",
            "docs_cd_or_true_mask",
            "docs_cd_or_copy_mask",
            "docs_pushd_or_true_mask",
            "docs_cd_and_true_mask",
            "docs_errexit_off_cd_mask",
            "docs_line_continue_copy",
            "docs_cd_glob_copy",
            "docs_leading_redirect_copy",
            "docs_and_cd_rhs",
            "docs_or_cd_rhs",
            "docs_or_assign_rhs",
            "docs_and_assign_rhs",
            "docs_set_quoted_e",
            "docs_set_quoted_o",
            "docs_set_assign_prefix",
            "docs_set_builtin",
            "docs_set_dead_branch",
            "docs_cd_quoted_glob",
            "docs_cd_partial_quote_glob",
            "docs_shell_bash_no_errexit",
            "docs_echo_unquoted",
            "docs_echo_backtick",
            "docs_quoted_command_cp",
            "docs_two_redirects",
            "docs_absolute_cd_copy",
            "docs_builtin_cd_copy",
            "docs_cd_glob_after_copy",
            "docs_pushd_glob_after_copy",
            "docs_set_double_dash_e",
            "docs_eval_quoted_set",
            "docs_eval_quoted_cd",
            "docs_shell_script_arg",
            "docs_workspace_root_copy",
            "docs_workspace_root_cd",
            "docs_shopt_o_errexit",
            "docs_shopt_uo_errexit",
            "docs_redirect_fd_copy",
            "docs_redirect_cd_copy",
            "docs_redirect_assign_copy",
            "docs_function_copy",
            "docs_function_cd",
            "docs_function_set",
            "docs_alias_cd",
            "docs_cd_previous",
            "docs_cdpath_cd",
            "docs_trap_exit_copy",
            "docs_source_copy",
            "docs_quoted_var_cd",
            "docs_glob_var_cd",
            "docs_if_assign_archive",
            "docs_if_keeps_archive",
            "docs_extra_fi_copy",
            "docs_nested_fi_assign",
            "docs_previous_directory_state",
            "docs_oldpwd_assignment",
            "docs_function_arguments",
            "docs_eval_unquoted",
            "docs_eval_stored_word",
            "docs_eval_function",
            "docs_trap_option_end",
            "docs_trap_signal_words",
            "docs_set_positionals",
            "docs_cdpath_workspace_prefix",
            "docs_stored_directory_word",
            "docs_stored_directory_suffix",
            "docs_stored_directory_chain",
            "docs_stored_workspace_word",
            "docs_cdpath_brace_prefix",
            "docs_cdpath_mixed_prefix",
            "docs_oldpwd_parameter",
            "docs_oldpwd_workspace_prefix",
            "docs_function_argument_suffix",
            "docs_function_argument_directory",
            "docs_set_positional_suffix",
            "docs_trap_option_signals",
            "docs_eval_unquoted_stored",
            "docs_sourced_function",
        )
        for directory in copy_dirs:
            fixture = _call_checker(_read_policy_fixture, directory, "docs.yml")
            with self.assertRaises(AssertionError) as caught:
                _call_checker(
                    assert_docs_no_archive_benchmark_dashboard_copy,
                    fixture,
                    path=directory,
                )
            self.assertIn(ARCHIVE_BENCHMARK_DASHBOARD, str(caught.exception), directory)
        for directory, snippet in (
            ("docs_dashboard_destination", "destination_dir"),
            ("docs_keep_files_false", "keep_files"),
            ("docs_force_orphan_true", "force_orphan"),
            ("docs_force_orphan_expression", "force_orphan"),
            ("docs_archive_publish_dir", "publish_dir"),
            ("docs_archive_publish_file", "publish_dir"),
            ("docs_live_dashboard_destination", "destination_dir"),
            ("docs_live_dashboard_child", "destination_dir"),
            ("docs_destination_expression", "destination_dir"),
            ("docs_destination_concat", "destination_dir"),
            ("docs_destination_prefix", "destination_dir"),
            ("docs_destination_suffix", "destination_dir"),
            ("docs_destination_between", "destination_dir"),
            ("docs_jamesives_archive_folder", "folder"),
            ("docs_jamesives_dashboard_target", "target-folder"),
            ("docs_jamesives_omitted_target", "worktree root"),
            ("docs_jamesives_root_dot", "worktree root"),
            ("docs_jamesives_root_empty", "worktree root"),
            ("docs_jamesives_root_slash", "worktree root"),
            ("docs_jamesives_root_dotdot", "worktree root"),
            ("docs_jamesives_root_blank", "worktree root"),
            ("docs_jamesives_alert_data", "benchmark_data"),
            ("docs_jamesives_alert_data_child", "benchmark_data"),
            ("docs_jamesives_alert_data_clean_false", "benchmark_data"),
            ("docs_jamesives_alert_data_expr", "benchmark_data"),
            ("docs_jamesives_alert_leading_slash", "benchmark_data"),
            ("docs_jamesives_branch_main", "branch"),
            ("docs_jamesives_force_omitted", "force"),
            ("docs_jamesives_force_true", "force"),
            ("docs_jamesives_single_commit", "single-commit"),
        ):
            fixture = _call_checker(_read_policy_fixture, directory, "docs.yml")
            with self.assertRaises(AssertionError) as caught:
                _call_checker(
                    assert_docs_no_archive_benchmark_dashboard_copy,
                    fixture,
                    path=directory,
                )
            self.assertIn(snippet, str(caught.exception), directory)
        false_keep = docs.replace("keep_files: true", "keep_files: false", 1)
        with self.assertRaises(AssertionError) as caught:
            _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, false_keep)
        self.assertIn("keep_files", str(caught.exception))
        string_false = docs.replace("keep_files: true", 'keep_files: "false"', 1)
        with self.assertRaises(AssertionError) as caught:
            _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, string_false)
        self.assertIn("keep_files", str(caught.exception))
        listed_with = """
name: Documentation
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with: []
"""
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                listed_with,
                path="with-list",
            )
        self.assertIn("missing with", str(caught.exception))
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                "name: n\non: [push]\njobs:\n  build: []\n",
                path="job-list",
            )
        self.assertIn("not a mapping", str(caught.exception))
        with self.assertRaises(AssertionError) as caught:
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                "name: n\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
                path="no-steps",
            )
        self.assertIn("missing steps", str(caught.exception))

    def _assert_pages_form(self, directory: str) -> None:
        """The form writes under bash -e, and the checker records that copy."""
        text = _read_policy_fixture(directory, "docs.yml")
        data = yaml.safe_load(text)
        step = data["jobs"]["build-and-deploy"]["steps"][0]
        script = step["run"]
        cwd = _normalize_cwd(step.get("working-directory"))
        self.assertIs(_archive_copied_onto_book(script, cwd), True, directory)
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            archive = root / "docs" / "archive" / "benchmark_dashboard"
            archive.mkdir(parents=True)
            (archive / "index.html").write_text("archive-marker\n", encoding="utf-8")
            (root / "docs" / "book" / "dashboard").mkdir(parents=True)
            fixture = ROOT / "scripts" / "ci" / "testdata" / directory
            for path in fixture.rglob("*"):
                if not path.is_file() or path.name == "docs.yml":
                    continue
                dest = root / path.relative_to(ROOT)
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(path.read_bytes())
            env = os.environ.copy()
            env["GITHUB_WORKSPACE"] = str(root)
            env.pop("CDPATH", None)
            env.pop("OLDPWD", None)
            proc = subprocess.run(
                ["bash", "-e", "-c", script],
                cwd=root / cwd if cwd else root,
                env=env,
                capture_output=True,
                text=True,
                timeout=10,
                check=False,
            )
            target = (
                root / "docs" / "book" / "dashboard" / "benchmark_dashboard" / "index.html"
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertEqual(
                target.read_text(encoding="utf-8"), "archive-marker\n", directory
            )

    def test_previous_directory_state_writes(self) -> None:
        self._assert_pages_form("docs_previous_directory_state")

    def test_oldpwd_assignment_writes(self) -> None:
        self._assert_pages_form("docs_oldpwd_assignment")

    def test_function_arguments_write(self) -> None:
        self._assert_pages_form("docs_function_arguments")

    def test_eval_unquoted_words_write(self) -> None:
        self._assert_pages_form("docs_eval_unquoted")

    def test_eval_stored_word_writes(self) -> None:
        self._assert_pages_form("docs_eval_stored_word")

    def test_eval_function_definition_writes(self) -> None:
        self._assert_pages_form("docs_eval_function")

    def test_trap_option_end_writes(self) -> None:
        self._assert_pages_form("docs_trap_option_end")

    def test_trap_signal_words_write(self) -> None:
        self._assert_pages_form("docs_trap_signal_words")

    def test_set_positionals_write(self) -> None:
        self._assert_pages_form("docs_set_positionals")

    def test_cdpath_workspace_prefix_writes(self) -> None:
        self._assert_pages_form("docs_cdpath_workspace_prefix")

    def test_stored_directory_word_writes(self) -> None:
        self._assert_pages_form("docs_stored_directory_word")

    def test_directory_parameter_forms_write(self) -> None:
        for directory in (
            "docs_stored_directory_suffix",
            "docs_stored_directory_chain",
            "docs_stored_workspace_word",
            "docs_cdpath_brace_prefix",
            "docs_cdpath_mixed_prefix",
            "docs_oldpwd_parameter",
            "docs_oldpwd_workspace_prefix",
            "docs_function_argument_suffix",
            "docs_function_argument_directory",
            "docs_set_positional_suffix",
            "docs_trap_option_signals",
            "docs_eval_unquoted_stored",
            "docs_sourced_function",
        ):
            with self.subTest(directory=directory):
                self._assert_pages_form(directory)

    def test_scan_loops_terminate(self) -> None:
        holder: dict[str, object] = {}

        def run() -> None:
            holder["short"] = _strip_trailing_redirects(["cp", "a"], [False], [None])
            holder["long"] = _strip_trailing_redirects(
                ["cp", "a"],
                [False, False, False, False],
                [None, None, None, None],
            )
            holder["expr"] = _reduce_literal_expressions("${{ 'lit' }}")
            holder["fn"] = _extract_functions("publish() { cp a b; }\npublish")

        worker = threading.Thread(target=run, daemon=True)
        worker.start()
        worker.join(2.0)
        self.assertFalse(worker.is_alive())
        short_words, short_quoted, short_globs = holder["short"]
        self.assertEqual(len(short_quoted), len(short_words))
        self.assertEqual(len(short_globs), len(short_words))
        long_words, long_quoted, _long_globs = holder["long"]
        self.assertEqual(long_words, ["cp", "a"])
        self.assertEqual(len(long_quoted), 4)
        self.assertEqual(holder["expr"], "lit")
        _text, functions = holder["fn"]
        self.assertIn("publish", functions)

    def test_parameter_expansion_and_option_prefixes(self) -> None:
        """Pin expansion, option prefixes, and trap or source words the suite reads."""
        self.assertEqual(_expand_parameters("$(echo a)", {}, ()), "$(echo a)")
        self.assertEqual(_expand_parameters("`echo a`", {}, ()), "`echo a`")
        self.assertEqual(
            _expand_parameters("$(echo $B)", {"B": "docs"}, ()), "$(echo $B)"
        )
        self.assertEqual(
            _expand_parameters("`echo $B`", {"B": "docs"}, ()), "`echo $B`"
        )
        self.assertEqual(_expand_parameters("$10", {}, ("docs",)), "docs0")
        self.assertEqual(
            _expand_parameters("${10}", {}, tuple(f"p{n}" for n in range(1, 11))),
            "p10",
        )
        self.assertEqual(
            _expand_parameters("$A/archive", {"A": "docs"}, ()), "docs/archive"
        )
        self.assertEqual(
            _expand_parameters("${A}x", {"A": "docs"}, ()), "docsx"
        )
        self.assertEqual(
            _expand_parameters("$A", {"A": "$B", "B": "docs"}, ()), "docs"
        )
        self.assertEqual(
            _expand_parameters("${A}", {"A": "${B}", "B": "docs"}, ()), "docs"
        )
        self.assertEqual(
            _expand_parameters("$1", {}, ("$2", "docs")), "docs"
        )
        self.assertEqual(
            _expand_parameters("$A", {"A": "x"}, (), 8), "x"
        )
        self.assertEqual(
            _expand_parameters("$A", {"A": "x"}, (), 9), "$A"
        )
        self.assertIsNone(_lookup_parameter("0", {}, ("a",)))
        self.assertEqual(_lookup_parameter("2", {}, ("a",)), None)
        self.assertEqual(_lookup_parameter("GITHUB_WORKSPACE", {}, ()), "")
        self.assertEqual(
            _lookup_parameter("GITHUB_WORKSPACE", {"GITHUB_WORKSPACE": "kept"}, ()),
            "kept",
        )
        self.assertEqual(
            _expand_parameters("$GITHUB_WORKSPACE/docs", {}, ()), "/docs"
        )
        self.assertEqual(
            _expand_assignment_value("missing:$GITHUB_WORKSPACE/docs", {}, ()),
            "missing:/docs",
        )
        self.assertIsNone(_match_function("xpublish() { }", 1))
        self.assertEqual(
            _matching_brace('{a"\\"}"b}', 0),
            len('{a"\\"}"b}') - 1,
        )
        copy = (
            "cp -r docs/archive/benchmark_dashboard "
            "docs/book/dashboard/benchmark_dashboard"
        )
        self.assertIs(
            _archive_copied_onto_book("source scripts/ci/testdata/docs_source_copy/payload.sh", ""),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("command set +e\ncd nope\n" + copy, ""),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("builtin set +e\ncd nope\n" + copy, ""),
            True,
        )
        self.assertIs(
            _archive_copied_onto_book("eval set +e\ncd nope\n" + copy, ""),
            True,
        )
        self.assertIs(_archive_copied_onto_book("trap '" + copy + "'\ntrue", ""), False)
        self.assertIs(
            _archive_copied_onto_book("cd $GITHUB_WORKSPACE/docs/archive\n" + "cp -r benchmark_dashboard ../book/dashboard/benchmark_dashboard", ""),
            True,
        )
        states = _shell_states("set -- one two\ntrue", "")
        self.assertEqual(states[0].positionals, ("one", "two"))
        for prefix in ("command", "builtin", "eval"):
            prefixed = _shell_states(f"{prefix} set -- one\ntrue", "")
            self.assertEqual(prefixed[0].positionals, ("one",), prefix)
        again = _shell_states(
            "OLDPWD=docs/archive\ncd docs/book\nif false; then OLDPWD=docs/archive; fi\ncd -\ntrue",
            "",
        )
        self.assertEqual(
            {item.cwd for item in again}, {"docs/book", "docs/book/docs/archive"}
        )

    def test_shell_state_cap_boundary(self) -> None:
        under = "\n".join(f"if false; then cd d{index}; fi" for index in range(5))
        states = _shell_states(under, "")
        self.assertEqual(len(states), 32)
        over = under + "\nif false; then cd /cap-extra; fi"
        with self.assertRaises(AssertionError) as caught:
            _shell_states(over, "")
        self.assertIn("shell state cap exceeded: 64", str(caught.exception))
        held = [_ShellState(f"c{index}", (), (), [], True) for index in range(32)]
        self.assertEqual(len(_dedupe_states(held)), 32)
        held.append(_ShellState("c32", (), (), [], True))
        with self.assertRaises(AssertionError) as extra:
            _dedupe_states(held)
        self.assertIn("shell state cap exceeded: 33", str(extra.exception))
        distinguished = _dedupe_states(
            [
                _ShellState("docs/book", (), (), [], True, oldpwd=""),
                _ShellState("docs/book", (), (), [], True, oldpwd="docs/archive"),
            ]
        )
        self.assertEqual(len(distinguished), 2)
        self.assertEqual({item.oldpwd for item in distinguished}, {"", "docs/archive"})

    def test_shell_state_cap_bounds_conditional_directories(self) -> None:
        lines = [f"if false; then cd d{index}; fi" for index in range(20)]
        lines.append(
            "cp -r docs/archive/benchmark_dashboard docs/book/dashboard/benchmark_dashboard"
        )
        script = "\n".join(lines)
        probe = (
            "import importlib.util\n"
            f"spec = importlib.util.spec_from_file_location('policy_under_test', {__file__!r})\n"
            "module = importlib.util.module_from_spec(spec)\n"
            "spec.loader.exec_module(module)\n"
            f"script = {script!r}\n"
            "try:\n"
            "    module._shell_states(script, '')\n"
            "except AssertionError as exc:\n"
            "    print(exc)\n"
            "else:\n"
            "    raise SystemExit('no raise')\n"
        )
        try:
            proc = subprocess.run(
                [sys.executable, "-c", probe],
                cwd=ROOT,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        except subprocess.TimeoutExpired:
            self.fail("shell state scan did not finish")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        message = proc.stdout.strip()
        self.assertIn("shell state cap exceeded:", message)
        count = int(message.rsplit(":", 1)[-1])
        self.assertEqual(count, 64)

    def test_identical_conditional_directories_stay_deduped(self) -> None:
        lines = ["if false; then cd /same; fi" for _ in range(12)]
        lines.append("cp other dest")
        script = "\n".join(lines)
        started = time.perf_counter()
        states = _shell_states(script, "")
        elapsed = time.perf_counter() - started
        self.assertLess(elapsed, 1.0)
        self.assertLessEqual(len(states), 2)
        self.assertFalse(_archive_copied_onto_book(script, ""))

    def test_literal_expression_scan_terminates(self) -> None:
        holder: dict[str, object] = {}

        def run() -> None:
            holder["value"] = _reduce_literal_expressions("${{ 'lit' }}")
            holder["wrapped"] = _reduce_literal_expressions("pre${{ 'mid' }}post")
            holder["between"] = _reduce_literal_expressions("${{ 'a' }}-${{ 'b' }}")

        worker = threading.Thread(target=run, daemon=True)
        worker.start()
        worker.join(2.0)
        self.assertFalse(worker.is_alive())
        self.assertEqual(holder.get("value"), "lit")
        self.assertEqual(holder.get("wrapped"), "premidpost")
        self.assertEqual(holder.get("between"), "a-b")

    def test_call_checker_reports_a_crash(self) -> None:
        def crash() -> None:
            raise RuntimeError("crash")

        with self.assertRaises(AssertionError) as caught:
            _call_checker(crash)
        self.assertIn("RuntimeError", str(caught.exception))


if __name__ == "__main__":
    unittest.main()

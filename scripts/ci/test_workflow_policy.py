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
_ECHO_METACHARACTERS = frozenset("$`;&|<>(){}\\")
_TARGET_DIRECTORY_PREFIXES = ("--target-directory=", "--target=")
_ASSIGNMENT_WORD = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$")
_JAMESIVES_DEPLOY = "github-pages-deploy-action"
_JAMESIVES_FOLDER = "docs/benchmark_data"
_JAMESIVES_TARGET = "dashboard/benchmark_data"
_LIVE_DASHBOARD_DEST = "dashboard/benchmark_dashboard"


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
            return stripped[len(prefix) :]
    return stripped


def _unwrap_echo_literal(word: str) -> str:
    """Reduce echo of one literal path. Leave every other substitution unchanged."""
    match = _ECHO_LITERAL.fullmatch(word)
    if match is None:
        return word
    inner = match.group(1).strip()
    if len(inner) >= 2 and inner[0] in "'\"" and inner[-1] == inner[0]:
        inner = inner[1:-1]
    if inner == "" or any(char.isspace() for char in inner):
        return word
    if any(char in _ECHO_METACHARACTERS for char in inner):
        return word
    return inner


def _resolve_run_path(token: str, cwd: str) -> str:
    """Resolve one shell word against cwd the way a workspace-relative shell would."""
    token = _unwrap_echo_literal(token.strip())
    token = _strip_workspace(token.strip())
    if token.endswith("/*"):
        token = token[:-2]
    slash_token = token.replace("\\", "/").strip("/")
    cwd_norm = _posix_norm(cwd)
    alone = _posix_norm(slash_token)
    if not cwd_norm or alone == cwd_norm or alone.startswith(cwd_norm + "/"):
        return alone
    return _posix_norm(f"{cwd_norm}/{slash_token}")


def _shell_words(command: str) -> list[str]:
    words: list[str] = []
    buf: list[str] = []
    quote: str | None = None
    for char in command:
        if quote is not None:
            if char == quote:
                quote = None
            else:
                buf.append(char)
            continue
        if char in ("'", '"'):
            quote = char
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


def _split_shell_commands(run_text: str) -> list[tuple[str, bool]]:
    """Split on newline, `;`, `&&`, `||`, `|`, and a single `&`.

    The bool is true when a directory change in that command stays in effect
    for the next command. Newline, `;`, `&&`, and `||` carry it. A single `|`
    and a single `&` do not. `&&` and `||` are one operator even when the next
    command is flush against them. The scan raises if the index does not advance.
    """
    commands: list[tuple[str, bool]] = []
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
        if char in ("|", "&"):
            doubled = index + 1 < len(run_text) and run_text[index + 1] == char
            # && and || run in the current shell. A single | is a pipeline
            # and a single & backgrounds the command. One advance consumes the
            # operator, so the next character stays in the following command
            # when there is no space.
            carry = doubled
            index += 2 if doubled else 1
            commands.append(("".join(current), carry))
            current = []
            continue
        if char in ("\n", ";"):
            commands.append(("".join(current), True))
            current = []
            index += 1
            continue
        current.append(char)
        index += 1
    commands.append(("".join(current), True))
    return [(command.strip(), carry) for command, carry in commands if command.strip()]


def _command_body(words: list[str]) -> list[str]:
    """Drop leading `{` group tokens and a leading `command` word.

    `command` followed by an option stays in place. Brace expansion is one
    word and stays in place. A parenthesized subshell is not removed. The
    scan raises if the index does not advance.
    """
    index = 0
    limit = len(words)
    previous = -1
    while index < limit:
        if index <= previous:
            raise AssertionError("command scan did not advance")
        previous = index
        if words[index] == "{":
            index += 1
            continue
        if (
            words[index] == "command"
            and index + 1 < limit
            and not words[index + 1].startswith("-")
        ):
            index += 1
            continue
        break
    return words[index:]


def _directory_update(words: list[str], current: str, stack: list[str]) -> str | None:
    """Return the cwd after cd, pushd, or popd. None when the command is not one of those.

    The directory is the second word. An option in that word is not a directory.
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


def _command_basename(word: str) -> str:
    return word.replace("\\", "/").rsplit("/", 1)[-1]


def _recordable_assignment(value: str) -> bool:
    return "$" not in value and "`" not in value


def _expand_assigned_word(word: str, env: dict[str, str]) -> str:
    """Replace a whole-word $name or ${name}, including a one-character name.

    Leave every other word unchanged.
    """
    if (
        word.startswith("${")
        and word.endswith("}")
        and len(word) >= 4
        and word[2:-1].isidentifier()
    ):
        return env.get(word[2:-1], word)
    if word.startswith("$") and len(word) >= 2 and word[1:].isidentifier():
        return env.get(word[1:], word)
    return word


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
    word is skipped before cd, pushd, popd, and a copy, and so is a leading
    { group token. command followed by an option is left in place. A leading
    NAME=value is skipped. An assignment-only command stores a value with no
    $ or backtick when the separator keeps shell state. A later copy word that
    is $name or ${name} uses that value, including a one-character name.
    The assignment scan raises if its index does not advance.
    """
    paths: list[str] = []
    current = _posix_norm(cwd)
    stack: list[str] = []
    env: dict[str, str] = {}
    for command, carry in _split_shell_commands(run_text):
        saved_cwd = current
        saved_stack = list(stack)
        words = _command_body(_shell_words(command))
        if not words:
            continue
        assignments: list[tuple[str, str]] = []
        index = 0
        previous = -1
        while index < len(words):
            if index <= previous:
                raise AssertionError("assignment scan did not advance")
            previous = index
            match = _ASSIGNMENT_WORD.match(words[index])
            if match is None:
                break
            assignments.append((match.group(1), match.group(2)))
            index += 1
        rest = words[index:]
        if not rest:
            if carry:
                for name, value in assignments:
                    if _recordable_assignment(value):
                        env[name] = value
                    else:
                        env.pop(name, None)
            continue
        updated = _directory_update(rest, current, stack)
        if updated is not None:
            current = updated
        else:
            copy_words = rest
            command_name = _command_basename(copy_words[0])
            if command_name in _COPY_COMMANDS:
                for word in copy_words[1:]:
                    expanded = _expand_assigned_word(word, env)
                    path_word = _path_from_copy_word(command_name, expanded)
                    if path_word is None:
                        continue
                    paths.append(_resolve_run_path(path_word, current))
        if not carry:
            current = saved_cwd
            stack[:] = saved_stack
    return paths


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
    """Unset and false are the only allowed force_orphan values."""
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
    """
    if "${{" not in value:
        return value
    parts: list[str] = []
    index = 0
    while True:
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
        index = end + 2


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
    """folder, target-folder, branch, and force are what this action deploys.

    target-folder must name a directory other than the worktree root, the
    published dashboard, and the alert data directory. branch must be unset,
    blank, or gh-pages. force must be an explicit false. Other inputs on a
    documentation step are not read.
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
    does not match. single-commit must be a known false. branch must be unset,
    blank, or gh-pages. repository-name must be unset or the empty string. A
    value that is only space does not pass. force must be
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
    does not match. single-commit must be a known false. branch must be unset,
    blank, or gh-pages. repository-name must be unset or the empty string. A
    value that is only space does not pass. force must be
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
    target-folder, branch, and force. folder and target-folder use the same
    normalization as publish_dir, including a dropped leading slash. folder
    must stay off docs/archive/benchmark_dashboard and files under it.
    Ancestors are not that check. A missing folder does not raise.
    target-folder must be present. A missing value, a blank value, and a path
    that normalizes to empty are the worktree root, and that root raises.
    ".", "/", and dashboard/benchmark_dashboard/../.. are that root.
    target-folder must not be dashboard or dashboard/benchmark_dashboard, or a
    path under that directory, and must not be dashboard/benchmark_data or a
    path under it. branch must be unset, blank, or gh-pages. force must be
    YAML false or the false string. Unset force is not false. Other inputs on
    that step are not read.

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
    prefixes are stripped before that directory is used. cd, pushd, and popd
    use the second word. An option in that word is not a directory. A directory
    change before &&, ||, a newline, or ; applies to later commands in the
    step, including when the next command is flush against && or ||. A
    directory change before a single | or a single & does not. The scan
    raises if its index does not advance. The
    copy command is cp, mv, or rsync, matched by basename. A backslash in that
    word is a separator. A leading command word is skipped before cd, pushd,
    popd, and a copy. A leading { group token is skipped the same way. That
    scan raises if its index does not advance. command
    followed by an option is left in place. A leading NAME=value is skipped.
    An assignment-only command stores a value with no $ or backtick when the
    separator keeps shell state, and a later copy word that is $name or
    ${name} uses it, including a one-character name. cp and mv contribute a
    --target-directory= or --target= value. echo, or echo -n, of one literal
    path is reduced. A trailing /* is removed. The check reads one step at a
    time.

    Untested classes: a copy split across multiple steps; a copy performed by a
    uses: action; a command word other than cp, mv, or rsync; the word command
    followed by an option; brace expansion; variable command words; escape
    splitting; a |, &, or ; inside a command substitution;
    actions/deploy-pages; actions/upload-pages-artifact; a destination_dir,
    publish_dir, folder, or target-folder expression that is not a quoted
    literal; a command substitution other than echo or echo -n of one literal
    path; a cd whose directory is not the second word, including cd -- and
    cd -P; a pushd or popd directory that is not the second word; a glued -t
    destination; a copy word containing *, ?, or [ other than a trailing /*;
    a working-directory expression other than github.workspace; any input other
    than folder, target-folder, branch, and force on a documentation step
    whose uses contains github-pages-deploy-action; a folder or target-folder
    that begins with a backslash; a parenthesized subshell.
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
            if not isinstance(run_text, str):
                raise AssertionError(f"{path}: step run text is not a string")
            if not run_text.strip():
                continue
            paths = _run_paths(run_text, _step_cwd(step, job_cwd))
            if any(_is_archive_path(item) for item in paths) and any(
                _is_book_dashboard_target(item) for item in paths
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
            "docs/archive/benchmark_dashboard",
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
        self.assertEqual(_split_shell_commands("  cp a b  "), [("cp a b", True)])
        self.assertEqual(
            _split_shell_commands("cp a b;   ; cp c d"),
            [("cp a b", True), ("cp c d", True)],
        )
        self.assertEqual(_run_paths("command", ""), [])
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

    def test_docs_no_archive_benchmark_dashboard_copy(self) -> None:
        docs = (WORKFLOWS / "docs.yml").read_text(encoding="utf-8")
        _call_checker(assert_docs_no_archive_benchmark_dashboard_copy, docs)
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
        ):
            varied = book.replace("force: false\n", "force: false\n" + addition, 1)
            self.assertIn(addition.strip(), varied)
            _call_checker(
                assert_docs_no_archive_benchmark_dashboard_copy,
                varied,
                path="jamesives-docs-branch",
            )
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
            "docs_trailing_slash_cwd",
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
            "docs_path_equals_cwd",
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


if __name__ == "__main__":
    unittest.main()

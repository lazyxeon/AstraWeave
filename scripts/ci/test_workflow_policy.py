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


# Plain false, False, and FALSE are the only continue-on-error values
# that are not violations. Quoted spellings are strings, which actionlint
# rejects. Words such as no/off, numbers, empty values, block scalars, and
# expressions (including ${{ false }}) are violations. The key match is
# case-sensitive.
_PLAIN_FALSE = frozenset({"false", "False", "FALSE"})
_ACCEPTED_SCALAR_STYLES = frozenset({None})
_FORBIDDEN_CONTINUE_EXAMPLES = (
    "true",
    '"true"',
    "'true'",
    "yes",
    "on",
    "1",
    "no",
    "off",
    "0",
    '""',
    "${{ false }}",
    "${{ matrix.experimental || false }}",
    '"false"',
    "'false'",
    '"False"',
    "'False'",
    '"FALSE"',
    "'FALSE'",
)
_ALLOWED_CONTINUE_EXAMPLES = (
    "false",
    "False",
    "FALSE",
)


class _YamlScalar(str):
    """Scalar text plus the YAML style that produced it."""

    __slots__ = ("style", "explicit_tag")

    def __new__(
        cls,
        text: str,
        style: str | None,
        explicit_tag: bool = False,
    ) -> "_YamlScalar":
        obj = str.__new__(cls, text)
        obj.style = style
        obj.explicit_tag = explicit_tag
        return obj


class _WorkflowLoader(yaml.SafeLoader):
    """Safe loader that keeps each scalar's spelling and quote style.

    Implicit resolvers are off so `on`, `yes`, `no`, and `off` stay words.
    An explicit tag is recorded on the scalar. GitHub Actions rejects YAML
    merge keys and duplicate keys, so this loader does not expand a merge
    key and does not keep both copies of a duplicate key. A file GitHub
    rejects cannot run.
    """

    def compose_scalar_node(self, anchor):  # type: ignore[no-untyped-def]
        event = self.get_event()
        tag = event.tag
        # `!` is the non-specific tag, and `!<!> ` is that tag written
        # verbatim. Both are explicit. The checker fails closed on them.
        explicit = tag is not None
        if tag is None:
            tag = self.resolve(yaml.ScalarNode, event.value, event.implicit)
        elif tag == "!":
            tag = "tag:yaml.org,2002:str"
        node = yaml.ScalarNode(
            tag,
            event.value,
            event.start_mark,
            event.end_mark,
            style=event.style,
        )
        node.explicit_tag = explicit
        if anchor is not None:
            self.anchors[anchor] = node
        return node


# `add_constructor` copies the class dict when the name is not already
# in `cls.__dict__`. Assigning a copy is the same map the method would copy.
_WorkflowLoader.yaml_constructors = dict(yaml.SafeLoader.yaml_constructors)
_WorkflowLoader.yaml_implicit_resolvers = {}


def _construct_kept_scalar(loader: yaml.SafeLoader, node: yaml.Node) -> _YamlScalar:
    text = loader.construct_scalar(node)
    # `construct_scalar` returns a str. The fallback is the same value.
    if not isinstance(text, str):
        text = str(text)
    return _YamlScalar(text, node.style, explicit_tag=bool(getattr(node, "explicit_tag", False)))


_WorkflowLoader.add_constructor("tag:yaml.org,2002:str", _construct_kept_scalar)


def _load_workflow(text: str) -> object:
    loaded = yaml.load(text, Loader=_WorkflowLoader)  # noqa: S506
    return loaded


# Only the repo-root checkout and build trees. A tracked action under
# `tools/target` or `tools/node_modules` is still a workflow step.
_PRUNE_ROOT_DIRS = frozenset({".git", "target"})


def _tracked_action_paths(root: Path) -> list[Path]:
    """Tracked `action.yml` and `action.yaml` files, from `git ls-files`."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=False,
        capture_output=True,
    )
    if listed.returncode != 0:
        return []
    found: list[Path] = []
    for raw in listed.stdout.split(b"\0"):
        if not raw:
            continue
        rel = raw.decode("utf-8", "surrogateescape").replace("\\", "/")
        name = rel.rsplit("/", 1)[-1]
        if name not in ("action.yml", "action.yaml"):
            continue
        if rel.split("/", 1)[0] in _PRUNE_ROOT_DIRS:
            continue
        found.append(root / rel)
    return found


def policy_yaml_paths(root: Path | None = None) -> list[Path]:
    """Workflow files and composite actions the step check reads.

    Workflows are the files in `.github/workflows`. Action files are the
    tracked `action.yml` and `action.yaml` names from `git ls-files`.
    A path whose first component is `target` or `.git` is not scanned.
    """
    root = ROOT if root is None else root
    workflows = root / ".github" / "workflows"
    paths: list[Path] = []
    if workflows.is_dir():
        paths.extend(workflows.glob("*.yml"))
        paths.extend(workflows.glob("*.yaml"))
    paths.extend(_tracked_action_paths(root))
    return sorted(set(paths))


def _track_tree(root: Path) -> None:
    """Stage every file under `root`, including gitignored paths."""
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(["git", "add", "-A", "-f"], cwd=root, check=True)


def _append_step_list(found: list[tuple[str, object]], owner: str, steps: object) -> None:
    if not isinstance(steps, list):
        raise ValueError(f"{owner} steps is not a list")
    for step in steps:
        found.append((owner, step))


def _iter_step_mappings(doc: object) -> list[tuple[str, object]]:
    """Steps from jobs.*.steps and from a composite action's runs.steps."""
    if not isinstance(doc, dict):
        return []
    found: list[tuple[str, object]] = []
    jobs = doc.get("jobs")
    if isinstance(jobs, dict):
        for job_id, job in jobs.items():
            if not isinstance(job, dict) or "steps" not in job:
                continue
            _append_step_list(found, str(job_id), job.get("steps"))
    runs = doc.get("runs")
    if isinstance(runs, dict) and "steps" in runs:
        _append_step_list(found, "runs", runs.get("steps"))
    return found


def iter_workflow_steps(text: str) -> list[tuple[str, str, object]]:
    """Return (job id, step name, step mapping) for every step.

    A composite action contributes its `runs.steps`. Job-level keys are not steps.
    """
    rows: list[tuple[str, str, object]] = []
    for job_id, step in _iter_step_mappings(_load_workflow(text)):
        name = ""
        if isinstance(step, dict):
            raw_name = step.get("name")
            if raw_name is not None:
                name = str(raw_name)
            rows.append((job_id, name, step))
            continue
        rows.append((job_id, name, {}))
    return rows


def parsed_step_count(text: str) -> int:
    """Step count from the stock safe loader, independent of the policy walker."""
    doc = yaml.safe_load(text)
    if not isinstance(doc, dict):
        return 0
    total = 0
    jobs = doc.get("jobs")
    if isinstance(jobs, dict):
        for job in jobs.values():
            if isinstance(job, dict) and isinstance(job.get("steps"), list):
                total += len(job["steps"])
    runs = doc.get("runs")
    if isinstance(runs, dict) and isinstance(runs.get("steps"), list):
        total += len(runs["steps"])
    return total


def document_has_steps(text: str) -> bool:
    doc = yaml.safe_load(text)
    if not isinstance(doc, dict):
        return False
    jobs = doc.get("jobs")
    if isinstance(jobs, dict):
        for job in jobs.values():
            if isinstance(job, dict) and "steps" in job:
                return True
    runs = doc.get("runs")
    return isinstance(runs, dict) and "steps" in runs


def scan_policy_tree(root: Path) -> list[str]:
    """Step-level continue-on-error violations under `root`.

    The walked count has to match the stock safe-loader count. A document
    that has a steps key and no steps is a violation. A steps value that is
    not a list raises.
    """
    violations: list[str] = []
    for path in policy_yaml_paths(root):
        text = path.read_text(encoding="utf-8")
        rel = path.relative_to(root).as_posix()
        walked = iter_workflow_steps(text)
        parsed = parsed_step_count(text)
        if len(walked) != parsed:
            violations.append(f"{rel}: walked {len(walked)} != parsed {parsed}")
        if document_has_steps(text) and not walked:
            violations.append(f"{rel}: steps key yielded no steps")
        for _job, step_name, step in walked:
            if step_continue_on_error(step):
                violations.append(
                    f"{rel} step {step_name!r} has step-level continue-on-error"
                )
    return violations


def _mapping_from_step(step: object) -> dict | None:
    if isinstance(step, dict):
        return step
    if not isinstance(step, str):
        return None
    loaded = _load_workflow(step)
    if isinstance(loaded, list):
        if len(loaded) == 1 and isinstance(loaded[0], dict):
            return loaded[0]
        # Several mappings in one step string are not one step. Using the
        # first item would ignore a later continue-on-error.
        if len(loaded) > 1:
            return {"continue-on-error": _YamlScalar("", "!")}
        return None
    if isinstance(loaded, dict):
        return loaded
    return None


def step_continue_on_error_value(step: object) -> str | None:
    """The step mapping's continue-on-error scalar, or None when the key is absent.

    A non-string value or an explicit tag is a rejected scalar. A plain
    scalar keeps the style the loader recorded. Quoted scalars are rejected.
    """
    mapping = _mapping_from_step(step)
    if not isinstance(mapping, dict) or "continue-on-error" not in mapping:
        return None
    value = mapping["continue-on-error"]
    if not isinstance(value, str) or getattr(value, "explicit_tag", False):
        return _YamlScalar("", "!")
    return value


def step_continue_on_error_forbidden(value: str | None) -> bool:
    if value is None:
        return False
    style = getattr(value, "style", None)
    if style not in _ACCEPTED_SCALAR_STYLES:
        return True
    return str(value) not in _PLAIN_FALSE


def step_continue_on_error(step_text: str) -> bool:
    return step_continue_on_error_forbidden(step_continue_on_error_value(step_text))


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

    def test_pull_request_sanitizer_jobs(self) -> None:
        text = (WORKFLOWS / "sanitizers.yml").read_text(encoding="utf-8")
        self.assertTrue(has_pull_request(text))
        thread = text.split("\n  thread-sanitizer:\n", 1)[1].split("\n  determinism:\n", 1)[0]
        self.assertIn("github.event_name == 'pull_request'", thread)
        self.assertIn("-Zbuild-std", thread)
        self.assertIn('-Zsanitizer=thread', thread)
        self.assertNotIn("RUSTFLAGS=", thread)
        self.assertIn("-p astraweave-ai", thread)
        self.assertIn("-p astraweave-coordination", thread)
        self.assertIn("-p astraweave-ecs", thread)
        self.assertIn('"$GITHUB_WORKSPACE/scripts/tsan_suppressions.txt"', thread)
        self.assertNotIn("suppressions=scripts/tsan_suppressions.txt", thread)
        asan = text.split("\n  asan-critical:\n", 1)[1].split("\n  asan-full:\n", 1)[0]
        self.assertIn("github.event_name != 'pull_request'", asan)
        leak = text.split("\n  leak-sanitizer:\n", 1)[1].split("\n  thread-sanitizer:\n", 1)[0]
        self.assertNotIn("\n    if:", leak)
        self.assertIn("-p aw-save", leak)

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

    def test_step_continue_on_error_covers_every_step(self) -> None:
        paths = policy_yaml_paths()
        workflow_files = [
            path for path in paths if path.parent == WORKFLOWS and path.suffix == ".yml"
        ]
        self.assertEqual(
            {path.name for path in workflow_files},
            {path.name for path in WORKFLOWS.glob("*.yml")},
        )
        self.assertEqual(len(workflow_files), 37)
        rels = {path.relative_to(ROOT).as_posix() for path in paths}
        self.assertIn(".github/actions/rust-ci-setup/action.yml", rels)
        self.assertIn(".github/actions/rust-nightly/action.yml", rels)
        self.assertEqual(scan_policy_tree(ROOT), [])
        self.assertGreater(sum(len(iter_workflow_steps(path.read_text(encoding="utf-8"))) for path in paths), 0)
        for name in (
            "aw-editor-tests.yml",
            "material-validation.yml",
            "pbr-pipeline-ci.yml",
            "codeql-analysis.yml",
        ):
            live = (WORKFLOWS / name).read_text(encoding="utf-8")
            self.assertGreater(len(iter_workflow_steps(live)), 0, name)
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertIn("\n    continue-on-error: true\n", editor)

    def test_scan_policy_tree_flags_injected_keys(self) -> None:
        same_indent = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "    - name: first\n"
            "      run: echo ok\n"
            "    - name: second\n"
            "      run: echo bad\n"
            "      continue-on-error: true\n"
        )
        nested = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: first\n"
            "        run: echo ok\n"
            "      - name: second\n"
            "        run: echo bad\n"
            "        continue-on-error: true\n"
        )
        composite = (
            "name: demo\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - name: first\n"
            "      shell: bash\n"
            "      run: echo ok\n"
            "    - name: second\n"
            "      shell: bash\n"
            "      run: echo bad\n"
            "      continue-on-error: true\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            files = {
                ".github/workflows/flush.yml": same_indent,
                ".github/workflows/extra.yaml": nested,
                ".github/actions/demo/action.yml": composite,
                "tools/local/action.yml": composite,
                "target/hidden/action.yml": composite,
            }
            files["tools/target/zz/action.yml"] = composite
            files["tools/node_modules/zz/action.yml"] = composite
            for rel, text in files.items():
                path = root / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
            _track_tree(root)
            violations = scan_policy_tree(root)
            flagged = {item.split(" step ", 1)[0] for item in violations}
            self.assertEqual(
                flagged,
                {
                    ".github/workflows/flush.yml",
                    ".github/workflows/extra.yaml",
                    ".github/actions/demo/action.yml",
                    "tools/local/action.yml",
                    "tools/target/zz/action.yml",
                    "tools/node_modules/zz/action.yml",
                },
            )
            self.assertTrue(all("second" in item for item in violations))
            self.assertFalse(any(item.startswith("target/") for item in violations))

    def test_scan_policy_tree_flags_an_empty_step_list(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / ".github" / "workflows" / "empty.yml"
            path.parent.mkdir(parents=True)
            path.write_text("jobs:\n  build:\n    steps: []\n", encoding="utf-8")
            violations = scan_policy_tree(root)
            self.assertTrue(any("steps key yielded no steps" in item for item in violations))

    def test_scan_policy_tree_flags_a_short_walk(self) -> None:
        document = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    steps:\n"
            "      - name: one\n"
            "        run: echo\n"
            "      - name: two\n"
            "        run: echo\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / ".github" / "workflows" / "two.yml"
            path.parent.mkdir(parents=True)
            path.write_text(document, encoding="utf-8")
            self.assertEqual(scan_policy_tree(root), [])
            saved = iter_workflow_steps

            def short(text: str) -> list[tuple[str, str, object]]:
                return saved(text)[:1]

            globals()["iter_workflow_steps"] = short
            try:
                violations = scan_policy_tree(root)
            finally:
                globals()["iter_workflow_steps"] = saved
            self.assertTrue(any("walked 1 != parsed 2" in item for item in violations))

    def test_parsed_step_count_ignores_the_walker(self) -> None:
        document = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    steps:\n"
            "      - name: one\n"
            "        run: echo\n"
            "      - name: two\n"
            "        run: echo\n"
            "runs:\n"
            "  steps:\n"
            "    - name: composite\n"
            "      run: echo\n"
        )
        self.assertEqual(parsed_step_count(document), 3)
        self.assertEqual(len(iter_workflow_steps(document)), parsed_step_count(document))
        saved = iter_workflow_steps

        def boom(_text: str) -> list[tuple[str, str, object]]:
            raise AssertionError("parsed_step_count called the policy walker")

        globals()["iter_workflow_steps"] = boom
        try:
            self.assertEqual(parsed_step_count(document), 3)
        finally:
            globals()["iter_workflow_steps"] = saved

    def test_steps_value_must_be_a_list(self) -> None:
        document = "jobs:\n  build:\n    steps:\n      name: not-a-list\n"
        with self.assertRaises(ValueError):
            iter_workflow_steps(document)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / ".github" / "workflows" / "bad.yml"
            path.parent.mkdir(parents=True)
            path.write_text(document, encoding="utf-8")
            with self.assertRaises(ValueError):
                scan_policy_tree(root)

    def test_invalid_yaml_is_not_swallowed(self) -> None:
        document = "jobs: [\n"
        with self.assertRaises(yaml.YAMLError):
            _load_workflow(document)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / ".github" / "workflows" / "bad.yml"
            path.parent.mkdir(parents=True)
            path.write_text(document, encoding="utf-8")
            with self.assertRaises(yaml.YAMLError):
                scan_policy_tree(root)

    def test_non_mapping_step_is_still_counted(self) -> None:
        document = (
            "jobs:\n"
            "  build:\n"
            "    steps:\n"
            "      - just text\n"
            "      - name: second\n"
            "        run: echo\n"
        )
        steps = iter_workflow_steps(document)
        self.assertEqual(len(steps), 2)
        self.assertEqual(len(steps), parsed_step_count(document))
        self.assertEqual([name for _job, name, _step in steps], ["", "second"])

    def test_explicit_tags_and_null_are_forbidden(self) -> None:
        forms = (
            "continue-on-error: !!bool false\n",
            "continue-on-error: !!str false\n",
            "continue-on-error: !!null\n",
            "continue-on-error: !!bool true\n",
            "continue-on-error: !!int 1\n",
        )
        for form in forms:
            mapping = _load_workflow(form)
            self.assertTrue(step_continue_on_error(mapping), form)
            self.assertIsNotNone(step_continue_on_error_value(mapping), form)
        for form in (
            "continue-on-error: ! false\n",
            "continue-on-error: !<!> false\n",
        ):
            mapping = _load_workflow(form)
            self.assertTrue(step_continue_on_error(mapping), form)
            self.assertTrue(
                step_continue_on_error_forbidden(step_continue_on_error_value(mapping)),
                form,
            )

    def test_merge_key_steps_disagree_with_the_safe_loader(self) -> None:
        text = (
            "base: &base\n"
            "  steps:\n"
            "    - run: echo one\n"
            "jobs:\n"
            "  build:\n"
            "    <<: *base\n"
        )
        self.assertEqual(iter_workflow_steps(text), [])
        self.assertEqual(parsed_step_count(text), 1)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / ".github" / "workflows" / "merge.yml"
            path.parent.mkdir(parents=True)
            path.write_text(text, encoding="utf-8")
            violations = scan_policy_tree(root)
            self.assertTrue(any("walked 0 != parsed 1" in item for item in violations))

    def test_reusable_call_without_steps_is_not_a_step_list(self) -> None:
        text = "jobs:\n  call:\n    uses: org/repo/.github/workflows/ci.yml@main\n"
        self.assertEqual(iter_workflow_steps(text), [])
        self.assertEqual(parsed_step_count(text), 0)
        self.assertFalse(document_has_steps(text))

    def test_multi_mapping_step_text_fails_closed(self) -> None:
        several = "- continue-on-error: false\n- continue-on-error: true\n"
        self.assertTrue(step_continue_on_error(several))
        one = "- continue-on-error: true\n"
        self.assertTrue(step_continue_on_error(one))

    def test_folded_false_is_forbidden(self) -> None:
        folded = "name: bad\ncontinue-on-error: >-\n  false\n"
        self.assertTrue(step_continue_on_error(folded))
        self.assertEqual(getattr(step_continue_on_error_value(folded), "style", None), ">")

    def test_mapping_step_text_without_a_dash_is_checked(self) -> None:
        self.assertTrue(step_continue_on_error("name: bad\ncontinue-on-error: true\n"))

    def test_document_has_steps_sees_composite_runs(self) -> None:
        composite = "name: demo\nruns:\n  using: composite\n  steps:\n    - run: echo\n"
        self.assertTrue(document_has_steps(composite))
        self.assertFalse(document_has_steps("name: sample\n"))

    def test_alias_of_a_forbidden_value_is_flagged(self) -> None:
        document = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    steps:\n"
            "      - name: bad\n"
            "        run: echo\n"
            "        continue-on-error: &flag true\n"
            "      - name: also\n"
            "        run: echo\n"
            "        continue-on-error: *flag\n"
        )
        flagged = [name for _job, name, step in iter_workflow_steps(document) if step_continue_on_error(step)]
        self.assertEqual(flagged, ["bad", "also"])

    def test_empty_steps_list_fails_the_nonempty_rule(self) -> None:
        empty = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps: []\n"
        )
        self.assertTrue(document_has_steps(empty))
        self.assertEqual(iter_workflow_steps(empty), [])
        with self.assertRaises(AssertionError):
            self.assertGreater(len(iter_workflow_steps(empty)), 0)

    def test_same_column_step_list_is_checked(self) -> None:
        fixture = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "    - name: Run Unit Tests\n"
            "      run: cargo test\n"
            "      continue-on-error: true\n"
        )
        steps = iter_workflow_steps(fixture)
        self.assertEqual(len(steps), 1)
        self.assertTrue(step_continue_on_error(steps[0][2]))
        for name in (
            "aw-editor-tests.yml",
            "material-validation.yml",
            "pbr-pipeline-ci.yml",
            "codeql-analysis.yml",
        ):
            text = (WORKFLOWS / name).read_text(encoding="utf-8")
            marker = "\n    steps:\n    - "
            self.assertIn(marker, text, name)
            injected = text.replace(
                marker,
                "\n    steps:\n    - continue-on-error: true\n      ",
                1,
            )
            self.assertNotEqual(injected, text, name)
            flagged = [
                step_name
                for _job, step_name, step in iter_workflow_steps(injected)
                if step_continue_on_error(step)
            ]
            self.assertTrue(flagged, name)

    def test_continue_on_error_spellings(self) -> None:
        forms = (
            '      - name: bad\n        "continue-on-error": true\n',
            "      - name: bad\n        continue-on-error : true\n",
            "      - name: bad\n        ? continue-on-error\n        : true\n",
        )
        for step in forms:
            self.assertTrue(step_continue_on_error(step), step)
        different_case = "      - name: bad\n        Continue-On-Error: true\n"
        self.assertFalse(step_continue_on_error(different_case))
        self.assertIsNone(step_continue_on_error_value(different_case))

    def test_flow_style_steps_are_checked(self) -> None:
        documents = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - {name: s, run: echo, continue-on-error: true}\n",
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps: [{run: echo, continue-on-error: true}]\n",
            "name: sample\n"
            "jobs: {a: {runs-on: ubuntu-latest, steps: [{run: echo, continue-on-error: true}]}}\n",
        )
        for document in documents:
            steps = iter_workflow_steps(document)
            self.assertGreaterEqual(len(steps), 1, document)
            self.assertTrue(any(step_continue_on_error(step) for _j, _n, step in steps), document)

    def test_step_layout_variants(self) -> None:
        dash_spaces = (
            "name: sample\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      -   name: bad\n"
            "          run: echo\n"
            "          continue-on-error: true\n"
        )
        commented_job = (
            "name: sample\n"
            "jobs:\n"
            "  a:  # c\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: bad\n"
            "        continue-on-error: true\n"
        )
        indented_jobs = (
            "name: sample\n"
            "jobs:\n"
            "    build:\n"
            "        runs-on: ubuntu-latest\n"
            "        steps:\n"
            "          - name: bad\n"
            "            continue-on-error: true\n"
        )
        quoted_jobs = (
            "name: sample\n"
            '"jobs":\n'
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: bad\n"
            "        continue-on-error: true\n"
        )
        for document in (dash_spaces, commented_job, indented_jobs, quoted_jobs):
            steps = iter_workflow_steps(document)
            self.assertEqual(len(steps), parsed_step_count(document))
            self.assertTrue(any(step_continue_on_error(step) for _j, _n, step in steps), document)

    def test_block_scalar_text_is_not_a_step(self) -> None:
        document = (
            "name: sample\n"
            "jobs:\n"
            "  dash:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: Prepare Dashboard Assets\n"
            "        run: |\n"
            "          cat <<'EOF'\n"
            "          - name: fake\n"
            "            continue-on-error: true\n"
            "          EOF\n"
            "      - name: real\n"
            "        run: echo ok\n"
        )
        steps = iter_workflow_steps(document)
        self.assertEqual([name for _job, name, _step in steps], ["Prepare Dashboard Assets", "real"])
        self.assertEqual(len(steps), parsed_step_count(document))
        for _job, _name, step in steps:
            self.assertFalse(step_continue_on_error(step))
        real_bad = document.replace("      - name: real\n        run: echo ok\n", "      - name: real\n        continue-on-error: true\n        run: echo ok\n", 1)
        flagged = [name for _job, name, step in iter_workflow_steps(real_bad) if step_continue_on_error(step)]
        self.assertEqual(flagged, ["real"])
        block_false = "      - name: bad\n        continue-on-error: |-\n          false\n"
        self.assertTrue(step_continue_on_error(block_false))

    def test_nested_with_env_run_are_not_the_step_key(self) -> None:
        step = (
            "      - name: input\n"
            '        run: "echo continue-on-error: true"\n'
            "        env:\n"
            "          continue-on-error: true\n"
            "        uses: example/action@v1\n"
            "        with:\n"
            "          continue-on-error: true\n"
        )
        self.assertFalse(step_continue_on_error(step))
        self.assertIsNone(step_continue_on_error_value(step))

    def test_comment_is_not_a_continue_on_error_key(self) -> None:
        commented = (
            "      - name: clean\n"
            "        run: echo hi\n"
            "        # continue-on-error: true\n"
        )
        self.assertFalse(step_continue_on_error(commented))
        inline = "      - name: bad\n        continue-on-error: true # note\n"
        self.assertTrue(step_continue_on_error(inline))
        kept = "      - name: ok\n        continue-on-error: false # kept\n"
        self.assertFalse(step_continue_on_error(kept))

    def test_dash_line_continue_on_error_is_checked(self) -> None:
        step = "      - continue-on-error: true\n        name: bad\n"
        self.assertTrue(step_continue_on_error(step))
        self.assertEqual(step_continue_on_error_value(step), "true")

    def test_later_job_and_later_step_are_checked(self) -> None:
        document = (
            "name: sample\n"
            "jobs:\n"
            "  first:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: clean\n"
            "        run: echo ok\n"
            "  second:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - name: also-clean\n"
            "        run: echo ok\n"
            "      - name: bad\n"
            "        run: echo bad\n"
            "        continue-on-error: true\n"
        )
        steps = iter_workflow_steps(document)
        self.assertEqual(len(steps), 3)
        self.assertEqual(len(steps), parsed_step_count(document))
        flagged = [(job, name) for job, name, step in steps if step_continue_on_error(step)]
        self.assertEqual(flagged, [("second", "bad")])

    def test_yaml_workflows_and_composite_actions_are_scanned(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            workflow_dir = root / ".github" / "workflows"
            action_dir = root / ".github" / "actions" / "demo"
            workflow_dir.mkdir(parents=True)
            action_dir.mkdir(parents=True)
            workflow = (
                "name: sample\n"
                "jobs:\n"
                "  build:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - name: bad\n"
                "        continue-on-error: true\n"
            )
            composite = (
                "name: demo\n"
                "runs:\n"
                "  using: composite\n"
                "  steps:\n"
                "    - name: bad\n"
                "      shell: bash\n"
                "      run: echo\n"
                "      continue-on-error: true\n"
            )
            (workflow_dir / "extra.yaml").write_text(workflow, encoding="utf-8")
            (action_dir / "action.yaml").write_text(composite, encoding="utf-8")
            _track_tree(root)
            found = {path.relative_to(root).as_posix(): path.read_text(encoding="utf-8") for path in policy_yaml_paths(root)}
            self.assertIn(".github/workflows/extra.yaml", found)
            self.assertIn(".github/actions/demo/action.yaml", found)
            for rel, text in found.items():
                self.assertGreater(len(iter_workflow_steps(text)), 0, rel)
                self.assertTrue(
                    any(step_continue_on_error(step) for _j, _n, step in iter_workflow_steps(text)),
                    rel,
                )
        live = {path.relative_to(ROOT).as_posix() for path in policy_yaml_paths()}
        self.assertIn(".github/actions/rust-ci-setup/action.yml", live)
        self.assertIn(".github/actions/rust-nightly/action.yml", live)
        for rel in (
            ".github/actions/rust-ci-setup/action.yml",
            ".github/actions/rust-nightly/action.yml",
        ):
            text = (ROOT / rel).read_text(encoding="utf-8")
            self.assertGreater(len(iter_workflow_steps(text)), 0, rel)
            self.assertEqual(len(iter_workflow_steps(text)), parsed_step_count(text), rel)

    def test_continue_on_error_rejected_values(self) -> None:
        self.assertGreaterEqual(len(_FORBIDDEN_CONTINUE_EXAMPLES), 12)
        checked = 0
        for value in _FORBIDDEN_CONTINUE_EXAMPLES:
            step = f"      - name: bad\n        continue-on-error: {value}\n"
            self.assertTrue(step_continue_on_error(step), value)
            self.assertTrue(step_continue_on_error_forbidden(step_continue_on_error_value(step)), value)
            checked += 1
        self.assertEqual(checked, len(_FORBIDDEN_CONTINUE_EXAMPLES))
        empty = "      - name: bad\n        continue-on-error:\n"
        self.assertTrue(step_continue_on_error(empty))

    def test_continue_on_error_allowed_spellings(self) -> None:
        self.assertEqual(
            _ALLOWED_CONTINUE_EXAMPLES,
            (
                "false",
                "False",
                "FALSE",
            ),
        )
        checked = 0
        for value in _ALLOWED_CONTINUE_EXAMPLES:
            step = f"      - name: ok\n        continue-on-error: {value}  # kept\n"
            self.assertFalse(step_continue_on_error(step), value)
            checked += 1
        self.assertEqual(checked, 3)

    def test_job_level_continue_on_error_is_not_a_step(self) -> None:
        job_only = (
            "name: sample\n"
            "jobs:\n"
            "  test:\n"
            "    continue-on-error: true\n"
            "    steps:\n"
            "      - name: clean\n"
            '        run: "echo continue-on-error: true"\n'
            "        # continue-on-error: true\n"
            "      - name: input\n"
            "        uses: example/action@v1\n"
            "        with:\n"
            "          continue-on-error: true\n"
            "        env: |\n"
            "          continue-on-error: true\n"
        )
        seen = iter_workflow_steps(job_only)
        self.assertEqual([step_name for _job, step_name, _step in seen], ["clean", "input"])
        self.assertEqual(len(seen), parsed_step_count(job_only))
        for _job, _step_name, step in seen:
            self.assertFalse(step_continue_on_error(step))

    def test_inverting_forbidden_value_fails(self) -> None:
        source = inspect.getsource(step_continue_on_error_forbidden)
        mutated = source.replace(
            "return str(value) not in _PLAIN_FALSE",
            "return not (str(value) not in _PLAIN_FALSE)",
            1,
        )
        self.assertNotEqual(source, mutated)
        self.assertIn("return str(value) not in _PLAIN_FALSE", source)
        namespace = {
            "_PLAIN_FALSE": _PLAIN_FALSE,
            "_ACCEPTED_SCALAR_STYLES": _ACCEPTED_SCALAR_STYLES,
        }
        exec(mutated, namespace)  # noqa: S102
        broken = namespace["step_continue_on_error_forbidden"]
        self.assertTrue(step_continue_on_error_forbidden("true"))
        self.assertFalse(step_continue_on_error_forbidden(None))
        self.assertFalse(step_continue_on_error_forbidden("false"))
        self.assertFalse(broken(None))
        self.assertTrue(broken("false"))
        with self.assertRaises(AssertionError):
            self.assertTrue(broken("true"))

    def test_mutation_crate_uploads_when_mutants_fail(self) -> None:
        text = mutation_testing_text()
        pairs = (
            ("mutants-ecs", "mutants-core"),
            ("mutants-core", "mutants-ai"),
            ("mutants-behavior", "mutants-nav"),
            ("mutants-nav", "mutants-summary"),
        )
        for job_id, nxt in pairs:
            job = text.split(f"  {job_id}:\n", 1)[1].split(f"\n  {nxt}:\n", 1)[0]
            upload = job_step_block(job, "Upload mutation results")
            self.assertIn("if: always()", upload, job_id)
            self.assertNotIn("continue-on-error", upload, job_id)
            crate = f"astraweave-{job_id.removeprefix('mutants-')}"
            run = job_step_block(job, f"Run mutation testing on {crate}")
            self.assertNotIn("if: always()", run, job_id)
        ecs = text.split("  mutants-ecs:\n", 1)[1].split("\n  mutants-core:\n", 1)[0]
        ecs_run = job_step_block(ecs, "Run mutation testing on astraweave-ecs")
        self.assertIn("--test-threads=1", ecs_run)
        report = job_step_block(ecs, "Generate mutation report")
        self.assertIn("if: always()", report)
        self.assertNotIn("continue-on-error", report)


if __name__ == "__main__":
    unittest.main()

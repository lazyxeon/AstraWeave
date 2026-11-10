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


# A matching quote, or a one-level >, >-, |, or |- block scalar, is valid
# YAML for a uses value. Flatten those forms before the major-tag and SHA
# checks so the ref group cannot keep the quote or the block marker.
_QUOTED_USES_RE = re.compile(
    r"^(?P<lead>\s*(?:-\s+)?)uses:\s*(?P<q>['\"])(?P<value>.*?)(?P=q)(?P<tail>\s*(?:#.*)?)?$"
)
_BLOCK_USES_RE = re.compile(
    r"^(?P<lead>\s*(?:-\s+)?)uses:\s*(?P<style>[>|])(?P<chomp>[+-]?)[0-9]*\s*(?:#.*)?$"
)


def _normalize_uses_quotes(line: str) -> str:
    match = _QUOTED_USES_RE.match(line)
    if match is None:
        return line
    tail = match.group("tail") or ""
    return f"{match.group('lead')}uses: {match.group('value')}{tail}"


def _uses_scan_lines(text: str) -> list[str]:
    """One scan line per source line. Block-scalar continuations become blank."""
    raw = text.splitlines()
    out = list(raw)
    index = 0
    while index < len(raw):
        match = _BLOCK_USES_RE.match(raw[index])
        if match is None:
            out[index] = _normalize_uses_quotes(raw[index])
            index += 1
            continue
        base = len(raw[index]) - len(raw[index].lstrip(" "))
        cursor = index + 1
        parts: list[str] = []
        while cursor < len(raw):
            line = raw[cursor]
            if not line.strip():
                peek = cursor + 1
                while peek < len(raw) and not raw[peek].strip():
                    peek += 1
                if peek < len(raw) and (len(raw[peek]) - len(raw[peek].lstrip(" "))) > base:
                    cursor = peek
                    continue
                break
            if (len(line) - len(line.lstrip(" "))) <= base:
                break
            parts.append(line.strip())
            cursor += 1
        joiner = " " if match.group("style") == ">" else "\n"
        out[index] = f"{match.group('lead')}uses: {joiner.join(parts).strip()}"
        for blank in range(index + 1, cursor):
            out[blank] = ""
        index = cursor
    return out


def _unquote_scalar(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


class _MarkedDict(dict):
    def __init__(self, line: int) -> None:
        super().__init__()
        self.yaml_line = line
        self.key_lines: dict[object, int] = {}


class _MarkedList(list):
    yaml_line = 1


class _MarkedLoader(yaml.SafeLoader):
    """SafeLoader that records the line of each mapping, key, and sequence."""


def _construct_marked_map(loader: yaml.SafeLoader, node: yaml.Node) -> _MarkedDict:
    loader.flatten_mapping(node)
    mapping = _MarkedDict(node.start_mark.line + 1)
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=True)
        value = loader.construct_object(value_node, deep=True)
        mapping[key] = value
        mapping.key_lines[key] = key_node.start_mark.line + 1
    return mapping


def _construct_marked_seq(loader: yaml.SafeLoader, node: yaml.Node) -> _MarkedList:
    seq = _MarkedList()
    seq.yaml_line = node.start_mark.line + 1
    for child in node.value:
        seq.append(loader.construct_object(child, deep=True))
    return seq


_MarkedLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _construct_marked_map,
)
_MarkedLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_SEQUENCE_TAG,
    _construct_marked_seq,
)


def load_workflow_yaml(text: str) -> tuple[object, str | None]:
    """Parse workflow YAML. `yaml.safe_load` is the acceptance check.

    A SafeLoader subclass then keeps line numbers for the same document.
    Invalid YAML is a violation, not a pass.
    """
    try:
        yaml.safe_load(text)
    except yaml.YAMLError as exc:
        return None, str(exc).splitlines()[0]
    try:
        marked = yaml.load(text, _MarkedLoader)  # noqa: S506 — SafeLoader subclass
    except yaml.YAMLError as exc:
        return None, str(exc).splitlines()[0]
    return marked, None


def _iter_uses(node: object):
    if isinstance(node, dict):
        key_lines = getattr(node, "key_lines", {})
        for key, value in node.items():
            if key == "uses" and isinstance(value, str):
                yield key_lines.get(key, getattr(node, "yaml_line", 1)), value
            else:
                yield from _iter_uses(value)
    elif isinstance(node, list):
        for item in node:
            yield from _iter_uses(item)


def _parsed_uses(text: str) -> tuple[list[tuple[int, str]], str | None]:
    doc, err = load_workflow_yaml(text)
    if err:
        return [], err
    if doc is None:
        return [], None
    return list(_iter_uses(doc)), None


def _action_name(uses: str) -> str:
    if uses.startswith("docker://") or "@" not in uses:
        return uses
    return uses.rsplit("@", 1)[0]


def _classify_uses(uses: str) -> tuple[str, str | None]:
    """Return (`policy` or `third` or `skip`, violation or None)."""
    if uses.startswith("./"):
        return "skip", None
    if uses.startswith("docker://"):
        if _DOCKER_PIN_RE.fullmatch(uses):
            return "third", None
        return "third", f"{uses} {DOCKER_DIGEST_MSG}"
    if "@" not in uses:
        return "third", f"{uses} {THIRD_PARTY_SHA_MSG}"
    action, ref = uses.rsplit("@", 1)
    if action in ACTION_MAJOR_FLOORS:
        return "policy", action_ref_violation(action, ref)
    if _SHA_REF_RE.fullmatch(ref):
        return "third", None
    return "third", f"{action}@{ref} {THIRD_PARTY_SHA_MSG}"


def _pin_buckets(text: str) -> tuple[list[str], list[str]]:
    policy: list[str] = []
    third: list[str] = []
    found, err = _parsed_uses(text)
    prefix_line = "1: "
    if err:
        message = f"{prefix_line}{err}"
        return [message], [message]
    for line_no, uses in found:
        bucket, violation = _classify_uses(uses)
        if not violation:
            continue
        rendered = f"{line_no}: {violation}"
        if bucket == "policy":
            policy.append(rendered)
        elif bucket == "third":
            third.append(rendered)
    return policy, third


def workflow_action_pin_violations(text: str, *, path: str = "") -> list[str]:
    prefix = f"{path}: " if path else ""
    policy, _third = _pin_buckets(text)
    return [f"{prefix}{item}" for item in policy]


def _is_yaml_true(value: object) -> bool:
    if value is True:
        return True
    return isinstance(value, str) and value == "true"


def _exact_version(value: object) -> bool:
    return isinstance(value, str) and _EXACT_VERSION_RE.fullmatch(value) is not None


def _mapping(value: object) -> dict:
    return value if isinstance(value, dict) else {}


def _key_outside_with(step: dict, key: str) -> bool:
    if key in step:
        return True
    env = step.get("env")
    return isinstance(env, dict) and key in env


def _iter_steps(doc: object):
    if isinstance(doc, dict):
        jobs = doc.get("jobs")
        if isinstance(jobs, dict):
            for job_id, job in jobs.items():
                if not isinstance(job, dict):
                    continue
                steps = job.get("steps")
                if isinstance(steps, list):
                    for step in steps:
                        if isinstance(step, dict):
                            yield str(job_id), job, step
        runs = doc.get("runs")
        if isinstance(runs, dict) and isinstance(runs.get("steps"), list):
            for step in runs["steps"]:
                if isinstance(step, dict):
                    yield "", runs, step
    elif isinstance(doc, list):
        for step in doc:
            if isinstance(step, dict):
                yield "", {}, step


def _is_codecov_step(step: dict) -> bool:
    uses = step.get("uses")
    return isinstance(uses, str) and uses.startswith("codecov/codecov-action@")


def _codecov_steps(text: str) -> tuple[list[dict] | None, str | None]:
    doc, err = load_workflow_yaml(text)
    if err:
        return None, err
    if doc is None:
        return [], None
    return [step for _job_id, _job, step in _iter_steps(doc) if _is_codecov_step(step)], None


def _step_line(step: dict) -> int:
    return int(getattr(step, "yaml_line", 1))


def _has_token_key(node: object) -> bool:
    if isinstance(node, dict):
        for key, value in node.items():
            if key in {"token", "CODECOV_TOKEN"}:
                return True
            if _has_token_key(value):
                return True
    elif isinstance(node, list):
        return any(_has_token_key(item) for item in node)
    return False


def _has_secrets_ref(node: object) -> bool:
    if isinstance(node, str):
        return "secrets." in node
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(key, str) and "secrets." in key:
                return True
            if _has_secrets_ref(value):
                return True
    elif isinstance(node, list):
        return any(_has_secrets_ref(item) for item in node)
    return False


def _is_upload_only(job: dict) -> bool:
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        return False
    saw_codecov = False
    for step in steps:
        if not isinstance(step, dict) or "run" in step:
            return False
        uses = step.get("uses")
        if not isinstance(uses, str):
            return False
        name = _action_name(uses)
        if name in _UPLOAD_STEP_ACTIONS:
            continue
        if name != "codecov/codecov-action":
            return False
        ref = uses.rsplit("@", 1)[1] if "@" in uses else ""
        if _SHA_REF_RE.fullmatch(ref) is None:
            return False
        saw_codecov = True
    return saw_codecov


def _permission_value_violations(
    perms: object,
    *,
    line: int,
    allow_id_token: bool,
) -> list[tuple[int, str]]:
    key_lines = getattr(perms, "key_lines", {})
    if isinstance(perms, str):
        if perms in {"write", "write-all"}:
            return [(line, COVERAGE_WRITE_MSG)]
        return [(line, TOP_PERMISSIONS_MSG)]
    if not isinstance(perms, dict):
        return [(line, TOP_PERMISSIONS_MSG)]
    found: list[tuple[int, str]] = []
    for key, value in perms.items():
        value_text = value if isinstance(value, str) else ""
        entry_line = int(key_lines.get(key, line))
        if value_text == "write-all" or (value_text == "write" and key != "id-token"):
            found.append((entry_line, COVERAGE_WRITE_MSG))
        elif key == "id-token" and value_text == "write" and not allow_id_token:
            found.append((entry_line, COVERAGE_ID_TOKEN_JOB_MSG))
    return found


THIRD_PARTY_SHA_MSG = "third-party action must be a 40-char commit SHA"
CODECOV_FAIL_CI_MSG = "codecov step must set fail_ci_if_error: true"
CODECOV_VERSION_MSG = "codecov step must set version to an exact vX.Y.Z"
CODECOV_SEARCH_MSG = "codecov step must set disable_search: true"
CODECOV_FILES_MSG = "codecov step must set files to an explicit path"
CODECOV_OIDC_MSG = "codecov step must set use_oidc: true"
CODECOV_CONTINUE_MSG = "codecov job or step must not set continue-on-error"
COVERAGE_WRITE_MSG = "coverage workflow must not grant write permissions"
COVERAGE_ID_TOKEN_JOB_MSG = "id-token: write is only allowed on a job that runs codecov"
CODECOV_ID_TOKEN_MSG = "codecov job must set id-token: write"
CODECOV_UPLOAD_JOB_MSG = "codecov action must run only in an upload job"
CODECOV_TOKEN_MSG = "codecov step must not set a token input or a secrets reference"
TOP_PERMISSIONS_MSG = "workflow must set top-level permissions to {}"
JOB_CONTENTS_READ_MSG = "job must set permissions to contents: read"
DOCKER_DIGEST_MSG = "docker action must be pinned with @sha256"
_SHA_REF_RE = re.compile(r"[0-9a-f]{40}")
_DOCKER_PIN_RE = re.compile(r"^docker://\S+@sha256:[0-9a-f]{64}$")
_EXACT_VERSION_RE = re.compile(r"v[0-9]+\.[0-9]+\.[0-9]+")
_UPLOAD_STEP_ACTIONS = frozenset({"actions/checkout", "actions/download-artifact"})
_LEAST_PRIVILEGE_WORKFLOWS = frozenset({"kani.yml", "editor-ci.yml"})
_MISSING = object()
KANI_VERSION_MSG = "kani step must set kani-version to an exact X.Y.Z"
LLVM_COV_CARGO_MSG = "cargo install cargo-llvm-cov must set --version X.Y.Z and --locked"
LLVM_COV_ACTION_MSG = "install-action cargo-llvm-cov must set tool: cargo-llvm-cov@X.Y.Z"
_COVERAGE_WORKFLOW_NAMES = frozenset({"coverage.yml", "aw-editor-tests.yml"})
_JOB_HEADER_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*(?:#.*)?$")
_CONTINUE_ON_ERROR_RE = re.compile(r"^\s*continue-on-error\s*:")
_STEP_DASH_RE = re.compile(r"^(\s*)-\s+\S")
_KANI_VERSION_RE = re.compile(
    r"(?m)^\s*kani-version:\s*['\"]?[0-9]+\.[0-9]+\.[0-9]+['\"]?\s*(?:#.*)?$"
)
_LLVM_CARGO_VERSION_RE = re.compile(r"--version\s+[0-9]+\.[0-9]+\.[0-9]+\b")
_LLVM_TOOL_RE = re.compile(
    r"(?m)^\s*tool:\s*cargo-llvm-cov@[0-9]+\.[0-9]+\.[0-9]+\s*(?:#.*)?$"
)


def third_party_action_pin_violations(text: str, *, path: str = "") -> list[str]:
    prefix = f"{path}: " if path else ""
    _policy, third = _pin_buckets(text)
    return [f"{prefix}{item}" for item in third]


_STEPS_HEADER_RE = re.compile(r"^(\s*)steps:\s*(?:#.*)?$")


def _step_blocks(text: str) -> list[tuple[int, str]]:
    """Steps under a `steps:` key. Schedule and matrix lists are not steps."""
    lines = text.splitlines()
    blocks: list[tuple[int, str]] = []
    index = 0
    while index < len(lines):
        header = _STEPS_HEADER_RE.match(lines[index])
        if not header:
            index += 1
            continue
        steps_indent = len(header.group(1))
        index += 1
        region_end = index
        while region_end < len(lines):
            line = lines[region_end]
            if not line.strip() or line.lstrip().startswith("#"):
                region_end += 1
                continue
            indent = len(line) - len(line.lstrip(" "))
            if indent < steps_indent:
                break
            if indent == steps_indent and _STEP_DASH_RE.match(line) is None:
                break
            region_end += 1
        dash_lines = [
            (cursor, len(matched.group(1)))
            for cursor in range(index, region_end)
            if (matched := _STEP_DASH_RE.match(lines[cursor]))
        ]
        if dash_lines:
            step_indent = min(indent for _, indent in dash_lines)
            directs = [cursor for cursor, indent in dash_lines if indent == step_indent]
            for position, start in enumerate(directs):
                end = directs[position + 1] if position + 1 < len(directs) else region_end
                blocks.append((start + 1, "\n".join(lines[start:end])))
        index = region_end
    return blocks


def _explicit_file_value(value: object) -> bool:
    if not isinstance(value, str):
        return False
    text = value.strip()
    return text not in {"", "|", ">", "~", "null"}


def codecov_fail_ci_violations(text: str, *, path: str = "") -> list[str]:
    """Read fail_ci_if_error and version from the step's with: mapping only."""
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for step in steps or []:
        line_no = _step_line(step)
        body = _mapping(step.get("with"))
        if _key_outside_with(step, "fail_ci_if_error") or not _is_yaml_true(
            body.get("fail_ci_if_error")
        ):
            violations.append(f"{prefix}{line_no}: {CODECOV_FAIL_CI_MSG}")
        if _key_outside_with(step, "version") or not _exact_version(body.get("version")):
            violations.append(f"{prefix}{line_no}: {CODECOV_VERSION_MSG}")
    return violations


def _job_blocks(text: str) -> list[tuple[int, str, str]]:
    """Return (line number, job id, block) for each job under jobs:."""
    lines = text.splitlines()
    jobs_at = next(
        (index for index, line in enumerate(lines) if line == "jobs:" or line.startswith("jobs:")),
        None,
    )
    if jobs_at is None:
        return []
    headers: list[int] = []
    end = len(lines)
    for index in range(jobs_at + 1, len(lines)):
        line = lines[index]
        if line and not line.startswith(" ") and not line.startswith("#"):
            end = index
            break
        if _JOB_HEADER_RE.match(line):
            headers.append(index)
    blocks: list[tuple[int, str, str]] = []
    for position, start in enumerate(headers):
        stop = headers[position + 1] if position + 1 < len(headers) else end
        job_id = _JOB_HEADER_RE.match(lines[start]).group(1)
        blocks.append((start + 1, job_id, "\n".join(lines[start:stop])))
    return blocks


def _job_has_codecov(job: object) -> bool:
    if not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list):
        return False
    return any(isinstance(step, dict) and _is_codecov_step(step) for step in steps)


def _codecov_continue_scope(text: str) -> set[str] | None:
    """Job ids that run Codecov, plus the jobs named in their needs:."""
    doc, err = load_workflow_yaml(text)
    if err or not isinstance(doc, dict):
        return None
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return set()
    uploaders = [
        str(job_id)
        for job_id, job in jobs.items()
        if _job_has_codecov(job)
    ]
    scope = set(uploaders)
    for job_id in uploaders:
        needs = jobs[job_id].get("needs") if isinstance(jobs[job_id], dict) else None
        if isinstance(needs, str):
            scope.add(needs)
        elif isinstance(needs, list):
            scope.update(item for item in needs if isinstance(item, str))
    return scope


def codecov_continue_on_error_violations(text: str, *, path: str = "") -> list[str]:
    """A Codecov job, and a job that job needs, must not set continue-on-error.

    The job key and every step in those jobs are in scope. Other jobs are not.
    """
    violations: list[str] = []
    prefix = f"{path}: " if path else ""
    scope = _codecov_continue_scope(text)
    for job_line, job_id, block in _job_blocks(text):
        if scope is None:
            in_scope = "codecov/codecov-action@" in block
        else:
            in_scope = job_id in scope
        if not in_scope:
            continue
        for offset, line in enumerate(block.splitlines()):
            if _CONTINUE_ON_ERROR_RE.match(line) is None:
                continue
            violations.append(f"{prefix}{job_line + offset}: {CODECOV_CONTINUE_MSG}")
    return violations


def codecov_explicit_file_violations(text: str, *, path: str = "") -> list[str]:
    """Read disable_search and files from the step's with: mapping only."""
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for step in steps or []:
        line_no = _step_line(step)
        body = _mapping(step.get("with"))
        if _key_outside_with(step, "disable_search") or not _is_yaml_true(
            body.get("disable_search")
        ):
            violations.append(f"{prefix}{line_no}: {CODECOV_SEARCH_MSG}")
        if _key_outside_with(step, "files") or not _explicit_file_value(body.get("files")):
            violations.append(f"{prefix}{line_no}: {CODECOV_FILES_MSG}")
    return violations


def codecov_oidc_violations(text: str, *, path: str = "") -> list[str]:
    """Read use_oidc from the step's with: mapping only."""
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for step in steps or []:
        line_no = _step_line(step)
        body = _mapping(step.get("with"))
        if _key_outside_with(step, "use_oidc") or not _is_yaml_true(body.get("use_oidc")):
            violations.append(f"{prefix}{line_no}: {CODECOV_OIDC_MSG}")
    return violations


def codecov_token_violations(text: str, *, path: str = "") -> list[str]:
    """Reject a token key, a CODECOV_TOKEN key, and any secrets. reference.

    The secret name is not special-cased. The key may sit under with:, env:,
    or on the step itself.
    """
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for step in steps or []:
        if _has_token_key(step) or _has_secrets_ref(step):
            violations.append(f"{prefix}{_step_line(step)}: {CODECOV_TOKEN_MSG}")
    return violations


def coverage_permission_violations(text: str, *, path: str = "") -> list[str]:
    """Parsed permissions for coverage.yml and aw-editor-tests.yml.

    Top-level permissions must be {}. A missing key fails. write-all, and any
    permission value write, fail, including a flow mapping or a quoted key.
    id-token: write is allowed only on an upload-only job: every step is
    actions/checkout or actions/download-artifact, plus the SHA-pinned
    codecov action, and that job must set id-token: write. Other permission
    values are not this check.
    """
    if Path(path).name not in _COVERAGE_WORKFLOW_NAMES:
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return [f"{prefix}1: {TOP_PERMISSIONS_MSG}"]
    violations: list[str] = []
    perms = doc.get("permissions", _MISSING)
    top_line = int(getattr(doc, "key_lines", {}).get("permissions", 1))
    if perms is _MISSING or perms is None:
        violations.append(f"{prefix}{top_line}: {TOP_PERMISSIONS_MSG}")
    elif isinstance(perms, str):
        message = COVERAGE_WRITE_MSG if perms in {"write", "write-all"} else TOP_PERMISSIONS_MSG
        violations.append(f"{prefix}{top_line}: {message}")
    elif isinstance(perms, dict):
        if len(perms) != 0:
            found = _permission_value_violations(perms, line=top_line, allow_id_token=False)
            if found:
                violations.extend(f"{prefix}{line}: {message}" for line, message in found)
            else:
                violations.append(f"{prefix}{top_line}: {TOP_PERMISSIONS_MSG}")
    else:
        violations.append(f"{prefix}{top_line}: {TOP_PERMISSIONS_MSG}")

    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return violations
    job_lines = getattr(jobs, "key_lines", {})
    for job_id, job in jobs.items():
        if not isinstance(job, dict):
            continue
        job_line = int(job_lines.get(job_id, 1))
        upload_only = _is_upload_only(job)
        has_codecov = _job_has_codecov(job)
        job_perms = job.get("permissions", _MISSING)
        perm_line = int(getattr(job, "key_lines", {}).get("permissions", job_line))
        if job_perms is not _MISSING and job_perms is not None:
            found = _permission_value_violations(
                job_perms,
                line=perm_line,
                allow_id_token=upload_only,
            )
            violations.extend(f"{prefix}{line}: {message}" for line, message in found)
        has_id_write = isinstance(job_perms, dict) and job_perms.get("id-token") == "write"
        if upload_only and not has_id_write:
            violations.append(f"{prefix}{job_line}: {CODECOV_ID_TOKEN_MSG}")
        if has_codecov and not upload_only:
            violations.append(f"{prefix}{job_line}: {CODECOV_UPLOAD_JOB_MSG}")
    return violations


def least_privilege_permission_violations(text: str, *, path: str = "") -> list[str]:
    """kani.yml and editor-ci.yml: top-level {} and each job exactly contents: read."""
    if Path(path).name not in _LEAST_PRIVILEGE_WORKFLOWS:
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err or not isinstance(doc, dict):
        return [f"{prefix}1: {err or TOP_PERMISSIONS_MSG}"]
    violations: list[str] = []
    perms = doc.get("permissions", _MISSING)
    if not (isinstance(perms, dict) and len(perms) == 0):
        line = int(getattr(doc, "key_lines", {}).get("permissions", 1))
        violations.append(f"{prefix}{line}: {TOP_PERMISSIONS_MSG}")
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return violations
    job_lines = getattr(jobs, "key_lines", {})
    for job_id, job in jobs.items():
        job_line = int(job_lines.get(job_id, 1))
        job_perms = job.get("permissions", _MISSING) if isinstance(job, dict) else _MISSING
        exact = (
            isinstance(job_perms, dict)
            and len(job_perms) == 1
            and job_perms.get("contents") == "read"
        )
        if exact:
            continue
        entry = job_line
        if isinstance(job, dict):
            entry = int(getattr(job, "key_lines", {}).get("permissions", job_line))
        violations.append(f"{prefix}{entry}: {JOB_CONTENTS_READ_MSG}")
    return violations


def kani_version_violations(text: str, *, path: str = "") -> list[str]:
    violations: list[str] = []
    prefix = f"{path}: " if path else ""
    for line_no, block in _step_blocks(text):
        if "model-checking/kani-github-action@" not in block:
            continue
        if _KANI_VERSION_RE.search(block) is None:
            violations.append(f"{prefix}{line_no}: {KANI_VERSION_MSG}")
    return violations


def llvm_cov_pin_violations(text: str, *, path: str = "") -> list[str]:
    violations: list[str] = []
    prefix = f"{path}: " if path else ""
    for line_no, line in enumerate(text.splitlines(), start=1):
        if "cargo install" not in line or "cargo-llvm-cov" not in line:
            continue
        if "--locked" in line and _LLVM_CARGO_VERSION_RE.search(line):
            continue
        violations.append(f"{prefix}{line_no}: {LLVM_COV_CARGO_MSG}")
    for line_no, block in _step_blocks(text):
        if "taiki-e/install-action@" not in block or "cargo-llvm-cov" not in block:
            continue
        if _LLVM_TOOL_RE.search(block):
            continue
        violations.append(f"{prefix}{line_no}: {LLVM_COV_ACTION_MSG}")
    return violations


def github_action_surfaces() -> list[Path]:
    roots = (WORKFLOWS, ROOT / ".github" / "actions")
    found: list[Path] = []
    for root in roots:
        found.extend(root.rglob("*.yml"))
        found.extend(root.rglob("*.yaml"))
    return sorted(found)


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

    def test_third_party_actions_are_commit_pinned(self) -> None:
        violations: list[str] = []
        codecov: list[str] = []
        for path in github_action_surfaces():
            text = path.read_text(encoding="utf-8")
            rel = str(path.relative_to(ROOT))
            violations.extend(third_party_action_pin_violations(text, path=rel))
            codecov.extend(codecov_fail_ci_violations(text, path=rel))
            codecov.extend(codecov_continue_on_error_violations(text, path=rel))
            codecov.extend(codecov_explicit_file_violations(text, path=rel))
            codecov.extend(codecov_oidc_violations(text, path=rel))
            codecov.extend(codecov_token_violations(text, path=rel))
            codecov.extend(coverage_permission_violations(text, path=rel))
            codecov.extend(least_privilege_permission_violations(text, path=rel))
            codecov.extend(kani_version_violations(text, path=rel))
            codecov.extend(llvm_cov_pin_violations(text, path=rel))
        self.assertEqual(violations, [])
        self.assertEqual(codecov, [])
        ignored = "uses: actions/checkout@v7\nuses: ./.github/actions/rust-ci-setup\n"
        self.assertEqual(third_party_action_pin_violations(ignored), [])

    def test_codecov_action_v7_tag_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(third_party_action_pin_violations(text, path="coverage.yml"), [])
        mutant = re.sub(
            r"codecov/codecov-action@[0-9a-f]{40}",
            "codecov/codecov-action@v7",
            text,
            count=1,
        )
        self.assertNotEqual(mutant, text)
        violations = third_party_action_pin_violations(mutant, path="coverage.yml")
        self.assertTrue(violations)
        self.assertIn("codecov/codecov-action@v7", violations[0])
        self.assertIn(THIRD_PARTY_SHA_MSG, violations[0])

    def test_codecov_fail_ci_if_error_false_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_fail_ci_violations(text, path="coverage.yml"), [])
        mutant = text.replace("fail_ci_if_error: true", "fail_ci_if_error: false", 1)
        self.assertNotEqual(mutant, text)
        violations = codecov_fail_ci_violations(mutant, path="coverage.yml")
        upload_line = next(
            number
            for number, line in enumerate(mutant.splitlines(), start=1)
            if "Upload coverage to Codecov" in line
        )
        self.assertEqual(
            violations,
            [f"coverage.yml: {upload_line}: {CODECOV_FAIL_CI_MSG}"],
        )
        # A later step that mentions the true value does not cover this step.
        decoy = mutant.replace(
            "          use_oidc: true\n",
            "          use_oidc: true\n\n"
            "      - name: Not codecov\n"
            '        run: "echo fail_ci_if_error: true"\n',
            1,
        )
        self.assertNotEqual(decoy, mutant)
        decoy_violations = codecov_fail_ci_violations(decoy, path="coverage.yml")
        self.assertTrue(decoy_violations)
        self.assertIn(CODECOV_FAIL_CI_MSG, decoy_violations[0])

    def test_codecov_continue_on_error_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_continue_on_error_violations(text, path="coverage.yml"), [])
        self.assertNotIn("continue-on-error", text)
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        editor_jobs = {job_id: block for _, job_id, block in _job_blocks(editor)}
        self.assertNotIn("continue-on-error", editor_jobs["coverage"])
        self.assertIn("continue-on-error: true", editor_jobs["test"])
        self.assertEqual(
            codecov_continue_on_error_violations(editor, path="aw-editor-tests.yml"),
            [],
        )
        job_mutant = text.replace(
            "    timeout-minutes: 45\n",
            "    timeout-minutes: 45\n    continue-on-error: true\n",
            1,
        )
        self.assertNotEqual(job_mutant, text)
        job_line = next(
            number
            for number, line in enumerate(job_mutant.splitlines(), start=1)
            if line.strip() == "continue-on-error: true"
        )
        job_violations = codecov_continue_on_error_violations(job_mutant, path="coverage.yml")
        self.assertEqual(
            job_violations,
            [f"coverage.yml: {job_line}: {CODECOV_CONTINUE_MSG}"],
        )
        step_mutant = text.replace(
            "        uses: codecov/codecov-action@",
            "        continue-on-error: true\n        uses: codecov/codecov-action@",
            1,
        )
        self.assertNotEqual(step_mutant, text)
        step_line = next(
            number
            for number, line in enumerate(step_mutant.splitlines(), start=1)
            if line.strip() == "continue-on-error: true"
        )
        step_violations = codecov_continue_on_error_violations(step_mutant, path="coverage.yml")
        self.assertEqual(
            step_violations,
            [f"coverage.yml: {step_line}: {CODECOV_CONTINUE_MSG}"],
        )
        sibling = text.replace(
            "      - name: Install cargo-tarpaulin\n        run:",
            "      - name: Install cargo-tarpaulin\n        continue-on-error: true\n        run:",
            1,
        )
        self.assertNotEqual(sibling, text)
        sibling_line = next(
            number
            for number, line in enumerate(sibling.splitlines(), start=1)
            if line.strip() == "continue-on-error: true"
        )
        sibling_violations = codecov_continue_on_error_violations(sibling, path="coverage.yml")
        self.assertEqual(
            sibling_violations,
            [f"coverage.yml: {sibling_line}: {CODECOV_CONTINUE_MSG}"],
        )
        editor_mutant = editor.replace(
            "    - name: Generate Coverage Report\n      run: |\n",
            "    - name: Generate Coverage Report\n      continue-on-error: true\n      run: |\n",
            1,
        )
        self.assertNotEqual(editor_mutant, editor)
        editor_line = editor_mutant.split("      continue-on-error: true\n      run: |")[0].count("\n") + 1
        self.assertEqual(
            codecov_continue_on_error_violations(editor_mutant, path="aw-editor-tests.yml"),
            [f"aw-editor-tests.yml: {editor_line}: {CODECOV_CONTINUE_MSG}"],
        )

    def test_quoted_uses_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        quoted_sha = (
            'uses: "codecov/codecov-action@'
            '303a32d7a59b442fa8d48b6a1cc6825c09c847a5" # v7.1.1\n'
        )
        self.assertEqual(third_party_action_pin_violations(quoted_sha, path="coverage.yml"), [])
        single = "uses: 'codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5'\n"
        self.assertEqual(third_party_action_pin_violations(single, path="coverage.yml"), [])
        mutant = text.replace(
            "uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1",
            'uses: "codecov/codecov-action@v7"',
            1,
        )
        self.assertNotEqual(mutant, text)
        violations = third_party_action_pin_violations(mutant, path="coverage.yml")
        self.assertTrue(violations)
        self.assertIn("codecov/codecov-action@v7", violations[0])
        self.assertNotIn('"', violations[0].split("uses", 1)[-1])
        self.assertEqual(
            workflow_action_pin_violations('uses: "actions/checkout@v7"\n', path="coverage.yml"),
            [],
        )
        below = workflow_action_pin_violations(
            '      - uses: "actions/checkout@v6" # floating\n',
            path="coverage.yml",
        )
        self.assertEqual(len(below), 1)
        self.assertIn("actions/checkout@v6", below[0])
        self.assertIn(BELOW_FLOOR_MSG, below[0])
        self.assertNotIn('"', below[0])

    def test_block_scalar_uses_mutant_is_rejected(self) -> None:
        folded = "uses: >-\n  codecov/codecov-action@v7\n"
        violations = third_party_action_pin_violations(folded, path="coverage.yml")
        self.assertEqual(
            violations,
            [f"coverage.yml: 1: codecov/codecov-action@v7 {THIRD_PARTY_SHA_MSG}"],
        )
        plain = third_party_action_pin_violations(
            "uses: >\n  codecov/codecov-action@v7\n",
            path="coverage.yml",
        )
        self.assertIn("codecov/codecov-action@v7", plain[0])
        literal = third_party_action_pin_violations(
            "uses: |\n  codecov/codecov-action@v7\n",
            path="coverage.yml",
        )
        self.assertIn("codecov/codecov-action@v7", literal[0])
        strip = third_party_action_pin_violations(
            "uses: |-\n  codecov/codecov-action@v7\n",
            path="coverage.yml",
        )
        self.assertIn("codecov/codecov-action@v7", strip[0])
        quoted_master = third_party_action_pin_violations(
            'uses: "dtolnay/rust-toolchain@master"\n',
            path="kani.yml",
        )
        self.assertIn("dtolnay/rust-toolchain@master", quoted_master[0])
        self.assertNotIn('"', quoted_master[0])
        pinned = (
            "uses: >-\n"
            "  codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5\n"
        )
        self.assertEqual(third_party_action_pin_violations(pinned, path="coverage.yml"), [])
        floor = workflow_action_pin_violations(
            "      - uses: >-\n          actions/checkout@v6\n",
            path="coverage.yml",
        )
        self.assertEqual(len(floor), 1)
        self.assertIn("coverage.yml: 1:", floor[0])
        self.assertIn(BELOW_FLOOR_MSG, floor[0])
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        mutant = text.replace(
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1",
            "        uses: >-\n          codecov/codecov-action@v7",
            1,
        )
        self.assertNotEqual(mutant, text)
        file_violations = third_party_action_pin_violations(mutant, path="coverage.yml")
        self.assertEqual(len(file_violations), 1)
        self.assertIn("codecov/codecov-action@v7", file_violations[0])

    def test_codecov_version_removal_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_fail_ci_violations(text, path="coverage.yml"), [])
        quoted = text.replace("version: v11.3.2", 'version: "v11.3.2"', 1)
        self.assertEqual(codecov_fail_ci_violations(quoted, path="coverage.yml"), [])
        removed = text.replace("          version: v11.3.2\n", "", 1)
        self.assertNotEqual(removed, text)
        upload_line = next(
            number
            for number, line in enumerate(removed.splitlines(), start=1)
            if "Upload coverage to Codecov" in line
        )
        self.assertEqual(
            codecov_fail_ci_violations(removed, path="coverage.yml"),
            [f"coverage.yml: {upload_line}: {CODECOV_VERSION_MSG}"],
        )
        latest = text.replace("version: v11.3.2", "version: latest", 1)
        self.assertIn(CODECOV_VERSION_MSG, codecov_fail_ci_violations(latest, path="coverage.yml")[0])

    def test_codecov_file_search_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_explicit_file_violations(text, path="coverage.yml"), [])
        self.assertEqual(codecov_explicit_file_violations(editor, path="aw-editor-tests.yml"), [])
        upload_line = next(
            number
            for number, line in enumerate(text.splitlines(), start=1)
            if "Upload coverage to Codecov" in line
        )
        no_search = text.replace("          disable_search: true\n", "", 1)
        self.assertNotEqual(no_search, text)
        self.assertEqual(
            codecov_explicit_file_violations(no_search, path="coverage.yml"),
            [f"coverage.yml: {upload_line}: {CODECOV_SEARCH_MSG}"],
        )
        no_files = text.replace(
            "          files: ./tools/astraweave-assets/coverage/cobertura.xml\n",
            "",
            1,
        )
        self.assertEqual(
            codecov_explicit_file_violations(no_files, path="coverage.yml"),
            [f"coverage.yml: {upload_line}: {CODECOV_FILES_MSG}"],
        )
        searched = text.replace("disable_search: true", "disable_search: false", 1)
        self.assertIn(
            CODECOV_SEARCH_MSG,
            codecov_explicit_file_violations(searched, path="coverage.yml")[0],
        )

    def test_codecov_oidc_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertNotIn("CODECOV_TOKEN", text)
        self.assertNotIn("CODECOV_TOKEN", editor)
        self.assertEqual(codecov_oidc_violations(text, path="coverage.yml"), [])
        self.assertEqual(codecov_oidc_violations(editor, path="aw-editor-tests.yml"), [])
        removed = text.replace("          use_oidc: true\n", "", 1)
        self.assertNotEqual(removed, text)
        upload_line = next(
            number
            for number, line in enumerate(removed.splitlines(), start=1)
            if "Upload coverage to Codecov" in line
        )
        self.assertEqual(
            codecov_oidc_violations(removed, path="coverage.yml"),
            [f"coverage.yml: {upload_line}: {CODECOV_OIDC_MSG}"],
        )
        disabled = text.replace("use_oidc: true", "use_oidc: false", 1)
        self.assertIn(CODECOV_OIDC_MSG, codecov_oidc_violations(disabled, path="coverage.yml")[0])
        env_only = text.replace(
            "          use_oidc: true\n",
            "        env:\n          use_oidc: true\n",
            1,
        )
        self.assertNotEqual(env_only, text)
        self.assertTrue(codecov_oidc_violations(env_only, path="coverage.yml"))
        editor_removed = editor.replace("        use_oidc: true\n", "", 1)
        self.assertNotEqual(editor_removed, editor)
        self.assertTrue(codecov_oidc_violations(editor_removed, path="aw-editor-tests.yml"))

    def test_coverage_write_permission_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_permission_violations(text, path="coverage.yml"), [])
        self.assertEqual(coverage_permission_violations(editor, path="aw-editor-tests.yml"), [])
        self.assertEqual(
            coverage_permission_violations("permissions: write-all\n", path="scorecard.yml"),
            [],
        )
        write_all = text.replace("permissions: {}", "permissions: write-all", 1)
        self.assertNotEqual(write_all, text)
        write_all_line = next(
            number
            for number, line in enumerate(write_all.splitlines(), start=1)
            if line.strip() == "permissions: write-all"
        )
        self.assertEqual(
            coverage_permission_violations(write_all, path="coverage.yml"),
            [f"coverage.yml: {write_all_line}: {COVERAGE_WRITE_MSG}"],
        )
        contents = text.replace("contents: read", "contents: write", 1)
        contents_line = next(
            number
            for number, line in enumerate(contents.splitlines(), start=1)
            if line.strip() == "contents: write"
        )
        self.assertEqual(
            coverage_permission_violations(contents, path="coverage.yml"),
            [f"coverage.yml: {contents_line}: {COVERAGE_WRITE_MSG}"],
        )
        actions = editor.replace(
            "      contents: read\n",
            "      contents: read\n      actions: write\n",
            1,
        )
        self.assertNotEqual(actions, editor)
        actions_line = next(
            number
            for number, line in enumerate(actions.splitlines(), start=1)
            if line.strip() == "actions: write"
        )
        self.assertEqual(
            coverage_permission_violations(actions, path="aw-editor-tests.yml"),
            [f"aw-editor-tests.yml: {actions_line}: {COVERAGE_WRITE_MSG}"],
        )
        quoted = text.replace("contents: read", 'contents: "write"', 1)
        self.assertTrue(coverage_permission_violations(quoted, path="coverage.yml"))
        dropped = text.replace("      id-token: write\n", "", 1)
        self.assertNotEqual(dropped, text)
        job_line = next(
            number
            for number, line in enumerate(dropped.splitlines(), start=1)
            if line.strip() == "codecov:"
        )
        self.assertEqual(
            coverage_permission_violations(dropped, path="coverage.yml"),
            [f"coverage.yml: {job_line}: {CODECOV_ID_TOKEN_MSG}"],
        )
        other_job = editor.replace(
            "    permissions:\n      contents: read\n    runs-on: ${{ matrix.os }}\n",
            "    permissions:\n      contents: read\n      id-token: write\n    runs-on: ${{ matrix.os }}\n",
            1,
        )
        self.assertNotEqual(other_job, editor)
        other_line = next(
            number
            for number, line in enumerate(other_job.splitlines(), start=1)
            if line.strip() == "id-token: write"
        )
        self.assertEqual(
            coverage_permission_violations(other_job, path="aw-editor-tests.yml"),
            [f"aw-editor-tests.yml: {other_line}: {COVERAGE_ID_TOKEN_JOB_MSG}"],
        )
        top = text.replace("permissions: {}", "permissions:\n  id-token: write", 1)
        self.assertIn(
            COVERAGE_ID_TOKEN_JOB_MSG,
            coverage_permission_violations(top, path="coverage.yml")[0],
        )

    def test_parsed_uses_shapes_are_rejected(self) -> None:
        unpinned = "codecov/codecov-action@v7"
        pinned = (
            "codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5"
        )
        shapes = {
            "space": f"uses : {unpinned}\n",
            "quoted-key": f'"uses": {unpinned}\n',
            "plain-next-line": f"uses:\n  {unpinned}\n",
            "flow": f"steps:\n  - {{uses: {unpinned}}}\n",
            "explicit-key": f"? uses\n: {unpinned}\n",
            "escaped": 'uses: "codecov/codecov-action\\u0040v7"\n',
            "multiline": 'uses: "codecov/codecov-action@\\\n  v7"\n',
            "alias": f"pin: &unpinned {unpinned}\nuses: *unpinned\n",
            "indent-chomp": f"uses: >2-\n  {unpinned}\n",
            "docker": "uses: docker://alpine:3.19\n",
        }
        for name, snippet in shapes.items():
            violations = third_party_action_pin_violations(snippet, path="coverage.yml")
            self.assertTrue(violations, name)
            if name == "docker":
                self.assertIn(DOCKER_DIGEST_MSG, violations[0])
                self.assertIn("docker://alpine:3.19", violations[0])
            else:
                self.assertIn("codecov/codecov-action@v7", violations[0], name)
                self.assertIn(THIRD_PARTY_SHA_MSG, violations[0], name)
        self.assertEqual(
            third_party_action_pin_violations(f"uses: >2-\n  {pinned}\n", path="coverage.yml"),
            [],
        )
        self.assertEqual(
            third_party_action_pin_violations(
                f"pin: &pinned {pinned}\nuses: *pinned\n",
                path="coverage.yml",
            ),
            [],
        )
        digest = "a" * 64
        self.assertEqual(
            third_party_action_pin_violations(
                f"uses: docker://alpine@sha256:{digest}\n",
                path="coverage.yml",
            ),
            [],
        )
        unknown = third_party_action_pin_violations(
            "uses: actions/not-a-real-action@main\n",
            path="coverage.yml",
        )
        self.assertTrue(unknown)
        self.assertIn("actions/not-a-real-action@main", unknown[0])
        self.assertIn(THIRD_PARTY_SHA_MSG, unknown[0])
        self.assertEqual(
            workflow_action_pin_violations("uses: actions/checkout@v7\n", path="coverage.yml"),
            [],
        )
        self.assertEqual(
            third_party_action_pin_violations("uses: actions/checkout@v6\n", path="coverage.yml"),
            [],
        )

    def test_parsed_permission_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_permission_violations(text, path="coverage.yml"), [])
        flow = text.replace("permissions: {}", "permissions: {contents: write}", 1)
        self.assertNotEqual(flow, text)
        self.assertIn(
            COVERAGE_WRITE_MSG,
            coverage_permission_violations(flow, path="coverage.yml")[0],
        )
        flow_id = text.replace("permissions: {}", "permissions: {id-token: write}", 1)
        self.assertIn(
            COVERAGE_ID_TOKEN_JOB_MSG,
            coverage_permission_violations(flow_id, path="coverage.yml")[0],
        )
        quoted = text.replace("permissions: {}", '"permissions": {contents: write}', 1)
        self.assertIn(
            COVERAGE_WRITE_MSG,
            coverage_permission_violations(quoted, path="coverage.yml")[0],
        )
        missing = text.replace("permissions: {}\n", "", 1)
        self.assertNotEqual(missing, text)
        self.assertIn(
            TOP_PERMISSIONS_MSG,
            coverage_permission_violations(missing, path="coverage.yml")[0],
        )
        on_generate = text.replace(
            "    name: Generate Code Coverage\n    permissions:\n      contents: read\n",
            "    name: Generate Code Coverage\n    permissions:\n      contents: read\n      id-token: write\n",
            1,
        )
        self.assertNotEqual(on_generate, text)
        self.assertTrue(
            any(
                COVERAGE_ID_TOKEN_JOB_MSG in item
                for item in coverage_permission_violations(on_generate, path="coverage.yml")
            )
        )
        on_test = editor.replace(
            "    permissions:\n      contents: read\n    runs-on: ${{ matrix.os }}\n",
            "    permissions: {id-token: write, contents: read}\n    runs-on: ${{ matrix.os }}\n",
            1,
        )
        self.assertNotEqual(on_test, editor)
        self.assertIn(
            COVERAGE_ID_TOKEN_JOB_MSG,
            coverage_permission_violations(on_test, path="aw-editor-tests.yml")[0],
        )
        extra = text.replace(
            "      - name: Upload coverage to Codecov\n",
            "      - name: Extra\n        run: echo no\n      - name: Upload coverage to Codecov\n",
            1,
        )
        self.assertNotEqual(extra, text)
        extra_messages = " ".join(
            coverage_permission_violations(extra, path="coverage.yml")
        )
        self.assertIn(CODECOV_UPLOAD_JOB_MSG, extra_messages)
        self.assertIn(COVERAGE_ID_TOKEN_JOB_MSG, extra_messages)

    def test_kani_and_editor_ci_permission_mutants_are_rejected(self) -> None:
        for name in ("kani.yml", "editor-ci.yml"):
            text = (WORKFLOWS / name).read_text(encoding="utf-8")
            self.assertEqual(
                least_privilege_permission_violations(text, path=name),
                [],
                name,
            )
            missing = text.replace("permissions: {}\n", "", 1)
            self.assertNotEqual(missing, text, name)
            self.assertIn(
                TOP_PERMISSIONS_MSG,
                least_privilege_permission_violations(missing, path=name)[0],
                name,
            )
            widened = text.replace("contents: read", "contents: write", 1)
            self.assertNotEqual(widened, text, name)
            self.assertIn(
                JOB_CONTENTS_READ_MSG,
                least_privilege_permission_violations(widened, path=name)[0],
                name,
            )

    def test_kani_timeout_input_is_a_number(self) -> None:
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err)
        self.assertIsInstance(doc, dict)
        trigger = doc.get(True, doc.get("on"))
        self.assertIsInstance(trigger, dict)
        timeout = trigger["workflow_dispatch"]["inputs"]["timeout"]
        self.assertEqual(timeout.get("type"), "number")
        self.assertEqual(timeout.get("default"), 300)
        for line in text.splitlines():
            if "args:" in line:
                self.assertNotIn("github.event.inputs", line)
        self.assertEqual(text.count("60|120|300|600|900"), 4)
        self.assertEqual(text.count("steps.timeout.outputs.value"), 4)

    def test_codecov_token_secrets_and_with_placement_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_token_violations(text, path="coverage.yml"), [])
        uses_line = (
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        with:\n"
        )
        other_secret = text.replace(
            "          use_oidc: true\n",
            "          use_oidc: true\n          token: ${{ secrets.UPLOAD_CREDENTIAL }}\n",
            1,
        )
        self.assertNotEqual(other_secret, text)
        self.assertNotIn("CODECOV_TOKEN", other_secret)
        self.assertIn(
            CODECOV_TOKEN_MSG,
            codecov_token_violations(other_secret, path="coverage.yml")[0],
        )
        env_secret = text.replace(
            uses_line,
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        env:\n"
            "          NOT_THE_TOKEN_NAME: ${{ secrets.SOME_OTHER_SECRET }}\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(env_secret, text)
        self.assertNotIn("CODECOV_TOKEN", env_secret)
        self.assertIn(
            CODECOV_TOKEN_MSG,
            codecov_token_violations(env_secret, path="coverage.yml")[0],
        )
        reintroduced = text.replace(
            uses_line,
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        env:\n"
            "          CODECOV_TOKEN: literal-value\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(reintroduced, text)
        self.assertIn(
            CODECOV_TOKEN_MSG,
            codecov_token_violations(reintroduced, path="coverage.yml")[0],
        )
        moved_fail = text.replace("          fail_ci_if_error: true\n", "", 1)
        moved_fail = moved_fail.replace(
            uses_line,
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        env:\n"
            "          fail_ci_if_error: true\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(moved_fail, text)
        self.assertIn(
            CODECOV_FAIL_CI_MSG,
            codecov_fail_ci_violations(moved_fail, path="coverage.yml")[0],
        )
        moved_version = text.replace("          version: v11.3.2\n", "", 1)
        moved_version = moved_version.replace(
            uses_line,
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        env:\n"
            "          version: v11.3.2\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(moved_version, text)
        self.assertIn(
            CODECOV_VERSION_MSG,
            codecov_fail_ci_violations(moved_version, path="coverage.yml")[0],
        )
        moved_search = text.replace("          disable_search: true\n", "", 1)
        moved_search = moved_search.replace(
            uses_line,
            "        uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5 # v7.1.1\n"
            "        disable_search: true\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(moved_search, text)
        self.assertIn(
            CODECOV_SEARCH_MSG,
            codecov_explicit_file_violations(moved_search, path="coverage.yml")[0],
        )

    def test_kani_version_and_stable_ref_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_version_violations(text, path="kani.yml"), [])
        removed = text.replace("          kani-version: 0.68.0\n", "", 1)
        self.assertNotEqual(removed, text)
        self.assertTrue(kani_version_violations(removed, path="kani.yml"))
        self.assertIn(KANI_VERSION_MSG, kani_version_violations(removed, path="kani.yml")[0])
        latest = text.replace("kani-version: 0.68.0", "kani-version: latest", 1)
        latest_violations = kani_version_violations(latest, path="kani.yml")
        self.assertEqual(len(latest_violations), 1)
        self.assertIn(KANI_VERSION_MSG, latest_violations[0])
        stable = third_party_action_pin_violations(
            "uses: dtolnay/rust-toolchain@stable\n",
            path="kani.yml",
        )
        self.assertTrue(stable)
        self.assertIn("dtolnay/rust-toolchain@stable", stable[0])
        quoted = third_party_action_pin_violations(
            'uses: "dtolnay/rust-toolchain@stable"\n',
            path="kani.yml",
        )
        self.assertTrue(quoted)
        self.assertIn("dtolnay/rust-toolchain@stable", quoted[0])
        self.assertNotIn('"', quoted[0])
        for path in github_action_surfaces():
            for line in _uses_scan_lines(path.read_text(encoding="utf-8")):
                if "@" not in line or not re.match(r"^\s*(?:-\s+)?uses:\s*\S", line):
                    continue
                ref = line.split("@", 1)[1].split("#", 1)[0].strip()
                self.assertNotEqual(ref, "stable", path)

    def test_llvm_cov_pin_mutants_are_rejected(self) -> None:
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        editor_ci = (WORKFLOWS / "editor-ci.yml").read_text(encoding="utf-8")
        self.assertEqual(llvm_cov_pin_violations(editor, path="aw-editor-tests.yml"), [])
        self.assertEqual(llvm_cov_pin_violations(editor_ci, path="editor-ci.yml"), [])
        unpinned = editor.replace(
            "cargo install cargo-llvm-cov --version 0.9.1 --locked",
            "cargo install cargo-llvm-cov",
            1,
        )
        self.assertNotEqual(unpinned, editor)
        cargo_line = next(
            number
            for number, line in enumerate(unpinned.splitlines(), start=1)
            if "cargo install cargo-llvm-cov" in line
        )
        self.assertEqual(
            llvm_cov_pin_violations(unpinned, path="aw-editor-tests.yml"),
            [f"aw-editor-tests.yml: {cargo_line}: {LLVM_COV_CARGO_MSG}"],
        )
        unlocked = editor.replace(" --locked", "", 1)
        self.assertTrue(llvm_cov_pin_violations(unlocked, path="aw-editor-tests.yml"))
        no_tool = re.sub(
            r"\n\s*with:\n\s*tool: cargo-llvm-cov@0\.9\.1\n",
            "\n",
            editor_ci,
            count=1,
        )
        self.assertNotEqual(no_tool, editor_ci)
        self.assertTrue(llvm_cov_pin_violations(no_tool, path="editor-ci.yml"))
        self.assertIn(
            LLVM_COV_ACTION_MSG,
            llvm_cov_pin_violations(no_tool, path="editor-ci.yml")[0],
        )
        floating = editor_ci.replace("tool: cargo-llvm-cov@0.9.1", "tool: cargo-llvm-cov", 1)
        self.assertIn(
            LLVM_COV_ACTION_MSG,
            llvm_cov_pin_violations(floating, path="editor-ci.yml")[0],
        )

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


if __name__ == "__main__":
    unittest.main()

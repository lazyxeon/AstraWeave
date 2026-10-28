#!/usr/bin/env python3
"""Behavior checks for scripts/ci/affected_crates.py. No network."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "ci" / "affected_crates.py"


def run_files(files: str, scope: str | None = None) -> dict[str, str]:
    cmd = [sys.executable, str(SCRIPT), "--files", files]
    if scope:
        cmd.extend(["--scope", scope])
    out = subprocess.check_output(cmd, cwd=ROOT, text=True)
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

    def test_workflow_action_and_github_scripts_are_full(self) -> None:
        for path in (
            ".github/workflows/ci.yml",
            ".github/actions/rust-ci-setup/action.yml",
            ".github/scripts/install-prebuilt-cargo-tool.sh",
        ):
            result = run_files(path)
            self.assertEqual(result["full"], "true", path)
            self.assertEqual(result["docs_only"], "false", path)
            self.assertEqual(result["reason"], "workflow", path)

    def test_other_github_metadata_is_not_a_full_trigger(self) -> None:
        result = run_files(".github/CODEOWNERS")
        self.assertEqual(result["full"], "false")
        self.assertEqual(result["packages"], "")
        self.assertEqual(result["reason"], "no-crate-files")

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


class AffectedScopeTest(unittest.TestCase):
    def test_lockfile_and_workflow_run_every_scope(self) -> None:
        for scope in ("aravath", "aravath-determinism", "render-device", "net"):
            for files in ("Cargo.lock", "rust-toolchain.toml", ".github/workflows/ci.yml"):
                result = run_files(files, scope)
                self.assertEqual(result["run"], "true", f"{scope} {files}")
                self.assertEqual(result["full"], "true", f"{scope} {files}")

    def test_empty_diff_runs(self) -> None:
        result = run_files("", "net")
        self.assertEqual(result["reason"], "empty-diff")
        self.assertEqual(result["run"], "true")

    def test_docs_only_skips_scopes(self) -> None:
        result = run_files("docs/current/CI_ARCHITECTURE.md", "net")
        self.assertEqual(result["docs_only"], "true")
        self.assertEqual(result["run"], "false")

    def test_prompts_do_not_select_heavy_scopes(self) -> None:
        for scope in ("aravath", "aravath-determinism", "render-device", "net"):
            result = run_files("astraweave-prompts/src/lib.rs", scope)
            self.assertEqual(result["full"], "false", scope)
            self.assertEqual(result["run"], "false", scope)
            self.assertIn("astraweave-prompts", result["packages"])

    def test_direct_crates_select_their_scope(self) -> None:
        net = run_files("astraweave-net/src/lib.rs", "net")
        self.assertEqual(net["run"], "true")
        render = run_files("astraweave-render/src/lib.rs", "render-device")
        self.assertEqual(render["run"], "true")
        ecs = run_files("astraweave-ecs/src/lib.rs", "render-device")
        self.assertEqual(ecs["run"], "true")
        golden = run_files("aravath_runtime/src/lib.rs", "aravath-determinism")
        self.assertEqual(golden["run"], "true")
        save = run_files("persistence/aw-save/src/lib.rs", "aravath")
        self.assertEqual(save["run"], "true")

    def test_core_runs_net_scope_and_proto_does_not(self) -> None:
        # astraweave-net sits downstream of core, so a core edit still runs
        # the all-features net job. aw-net-proto is not a dependency of
        # astraweave-net; CI's own affected set still tests the proto crate.
        core = run_files("astraweave-core/src/lib.rs", "net")
        self.assertEqual(core["run"], "true")
        proto = run_files("net/aw-net-proto/src/lib.rs", "net")
        self.assertEqual(proto["run"], "false")
        self.assertIn("aw-net-proto", proto["packages"])
        self.assertNotIn("astraweave-net", proto["packages"].split(","))

    def test_render_change_runs_aravath_windows_scope(self) -> None:
        result = run_files("astraweave-render/src/lib.rs", "aravath")
        self.assertEqual(result["run"], "true")

    def test_unknown_scope_fails(self) -> None:
        proc = subprocess.run(
            [sys.executable, str(SCRIPT), "--files", "Cargo.lock", "--scope", "nope"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertNotEqual(proc.returncode, 0)


if __name__ == "__main__":
    unittest.main()

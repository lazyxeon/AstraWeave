#!/usr/bin/env python3
"""Behavior checks for scripts/ci/affected_crates.py. No network."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "ci" / "affected_crates.py"
sys.path.insert(0, str(SCRIPT.parent))
import affected_crates  # noqa: E402


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

    def test_rust_file_moved_onto_docs_is_not_docs_only(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = self._repo(Path(tmp))
            src = repo / "astraweave-math" / "src" / "lib.rs"
            src.parent.mkdir(parents=True)
            src.write_text(
                "".join(f"pub fn item_{i}() -> i32 {{ {i} }}\n" for i in range(200)),
                encoding="utf-8",
            )
            self._git(repo, ["add", "astraweave-math/src/lib.rs"])
            self._git(repo, ["commit", "-qm", "add"])
            dest = repo / "docs" / "alpha.md"
            dest.parent.mkdir()
            self._git(repo, ["mv", "astraweave-math/src/lib.rs", "docs/alpha.md"])
            dest.write_text(
                dest.read_text(encoding="utf-8") + "moved\n",
                encoding="utf-8",
            )
            self._git(repo, ["add", "-A"])
            self._git(repo, ["commit", "-qm", "move onto docs"])

            followed = self._names(repo, "HEAD^...HEAD")
            self.assertEqual(followed, ["docs/alpha.md"])
            skipped = run_files(",".join(followed))
            self.assertEqual(skipped["docs_only"], "true")

            files = affected_crates.diff_files(repo, "HEAD^", "HEAD")
            self.assertCountEqual(
                files,
                ["astraweave-math/src/lib.rs", "docs/alpha.md"],
            )
            result = run_files(",".join(files))
            self.assertEqual(result["docs_only"], "false")
            self.assertEqual(result["full"], "false")
            self.assertIn("astraweave-math", result["packages"].split(","))

    def test_cross_crate_rename_selects_both_crates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = self._repo(Path(tmp))
            src = repo / "astraweave-math" / "src" / "lib.rs"
            src.parent.mkdir(parents=True)
            src.write_text("pub fn item() -> i32 { 1 }\n", encoding="utf-8")
            self._git(repo, ["add", "astraweave-math/src/lib.rs"])
            self._git(repo, ["commit", "-qm", "add"])
            dest = repo / "astraweave-prompts" / "src" / "lib.rs"
            dest.parent.mkdir(parents=True)
            self._git(repo, ["mv", "astraweave-math/src/lib.rs", "astraweave-prompts/src/lib.rs"])
            self._git(repo, ["commit", "-qm", "move crate"])

            followed = self._names(repo, "HEAD^...HEAD")
            self.assertEqual(followed, ["astraweave-prompts/src/lib.rs"])
            destination_only = run_files(",".join(followed))
            self.assertNotIn("astraweave-math", destination_only["packages"].split(","))

            files = affected_crates.diff_files(repo, "HEAD^", "HEAD")
            self.assertCountEqual(
                files,
                ["astraweave-math/src/lib.rs", "astraweave-prompts/src/lib.rs"],
            )
            result = run_files(",".join(files))
            packages = set(result["packages"].split(","))
            self.assertIn("astraweave-math", packages)
            self.assertIn("astraweave-prompts", packages)
            self.assertEqual(result["docs_only"], "false")

    def _repo(self, tmp: Path) -> Path:
        repo = tmp / "repo"
        repo.mkdir()
        self._git(repo, ["init", "-q", "-b", "main"])
        self._git(repo, ["config", "user.email", "detect@example.com"])
        self._git(repo, ["config", "user.name", "detect"])
        self._git(repo, ["config", "diff.renames", "true"])
        return repo

    def test_merge_parent_matches_a_base_that_moved_forward(self) -> None:
        # GitHub's pull_request checkout is a merge commit. Its first parent
        # is the base commit that was merged. Three-dot against that parent
        # stays the pull request diff when the base branch later gains
        # commits. Depth 2 is enough to see that parent.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp) / "repo"
            repo.mkdir()
            self._git(repo, ["init", "-q", "-b", "main"])
            self._git(repo, ["config", "user.email", "detect@example.com"])
            self._git(repo, ["config", "user.name", "detect"])
            (repo / "README.md").write_text("base\n", encoding="utf-8")
            self._git(repo, ["add", "README.md"])
            self._git(repo, ["commit", "-qm", "base"])
            base = self._rev(repo, "HEAD")
            self._git(repo, ["checkout", "-q", "-b", "feature"])
            crate = repo / "astraweave-math" / "src"
            crate.mkdir(parents=True)
            (crate / "lib.rs").write_text("fn feature() {}\n", encoding="utf-8")
            self._git(repo, ["add", "astraweave-math/src/lib.rs"])
            self._git(repo, ["commit", "-qm", "feature"])
            feature = self._rev(repo, "HEAD")
            self._git(repo, ["checkout", "-q", "main"])
            (repo / "docs" / "later.md").parent.mkdir()
            (repo / "docs" / "later.md").write_text("later\n", encoding="utf-8")
            self._git(repo, ["add", "docs/later.md"])
            self._git(repo, ["commit", "-qm", "main moves"])
            moved = self._rev(repo, "HEAD")
            tree = self._rev(repo, f"{feature}^{{tree}}")
            merge = subprocess.check_output(
                ["git", "commit-tree", tree, "-p", base, "-p", feature, "-m", "merge"],
                cwd=repo,
                text=True,
            ).strip()
            self._git(repo, ["update-ref", "refs/pull/1/merge", merge])

            parent_files = self._names(repo, f"{merge}^1...{merge}")
            moved_files = self._names(repo, f"{moved}...{merge}")
            self.assertEqual(parent_files, ["astraweave-math/src/lib.rs"])
            self.assertEqual(parent_files, moved_files)
            self.assertEqual(
                run_files(",".join(parent_files)),
                run_files(",".join(moved_files)),
            )
            classified = run_files(",".join(parent_files))
            self.assertEqual(classified["full"], "false")
            self.assertEqual(classified["docs_only"], "false")
            self.assertIn("astraweave-math", classified["packages"].split(","))

            shallow = Path(tmp) / "shallow"
            shallow.mkdir()
            self._git(shallow, ["init", "-q"])
            self._git(
                shallow,
                ["fetch", "-q", "--depth=2", "--no-tags", str(repo), "refs/pull/1/merge"],
            )
            self._git(shallow, ["checkout", "-q", "--detach", "FETCH_HEAD"])
            self.assertEqual(self._rev(shallow, "HEAD"), merge)
            self.assertEqual(self._rev(shallow, "HEAD^1"), base)
            self.assertEqual(self._names(shallow, "HEAD^1...HEAD"), parent_files)
            self.assertEqual(self._names(shallow, ["HEAD^1", "HEAD"]), parent_files)

    def _git(self, cwd: Path, args: list[str]) -> None:
        subprocess.check_call(["git", *args], cwd=cwd, stdout=subprocess.DEVNULL)

    def _rev(self, cwd: Path, rev: str) -> str:
        return subprocess.check_output(
            ["git", "rev-parse", rev], cwd=cwd, text=True
        ).strip()

    def _names(self, cwd: Path, spec: str | list[str]) -> list[str]:
        args = ["git", "diff", "--name-only"]
        if isinstance(spec, list):
            args.extend(spec)
        else:
            args.append(spec)
        out = subprocess.check_output(args, cwd=cwd, text=True)
        return [line for line in out.splitlines() if line.strip()]


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

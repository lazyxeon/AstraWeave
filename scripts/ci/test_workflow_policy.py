#!/usr/bin/env python3
"""Static checks for the CI tiering rules. Does not call GitHub."""

from __future__ import annotations

import inspect
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
sys.path.insert(0, str(ROOT / "scripts" / "ci"))
import auto_retry  # noqa: E402
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

    def test_auto_retry_has_no_top_level_write_permissions(self) -> None:
        text = AUTO_RETRY_WORKFLOW.read_text(encoding="utf-8")
        self.assertEqual(root_level_write_permission_offenders(text), [])
        leaked = text.replace(
            "permissions: {}\n",
            "permissions:\n  actions: write\n",
            1,
        )
        self.assertNotEqual(leaked, text)
        self.assertEqual(root_level_write_permission_offenders(leaked), ["actions:write"])

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


AUTO_RETRY_WORKFLOW = WORKFLOWS / "auto-retry.yml"
AUTO_RETRY_ALLOWLIST = ROOT / ".github" / "flaky-jobs.json"
AUTO_RETRY_PERMISSIONS = {
    "actions": "write",
    "issues": "write",
    "pull-requests": "write",
}
EXPECTED_FLAKY_JOBS = {
    ("Performance Benchmarks", "Performance Benchmarks (shard 10)"),
    ("Performance Benchmarks", "Aggregate Benchmark Results"),
}


def _mutant_auto_retry(old: str, new: str) -> dict[str, object]:
    import sys
    import types

    source = (ROOT / "scripts" / "ci" / "auto_retry.py").read_text(encoding="utf-8")
    if old not in source:
        raise AssertionError(f"mutant anchor missing: {old!r}")
    mutated = source.replace(old, new, 1)
    if mutated == source:
        raise AssertionError("mutant did not change the source")
    name = "auto_retry_mutant"
    module = types.ModuleType(name)
    module.__dict__["__file__"] = str(ROOT / "scripts" / "ci" / "auto_retry.py")
    sys.modules[name] = module
    exec(compile(mutated, name, "exec"), module.__dict__)
    return module.__dict__


def _permission_value_is_write(value: str) -> bool:
    normalized = value.strip().lower()
    if normalized in ("read", "none"):
        return False
    return True


def _jobs_line_index(lines: list[str]) -> int | None:
    for index, line in enumerate(lines):
        if line == "jobs:" or line.startswith("jobs:"):
            return index
    return None


def root_level_write_permission_offenders(text: str) -> list[str]:
    """Top-level permissions before `jobs:` must not grant write (or bool true)."""
    lines = text.splitlines()
    jobs = _jobs_line_index(lines)
    head = lines[:jobs] if jobs is not None else lines
    offenders: list[str] = []
    index = 0
    while index < len(head):
        match = re.match(r"^permissions:\s*(.*)\s*$", head[index])
        if not match:
            index += 1
            continue
        rest = match.group(1).strip()
        if rest == "{}":
            index += 1
            continue
        if rest:
            index += 1
            continue
        index += 1
        while index < len(head):
            line = head[index]
            if not line.strip():
                index += 1
                continue
            if not line.startswith(" "):
                break
            current = len(line) - len(line.lstrip(" "))
            if current == 0:
                break
            item = re.match(r"^\s+([A-Za-z0-9-]+):\s*(\S+)\s*$", line)
            if not item:
                break
            key, val = item.group(1), item.group(2)
            if _permission_value_is_write(val):
                offenders.append(f"{key}:{val}")
            index += 1
    return offenders


def extract_job_block(text: str, job_key: str) -> str:
    """Return the YAML lines for one `jobs.<job_key>` entry (for permission_maps)."""
    lines = text.splitlines()
    prefix = f"  {job_key}:"
    start = -1
    for index, line in enumerate(lines):
        if line == prefix or line.startswith(prefix + " "):
            start = index
            break
    if start < 0:
        raise AssertionError(f"missing job {job_key!r}")
    block_lines: list[str] = []
    for line in lines[start + 1 :]:
        if re.match(r"^  [A-Za-z0-9_.-]+:\s*", line):
            break
        block_lines.append(line)
    return "\n".join(block_lines)


def permission_maps(text: str) -> list[dict[str, str]]:
    lines = text.splitlines()
    maps: list[dict[str, str]] = []
    index = 0
    while index < len(lines):
        match = re.match(r"^(\s*)permissions:\s*(.*)\s*$", lines[index])
        if not match:
            index += 1
            continue
        rest = match.group(2).strip()
        if rest == "{}":
            maps.append({})
            index += 1
            continue
        if rest:
            index += 1
            continue
        indent = len(match.group(1))
        block: dict[str, str] = {}
        index += 1
        while index < len(lines):
            line = lines[index]
            if not line.strip():
                index += 1
                continue
            current = len(line) - len(line.lstrip(" "))
            if current <= indent:
                break
            item = re.match(r"^\s+([A-Za-z0-9-]+):\s*(\S+)\s*$", line)
            if not item:
                break
            block[item.group(1)] = item.group(2)
            index += 1
        maps.append(block)
    return maps


def run_script_bodies(text: str) -> list[str]:
    lines = text.splitlines()
    bodies: list[str] = []
    index = 0
    while index < len(lines):
        folded = re.match(r"^(\s*)run:\s*\|[+-]?\s*$", lines[index])
        inline = re.match(r"^(\s*)run:\s*(\S.*)$", lines[index])
        if folded:
            indent = len(folded.group(1))
            index += 1
            buff: list[str] = []
            while index < len(lines):
                line = lines[index]
                if line.strip() and (len(line) - len(line.lstrip(" "))) <= indent:
                    break
                buff.append(line)
                index += 1
            bodies.append("\n".join(buff))
            continue
        if inline:
            bodies.append(inline.group(2))
        index += 1
    return bodies


def job_name_declared(workflow_text: str, job: str) -> bool:
    for line in workflow_text.splitlines():
        match = re.match(r"^\s*name:\s*(.+?)\s*$", line)
        if not match:
            continue
        declared = match.group(1).strip().strip("\"'")
        if declared == job:
            return True
        if "${{" not in declared:
            continue
        pattern = re.escape(declared)
        pattern = re.sub(r"\\\$\\\{\\\{.*?\\\}\\\}", ".*", pattern)
        if re.fullmatch(pattern, job):
            return True
    return False


def workflow_text_by_display_name() -> dict[str, str]:
    found: dict[str, str] = {}
    name_re = re.compile(r"^name:\s*(.+?)\s*$")
    for text in workflow_texts().values():
        for line in text.splitlines():
            match = name_re.match(line)
            if not match:
                continue
            value = match.group(1).strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
                value = value[1:-1]
            found[value] = text
            break
    return found


def auto_retry_violations(text: str, allow_workflows: set[str]) -> list[str]:
    violations: list[str] = []
    if "pull_request_target" in text:
        violations.append("pull_request_target")
    if "actions/checkout" in text:
        violations.append("checkout")
    if re.search(
        r"ref:\s*\$\{\{\s*github\.event\.workflow_run\.head_sha",
        text,
    ):
        violations.append("checkout-pr-head")
    retry_job = extract_job_block(text, "retry")
    if permission_maps(retry_job) != [AUTO_RETRY_PERMISSIONS]:
        violations.append(f"permissions {permission_maps(retry_job)}")
    if root_level_write_permission_offenders(text):
        violations.append(f"top-level-write {root_level_write_permission_offenders(text)}")
    if "continue-on-error" in text:
        violations.append("continue-on-error")
    if "|| true" in text or re.search(r"\|\|\s*exit\s+0\b", text):
        violations.append("ignored-failure")
    if "github.event.workflow_run.conclusion == 'failure'" not in text:
        violations.append("missing-failure-conclusion")
    if "github.event.workflow_run.conclusion == 'timed_out'" not in text:
        violations.append("timed-out-candidate")
    attempt = re.search(r"github\.event\.workflow_run\.run_attempt < (\d+)", text)
    if not attempt or int(attempt.group(1)) != 3:
        violations.append("attempt-cap")
    if (
        "github.event.workflow_run.head_repository.full_name == github.repository"
        not in text
    ):
        violations.append("fork-repository")
    watched = nightly_failure_watched_workflows(text)
    if set(watched) != allow_workflows:
        violations.append(f"workflows {watched}")
    known = workflow_display_names()
    for name in watched:
        if name not in known:
            violations.append(f"unknown-workflow {name}")
    if "name: Record the failure and retry allowlisted jobs" not in text:
        violations.append("record-step")
    if "auto_retry.py" not in text:
        violations.append("record-script")
    for body in run_script_bodies(text):
        if "${{" in body:
            violations.append("untrusted-interpolation")
            break
    if 'os.environ["REPO"]' not in text or 'os.environ["SCRIPT_SHA"]' not in text:
        violations.append("pinned-fetch")
    if 're.fullmatch(r"[0-9a-fA-F]{40}"' not in text or "/{sha}/" not in text:
        violations.append("sha-pin")
    if "github.sha" not in text:
        violations.append("unpinned-fetch")
    if 'host != "raw.githubusercontent.com"' not in text or "refusing redirect" not in text:
        violations.append("redirect-guard")
    if '".." in rel.split("/")' not in text:
        violations.append("path-traversal-check")
    for host in re.findall(r"https://([^/\s\"']+)", text):
        bare = host.split("@")[-1].split(":")[0]
        if bare != "raw.githubusercontent.com":
            violations.append(f"raw-host {bare}")
            break
    return violations


def benchmark_artifact_violations(text: str) -> list[str]:
    violations: list[str] = []
    if re.search(r"(?m)^\s*overwrite\s*:", text):
        violations.append("overwrite")
    names = re.findall(
        r"name: benchmark-results-\$\{\{ github\.sha \}\}[^\n]*",
        text,
    )
    if len(names) != 2:
        violations.append("artifact-name-count")
    for name in names:
        if "${{ github.run_attempt }}" not in name:
            violations.append("attempt-name")
    if "benchmark-results-${{ github.sha }}-shard-*" not in text:
        violations.append("download-pattern")
    if "--prune-older-shard-attempts benchmark-shards" not in text:
        violations.append("prune")
    if "*-shard-${shard_index}-attempt-*" not in text:
        violations.append("attempt-glob")
    if text.count("success() &&") < 2:
        violations.append("upload-success-guard")
    if text.count("continue-on-error:") != 2:
        violations.append("continue-on-error")
    return violations


def _allow(*pairs: tuple[str, str]) -> list[auto_retry.AllowJob]:
    return [
        auto_retry.AllowJob(workflow, job, "https://github.com/lazyxeon/AstraWeave/actions/runs/1")
        for workflow, job in pairs
    ]


LOG_143 = "\n".join(
    [
        "2026-10-30T13:24:47.9573516Z ##[error]Process completed with exit code 143.",
    ]
)
LOG_LOST_RUNNER = (
    "2026-10-30T13:24:47.9569595Z ##[error]The runner has received a shutdown signal. "
    "This can happen when the runner service is stopped, or a manually started runner is canceled."
)
LOG_ARTIFACT = (
    "2026-10-30T17:59:23.4758365Z ##[error]Unable to download artifact(s): "
    "Unable to download and extract artifact: Artifact download failed after 5 retries."
)
LOG_MISSING = "Missing shard artifacts:\nshard-10"
LOG_CRATES = "error: failed to download from `https://index.crates.io/config.json`"
LOG_TIMEOUT = "curl: (28) Connection timed out after 30000 milliseconds"
LOG_CACHE = "##[warning]Failed to restore cache entry. connect ETIMEDOUT"
LOG_CACHE_RESTORE = "##[warning]Failed to restore: connect ETIMEDOUT 10.0.0.1:443"
LOG_PACKAGE = "\n".join(
    [
        "Failed packages: 2",
        "Healthy: false",
        "##[error]Process completed with exit code 1.",
    ]
)
LOG_BOILERPLATE = "\n".join(
    [
        "SCCACHE_IDLE_TIMEOUT: 0",
        "CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse",
        "# https://rust-lang.zulipchat.com/#narrow/stream/246057-t-cargo/topic/timeout.20investigation",
        "##[start-action display=Work around spurious network errors in curl 8.0",
        "cache-all-crates: true",
        'echo "Missing shard artifacts:"',
        "Artifact download completed successfully.",
        "Artifact download URL: https://github.com/example/artifacts/1",
        "Process completed with exit code 1.",
        "also writes hits the service rate limit (cache write errors) and",
        "Updating crates.io index",
    ]
)
AGGREGATE_DOWNLOAD_STEPS = (
    ("Download benchmark shards", "success"),
    ("Upload combined benchmark artifact", "success"),
    ("Strict threshold validation (main branch only)", "skipped"),
    ("Enforce benchmark shard health", "failure"),
)


def _observed(
    names: list[str],
    logs: dict[str, str] | None = None,
    steps: dict[str, tuple[tuple[str, str], ...]] | None = None,
    cls: type | None = None,
) -> list:
    kind = cls or auto_retry.ObservedJob
    logs = logs or {}
    steps = steps or {}
    return [kind(name, logs.get(name, ""), tuple(steps.get(name, ()))) for name in names]


def _decide(
    workflow: str,
    failed: list[str],
    allow: list[auto_retry.AllowJob],
    attempt: int = 1,
    conclusion: str = "failure",
    logs: dict[str, str] | None = None,
    steps: dict[str, tuple[tuple[str, str], ...]] | None = None,
) -> auto_retry.Decision:
    return auto_retry.decide(
        workflow,
        _observed(failed, logs, steps),
        allow,
        attempt,
        conclusion,
    )


def _mutant_decide(
    namespace: dict[str, object],
    workflow: str,
    failed: list[str],
    allow: list,
    attempt: int = 1,
    conclusion: str = "failure",
    logs: dict[str, str] | None = None,
    steps: dict[str, tuple[tuple[str, str], ...]] | None = None,
):
    decide = namespace["decide"]
    observed = namespace["ObservedJob"]
    assert callable(decide)
    return decide(
        workflow,
        _observed(failed, logs, steps, cls=observed),
        allow,
        attempt,
        conclusion,
    )


class AutoRetryPolicyTest(unittest.TestCase):
    def test_checked_in_allowlist_matches_proven_flakes(self) -> None:
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        pairs = {(item.workflow, item.job) for item in allow}
        self.assertEqual(pairs, EXPECTED_FLAKY_JOBS)
        for item in allow:
            self.assertIn("actions/runs/", item.evidence)
            self.assertIn("issues/272", item.evidence)
            self.assertFalse(auto_retry.job_denied(item.workflow, item.job))
        shard = next(item for item in allow if item.job.endswith("(shard 10)"))
        aggregate = next(item for item in allow if item.job == "Aggregate Benchmark Results")
        self.assertIn("41059157117", shard.evidence)
        self.assertIn("41859768393", shard.evidence)
        self.assertIn("41081301546", aggregate.evidence)
        self.assertIn("41076516458", aggregate.evidence)
        declared = workflow_text_by_display_name()
        for item in allow:
            self.assertIn(item.workflow, declared)
            self.assertTrue(
                job_name_declared(declared[item.workflow], item.job),
                item.job,
            )

    def test_allowlist_only_retries_when_every_failed_job_is_listed(self) -> None:
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        shard = "Performance Benchmarks (shard 10)"
        aggregate = "Aggregate Benchmark Results"
        shard_logs = {shard: LOG_143}
        both_logs = {shard: LOG_143, aggregate: LOG_MISSING}
        self.assertTrue(
            _decide("Performance Benchmarks", [shard], allow, logs=shard_logs).retry
        )
        self.assertTrue(
            _decide("Performance Benchmarks", [shard, aggregate], allow, logs=both_logs).retry
        )
        cited = _decide(
            "Performance Benchmarks",
            [aggregate],
            allow,
            logs={aggregate: LOG_ARTIFACT},
            steps={aggregate: AGGREGATE_DOWNLOAD_STEPS},
        )
        self.assertTrue(cited.retry)
        self.assertEqual(cited.signatures, ("artifact-download",))
        other = "Performance Benchmarks (shard 11)"
        mixed = _decide(
            "Performance Benchmarks",
            [shard, other],
            allow,
            logs={shard: LOG_143, other: LOG_143},
        )
        self.assertFalse(mixed.retry)
        self.assertEqual(mixed.reason, "not-allowlisted")
        aw_build = _decide(
            "Performance Benchmarks",
            ["Benchmarks (aw_build)"],
            allow,
            logs={"Benchmarks (aw_build)": LOG_143},
        )
        self.assertFalse(aw_build.retry)
        self.assertEqual(aw_build.reason, "not-allowlisted")
        wrong_workflow = _decide(
            "Benchmark Regression Alerts",
            [aggregate],
            allow,
            logs={aggregate: LOG_ARTIFACT},
        )
        self.assertFalse(wrong_workflow.retry)
        self.assertEqual(wrong_workflow.reason, "not-allowlisted")
        nameless = _decide("Performance Benchmarks", [shard], allow)
        self.assertFalse(nameless.retry)
        self.assertEqual(nameless.reason, "no-signature")

    def test_denylist_wins_even_when_the_job_is_allowlisted(self) -> None:
        cases = [
            ("Performance Benchmarks", "Cargo Deny Check"),
            ("Performance Benchmarks", "cargo-deny"),
            ("Performance Benchmarks", "cargo_deny"),
            ("Performance Benchmarks", "cargo audit"),
            ("Performance Benchmarks", "cargo-audit"),
            ("Performance Benchmarks", "Security Audit (cargo-audit)"),
            ("Security Audit", "rollup"),
            ("Dependency Management & Security", "Security Audit & Dependency Analysis"),
            ("determinism-replay", "unrelated"),
            ("Performance Benchmarks", "hash (ubuntu-latest)"),
            ("Performance Benchmarks", "hash (windows-latest)"),
            ("Performance Benchmarks", "hash (macos-latest)"),
            ("Performance Benchmarks", "hash"),
            ("Aravath determinism", "Replay hash (ubuntu-latest)"),
            ("Performance Benchmarks", "compare"),
            ("Performance Benchmarks", "Workflow policy"),
            ("Performance Benchmarks", "Workflow policy tests"),
            ("Comprehensive CI Pipeline", "Gate (fmt, clippy, check)"),
            ("Comprehensive CI Pipeline", "Gate"),
            ("Comprehensive CI Pipeline", "Gate("),
            ("Comprehensive CI Pipeline", "gate extra"),
            ("Comprehensive CI Pipeline", "Ga\u200bte"),
            ("Comprehensive CI Pipeline", "G\u0430te"),
            ("Comprehensive CI Pipeline", "\uff27\uff41\uff54\uff45"),
            ("determinism replay", "unrelated"),
            ("Comprehensive CI Pipeline", "PR Rust checks"),
            ("Comprehensive CI Pipeline", "Detect affected crates"),
            ("Comprehensive CI Pipeline", "Test (ubuntu, build archive)"),
            ("Comprehensive CI Pipeline", "Test (ubuntu, 1/3)"),
            ("Comprehensive CI Pipeline", "Test (windows, 2/3)"),
            ("Comprehensive CI Pipeline", "Minimum Supported Rust Version"),
            ("Sanitizers", "Determinism Validation"),
        ]
        for workflow, job in cases:
            with self.subTest(workflow=workflow, job=job):
                self.assertTrue(auto_retry.job_denied(workflow, job), job)
                allow = _allow((workflow, job))
                decision = _decide(workflow, [job], allow, logs={job: LOG_143})
                self.assertFalse(decision.retry)
                self.assertEqual(decision.reason, "denylist")
        self.assertFalse(auto_retry.job_denied("Performance Benchmarks", "gatekeeper"))
        self.assertFalse(auto_retry.job_denied("Performance Benchmarks", "comparison"))
        self.assertFalse(
            auto_retry.job_denied("Performance Benchmarks", "Aggregate Benchmark Results")
        )
        self.assertFalse(
            auto_retry.job_denied("Performance Benchmarks", "Performance Benchmarks (shard 10)")
        )

    def test_denylist_rules_are_load_bearing(self) -> None:
        mutants = [
            (
                "return True  # deny:cargo-deny",
                "return False  # deny:cargo-deny",
                "Performance Benchmarks",
                "Cargo Deny Check",
            ),
            (
                "return True  # deny:cargo-audit",
                "return False  # deny:cargo-audit",
                "Performance Benchmarks",
                "cargo-audit",
            ),
            (
                "return True  # deny:security-audit",
                "return False  # deny:security-audit",
                "Security Audit",
                "rollup",
            ),
            (
                "return True  # deny:determinism-replay",
                "return False  # deny:determinism-replay",
                "determinism-replay",
                "unrelated",
            ),
            (
                "return True  # deny:replay-hash",
                "return False  # deny:replay-hash",
                "Aravath determinism",
                "Replay hash (ubuntu-latest)",
            ),
            (
                "return True  # deny:hash-job",
                "return False  # deny:hash-job",
                "Performance Benchmarks",
                "hash (ubuntu-latest)",
            ),
            (
                "return True  # deny:compare",
                "return False  # deny:compare",
                "Performance Benchmarks",
                "compare",
            ),
            (
                "return True  # deny:workflow-policy",
                "return False  # deny:workflow-policy",
                "Comprehensive CI Pipeline",
                "Workflow policy tests",
            ),
            (
                "return True  # deny:gate",
                "return False  # deny:gate",
                "Comprehensive CI Pipeline",
                "Gate (fmt, clippy, check)",
            ),
            (
                "return True  # deny:pr-rust",
                "return False  # deny:pr-rust",
                "Comprehensive CI Pipeline",
                "PR Rust checks",
            ),
            (
                "return True  # deny:detect-affected",
                "return False  # deny:detect-affected",
                "Comprehensive CI Pipeline",
                "Detect affected crates",
            ),
            (
                "return True  # deny:msrv",
                "return False  # deny:msrv",
                "Comprehensive CI Pipeline",
                "Minimum Supported Rust Version",
            ),
            (
                "return True  # deny:required-test",
                "return False  # deny:required-test",
                "Comprehensive CI Pipeline",
                "Test (ubuntu, 1/3)",
            ),
            (
                "return True  # deny:determinism-validation",
                "return False  # deny:determinism-validation",
                "Sanitizers",
                "Determinism Validation",
            ),
        ]
        for old, new, workflow, job in mutants:
            with self.subTest(job=job):
                allow = _allow((workflow, job))
                self.assertFalse(_decide(workflow, [job], allow, logs={job: LOG_143}).retry)
                namespace = _mutant_auto_retry(old, new)
                denied = namespace["job_denied"]
                self.assertTrue(callable(denied))
                self.assertFalse(denied(workflow, job))
                decision = _mutant_decide(
                    namespace, workflow, [job], allow, logs={job: LOG_143}
                )
                self.assertTrue(decision.retry)

    def test_deny_wins_mutant_retries_a_listed_gate(self) -> None:
        workflow = "Comprehensive CI Pipeline"
        job = "Gate (fmt, clippy, check)"
        real = _decide(workflow, [job], _allow((workflow, job)))
        self.assertEqual(real.reason, "denylist")
        namespace = _mutant_auto_retry(
            "denied = True  # deny-wins",
            "denied = False  # deny-wins",
        )
        allow = _allow((workflow, job))
        mutant = _mutant_decide(namespace, workflow, [job], allow, logs={job: LOG_143})
        self.assertTrue(mutant.retry)

    def test_allowlist_check_is_load_bearing(self) -> None:
        shard = "Performance Benchmarks (shard 10)"
        other = "Performance Benchmarks (shard 2)"
        allow = _allow(("Performance Benchmarks", shard))
        real = _decide("Performance Benchmarks", [shard, other], allow)
        self.assertEqual(real.reason, "not-allowlisted")
        logs = {shard: LOG_143, other: LOG_143}
        real = _decide("Performance Benchmarks", [shard, other], allow, logs=logs)
        self.assertEqual(real.reason, "not-allowlisted")
        namespace = _mutant_auto_retry(
            "if (workflow, job.name) not in allowed:  # allowlist-only",
            "if False:  # allowlist-only",
        )
        mutant = _mutant_decide(
            namespace,
            "Performance Benchmarks",
            [shard, other],
            allow,
            logs=logs,
        )
        self.assertTrue(mutant.retry)

    def test_attempt_cap_and_failure_conclusion(self) -> None:
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        shard = ["Performance Benchmarks (shard 10)"]
        logs = {shard[0]: LOG_143}
        self.assertEqual(auto_retry.MAX_ATTEMPT, 3)
        self.assertTrue(
            _decide("Performance Benchmarks", shard, allow, attempt=1, logs=logs).retry
        )
        self.assertTrue(
            _decide("Performance Benchmarks", shard, allow, attempt=2, logs=logs).retry
        )
        capped = _decide("Performance Benchmarks", shard, allow, attempt=3, logs=logs)
        self.assertFalse(capped.retry)
        self.assertEqual(capped.reason, "attempt-cap")
        self.assertEqual(
            _decide("Performance Benchmarks", shard, allow, attempt=0, logs=logs).reason,
            "attempt-cap",
        )
        for conclusion in ("success", "cancelled", "skipped"):
            decision = _decide(
                "Performance Benchmarks",
                shard,
                allow,
                conclusion=conclusion,
                logs=logs,
            )
            self.assertFalse(decision.retry)
            self.assertEqual(decision.reason, "not-failure")
        timed = _decide(
            "Performance Benchmarks",
            shard,
            allow,
            conclusion="timed_out",
            logs=logs,
        )
        self.assertTrue(timed.retry)
        timed_plain = _decide(
            "Performance Benchmarks", shard, allow, conclusion="timed_out"
        )
        self.assertEqual(timed_plain.reason, "no-signature")
        empty = _decide("Performance Benchmarks", [], allow)
        self.assertEqual(empty.reason, "no-failed-jobs")
        namespace = _mutant_auto_retry(
            "if attempt >= MAX_ATTEMPT:  # attempt-cap",
            "if False:  # attempt-cap",
        )
        self.assertTrue(
            _mutant_decide(
                namespace,
                "Performance Benchmarks",
                shard,
                allow,
                attempt=3,
                logs=logs,
            ).retry
        )
        namespace = _mutant_auto_retry(
            "if conclusion not in CANDIDATE_CONCLUSIONS:  # failure-only",
            "if False:  # failure-only",
        )
        self.assertTrue(
            _mutant_decide(
                namespace,
                "Performance Benchmarks",
                shard,
                allow,
                conclusion="success",
                logs=logs,
            ).retry
        )
        namespace = _mutant_auto_retry(
            "if not failed_jobs:  # empty-failed",
            "if False:  # empty-failed",
        )
        self.assertTrue(
            _mutant_decide(
                namespace, "Performance Benchmarks", [], allow, conclusion="failure"
            ).retry
        )

    def test_failed_job_names_ignore_success_and_cancelled(self) -> None:
        payload = [
            {
                "jobs": [
                    {"name": "Performance Benchmarks (shard 10)", "conclusion": "failure"},
                    {"name": "Performance Benchmarks (shard 0)", "conclusion": "success"},
                    {"name": "Benchmarks (aw_build)", "conclusion": "cancelled"},
                    {"name": "  ", "conclusion": "failure"},
                ]
            }
        ]
        self.assertEqual(
            auto_retry.failed_job_names(payload),
            ["Performance Benchmarks (shard 10)"],
        )
        timed = [
            {
                "jobs": [
                    {
                        "id": 7,
                        "name": "Performance Benchmarks (shard 10)",
                        "conclusion": "timed_out",
                        "steps": [
                            {"name": "Run benchmarks", "conclusion": "failure"},
                        ],
                    },
                    {"name": "Benchmarks (aw_build)", "conclusion": "cancelled"},
                ]
            }
        ]
        observed = auto_retry.candidate_jobs(timed)
        self.assertEqual(
            [(job.name, job.conclusion, job.job_id, job.steps) for job in observed],
            [
                (
                    "Performance Benchmarks (shard 10)",
                    "timed_out",
                    7,
                    (("Run benchmarks", "failure"),),
                )
            ],
        )
        self.assertEqual(auto_retry.failed_job_names(timed), [])

    def test_record_contains_run_attempt_jobs_and_conclusion(self) -> None:
        record = auto_retry.build_record(
            "https://github.com/lazyxeon/AstraWeave/actions/runs/41081301546",
            1,
            ["Aggregate Benchmark Results", "Performance Benchmarks (shard 10)"],
            "failure",
        )
        self.assertIn("https://github.com/lazyxeon/AstraWeave/actions/runs/41081301546", record)
        self.assertIn("- Attempt: 1", record)
        self.assertIn("Aggregate Benchmark Results", record)
        self.assertIn("Performance Benchmarks (shard 10)", record)
        self.assertIn("- Conclusion: failure", record)
        self.assertIn("- Signatures: (none)", record)
        self.assertIn(auto_retry.TRACKING_MARKER, record)
        self.assertEqual(record.splitlines()[0], auto_retry.BOT_MARKER)
        signed = auto_retry.build_record(
            "https://github.com/lazyxeon/AstraWeave/actions/runs/41081301546",
            1,
            ["Aggregate Benchmark Results"],
            "failure",
            ("artifact-download",),
        )
        self.assertIn("- Signatures: artifact-download", signed)
        self.assertTrue(auto_retry.tracking_issue_body().startswith(auto_retry.BOT_MARKER))
        self.assertIn(auto_retry.TRACKING_MARKER, auto_retry.tracking_issue_body())
        self.assertTrue(auto_retry.duplicate_comment(12).startswith(auto_retry.BOT_MARKER))

    def test_record_target_is_pr_or_tracking_issue(self) -> None:
        self.assertEqual(auto_retry.record_destination("schedule", "main", "main"), "issue")
        self.assertEqual(auto_retry.record_destination("push", "main", "main"), "issue")
        self.assertEqual(
            auto_retry.record_destination("pull_request", "cursor/scoped-auto-retry-6226", "main"),
            "pr",
        )
        self.assertEqual(
            auto_retry.record_destination("workflow_dispatch", "feature", "main"),
            "pr",
        )
        issues = [
            {"number": 9, "title": auto_retry.TRACKING_TITLE, "pull_request": {"url": "x"}, "body": auto_retry.TRACKING_MARKER},
            {"number": 40, "title": auto_retry.TRACKING_TITLE, "body": auto_retry.TRACKING_MARKER},
            {"number": 12, "title": auto_retry.TRACKING_TITLE, "body": auto_retry.TRACKING_MARKER},
            {"number": 4, "title": "something else", "body": auto_retry.TRACKING_MARKER},
        ]
        self.assertEqual(auto_retry.select_tracking_issue(issues), 12)

    def test_record_is_posted_before_rerun(self) -> None:
        class Transport:
            def __init__(self) -> None:
                self.calls: list[str] = []

            def skip(self, reason: str) -> None:
                self.calls.append(f"skip:{reason}")

            def post_record(self, record: str) -> None:
                self.calls.append("post")

            def rerun(self) -> None:
                self.calls.append("rerun")

        transport = Transport()
        decision = auto_retry.Decision(True, "retry")
        auto_retry.run_retry(decision, "record", transport)
        self.assertEqual(transport.calls, ["post", "rerun"])
        skipped = Transport()
        auto_retry.run_retry(auto_retry.Decision(False, "denylist"), "record", skipped)
        self.assertEqual(skipped.calls, ["skip:denylist"])

        class Boom:
            def skip(self, reason: str) -> None:
                raise AssertionError(reason)

            def post_record(self, record: str) -> None:
                raise RuntimeError("comment failed")

            def rerun(self) -> None:
                raise AssertionError("rerun must wait for the record")

        with self.assertRaises(RuntimeError):
            auto_retry.run_retry(auto_retry.Decision(True, "retry"), "record", Boom())

        namespace = _mutant_auto_retry(
            "    transport.post_record(record)  # record-before-rerun\n    transport.rerun()\n",
            "    transport.rerun()\n    transport.post_record(record)  # record-before-rerun\n",
        )
        run_retry = namespace["run_retry"]
        decision_type = namespace["Decision"]
        assert callable(run_retry) and callable(decision_type)
        swapped = Transport()
        run_retry(decision_type(True, "retry"), "record", swapped)
        self.assertEqual(swapped.calls, ["rerun", "post"])

    def test_raw_url_rejects_traversal(self) -> None:
        url = auto_retry.default_branch_raw_url(
            "lazyxeon/AstraWeave",
            "main",
            ".github/flaky-jobs.json",
        )
        self.assertEqual(
            url,
            "https://raw.githubusercontent.com/lazyxeon/AstraWeave/main/.github/flaky-jobs.json",
        )
        with self.assertRaises(ValueError):
            auto_retry.default_branch_raw_url("lazyxeon/AstraWeave", "main/../../etc", "a")
        with self.assertRaises(ValueError):
            auto_retry.default_branch_raw_url("not a repo", "main", "a")

    def test_workflow_policy_accepts_the_checked_in_file(self) -> None:
        text = AUTO_RETRY_WORKFLOW.read_text(encoding="utf-8")
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        self.assertEqual(auto_retry_violations(text, {item.workflow for item in allow}), [])
        self.assertNotIn("actions/checkout", text)
        self.assertNotIn("pull_request_target", text)
        self.assertIn("GH_TOKEN: ${{ github.token }}", text)
        self.assertNotIn("github.token", (ROOT / "scripts" / "ci" / "auto_retry.py").read_text(encoding="utf-8"))
        self.assertNotIn("shell=True", (ROOT / "scripts" / "ci" / "auto_retry.py").read_text(encoding="utf-8"))
        bodies = run_script_bodies(text)
        self.assertGreaterEqual(len(bodies), 2)
        self.assertTrue(any("auto_retry.py" in body for body in bodies))
        for body in bodies:
            self.assertNotIn("${{", body)
            self.assertNotIn("github.event.workflow_run.head_sha", body)

    def test_workflow_mutants_fail_the_policy(self) -> None:
        text = AUTO_RETRY_WORKFLOW.read_text(encoding="utf-8")
        allow_workflows = {workflow for workflow, _job in EXPECTED_FLAKY_JOBS}
        mutants = {
            "contents-permission": text.replace(
                "    permissions:\n      actions: write\n",
                "    permissions:\n      contents: read\n      actions: write\n",
                1,
            ),
            "top-level-write": text.replace(
                "permissions: {}\n",
                "permissions:\n  actions: write\n",
                1,
            ),
            "attempt-cap": text.replace("run_attempt < 3", "run_attempt < 9", 1),
            "checkout-head": text.replace(
                "    steps:\n",
                "    steps:\n"
                "      - name: Checkout pull request head\n"
                "        uses: actions/checkout@v7\n"
                "        with:\n"
                "          ref: ${{ github.event.workflow_run.head_sha }}\n",
                1,
            ),
            "interpolate-branch": text.replace(
                "run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py",
                "run: echo ${{ github.event.workflow_run.head_branch }}",
                1,
            ),
            "drop-record-step": text.replace(
                "name: Record the failure and retry allowlisted jobs",
                "name: Rerun quietly",
                1,
            ),
            "continue-on-error": text.replace(
                "        run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py\n",
                "        run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py\n"
                "        continue-on-error: true\n",
                1,
            ),
            "ignore-failure": text.replace(
                "run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py",
                "run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py || true",
                1,
            ),
            "ignore-exit-0": text.replace(
                "run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py",
                "run: python3 /tmp/auto-retry/scripts/ci/auto_retry.py || exit 0",
                1,
            ),
            "foreign-raw-host": text.replace(
                "https://raw.githubusercontent.com/",
                "https://evil.example/",
                1,
            ),
            "drop-redirect-guard": text.replace(
                'host != "raw.githubusercontent.com"',
                'host == "raw.githubusercontent.com"',
                1,
            ),
            "unpinned-fetch": text.replace("/{sha}/", "/{branch}/", 1),
            "fork-repository": text.replace(
                "      github.event.workflow_run.head_repository.full_name == github.repository\n",
                "",
                1,
            ),
            "drop-timed-out": text.replace(
                "github.event.workflow_run.conclusion == 'timed_out'",
                "github.event.workflow_run.conclusion == 'success'",
                1,
            ),
            "extra-workflow": text.replace(
                "      - Performance Benchmarks\n",
                "      - Performance Benchmarks\n      - determinism-replay\n",
                1,
            ),
            "pull-request-target": text.replace(
                "on:\n",
                "on:\n  pull_request_target:\n",
                1,
            ),
        }
        self.assertEqual(auto_retry_violations(text, allow_workflows), [])
        for name, mutant in mutants.items():
            with self.subTest(mutant=name):
                self.assertNotEqual(mutant, text)
                self.assertTrue(auto_retry_violations(mutant, allow_workflows), name)

    def test_infra_signature_matches_cited_flakes_and_rejects_regressions(self) -> None:
        self.assertEqual(auto_retry.infra_signature(LOG_143), "exit-143")
        self.assertEqual(auto_retry.infra_signature(LOG_LOST_RUNNER), "lost-runner")
        self.assertEqual(auto_retry.infra_signature(LOG_ARTIFACT), "artifact-download")
        self.assertEqual(auto_retry.infra_signature(LOG_MISSING), "missing-shard-artifact")
        self.assertEqual(auto_retry.infra_signature(LOG_CRATES), "crates-io")
        self.assertEqual(auto_retry.infra_signature(LOG_TIMEOUT), "network-timeout")
        self.assertEqual(auto_retry.infra_signature(LOG_CACHE), "cache-download")
        self.assertEqual(auto_retry.infra_signature(LOG_CACHE_RESTORE), "cache-download")
        self.assertIsNone(auto_retry.infra_signature(LOG_PACKAGE))
        self.assertIsNone(auto_retry.infra_signature(LOG_BOILERPLATE))
        self.assertIsNone(auto_retry.infra_signature("exit code 1430"))
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        shard = "Performance Benchmarks (shard 10)"
        aggregate = "Aggregate Benchmark Results"
        package = _decide(
            "Performance Benchmarks", [shard], allow, logs={shard: LOG_PACKAGE}
        )
        self.assertEqual(package.reason, "no-signature")
        both = _decide(
            "Performance Benchmarks",
            [shard, aggregate],
            allow,
            logs={shard: LOG_LOST_RUNNER, aggregate: LOG_MISSING},
        )
        self.assertTrue(both.retry)
        self.assertEqual(both.signatures, ("lost-runner", "missing-shard-artifact"))
        strict = _decide(
            "Performance Benchmarks",
            [aggregate],
            allow,
            logs={aggregate: LOG_ARTIFACT},
            steps={
                aggregate: (("Strict threshold validation (main branch only)", "failure"),)
            },
        )
        self.assertEqual(strict.reason, "strict-threshold")
        self.assertFalse(strict.retry)

    def test_signature_and_strict_checks_are_load_bearing(self) -> None:
        allow = auto_retry.load_allowlist(AUTO_RETRY_ALLOWLIST)
        shard = "Performance Benchmarks (shard 10)"
        aggregate = "Aggregate Benchmark Results"
        real = _decide(
            "Performance Benchmarks", [shard], allow, logs={shard: LOG_PACKAGE}
        )
        self.assertEqual(real.reason, "no-signature")
        namespace = _mutant_auto_retry(
            "if any(infra_signature(job.log) is None for job in failed_jobs):  # infra-signature",
            "if False:  # infra-signature",
        )
        mutant = _mutant_decide(
            namespace,
            "Performance Benchmarks",
            [shard],
            allow,
            logs={shard: LOG_PACKAGE},
        )
        self.assertTrue(mutant.retry)
        steps = {
            aggregate: (("Strict threshold validation (main branch only)", "failure"),)
        }
        real_strict = _decide(
            "Performance Benchmarks",
            [aggregate],
            allow,
            logs={aggregate: LOG_143},
            steps=steps,
        )
        self.assertEqual(real_strict.reason, "strict-threshold")
        namespace = _mutant_auto_retry(
            "if any(strict_step_failed(job.steps) for job in failed_jobs):  # strict-exclusion",
            "if False:  # strict-exclusion",
        )
        mutant_strict = _mutant_decide(
            namespace,
            "Performance Benchmarks",
            [aggregate],
            allow,
            logs={aggregate: LOG_143},
            steps=steps,
        )
        self.assertTrue(mutant_strict.retry)

    def test_name_normalization_is_load_bearing(self) -> None:
        cases = [
            (
                'text = unicodedata.normalize("NFKC", value)  # norm:nfkc',
                "text = value  # norm:nfkc",
                "Comprehensive CI Pipeline",
                "\uff27\uff41\uff54\uff45",
            ),
            (
                'text = "".join(ch for ch in text if unicodedata.category(ch) != "Cf")  # norm:zero-width',
                "text = text  # norm:zero-width",
                "Comprehensive CI Pipeline",
                "Ga\u200bte",
            ),
            (
                "text = text.translate(_HOMOGLYPHS)  # norm:homoglyph",
                "text = text  # norm:homoglyph",
                "Comprehensive CI Pipeline",
                "G\u0430te",
            ),
            (
                'text = text.replace("-", " ").replace("_", " ")  # norm:separators',
                "text = text  # norm:separators",
                "determinism-replay",
                "unrelated",
            ),
        ]
        for old, new, workflow, job in cases:
            with self.subTest(job=job):
                self.assertTrue(auto_retry.job_denied(workflow, job))
                namespace = _mutant_auto_retry(old, new)
                denied = namespace["job_denied"]
                self.assertTrue(callable(denied))
                self.assertFalse(denied(workflow, job))
                allow = _allow((workflow, job))
                decision = _mutant_decide(
                    namespace, workflow, [job], allow, logs={job: LOG_143}
                )
                self.assertTrue(decision.retry)

    def test_open_pr_matches_head_branch_and_repository(self) -> None:
        repo = "lazyxeon/AstraWeave"
        branch = "cursor/scoped-auto-retry-6226"
        pulls = [
            {
                "number": 1,
                "state": "open",
                "head": {"ref": "other", "repo": {"full_name": repo}},
            },
            {
                "number": 3,
                "state": "open",
                "head": {"ref": branch, "repo": {"full_name": "fork/AstraWeave"}},
            },
            {
                "number": 9,
                "state": "open",
                "head": {"ref": branch, "repo": {"full_name": repo}},
            },
            {
                "number": 4,
                "state": "closed",
                "head": {"ref": branch, "repo": {"full_name": repo}},
            },
        ]
        self.assertEqual(auto_retry.select_open_pr(pulls, branch, repo), 9)
        self.assertIsNone(auto_retry.select_open_pr(pulls, "missing", repo))
        self.assertFalse(auto_retry.repository_matches("fork/AstraWeave", repo))
        self.assertFalse(auto_retry.repository_matches("", repo))
        self.assertTrue(auto_retry.repository_matches(repo, repo))

    def test_live_attempt_blocks_a_stale_listener(self) -> None:
        self.assertEqual(auto_retry.live_run_guard(1, 3, "completed"), "stale-attempt")
        self.assertEqual(auto_retry.live_run_guard(3, 3, "completed"), "attempt-cap")
        self.assertEqual(auto_retry.live_run_guard(1, 1, "in_progress"), "run-in-progress")
        self.assertIsNone(auto_retry.live_run_guard(2, 2, "completed"))
        attempt, status, conclusion = auto_retry.parse_run(
            {"run_attempt": 2, "status": "completed", "conclusion": "failure"}
        )
        self.assertEqual((attempt, status, conclusion), (2, "completed", "failure"))
        with self.assertRaises(ValueError):
            auto_retry.parse_run({"run_attempt": True, "status": "completed", "conclusion": "failure"})
        self.assertIn("filter=latest", auto_retry.jobs_path("lazyxeon/AstraWeave", "99"))
        namespace = _mutant_auto_retry("filter=latest", "filter=all")
        path = namespace["jobs_path"]
        assert callable(path)
        self.assertNotIn("filter=latest", path("lazyxeon/AstraWeave", "99"))

    def test_prune_keeps_the_newest_shard_attempt(self) -> None:
        import tempfile
        from pathlib import Path

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            kept_names = []
            for name in (
                "benchmark-results-abc-shard-10-attempt-1",
                "benchmark-results-abc-shard-10-attempt-2",
                "benchmark-results-abc-shard-11",
                "notes.txt",
            ):
                path = root / name
                if name == "notes.txt":
                    path.write_text("keep", encoding="utf-8")
                    continue
                path.mkdir()
                (path / "benchmarks.json").write_text("[]", encoding="utf-8")
                kept_names.append(name)
            removed = auto_retry.prune_older_shard_attempts(root)
            self.assertEqual(removed, ["benchmark-results-abc-shard-10-attempt-1"])
            self.assertFalse((root / "benchmark-results-abc-shard-10-attempt-1").exists())
            self.assertTrue((root / "benchmark-results-abc-shard-10-attempt-2").is_dir())
            self.assertTrue((root / "benchmark-results-abc-shard-11").is_dir())
            self.assertTrue((root / "notes.txt").is_file())
            self.assertEqual(auto_retry.prune_older_shard_attempts(root / "missing"), [])

    def test_fetch_refuses_a_redirect_off_raw_githubusercontent(self) -> None:
        import textwrap
        import urllib.error
        import urllib.parse
        import urllib.request

        text = AUTO_RETRY_WORKFLOW.read_text(encoding="utf-8")
        marker = "def raw_url("
        start = text.rfind("\n", 0, text.index(marker)) + 1
        end = text.index("urllib.request.install_opener")
        chunk = textwrap.dedent(text[start:end])
        namespace: dict[str, object] = {"re": re, "urllib": urllib}
        exec(compile(chunk, "auto-retry.yml", "exec"), namespace)
        raw_url = namespace["raw_url"]
        reject_redirect = namespace["reject_redirect"]
        handler_type = namespace["RawRedirectHandler"]
        assert callable(raw_url) and callable(reject_redirect) and callable(handler_type)
        sha = "a" * 40
        url = raw_url("lazyxeon/AstraWeave", sha, "scripts/ci/auto_retry.py")
        self.assertEqual(
            url,
            f"https://raw.githubusercontent.com/lazyxeon/AstraWeave/{sha}/scripts/ci/auto_retry.py",
        )
        reject_redirect(url)
        for bad in (
            "https://evil.example/auto_retry.py",
            "https://raw.githubusercontent.com.evil.example/a",
            "http://raw.githubusercontent.com/lazyxeon/AstraWeave/main/a",
            "https://user:pass@raw.githubusercontent.com/lazyxeon/AstraWeave/a",
        ):
            with self.assertRaises(urllib.error.URLError):
                reject_redirect(bad)
        with self.assertRaises(SystemExit):
            raw_url("lazyxeon/AstraWeave", sha, "../etc/passwd")
        handler = handler_type()
        with self.assertRaises(urllib.error.URLError):
            handler.redirect_request(None, None, 302, "Found", {}, "https://evil.example/x")

    def test_benchmark_artifacts_are_attempt_aware(self) -> None:
        text = (WORKFLOWS / "benchmark.yml").read_text(encoding="utf-8")
        self.assertEqual(benchmark_artifact_violations(text), [])
        mutants = {
            "overwrite": text.replace(
                "name: benchmark-results-${{ github.sha }}-attempt-${{ github.run_attempt }}\n",
                "name: benchmark-results-${{ github.sha }}-attempt-${{ github.run_attempt }}\n"
                "          overwrite: true\n",
                1,
            ),
            "drop-attempt": text.replace(
                "-attempt-${{ github.run_attempt }}",
                "",
                1,
            ),
            "drop-success-guard": text.replace("success() &&\n", "", 1),
            "extra-continue-on-error": text.replace(
                "continue-on-error: true\n",
                "continue-on-error: true\n        continue-on-error: true\n",
                1,
            ),
        }
        for name, mutant in mutants.items():
            with self.subTest(mutant=name):
                self.assertNotEqual(mutant, text)
                self.assertTrue(benchmark_artifact_violations(mutant), name)


if __name__ == "__main__":
    unittest.main()

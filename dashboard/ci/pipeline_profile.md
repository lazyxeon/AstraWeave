# CI pipeline profile

Measured before any optimization in this series. The window is the newest 1,000 Actions runs whose `created_at` falls between 2026-10-25 00:03:16Z and 2026-10-27 23:44:02Z. The list call reported `total_count` 2,500 for a wider `created` filter and then returned an empty page after 1,000 rows, so this is the newest slice, not a uniform sample back to 13 October. Job records were fetched for 654 completed runs of the workflows below. Base commit for the tree that produced these runs is `99382af7` only at the end of the window; earlier runs are older main and open PRs.

Run-level queue (`run_started_at - created_at`) is about zero: GitHub starts the workflow record immediately. The queue that matters is the job start offset (`job.started_at - run_started_at`), which mixes runner wait with `needs`. Durations are `completed_at - started_at` and exclude that wait. Wall clock is the last non-skipped job's `completed_at` minus `run_started_at`. p95 on a sample of 7 is nearly the maximum; nightly workflows in this window have n of 3 to 10.

## Pull-request wall clock

These are successful pull-request runs unless noted.

| Workflow | n | Wall p50 | Wall p95 | What the time is |
|---|---:|---:|---:|---|
| Comprehensive CI, tests ran | 29 (success+failure) | 92.1 min | 121.0 min | Critical path. See below. |
| Comprehensive CI, ubuntu tests skipped | 4 | 18.3 min | 47.0 min | Docs-only or no crate. Compile jobs did not run. |
| determinism-replay | 35 success | 47.0 min | 83.2 min | Three OS hashes, then compare. |
| Aravath Windows | 25 success | 30.1 min | 46.4 min | Windows greybox dominates. |
| Aravath determinism | 39 success | 28.7 min | 56.4 min | Three OS golden hashes. |
| Render device validation | 45 success | 23.1 min | 60.7 min | Lavapipe plus Windows startup passes. |
| Net Tests | 43 success | 16.1 min | 51.5 min | Job itself is 3.6 min p50. The rest is queue. |
| Security Audit | 25 success | 11.2 min | 30.5 min | Deny is 57s p50, audit is 44s p50. The rest is queue. |

Billable minutes are the sum of job durations (runner time), p50 per pull-request run:

| Workflow | Billable p50 | Billable p95 | n |
|---|---:|---:|---:|
| Comprehensive CI Pipeline | 99.5 min | 114.9 min | 39 |
| determinism-replay | 68.1 min | 85.7 min | 42 |
| Aravath Windows | 41.8 min | 48.3 min | 42 |
| Aravath determinism | 36.8 min | 39.5 min | 42 |
| Render device validation | 26.9 min | 30.2 min | 45 |
| Documentation | 4.7 min | 6.2 min | 35 |
| Net Tests | 3.6 min | 17.3 min | 44 |
| Pack Guard | 3.4 min | 10.6 min | 43 |
| Trace Sync | 2.1 min | 4.0 min | 33 |
| Security Audit | 1.7 min | 2.9 min | 40 |
| Doc-Truth Lint | 1.1 min | 1.2 min | 39 |
| OpenSSF Scorecard | 0.7 min | 1.3 min | 43 |

A pull request that compiles crates therefore occupies on the order of 280 runner-minutes before the short lint workflows. Docs-only pulls still pay determinism-replay (no path filter, required) and the Aravath and render workflows (no crate filter).

## Critical path, code pull request

Exemplar success run [40235303264](https://github.com/lazyxeon/AstraWeave/actions/runs/40235303264) (wall 79.7 min, near the mixed p50; code PRs that actually test sit higher, p50 92.1 min). Offsets are minutes after `run_started_at`.

| Start | Duration | End | Job |
|---:|---:|---:|---|
| 6.7 | 0.8 | 7.4 | Detect affected crates |
| 8.3 | 27.1 | 35.5 | Gate (fmt, clippy, check) |
| 35.5 | 5.4 | 40.9 | Minimum Supported Rust Version |
| 35.5 | 29.6 | 65.1 | Test (windows, build archive) |
| 35.5 | 42.4 | 77.9 | Test (ubuntu, affected) |
| 68.9 | 5.1 | 74.0 | Test (windows, 3/3) |
| 70.6 | 6.0 | 76.5 | Test (windows, 2/3) |
| 70.2 | 7.1 | 77.3 | Test (windows, 1/3) |
| 79.6 | 0.1 | 79.7 | PR Rust checks |

Across pull-request runs the last job is always `PR Rust checks` (n=33). The work in front of it is the ubuntu nextest job racing the windows archive plus its three partitions. Gate duration when tests ran is 7.2 min p50 and 22.5 min p95 (n=29). The 27 min gate above is the slow tail, not the median.

Job start offsets on pull requests (p50), for context:

| Job | Duration p50 | Start offset p50 | n |
|---|---:|---:|---:|
| Detect affected crates | 48s | 7.5 min | 39 |
| Gate | 6.9 min | 18.1 min | 39 |
| Test (ubuntu, affected) | 32.0 min | 31.5 min | 39 |
| Test (windows, build archive) | 30.6 min | 31.5 min | 39 |
| Test (windows, 1/3) | 4.8 min | 75.1 min | 29 |
| Test (windows, 2/3) | 5.5 min | 70.6 min | 29 |
| Test (windows, 3/3) | 4.1 min | 68.9 min | 29 |
| hash (windows-latest), determinism-replay | 34.2 min | 9.6 min | 42 |
| Cargo Deny Check | 57s | 13.7 min | 40 |

The windows partition jobs are about 5 minutes of test execution. They start about 5 to 10 minutes after the archive job finishes, which is runner wait, and they are on the critical path whenever ubuntu nextest finishes first. Archive step `Archive tests once` is 28.8 min p50 (n=31). Ubuntu step `Nextest (affected crates, or the workspace on a full diff)` is 26.2 min p50 (n=30).

## Main and schedule

Push to `main` of Comprehensive CI in this window, all failures:

| Run | Wall | Last failing job |
|---|---:|---|
| [40145178400](https://github.com/lazyxeon/AstraWeave/actions/runs/40145178400) | 63.0 min | Test full (macos-latest) |
| [40218863874](https://github.com/lazyxeon/AstraWeave/actions/runs/40218863874) | 78.4 min | Test full (macos-latest) |
| [40227380244](https://github.com/lazyxeon/AstraWeave/actions/runs/40227380244) | 84.0 min | Test full (windows-latest) aggregator |
| [40239005398](https://github.com/lazyxeon/AstraWeave/actions/runs/40239005398) | 90.4 min | Test full (windows-latest) aggregator |
| [40241809225](https://github.com/lazyxeon/AstraWeave/actions/runs/40241809225) | 110.2 min | Test full (macos-latest) |
| [40228431072](https://github.com/lazyxeon/AstraWeave/actions/runs/40228431072) | 139.3 min | Test full (macos-latest) |
| [40576777318](https://github.com/lazyxeon/AstraWeave/actions/runs/40576777318) | 126.5 min | Test full (windows-latest) aggregator |

`Test full (macos-latest)` duration p50 is 50.5 min, p95 60.2 min, max 60.6 min (n=10, push and schedule). Nine conclusions are `failure`, one is `cancelled`. The cancelled job is schedule run [40451173602](https://github.com/lazyxeon/AstraWeave/actions/runs/40451173602): the job ran 60.6 min and `Workspace nextest` ran 50.0 min before the 60 minute job timeout. Other macOS failures die inside `Workspace nextest` at 16.9 to 50.6 min, so some are test failures rather than the timeout. Ubuntu `Test full` succeeds in this window (duration p50 24.7 min, n=10). Windows archive build succeeds (p50 31.6 min). All 10 `Test full (windows, 1/3)`, `2/3`, and `3/3` jobs fail, and the aggregator `Test full (windows-latest)` fails with them. Those windows partition failures are the subject of PR #467. The macOS GPU failures are the subject of PR #439. This profile does not change those jobs.

Other main-only heavy workflows, job duration p50 / p95:

| Job | n | p50 | p95 | Failures in window |
|---|---:|---:|---:|---|
| Performance Benchmarks shards (16-way) | 10 each | 19–46 min | up to 54 min | shard 10 failed 3 |
| Benchmark Regression Alerts shards | 7 | 20–48 min | up to 57 min | some shard failures |
| Mutants - astraweave-ecs | 3 | 57.3 min | 81.3 min | workflow failed, 0 timeouts |
| Toolchain Validation (stable) | 30 | 22.4 min | 40.8 min | 2 |
| Toolchain Validation (1.89.0) | 30 | 25.5 min | 39.9 min | 1 |
| CodeQL Analyze (rust) | 8 | 34.0 min | 41.0 min | 0 |
| AddressSanitizer (Full Suite) | 10 | 0s median, 26.8 min p95 | | often skipped by `if` |
| Fuzz nightly | 3 runs | wall ~8 min | | 3 workflow failures |

Benchmarks already do not run on an ordinary pull request (`pull_request` types `labeled` plus `run-benchmarks`, and a nightly schedule). They do run on every push to `main` and again on the nightly cron, so each main push pays 16 shards twice a day when the cron also fires.

## Duplicated work

| Work | Where it runs on a pull request | Second copy |
|---|---|---|
| `cargo deny` + `cargo audit` | `security-audit.yml`, prebuilt binaries, under 1 min of work | In this window, `aravath-windows.yml` job `cargo-audit, cargo-deny, SBOM` compiled `cargo-audit` 0.22.2, `cargo-deny` 0.20.2, and `cargo-cyclonedx` 0.5.9 with `cargo install` (step p50 6.4 min, n=38) and then ran deny and audit again. #476 (`a94e6838`) installs those same versions from sha256-pinned musl builds. CycloneDX SBOM and `scripts/check-aravath-network.sh` exist only on the Aravath job. |
| Replay hashes, three OS | `determinism-replay.yml` (required, no path filter) | `aravath-determinism.yml` runs `replay_matches_golden_hash` on three OS for every pull request. Different test. Not a substitute for the compare job. |
| Render startup / lavapipe | `render-device-validation.yml` on every pull request (lavapipe p50 12.6 min, Windows p50 16.4 min) | Comprehensive CI also tests `astraweave-render` when that crate is affected. The lavapipe job sets `WGPU_BACKEND=vulkan` and `WGPU_VALIDATION=1`. The env is not the same, so the workflow is not a pure duplicate. It still runs when the diff cannot change render or ecs. |
| `cargo test -p astraweave-net` | `net-tests.yml` (`--all-features`) on every non-markdown pull request | CI nextest includes `astraweave-net` when that crate is in the affected set. `--all-features` is the part that is not obviously the same invocation. |
| `cargo audit` / `cargo deny` nightly | `dependency-management.yml` schedule | Same checks as `security-audit.yml`, which already runs on push to `main` and on pull requests. |

`scripts/ci/affected_crates.py` already narrows Comprehensive CI. Since #475, a diff that edits `.github/workflows/`, `.github/actions/`, or `.github/scripts/` sets `full=true`. [CI_ARCHITECTURE.md](../current/CI_ARCHITECTURE.md) records that row. Other `.github/**` only, for example `CODEOWNERS`, stays fmt gate only. `Cargo.lock`, `rust-toolchain.toml`, and `.cargo/` also force `full=true`.

## Cache

`rust-ci-setup` already enables sccache (`SCCACHE_GHA_ENABLED`) and Swatinem `rust-cache` with `cache-targets: false` and `save-if` only on `refs/heads/main`. Two gate logs:

| Run | What compiled | Requests | Rust hits | Rust misses | Hit rate |
|---|---|---:|---:|---:|---:|
| [40593627231](https://github.com/lazyxeon/AstraWeave/actions/runs/40593627231) gate | lockfile PR, full gate | 1,090 | 226 | 464 | 38.52% |
| [40235303264](https://github.com/lazyxeon/AstraWeave/actions/runs/40235303264) gate | code PR, 27.1 min gate | 1,127 | (rate 21.17%) | 510 | 24.34% |

Hit rate was not sampled across the window. These two are not a miss spike on one cold runner; both are well under half. Target directories are not shared between jobs. Windows pull-request tests already build once and partition with a nextest archive. The archive step is still ~29 min.

## Timeouts

| Job | Cap | What happened |
|---|---:|---|
| Test full (macos-latest) | 60 | p95 60.2 min, one cancel at 60.6 min. See main table. |
| Test (ubuntu, affected) | 60 | p95 duration 45.0 min. The comment in `ci.yml` records an older 45 min cancel after 27,057 passes. Not the current cap. |
| Test (windows, build archive) | 75 | p95 39.1 min on pull requests. Under the cap. |
| Test full (windows, build archive) | 60 | p95 33.1 min. Under the cap. The partitions then fail for the reason in #467, not for this timeout. |
| hash (windows-latest) in determinism-replay | 75 | duration p95 36.4 min. The comment records an older 45 min cancel (run 39839010318). |
| Aravath Windows greybox | 180 | duration p95 27.1 min. |

## Serial work that sits on a critical path

On the windows pull-request archive job, `Net client binaries on this machine` and `Model HTTP probes on this machine` run in the same job as the archive. Partitions `needs` that job, so those follow-up tests delay the partitions even though the archive upload happens before the model probes. On run 40593627231 the archive step ended at 23:00:54, the net-client tests took about 2 min, the upload took about 8s, and the model probes took about 4.5 min. The partitions could not start until 23:07. This only shows up when `astraweave-llm` or `astraweave-embeddings` is in the affected set (or the diff is full). It is real, and it is smaller than the archive compile.

`determinism-replay.yml` fetches Aravath nature models twice in the hash job. That is seconds, not the 33 min `Replay hash` step on Windows (p50 32.7 min of the 34.2 min job).

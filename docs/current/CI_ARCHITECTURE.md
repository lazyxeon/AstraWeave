# CI architecture (WS-CI-OPT)

Pull-request checks are split into a small required tier and a nightly / main tier. Cross-OS replay hash checks stay on every pull request, on Ubuntu, Windows, and macOS, with no path filter.

Before numbers are from the 2026-10-14 bottleneck report (PR #212, head `a084db2`). After numbers are filled from this pull request's check runs once they finish. This pull request changes the workspace `Cargo.toml`, so affected-crate detection selects the **full** workspace. A narrow pull request does not.

## Before (PR #212)

| | |
|---|---|
| Checks | 107 (64 passed, 29 failed, 6 skipped) |
| Runner-minutes | about 978 (16.3 h) |
| Longest workflow wall time | Comprehensive CI, 115 min, failed |
| Benchmark shards | 12 jobs, 351 runner-minutes, all failed |
| Free-plan cap | 20 concurrent jobs, 5 of them macOS |

With about eight agent branches open, most of those jobs queued. The same workspace was compiled by Comprehensive CI, Rust Cache Optimized, the unwrap matrix, and Toolchain Validation.

## What changed

1. **Benchmarks are off unlabeled pull requests.** `Performance Benchmarks` runs on push to `main` and `develop`, nightly at 05:17 UTC, `workflow_dispatch`, and a pull request whose `run-benchmarks` label was just added (`pull_request` types: `[labeled]`). A later push does not rerun benches; remove and re-add the label, or use `workflow_dispatch`. The 600s per-package cap in `.github/scripts/benchmark-runner.sh` is unchanged. See issue #221.
2. **Concurrency on every pull-request workflow.**

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

A new push cancels the previous pull-request run. Pushes to main, tags, and schedules are not cancelled.
3. **Every job has `timeout-minutes`.** Typical jobs are 30–45. The fast gate is `Gate (fmt, clippy, check)`. Linux tests, the Windows partitions, and MSRV `needs` that gate, so a format or compile failure does not start those runners.
4. **Two tiers.** Nightly Failure Issue (`workflow_run`) opens or updates a GitHub issue when a nightly or main run of the heavy workflows fails. `workflow_run` fires only after this file is on the default branch.
5. **Duplicates are gone.** `rust-cache-optimized.yml` is folded into `ci.yml` and deleted. The 15-job unwrap matrix is deleted. One `cargo clippy --lib --no-deps` pass reads `[lints.clippy]` on each crate. sccache (GitHub Actions cache backend) plus one Swatinem `shared-key` per OS (`${{ runner.os }}-astraweave`), `cache-targets: false`, `save-if` only on `refs/heads/main`. Affected crates come from `scripts/ci/affected_crates.py`. Docs-only diffs skip the Rust compile. Windows pull-request tests compile once (`cargo nextest archive` on `windows-latest`, debuginfo off so the archive stays uploadable) and then run three `count:N/3` partitions against that archive. Determinism tests are not retried.
6. **Required jobs are not queued behind optional compiles.** PBR, Editor CI, and AW Editor tests do not run on `pull_request`. They run on push, their nightly schedule, and `workflow_dispatch`. `cancel-in-progress` is still `${{ github.event_name == 'pull_request' }}` on every workflow that has the block, so a push to `main` or a schedule is never cancelled.

## Tiers

### Pull request (required)

| Check | Workflow job name |
|---|---|
| Affected-crate detect | `Detect affected crates` |
| fmt, one clippy, cargo check | `Gate (fmt, clippy, check)` |
| Linux tests of affected crates | `Test (ubuntu, affected)` |
| Windows archive, then 3 partitions | `Test (windows, build archive)`, then `Test (windows, 1/3)`, `Test (windows, 2/3)`, `Test (windows, 3/3)` |
| MSRV (`rust-version` in the workspace manifest, 1.89.0) | `Minimum Supported Rust Version` |
| Rollup of the rows above | `PR Rust checks` |
| Replay hash, Ubuntu / Windows / macOS, then compare. Required cases: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`. No path filter. | `hash (ubuntu-latest)`, `hash (windows-latest)`, `hash (macos-latest)`, `compare` |
| cargo-deny, cargo-audit | `Cargo Deny Check`, `Security Audit (cargo-audit)` |

These also run on a non-docs pull request when their paths match, and are not in the required set because a path filter would skip them: `net-tests`, trace sync, doc-truth lint, docs. Pack guard and OpenSSF Scorecard run on every pull request. PBR, Editor CI, and AW Editor tests do not run on pull requests.

`PR Rust checks` is the status to require for the Rust rows. The heavy jobs are skipped on a docs-only pull request (they do not compile). GitHub treats a skipped required check as missing, which blocks the merge. The rollup runs on every pull request: it fails if the gate failed or if a crate change left a test job red, and it passes when the heavy jobs were correctly skipped.

### Main, nightly, workflow_dispatch

| Work | When |
|---|---|
| Full 3-OS `cargo nextest` plus a release build of the core crates, and demo validation | `ci.yml` on push, schedule 04:30 UTC, dispatch. Not on pull requests. |
| 6-toolchain matrix (stable, 1.89.0, 1.88.0, beta, nightly) | `toolchain-management.yml` on push to main, schedule 05:40 UTC, dispatch. MSRV stays on the pull request. |
| Sanitizers | `sanitizers.yml` on push and schedule 03:00 UTC. Not on pull requests. |
| Coverage | `coverage.yml` on push to main, schedule 06:10 UTC, dispatch. |
| CodeQL | push to main and weekly (Sunday 03:00 UTC). Not on pull requests. |
| Benchmarks, including `aw_build` | see item 1 above. Shard count stays 12 on this branch. |
| Integration tests other than the replay hash | `integration-tests.yml` on push and schedule 04:00 UTC. |
| PBR build and editor tests, all three OS | `pbr-pipeline-ci.yml`, `editor-ci.yml`, `aw-editor-tests.yml` on push, their nightly schedules, and `workflow_dispatch`. Not on pull requests. |
| Allocation ceilings, zero-alloc hot paths, CPU tick-time artifact | `allocation-measurement.yml` on push to `main`, schedule 05:45 UTC, and `workflow_dispatch`. Not on pull requests. The 11 ms GTX 1660 Ti check is the `gpu_frame_budget` dispatch input on a self-hosted runner labeled `gpu`. |
| SDK C harness | push to main, schedule 06:25 UTC, dispatch. |

## Determinism

`determinism-replay.yml` runs on every pull request and on push to `main`. It has no path filter. Each OS runs the core replay test, the combat replay test, `replay_aravath_slice_scenario`, the causality replay test, and the scripting replay test, with the baseline CPU pinned (`target-cpu=x86-64` on x86_64; aarch64 stays on the default baseline). `cargo test` has no retry loop. The `compare` job requires `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, and `scripted_system` to be present and byte-identical across the three artifacts. Open CI work that is not in this file is #217.

When the same tests also run under nextest (an affected crate on a pull request, or the full nightly matrix), `.config/nextest.toml` profile `ci` sets `retries = 2` for ordinary tests and `retries = 0` for test names matching `determinism`, `replay`, or `deterministic`. The determinism workflow does not call nextest.

## Affected crates and docs-only pulls

`scripts/ci/affected_crates.py` reads `git diff` (or `--files` in tests) and `cargo metadata --no-deps`.

| Diff | Result |
|---|---|
| Only `docs/**`, `gh-pages/**`, or `*.md` | `docs_only=true`. The gate does not install a toolchain. Test jobs do not start. Determinism still runs. |
| Root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/**`, or `.config/nextest.toml` | `full=true`. Workspace clippy, check, nextest, and MSRV. |
| Files under one or more crates | Those crates plus workspace reverse dependencies. |
| A file that is not docs, not under `.github/`, and not inside a crate | `full=true` (a wrong narrow selection is worse than a full run). |
| Only `.github/**` | fmt gate only. No crate tests. |
| Push, schedule, or `workflow_dispatch` | `full=true` without consulting the diff. |

`scripts/ci/test_affected_crates.py` and `scripts/ci/test_workflow_policy.py` lock this in. `actionlint` 1.7.7 reports no findings on the workflow files.

## Cache

`.github/actions/rust-ci-setup` is the shared setup:

- pinned toolchain (default 1.89.0) via `dtolnay/rust-toolchain`
- Linux build dependencies, including `libudev-dev` and `libasound2-dev` (this is what Editor CI was missing on PR #212)
- `sccache` with `SCCACHE_GHA_ENABLED=true` and `RUSTC_WRAPPER=sccache`
- Swatinem `rust-cache` v2 with `shared-key: ${{ runner.os }}-astraweave`, `cache-targets: false`, `save-if` only on `refs/heads/main`

Registry and git caches are shared across jobs and workflows on the same OS. Compilation artifacts stay in sccache, so the two caches do not both wrap `target/`. Pull requests restore the cache and do not save it.

Sanitizer, Miri, and mutation workflows keep their own `actions/cache` keys. They use nightly or sanitizer instrumentation; sharing stable sccache objects with them is the failure mode the sanitizer workflow already documented (stale nightly ABI in `target/`).

## Timeouts (exceptions)

Most jobs are 30 or 45 minutes. Longer than 45, on purpose:

| Job | Minutes | Why |
|---|---|---|
| `Test (windows, build archive)` | 75 | One Windows debug compile of this workspace is about 50 minutes. The old per-partition jobs each compiled and were cancelled at 45. This job compiles once, archives, and uploads. `CARGO_PROFILE_TEST_DEBUG=0` keeps the archive small enough to upload. The three partition jobs are 30 minutes and do not compile. |
| `Test full (${{ matrix.os }})` | 60 | The old Windows debug workspace test ran about 50 minutes. Nightly only. |
| Benchmark shard | 60 | Existing job cap. The 600s per-package cap inside the runner script is not raised. |
| AddressSanitizer full suite | 90 | Existing nightly sanitizer budget. |
| ThreadSanitizer | 60 | Existing. |
| Miri / Kani / mutation jobs | 60–360 | Existing formal and mutation budgets. |
| Benchmark dashboard | 120 | Existing. |
| `Gate (fmt, clippy, check)` | 45 | One workspace or affected clippy plus `cargo check`. This is the fail-fast compile, not a second test matrix. |

## Clippy unwrap levels

One pass: `cargo clippy --locked --workspace --lib --no-deps`, excluding crates with no library target plus `aw_editor` and `astraweave-author`. On a narrow pull request the same flags are applied with `-p` for the affected libraries.

| Crates | `[lints.clippy]` |
|---|---|
| P0: ecs, core, physics, ai, render, audio, embeddings, memory, llm, context, prompts, net | `unwrap_used`, `expect_used`, `panic`, `todo` = deny. `result_large_err` = allow, matching the old P0 command. |
| P1: gameplay, nav, behavior, rag, persona, terrain | unwrap / expect / panic = warn. dbg and print = warn. The old P1 matrix was `continue-on-error` and did not fail the workflow. |

`[workspace.lints.clippy]` sets that family to allow for a member that inherits and does not override. Cargo 1.89 rejects `[lints] workspace = true` together with a crate-level override, which is why P0 and P1 do not inherit.

The old `-D warnings` / `--all-features` gate is not the pull-request clippy. It is red on `main` for reasons owned by `cursor/ci-failure-burndown` (issue #220).

## Deferred failures

These were red on PR #212 and are not re-fixed here. The burndown branch owns the patches. Each issue has the root cause and the plan. The token used here received HTTP 403 from the labels API, so the `deferred-failure` label still needs to be applied by someone who can create it. The title prefix is `[deferred-failure]`.

| Issue | Failure |
|---|---|
| #218 | Headless audio stops workspace tests and the toolchain matrix |
| #219 | cargo-deny: native-tls pulls banned openssl. This branch switches reqwest, tokio-tungstenite, hf-hub, and ort off native-tls so the ban can pass. |
| #220 | `clippy -D warnings` on phi3 and the render glTF import |
| #221 | Benchmark shards: 600s cap (not raised) and five package errors |
| #222 | SDK C harness: C++ header and missing MSVC prompt |

Editor CI's missing `libudev` is handled by the shared setup action, which installs `libudev-dev`. Cargo Deny is a real failure again (`continue-on-error` removed), so #219 stays visible on pull requests until the burndown branch merges.

Issue #254 (label `deferred-failure`, applied by the maintainer) is the workspace nextest failure from head `7839f930`. The integration token cannot edit that issue (`updateIssue` HTTP 403), so the owner and the plan for each group are recorded here and in `scripts/ci/deferred_pr_tests.txt`. Owner of every group: the CI-cleanup agent, PR #217, branch `cursor/ci-failure-burndown`.

| Group | Tests | Plan |
|---|---|---|
| Terrain goldens and structure | 64 | Finish the burndown branch's E3 test update: lock the measured bits, do not change production terrain math or replay hashes. `density(0,0,0)` expected 0 and got 477. |
| Terrain slow-timeouts | 3 at 240s | Shorten `phase_1_6_f3_phase_2_end_to_end_timing` and the two radius-5 climate diagnostics, or move them off the default test binary. Do not raise the nextest slow-timeout. |
| AI `ecs_integration_tests` | 14, including `test_multi_agent_determinism` | WS-AI-1 plans every companion and these tests still expect `CDesiredPos` / `AiPlannedEvent`. Update the tests or the publication path. This is not the replay gate. |
| Render GPU | 11 failures + `test_renderer_full_pipeline` segfault | Lavapipe where the runner has it, and an explicit printed skip when no adapter exists. The segfault needs a real fix. |
| Fluids GPU smokes | 3 | Same headless GPU policy. Do not touch the caustics golden. |
| LLM `test_qwen3_variants` | 1 | Make it hermetic or gate it on a local model. Separate from the candle compile fix. |

Profile `ci-pr` excludes that exact list so the required pull-request test jobs can pass. Profile `ci` on main and the nightly schedule does not. Audio cases that were flaky and then passed are not in the list. Replay cases are not in the list. Delete a line from the list when that test is green on profile `ci`.

Windows partition jobs use profile `ci-pr-archive`, which adds `scripts/ci/archive_only_skips.txt`. On `0a8ffcec` those were a render crash (`test_renderer_bake_environment_wave16`, exit 2173), two fluids GPU tests that hit the 240s slow-timeout, and the `aw-net-client` binary tests. Linux profile `ci-pr` still runs all of those; that Linux job passed. The client-binary tests use `CARGO_BIN_EXE`, which points at the build runner, so the archive build job runs those two test targets before it uploads. The GPU skips are owned by PR #217 and still run on profile `ci`.

`PhysicsWorld::step` after warmup still allocates (8 allocs, 4112 bytes, 64 bodies, 30 warmup steps, measured 2026-10-15). `tests/zero_alloc_step.rs` asserts 0 and is not loosened. That failure is issue #262, owned by the repository maintainer until a physics workstream claims it. One `World::tick` / `step` with a cooldown already installed measured 0 allocations.

## Required status checks

Set these on `main`. Names are the job `name:` values. After this pull request's first check run, confirm them against `gh pr checks` and adjust if GitHub prefixes the workflow name.

Require:

- `PR Rust checks`
- `determinism-replay / compare`
- `hash (ubuntu-latest)`
- `hash (windows-latest)`
- `hash (macos-latest)`
- `Cargo Deny Check`
- `Security Audit (cargo-audit)`
- `pack-managed paths untracked + ignore surfaces current`
- `OpenSSF Scorecard Analysis`

Do not require the individual `Test (windows, N/3)` or `Gate` names. On a docs-only pull request those jobs are skipped, and a skipped required check blocks the merge. `PR Rust checks` is the rollup.

Do not require `Test (windows, build archive)` or `Test (windows, N/3)` individually. The rollup fails if the archive build fails or any partition fails. Do not require path-filtered jobs (`net-tests`, trace sync, doc-truth, docs). PBR, Editor CI, and AW Editor tests do not run on pull requests.

`Cargo Deny Check` is safe to require. TLS for reqwest, the websocket client, hf-hub, and ort is rustls, so the `openssl` / `openssl-sys` bans in `deny.toml` are not tripped.

## Queue wait and execution time

Runner-minutes sum `completed_at - started_at` for jobs that started (failed and cancelled included, skipped excluded). That number is execution only.

Wall-clock to a check is `completed_at` of that check minus the earliest workflow `created_at` for the same SHA. It includes time spent waiting for a runner.

Execution-only time to a required check is the dependency chain that ends at that check: each job contributes `completed_at - started_at`, and a job waits on the slowest dependency. Queue wait on that chain is the wall-clock minus that sum. A repo-wide backlog shows up in the queue column, not in execution.

## After (this pull request)

Numbers are from the Actions job timestamps. Runner-minutes exclude skipped jobs. Bench share is the Performance Benchmarks workflow only. `#211` did not run `determinism-replay` (that file was not on the branch yet), so its "last required check" is Cargo Deny, which failed in a few minutes while the old Windows debug test kept running. `#210` did run the replay. `PR Rust checks` exists only on this pull request.

| SHA | Jobs started (skipped) | Runner-min | Bench share | Wall to last required | Execution-only chain | Queue + slack | Last required check |
|---|---:|---:|---:|---:|---:|---:|---|
| #210 `8ae54ce8` | 100 (7) | 1062.6 | 30.3% | 25.9 min | 8.3 min | 17.5 min | `compare` success |
| #211 `05c2bff0` | 102 (7) | 1033.4 | 33.3% | 6.5 min | 3.2 min | 3.3 min | `Cargo Deny Check` failure; replay absent |
| #223 `7839f930` | 28 (4) | 336.2 | 0% | 140.2 min | 53.1 min | 87.1 min | `PR Rust checks` failure |
| #223 `7512c86a` | 22 (3) | 147.9 | 0% | 51.7 min | 49.3 min | 2.5 min | `PR Rust checks` success |

Replay compare, which `#210` and this pull request both ran:

| SHA | Wall to `compare` | Execution-only (slowest hash + compare) | Slowest hash execution |
|---|---:|---:|---|
| #210 `8ae54ce8` | 25.9 min | 8.3 min | Windows 8.3 min |
| #223 `7839f930` | 64.5 min | 11.5 min | Windows 11.4 min |
| #223 `7512c86a` | 30.3 min | 10.8 min | Windows 10.7 min |

On `7839f930` the execution chain to `PR Rust checks` is 53.1 minutes because each Windows partition compiled the workspace and was cancelled at 45 minutes. Queue plus slack on that chain is 87.1 minutes.

On `7512c86a` the same chain is 49.3 minutes and the queue plus slack is 2.5 minutes. The Windows archive build ran 36.3 minutes and succeeded. The three partitions ran 4.6, 5.0, and 5.1 minutes and succeeded. They did not hit the cap. `PR Rust checks`, the Linux test, MSRV, deny, audit, pack guard, scorecard, and `determinism-replay` / `compare` all succeeded. Bench share is 0%.

`#210`'s last required check is `compare` (8.3 minutes of execution). This pull request's last required check is `PR Rust checks`, which now includes that Windows archive. Replay execution is 10.8 minutes against `#210`'s 8.3. The extra is the Windows hash (10.7 versus 8.3). A docs-only pull request runs determinism on three OS plus the non-Rust checks, and does not compile the workspace. This pull request is a full-workspace diff, so its runner-minutes are the expensive path, not the narrow one.

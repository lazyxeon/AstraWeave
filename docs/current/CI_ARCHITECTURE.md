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

1. **Benchmarks are off unlabeled pull requests.** `Performance Benchmarks` runs on push to `main` and `develop`, `workflow_dispatch`, and a pull request whose `run-benchmarks` label was just added (`pull_request` types: `[labeled]`). There is no nightly cron. Push to `main` already runs the 16 shards, and the 05:17 UTC cron ran them again (run `40637628179` used 34625s of runner time). A failure on `main` still opens the nightly-failure issue, because that workflow reports a failed `workflow_run` whose head branch is `main`. A later push does not rerun benches; remove and re-add the label, or use `workflow_dispatch`. The 600s per-package cap in `.github/scripts/benchmark-runner.sh` is unchanged. See issue #221.
2. **Concurrency on every pull-request workflow.**

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

A new push cancels the previous pull-request run. Pushes to main, tags, and schedules are not cancelled.
3. **Every job has `timeout-minutes`.** Typical jobs are 30–45. The fast gate is `Gate (fmt, clippy, check)`. Linux tests, the Windows partitions, and MSRV `needs` that gate, so a format or compile failure does not start those runners.
4. **Two tiers.** Nightly Failure Issue (`workflow_run`) opens or updates a GitHub issue when a nightly or main run of the heavy workflows fails. `workflow_run` fires only after this file is on the default branch.
5. **Duplicates are gone.** `rust-cache-optimized.yml` is folded into `ci.yml` and deleted. The 15-job unwrap matrix is deleted. One `cargo clippy --lib --no-deps` pass reads `[lints.clippy]` on each crate. sccache (GitHub Actions cache backend) plus one Swatinem `shared-key` per OS (`${{ runner.os }}-astraweave`), `cache-targets: false`, `save-if` only on `refs/heads/main`. Affected crates come from `scripts/ci/affected_crates.py`. Docs-only diffs skip the Rust compile. Windows and Linux pull-request tests each compile once (`cargo nextest archive`, debuginfo off so the archive stays uploadable) and then run three `count:N/3` partitions against that archive. The Linux archive starts when the gate finishes, in parallel with the Windows archive. It is not built inside the gate. Profile `ci-pr-linux-archive` keeps the Linux GPU tests that `ci-pr-archive` skips, and drops only the four `CARGO_BIN_EXE` binaries, which the archive job runs on the build machine. Determinism tests are not retried.
6. **Required jobs are not queued behind optional compiles.** PBR, Editor CI, and AW Editor tests do not run on `pull_request`. They run on push, their nightly schedule, and `workflow_dispatch`. `cancel-in-progress` is still `${{ github.event_name == 'pull_request' }}` on every workflow that has the block, so a push to `main` or a schedule is never cancelled.

## Tiers

### Pull request (required)

| Check | Workflow job name |
|---|---|
| Affected-crate detect | `Detect affected crates` |
| fmt, one clippy, cargo check | `Gate (fmt, clippy, check)` |
| Linux archive, then 3 partitions | `Test (ubuntu, build archive)`, then `Test (ubuntu, 1/3)`, `Test (ubuntu, 2/3)`, `Test (ubuntu, 3/3)` |
| Windows archive, then 3 partitions | `Test (windows, build archive)`, then `Test (windows, 1/3)`, `Test (windows, 2/3)`, `Test (windows, 3/3)` |
| MSRV (`rust-version` in the workspace manifest, 1.89.0) | `Minimum Supported Rust Version` |
| Rollup of the rows above | `PR Rust checks` |
| Replay hash, Ubuntu / Windows / macOS, then compare. Nineteen required cases: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`, `weaving`, `traversal`, `traversal_deformed`, `worldgen_columns`, `deformed_chunk`, `local_sim`, `greybox_checkpoint`, `quest_ticks`, `persist_replay`. No path filter. | `hash (ubuntu-latest)`, `hash (windows-latest)`, `hash (macos-latest)`, `compare` |
| cargo-deny, cargo-audit | `Cargo Deny Check`, `Security Audit (cargo-audit)` |

These also run on a non-docs pull request when their paths match, and are not in the required set because a path filter would skip them: `net-tests`, trace sync, doc-truth lint, docs. Pack guard and OpenSSF Scorecard run on every pull request. PBR, Editor CI, and AW Editor tests do not run on pull requests.

`PR Rust checks` is the status to require for the Rust rows. The heavy jobs are skipped on a docs-only pull request (they do not compile). GitHub treats a skipped required check as missing, which blocks the merge. The rollup runs on every pull request: it fails if the gate failed or if a crate change left a test job red, and it passes when the heavy jobs were correctly skipped.

### Main, nightly, workflow_dispatch

| Work | When |
|---|---|
| Full 3-OS `cargo nextest` plus a release build of the core crates, and demo validation | `ci.yml` on push, schedule 04:30 UTC, dispatch. Not on pull requests. Ubuntu is one job. Windows and macOS each compile one archive, then run three `count:N/3` partitions. `Test full (windows-latest)` and `Test full (macos-latest)` are the rollups. |
| 5-toolchain matrix (stable, 1.89.0, 1.88.0, beta, nightly) | `toolchain-management.yml` on push to main, schedule 05:40 UTC, dispatch. The checkout sets `lfs: true` (#348) so `ASSET_MANIFEST.tsv` hashes the Aravath blobs, not the pointer files. `Toolchain Validation (1.89.0)` runs workspace clippy `-D warnings`, with that workflow's package excludes. #360 closed the `expect` on schedule registration. #358 is closed. The fix is #360. That 1.89.0 clippy step is expected green. The CI agent is confirming it on main. #375 removes the 50 ms debug wall-clock assert from `view_ring_lod_keeps_the_forest_under_budget`. The test still requires `total > 400` and `kept < 900`. macOS 1.89 on run 39084152246 measured 50.776916 ms, failed `as_millis() < 50`, and skipped `Run clippy with toolchain`. That host-clock assert no longer skips clippy. MSRV stays on the pull request. |
| Sanitizers | `sanitizers.yml` on push and schedule 03:00 UTC. Not on pull requests. |
| Coverage | `coverage.yml` on push to main, schedule 06:10 UTC, dispatch. |
| CodeQL | push to main and weekly (Sunday 03:00 UTC). Not on pull requests. |
| Benchmarks, including `aw_build` | see item 1 above. Shard count is 16 (`shard_index` 0–15, `--shard-count 16`). Shards 13, 14, and 15 are `astraweave-render@a`, `@b`, and `@c` (explicit thirds of that crate's bench targets). #374 flat-samples `fluid_baseline_50k` and is on main. Shard 0 is `astraweave-net`, `astraweave-profiling`, `astraweave-water`, and `astraweave-localsim` (#351). Each sharded bench target gets `timeout 600` after `cargo bench --no-run`. Compilation is outside that limit. The `aw_build` job is separate and uses its 30-minute job timeout. The aggregate expects 53 package records. `github-action-benchmark` stores `dev/bench` on branch `benchmark-data`. `gh-pages` is the docs site, and main also has a `gh-pages/` directory, so the old checkout never published that history. The PR comment step clones `github.com/<owner>/<repo>`, does not auto-push, and comments only when a result exceeds the alert threshold. |
| Integration tests other than the replay hash | `integration-tests.yml` on push and schedule 04:00 UTC. |
| PBR build and editor tests, all three OS | `pbr-pipeline-ci.yml`, `editor-ci.yml`, `aw-editor-tests.yml` on push, their nightly schedules, and `workflow_dispatch`. Not on pull requests. |
| Allocation ceilings, zero-alloc hot paths, CPU tick-time artifact | `allocation-measurement.yml` on push to `main`, schedule 05:45 UTC, and `workflow_dispatch`. Not on pull requests. The 11 ms GTX 1660 Ti check is the `gpu_frame_budget` dispatch input on a self-hosted runner labeled `gpu`. #216 is on main. The recorded heavy scene on a GTX 1660 Ti Max-Q is `median_gpu_frame_ms=3.688`, `budget_11ms=pass`, with the amortized cloud shadow at 0.323 ms. #371 and #372 stay open. |
| SDK C harness | push to main, schedule 06:25 UTC, dispatch. |

## Determinism

`determinism-replay.yml` runs on every pull request and on push to `main`. It has no path filter. Each OS runs the core replay test, the combat replay test, `replay_aravath_slice_scenario`, the causality replay test, the scripting replay test, `astraweave-physics` `traversal_replay`, `astraweave-terrain` `worldgen_hash`, `astraweave-localsim` `replay_hash`, `aravath_runtime` `checkpoint_replay`, `astraweave-quests` `quest_tick_replay`, and `astraweave-persistence-ecs` `persist_replay`, with the baseline CPU pinned (`target-cpu=x86-64` on x86_64; aarch64 stays on the default baseline). `cargo test` has no retry loop. The hash step sets `CARGO_PROFILE_TEST_DEBUG=0` on every OS (debuginfo off, debug assertions on) so the Windows link is not carrying full debuginfo. The `compare` job requires `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`, `weaving`, `traversal`, `traversal_deformed`, `worldgen_columns`, `deformed_chunk`, `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay` to be present and byte-identical across the three artifacts. `traversal` and `traversal_deformed` come from `replay-hash-traversal.json`. `worldgen_columns` and `deformed_chunk` are appended to `replay-hash.json` by `worldgen_hash` before the merge step. `local_sim` comes from `replay-hash-localsim.json`. `greybox_checkpoint`, `quest_ticks`, and `persist_replay` come from `replay-hash-greybox.json`, `replay-hash-quests.json`, and `replay-hash-persist.json`. `.github/workflows/determinism-profile.yml` runs the seven terrain bit locks under `--release` nightly at 05:30 UTC on Ubuntu, macOS, and Windows. The job `terrain-bit-locks` has `timeout-minutes: 45`. It is not a pull-request check. #327 is on `main`. The workspace `tokio` dependency does not enable `net`, and #326 is closed. Benchmark targets whose measurement exceeds 600s stay #224. The `weaving` case still hashes `NullField`. The greybox local field is `astraweave-localsim` (#330).

When the same tests also run under nextest (an affected crate on a pull request, or the full nightly matrix), `.config/nextest.toml` profile `ci` sets `retries = 2` for ordinary tests and `retries = 0` for test names matching `determinism`, `replay`, or `deterministic`. The determinism workflow does not call nextest.

## Affected crates and docs-only pulls

`scripts/ci/affected_crates.py` reads `git diff --no-renames --name-only` (or `--files` in tests) and `cargo metadata --no-deps`. A rename is the old path and the new path, so a Rust file moved onto a docs path is not docs-only, and a move between crates selects both.

| Diff | Result |
|---|---|
| Only `docs/**`, `gh-pages/**`, or `*.md` | `docs_only=true`. The gate does not install a toolchain. Test jobs do not start. Determinism still runs. |
| Root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/**`, or `.config/nextest.toml` | `full=true`. Workspace clippy, check, nextest, and MSRV. |
| `.github/workflows/**`, `.github/actions/**`, or `.github/scripts/**` | `full=true`. A workflow, composite action, or CI script edit runs the full set. |
| Files under one or more crates | Those crates plus workspace reverse dependencies. |
| A file that is not docs, not under `.github/`, and not inside a crate | `full=true` (a wrong narrow selection is worse than a full run). |
| Other `.github/**` only (for example `CODEOWNERS`) | fmt gate only. No crate tests. |
| Push, schedule, or `workflow_dispatch` | `full=true` without consulting the diff. |
| Pull request whose checkout has no first parent | `full=true`, `reason=missing-merge-parent`. |

`Detect affected crates` checks out with `fetch-depth: 2` and `fetch-tags: false`. On a pull request that checkout is `refs/pull/N/merge`, and the classifier diffs `HEAD^1...HEAD`. The first parent is the base commit of the merge GitHub built. `actions/checkout` treats depth 0 as every branch and every tag, including when `fetch-tags` is false. Run `40747800523` (pull request #484, head `710d84a3`) spent 600s in that fetch. Classify was skipped, and `PR Rust checks` failed because `changes=cancelled`.

`--scope` on the same script sets `run=true` or `run=false` for one heavy workflow. `run=false` only when the diff is known and the expanded crate set misses that scope. An empty diff, any full trigger above, or a scope package missing from metadata sets `run=true`.

| Scope | Workflow | Runs when the expanded set contains |
|---|---|---|
| `aravath` | `aravath-windows.yml` | `aravath_runtime`, `aw-save`, or `astraweave-render` |
| `aravath-determinism` | `aravath-determinism.yml` | `aravath_runtime` |
| `render-device` | `render-device-validation.yml` | `astraweave-render` or `astraweave-ecs` |
| `net` | `net-tests.yml` | `astraweave-net` |

Those four workflows still trigger on every pull request. The heavy jobs are conditional. A skipped job reports success to branch protection. `on.paths` is not used: a path filter would leave a required check pending. `determinism-replay.yml` has no scope and no path filter. The Aravath golden-hash matrix still expands when the scope is cold, so the three `Replay hash (...)` check names are created; the compile steps are skipped. `net-tests.yml` no longer uses `paths-ignore` for markdown. A docs-only pull request classifies `run=false` and skips the `net-tests` job by condition instead of omitting the workflow. Push, schedule, and `workflow_dispatch` do not consult the diff.

`scripts/ci/test_affected_crates.py` and `scripts/ci/test_workflow_policy.py` lock this in. `actionlint` 1.7.7 reports no findings on the workflow files.

## Cache

`.github/actions/rust-ci-setup` is the shared setup:

- pinned toolchain (default 1.89.0) via `dtolnay/rust-toolchain`
- Linux build dependencies, including `libudev-dev` and `libasound2-dev` (this is what Editor CI was missing on PR #212)
- `sccache` 0.19.0 with `SCCACHE_GHA_ENABLED=true` and `RUSTC_WRAPPER=sccache`. `CARGO_INCREMENTAL=0` is set in this action and in `ci.yml`. Pull requests and non-main refs set `SCCACHE_GHA_RW_MODE=READ_ONLY`. Pushes and schedules on `refs/heads/main` set `READ_WRITE`. The action does not set `RUSTFLAGS`: Cargo 1.89 replaces every `rustflags` entry in `.cargo/config.toml` when that variable is set. The Linux CPU pin is `[target.x86_64-unknown-linux-gnu] rustflags = ["-C", "target-cpu=x86-64"]`, beside the Windows 16 MiB stack flags. aarch64 keeps the default baseline. sccache 0.19 does not cache `bin`, `cdylib`, or `proc-macro` crate types.
- Swatinem `rust-cache` v2 with `shared-key: ${{ runner.os }}-astraweave`, `cache-targets: false`, `save-if` only on `refs/heads/main`

Registry and git caches are shared across jobs and workflows on the same OS. Compilation artifacts stay in sccache, so the two caches do not both wrap `target/`. Pull requests restore both caches and do not save them. Job `121303738868` (run `40235303264`) recorded rust hit rate 11.01%, 263 cache write errors, and 826 non-cacheable calls (621 `crate-type`, 113 `bin`, 66 `proc-macro`) while `effective_cache_mode` was `ReadWrite`. Gate samples from the same window were 24.34% and 38.52% overall.

Sanitizer, Miri, and mutation workflows keep their own `actions/cache` keys. They use nightly or sanitizer instrumentation; sharing stable sccache objects with them is the failure mode the sanitizer workflow already documented (stale nightly ABI in `target/`).

## Timeouts (exceptions)

Most jobs are 30 or 45 minutes. Longer than 45, on purpose:

| Job | Minutes | Why |
|---|---|---|
| `Test (windows, build archive)` | 75 | One Windows debug compile of this workspace is about 50 minutes. The old per-partition jobs each compiled and were cancelled at 45. This job compiles once, archives, and uploads. `CARGO_PROFILE_TEST_DEBUG=0` keeps the archive small enough to upload. The three partition jobs are 30 minutes and do not compile. |
| `Test full (ubuntu-latest)` | 60 | The old Windows debug workspace test ran about 50 minutes. Nightly only. The job name is still `Test full (${{ matrix.os }})` with `os: [ubuntu-latest]`. |
| `Test full (macos, build archive)` and `Test full (macos, N/3)` | 60 | Issue #274. Push run 40879883973 cancelled the single macOS job at 60m22s during nextest (test 17213/27309). Compile once, then three partitions, each with a 60-minute budget. Same shape as Windows Test full (#433). `CARGO_PROFILE_TEST_DEBUG=0`. The rollup `Test full (macos-latest)` is 5 minutes. |
| Benchmark shard | 60 | Existing job cap. Each bench target still gets 600s after compilation. |
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

The old `-D warnings` / `--all-features` gate is not the pull-request clippy. #217 is on `main` at `a66304dc`. Its ledger records the phi3 and glTF import fixes that used to fail that invocation. This page does not claim a fresh workspace `-D warnings` run.

## Deferred failures

These were red on PR #212. #217 (`a66304dc`) is the burndown that landed. The rows below are the state written before that merge. The ledger at `CI_FAILURE_LEDGER.md` is the disposition. #219 and #256 are closed: `openssl` and `openssl-sys` stay banned in `deny.toml`, and neither name is in `Cargo.lock`. #327 is on `main` and #326 is closed. The title prefix is `[deferred-failure]`.

| Issue | Failure |
|---|---|
| #218 | Closed. GitHub attributes the close to #333. The null backend and `aw_audio_probe` landed in #217. |
| #219 | Closed by #217. reqwest, tokio-tungstenite, and hf-hub use rustls. `ort` does not enable `download-binaries`. The openssl ban is unchanged. |
| #220 | `clippy -D warnings` on phi3 and the render glTF import |
| #221 | Benchmark shards: 600s cap (not raised) and five package errors |
| #222 | SDK C harness: C++ header and missing MSVC prompt |

`aw-editor-tests.yml` installs `pkg-config`, `libudev-dev`, and `libasound2-dev` on Ubuntu. The sanitizer jobs install `pkg-config` and `libudev-dev`. `editor-ci.yml` uses `rust-ci-setup`, which already installed `libudev-dev` and now also installs `mesa-vulkan-drivers`, `libvulkan1`, and `vulkan-tools`. Cargo Deny remains a failing check. #219 is closed.

Issue #254 is the workspace nextest failure from head `7839f930`. #217 fixed the audio fixture directory. #333 (`30afc26d`) removed the terrain rows and the AI planning rows from `scripts/ci/deferred_pr_tests.txt`. #254 and #242 stay open. #314 stays open: the folder-scan ordering is fixed, and the issue still names `test_distance_attenuation_files`.

| Group | Tests | Plan |
|---|---|---|
| Terrain goldens and structure | Were 64 | **Off the skip list (#333).** Goldens lock `NoiseConfig::default`. #242 stays open. |
| Terrain slow-timeouts | Were 3 at 240s | **Off the skip list (#333).** The radius-5 diagnostics and `phase_1_6_f3_phase_2_end_to_end_timing` check one chunk per climate. The slow-timeout is not raised. |
| AI `ecs_integration_tests` | Were 14 | **Off the skip list (#333).** `sys_ai_planning` keeps `RuleOrchestrator` when `veilweaver_slice` is unified in. #254 stays open. |
| Render GPU | The other rows stay excluded | `test_renderer_lifecycle_stress_test` is green (#355, #352 closed): one shared wgpu Instance and Adapter, the first create/destroy is the warm baseline, growth after that stays under 50 MB, and `generate_report` shows zero user-held buffers, textures, devices, and queues. The line is still in `scripts/ci/deferred_pr_tests.txt`, so profile `ci-pr` still skips it. `test_buffer_lifecycle_no_leaks` stays with #246. `test_renderer_full_pipeline` is still the segfault. |
| Fluids GPU smokes | 3 | Still excluded from profile `ci-pr`. Do not touch the caustics golden. |
| LLM `test_qwen3_variants` | 1 | Still excluded from profile `ci-pr`. Make it hermetic or gate it on a local model. |

Profile `ci-pr` excludes the rows that remain in `scripts/ci/deferred_pr_tests.txt`. Profile `ci` on main and the nightly schedule does not. Replay cases are not in the list. Delete a line from the list when that test is green on profile `ci`. #356 split the cinematics, net-ecs, and scene benches (shards 1, 5, and 9) into per-group targets. #361 did the same for dialogue, embeddings, and fluids on shards 2 and 4. #359 is closed. The 600s limit stays. #374 flat-samples `fluid_baseline_50k`: `SamplingMode::Flat`, `sample_size(10)`, a 1 ms warm-up, and a 1 s measurement. It still steps 50_000 particles. 10k and 20k stay at `sample_size(30)`. The shard 4 timeout is resolved. The 600s cap is unchanged. #377 is on main. The matrix is 16 shards. `camera_primitives_instancing`, `gpu_memory_terrain_skinning_depth_overlay`, `graph_mesh_material_texture`, `ibl_deferred`, and `transparency_environment_msaa` use `SamplingMode::Flat`, `sample_size(10)`, a 1 ms warm-up, and a 1 s measurement. In `ibl_deferred`, spherical harmonics then sets `sample_size(50)` and the BRDF LUT row sets `sample_size(30)`. `large_forest_10k_trees` scatters with `StdRng` seeded `0xA57A_13E7`. #367 is on main: `raycast_with_and_without_normal` calls `world.step()` before the cast, so shard 7 no longer exits 101 on an empty query pipeline. The fluids GPU timing walk runs only when `AW_FLUID_GPU_TIMINGS` is set. #224 stays open.

Windows pull-request partition jobs use profile `ci-pr-archive`, which adds `scripts/ci/archive_only_skips.txt`. On `0a8ffcec` those were a render crash (`test_renderer_bake_environment_wave16`, exit 2173), two fluids GPU tests that hit the 240s slow-timeout, and the `aw-net-client` binary tests. Linux profile `ci-pr` still runs all of those; that Linux job passed. The client-binary tests use `CARGO_BIN_EXE`, which points at the build runner, so the archive build job runs those two test targets before it uploads. The model HTTP probes (`loopback_http_proxy`, `ollama_chat_probe`) have the same path problem and run on that build job too. Nightly Windows and macOS Test full partitions use profile `ci-full-archive`, which drops only those four binaries. The Test full archive job runs them with profile `ci`. The GPU rows stay on profile `ci`, so nightly Windows and macOS still run them. The explicit GPU skip from #217 is on `main` and still runs on profile `ci`. macOS partitions each write `target/gpu-skip-reasons.txt`. `Test full (macos-latest)` concatenates those logs and runs `scripts/ci/check-gpu-skip-log.sh`, so every name in `scripts/ci/gpu-skip-required-macos.txt` is still required.

The 2026-10-15 measurement was 8 allocs and 4112 bytes on a settled `PhysicsWorld::step` (64 bodies, 30 warmup steps). #350 (`f2075426`) vendors `rapier3d` 0.22 so that step returns the modified-collider buffer. `one_physics_step_allocates_nothing` stays at 0. An in-house physics engine is not planned work. #262 stays open. One `World::tick` / `step` with a cooldown already installed measured 0 allocations.

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

`aravath-windows.yml` job `cargo-audit, cargo-deny, SBOM` installs `cargo-audit` 0.22.2, `cargo-deny` 0.20.2, and `cargo-cyclonedx` 0.5.9 from sha256-pinned musl builds via `.github/scripts/install-prebuilt-cargo-tool.sh`. It still runs `cargo deny`, `cargo audit`, `scripts/check-aravath-network.sh`, and `cargo cyclonedx`. That job is not in the require list above. `Cargo Deny Check` and `Security Audit (cargo-audit)` stay the pull-request gate for those two tools.

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

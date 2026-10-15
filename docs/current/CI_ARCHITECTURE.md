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
5. **Duplicates are gone.** `rust-cache-optimized.yml` is folded into `ci.yml` and deleted. The 15-job unwrap matrix is deleted. One `cargo clippy --lib --no-deps` pass reads `[lints.clippy]` on each crate. sccache (GitHub Actions cache backend) plus one Swatinem `shared-key` per OS (`${{ runner.os }}-astraweave`), `cache-targets: false`, `save-if` only on `refs/heads/main`. Affected crates come from `scripts/ci/affected_crates.py`. Docs-only diffs skip the Rust compile. Windows pull-request tests use `cargo nextest` in three `count:N/3` partitions. Determinism tests are not retried.

## Tiers

### Pull request (required)

| Check | Workflow job name |
|---|---|
| Affected-crate detect | `Detect affected crates` |
| fmt, one clippy, cargo check | `Gate (fmt, clippy, check)` |
| Linux tests of affected crates | `Test (ubuntu, affected)` |
| Windows build and test, 3 partitions | `Test (windows, 1/3)`, `Test (windows, 2/3)`, `Test (windows, 3/3)` |
| MSRV (`rust-version` in the workspace manifest, 1.89.0) | `Minimum Supported Rust Version` |
| Rollup of the rows above | `PR Rust checks` |
| Replay hash, Ubuntu / Windows / macOS, then compare. Required cases: `planner_pure`, `perception_world`, `gameplay_rng`. No path filter. | `hash (ubuntu-latest)`, `hash (windows-latest)`, `hash (macos-latest)`, `compare` |
| cargo-deny, cargo-audit | `Cargo Deny Check`, `Security Audit (cargo-audit)` |

These also run on a non-docs pull request when their paths match, and are not in the required set because a path filter would skip them: `net-tests`, `editor-ci`, PBR (Ubuntu and Windows only), AW Editor tests (Ubuntu and Windows only), trace sync, doc-truth lint, docs. Pack guard and OpenSSF Scorecard run on every pull request.

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
| macOS PBR build and macOS editor tests | those workflows' nightly schedule and push. Pull requests keep Ubuntu and Windows. |
| SDK C harness | push to main, schedule 06:25 UTC, dispatch. |

## Determinism

`determinism-replay.yml` runs on every pull request and on push to `main`. It has no path filter. Each OS runs `cargo test -p astraweave-core --test replay_hash` with the baseline CPU pinned (`target-cpu=x86-64` on x86_64; aarch64 stays on the default baseline). `cargo test` has no retry loop. The `compare` job requires `planner_pure`, `perception_world`, and `gameplay_rng` to be present and byte-identical across the three artifacts.

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

Do not require path-filtered jobs (`net-tests`, `editor-ci`, PBR, AW Editor, trace sync, doc-truth, docs). They will not run on unrelated pulls.

`Cargo Deny Check` is safe to require. TLS for reqwest, the websocket client, hf-hub, and ort is rustls, so the `openssl` / `openssl-sys` bans in `deny.toml` are not tripped.

## After (this pull request)

Filled in once `gh pr checks` and the Actions API return timings for this branch. Until then the design target for a narrow crate pull request is about 15–20 checks (the required set plus path-filtered jobs that happen to match), against about 107 before. A docs-only pull request runs determinism on three OS plus the non-Rust checks, and does not compile the workspace. This pull request is a full-workspace diff, so its runner-minutes are the expensive path, not the narrow one.

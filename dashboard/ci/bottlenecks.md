# CI bottlenecks, ranked

Source: [pipeline_profile.md](pipeline_profile.md). Rank is by pull-request wall clock first, then runner time that feeds the queue, then main-only cost. Payoff is estimated from the measured p50, not from a change that had landed when the window closed.

#476 and #478 landed after that window. The window ends at 2026-10-27 23:44:02Z. #478 (`a817c3a3`) merged at 2026-10-28 02:07:58Z and gives nightly `terrain-bit-locks` `timeout-minutes: 45`. #476 (`a94e6838`) merged at 2026-10-28 02:08:07Z and installs `cargo-audit` 0.22.2, `cargo-deny` 0.20.2, and `cargo-cyclonedx` 0.5.9 in the Aravath supply-chain job from sha256-pinned musl builds. The 6.4 min figure in row 5 is entirely the `cargo install` measurement from inside the window. It is not mixed with the prebuilt install. The current job list is [CI_ARCHITECTURE.md](../current/CI_ARCHITECTURE.md).

PRs #439 and #466–#472 are the other CI agent's main-red fixes. Rows that name those PRs are not work for this series to edit.

## 1. Code pull requests wait ~92 minutes on Comprehensive CI

p50 wall is 92.1 min when ubuntu tests run (n=29), p95 121.0 min. The tail is `Test (ubuntu, affected)` (duration p50 32.0 min) racing `Test (windows, build archive)` (duration p50 30.6 min, archive step 28.8 min) and then three partition jobs of about 5 min that start several minutes after the archive upload. `PR Rust checks` is only the aggregator.

Runner wait is in the start offsets: Detect p50 7.5 min before a 48s job, Gate p50 18.1 min before a 6.9 min job, Security Deny p50 13.7 min before a 57s job. A pull request launches the CI matrix, three determinism hashes, three Aravath determinism hashes, the Aravath Windows build, lavapipe, the Windows render job, and net tests at once.

Do not start test jobs before the gate. `test_workflow_policy.py` requires `needs: [changes, gate]`, and a red clippy should not spend a Windows compile.

## 2. macOS Test full hits 60 minutes, and main CI is red

`Test full (macos-latest)` p50 50.5 min, p95 60.2 min, max 60.6 min (n=10). One schedule run was cancelled at the job cap. The other failures are inside `Workspace nextest`. Every main push of Comprehensive CI in the window failed, on this job or on the Windows Test full aggregator.

Not edited here. #439 is the macOS GPU skip. #467 is the Windows archive `CARGO_BIN_EXE` failure. Moving Test full off `main` would hide those failures. Raising the timeout without splitting or skipping a hang does not make the suite finish, and a wider cap is the wrong first move while #439 is in flight.

## 3. Heavy workflows run on diffs that cannot affect them

Billable p50 per pull request: Aravath Windows 41.8 min, Aravath determinism 36.8 min, Render device validation 26.9 min, Net Tests 3.6 min. Combined about 109 runner-minutes, on top of the queue those jobs add to row 1.

`affected_crates.py` already expands reverse dependencies and fails safe to a full run for unmapped files, `Cargo.lock`, the toolchain, and the workspace manifest. It does not yet force a full run for `.github/workflows/` (those diffs are fmt-only). A scope check can skip the four workflows when the expanded package set misses their crates, and must run them when `full=true`, when the diff is empty, or when a scope package is missing from metadata.

`determinism-replay.yml` stays on every pull request. No path filter. The compare job stays required. The Aravath golden hash is a different test and can be scoped; the cross-OS replay workflow cannot.

Skipping a job with `on.paths` leaves a required check pending. A job-level `if` reports success when the job is skipped. A matrix that is skipped before it expands does not create the per-OS check names, so the Aravath determinism matrix has to start and exit rather than disappear.

## 4. determinism-replay Windows hash is ~34 minutes and required

`hash (windows-latest)` duration p50 34.2 min, p95 36.4 min (n=42). The `Replay hash` step is 32.7 min p50. p50 and p95 are close, so this is the compile and the tests, not a timeout and not a one-off cache miss. It cannot move to nightly. Cache work that changes `RUSTFLAGS` or the baseline CPU pin is out of bounds. Two samples are not enough to retune sccache (row 6).

## 5. The Aravath supply-chain's 6.4 min install was `cargo install`

`Install audit tools` p50 6.4 min (n=38) was `cargo install` of audit 0.22.2, deny 0.20.2, and cyclonedx 0.5.9. That sample is entirely from the profile window, before #476. `security-audit.yml` and `dependency-management.yml` already installed audit and deny from pinned prebuilt tarballs with sha256 checks. The same versions were hardcoded in the Aravath job. #476 replaced the three `cargo install --locked --version` lines with `.github/scripts/install-prebuilt-cargo-tool.sh`. The script pins those versions to the musl release tarballs and checks sha256 before extract. The CycloneDX 0.5.9 release publishes `cargo-cyclonedx-x86_64-unknown-linux-musl.tar.xz`.

`cargo deny`, `cargo audit`, `scripts/check-aravath-network.sh`, and `cargo cyclonedx` still run after that install. The 6.4 min figure stays the historical measurement. Deleting the Aravath deny/audit steps because Security Audit also runs them would drop a required-looking job name (`cargo-audit, cargo-deny, SBOM`) onto a different workflow, and branch protection is not readable with this token (403 on branch protection).

Merging the two Security Audit jobs into one was rejected. They already run in parallel and each is under a minute. One job would serialize them. The wall clock is the 13 min queue, which row 3 attacks by launching fewer competing jobs.

## 6. sccache rust hit rate was 21% and 38% on two gate logs

Runs 40235303264 (24.34% overall, 21.17% rust) and 40593627231 (38.52% overall, 226 rust hits, 464 rust misses). `cache-targets: false` is deliberate so sccache and Swatinem do not wrap the same directory. A key or backend change without a controlled rebuild of the same SHA can slow the determinism hashes. Not done in this series. The samples are the baseline for a later cache-only change.

## 7. Main push pays benchmarks twice and a wide toolchain matrix

Sixteen benchmark shards, job p50 from about 19 min to 46 min, run on push to `main` and again from the nightly cron. Toolchain validation on main is p50 22–25 min for stable and 1.89.0, p95 about 40 min, and #468 is fixing a macOS failure in that workflow. Mutation ecs is 57 min p50 and already nightly. None of these are on the pull-request path. Moving the extra toolchains or the duplicate benchmark cron is real runner time, and it waits until #466 and #468 land so this series does not edit those workflows.

## Rejected

| Idea | Why it stays rejected |
|---|---|
| Path-filter or nightly-only `determinism-replay` | Required compare job. Replay hashes on Ubuntu, Windows, and macOS stay on the pull request. The workflow test forbids `paths` and `paths-ignore`. |
| `continue-on-error`, `#[ignore]`, or an allowed-to-fail job | Forbidden. A red main stays red. |
| `RUSTSEC` or `cargo audit` ignore | Forbidden. The Aravath deny retries are a flake or a real advisory, not an ignore entry. |
| Drop Windows partitions or the archive | The partitions are what keep the pull-request Windows test under the old 45 min compile-per-shard failure. #467 is fixing the partition filter, not removing it. |
| Share `target/` across OS jobs | Different toolchains and linkers. The Windows archive is the sharing mechanism that already exists. |
| Start tests in parallel with the gate | A clippy failure would still pay for the Windows archive. The gate-before-tests rule is tested. |
| Treat Lavapipe time as the GPU frame budget | Forbidden. No frame-time number is invented here. |

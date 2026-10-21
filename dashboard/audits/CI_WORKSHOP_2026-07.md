# CI workshop outcome — 2026-07-21

**Supersession (2026-07-21): this note supersedes `AD6_CAMPAIGN_CLOSEOUT.md` §3 as the expected CI-board baseline.**

Implementation commits: `1df55f19f5cef65892ef49af9b27f45cea8e4219` (`ci: retire workflow exceptions`), `2c1ee9f3b85f111510ced48ccbae80b444337255` (`ci: allow cold downstream builds`), `cefebfed68c3cef0228fe38efbe7147a75731c55` (`ci: pin tarpaulin for Rust 1.89`), `e91547a42c3787f2065f65e4067a61f1ccbc835f` (`ci: use non-yanked tarpaulin release`), and `273ded3a979f22a95824d50a669796d57febcf8b` (`ci: separate demo build from smoke timeout`).

## 1. Timeout-cancelled workflows

### Diagnosis

The last three pre-fix runs of each workflow all exhausted their job budgets:

| Workflow | Runs | Where the budget went |
|---|---|---|
| Comprehensive CI Pipeline | `29794346135`, `29711699862`, `29710167176` | Linux/macOS `Fetch dependencies` occupied almost the entire 30-minute job. Latest spans were 28m45s on Ubuntu and 28m28s on macOS, versus 4s on Windows; the prior two runs repeated the same 28m43s–28m51s Linux/macOS shape. |
| Performance Benchmarks | `29794346172`, `29711699867`, `29710167180` | Serial `Run benchmarks` consumed 43m49s, 43m03s, and 43m25s before the 45-minute cancellations. All three had zero cache hits (777–778 misses); the latest reached package 10 of 48. |
| Benchmark Regression Alerts | `29794346175`, `29711699864`, `29710167143` | Serial `Run benchmarks` consumed 58m44s, 58m31s, and 58m39s before the 60-minute cancellations. All three had zero cache hits (800 misses); the latest reached package 12 of 48. |

The benchmark workflows were in a cache death spiral: cancellation preceded the cache post-save step, so every retry stayed cold. Comprehensive CI was a separate defect. `Cargo.lock` and the manifests contain no git dependency, and the repository has no registry-protocol override or `CARGO_NET_GIT_FETCH_WITH_CLI` setting. The workflow exported `RUSTC_WRAPPER=sccache` together with `SCCACHE_NO_DAEMON=1`; the logs stopped before registry activity and showed a non-cacheable compiler request. Cargo was stuck probing rustc through unsupported foreground/no-daemon sccache operation, not spending 28 minutes on the registry.

### Fix and measured result

- Removed `SCCACHE_NO_DAEMON=1`; no timeout or registry workaround was added. In the first post-fix run, Ubuntu dependency fetch fell from 28m45s to 7s.
- That run exposed a second cold-run defect: a Windows downstream lane had no cache and `cargo build --frozen` rejected absent registry content. The three downstream build commands now use `--locked`, retaining lockfile enforcement while allowing cold dependency download.
- A later complete run exposed a third cold-run defect: `timeout 120s make smoke-veilweaver` spent the entire runtime budget compiling and exited 124 before the smoke executable started. The workflow now performs `cargo build --locked -p veilweaver_slice_loader` first under the existing 20-minute job budget, then retains the 120-second cap around the actual smoke invocation.
- Added only deterministic `--shard-index` / `--shard-count` selection to `.github/scripts/benchmark-runner.sh`; its 48-package discovery and execution/result logic were not rewritten.
- Both benchmark workflows now run 12 parallel shards of four packages, with per-shard caches/artifacts and an aggregate job. Performance's budget is 60 minutes; Alerts remains 60 minutes.

Initial cold-headroom arithmetic was `4 packages × 10-minute per-package cap + <1m30s measured setup = <41m30s`, leaving at least 18m30s (44.6%) inside 60 minutes. The verification run disproved that as a sufficient sizing model: Alert shard 5 remained active for 65m00s, and the run was manually cancelled. No further timeout inflation was applied.

The completed shards also exposed a pre-existing result-discovery defect outside the director-authorized selection-only runner change. Performance run [`29868534384`](https://github.com/lazyxeon/AstraWeave/actions/runs/29868534384) completed **failure**: eight shards succeeded, while shards 2, 6, 7, and 8 failed and aggregation was skipped. In shard 7, all four package executions succeeded, yet the runner reported `No criterion results found` for `astraweave-asset-pipeline`, `astraweave-ipc`, `astraweave-pcg`, and `astraweave-security`, then `Total benchmarks collected: 0`. Shard 8 combined the same missing-result behavior with real headless-audio and persistence benchmark execution failures.

Alert run [`29868534385`](https://github.com/lazyxeon/AstraWeave/actions/runs/29868534385) completed **failure**: seven shards succeeded; shards 2, 6, 7, and 8 failed on the same zero-result groups; shard 5 was cancelled after 65m00s; aggregation was skipped. These are neither genuine performance regressions nor timeout retirement. The next CI runner beat needs explicit authorization to make Criterion result discovery recursive/per-package, preserve failures rather than succeeding when any one package yields output, capture timings durably, and then rebalance the measured package groups. Curating a push subset is not recommended because it would reduce the ratified 48-package coverage.

Comprehensive CI replacement run [`29879015995`](https://github.com/lazyxeon/AstraWeave/actions/runs/29879015995) completed **failure**, not cancelled. All four quick-check/MSRV gates, Coverage, Benchmarks, all six build jobs, all three release test jobs, and Demo Validation were green. The final Demo job completed in 5m59s: its separated locked build finished in 3m45s, the subsequent `cargo run` rebuild took 0.26s, and `veilweaver_slice_loader` ran successfully. This proves the 120-second timeout now governs runtime rather than cold compilation.

The final run's remaining reds are code/test findings outside CI-W's write surface: Code Quality failed pinned formatting; macOS debug failed `astraweave-fluids::caustics::tests::test_sample_multi_point_golden` (676 passed / 1 failed); Ubuntu debug failed 202 of 240 `astraweave-audio` tests because no audio device was available; Windows debug failed five `dialogue_runtime` audio tests and exited 139. The coverage installer separately proved the non-yanked tarpaulin 0.35.2 pin on Rust 1.89; its lockfile uses `cargo-platform 0.3.0`, declared Rust 1.83.

## 2. Rust Toolchain Management

Run `29710167136` failed with `error: no such command: check-all`. `.cargo/config.toml` defines none of the workflow's `check-all`, `test-all`, or `clippy-all` aliases. Per the director ruling, the workflow now uses explicit locked workspace `cargo check`, `cargo test --lib`, and `cargo clippy` commands with its existing exclusion list; `.cargo/config.toml` was not changed.

Dispatched run [`29867804192`](https://github.com/lazyxeon/AstraWeave/actions/runs/29867804192) completed **failure**. The MSRV job was green and all nine validation lanes passed the explicit `Check workspace compatibility` step, proving the missing-alias defect retired. All nine then failed the explicit library-test step in headless `astraweave-audio`; the first Linux lane reported 38 passed / 202 failed, with representative failures saying the requested audio device was unavailable. The ratified workflow command was not weakened by excluding the crate. Owner: audio test-harness/code-hygiene beat.

## 3. Pinned-toolchain sweep

Rust Cache run `29794346157` installed current stable Rust 1.97.1 instead of the repository pin in `rust-toolchain.toml` (`1.89.0`). The same-class sweep found 50 literal `dtolnay/rust-toolchain@stable` occurrences across 22 workflow files. All 50 now uniformly use the version-tagged `dtolnay/rust-toolchain@1.89.0` reference. The post-edit sweep found zero `@stable` sites and 53 pinned references (50 changed plus three pre-existing).

Rust Cache runs `29867682254` and [`29868534412`](https://github.com/lazyxeon/AstraWeave/actions/runs/29868534412) installed Rust 1.89.0, fetched dependencies, and compiled the workspace, proving the pin correction. They completed **failure** only because `cargo fmt --all --check` found repository-wide formatting differences. Fresh evidence therefore contradicts the close-out premise that the pinned formatter accepts the tree: the remainder is code-format debt, not workflow drift. Owner: code-hygiene/formatting beat.

Future work: 50 hardcoded pin sites will rot on the next toolchain bump. A separately ratified CI-maintenance beat should centralize toolchain installation in a composite action or reusable workflow.

## 4. Clippy Unwrap-Prevention

Baseline run `29794346156` failed only in P0 `astraweave-render`. Cargo linted its transitive `astraweave-terrain` dependency and raised six `clippy::expect_used` findings in `astraweave-terrain/src/spline_types.rs` (lines 571, 590, 605, 637, 647, and 652). Terrain's absence from the declared matrices did not mean Cargo omitted dependencies. The close-out named the right findings but the wrong mechanism; E3-PF §4.4's statement that fixing them could not flip the board is contradicted.

The workflow now adds `--no-deps` so each lane lints only its declared package and explicitly adds `astraweave-terrain` to the P1 warn-only matrix. Run [`29868534955`](https://github.com/lazyxeon/AstraWeave/actions/runs/29868534955) completed **success** across all 18 lanes. The explicit terrain P1 lane emitted exactly six `expect_used` diagnostics without gating; render P0 passed with `--no-deps`. No code changed. Beat T.2 retains the six-findings hygiene handoff.

## 5. Local validation

- `actionlint 1.7.12`: **PASS** for every edited workflow before each workflow commit, including the later `ci.yml`/`coverage.yml` tarpaulin corrections and the final separated Demo build.
- `shellcheck 0.11.0 .github/scripts/benchmark-runner.sh`: **PASS**.
- `bash -n .github/scripts/benchmark-runner.sh`: **PASS**.
- Invalid shard argument test (`--shard-count 0`): rejected with exit 2 as designed.
- `git diff --check`: **PASS** before each commit.

## 6. Expected CI board

Expected board after CI-W: **all green except:**

- **Comprehensive CI Pipeline** — all identified timeout/stall, cold-fetch, tarpaulin, and smoke-budget workflow defects are retired. Remaining blockers are pinned formatting, the macOS fluids golden test, and Linux/Windows audio tests. Owners: formatting, fluids, and audio code-hygiene/test-harness beats.
- **Performance Benchmarks** — full 48-package sharding now permits most cold shards to complete and save caches, but the runner's non-recursive/per-package result discovery leaves zero-result shards red. Owner: separately authorized CI benchmark-runner beat.
- **Benchmark Regression Alerts** — same result-discovery blocker, plus measured shard 5 exceeded the 60-minute design envelope. Owner: separately authorized CI benchmark-runner beat.
- **Rust Toolchain Management** — nonexistent aliases are retired; headless `astraweave-audio` library tests fail. Owner: audio test-harness/code-hygiene beat.
- **Rust Cache Optimized Build** — toolchain drift is retired; the pinned formatter detects repository-wide formatting debt. Owner: code-hygiene/formatting beat.

Clippy Unwrap-Prevention is green. The named remainder above is the honest gate baseline; no workflow was deleted, disabled, or weakened to make the board green.

- **2026-07-22 — FMT-1:** Pinned-formatting debt is retired under Rust 1.89.0. On Windows, `cargo fmt --all` hits the OS-206 command-length limit, so the repeatable workaround is to derive all 133 workspace members from `cargo metadata --no-deps` and format/check each package; OS-5 sandbox write enforcement additionally required this session-scoped full-access workaround. Rust Cache Optimized Build is expected green, while Comprehensive CI's remaining named exceptions are the macOS fluids golden test and headless Linux/Windows audio tests (owners unchanged).
- **2026-07-24 — CLIP-1:** Terrain's Code Quality clippy exception is retired: the original seven blockers plus the director-authorized lib-test and integration-test findings now pass `cargo clippy -p astraweave-terrain --locked --all-features --all-targets -- -D warnings`. The six `spline_types.rs` `expect_used` findings remain unchanged under T.2 ownership; the terrain lib baseline remains 797 passed / 7 known golden failures / 3 ignored under T.G ownership. Rust Cache Optimized Build and Comprehensive CI's Code Quality gate are expected green.
- **2026-07-24 — CLIP-1 Code Quality handoff:** A no-edit sweep of all 121 Comprehensive Code Quality members found 99 passing packages, 156 unique Rust diagnostics across 21 packages, and one Windows `tikv-jemalloc-sys` environment-blocked package. `AIArbiter.fast_executor` is retained with a field-level `dead_code` allowance as a deferred fast-path wiring item; `astraweave-llm/src/phi3.rs` is STOP-scoped to a dedicated LLM beat because its eight findings include three true compile errors (`E0195`, `E0599`, and `E0061`). The remaining enumerated groups await deliberate director scope; no workflow, crate, feature, target, or lint tier was weakened.
- **2026-07-24 — CI-BR:** Recursive per-package Criterion collection, always-running honest aggregation, and durable package timings retire the zero-result/skipped-aggregate defects while preserving all 48 benchmark packages; the measured 12×4 map uses each package's slower timing from Alerts `30133613866` and Performance `30133654366`, with a 50% package-time reserve plus 240s setup allowance keeping every group below 2,831/3,600s. Both workflows are expected red only on named package-side findings: stale core-bench compilation; embeddings/physics correctness assertions; headless audio and intermittent input runner-resource failures; and 600s workload caps in AI, cinematics, dialogue, ECS, fluids, net-ECS, persistence-ECS, RAG, render, scene, SDK, and UI. Owners: the named package benchmark/code owners, with audio/input environment hardening owned by their test-harness/CI owners.
- **2026-07-24 — CI-BR verification addendum:** Final Performance run `30136578958` additionally measured `astraweave-quests` crossing the unchanged 600s package cap after it completed in 531s during Alerts; quests therefore joins the named package-workload owner set, with the expected red-only-on-owned-findings board state unchanged.
- **2026-07-29 — CI-BR evidence clarification:** Final Alerts `30136578973` and Performance `30136578958` truthfully account for all 48 package records as 46 packages with collected Criterion output + 0 collection exceptions + 2 named no-output execution failures (`astraweave-core` stale-bench compilation and `astraweave-audio` headless-device failure), while retaining the red outcome; for the original shard-5 map (`astraweave-terrain`, `astraweave-fluids`, `astraweave-observability`, `astraweave-sdk`), retained measurement artifacts from Alerts `30133613866` and Performance `30133654366` give slower times of 264 + 600 + 366 + 600 = 1,830s and a 1.5× reserve + 240s setup estimate of 2,985/3,600s (615s headroom), so the earlier 65m cancellation was anomalous rather than explained by intrinsic measured package load, and its exact cause is unavailable because historical run `29868534385` retained no shard-5 steps, log, or artifact.

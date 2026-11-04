# Rust toolchain

Checked against `main` at `cdea968c` (2026-10-16). The status line under Pull-request compile is current as of `556a305c` (2026-10-20).

AstraWeave pins Rust **1.89.0** in `rust-toolchain.toml`, with `rustfmt`, `clippy`, `rust-src`, `rust-analyzer`, and `llvm-tools-preview`. Targets listed there:

- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

```bash
rustup show
cargo --version
```

`rustup show` should report the pinned 1.89.0 channel. A newer default toolchain on the machine does not replace the pin inside this repository.

## What the repo configures

`.cargo/config.toml` sets a 16 MB Windows stack, `[target.x86_64-unknown-linux-gnu] rustflags = ["-C", "target-cpu=x86-64"]`, and three editor aliases (`editor`, `editor-release`, `editor-dev`) plus `xtask`. aarch64 keeps the default baseline. It does not set `rustc-wrapper = "sccache"`.

CI caching lives in `.github/workflows/ci.yml` (Swatinem's rust-cache action, one shared key per OS, `cache-targets: false`, save on `main`). The former `rust-cache-optimized.yml` file is not in the tree. There is no measured "50–80% faster" figure to cite from the current workflow.

The workspace has **141** members (`cargo metadata --no-deps` on this commit). The count at `5cba174c` was 140. The June 2026 count of 130 is older.

## Pull-request compile

`ci.yml` runs a fmt/clippy/check gate, then a Linux nextest archive and three partitions and a Windows archive and three partitions (debuginfo off on both archives), and an MSRV job at 1.89.0. Docs-only diffs skip that compile. Replay hashes still run on Linux, Windows, and macOS, with `CARGO_PROFILE_TEST_DEBUG=0` on every OS. Pull requests set `SCCACHE_GHA_RW_MODE=READ_ONLY`. See `docs/current/CI_ARCHITECTURE.md`.

Current status: #217, #327, #275, #329, #330, #331, #332, #333, #340, #343, #348, and #351 are on `main` (`556a305c`). The Rapier re-apply note is `third_party/rapier3d/VENDORED.md`. The upstream draft is `docs/current/RAPIER_BUFFER_REUSE_UPSTREAM.md` and is not filed. #368 covers the six editor clicks with synthetic `RawInput`. #366 step 2, `egui_kittest`, waits on upstream AccessKit and zbus 6. #361 split shards 2 and 4. #359 is closed. The fluids GPU timing walk runs only when `AW_FLUID_GPU_TIMINGS` is set. #224 stays open. #365 drops AccessKit and the editor `kittest` feature. File dialogs are `rfd` 0.17. RUSTSEC-2026-0370, 0371, and 0372 are resolved with no ignore. #387 builds postcard with `default-features = false` plus `alloc`, so `atomic-polyfill` is gone. #320 is closed. The fix is #387. `rustls-webpki` is 0.103.16 and `webpki-roots` is 1.0.10. RUSTSEC-2026-0381 and RUSTSEC-2026-0382 are fixed with no ignore. #366 step 2 is in progress. #350 vendors `rapier3d` 0.22 under `[patch.crates-io]`. A settled physics step allocates nothing. An in-house physics engine is not planned work. #262 stays open. `toolchain-management.yml` checks out Git LFS. Benchmark shard 0 includes `astraweave-localsim`. The matrix is 16 shards (`0`–`15`). `astraweave-render` is shards 13, 14, and 15. Each sharded bench target gets 600s after compilation. The `aw_build` job is separate and uses its 30-minute job timeout. #366 step 2 waits on upstream AccessKit and zbus 6 and is not on `main`. #367 is on `main`: `raycast_with_and_without_normal` calls `world.step()` before the cast. #360 is on `main`. `add_system_known_stage` registers the stage first. #358 is closed. The fix is #360. `Toolchain Validation (1.89.0)` is expected green. The CI agent is confirming it on main. #375 removes the 50 ms debug wall-clock assert from `view_ring_lod_keeps_the_forest_under_budget`. The test still requires `total > 400` and `kept < 900`. macOS 1.89 on run 39084152246 measured 50.776916 ms and skipped `Run clippy with toolchain`. That host-clock assert no longer skips clippy. #374 flat-samples `fluid_baseline_50k` (`SamplingMode::Flat`, `sample_size(10)`, 1 ms warm-up, 1 s measurement). The shard 4 timeout is resolved. The 600s cap is unchanged. #377 is on main. Five render benches use flat `sample_size(10)`, a 1 ms warm-up, and a 1 s measurement. `large_forest_10k_trees` is seeded. Benchmark history is `dev/bench` on branch `benchmark-data`. #352 is closed. The greybox golden is `b6a9d5a2c5da28a6` and #381 runs it on macOS as well as Linux and Windows. #281, #283, and #284 are closed. `CLOUD_SHADOW_STRENGTH` is 4 (#384). #382 reads `CAiController.gather` only when `ai-goap` is on. The default build returns an error instead. #257, #258, and #335 are closed. #334 and #383 stay open. #385 drops the RUSTSEC-2025-0119 ignore. `indicatif` is 0.18.6. #307 is closed. Required replay cases are nineteen, including `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`. `gravity.rs` is deleted. #253, #273, #336, and #338 are closed. #218 is closed. The workspace `tokio` dependency does not enable `net`. #326 is closed. Greybox startup runs `run_aravath_slice` once. Each sword hit reuses that `sword_hp`. A checkpoint stores the demolition command log. Profile `ci-pr` still excludes GPU render passes, lavapipe fluid smokes, and `test_qwen3_variants`. #254, #242, #314, #304, and #274 stay open. `AIArbiter` and `GoapPlanner` stay off the deterministic tick. #366 step 2 (`egui_kittest` waits on upstream AccessKit and zbus 6) is not on `main`. #367 is on `main`. #216 is on `main`. The heavy-scene GPU frame on a GTX 1660 Ti Max-Q is 3.688 ms, `budget_11ms=pass`, and the amortized cloud shadow is 0.323 ms. #371 and #372 stay open.

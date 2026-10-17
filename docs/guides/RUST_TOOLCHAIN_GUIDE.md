# Rust toolchain

Checked against `main` at `cdea968c` (2026-10-16).

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

`.cargo/config.toml` sets a 16 MB Windows stack and three editor aliases (`editor`, `editor-release`, `editor-dev`) plus `xtask`. It does not set `rustc-wrapper = "sccache"`.

CI caching lives in `.github/workflows/ci.yml` (Swatinem's rust-cache action, one shared key per OS, `cache-targets: false`, save on `main`). The former `rust-cache-optimized.yml` file is not in the tree. There is no measured "50–80% faster" figure to cite from the current workflow.

The workspace has **141** members (`cargo metadata --no-deps` on this commit). The count at `5cba174c` was 140. The June 2026 count of 130 is older.

## Pull-request compile

`ci.yml` runs a fmt/clippy/check gate, tests of affected crates, a Windows archive split into three partitions, and an MSRV job at 1.89.0. Docs-only diffs skip that compile. Replay hashes still run on Linux, Windows, and macOS. See `docs/current/CI_ARCHITECTURE.md`.

#217, #327, #275, #329, #330, #331, #332, #333, #340, #343, #348, and #351 are on `main` (`0758171b`). `toolchain-management.yml` checks out Git LFS. Benchmark shard 0 includes `astraweave-localsim`. The matrix is 14 shards. Each sharded bench target gets 600s after compilation. The `aw_build` job is separate and uses its 30-minute job timeout. #350 is in progress and is not on `main`. The greybox golden is `b6a9d5a2c5da28a6`. Required replay cases are nineteen, including `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`. `gravity.rs` is deleted. #253, #273, #336, and #338 are closed. #218 is closed. The workspace `tokio` dependency does not enable `net`. #326 is closed. Greybox startup runs `run_aravath_slice` once. Each sword hit reuses that `sword_hp`. A checkpoint stores the demolition command log. Profile `ci-pr` still excludes GPU render passes, lavapipe fluid smokes, and `test_qwen3_variants`. #254, #242, #314, #304, and #274 stay open. `AIArbiter` and `GoapPlanner` stay off the deterministic tick. Render frame budget #216 and physics zero-alloc #350 are not on `main`.

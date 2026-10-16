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

#217, #327, #275, #329, #330, #331, and #343 are on `main` (`fb1c8ec2`). The greybox golden is `b6a9d5a2c5da28a6`. Required replay cases are sixteen, including `local_sim`. `gravity.rs` is deleted. #253 and #336 are closed. The workspace `tokio` dependency does not enable `net`. #326 is closed. Greybox startup runs `run_aravath_slice` once. Each sword hit reuses that `sword_hp`. A checkpoint stores the demolition command log. `AIArbiter` and `GoapPlanner` stay off the deterministic tick. Render frame budget #216 is not on `main`.

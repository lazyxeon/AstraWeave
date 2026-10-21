# Build quick reference

Checked against `main` at `10757c81` (2026-10-16). The player-facing name is Aravath. The windowed demo is `cargo run -p aravath_runtime --bin aravath_greybox`. Its golden is `b6a9d5a2c5da28a6`. Required replay cases are nineteen, including `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`. `gravity.rs` is deleted (#336 closed). `AIArbiter` and `GoapPlanner` stay off the deterministic tick (#331).

## Daily commands

```bash
rustup show   # must report 1.89.0, from rust-toolchain.toml

cargo check -p astraweave-core
cargo test -p astraweave-core
cargo fmt --all
cargo clippy -p astraweave-core --all-features -- -D warnings

cargo run -p hello_companion --release
cargo xtask fetch-assets
cargo editor          # aw_editor, release-fast profile
```

Workspace-wide `cargo check --workspace` and `cargo test --workspace` are real commands. The editor aliases are only `editor`, `editor-release`, and `editor-dev` (`.cargo/config.toml`).

This page does not exclude crates. A June 2026 audit recorded the former authoring and egui demo failures as compiling. This pass did not re-run `cargo check --workspace`.

## What CI runs

Pull requests use `.github/workflows/ci.yml`. The old `rust-cache-optimized.yml` workflow file is gone; that work was folded into `ci.yml`. Cross-OS replay hashes are `.github/workflows/determinism-replay.yml`.

Required replay cases: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`, `weaving`, `traversal`, `traversal_deformed`, `worldgen_columns`, `deformed_chunk`.

Job tiers and which checks are required are written up in `docs/current/CI_ARCHITECTURE.md`. #217 and #327 are on `main`. #326 is closed.

## Toolchain

- Pin: Rust 1.89.0 (`rust-toolchain.toml`)
- Targets in that file: `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Windows link stack is 16 MB (`.cargo/config.toml`), because some state structs are large
- `sccache` is not configured as `rustc-wrapper` in `.cargo/config.toml`

## Determinism

Do not set `target-cpu=native` for replay hashes. The workflow pins `x86-64` on x86_64 hosts and leaves Apple Silicon on the default baseline.

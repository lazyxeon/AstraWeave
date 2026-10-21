# AstraWeave

AstraWeave is an AI-native game engine with a deterministic ECS. AI agents are first-class citizens. The game this engine is being built for is **Aravath**. Local LLMs propose. The deterministic simulation validates and executes. Assets in the tree are CC0. The frame target is under 11 ms on a GTX 1660 Ti. The measured heavy-scene GPU frame on that card is 3.688 ms (`budget_11ms=pass`). The full write-up, with the supply-chain pins and the per-system table, is [ENGINE_STATUS.md](docs/current/ENGINE_STATUS.md).

[![Kani Formal Verification](https://github.com/lazyxeon/AstraWeave/actions/workflows/kani.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/kani.yml) [![OpenSSF Scorecard](https://github.com/lazyxeon/AstraWeave/actions/workflows/scorecard.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/scorecard.yml) [![Miri UB Detection](https://github.com/lazyxeon/AstraWeave/actions/workflows/miri.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/miri.yml) [![Security Audit](https://github.com/lazyxeon/AstraWeave/actions/workflows/security-audit.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/security-audit.yml) [![Mutation Testing](https://github.com/lazyxeon/AstraWeave/actions/workflows/mutation-testing.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/mutation-testing.yml) [![Sanitizers](https://github.com/lazyxeon/AstraWeave/actions/workflows/sanitizers.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/sanitizers.yml) [![CodeQL Analysis](https://github.com/lazyxeon/AstraWeave/actions/workflows/codeql-analysis.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/codeql-analysis.yml)

## Current status

Main is `49d289eb` (#397). #390 is merged as `5a52b039`. The rows below were checked at `556a305c` except the CI badge row and the weave-origin row.

| Topic | State |
| --- | --- |
| Toolchain | Rust 1.89.0, pinned in `rust-toolchain.toml` |
| Workspace | 142 members at `10757c81` |
| Determinism | Nineteen required replay cases, compared on Linux, Windows, and macOS |
| Greybox golden | `b6a9d5a2c5da28a6` |
| Frame budget | 3.688 ms GPU frame, 11 ms target, GTX 1660 Ti. #371 and #372 stay open |
| Supply chain | Postcard is alloc-only. `rustls-webpki` is 0.103.16. `webpki-roots` is 1.0.10. Ignored leftovers are #305, #306, #308–#310, and #321–#323 |
| CI badges | #390 is merged. It dropped the Clippy unwrap-prevention badge. That workflow was deleted in #223. `astraweave-ai` mutants run as 8 round-robin shards. The 70% gate is caught / (caught + missed). Timeouts are not caught. `cargo-mutants` is 27.1.0 |
| Weave origins | #388 closed #295. `queue_weave` returns `OutsideIsland` outside ±2048 cells and does not saturate. The `weaving` case still hashes `NullField` |

The nineteen cases are `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`, `weaving`, `traversal`, `traversal_deformed`, `worldgen_columns`, `deformed_chunk`, `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`.

## Features

- Single-threaded deterministic tick. Replay hashes must match across operating systems.
- `sys_ai_planning` runs `RuleOrchestrator`. `dispatch_planner` is not on that tick. `PlannerMode::GOAP` reads `CAiController.gather` only when `ai-goap` is on.
- Windowed Aravath greybox: CC0 props, sword combat, and a checkpoint of the fight fields.
- wgpu rendering with a shared editor and engine path. Cloud-shadow strength is 4. The amortized `cloud_shadow` pass is 0.323 ms at `939a21bb`, which is not the 3.688 ms GPU frame.
- Rapier is vendored so a settled physics step allocates nothing. An in-house physics engine is not planned work.

## Quick start

```bash
cargo check -p astraweave-core
cargo test -p aravath_runtime --lib replay_matches_golden_hash
cargo run -p hello_companion --release
cargo xtask fetch-assets
cargo editor
```

Docs gates: `cargo run -p aw_doc_lint -- --mode warn` and `cargo run -p aw_trace_sync -- --check`.

## In progress

These pulls are not on `main`.

- **#334** — delete `AIArbiter` once benches have a deterministic replacement
- **#366** — step 2: `egui_kittest` waits on upstream AccessKit and zbus 6
- **#383** — port advanced GOAP and `AIArbiter` into the deterministic HTN tick

## Docs

- [Engine status](docs/current/ENGINE_STATUS.md) — the long status page this file used to be
- [Project status](docs/current/PROJECT_STATUS.md)
- [Architecture map](docs/architecture/ARCHITECTURE_MAP.md)
- [CI](docs/current/CI_ARCHITECTURE.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

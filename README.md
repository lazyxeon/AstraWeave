<h1 align="center">AstraWeave</h1>

<p align="center">A deterministic game engine. The game is Aravath.</p>

<p align="center">
  <a href="https://github.com/lazyxeon/AstraWeave/actions/workflows/ci.yml"><img alt="CI status" src="https://github.com/lazyxeon/AstraWeave/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="rust-toolchain.toml"><img alt="Rust 1.89.0" src="https://img.shields.io/badge/rust-1.89.0-orange"></a>
</p>

AstraWeave is an AI-native game engine with a deterministic ECS. Companions are part of the simulation. Local LLMs propose. The deterministic simulation validates and executes. Assets in the tree are CC0.

<p align="center">
  <!-- Replace this hero with a greybox demo capture when one is ready. -->
  <img alt="AstraWeave nebula logomark" src="assets/Astraweave_logo.jpg" width="480">
</p>

## Features

| | |
| --- | --- |
| Replay | One thread per tick. The required cases hash on Linux, Windows, and macOS. [Workflow](.github/workflows/determinism-replay.yml) |
| Companions | The default tick runs `RuleOrchestrator`. A local model proposes off that tick. The simulation validates and executes. [AI pipeline](docs/architecture/ai_pipeline.md) |
| ECS | `astraweave-ecs` is the schedule. [Foundation](docs/architecture/ecs_math_core_sdk_foundation.md) |
| Renderer | The engine and the editor share one wgpu path. [Render](docs/architecture/render_pipeline_material_system_shader_infrastructure.md) |
| World | Physics and the local field are on the sim tick. The loaded set is `ColumnStream`. The hashed ground is the island heightfield. `weave_tick` is the weave example and tests. The `weaving` case hashes `NullField`. [Physics](docs/architecture/physics.md) · [Terrain](docs/architecture/terrain.md) · [Weaving](docs/architecture/weaving.md) |
| Aravath | A windowed greybox with CC0 props. [Demo](docs/current/ARAVATH_DEMO.md) |

## Quick start

```bash
cargo check -p astraweave-core
cargo test -p aravath_runtime --lib replay_matches_golden_hash
cargo run -p hello_companion --release
cargo xtask fetch-assets
cargo editor
```

`cargo editor` is the alias in `.cargo/config.toml`. The windowed demo is `cargo run -p aravath_runtime --bin aravath_greybox --release`, documented on the demo page.

## Status

| | |
| --- | --- |
| Toolchain | Rust 1.89.0, pinned in `rust-toolchain.toml` |
| Replay | Nineteen required cases. The list and the golden are in [engine status](docs/current/ENGINE_STATUS.md). |
| Frame | The target and the measured sample are in [engine status](docs/current/ENGINE_STATUS.md). |
| Snapshot | The previous root page, citations included, is the [project brief](docs/current/PROJECT_BRIEF.md). |

## In progress

These pulls are not on main.

- **#334** — delete `AIArbiter` once benches have a deterministic replacement
- **#366** — step 2: `egui_kittest` waits on upstream AccessKit and zbus 6
- **#383** — port advanced GOAP and `AIArbiter` into the deterministic HTN tick

## Docs

- [Team roadmap](docs/ROADMAP.md) (Vote 6 source of truth; team's internal sitrep page, sign-in required, at [astraweave-sitrep-and-roadmap](https://astraweave-sitrep-and-roadmap-7wkx.pages.grok.bot/))
- [Engine status](docs/current/ENGINE_STATUS.md)
- [Architecture map](docs/architecture/ARCHITECTURE_MAP.md)
- [Contributing](CONTRIBUTING.md)
- [Agents](AGENTS.md)
- [Security](SECURITY.md)

## TL;DR

- **What it is.** A Rust game engine. The player-facing game is Aravath. Assets in the tree are CC0.
- **How a tick works.** One thread. Perception, simulation, planning, physics, then presentation. Replay hashes have to match across operating systems.
- **How AI fits.** The default tick runs `RuleOrchestrator`. A local model proposes off the tick. The simulation validates and executes.
- **What to run.** The five commands above. `hello_companion` is the headless AI demo. The greybox is the window.
- **Where the numbers live.** Measurements, commit SHAs, and the replay list are in [engine status](docs/current/ENGINE_STATUS.md), not on this page.

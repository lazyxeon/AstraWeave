<p align="center">
  <img src="assets/Astraweave_logo.jpg" alt="AstraWeave nebula logomark" width="360" />
</p>

<h1 align="center">AstraWeave — AI-Native Game Engine</h1>

<div align="center">

[![Kani Formal Verification](https://github.com/lazyxeon/AstraWeave/actions/workflows/kani.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/kani.yml) [![OpenSSF Scorecard](https://github.com/lazyxeon/AstraWeave/actions/workflows/scorecard.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/scorecard.yml) [![Miri UB Detection](https://github.com/lazyxeon/AstraWeave/actions/workflows/miri.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/miri.yml) [![Security Audit](https://github.com/lazyxeon/AstraWeave/actions/workflows/security-audit.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/security-audit.yml) [![Mutation Testing](https://github.com/lazyxeon/AstraWeave/actions/workflows/mutation-testing.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/mutation-testing.yml) [![Sanitizers](https://github.com/lazyxeon/AstraWeave/actions/workflows/sanitizers.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/sanitizers.yml) [![CodeQL Analysis](https://github.com/lazyxeon/AstraWeave/actions/workflows/codeql-analysis.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/codeql-analysis.yml) [![Clippy Lint (Unwrap Prevention)](https://github.com/lazyxeon/AstraWeave/actions/workflows/clippy-unwrap-prevention.yml/badge.svg)](https://github.com/lazyxeon/AstraWeave/actions/workflows/clippy-unwrap-prevention.yml)

</div>

<p align="center">
  <a href="https://github.com/lazyxeon/AstraWeave/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lazyxeon/AstraWeave?style=for-the-badge" alt="License" /></a>
  <a href="https://github.com/lazyxeon/AstraWeave/blob/main/rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.89.0-orange.svg?style=for-the-badge" alt="Rust toolchain" /></a>
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue.svg?style=for-the-badge" alt="Platforms" />
</p>

<div align="center">

**An AI-native game engine with a deterministic ECS. AI agents are first-class citizens.**
The game this engine is being built for is **Aravath**.

📚 [Documentation](docs/) • 📊 [Benchmarks](docs/masters/MASTER_BENCHMARK_REPORT.md) • 🗺️ [Roadmap](docs/masters/MASTER_ROADMAP.md) • 🧪 [Coverage](docs/current/MASTER_COVERAGE_REPORT.md) • 🕸️ [Workspace Map](https://lazyxeon.github.io/AstraWeave/architecture/) • ⚡ [Benchmark dashboard](https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/) • 🌐 [GitHub Pages](https://lazyxeon.github.io/AstraWeave/)

</div>

---

## Current state (2026-10-16)

Checked against `main` at `d6903d3a`. Numbers that were not re-measured in this pass are dated below. This page does not assign a letter grade.

| Topic | What is true on this commit |
| --- | --- |
| **Workspace** | **141** members via `cargo metadata --no-deps`. **58** of those names start with `astraweave-` (`astraweave-veil` is the new scar-field crate). **60** manifests live under `examples/`. The count at `5cba174c` was 140. |
| **Toolchain** | Rust **1.89.0**, pinned in `rust-toolchain.toml`. Targets: Linux x86_64, Windows MSVC, macOS x86_64 and Apple Silicon. |
| **CI platforms** | Pull-request replay hashes run on `ubuntu-latest`, `windows-latest`, and `macos-latest`. That is the supported cross-OS determinism matrix. It is not a claim that every example window has been run on all three. |
| **Determinism** | Fifteen required replay cases, including `traversal`, `traversal_deformed`, `worldgen_columns`, and `deformed_chunk`. The compare job fails if any case is missing or its SHA-256 differs across the three OS artifacts. See [Determinism](#determinism). |
| **AI rule** | Local LLMs propose. The deterministic simulation validates and executes. The orchestrator default, when `OLLAMA_MODEL` is unset, is `phi3:medium` (`astraweave-ai`). The Qwen client constant is `qwen3.5:4b`. Ollama constructors keep a loopback URL. A non-loopback URL is replaced with `http://127.0.0.1:11434` unless the caller uses `allowing_remote` (#316). |
| **Assets** | The project rule is **CC0 only**. `assets/README.md` still says "Mixed" and this pass did not re-audit every file's license. |
| **Frame time** | The target is under 11 ms per frame on a GTX 1660 Ti-class GPU. It is not yet measured on that hardware: the GPU timer read 0 ms, and the readback is in progress in #216. Do not treat the older CPU `profiling_demo` times as this measurement. |
| **Coverage and Miri** | Whole-workspace line coverage **57.35%** was measured 2026-06-29 and has not been re-run on the 141-member tree. Miri **1,059** tests, 0 UB, was measured 2026-06-27 across ecs, math, core, and sdk. The February 2026 report records an older, smaller run. Kani proof attributes were **69** on 2026-06-13 and were not re-counted here. |
| **Build** | This pass did not run `cargo check --workspace`. The last clean workspace check written into these docs is 2026-06-10, at 130 members. |
| **Supply chain** | #325 (`8e5dd73c`) makes `cargo deny` advisories and `cargo audit` pass. Patched crates are locked (`rustls` 0.23.46, `tokio-rustls` 0.26.4, `openh264` 0.9.9). Ignored leftovers are #305–#311 and #320–#324. #217 removed `openssl` from the lockfile. `deny.toml` still bans `openssl` and `openssl-sys`. #219 and #256 are closed. #327 (`87fbe079`) drops `net` from the workspace `tokio` features. `astraweave-net`, `astraweave-ipc`, and `astraweave-net-ecs` opt in. #326 is closed. |
| **Traces** | **31** subsystem traces under `docs/architecture/`, including `veil.md`, listed in the generated table in `CLAUDE.md`. The June "13 traces" figure is the original campaign, not the current set. |

### On main since the June snapshot

- **AI orchestration and dynamic nav reroute (#251).** `NavTiles` can take a geometry change and recompute a route. Replay case `nav_reroute`.
- **Bridge reroute and `aravath_runtime` (#263).** Headless `BridgeAgent` walks a bridge-and-ford graph, detaches the deck at tick 30, and leaves the span for the ford. The greybox window does not draw that body. Player-facing scene label is `Aravath`.
- **Security caps (#312, #316).** #312 bounds model text, imports, admin auth, and socket frames, on top of the Rhai sandbox ceilings (operation budget, string/array/map sizes, mod counts). `default_allowed_llm_domains` is localhost only. #316 caps image decode at a 4096-pixel edge and 64 MiB (`open_limited_image` / `decode_limited_image`). Render, the editor, blend import, and `aw_asset_cli` call that decoder. `astraweave-asset-pipeline` doc comments still show an uncapped open; that crate is the dormant cook path, not a live caller. `aw_asset_cli` rejects glTF buffer and image file URIs that fail `safe_under` before import. `OllamaClient::new` keeps only loopback URLs unless `allowing_remote` is used.
- **AI decision debugger (#287).** Zero-cost decision ring plus the headless `aw_decision` inspector (`show` / `diff`). Tracing is off unless the caller records.
- **Causal consequence engine (#286).** `CausalWorld::tick` admits recorded inputs and fires a baked rule table. It does not call a model. Replay case `causal_bridge`. Editor play does not call it. The causality crate does not read a geometry notice; `BridgeAgent` drains the collapse into the cut.
- **Rhai mod sandbox and loader (#214, #318).** Gameplay and mod scripts run on the security sandbox. `build_app` installs `ModScriptPlugin` after `register_mod_runtime`. Replay case `scripted_system`. Gameplay `script_system` does not inject `PhysicsProxy` or `NavMeshProxy` (#318). The host still uses `PhysicsWorld` for collision events and spawned bodies. `scripting_advanced_demo` still inserts physics and nav resources, and `advanced_ai.rhai` no longer calls them: sight is distance under 20, and a finished patrol point clears `path`. `scripting_playground` still runs triggers and `apply_damage`, and the player script no longer raycasts.
- **Destruction (#243).** Command log, support graph, fracture, terrain deltas, and debris. Sim state is `Fixed64`. Display meshes are outside the hash. Replay case `destruction`.
- **Gameplay QA (#277, replay #280).** Headless Aravath-slice scenario hashed as `gameplay_qa_aravath_slice`. The #277 document is the plan; the required replay case is what landed.
- **Weaving framework (#252).** `weave_tick` stamps data-driven templates (heat, freeze, density, gravity, and the other built-in ids) into `FieldWrite`s on a `SimField`. Replay case `weaving` runs 300 ticks on `NullField`, whose field digest is zeros until the local sim in #253 is on main. Freeze records an ice sheet and a nav change. `examples/aravath_weave` is the headless caller. `astraweave-core`, `astraweave-gameplay`, `astraweave-scripting`, and `astraweave-security` depend on the crate. The anchor-slice helpers are still plain functions.
- **Aravath greybox (#205).** `cargo run -p aravath_runtime --bin aravath_greybox` opens the window. The on-screen player is the CC0 KayKit knight with the KayKit sword and shield. The three foes are CC0 KayKit skeletons. Ground albedos are CC0 Poly Haven maps. Foliage is CC0 KayKit and Kenney. Credits are in `assets/aravath/CREDITS.md`. The world streams a three-chunk ring. Sword hits land once, inside the active window, and lock-on drops when that enemy dies. Dig and place still use `dig_sphere` and the voxel edit log. The greybox replay hash is `b663da52ea1d566e` in `aravath_runtime/golden/replay.hash`. That check runs on Linux and Windows only. It is a separate hash from the fifteen three-OS cases. Since #215, `ColumnStream` is the keep and evict set for the radius-3 ring. Those chunks still fill from the island heightfield. A connecting sword hit calls `run_aravath_slice` (#275). Dealt damage is `40 - sword_hp`. The slice test locks that remainder at 15, so the hit deals 25, and a broken cover bond shows the notice `Cover broke` once. The 11 ms frame budget is still not measured on a GTX 1660 Ti.
- **Traversal (#244).** `astraweave-physics` steps movement at a fixed 60 Hz. The sim owns displacement. Ground, jump, and roll are hashed for 180 ticks as required case `traversal`. `traversal_deformed` walks and climbs terrain that a brush cuts mid-run: a one-voxel seam ramps, a destroyed handhold detaches, and horizontal momentum is kept. Climb is hybrid (markup wins, otherwise a flagged surface). The Aravath motor (ground, air, crouch, sprint, roll) steps once per tick. Roll invulnerability is ticks 5 through 19. Swim, slide, prone, wall-run, glide, ledge, and shimmy are in the course script. Foot and hand IK stay on the presentation graph and are outside the hash.
- **CI burndown (#217).** `aw-editor-tests.yml` and the sanitizer jobs install `pkg-config` and `libudev-dev`. Audio nextest fixtures live under `astraweave-audio/target/audio-test-assets/<pid>`. `fluid.wgsl` uses `safe_normalize` and takes velocity from the pre-clamp prediction so a zero vector and a wall hit stay finite on lavapipe. reqwest, the websocket client, and `hf-hub` use rustls. `ort` does not enable `download-binaries`. The fifteen replay cases are unchanged. The disposition of the older red checks is `docs/current/CI_FAILURE_LEDGER.md`.
- **Greybox tokio graph (#327).** The workspace `tokio` features are `fs`, `io-util`, `macros`, `rt`, `rt-multi-thread`, `sync`, and `time`. `astraweave-net`, `astraweave-ipc`, and `astraweave-net-ecs` add `net`. `astraweave-blend` adds `process`. `aw_asset_cli` does not depend on tokio. #326 is closed.
- **Greybox combat slice (#275).** `Combat::tick` calls `astraweave_weaving::run_aravath_slice` on a connecting sword hit. The foe loses `40 - sword_hp`. `cover_intact == false` sets `cover_broken`, and the window shows `Cover broke` for 2.5 seconds, once. The greybox golden stays `b663da52ea1d566e`.
- **Terrain columns and scatter (#215).** `column_gen` is the integer forest record. Height, material, and a prop are functions of `(FOREST_WORLD_SEED, x, z)` with wrapping integer arithmetic. Species and yaw come from `SimRng` on `SIM_SYSTEM_FOREST_SCATTER`. The road stays empty. `worldgen_columns` is the SHA-256 of `worldgen_record()`: three chunks, including a negative coordinate and a road chunk. `deformed_chunk` is the SHA-256 of a sphere brush and one scatter detach on chunk `(-1, 2)`, then the occupancy after that chunk is evicted and loaded again. A `Detach` stays down across that reload. `ColumnStream` is the greybox keep set. The window still fills those chunks from the island heightfield, and its props still come from `aravath_runtime`'s local scatter. The editor climate mesh is the preview. Scene save and the Aravath checkpoint store `worldgen_record()`. Apply Erosion appends destruction commands and leaves that record in place. The CC0 pine-forest files, including Poly Haven `pine_tree_01`, are hashed in `assets/terrain/polyhaven/pine_forest/manifest.json`. KayKit and Kenney stay the greybox props.

## In progress

These pulls are not on `main`.

- **#253** — local fluid sim
- **#216** — render frame budget (GPU timer readback)

---

## Determinism

Determinism is non-negotiable. `.github/workflows/determinism-replay.yml` runs on every pull request and on push to `main`, with no path filter. Each OS pins a baseline CPU (`target-cpu=x86-64` on x86_64; Apple Silicon stays on the default aarch64 baseline). `cargo test` in this workflow is not retried.

The compare job requires these fifteen cases to be present and byte-identical across Linux, Windows, and macOS:

`planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, `nav_reroute`, `causal_bridge`, `scripted_system`, `weaving`, `traversal`, `traversal_deformed`, `worldgen_columns`, `deformed_chunk`.

`worldgen_columns` and `deformed_chunk` are written into `target/replay-hash.json` by `cargo test -p astraweave-terrain --test worldgen_hash` before the merge step. They do not have a separate artifact file.

The ECS schedule itself is single-threaded per tick, in this order: `PRE_SIMULATION`, `PERCEPTION`, `SIMULATION`, `SYNC`, `AI_PLANNING`, `PHYSICS`, `POST_SIMULATION`, `PRESENTATION`. `ParallelSchedule` was removed on 2026-04-18 ([audit](docs/audits/parallel_schedule_removal_2026-04-18.md)). Rayon, tokio, and GPU compute live in subsystems, not inside the tick schedule.

---

## Quick start

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave

# Toolchain is pinned. Confirm it before a long build.
rustup show

# Core library
cargo build --release -p astraweave-core

# Flagship companion demo. Seven modes; several need extra features.
cargo run -p hello_companion --release

# Visual editor. Fetch packs first. Alias runs aw_editor with the release-fast profile.
cargo xtask fetch-assets
cargo editor
```

Editor aliases in `.cargo/config.toml`: `editor`, `editor-release`, `editor-dev`. Use `cargo check -p <crate>` or `cargo check --workspace`.

Setup notes: [build quick reference](docs/guides/BUILD_QUICK_REFERENCE.md), [toolchain](docs/guides/RUST_TOOLCHAIN_GUIDE.md), [contributing](CONTRIBUTING.md). The file `docs/QUICKSTART.md` is the feature-gated advanced GOAP tutorial, not the engine install guide.

---

## How the loop works

```text
Perception → local model proposes → plan → deterministic validation → simulation
```

1. **Perception.** Agents read a `WorldSnapshot`.
2. **Proposal.** A local LLM, a utility score, GOAP, or a behavior tree proposes a plan.
3. **Validation.** The tool sandbox and the sim accept or reject the plan.
4. **Action.** Accepted steps run as deterministic ECS commands.

`hello_companion` exposes seven modes: Classical, Behavior Tree, Utility, LLM, Hybrid, Ensemble, and Arbiter. Behavior Tree, Utility, and Ensemble need `--features llm`. LLM and Hybrid need `--features ollama`. Arbiter needs `--features llm_orchestrator`. A default `cargo run -p hello_companion` does not enable all seven.

The 12,700-agent figure is a CPU capacity estimate for a mid-complexity AI loop on an i5-10300H with the platform allocator, measured 2026-06-27. It is not a full-frame GPU result. See [CLAIMS_REGISTRY.md](docs/campaigns/doc-truth/CLAIMS_REGISTRY.md#agents-capacity-60fps) and the [benchmark hardware note](docs/masters/MASTER_BENCHMARK_REPORT.md#benchmark-hardware).

---

## Systems on this tree

| Area | State on `d6903d3a` |
| --- | --- |
| **ECS, math, core, sdk** | Single-threaded archetype schedule. Miri-clean as of the 2026-06-27 run (1,059 tests). |
| **Combat** | `astraweave-combat` resolves damage in Q32.32. Replay case `combat_resolve`. |
| **Navigation** | Navmesh plus tile routes. `nav_reroute` is a required replay case. |
| **Destruction** | `astraweave-destruction`, `Fixed64` sim, required replay case. |
| **Causality** | `astraweave-causality`, baked rules, required replay case. Headless Aravath caller only. |
| **Scripting** | Sandboxed Rhai, mod loader, tick-boundary reload. Required replay case `scripted_system`. Gameplay scripts do not receive physics or nav proxies (#318). See [scripting.md](docs/architecture/scripting.md). |
| **Security** | Path checks, the deterministic Rhai sandbox, the #312 bounds, and the #316 image, glTF, and Ollama caps are wired. Anticheat, signatures, and the secrets crate stay dormant. See [security.md](docs/architecture/security.md). |
| **Decision inspector** | `tools/aw_decision`. |
| **Aravath greybox** | Windowed demo `aravath_greybox` (#205). CC0 KayKit characters, streamed three-chunk world, sword combat. A connecting hit calls `run_aravath_slice` (#275) and can show `Cover broke`. `BridgeAgent` is in the same crate and is not drawn. Greybox replay is Linux and Windows only. |
| **Rendering** | wgpu path: Disney BRDF with multi-scatter compensation, clustered forward lighting, IBL cubemaps, ACES / AgX / Reinhard in one post pipeline, 4-cascade CSM. Editor and engine share that path; a SHA-256 parity harness covers the May 2026 seams ([architecture map](docs/architecture/ARCHITECTURE_MAP.md)). Nanite-style and VXGI surfaces exist in-tree; this page does not claim they hold the frame budget. |
| **Camera** | `astraweave-camera` is the upload contract (`RenderView`). |
| **Physics** | Rapier3D wrapper. The in-crate `SpatialHash` is not the broadphase. Traversal (#244) is a 60 Hz tick sim in this crate. Required cases `traversal` and `traversal_deformed`. |
| **Terrain** | Integer columns in `column_gen` are the saved ground (#215). Required cases `worldgen_columns` and `deformed_chunk`. The editor climate heightmap is the preview mesh. The greybox collider still comes from the island voxel fill. The June 2026 asset-quality note still stands: the BC7/KTX2 cook path was non-functional and runtime uploads were uncompressed RGBA8. Not re-tested here. |
| **Weaving** | Data-driven casts on main (#252). Required replay case `weaving` hashes `NullField` until local sim #253. See [weaving.md](docs/architecture/weaving.md). |
| **Water** | View-side `astraweave-water` facade plus `astraweave-render` surface. `astraweave-fluids` is the deprecated accent remnant, not a game-loop solver. A local fluid sim is in progress in #253 and is not this water path. |
| **Networking** | Snapshot net (`astraweave-net`, JSON over WebSocket) and a separate ECS/matchmaking stack. Matchmaking input frames use HMAC-SHA256, verify-first, kick by default (June 2026). No replay protection. Server-to-client messages are unsigned. See [net_ecs.md](docs/architecture/net_ecs.md). |
| **Editor** | `aw_editor`, reached with `cargo editor`. June 2026 multi-tool sub-phase 5 was still open in the campaign doc; this pass did not close it. |
| **Memory, RAG, coordination, advanced GOAP** | Present and tested. Not on the default sim tick. `VectorStore::search` is a linear scan. |

---

## Build, test, and docs gates

```bash
cargo check -p <crate>
cargo test -p <crate>
cargo fmt --all
cargo clippy -p <crate> --all-features -- -D warnings
cargo run -p aw_doc_lint -- --mode warn
cargo run -p aw_trace_sync -- --check
```

Pull-request Rust work is tiered. The required replay check is `determinism-replay / compare`. Docs-only diffs skip the workspace compile; they still run the replay hashes. The full job list is [CI_ARCHITECTURE.md](docs/current/CI_ARCHITECTURE.md). #217 and #327 are on `main`. #326 is closed.

`aw_doc_lint` reads the closed vocabulary in [CLOSED_VOCABULARY_LINT.md](docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md). It also checks this README against the replay workflow: every required case name, the unmeasured 11 ms target, the in-progress issue list, and the absence of the retired game title in player-facing docs. That front-door check fails the process even while the vocabulary scan is still in warn mode.

---

## Older write-ups

These are dated records. Where they disagree with the table at the top, the table wins.

- [Architecture map](docs/architecture/ARCHITECTURE_MAP.md) — crate graph and seams. Header counts were reconciled 2026-10-16; the long body still contains June campaign notes.
- [Miri report](docs/current/MIRI_VALIDATION_REPORT.md) — February 2026 run. The later count is 1,059.
- [Coverage report](docs/current/MASTER_COVERAGE_REPORT.md) — 57.35% headline is 2026-06-29. Per-crate tiers below that headline are older.
- [Benchmark report](docs/masters/MASTER_BENCHMARK_REPORT.md) — CPU and historical GPU probes. Not a passing full-frame budget.
- [Roadmap](docs/masters/MASTER_ROADMAP.md) — strategy log. The June 2026 "current state" table is a snapshot.
- [Editor behavioral audit](docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md) and [unified pipeline campaign](docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md) — April 2026 correctness work. They are not a current health grade.
- [Project status](docs/current/PROJECT_STATUS.md) — session log. A 2026-10-16 banner sits above the June narrative.

---

<div align="center">

If this experiment interests you, star the repo.

</div>

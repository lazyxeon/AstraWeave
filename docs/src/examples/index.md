# Working examples

Start here, from a clone of the repository, with Rust 1.89.0 (the pin in `rust-toolchain.toml`).

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
cargo run -p hello_companion --release
```

That is the headless companion. The next three commands are the rest of the [first game](../game-dev/first-game.md) path: quest text, a physics swing, and the window.

```bash
cargo run -p quest_dialogue_demo --release
cargo run -p combat_physics_demo --release
cargo xtask fetch-assets
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`quest_dialogue_demo` reads `assets/dialogue_intro.toml` and `assets/quests_main.toml` from the current directory, so run it at the repo root.

There is no `unified_showcase` package. The rendering walkthrough is [Rendering showcase](./unified-showcase.md) (`shadow_csm_demo` and `aravath_greybox`).

## Walkthroughs

| Guide | Package | Command |
| --- | --- | --- |
| [Hello Companion](./hello-companion.md) | `hello_companion` | `cargo run -p hello_companion --release` |
| [Your first companion](../getting-started/first-companion.md) | same binary, edited in place | same command after you edit `setup_world` |
| [Adaptive boss](./adaptive-boss.md) | `adaptive_boss` | `cargo run -p adaptive_boss --release` |
| [Physics demo](./physics-demo.md) | `physics_demo3d` | `cargo run -p physics_demo3d --release` |
| [Navmesh demo](./navmesh-demo.md) | `navmesh_demo` | `cargo run -p navmesh_demo --release` |
| [Audio](./audio-spatial.md) | `audio_spatial_demo` | `cargo run -p audio_spatial_demo --release` |
| [Fluids demo](./fluids-demo.md) | `fluids_demo` | `cargo run -p fluids_demo --release` |
| [Rendering showcase](./unified-showcase.md) | `shadow_csm_demo` | `cargo run -p shadow_csm_demo --release` |
| [First game](../game-dev/first-game.md) | the four commands above | see that page |

`fluids_demo` uses `astraweave_fluids::FluidSystem`. The crate has no `PCISPHSystem`. `navmesh_demo` calls `NavMesh::find_path`. The replayed path query is `NavTiles::query_path`. `find_path` is not the hashed route.

`adaptive_boss --features veilweaver_slice` switches `BossDirector` to `OathboundWardenDirector` (`examples/adaptive_boss/src/main.rs`). The feature name is an identifier. The player-facing game is Aravath.

## AI flags on hello_companion

The default run is `RuleOrchestrator`. Other flags are on the [Hello Companion](./hello-companion.md) page. `--arbiter` calls `GoapOrchestrator`, not `AIArbiter`. See [AI Arbiter](../core-systems/ai/arbiter.md).

## More packages

These names are also under `examples/` in the workspace `members` list. `cargo run -p <name> --release` is the command when the crate has a binary. This index does not claim a fresh run for every one of them:

`companion_profile`, `llm_toolcall`, `phase_director`, `persona_loader`, `ollama_probe`, `phi3_demo`, `ecs_ai_demo`, `ecs_ai_showcase`, `core_loop_bt_demo`, `core_loop_goap_demo`, `ipc_loopback`, `coop_server`, `coop_client`, `terrain_demo`, `visual_3d`, `ui_controls_demo`, `debug_overlay`, `rhai_authoring`, `npc_town_demo`, `weaving_playground`, `cutscene_render_demo`.

`core_loop_goap_demo` is not the deterministic tick. `sys_ai_planning` boxes `RuleOrchestrator` (`astraweave-ai/src/ecs_ai_plugin.rs`). The window from the commands above is `aravath_greybox`. Its tick calls `BridgeAgent::step` (`aravath_runtime/src/sim.rs`). `BridgeAgent::refresh` calls `NavTiles::query_path` (`aravath_runtime/src/bridge_agent.rs`).

Snapshot net is JSON over WebSocket (`tokio-tungstenite`). Read the networking row on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page before treating `coop_server` as the security model.

## Add an example

Copy `examples/hello_companion` and add the new directory to the workspace `members` list in the root `Cargo.toml`. `astraweave_core::World` has `spawn(name, pos, team, hp, ammo)`. `astraweave_ecs::App` has `add_system` and `run_fixed`. It does not have `world.add_system` followed by `world.run()`.

## Related

- [Troubleshooting](./troubleshooting.md)
- [Building](../dev/building.md)
- [Benchmarks](../performance/benchmarks.md)

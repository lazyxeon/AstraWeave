# Working Examples

Example binaries are workspace members under `examples/`. Run one with `cargo run -p <package> --release`.

This page lists packages that exist in the root `Cargo.toml`. It does not certify a fresh run on every platform. Packages that older pages marked "expected panic" or "does not compile" are not given that status here.

There is no `unified_showcase` package. See [Unified Showcase](./unified-showcase.md).

## AI

### hello_companion

```bash
cargo run -p hello_companion --release
```

Headless showcase. The default run is `RuleOrchestrator`. It does not panic with `LosBlocked`. Mode flags and features are on the [walkthrough](./hello-companion.md).

### adaptive_boss

```bash
cargo run -p adaptive_boss --release
```

`examples/adaptive_boss` builds a small arena and steps `BossDirector`. The `veilweaver_slice` feature switches that import to `OathboundWardenDirector`. The feature is an identifier. The player-facing game name is Aravath.

```bash
cargo run -p adaptive_boss --release --features veilweaver_slice
```

[Walkthrough](./adaptive-boss.md)

### Other AI packages

These package names are in the workspace. Read the crate before treating a README inside the example as current:

- `companion_profile`
- `llm_toolcall`
- `phase_director`
- `persona_loader`
- `ollama_probe`
- `phi3_demo`
- `ecs_ai_demo`
- `ecs_ai_showcase`
- `core_loop_bt_demo`
- `core_loop_goap_demo`

`core_loop_goap_demo` is not `AIArbiter`, and it is not the deterministic tick. The tick's planner is `RuleOrchestrator`.

## Simulation and rendering

| Package | Command |
| --- | --- |
| `physics_demo3d` | `cargo run -p physics_demo3d --release` |
| `navmesh_demo` | `cargo run -p navmesh_demo --release` |
| `audio_spatial_demo` | `cargo run -p audio_spatial_demo --release` |
| `fluids_demo` | `cargo run -p fluids_demo --release` |
| `terrain_demo` | `cargo run -p terrain_demo --release` |
| `shadow_csm_demo` | `cargo run -p shadow_csm_demo --release` |
| `aravath_runtime` | `cargo run -p aravath_runtime --bin aravath_greybox --release` |

`fluids_demo` uses `astraweave-fluids`. That crate is not a game-loop solver, and this tree does not name a PCISPH solver. `navmesh_demo` is a navmesh example. The replayed path query is `NavTiles::query_path`, not `NavMesh::find_path`.

Walkthroughs: [physics](./physics-demo.md), [navmesh](./navmesh-demo.md), [audio](./audio-spatial.md), [fluids](./fluids-demo.md).

## Networking packages

`ipc_loopback`, `coop_server`, and `coop_client` are workspace members:

```bash
cargo run -p ipc_loopback --release
cargo run -p coop_server --release
cargo run -p coop_client --release
```

Snapshot net in the engine is JSON over WebSocket (`tokio-tungstenite`). See the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) networking row before treating an example as the security model.

## Check the toolchain

```bash
rustc --version
cargo test -p astraweave-input
```

Inside this repository, `rustc --version` should be 1.89.0.

## Adding an example

Copy an existing example directory and add its path to the workspace `members` list. The snippets that used to show `world.add_system` and `world.run` are not the `astraweave-ecs` API. `App`, `Schedule`, and `World` are the types in `astraweave-ecs`.

## Related

- [Hello Companion](./hello-companion.md)
- [Troubleshooting](./troubleshooting.md)
- [Building](../dev/building.md)

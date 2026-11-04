# Building Your First Game

There is no `companion_quest` package. This guide runs four workspace binaries, in order, from the repository root. Together they are a companion, a quest, a physics hit, and a window. Each command is a package in the root `Cargo.toml` `members` list.

Rust is 1.89.0, pinned by `rust-toolchain.toml`.

## 1. Companion plan

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
cargo run -p hello_companion --release
```

Package `hello_companion`. Default features do not call a model. `RuleOrchestrator::propose_plan` (`astraweave-ai/src/orchestrator.rs`) returns smoke, a move, and cover fire for the world in `examples/hello_companion/src/main.rs`. The smoke throw hits `EngineError::LosBlocked` (`line of sight blocked`). The process prints that and exits 0. The walkthrough is [Your first companion](../getting-started/first-companion.md).

The spawn you will see again in other examples is:

```rust
w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
```

That is `astraweave_core::World::spawn` in `astraweave-core/src/world.rs`. It is not a Bevy tuple spawn.

## 2. Dialogue and a quest

```bash
cargo run -p quest_dialogue_demo --release
```

Run that from the repository root. `examples/quest_dialogue_demo/src/main.rs` reads two files with paths relative to the working directory:

- `assets/dialogue_intro.toml`
- `assets/quests_main.toml`

It parses `astraweave_gameplay::dialogue::Dialogue`, prints each line, and auto-picks choice `0` until `node.end`. It then parses `astraweave_gameplay::quests::Quest`, calls `QuestLog::add`, `progress_gather("q_tutorial", "Crystal", 2)` and `progress_gather(..., 1)`, and prints `Quest completed?` from `log.is_done("q_tutorial")`.

If you run it from another directory, those `read_to_string` calls fail because the paths are not absolute.

## 3. A physics hit

```bash
cargo run -p combat_physics_demo --release
```

`examples/combat_physics_demo/src/main.rs` builds a Rapier world and one swing:

```rust
let mut phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
let _ground = phys.create_ground_plane(vec3(50.0, 0.0, 50.0), 1.0);
```

`PhysicsWorld::new` takes a `Vec3` gravity (`astraweave-physics/src/lib.rs`). The workspace dependency is `rapier3d` 0.22. The demo then `add_dynamic_box` for a player and an enemy and uses `astraweave_gameplay` combat types (`Combatant`, `Stats`, `Parry`). Read the rest of that `main` for the swing. Do not expect the old sample log that named Rapier 0.17 or Hermes. This binary does not connect to Ollama.

## 4. Open the window

The windowed slice is `aravath_greybox`, a binary of package `aravath_runtime`:

```bash
cargo xtask fetch-assets
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`cargo xtask fetch-assets` is the asset fetch in `tools/xtask`. The greybox behavior (KayKit props, streamed chunks, sword hit, dig brush) is the Aravath greybox row on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page. `cargo editor` is the alias in `.cargo/config.toml` for `aw_editor`. It is a separate window from the greybox.

The default `hello_companion` run stays headless. Do not use it to test the GPU.

## 5. A boss in the same world type

```bash
cargo run -p adaptive_boss --release
```

`examples/adaptive_boss/src/main.rs` spawns player, companion, and boss with the same `World::spawn` signature, builds a `WorldSnapshot`, and steps `BossDirector` from `astraweave-director`. `--features veilweaver_slice` switches the import to `OathboundWardenDirector`. That feature name is an identifier. The player-facing game name is Aravath.

## What a single game loop is

These binaries are separate `main`s. They show the types you compose:

| Piece | Crate | Seen in |
| --- | --- | --- |
| Grid world and snapshot | `astraweave-core` | `hello_companion`, `adaptive_boss` |
| Plan | `astraweave-ai::RuleOrchestrator` | `hello_companion` |
| Accept or reject the plan | `validate_and_execute` | `hello_companion` |
| Lines and quests | `astraweave-gameplay` | `quest_dialogue_demo` |
| Rapier bodies | `astraweave-physics::PhysicsWorld` | `combat_physics_demo` |
| Window | `aravath_runtime` bin `aravath_greybox` | engine status |

The deterministic tick does not call `AIArbiter`. A local model, when you enable one, proposes off that tick. The sim validates.

## Related

- [Hello Companion](../examples/hello-companion.md)
- [Crafting and combat](./crafting-combat.md)
- [Dialogue](./dialogue.md)
- [Save and load](./save-load.md)
- [Examples](../examples/index.md)

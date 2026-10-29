# Your First AI Companion

You will clone the repository, run `hello_companion`, read the classical plan it builds, and change that world.

The binary is the workspace package `hello_companion` (`examples/hello_companion`). You do not create a new crate. A new crate would have to be added to the root `Cargo.toml` `members` list. This tutorial stays on the package that is already a member.

## 1. Toolchain

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustup show
rustc --version
```

`rust-toolchain.toml` pins channel `1.89.0`. Inside this directory, `rustc --version` should report 1.89.0.

## 2. Run the companion

```bash
cargo run -p hello_companion --release
```

That command uses the default features in `examples/hello_companion/Cargo.toml` (`fast-alloc` only). No GPU and no Ollama process are required. The program is headless.

You should see the banner from `main`, then `Using Classical AI (RuleOrchestrator).`, then `Generated 3 steps`, `Plan plan-0 with 3 steps`, and:

```text
⚠️  Execution failed: line of sight blocked. Continuing...
```

That string is `EngineError::LosBlocked` (`astraweave-core/src/schema.rs` displays it as `line of sight blocked`). The example prints it from `Execution failed: {}. Continuing...` and returns `Ok(())`. It does not panic. A debug run on this tree left the companion at `(2, 3)`, the enemy at `(12, 2)` with HP 60, and printed `t=5.00` after twenty `run_fixed(1)` calls.

## 3. The calls that produced that output

`setup_world` uses `astraweave_core::World`:

```rust
let mut w = World::new();
let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
let comp = w.spawn("Companion", IVec2 { x: 2, y: 3 }, Team { id: 1 }, 80, 30);
let enemy = w.spawn("Rival", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);
```

`spawn` is declared in `astraweave-core/src/world.rs` as `spawn(&mut self, name: &str, pos: IVec2, team: Team, hp: i32, ammo: i32) -> Entity`.

A column of obstacles is inserted at `x = 6`, `y = 1..=8`. The companion's `throw:smoke` cooldown is set to `0.0`. `build_snapshot` (`astraweave-core/src/perception.rs`) is called with `PerceptionConfig { los_max: 12 }` and objective `Some("extract")`.

The orchestrator is `RuleOrchestrator` from `astraweave-ai`. `propose_plan` (`astraweave-ai/src/orchestrator.rs`) sees an enemy and a ready smoke cooldown, and returns:

- `Throw` smoke at the integer midpoint of companion and enemy, here `(7, 2)`
- `MoveTo` two steps toward the enemy, here `(4, 1)`
- `CoverFire` on the enemy for `2.5`

`validate_and_execute` (`astraweave-core/src/validation.rs`) checks each step in order. The first step is `Throw`. The wall at `x = 6` makes that throw `EngineError::LosBlocked`. Later steps, including `MoveTo`, are not applied. `MoveTo` itself returns `EngineError::NoPath` only when `path_exists` fails.

`build_app(world, 0.25)` (`astraweave-core/src/ecs_adapter.rs`) stores the legacy world on an `astraweave_ecs::App` and sets resource `Dt(0.25)`. `run_fixed(1)` runs the ECS schedule once. The demo calls it 20 times. `Dt` is not a 60 Hz claim.

## 4. Change the companion's world

Edit `examples/hello_companion/src/main.rs`, function `setup_world`.

Move the rival off the row, for example `IVec2 { x: 12, y: 6 }`, or delete the obstacle loop. Save the file and run the same command:

```bash
cargo run -p hello_companion --release
```

The plan math follows the new positions: midpoint `((comp.x + enemy.x) / 2, (comp.y + enemy.y) / 2)`, and the move is two cells along each axis toward the enemy. Read the validator log to see whether `MoveTo` was applied.

## 5. Turn on another planner

Classical is the default. The other flags are in [Hello Companion](../examples/hello-companion.md). The one that surprises people:

```bash
cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter
```

That flag calls `GoapOrchestrator`, not `AIArbiter`. The type `AIArbiter` is documented on the [arbiter page](../core-systems/ai/arbiter.md). It is not on the deterministic tick.

`--bt`, `--utility`, and `--ensemble` need the `llm` feature and do not open a socket. `--llm` and `--hybrid` need a local Ollama on loopback and `--features llm,ollama`. Those two flags use `qwen3.5:4b` (`DEFAULT_QWEN_INSTRUCT_MODEL` in `astraweave-llm/src/qwen3_ollama.rs`).

## 6. Where this sits in a game

The same `World`, `RuleOrchestrator`, and `validate_and_execute` types are what you keep when you add dialogue, combat, or a window. That path is [Building your first game](../game-dev/first-game.md).

## Related

- [Quick start](./quick-start.md)
- [Examples index](../examples/index.md)
- [Engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md)

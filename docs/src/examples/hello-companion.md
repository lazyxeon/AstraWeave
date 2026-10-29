# Hello Companion

`examples/hello_companion` is the companion binary. The package name is `hello_companion`. This page walks that file from the command you type to the plan the classical orchestrator returns.

Run it from the repository root:

```bash
cargo run -p hello_companion --release
```

The crate default features are `fast-alloc` only (`examples/hello_companion/Cargo.toml`). That build does not open a window and does not call a model. It selects Classical (`RuleOrchestrator`).

## What the process does

`main` without the `visual` feature is `examples/hello_companion/src/main.rs`. It prints the banner, calls `select_ai_mode`, then `run_single_demo`.

`run_single_demo` does four things:

1. `setup_world` builds an `astraweave_core::World` and a `WorldSnapshot`.
2. `build_app(w, 0.25)` (`astraweave-core/src/ecs_adapter.rs`) inserts that world into an `astraweave_ecs::App` with a `Dt(0.25)` resource. The `0.25` is that resource. It is not a claim that this binary ticks at 60 Hz.
3. `generate_plan` asks the selected orchestrator for a `PlanIntent`.
4. `validate_and_execute` (`astraweave-core/src/validation.rs`) applies the plan or returns an `EngineError`. On `Err` the example prints the error and continues. It then calls `app.run_fixed(1)` twenty times and prints positions.

The strings the binary prints are the `println!` calls in that file, including `Using Classical AI (RuleOrchestrator).`, `Classical AI (RuleOrchestrator)`, `Generated {} step plan`, `Executing Plan`, and `Post-execution State`. The validator logs `Plan {} with {} steps`.

## The world

`setup_world` in the same file:

```rust
let mut w = World::new();
for x in 6..=6 {
    for y in 1..=8 {
        w.obstacles.insert((x, y));
    }
}
let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
let comp = w.spawn("Companion", IVec2 { x: 2, y: 3 }, Team { id: 1 }, 80, 30);
let enemy = w.spawn("Rival", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);
```

`World::spawn` is `astraweave_core::World::spawn(&mut self, name, pos, team, hp, ammo)`. The companion cooldown `throw:smoke` is set to `0.0`. The snapshot is `build_snapshot` with `PerceptionConfig { los_max: 12 }` and objective `extract`.

`validate_and_execute` is called with `ValidateCfg { world_bounds: (0, 0, 19, 9) }` and the companion entity id.

## The classical plan

`generate_classical_plan` constructs `RuleOrchestrator` and calls `propose_plan`. That method is `astraweave-ai/src/orchestrator.rs`. When the snapshot has an enemy and the smoke cooldown is at or below zero, the plan is three steps:

1. `ActionStep::Throw { item: "smoke", x: mid.x, y: mid.y }` where `mid` is the integer midpoint of the companion and the first enemy.
2. `ActionStep::MoveTo` two cells toward that enemy on each axis, using `saturating_add` and `signum`.
3. `ActionStep::CoverFire` on that enemy for `2.5` seconds.

For the coordinates `setup_world` uses, the companion is `(2, 3)` and the enemy is `(12, 2)`, so the midpoint is `(7, 2)` and the move target is `(4, 1)`. The obstacle column is `x = 6`.

`validate_and_execute` walks steps in order. `ActionStep::Throw` returns `EngineError::LosBlocked` when line of sight fails (`astraweave-core/src/validation.rs`). That variant's display text is `line of sight blocked` (`astraweave-core/src/schema.rs`). A debug `cargo run -p hello_companion` on this tree printed:

```text
Generated 3 steps
✅ Generated 3 step plan in <milliseconds>

--- Executing Plan @ t=0.00 ---
   Plan plan-0 with 3 steps
⚠️  Execution failed: line of sight blocked. Continuing...

--- Post-execution State @ t=5.00 ---
Companion: IVec2 { x: 2, y: 3 }
Enemy:     IVec2 { x: 12, y: 2 }
Enemy HP:  60
```

The millisecond field is whatever that process measured. The step count, the error line, the positions, and `t=5.00` follow from the source above.

The throw fails, so `MoveTo` does not run and the companion stays at `(2, 3)`. The process exits 0. `t=5.00` is twenty `run_fixed(1)` calls with `Dt(0.25)`. `MoveTo` would return `EngineError::NoPath` (`path not found`) if a later step reached a blocked path. This world does not get that far.

## Change it and run it again

Edit `setup_world` in `examples/hello_companion/src/main.rs`. Move the enemy, or clear the obstacle loop, then:

```bash
cargo run -p hello_companion --release
```

The same four steps run. The plan id is `plan-{}` from `(snap.t * 1000.0) as i64` inside `RuleOrchestrator::propose_plan`.

## Other modes

Flags go after `--`. Features go before it.

| Command | What runs |
| --- | --- |
| `cargo run -p hello_companion --release` | `RuleOrchestrator` |
| `cargo run -p hello_companion --release --features llm -- --bt` | Behavior tree in this file, feature `llm` |
| `cargo run -p hello_companion --release --features llm -- --utility` | Utility scorer, feature `llm` |
| `cargo run -p hello_companion --release --features llm,ollama -- --llm` | `Qwen3Ollama`, constant `DEFAULT_QWEN_INSTRUCT_MODEL` (`qwen3.5:4b`) |
| `cargo run -p hello_companion --release --features llm,ollama -- --hybrid` | LLM, then `generate_classical_plan` if that call returns `Err` |
| `cargo run -p hello_companion --release --features llm -- --ensemble` | Ensemble, feature `llm` |
| `cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter` | `GoapOrchestrator::propose_plan`, which calls `htn::legacy_goap_plan` |
| `cargo run -p hello_companion --release --features visual` | `visual_demo::run_visual_demo`. This one opens a window. |

`select_ai_mode`: with `ollama` compiled and no flag, the mode is Hybrid. With `llm` and not `ollama`, the mode is Behavior Tree. A flag whose feature is missing prints a warning and uses Classical.

`--arbiter` does not construct `AIArbiter`. See [AI Arbiter](../core-systems/ai/arbiter.md).

## What this binary is not

It is not the cross-OS replay job. The deterministic tick's planner is `RuleOrchestrator` inside `sys_ai_planning`, which is a different call site from this example's direct `propose_plan`. Measurements live on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page.

## Related

- [Your first companion](../getting-started/first-companion.md)
- [Examples index](./index.md)
- [AI system](../core-systems/ai/index.md)

//! Caps enforced inside the fixed tick. A machine cannot do extra work and diverge.

/// Load-push iterations. Never "until converged".
pub const STRESS_ITERS: u32 = 8;

/// Nodes whose load is pushed in one tick. The rest stay dirty.
pub const MAX_NODES_PER_TICK: usize = 1024;

/// Gameplay debris slots. Allocated with the world.
pub const MAX_SIM_DEBRIS: usize = 256;

/// Chunks whose display mesh may be rebuilt in one tick.
pub const MAX_REMESHES_PER_TICK: usize = 4;

/// Cells a single brush may touch. Larger brushes are rejected.
pub const MAX_BRUSH_CELLS: u32 = 4096;

/// Ticks under the sleep threshold before a slot merges to static.
pub const SLEEP_TICKS: u16 = 30;

/// Commands kept for late join. Past this, late join is refused.
pub const MAX_LOG_COMMANDS: usize = 65_536;

/// Scripted demolition length. The replay case runs this many ticks.
pub const DEMOLITION_TICKS: u64 = 300;

/// Fixed tick, matching the engine's 60 Hz step.
pub const TICK_HZ: i64 = 60;

# AstraWeave Interfaces (MVP Contracts)

Purpose: Provide short, copy-pastable contracts so implementation can proceed consistently across crates, tests, and IPC.

Versioning: All structs/messages include a `version: u16` field. Breaking changes bump minor, with migration notes.

## Core AI Contracts

```rust
// Workhorse snapshot filtered by perception rules (LOS, range, noise)
// See `astraweave-core/src/schema.rs` `WorldSnapshot` for the authoritative field set.

// Planner output – deterministic sequence of atomic actions
// See `astraweave-core/src/schema.rs` `PlanIntent` / `ActionStep`.

// Tool registry & validation constraints
bitflags::bitflags! {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ConstraintFlags: u32 {
        const LOS = 1<<0; const COOLDOWN = 1<<1; const STAMINA = 1<<2; const NAV = 1<<3; const BUDGET = 1<<4; const PHYSICS = 1<<5;
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolSpec { pub name: &'static str, pub args: Vec<ArgSpec>, pub constraints: ConstraintFlags }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ArgSpec { pub name: &'static str, pub ty: &'static str, pub optional: bool }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolRegistry { pub version: u16, pub tools: Vec<ToolSpec> }

// Orchestrator contract
// See `astraweave-ai/src/orchestrator.rs` for the `Orchestrator` trait.
```

## Navigation & Physics

```rust
// Nav baking job – inputs for tiled bake; output handled via crate API
pub struct NavBakeJob {
    pub cell_size: f32,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub include_dynamic: bool,
}

pub struct NavAgent { pub radius: f32, pub height: f32, pub max_slope_deg: f32 }

pub struct CharacterControllerTickInput {
    pub desired_move_ws: [f32; 3],
    pub jump: bool,
    pub crouch: bool,
    pub dt: f32,
}

pub struct CharacterControllerTickOutput {
    pub new_pos_ws: [f32; 3],
    pub on_ground: bool,
    pub hit_slope_limit: bool,
    pub step_up_down: f32,
}
```

## Networking Envelopes (Rust model)

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MsgHeader { pub version: u16, pub session_id: u64, pub seq: u64, pub sent_ms: u64 }

// See `astraweave-net/src/lib.rs` `Msg` enum.
```

## Protobuf Schemas (snapshot + intent)

```proto
syntax = "proto3";
package astraweave;

message Header { uint32 version = 1; uint64 tick = 2; uint64 time_ms = 3; }

message Vec3 { float x = 1; float y = 2; float z = 3; }

message EntityView { uint64 id = 1; string kind = 2; Vec3 pos = 3; Vec3 vel = 4; uint32 faction = 5; bool los_visible = 6; uint32 health_cur = 7; uint32 health_max = 8; }

message WorldSnapshot { uint32 version = 1; uint64 tick = 2; uint64 time_ms = 3; uint64 self_id = 4; repeated EntityView nearby = 5; }

message ActionStep { string verb = 1; string args_json = 2; uint32 min_ms = 3; uint32 max_ms = 4; }

message PlanIntent { uint32 version = 1; uint64 agent_id = 2; uint64 issued_tick = 3; repeated ActionStep steps = 4; string justification = 5; }
```

Notes
- **These protobuf schemas are a non-implemented design sketch, not the current wire format.** The actual network wire format is JSON over WebSocket (`tokio-tungstenite`), not protobuf — see [`networking.md`](../src/core-systems/networking.md). The authoritative in-code types are in `astraweave-core/src/schema.rs` (`WorldSnapshot`, `PlanIntent`); the message fields above may not match. Preserve versioning, determinism, and validation boundaries if this is ever implemented.
- Keep JSON/Proto schemas round-trippable; add migrations in `astraweave-memory` when fields change.

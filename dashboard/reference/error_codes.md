# AstraWeave Error & Validation Taxonomy (MVP)

Purpose: Enumerate engine-wide error kinds to standardize validation, logging, recovery, and tests.

## Tool Validation Errors

```rust
#[non_exhaustive]
pub enum ValidateErr {
    LosBlocked { from: u64, to: u64 },
    Cooldown   { tool: String, remaining_ms: u32 },
    NavInvalid { reason: NavFail, pos: [f32; 3] },
    Resource   { kind: ResKind, needed: u32, have: u32 },
    PhysicsConstraint { desc: &'static str },
    BudgetExceeded { budget_ms: u32 },
}

pub enum NavFail { NoPath, OutsideNavmesh, TooNarrow, SteepSlope, DynamicBlocked }
pub enum ResKind { Stamina, Ammo, Energy, Item }
```

## Engine/Subsystem Errors

- Render: ShaderCompile, PipelineCreate, SurfaceLost, OutOfMemory
- Audio: DeviceLost, DecodeFailed, StreamUnderrun
- Physics: InvalidCollider, QueryFailure
- Nav: BakeFailed, InvalidParams
- AI: PlannerTimeout, OrchestratorInternal
- IO/Asset: ImporterError(kind), CacheMiss, BadGuid
- Networking: Disconnect(reason), ProtocolMismatch, Timeout

## Recovery Hints

- LosBlocked → sidestep, re-plan with `hide` or `rally_on_player`
- Cooldown → defer tool and use filler verb (`stay`/`cover_fire` low-rate)
- NavInvalid → re-path, shrink agent radius, or request off-mesh link
- Resource → switch weapon, reload, or seek resource
- PhysicsConstraint → choose alternate approach path
- BudgetExceeded → enter defensive micro-policy until plan arrives

## Logging & Telemetry

- Include: tick, agent_id, verb, snapshot hash, seed, position.
- Redact PII; align with `SECURITY_AUDIT_GUIDE.md`.

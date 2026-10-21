# Loomspire Isle Greybox Specification

## Goals

- Provide a blockout blueprint for the 30-minute Veilweaver vertical slice.
- Define spatial metrics, traversal order, and trigger hooks for weaving tutorials and encounters.
- Align level streaming cells with existing `astraweave-scene` pipeline (four primary cells + boss arena additive cell).

## Zone Layout Overview

```
                 [Z4 Boss Courtyard]
                        ^
                        |
              [Z3 Loom Crossroads]
                        ^
                        |
    [Z2 Echo Grove] <---> Optional side alcove
                        ^
                        |
            [Z1 Frayed Causeway]
                        ^
                        |
            Start: Loom Gate Platform
```

| Zone | Cell ID | Dimensions (m) | Elevation Offset | Key Purpose |
|------|---------|----------------|------------------|-------------|
| Start | `GridCoord(100,0,0)` | 20 × 20 | 0 | Cinematic spawn, dialogue intro |
| Z1 | `GridCoord(101,0,0)` | 12 × 60 | +5 | Weaving tutorial bridge + Echo Shard cache |
| Z2 | `GridCoord(102,0,0)` | 45 × 45 | +12 | Skirmish gauntlet, cover weaving |
| Z2a | `GridCoord(102,1,0)` | 10 × 10 | +14 | Optional updraft alcove with Echo Shard |
| Z3 | `GridCoord(103,0,0)` | 35 × 30 | +20 | Storm routing choice, branching data flag |
| Z4 | `GridCoord(104,0,0)` | 55 × 55 | +28 | Adaptive boss fight with arena weaving |

## Streaming & Navigation Cells

- Each zone maps to a `SceneCell` with deterministic load order: gate → causeway → grove → crossroads → arena.
- Additive cell `echo_side_01` (12 × 18, +14 m) provides optional Echo Shard pickup; accessible via short glide.
- Navigation meshes: author coarse nav nodes first, refine after blockout review. Maintain 2 m clearance on bridge segments for deterministic collision.

## Tutorial Beats & Triggers

### Z1 — Frayed Causeway

| Trigger | Location (m) | Description |
|---------|--------------|-------------|
| `tut_start` | (0, 0, 0) | Activates dialogue nodes `n3`/`n3a`, highlights anchor. |
| `tut_anchor_hold` | (0, 18, 0) | Monitors weaving progress; failsafe resets anchor if stability drops below 20%. |
| `tut_success` | (0, 24, 0) | Plays bridge stabilization animation, grants Echo Shards (2). |

- Bridge segments: three modular blocks (4 m each) with physics toggle. Use placeholder gray materials (`greybox_concrete_albedo.png`).
- Guardrails intentionally absent to reinforce risk; add low barrier after playtest if falls exceed tolerance.

### Z2 — Echo Grove Skirmish

| Trigger | Location (m) | Description |
|---------|--------------|-------------|
| `combat_spawn` | Center (0, 0, 0) | Spawns 4 Rift Stalkers, 1 Echo-bound Sentinel. |
| `weave_cover_a` | (-6, 3, 0) | Deployable barricade anchor (cost 1 Echo Shard). |
| `weave_cover_b` | (8, -5, 0) | Secondary barricade anchor; optional. |
| `combat_complete` | (0, 0, 0) | Checks enemy defeat, awards Echo Dash unlock. |

- Vegetation proxies: cylinder clusters (0.6 m radius, 3.5 m tall) representing crystal-thread trees.
- Lighting cue: place two emissive placeholder pillars (1 m radius) to act as readability beacons.

### Z3 — Loom Crossroads

| Trigger | Location (m) | Description |
|---------|--------------|-------------|
| `choice_prompt` | (0, 0, 0) | Opens storm routing UI, ties into dialogue nodes `storm_stabilize` / `storm_redirect`. |
| `storm_stable_fx` | (0, 10, 0) | Activates when player chooses stabilization (blue particle placeholder). |
| `storm_redirect_fx` | (0, -10, 0) | Activates for redirect choice (amber particle placeholder). |

- Platform structure: three concentric rings; outer ring contains inactive conduit props.
- Ensure 3 m wide lanes for looped traversal; leave central plinth (radius 4 m) for cinematic focus.

### Z4 — Oathbound Warden Arena

| Trigger | Location (m) | Description |
|---------|--------------|-------------|
| `boss_intro` | (0, -10, 0) | Starts boss cinematic and selects arena variant based on storm choice. |
| `arena_anchor_a` | (10, 0, 0) | Repairable thread anchor; boss targets during Phase 2. |
| `arena_anchor_b` | (-10, 0, 0) | Mirrored anchor. |
| `phase_transition` | (0, 0, 0) | Switches VFX/material set after sigil break 2. |
| `boss_defeat` | (0, 0, 0) | Triggers outro metrics overlay and dialogue nodes `n11_stable` or `n11_redirect`. |

- Arena geometry: octagonal platform with 6 m edge lengths; three gravity pylons (2 m radius) positioned at 120°.
- Add 1.5 m lip to outer edge to discourage accidental falls while preserving tension.

## Metrics & Player Readability

- Player jump: 3.5 m max horizontal, 1.2 m vertical when unassisted. All critical gaps ≤3 m.
- Weaving anchors: 1.2 m tall pedestals; highlight radius 2.5 m. Color code (cyan = structural, amber = tactical).
- Echo Shard pickups: glowing rhombus proxies (0.8 m tall). Place two in Z1, three in Z2 side alcoves, one in Z3 central ring.

## Implementation Checklist

- [ ] Create placeholder meshes in `assets/greybox/loomspire/` (FBX or GLTF) for each zone block.
- [ ] Author `.ron` scene descriptors referencing new meshes, anchors, and trigger ids.
- [ ] Wire triggers to tutorial scripts (`tut_start`, `choice_prompt`, etc.) and link to dialogue node ids specified above.
- [ ] Validate navmesh coverage using default capsule (0.5 m radius, 1.8 m height).
- [ ] Run `cargo check -p astraweave-scene` after adding new cell descriptors.

## Dependencies & Notes

- Rendering: requires twilight skybox variant and emissive placeholder materials—coordinate with rendering pipeline schedule.
- Audio: ambient wind loop per zone plus boss layering cues; capture requirements in material/audio plan update.
- Determinism: all triggers should use fixed seed sequences; avoid random spawn offsets during greybox phase.

---

*Prepared to guide blockout implementation for the Veilweaver vertical slice.*


# Veilweaver Vertical Slice Implementation Roadmap

## Objective

Provide a cross-crate execution plan that transforms design specs into the playable 30-minute slice.

## Workstreams & Owners

| Workstream | Primary Crates | Key Tasks | Milestones |
|------------|----------------|-----------|------------|
| Level & Streaming | `astraweave-scene`, `astraweave-terrain`, `assets/cells`, `examples/veilweaver_slice_loader` | Author Loomspire greybox `.ron` cells, implement trigger scripts for tutorials, ensure deterministic cell loading, validate with slice loader example | Greybox walkthrough ready end of Week 1 |
| Weaving Systems | `astraweave-weaving`, `astraweave-gameplay` | Add tutorial-specific anchors, thread stability budget tracking, anchor repair actions | Tutorial loop functional Week 2 |
| Companion AI | `astraweave-ai`, `astraweave-gameplay`, `astraweave-observability` | Implement GOAP goals/actions (see `ARIA_COMPANION_BEHAVIOR.md`), add Echo charge resource, emit telemetry events | Companion adaptive unlock milestone Week 3 |
| Boss Director | `astraweave-director`, `astraweave-gameplay`, `astraweave-render` | Integrate Oathbound Warden state machine, arena modifiers, adaptive ability selection, VFX hooks | Boss phase transitions stable Week 4 |
| UI & Telemetry | `astraweave-ui`, `telemetry_hud`, `astraweave-audio` | Build thread HUD, storm decision UI, post-run recap, companion/boss telemetry cards, ambient/audio cues | UI alpha Week 3, polished in Week 6 |
| Rendering & Materials | `astraweave-render`, `assets/materials/loomspire` | Produce twilight skybox, material sets, weaving VFX previews, storm variant effects | Material review Week 3, VFX iteration Week 4 |
| Audio | `astraweave-audio`, `assets/voices`, `assets/ambient` | Record/placeholder VO for new dialogue nodes, compose zone ambience, layer boss themes, weave SFX | Initial mix Week 5, final pass Week 6 |

## Cross-Cutting Tasks

1. **Telemetry Integration**
   - Ensure all new systems emit events consumed by post-run recap.
   - Update `examples/veilweaver_demo` to optionally run slice telemetry validations.
2. **Determinism Validation**
   - Add integration test covering storm choice branch consistency (3-run check).
   - Record seeds & event logs for regression tracking.
3. **Documentation & Master Reports**
   - Update `VEILWEAVER_VERTICAL_SLICE_PLAN.md` revision history each milestone.
   - If work exceeds thresholds, propagate metrics to master roadmap/coverage reports.

## Suggested Sequencing

1. Finalize greybox geometry & triggers → unlock gating for mechanics/AI.
2. Implement weaving tutorial logic + Echo resource tracking → required for companion support.
3. Build companion action set and telemetry → prerequisite for boss adaptation data.
4. Develop boss encounter state machine → dependent on choice flag and companion events.
5. Layer UI, VFX, audio polish in parallel once gameplay loop stabilized.

## Risk Mitigation

- **Complexity Creep**: enforce scope guard (single island, one boss) via weekly review.
- **Performance**: monitor frame time using Tracy instrumentation during Week 4+ playtests.
- **LLM Dependency**: maintain offline fallback scripts for Aria banter to guarantee deterministic builds.

---

*Roadmap ties design deliverables to concrete implementation steps across the AstraWeave codebase.*


# T.W-R Gate Ratification Record — 2026-07-21 (director)

> **Purpose:** the gate record for `docs/audits/TWR_WATER_RECON.md` (commit `09c589a5d`). The director read the recon's §3.5 recommendation and §4 open questions and dispositioned them as recorded below on 2026-07-21. **These decisions are made** — recorded so future beats read a document, not a chat. Committed by session T.W.1; do not reopen without a new director gate.

---

## 1. Beat split: T.W → T.W.1 + T.W.2

The recon's two-phase recommendation is **RATIFIED**. T.W splits into:

- **T.W.1** (this beat): surface the existing water system in the editor — format fix, E3-aware gate, sea-level unification, hygiene deletions, enable/level UI, trivial per-archetype style, dip census. No paint.
- **T.W.2**: paint — carve-brush (sea-connected water) + volume entities (perched water). **Drafted only after T.W.1's gate.**

## 2. The seven recon §4 answers (verbatim dispositions)

| # | Question | Director's answer |
|---|---|---|
| 1 | Sea-level model | **Sea level Y=2.0 is THE world sea level, single source of truth.** Constants unified/tied |
| 2 | Inland sub-sea dips | **Provisionally a FEATURE** (emergent ponds/oases — accept plane flooding; no chunk masking). Final ruling at T.W.1's gate, informed by the dip census. Pathological speckling would be fixed by floor tuning in T.2, **never by a second water authority** |
| 3 | Paint semantics | §3.5 order ratified: carve-brush (a) first, volume entities (c) for perched water, mask-paint (b) deferred — all T.W.2 |
| 4 | Persistence | **Session-transient stands.** Terrain-edit persistence is the roadmap's M3 concern, not the water beats' |
| 5 | River scope | **Rivers stay with the future hydrology campaign.** T.W water = sea-connected (+ authored volumes in T.W.2) |
| 6 | Hygiene rider | **Ratified in full**: delete the dead SPH-era Terrain-panel block (`FluidSimParams`/`WaterBodyPreset`/`DetectedWaterBodyInfo`/detection fields), the zero-caller `apply_brush_paint`, and the W.2c.2 hardcoded scaffolding weaves (replaced by nothing; a real editor weave feed is future work) |
| 7 | `WaterStyle` per archetype | **In v1 only if trivial** (a simple `WorldArchetypeId` → existing-`WaterStyle` mapping). If more than a mapping, ship Ocean-hardcoded and note for T.2 |

## 3. T.1 rung-3: **PASS** (declared 2026-07-21)

The director ran the T.1 repro (`T1_BEACH_OUTCOME.md` §4 — Mediterranean seed 12345, radius ≥6, vs Desert) and declared the render check **passed**: beach reads distinctly from desert at a coastline. **T.1 (distinct beach material, commit `1f0b48b5e`) is closed at all three rungs.** This record replaces the chat-only declaration.

---

*Recorded from the director's 2026-07-21 dispositions by session T.W.1. Gate artifacts: this record + `TWR_WATER_RECON.md` (`09c589a5d`).*

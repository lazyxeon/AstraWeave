# Veilweaver Vertical Slice Plan — 30-Minute Demo

## Purpose

- Deliver a polished, replayable 30-minute vertical slice that proves the AI-native pillars of Veilweaver.
- Showcase fate-weaving traversal, companion co-op tactics, and an adaptive boss that reacts across phases.
- Produce a tightly scoped content set that exercises AstraWeave Priority 1 UI, rendering, and AI subsystems while remaining production-feasible.

## Experience Outline (Target Runtime: 30 Minutes)

| Minute Mark | Beat | Location | Core Systems | Player Takeaway |
|-------------|------|----------|--------------|-----------------|
| 0–3 | **Cold-Open Cinematic + Tutorial Walk** | Twilight causeway leading to the Loomspire Isle | Sequenced cinematic (astraweave-cinematics), minimal traversal control | Establish tone, introduce companion Aria, set initial objective |
| 3–8 | **Fate-Weaving Tutorial** | Frayed Aether Bridge | Fate-thread weaving mini-puzzles, thread HUD, echo resource pickup | Learn to reinforce / redirect threads, collect Echo Shards |
| 8–14 | **Skirmish Gauntlet** | Crystal-thread forest grove | Echo-infused combat, companion assist calls, cover reshaping weave | Demonstrate combat loop and companion responsiveness |
| 14–20 | **World State Choice** | Loomspire Crossroads | Narrative choice, branching thread map, UI decision overlay | Player alters storm route, companion comments, sets boss modifiers |
| 20–27 | **Boss Arena: The Oathbound Warden** | Gravity-anchored courtyard | Adaptive boss AI, arena weaving (hazards on/off), companion combo moves | Boss evolves tactics based on player choices; demonstrates director |
| 27–30 | **Resolution & Metrics Recap** | Sky pier overlooking isle | Dialogue resolution, metrics HUD (companion bonding, boss adaptations) | Reinforce persistence hooks, tease broader campaign |

## Storyboard Summary

1. **Scene A — Loom Awakening (0–2 min)**
   - Visual: Cinematic fly-through of floating archipelago under eternal dusk.
   - Actions: Player avatar Talren awakens at broken loom gate; companion Aria links via holo-thread.
   - Systems: Pre-rendered camera path + Dialogue from `docs/projects/veilweaver/assets/dialogue_intro.toml` extended with voice packs.
   - Goal: Establish stakes (“The Loomspire resonates—threads fray unless we act”).

2. **Scene B — Guided Approach (2–5 min)**
   - Visual: Player-controlled walk across a fractured bridge; ambient particles denote destabilized threads.
   - Interactions: Lightweight weaving prompt (press + hold to project thread anchor) to rebuild pathway segments.
   - Tutorial UI: Thread HUD overlay (Priority 1 UI framework) introduces “Stability Meter” and “Echo Shards”.

3. **Scene C — Echo Grove Skirmish (5–12 min)**
   - Visual: Bioluminescent flora, moving thread veins along trees.
   - Gameplay: Encounter waves of `Rift Stalker` pack + `Echo-bound Sentinel`. Player uses woven barricades to funnel enemies. Aria calls out flanks, adapts healing/damage support based on player health.
   - Cinematic beats: Slow-motion highlight when fate-thread counter is triggered (astraweave-cinematics partial camera override).

4. **Scene D — Loom Crossroads Dilemma (12–18 min)**
   - Visual: Central loom node with three tethered storm conduits.
   - Choice: Player either stabilizes storm (safer arena but boss gains shielding) or re-routes storm (harder traversal but boss loses fog reinforcements). UI uses radial menu from Priority 1 UI plan.
   - Companion Dialogue: Branching lines referencing choice (persisted for end recap).

5. **Scene E — The Oathbound Warden (18–27 min)**
   - Phase 1: Warden tests player, raising kinetic barriers if player used ranged focus earlier.
   - Phase 2: Boss references choice from Scene D (storm shielding vs fog removal) and adapts attack schedule.
   - Set pieces: Fate-thread anchors maybe targeted by boss requiring quick repairs (weaving under pressure).
   - Companion synergy: Aria executes combo when player staggers boss; uses dynamic LLM-driven callouts.

6. **Scene F — Debrief & Hook (27–30 min)**
   - Visual: Sky pier with view of deeper archipelago.
   - Systems: Metrics overlay summarizing run (time, echoes collected, companion affinity). Save hook for future builds.
   - Narrative: Tease next objective (“Threads converge on the Obsidian Choir—choose our next weave”).

## Progression & Pacing

- **Player Progression**
  - Start with base fate-weaver toolkit: Thread Anchor (bridge repair) and Echo Pulse (light attack).
  - Unlock **Resonant Guard** during first combat wave (defensive weave) and **Echo Dash** after crossroads choice.
  - Temporary upgrades tied to Echo Shards (consumable buff nodes) to avoid long-term inventory overhead.

- **Companion Progression**
  - Aria begins with Support Protocol (healing pulse) and Tactical Ping (marks enemy).
  - Learns one adaptive behavior mid-slice based on observed player preference:
    - If player uses melee >70%: unlock **Threadbind Riposte** (counterattack assist).
    - If player uses weaving defensively >50%: unlock **Stability Surge** (area reinforce).
  - Progress stored to recap screen (foundation for persistence system).

- **World State Progression**
  - Storm control decision toggles global modifiers for boss fight and environment VFX.
  - Environmental storytelling via thread resonance logs that unlock concept art (optional collectible).

## Characters & Encounter Archetypes

### Playable Protagonist — Talren Veyl
- Role: Player-controlled Veilweaver initiate.
- Loadout: Fate Loom gauntlet (primary), Echo Dagger (secondary).
- Animation Set: 12 locomotion clips (idle, walk, run, weave stance), 6 combat moves, 3 weaving gestures.
- VO: Minimal for slice (battle grunts + 2 dialogue responses).

### Companion — Aria (AI-First Showcase)
- Personality: Analytical but empathetic; calibrates tone based on player risk profile.
- Systems: Uses astraweave-ai orchestrator with GOAP baseline + Hermes LLM fallback for banter.
- Mechanics: Provides buffs, callouts, and executes combo prompts; tracks short-term memory for boss adaptation commentary.

### Key NPC — Loom Warden Hologram
- Occurs during crossroads to deliver exposition; static projection requiring limited animation.
- Acts as UI narrative anchor explaining consequences of weaving choice.

### Enemy Archetypes
- **Rift Stalker (Grunt)**: Fast melee, disrupts weaving by applying thread decay. Requires quick stagger response.
- **Echo-bound Sentinel (Elite)**: Ranged crystal cannon; deploys barriers influenced by player behavior (mirrors adaptation theme).
- **Stormbound Wisp (Support)**: Spawned during boss fight; heals boss unless threads severed.

### Boss — The Oathbound Warden
- Form: Humanoid titan entwined with animated chains.
- Phases:
  1. **Assessment** — Mirrors player attack patterns; uses data from companion callouts to choose counters.
  2. **Fulcrum Shift** — Reconfigures arena (raises pillars or opens gaps) based on weaving decision.
  3. **Directive Override** — Gains new ability depending on player’s favored tactic (e.g., anti-ranged dampening field or anti-melee counter-shock).
- Defeat Condition: Break three binding sigils (requires weaving under pressure + combat synergy).

## Asset & Texture Requirements

### Environment
- **Biome Kit: Loomspire Isle**
  - Modular floating terrain meshes (rock base, crystal flora, thread conduits).
  - Texture sets: twilight sky gradient, iridescent crystal, woven metal. Leverage `astraweave-render` material arrays (albedo, normal, MRA). Target 2K textures for hero assets, 1K for modular pieces.
- **Bridge & Thread Props**
  - Fate-thread bridge segments with emissive thread core (animated shader for weaving interactions).
  - Thread anchor nodes (interactive props with blend shapes for reinforced/damaged states).
- **Boss Arena**
  - Circular platform with gravity pylons, dynamic storm skybox variant (hooks into Priority 2 Rendering plan). Include decal set for boss phase transitions.

### Characters & Creatures
- **Talren**: Medium-poly hero mesh (~60k tris), modular armor plates (allows recolor). Texture sets (albedo/normal/MRA) + emissive accents.
- **Aria**: Companion model with cloak, luminescent thread core. Facial rig for limited expressions.
- **Rift Stalker**: Quadruped with segmented limbs; two texture variants for palette swap.
- **Echo-bound Sentinel**: Bipedal golem with crystalline weapon; includes broken state mesh.
- **Oathbound Warden**: Large rig (4-5m tall), chain accessories, glowing sigils. Requires unique shader for adaptive glow (scale intensity with boss phase).
- **Stormbound Wisp**: Particle-based entity using billboard textures + mesh core.

### VFX & Particles
- Weaving threads (spline-based, color shifts to indicate state).
- Echo collection burst (glittering shards + radial shockwave).
- Boss phase transition (thread shatter + chain recoil).
- Companion combo highlight (glyph overlay at enemy location).

### UI/HUD Elements
- Thread stability bar, Echo shard counter, companion affinity meter.
- Decision radial UI (storm choice) with iconography for consequences.
- Post-run metrics panel with dynamic callouts.

### Audio (Non-texture but integral)
- UI stingers for weaving success/failure.
- Ambient loop for twilight archipelago.
- Boss theme with adaptive layers (ties into Priority 4 audio plan).

## Engine Integration Notes

- **UI Framework**: Leverage Phase 8 Priority 1 UI components (menu.rs, HUD modules). Need custom widgets for thread HUD and decisions.
- **AI Systems**: Use astraweave-ai GOAP for base behaviors; integrate Hermes 2 Pro for Aria’s banter and boss adaptive quips (async planning with prerecorded fallbacks if offline).
- **Cinematics**: Extend `astraweave-cinematics` timeline to support hybrid interactive sequences (Scene B quick control handoff).
- **Telemetry**: Reuse `examples/veilweaver_demo` telemetry pipeline for performance capture; expose results on post-run panel.
- **Runtime Harness**: Enable the `veilweaver_slice` feature on `examples/veilweaver_demo` (powered by `veilweaver_slice_runtime`) to boot the slice ECS runtime directly for integration testing.
- **Weaving Mechanics**: Build atop `astraweave-weaving` crate (ensure deterministic world state changes, budget-limited modifications).

## Production Roadmap (6-8 Weeks)

1. **Week 1 — Narrative & Blockout**
   - Greybox Loomspire Isle, script cinematics A and B, expand dialogue TOML.
2. **Week 2 — Core Mechanics**
   - Implement weaving tutorial and Echo Grove combat prototypes; integrate HUD elements.
3. **Week 3 — Companion & AI**
   - Build Aria behavior tree/GOAP states, wire LLM banter fallback, implement adaptive unlock logic.
4. **Week 4 — Boss Director**
   - Prototype Oathbound Warden phases, connect choice modifiers, ensure deterministic adaptation.
5. **Week 5 — Art & Audio Pass**
   - Import hero environment kit, texture characters, author key VFX, stub adaptive music.
6. **Week 6 — Polish & Validation**
   - Performance tuning, telemetry integration, user testing for 30-minute pacing, update master reports if thresholds met.
7. **Optional Week 7–8**
   - Accessibility, localization hooks, additional replay incentives (time trials, higher difficulty toggle).

## Rendering & Audio Pipeline Sync

| Track | Owner Modules | Tasks | Dependencies | Delivery |
|-------|---------------|-------|--------------|----------|
| Materials & Lighting | `astraweave-render` (`materials`, `post_fx`), asset team | Author twilight skybox variant, Loomspire material arrays (rock, crystal, thread), placeholder emissive props for greybox review | Week 2 greybox geometry locked; requires Phase 8 Priority 2 shader validation | Week 3 end (ready for art pass) |
| VFX | `astraweave-render` particle framework | Create weaving thread shader graph, storm stabilization/redirect VFX toggles, boss phase transition burst | Material palettes finalized; depends on storm choice data flag | Week 4 mid (boss prototyping) |
| Audio | `astraweave-audio` mixer, `priority_4` plan | Produce ambient loops per zone, weave interaction SFX, boss adaptive music layers (calm/intense/finale) | Needs zone timings from level script; integrates with UI telemetry events | Week 5 start for implementation, full mix by Week 6 |
| UI & Telemetry | `astraweave-ui`, `telemetry_hud` | Implement thread HUD widgets, storm decision radial UI, post-run recap metrics | UI framework milestone (Phase 8 Priority 1) + telemetry hooks from gameplay systems | Week 3 alpha, Week 6 polish |

- Coordinate material authoring with `PHASE_8_PRIORITY_2_RENDERING_PLAN.md` milestones (shadow validation before Week 4).
- Audio track leverages existing four-bus mixer; reserve 2 SFX channels for weaving feedback to avoid clipping.
- Establish weekly sync (Fridays) to review rendering/audio progress against greybox updates; log outcomes in journey docs.

## Deliverables Checklist

- Updated documentation linking to this plan.
- Playable build with automated start-to-finish test script (recording runtime & key metrics).
- Asset list registered in `assets/materials/loomspire/` with metadata for material manager.
- Companion dialogue extensions in TOML with branching for choice outcomes.
- Boss telemetry hooks capturing adaptation decisions (for validation reports).
- `cargo run -p veilweaver_slice_loader` (no features) to validate cell streaming; run gameplay harnesses with `--features veilweaver_slice` to enable companion and boss logic when the slice integration crate is ready.

---

*This plan aligns Veilweaver’s existing AI-first vision with a concrete 30-minute slice, ensuring each subsystem (weaving, AI companion, adaptive boss) is showcased within a production-feasible scope.*


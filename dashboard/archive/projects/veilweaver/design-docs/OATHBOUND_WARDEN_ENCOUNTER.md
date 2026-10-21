# Oathbound Warden Encounter Spec

## Encounter Goals

- Showcase adaptive boss behavior reacting to player choices and tactics.
- Exercise weaving under pressure with arena anchors targeted by the boss.
- Provide clear telegraphs and fairness while demonstrating AstraWeave’s deterministic AI director.

## Arena Layout

- Octagonal platform (radius 18 m) with three gravity pylons at 120°.
- Two weave anchors (`arena_anchor_a`, `arena_anchor_b`) positioned east/west at 10 m radius.
- Environmental variant toggled by storm routing choice:
  - **Stabilized**: Clear visibility, two armor conduits breakable after heat vent telegraph.
  - **Redirected**: Particle-heavy atmosphere, motes traverse platform causing periodic vision occlusion.

## Phase Breakdown

| Phase | Trigger | Boss Behavior | Player Counter |
|-------|---------|---------------|----------------|
| Assessment | Encounter start | Cycles between Cleave Combo and Chain Lash; samples player damage types every 5 s | Observe telegraphs, avoid lash by rolling through gap |
| Fulcrum Shift | Boss health ≤ 66% | Applies storm choice modifiers (armor plates or motes). Targets weave anchors with `Anchor Rupture` | Repair anchors while dodging targeted AoEs |
| Directive Override | Boss health ≤ 33% | Activates adaptive ability based on player tactic (Anti-Ranged field or Counter-Shock aura). Summons Stormbound Wisps | Break sigils while interrupting wisps; coordinate with Aria combo |

## Ability Catalog

- **Cleave Combo**: 3-hit frontal cleave; last hit leaves lingering thread fracture. Telegraph: glowing chains tighten.
- **Chain Lash**: Mid-range swipe hitting 90° arc; used to punish kiting.
- **Anchor Rupture**: Chooses active arena anchor, charges for 2 s, then fires beam dealing damage and destabilizing thread.
- **Storm Barrier (Stabilize path)**: Deploys armor plating; requires player to hit glowing vents post-charge.
- **Mote Surge (Redirect path)**: Releases drifting motes that obscure vision; each mote collision deals minor damage.
- **Adaptive Abilities**:
  - `AntiRangedField`: Reduces ranged damage by 40%; triggered if ranged > 35% of player damage.
  - `CounterShockAura`: Reflects 15% melee damage; triggered otherwise.
- **Summon Wisps**: Spawns two support adds that channel heal beams until interrupted.

## Deterministic Director Logic

1. **Telemetry Sampling**
   - Input: Player damage breakdown, weave usage, companion action log.
   - Updated at start of each phase and every 10 s thereafter.
2. **Decision Tree**
   - Evaluate storm choice flag (stable vs redirect) → set arena profile.
   - Use tactic ratios to pick adaptive ability for Phase 3.
   - Record selection in `BossAdaptationEvent` for recap.
3. **Constraint Enforcement**
   - Cooldown gating ensures adaptive ability triggers at most once per phase.
   - Anchor Rupture cannot target same anchor twice in succession.

## Telemetry & UI Integration

- Emit `BossEvent::ArenaModifier { modifier_id }` when Fulcrum Shift applies storm choice.
- Emit `BossEvent::AdaptiveSelection { ability_id, reason }` upon entering Directive Override.
- Post-run recap panel surfaces: `ArmorPlatesBroken`, `AnchorsRepaired`, `AdaptiveAbilitySeen`.

## Implementation Tasks

1. Extend `astraweave-director` to support tactic sampling and event emission for Warden.
2. Wire `OathboundWardenDirector` (feature `veilweaver_slice`) into boss encounter harnesses (see `examples/adaptive_boss`).
3. Author encounter script in `astraweave-gameplay` with state machine controlling phases.
4. Implement telegraph VFX (heat vent glow, motes, chain highlights) via `astraweave-render` particle system.
5. Create audio cues for phase transitions and adaptive ability triggers using `astraweave-audio` mixer.

---

*Guides the creation of the Oathbound Warden adaptive boss fight for the vertical slice.*


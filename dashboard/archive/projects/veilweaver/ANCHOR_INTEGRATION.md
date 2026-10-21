# Anchor Integration System - Veilweaver Vertical Slice

**Version**: 1.0 (Week 1 Day 6)  
**Date**: November 8, 2025  
**Scope**: Fate-weaving anchor mechanics for greybox zones  
**Status**: ✅ Design Complete, ⚠️ Implementation Week 2  

---

## Executive Summary

The **Anchor System** is Veilweaver's core narrative and gameplay mechanic. Anchors are reality-stabilization devices built by ancient Weavers to prevent causality collapse. Players use **fate-weaving abilities** to repair decaying anchors, spending **Echo currency** earned from combat and exploration. Anchor stability directly impacts world state: high stability = stable reality, low stability = reality distortion effects (visual glitches, physics anomalies, enemy spawns).

**Week 1 Greybox Coverage**:
- **4 anchors** across 3 zones (Z0: 1, Z1: 2, Z2: 1)
- **3 stability states** (100%, 70%, 0%)
- **Echo economy** (earn 2-3 from Z0, spend 1-2 per repair)
- **Tutorial progression** (inspect → repair → ability unlock)

**Week 2 Implementation**:
- Interactive anchor inspection UI (proximity trigger, stability meter, repair prompt)
- Echo currency system (HUD display, pickup particles, transaction feedback)
- Anchor repair VFX (weaving animation, stability increase, glow change)
- Ability unlock system (Echo Dash unlocked by repairing Z2 vista anchor)

---

## Section 1: Anchor Lifecycle

### A. Anchor States

Anchors progress through 5 stability states based on decay over time:

| State | Stability | Visual | Audio | Physics | Gameplay Impact |
|-------|-----------|--------|-------|---------|-----------------|
| **Perfect** | 1.0 (100%) | Bright blue glow, no particles | Resonant hum (440 Hz, stable) | No distortion | Ideal condition, tutorial reference |
| **Stable** | 0.7-0.99 (70-99%) | Dim blue glow, few decay particles | Flickering hum (440 Hz, occasional static) | Rare visual glitches | Playable, minor narrative tension |
| **Unstable** | 0.4-0.69 (40-69%) | Yellow glow, many decay particles | Distorted hum (400-480 Hz, modulated) | Frequent visual glitches, objects flicker | High narrative tension, repair urgent |
| **Critical** | 0.1-0.39 (10-39%) | Red glow, dense particle storm | Harsh static (200-600 Hz, chaotic) | Reality tears visible, objects phase | Mission at risk, repair mandatory |
| **Broken** | 0.0 (0%) | No glow, reality tear vortex | Silence (eerie absence) | Large reality tears, mission failure | Game over state (zone collapses) |

**Decay Rate**:
- Passive decay: -0.01 stability per 60 seconds (1% per minute)
- Combat stress: -0.05 stability per enemy killed nearby (5% per kill)
- Repair bonus: +0.3 stability per repair (30% increase)
- **Z2 vista anchor starts at 0.7 (70%)** - decays to 0.4 (40%) after 30 minutes if not repaired

### B. Interaction Flow

**Player proximity trigger** (3m radius):
1. **Approach** (3m): Anchor glows brighter, UI prompt appears ("Inspect Anchor [E]")
2. **Inspect** (press E): Camera focus shifts to anchor, stability meter displays, repair prompt appears
3. **Decide**: Player chooses to repair (spend Echoes) or skip (continue with current stability)
4. **Repair** (if chosen):
   - Deduct Echo cost from currency pool
   - Play weaving animation (5-second channeling, player cannot move)
   - VFX: Threads of light wrap around anchor, stability increases +30%
   - SFX: Resonant chord (C major, 3 octaves)
   - Particle burst: Blue particles radiate outward (1m radius)
5. **Completion**: Camera returns to player, UI prompt disappears, anchor glow updates to new stability state

**Ability Unlock (Z2 vista anchor only)**:
- If player repairs Z2 vista anchor (70% → 100%), unlock **Echo Dash** ability
- Echo Dash: Short teleport (5m range), costs 1 Echo per use
- Tutorial text: "New Ability Unlocked: Echo Dash (Shift + Direction)"
- Used in Z1 Echo Grove to dodge Rift Stalker attacks

---

## Section 2: Echo Currency System

### A. Echo Sources & Costs

**Echo Acquisition**:

| Source | Echo Amount | Zone | Condition |
|--------|-------------|------|-----------|
| Z0 Tutorial Reward | +2-3 Echoes | Loomspire Sanctum | Complete cinematic_intro |
| Rift Stalker Kill | +1 Echo | Echo Grove | Per enemy (4 enemies = 4 Echoes) |
| Echo-bound Sentinel Kill | +2 Echoes | Echo Grove | Boss enemy (1 per playthrough) |
| Hidden Echo Shard | +1 Echo | Fractured Cliffs (optional) | Z=100m, left cliff alcove |
| **Total Available** | **9-10 Echoes** | **Across 3 zones** | **Week 1 greybox** |

**Echo Costs**:

| Action | Echo Cost | Zone | Effect |
|--------|-----------|------|--------|
| Repair Z0 loomspire_central_anchor | 5 Echoes | Loomspire Sanctum | Tutorial (too expensive, teaches resource scarcity) |
| Repair Z2 vista_tutorial_anchor | 2 Echoes | Fractured Cliffs | Unlocks Echo Dash ability |
| Repair Z1 cover_anchor_northwest | 1 Echo | Echo Grove | Deploy barricade (tactical cover) |
| Repair Z1 cover_anchor_southeast | 1 Echo | Echo Grove | Deploy barricade (tactical cover) |
| Use Echo Dash ability | 1 Echo | Any zone (after unlock) | Short teleport (5m range) |

**Optimal Resource Path**:
1. Z0: Earn 2-3 Echoes (tutorial reward), skip loomspire anchor repair (too expensive)
2. Z2: Spend 2 Echoes to repair vista anchor (unlock Echo Dash, critical for combat)
3. Z1: Earn 6 Echoes from combat (4 Stalkers + 1 Sentinel)
4. Z1: Spend 1-2 Echoes on barricade anchors (tactical advantage)
5. Z1: Reserve 3-4 Echoes for Echo Dash uses (dodge/mobility)

**Economy Balance**:
- Players start with 2-3 Echoes (enough for vista anchor + 1 spare)
- Combat zone provides 6 Echoes (enough for barricades + Echo Dash uses)
- Encourages tactical decisions (repair barricades vs save for mobility)
- Optional content (hidden shard) provides buffer for mistakes

### B. HUD Display

**Echo Currency UI** (top-right corner):
- Icon: Glowing blue shard (16×16 pixels)
- Count: Large number (e.g., "5") next to icon
- Animation: Glow pulse when Echoes gained (1-second fade)
- Tooltip (on hover): "Echoes: Currency for repairing anchors and weaving abilities"

**Anchor Stability Meter** (appears during inspection):
- Position: Center screen, below anchor model
- Visual: Horizontal bar (200×20 pixels)
- Fill color: Gradient (red → yellow → blue) based on stability
- Percentage text: "Stability: 70%" (large font, white)
- Repair cost: "Repair Cost: 2 Echoes" (below bar, yellow if affordable, red if not)

**Transaction Feedback**:
- Echo gain: "+1 Echo" text floats upward from pickup location (1-second duration)
- Echo spend: "-2 Echoes" text appears in HUD (1-second duration, red color)
- Insufficient Echoes: "Not enough Echoes" text (center screen, 2-second duration, red)

---

## Section 3: Greybox Zone Integration

### A. Z0: Loomspire Sanctum

**Anchor**: `loomspire_central_anchor`
- **Position**: (0, 2, 0) - Center of platform, elevated 2m
- **Initial Stability**: 1.0 (100%) - Perfect condition (tutorial reference)
- **Repair Cost**: 5 Echoes (intentionally too expensive for tutorial)
- **Purpose**: Teach anchor inspection UI, establish visual baseline for "perfect" state
- **Narrative**: Seris (companion NPC) explains: "This anchor is stable. Most aren't."

**Tutorial Flow**:
1. Player spawns at Z=-5, cinematic_intro plays (15s)
2. Dialogue "intro_awakening" triggers (Seris introduces anchors)
3. Player walks toward center, anchor glows brighter at 3m proximity
4. UI prompt: "Inspect Anchor [E]" (teaches inspection mechanic)
5. Player inspects: Stability 100%, Repair Cost 5 Echoes (player has 2-3)
6. Seris dialogue: "You don't have enough Echoes. Good—save them for the cliffs."
7. Player exits inspection, continues to Z2 Fractured Cliffs transition

**Echo Reward**: +2-3 Echoes (granted during cinematic_intro, represents "tutorial stipend")

### B. Z2: Fractured Cliffs

**Anchor**: `vista_tutorial_anchor`
- **Position**: (0, 11, 200) - Vista platform center, elevated 10m above path
- **Initial Stability**: 0.7 (70%) - Stable but decaying (dim blue, few particles)
- **Repair Cost**: 2 Echoes (affordable with tutorial stipend)
- **Purpose**: First repairable anchor, unlocks Echo Dash ability
- **Narrative**: Seris dialogue "vista_overview" (line 3): "Test your weaving on that anchor first. You'll need the practice."

**Tutorial Flow**:
1. Player walks 200m linear path (5-6 min journey)
2. Dialogue triggers: "journey_awakening" (Z=0), "anchor_lore" (Z=100), "vista_overview" (Z=200)
3. Player reaches vista platform, anchor visible at center
4. UI prompt: "Inspect Anchor [E]" (anchor glows yellow, decay particles visible)
5. Player inspects: Stability 70%, Repair Cost 2 Echoes (player has 2-3)
6. Seris dialogue: "If you repair it, the Echo Dash ability unlocks. You'll need it for the grove."
7. Player chooses to repair (recommended path):
   - Spend 2 Echoes (0-1 Echo remaining)
   - Weaving animation plays (5-second channel)
   - Stability increases to 100% (glow changes from yellow to bright blue)
   - Particle burst, SFX chord (C major)
   - Tutorial text: "New Ability Unlocked: Echo Dash (Shift + Direction)"
8. Player exits inspection, continues to Z1 Echo Grove transition

**Echo Cost**: -2 Echoes (unlock Echo Dash ability)

### C. Z1: Echo Grove

**Anchor 1**: `cover_anchor_northwest`
- **Position**: (-6, 0.5, 3) - Northwest quadrant, ground level
- **Initial Stability**: 0.0 (0%) - Broken (no glow, reality tear)
- **Repair Cost**: 1 Echo (cheap tactical upgrade)
- **Purpose**: Deploy barricade cover (3m × 1m × 2m wall), blocks enemy line of sight
- **Narrative**: Seris calls out: "Anchor northwest—repair it for cover!"

**Anchor 2**: `cover_anchor_southeast`
- **Position**: (8, 0.5, -5) - Southeast quadrant, ground level
- **Initial Stability**: 0.0 (0%) - Broken (no glow, reality tear)
- **Repair Cost**: 1 Echo (cheap tactical upgrade)
- **Purpose**: Deploy barricade cover (3m × 1m × 2m wall), blocks enemy line of sight
- **Narrative**: Seris calls out: "Southeast anchor—use it for flanking cover!"

**Combat Flow**:
1. Player enters Echo Grove from Z2 (spawn at southwest corner)
2. Trigger "combat_spawn" fires → 4 Rift Stalkers + 1 Echo-bound Sentinel spawn
3. Seris dialogue (existing dialogue_intro.toml, node n6): "Crystal-thread grove ahead. Rift Stalkers love that cover; Sentinels will shell us if we stay still."
4. Player fights enemies:
   - Option A: Repair northwest anchor (1 Echo) → Deploy barricade → Use cover for ranged attacks
   - Option B: Repair southeast anchor (1 Echo) → Deploy barricade → Use cover for flanking
   - Option C: Skip anchors, use Echo Dash (1 Echo per use) for mobility-based combat
5. Enemies defeated:
   - Rift Stalkers drop 4 Echoes total (1 per enemy)
   - Echo-bound Sentinel drops 2 Echoes
6. Trigger "combat_complete" fires → Award victory, transition to Z3 Loom Crossroads (Week 2)

**Echo Balance**:
- Player enters with 0-1 Echo (spent 2 on vista anchor)
- Player earns 6 Echoes from combat
- Player can spend 0-2 Echoes on barricades (tactical choice)
- Player reserves 3-4 Echoes for Echo Dash mobility (dodge attacks)
- Net gain: +4-6 Echoes (sufficient for Week 2 zones)

---

## Section 4: Technical Implementation (Week 2)

### A. Anchor Component (ECS)

**Rust Struct** (astraweave-weaving crate):
```rust
pub struct Anchor {
    pub id: String,                  // "loomspire_central_anchor"
    pub position: Vec3,              // (0, 2, 0)
    pub stability: f32,              // 0.0-1.0 (0% to 100%)
    pub repair_cost: u32,            // Echo cost (1-5)
    pub decay_rate: f32,             // Per-second decay (-0.000167 = -1% per minute)
    pub last_repair_time: f32,       // Game time of last repair
    pub interaction_radius: f32,     // Proximity trigger (3.0m)
    pub ability_unlock: Option<String>, // Some("echo_dash") or None
}
```

**Systems**:
- `anchor_decay_system`: Ticks stability down based on decay_rate
- `anchor_proximity_system`: Detects player within interaction_radius, shows UI prompt
- `anchor_interaction_system`: Handles E key press, opens inspection UI
- `anchor_repair_system`: Deducts Echoes, plays VFX, increases stability

### B. Echo Currency Component

**Rust Struct** (astraweave-weaving crate):
```rust
pub struct EchoCurrency {
    pub amount: u32,                 // Current Echo count (0-99)
    pub transaction_log: Vec<Transaction>, // History for debugging
}

pub struct Transaction {
    pub delta: i32,                  // +2 (gain) or -2 (spend)
    pub source: String,              // "tutorial_reward", "rift_stalker_kill", "repair_anchor"
    pub timestamp: f32,              // Game time
}
```

**Systems**:
- `echo_pickup_system`: Detects player collision with Echo Shard pickups, grants Echoes
- `echo_transaction_system`: Handles Echo spending (anchor repairs, Echo Dash ability)
- `echo_hud_system`: Updates HUD display (top-right corner count)

### C. VFX & SFX Integration

**Visual Effects** (astraweave-render crate):
- Anchor glow: Emissive material (color changes based on stability state)
- Decay particles: Particle system (spawn rate increases as stability decreases)
- Repair animation: Thread particle system (5-second duration, wraps around anchor)
- Reality tears: Distortion shader (UV offset based on noise texture)

**Audio Effects** (astraweave-audio crate):
- Anchor hum: Looping audio (pitch modulated by stability, 440 Hz @ 100%)
- Repair SFX: C major chord (C4, E4, G4, synthesized or recorded)
- Echo pickup: Crystalline chime (short, bright)
- Insufficient Echoes: Error buzzer (harsh, brief)

### D. UI Implementation

**Inspection UI** (astraweave-ui crate, egui or custom):
- Modal dialog: Center screen, semi-transparent background (darken game view)
- Anchor model: 3D preview (rotate slowly, show glow/particles)
- Stability meter: Horizontal bar (200×20 pixels, gradient fill)
- Repair button: Large button ("Repair [2 Echoes]"), disabled if insufficient currency
- Cancel button: Small button ("Cancel [ESC]"), returns to gameplay

**HUD Integration**:
- Echo count: Top-right corner (icon + number)
- Interaction prompt: Bottom-center ("Inspect Anchor [E]"), fades in/out based on proximity
- Transaction feedback: Center screen, floats upward ("+1 Echo", "-2 Echoes")

---

## Section 5: Narrative Integration

### A. Dialogue Connections

**Z0 Loomspire Sanctum**:
- Dialogue "intro_awakening" (existing dialogue_intro.toml, node n0-n5)
- Key line (n3a): "Focus on the loom nodes, project the thread, hold until the stability crest lights. Release early and the span snaps back."
- **Integration**: Line references anchor repair mechanic (5-second channeling = "hold until stability crest lights")

**Z2 Fractured Cliffs**:
- Dialogue "anchor_lore" (existing dialogue_intro.toml, should be added as new node)
- Key line: "Your gift—the weaving—it can repair them. Temporarily."
- **Integration**: Teaches player that repairs are temporary (decay continues over time)

**Z1 Echo Grove**:
- Dialogue "combat_barks" (existing dialogue_intro.toml, node n6a/n6b)
- Key line (n6a): "I'll mark its shield arcs and feed you openings."
- **Integration**: Seris calls out anchor positions during combat ("Northwest anchor—repair for cover!")

### B. Cinematic Integration

**Z0 cinematic: loom_awakening** (Day 7 TODO):
- Camera orbits loomspire_central_anchor (show perfect stability as baseline)
- Zoom into anchor glow (teach visual language: bright blue = stable)
- Companion NPC gestures toward anchor (establish inspection mechanic)

**Z2 cinematic: vista_pan** (Day 7 TODO):
- Camera pans to vista_tutorial_anchor (show yellow glow, decay particles)
- Zoom into anchor (contrast with Z0 perfect anchor: dim vs bright, particles vs clean)
- Companion NPC points toward Echo Grove (foreshadow combat zone)

### C. Tutorial Progression

**Phase 1: Introduction (Z0)**
- Objective: Learn anchor inspection UI
- Mechanic: Proximity trigger → Inspect prompt → View stability meter
- Outcome: Player understands anchor states (perfect = blue, stable = dim blue)

**Phase 2: First Repair (Z2)**
- Objective: Repair decaying anchor, unlock Echo Dash
- Mechanic: Spend 2 Echoes → Weaving animation → Stability increases
- Outcome: Player understands repair cost, ability unlocks, resource economy

**Phase 3: Tactical Anchors (Z1)**
- Objective: Use anchors for combat advantage
- Mechanic: Repair broken anchors (0% → 100%) → Deploy barricades → Use cover
- Outcome: Player understands tactical anchor use (not just narrative/visual)

---

## Section 6: Validation & Testing

### A. Acceptance Criteria

**Anchor System**:
- [x] **4 anchors defined** in zone descriptors (Z0: 1, Z1: 2, Z2: 1)
- [ ] **Proximity triggers functional** (3m radius, UI prompt appears)
- [ ] **Inspection UI functional** (stability meter, repair cost, button states)
- [ ] **Repair mechanic functional** (Echo deduction, VFX, stability increase)
- [ ] **Ability unlock functional** (Z2 vista anchor grants Echo Dash)

**Echo Currency**:
- [x] **Echo sources documented** (tutorial reward, enemy drops, hidden shard)
- [x] **Echo costs documented** (anchor repairs, Echo Dash ability)
- [ ] **HUD display functional** (top-right count, transaction feedback)
- [ ] **Transaction system functional** (gain/spend, insufficient funds check)

**Narrative Integration**:
- [x] **Dialogue references anchors** (intro_awakening, anchor_lore, vista_overview)
- [ ] **Cinematics show anchors** (loom_awakening orbits anchor, vista_pan zooms anchor)
- [x] **Tutorial progression designed** (inspect → repair → tactical use)

**Week 1 Status**: ✅ Design 100% complete, ⚠️ Implementation 0% (Week 2 TODO)

### B. Test Scenarios

**Test 1: Z0 Tutorial Anchor**
1. Start playthrough, spawn in Z0 Loomspire Sanctum
2. Walk toward center platform, expect UI prompt "Inspect Anchor [E]" at 3m
3. Press E, expect inspection UI (Stability: 100%, Repair Cost: 5 Echoes)
4. Expect "Not enough Echoes" message (player has 2-3, needs 5)
5. Press ESC to cancel, continue to Z2 transition

**Test 2: Z2 Vista Anchor Repair**
1. Reach Z2 vista platform (Z=200m)
2. Approach vista_tutorial_anchor, expect UI prompt at 3m
3. Press E, expect inspection UI (Stability: 70%, Repair Cost: 2 Echoes)
4. Click "Repair" button, expect:
   - Echo count decreases from 2 to 0
   - 5-second weaving animation plays
   - Stability increases from 70% to 100%
   - Glow changes from yellow to bright blue
   - Tutorial text: "New Ability Unlocked: Echo Dash"
5. Press ESC, verify Echo Dash available in ability bar

**Test 3: Z1 Combat Anchors**
1. Enter Z1 Echo Grove, trigger combat_spawn
2. Locate cover_anchor_northwest at (-6, 0.5, 3)
3. Repair anchor (1 Echo), expect barricade spawns (3m × 1m × 2m wall)
4. Use barricade for cover, expect Rift Stalker attacks blocked by wall
5. Locate cover_anchor_southeast at (8, 0.5, -5)
6. Repair anchor (1 Echo), expect second barricade spawns
7. Defeat all enemies, expect 6 Echoes total dropped

**Test 4: Echo Economy Balance**
1. Complete Z0 → Z2 → Z1 sequence
2. Track Echo transactions:
   - Start: 0 Echoes
   - Z0 reward: +2-3 Echoes (total: 2-3)
   - Z2 repair: -2 Echoes (total: 0-1)
   - Z1 combat: +6 Echoes (total: 6-7)
   - Z1 barricades: -0 to -2 Echoes (total: 4-7)
3. Verify player has 4-7 Echoes at end (sufficient for Week 2 zones)

---

## Section 7: Known Limitations & Future Work

### Week 1 Limitations

**Anchor Decay**:
- Passive decay not implemented (stability static after repair)
- Combat stress decay not implemented (killing enemies doesn't affect anchors)
- Limitation: Anchors stay at repaired stability forever (no time pressure)
- Impact: Reduces strategic depth (no "repair priority" decisions)

**Visual Polish**:
- Anchor models are placeholder cubes (1m × 1m × 1m grey boxes)
- Decay particles not implemented (greybox phase)
- Reality tears not implemented (distortion shader Week 2)
- Limitation: Stability states not visually distinct (all look the same)

**Audio**:
- Anchor hum audio not implemented (silent anchors)
- Repair SFX not implemented (no audio feedback)
- Limitation: Reduces immersion, no audio cues for stability state

### Week 2-3 Extensions

**Advanced Anchor Types**:
- **Gravity anchors**: Repair to invert gravity in area (Z3 boss arena mechanic)
- **Time anchors**: Repair to slow time in area (bullet-time combat)
- **Phase anchors**: Repair to reveal hidden paths (exploration reward)

**Dynamic Decay**:
- Implement passive decay (-1% per minute)
- Implement combat stress decay (-5% per nearby kill)
- Add "anchor collapse" event (0% stability → reality tear expands → zone failure)

**Multiplayer Anchors**:
- **Shared stability**: All players see same anchor state
- **Cooperative repair**: Multiple players can channel simultaneously (faster repair)
- **Sabotage mechanic**: Enemy players can damage anchors (PvP zones)

**Anchor Crafting**:
- Player can craft portable anchors (10 Echoes + rare materials)
- Place anchor anywhere (cooldown: 5 minutes)
- Used for custom safe zones, speedrun routing, creative mode

---

## Appendices

### A. Zone Descriptor Anchor References

**Z0_loomspire_sanctum.ron** (lines 33-38):
```ron
anchors: [
    (
        id: "loomspire_central_anchor",
        pos: (0.0, 2.0, 0.0),
        stability: 1.0,
        repair_cost: 5,
    ),
],
```

**Z1_echo_grove.ron** (lines 40-51):
```ron
anchors: [
    (
        id: "cover_anchor_northwest",
        pos: (-6.0, 0.5, 3.0),
        stability: 0.0,
        repair_cost: 1,
    ),
    (
        id: "cover_anchor_southeast",
        pos: (8.0, 0.5, -5.0),
        stability: 0.0,
        repair_cost: 1,
    ),
],
```

**Z2_fractured_cliffs.ron** (lines 36-42):
```ron
anchors: [
    (
        id: "vista_tutorial_anchor",
        pos: (0.0, 11.0, 200.0),
        stability: 0.7,
        repair_cost: 2,
    ),
],
```

### B. Echo Economy Flowchart

```
START (0 Echoes)
    ↓
Z0: Tutorial Reward (+2-3 Echoes) → 2-3 Echoes
    ↓
Z0: Skip loomspire anchor (too expensive) → 2-3 Echoes
    ↓
Z2: Repair vista anchor (-2 Echoes) → 0-1 Echoes
    ↓
Z2: Echo Dash unlocked (ability) → 0-1 Echoes
    ↓
Z1: Rift Stalkers defeated (+4 Echoes) → 4-5 Echoes
    ↓
Z1: Echo-bound Sentinel defeated (+2 Echoes) → 6-7 Echoes
    ↓
Z1: Optional: Repair northwest barricade (-1 Echo) → 5-6 Echoes
    ↓
Z1: Optional: Repair southeast barricade (-1 Echo) → 4-5 Echoes
    ↓
Z1: Optional: Use Echo Dash (−1 Echo per use, 0-3 uses) → 1-7 Echoes
    ↓
END (4-7 Echoes remaining for Week 2)
```

**Optimal Path**: Repair vista anchor (unlock Echo Dash) → Repair 1 barricade (tactical cover) → Reserve 3-4 Echoes for mobility

### C. Stability State Visual Reference

**ASCII Art Representation** (Week 2 concept art reference):

```
PERFECT (1.0 / 100%):
    ╔═════════════╗
    ║  ▓▓▓▓▓▓▓▓▓  ║  ← Bright blue glow (emissive 2.0)
    ║  ▓ Anchor ▓  ║
    ║  ▓▓▓▓▓▓▓▓▓  ║
    ╚═════════════╝
    (No particles, stable hum 440 Hz)

STABLE (0.7 / 70%):
    ╔═════════════╗
    ║  ▒▒▒▒▒▒▒▒▒  ║  ← Dim blue glow (emissive 1.0)
    ║  ▒ Anchor ▒  ║  ← Few decay particles (5-10 visible)
    ║  ▒▒▒▒▒▒▒▒▒  ║
    ╚═════════════╝
    (Flickering hum 440 Hz + static)

UNSTABLE (0.4 / 40%):
    ╔═════════════╗
    ║  ░░░░░░░░░  ║  ← Yellow glow (emissive 0.5)
    ║  ░ Anchor ░  ║  ← Many decay particles (20-30 visible)
    ║  ░░░░░░░░░  ║
    ╚═════════════╝
    (Distorted hum 400-480 Hz modulated)

CRITICAL (0.1 / 10%):
    ╔═════════════╗
    ║  ████████  ║  ← Red glow (emissive 0.2)
    ║  █ Anchor █  ║  ← Dense particle storm (50+ visible)
    ║  ████████  ║  ← Reality tears (distortion shader)
    ╚═════════════╝
    (Harsh static 200-600 Hz chaotic)

BROKEN (0.0 / 0%):
    ╔═════════════╗
    ║             ║  ← No glow (emissive 0.0)
    ║   [VOID]    ║  ← Reality tear vortex (black hole effect)
    ║             ║
    ╚═════════════╝
    (Silence, eerie absence of sound)
```

---

**Document Version**: 1.0  
**Last Updated**: November 8, 2025  
**Implementation Status**: ✅ Design Complete, ⚠️ Week 2 TODO  
**Next Milestone**: Day 7 Cinematics (anchor visual references for camera orbits)

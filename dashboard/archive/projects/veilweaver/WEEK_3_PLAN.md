# Week 3 Plan: Veilweaver Gameplay Integration

**Date**: November 8, 2025  
**Duration**: 8-12 hours (estimated)  
**Dependencies**: Week 2 Complete (Anchor System ✅)  
**Objective**: Integrate anchor system into playable Veilweaver demo with enemy AI, combat, and basic level

---

## Executive Summary

Week 3 will transform the anchor system from isolated components into a **playable game loop**:
- **Player** explores greybox level
- **Discovers** unstable anchors (proximity detection)
- **Fights** enemies disrupting anchors (combat)
- **Repairs** anchors (currency + UI)
- **Unlocks** abilities (progression)
- **Completes** objectives (quest system)

### Success Criteria
- ✅ Playable 5-10 minute demo loop
- ✅ 3+ zone anchors placed in level
- ✅ Enemy AI (patrol, attack anchors, engage player)
- ✅ Combat system (player attacks, damage, death)
- ✅ Quest system (objectives, completion tracking)
- ✅ 100+ new tests (270+ total)

---

## Week 3 Roadmap

### Day 1-2: Enemy AI & Combat (3-4h)
**Objective**: Create enemies that disrupt anchors and engage player

**Deliverables**:
1. **Enemy Component** (`enemy.rs`, ~300 lines, 20 tests)
   - Health tracking (0-100 HP)
   - AI state machine (Patrol, AttackAnchor, EngagePlayer, Flee)
   - Movement (speed, acceleration, pathfinding integration)
   - Attack behavior (melee range, damage, cooldown)

2. **Combat System** (`combat.rs`, ~400 lines, 25 tests)
   - Player attacks (primary, Echo Dash)
   - Enemy attacks (melee damage)
   - Damage calculation (base damage, modifiers)
   - Death handling (despawn, loot drops)
   - Combat events (damage dealt, enemy killed)

3. **Enemy Spawner** (`spawner.rs`, ~200 lines, 15 tests)
   - Wave system (1-5 enemies per wave)
   - Spawn points (fixed positions in level)
   - Spawn conditions (time-based, anchor state-based)
   - Difficulty scaling (more enemies if anchors broken)

**Integration**:
- Enemies patrol near broken anchors
- Enemies attack anchors (accelerate decay)
- Player kills enemies near anchors (earn Echoes)
- Killing enemies near anchors reduces spawn rate

**Tests**: 60 new tests (geometry collision, AI state transitions, damage calculation)

---

### Day 3-4: Quest System (2-3h)
**Objective**: Create quest objectives and completion tracking

**Deliverables**:
1. **Quest Component** (`quest.rs`, ~350 lines, 20 tests)
   - Quest data structure (ID, title, description, objectives)
   - Objective types (RepairAnchors, KillEnemies, CollectEchoes, ReachLocation)
   - Progress tracking (0/3 anchors repaired, 5/10 enemies killed)
   - Completion detection (all objectives met)
   - Rewards (Echoes, abilities, unlocks)

2. **Quest UI** (`quest_ui.rs`, ~250 lines, 15 tests)
   - Quest tracker (top-left corner)
   - Objective list (with progress bars)
   - Completion notification (popup)
   - Quest log (press Q to view all quests)

3. **Starter Quests** (data files)
   - **Tutorial Quest**: "Stabilize Reality" (repair 1 anchor, collect 10 Echoes)
   - **Combat Quest**: "Clear the Rift" (kill 5 enemies, repair 2 anchors)
   - **Exploration Quest**: "Survey the Loom" (discover all 3 zone anchors)

**Integration**:
- Quest triggers when entering zone
- Objectives update on anchor repair, enemy kill, Echo pickup
- Quest completion shows notification, grants rewards
- Quest log accessible via 'Q' key

**Tests**: 35 new tests (objective tracking, completion detection, rewards)

---

### Day 5: Level Integration (2-3h)
**Objective**: Place anchors, enemies, and spawn points in greybox level

**Deliverables**:
1. **Level Data** (`level_veilweaver.ron`, ~200 lines)
   - 3 zone anchors (positions, states, abilities)
     - **Zone 0 (Tutorial)**: Perfect anchor, unlocks Echo Dash, 10 Echoes to repair
     - **Zone 1 (Combat)**: Unstable anchor, 5 nearby enemies, unlocks Barricade Deploy, 50 Echoes
     - **Zone 2 (Vista)**: Critical anchor, no ability, 30 Echoes, scenic overlook
   - 5 enemy spawn points (positions, wave configs)
   - 10 Echo pickups (scattered around level, 5-10 Echoes each)
   - 3 quests (starter set)

2. **Level Loader** (`level_loader.rs`, ~150 lines, 10 tests)
   - Load `.ron` file from assets
   - Spawn anchors with correct states
   - Place enemy spawn points
   - Spawn Echo pickups
   - Initialize quests

3. **Camera System** (extend existing)
   - Third-person follow camera
   - Smooth movement (damping, no jitter)
   - World→screen transform for UI (repair progress bar)

**Integration**:
- Load level on game start
- Player spawns at Zone 0
- Anchors visible with VFX
- Enemies patrol near broken anchors
- Quests auto-start on proximity

**Tests**: 10 new tests (level loading, spawn validation)

---

### Day 6-7: Polish & Testing (2-3h)
**Objective**: Bug fixes, performance validation, playability testing

**Deliverables**:
1. **Performance Validation**
   - 60 FPS @ 10 enemies + 3 anchors + player
   - Particle system cap enforcement (500 particles)
   - Audio channel management (8 concurrent sounds)

2. **Bug Fixes**
   - Edge cases (enemy stuck, anchor repair interrupted, quest not completing)
   - UI overlap (modal + HUD + notification + quest tracker)
   - Audio mixing (hum + combat + UI sounds balanced)

3. **Playability Testing**
   - 5-10 minute gameplay loop functional
   - Tutorial quest completable
   - Combat feels responsive
   - Anchor repair satisfying (VFX + audio + UI feedback)

4. **Week 3 Completion Report** (`WEEK_3_COMPLETE.md`)
   - Cumulative metrics (270+ tests, 7,000+ LOC)
   - Playability validation results
   - Performance benchmarks
   - Known issues + next steps

**Tests**: Review existing tests, fix any failures, add edge case coverage

---

## Detailed Component Specifications

### 1. Enemy Component (`enemy.rs`)

**Purpose**: AI-driven enemies that disrupt anchors and attack player

**Data Structure**:
```rust
pub struct Enemy {
    pub health: f32,              // 0-100 HP
    pub max_health: f32,          // 100 HP
    pub speed: f32,               // 3.0 units/sec
    pub state: EnemyState,        // AI state
    pub target_anchor_id: Option<usize>, // Which anchor to attack
    pub attack_damage: f32,       // 10 HP per hit
    pub attack_cooldown: f32,     // 1.0s between attacks
    pub attack_timer: f32,        // Current cooldown
    pub patrol_center: Vec3,      // Patrol origin
    pub patrol_radius: f32,       // 5.0 units
    pub aggro_range: f32,         // 10.0 units (engage player)
    pub flee_health: f32,         // 20 HP (flee when below)
}

pub enum EnemyState {
    Patrol,           // Walk randomly near patrol_center
    AttackAnchor,     // Move toward + attack target anchor
    EngagePlayer,     // Move toward + attack player
    Flee,             // Run away from player
    Dead,             // Despawn after 2s
}
```

**Key Methods**:
- `new(position, patrol_radius) -> Self`
- `update(delta_time, player_pos, anchors) -> EnemyBehavior`
- `take_damage(amount) -> bool` (returns true if killed)
- `can_attack() -> bool` (cooldown check)
- `attack() -> f32` (returns damage, resets cooldown)

**AI Logic** (per frame):
1. If health < flee_health → Flee state
2. If player within aggro_range → EngagePlayer state
3. If assigned anchor broken → AttackAnchor state
4. Else → Patrol state

**Tests** (20):
- Enemy creation
- Health tracking (take damage, death)
- AI state transitions (patrol → engage, engage → flee)
- Attack cooldown
- Damage calculation
- Patrol movement (stay within radius)
- Aggro detection (player in range)
- Anchor targeting (closest broken anchor)

---

### 2. Combat System (`combat.rs`)

**Purpose**: Handle damage, death, combat events

**Data Structure**:
```rust
pub struct CombatSystem {
    pub player_health: f32,       // 100 HP
    pub player_max_health: f32,   // 100 HP
    pub player_attack_damage: f32, // 20 HP
    pub echo_dash_damage: f32,    // 50 HP (special)
    pub combat_events: Vec<CombatEvent>,
}

pub enum CombatEvent {
    PlayerDamaged { amount: f32 },
    EnemyDamaged { enemy_id: usize, amount: f32 },
    EnemyKilled { enemy_id: usize, position: Vec3 },
    PlayerKilled,
}
```

**Key Methods**:
- `player_attack(enemy_id, enemies) -> Option<CombatEvent>`
- `echo_dash_attack(position, enemies) -> Vec<CombatEvent>` (AoE)
- `enemy_attack(player) -> Option<CombatEvent>`
- `apply_damage(target, amount) -> bool` (returns true if killed)
- `poll_events() -> Vec<CombatEvent>` (drain event queue)

**Integration**:
- Player input (left click) → `player_attack(closest_enemy)`
- Echo Dash ability → `echo_dash_attack(player_pos + dash_direction)`
- Enemy AI (EngagePlayer state) → `enemy_attack(player)`
- Combat events → spawn damage numbers, play sounds, award Echoes

**Tests** (25):
- Player attack (single target)
- Echo Dash (AoE, 3.0 unit radius)
- Enemy attack (player damage)
- Death detection (player, enemy)
- Combat events (creation, polling)
- Damage calculation (base + modifiers)
- Critical hits (future: 20% chance, 1.5× damage)

---

### 3. Quest System (`quest.rs`)

**Purpose**: Track objectives, reward completion

**Data Structure**:
```rust
pub struct Quest {
    pub id: String,               // "tutorial_stabilize"
    pub title: String,            // "Stabilize Reality"
    pub description: String,      // "Repair the first anchor..."
    pub objectives: Vec<QuestObjective>,
    pub rewards: QuestRewards,
    pub status: QuestStatus,      // NotStarted, InProgress, Completed
}

pub struct QuestObjective {
    pub kind: ObjectiveKind,
    pub target: u32,              // 3 anchors, 10 enemies, etc.
    pub current: u32,             // 1/3, 5/10, etc.
}

pub enum ObjectiveKind {
    RepairAnchors,
    KillEnemies,
    CollectEchoes,
    ReachLocation { position: Vec3, radius: f32 },
}

pub struct QuestRewards {
    pub echoes: u32,              // 50 Echoes
    pub ability: Option<AbilityType>, // EchoDash
    pub unlock: Option<String>,   // "barricade_blueprint"
}
```

**Key Methods**:
- `new(id, title, description, objectives, rewards) -> Self`
- `start(&mut self)` (NotStarted → InProgress)
- `update_objective(kind, amount)` (increment progress)
- `check_completion() -> bool` (all objectives met?)
- `complete(&mut self) -> QuestRewards` (InProgress → Completed)

**Integration**:
- Anchor repaired → `quest.update_objective(RepairAnchors, 1)`
- Enemy killed → `quest.update_objective(KillEnemies, 1)`
- Echo picked up → `quest.update_objective(CollectEchoes, amount)`
- Quest completed → show notification, grant rewards, update UI

**Tests** (20):
- Quest creation
- Objective tracking (repair, kill, collect, reach)
- Completion detection
- Rewards (Echoes, abilities)
- Multiple quests (concurrent tracking)
- Quest status transitions

---

### 4. Quest UI (`quest_ui.rs`)

**Purpose**: Display quest tracker and completion notifications

**UI Layout**:
```
┌─────────────────────────────────┐ (Top-left, 250px wide)
│ ACTIVE QUESTS                   │
│ ─────────────────────────────── │
│ [!] Stabilize Reality           │
│   ⚬ Repair anchor (0/1)         │
│   ⚬ Collect Echoes (5/10)       │
│ ─────────────────────────────── │
│ Press Q for Quest Log           │
└─────────────────────────────────┘

(Quest Completion Notification - center screen, 4s)
┌─────────────────────────────────┐
│       ✓ QUEST COMPLETE!         │
│                                 │
│    Stabilize Reality            │
│                                 │
│ Rewards:                        │
│   +50 Echoes                    │
│   Echo Dash Unlocked            │
└─────────────────────────────────┘
```

**Key Methods**:
- `new() -> Self`
- `add_quest(quest: &Quest)` (add to tracker)
- `remove_quest(quest_id: &str)` (remove from tracker)
- `update(quests: &[Quest])` (refresh progress)
- `show_completion(quest: &Quest)` (4s notification)
- `render(&egui::Context)` (draw UI)

**Tests** (15):
- Quest tracker display
- Objective progress bars
- Completion notification
- Quest log (Q key)
- Multiple quests (3+ concurrent)

---

## Performance Targets

### CPU Budget (@ 60 FPS, 16.67ms frame)
| System | Budget | Expected |
|--------|--------|----------|
| Enemy AI | 2.0 ms | 1.5 ms (10 enemies @ 150 µs each) |
| Combat | 0.5 ms | 0.3 ms (collision checks, damage calc) |
| Quests | 0.2 ms | 0.1 ms (objective tracking) |
| Anchor System | 1.0 ms | 0.8 ms (decay, audio, particles) |
| UI | 0.5 ms | 0.4 ms (modal + HUD + quest + notification) |
| **Total Game Logic** | **4.2 ms** | **3.1 ms (18.6% of 16.67ms)** |

**Remaining Budget**: 12.57 ms for rendering, physics, input

### Memory Footprint
```
10 enemies @ 128 bytes each        = 1.3 KB
3 anchors @ 400 bytes each         = 1.2 KB
1 combat system @ 256 bytes        = 0.3 KB
3 quests @ 512 bytes each          = 1.5 KB
Quest UI @ 128 bytes               = 0.1 KB
─────────────────────────────────────────
Total:                               4.4 KB (negligible)
```

### Test Coverage Target
- **Week 2 Baseline**: 169 tests
- **Week 3 New Tests**: 100+ tests
- **Week 3 Total**: 270+ tests (100% pass rate)

---

## Risk Assessment

### HIGH RISK
1. **Enemy Pathfinding** (NavMesh integration complex)
   - **Mitigation**: Use simple direct movement for Week 3, defer advanced pathfinding to Week 4
   - **Fallback**: Enemies move toward target in straight line, stop if stuck

2. **Combat Feel** (attacks may feel unresponsive)
   - **Mitigation**: Add hit feedback (screen shake, slow-mo, damage numbers)
   - **Fallback**: Start with simple damage numbers, iterate based on playtesting

### MEDIUM RISK
3. **Quest System Complexity** (many edge cases)
   - **Mitigation**: Start with 3 simple quests, expand in Week 4
   - **Fallback**: Tutorial quest only (repair 1 anchor)

4. **Performance** (10 enemies + 3 anchors + particles)
   - **Mitigation**: Profile early, optimize hotspots
   - **Fallback**: Reduce enemy count to 5, cap particles at 300

### LOW RISK
5. **Level Integration** (asset loading errors)
   - **Mitigation**: Use `.ron` format (robust error messages)
   - **Fallback**: Hardcode initial state in code

6. **UI Overlap** (modal + HUD + quest + notification)
   - **Mitigation**: Z-ordering, careful layout positioning
   - **Fallback**: Disable quest tracker when modal open

---

## Dependencies & Prerequisites

### Week 2 Complete ✅
- [x] Anchor system (169 tests passing)
- [x] VFX shader, particles, audio
- [x] UI system (modal, HUD, notification, progress bar)

### External Crates Required
- `rand` (already in Cargo.toml) - Random patrol movement
- `glam` (already in use) - Vector math
- `egui` (already in use) - Quest UI
- `serde`, `ron` (already in use) - Level data serialization

### Assets Required
- **Audio** (6 files from Week 2 specs) - Create during Week 3
- **Enemy Model** (placeholder cube) - Use greybox geometry
- **Combat Sounds** (attack, hit, death) - Simple impact sounds

---

## Week 3 Success Metrics

### Functionality (100%)
- [x] Player can move, attack, repair anchors
- [x] Enemies patrol, attack anchors, engage player
- [x] Combat system functional (damage, death)
- [x] Quest system tracks objectives
- [x] 3 zone anchors placed in level
- [x] 5-10 minute gameplay loop complete

### Quality (100%)
- [x] 270+ tests passing (100% pass rate)
- [x] Zero compilation warnings
- [x] 60 FPS @ 10 enemies + 3 anchors
- [x] UI responsive (no overlap, clear feedback)

### Documentation (100%)
- [x] Week 3 completion report (~15,000 words)
- [x] API documentation (enemy, combat, quest)
- [x] Level data format documented
- [x] Playability test results

---

## Next Steps (After Week 3)

### Week 4: Advanced Features (8-12h)
- Advanced enemy AI (formations, tactics)
- Ability system (Echo Dash, Barricade Deploy)
- More quest types (escort, defend, puzzle)
- Audio assets (create 6 sounds from Week 2 specs)

### Week 5: Polish & Content (8-12h)
- Visual polish (post-processing, lighting)
- More enemies (3 types: melee, ranged, heavy)
- More quests (10 total)
- Boss fight (Zone 3)

### Week 6: Multiplayer Prep (12-16h)
- Network synchronization (anchor states, enemy positions)
- Replication (client prediction, server reconciliation)
- Lobby system (matchmaking, squad formation)

---

## Validation Checklist

Week 3 will be considered complete when:
- [ ] 270+ tests passing (100%)
- [ ] Zero compilation warnings
- [ ] 5-10 minute gameplay loop functional
- [ ] Tutorial quest completable
- [ ] Combat feels responsive (playtesting)
- [ ] 60 FPS @ 10 enemies + 3 anchors
- [ ] Week 3 completion report written

---

## Conclusion

Week 3 will transform the anchor system into a **playable game demo** with:
- ✅ Enemy AI (patrol, attack, engage)
- ✅ Combat system (player attacks, damage, death)
- ✅ Quest system (objectives, completion, rewards)
- ✅ Level integration (3 anchors, 5 spawn points, 10 Echo pickups)
- ✅ 5-10 minute gameplay loop

**Estimated Time**: 8-12 hours (maintaining 70-75% under budget efficiency)

**Next**: Day 1 - Create Enemy AI component. Ready to proceed! 🚀

---

**Plan Version**: 1.0  
**Author**: AI Copilot (100% AI-generated)  
**Project**: AstraWeave AI-Native Gaming Engine  
**License**: MIT  
**Status**: Ready for Week 3 Day 1

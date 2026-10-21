# Week 1 Days 3-7: Greybox & Narrative Implementation Plan

**Status**: 📋 PLANNED  
**Priority**: P0 - Foundation (follows weaving P0 blocker resolution)  
**Dependencies**: Weaving coverage complete ✅ (94.26%, 64 tests)  
**Timeline**: 5 days (Days 3-7)  
**Estimated Effort**: 20-30 hours  

---

## Context

**Prerequisite Work**:
- ✅ **Days 1-2 COMPLETE**: Weaving test sprint (90.66% → 94.26%, +43 tests, 4 hours)
- ✅ **P0 Blocker RESOLVED**: Foundation audit upgraded to A+ status
- ✅ **Master reports updated**: All documentation reflects weaving resolution

**Current State**:
- **Engine Status**: Production-ready (12,700+ agents @ 60 FPS, 100% determinism)
- **Rendering**: GPU skinning ✅, mesh optimization ✅, shadow maps (CSM) ✅, post-FX ✅
- **AI Systems**: 6 AI modes functional (Classical, BehaviorTree, Utility, LLM, Hybrid, Ensemble)
- **Asset Pipeline**: Partial (polyhaven, quaternius manifests exist, but NO greybox workflow documented)
- **Narrative System**: Dialogue TOML exists (`dialogue_intro.toml`), but NO integration with zones

**Gap Analysis** (from Foundation Audit):
1. ⚠️ **Asset Pipeline**: Undefined greybox mesh workflow (P1 High priority)
2. ⚠️ **Greybox Geometry**: NO placeholder meshes for Veilweaver zones exist
3. ⚠️ **Narrative Integration**: Dialogue TOML nodes not wired to trigger volumes
4. ⚠️ **Scene Descriptors**: NO `.ron` scene files for Veilweaver zones

**Objective**: Transform Veilweaver from "design docs" to "walkable greybox" with functional narrative flow.

---

## Week 1 Days 3-7 Roadmap

### Day 3: Asset Pipeline Setup (1 day, 4-6 hours)

**Goal**: Define and document the greybox asset workflow for Veilweaver.

**Tasks**:

1. **Greybox Mesh Format Decision** (1 hour)
   - Research: GLTF 2.0 vs FBX for AstraWeave
   - Decision criteria: wgpu compatibility, animation support, toolchain availability
   - Document: Create `GREYBOX_ASSET_WORKFLOW.md` with chosen format
   - Recommendation: GLTF 2.0 (open standard, wgpu native support, Blender export)

2. **Scene Descriptor Template** (1-2 hours)
   - Create `.ron` template for Veilweaver zones
   - Fields: `zone_id`, `mesh_path`, `spawn_points`, `triggers`, `anchors`, `navigation_mesh`
   - Example template for Z0 (Loomspire Sanctum):
     ```ron
     (
         zone_id: "Z0_loomspire_sanctum",
         mesh_path: "assets/models/greybox/loomspire_sanctum_greybox.glb",
         spawn_points: [
             (pos: (0.0, 0.0, 0.0), facing: (0.0, 0.0, 1.0)),
         ],
         triggers: [
             (name: "tutorial_start", bounds: ((−10.0, −10.0), (10.0, 10.0)), action: "StartTutorial"),
         ],
         anchors: [
             (id: "loomspire_central_anchor", pos: (0.0, 1.0, 0.0), stability: 1.0),
         ],
         navigation_mesh: "assets/navmeshes/loomspire_sanctum_navmesh.ron",
     )
     ```
   - Save as: `docs/projects/veilweaver/templates/zone_descriptor_template.ron`

3. **Material & Texture Conventions** (30 minutes)
   - Define placeholder material naming: `greybox_floor`, `greybox_wall`, `greybox_obstacle`
   - Document texture resolution: 512×512 for greybox phase
   - Create minimal material TOML (if not using existing `materials.toml`)

4. **Asset Import Workflow** (1-2 hours)
   - Document Blender → GLTF → AstraWeave pipeline
   - Create checklist for greybox mesh creation:
     * [ ] Model dimensions match spec (e.g., 50m diameter for Loomspire)
     * [ ] Origin at world center (0, 0, 0)
     * [ ] Collision mesh tagged with `_collision` suffix
     * [ ] Export as GLTF 2.0 (`.glb` embedded textures OR `.gltf` + `.bin`)
     * [ ] Validate with `veilweaver_slice_runtime` (if it exists)
   - Save as: `docs/projects/veilweaver/GREYBOX_ASSET_WORKFLOW.md`

5. **Test Mesh Validation** (1 hour)
   - Create simple 10×10m cube mesh in Blender
   - Export as `test_greybox.glb`
   - Load in AstraWeave renderer (or create minimal example if needed)
   - Verify: Mesh loads, materials apply, no crashes
   - Document any issues for troubleshooting

**Deliverables**:
- [ ] `GREYBOX_ASSET_WORKFLOW.md` (format decision, workflow steps, checklist)
- [ ] `templates/zone_descriptor_template.ron` (reusable `.ron` template)
- [ ] `test_greybox.glb` (validation mesh)
- [ ] Pipeline validation report (test results, issues)

**Success Criteria**:
- ✅ Asset format chosen (GLTF 2.0 recommended)
- ✅ `.ron` template created with all required fields
- ✅ Workflow documented with clear steps
- ✅ Test mesh loads in renderer without errors

---

### Days 4-5: Greybox Mesh Creation (2 days, 8-12 hours)

**Goal**: Create walkable placeholder meshes for 3 priority Veilweaver zones.

**Zones to Create** (from LOOMSPIRE_GREYBOX_SPEC.md):
1. **Z0: Loomspire Sanctum** (central hub, 50m diameter)
2. **Z1: Echo Grove** (combat zone, 100m × 100m)
3. **Z2: Fractured Cliffs** (narrative zone, linear path ~200m)

**Tools**:
- Blender 4.3+ (GLTF export)
- OR: Procedural mesh generation in Rust (if Blender unavailable)

---

#### Day 4 Morning: Loomspire Sanctum (3-4 hours)

**Reference**: `docs/projects/veilweaver/design-docs/LOOMSPIRE_GREYBOX_SPEC.md`

**Tasks**:

1. **Read Specification** (15 minutes)
   - Read LOOMSPIRE_GREYBOX_SPEC.md
   - Note: 50m diameter, 3 tiers (ground, mezzanine, observation), weaving chamber centerpiece
   - Dimensions: Ground floor 50m diameter, mezzanine 30m, observation 15m

2. **Blender Modeling** (2-3 hours)
   - Create 3-tier circular structure:
     * Ground floor: Cylinder 50m diameter × 5m height
     * Mezzanine: Cylinder 30m diameter × 3m height (Y offset +5m)
     * Observation: Cylinder 15m diameter × 2m height (Y offset +8m)
   - Add weaving chamber: Cube 10m × 10m × 10m at center (Y offset +1m)
   - Add stairs/ramps between tiers (simple sloped cubes)
   - Add collision mesh (simplified version of visible mesh)
   - Materials: Assign `greybox_floor`, `greybox_wall`

3. **Export & Validate** (30 minutes)
   - File → Export → GLTF 2.0 (.glb)
   - Settings: Embedded textures, Y up, scale 1.0
   - Save as: `assets/models/greybox/loomspire_sanctum_greybox.glb`
   - Load in renderer, verify dimensions (50m diameter visible)

4. **Scene Descriptor** (30 minutes)
   - Create `assets/cells/Z0_loomspire_sanctum.ron`
   - Fill template with Loomspire data:
     * Spawn point: (0, 0, −20) facing (0, 0, 1)
     * Tutorial trigger: Bounds ((−5, −5), (5, 5)), action "StartWeavingTutorial"
     * Central anchor: (0, 1, 0), stability 1.0
   - Validate: RON parses correctly (`cargo check` or manual parse test)

**Deliverables**:
- [ ] `loomspire_sanctum_greybox.glb` (3-tier structure, 50m diameter)
- [ ] `Z0_loomspire_sanctum.ron` (scene descriptor with spawn, triggers, anchors)
- [ ] Screenshot or viewport render (for documentation)

---

#### Day 4 Afternoon: Echo Grove (3-4 hours)

**Reference**: Combat zone requirements (from QUICK_ACTION_CHECKLIST.md)

**Tasks**:

1. **Design Layout** (30 minutes)
   - 100m × 100m forest clearing (flat terrain)
   - 5-7 cover positions:
     * Large rocks (3m × 3m × 2m cubes)
     * Fallen logs (10m × 1m × 1m boxes)
     * Tree stumps (2m × 2m × 1m cylinders)
   - Open sightlines: Ensure player can see 40-60m in multiple directions

2. **Blender Modeling** (2-3 hours)
   - Create ground plane: 100m × 100m × 1m
   - Add cover elements (5-7 objects):
     * 3 large rocks at (−30, 0, 20), (30, 0, −20), (−15, 0, −30)
     * 2 fallen logs at (0, 0, 30), (20, 0, 0)
     * 2 tree stumps at (−40, 0, −10), (40, 0, 10)
   - Add collision meshes for each cover element
   - Materials: `greybox_floor` (ground), `greybox_obstacle` (cover)

3. **Export & Validate** (30 minutes)
   - Export as: `assets/models/greybox/echo_grove_greybox.glb`
   - Load in renderer, verify:
     * Ground plane 100m × 100m visible
     * Cover elements positioned correctly
     * Player can navigate between cover

4. **Scene Descriptor** (30 minutes)
   - Create `assets/cells/Z1_echo_grove.ron`
   - Spawn point: (−40, 0, −40) facing (1, 0, 1) (southwest corner)
   - Combat trigger: Bounds ((−50, −50), (50, 50)), action "StartCombatEncounter"
   - Enemy spawn points: 5 positions around perimeter
   - Navmesh placeholder: `"assets/navmeshes/echo_grove_navmesh.ron"` (defer generation)

**Deliverables**:
- [ ] `echo_grove_greybox.glb` (100m × 100m with 5-7 cover elements)
- [ ] `Z1_echo_grove.ron` (scene descriptor with combat triggers, spawn points)
- [ ] Line-of-sight validation notes (cover effectiveness)

---

#### Day 5 Morning: Fractured Cliffs (3-4 hours)

**Reference**: Narrative zone requirements

**Tasks**:

1. **Design Path** (30 minutes)
   - Linear path ~200m long (player progression: start → vista → end)
   - 3 dialogue trigger points:
     * Point 1: (0, 0, 0) - Intro dialogue (companion awakening)
     * Point 2: (0, 0, 100) - Mid-path dialogue (lore reveal)
     * Point 3: (0, 10, 200) - Vista overlook (narrative beat)
   - Path width: 5-10m (prevent player getting lost)
   - Elevation change: +10m at vista (dramatic view)

2. **Blender Modeling** (2-3 hours)
   - Create cliff terrain:
     * Base path: 200m × 10m × 1m (flat sections)
     * Slope section: 50m × 10m (30° incline to vista)
     * Vista platform: 15m × 15m × 1m at Y=+10m
   - Add cliff walls: Cubes 200m long × 20m high (left/right of path)
   - Add hazard markers: Red cubes at cliff edges (3m × 1m × 1m)
   - Materials: `greybox_floor` (path), `greybox_wall` (cliffs)

3. **Export & Validate** (30 minutes)
   - Export as: `assets/models/greybox/fractured_cliffs_greybox.glb`
   - Verify: Path 200m visible, vista elevated +10m, cliff walls prevent falling

4. **Scene Descriptor** (30 minutes)
   - Create `assets/cells/Z2_fractured_cliffs.ron`
   - Spawn point: (0, 0, −10) facing (0, 0, 1)
   - Dialogue triggers:
     * Trigger 1: (0, 0, 0), action "TriggerDialogue_Intro"
     * Trigger 2: (0, 0, 100), action "TriggerDialogue_Lore"
     * Trigger 3: (0, 10, 200), action "TriggerDialogue_Vista"
   - Navmesh placeholder

**Deliverables**:
- [ ] `fractured_cliffs_greybox.glb` (200m path with 3 dialogue points)
- [ ] `Z2_fractured_cliffs.ron` (scene descriptor with narrative triggers)
- [ ] Path validation notes (walkability, visibility)

---

#### Day 5 Afternoon: Validation & Refinement (2-3 hours)

**Tasks**:

1. **Mesh Validation** (1 hour)
   - Load all 3 greybox meshes in renderer
   - Check for issues:
     * Missing textures
     * Scale problems (too big/small)
     * Collision gaps (player falling through)
   - Fix any critical errors (re-export if needed)

2. **Scene Descriptor Validation** (1 hour)
   - Parse all 3 `.ron` files programmatically (or manually)
   - Verify fields match template:
     * `zone_id` strings correct
     * `mesh_path` points to existing files
     * `spawn_points` have valid coordinates
     * `triggers` have valid actions
   - Fix any parsing errors

3. **Integration Test** (1 hour)
   - Attempt to load Z0 (Loomspire) in `veilweaver_slice_runtime` (if it exists)
   - OR: Create minimal example `examples/greybox_viewer`:
     ```rust
     // Load scene descriptor
     // Spawn mesh from GLTF
     // Position camera at spawn point
     // Render 1 frame (validation only)
     ```
   - Document any runtime issues

**Deliverables**:
- [ ] Validation report (all 3 meshes load correctly)
- [ ] Bug fixes applied (if any)
- [ ] Integration test results

---

### Day 6: Scene Descriptors & Trigger Integration (1 day, 4-6 hours)

**Goal**: Wire dialogue TOML nodes to trigger volumes in greybox zones.

**Tasks**:

1. **Read Dialogue TOML** (30 minutes)
   - Read `assets/dialogue_intro.toml`
   - Identify nodes referenced in zone descriptors:
     * "TriggerDialogue_Intro" → Which dialogue node?
     * "TriggerDialogue_Lore" → Which node?
     * "TriggerDialogue_Vista" → Which node?
   - Document mapping in `NARRATIVE_TRIGGER_MAPPING.md`

2. **Expand Zone Descriptors** (2-3 hours)
   - Update `Z0_loomspire_sanctum.ron`:
     * Add `dialogue_nodes: ["intro_awakening", "tutorial_start"]`
     * Link tutorial trigger to `"tutorial_start"` node
   - Update `Z2_fractured_cliffs.ron`:
     * Add `dialogue_nodes: ["intro_awakening", "lore_reveal", "vista_beat"]`
     * Link each trigger to corresponding node
   - Validate: All dialogue node IDs exist in `dialogue_intro.toml`

3. **Anchor Integration** (1-2 hours)
   - Update `Z0_loomspire_sanctum.ron`:
     * Add `anchors` field with `loomspire_central_anchor`
     * Fields: `id`, `pos`, `stability`, `repair_cost`, `max_stability`
   - Document anchor-weaving linkage in `ANCHOR_INTEGRATION.md`:
     * How tutorial triggers anchor repair sequence
     * How anchor stability affects narrative flow

4. **Metadata Extraction** (1 hour)
   - Verify all `.ron` files parse correctly
   - Create validation script (PowerShell or Rust):
     ```powershell
     # Validate all zone descriptors
     Get-ChildItem assets/cells/*.ron | ForEach-Object {
         Write-Host "Validating $_..."
         # Parse RON (manual check or cargo test)
     }
     ```
   - Fix any parsing errors

**Deliverables**:
- [ ] Updated `.ron` files (3 zones with dialogue nodes linked)
- [ ] `NARRATIVE_TRIGGER_MAPPING.md` (trigger → dialogue node mapping)
- [ ] `ANCHOR_INTEGRATION.md` (anchor → weaving system linkage)
- [ ] Validation script (PowerShell or Rust)

**Success Criteria**:
- ✅ All dialogue node IDs exist in `dialogue_intro.toml`
- ✅ All triggers have valid action strings
- ✅ All `.ron` files parse without errors
- ✅ Anchor integration documented

---

### Day 7: Cinematics & Greybox Walkthrough (1 day, 4-6 hours)

**Goal**: Script cinematics A/B and validate complete greybox experience.

**Tasks**:

1. **Read Cinematics Spec** (30 minutes)
   - Search for `CINEMATICS_SPEC.md` or similar (may not exist)
   - If missing, define minimal requirements:
     * **Cinematic A: Loom Awakening** (30 seconds)
       - Camera: Slow pan around Loomspire central chamber
       - Voiceover: Intro narration (TTS or silent)
       - Trigger: On entering Z0 first time
     * **Cinematic B: Guided Approach** (15 seconds)
       - Camera: Follow companion to tutorial anchor
       - Action: Companion walks to anchor position
       - Trigger: After Cinematic A completes

2. **Script Cinematics** (2-3 hours)
   - Create `cinematics/loom_awakening.ron` (Cinematic A):
     ```ron
     (
         id: "loom_awakening",
         duration: 30.0,
         camera_track: [
             (time: 0.0, pos: (0, 5, −20), look_at: (0, 1, 0)),
             (time: 30.0, pos: (20, 5, 0), look_at: (0, 1, 0)), // 360° pan
         ],
         audio_track: "assets/audio/intro_narration.ogg", // Placeholder
         triggers_on_end: "start_tutorial",
     )
     ```
   - Create `cinematics/guided_approach.ron` (Cinematic B):
     ```ron
     (
         id: "guided_approach",
         duration: 15.0,
         camera_track: [
             (time: 0.0, pos: (−10, 2, −10), look_at: (0, 1, 0)),
             (time: 15.0, pos: (0, 2, 5), look_at: (0, 1, 0)), // Follow companion
         ],
         actor_actions: [
             (actor: "companion", action: "WalkTo", target: (0, 0, 0)), // Walk to anchor
         ],
         triggers_on_end: "enable_player_control",
     )
     ```
   - Validate: RON files parse correctly

3. **Integrate Dialogue TOML** (1 hour)
   - Update `Z0_loomspire_sanctum.ron`:
     * Add `cinematic_triggers: ["loom_awakening"]` (plays on zone enter)
   - Update `assets/dialogue_intro.toml` (if needed):
     * Ensure "tutorial_start" node follows cinematic sequence
     * Add dependencies: `requires_completed: ["loom_awakening"]`

4. **Greybox Walkthrough Validation** (1-2 hours)
   - **Manual Walkthrough** (if runtime exists):
     * Load Z0 (Loomspire Sanctum)
     * Verify: Spawn at correct position, cinematic A triggers
     * Walk to tutorial anchor, verify: Cinematic B triggers
     * Enter Z2 (Fractured Cliffs), verify: Dialogue triggers fire at correct positions
   - **Automated Validation** (if manual not possible):
     * Create test: `test_greybox_walkthrough_sequence`
     * Steps:
       1. Load Z0 scene descriptor
       2. Spawn player at spawn point
       3. Move player to tutorial trigger bounds
       4. Assert: Tutorial dialogue node triggered
     * Document: Any missing functionality for future implementation

5. **Documentation & Milestone** (1 hour)
   - Create `GREYBOX_WALKTHROUGH_REPORT.md`:
     * All 3 zones load successfully: ✅/❌
     * Dialogue triggers fire correctly: ✅/❌
     * Cinematics play (if implemented): ✅/❌/⚠️ (not implemented yet)
     * Known issues: List any bugs or missing features
   - Update QUICK_ACTION_CHECKLIST.md:
     * Mark Days 3-7 COMPLETE ✅
     * Document time spent vs estimate
   - **Milestone**: ✅ Greybox walkthrough ready (with caveats)

**Deliverables**:
- [ ] `loom_awakening.ron` (Cinematic A spec)
- [ ] `guided_approach.ron` (Cinematic B spec)
- [ ] Updated `Z0_loomspire_sanctum.ron` (with cinematic triggers)
- [ ] `GREYBOX_WALKTHROUGH_REPORT.md` (validation results)
- [ ] QUICK_ACTION_CHECKLIST.md updated (Days 3-7 COMPLETE)

**Success Criteria**:
- ✅ All 3 greybox zones exist as `.glb` files
- ✅ All 3 scene descriptors parse correctly
- ✅ Dialogue triggers mapped to TOML nodes
- ✅ Cinematics scripted (even if not implemented in runtime yet)
- ✅ Walkthrough validation completed (manual or automated)
- ✅ Known issues documented for Week 2+

---

## Timeline & Estimates

| Day | Tasks | Estimated Hours | Cumulative |
|-----|-------|-----------------|------------|
| **Day 3** | Asset pipeline setup, test mesh | 4-6h | 4-6h |
| **Day 4** | Loomspire + Echo Grove greybox | 6-8h | 10-14h |
| **Day 5** | Fractured Cliffs + validation | 5-7h | 15-21h |
| **Day 6** | Scene descriptors + narrative integration | 4-6h | 19-27h |
| **Day 7** | Cinematics + walkthrough validation | 4-6h | 23-33h |

**Total Estimate**: 23-33 hours (average 28 hours)  
**Target**: Complete in 5 days (4-6 hours/day pace)

---

## Risks & Mitigation

### Risk 1: Blender Unavailable
**Probability**: Medium  
**Impact**: High (can't create greybox meshes)  
**Mitigation**:
- Option A: Use procedural mesh generation in Rust (create cubes/cylinders via code)
- Option B: Use pre-existing test meshes from `assets/models/` (adapt existing geometry)
- Option C: Defer mesh creation, focus on scene descriptors + narrative (Day 3 + Day 6-7 only)

### Risk 2: Runtime Doesn't Support GLTF
**Probability**: Low (wgpu has GLTF support)  
**Impact**: High (can't load greybox meshes)  
**Mitigation**:
- Verify GLTF support in `astraweave-render` before Day 3
- If unsupported, use `.obj` format or implement GLTF loader (2-4 hours)

### Risk 3: Scene Descriptor Parser Missing
**Probability**: Medium  
**Impact**: Medium (can't load `.ron` files)  
**Mitigation**:
- Create minimal RON parser using `ron` crate (1-2 hours)
- Add to `astraweave-scene` or create `veilweaver-scene-loader` crate

### Risk 4: Dialogue System Not Integrated
**Probability**: High (dialogue exists but may not be wired to triggers)  
**Impact**: Low (can document mapping, defer implementation to Week 2)  
**Mitigation**:
- Focus on documentation (Day 6) rather than implementation
- Create integration plan for Week 2 (2-3 hours to wire triggers)

### Risk 5: Time Overrun
**Probability**: Medium (5 days is tight for 28 hours of work)  
**Impact**: Medium (delays Week 2 start)  
**Mitigation**:
- Prioritize: Day 3 (pipeline) + Day 4-5 (meshes) are CRITICAL
- Optional: Day 6-7 (cinematics) can be deferred if time runs short
- Extend to 6-7 days if needed (acceptable for foundation work)

---

## Dependencies

**Before Day 3**:
- ✅ Weaving coverage complete (94.26%, blocker resolved)
- ✅ Foundation audit updated (A+ status)
- ✅ Master reports updated (all docs consistent)

**External Dependencies**:
- Blender 4.3+ (for mesh creation) OR procedural mesh generation
- wgpu GLTF loader (likely exists, verify)
- RON parser (likely exists in `astraweave-scene`, verify)

**Blocking Issues**:
- None currently identified (all systems production-ready)

---

## Success Metrics

**Quantitative**:
- ✅ **3 greybox meshes created**: Loomspire, Echo Grove, Fractured Cliffs
- ✅ **3 scene descriptors authored**: `.ron` files parse correctly
- ✅ **5+ dialogue triggers mapped**: Triggers → TOML nodes documented
- ✅ **2 cinematics scripted**: Loom Awakening + Guided Approach (even if not implemented)
- ✅ **1 validation report**: Walkthrough results documented

**Qualitative**:
- ✅ **Asset pipeline defined**: Clear workflow from Blender → AstraWeave
- ✅ **Greybox experience walkable**: Player can navigate zones (if runtime supports)
- ✅ **Narrative flow documented**: Dialogue triggers → cinematics → tutorial clear
- ✅ **Known issues documented**: Any gaps for Week 2+ identified

**Timeline**:
- ✅ **Days 3-7 complete within 5-7 days** (acceptable variance)
- ✅ **Time tracking**: Actual hours vs estimate documented
- ✅ **Week 2 ready**: No blockers for core mechanics implementation

---

## Next Steps After Day 7

**Week 2 (Days 8-14): Core Mechanics**
- Implement weaving tutorial (Z1 zone work)
- Echo Grove combat encounter (AI + combat physics integration)
- Thread HUD (UI system work)
- **Milestone**: ✅ Tutorial loop functional

**Week 3 (Days 15-21): Companion AI**
- GOAP goals/actions (6 actions: MoveTo, Attack, TakeCover, UseAbility, Retreat, Heal)
- Adaptive unlock logic (telemetry-driven progression)
- **Milestone**: ✅ Companion adaptive unlock milestone

**Week 4 (Days 22-28): Boss Director**
- Oathbound Warden state machine (Vigilant → Challenging → Wrathful)
- Adaptive selection (player performance → difficulty adjustment)
- Arena modifiers (dynamic obstacles, hazards)
- **Milestone**: ✅ Boss phase transitions stable

**Weeks 5-6 (Days 29-42): Polish & Validation**
- Performance profiling (60 FPS validation)
- Edge case testing (100+ test cases)
- Documentation finalization
- **Milestone**: ✅ Vertical slice complete

---

## Documentation Structure

```
docs/projects/veilweaver/
├── WEEK_1_DAYS_3_7_GREYBOX_PLAN.md (this file)
├── GREYBOX_ASSET_WORKFLOW.md (Day 3 deliverable)
├── NARRATIVE_TRIGGER_MAPPING.md (Day 6 deliverable)
├── ANCHOR_INTEGRATION.md (Day 6 deliverable)
├── GREYBOX_WALKTHROUGH_REPORT.md (Day 7 deliverable)
├── templates/
│   └── zone_descriptor_template.ron (Day 3 deliverable)
└── cinematics/
    ├── loom_awakening.ron (Day 7 deliverable)
    └── guided_approach.ron (Day 7 deliverable)

assets/
├── models/greybox/
│   ├── loomspire_sanctum_greybox.glb (Day 4 deliverable)
│   ├── echo_grove_greybox.glb (Day 4 deliverable)
│   ├── fractured_cliffs_greybox.glb (Day 5 deliverable)
│   └── test_greybox.glb (Day 3 deliverable)
├── cells/
│   ├── Z0_loomspire_sanctum.ron (Day 4-6 deliverable)
│   ├── Z1_echo_grove.ron (Day 4-6 deliverable)
│   └── Z2_fractured_cliffs.ron (Day 5-6 deliverable)
└── navmeshes/ (placeholders, defer generation to Week 2)
    ├── loomspire_sanctum_navmesh.ron (deferred)
    ├── echo_grove_navmesh.ron (deferred)
    └── fractured_cliffs_navmesh.ron (deferred)
```

---

## Appendix: Quick Reference

### GLTF 2.0 Export Settings (Blender)
```
Format: GLTF Binary (.glb)
Embed: Textures ✅, Images ✅
Geometry: Normals ✅, UVs ✅, Vertex Colors ❌
Animation: Keyframes ❌ (greybox is static)
Compression: None (greybox is small)
Y Up: ✅
Scale: 1.0
```

### Zone Descriptor RON Schema (v1.0)
```ron
(
    zone_id: String,                      // Unique identifier (e.g., "Z0_loomspire_sanctum")
    mesh_path: String,                    // Path to GLTF mesh (e.g., "assets/models/greybox/...")
    spawn_points: Vec<SpawnPoint>,        // Player spawn locations
    triggers: Vec<Trigger>,               // Event triggers (dialogue, combat, etc.)
    anchors: Vec<Anchor>,                 // Weaving anchors for zone
    navigation_mesh: String,              // Path to navmesh (or "placeholder")
    dialogue_nodes: Vec<String>,          // Dialogue node IDs from dialogue_intro.toml
    cinematic_triggers: Vec<String>,      // Cinematic IDs to play on zone enter
)
```

### Greybox Material Naming
- `greybox_floor`: Grey diffuse (0.5, 0.5, 0.5), used for ground planes
- `greybox_wall`: Dark grey (0.3, 0.3, 0.3), used for walls/cliffs
- `greybox_obstacle`: Red tint (0.6, 0.3, 0.3), used for cover/hazards

### Validation Commands
```powershell
# Verify all RON files parse
Get-ChildItem assets/cells/*.ron | ForEach-Object {
    Write-Host "Parsing $_..."
    # Manual check or: cargo test --test ron_parser
}

# Verify GLTF meshes exist
Get-ChildItem assets/models/greybox/*.glb

# Run greybox viewer (if exists)
cargo run -p veilweaver_slice_runtime --release -- --zone Z0_loomspire_sanctum
```

---

**Status**: 📋 PLANNED (ready to begin Day 3 on user confirmation)  
**Priority**: P0 - Foundation (critical for Veilweaver vertical slice)  
**Estimated Start**: November 8, 2025 (after master report update complete)  
**Estimated Completion**: November 13-15, 2025 (5-7 days)


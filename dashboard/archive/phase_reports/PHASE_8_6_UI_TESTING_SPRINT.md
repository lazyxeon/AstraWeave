# Phase 8.6: UI Testing Sprint

**Date**: November 18, 2025  
**Duration**: 10-12 days  
**Status**: 🚀 **READY TO START**  
**Objective**: Achieve 80%+ UI test coverage across astraweave-ui

---

## Executive Summary

**Mission**: Raise astraweave-ui test coverage from **19.83%** to **80%+** through systematic testing of HUD, menus, panels, and egui integration.

**Approach**: Focus on data/logic/state testing rather than visual rendering (defer visual regression to manual QA).

**Target Metrics**:
- **Coverage**: 19.83% → 80%+ (60.17 point gain)
- **Tests**: 98 current → 152+ total (+54 tests minimum)
- **Quality**: Zero regressions, all existing 98 tests must pass
- **Performance**: No HUD update degradation (maintain <1ms frame time)

---

## Current State Analysis

**Module Coverage Breakdown**:
- `hud.rs` (2,678 LOC): ~50 untested functions (largest gap)
- `panels.rs` (307 LOC): 8 untested functions (interaction logic)
- `menus.rs` (537 LOC): 3 untested functions (egui rendering)
- `layer.rs` (168 LOC): 7 untested functions (wgpu/winit integration)
- `persistence.rs` (112 LOC): Well covered, 1-2 edge cases
- `menu.rs` (490 LOC): Well covered (17 inline tests)
- `state.rs` (103 LOC): Well covered (6 inline tests)

**Existing Tests**: 98 total (70 integration, 28 unit)

**Critical Gaps**:
1. HudManager methods (~30 public methods untested)
2. Quest/Notification/Combo systems (Week 4 features)
3. Easing functions and animation timings
4. Damage number physics (arc motion, shake)

---

## Sprint Priorities

### Priority 1: Core HUD Logic (25 tests, 5-6 hours)

**Day 1-2: Physics & Animations**

Tests to create:
1. **Easing functions** (2 tests)
   - `test_ease_out_cubic()` - Verify curve (0→1, smooth deceleration)
   - `test_ease_in_out_quad()` - Verify curve (0→1→0, acceleration + decel)

2. **DamageNumber physics** (2 tests)
   - `test_calculate_offset_arc_motion()` - Verify vertical arc (gravity simulation)
   - `test_calculate_shake_rotation()` - Verify shake angle randomness

3. **Quest logic** (4 tests)
   - `test_quest_completion_partial()` - 1/2 objectives complete = 50%
   - `test_quest_completion_full()` - 2/2 objectives complete = 100%
   - `test_quest_is_complete_true()` - All objectives completed
   - `test_quest_is_complete_false()` - At least one incomplete

4. **ComboTracker** (5 tests)
   - `test_record_hit_increments_count()` - Count increases on hit
   - `test_get_combo_count()` - Retrieves current combo
   - `test_get_combo_damage()` - Accumulates damage values
   - `test_cleanup_expired_combos()` - Window expiry (1 second)
   - `test_combo_window_reset()` - Reset after expiry

5. **QuestNotification** (6 tests)
   - `test_new_quest_notification()` - Constructor validation
   - `test_objective_complete_notification()` - Constructor validation
   - `test_quest_complete_notification()` - Constructor validation
   - `test_notification_update_aging()` - Time progression
   - `test_calculate_slide_offset_ease_in()` - 0-0.3s sliding in
   - `test_calculate_alpha_fade_out()` - 1.7-2.0s fading

6. **NotificationQueue** (3 tests)
   - `test_push_notification()` - Queue insertion
   - `test_update_removes_expired()` - Cleanup after 2 seconds
   - `test_has_active_notifications()` - Active check

7. **PingMarker** (3 tests)
   - `test_ping_new_active()` - Newly created ping is active
   - `test_ping_is_active_lifetime()` - Expires after 2 seconds
   - `test_ping_age_normalized()` - 0.0→1.0 over lifetime

**Acceptance Criteria**:
- ✅ 25 tests passing
- ✅ Zero test flakiness (deterministic timing)
- ✅ Coverage: hud.rs +15-20%

---

### Priority 2: HUD State Management (20 tests, 4-5 hours)

**Day 3-4: HudManager Integration**

Tests to create:
1. **Visibility toggles** (5 tests)
   - `test_toggle_visibility_master()` - F3 toggles entire HUD
   - `test_set_visible_individual()` - Set health/minimap/quest visibility
   - `test_is_visible_query()` - Query individual element visibility
   - `test_toggle_debug_mode()` - Debug overlay toggle
   - `test_visibility_persistence()` - State persists across updates

2. **Dialogue flow** (4 tests)
   - `test_start_dialogue_sets_state()` - show_dialogue=true, active_dialogue populated
   - `test_end_dialogue_clears_state()` - show_dialogue=false, active_dialogue=None
   - `test_select_dialogue_choice_returns_next()` - Choice ID → next node ID
   - `test_select_invalid_choice_returns_none()` - Invalid choice handling

3. **Tooltips** (2 tests)
   - `test_show_tooltip_sets_data()` - Tooltip data populated
   - `test_hide_tooltip_clears_data()` - Tooltip data cleared

4. **Damage/Pings** (3 tests)
   - `test_spawn_damage_with_combo()` - Combo tracking increments
   - `test_spawn_ping_adds_marker()` - Ping added to active markers
   - `test_update_cleans_old_damage_numbers()` - Cleanup after 2 seconds

5. **HudManager update loop** (3 tests)
   - `test_update_progresses_animations()` - HealthAnimation time advances
   - `test_update_cleans_expired_notifications()` - Notification queue cleanup
   - `test_update_cleans_expired_pings()` - Ping marker cleanup

6. **HudState serialization** (2 tests)
   - `test_hud_state_default()` - Default values validation
   - `test_hud_state_toggle_flags()` - show_health, show_minimap, show_quest_tracker

7. **PoiType helpers** (1 test)
   - `test_poi_type_icon_color_mappings()` - All PoiType variants have valid icon/color

**Acceptance Criteria**:
- ✅ 20 tests passing
- ✅ State transitions validated (dialogue, tooltips, visibility)
- ✅ Coverage: hud.rs +10-15%, total ~50%+

---

### Priority 3: Edge Cases & Integration (9 tests, 2-3 hours)

**Day 5: Finalization**

Tests to create:
1. **Minimap zoom** (2 tests)
   - `test_set_minimap_zoom_clamping()` - Clamps to 0.5-2.0 range
   - `test_minimap_zoom_getter()` - Returns current zoom value

2. **Audio callbacks** (2 tests)
   - `test_minimap_click_callback_invoked()` - Callback fires on click
   - `test_ping_spawn_callback_invoked()` - Callback fires on ping spawn

3. **Persistence edge cases** (2 tests)
   - `test_save_settings_permission_denied()` - Handles write errors gracefully
   - `test_load_settings_corrupted_file()` - Falls back to defaults

4. **panels.rs interactions** (2 tests)
   - `test_crafting_button_click_returns_item()` - UiResult contains crafted item
   - `test_panel_visibility_flags()` - Inventory/map/quest/settings toggles

5. **UiData construction** (1 test)
   - `test_ui_data_initialization()` - Lifetime-bound struct validates

**Acceptance Criteria**:
- ✅ 9 tests passing
- ✅ Edge cases covered (errors, invalid input)
- ✅ Coverage: 80%+ overall target achieved

---

## Test Infrastructure To Build

### Reusable Fixtures (`tests/fixtures/mod.rs`)

```rust
use astraweave_ui::hud::*;

/// Test quest with 2 objectives
pub fn test_quest() -> Quest {
    Quest {
        id: 1,
        title: "Test Quest".into(),
        description: "Test description".into(),
        objectives: vec![
            Objective { 
                id: 1, 
                description: "Kill 5 enemies".into(), 
                completed: false, 
                progress: Some((0, 5)) 
            },
            Objective { 
                id: 2, 
                description: "Talk to NPC".into(), 
                completed: false, 
                progress: None 
            },
        ],
    }
}

/// Test enemy at origin
pub fn test_enemy(id: u32) -> EnemyData {
    EnemyData::new(id, (0.0, 1.0, 0.0), 100.0, EnemyFaction::Hostile)
}

/// Float comparison with epsilon
pub fn assert_float_eq(a: f32, b: f32, epsilon: f32) {
    assert!((a - b).abs() < epsilon, "Expected {}, got {} (diff > {})", a, b, epsilon);
}

/// Time-stepped update helper
pub fn advance_time(hud: &mut HudManager, dt: f32) {
    hud.update(dt);
}
```

### Test Patterns

**Pattern 1: Data Logic Testing (No egui)**
```rust
#[test]
fn test_damage_number_arc_motion() {
    let dmg = DamageNumber::new(50, (0.0, 1.0, 0.0), DamageType::Critical);
    
    let age = 0.5; // 500ms elapsed
    let (offset_x, offset_y) = dmg.calculate_offset(age);
    
    // Verify physics
    assert!(offset_y < 0.0, "Gravity should move downward");
    assert!(offset_x.abs() > 0.0, "Should have horizontal velocity");
}
```

**Pattern 2: State Transition Testing**
```rust
#[test]
fn test_hud_dialogue_flow() {
    let mut hud = HudManager::new();
    
    let dialogue = DialogueNode {
        id: 1,
        speaker_name: "NPC".into(),
        text: "Hello!".into(),
        choices: vec![DialogueChoice { id: 1, text: "Hi".into(), next_node: Some(2) }],
        portrait_id: None,
    };
    
    hud.start_dialogue(dialogue);
    assert!(hud.state().show_dialogue);
    
    let next = hud.select_dialogue_choice(1);
    assert_eq!(next, Some(2));
    
    hud.end_dialogue();
    assert!(!hud.state().show_dialogue);
}
```

**Pattern 3: Animation Timing Testing**
```rust
#[test]
fn test_quest_notification_slide_timing() {
    let mut notif = QuestNotification::new_quest("Test".into(), "Desc".into());
    
    // Ease-in phase (0-0.3s)
    notif.animation_time = 0.15;
    let offset = notif.calculate_slide_offset();
    assert!(offset < 0.0 && offset > -100.0, "Should be sliding in");
    
    // Hold phase (0.3-1.7s)
    notif.animation_time = 1.0;
    assert_eq!(notif.calculate_slide_offset(), 0.0, "Should be on-screen");
    
    // Ease-out phase (1.7-2.0s)
    notif.animation_time = 1.9;
    let offset = notif.calculate_slide_offset();
    assert!(offset < 0.0, "Should be sliding out");
}
```

---

## Daily Breakdown

### Day 1: Easing, Physics, Quest Logic (~3 hours)
**Tests**: 8 tests (easing, damage physics, quest completion)  
**Focus**: Mathematical correctness, deterministic results

**Tasks**:
1. Create `tests/fixtures/mod.rs` with helpers
2. Implement easing function tests (2)
3. Implement damage number physics tests (2)
4. Implement quest logic tests (4)

**Validation**: `cargo test -p astraweave-ui -- easing physics quest`

---

### Day 2: Combo, Notifications, Pings (~3 hours)
**Tests**: 17 tests (ComboTracker, QuestNotification, NotificationQueue, PingMarker)  
**Focus**: Time-based systems, queueing behavior

**Tasks**:
1. Implement ComboTracker tests (5)
2. Implement QuestNotification tests (6)
3. Implement NotificationQueue tests (3)
4. Implement PingMarker tests (3)

**Validation**: `cargo test -p astraweave-ui -- combo notification ping`

---

### Day 3: HudManager Visibility & Dialogue (~2.5 hours)
**Tests**: 11 tests (visibility toggles, dialogue flow, tooltips)  
**Focus**: State management, user interaction simulation

**Tasks**:
1. Implement visibility toggle tests (5)
2. Implement dialogue flow tests (4)
3. Implement tooltip tests (2)

**Validation**: `cargo test -p astraweave-ui -- hud_manager`

---

### Day 4: HudManager Damage, Pings, Update (~2 hours)
**Tests**: 9 tests (damage spawning, ping spawning, update loop, state, PoiType)  
**Focus**: Integration of subsystems

**Tasks**:
1. Implement damage/ping tests (3)
2. Implement update loop tests (3)
3. Implement HudState tests (2)
4. Implement PoiType helper test (1)

**Validation**: `cargo test -p astraweave-ui -- update spawn poi`

---

### Day 5: Edge Cases & Final Push (~2-3 hours)
**Tests**: 9 tests (minimap zoom, callbacks, persistence, panels, UiData)  
**Focus**: Error handling, edge cases, final coverage push

**Tasks**:
1. Implement minimap zoom tests (2)
2. Implement audio callback tests (2)
3. Implement persistence edge case tests (2)
4. Implement panels interaction tests (2)
5. Implement UiData test (1)
6. Run full coverage: `cargo llvm-cov -p astraweave-ui`
7. Validate 80%+ coverage achieved

**Validation**: `cargo test -p astraweave-ui` (all 152+ tests passing)

---

## Success Metrics

**Coverage Targets**:
- **hud.rs**: 19.83% → 70%+ (~50 point gain)
- **panels.rs**: Partial → 80%+
- **persistence.rs**: ~70% → 85%+
- **Overall**: 19.83% → 80%+ (60.17 point gain)

**Test Count**: 98 → 152+ (+54 minimum)

**Quality Gates**:
- ✅ Zero test failures (all 152+ pass)
- ✅ Zero test flakiness (deterministic timing)
- ✅ Zero compilation warnings in test code
- ✅ Performance regression: <5% HUD update time increase

**Deliverables**:
1. `tests/fixtures/mod.rs` - Reusable test utilities
2. `tests/hud_logic_tests.rs` - Physics, animations, quest logic
3. `tests/hud_manager_tests.rs` - State management, integration
4. `tests/edge_case_tests.rs` - Error handling, edge cases
5. `PHASE_8_6_UI_TESTING_COMPLETE.md` - Completion report with metrics

---

## Risks & Mitigations

**Risk 1**: egui rendering difficult to test  
**Mitigation**: Test data/logic/state, not rendering (defer visual to manual QA)

**Risk 2**: Time-based tests may be flaky  
**Mitigation**: Use deterministic time stepping (no real-time `Instant::now()`)

**Risk 3**: Coverage may not reach 80% due to private methods  
**Mitigation**: Prioritize public API, integration tests validate private indirectly

**Risk 4**: Existing 98 tests may break during refactoring  
**Mitigation**: Run `cargo test -p astraweave-ui` after every change, fix immediately

---

## Next Steps After Sprint

1. **Performance benchmarks**: HUD update with 1000 damage numbers
2. **Visual regression**: Screenshot comparison for menus/panels
3. **Accessibility testing**: Keyboard navigation, screen reader support
4. **Integration with editor**: `aw_editor` UI testing using same patterns

---

**Sprint Owner**: Verdent AI  
**Estimated Effort**: 12-15 hours (10-12 days calendar time)  
**Dependencies**: None (all infrastructure exists)  
**Blockers**: None identified

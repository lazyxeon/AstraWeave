# Comprehensive Mutation Test Plan for P0 Crates

**Version**: 1.1.0  
**Date**: January 31, 2026  
**Status**: ✅ COMPLETE  
**Objective**: Add all 3 mutation-resistant test types to all 7 P0 crates

---

## Executive Summary

This plan detailed the systematic addition of **boundary condition tests**, **comparison operator tests**, and **boolean return path tests** to all 7 P0 crates. These test types specifically target the mutations that `cargo mutants` generates, ensuring maximum mutation kill rate.

### ✅ IMPLEMENTATION COMPLETE

| Crate | Tests Added | Status |
|-------|-------------|--------|
| astraweave-core | 465 | ✅ Complete |
| astraweave-ecs | 300 | ✅ Complete |
| astraweave-physics | 560 | ✅ Complete |
| astraweave-ai | 244 | ✅ Complete |
| astraweave-render | 152 | ✅ Complete |
| astraweave-terrain | 140 | ✅ Complete |
| astraweave-prompts | 88 | ✅ Complete |
| **TOTAL** | **1,949** | ✅ **ALL COMPLETE** |

### P0 Crates (7 total)
1. **astraweave-core** - Schema, validation, world state
2. **astraweave-ecs** - Entity component system, RNG, events
3. **astraweave-physics** - Spatial hash, projectiles, destruction, cloth
4. **astraweave-ai** - AI controllers, orchestrators, arbiter
5. **astraweave-render** - Time of day, weather, camera, culling
6. **astraweave-terrain** - Heightmaps, chunks, erosion, LOD blending
7. **astraweave-prompts** - Template validation, sanitization, optimization

---

## Test Type Definitions

### Type 1: Boundary Condition Tests
**Purpose**: Test values at exact boundaries to catch `<` vs `<=` and `>` vs `>=` mutations.

**Pattern**:
```rust
// For condition: if x < 0.0 || x > 1.0 { return Err(...) }
#[test]
fn intensity_at_lower_bound_is_valid() {
    assert!(validate(0.0).is_ok());   // AT boundary (0.0) should pass
}
#[test]
fn intensity_below_lower_bound_fails() {
    assert!(validate(-0.001).is_err()); // BELOW boundary fails
}
#[test]
fn intensity_at_upper_bound_is_valid() {
    assert!(validate(1.0).is_ok());   // AT boundary (1.0) should pass
}
#[test]
fn intensity_above_upper_bound_fails() {
    assert!(validate(1.001).is_err()); // ABOVE boundary fails
}
```

### Type 2: Comparison Operator Tests
**Purpose**: Test both sides of comparisons to catch operator swaps (`<` ↔ `>`, `<=` ↔ `>=`, `==` ↔ `!=`).

**Pattern**:
```rust
// For condition: if dist <= 3.0 { /* close */ } else { /* far */ }
#[test]
fn distance_exactly_at_threshold() {
    assert_eq!(classify(3.0), "close"); // AT boundary = close
}
#[test]
fn distance_just_below_threshold() {
    assert_eq!(classify(2.999), "close"); // BELOW = close
}
#[test]
fn distance_just_above_threshold() {
    assert_eq!(classify(3.001), "far"); // ABOVE = far
}
```

### Type 3: Boolean Return Path Tests
**Purpose**: Test all paths through boolean-returning functions to catch return value swaps.

**Pattern**:
```rust
// For: fn is_valid(&self) -> bool { self.x >= 0 && self.y >= 0 }
#[test]
fn is_valid_all_positive() {
    assert!(Point::new(1, 1).is_valid()); // Both pass
}
#[test]
fn is_valid_x_negative() {
    assert!(!Point::new(-1, 1).is_valid()); // First fails
}
#[test]
fn is_valid_y_negative() {
    assert!(!Point::new(1, -1).is_valid()); // Second fails
}
#[test]
fn is_valid_both_negative() {
    assert!(!Point::new(-1, -1).is_valid()); // Both fail
}
```

---

## Phase 1: astraweave-core (Priority 1)

### 1.1 Boundary Conditions to Test

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| schema.rs:502 | `TerrainGenerationRequest::validate()` | `intensity < 0.0 \|\| intensity > 1.0` | 4 (0.0, -0.001, 1.0, 1.001) |
| schema.rs:507 | `TerrainGenerationRequest::validate()` | `narrative_reason.len() > 100` | 3 (99, 100, 101 chars) |
| validation.rs:298 | Cooldown check | `cooldown > 0.0` | 3 (0.0, 0.001, -0.001) |
| validation.rs:317 | Ammo check | `rounds <= 0` | 3 (0, 1, -1) |
| validation.rs:335 | Health check | `hp <= 0` | 3 (0, 1, -1) |
| tools.rs:114 | Bounds check | `nx < minx \|\| ny < miny \|\| nx > maxx \|\| ny > maxy` | 8 tests |
| tools.rs:203 | g-score check | `ng < g.get(&pos)...` | 3 tests |
| perception.rs:53 | LOS check | `manhattan > cfg.los_max` | 3 tests |
| world.rs:73 | Entity ID check | `id >= self.next_id` | 3 tests |

### 1.2 Comparison Operators to Test

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| tools.rs:80,83 | Movement | `x != b_glam.x`, `y != b_glam.y` | 4 tests (each axis equal/not) |
| tools.rs:108 | Goal reached | `p.x == goal.x && p.y == goal.y` | 4 tests |
| tools.rs:182 | Path check | `prev == cur` | 3 tests |
| world.rs:186 | Team filter | `t.id == team_id` | 3 tests |
| world.rs:192 | Team filter | `t.id != team_id` | 3 tests |
| ecs_bridge.rs:19,25 | Entity mapping | `old_ecs != ecs_e`, `old_legacy != legacy` | 4 tests |

### 1.3 Boolean Return Paths to Test

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| schema.rs | `IVec2::is_zero()` | bool | 3 (zero, x≠0, y≠0) |
| schema.rs | `WorldSnapshot::has_no_enemies()` | bool | 2 (empty, non-empty) |
| schema.rs | `WorldSnapshot::has_ammo()` | bool | 3 (>0, =0, <0) |
| schema.rs | `WorldSnapshot::has_pois()` | bool | 2 (empty, non-empty) |
| schema.rs | `WorldSnapshot::has_objective()` | bool | 2 (None, Some) |

**Estimated Tests for astraweave-core**: ~55 new tests

---

## Phase 2: astraweave-ecs (Priority 2)

### 2.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| archetype.rs:292 | swap_remove | `row < entities_len - 1` | 3 tests |
| archetype.rs:523 | entity lookup | `id >= self.entity_to_archetype.len()` | 3 tests |
| archetype.rs:533 | entity lookup | `id < self.entity_to_archetype.len()` | 3 tests |
| blob_vec.rs:62 | allocation | `capacity > 0` | 2 tests |
| blob_vec.rs:112 | reserve | `required_cap <= self.capacity` | 3 tests |
| blob_vec.rs:150 | push | `self.len == self.capacity` | 3 tests |
| blob_vec.rs:188 | get | `index >= self.len` | 3 tests |
| blob_vec.rs:230 | swap_remove | `index != last_index` | 3 tests |
| blob_vec.rs:315 | pop | `self.len == 0` | 2 tests |
| rng.rs | gen_range | min/max bounds | 4 tests |

### 2.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| blob_vec.rs:124 | new_data | `self.capacity == 0` | 2 tests |
| blob_vec.rs:348 | swap_replace | `index != last_index` | 3 tests |

### 2.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| rng.rs | `gen_bool(p)` | bool | 4 (p=0, p=1, p=0.5, edge) |
| events.rs | `is_empty::<T>()` | bool | 2 |
| archetype.rs | `contains(id)` | bool | 3 |

**Estimated Tests for astraweave-ecs**: ~45 new tests

---

## Phase 3: astraweave-physics (Priority 3)

### 3.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| async_scheduler.rs:50 | timing | `total_duration.as_nanos() == 0` | 2 tests |
| destruction.rs:97 | debris count | `debris.len() >= piece_count` | 3 tests |
| destruction.rs:306 | health | `self.health <= 0.0` | 3 tests |
| destruction.rs:308 | damage threshold | `health < max_health * 0.5` | 3 tests |
| destruction.rs:319 | force threshold | `accumulated_force >= threshold` | 3 tests |
| cloth.rs:38 | inv_mass | `mass > 0.0` | 3 tests |
| cloth.rs:111 | length | `current_length < 0.0001` | 3 tests |
| cloth.rs:122 | weight | `total_weight > 0.0` | 3 tests |
| cloth.rs:160 | collision | `dist < radius` | 3 tests |
| cloth.rs:210 | constraint | `dist < 0.0` | 3 tests |
| cloth.rs:378 | index | `index < self.particles.len()` | 3 tests |
| spatial_hash | AABB intersects | all 6 plane checks | 12 tests |

### 3.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| destruction.rs:300 | state check | `state != Intact && state != Damaged` | 4 tests |
| cloth.rs:299,306,313 | grid edges | `x < width - 1`, `y < height - 1` | 6 tests |
| cloth.rs:326,332 | bend springs | `x < width - 2`, `y < height - 2` | 4 tests |

### 3.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| spatial_hash.rs | `AABB::intersects()` | bool | 6 (each axis, overlap/gap) |
| spatial_hash.rs | `AABB::contains_point()` | bool | 4 |
| destruction.rs | `Destructible::is_intact()` | bool | 3 |
| destruction.rs | `Destructible::is_destroyed()` | bool | 3 |

**Estimated Tests for astraweave-physics**: ~70 new tests

---

## Phase 4: astraweave-ai (Priority 4)

### 4.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| ai_arbiter.rs:386 | plan execution | `step_index < plan.steps.len()` | 3 tests |
| ai_arbiter.rs:393 | plan completion | `next_index >= plan.steps.len()` | 3 tests |
| ai_arbiter.rs:557 | cooldown | `cooldown_elapsed < llm_request_cooldown` | 4 tests |
| orchestrator.rs:80 | smoke cd | `smoke_cd <= 0.0` | 3 tests |
| orchestrator.rs:183 | cd check | `cd <= 0.0` | 3 tests |
| orchestrator.rs:215 | distance | `dist <= 3.0` | 3 tests |
| orchestrator.rs:309 | melee dist | `dist <= 2` | 3 tests |
| orchestrator.rs:339 | melee dist | `dist <= 2` | 3 tests |
| async_task.rs:156 | timeout | `elapsed > timeout` | 3 tests |
| tool_sandbox.rs:340 | ammo check | `ammo == 0` | 3 tests |

### 4.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| ai_arbiter.rs:473 | mode check | `mode != GOAP` | 3 tests |
| ai_arbiter.rs:489 | mode check | `mode != BehaviorTree` | 3 tests |
| ai_arbiter.rs:551 | mode check | `mode != GOAP` | 3 tests |
| llm_executor.rs:257 | delay check | `delay_ms > 0` | 2 tests |

### 4.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| core_loop.rs | `PlannerMode::is_always_available()` | bool | 3 |
| core_loop.rs | `PlannerMode::requires_bt_feature()` | bool | 3 |
| core_loop.rs | `PlannerMode::requires_goap_feature()` | bool | 3 |
| core_loop.rs | `CAiController::has_policy()` | bool | 2 |
| core_loop.rs | `CAiController::requires_feature()` | bool | 3 |
| ai_arbiter.rs | `AIArbiter::is_llm_active()` | bool | 2 |

**Estimated Tests for astraweave-ai**: ~50 new tests

---

## Phase 5: astraweave-render (Priority 5)

### 5.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| camera.rs:165 | deadzone | `delta.x.abs() < deadzone && delta.y.abs() < deadzone` | 4 tests |
| camera.rs:260 | velocity | `vel.length_squared() > 0.0` | 3 tests |
| decals.rs:87 | fade | `fade_duration > 0.0` | 3 tests |
| decals.rs:89 | fade complete | `fade_time >= fade_duration` | 3 tests |
| culling.rs:113 | normalize | `len > 0.0` | 3 tests |
| culling.rs:141 | frustum | `dist < -radius` | 3 tests |
| deferred.rs:157 | resize | `width == self.width && height == self.height` | 3 tests |
| environment.rs:67 | sun height | `sun_height.abs() < 0.01` | 3 tests |
| environment.rs:89 | sun position | `sun_pos.y > 0.1` | 3 tests |
| environment.rs:100,104 | sun height | `sun_height > 0.2`, `sun_height > -0.2` | 4 tests |
| clustered_forward.rs:685 | light cull | `distance > radius` | 3 tests |

### 5.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| camera.rs:133 | mouse button | `button == MouseButton::Right` | 2 tests |
| environment.rs | twilight checks | Multiple comparisons | 6 tests |

### 5.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| environment.rs | `TimeOfDay::is_day()` | bool | 4 (noon, midnight, dawn, dusk) |
| environment.rs | `TimeOfDay::is_night()` | bool | 4 |
| environment.rs | `TimeOfDay::is_twilight()` | bool | 4 |
| weather.rs | `WeatherSystem::is_raining()` | bool | 3 |
| culling.rs | Frustum plane checks | bool | 6 |

**Estimated Tests for astraweave-render**: ~60 new tests

---

## Phase 6: astraweave-terrain (Priority 6)

### 6.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| heightmap.rs | from_data validation | resolution² == data.len() | 3 tests |
| heightmap.rs | get_height bounds | x < resolution, y < resolution | 4 tests |
| biome_blending.rs:62 | weight | `total_weight > 0.0` | 3 tests |
| biome_blending.rs:177 | blend radius | `effective_distance < blend_radius` | 3 tests |
| biome_blending.rs:188 | weight threshold | `weight > min_weight_threshold` | 3 tests |
| biome_blending.rs:259 | grid bounds | `nx >= 0 && nx < resolution` | 4 tests |
| lod_blending.rs:99 | morph start | `distance <= morph_start` | 3 tests |
| lod_blending.rs:101 | morph end | `distance >= morph_end` | 3 tests |
| lod_blending.rs:118 | morph factor | `morph_factor <= 0.0` | 3 tests |
| lod_blending.rs:126 | morph factor | `morph_factor >= 1.0` | 3 tests |
| background_loader.rs:208 | task limit | `active_task_count >= max_concurrent` | 3 tests |
| background_loader.rs:390 | throttle | `smoothed_time > threshold && loaded > 50` | 4 tests |
| background_loader.rs:419 | velocity | `velocity.length() < 0.1` | 3 tests |
| background_loader.rs:617 | chunk limit | `loaded.len() <= max_loaded` | 3 tests |
| advanced_erosion.rs | Multiple radius/distance checks | | 10 tests |

### 6.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| background_loader.rs:33 | frustum priority | `in_frustum != other.in_frustum` | 3 tests |
| lib.rs:314 | score comparison | `score > best_score` | 3 tests |
| lod_blending.rs:315 | threshold | `distance < threshold` | 3 tests |

### 6.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| chunk.rs | `Chunk::is_loaded()` | bool | 2 |
| chunk.rs | `Chunk::needs_update()` | bool | 3 |
| heightmap.rs | `Heightmap::is_valid_coord()` | bool | 4 |
| lod_blending.rs | morph factor checks | bool | 4 |

**Estimated Tests for astraweave-terrain**: ~75 new tests

---

## Phase 7: astraweave-prompts (Priority 7)

### 7.1 Boundary Conditions

| File | Function | Condition | Tests Needed |
|------|----------|-----------|--------------|
| helpers.rs:235 | length check | `prompt.len() > max_length` | 3 tests |
| helpers.rs:239 | length check | `prompt.len() < min_length` | 3 tests |
| helpers.rs:271 | brace balance | `brace_count < 0` | 3 tests |
| helpers.rs:419,421 | readability | `avg_words <= 15.0`, `<= 25.0` | 4 tests |
| optimization.rs:110 | compression | `prompt.len() > max_prompt_length` | 3 tests |
| optimization.rs:168 | TTL check | `elapsed.as_secs() > ttl_seconds` | 3 tests |
| optimization.rs:183 | cache size | `cache.len() >= max_size` | 3 tests |
| sanitize.rs:221 | input limit | `max_user_input_length >= 1_000_000` | 3 tests |
| sanitize.rs:307 | trust level | `trust_level >= Developer` | 3 tests |
| sanitize.rs:312 | input length | `input.len() > max_user_input_length` | 3 tests |
| sanitize.rs:382 | var name length | `name.len() > max_variable_name_length` | 3 tests |
| sanitize.rs:411 | truncation | `input.len() <= max_length` | 3 tests |
| sanitize.rs:418 | word boundary | `last_space > max_length / 2` | 3 tests |
| lib.rs:242 | TTL | `ttl_seconds >= 3600` | 3 tests |

### 7.2 Comparison Operators

| File | Line | Condition | Tests Needed |
|------|------|-----------|--------------|
| helpers.rs:394,397 | brace matching | `char == '{'`, `char == '}'` | 4 tests |
| sanitize.rs:458 | newline | `c == '\n'` | 2 tests |
| library.rs:338 | extension | `ext == "hbs"` | 2 tests |
| loader.rs:51 | extension match | iterator any() | 3 tests |

### 7.3 Boolean Return Paths

| File | Function | Returns | Tests Needed |
|------|----------|---------|--------------|
| helpers.rs | `is_balanced()` | bool | 4 (balanced, unbalanced variants) |
| helpers.rs | `has_unclosed_placeholders()` | bool | 4 |
| sanitize.rs | various validation | bool | 6 |
| optimization.rs | `is_cached()` | bool | 3 |

**Estimated Tests for astraweave-prompts**: ~65 new tests

---

## Implementation Order

| Phase | Crate | Estimated Tests | Priority |
|-------|-------|-----------------|----------|
| 1 | astraweave-core | ~55 | Critical (foundation) |
| 2 | astraweave-ecs | ~45 | Critical (ECS core) |
| 3 | astraweave-physics | ~70 | High (physics correctness) |
| 4 | astraweave-ai | ~50 | High (AI decisions) |
| 5 | astraweave-render | ~60 | Medium (rendering logic) |
| 6 | astraweave-terrain | ~75 | Medium (terrain correctness) |
| 7 | astraweave-prompts | ~65 | High (LLM safety) |

**Total Estimated New Tests**: ~420 tests

---

## Quality Criteria

For each test added:
1. ✅ **Specific Value**: Test exact boundary value (not just "near" boundary)
2. ✅ **Both Sides**: Test value just below AND just above boundary
3. ✅ **Assertion Message**: Include meaningful failure message
4. ✅ **Independence**: Each test tests ONE specific mutation scenario
5. ✅ **Reproducibility**: No random values (use fixed seeds where needed)

---

## Validation Process

After adding tests to each crate:
1. Run `cargo test -p <crate> --lib` to verify all tests pass
2. Run `cargo clippy -p <crate>` to ensure no warnings
3. Run `cargo mutants -p <crate> --list` to count mutations
4. (Optional) Run `cargo mutants -p <crate> --in-place -j 2` to validate kill rate

---

## Success Metrics

| Metric | Target |
|--------|--------|
| All tests pass | 100% |
| No compiler warnings | 0 |
| Mutation kill rate (estimated) | 85-95% |
| Coverage of boundary conditions | 100% of identified |

---

## Next Steps

1. ✅ Plan created
2. 🔄 Phase 1: Implement astraweave-core tests
3. 🔄 Phase 2: Implement astraweave-ecs tests
4. 🔄 Phase 3: Implement astraweave-physics tests
5. 🔄 Phase 4: Implement astraweave-ai tests
6. 🔄 Phase 5: Implement astraweave-render tests
7. 🔄 Phase 6: Implement astraweave-terrain tests
8. 🔄 Phase 7: Implement astraweave-prompts tests
9. 📊 Final validation and report update

---

**Document Version**: 1.0.0  
**Created**: January 31, 2026  
**Author**: GitHub Copilot

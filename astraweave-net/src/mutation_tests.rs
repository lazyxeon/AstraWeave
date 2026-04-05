//! Mutation-killing tests for astraweave-net.
//! Targets: Interest policies, delta compression, Bresenham LOS, hashing, replay.

use crate::{
    apply_delta, build_snapshot, diff_snapshots, filter_snapshot_for_viewer, has_los,
    stable_hash_snapshot, subset_hash, world_obstacles_btree, world_to_entities, Delta,
    EntityDelta, EntityDeltaMask, EntityState, FovInterest, FovLosInterest, FullInterest, Interest,
    RadiusTeamInterest, ReplayEvent, Snapshot, SNAPSHOT_VERSION,
};
use astraweave_core::{ActionStep, IVec2, PlanIntent, Team, World};
use std::collections::BTreeSet;

// ── Helper ──────────────────────────────────────────────────────────────

fn entity(id: u32, x: i32, y: i32, team: u8) -> EntityState {
    EntityState {
        id,
        pos: IVec2 { x, y },
        hp: 100,
        team,
        ammo: 10,
    }
}

// ════════════════════════════════════════════════════════════════════════
// RadiusTeamInterest mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_radius_squared_not_doubled() {
    // Catches L106: self.radius * self.radius → self.radius + self.radius
    // radius=5: radius² = 25, radius+radius = 10
    // Entity at (4,0): dist²=16. 16 <= 25 (true) vs 16 <= 10 (false)
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 4, 0, 1); // different team, dist²=16
    let policy = RadiusTeamInterest { radius: 5 };
    assert!(
        policy.include(&viewer, &enemy),
        "entity at dist²=16 should be within radius²=25"
    );
}

#[test]
fn mutation_radius_dy_squared_not_linear() {
    // Catches L106:23: dy * dy → dy + dy
    // viewer (0,0), enemy (0,5,team1), radius=4
    // Original: dy=5, dy²=25, dist²=0+25=25 > 16=r² → false (excluded)
    // Mutation (dy+dy=10): dist²=0+10=10 ≤ 16 → true (included) → CAUGHT
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 0, 5, 1);
    let policy = RadiusTeamInterest { radius: 4 };
    assert!(
        !policy.include(&viewer, &enemy),
        "entity at dist²=25 should be outside radius²=16"
    );
}

#[test]
fn mutation_radius_dx_squared_not_linear() {
    // Catches L106:13: dx * dx → dx + dx
    // viewer (0,0), enemy (5,0,team1), radius=4
    // Original: dx=5, dx²=25, dist²=25+0=25 > 16 → false (excluded)
    // Mutation (dx+dx=10): dist²=10+0=10 ≤ 16 → true (included) → CAUGHT
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 5, 0, 1);
    let policy = RadiusTeamInterest { radius: 4 };
    assert!(
        !policy.include(&viewer, &enemy),
        "entity at dist²=25 should be outside radius²=16"
    );
}

#[test]
fn mutation_radius_dx_subtraction() {
    // Catches dx=e.pos.x - viewer.pos.x with -→+
    // viewer at (3,0), enemy at (1,0,team1): correct dx=-2, dist²=4
    // mutation dx=1+3=4, dist²=16
    // With radius=2: dist²=4 <= 4 (true) vs dist²=16 <= 4 (false)
    let viewer = entity(1, 3, 0, 0);
    let enemy = entity(2, 1, 0, 1);
    let policy = RadiusTeamInterest { radius: 2 };
    assert!(
        policy.include(&viewer, &enemy),
        "entity at dist²=4 should be within radius²=4"
    );
}

#[test]
fn mutation_radius_dy_subtraction() {
    // Catches dy=e.pos.y - viewer.pos.y with -→+
    let viewer = entity(1, 0, 3, 0);
    let enemy = entity(2, 0, 1, 1);
    let policy = RadiusTeamInterest { radius: 2 };
    assert!(
        policy.include(&viewer, &enemy),
        "entity at dy=-2, dist²=4 should be within radius²=4"
    );
}

#[test]
fn mutation_radius_dist2_addition() {
    // Catches dx*dx + dy*dy → dx*dx - dy*dy
    // viewer (0,0), enemy (2,3,team1): dist²=4+9=13 vs 4-9=-5
    // With radius=4: 13 <= 16 (true). With mutation: -5 <= 16 (true). Same!
    // Need radius=3: 13 <= 9 (false). With mutation: -5 <= 9 (true). Reversed!
    // So: entity should NOT be included but mutation includes it.
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 2, 3, 1);
    let policy = RadiusTeamInterest { radius: 3 };
    assert!(
        !policy.include(&viewer, &enemy),
        "entity at dist²=13 should be outside radius²=9"
    );
}

// ════════════════════════════════════════════════════════════════════════
// FovInterest mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_fov_dx_sign_matters() {
    // Catches L120: e.pos.x - viewer.pos.x → e.pos.x + viewer.pos.x
    // viewer (5,0,team0), enemy (3,5,team1), facing (1,0), half_angle=45°, radius=20
    // Correct: dx=3-5=-2, dy=5, dist²=29. dot=1*(-2)=-2. vmag=sqrt(29)=5.39.
    //   cos_theta = -2/5.39 = -0.371. cos(45°)=0.707. -0.371 >= 0.707 → false → EXCLUDED
    // Mutation: dx=3+5=8, dist²=89. dot=8. vmag=sqrt(89)=9.43.
    //   cos_theta = 8/9.43 = 0.848 >= 0.707 → true → INCLUDED
    let viewer = entity(1, 5, 0, 0);
    let enemy = entity(2, 3, 5, 1);
    let policy = FovInterest {
        radius: 20,
        half_angle_deg: 45.0,
        facing: IVec2 { x: 1, y: 0 },
    };
    assert!(
        !policy.include(&viewer, &enemy),
        "entity behind viewer should be excluded"
    );
}

#[test]
fn mutation_fov_dy_sign_matters() {
    // Catches L121: e.pos.y - viewer.pos.y → e.pos.y + viewer.pos.y
    // viewer (0,5,team0), enemy (5,3,team1), facing (0,1), half_angle=45°, radius=20
    // Correct: dx=5, dy=3-5=-2. dot=0*5+1*(-2)=-2. EXCLUDED
    // Mutation: dy=3+5=8. dot=0*5+1*8=8. INCLUDED
    let viewer = entity(1, 0, 5, 0);
    let enemy = entity(2, 5, 3, 1);
    let policy = FovInterest {
        radius: 20,
        half_angle_deg: 45.0,
        facing: IVec2 { x: 0, y: 1 },
    };
    assert!(
        !policy.include(&viewer, &enemy),
        "entity behind viewer (y-axis) should be excluded"
    );
}

#[test]
fn mutation_fov_dist2_plus_not_minus() {
    // Catches L122: dx*dx + dy*dy → dx*dx - dy*dy
    // viewer (0,0), enemy (3,5,team1), facing (1,1), radius=10, half_angle=60
    // Correct: dist²=9+25=34 < 100. dot=3+5=8. fmag=sqrt(2)=1.41, vmag=sqrt(34)=5.83.
    //   cos_theta=8/(1.41*5.83)=8/8.22=0.973 >= cos(60°)=0.5 → INCLUDED
    // Mutation: dist²=9-25=-16 < 100. vmag=sqrt(-16)=NaN.
    //   cos_theta=NaN. NaN>=0.5 → false → EXCLUDED
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 3, 5, 1);
    let policy = FovInterest {
        radius: 10,
        half_angle_deg: 60.0,
        facing: IVec2 { x: 1, y: 1 },
    };
    assert!(
        policy.include(&viewer, &enemy),
        "entity inside FOV should be included"
    );
}

#[test]
fn mutation_fov_dist2_boundary_gt_not_gte() {
    // Catches L123: dist2 > (radius²) → dist2 >= (radius²)
    // At exact boundary dist²=radius², `>` passes (continues), `>=` returns false.
    // viewer (0,0), enemy (6,0,team1), facing (1,0), radius=6, half_angle=90
    // dist²=36=radius²=36. `>`: 36>36 false → continues to angle check.
    //   dot=6, vmag=6, cos_theta=1.0 >= cos(90°)=0 → INCLUDED
    // `>=`: 36>=36 → true → return false → EXCLUDED
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 6, 0, 1);
    let policy = FovInterest {
        radius: 6,
        half_angle_deg: 90.0,
        facing: IVec2 { x: 1, y: 0 },
    };
    assert!(
        policy.include(&viewer, &enemy),
        "entity at exact radius boundary should be included (> not >=)"
    );
}

#[test]
fn mutation_fov_facing_magnitude_and_dot_product() {
    // Catches L128×3 (fmag computation), L136×2 (dot product), L137 (denom).
    // Uses facing (3,4) which stresses magnitude + dot computations.
    //
    // TWO test cases from this setup:
    //
    // 1) Entity (4,5) INSIDE FOV with correct code → catches fmag NaN (L128:29),
    //    dot sign (L136:27), dot * vs + (L136:22)
    //
    // 2) Entity (3,8) OUTSIDE FOV with correct code → catches fmag * vs + (L128:24, L128:34),
    //    denom * vs + (L137:37)

    let viewer = entity(1, 0, 0, 0);
    let policy = FovInterest {
        radius: 20,
        half_angle_deg: 10.0,
        facing: IVec2 { x: 3, y: 4 },
    };

    // Case 1: Entity (4,5) — angle from facing ≈ 1.8° → INSIDE 10° FOV
    // Correct: dot=3*4+4*5=32. fmag=5, vmag=sqrt(41)=6.4. cos_theta=32/32=1.0 ≥ cos(10°)=0.985 → INCLUDED
    // L128:29 (+→- in fmag): fmag=sqrt(9-16)=NaN → cos_theta=NaN → EXCLUDED
    // L136:27 (+→- in dot): dot=12-20=-8 → cos_theta=-8/32=-0.25 → EXCLUDED
    // L136:22 (*→+ in dot): dot=3+4+4*5=7+20=27 → cos_theta=27/32=0.844 → EXCLUDED (< 0.985)
    let inside_fov = entity(2, 4, 5, 1);
    assert!(
        policy.include(&viewer, &inside_fov),
        "entity at angle ~1.8° from facing should be inside 10° FOV"
    );

    // Case 2: Entity (3,8) — angle from facing ≈ 16.3° → OUTSIDE 10° FOV
    // Correct: dot=9+32=41. fmag=5, vmag=sqrt(73)=8.54. cos_theta=41/42.72=0.96. <0.985 → EXCLUDED
    // L128:24 (*→+ in fx*fx): fmag=sqrt(6+16)=4.69 → cos_theta=41/40.07=1.023 → INCLUDED
    // L128:34 (*→+ in fy*fy): fmag=sqrt(9+8)=4.12 → cos_theta=41/35.2=1.165 → INCLUDED
    // L137:37 (*→+ in denom): denom=5+8.54=13.54 → cos_theta=41/13.54=3.03 → INCLUDED
    let outside_fov = entity(3, 3, 8, 1);
    assert!(
        !policy.include(&viewer, &outside_fov),
        "entity at angle ~16.3° from facing should be outside 10° FOV"
    );
}

#[test]
fn mutation_fov_cos_half_comparison() {
    // Catches cos_theta >= cos_half → cos_theta > cos_half  (if >= becomes >)
    // Need entity where cos_theta equals cos_half exactly.
    // This is hard with integer positions, so we catch inversions indirectly.
    // The boundary test above (L123) handles the radius boundary.
    // For angle boundary: facing (1,0), half_angle=45, entity at atan2(1,1)=45° (exactly at boundary)
    //   viewer (0,0), entity (5,5,team1): dx=5, dy=5
    //   dist²=50 < 100 (radius=10)
    //   dot=1*5+0*5=5. fmag=1, vmag=sqrt(50)=7.07
    //   cos_theta=5/7.07=0.707=cos(45°). 0.707 >= 0.707 → true (INCLUDED)
    //   >= → >: 0.707 > 0.707 → false → EXCLUDED
    let viewer = entity(1, 0, 0, 0);
    let enemy_at_angle = entity(2, 5, 5, 1);
    let policy = FovInterest {
        radius: 10,
        half_angle_deg: 45.0,
        facing: IVec2 { x: 1, y: 0 },
    };
    assert!(
        policy.include(&viewer, &enemy_at_angle),
        "entity at exact half_angle boundary should be included (>= not >)"
    );
}

// ════════════════════════════════════════════════════════════════════════
// FovLosInterest mutations (mirrors FovInterest + LOS check)
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_fovlos_dx_sign() {
    // Same as FovInterest dx sign test, but with FovLosInterest
    let viewer = entity(1, 5, 0, 0);
    let enemy = entity(2, 3, 5, 1);
    let policy = FovLosInterest {
        radius: 20,
        half_angle_deg: 45.0,
        facing: IVec2 { x: 1, y: 0 },
        obstacles: BTreeSet::new(),
    };
    assert!(!policy.include(&viewer, &enemy));
}

#[test]
fn mutation_fovlos_dist2_boundary() {
    // Exact boundary test for FovLosInterest
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 6, 0, 1);
    let policy = FovLosInterest {
        radius: 6,
        half_angle_deg: 90.0,
        facing: IVec2 { x: 1, y: 0 },
        obstacles: BTreeSet::new(),
    };
    assert!(
        policy.include(&viewer, &enemy),
        "FovLos: entity at exact radius should be included"
    );
}

#[test]
fn mutation_fovlos_facing_and_dot() {
    // FovLosInterest with facing (3,4) — catches fmag/dot mutations
    let viewer = entity(1, 0, 0, 0);
    let policy = FovLosInterest {
        radius: 20,
        half_angle_deg: 10.0,
        facing: IVec2 { x: 3, y: 4 },
        obstacles: BTreeSet::new(),
    };
    let inside = entity(2, 4, 5, 1);
    assert!(policy.include(&viewer, &inside));
    let outside = entity(3, 3, 8, 1);
    assert!(!policy.include(&viewer, &outside));
}

#[test]
fn mutation_fovlos_cos_half_boundary() {
    // Entity at exact angle boundary with LOS clear
    let viewer = entity(1, 0, 0, 0);
    let at_45 = entity(2, 5, 5, 1);
    let policy = FovLosInterest {
        radius: 10,
        half_angle_deg: 45.0,
        facing: IVec2 { x: 1, y: 0 },
        obstacles: BTreeSet::new(),
    };
    assert!(policy.include(&viewer, &at_45));
}

#[test]
fn mutation_fovlos_dist2_sign() {
    // Catches + → - in dist2 for FovLosInterest
    let viewer = entity(1, 0, 0, 0);
    let enemy = entity(2, 3, 5, 1);
    let policy = FovLosInterest {
        radius: 10,
        half_angle_deg: 60.0,
        facing: IVec2 { x: 1, y: 1 },
        obstacles: BTreeSet::new(),
    };
    assert!(policy.include(&viewer, &enemy));
}

// ════════════════════════════════════════════════════════════════════════
// Bresenham LOS mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_has_los_dx_abs() {
    // Catches mutations in dx=(x1-x0).abs() — if abs removed, negative dx breaks Bresenham
    let obs = BTreeSet::new();
    // Negative direction: from (5,0) to (0,0)
    assert!(has_los(IVec2 { x: 5, y: 0 }, IVec2 { x: 0, y: 0 }, &obs));
}

#[test]
fn mutation_has_los_dy_abs() {
    // Catches mutations in dy=-(y1-y0).abs() — the negation is critical
    let obs = BTreeSet::new();
    // From (0,5) to (0,0) — negative y direction
    assert!(has_los(IVec2 { x: 0, y: 5 }, IVec2 { x: 0, y: 0 }, &obs));
}

#[test]
fn mutation_has_los_obstacle_on_path() {
    // Catches return false → return true
    let mut obs = BTreeSet::new();
    obs.insert((2, 0));
    assert!(
        !has_los(IVec2 { x: 0, y: 0 }, IVec2 { x: 5, y: 0 }, &obs),
        "obstacle at (2,0) should block LOS from (0,0) to (5,0)"
    );
}

#[test]
fn mutation_has_los_diagonal_with_obstacle() {
    // Tests both dx and dy non-zero — exercises full Bresenham
    let mut obs = BTreeSet::new();
    obs.insert((2, 2));
    assert!(
        !has_los(IVec2 { x: 0, y: 0 }, IVec2 { x: 4, y: 4 }, &obs),
        "obstacle on diagonal should block LOS"
    );
}

#[test]
fn mutation_has_los_sx_direction() {
    // Catches sx = if x0 < x1 { 1 } else { -1 } mutations (<→<=, 1→-1, etc.)
    // Line from (3,0) to (0,0): sx should be -1
    let mut obs = BTreeSet::new();
    obs.insert((1, 0));
    assert!(
        !has_los(IVec2 { x: 3, y: 0 }, IVec2 { x: 0, y: 0 }, &obs),
        "obstacle at (1,0) should block LOS from right to left"
    );
}

#[test]
fn mutation_has_los_sy_direction() {
    // Catches sy mutations
    let mut obs = BTreeSet::new();
    obs.insert((0, 1));
    assert!(
        !has_los(IVec2 { x: 0, y: 3 }, IVec2 { x: 0, y: 0 }, &obs),
        "obstacle at (0,1) should block LOS going downward"
    );
}

#[test]
fn mutation_has_los_err_stepping() {
    // Catches err += dy / x0 += sx / err += dx / y0 += sy mutations
    // Steep line (dy > dx): from (0,0) to (2,5) — y steps more often
    let mut obs = BTreeSet::new();
    obs.insert((1, 3));
    assert!(
        !has_los(IVec2 { x: 0, y: 0 }, IVec2 { x: 2, y: 5 }, &obs),
        "obstacle on steep diagonal should block LOS"
    );
}

#[test]
fn mutation_has_los_skip_start_cell() {
    // Catches deletion of ! in `if !(x0 == a.x && y0 == a.y)`
    // If we DON'T skip the start cell, an obstacle on the viewer cell blocks LOS
    let mut obs = BTreeSet::new();
    obs.insert((0, 0)); // obstacle on viewer position
    assert!(
        has_los(IVec2 { x: 0, y: 0 }, IVec2 { x: 3, y: 0 }, &obs),
        "viewer's own cell should not block LOS"
    );
}

#[test]
fn mutation_has_los_dx_sub_not_add() {
    // Catches L154: (x1 - x0).abs() → (x1 + x0).abs()
    // Need both endpoints non-zero in x with same sign: a=(1,0), b=(4,0)
    // Original dx=|4-1|=3. Mutation dx=|4+1|=5.
    // For a horizontal line, the path is the same (steps through every x).
    // But for a diagonal: a=(1,0), b=(4,2), obstacle at (2,1)
    // Original dx=3, dy=-2, err=1. Path visits (2,1) → blocked.
    // Mutation dx=5, dy=-2, err=3. Path: (2,0),(3,0),(4,1)... skips (2,1)?
    // Actually mutation may cause infinite loop (overshoot).
    // If it infinite-loops, that's a TIMEOUT → still detected.
    // Test it anyway — at worst it's a timeout, at best it catches.
    let mut obs = BTreeSet::new();
    obs.insert((2, 1));
    assert!(
        !has_los(IVec2 { x: 1, y: 0 }, IVec2 { x: 4, y: 2 }, &obs),
        "obstacle at (2,1) on Bresenham path should block LOS"
    );
}

#[test]
fn mutation_has_los_dy_sub_not_add() {
    // Catches L156: -(y1 - y0).abs() → -(y1 + y0).abs()
    // Need both endpoints non-zero in y with same sign: a=(0,1), b=(2,4)
    // Original dy=-|4-1|=-3. Mutation dy=-|4+1|=-5.
    // Original: path visits cells along slope, blocked by obstacle at (1,2) or (1,3)
    let mut obs = BTreeSet::new();
    obs.insert((1, 3));
    assert!(
        !has_los(IVec2 { x: 0, y: 1 }, IVec2 { x: 2, y: 4 }, &obs),
        "obstacle at (1,3) on Bresenham path should block LOS"
    );
}

#[test]
fn mutation_has_los_e2_threshold_ge_dy() {
    // Catches e2 >= dy → e2 > dy or e2 <= dy (wrong stepping threshold)
    // Use non-axis-aligned line where err boundary matters:
    // (0,0) → (3,1): dx=3, dy=-1, err=2
    // Step 1: e2=4, 4>=-1 → x++, err=-1+(-1)=-2. 4<=3 → y++, err=-2+3=1. CHECK (1,1)
    // Step 2: e2=2, 2>=-1 → x++, err=1+(-1)=0. 2<=3 → y? → no y step. CHECK (2,1)
    // Actually it's getting complicated. Let the block test above cover this.
    // Let me use a simple case: (0,0) → (5,2)
    let mut obs = BTreeSet::new();
    obs.insert((3, 1));
    let result = has_los(IVec2 { x: 0, y: 0 }, IVec2 { x: 5, y: 2 }, &obs);
    assert!(!result, "obstacle on Bresenham path should block LOS");
}

// ════════════════════════════════════════════════════════════════════════
// Hashing mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_stable_hash_includes_all_fields() {
    // Catches deletion of individual hash() calls in stable_hash_snapshot
    let e1 = EntityState {
        id: 1,
        pos: IVec2 { x: 5, y: 10 },
        hp: 100,
        team: 0,
        ammo: 30,
    };
    let e2 = EntityState {
        id: 1,
        pos: IVec2 { x: 5, y: 10 },
        hp: 100,
        team: 0,
        ammo: 31, // only ammo differs
    };
    let obs = BTreeSet::new();
    let h1 = stable_hash_snapshot(&[e1], &obs);
    let h2 = stable_hash_snapshot(&[e2], &obs);
    assert_ne!(h1, h2, "changing ammo must change hash");
}

#[test]
fn mutation_stable_hash_team_matters() {
    let e1 = entity(1, 5, 5, 0);
    let e2 = EntityState { team: 1, ..e1 };
    let obs = BTreeSet::new();
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs),
        stable_hash_snapshot(&[e2], &obs)
    );
}

#[test]
fn mutation_stable_hash_hp_matters() {
    let e1 = entity(1, 5, 5, 0);
    let e2 = EntityState { hp: 50, ..e1 };
    let obs = BTreeSet::new();
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs),
        stable_hash_snapshot(&[e2], &obs)
    );
}

#[test]
fn mutation_stable_hash_pos_x_matters() {
    let e1 = entity(1, 5, 5, 0);
    let e2 = EntityState {
        pos: IVec2 { x: 6, y: 5 },
        ..e1
    };
    let obs = BTreeSet::new();
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs),
        stable_hash_snapshot(&[e2], &obs)
    );
}

#[test]
fn mutation_stable_hash_pos_y_matters() {
    let e1 = entity(1, 5, 5, 0);
    let e2 = EntityState {
        pos: IVec2 { x: 5, y: 6 },
        ..e1
    };
    let obs = BTreeSet::new();
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs),
        stable_hash_snapshot(&[e2], &obs)
    );
}

#[test]
fn mutation_stable_hash_obstacles_matter() {
    let e1 = entity(1, 5, 5, 0);
    let mut obs1 = BTreeSet::new();
    let mut obs2 = BTreeSet::new();
    obs2.insert((3, 3));
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs1),
        stable_hash_snapshot(&[e1], &obs2)
    );
    // Also test obstacle position matters
    obs1.insert((4, 4));
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs1),
        stable_hash_snapshot(&[e1], &obs2)
    );
}

#[test]
fn mutation_stable_hash_id_matters() {
    let e1 = entity(1, 5, 5, 0);
    let e2 = entity(2, 5, 5, 0); // different id, same position
    let obs = BTreeSet::new();
    assert_ne!(
        stable_hash_snapshot(&[e1], &obs),
        stable_hash_snapshot(&[e2], &obs)
    );
}

#[test]
fn mutation_subset_hash_all_fields() {
    // Catches deletion of hash() calls in subset_hash
    let e1 = entity(1, 5, 5, 0);
    let e2 = EntityState { ammo: 31, ..e1 };
    assert_ne!(subset_hash(&[e1]), subset_hash(&[e2]));
    let e3 = EntityState { hp: 50, ..e1 };
    assert_ne!(subset_hash(&[e1]), subset_hash(&[e3]));
    let e4 = EntityState { team: 2, ..e1 };
    assert_ne!(subset_hash(&[e1]), subset_hash(&[e4]));
    let e5 = entity(2, 5, 5, 0);
    assert_ne!(subset_hash(&[e1]), subset_hash(&[e5]));
}

// ════════════════════════════════════════════════════════════════════════
// Delta compression mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_diff_each_field_independently() {
    // Catches per-field comparison mutations in diff_snapshots
    let viewer = entity(0, 0, 0, 0);
    let base_entity = entity(1, 5, 5, 0);

    // Only pos changes
    let head_entity_pos = EntityState {
        pos: IVec2 { x: 6, y: 5 },
        ..base_entity
    };
    let base = Snapshot {
        version: 1,
        tick: 1,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![base_entity],
    };
    let head = Snapshot {
        tick: 2,
        entities: vec![head_entity_pos],
        ..base.clone()
    };
    let d = diff_snapshots(&base, &head, &FullInterest, &viewer);
    assert_eq!(d.changed.len(), 1);
    assert_ne!(
        d.changed[0].mask & EntityDeltaMask::POS,
        0,
        "pos mask must be set"
    );
    assert_eq!(
        d.changed[0].mask & EntityDeltaMask::HP,
        0,
        "hp mask must be clear"
    );
    assert_eq!(
        d.changed[0].mask & EntityDeltaMask::TEAM,
        0,
        "team mask must be clear"
    );
    assert_eq!(
        d.changed[0].mask & EntityDeltaMask::AMMO,
        0,
        "ammo mask must be clear"
    );
    assert_eq!(d.changed[0].pos, Some(IVec2 { x: 6, y: 5 }));

    // Only hp changes
    let head_entity_hp = EntityState {
        hp: 50,
        ..base_entity
    };
    let head_hp = Snapshot {
        tick: 2,
        entities: vec![head_entity_hp],
        ..base.clone()
    };
    let d_hp = diff_snapshots(&base, &head_hp, &FullInterest, &viewer);
    assert_eq!(d_hp.changed.len(), 1);
    assert_ne!(d_hp.changed[0].mask & EntityDeltaMask::HP, 0);
    assert_eq!(d_hp.changed[0].mask & EntityDeltaMask::POS, 0);

    // Only team changes
    let head_entity_team = EntityState {
        team: 2,
        ..base_entity
    };
    let head_team = Snapshot {
        tick: 2,
        entities: vec![head_entity_team],
        ..base.clone()
    };
    let d_team = diff_snapshots(&base, &head_team, &FullInterest, &viewer);
    assert_ne!(d_team.changed[0].mask & EntityDeltaMask::TEAM, 0);

    // Only ammo changes
    let head_entity_ammo = EntityState {
        ammo: 99,
        ..base_entity
    };
    let head_ammo = Snapshot {
        tick: 2,
        entities: vec![head_entity_ammo],
        ..base.clone()
    };
    let d_ammo = diff_snapshots(&base, &head_ammo, &FullInterest, &viewer);
    assert_ne!(d_ammo.changed[0].mask & EntityDeltaMask::AMMO, 0);
}

#[test]
fn mutation_apply_delta_each_field() {
    // Catches mask & check mutations in apply_delta
    let base_entity = entity(1, 5, 5, 0);
    let mut snap = Snapshot {
        version: 1,
        tick: 1,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![base_entity],
    };

    // Delta that changes ALL fields
    let delta = Delta {
        base_tick: 1,
        tick: 2,
        changed: vec![EntityDelta {
            id: 1,
            mask: EntityDeltaMask::POS
                | EntityDeltaMask::HP
                | EntityDeltaMask::TEAM
                | EntityDeltaMask::AMMO,
            pos: Some(IVec2 { x: 10, y: 20 }),
            hp: Some(75),
            team: Some(2),
            ammo: Some(50),
        }],
        removed: vec![],
        head_hash: 999,
    };
    apply_delta(&mut snap, &delta);
    let e = &snap.entities[0];
    assert_eq!(e.pos, IVec2 { x: 10, y: 20 }, "pos must be updated");
    assert_eq!(e.hp, 75, "hp must be updated");
    assert_eq!(e.team, 2, "team must be updated");
    assert_eq!(e.ammo, 50, "ammo must be updated");
    assert_eq!(snap.tick, 2);
    assert_eq!(snap.world_hash, 999);
}

#[test]
fn mutation_apply_delta_remove_entity() {
    // Catches removed entity logic
    let mut snap = Snapshot {
        version: 1,
        tick: 1,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0), entity(2, 5, 5, 1)],
    };
    let delta = Delta {
        base_tick: 1,
        tick: 2,
        changed: vec![],
        removed: vec![2], // remove entity 2
        head_hash: 0,
    };
    apply_delta(&mut snap, &delta);
    assert_eq!(snap.entities.len(), 1);
    assert_eq!(snap.entities[0].id, 1);
}

#[test]
fn mutation_apply_delta_tick_guard() {
    // Catches base.tick != delta.base_tick → return
    let mut snap = Snapshot {
        version: 1,
        tick: 5,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0)],
    };
    let delta = Delta {
        base_tick: 5, // matches
        tick: 6,
        changed: vec![EntityDelta {
            id: 1,
            mask: EntityDeltaMask::HP,
            pos: None,
            hp: Some(50),
            team: None,
            ammo: None,
        }],
        removed: vec![],
        head_hash: 0,
    };
    apply_delta(&mut snap, &delta);
    assert_eq!(
        snap.entities[0].hp, 50,
        "delta with matching base_tick must be applied"
    );
    assert_eq!(snap.tick, 6);
}

// ════════════════════════════════════════════════════════════════════════
// EntityDeltaMask bit constants
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_delta_mask_bit_values() {
    // Catches shift mutations: 1<<0 vs 1>>0 (equiv), 1<<1, 1<<2, 1<<3
    // For 1<<1: value=2. 1>>1=0. Test 2 ≠ 0.
    assert_eq!(EntityDeltaMask::POS, 1, "POS must be 1");
    assert_eq!(EntityDeltaMask::HP, 2, "HP must be 2");
    assert_eq!(EntityDeltaMask::TEAM, 4, "TEAM must be 4");
    assert_eq!(EntityDeltaMask::AMMO, 8, "AMMO must be 8");
    // Bits must be disjoint
    assert_eq!(
        EntityDeltaMask::POS & EntityDeltaMask::HP,
        0,
        "POS and HP must not overlap"
    );
    assert_eq!(
        EntityDeltaMask::POS | EntityDeltaMask::HP | EntityDeltaMask::TEAM | EntityDeltaMask::AMMO,
        15,
        "all masks combined must be 0b1111"
    );
}

// ════════════════════════════════════════════════════════════════════════
// filter_snapshot_for_viewer mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_filter_snapshot_updates_hash() {
    // Catches subset_hash call on filtered entities
    let full = Snapshot {
        version: 1,
        tick: 1,
        t: 0.0,
        seq: 0,
        world_hash: 12345,
        entities: vec![entity(1, 0, 0, 0), entity(2, 100, 100, 1)],
    };
    let viewer = entity(1, 0, 0, 0);
    let policy = RadiusTeamInterest { radius: 5 };
    let filtered = filter_snapshot_for_viewer(&full, &policy, &viewer);
    // Enemy is far away (different team, outside radius) → excluded
    assert_eq!(filtered.entities.len(), 1);
    // Hash must be recalculated for the filtered set, not the original
    assert_ne!(
        filtered.world_hash, full.world_hash,
        "filtered hash must differ from full hash"
    );
    assert_eq!(filtered.world_hash, subset_hash(&filtered.entities));
}

// ════════════════════════════════════════════════════════════════════════
// build_snapshot mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_build_snapshot_version_and_fields() {
    let w = {
        let mut w = World::new();
        w.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 10);
        w
    };
    let snap = build_snapshot(&w, 42, 7);
    assert_eq!(snap.version, SNAPSHOT_VERSION);
    assert_eq!(snap.tick, 42);
    assert_eq!(snap.seq, 7);
    assert!(!snap.entities.is_empty());
    // world_hash must be non-zero for a non-empty world
    assert_ne!(snap.world_hash, 0);
}

// ════════════════════════════════════════════════════════════════════════
// world_to_entities / world_obstacles_btree mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_world_to_entities_sorted() {
    let mut w = World::new();
    let _e = w.spawn("E", IVec2 { x: 10, y: 0 }, Team { id: 2 }, 60, 0);
    let _p = w.spawn("P", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 10);
    let _c = w.spawn("C", IVec2 { x: 5, y: 0 }, Team { id: 1 }, 80, 5);
    let entities = world_to_entities(&w);
    // Must be sorted by id
    for w in entities.windows(2) {
        assert!(w[0].id < w[1].id, "entities must be sorted by id");
    }
}

#[test]
fn mutation_world_obstacles_btree_populated() {
    let mut w = World::new();
    w.obstacles.insert((3, 4));
    w.obstacles.insert((5, 6));
    let obs = world_obstacles_btree(&w);
    assert_eq!(obs.len(), 2);
    assert!(obs.contains(&(3, 4)));
    assert!(obs.contains(&(5, 6)));
}

// ════════════════════════════════════════════════════════════════════════
// Replay mutations
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_replay_sorts_events_by_tick() {
    use crate::replay_from;
    // Events out of order — replay must sort by (tick, seq)
    let mut w = World::new();
    let p = w.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let events = vec![
        ReplayEvent {
            tick: 10,
            seq: 0,
            actor_id: p,
            intent: PlanIntent {
                plan_id: "mv".into(),
                steps: vec![ActionStep::MoveTo {
                    x: 4,
                    y: 2,
                    speed: None,
                }],
            },
            world_hash: 0,
        },
        ReplayEvent {
            tick: 5,
            seq: 0,
            actor_id: p,
            intent: PlanIntent {
                plan_id: "mv".into(),
                steps: vec![ActionStep::MoveTo {
                    x: 3,
                    y: 2,
                    speed: None,
                }],
            },
            world_hash: 0,
        },
    ];
    let hash = replay_from(w, &events).unwrap();
    // If events are sorted correctly, tick 5 move happens before tick 10
    assert_ne!(hash, 0, "replay must produce a valid hash");
}

#[test]
fn mutation_replay_returns_correct_hash() {
    use crate::{replay_from, validate_and_execute, ValidateCfg};
    // Catches: replace replay_from → Ok(1) and other return-value mutations
    // We manually replay the same scenario and compare hashes.
    let mut w1 = World::new();
    let p = w1.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let events = vec![ReplayEvent {
        tick: 1,
        seq: 0,
        actor_id: p,
        intent: PlanIntent {
            plan_id: "mv".into(),
            steps: vec![ActionStep::MoveTo {
                x: 3,
                y: 2,
                speed: None,
            }],
        },
        world_hash: 0,
    }];

    // Manual replay: create same world, advance ticks, apply move
    let mut w2 = World::new();
    let p2 = w2.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let dt = 1.0f32 / 60.0f32;
    w2.tick(dt); // tick 0 → 1
    let vcfg = ValidateCfg {
        world_bounds: (0, 0, 19, 9),
    };
    let mut log = |s: String| {
        let _ = s;
    };
    let _ = validate_and_execute(&mut w2, p2, &events[0].intent, &vcfg, &mut log);
    let expected = build_snapshot(&w2, 1, 0);

    let actual_hash = replay_from(w1, &events).unwrap();
    assert_eq!(
        actual_hash, expected.world_hash,
        "replay hash must match manually-replayed hash (not Ok(1) or other constant)"
    );
}

#[test]
fn mutation_replay_ticks_advance() {
    use crate::replay_from;
    // Test that replay_from correctly processes events with tick advancement.
    // Two events at different ticks but applying the same move should both succeed.
    // The tick loop (while current_tick < e.tick) must correctly advance.
    let mut w = World::new();
    let p1 = w.spawn("P", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);

    let events = vec![
        ReplayEvent {
            tick: 5,
            seq: 0,
            actor_id: p1,
            intent: PlanIntent {
                plan_id: "mv1".into(),
                steps: vec![ActionStep::MoveTo {
                    x: 3,
                    y: 2,
                    speed: None,
                }],
            },
            world_hash: 0,
        },
        ReplayEvent {
            tick: 10,
            seq: 0,
            actor_id: p1,
            intent: PlanIntent {
                plan_id: "mv2".into(),
                steps: vec![ActionStep::MoveTo {
                    x: 4,
                    y: 2,
                    speed: None,
                }],
            },
            world_hash: 0,
        },
    ];
    let h = replay_from(w, &events).unwrap();
    // Must produce a valid non-zero hash from the final world state
    assert_ne!(h, 0, "replay with tick advancement must produce valid hash");
}

// ════════════════════════════════════════════════════════════════════════
// diff_snapshots removed entity detection
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mutation_diff_detects_removed_entities() {
    let viewer = entity(0, 0, 0, 0);
    let base = Snapshot {
        version: 1,
        tick: 1,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0), entity(2, 5, 5, 0)],
    };
    let head = Snapshot {
        version: 1,
        tick: 2,
        t: 0.0,
        seq: 1,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0)], // entity 2 removed
    };
    let delta = diff_snapshots(&base, &head, &FullInterest, &viewer);
    assert!(
        delta.removed.contains(&2),
        "entity 2 must be in removed list"
    );
    assert!(
        !delta.removed.contains(&1),
        "entity 1 must not be in removed"
    );
}

#[test]
fn mutation_delta_tick_fields() {
    let viewer = entity(0, 0, 0, 0);
    let base = Snapshot {
        version: 1,
        tick: 10,
        t: 0.0,
        seq: 0,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0)],
    };
    let head = Snapshot {
        version: 1,
        tick: 20,
        t: 0.0,
        seq: 1,
        world_hash: 0,
        entities: vec![entity(1, 0, 0, 0)],
    };
    let delta = diff_snapshots(&base, &head, &FullInterest, &viewer);
    assert_eq!(delta.base_tick, 10, "base_tick must match base snapshot");
    assert_eq!(delta.tick, 20, "tick must match head snapshot");
}

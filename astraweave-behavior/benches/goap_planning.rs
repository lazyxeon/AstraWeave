use astraweave_behavior::goap::*;
use astraweave_behavior::goap_cache::*; // Week 3 Action 9
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// Benchmark GOAP planning with a simple scenario
fn bench_goap_planning_simple(c: &mut Criterion) {
    c.bench_function("goap_planning_simple", |b| {
        // Initial state: agent has no weapon, is not at cover
        let initial = WorldState::from_facts(&[
            ("has_weapon", false),
            ("at_cover", false),
            ("enemy_visible", true),
        ]);

        // Goal: be at cover and have weapon
        let goal = GoapGoal::new(
            "combat_ready",
            WorldState::from_facts(&[("has_weapon", true), ("at_cover", true)]),
        );

        // Available actions
        let actions = vec![
            GoapAction::new("pick_up_weapon")
                .with_cost(1.0)
                .with_precondition("has_weapon", false)
                .with_effect("has_weapon", true),
            GoapAction::new("move_to_cover")
                .with_cost(2.0)
                .with_precondition("at_cover", false)
                .with_effect("at_cover", true),
        ];

        let planner = GoapPlanner::new();

        b.iter(|| {
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            black_box(plan)
        });
    });
}

/// Benchmark GOAP planning with moderate complexity (10 actions)
fn bench_goap_planning_moderate(c: &mut Criterion) {
    c.bench_function("goap_planning_10_actions", |b| {
        // More complex state space
        let initial = WorldState::from_facts(&[
            ("has_weapon", false),
            ("has_ammo", false),
            ("at_cover", false),
            ("enemy_visible", true),
            ("low_health", true),
        ]);

        let goal = GoapGoal::new(
            "tactical_ready",
            WorldState::from_facts(&[
                ("has_weapon", true),
                ("has_ammo", true),
                ("at_cover", true),
                ("low_health", false),
            ]),
        );

        // 10 available actions with dependencies
        let actions = vec![
            GoapAction::new("pick_up_weapon")
                .with_cost(1.0)
                .with_precondition("has_weapon", false)
                .with_effect("has_weapon", true),
            GoapAction::new("pick_up_ammo")
                .with_cost(1.0)
                .with_precondition("has_ammo", false)
                .with_effect("has_ammo", true),
            GoapAction::new("move_to_cover")
                .with_cost(2.0)
                .with_precondition("at_cover", false)
                .with_effect("at_cover", true),
            GoapAction::new("use_medkit")
                .with_cost(3.0)
                .with_precondition("low_health", true)
                .with_effect("low_health", false),
            GoapAction::new("reload_weapon")
                .with_cost(1.5)
                .with_precondition("has_weapon", true)
                .with_precondition("has_ammo", true)
                .with_effect("weapon_loaded", true),
            GoapAction::new("aim_at_enemy")
                .with_cost(0.5)
                .with_precondition("has_weapon", true)
                .with_effect("aiming", true),
            GoapAction::new("sprint")
                .with_cost(1.0)
                .with_effect("moving_fast", true),
            GoapAction::new("crouch")
                .with_cost(0.5)
                .with_effect("crouched", true),
            GoapAction::new("scan_area")
                .with_cost(1.0)
                .with_effect("area_scanned", true),
            GoapAction::new("call_for_backup")
                .with_cost(2.0)
                .with_effect("backup_called", true),
        ];

        let planner = GoapPlanner::new();

        b.iter(|| {
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            black_box(plan)
        });
    });
}

/// Benchmark GOAP planning with complex scenario (20 actions)
fn bench_goap_planning_complex(c: &mut Criterion) {
    c.bench_function("goap_planning_20_actions", |b| {
        // Complex tactical scenario
        let initial = WorldState::from_facts(&[
            ("has_weapon", false),
            ("has_ammo", false),
            ("at_cover", false),
            ("enemy_visible", true),
            ("low_health", true),
            ("team_nearby", false),
        ]);

        let goal = GoapGoal::new(
            "combat_ready_complex",
            WorldState::from_facts(&[
                ("has_weapon", true),
                ("has_ammo", true),
                ("at_cover", true),
                ("low_health", false),
                ("ready_to_engage", true),
            ]),
        );

        // 20 tactical actions
        let actions = vec![
            GoapAction::new("pick_up_weapon")
                .with_cost(1.0)
                .with_effect("has_weapon", true),
            GoapAction::new("pick_up_ammo")
                .with_cost(1.0)
                .with_effect("has_ammo", true),
            GoapAction::new("move_to_cover")
                .with_cost(2.0)
                .with_effect("at_cover", true),
            GoapAction::new("use_medkit")
                .with_cost(3.0)
                .with_precondition("low_health", true)
                .with_effect("low_health", false),
            GoapAction::new("reload_weapon")
                .with_cost(1.5)
                .with_precondition("has_weapon", true)
                .with_effect("weapon_loaded", true),
            GoapAction::new("aim_at_enemy")
                .with_cost(0.5)
                .with_precondition("has_weapon", true)
                .with_effect("aiming", true),
            GoapAction::new("sprint")
                .with_cost(1.0)
                .with_effect("moving_fast", true),
            GoapAction::new("crouch")
                .with_cost(0.5)
                .with_effect("crouched", true),
            GoapAction::new("scan_area")
                .with_cost(1.0)
                .with_effect("area_scanned", true),
            GoapAction::new("call_for_backup")
                .with_cost(2.0)
                .with_effect("backup_called", true),
            GoapAction::new("throw_grenade")
                .with_cost(2.5)
                .with_precondition("has_weapon", true)
                .with_effect("grenade_thrown", true),
            GoapAction::new("take_tactical_position")
                .with_cost(1.5)
                .with_effect("tactical_position", true),
            GoapAction::new("suppress_fire")
                .with_cost(2.0)
                .with_precondition("has_ammo", true)
                .with_effect("suppressing", true),
            GoapAction::new("flank_enemy")
                .with_cost(3.0)
                .with_effect("flanking", true),
            GoapAction::new("check_corners")
                .with_cost(1.0)
                .with_effect("corners_clear", true),
            GoapAction::new("mark_target")
                .with_cost(0.5)
                .with_effect("target_marked", true),
            GoapAction::new("switch_weapon")
                .with_cost(1.0)
                .with_precondition("has_weapon", true)
                .with_effect("weapon_switched", true),
            GoapAction::new("deploy_shield")
                .with_cost(2.0)
                .with_effect("shield_deployed", true),
            GoapAction::new("prepare_ambush")
                .with_cost(2.5)
                .with_effect("ambush_ready", true),
            GoapAction::new("confirm_ready")
                .with_cost(0.5)
                .with_precondition("has_weapon", true)
                .with_precondition("has_ammo", true)
                .with_precondition("at_cover", true)
                .with_effect("ready_to_engage", true),
        ];

        let planner = GoapPlanner::new();

        b.iter(|| {
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            black_box(plan)
        });
    });
}

/// Benchmark goal evaluation
fn bench_goal_evaluation(c: &mut Criterion) {
    c.bench_function("goap_goal_evaluation", |b| {
        let state = WorldState::from_facts(&[
            ("has_weapon", true),
            ("has_ammo", true),
            ("at_cover", false),
        ]);

        let goal = GoapGoal::new(
            "combat_ready",
            WorldState::from_facts(&[("has_weapon", true), ("has_ammo", true), ("at_cover", true)]),
        )
        .with_priority(10.0);

        b.iter(|| {
            let satisfied = goal.is_satisfied(black_box(&state));
            black_box(satisfied)
        });
    });
}

/// Benchmark action precondition checking
fn bench_action_preconditions(c: &mut Criterion) {
    c.bench_function("goap_action_preconditions", |b| {
        let state = WorldState::from_facts(&[
            ("has_weapon", true),
            ("has_ammo", true),
            ("weapon_loaded", false),
        ]);

        let action = GoapAction::new("reload_weapon")
            .with_cost(1.5)
            .with_precondition("has_weapon", true)
            .with_precondition("has_ammo", true)
            .with_precondition("weapon_loaded", false)
            .with_effect("weapon_loaded", true);

        b.iter(|| {
            let can_apply = action.can_apply(black_box(&state));
            black_box(can_apply)
        });
    });
}

/// Benchmark cached GOAP planner with cold cache (first access)
fn bench_goap_caching_cold(c: &mut Criterion) {
    c.bench_function("goap_caching_cold_cache", |b| {
        // Complex scenario (15 actions) to show cache benefit
        let initial = WorldState::from_facts(&[
            ("has_weapon", false),
            ("has_ammo", false),
            ("at_cover", false),
            ("enemy_visible", true),
            ("low_health", true),
            ("has_grenade", false),
        ]);

        let goal = GoapGoal::new(
            "tactical_ready",
            WorldState::from_facts(&[
                ("has_weapon", true),
                ("has_ammo", true),
                ("at_cover", true),
                ("low_health", false),
            ]),
        );

        let actions = create_complex_action_set();

        b.iter(|| {
            // Create new planner each iteration (cold cache)
            let mut planner = CachedGoapPlanner::new(100);
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            black_box(plan)
        });
    });
}

/// Benchmark cached GOAP planner with warm cache (90% hit rate scenario)
fn bench_goap_caching_warm(c: &mut Criterion) {
    c.bench_function("goap_caching_warm_cache_90pct", |b| {
        // Pre-warm cache with 10 common scenarios
        let mut planner = CachedGoapPlanner::new(100);
        let scenarios = create_warm_cache_scenarios();

        // Prime cache
        for (state, goal, actions) in &scenarios {
            planner.plan(state, goal, actions);
        }

        // Benchmark: 90% scenarios from cache, 10% new
        let mut iteration = 0;
        b.iter(|| {
            iteration += 1;
            let scenario_idx = if iteration % 10 == 0 {
                // 10% miss - new scenario
                scenarios.len() % 20 // Use modulo to avoid out of bounds
            } else {
                // 90% hit - cached scenario
                iteration % scenarios.len()
            };

            let (state, goal, actions) = &scenarios[scenario_idx % scenarios.len()];
            let plan = planner.plan(black_box(state), black_box(goal), black_box(actions));
            black_box(plan)
        });
    });
}

/// Benchmark cache hit vs miss comparison
fn bench_goap_cache_hit_vs_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("goap_cache_comparison");

    let initial = WorldState::from_facts(&[
        ("has_weapon", false),
        ("has_ammo", false),
        ("at_cover", false),
    ]);

    let goal = GoapGoal::new(
        "ready",
        WorldState::from_facts(&[("has_weapon", true), ("at_cover", true)]),
    );

    let actions = create_complex_action_set();

    // Benchmark cache miss (first access)
    group.bench_function("cache_miss", |b| {
        let mut planner = CachedGoapPlanner::new(100);
        planner.plan(&initial, &goal, &actions); // Prime cache

        // Clear cache to force miss
        planner.clear_cache();

        b.iter(|| {
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            planner.clear_cache(); // Force miss every iteration
            black_box(plan)
        });
    });

    // Benchmark cache hit
    group.bench_function("cache_hit", |b| {
        let mut planner = CachedGoapPlanner::new(100);
        planner.plan(&initial, &goal, &actions); // Prime cache

        b.iter(|| {
            let plan = planner.plan(black_box(&initial), black_box(&goal), black_box(&actions));
            black_box(plan)
        });
    });

    group.finish();
}

// Helper function to create complex action set (15 actions)
fn create_complex_action_set() -> Vec<GoapAction> {
    vec![
        GoapAction::new("pick_up_weapon").with_effect("has_weapon", true),
        GoapAction::new("pick_up_ammo").with_effect("has_ammo", true),
        GoapAction::new("move_to_cover").with_effect("at_cover", true),
        GoapAction::new("use_medkit").with_effect("low_health", false),
        GoapAction::new("reload")
            .with_precondition("has_ammo", true)
            .with_effect("weapon_loaded", true),
        GoapAction::new("aim")
            .with_precondition("has_weapon", true)
            .with_effect("aiming", true),
        GoapAction::new("sprint").with_effect("moving_fast", true),
        GoapAction::new("crouch").with_effect("crouched", true),
        GoapAction::new("scan_area").with_effect("area_scanned", true),
        GoapAction::new("throw_grenade")
            .with_precondition("has_grenade", true)
            .with_effect("grenade_thrown", true),
        GoapAction::new("find_grenade").with_effect("has_grenade", true),
        GoapAction::new("call_backup").with_effect("backup_called", true),
        GoapAction::new("flank_enemy").with_effect("flanking", true),
        GoapAction::new("retreat").with_effect("retreating", true),
        GoapAction::new("advance").with_effect("advancing", true),
    ]
}

// Helper function to create warm cache scenarios
fn create_warm_cache_scenarios() -> Vec<(WorldState, GoapGoal, Vec<GoapAction>)> {
    let actions = create_complex_action_set();

    vec![
        // Scenario 1: Need weapon and cover
        (
            WorldState::from_facts(&[("has_weapon", false), ("at_cover", false)]),
            GoapGoal::new(
                "ready",
                WorldState::from_facts(&[("has_weapon", true), ("at_cover", true)]),
            ),
            actions.clone(),
        ),
        // Scenario 2: Need ammo
        (
            WorldState::from_facts(&[("has_weapon", true), ("has_ammo", false)]),
            GoapGoal::new("reload", WorldState::from_facts(&[("weapon_loaded", true)])),
            actions.clone(),
        ),
        // Scenario 3: Need healing
        (
            WorldState::from_facts(&[("low_health", true)]),
            GoapGoal::new("heal", WorldState::from_facts(&[("low_health", false)])),
            actions.clone(),
        ),
        // Scenario 4: Need grenade
        (
            WorldState::from_facts(&[("has_grenade", false)]),
            GoapGoal::new(
                "grenade_ready",
                WorldState::from_facts(&[("has_grenade", true)]),
            ),
            actions.clone(),
        ),
        // Scenario 5: Already at goal (cache empty plan)
        (
            WorldState::from_facts(&[("has_weapon", true), ("at_cover", true)]),
            GoapGoal::new(
                "ready",
                WorldState::from_facts(&[("has_weapon", true), ("at_cover", true)]),
            ),
            actions.clone(),
        ),
    ]
}

criterion_group!(
    benches,
    bench_goap_planning_simple,
    bench_goap_planning_moderate,
    bench_goap_planning_complex,
    bench_goal_evaluation,
    bench_action_preconditions,
    bench_goap_caching_cold,
    bench_goap_caching_warm,
    bench_goap_cache_hit_vs_miss,
);
criterion_main!(benches);

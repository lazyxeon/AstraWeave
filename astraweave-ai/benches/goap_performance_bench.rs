#[cfg(feature = "planner_advanced")]
use astraweave_ai::goap::*;
#[cfg(feature = "planner_advanced")]
use criterion::BenchmarkId;
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "planner_advanced")]
use std::collections::BTreeMap;
use std::hint::black_box;

#[cfg(feature = "planner_advanced")]
fn create_test_planner() -> AdvancedGOAP {
    let mut planner = AdvancedGOAP::new();
    register_all_actions(&mut planner);
    planner
}

#[cfg(feature = "planner_advanced")]
fn create_test_world() -> WorldState {
    let mut world = WorldState::new();
    world.set("my_ammo", StateValue::Int(30));
    world.set("my_hp", StateValue::Int(100));
    world.set("my_x", StateValue::Int(0));
    world.set("my_y", StateValue::Int(0));
    world.set("enemy_x", StateValue::Int(10));
    world.set("enemy_y", StateValue::Int(10));
    world.set("enemy_hp", StateValue::Int(100));
    world.set("in_combat", StateValue::Bool(true));
    world
}

#[cfg(feature = "planner_advanced")]
fn create_simple_goal() -> Goal {
    let mut desired = BTreeMap::new();
    desired.insert("enemy_defeated".to_string(), StateValue::Bool(true));
    Goal::new("defeat_enemy", desired).with_priority(8.0)
}

#[cfg(feature = "planner_advanced")]
fn create_hierarchical_goal(depth: usize) -> Goal {
    let mut desired = BTreeMap::new();
    desired.insert(format!("level_{}_complete", depth), StateValue::Bool(true));

    let mut goal = Goal::new(format!("level_{}", depth), desired).with_priority(8.0);

    if depth > 0 {
        let sub_goal = create_hierarchical_goal(depth - 1);
        goal = goal.with_sub_goals(vec![sub_goal]);
    }

    goal
}

#[cfg(feature = "planner_advanced")]
fn bench_simple_planning(c: &mut Criterion) {
    let planner = create_test_planner();
    let world = create_test_world();
    let goal = create_simple_goal();

    c.bench_function("simple_plan", |b| {
        b.iter(|| planner.plan(black_box(&world), black_box(&goal)));
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_hierarchical_planning(c: &mut Criterion) {
    let mut group = c.benchmark_group("hierarchical_plan");
    let planner = create_test_planner();
    let world = create_test_world();

    for depth in [1, 2, 3].iter() {
        let goal = create_hierarchical_goal(*depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &_depth| {
            b.iter(|| planner.plan(black_box(&world), black_box(&goal)));
        });
    }
    group.finish();
}

#[cfg(feature = "planner_advanced")]
fn bench_goal_validation(c: &mut Criterion) {
    let validator = GoalValidator::new();

    let mut desired = BTreeMap::new();
    desired.insert("test".to_string(), StateValueDef::Bool(true));

    let goal_def = GoalDefinition {
        name: "test_goal".to_string(),
        priority: Some(5.0),
        deadline_seconds: Some(60.0),
        decomposition: Some("sequential".to_string()),
        max_depth: Some(5),
        desired_state: desired,
        sub_goals: None,
    };

    c.bench_function("goal_validation", |b| {
        b.iter(|| validator.validate(black_box(&goal_def)));
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_plan_visualization(c: &mut Criterion) {
    let planner = create_test_planner();
    let world = create_test_world();
    let goal = create_simple_goal();
    let plan = planner.plan(&world, &goal).unwrap();

    let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
    let actions: Vec<Box<dyn Action>> = vec![]; // Would need actual actions
    let history = ActionHistory::new();

    c.bench_function("plan_visualization", |b| {
        b.iter(|| {
            visualizer.visualize_plan(
                black_box(&plan),
                black_box(&actions),
                black_box(&history),
                black_box(&world),
            )
        });
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_plan_analysis(c: &mut Criterion) {
    let planner = create_test_planner();
    let world = create_test_world();
    let goal = create_simple_goal();
    let plan = planner.plan(&world, &goal).unwrap();

    let actions: Vec<Box<dyn Action>> = vec![]; // Would need actual actions
    let history = ActionHistory::new();

    c.bench_function("plan_analysis", |b| {
        b.iter(|| {
            PlanAnalyzer::analyze(
                black_box(&plan),
                black_box(&actions),
                black_box(&history),
                black_box(&world),
            )
        });
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_goal_scheduler(c: &mut Criterion) {
    let planner = create_test_planner();
    let world = create_test_world();
    let mut scheduler = GoalScheduler::new();

    // Add multiple goals
    for i in 0..5 {
        let mut desired = BTreeMap::new();
        desired.insert(format!("goal_{}", i), StateValue::Bool(true));
        let goal = Goal::new(format!("goal_{}", i), desired).with_priority(i as f32);
        scheduler.add_goal(goal);
    }

    c.bench_function("goal_scheduler_update", |b| {
        b.iter(|| scheduler.update(black_box(0.0), black_box(&world), black_box(&planner)));
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_world_state_operations(c: &mut Criterion) {
    let mut world = WorldState::new();

    c.bench_function("worldstate_set", |b| {
        b.iter(|| world.set(black_box("test_key"), black_box(StateValue::Int(42))));
    });

    world.set("test_key", StateValue::Int(42));
    c.bench_function("worldstate_get", |b| {
        b.iter(|| world.get(black_box("test_key")));
    });

    let mut effects = BTreeMap::new();
    effects.insert("key1".to_string(), StateValue::Int(10));
    effects.insert("key2".to_string(), StateValue::Bool(true));

    c.bench_function("worldstate_apply_effects", |b| {
        b.iter(|| world.apply_effects(black_box(&effects)));
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_action_history(c: &mut Criterion) {
    let mut history = ActionHistory::new();

    c.bench_function("history_record_success", |b| {
        b.iter(|| history.record_success(black_box("test_action"), black_box(1.0)));
    });

    c.bench_function("history_get_stats", |b| {
        b.iter(|| history.get_action_stats(black_box("test_action")));
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_learning_manager(c: &mut Criterion) {
    let config = GOAPConfig::default();
    let mut manager = LearningManager::new(config);
    let mut history = ActionHistory::new();

    // Add some history
    for _ in 0..10 {
        history.record_success("test_action", 1.0);
    }

    c.bench_function("learning_get_probability", |b| {
        b.iter(|| manager.get_probability(black_box("test_action"), black_box(&history)));
    });
}

#[cfg(feature = "planner_advanced")]
criterion_group!(
    benches,
    bench_simple_planning,
    bench_hierarchical_planning,
    bench_goal_validation,
    bench_plan_visualization,
    bench_plan_analysis,
    bench_goal_scheduler,
    bench_world_state_operations,
    bench_action_history,
    bench_learning_manager,
);

#[cfg(not(feature = "planner_advanced"))]
fn bench_feature_disabled(c: &mut Criterion) {
    c.bench_function("planner_advanced_feature_disabled", |b| {
        b.iter(|| black_box(0))
    });
}

#[cfg(not(feature = "planner_advanced"))]
criterion_group!(benches, bench_feature_disabled);

criterion_main!(benches);

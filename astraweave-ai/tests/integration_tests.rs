//! Phase 4: Integrated AI Loop Tests
//!
//! Tests full Perception → Planning → Action loop under realistic conditions.
//!
//! **Success Criteria**:
//! - ✅ 95% of frames complete within 16.67ms budget (60 FPS)
//! - ✅ Boss AI maintains <10ms planning time under stress
//! - ✅ Squad coordination produces valid tactical plans

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;
use std::time::Instant;

/// Create a combat scenario snapshot
fn create_combat_snapshot(frame: usize, agent_id: usize) -> WorldSnapshot {
    let mut enemies = vec![];
    for i in 0..5 {
        enemies.push(EnemyState {
            id: (100 + i) as u32,
            pos: IVec2 {
                x: 20 + (i as i32) * 3,
                y: 15 + ((frame + i) % 3) as i32,
            },
            hp: 50 + (i as i32) * 10,
            cover: if i % 2 == 0 { CoverType::Low } else { CoverType::High },
            last_seen: (i as f32) * 0.5,
        });
    }

    WorldSnapshot {
        t: frame as f32,
        me: CompanionState {
            ammo: 10 - (agent_id % 3) as i32,
            cooldowns: BTreeMap::new(),
            morale: 1.0 - (agent_id as f32 * 0.01),
            pos: IVec2 {
                x: 10 + (agent_id % 5) as i32,
                y: 10 + (agent_id / 5) as i32,
            },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 10, y: 10 },
            stance: "stand".into(),
            orders: vec!["advance".to_string()],
        },
        enemies,
        pois: vec![],
        obstacles: vec![],
        objective: Some("eliminate_enemies".to_string()),
    }
}

#[test]
fn test_full_ai_loop_60fps() {
    println!("\n=== TEST: Full AI Loop @ 60 FPS (676 agents) ===");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let agent_count = 676;
    let frame_count = 100; // 100 frames = ~1.67 seconds @ 60 FPS
    let target_frame_time_ms = 16.67; // 60 FPS budget

    let mut frame_times = vec![];

    for frame in 0..frame_count {
        let frame_start = Instant::now();

        // Perception Phase: Create snapshots for all agents
        let snapshots: Vec<_> = (0..agent_count)
            .map(|agent_id| create_combat_snapshot(frame, agent_id))
            .collect();

        // Planning Phase: Plan for all agents
        for snapshot in &snapshots {
            let _plan = dispatch_planner(&controller, snapshot).expect("Should produce plan");
        }

        // Action Phase: (simulated - just validate plans are generated)
        // In real implementation, this would apply actions to ECS

        let frame_time = frame_start.elapsed();
        frame_times.push(frame_time.as_secs_f64() * 1000.0);
    }

    // Calculate statistics
    let total_avg: f64 = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
    let min_time = frame_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_time = frame_times
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // Count frames within budget
    let within_budget = frame_times
        .iter()
        .filter(|&&t| t < target_frame_time_ms)
        .count();
    let budget_percentage = (within_budget as f64 / frame_count as f64) * 100.0;

    println!("   Agents: {}", agent_count);
    println!("   Frames: {}", frame_count);
    println!("   Avg frame time: {:.3} ms", total_avg);
    println!("   Min frame time: {:.3} ms", min_time);
    println!("   Max frame time: {:.3} ms", max_time);
    println!(
        "   Within budget: {}/{} ({:.1}%)",
        within_budget, frame_count, budget_percentage
    );

    // Validate >95% frames within budget
    assert!(
        budget_percentage >= 95.0,
        "Should have >95% frames within budget, got {:.1}%",
        budget_percentage
    );

    println!(
        "✅ 60 FPS target met: {:.1}% frames < {:.2} ms",
        budget_percentage, target_frame_time_ms
    );
}

#[test]
fn test_boss_ai_stress() {
    println!("\n=== TEST: Boss AI Stress (Complex Planning Under Load) ===");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Simulate boss AI with complex decision-making
    let iterations = 1_000;
    let target_time_ms = 10.0; // 10ms per plan target

    let mut plan_times = vec![];

    for i in 0..iterations {
        let snapshot = create_combat_snapshot(i, 0); // Boss is agent 0

        let start = Instant::now();
        let _plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");
        let duration = start.elapsed();

        plan_times.push(duration.as_secs_f64() * 1000.0);
    }

    // Calculate statistics
    let avg_time: f64 = plan_times.iter().sum::<f64>() / plan_times.len() as f64;
    let max_time = plan_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let p95_time = {
        let mut sorted = plan_times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[(sorted.len() as f64 * 0.95) as usize]
    };

    println!("   Iterations: {}", iterations);
    println!("   Avg plan time: {:.3} ms", avg_time);
    println!("   Max plan time: {:.3} ms", max_time);
    println!("   P95 plan time: {:.3} ms", p95_time);

    // Validate <10ms average
    assert!(
        avg_time < target_time_ms,
        "Boss AI should plan in <{}ms, got {:.3}ms",
        target_time_ms,
        avg_time
    );

    println!(
        "✅ Boss AI stress: {:.3} ms < {} ms",
        avg_time, target_time_ms
    );
}

#[test]
fn test_multi_agent_coordination() {
    println!("\n=== TEST: Multi-Agent Coordination (Squad Tactics) ===");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Create a squad of 4 agents
    let squad_size = 4;
    let mut squad_plans = vec![];

    for agent_id in 0..squad_size {
        let snapshot = create_combat_snapshot(0, agent_id);
        let plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");
        squad_plans.push(plan);
    }

    println!("   Squad size: {}", squad_size);
    for (i, plan) in squad_plans.iter().enumerate() {
        println!(
            "     Agent {}: {} steps (plan_id: {})",
            i,
            plan.steps.len(),
            plan.plan_id
        );
    }

    // Validate all agents produced plans
    assert_eq!(
        squad_plans.len(),
        squad_size,
        "All squad members should have plans"
    );

    // Validate plans are non-empty for combat scenarios
    let non_empty_plans = squad_plans.iter().filter(|p| !p.steps.is_empty()).count();
    assert!(
        non_empty_plans > 0,
        "At least some agents should have action steps"
    );

    println!(
        "✅ Squad coordination: {}/{} agents with action plans",
        non_empty_plans, squad_size
    );
}

#[test]
fn test_perception_planning_pipeline() {
    println!("\n=== TEST: Perception → Planning Pipeline ===");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let agent_count = 100;
    let start = Instant::now();

    // Phase 1: Perception (create snapshots)
    let perception_start = Instant::now();
    let snapshots: Vec<_> = (0..agent_count)
        .map(|agent_id| create_combat_snapshot(0, agent_id))
        .collect();
    let perception_time = perception_start.elapsed();

    // Phase 2: Planning (generate plans)
    let planning_start = Instant::now();
    let plans: Vec<_> = snapshots
        .iter()
        .map(|snapshot| dispatch_planner(&controller, snapshot).expect("Should produce plan"))
        .collect();
    let planning_time = planning_start.elapsed();

    let total_time = start.elapsed();

    println!("   Agents: {}", agent_count);
    println!("   Perception: {:?}", perception_time);
    println!("   Planning: {:?}", planning_time);
    println!("   Total: {:?}", total_time);
    println!(
        "   Perception %: {:.1}%",
        (perception_time.as_secs_f64() / total_time.as_secs_f64()) * 100.0
    );
    println!(
        "   Planning %: {:.1}%",
        (planning_time.as_secs_f64() / total_time.as_secs_f64()) * 100.0
    );

    assert_eq!(plans.len(), agent_count);

    println!("✅ Pipeline validated: {} agents processed", agent_count);
}

#[test]
fn test_ai_loop_memory_efficiency() {
    println!("\n=== TEST: AI Loop Memory Efficiency ===");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Test that repeated planning doesn't leak memory (simple check)
    let iterations = 10_000;
    let start = Instant::now();

    for i in 0..iterations {
        let snapshot = create_combat_snapshot(i, 0);
        let _plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");
        // Plan is dropped here, memory should be reclaimed
    }

    let duration = start.elapsed();
    let per_iteration_us = (duration.as_micros() as f64) / iterations as f64;

    println!("   Iterations: {}", iterations);
    println!("   Total time: {:?}", duration);
    println!("   Per-iteration: {:.3} µs", per_iteration_us);

    // Should maintain consistent performance (no memory leaks slowing down)
    assert!(
        per_iteration_us < 10.0,
        "Performance should remain stable, got {:.3} µs",
        per_iteration_us
    );

    println!(
        "✅ Memory efficiency validated: stable performance over {} iterations",
        iterations
    );
}

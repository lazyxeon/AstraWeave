//! hello_companion - Advanced AI Showcase with Qwen3-8B + Phase 7 Features
//!
//! Demonstrates 7 AI modes + Phase 7 enhancements:
//! 1. Classical (RuleOrchestrator - baseline)
//! 2. BehaviorTree (Hierarchical reasoning)
//! 3. Utility (Score-based selection)
//! 4. LLM (Qwen3-8B via Ollama with Phase 7 enhancements)
//! 5. Hybrid (LLM with Classical fallback)
//! 6. Ensemble (Voting across all modes)
//! 7. Arbiter (GOAP + Qwen3 Hybrid - instant control + strategic planning)
//!
//! Phase 7 Features Showcased:
//! - 37-tool vocabulary (expanded from 4)
//! - 4-tier fallback system (Full LLM → Simplified → Heuristic → Emergency)
//! - Semantic cache similarity matching (Jaccard algorithm)
//! - 5-stage JSON parsing with hallucination detection
//! - Enhanced prompt engineering with few-shot learning
//! - GOAP+Hermes Arbiter: Zero user-facing latency via async LLM planning
//!
//! Usage:
//!   cargo run -p hello_companion --release                                        # Classical (default)
//!   cargo run -p hello_companion --release --features llm,ollama                 # Qwen3-8B
//!   cargo run -p hello_companion --release --features llm,ollama -- --bt         # BehaviorTree
//!   cargo run -p hello_companion --release --features llm,ollama -- --utility    # Utility AI
//!   cargo run -p hello_companion --release --features llm,ollama -- --hybrid     # LLM + fallback
//!   cargo run -p hello_companion --release --features llm,ollama -- --ensemble   # All modes voting
//!   cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter  # GOAP + Qwen3 Arbiter
//!   cargo run -p hello_companion --release --features visual                     # Visual demo with 3 AI modes
//!   cargo run -p hello_companion --release --features llm,ollama,metrics -- --demo-all --metrics --export-metrics --phase7

// Suppress dead code warnings for CLI-mode code when visual feature is active
#![cfg_attr(feature = "visual", allow(dead_code))]

// Global allocator selection — opt-in mimalloc. No-op when `fast-alloc` is off.
#[cfg(feature = "fast-alloc")]
astraweave_alloc::setup_global_allocator!();

// Visual demo modules (enabled with --features visual)
#[cfg(feature = "visual")]
mod chat_ui;
#[cfg(feature = "visual")]
mod dialogue_bank;
#[cfg(feature = "visual")]
mod llm_worker;
#[cfg(feature = "visual")]
mod scene;
#[cfg(feature = "visual")]
mod visual_demo;

use astraweave_ai::{Orchestrator, RuleOrchestrator};

#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::GoapOrchestrator;

use astraweave_core::ecs_adapter::build_app;
use astraweave_core::ecs_bridge::EntityBridge;
use astraweave_core::{
    build_snapshot, validate_and_execute, IVec2, PerceptionConfig, PlanIntent, Team, ValidateCfg,
    World, WorldSnapshot,
};
use astraweave_core::{CAmmo, CCooldowns, CHealth, CPos};
use astraweave_ecs as ecs;

#[cfg(feature = "llm")]
use astraweave_core::{ActionStep, ToolRegistry};

#[cfg(feature = "llm")]
use astraweave_llm::{plan_from_llm, PlanSource};

#[cfg(feature = "ollama")]
use astraweave_llm::qwen3_ollama::Qwen3Ollama;

use anyhow::Result;

// ============================================================================
// AI MODE SELECTION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AIMode {
    Classical, // RuleOrchestrator (always available)
    #[cfg(feature = "llm")]
    BehaviorTree, // Hierarchical reasoning
    #[cfg(feature = "llm")]
    Utility, // Score-based selection
    #[cfg(feature = "ollama")]
    LLM, // Qwen3-8B via Ollama
    #[cfg(feature = "ollama")]
    Hybrid, // LLM + Classical fallback
    #[cfg(feature = "llm")]
    Ensemble, // Voting across all modes
    #[cfg(feature = "llm_orchestrator")]
    Arbiter, // GOAP + Qwen3 hybrid (instant control)
}

impl std::fmt::Display for AIMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIMode::Classical => write!(f, "Classical (RuleOrchestrator)"),
            #[cfg(feature = "llm")]
            AIMode::BehaviorTree => write!(f, "BehaviorTree (Hierarchical)"),
            #[cfg(feature = "llm")]
            AIMode::Utility => write!(f, "Utility (Score-based)"),
            #[cfg(feature = "ollama")]
            AIMode::LLM => write!(f, "LLM (Qwen3-8B via Ollama)"),
            #[cfg(feature = "ollama")]
            AIMode::Hybrid => write!(f, "Hybrid (LLM + Fallback)"),
            #[cfg(feature = "llm")]
            AIMode::Ensemble => write!(f, "Ensemble (Voting)"),
            #[cfg(feature = "llm_orchestrator")]
            AIMode::Arbiter => write!(f, "Arbiter (GOAP + Qwen3 Hybrid)"),
        }
    }
}

// ============================================================================
// METRICS TRACKING
// ============================================================================

#[cfg(feature = "metrics")]
use chrono::Utc;
#[cfg(feature = "metrics")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "metrics")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AIMetrics {
    mode: String,
    plan_steps: usize,
    latency_ms: f64,
    timestamp: String,
    success: bool,
    error: Option<String>,
    // Phase 7 enhancements
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_used: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_method: Option<String>,
}

#[cfg(feature = "metrics")]
impl AIMetrics {
    fn new(
        mode: &str,
        plan_steps: usize,
        latency_ms: f64,
        success: bool,
        error: Option<String>,
    ) -> Self {
        Self {
            mode: mode.to_string(),
            plan_steps,
            latency_ms,
            timestamp: Utc::now().to_rfc3339(),
            success,
            error,
            tools_used: None,
            fallback_tier: None,
            cache_decision: None,
            parse_method: None,
        }
    }

    fn with_phase7_data(
        mut self,
        tools_used: Vec<String>,
        fallback_tier: Option<String>,
        cache_decision: Option<String>,
        parse_method: Option<String>,
    ) -> Self {
        self.tools_used = Some(tools_used);
        self.fallback_tier = fallback_tier;
        self.cache_decision = cache_decision;
        self.parse_method = parse_method;
        self
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

#[cfg(feature = "visual")]
fn main() -> Result<()> {
    visual_demo::run_visual_demo()
}

#[cfg(not(feature = "visual"))]
fn main() -> Result<()> {
    // Console-based demo (original behavior)
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   AstraWeave AI Companion Demo - Advanced Showcase        ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let demo_all = args.contains(&"--demo-all".to_string());
    let _show_metrics = args.contains(&"--metrics".to_string());
    let _export_metrics = args.contains(&"--export-metrics".to_string());

    #[cfg(feature = "metrics")]
    let mut all_metrics: Vec<AIMetrics> = Vec::new();

    if demo_all {
        #[cfg(feature = "llm")]
        {
            println!("🎯 Demo Mode: Running ALL AI systems for comparison\n");

            let modes = vec![
                AIMode::Classical,
                AIMode::BehaviorTree,
                AIMode::Utility,
                #[cfg(feature = "ollama")]
                AIMode::LLM,
                #[cfg(feature = "ollama")]
                AIMode::Hybrid,
                AIMode::Ensemble,
            ];

            for mode in &modes {
                println!("─────────────────────────────────────────────────────────────");
                println!("Running: {}", mode);
                println!("─────────────────────────────────────────────────────────────");

                #[cfg(feature = "metrics")]
                let metrics = run_single_demo(*mode)?;

                #[cfg(not(feature = "metrics"))]
                run_single_demo(*mode)?;

                #[cfg(feature = "metrics")]
                all_metrics.push(metrics);

                println!();
            }

            #[cfg(feature = "metrics")]
            if show_metrics {
                print_metrics_table(&all_metrics);
            }

            #[cfg(feature = "metrics")]
            if export_metrics {
                export_metrics_to_files(&all_metrics)?;
            }

            return Ok(());
        }

        #[cfg(not(feature = "llm"))]
        {
            println!("⚠️  --demo-all requires the 'llm' feature flag");
            println!("    Run: cargo run --release -p hello_companion --features llm,ollama -- --demo-all\n");
            return Ok(());
        }
    }

    // Single mode
    let mode = select_ai_mode(&args);
    println!("🤖 AI Mode: {}\n", mode);

    #[cfg(feature = "metrics")]
    let metrics = run_single_demo(mode)?;

    #[cfg(not(feature = "metrics"))]
    run_single_demo(mode)?;

    #[cfg(feature = "metrics")]
    {
        if show_metrics {
            print_metrics_table(&[metrics.clone()]);
        }
        if export_metrics {
            export_metrics_to_files(&[metrics])?;
        }
    }

    Ok(())
}

// ============================================================================
// SINGLE DEMO RUN
// ============================================================================

// Helper to sync legacy world changes to ECS
fn update_ecs_from_legacy(world: &mut ecs::World, legacy_id: u32) {
    // Get ECS entity from bridge
    let ecs_entity = if let Some(bridge) = world.get_resource::<EntityBridge>() {
        bridge.get_by_legacy(&legacy_id)
    } else {
        None
    };

    if let Some(e) = ecs_entity {
        // Get legacy world
        let (pos, hp, ammo, cooldowns) = if let Some(w) = world.get_resource::<World>() {
            let pos = w.pose(legacy_id).map(|p| p.pos);
            let hp = w.health(legacy_id).map(|h| h.hp);
            let ammo = w.ammo(legacy_id).map(|a| a.rounds);
            let cooldowns = w.cooldowns(legacy_id).map(|cds| cds.map.clone());
            (pos, hp, ammo, cooldowns)
        } else {
            (None, None, None, None)
        };

        if let Some(p) = pos {
            world.insert(e, CPos { pos: p });
        }
        if let Some(h) = hp {
            world.insert(e, CHealth { hp: h });
        }
        if let Some(a) = ammo {
            world.insert(e, CAmmo { rounds: a });
        }
        if let Some(cds) = cooldowns {
            let map: astraweave_core::cooldowns::Map = cds
                .iter()
                .map(|(k, v)| {
                    (
                        astraweave_core::cooldowns::CooldownKey::from(k.as_str()),
                        *v,
                    )
                })
                .collect();
            world.insert(e, CCooldowns { map });
        }
    }
}

#[cfg(feature = "metrics")]
fn run_single_demo(mode: AIMode) -> Result<AIMetrics> {
    use std::time::Instant;

    // Setup world
    let (w, _player, comp, enemy, snap) = setup_world()?;

    // Build ECS App
    let mut app = build_app(w, 0.25);

    // Generate plan with timing
    let start = Instant::now();
    let plan_result = generate_plan(&snap, mode);
    let elapsed = start.elapsed();
    let latency_ms = elapsed.as_secs_f64() * 1000.0;

    let (plan, success, error) = match plan_result {
        Ok(p) => (p, true, None),
        Err(e) => {
            println!("❌ Plan generation failed: {}", e);
            return Ok(AIMetrics::new(
                &mode.to_string(),
                0,
                latency_ms,
                false,
                Some(e.to_string()),
            ));
        }
    };

    println!(
        "✅ Generated {} step plan in {:.3}ms",
        plan.steps.len(),
        latency_ms
    );

    // Execute plan
    let v_cfg = ValidateCfg {
        world_bounds: (0, 0, 19, 9),
    };

    let mut log = |line: String| {
        println!("   {}", line);
    };

    {
        let w = app.world.get_resource_mut::<World>().unwrap();
        println!("\n--- Executing Plan @ t={:.2} ---", w.t);
        if let Err(e) = validate_and_execute(&mut *w, comp, &plan, &v_cfg, &mut log) {
            println!("⚠️  Execution failed: {}. Continuing...", e);
        }
    }

    // Sync legacy changes to ECS
    update_ecs_from_legacy(&mut app.world, comp);

    // Simulate time passage using ECS
    for _ in 0..20 {
        app = app.run_fixed(1);
    }

    let w = app.world.get_resource::<World>().unwrap();
    println!("\n--- Post-execution State @ t={:.2} ---", w.t);
    if let Some(comp_pos) = w.pos_of(comp) {
        println!("Companion: {:?}", comp_pos);
    }
    if let Some(enemy_pos) = w.pos_of(enemy) {
        println!("Enemy:     {:?}", enemy_pos);
    }
    if let Some(enemy_hp) = w.health(enemy) {
        println!("Enemy HP:  {}", enemy_hp.hp);
    }

    let mut metrics = AIMetrics::new(
        &mode.to_string(),
        plan.steps.len(),
        latency_ms,
        success,
        error,
    );

    #[cfg(feature = "llm")]
    {
        let tools = extract_tools_used(&plan);
        metrics = metrics.with_phase7_data(tools, None, None, None);
    }

    Ok(metrics)
}

#[cfg(not(feature = "metrics"))]
fn run_single_demo(mode: AIMode) -> Result<()> {
    use std::time::Instant;

    let (w, _player, comp, enemy, snap) = setup_world()?;

    // Build ECS App
    let mut app = build_app(w, 0.25);

    let start = Instant::now();
    let plan = generate_plan(&snap, mode)?;
    let elapsed = start.elapsed();

    println!(
        "✅ Generated {} step plan in {:.3}ms",
        plan.steps.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    let v_cfg = ValidateCfg {
        world_bounds: (0, 0, 19, 9),
    };

    let mut log = |line: String| {
        println!("   {}", line);
    };

    {
        let w = app.world.get_resource_mut::<World>().unwrap();
        println!("\n--- Executing Plan @ t={:.2} ---", w.t);
        if let Err(e) = validate_and_execute(&mut *w, comp, &plan, &v_cfg, &mut log) {
            println!("⚠️  Execution failed: {}. Continuing...", e);
        }
    }

    // Sync legacy changes to ECS
    update_ecs_from_legacy(&mut app.world, comp);

    // Simulate time passage using ECS
    for _ in 0..20 {
        app = app.run_fixed(1);
    }

    let w = app.world.get_resource::<World>().unwrap();
    println!("\n--- Post-execution State @ t={:.2} ---", w.t);
    if let Some(comp_pos) = w.pos_of(comp) {
        println!("Companion: {:?}", comp_pos);
    }
    if let Some(enemy_pos) = w.pos_of(enemy) {
        println!("Enemy:     {:?}", enemy_pos);
    }
    if let Some(enemy_hp) = w.health(enemy) {
        println!("Enemy HP:  {}", enemy_hp.hp);
    }

    Ok(())
}

// ============================================================================
// WORLD SETUP
// ============================================================================

fn setup_world() -> Result<(World, u32, u32, u32, WorldSnapshot)> {
    let mut w = World::new();

    // Create vertical wall obstacle
    for x in 6..=6 {
        for y in 1..=8 {
            w.obstacles.insert((x, y));
        }
    }

    // Spawn entities
    let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let comp = w.spawn("Companion", IVec2 { x: 2, y: 3 }, Team { id: 1 }, 80, 30);
    let enemy = w.spawn("Rival", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);

    // Prime companion cooldowns
    if let Some(cd) = w.cooldowns_mut(comp) {
        cd.map.insert("throw:smoke".into(), 0.0);
    }

    // Build snapshot
    let p_cfg = PerceptionConfig { los_max: 12 };
    let enemies = vec![enemy];
    let snap = build_snapshot(&w, player, comp, &enemies, Some("extract".into()), &p_cfg);

    Ok((w, player, comp, enemy, snap))
}

// ============================================================================
// AI MODE SELECTION
// ============================================================================

fn select_ai_mode(args: &[String]) -> AIMode {
    // Check for explicit mode flags
    if args.contains(&"--arbiter".to_string()) {
        #[cfg(feature = "llm_orchestrator")]
        return AIMode::Arbiter;

        #[cfg(not(feature = "llm_orchestrator"))]
        {
            println!("⚠️  Arbiter mode requires --features llm_orchestrator");
            return AIMode::Classical;
        }
    }

    if args.contains(&"--bt".to_string()) {
        #[cfg(feature = "llm")]
        return AIMode::BehaviorTree;

        #[cfg(not(feature = "llm"))]
        {
            println!("⚠️  BehaviorTree mode requires --features llm");
            return AIMode::Classical;
        }
    }

    if args.contains(&"--utility".to_string()) {
        #[cfg(feature = "llm")]
        return AIMode::Utility;

        #[cfg(not(feature = "llm"))]
        {
            println!("⚠️  Utility mode requires --features llm");
            return AIMode::Classical;
        }
    }

    if args.contains(&"--llm".to_string()) {
        #[cfg(feature = "ollama")]
        return AIMode::LLM;

        #[cfg(not(feature = "ollama"))]
        {
            println!("⚠️  LLM mode requires --features llm,ollama");
            return AIMode::Classical;
        }
    }

    if args.contains(&"--hybrid".to_string()) {
        #[cfg(feature = "ollama")]
        return AIMode::Hybrid;

        #[cfg(not(feature = "ollama"))]
        {
            println!("⚠️  Hybrid mode requires --features llm,ollama");
            return AIMode::Classical;
        }
    }

    if args.contains(&"--ensemble".to_string()) {
        #[cfg(feature = "llm")]
        return AIMode::Ensemble;

        #[cfg(not(feature = "llm"))]
        {
            println!("⚠️  Ensemble mode requires --features llm");
            return AIMode::Classical;
        }
    }

    // Default behavior
    #[cfg(feature = "ollama")]
    {
        println!("💡 Ollama features enabled. Using Hybrid mode (LLM + fallback).");
        println!("   Use --llm for pure LLM, --bt for BehaviorTree, etc.\n");
        return AIMode::Hybrid;
    }

    #[cfg(all(feature = "llm", not(feature = "ollama")))]
    {
        println!("💡 LLM features enabled. Using BehaviorTree mode.");
        println!("   Enable Ollama with --features llm,ollama for Qwen3-8B\n");
        return AIMode::BehaviorTree;
    }

    #[cfg(not(feature = "llm"))]
    {
        println!("💡 Using Classical AI (RuleOrchestrator).");
        println!("   Enable advanced modes with --features llm,ollama\n");
        AIMode::Classical
    }
}

// ============================================================================
// PLAN GENERATION ROUTER
// ============================================================================

fn generate_plan(snap: &WorldSnapshot, mode: AIMode) -> Result<PlanIntent> {
    match mode {
        AIMode::Classical => generate_classical_plan(snap),

        #[cfg(feature = "llm")]
        AIMode::BehaviorTree => generate_bt_plan(snap),

        #[cfg(feature = "llm")]
        AIMode::Utility => generate_utility_plan(snap),

        #[cfg(feature = "ollama")]
        AIMode::LLM => generate_llm_plan(snap),

        #[cfg(feature = "ollama")]
        AIMode::Hybrid => {
            println!("🎯 Trying LLM with classical fallback...");
            match generate_llm_plan(snap) {
                Ok(plan) => {
                    println!("   ✅ LLM succeeded");
                    Ok(plan)
                }
                Err(e) => {
                    println!("   ⚠️  LLM failed: {}. Falling back...", e);
                    generate_classical_plan(snap)
                }
            }
        }

        #[cfg(feature = "llm")]
        AIMode::Ensemble => generate_ensemble_plan(snap),

        #[cfg(feature = "llm_orchestrator")]
        AIMode::Arbiter => {
            // Note: Arbiter requires stateful usage for proper operation.
            // This single-call demo doesn't showcase the full hybrid pattern.
            // See arbiter example in the arbiter module for proper usage.
            println!("⚠️  Arbiter mode requires stateful usage");
            println!("   Using GOAP for this single-shot demo");
            generate_goap_plan(snap)
        }
    }
}

// ============================================================================
// CLASSICAL AI (Baseline)
// ============================================================================

fn generate_classical_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    println!("🤖 Classical AI (RuleOrchestrator)");
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(snap);
    println!("   Generated {} steps", plan.steps.len());
    Ok(plan)
}

// ============================================================================
// GOAP AI (Goal-Oriented Action Planning)
// ============================================================================

#[cfg(feature = "llm_orchestrator")]
fn generate_goap_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    println!("🎯 GOAP AI (Goal-Oriented Action Planning)");
    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(snap);
    println!(
        "   Generated {} steps (move-to-engage tactical plan)",
        plan.steps.len()
    );
    Ok(plan)
}

// ============================================================================
// BEHAVIORTREE AI
// ============================================================================

#[cfg(feature = "llm")]
fn generate_bt_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    use astraweave_behavior::{BehaviorContext, BehaviorGraph, BehaviorNode};

    println!("🌳 BehaviorTree AI (Hierarchical)");

    // Create behavior tree with correct API
    // Root selector: Try combat if enemies present, else move to objective
    let has_enemies = !snap.enemies.is_empty();

    let combat_sequence = BehaviorNode::Sequence(vec![
        BehaviorNode::Condition("has_enemies".to_string()),
        BehaviorNode::Action("throw_smoke".to_string()),
        BehaviorNode::Action("cover_fire".to_string()),
    ]);

    let move_sequence =
        BehaviorNode::Sequence(vec![BehaviorNode::Action("move_to_objective".to_string())]);

    let root = BehaviorNode::Selector(vec![combat_sequence, move_sequence]);
    let graph = BehaviorGraph::new(root);

    // Create behavior context
    let context = BehaviorContext::new();

    // Execute BT
    let _status = graph.tick(&context);

    // Build plan from BT execution
    let plan_id = format!("bt_{}", snap.t);

    let plan = if has_enemies {
        // Combat path
        let first_enemy = &snap.enemies[0];
        PlanIntent {
            plan_id,
            steps: vec![
                ActionStep::Throw {
                    item: "smoke".into(),
                    x: first_enemy.pos.x,
                    y: first_enemy.pos.y,
                },
                ActionStep::CoverFire {
                    target_id: first_enemy.id,
                    duration: 2.0,
                },
            ],
        }
    } else {
        // Move to objective (derive from POIs or use companion position + offset)
        let target_pos = snap.pois.first().map(|poi| poi.pos).unwrap_or(IVec2 {
            x: snap.me.pos.x + 5,
            y: snap.me.pos.y,
        });

        PlanIntent {
            plan_id,
            steps: vec![ActionStep::MoveTo {
                x: target_pos.x,
                y: target_pos.y,
                speed: None,
            }],
        }
    };

    println!("   BT executed {} steps", plan.steps.len());
    Ok(plan)
}

// ============================================================================
// UTILITY AI
// ============================================================================

#[cfg(feature = "llm")]
fn generate_utility_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    println!("📊 Utility AI (Score-based)");

    // Score possible actions
    let mut scores = vec![
        ("MoveTo", calculate_move_score(snap)),
        ("ThrowSmoke", calculate_smoke_score(snap)),
        ("CoverFire", calculate_coverfire_score(snap)),
    ];

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("   Action scores:");
    for (action, score) in &scores {
        println!("      {} = {:.2}", action, score);
    }

    let best_action = scores[0].0;
    println!("   Selected: {}", best_action);

    let plan_id = format!("utility_{}", snap.t);

    // Convert to plan
    let plan = match best_action {
        "MoveTo" => {
            let target_pos = snap.pois.first().map(|poi| poi.pos).unwrap_or(IVec2 {
                x: snap.me.pos.x + 5,
                y: snap.me.pos.y,
            });

            PlanIntent {
                plan_id,
                steps: vec![ActionStep::MoveTo {
                    x: target_pos.x,
                    y: target_pos.y,
                    speed: None,
                }],
            }
        }
        "ThrowSmoke" => {
            let target_pos = snap
                .enemies
                .first()
                .map(|e| e.pos)
                .or_else(|| snap.pois.first().map(|poi| poi.pos))
                .unwrap_or(snap.me.pos);

            PlanIntent {
                plan_id,
                steps: vec![ActionStep::Throw {
                    item: "smoke".into(),
                    x: target_pos.x,
                    y: target_pos.y,
                }],
            }
        }
        "CoverFire" => {
            let target_id = snap.enemies.first().map(|e| e.id).unwrap_or(0);
            PlanIntent {
                plan_id,
                steps: vec![ActionStep::CoverFire {
                    target_id,
                    duration: 2.0,
                }],
            }
        }
        _ => PlanIntent {
            plan_id,
            steps: vec![],
        },
    };

    Ok(plan)
}

#[cfg(feature = "llm")]
fn calculate_move_score(snap: &WorldSnapshot) -> f32 {
    // Calculate distance to objective (use POI or default position)
    let target_pos = snap.pois.first().map(|poi| poi.pos).unwrap_or(IVec2 {
        x: snap.me.pos.x + 5,
        y: snap.me.pos.y,
    });

    let dist_to_obj =
        ((target_pos.x - snap.me.pos.x).pow(2) + (target_pos.y - snap.me.pos.y).pow(2)) as f32;
    let threat_penalty = snap.enemies.len() as f32 * 0.3;

    (10.0 / (1.0 + dist_to_obj)) - threat_penalty
}

#[cfg(feature = "llm")]
fn calculate_smoke_score(snap: &WorldSnapshot) -> f32 {
    if snap.enemies.is_empty() {
        return 0.0;
    }

    let threat_count = snap.enemies.len() as f32;
    let has_smoke_cd = snap
        .me
        .cooldowns
        .get("throw:smoke")
        .map(|cd| *cd == 0.0)
        .unwrap_or(false);

    if has_smoke_cd {
        threat_count * 2.0
    } else {
        0.0
    }
}

#[cfg(feature = "llm")]
fn calculate_coverfire_score(snap: &WorldSnapshot) -> f32 {
    if snap.enemies.is_empty() {
        return 0.0;
    }

    let has_ammo = snap.me.ammo > 0;
    if has_ammo {
        snap.enemies.len() as f32 * 1.5
    } else {
        0.0
    }
}

// ============================================================================
// LLM AI (Qwen3-8B via Ollama)
// ============================================================================

#[cfg(feature = "ollama")]
fn generate_llm_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    use anyhow::Context;
    println!("🧠 LLM AI (Qwen3-8B via Ollama)");

    // Check Ollama availability first
    check_ollama_available()?;

    // Create Qwen3Ollama client (5GB Q4_K_M - Qwen3-8B with dual thinking modes)
    // Qwen3-8B has native tool calling and excellent structured output for AstraWeave's 37-tool system
    // TEMPERATURE EXPERIMENT: Change this value to test different configurations
    // - 0.3 = Deterministic (high consistency, low creativity)
    // - 0.5 = Balanced (BASELINE - 100% success rate validated)
    // - 0.7 = Creative (high diversity, potential lower consistency)
    //
    // LATENCY OPTIMIZATION: Reduced max_tokens from 1024 to 256
    // - Prevents overly verbose plans (~0.5-1s savings on generation)
    // - Combined with Tier 2 (SimplifiedLlm) for maximum latency reduction
    let client = Qwen3Ollama::localhost()
        .with_temperature(0.5) // ⚠️ MODIFY THIS for temperature experiments
        .with_max_tokens(256); // Reduced from 1024 for faster generation

    // Create tool registry
    let registry = create_tool_registry();

    // Create async runtime
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    // Call LLM
    let result = rt.block_on(async { plan_from_llm(&client, snap, &registry).await });

    match result {
        PlanSource::Llm(plan) => {
            println!("   ✅ Qwen3-8B generated {} steps", plan.steps.len());
            Ok(plan)
        }
        PlanSource::Fallback { plan, reason } => {
            println!("   ⚠️  Qwen3-8B returned fallback: {}", reason);
            Ok(plan)
        }
    }
}

#[cfg(feature = "ollama")]
fn check_ollama_available() -> Result<()> {
    use anyhow::Context;
    println!("   Checking Ollama availability...");

    // Use tokio runtime for async reqwest
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get("http://localhost:11434/api/tags")
            .send()
            .await
            .context("Ollama not running. Start with: ollama serve")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama responded with error status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let models = json["models"]
            .as_array()
            .context("No models found in Ollama response")?;

        let has_phi3 = models.iter().any(|m| {
            m["name"]
                .as_str()
                .map(|n| n.starts_with("phi"))
                .unwrap_or(false)
        });

        if !has_phi3 {
            anyhow::bail!(
                "phi3 model not found. Install with: ollama pull phi3\n   Available models: {:?}",
                models
                    .iter()
                    .filter_map(|m| m["name"].as_str())
                    .collect::<Vec<_>>()
            );
        }

        println!("   ✅ Ollama + phi3 confirmed");
        Ok(())
    })
}

#[cfg(feature = "llm")]
fn create_tool_registry() -> ToolRegistry {
    use astraweave_core::{Constraints, ToolSpec};

    // Helper to create tool specs matching ActionStep schema
    fn tool(name: &str, args: Vec<(&str, &str)>) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            args: args
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    ToolRegistry {
        tools: vec![
            // MOVEMENT (6 tools) - Position-based
            tool("MoveTo", vec![("x", "i32"), ("y", "i32")]),
            tool("TakeCover", vec![]), // Optional position param
            tool("Patrol", vec![("waypoints", "Vec<IVec2>")]),
            // MOVEMENT (3 tools) - Target-based
            tool(
                "Approach",
                vec![("target_id", "Entity"), ("distance", "f32")],
            ),
            tool(
                "Retreat",
                vec![("target_id", "Entity"), ("distance", "f32")],
            ),
            tool(
                "Strafe",
                vec![("target_id", "Entity"), ("direction", "enum[Left,Right]")],
            ),
            // OFFENSIVE (5 tools) - Target-based
            tool("Attack", vec![("target_id", "Entity")]),
            tool("AimedShot", vec![("target_id", "Entity")]),
            tool("QuickAttack", vec![("target_id", "Entity")]),
            tool("HeavyAttack", vec![("target_id", "Entity")]),
            tool(
                "CoverFire",
                vec![("target_id", "Entity"), ("duration", "f32")],
            ),
            // OFFENSIVE (3 tools) - Position-based
            tool(
                "AoEAttack",
                vec![("x", "i32"), ("y", "i32"), ("radius", "f32")],
            ),
            tool("ThrowExplosive", vec![("x", "i32"), ("y", "i32")]),
            tool("Charge", vec![("target_id", "Entity")]),
            // DEFENSIVE (6 tools)
            tool("Block", vec![]),
            tool("Dodge", vec![]), // Optional direction
            tool("Parry", vec![]),
            tool("ThrowSmoke", vec![("x", "i32"), ("y", "i32")]),
            tool("Heal", vec![]), // Optional target_id
            tool("UseDefensiveAbility", vec![("ability_name", "String")]),
            // EQUIPMENT (5 tools)
            tool("EquipWeapon", vec![("weapon_name", "String")]),
            tool("SwitchWeapon", vec![("slot", "u32")]),
            tool("Reload", vec![]),
            tool("UseItem", vec![("item_name", "String")]),
            tool("DropItem", vec![("item_name", "String")]),
            // TACTICAL (7 tools)
            tool("CallReinforcements", vec![("count", "u32")]),
            tool("MarkTarget", vec![("target_id", "Entity")]),
            tool("RequestCover", vec![("duration", "f32")]),
            tool("CoordinateAttack", vec![("target_id", "Entity")]),
            tool("SetAmbush", vec![("position", "IVec2")]),
            tool("Distract", vec![("target_id", "Entity")]),
            tool("Regroup", vec![("rally_point", "IVec2")]),
            // UTILITY (5 tools)
            tool("Scan", vec![("radius", "f32")]),
            tool("Wait", vec![("duration", "f32")]),
            tool("Interact", vec![("target_id", "Entity")]),
            tool("UseAbility", vec![("ability_name", "String")]),
            tool("Taunt", vec![("target_id", "Entity")]),
        ],
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    }
}

// ============================================================================
// ENSEMBLE AI (Voting)
// ============================================================================

#[cfg(feature = "llm")]
fn generate_ensemble_plan(snap: &WorldSnapshot) -> Result<PlanIntent> {
    println!("🎭 Ensemble AI (Voting across modes)");

    // Generate plans from all available modes
    let mut plans = vec![];

    println!("   Collecting votes:");

    // Classical
    if let Ok(plan) = generate_classical_plan(snap) {
        plans.push(("Classical", plan));
    }

    // BehaviorTree
    if let Ok(plan) = generate_bt_plan(snap) {
        plans.push(("BehaviorTree", plan));
    }

    // Utility
    if let Ok(plan) = generate_utility_plan(snap) {
        plans.push(("Utility", plan));
    }

    // LLM (if ollama enabled)
    #[cfg(feature = "ollama")]
    if let Ok(plan) = generate_llm_plan(snap) {
        plans.push(("LLM", plan));
    }

    if plans.is_empty() {
        anyhow::bail!("No plans generated for ensemble");
    }

    println!("\n   Voting results ({} votes):", plans.len());

    // Calculate similarity scores
    let mut scores = vec![];
    for (i, (name_i, plan_i)) in plans.iter().enumerate() {
        let mut similarity_sum = 0.0;
        for (j, (_, plan_j)) in plans.iter().enumerate() {
            if i != j {
                similarity_sum += calculate_plan_similarity(plan_i, plan_j);
            }
        }
        let avg_similarity = if plans.len() > 1 {
            similarity_sum / (plans.len() - 1) as f32
        } else {
            1.0
        };
        scores.push((name_i, avg_similarity, plan_i));
        println!("      {} = {:.2} similarity", name_i, avg_similarity);
    }

    // Select plan with highest average similarity (consensus)
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let winner = scores[0];

    println!("   🏆 Winner: {} (consensus: {:.2})", winner.0, winner.1);

    Ok(winner.2.clone())
}

#[cfg(feature = "llm")]
fn calculate_plan_similarity(plan_a: &PlanIntent, plan_b: &PlanIntent) -> f32 {
    use std::collections::HashSet;

    // Jaccard similarity on action types
    let actions_a: HashSet<_> = plan_a.steps.iter().map(action_type_string).collect();
    let actions_b: HashSet<_> = plan_b.steps.iter().map(action_type_string).collect();

    let intersection = actions_a.intersection(&actions_b).count();
    let union = actions_a.union(&actions_b).count();

    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

#[cfg(feature = "llm")]
fn action_type_string(step: &ActionStep) -> String {
    match step {
        // Movement (6)
        ActionStep::MoveTo { .. } => "move_to".to_string(),
        ActionStep::Approach { .. } => "approach".to_string(),
        ActionStep::Retreat { .. } => "retreat".to_string(),
        ActionStep::TakeCover { .. } => "take_cover".to_string(),
        ActionStep::Strafe { .. } => "strafe".to_string(),
        ActionStep::Patrol { .. } => "patrol".to_string(),

        // Offensive (8)
        ActionStep::Attack { .. } => "attack".to_string(),
        ActionStep::AimedShot { .. } => "aimed_shot".to_string(),
        ActionStep::QuickAttack { .. } => "quick_attack".to_string(),
        ActionStep::HeavyAttack { .. } => "heavy_attack".to_string(),
        ActionStep::AoEAttack { .. } => "aoe_attack".to_string(),
        ActionStep::ThrowExplosive { .. } => "throw_explosive".to_string(),
        ActionStep::CoverFire { .. } => "cover_fire".to_string(),
        ActionStep::Charge { .. } => "charge".to_string(),

        // Defensive (6)
        ActionStep::Block { .. } => "block".to_string(),
        ActionStep::Dodge { .. } => "dodge".to_string(),
        ActionStep::Parry { .. } => "parry".to_string(),
        ActionStep::ThrowSmoke { .. } => "throw_smoke".to_string(),
        ActionStep::Heal { .. } => "heal".to_string(),
        ActionStep::UseDefensiveAbility { .. } => "use_defensive_ability".to_string(),

        // Equipment (5)
        ActionStep::EquipWeapon { .. } => "equip_weapon".to_string(),
        ActionStep::SwitchWeapon { .. } => "switch_weapon".to_string(),
        ActionStep::Reload => "reload".to_string(),
        ActionStep::UseItem { .. } => "use_item".to_string(),
        ActionStep::DropItem { .. } => "drop_item".to_string(),

        // Tactical (7)
        ActionStep::CallReinforcements { .. } => "call_reinforcements".to_string(),
        ActionStep::MarkTarget { .. } => "mark_target".to_string(),
        ActionStep::RequestCover { .. } => "request_cover".to_string(),
        ActionStep::CoordinateAttack { .. } => "coordinate_attack".to_string(),
        ActionStep::SetAmbush { .. } => "set_ambush".to_string(),
        ActionStep::Distract { .. } => "distract".to_string(),
        ActionStep::Regroup { .. } => "regroup".to_string(),

        // Utility (5)
        ActionStep::Scan { .. } => "scan".to_string(),
        ActionStep::Wait { .. } => "wait".to_string(),
        ActionStep::Interact { .. } => "interact".to_string(),
        ActionStep::UseAbility { .. } => "use_ability".to_string(),
        ActionStep::Taunt { .. } => "taunt".to_string(),

        // Legacy (2)
        ActionStep::Throw { .. } => "throw".to_string(),
        ActionStep::Revive { .. } => "revive".to_string(),

        // Terrain (1)
        ActionStep::ModifyTerrain { .. } => "modify_terrain".to_string(),
    }
}

// Extract list of tool names from plan for Phase 7 metrics
#[cfg(feature = "llm")]
fn extract_tools_used(plan: &PlanIntent) -> Vec<String> {
    plan.steps
        .iter()
        .map(|step| action_type_string(step))
        .collect()
}

// ============================================================================
// METRICS DISPLAY & EXPORT
// ============================================================================

#[cfg(feature = "metrics")]
fn print_metrics_table(metrics: &[AIMetrics]) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║                              AI METRICS SUMMARY (Phase 7)                             ║"
    );
    println!(
        "╠════════════════════════════════╦═══════╦═══════════╦═══════╦════════════════════════╣"
    );
    println!(
        "║ Mode                           ║ Steps ║ Latency   ║ Status║ Tools Used             ║"
    );
    println!(
        "╠════════════════════════════════╬═══════╬═══════════╬═══════╬════════════════════════╣"
    );

    for m in metrics {
        let status = if m.success { "✅" } else { "❌" };
        let tools = m
            .tools_used
            .as_ref()
            .map(|t| t.join(", "))
            .unwrap_or_else(|| "-".to_string());
        let tools_display = if tools.len() > 20 {
            format!("{}...", &tools[..20])
        } else {
            tools
        };

        println!(
            "║ {:30} ║ {:5} ║ {:7.2}ms ║  {}   ║ {:22} ║",
            m.mode, m.plan_steps, m.latency_ms, status, tools_display
        );

        // Show Phase 7 details if available
        if m.fallback_tier.is_some() || m.cache_decision.is_some() || m.parse_method.is_some() {
            if let Some(tier) = &m.fallback_tier {
                println!("║   Fallback Tier: {:61} ║", tier);
            }
            if let Some(cache) = &m.cache_decision {
                println!("║   Cache:         {:61} ║", cache);
            }
            if let Some(parse) = &m.parse_method {
                println!("║   Parse Method:  {:61} ║", parse);
            }
            println!("╠════════════════════════════════╬═══════╬═══════════╬═══════╬════════════════════════╣");
        }
    }

    println!(
        "╚════════════════════════════════╩═══════╩═══════════╩═══════╩════════════════════════╝"
    );

    // Calculate summary stats
    let total_runs = metrics.len();
    let successful = metrics.iter().filter(|m| m.success).count();
    let avg_latency = metrics.iter().map(|m| m.latency_ms).sum::<f64>() / total_runs as f64;
    let avg_steps = metrics.iter().map(|m| m.plan_steps).sum::<usize>() as f64 / total_runs as f64;

    // Phase 7 stats
    let unique_tools: std::collections::HashSet<String> = metrics
        .iter()
        .filter_map(|m| m.tools_used.as_ref())
        .flatten()
        .cloned()
        .collect();

    println!("\n📊 Summary:");
    println!("   Total runs:        {}", total_runs);
    println!(
        "   Successful:        {} ({:.1}%)",
        successful,
        (successful as f64 / total_runs as f64) * 100.0
    );
    println!("   Avg latency:       {:.2}ms", avg_latency);
    println!("   Avg steps:         {:.1}", avg_steps);
    println!("\n🔧 Phase 7 Features:");
    println!(
        "   Unique tools used: {} / 37 available",
        unique_tools.len()
    );
    println!(
        "   Tools: {}",
        unique_tools
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    if unique_tools.len() > 10 {
        println!("          ... and {} more", unique_tools.len() - 10);
    }
}

#[cfg(feature = "metrics")]
fn export_metrics_to_files(metrics: &[AIMetrics]) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // Export JSON
    let json =
        serde_json::to_string_pretty(metrics).context("Failed to serialize metrics to JSON")?;

    let mut json_file =
        File::create("hello_companion_metrics.json").context("Failed to create JSON file")?;
    json_file
        .write_all(json.as_bytes())
        .context("Failed to write JSON")?;

    println!("\n✅ Exported JSON: hello_companion_metrics.json");

    // Export CSV
    let mut csv_file =
        File::create("hello_companion_metrics.csv").context("Failed to create CSV file")?;

    writeln!(csv_file, "Mode,Steps,Latency_ms,Timestamp,Success,Error")?;
    for m in metrics {
        writeln!(
            csv_file,
            "{},{},{:.3},{},{},{}",
            m.mode,
            m.plan_steps,
            m.latency_ms,
            m.timestamp,
            m.success,
            m.error.as_deref().unwrap_or("")
        )?;
    }

    println!("✅ Exported CSV:  hello_companion_metrics.csv");

    Ok(())
}

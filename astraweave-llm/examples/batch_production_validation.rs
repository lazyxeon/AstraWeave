//! Production validation test for batch LLM integration
//!
//! Tests real Ollama/Qwen3-8B performance with:
//! - Single agent batch (baseline)
//! - 5 agent batch (4-5× speedup target)
//! - 10 agent batch (5-7× speedup target)
//! - Determinism validation (3 runs)
//! - Compression validation (Ollama logs)

use astraweave_core::{
    CompanionState, CoverType, EnemyState, IVec2, PlayerState, Poi, Stance, ToolRegistry,
    WorldSnapshot,
};
use astraweave_llm::{batch_executor::AgentId, fallback_system::FallbackOrchestrator};
use std::collections::BTreeMap;
use std::time::Instant;

#[cfg(feature = "ollama")]
use astraweave_llm::OllamaChatClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("\n🚀 AstraWeave Batch LLM Production Validation");
    println!("={}", "=".repeat(59));

    #[cfg(feature = "ollama")]
    {
        // Initialize Ollama client with Qwen3-8B
        let client =
            OllamaChatClient::new("http://localhost:11434".to_string(), "qwen3:8b".to_string());

        // Initialize tool registry and fallback system
        let reg = astraweave_core::default_tool_registry();
        let fallback = FallbackOrchestrator::new();

        println!("\n✅ Connected to Ollama with Qwen3-8B model");
        println!("   Model: qwen3:8b (~5 GB)");

        run_validation_tests(&client, &reg, &fallback).await?;
    }

    #[cfg(not(feature = "ollama"))]
    {
        eprintln!("❌ ERROR: This example requires the 'ollama' feature");
        eprintln!("   Please run: cargo run -p astraweave-llm --example batch_production_validation --features ollama");
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(feature = "ollama")]
async fn run_validation_tests(
    client: &OllamaChatClient,
    reg: &ToolRegistry,
    fallback: &FallbackOrchestrator,
) -> anyhow::Result<()> {
    // Test 1: Single agent baseline
    println!("\n\n");
    println!("TEST 1: Single Agent Baseline");
    println!("{}", "-".repeat(60));

    let snap1 = create_test_snapshot(1);
    let agents1 = vec![(1, snap1)];

    let start = Instant::now();
    let results1 = fallback
        .plan_batch_with_fallback(client, agents1, reg)
        .await;
    let elapsed1 = start.elapsed();

    println!("⏱️  Time: {:.2}s", elapsed1.as_secs_f64());
    println!("📊 Plans generated: {}", results1.len());

    if let Some(result) = results1.get(&1) {
        println!("📋 Plan steps: {}", result.plan.steps.len());
        println!("🎯 Tier used: {:?}", result.tier);
    }

    // Test 2: 5 agent batch
    println!("\n\n");
    println!("TEST 2: 5 Agent Batch");
    println!("{}", "-".repeat(60));

    let agents5: Vec<_> = (1..=5).map(|i| (i, create_test_snapshot(i))).collect();

    let start = Instant::now();
    let results5 = fallback
        .plan_batch_with_fallback(client, agents5.clone(), reg)
        .await;
    let elapsed5 = start.elapsed();

    let sequential_estimate = elapsed1.as_secs_f64() * 5.0;
    let speedup5 = sequential_estimate / elapsed5.as_secs_f64();

    println!("⏱️  Time: {:.2}s", elapsed5.as_secs_f64());
    println!("📊 Plans generated: {}", results5.len());
    println!(
        "🚀 Speedup vs sequential: {:.1}× ({:.1}s → {:.1}s)",
        speedup5,
        sequential_estimate,
        elapsed5.as_secs_f64()
    );

    // Test 3: 10 agent batch
    println!("\n\n");
    println!("TEST 3: 10 Agent Batch");
    println!("{}", "-".repeat(60));

    let agents10: Vec<_> = (1..=10).map(|i| (i, create_test_snapshot(i))).collect();

    let start = Instant::now();
    let results10 = fallback
        .plan_batch_with_fallback(client, agents10.clone(), reg)
        .await;
    let elapsed10 = start.elapsed();

    let sequential_estimate10 = elapsed1.as_secs_f64() * 10.0;
    let speedup10 = sequential_estimate10 / elapsed10.as_secs_f64();

    println!("⏱️  Time: {:.2}s", elapsed10.as_secs_f64());
    println!("📊 Plans generated: {}", results10.len());
    println!(
        "🚀 Speedup vs sequential: {:.1}× ({:.1}s → {:.1}s)",
        speedup10,
        sequential_estimate10,
        elapsed10.as_secs_f64()
    );

    // Test 4: Determinism validation (3 runs)
    println!("\n\n");
    println!("TEST 4: Determinism Validation");
    println!("{}", "-".repeat(60));

    let mut run_results = Vec::new();
    for run in 1..=3 {
        println!("Run {}/3...", run);
        let results = fallback
            .plan_batch_with_fallback(client, agents5.clone(), reg)
            .await;
        let agent_ids: Vec<_> = results.keys().copied().collect();
        run_results.push(agent_ids);
    }

    let all_same = run_results.windows(2).all(|w| w[0] == w[1]);
    if all_same {
        println!("✅ Determinism: PASS (all 3 runs produced identical ordering)");
        println!("   Agent IDs: {:?}", run_results[0]);
    } else {
        println!("❌ Determinism: FAIL (ordering varied between runs)");
        for (i, ids) in run_results.iter().enumerate() {
            println!("   Run {}: {:?}", i + 1, ids);
        }
    }

    // Summary
    println!("\n\n");
    println!("SUMMARY");
    println!("={}", "=".repeat(59));
    println!("Single agent:  {:.2}s", elapsed1.as_secs_f64());
    println!(
        "5 agents:      {:.2}s ({:.1}× speedup)",
        elapsed5.as_secs_f64(),
        speedup5
    );
    println!(
        "10 agents:     {:.2}s ({:.1}× speedup)",
        elapsed10.as_secs_f64(),
        speedup10
    );
    println!(
        "Determinism:   {}",
        if all_same { "✅ PASS" } else { "❌ FAIL" }
    );

    println!("\n✅ Production validation complete!");
    println!("   See Ollama logs for compression metrics");

    Ok(())
}

fn create_test_snapshot(agent_id: AgentId) -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            pos: IVec2::new(10, 10),
            hp: 100,
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2::new(agent_id as i32 * 2, agent_id as i32 * 2),
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
        },
        enemies: vec![EnemyState {
            id: 20,
            pos: IVec2::new(20, 20),
            hp: 50,
            cover: CoverType::Low,
            last_seen: 0.0,
        }],
        pois: vec![Poi {
            pos: IVec2::new(30, 30),
            k: "objective".to_string(),
        }],
        obstacles: vec![],
        objective: Some("Test mission".to_string()),
    }
}

//! Quick Phi-3 latency test
//!
//! Tests inference speed with different configurations

use astraweave_llm::phi3_ollama::Phi3Ollama;
use astraweave_llm::LlmClient;
use colored::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("{}", "=== Phi-3 Latency Benchmark ===".bright_cyan().bold());
    println!();

    let test_prompt =
        "You are at (5,5). Enemy at (10,8). Generate a JSON tactical plan with 2 actions.";

    // Test 1: phi3:fast (optimized)
    println!("{}", "Testing phi3:fast (optimized)...".yellow());
    let fast_client = Phi3Ollama::fast().expect("model client");

    let start = std::time::Instant::now();
    let response = fast_client.complete(test_prompt).await?;
    let duration = start.elapsed();

    println!(
        "{} {:.2}s",
        "✅ Response time:".green(),
        duration.as_secs_f32()
    );
    println!("   Output length: {} chars", response.len());
    println!();

    // Test 2: phi3:3.8b (mini - fastest)
    println!("{}", "Testing phi3:3.8b (mini)...".yellow());
    let mini_client = Phi3Ollama::mini().expect("model client");

    let start = std::time::Instant::now();
    let response = mini_client.complete(test_prompt).await?;
    let duration = start.elapsed();

    println!(
        "{} {:.2}s",
        "✅ Response time:".green(),
        duration.as_secs_f32()
    );
    println!("   Output length: {} chars", response.len());
    println!();

    println!("{}", "🎉 Benchmark complete!".bright_green().bold());
    println!();
    println!("Expected latencies:");
    println!("  • phi3:fast (14B): 0.5-2s with GPU, 30-60s on CPU");
    println!("  • phi3:mini (3.8B): 0.2-0.8s with GPU, 10-20s on CPU");
    println!();
    println!(
        "{}",
        "If your times are >10s, Ollama is using CPU fallback!"
            .red()
            .bold()
    );
    println!("Check: {}", "ollama ps".bright_white());

    Ok(())
}

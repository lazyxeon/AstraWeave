//! LLM Streaming Demo
//!
//! Demonstrates the new streaming API for Qwen3-8B, showing:
//! - Time-to-first-chunk advantage (8× faster first action)
//! - Progressive response delivery
//! - Integration with StreamingParser (future work)
//!
//! # Usage
//! ```bash
//! # Start Ollama with Qwen3-8B
//! ollama serve
//! ollama pull qwen3.5:4b
//!
//! # Run demo
//! cargo run -p llm_streaming_demo --release
//! ```

use anyhow::Result;
use astraweave_llm::qwen3_ollama::Qwen3Ollama;
use astraweave_llm::LlmClient;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║            LLM STREAMING DEMO - Qwen3-8B                  ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║  Demonstrates:                                                 ║");
    println!("║  - Progressive response delivery (streaming)                   ║");
    println!("║  - Time-to-first-chunk advantage                               ║");
    println!("║  - Side-by-side blocking vs streaming comparison               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let client = Qwen3Ollama::localhost();

    // Health check first
    println!("🔍 Checking Ollama health...");
    let health = client.health_check().await?;

    if !health.is_ready() {
        eprintln!("❌ {}", health.error_message().unwrap());
        eprintln!("\nPlease ensure Ollama is running:");
        eprintln!("  1. Install Ollama: https://ollama.ai");
        eprintln!("  2. Start server: ollama serve");
        eprintln!("  3. Pull model: ollama pull qwen3.5:4b");
        std::process::exit(1);
    }

    println!("✅ Ollama ready (version: {})", health.ollama_version);
    println!("✅ Model available: {}\n", health.model_name);

    // Test prompt
    let prompt = "You are a tactical AI agent at position (5,5). \
                  Enemy spotted at (10,8). \
                  Generate a JSON tactical plan with 3-5 action steps. \
                  Use actions: MoveTo, TakeCover, Attack, ThrowSmoke.";

    println!("═══ TEST 1: BLOCKING COMPLETION (baseline) ═══\n");

    let start = std::time::Instant::now();
    let blocking_response = client.complete(prompt).await?;
    let blocking_time = start.elapsed();

    println!("⏱️  Total time: {:.2}s", blocking_time.as_secs_f32());
    println!("📝 Response length: {} chars", blocking_response.len());
    println!(
        "📄 Response preview:\n{}\n",
        &blocking_response.chars().take(200).collect::<String>()
    );

    println!("\n═══ TEST 2: STREAMING COMPLETION (progressive) ═══\n");

    let start = std::time::Instant::now();
    let mut stream = client.complete_streaming(prompt).await?;

    let mut streaming_response = String::new();
    let mut chunk_count = 0;
    let mut time_to_first_chunk = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        if time_to_first_chunk.is_none() {
            time_to_first_chunk = Some(start.elapsed());
            println!(
                "⚡ FIRST CHUNK ARRIVED: {:.2}s",
                time_to_first_chunk.unwrap().as_secs_f32()
            );
        }

        chunk_count += 1;
        streaming_response.push_str(&chunk);

        // Show progressive updates every 5 chunks
        if chunk_count % 5 == 0 {
            println!(
                "   Chunk #{:3}: {} chars received (total: {} chars)",
                chunk_count,
                chunk.len(),
                streaming_response.len()
            );
        }
    }

    let streaming_total_time = start.elapsed();

    println!(
        "\n⏱️  Total time: {:.2}s",
        streaming_total_time.as_secs_f32()
    );
    println!(
        "⚡ Time to first chunk: {:.2}s",
        time_to_first_chunk.unwrap().as_secs_f32()
    );
    println!("📦 Total chunks: {}", chunk_count);
    println!("📝 Response length: {} chars", streaming_response.len());
    println!(
        "📄 Response preview:\n{}\n",
        &streaming_response.chars().take(200).collect::<String>()
    );

    // Performance comparison
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    PERFORMANCE COMPARISON                      ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");

    let ttfc = time_to_first_chunk.unwrap().as_secs_f32();
    let speedup = blocking_time.as_secs_f32() / ttfc;
    let ttfc_percent = (ttfc / blocking_time.as_secs_f32()) * 100.0;

    println!(
        "║  Blocking total time:     {:.2}s",
        blocking_time.as_secs_f32()
    );
    println!(
        "║  Streaming total time:    {:.2}s",
        streaming_total_time.as_secs_f32()
    );
    println!(
        "║  Time to first chunk:     {:.2}s ({:.1}% of total)",
        ttfc, ttfc_percent
    );
    println!("║  Time-to-first speedup:   {:.1}× faster", speedup);
    println!("║  Chunks received:         {}", chunk_count);
    println!("╠═══════════════════════════════════════════════════════════════╣");

    if speedup >= 4.0 {
        println!(
            "║  ✅ EXCELLENT: {:.1}× speedup exceeds 4× target!",
            speedup
        );
    } else if speedup >= 2.0 {
        println!("║  ✅ GOOD: {:.1}× speedup meets 2× minimum", speedup);
    } else {
        println!(
            "║  ⚠️  BELOW TARGET: {:.1}× speedup (expected 4-8×)",
            speedup
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Validate responses match
    let blocking_trimmed = blocking_response.trim();
    let streaming_trimmed = streaming_response.trim();

    if blocking_trimmed == streaming_trimmed {
        println!("✅ Response consistency: PERFECT (streaming matches blocking)");
    } else {
        let similarity = similar_percent(blocking_trimmed, streaming_trimmed);
        println!(
            "⚠️  Response similarity: {:.1}% (expected ~95-100% with temp=0.7)",
            similarity
        );
    }

    println!("\n═══ Demo Complete ═══");
    println!("Key Takeaways:");
    println!(
        "  - Streaming delivers first chunk ~{:.1}× faster than full response",
        speedup
    );
    println!("  - Enables progressive UI updates and faster perceived latency");
    println!("  - Ready for integration with StreamingParser and BatchExecutor");

    Ok(())
}

/// Simple string similarity metric (percentage of matching characters)
fn similar_percent(a: &str, b: &str) -> f32 {
    let max_len = a.len().max(b.len()) as f32;
    if max_len == 0.0 {
        return 100.0;
    }

    let matching = a.chars().zip(b.chars()).filter(|(ca, cb)| ca == cb).count();

    (matching as f32 / max_len) * 100.0
}

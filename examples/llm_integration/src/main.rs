use astraweave_core::*;
#[cfg(feature = "ollama")]
use astraweave_llm::qwen3_ollama::DEFAULT_QWEN_INSTRUCT_MODEL;
// Display-only fallback for the help text below: the `qwen3_ollama` module is
// `#[cfg(feature = "ollama")]`, but the help text that prints this name runs
// unconditionally. Mirrors the canonical value in `qwen3_ollama.rs`.
#[cfg(not(feature = "ollama"))]
const DEFAULT_QWEN_INSTRUCT_MODEL: &str = "qwen3.5:4b";
use astraweave_llm::{plan_from_llm, MockLlm, PlanSource};
#[cfg(feature = "ollama")]
use astraweave_llm::{LlmClient, LocalHttpClient};
#[cfg(feature = "ollama")]
use std::env;

/// Comprehensive LLM integration example demonstrating multiple client types
///
/// This example is configured to use Qwen by default.
/// To use Qwen with Ollama:
///   1. Make sure Ollama is running: `ollama serve`
///   2. Pull Qwen: `ollama pull qwen3.5:4b`
///   3. Run this example: `cargo run -p llm_integration --features ollama`
///
/// Or set environment variables:
///   OLLAMA_URL=http://localhost:11434
///   OLLAMA_MODEL=qwen3.5:4b
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("AstraWeave LLM Integration Example");
    println!("==================================");

    // Create a test scenario
    let world_snapshot = create_test_scenario();
    let tool_registry = create_tool_registry();

    println!("\nWorld Snapshot:");
    println!("{}", serde_json::to_string_pretty(&world_snapshot)?);

    println!("\nTool Registry:");
    println!("{}", serde_json::to_string_pretty(&tool_registry)?);

    // 1. Test MockLlm (always available)
    println!("\n1. Testing MockLlm Client");
    println!("--------------------------");
    test_mock_client(&world_snapshot, &tool_registry).await?;

    // 2. Test Ollama Chat client with Qwen (default model for this project)
    #[cfg(feature = "ollama")]
    {
        let ollama_url =
            env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        println!("\n2. Testing Ollama Chat Client at {}", ollama_url);
        println!(
            "   Default Model: Qwen instruct ({})",
            DEFAULT_QWEN_INSTRUCT_MODEL
        );
        println!("-------------------------");

        // Quick health check: GET /api/tags is a safe, browser-friendly endpoint that lists available models.
        // Note: /api/chat and /api/generate are POST-only endpoints. Clicking a browser link to them will
        // perform a GET and typically return HTTP 405 Method Not Allowed. To inspect models, open the /api/tags URL.
        if let Err(e) = probe_ollama_tags(&ollama_url).await {
            println!("Warning: failed to probe Ollama /api/tags: {}", e);
            println!("Proceeding to attempt a POST to the chat endpoint; this may fail if the model is not loaded.");
        }

        test_ollama_client(&world_snapshot, &tool_registry, &ollama_url).await?;
    }

    #[cfg(not(feature = "ollama"))]
    {
        println!("\n2. Ollama Chat Client (Skipped - rebuild with --features ollama to enable)");
        println!("   To use Qwen: cargo run -p llm_integration --features ollama");
    }

    // 3. Test LocalHttpClient (if URL provided)
    #[cfg(feature = "ollama")]
    if let Ok(local_url) = env::var("LOCAL_LLM_URL") {
        println!("\n3. Testing Local HTTP Client");
        println!("-----------------------------");
        test_local_http_client(&world_snapshot, &tool_registry, &local_url).await?;
    } else {
        println!(
            "\n3. Local HTTP Client (Skipped - set LOCAL_LLM_URL environment variable to test)"
        );
    }

    #[cfg(not(feature = "ollama"))]
    {
        println!("\n3. Local HTTP Client (Skipped - requires --features ollama)");
    }

    println!("\nExample completed successfully!");
    println!("\n=== Qwen Configuration ===");
    println!("To use Qwen (required local LLM for this project):");
    println!("  1. Install Ollama: https://ollama.ai/download");
    println!(
        "  2. Pull Qwen: ollama pull {}",
        DEFAULT_QWEN_INSTRUCT_MODEL
    );
    println!("  3. Run with features: cargo run -p llm_integration --features ollama");
    println!("\nEnvironment Variables:");
    println!("  OLLAMA_URL=http://localhost:11434 (default)");
    println!("  OLLAMA_MODEL={} (default)", DEFAULT_QWEN_INSTRUCT_MODEL);
    println!("  LOCAL_LLM_URL=<your-url> (for alternative endpoints)");

    Ok(())
}

async fn test_mock_client(snap: &WorldSnapshot, reg: &ToolRegistry) -> anyhow::Result<()> {
    let client = MockLlm;
    let plan_source = plan_from_llm(&client, snap, reg).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { plan: p, reason } => {
            println!("MockLlm fell back: {}", reason);
            p
        }
        _ => {
            println!("Unknown plan source");
            return Ok(());
        }
    };
    println!("✓ MockLlm generated plan:");
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

#[cfg(feature = "ollama")]
async fn test_ollama_client(
    snap: &WorldSnapshot,
    reg: &ToolRegistry,
    url: &str,
) -> anyhow::Result<()> {
    // Prefer explicit override, otherwise use the project default. Do not pick the
    // first installed model: that silently reintroduced Phi-3 in prior integrations.
    let model =
        env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_QWEN_INSTRUCT_MODEL.to_string());
    println!("Using Ollama model: {}", model);
    let client = astraweave_llm::OllamaChatClient::new(url.to_string(), model);

    // Helpful link guidance: GET /api/tags is safe to open in a browser and will list models.
    // The chat endpoint (/api/chat) requires POST and clicking it in a browser (GET) will return 405 Method Not Allowed.
    println!("Connecting to Ollama at: {}", url);
    println!(
        " - Inspect available models (safe to open in browser): {}/api/tags",
        url.trim_end_matches('/')
    );
    println!(
        " - Chat endpoint (POST-only, will return 405 if opened with GET): {}/api/chat",
        url.trim_end_matches('/')
    );
    // Always fetch and print the raw response first so we can inspect Ollama output
    match client
        .complete(&astraweave_llm::build_prompt(snap, reg))
        .await
    {
        Ok(raw) => {
            println!("--- Raw Ollama response start ---");
            println!("{}", raw);
            println!("--- Raw Ollama response end ---");
            // Now attempt to parse/convert into PlanIntent
            match astraweave_llm::parse_llm_plan(&raw, reg) {
                Ok(plan) => {
                    println!("✓ Ollama produced a valid plan (direct parse):");
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                Err(_) => {
                    // Fall back to the full plan_from_llm which includes extraction/recovery steps
                    let plan_source = astraweave_llm::plan_from_llm(&client, snap, reg).await;
                    let plan = match plan_source {
                        PlanSource::Llm(p) => p,
                        PlanSource::Fallback { plan: p, reason } => {
                            println!("Ollama fell back: {}", reason);
                            p
                        }
                        _ => {
                            anyhow::bail!("Unsupported LLM plan source returned by astraweave-llm")
                        }
                    };
                    println!("✓ Ollama generated plan (via plan_from_llm):");
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch raw response from Ollama: {}", e);
            println!("  Make sure Ollama is running and the model is available");
        }
    }
    Ok(())
}

#[cfg(feature = "ollama")]
async fn test_local_http_client(
    snap: &WorldSnapshot,
    reg: &ToolRegistry,
    url: &str,
) -> anyhow::Result<()> {
    let model = env::var("LOCAL_LLM_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
    let client = if let Ok(api_key) = env::var("LOCAL_LLM_API_KEY") {
        LocalHttpClient::with_api_key(url.to_string(), model, api_key)
    } else {
        LocalHttpClient::new(url.to_string(), model)
    };

    println!("Connecting to local LLM at: {}", url);
    let plan_source = plan_from_llm(&client, snap, reg).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { plan: p, reason } => {
            println!("Local HTTP client fell back: {}", reason);
            p
        }
        _ => anyhow::bail!("Unsupported LLM plan source returned by astraweave-llm"),
    };
    println!("✓ Local HTTP client generated plan:");
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

#[cfg(feature = "ollama")]
async fn probe_ollama_tags(base_url: &str) -> anyhow::Result<()> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    println!("Probing Ollama for available models at: {} (GET)", url);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to reach Ollama /api/tags: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Ollama /api/tags returned {}: {}",
            status,
            txt
        ));
    }

    let body = resp.text().await.unwrap_or_default();
    println!("Ollama /api/tags response: {}", body);
    Ok(())
}

fn create_test_scenario() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 85,
            pos: IVec2 { x: 2, y: 3 },
            stance: "crouch".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 25,
            cooldowns: Default::default(),
            morale: 0.8,
            pos: IVec2 { x: 4, y: 3 },
        },
        enemies: vec![
            EnemyState {
                id: 101,
                pos: IVec2 { x: 15, y: 5 },
                hp: 75,
                cover: "high".into(),
                last_seen: 0.5,
            },
            EnemyState {
                id: 102,
                pos: IVec2 { x: 12, y: 8 },
                hp: 40,
                cover: "none".into(),
                last_seen: 1.0,
            },
        ],
        pois: vec![
            Poi {
                k: "extract_point".into(),
                pos: IVec2 { x: 20, y: 10 },
            },
            Poi {
                k: "ammo_cache".into(),
                pos: IVec2 { x: 8, y: 6 },
            },
        ],
        obstacles: vec![],
        objective: Some("Reach extraction point while providing cover".into()),
    }
}

fn create_tool_registry() -> ToolRegistry {
    ToolRegistry {
        tools: vec![
            ToolSpec {
                name: "move_to".into(),
                args: [("x", "i32"), ("y", "i32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "throw".into(),
                args: [
                    ("item", "enum[smoke,grenade,flashbang]"),
                    ("x", "i32"),
                    ("y", "i32"),
                ]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
            },
            ToolSpec {
                name: "cover_fire".into(),
                args: [("target_id", "u32"), ("duration", "f32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "revive".into(),
                args: [("ally_id", "u32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
        ],
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    }
}

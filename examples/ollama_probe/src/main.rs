use futures_util::StreamExt;
use reqwest::Client;
use std::env;

const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:4b";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
    println!("Probe Ollama {} model {}", url, model);

    let client = Client::new();
    // Build a realistic planning prompt (mirrors astraweave_llm::build_prompt)
    let snapshot = serde_json::json!({
        "t": 1.0,
        "player": { "hp": 85, "pos": { "x": 2, "y": 3 }, "stance": "crouch", "orders": [] },
        "me": { "ammo": 25, "cooldowns": {}, "morale": 0.8, "pos": { "x": 4, "y": 3 } },
        "enemies": [ { "id": 101, "pos": { "x": 15, "y": 5 }, "hp": 75, "cover": "high", "last_seen": 0.5 }, { "id": 102, "pos": { "x": 12, "y": 8 }, "hp": 40, "cover": "none", "last_seen": 1.0 } ],
        "pois": [ { "k": "extract_point", "pos": { "x": 20, "y": 10 } }, { "k": "ammo_cache", "pos": { "x": 8, "y": 6 } } ],
        "objective": "Reach extraction point while providing cover"
    });

    let tool_list = " - move_to { x: i32, y: i32 }\n - throw { item: enum[smoke,grenade,flashbang], x: i32, y: i32 }\n - cover_fire { duration: f32, target_id: u32 }\n - revive { ally_id: u32 }";
    let schema = r#"
Strict JSON schema:
{
  "plan_id": "string",
  "steps": [
     {"act":"MoveTo","x":INT,"y":INT} |
     {"act":"Throw","item":"smoke|grenade","x":INT,"y":INT} |
     {"act":"CoverFire","target_id":INT,"duration":FLOAT} |
     {"act":"Revive","ally_id":INT}
  ]
}
Return ONLY JSON with no commentary.
"#;

    let prompt = format!(
        "You are an AI game companion planner. Convert the world snapshot into a legal action plan.\nUse ONLY allowed tools and arguments. Do not exceed cooldown or LOS checks (the engine will validate).\nAllowed tools:\n{tools}\n\nSnapshot (redacted):\n{snap}\n\n{schema}",
        tools = tool_list,
        snap = serde_json::to_string_pretty(&snapshot).unwrap(),
        schema = schema
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": true,
        "think": false,
        "options": {
            "num_predict": 192,
            "temperature": 0.5,
            "top_k": 20
        }
    });
    let resp = client
        .post(format!("{}/api/chat", url.trim_end_matches('/')))
        .json(&body)
        .send()
        .await?;
    println!("status {}", resp.status());
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut acc = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(b) => {
                if let Ok(s) = std::str::from_utf8(&b) {
                    buf.push_str(s);
                    while let Some(pos) = buf.find('\n') {
                        let line = buf[..pos].trim();
                        let rest = buf[pos + 1..].to_string();
                        if !line.is_empty() {
                            println!("LINE: {}", line);
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(msg) = v.get("message") {
                                    if let Some(c) = msg.get("content").and_then(|x| x.as_str()) {
                                        acc.push_str(c);
                                    }
                                    if let Some(c) = msg.get("thinking").and_then(|x| x.as_str()) {
                                        if acc.is_empty() {
                                            acc.push_str(c);
                                        }
                                    }
                                }
                                if let Some(r) = v.get("response").and_then(|x| x.as_str()) {
                                    acc.push_str(r);
                                }
                            }
                        }
                        buf = rest;
                    }
                }
            }
            Err(e) => {
                println!("stream err: {}", e);
            }
        }
    }
    println!("acc len {}", acc.len());
    std::fs::write("target/ollama_probe_assistant_acc.txt", &acc)?;
    Ok(())
}

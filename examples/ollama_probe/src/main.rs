use futures_util::StreamExt;
use std::env;

const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:4b";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = astraweave_llm::loopback_model_url(
        env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
        false,
    );
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
    println!("Probe Ollama {} model {}", url, model);

    let client = astraweave_llm::loopback_http_client()
        .build()
        .map_err(|err| anyhow::anyhow!("loopback http client: {err}"))?;
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
    let idle = astraweave_llm::stream_read_idle();
    let requested = std::time::Instant::now();
    let lifetime_total =
        astraweave_llm::stream_call_lifetime(astraweave_llm::DEFAULT_STREAM_LIFETIME);
    let resp = astraweave_llm::send_headers_for_call(
        idle,
        lifetime_total.saturating_sub(requested.elapsed()),
        client
            .post(format!("{}/api/chat", url.trim_end_matches('/')))
            .json(&body)
            .send(),
    )
    .await?;
    println!("status {}", resp.status());
    let overall = idle.saturating_mul(2);
    let lifetime = lifetime_total.saturating_sub(requested.elapsed());
    let mut stream = Box::pin(astraweave_llm::bytes_within_deadline(
        Box::pin(resp.bytes_stream()),
        idle,
        overall,
        lifetime,
    ));
    let mut seen = 0usize;
    let mut buf = Vec::new();
    let mut acc = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(b) => {
                if let Err(err) = astraweave_llm::note_stream_bytes(&mut seen, b.len()) {
                    println!("stream err: {err:#}");
                    break;
                }
                let mut capped = false;
                let mut utf8_error: Option<anyhow::Error> = None;
                for item in take_complete_lines(&mut buf, &b) {
                    let line = match item {
                        Ok(line) => line,
                        Err(err) => {
                            utf8_error = Some(err);
                            break;
                        }
                    };
                    println!("LINE: {line}");
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(msg) = v.get("message") {
                            if let Some(c) = msg.get("content").and_then(|x| x.as_str()) {
                                if !push_capped(&mut acc, c) {
                                    capped = true;
                                    break;
                                }
                            }
                            if let Some(c) = msg.get("thinking").and_then(|x| x.as_str()) {
                                if acc.is_empty() && !push_capped(&mut acc, c) {
                                    capped = true;
                                    break;
                                }
                            }
                        }
                        if let Some(r) = v.get("response").and_then(|x| x.as_str()) {
                            if !push_capped(&mut acc, r) {
                                capped = true;
                                break;
                            }
                        }
                    }
                }
                if let Some(err) = utf8_error {
                    println!("stream err: {err:#}");
                    println!("acc len {}", acc.len());
                    std::fs::write("target/ollama_probe_assistant_acc.txt", &acc)?;
                    return Err(err);
                }
                if capped {
                    println!("stream err: model text exceeds 64 KiB cap");
                    break;
                }
            }
            Err(e) => {
                println!("stream err: {e:#}");
                break;
            }
        }
    }
    println!("acc len {}", acc.len());
    std::fs::write("target/ollama_probe_assistant_acc.txt", &acc)?;
    Ok(())
}

/// Append `chunk` and return complete lines.
///
/// UTF-8 is checked per line. A multibyte character split across chunks stays
/// in `buf` until the newline that finishes it. A bad line is one error after
/// the lines already accepted. Lines after it in this chunk are not decoded.
/// The consumed prefix, including that bad line, is drained.
fn take_complete_lines(buf: &mut Vec<u8>, chunk: &[u8]) -> Vec<Result<String, anyhow::Error>> {
    buf.extend_from_slice(chunk);
    let mut start = 0usize;
    let mut lines = Vec::new();
    while let Some(rel) = buf[start..].iter().position(|byte| *byte == b'\n') {
        let nl = start + rel;
        let mut end_line = nl;
        if end_line > start && buf[end_line - 1] == b'\r' {
            end_line -= 1;
        }
        let line_start = start;
        start = nl + 1;
        if line_start == end_line {
            continue;
        }
        match std::str::from_utf8(&buf[line_start..end_line]) {
            Ok(line) => {
                let line = line.trim();
                if !line.is_empty() {
                    lines.push(Ok(line.to_string()));
                }
            }
            Err(_) => {
                lines.push(Err(anyhow::anyhow!("model stream line is not utf-8")));
                break;
            }
        }
    }
    if start == buf.len() {
        buf.clear();
    } else if start > 0 {
        buf.drain(..start);
    }
    lines
}

fn push_capped(acc: &mut String, extra: &str) -> bool {
    if extra.is_empty() {
        return true;
    }
    if acc.len().saturating_add(extra.len()) > astraweave_llm::MAX_MODEL_JSON_BYTES {
        return false;
    }
    acc.push_str(extra);
    true
}

#[cfg(test)]
mod tests {
    use super::take_complete_lines;

    #[test]
    fn a_multibyte_character_split_across_chunks_stays_one_line() {
        let mut buf = Vec::new();
        let first = take_complete_lines(&mut buf, b"{\"message\":{\"content\":\"caf\xc3");
        assert!(first.is_empty(), "{first:?}");
        assert!(!buf.is_empty(), "the split character was dropped");
        let second = take_complete_lines(&mut buf, b"\xa9\"}}\n");
        assert_eq!(second.len(), 1, "{second:?}");
        assert!(
            second[0].as_ref().expect("line").contains("caf\u{e9}"),
            "{second:?}"
        );
        assert!(buf.is_empty(), "complete line left a tail: {buf:?}");
    }

    #[test]
    fn a_good_line_before_bad_utf8_is_kept_and_stops() {
        let mut buf = Vec::new();
        let items = take_complete_lines(&mut buf, b"{\"message\":{\"content\":\"hello\"}}\n\xff\n");
        assert_eq!(items.len(), 2, "{items:?}");
        let line = items[0].as_ref().expect("good");
        assert!(line.contains("hello"), "{line}");
        assert!(
            items[1]
                .as_ref()
                .expect_err("bad")
                .to_string()
                .contains("utf-8"),
            "{items:?}"
        );
        assert!(buf.is_empty(), "the good line was not drained: {buf:?}");
    }
}

use anyhow::Result;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    let client = astraweave_llm::loopback_http_client()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|err| anyhow::anyhow!("loopback http client: {err}"))?;
    let url =
        astraweave_llm::loopback_model_url("http://127.0.0.1:11434/api/chat".to_string(), false);
    let body = serde_json::json!({
        "model": "phi3:medium",
        "messages": [{"role": "user", "content": "Please reply with only JSON: {\"plan_id\":\"test-plan\",\"steps\":[]}"}],
        "stream": false
    });

    println!("POSTING to {}", url);
    let resp = client.post(url).json(&body).send().await?;
    println!("Status: {}", resp.status());
    let text = resp.text().await?;
    println!("Raw response:\n{}", text);

    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        println!("Parsed JSON: {}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("Response not JSON");
    }

    Ok(())
}

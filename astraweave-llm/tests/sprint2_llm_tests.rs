use anyhow::Result;
use astraweave_llm::{LlmClient, MockLlm};
use futures_util::StreamExt;
use std::pin::Pin;

// Mock client that supports streaming
struct MockStreamingClient {
    chunks: Vec<String>,
}

impl MockStreamingClient {
    fn new(chunks: Vec<&str>) -> Self {
        Self {
            chunks: chunks.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for MockStreamingClient {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self.chunks.join(""))
    }

    async fn complete_streaming(
        &self,
        _prompt: &str,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<String>> + Send>>> {
        let chunks = self.chunks.clone();
        let stream = futures_util::stream::iter(chunks).map(Ok);
        Ok(Box::pin(stream))
    }
}

#[tokio::test]
async fn test_mock_llm_streaming_default() {
    // MockLlm uses default implementation which wraps complete()
    let client = MockLlm;
    let mut stream = client.complete_streaming("test").await.unwrap();

    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        response.push_str(&chunk.unwrap());
    }

    assert!(response.contains("llm-mock"));
}

#[tokio::test]
async fn test_custom_streaming_client() {
    let client = MockStreamingClient::new(vec!["Hello", " ", "World"]);
    let mut stream = client.complete_streaming("test").await.unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap());
    }

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], "Hello");
    assert_eq!(chunks[1], " ");
    assert_eq!(chunks[2], "World");
}

#[cfg(feature = "ollama")]
fn ollama_chat_probe(mode: &str) -> std::process::Output {
    // Timeouts are on the child only. `Command::env` does not call `set_var`.
    std::process::Command::new(env!("CARGO_BIN_EXE_ollama_chat_probe"))
        .arg(mode)
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .env("OLLAMA_NONSTREAM_TIMEOUT_SECS", "1")
        .env("OLLAMA_NONSTREAM_ATTEMPTS", "1")
        .output()
        .expect("spawn ollama chat probe")
}

#[cfg(feature = "ollama")]
#[test]
fn test_ollama_client_connection_error() {
    let out = ollama_chat_probe("complete");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
    let err = stdout.to_lowercase();
    assert!(
        err.contains("error") || err.contains("fail") || err.contains("timeout"),
        "{stdout} {stderr}"
    );
}

#[cfg(feature = "ollama")]
#[test]
fn test_ollama_batch_execution() {
    let out = ollama_chat_probe("batch");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
}

#[test]
fn test_streaming_parser_code_fences() {
    use astraweave_llm::streaming_parser::StreamingBatchParser;

    let json = r#"```json
    [
        {"agent_id": 1, "plan_id": "p1", "steps": []}
    ]
    ```"#;

    let mut parser = StreamingBatchParser::with_expected_count(1);
    let plans = parser.feed_chunk(json).unwrap();

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].agent_id, 1);
}

#[test]
fn test_streaming_parser_malformed() {
    use astraweave_llm::streaming_parser::StreamingBatchParser;

    // Malformed JSON inside array
    let json = r#" [
        {"agent_id": 1, "plan_id": "p1", "steps": [}
    ]"#;

    let mut parser = StreamingBatchParser::with_expected_count(1);
    let plans = parser.feed_chunk(json).unwrap();

    // Should parse nothing or error, but definitely not crash
    assert_eq!(plans.len(), 0);
}

#[test]
fn test_streaming_parser_performance() {
    use astraweave_llm::streaming_parser::StreamingBatchParser;
    use std::time::Instant;

    // Generate a large JSON array (approx 1MB)
    let mut json = String::from("[\n");
    for i in 0..1000 {
        json.push_str(&format!(
            r#"{{"agent_id": {}, "plan_id": "p{}", "steps": []}},"#,
            i, i
        ));
    }
    // Remove last comma and close array
    json.pop();
    json.push_str("\n]");

    let start = Instant::now();
    let mut parser = StreamingBatchParser::with_expected_count(1000);
    let plans = parser.feed_chunk(&json).unwrap();
    let duration = start.elapsed();

    assert_eq!(plans.len(), 1000);
    // Should be fast (< 100ms)
    assert!(
        duration.as_millis() < 100,
        "Parsing took too long: {}ms",
        duration.as_millis()
    );
}

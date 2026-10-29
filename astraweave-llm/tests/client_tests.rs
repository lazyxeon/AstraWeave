#![cfg(feature = "ollama")]
use astraweave_llm::{LlmClient, OllamaChatClient, OllamaClient};
use mockito::{Matcher, Server};
use std::sync::Mutex;

/// Serializes env edits. `OLLAMA_LOW_LATENCY` is read when the client is built.
static CLIENT_ENV: Mutex<()> = Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    CLIENT_ENV.lock().unwrap_or_else(|err| err.into_inner())
}

struct NonStreamAttempts {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl NonStreamAttempts {
    fn new() -> Self {
        let lock = env_lock();
        // SAFETY: every test in this binary holds CLIENT_ENV before it builds a client.
        unsafe {
            std::env::set_var("OLLAMA_LOW_LATENCY", "false");
            std::env::set_var("OLLAMA_NONSTREAM_ATTEMPTS", "2");
        }
        Self { _lock: lock }
    }
}

impl Drop for NonStreamAttempts {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("OLLAMA_LOW_LATENCY");
            std::env::remove_var("OLLAMA_NONSTREAM_ATTEMPTS");
        }
    }
}

fn move_to_format() -> serde_json::Value {
    let reg = astraweave_core::ToolRegistry {
        tools: vec![astraweave_core::ToolSpec {
            name: "MoveTo".into(),
            args: [("x".into(), "i32".into()), ("y".into(), "i32".into())]
                .into_iter()
                .collect(),
        }],
        constraints: astraweave_core::Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };
    astraweave_llm::plan_format::plan_format_for_request(&reg).expect("schema")
}

fn plan_json() -> &'static str {
    r#"{"plan_id":"p","steps":[{"act":"MoveTo","x":1,"y":2}]}"#
}

fn chat_line(content: &str) -> String {
    format!("{}\n", serde_json::json!({"message": {"content": content}}))
}

fn assert_plan(text: &str) {
    let reg = astraweave_core::ToolRegistry {
        tools: vec![astraweave_core::ToolSpec {
            name: "MoveTo".into(),
            args: std::collections::BTreeMap::new(),
        }],
        constraints: astraweave_core::Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };
    let plan = astraweave_llm::parse_llm_plan(text, &reg).expect("parsed plan");
    assert_eq!(plan.plan_id, "p");
    assert_eq!(plan.steps.len(), 1);
}

#[tokio::test]
async fn test_ollama_chat_client_success() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{\"message\": {\"content\": \"{\\\"plan_id\\\":\\\"test\\\"}\"}}\n")
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("model client");
    let response = client.complete("test prompt").await.unwrap();

    assert_eq!(response, "{\"plan_id\":\"test\"}");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_ollama_chat_client_sends_registry_format() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/api/chat")
        .match_body(mockito::Matcher::Regex(
            r#""format":\{.*"enum":\["MoveTo"\].*"x":\{"type":"integer"\}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{\"message\": {\"content\": \"{\\\"plan_id\\\":\\\"fmt\\\"}\"}}\n")
        .create_async()
        .await;

    let reg = astraweave_core::ToolRegistry {
        tools: vec![astraweave_core::ToolSpec {
            name: "MoveTo".into(),
            args: [("x".into(), "i32".into()), ("y".into(), "i32".into())]
                .into_iter()
                .collect(),
        }],
        constraints: astraweave_core::Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };
    let format = astraweave_llm::plan_format::plan_format_for_request(&reg).expect("schema");
    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("model client");
    let response = client
        .complete_with_format("test prompt", &format)
        .await
        .unwrap();

    assert_eq!(response, "{\"plan_id\":\"fmt\"}");
    assert!(!format.to_string().contains("CoverFire"));
    mock.assert_async().await;
}

#[tokio::test]
async fn test_ollama_chat_client_failure() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/api/chat")
        .with_status(500)
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("model client");
    let result = client.complete("test prompt").await;

    assert!(result.is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn generate_schema_400_retries_without_format() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();
    let rejected = server
        .mock("POST", "/api/generate")
        .match_body(Matcher::Regex(r#""format":\{"#.to_string()))
        .with_status(400)
        .with_body(r#"{"error":"invalid format: schema"}"#)
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("POST", "/api/generate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::json!({"response": plan_json()}).to_string())
        .expect(1)
        .create_async()
        .await;

    let client = OllamaClient::new(url, "test-model").expect("client");
    let text = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect("retry succeeded");
    assert_plan(&text);
    rejected.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn generate_400_without_schema_phrase_does_not_retry() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();
    let once = server
        .mock("POST", "/api/generate")
        .with_status(400)
        .with_body(r#"{"error":"model not found"}"#)
        .expect(1)
        .create_async()
        .await;

    let client = OllamaClient::new(url, "test-model").expect("client");
    let err = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect_err("no retry");
    let message = err.to_string();
    assert!(
        message.contains("model not found") || message.contains("400"),
        "{message}"
    );
    once.assert_async().await;
}

#[tokio::test]
async fn chat_stream_schema_400_retries_without_format() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();
    let rejected = server
        .mock("POST", "/api/chat")
        .match_body(Matcher::Regex(r#""format":\{"#.to_string()))
        .with_status(400)
        .with_body(r#"{"error":"invalid format: schema"}"#)
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(chat_line(plan_json()))
        .expect(1)
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("client");
    let text = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect("stream retry");
    assert_plan(&text);
    rejected.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn chat_nonstream_schema_400_retries_without_format() {
    let _env = NonStreamAttempts::new();
    let mut server = Server::new_async().await;
    let url = server.url();
    let rejected = server
        .mock("POST", "/api/chat")
        .match_body(Matcher::Regex(r#""format":\{"#.to_string()))
        .with_status(400)
        .with_body(r#"{"error":"invalid format: schema"}"#)
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(chat_line(plan_json()))
        .expect(1)
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("client");
    let text = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect("non-stream retry");
    assert_plan(&text);
    rejected.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn chat_404_generate_schema_400_retries_without_format() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();
    let chat_missing = server
        .mock("POST", "/api/chat")
        .with_status(404)
        .with_body("missing")
        .expect(1)
        .create_async()
        .await;
    let generate_rejected = server
        .mock("POST", "/api/generate")
        .match_body(Matcher::Regex(r#""format":\{"#.to_string()))
        .with_status(400)
        .with_body(r#"{"error":"unsupported format"}"#)
        .expect(1)
        .create_async()
        .await;
    let generate_ok = server
        .mock("POST", "/api/generate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            "{}\n",
            serde_json::json!({"response": plan_json(), "done": true})
        ))
        .expect(1)
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("client");
    let text = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect("generate retry");
    assert_plan(&text);
    chat_missing.assert_async().await;
    generate_rejected.assert_async().await;
    generate_ok.assert_async().await;
}

#[tokio::test]
async fn chat_400_without_schema_phrase_does_not_retry() {
    let _guard = env_lock();
    let mut server = Server::new_async().await;
    let url = server.url();
    let once = server
        .mock("POST", "/api/chat")
        .with_status(400)
        .with_body(r#"{"error":"model not found"}"#)
        .expect(1)
        .create_async()
        .await;

    let client = OllamaChatClient::new(url, "test-model".to_string()).expect("client");
    let err = client
        .complete_with_format("plan", &move_to_format())
        .await
        .expect_err("no retry");
    assert!(err.to_string().contains("400"), "{err}");
    once.assert_async().await;
}

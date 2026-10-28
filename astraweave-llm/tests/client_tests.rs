#![cfg(feature = "ollama")]
use astraweave_llm::{LlmClient, OllamaChatClient};
use mockito::Server;

#[tokio::test]
async fn test_ollama_chat_client_success() {
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

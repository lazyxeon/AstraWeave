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

//! Hermes 2 Pro Integration via Ollama
//!
//! **DEPRECATED**: This module is superseded by [`qwen3_ollama`](crate::qwen3_ollama).
//! Qwen3-8B provides 4× larger context (32K vs 8K), dual thinking modes,
//! and improved structured output. Use [`Qwen3Ollama`](crate::qwen3_ollama::Qwen3Ollama)
//! for all new code.
//!
//! This module is retained for backward compatibility and will be removed in a future release.
//!
//! ## Migration
//! ```rust,ignore
//! // Old:
//! use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
//! let client = Hermes2ProOllama::localhost().expect("model client");
//!
//! // New:
//! use astraweave_llm::qwen3_ollama::Qwen3Ollama;
//! let client = Qwen3Ollama::localhost().expect("model client");
//! ```
//!
//! ---
//!
//! This module provides a production-ready interface to Nous Research's Hermes 2 Pro Mistral 7B model
//! via Ollama, which handles model management, quantization, and inference.
//!
//! # Why Hermes 2 Pro?
//! - **Function Calling**: Native support for OpenAI-compatible tool use
//! - **Higher Success Rate**: 75-85% vs 40-50% with Phi-3 (1.75× improvement)
//! - **Larger Context**: 8192 tokens (2× Phi-3's 4096)
//! - **JSON Reliability**: Trained specifically for structured output
//!
//! # Why Ollama?
//! - **Zero Setup**: No manual model downloads or GGUF files
//! - **Auto Quantization**: Ollama handles Q4/Q5/Q8 automatically
//! - **GPU Support**: Works with CUDA, Metal, ROCm out-of-the-box
//! - **Production Ready**: Used by thousands of applications
//!
//! # Quick Start
//! ```bash
//! # Install Ollama: https://ollama.ai
//! ollama pull adrienbrault/nous-hermes2pro:Q4_K_M    # Downloads Hermes 2 Pro Q4 (~4.4GB)
//! ollama serve                                        # Start server on localhost:11434
//! ```
//!
//! # Usage
//! ```ignore
//! use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
//! use astraweave_llm::LlmClient;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let client = Hermes2ProOllama::new("http://localhost:11434", "adrienbrault/nous-hermes2pro:Q4_K_M").expect("model client");
//!     let response = client.complete("You are a game AI. Plan your next action.").await?;
//!     Ok(())
//! }
//! ```

use crate::LlmClient;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Stateful chat session for Hermes 2 Pro
///
/// Manages conversation history and context window automatically.
pub struct ChatSession {
    client: Hermes2ProOllama,
    history: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(client: Hermes2ProOllama) -> Self {
        let history = if let Some(sys) = &client.system_prompt {
            vec![ChatMessage {
                role: "system".to_string(),
                content: sys.clone(),
            }]
        } else {
            Vec::new()
        };

        Self {
            client,
            history: Arc::new(Mutex::new(history)),
        }
    }

    /// Send a message and get response (updates history)
    pub async fn send(&self, message: &str) -> Result<String> {
        let mut history = self.history.lock().await;

        // Add user message
        history.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        // Prepare request
        let response = self.client.chat(&history).await?;

        // Add assistant response
        history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.clone(),
        });

        Ok(response)
    }

    /// Clear history (preserves system prompt)
    pub async fn clear(&self) {
        let mut history = self.history.lock().await;
        history.retain(|m| m.role == "system");
    }

    /// Get current history
    pub async fn get_history(&self) -> Vec<ChatMessage> {
        self.history.lock().await.clone()
    }
}

/// Hermes 2 Pro client using Ollama backend
///
/// This is the **recommended** way to use Hermes 2 Pro in AstraWeave. Ollama handles
/// all the complexity of model loading, quantization, and GPU acceleration.
///
/// ## Model Variant
/// - `adrienbrault/nous-hermes2pro:Q4_K_M` - 7B parameters, ~4.4GB - **RECOMMENDED**
///
/// This is a Q4_K_M quantization of Nous Research's Hermes 2 Pro Mistral 7B model,
/// specifically trained for function calling and tool use (OpenAI-compatible format).
///
/// ## Performance (RTX 3060, 12GB VRAM)
/// - Load time: 2-3 seconds (first request)
/// - Inference: 35-50 tokens/sec @ Q4_K_M
/// - Memory: ~5GB VRAM
/// - Success rate: 75-85% (vs 40-50% Phi-3)
/// - Latency: 2-4s average (vs 3-5s Phi-3)
///
/// ## Migration from Phi-3
/// Hermes 2 Pro uses ChatML format (`<|im_start|>` / `<|im_end|>`) instead of
/// Phi-3's custom format, but Ollama handles this automatically. No code changes
/// needed for chat formatting!
#[derive(Debug, Clone)]
pub struct Hermes2ProOllama {
    /// Ollama API endpoint (default: http://localhost:11434)
    url: String,
    allow_remote: bool,

    /// Model name (default: "adrienbrault/nous-hermes2pro:Q4_K_M")
    pub model: String,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,

    /// Maximum tokens to generate
    pub max_tokens: usize,

    /// System prompt for game AI context
    pub system_prompt: Option<String>,

    client: reqwest::Client,
}

impl Hermes2ProOllama {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a new Hermes 2 Pro client with default settings
    ///
    /// # Arguments
    /// * `url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (default: "adrienbrault/nous-hermes2pro:Q4_K_M")
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
    /// let client = Hermes2ProOllama::new("http://localhost:11434", "adrienbrault/nous-hermes2pro:Q4_K_M").expect("model client");
    /// ```
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::assemble(url, model, false)
    }

    /// Keep a non-loopback URL and use reqwest's default client for it.
    ///
    /// That client honors `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, and it
    /// follows redirects. A loopback URL still uses [`crate::loopback_http_client`].
    /// `new` returns an error when the URL would be rewritten.
    pub fn allowing_remote(url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::assemble(url, model, true)
    }

    fn assemble(
        url: impl Into<String>,
        model: impl Into<String>,
        allow_remote: bool,
    ) -> Result<Self> {
        let url = crate::require_kept_model_url(url.into(), allow_remote)?;
        let client = crate::build_model_client(&url, allow_remote)?;
        Ok(Self {
            url,
            allow_remote,
            model: model.into(),
            temperature: 0.7,
            max_tokens: 512,
            system_prompt: Some(DEFAULT_SYSTEM_PROMPT.to_string()),
            client,
        })
    }

    /// Create Hermes 2 Pro client for localhost (convenience method)
    ///
    /// Uses the recommended Q4_K_M quantization for best balance of speed and quality.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
    /// let client = Hermes2ProOllama::localhost().expect("model client"); // Uses adrienbrault/nous-hermes2pro:Q4_K_M
    /// ```
    pub fn localhost() -> Result<Self> {
        Self::new(
            "http://localhost:11434",
            "adrienbrault/nous-hermes2pro:Q4_K_M",
        )
    }

    /// Create optimized Hermes 2 Pro client for low-latency game AI
    ///
    /// Uses lower temperature (0.5) and fewer max tokens (128) for faster inference.
    /// Expected latency: 1-3s on GTX 1660 Ti / RTX 3060.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
    /// let client = Hermes2ProOllama::fast().expect("model client"); // Low latency variant
    /// ```
    pub fn fast() -> Result<Self> {
        Ok(Self::new(
            "http://localhost:11434",
            "adrienbrault/nous-hermes2pro:Q4_K_M",
        )?
        .with_temperature(0.5)
        .with_max_tokens(128))
    }

    /// Set temperature (0.0 = deterministic, 1.0 = creative)
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// Set max tokens to generate
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// Set custom system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Clear system prompt (use only user messages)
    pub fn without_system_prompt(mut self) -> Self {
        self.system_prompt = None;
        self
    }

    /// Create a stateful chat session
    pub fn create_session(&self) -> ChatSession {
        ChatSession::new(self.clone())
    }

    /// Internal chat method using /api/chat
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        let url = format!("{}/api/chat", self.url);

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": 8192,
            }
        });

        let response_json: serde_json::Value = crate::with_call_deadline(120, async {
            let response = crate::send_for_headers(
                crate::stream_read_idle(),
                client.post(&url).json(&body).send(),
            )
            .await
            .context("Failed to send request to Ollama")?;

            if !response.status().is_success() {
                anyhow::bail!("Ollama returned error: {}", response.status());
            }

            crate::read_capped_json(response)
                .await
                .context("Failed to parse Ollama response")
        })
        .await?;

        let message = response_json["message"]["content"]
            .as_str()
            .context("Missing 'message.content' field in Ollama output")?
            .to_string();

        Ok(message)
    }

    /// Check if Ollama server is running and model is available
    pub async fn health_check(&self) -> Result<HealthStatus> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        // Check server is running
        let server_url = format!("{}/api/tags", self.url);
        let response =
            crate::send_for_headers(crate::stream_read_idle(), client.get(&server_url).send())
                .await
                .context("Failed to connect to Ollama server")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama server returned error: {}", response.status());
        }

        // Parse available models
        let body: serde_json::Value = crate::read_capped_json(response)
            .await
            .context("Failed to parse Ollama response")?;

        let models = body["models"]
            .as_array()
            .context("Invalid response from Ollama")?;

        let model_available = models
            .iter()
            .any(|m| m["name"].as_str().unwrap_or("") == self.model);

        Ok(HealthStatus {
            server_running: true,
            model_available,
            model_name: self.model.clone(),
            ollama_version: body["version"].as_str().unwrap_or("unknown").to_string(),
        })
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub server_running: bool,
    pub model_available: bool,
    pub model_name: String,
    pub ollama_version: String,
}

impl HealthStatus {
    pub fn is_ready(&self) -> bool {
        self.server_running && self.model_available
    }

    pub fn error_message(&self) -> Option<String> {
        if !self.server_running {
            Some("Ollama server not running. Start with: ollama serve".to_string())
        } else if !self.model_available {
            Some(format!(
                "Model '{}' not found. Download with: ollama pull {}",
                self.model_name, self.model_name
            ))
        } else {
            None
        }
    }
}

/// Default system prompt for game AI
///
/// Hermes 2 Pro is trained for function calling, so this prompt emphasizes
/// the JSON schema for AstraWeave's 37-tool action system.
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a tactical AI agent in a real-time game.
Your responses must be valid JSON following this schema:
{
  "plan_id": "unique_id",
  "reasoning": "brief explanation",
  "steps": [
    {"act": "MoveTo", "x": 10, "y": 5},
    {"act": "CoverFire", "target_id": 99, "duration": 2.0}
  ]
}

Available actions: MoveTo, Throw, CoverFire, Revive.
Always prioritize team survival and tactical advantage."#;

impl Hermes2ProOllama {
    async fn complete_formatted(
        &self,
        prompt: &str,
        format_schema: Option<&serde_json::Value>,
    ) -> Result<String> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        let url = format!("{}/api/chat", self.url);

        // Build messages array
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": prompt}));

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": 8192,  // Hermes 2 Pro supports 8192 tokens (2× Phi-3)
            }
        });
        if let Some(schema) = format_schema {
            crate::plan_format::insert_schema_format(&mut body, schema);
        }

        // ═══ DEBUG LOGGING ═══
        eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
        eprintln!("║        PROMPT SENT TO HERMES 2 PRO (via Ollama /api/chat)    ║");
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("Model: {}", self.model);
        eprintln!("Temperature: {}", self.temperature);
        eprintln!("Max Tokens: {}", self.max_tokens);
        eprintln!("Context Window: 8192 tokens");
        eprintln!("Prompt Length: {} chars", prompt.len());
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("{}", prompt);
        eprintln!("╚═══════════════════════════════════════════════════════════════╝\n");

        tracing::debug!("Sending request to Ollama: {}", self.model);
        let start = std::time::Instant::now();

        let response_json: serde_json::Value = crate::with_call_deadline(120, async {
            crate::post_ollama_json_value(client, &url, body).await
        })
        .await?;

        let text = response_json["message"]["content"]
            .as_str()
            .context("Missing 'message.content' field in Ollama output")?
            .to_string();

        let duration = start.elapsed();

        // ═══ DEBUG LOGGING ═══
        eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
        eprintln!("║        HERMES 2 PRO RAW RESPONSE (via Ollama)                ║");
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("Response Time: {:.2}s", duration.as_secs_f32());
        eprintln!("Response Length: {} chars", text.len());
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("{}", text);
        eprintln!("╚═══════════════════════════════════════════════════════════════╝\n");

        tracing::debug!(
            "Received {} chars from Ollama in {:.2}s",
            text.len(),
            duration.as_secs_f32()
        );

        Ok(text)
    }
}

/// Implement LlmClient trait for Ollama-based Hermes 2 Pro
#[async_trait]
impl LlmClient for Hermes2ProOllama {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.complete_formatted(prompt, None).await
    }

    async fn complete_with_format(
        &self,
        prompt: &str,
        format: &serde_json::Value,
    ) -> Result<String> {
        self.complete_formatted(prompt, Some(format)).await
    }

    /// Complete with streaming support (progressive response delivery)
    ///
    /// Returns a stream of text chunks as they arrive from Ollama. Enables:
    /// - Lower time-to-first-token (~8× faster first action vs blocking)
    /// - Progressive JSON parsing with StreamingParser
    /// - Batch inference with early plan delivery
    ///
    /// # Performance
    /// - Time-to-first-chunk: ~100-300ms (vs 1-3s for full response)
    /// - Chunk frequency: ~50-100ms intervals
    /// - Compatible with BatchExecutor for multi-agent inference
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
    /// # use astraweave_llm::LlmClient;
    /// # use futures_util::StreamExt;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Hermes2ProOllama::localhost().expect("model client");
    /// let mut stream = client.complete_streaming("Generate plan").await?;
    ///
    /// let mut full_response = String::new();
    /// while let Some(chunk) = stream.next().await {
    ///     let text = chunk?;
    ///     full_response.push_str(&text);
    ///     println!("Progress: {} chars", full_response.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn complete_streaming(
        &self,
        prompt: &str,
    ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String>> + Send>>> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;

        let idle = crate::stream_read_idle();
        let client = &self.client;

        let url = format!("{}/api/chat", self.url);

        // Build messages array
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": prompt}));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,  // ← Enable streaming!
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": 8192,
            }
        });

        // ═══ DEBUG LOGGING ═══
        eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
        eprintln!("║    STREAMING REQUEST TO HERMES 2 PRO (via Ollama /api/chat)  ║");
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("Model: {}", self.model);
        eprintln!("Temperature: {}", self.temperature);
        eprintln!("Max Tokens: {}", self.max_tokens);
        eprintln!("Stream: ENABLED");
        eprintln!("Prompt Length: {} chars", prompt.len());
        eprintln!("╚═══════════════════════════════════════════════════════════════╝\n");

        tracing::debug!("Sending streaming request to Ollama: {}", self.model);

        let requested = std::time::Instant::now();
        let lifetime_total = crate::stream_call_lifetime(crate::DEFAULT_STREAM_LIFETIME);
        let response = crate::send_headers_for_call(
            idle,
            lifetime_total.saturating_sub(requested.elapsed()),
            client.post(&url).json(&body).send(),
        )
        .await
        .context("Failed to send streaming request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama streaming returned error: {}", response.status());
        }

        // The header wait is the idle limit or the time still left in the
        // 120s lifetime, whichever is shorter. Silence on the body stops
        // after one idle period. A non-empty chunk starts a new gap. An
        // empty chunk does not. One error, or done:true, ends the stream.
        let overall = idle.saturating_mul(2);
        let lifetime = lifetime_total.saturating_sub(requested.elapsed());
        let text_stream = crate::scan_model_ndjson(
            Box::pin(response.bytes_stream()),
            idle,
            overall,
            lifetime,
            hermes_ndjson_line,
        );

        Ok(Box::pin(text_stream))
    }
}

fn hermes_ndjson_line(line: &str) -> crate::NdjsonYield {
    let json = match serde_json::from_str::<serde_json::Value>(line) {
        Ok(json) => json,
        Err(err) => {
            tracing::warn!("Failed to parse NDJSON line: {err} | Line: {line}");
            return crate::NdjsonYield {
                pieces: Vec::new(),
                stop: false,
            };
        }
    };
    let mut line_content = String::new();
    if let Some(content) = json["message"]["content"].as_str() {
        if !content.is_empty() {
            line_content = content.to_string();
        }
    } else if let Some(response) = json["response"].as_str() {
        if !response.is_empty() {
            line_content = response.to_string();
        }
    }
    let stop = json["done"].as_bool() == Some(true);
    if stop {
        tracing::debug!("Streaming complete (done: true)");
    }
    crate::NdjsonYield {
        pieces: if line_content.is_empty() {
            Vec::new()
        } else {
            vec![line_content]
        },
        stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes2pro_ollama_creation() {
        let client = Hermes2ProOllama::new(
            "http://localhost:11434",
            "adrienbrault/nous-hermes2pro:Q4_K_M",
        )
        .expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, "adrienbrault/nous-hermes2pro:Q4_K_M");
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 512);
        assert!(client.system_prompt.is_some());
    }

    #[test]
    fn test_localhost_convenience() {
        let client = Hermes2ProOllama::localhost().expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, "adrienbrault/nous-hermes2pro:Q4_K_M");
    }

    #[test]
    fn test_builder_pattern() {
        let client = Hermes2ProOllama::localhost()
            .expect("model client")
            .with_temperature(0.5)
            .with_max_tokens(256)
            .without_system_prompt();

        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 256);
        assert!(client.system_prompt.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_health_check() {
        let client = Hermes2ProOllama::localhost().expect("model client");
        let health = client.health_check().await;

        // If Ollama is running, this should succeed
        if let Ok(status) = health {
            assert!(status.server_running);
            println!("Ollama version: {}", status.ollama_version);
            println!("Model available: {}", status.model_available);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + adrienbrault/nous-hermes2pro:Q4_K_M
    async fn test_complete() {
        let client = Hermes2ProOllama::localhost().expect("model client");

        // Check health first
        let health = client.health_check().await.expect("Health check failed");
        if !health.is_ready() {
            panic!("{}", health.error_message().unwrap());
        }

        let prompt = "You are at position (5,5). Enemy at (10,8). Generate a tactical plan.";
        let response = client.complete(prompt).await.expect("Completion failed");

        assert!(!response.is_empty());
        println!("Hermes 2 Pro response:\n{}", response);

        // Try to parse as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
        if let Ok(json) = parsed {
            println!(
                "Parsed JSON: {}",
                serde_json::to_string_pretty(&json).unwrap()
            );
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + adrienbrault/nous-hermes2pro:Q4_K_M
    async fn test_complete_streaming() {
        use futures_util::StreamExt;

        let client = Hermes2ProOllama::localhost().expect("model client");

        // Check health first
        let health = client.health_check().await.expect("Health check failed");
        if !health.is_ready() {
            panic!("{}", health.error_message().unwrap());
        }

        let prompt = "You are at position (5,5). Enemy at (10,8). Generate a tactical plan.";

        println!("\n═══ Starting Streaming Test ═══");
        let start = std::time::Instant::now();

        let mut stream = client
            .complete_streaming(prompt)
            .await
            .expect("Streaming failed to start");

        let mut full_response = String::new();
        let mut chunk_count = 0;
        let mut time_to_first_chunk = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.expect("Chunk error");

            if time_to_first_chunk.is_none() {
                time_to_first_chunk = Some(start.elapsed());
                println!(
                    "⚡ Time to first chunk: {:.2}s",
                    time_to_first_chunk.unwrap().as_secs_f32()
                );
            }

            chunk_count += 1;
            full_response.push_str(&chunk);

            println!(
                "Chunk #{}: {} chars (total: {})",
                chunk_count,
                chunk.len(),
                full_response.len()
            );
        }

        let total_time = start.elapsed();

        println!("\n═══ Streaming Complete ═══");
        println!("Total chunks: {}", chunk_count);
        println!("Total time: {:.2}s", total_time.as_secs_f32());
        println!(
            "Time to first chunk: {:.2}s",
            time_to_first_chunk.unwrap().as_secs_f32()
        );
        println!("Final response length: {} chars", full_response.len());
        println!("\nFull response:\n{}", full_response);

        assert!(
            !full_response.is_empty(),
            "Streaming returned empty response"
        );
        assert!(chunk_count > 0, "No chunks received");

        // Verify time-to-first-chunk is significantly faster than total time
        let ttfc_ratio = time_to_first_chunk.unwrap().as_secs_f32() / total_time.as_secs_f32();
        println!(
            "\nTime-to-first-chunk ratio: {:.1}% of total time",
            ttfc_ratio * 100.0
        );

        // Try to parse as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&full_response);
        if let Ok(json) = parsed {
            println!(
                "\nParsed JSON: {}",
                serde_json::to_string_pretty(&json).unwrap()
            );
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + adrienbrault/nous-hermes2pro:Q4_K_M
    async fn test_streaming_vs_blocking_consistency() {
        use futures_util::StreamExt;

        let client = Hermes2ProOllama::localhost()
            .expect("model client")
            .with_temperature(0.0); // Deterministic for consistency check

        let prompt = "Generate JSON: {\"action\": \"move\", \"x\": 5, \"y\": 10}";

        // Get blocking response
        let blocking_response = client.complete(prompt).await.expect("Blocking failed");

        // Get streaming response
        let mut stream = client
            .complete_streaming(prompt)
            .await
            .expect("Streaming failed");
        let mut streaming_response = String::new();
        while let Some(chunk) = stream.next().await {
            streaming_response.push_str(&chunk.expect("Chunk error"));
        }

        println!(
            "Blocking response ({} chars):\n{}",
            blocking_response.len(),
            blocking_response
        );
        println!(
            "\nStreaming response ({} chars):\n{}",
            streaming_response.len(),
            streaming_response
        );

        // Responses should be identical (or at least very similar with temp=0.0)
        assert_eq!(
            blocking_response.trim(),
            streaming_response.trim(),
            "Streaming and blocking responses should match with temperature=0.0"
        );
    }
}

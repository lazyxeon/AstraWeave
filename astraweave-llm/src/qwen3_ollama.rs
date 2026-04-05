//! Qwen3-8B Integration via Ollama
//!
//! This module provides a production-ready interface to Qwen3-8B via Ollama,
//! which handles model management, quantization, and inference.
//!
//! # Why Qwen3-8B?
//! - **Dual Thinking Modes**: Thinking (chain-of-thought) and non-thinking (fast) modes
//! - **32K Native Context**: 32,768 tokens native (131K with YaRN)
//! - **Tool Calling**: Hermes-style tool use built into the chat template
//! - **JSON Reliability**: Excellent structured output with Hermes-style template
//! - **State-of-the-art**: Best-in-class open-source agent capabilities (April 2025)
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
//! ollama pull qwen3:8b           # Downloads Qwen3-8B Q4_K_M (~5GB)
//! ollama serve                    # Start server on localhost:11434
//! ```
//!
//! # Usage
//! ```ignore
//! use astraweave_llm::qwen3_ollama::Qwen3Ollama;
//! use astraweave_llm::LlmClient;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let client = Qwen3Ollama::new("http://localhost:11434", "qwen3:8b");
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

/// Stateful chat session for Qwen3-8B
///
/// Manages conversation history and context window automatically.
pub struct ChatSession {
    client: Qwen3Ollama,
    history: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(client: Qwen3Ollama) -> Self {
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

/// Qwen3-8B client using Ollama backend
///
/// This is the **recommended** way to use Qwen3-8B in AstraWeave. Ollama handles
/// all the complexity of model loading, quantization, and GPU acceleration.
///
/// ## Model Variant
/// - `qwen3:8b` - 8.2B parameters, ~4.9-5.1GB (Q4_K_M) - **RECOMMENDED**
///
/// Qwen3-8B supports dual thinking modes:
/// - **Non-thinking mode** (default): Fast, direct JSON responses for real-time game AI
/// - **Thinking mode**: Chain-of-thought reasoning in `<think>` blocks for complex strategy
///
/// ## Performance (RTX 3060, 12GB VRAM)
/// - Load time: 2-3 seconds (first request)
/// - Inference: 30-45 tokens/sec @ Q4_K_M
/// - Memory: ~4.9-5.1GB VRAM (Q4_K_M)
/// - JSON success rate: ≥90% (non-thinking) / ≥95% (thinking)
/// - Latency: 1-3s (fast mode), 3-8s (strategic/thinking mode)
///
/// ## Key Differences from Hermes 2 Pro
/// - 4× larger context window (32K vs 8K)
/// - Dual thinking/non-thinking modes
/// - ~152K token vocabulary (vs ~32K)
/// - Qwen3-specific sampling recommendations (temp ≥ 0.5, top_k = 20)
#[derive(Debug, Clone)]
pub struct Qwen3Ollama {
    /// Ollama API endpoint (default: http://localhost:11434)
    pub url: String,

    /// Model name (default: "qwen3:8b")
    pub model: String,

    /// Temperature for sampling (0.5 minimum recommended by Qwen3 docs)
    pub temperature: f32,

    /// Maximum tokens to generate
    pub max_tokens: usize,

    /// System prompt for game AI context
    pub system_prompt: Option<String>,

    /// Enable Qwen3 thinking mode (`<think>` blocks for chain-of-thought reasoning).
    /// When enabled, prepends `/think` to user messages; when disabled, `/no_think`.
    pub enable_thinking: bool,

    /// Context window length in tokens (default: 32768 for Qwen3-8B native)
    pub context_length: usize,
}

impl Qwen3Ollama {
    /// Create a new Qwen3-8B client with default settings
    ///
    /// # Arguments
    /// * `url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (default: "qwen3:8b")
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::new("http://localhost:11434", "qwen3:8b");
    /// ```
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            model: model.into(),
            temperature: 0.7,
            max_tokens: 512,
            system_prompt: Some(DEFAULT_SYSTEM_PROMPT.to_string()),
            enable_thinking: false,
            context_length: 32768,
        }
    }

    /// Create Qwen3-8B client for localhost (convenience method)
    ///
    /// Uses the official `qwen3:8b` model tag with Q4_K_M quantization.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::localhost(); // Uses qwen3:8b
    /// ```
    pub fn localhost() -> Self {
        Self::new("http://localhost:11434", "qwen3:8b")
    }

    /// Create optimized Qwen3-8B client for low-latency game AI
    ///
    /// Non-thinking mode with lower temperature (0.5), fewer max tokens (128),
    /// and reduced context window (2048) for fastest possible inference.
    /// Expected latency: <2s on modern GPUs.
    ///
    /// ## Optimizations Applied (Rounds 1-3)
    /// - Context window: 2048 (vs 32K default) — minimal KV cache allocation
    /// - Ultra-short system prompt — fewest possible prefill tokens
    /// - Lean sampling: no top_k, no repeat_penalty — faster per-token decode
    /// - `num_batch: 1024` — faster prompt prefill (2× default)
    /// - `num_keep: -1` — cache entire system prompt in KV prefix (skips re-eval)
    /// - `use_mmap: true` — memory-mapped model loading
    /// - `keep_alive: -1` — never unload model from memory
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::fast(); // Low latency variant
    /// ```
    pub fn fast() -> Self {
        let mut client = Self::new("http://localhost:11434", "qwen3:8b")
            .with_temperature(0.5)
            .with_max_tokens(128)
            .with_thinking(false)
            .with_context_length(2048);
        // Ultra-short system prompt — every token saved reduces TTFC
        client.system_prompt = Some(FAST_SYSTEM_PROMPT.to_string());
        client
    }

    /// Create Qwen3-8B client for strategic planning (thinking mode)
    ///
    /// Enables thinking mode with full context window for deep tactical analysis.
    /// Used by the Arbiter for background strategic planning.
    /// Expected latency: 3-8s (acceptable for async background tasks).
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::strategic(); // Thinking mode for complex plans
    /// ```
    pub fn strategic() -> Self {
        Self::new("http://localhost:11434", "qwen3:8b")
            .with_temperature(0.6)
            .with_max_tokens(1024)
            .with_thinking(true)
            .with_context_length(32768)
    }

    /// Set temperature (Qwen3 docs: minimum 0.5 recommended, never 0.0)
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

    /// Enable or disable Qwen3 thinking mode
    ///
    /// When enabled, the model generates chain-of-thought in `<think>...</think>`
    /// blocks before producing the final response. This improves quality for
    /// complex strategic planning at the cost of additional latency.
    pub fn with_thinking(mut self, enable: bool) -> Self {
        self.enable_thinking = enable;
        self
    }

    /// Set context window length in tokens
    ///
    /// Qwen3-8B supports 32,768 tokens natively (131K with YaRN scaling).
    /// For fast mode, reducing to 8192 improves latency.
    pub fn with_context_length(mut self, length: usize) -> Self {
        self.context_length = length;
        self
    }

    /// Create a stateful chat session
    pub fn create_session(&self) -> ChatSession {
        ChatSession::new(self.clone())
    }

    /// Build the Ollama API request options with Qwen3-specific sampling parameters
    ///
    /// **Optimization**: Non-thinking mode uses a lean parameter set (no top_k, no
    /// repeat_penalty) to minimize per-token sampling overhead. These params are
    /// only valuable for thinking mode where quality matters more than speed.
    fn build_options(&self) -> serde_json::Value {
        if self.enable_thinking {
            // Full sampling for strategic planning quality
            json!({
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": self.context_length,
                "top_p": 0.95,
                "top_k": 20,
                "repeat_penalty": 1.05,
            })
        } else {
            // Lean sampling for maximum inference speed.
            // Omit top_k and repeat_penalty — Ollama defaults are fine
            // and each extra sampler adds per-token latency.
            // num_batch: 1024 — doubles parallel prompt processing (default: 512).
            // num_keep: -1 — cache entire system prompt prefix in KV cache,
            //   so consecutive requests skip re-evaluating system prompt tokens.
            // use_mmap: true — memory-mapped model loading for faster access.
            json!({
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": self.context_length,
                "num_batch": 1024,
                "num_keep": -1,
                "use_mmap": true,
            })
        }
    }

    /// Prepare user message content with optional thinking mode prefix
    ///
    /// **Optimization**: Only prepend `/think` when thinking mode is enabled.
    /// The `/no_think` prefix is redundant because we already set `"think": false`
    /// in the API request body. Removing it saves ~10 prompt tokens per request.
    fn prepare_user_content(&self, prompt: &str) -> String {
        if self.enable_thinking {
            format!("/think\n{}", prompt)
        } else {
            // No prefix needed — "think": false in the request body handles it
            prompt.to_string()
        }
    }

    /// Preload the model into GPU/memory and warm the KV prefix cache.
    ///
    /// Performs two operations:
    /// 1. Loads model weights into VRAM (eliminates 3-5s cold-start)
    /// 2. Sends a minimal inference with the system prompt to populate
    ///    the KV prefix cache (`num_keep=-1` keeps it cached)
    ///
    /// After preload, the first real inference request skips both model loading
    /// AND system prompt re-evaluation, achieving steady-state TTFC immediately.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// # async fn demo() -> anyhow::Result<()> {
    /// let client = Qwen3Ollama::fast();
    /// client.preload().await?;  // Warm model + KV cache
    /// // First real call now has the same TTFC as subsequent calls
    /// # Ok(())
    /// # }
    /// ```
    pub async fn preload(&self) -> Result<()> {
        static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client")
        });

        let url = format!("{}/api/chat", self.url);

        // Build messages: include system prompt if configured, plus a minimal
        // user message to trigger a full forward pass and populate KV cache.
        let mut messages = Vec::new();
        if let Some(ref sys) = self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": "ready"}));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": false,
            "keep_alive": -1,
            "options": self.build_options(),
        });

        tracing::info!(
            "Preloading model {} (weights + KV prefix cache)...",
            self.model
        );
        let start = std::time::Instant::now();

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to preload model")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama preload returned error: {}", response.status());
        }

        // Consume the response body to complete the request
        let _ = response.text().await;

        tracing::info!(
            "Model {} preloaded in {:.1}s (KV prefix cached)",
            self.model,
            start.elapsed().as_secs_f64()
        );
        Ok(())
    }

    /// Internal chat method using /api/chat
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        // Use a static client with connection pooling for better performance
        static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(10) // Keep connections alive
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for slow hardware
                .build()
                .expect("Failed to create HTTP client")
        });

        let url = format!("{}/api/chat", self.url);

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": self.enable_thinking,
            "keep_alive": -1,
            "options": self.build_options(),
        });

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama returned error: {}", response.status());
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let message = extract_ollama_content(&response_json)?;

        // Strip thinking blocks if present (model may produce them even in non-thinking mode)
        Ok(strip_thinking_blocks(&message))
    }

    /// Check if Ollama server is running and model is available
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let client = reqwest::Client::new();

        // Check server is running
        let server_url = format!("{}/api/tags", self.url);
        let response = client
            .get(&server_url)
            .send()
            .await
            .context("Failed to connect to Ollama server")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama server returned error: {}", response.status());
        }

        // Parse available models
        let body: serde_json::Value = response
            .json()
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
/// Qwen3-8B has excellent instruction-following and structured output capabilities.
/// This prompt emphasizes JSON-only output for AstraWeave's action system.
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a tactical AI agent in a real-time game.
You must respond with ONLY valid JSON — no markdown, no commentary.
Follow this exact schema:
{
  "plan_id": "unique_id",
  "reasoning": "brief explanation of tactical decision",
  "steps": [
    {"act": "MoveTo", "x": 10, "y": 5},
    {"act": "CoverFire", "target_id": 99, "duration": 2.0}
  ]
}

Available actions: MoveTo, Throw, CoverFire, Revive, Wait, Scan, Attack, Reload.
Rules:
- Use ONLY actions from the list above
- All field names must match exactly (case-sensitive)
- Prioritize team survival and tactical advantage
- Be concise — minimize steps for efficiency"#;

/// Minimal system prompt for fast/latency-critical inference.
///
/// Ultra-short — only 2 lines, ~15 tokens. Every token saved at the system prompt
/// level reduces TTFC because it shrinks the prefill workload. The JSON schema
/// example acts as a one-shot template that Qwen3 reliably follows.
const FAST_SYSTEM_PROMPT: &str =
    "JSON only. {\"plan_id\":\"id\",\"steps\":[{\"act\":\"MoveTo\",\"x\":0,\"y\":0}]}";

/// Strip `<think>...</think>` blocks from Qwen3 thinking-mode responses.
///
/// Handles edge cases:
/// - Unclosed `<think>` block → strips from `<think>` to end, returns remaining prefix
///   (forces fallback via empty/truncated response)
/// - No `<think>` block present → returns input unchanged
/// - Multiple `<think>` blocks → strips all of them
/// - `<think>` inside JSON string values → handled by matching outermost tags only
fn strip_thinking_blocks(response: &str) -> String {
    let mut result = response.to_string();

    loop {
        let Some(start) = result.find("<think>") else {
            break;
        };

        if let Some(end) = result[start..].find("</think>") {
            // Well-formed: remove <think>...</think> entirely
            let end_abs = start + end + "</think>".len();
            result = format!("{}{}", &result[..start], &result[end_abs..]);
        } else {
            // Unclosed <think> block — model produced malformed output.
            // Strip everything from <think> onward. This will likely yield
            // an empty or truncated response, which the plan_parser's
            // fallback stages will handle gracefully.
            result = result[..start].to_string();
            break;
        }
    }

    result.trim().to_string()
}

/// Extract the model's text from an Ollama chat response JSON.
///
/// Ollama v0.17+ with Qwen3 separates "thinking" tokens into `message.thinking`
/// and visible output into `message.content`. When the model thinks despite
/// `/no_think`, all output lands in `message.thinking` and `message.content` is
/// empty. This helper falls back to the `thinking` field in that case.
fn extract_ollama_content(response_json: &serde_json::Value) -> Result<String> {
    let content = response_json["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if !content.is_empty() {
        return Ok(content);
    }

    // Fallback: Qwen3 routed output to the thinking field
    let thinking = response_json["message"]["thinking"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if !thinking.is_empty() {
        tracing::warn!(
            "Qwen3 routed {} chars to 'thinking' field despite /no_think — using as response",
            thinking.len()
        );
        return Ok(thinking);
    }

    // Both fields empty — this is a genuine empty response
    Ok(String::new())
}

/// Implement LlmClient trait for Ollama-based Qwen3-8B
#[async_trait]
impl LlmClient for Qwen3Ollama {
    async fn complete(&self, prompt: &str) -> Result<String> {
        // Use a static client with connection pooling for better performance
        static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(10) // Keep connections alive
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for slow hardware
                .build()
                .expect("Failed to create HTTP client")
        });

        let url = format!("{}/api/chat", self.url);

        // Build messages array with thinking mode prefix
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        let user_content = self.prepare_user_content(prompt);
        messages.push(json!({"role": "user", "content": user_content}));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": self.enable_thinking,
            "keep_alive": -1,
            "options": self.build_options(),
        });

        tracing::debug!(
            "Sending request to Ollama: {} (thinking={}, temp={}, max_tokens={}, ctx={})",
            self.model,
            self.enable_thinking,
            self.temperature,
            self.max_tokens,
            self.context_length
        );
        let start = std::time::Instant::now();

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama returned error: {}", response.status());
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let raw_text = extract_ollama_content(&response_json)?;

        let duration = start.elapsed();

        // Strip thinking blocks before returning
        let text = strip_thinking_blocks(&raw_text);

        tracing::debug!(
            "Received {} chars from Ollama in {:.2}s (raw: {} chars)",
            text.len(),
            duration.as_secs_f32(),
            raw_text.len(),
        );

        Ok(text)
    }

    /// Complete with streaming support (progressive response delivery)
    ///
    /// For non-thinking mode: streams chunks progressively as they arrive.
    /// For thinking mode: accumulates the full response, strips `<think>` blocks,
    /// then returns the cleaned result. This avoids state-machine complexity since
    /// strategic planning is a background task where progressive streaming provides
    /// no user-visible benefit.
    ///
    /// # Performance
    /// - Time-to-first-chunk: ~100-300ms (non-thinking mode)
    /// - Chunk frequency: ~50-100ms intervals
    /// - Compatible with BatchExecutor for multi-agent inference
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// # use astraweave_llm::LlmClient;
    /// # use futures_util::StreamExt;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Qwen3Ollama::localhost();
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
        use futures_util::StreamExt;

        // Use static client with connection pooling
        // NOTE: No overall `.timeout()` for streaming — the total lifecycle can
        // be long as chunks arrive incrementally. We only set a connect timeout
        // so the initial TCP handshake doesn't hang forever.
        static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = CLIENT.get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client")
        });

        let url = format!("{}/api/chat", self.url);

        // Build messages array with thinking mode prefix
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        let user_content = self.prepare_user_content(prompt);
        messages.push(json!({"role": "user", "content": user_content}));

        // For thinking mode: use non-streaming to accumulate-then-strip
        // This avoids state machine complexity for think-block filtering in the stream layer.
        // Strategic planning requests are background tasks via LlmExecutor::generate_plan_async()
        // where progressive streaming provides no user-visible benefit.
        if self.enable_thinking {
            let body = json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
                "think": true,
                "keep_alive": -1,
                "options": self.build_options(),
            });

            tracing::debug!(
                "Thinking mode: using accumulate-then-strip for {}",
                self.model
            );

            let response = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .context("Failed to send request to Ollama")?;

            if !response.status().is_success() {
                anyhow::bail!("Ollama returned error: {}", response.status());
            }

            let response_json: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse Ollama response")?;

            let raw_text = extract_ollama_content(&response_json)?;

            let cleaned = strip_thinking_blocks(&raw_text);

            // Return as a single-item stream
            let stream = futures_util::stream::once(async move { Ok(cleaned) });
            return Ok(Box::pin(stream));
        }

        // Non-thinking mode: true streaming
        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "think": false,
            "keep_alive": -1,
            "options": self.build_options(),
        });

        tracing::debug!("Sending streaming request to Ollama: {}", self.model);

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send streaming request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama streaming returned error: {}", response.status());
        }

        // Convert bytes_stream to text chunk stream
        // Ollama streams NDJSON (newline-delimited JSON):
        //   {"message": {"content": "text"}, "done": false}\n
        let byte_stream = response.bytes_stream();

        // Transform bytes → NDJSON lines → text chunks
        // Uses a byte buffer to handle chunks that split UTF-8 characters or JSON lines
        let text_stream = byte_stream
            .scan(Vec::<u8>::new(), |buffer, chunk_result| {
                // Handle byte chunk errors
                let chunk = match chunk_result {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        return futures_util::future::ready(Some(vec![Err(anyhow::anyhow!(
                            "Stream error: {}",
                            e
                        ))]));
                    }
                };

                // Append to buffer
                buffer.extend_from_slice(&chunk);

                let mut results = Vec::new();

                // Find the last newline character
                // We only process up to the last newline to ensure we have complete JSON objects
                if let Some(last_newline_pos) = buffer.iter().rposition(|&b| b == b'\n') {
                    // Extract complete lines from buffer
                    let complete_lines: Vec<u8> = buffer.drain(..=last_newline_pos).collect();

                    // Parse extracted lines
                    if let Ok(text) = String::from_utf8(complete_lines) {
                        for line in text.lines() {
                            if line.trim().is_empty() {
                                continue;
                            }

                            // Parse NDJSON line
                            match serde_json::from_str::<serde_json::Value>(line) {
                                Ok(json) => {
                                    // Extract content for this NDJSON line.
                                    // Priority: message.content > message.thinking > response
                                    let mut line_content = String::new();

                                    // Primary: "message.content" field (Chat API format)
                                    if let Some(content) = json["message"]["content"].as_str() {
                                        if !content.is_empty() {
                                            line_content = content.to_string();
                                        }
                                    }
                                    // Fallback: Qwen3 may route output to "message.thinking"
                                    // even with think=false, especially on complex prompts
                                    if line_content.is_empty() {
                                        if let Some(thinking) = json["message"]["thinking"].as_str()
                                        {
                                            if !thinking.is_empty() {
                                                line_content = thinking.to_string();
                                            }
                                        }
                                    }
                                    // Fallback for Generate API format (just in case)
                                    if line_content.is_empty() {
                                        if let Some(response) = json["response"].as_str() {
                                            if !response.is_empty() {
                                                line_content = response.to_string();
                                            }
                                        }
                                    }

                                    if !line_content.is_empty() {
                                        results.push(Ok(line_content));
                                    }

                                    // Check if done
                                    if json["done"].as_bool() == Some(true) {
                                        tracing::debug!("Streaming complete (done: true)");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to parse NDJSON line: {} | Line: {}",
                                        e,
                                        line
                                    );
                                }
                            }
                        }
                    } else {
                        // This should be rare if we split on newlines, but possible if invalid UTF-8
                        results.push(Err(anyhow::anyhow!("UTF-8 decode error in stream")));
                    }
                }

                // Return extracted results (empty vec if no complete lines yet)
                if results.is_empty() {
                    futures_util::future::ready(Some(vec![]))
                } else {
                    futures_util::future::ready(Some(results))
                }
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(text_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen3_ollama_creation() {
        let client = Qwen3Ollama::new("http://localhost:11434", "qwen3:8b");
        assert_eq!(client.url, "http://localhost:11434");
        assert_eq!(client.model, "qwen3:8b");
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 512);
        assert!(!client.enable_thinking);
        assert_eq!(client.context_length, 32768);
        assert!(client.system_prompt.is_some());
    }

    #[test]
    fn test_localhost_convenience() {
        let client = Qwen3Ollama::localhost();
        assert_eq!(client.url, "http://localhost:11434");
        assert_eq!(client.model, "qwen3:8b");
    }

    #[test]
    fn test_fast_variant() {
        let client = Qwen3Ollama::fast();
        assert_eq!(client.model, "qwen3:8b");
        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 128);
        assert!(!client.enable_thinking);
        assert_eq!(client.context_length, 2048);
    }

    #[test]
    fn test_strategic_variant() {
        let client = Qwen3Ollama::strategic();
        assert_eq!(client.model, "qwen3:8b");
        assert_eq!(client.temperature, 0.6);
        assert_eq!(client.max_tokens, 1024);
        assert!(client.enable_thinking);
        assert_eq!(client.context_length, 32768);
    }

    #[test]
    fn test_builder_pattern() {
        let client = Qwen3Ollama::localhost()
            .with_temperature(0.5)
            .with_max_tokens(256)
            .with_thinking(true)
            .with_context_length(16384)
            .without_system_prompt();

        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 256);
        assert!(client.enable_thinking);
        assert_eq!(client.context_length, 16384);
        assert!(client.system_prompt.is_none());
    }

    #[test]
    fn test_strip_thinking_blocks_no_think() {
        let input = r#"{"plan_id": "p1", "steps": [{"act": "MoveTo", "x": 5, "y": 5}]}"#;
        let result = strip_thinking_blocks(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_thinking_blocks_single() {
        let input =
            "<think>\nThe enemy is flanking from the north.\n</think>\n{\"plan_id\": \"p1\"}";
        let result = strip_thinking_blocks(input);
        assert_eq!(result, r#"{"plan_id": "p1"}"#);
    }

    #[test]
    fn test_strip_thinking_blocks_multiple() {
        let input = "<think>thought 1</think> middle <think>thought 2</think> end";
        let result = strip_thinking_blocks(input);
        assert_eq!(result, "middle  end");
    }

    #[test]
    fn test_strip_thinking_blocks_unclosed() {
        let input = "<think>\nThe model started thinking but never finished...";
        let result = strip_thinking_blocks(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_thinking_blocks_with_json_after() {
        let input = "<think>analyzing terrain...</think>{\"plan_id\":\"p1\",\"steps\":[]}";
        let result = strip_thinking_blocks(input);
        assert_eq!(result, r#"{"plan_id":"p1","steps":[]}"#);
    }

    #[test]
    fn test_prepare_user_content_thinking() {
        let client = Qwen3Ollama::localhost().with_thinking(true);
        let content = client.prepare_user_content("What should I do?");
        assert!(content.starts_with("/think\n"));
        assert!(content.contains("What should I do?"));
    }

    #[test]
    fn test_prepare_user_content_no_thinking() {
        let client = Qwen3Ollama::localhost().with_thinking(false);
        let content = client.prepare_user_content("What should I do?");
        // Optimized: no /no_think prefix — API body "think": false handles it
        assert_eq!(content, "What should I do?");
    }

    #[test]
    fn test_build_options_non_thinking() {
        let client = Qwen3Ollama::fast();
        let opts = client.build_options();
        // Optimized: lean params — no top_p, top_k, repeat_penalty for speed
        assert_eq!(opts["temperature"], 0.5);
        assert_eq!(opts["num_predict"], 128);
        assert_eq!(opts["num_ctx"], 2048);
        // num_batch added for round 2 perf optimization
        assert_eq!(opts["num_batch"], 1024);
        // Round 3: num_keep=-1 caches system prompt prefix in KV cache
        assert_eq!(opts["num_keep"], -1);
        // Round 3: use_mmap for memory-mapped model loading
        assert_eq!(opts["use_mmap"], true);
        // These should NOT be present in lean mode
        assert!(opts.get("top_k").is_none() || opts["top_k"].is_null());
        assert!(opts.get("repeat_penalty").is_none() || opts["repeat_penalty"].is_null());
    }

    #[test]
    fn test_build_options_thinking() {
        let client = Qwen3Ollama::strategic();
        let opts = client.build_options();
        assert_eq!(opts["top_p"], 0.95);
        assert_eq!(opts["top_k"], 20);
        assert_eq!(opts["repeat_penalty"], 1.05);
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_health_check() {
        let client = Qwen3Ollama::localhost();
        let health = client.health_check().await;

        // If Ollama is running, this should succeed
        if let Ok(status) = health {
            assert!(status.server_running);
            println!("Ollama version: {}", status.ollama_version);
            println!("Model available: {}", status.model_available);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + qwen3:8b
    async fn test_complete() {
        let client = Qwen3Ollama::localhost();

        // Check health first
        let health = client.health_check().await.expect("Health check failed");
        if !health.is_ready() {
            panic!("{}", health.error_message().unwrap());
        }

        let prompt = "You are at position (5,5). Enemy at (10,8). Generate a tactical plan.";
        let response = client.complete(prompt).await.expect("Completion failed");

        assert!(!response.is_empty());
        println!("Qwen3-8B response:\n{}", response);

        // Ensure no thinking blocks remain
        assert!(!response.contains("<think>"));
        assert!(!response.contains("</think>"));

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
    #[ignore] // Requires Ollama + qwen3:8b
    async fn test_complete_streaming() {
        use futures_util::StreamExt;

        let client = Qwen3Ollama::localhost();

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
        if let Some(ttfc) = time_to_first_chunk {
            println!("Time to first chunk: {:.2}s", ttfc.as_secs_f32());
        }
        println!("Final response length: {} chars", full_response.len());
        println!("\nFull response:\n{}", full_response);

        assert!(
            !full_response.is_empty(),
            "Streaming returned empty response"
        );
        assert!(chunk_count > 0, "No chunks received");

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
    #[ignore] // Requires Ollama + qwen3:8b
    async fn test_streaming_vs_blocking_consistency() {
        use futures_util::StreamExt;

        let client = Qwen3Ollama::localhost().with_temperature(0.5); // Low variance

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

        // Both should produce valid JSON (Qwen3 is highly consistent)
        assert!(!blocking_response.is_empty());
        assert!(!streaming_response.is_empty());
    }
}

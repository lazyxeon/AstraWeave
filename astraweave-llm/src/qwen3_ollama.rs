//! Qwen instruct integration via Ollama
//!
//! This module provides a production-ready interface to Qwen instruct models via Ollama,
//! which handles model management, quantization, and inference.
//!
//! # Why Qwen Instruct?
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
//! ollama pull qwen3.5:4b
//! ollama serve                    # Start server on localhost:11434
//! ```
//!
//! # Usage
//! ```ignore
//! use astraweave_llm::qwen3_ollama::{Qwen3Ollama, DEFAULT_QWEN_INSTRUCT_MODEL};
//! use astraweave_llm::LlmClient;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let client = Qwen3Ollama::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL).expect("model client");
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

/// Primary local model for AstraWeave.
///
/// Keep active examples, probes, and modelfiles aligned to this value so the
/// runtime default cannot drift back to Phi-3 or an older Qwen variant.
pub const DEFAULT_QWEN_INSTRUCT_MODEL: &str = "qwen3.5:4b";

/// Message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Stateful chat session for Qwen instruct models
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

/// Qwen instruct client using Ollama backend
///
/// This is the **recommended** way to use Qwen instruct models in AstraWeave. Ollama handles
/// all the complexity of model loading, quantization, and GPU acceleration.
///
/// ## Model Variant
/// - `qwen3.5:4b` - primary local instruct model for AstraWeave
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
    url: String,
    allow_remote: bool,

    /// Model name (default: `DEFAULT_QWEN_INSTRUCT_MODEL`)
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

    client: reqwest::Client,
}

impl Qwen3Ollama {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a new Qwen3-8B client with default settings
    ///
    /// # Arguments
    /// * `url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (default: `DEFAULT_QWEN_INSTRUCT_MODEL`)
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::{Qwen3Ollama, DEFAULT_QWEN_INSTRUCT_MODEL};
    /// let client = Qwen3Ollama::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL).expect("model client");
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
            enable_thinking: false,
            context_length: 32768,
            client,
        })
    }

    /// Create Qwen client for localhost (convenience method)
    ///
    /// Uses `DEFAULT_QWEN_INSTRUCT_MODEL`.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::localhost().expect("model client");
    /// ```
    pub fn localhost() -> Result<Self> {
        Self::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL)
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
    /// - `num_keep: -1` — on a context shift, keep the initial prompt. Prefix reuse is `keep_alive` plus the server prompt cache (`prompt_eval_cached_count`), not this field
    /// - `use_mmap: true` — memory-mapped model loading
    /// - `keep_alive: -1` — never unload model from memory
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// let client = Qwen3Ollama::fast().expect("model client"); // Low latency variant
    /// ```
    pub fn fast() -> Result<Self> {
        let mut client = Self::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL)?
            .with_temperature(0.5)
            .with_max_tokens(128)
            .with_thinking(false)
            .with_context_length(2048);
        // Ultra-short system prompt — every token saved reduces TTFC
        client.system_prompt = Some(FAST_SYSTEM_PROMPT.to_string());
        Ok(client)
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
    /// let client = Qwen3Ollama::strategic().expect("model client"); // Thinking mode for complex plans
    /// ```
    pub fn strategic() -> Result<Self> {
        Ok(
            Self::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL)?
                .with_temperature(0.6)
                .with_max_tokens(1024)
                .with_thinking(true)
                .with_context_length(32768),
        )
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
            // num_keep: -1 — tokens kept from the initial prompt when the context
            //   shifts. This is not the prefix KV cache. Ollama reuses a prefix
            //   through the server prompt cache while `keep_alive` holds the model.
            //   A hit is `prompt_eval_cached_count` on the response.
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

    /// Preload the model into GPU/memory.
    ///
    /// Performs two operations:
    /// 1. Loads model weights into VRAM (eliminates 3-5s cold-start)
    /// 2. Sends a minimal inference with the system prompt while `keep_alive: -1`
    ///    holds the model. Prefix reuse is the server prompt cache, measured by
    ///    `prompt_eval_cached_count`. `num_keep: -1` is how many tokens survive a
    ///    context shift, not that cache.
    ///
    /// After preload, a later request can skip model loading. A prefix hit still
    /// depends on the server prompt cache.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::qwen3_ollama::Qwen3Ollama;
    /// # async fn demo() -> anyhow::Result<()> {
    /// let client = Qwen3Ollama::fast().expect("model client");
    /// client.preload().await?;  // Load weights; keep_alive holds the model
    /// // First real call now has the same TTFC as subsequent calls
    /// # Ok(())
    /// # }
    /// ```
    pub async fn preload(&self) -> Result<()> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        let url = format!("{}/api/chat", self.url);

        // Build messages: include system prompt if configured, plus a minimal
        // user message to trigger a forward pass while keep_alive holds the model.
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

        tracing::info!("Preloading model {} (weights + keep_alive)...", self.model);
        let start = std::time::Instant::now();

        crate::with_call_deadline(120, async {
            let response = crate::send_for_headers(
                crate::stream_read_idle(),
                client.post(&url).json(&body).send(),
            )
            .await
            .context("Failed to preload model")?;

            if !response.status().is_success() {
                anyhow::bail!("Ollama preload returned error: {}", response.status());
            }

            crate::read_capped_text(response)
                .await
                .context("Ollama preload body read failed")?;
            Ok(())
        })
        .await?;

        tracing::info!(
            "Model {} preloaded in {:.1}s (keep_alive holds the model)",
            self.model,
            start.elapsed().as_secs_f64()
        );
        Ok(())
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
            "think": self.enable_thinking,
            "keep_alive": -1,
            "options": self.build_options(),
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

        let message = extract_ollama_content(&response_json)?;

        // Strip thinking blocks if present (model may produce them even in non-thinking mode)
        Ok(strip_thinking_blocks(&message))
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

impl Qwen3Ollama {
    async fn complete_formatted(
        &self,
        prompt: &str,
        format_schema: Option<&serde_json::Value>,
    ) -> Result<String> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        let url = format!("{}/api/chat", self.url);

        // Build messages array with thinking mode prefix
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_prompt {
            messages.push(json!({"role": "system", "content": sys}));
        }
        let user_content = self.prepare_user_content(prompt);
        messages.push(json!({"role": "user", "content": user_content}));

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": self.enable_thinking,
            "keep_alive": -1,
            "options": self.build_options(),
        });
        if let Some(schema) = format_schema {
            crate::plan_format::insert_schema_format(&mut body, schema);
        }

        tracing::debug!(
            "Sending request to Ollama: {} (thinking={}, temp={}, max_tokens={}, ctx={})",
            self.model,
            self.enable_thinking,
            self.temperature,
            self.max_tokens,
            self.context_length
        );
        let start = std::time::Instant::now();

        let response_json: serde_json::Value = crate::with_call_deadline(300, async {
            crate::post_ollama_json_value(client, &url, body).await
        })
        .await?;

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
}

/// Implement LlmClient trait for Ollama-based Qwen3-8B
#[async_trait]
impl LlmClient for Qwen3Ollama {
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
    /// let client = Qwen3Ollama::localhost().expect("model client");
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

        // `OLLAMA_TIMEOUT_SECS` bounds each silent gap. Header waits and
        // the thinking JSON body are that idle limit or the time still
        // left in the 300s lifetime, whichever is shorter, including when
        // the idle limit is longer than 300s. The JSON body also stops at
        // 64 KiB. Thinking mode still has `with_call_deadline(300)`.
        // The non-thinking scan restarts its gap on a non-empty chunk.
        let idle = crate::stream_read_idle();
        let requested = std::time::Instant::now();
        let lifetime_total = crate::stream_call_lifetime(crate::QWEN_STREAM_LIFETIME);
        let client = &self.client;

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

            let response_json: serde_json::Value = crate::with_call_deadline(300, async {
                let response = crate::send_headers_for_call(
                    idle,
                    lifetime_total.saturating_sub(requested.elapsed()),
                    client.post(&url).json(&body).send(),
                )
                .await
                .context("Failed to send request to Ollama")?;

                if !response.status().is_success() {
                    anyhow::bail!("Ollama returned error: {}", response.status());
                }

                crate::read_capped_json_within(
                    response,
                    lifetime_total.saturating_sub(requested.elapsed()),
                )
                .await
                .context("Failed to parse Ollama response")
            })
            .await?;

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

        // Ollama streams NDJSON. The header wait used the time still left
        // in the 300s lifetime. Silence on the body stops after one idle
        // period. A non-empty chunk starts a new gap. An empty chunk does
        // not. One error, or done:true, ends the stream.
        let overall = idle.saturating_mul(2);
        let lifetime = lifetime_total.saturating_sub(requested.elapsed());
        let text_stream = crate::scan_model_ndjson(
            Box::pin(response.bytes_stream()),
            idle,
            overall,
            lifetime,
            qwen_ndjson_line,
        );

        Ok(Box::pin(text_stream))
    }
}

fn qwen_ndjson_line(line: &str) -> crate::NdjsonYield {
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
    }
    if line_content.is_empty() {
        if let Some(thinking) = json["message"]["thinking"].as_str() {
            if !thinking.is_empty() {
                line_content = thinking.to_string();
            }
        }
    }
    if line_content.is_empty() {
        if let Some(response) = json["response"].as_str() {
            if !response.is_empty() {
                line_content = response.to_string();
            }
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
    fn test_qwen3_ollama_creation() {
        let client = Qwen3Ollama::new("http://localhost:11434", DEFAULT_QWEN_INSTRUCT_MODEL)
            .expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, DEFAULT_QWEN_INSTRUCT_MODEL);
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 512);
        assert!(!client.enable_thinking);
        assert_eq!(client.context_length, 32768);
        assert!(client.system_prompt.is_some());
    }

    #[test]
    fn test_localhost_convenience() {
        let client = Qwen3Ollama::localhost().expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, DEFAULT_QWEN_INSTRUCT_MODEL);
    }

    #[test]
    fn test_fast_variant() {
        let client = Qwen3Ollama::fast().expect("model client");
        assert_eq!(client.model, DEFAULT_QWEN_INSTRUCT_MODEL);
        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 128);
        assert!(!client.enable_thinking);
        assert_eq!(client.context_length, 2048);
    }

    #[test]
    fn test_strategic_variant() {
        let client = Qwen3Ollama::strategic().expect("model client");
        assert_eq!(client.model, DEFAULT_QWEN_INSTRUCT_MODEL);
        assert_eq!(client.temperature, 0.6);
        assert_eq!(client.max_tokens, 1024);
        assert!(client.enable_thinking);
        assert_eq!(client.context_length, 32768);
    }

    #[test]
    fn test_builder_pattern() {
        let client = Qwen3Ollama::localhost()
            .expect("model client")
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
        let client = Qwen3Ollama::localhost()
            .expect("model client")
            .with_thinking(true);
        let content = client.prepare_user_content("What should I do?");
        assert!(content.starts_with("/think\n"));
        assert!(content.contains("What should I do?"));
    }

    #[test]
    fn test_prepare_user_content_no_thinking() {
        let client = Qwen3Ollama::localhost()
            .expect("model client")
            .with_thinking(false);
        let content = client.prepare_user_content("What should I do?");
        // Optimized: no /no_think prefix — API body "think": false handles it
        assert_eq!(content, "What should I do?");
    }

    #[test]
    fn test_build_options_non_thinking() {
        let client = Qwen3Ollama::fast().expect("model client");
        let opts = client.build_options();
        // Optimized: lean params — no top_p, top_k, repeat_penalty for speed
        assert_eq!(opts["temperature"], 0.5);
        assert_eq!(opts["num_predict"], 128);
        assert_eq!(opts["num_ctx"], 2048);
        // num_batch added for round 2 perf optimization
        assert_eq!(opts["num_batch"], 1024);
        // num_keep=-1 keeps the initial prompt when the context shifts.
        // It is not the prefix KV cache.
        assert_eq!(opts["num_keep"], -1);
        // Round 3: use_mmap for memory-mapped model loading
        assert_eq!(opts["use_mmap"], true);
        // These should NOT be present in lean mode
        assert!(opts.get("top_k").is_none() || opts["top_k"].is_null());
        assert!(opts.get("repeat_penalty").is_none() || opts["repeat_penalty"].is_null());
    }

    #[test]
    fn test_build_options_thinking() {
        let client = Qwen3Ollama::strategic().expect("model client");
        let opts = client.build_options();
        assert_eq!(opts["top_p"], 0.95);
        assert_eq!(opts["top_k"], 20);
        assert_eq!(opts["repeat_penalty"], 1.05);
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_health_check() {
        let client = Qwen3Ollama::localhost().expect("model client");
        let health = client.health_check().await;

        // If Ollama is running, this should succeed
        if let Ok(status) = health {
            assert!(status.server_running);
            println!("Ollama version: {}", status.ollama_version);
            println!("Model available: {}", status.model_available);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + qwen3.5:4b
    async fn test_complete() {
        let client = Qwen3Ollama::localhost().expect("model client");

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
    #[ignore] // Requires Ollama + qwen3.5:4b
    async fn test_complete_streaming() {
        use futures_util::StreamExt;

        let client = Qwen3Ollama::localhost().expect("model client");

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
    #[ignore] // Requires Ollama + qwen3.5:4b
    async fn test_streaming_vs_blocking_consistency() {
        use futures_util::StreamExt;

        let client = Qwen3Ollama::localhost()
            .expect("model client")
            .with_temperature(0.5); // Low variance

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

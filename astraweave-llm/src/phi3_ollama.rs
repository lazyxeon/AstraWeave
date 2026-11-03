//! Phi-3 Integration via Ollama
//!
//! This module provides a production-ready interface to Microsoft's Phi-3 Medium model
//! via Ollama, which handles model management, quantization, and inference.
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
//! ollama pull phi3:medium    # Downloads Phi-3 Medium Q4 (~7GB)
//! ollama serve               # Start server on localhost:11434
//! ```
//!
//! # Usage
//! ```no_run
//! use astraweave_llm::phi3_ollama::Phi3Ollama;
//! use astraweave_llm::LlmClient;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let client = Phi3Ollama::new("http://localhost:11434", "phi3:medium").expect("model client");
//! let response = client.complete("You are a game AI. Plan your next action.").await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::LlmClient;

/// Phi-3 client using Ollama backend
///
/// This is the **recommended** way to use Phi-3 in AstraWeave. Ollama handles
/// all the complexity of model loading, quantization, and GPU acceleration.
///
/// ## Model Variants
/// - `phi3:mini` - 3.8B parameters, ~2.3GB
/// - `phi3:medium` - 14B parameters, ~7.9GB (Q4) - **RECOMMENDED**
/// - `phi3:large` - 42B parameters, ~24GB
///
/// ## Performance (RTX 3060, 12GB VRAM)
/// - Load time: 2-3 seconds (first request)
/// - Inference: 30-40 tokens/sec @ Q4
/// - Memory: ~8GB VRAM for phi3:medium
#[derive(Debug, Clone)]
pub struct Phi3Ollama {
    /// Ollama API endpoint (default: http://localhost:11434)
    url: String,
    allow_remote: bool,

    /// Model name (e.g., "phi3:medium")
    pub model: String,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,

    /// Maximum tokens to generate
    pub max_tokens: usize,

    /// System prompt for game AI context
    pub system_prompt: Option<String>,

    client: reqwest::Client,
}

impl Phi3Ollama {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a new Phi-3 client with default settings
    ///
    /// # Arguments
    /// * `url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "phi3:medium")
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::phi3_ollama::Phi3Ollama;
    /// let client = Phi3Ollama::new("http://localhost:11434", "phi3:medium").expect("model client");
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

    /// Create Phi-3 client for localhost (convenience method)
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::phi3_ollama::Phi3Ollama;
    /// let client = Phi3Ollama::localhost().expect("model client"); // Uses phi3:medium
    /// ```
    pub fn localhost() -> Result<Self> {
        Self::new("http://localhost:11434", "phi3:medium")
    }

    /// Create optimized Phi-3 client for low-latency game AI
    ///
    /// Uses phi3:game model (mini variant optimized for 6GB VRAM).
    /// Expected latency: 500ms-2s on GTX 1660 Ti / RTX 3060.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::phi3_ollama::Phi3Ollama;
    /// let client = Phi3Ollama::fast().expect("model client"); // Low latency variant
    /// ```
    pub fn fast() -> Result<Self> {
        Ok(Self::new("http://localhost:11434", "phi3:game")?
            .with_temperature(0.5)
            .with_max_tokens(128))
    }

    /// Create ultra-fast Phi-3 client using the mini model
    ///
    /// Uses phi3:3.8b (mini) for fastest inference. Trade-off: slightly lower quality.
    /// Expected latency: 200-800ms (3-5x faster than medium).
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::phi3_ollama::Phi3Ollama;
    /// let client = Phi3Ollama::mini().expect("model client"); // Fastest variant
    /// ```
    pub fn mini() -> Result<Self> {
        Ok(Self::new("http://localhost:11434", "phi3:3.8b")?
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

impl Phi3Ollama {
    async fn complete_formatted(
        &self,
        prompt: &str,
        format_schema: Option<&serde_json::Value>,
    ) -> Result<String> {
        crate::ensure_model_request(&self.url, self.allow_remote)?;
        let client = &self.client;

        let url = format!("{}/api/generate", self.url);

        // Build request with system prompt if configured
        let full_prompt = if let Some(ref system) = self.system_prompt {
            format!("{}\n\n{}", system, prompt)
        } else {
            prompt.to_string()
        };

        let mut body = json!({
            "model": self.model,
            "prompt": full_prompt,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
                "num_ctx": 4096,  // Reduced context window for speed
            }
        });
        if let Some(schema) = format_schema {
            crate::plan_format::insert_schema_format(&mut body, schema);
        }

        // ═══ PHASE 7 DEBUG LOGGING ═══
        eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
        eprintln!("║           PROMPT SENT TO PHI-3 (via Ollama)                  ║");
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("Model: {}", self.model);
        eprintln!("Temperature: {}", self.temperature);
        eprintln!("Max Tokens: {}", self.max_tokens);
        eprintln!("Prompt Length: {} chars", full_prompt.len());
        eprintln!("╠═══════════════════════════════════════════════════════════════╣");
        eprintln!("{}", full_prompt);
        eprintln!("╚═══════════════════════════════════════════════════════════════╝\n");

        tracing::debug!("Sending request to Ollama: {}", self.model);
        let start = std::time::Instant::now();

        let response_json: serde_json::Value = crate::with_call_deadline(120, async {
            crate::post_ollama_json_value(client, &url, body).await
        })
        .await?;

        let text = response_json["response"]
            .as_str()
            .context("Missing 'response' field in Ollama output")?
            .to_string();

        let duration = start.elapsed();

        // ═══ PHASE 7 DEBUG LOGGING ═══
        eprintln!("\n╔═══════════════════════════════════════════════════════════════╗");
        eprintln!("║           PHI-3 RAW RESPONSE (via Ollama)                    ║");
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

/// Implement LlmClient trait for Ollama-based Phi-3
#[async_trait]
impl LlmClient for Phi3Ollama {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi3_ollama_creation() {
        let client =
            Phi3Ollama::new("http://localhost:11434", "phi3:medium").expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, "phi3:medium");
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 512);
        assert!(client.system_prompt.is_some());
    }

    #[test]
    fn test_localhost_convenience() {
        let client = Phi3Ollama::localhost().expect("model client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, "phi3:medium");
    }

    #[test]
    fn test_builder_pattern() {
        let client = Phi3Ollama::localhost()
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
        let client = Phi3Ollama::localhost().expect("model client");
        let health = client.health_check().await;

        // If Ollama is running, this should succeed
        if let Ok(status) = health {
            assert!(status.server_running);
            println!("Ollama version: {}", status.ollama_version);
            println!("Model available: {}", status.model_available);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama + phi3:medium
    async fn test_complete() {
        let client = Phi3Ollama::localhost().expect("model client");

        // Check health first
        let health = client.health_check().await.expect("Health check failed");
        if !health.is_ready() {
            panic!("{}", health.error_message().unwrap());
        }

        let prompt = "You are at position (5,5). Enemy at (10,8). Generate a tactical plan.";
        let response = client.complete(prompt).await.expect("Completion failed");

        assert!(!response.is_empty());
        println!("Phi-3 response:\n{}", response);

        // Try to parse as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
        if let Ok(json) = parsed {
            println!(
                "Parsed JSON: {}",
                serde_json::to_string_pretty(&json).unwrap()
            );
        }
    }
}

//! Background LLM worker for the hello_companion visual demo.
//!
//! Goals:
//! - Reuse a single Tokio runtime instead of creating one per request.
//! - Provide a small in-memory cache for repeated prompts (demo responsiveness).
//! - Keep the visual demo API simple (request -> oneshot receiver).
//! - Support streaming partial output for smoother Arbiter experience.
//! - Apply response modifiers (length, style) based on user constraints.

#![allow(dead_code)] // Some methods/fields reserved for future streaming expansion

use astraweave_llm::qwen3_ollama::Qwen3Ollama;
use astraweave_llm::LlmClient;
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// Response modifiers parsed from user input.
#[derive(Clone, Debug, Default)]
pub struct ResponseModifiers {
    pub brief: bool,
    pub detailed: bool,
    pub no_spoilers: bool,
    pub step_by_step: bool,
}

impl ResponseModifiers {
    /// Parse modifiers from user input.
    pub fn from_input(input: &str) -> Self {
        let lower = input.to_lowercase();
        Self {
            brief: lower.contains("brief")
                || lower.contains("short")
                || lower.contains("quick")
                || lower.contains("concise"),
            detailed: lower.contains("detail")
                || lower.contains("elaborate")
                || lower.contains("thorough")
                || lower.contains("deep"),
            no_spoilers: lower.contains("no spoiler")
                || lower.contains("hint only")
                || lower.contains("don't spoil"),
            step_by_step: lower.contains("step by step")
                || lower.contains("step-by-step")
                || lower.contains("walkthrough"),
        }
    }

    /// Build a system prompt suffix based on modifiers.
    pub fn to_system_suffix(&self) -> String {
        let mut parts = Vec::new();
        if self.brief {
            parts.push("Keep your response SHORT (1-2 sentences max).");
        }
        if self.detailed {
            parts.push("Provide a DETAILED response with examples and reasoning.");
        }
        if self.no_spoilers {
            parts.push("DO NOT give direct solutions or spoilers. Give gentle hints only.");
        }
        if self.step_by_step {
            parts.push("Structure your response as numbered steps.");
        }
        parts.join(" ")
    }
}

pub struct LlmWorker {
    tx: mpsc::Sender<Request>,
    /// Streaming channel for partial tokens (Arbiter mode shows these).
    streaming_rx: Arc<Mutex<Option<std::sync::mpsc::Receiver<String>>>>,
}

struct Request {
    prompt: String,
    modifiers: ResponseModifiers,
    reply_tx: tokio::sync::oneshot::Sender<String>,
    stream_tx: Option<std::sync::mpsc::Sender<String>>,
}

struct PromptCache {
    map: HashMap<String, String>,
    order: VecDeque<String>,
    capacity: usize,
}

impl PromptCache {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        self.map.get(key).cloned()
    }

    fn put(&mut self, key: String, value: String) {
        if self.map.contains_key(&key) {
            // Keep existing insertion order stable; update value.
            self.map.insert(key, value);
            return;
        }

        self.map.insert(key.clone(), value);
        self.order.push_back(key);

        while self.map.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

impl LlmWorker {
    pub fn new(client: Qwen3Ollama) -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        let streaming_rx = Arc::new(Mutex::new(None));

        let cache = Arc::new(Mutex::new(PromptCache::new(96)));
        let cache_thread = cache.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                // Process requests sequentially; this keeps behavior stable.
                while let Ok(req) = rx.recv() {
                    // Build effective prompt with modifiers.
                    let suffix = req.modifiers.to_system_suffix();
                    let effective_prompt = if suffix.is_empty() {
                        req.prompt.clone()
                    } else {
                        format!("{}\n\n[Style: {}]", req.prompt, suffix)
                    };

                    // Cache hit: respond instantly (no streaming for cached).
                    if let Ok(mut cache) = cache_thread.lock() {
                        if let Some(cached) = cache.get(&effective_prompt) {
                            // Send final result.
                            let _ = req.reply_tx.send(cached.clone());
                            // Also send to stream channel if present.
                            if let Some(ref stx) = req.stream_tx {
                                let _ = stx.send(cached);
                            }
                            continue;
                        }
                    }

                    // Cache miss: call LLM with a timeout.
                    let response = tokio::time::timeout(
                        Duration::from_secs(18),
                        client.complete(&effective_prompt),
                    )
                    .await;

                    match response {
                        Ok(Ok(text)) => {
                            // Simulate streaming by sending word chunks.
                            if let Some(ref stx) = req.stream_tx {
                                let words: Vec<&str> = text.split_whitespace().collect();
                                let mut partial = String::new();
                                for (i, word) in words.iter().enumerate() {
                                    if i > 0 {
                                        partial.push(' ');
                                    }
                                    partial.push_str(word);
                                    // Send partial every ~3 words for natural pacing.
                                    if (i + 1) % 3 == 0 || i == words.len() - 1 {
                                        let _ = stx.send(partial.clone());
                                    }
                                }
                            }

                            if let Ok(mut cache) = cache_thread.lock() {
                                cache.put(effective_prompt, text.clone());
                            }
                            let _ = req.reply_tx.send(text);
                        }
                        // On timeout or error, drop the sender so the receiver closes.
                        _ => {
                            // No send: visual demo will fall back in-character.
                        }
                    }
                }
            });
        });

        Self { tx, streaming_rx }
    }

    /// Simple request (no modifiers, no streaming).
    pub fn request(&self, prompt: String) -> tokio::sync::oneshot::Receiver<String> {
        self.request_with_options(prompt, ResponseModifiers::default(), false)
            .0
    }

    /// Request with modifiers parsed from user input.
    pub fn request_with_modifiers(
        &self,
        prompt: String,
        input: &str,
    ) -> tokio::sync::oneshot::Receiver<String> {
        let modifiers = ResponseModifiers::from_input(input);
        self.request_with_options(prompt, modifiers, false).0
    }

    /// Request with streaming support (returns oneshot + mpsc receiver for partial tokens).
    pub fn request_streaming(
        &self,
        prompt: String,
        input: &str,
    ) -> (
        tokio::sync::oneshot::Receiver<String>,
        std::sync::mpsc::Receiver<String>,
    ) {
        let modifiers = ResponseModifiers::from_input(input);
        self.request_with_options(prompt, modifiers, true)
    }

    fn request_with_options(
        &self,
        prompt: String,
        modifiers: ResponseModifiers,
        streaming: bool,
    ) -> (
        tokio::sync::oneshot::Receiver<String>,
        std::sync::mpsc::Receiver<String>,
    ) {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let (stream_tx, stream_rx) = std::sync::mpsc::channel();

        let stream_tx_opt = if streaming { Some(stream_tx) } else { None };

        let _ = self.tx.send(Request {
            prompt,
            modifiers,
            reply_tx,
            stream_tx: stream_tx_opt,
        });

        (reply_rx, stream_rx)
    }
}

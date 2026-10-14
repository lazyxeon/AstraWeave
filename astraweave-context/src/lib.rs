#![forbid(unsafe_code)]
/*!
# AstraWeave Context Management

Advanced context management and conversation history for AI-native gaming. This crate provides:

- **Conversation History**: Efficient storage and retrieval of conversation turns
- **Context Windows**: Token-aware context management with sliding windows
- **Summarization**: LLM-based conversation summarization for long-term memory
- **Token Management**: Precise token counting and budget management
- **Multi-Agent Support**: Context isolation and sharing between different AI agents

## Quick Start

```rust
use astraweave_context::{ConversationHistory, ContextConfig, Role};
use astraweave_llm::MockLlm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create conversation history with token limit
    let config = ContextConfig {
        max_tokens: 4096,
        sliding_window_size: 10,
        ..Default::default()
    };

    let mut history = ConversationHistory::new(config);

    // Add messages
    history.add_message(Role::User, "Hello, how are you?".to_string()).await?;
    history.add_message(Role::Assistant, "I'm doing well, thank you!".to_string()).await?;

    // Get context for LLM prompt
    let context = history.get_context(2048).await?;
    println!("Context: {}", context);

    Ok(())
}
```
*/

pub mod history;
pub mod summarizer;
pub mod token_counter;
pub mod window;

pub use history::*;
pub use summarizer::*;
pub use token_counter::*;
pub use window::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for context management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum tokens allowed in context window
    pub max_tokens: usize,

    /// Number of recent messages to keep in sliding window
    pub sliding_window_size: usize,

    /// Strategy for handling context overflow
    pub overflow_strategy: OverflowStrategy,

    /// Whether to enable automatic summarization
    pub enable_summarization: bool,

    /// Minimum messages before triggering summarization
    pub summarization_threshold: usize,

    /// Token encoding model (e.g., "cl100k_base" for GPT-4)
    pub encoding_model: String,

    /// Whether to preserve system messages
    pub preserve_system_messages: bool,

    /// Context sharing settings
    pub sharing_config: SharingConfig,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            sliding_window_size: 20,
            overflow_strategy: OverflowStrategy::SlidingWindow,
            enable_summarization: true,
            summarization_threshold: 50,
            encoding_model: "cl100k_base".to_string(),
            preserve_system_messages: true,
            sharing_config: SharingConfig::default(),
        }
    }
}

/// Strategies for handling context overflow
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OverflowStrategy {
    /// Remove oldest messages (FIFO)
    SlidingWindow,
    /// Summarize old messages
    Summarization,
    /// Hybrid: summarize old, sliding window for recent
    Hybrid,
    /// Truncate from beginning
    TruncateStart,
    /// Truncate from middle (keep start and end)
    TruncateMiddle,
}

/// Context sharing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    /// Whether this context can be shared with other agents
    pub allow_sharing: bool,

    /// Maximum number of agents that can share this context
    pub max_shared_agents: usize,

    /// Whether to isolate sensitive information
    pub isolate_sensitive: bool,

    /// Tags for context categorization
    pub tags: Vec<String>,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            allow_sharing: false,
            max_shared_agents: 3,
            isolate_sensitive: true,
            tags: Vec::new(),
        }
    }
}

/// Role of a message participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Role {
    /// System message (instructions, context)
    System,
    /// User input
    User,
    /// AI assistant response
    Assistant,
    /// Function/tool call
    Function,
    /// Another AI agent
    Agent(u32),
}

impl Role {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Function => "function",
            Role::Agent(_) => "agent",
        }
    }
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,

    /// Role of the message sender
    pub role: Role,

    /// Message content
    pub content: String,

    /// Timestamp when message was created
    pub timestamp: u64,

    /// Token count for this message
    pub token_count: usize,

    /// Message metadata
    pub metadata: HashMap<String, String>,

    /// Whether this message should be preserved during pruning
    pub preserve: bool,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: current_timestamp(),
            token_count: 0, // Will be calculated when added to history
            metadata: HashMap::new(),
            preserve: false,
        }
    }

    /// Message whose id and timestamp are functions of the sim tick.
    ///
    /// `Message::new` draws a UUID and reads the wall clock. The tick must
    /// not call it.
    pub fn recorded(tick: u64, seq: u32, role: Role, content: String) -> Self {
        Self {
            id: format!("m-{tick}-{seq}"),
            role,
            timestamp: tick,
            token_count: content.len(),
            content,
            metadata: HashMap::new(),
            preserve: true,
        }
    }

    /// Create a preserved message (won't be pruned)
    pub fn new_preserved(role: Role, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: current_timestamp(),
            token_count: 0,
            metadata: HashMap::new(),
            preserve: true,
        }
    }

    /// Add metadata to the message
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Format message for LLM prompt
    pub fn format_for_prompt(&self) -> String {
        match self.role {
            Role::System => format!("SYSTEM: {}", self.content),
            Role::User => format!("USER: {}", self.content),
            Role::Assistant => format!("ASSISTANT: {}", self.content),
            Role::Function => format!("FUNCTION: {}", self.content),
            Role::Agent(id) => format!("AGENT_{}: {}", id, self.content),
        }
    }
}

/// Context window metrics and statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextMetrics {
    /// Total messages in history
    pub total_messages: usize,

    /// Current token count
    pub current_tokens: usize,

    /// Maximum tokens allowed
    pub max_tokens: usize,

    /// Token utilization (0.0 to 1.0)
    pub utilization: f32,

    /// Number of times context was pruned
    pub prune_count: u64,

    /// Number of messages summarized
    pub summarized_messages: usize,

    /// Average message length in tokens
    pub avg_message_tokens: f32,

    /// Time spent on context operations (ms)
    pub processing_time_ms: u64,
}

/// Context operation types for monitoring
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContextOperation {
    AddMessage,
    GetContext,
    Prune,
    Summarize,
    TokenCount,
}

/// Get current Unix timestamp in seconds
///
/// Returns 0 if the system clock is before UNIX_EPOCH (extremely rare edge case).
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_config() {
        let config = ContextConfig::default();
        assert_eq!(config.max_tokens, 4096);
        assert!(config.enable_summarization);
    }

    #[test]
    fn test_message_creation() {
        let message = Message::new(Role::User, "Hello world".to_string());
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content, "Hello world");
        assert!(!message.preserve);
    }

    #[test]
    fn test_preserved_message() {
        let message = Message::new_preserved(Role::System, "System prompt".to_string());
        assert!(message.preserve);
    }

    #[test]
    fn test_message_formatting() {
        let user_msg = Message::new(Role::User, "Hello".to_string());
        let assistant_msg = Message::new(Role::Assistant, "Hi there".to_string());
        let agent_msg = Message::new(Role::Agent(1), "Agent response".to_string());

        assert_eq!(user_msg.format_for_prompt(), "USER: Hello");
        assert_eq!(assistant_msg.format_for_prompt(), "ASSISTANT: Hi there");
        assert_eq!(agent_msg.format_for_prompt(), "AGENT_1: Agent response");
    }

    #[test]
    fn test_role_string_conversion() {
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::Function.as_str(), "function");
        assert_eq!(Role::Agent(5).as_str(), "agent");
    }
}

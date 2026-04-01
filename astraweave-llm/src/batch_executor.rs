//! Batch Inference Executor
//!
//! Enables processing multiple agents in a single LLM call to amortize latency
//! across agents. This is critical for scalability in multi-agent scenarios.
//!
//! # Architecture
//!
//! ```text
//! Agent 1 + Snapshot 1 ┐
//! Agent 2 + Snapshot 2 ├─→ Batch Prompt (all agents) ─→ LLM (single call)
//! Agent 3 + Snapshot 3 ┘                                     ↓
//!                                                   JSON Array: [Plan1, Plan2, Plan3]
//!                                                             ↓
//!                                           Distribute: Agent1←Plan1, Agent2←Plan2, Agent3←Plan3
//! ```
//!
//! # Performance
//!
//! - **Single agent**: 1.6-2.1s per plan (compressed prompts)
//! - **Batch of 5**: ~2-3s total = **0.4-0.6s per agent** (4-5× faster)
//! - **Batch of 10**: ~3-4s total = **0.3-0.4s per agent** (5-7× faster)
//!
//! # Determinism
//!
//! - Agents processed in deterministic order (sorted by ID)
//! - Same input order → same plan order
//! - Critical for replay and multiplayer

use anyhow::{bail, Context, Result};
use astraweave_core::{ActionStep, PlanIntent, WorldSnapshot};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::LlmClient;

/// Agent identifier (must be unique within batch)
pub type AgentId = u32;

/// Batch request: multiple agents with their snapshots
#[derive(Debug, Clone)]
pub struct BatchRequest {
    /// Agents to plan for (sorted by ID for determinism)
    pub agents: Vec<(AgentId, WorldSnapshot)>,
}

impl BatchRequest {
    /// Create new batch request
    pub fn new(agents: Vec<(AgentId, WorldSnapshot)>) -> Self {
        let mut sorted_agents = agents;
        // CRITICAL: Sort by ID for determinism
        sorted_agents.sort_by_key(|(id, _)| *id);

        Self {
            agents: sorted_agents,
        }
    }

    /// Add agent to batch
    pub fn add_agent(&mut self, id: AgentId, snapshot: WorldSnapshot) {
        self.agents.push((id, snapshot));
        // Re-sort to maintain determinism
        self.agents.sort_by_key(|(id, _)| *id);
    }

    /// Get batch size
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

/// Batch response: plans mapped to agent IDs
#[derive(Debug, Clone)]
pub struct BatchResponse {
    /// Plans indexed by agent ID
    pub plans: HashMap<AgentId, PlanIntent>,
}

impl BatchResponse {
    /// Create new batch response
    pub fn new() -> Self {
        Self {
            plans: HashMap::new(),
        }
    }

    /// Add plan for agent
    pub fn add_plan(&mut self, id: AgentId, plan: PlanIntent) {
        self.plans.insert(id, plan);
    }

    /// Get plan for agent
    pub fn get_plan(&self, id: AgentId) -> Option<&PlanIntent> {
        self.plans.get(&id)
    }

    /// Get number of plans
    pub fn len(&self) -> usize {
        self.plans.len()
    }

    /// Check if response is empty
    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }
}

impl Default for BatchResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch prompt builder
pub struct BatchPromptBuilder;

impl BatchPromptBuilder {
    /// Build prompt for batch of agents
    ///
    /// # Format
    /// ```text
    /// You are planning for N agents. Generate EXACTLY N plans in JSON array format.
    ///
    /// Agents:
    /// 1. Agent 1: {snapshot}
    /// 2. Agent 2: {snapshot}
    /// ...
    ///
    /// Return JSON:
    /// [
    ///   {"agent_id": 1, "plan_id": "p1", "steps": [...]},
    ///   {"agent_id": 2, "plan_id": "p2", "steps": [...]},
    ///   ...
    /// ]
    /// ```
    pub fn build_batch_prompt(request: &BatchRequest, tool_list: &str) -> String {
        let n = request.agents.len();

        let mut prompt = format!(
            r#"You are planning for {} agents. Generate EXACTLY {} plans in JSON array format.

CRITICAL RULES:
- Return a JSON ARRAY with {} elements
- Each element MUST have "agent_id", "plan_id", "steps"
- agent_id MUST match the agent number (1, 2, 3, ...)
- Use ONLY these tools: {}

"#,
            n, n, n, tool_list
        );

        // Add agent snapshots
        prompt.push_str("Agents:\n");
        for (idx, (agent_id, snapshot)) in request.agents.iter().enumerate() {
            let agent_num = idx + 1; // 1-indexed for LLM

            // Use compact JSON for snapshot
            let snap_json = serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string());

            prompt.push_str(&format!(
                "{}. Agent {} (ID {}): {}\n",
                agent_num, agent_num, agent_id, snap_json
            ));
        }

        // Add output schema
        prompt.push_str(
            r#"
Return ONLY JSON array (no markdown, no commentary):
[
  {"agent_id": 1, "plan_id": "batch-p1", "steps": [{"act": "MoveTo", "x": 10, "y": 5}, ...]},
  {"agent_id": 2, "plan_id": "batch-p2", "steps": [...]},
  ...
]
"#,
        );

        prompt
    }
}

/// Batch response parser
#[derive(Debug, Deserialize, Serialize)]
struct BatchPlanEntry {
    agent_id: u32,
    plan_id: String,
    steps: Vec<ActionStep>, // Directly deserialize to ActionStep
}

pub struct BatchResponseParser;

impl BatchResponseParser {
    /// Parse LLM response into batch response
    pub fn parse_batch_response(json_text: &str, request: &BatchRequest) -> Result<BatchResponse> {
        // Try direct parse as array
        let entries: Vec<BatchPlanEntry> = serde_json::from_str(json_text.trim())
            .context("Failed to parse batch response as JSON array")?;

        // Validate we got correct number of plans
        if entries.len() != request.agents.len() {
            bail!(
                "Batch response has {} plans but request had {} agents",
                entries.len(),
                request.agents.len()
            );
        }

        let mut response = BatchResponse::new();

        // Map plans to agent IDs
        for entry in entries {
            // Find agent by 1-indexed position (LLM uses 1,2,3...)
            let agent_idx = (entry.agent_id as usize).saturating_sub(1);

            if agent_idx >= request.agents.len() {
                bail!(
                    "Invalid agent_id {} in batch response (max: {})",
                    entry.agent_id,
                    request.agents.len()
                );
            }

            let (agent_id, _) = request.agents[agent_idx];

            // Steps are already ActionStep objects from deserialization
            let step_count = entry.steps.len();
            let plan = PlanIntent {
                plan_id: entry.plan_id.clone(),
                steps: entry.steps,
            };

            response.add_plan(agent_id, plan);

            debug!(
                "Parsed plan for agent {} (ID {}): {} steps",
                entry.agent_id, agent_id, step_count
            );
        }

        Ok(response)
    }
}

/// Batch inference executor
pub struct BatchInferenceExecutor {
    /// Maximum batch size (default: 10)
    max_batch_size: usize,

    /// Current batch being accumulated
    current_batch: Option<BatchRequest>,
}

impl BatchInferenceExecutor {
    /// Create new batch executor
    pub fn new() -> Self {
        Self {
            max_batch_size: 10,
            current_batch: None,
        }
    }

    /// Create with custom max batch size
    pub fn with_max_batch_size(max_batch_size: usize) -> Self {
        Self {
            max_batch_size,
            current_batch: None,
        }
    }

    /// Queue agent for batch processing
    pub fn queue_agent(&mut self, id: AgentId, snapshot: WorldSnapshot) {
        match &mut self.current_batch {
            Some(batch) => {
                batch.add_agent(id, snapshot);
            }
            None => {
                let mut batch = BatchRequest::new(Vec::new());
                batch.add_agent(id, snapshot);
                self.current_batch = Some(batch);
            }
        }
    }

    /// Check if batch is ready (reached max size or has agents)
    pub fn is_ready(&self) -> bool {
        self.current_batch
            .as_ref()
            .map(|b| b.size() >= self.max_batch_size)
            .unwrap_or(false)
    }

    /// Get current batch size
    pub fn batch_size(&self) -> usize {
        self.current_batch.as_ref().map(|b| b.size()).unwrap_or(0)
    }

    /// Flush current batch (return and clear)
    pub fn flush_batch(&mut self) -> Option<BatchRequest> {
        self.current_batch.take()
    }

    /// Execute batch inference with streaming support
    ///
    /// # Arguments
    /// * `llm_client` - LLM client (supports streaming for progressive results)
    /// * `tool_list` - Tool vocabulary string (e.g., "MoveTo|Attack|Reload")
    ///
    /// # Returns
    /// BatchResponse with plans mapped to agent IDs
    ///
    /// # Performance
    /// - Uses streaming to reduce time-to-first-plan
    /// - Accumulates chunks until full response received
    /// - Parses complete JSON array at once (deterministic)
    ///
    /// # Example
    /// ```ignore
    /// use astraweave_llm::{BatchInferenceExecutor, LlmClient, MockLlm};
    ///
    /// async fn example() -> anyhow::Result<()> {
    ///     let mut executor = BatchInferenceExecutor::new();
    ///     executor.queue_agent(1, snapshot1);
    ///     executor.queue_agent(2, snapshot2);
    ///
    ///     let llm_client: &dyn LlmClient = &MockLlm;
    ///     let response = executor.execute_batch(llm_client, "MoveTo|Attack").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_batch(
        &mut self,
        llm_client: &dyn LlmClient,
        tool_list: &str,
    ) -> Result<BatchResponse> {
        let batch = self.flush_batch().context("No batch to execute")?;

        // Build batch prompt
        let prompt = BatchPromptBuilder::build_batch_prompt(&batch, tool_list);

        debug!(
            "Executing batch of {} agents (prompt: {} chars)",
            batch.size(),
            prompt.len()
        );

        // Stream LLM response and accumulate chunks
        let mut stream = llm_client
            .complete_streaming(&prompt)
            .await
            .context("Failed to start streaming LLM request")?;

        let mut accumulated = String::new();
        let mut chunk_count = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to receive streaming chunk")?;

            accumulated.push_str(&chunk);
            chunk_count += 1;

            debug!(
                "Received chunk #{}: {} chars (total: {})",
                chunk_count,
                chunk.len(),
                accumulated.len()
            );
        }

        debug!(
            "Streaming complete: {} chunks, {} total chars",
            chunk_count,
            accumulated.len()
        );

        // Parse accumulated response
        let response = BatchResponseParser::parse_batch_response(&accumulated, &batch)
            .context("Failed to parse batch response")?;

        debug!(
            "Batch inference complete: {} plans generated",
            response.len()
        );

        Ok(response)
    }
}

impl Default for BatchInferenceExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState};
    use std::collections::BTreeMap;
    fn create_test_snapshot(agent_x: i32, agent_y: i32) -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2 {
                    x: agent_x,
                    y: agent_y,
                },
                morale: 80.0,
                ammo: 30,
                cooldowns: BTreeMap::new(),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 100,
                cover: CoverType::None,
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("eliminate".to_string()),
        }
    }

    #[test]
    fn test_batch_request_determinism() {
        // Create batch with agents in random order
        let agents = vec![
            (3, create_test_snapshot(3, 3)),
            (1, create_test_snapshot(1, 1)),
            (2, create_test_snapshot(2, 2)),
        ];

        let batch = BatchRequest::new(agents);

        // Verify they're sorted by ID
        assert_eq!(batch.agents[0].0, 1);
        assert_eq!(batch.agents[1].0, 2);
        assert_eq!(batch.agents[2].0, 3);

        // Verify snapshots match
        assert_eq!(batch.agents[0].1.me.pos.x, 1);
        assert_eq!(batch.agents[1].1.me.pos.x, 2);
        assert_eq!(batch.agents[2].1.me.pos.x, 3);
    }

    #[test]
    fn test_batch_request_add_agent() {
        let mut batch = BatchRequest::new(vec![(2, create_test_snapshot(2, 2))]);

        batch.add_agent(1, create_test_snapshot(1, 1));
        batch.add_agent(3, create_test_snapshot(3, 3));

        // Should be sorted: 1, 2, 3
        assert_eq!(batch.agents[0].0, 1);
        assert_eq!(batch.agents[1].0, 2);
        assert_eq!(batch.agents[2].0, 3);
        assert_eq!(batch.size(), 3);
    }

    #[test]
    fn test_batch_response_operations() {
        let mut response = BatchResponse::new();

        let plan1 = PlanIntent {
            plan_id: "p1".to_string(),
            steps: vec![],
        };

        let plan2 = PlanIntent {
            plan_id: "p2".to_string(),
            steps: vec![],
        };

        response.add_plan(1, plan1);
        response.add_plan(2, plan2);

        assert_eq!(response.len(), 2);
        assert!(response.get_plan(1).is_some());
        assert!(response.get_plan(2).is_some());
        assert!(response.get_plan(3).is_none());

        assert_eq!(response.get_plan(1).unwrap().plan_id, "p1");
        assert_eq!(response.get_plan(2).unwrap().plan_id, "p2");
    }

    #[test]
    fn test_batch_prompt_builder() {
        let agents = vec![
            (1, create_test_snapshot(5, 5)),
            (2, create_test_snapshot(7, 7)),
        ];

        let batch = BatchRequest::new(agents);
        let prompt = BatchPromptBuilder::build_batch_prompt(&batch, "MoveTo|Attack|Reload");

        // Verify prompt structure
        assert!(prompt.contains("planning for 2 agents"));
        assert!(prompt.contains("EXACTLY 2 plans"));
        assert!(prompt.contains("MoveTo|Attack|Reload"));
        assert!(prompt.contains("Agent 1 (ID 1)"));
        assert!(prompt.contains("Agent 2 (ID 2)"));
        assert!(prompt.contains("agent_id"));
        assert!(prompt.contains("plan_id"));

        println!("Batch prompt ({} chars):\n{}", prompt.len(), prompt);
    }

    #[test]
    fn test_batch_executor_queuing() {
        let mut executor = BatchInferenceExecutor::new();

        assert_eq!(executor.batch_size(), 0);
        assert!(!executor.is_ready());

        // Queue agents
        for i in 1..=5 {
            executor.queue_agent(i, create_test_snapshot(i as i32, i as i32));
        }

        assert_eq!(executor.batch_size(), 5);
        assert!(!executor.is_ready()); // max_batch_size is 10

        // Queue more to reach threshold
        for i in 6..=10 {
            executor.queue_agent(i, create_test_snapshot(i as i32, i as i32));
        }

        assert_eq!(executor.batch_size(), 10);
        assert!(executor.is_ready());
    }

    #[test]
    fn test_batch_executor_flush() {
        let mut executor = BatchInferenceExecutor::new();

        executor.queue_agent(1, create_test_snapshot(1, 1));
        executor.queue_agent(2, create_test_snapshot(2, 2));

        let batch = executor.flush_batch();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().size(), 2);

        // After flush, batch should be empty
        assert_eq!(executor.batch_size(), 0);
        assert!(executor.flush_batch().is_none());
    }

    #[test]
    fn test_batch_executor_custom_size() {
        let mut executor = BatchInferenceExecutor::with_max_batch_size(3);

        executor.queue_agent(1, create_test_snapshot(1, 1));
        executor.queue_agent(2, create_test_snapshot(2, 2));
        assert!(!executor.is_ready());

        executor.queue_agent(3, create_test_snapshot(3, 3));
        assert!(executor.is_ready()); // Reached max_batch_size=3
    }

    #[test]
    fn test_batch_response_parser_simple() {
        let json = r#"[
            {"agent_id": 1, "plan_id": "p1", "steps": [{"act": "MoveTo", "x": 10, "y": 5}]},
            {"agent_id": 2, "plan_id": "p2", "steps": [{"act": "Attack", "target_id": 1}]}
        ]"#;

        let agents = vec![
            (10, create_test_snapshot(1, 1)), // agent_id=10, LLM uses 1
            (20, create_test_snapshot(2, 2)), // agent_id=20, LLM uses 2
        ];

        let batch = BatchRequest::new(agents);
        let response = BatchResponseParser::parse_batch_response(json, &batch);

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.len(), 2);

        // Plans should be mapped to actual agent IDs (10, 20)
        assert!(response.get_plan(10).is_some());
        assert!(response.get_plan(20).is_some());

        assert_eq!(response.get_plan(10).unwrap().plan_id, "p1");
        assert_eq!(response.get_plan(20).unwrap().plan_id, "p2");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Integration Tests with LlmClient
    // ═══════════════════════════════════════════════════════════════════════

    /// Mock LLM client that returns batch JSON response
    struct MockBatchLlm {
        response: String,
    }

    impl MockBatchLlm {
        fn new(response: String) -> Self {
            Self { response }
        }

        /// Create mock that returns valid batch response for N agents
        fn for_agents(count: usize) -> Self {
            let mut plans = Vec::new();
            for i in 1..=count {
                plans.push(format!(
                    r#"{{"agent_id": {}, "plan_id": "batch-p{}", "steps": [{{"act": "MoveTo", "x": {}, "y": {}}}]}}"#,
                    i, i, i * 10, i * 5
                ));
            }
            let json = format!("[{}]", plans.join(","));
            Self::new(json)
        }
    }

    #[async_trait::async_trait]
    impl crate::LlmClient for MockBatchLlm {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn complete_streaming(
            &self,
            _prompt: &str,
        ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String>> + Send>>>
        {
            // Simulate streaming by chunking response into 3 chunks
            let response = self.response.clone();
            let chunk_size = response.len() / 3;

            let chunks: Vec<String> = if chunk_size > 0 {
                vec![
                    response[..chunk_size].to_string(),
                    response[chunk_size..chunk_size * 2].to_string(),
                    response[chunk_size * 2..].to_string(),
                ]
            } else {
                vec![response]
            };

            Ok(Box::pin(futures_util::stream::iter(
                chunks.into_iter().map(Ok),
            )))
        }
    }

    #[tokio::test]
    async fn test_execute_batch_with_mock_llm() {
        let mut executor = BatchInferenceExecutor::new();

        // Queue 2 agents
        executor.queue_agent(10, create_test_snapshot(1, 1));
        executor.queue_agent(20, create_test_snapshot(2, 2));

        // Create mock LLM that returns 2 plans
        let llm_client = MockBatchLlm::for_agents(2);

        // Execute batch
        let response = executor.execute_batch(&llm_client, "MoveTo|Attack").await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.len(), 2);

        // Verify plans mapped to correct agent IDs
        assert!(response.get_plan(10).is_some());
        assert!(response.get_plan(20).is_some());

        assert_eq!(response.get_plan(10).unwrap().plan_id, "batch-p1");
        assert_eq!(response.get_plan(20).unwrap().plan_id, "batch-p2");
    }

    #[tokio::test]
    async fn test_execute_batch_with_streaming() {
        let mut executor = BatchInferenceExecutor::new();

        // Queue 3 agents to test larger batch
        executor.queue_agent(1, create_test_snapshot(1, 1));
        executor.queue_agent(2, create_test_snapshot(2, 2));
        executor.queue_agent(3, create_test_snapshot(3, 3));

        // Mock returns 3 plans (will be streamed in 3 chunks)
        let llm_client = MockBatchLlm::for_agents(3);

        let response = executor
            .execute_batch(&llm_client, "MoveTo|Attack|Reload")
            .await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.len(), 3);

        // Verify all 3 plans present
        for agent_id in 1..=3 {
            assert!(response.get_plan(agent_id).is_some());
            let plan = response.get_plan(agent_id).unwrap();
            assert_eq!(plan.plan_id, format!("batch-p{}", agent_id));
        }
    }

    #[tokio::test]
    async fn test_execute_batch_deterministic_ordering() {
        let mut executor = BatchInferenceExecutor::new();

        // Queue agents in non-sorted order
        executor.queue_agent(3, create_test_snapshot(3, 3));
        executor.queue_agent(1, create_test_snapshot(1, 1));
        executor.queue_agent(2, create_test_snapshot(2, 2));

        let llm_client = MockBatchLlm::for_agents(3);

        // Execute multiple times - should get same results
        for _ in 0..3 {
            let mut exec = BatchInferenceExecutor::new();
            exec.queue_agent(3, create_test_snapshot(3, 3));
            exec.queue_agent(1, create_test_snapshot(1, 1));
            exec.queue_agent(2, create_test_snapshot(2, 2));

            let response = exec.execute_batch(&llm_client, "MoveTo").await.unwrap();

            // Plans should always be mapped to same agent IDs (sorted)
            assert_eq!(response.len(), 3);
            assert!(response.get_plan(1).is_some());
            assert!(response.get_plan(2).is_some());
            assert!(response.get_plan(3).is_some());
        }
    }

    #[tokio::test]
    async fn test_execute_batch_empty_error() {
        let mut executor = BatchInferenceExecutor::new();
        let llm_client = MockBatchLlm::for_agents(0);

        // Try to execute without queuing agents
        let result = executor.execute_batch(&llm_client, "MoveTo").await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No batch to execute"));
    }

    #[tokio::test]
    async fn test_execute_batch_invalid_response() {
        let mut executor = BatchInferenceExecutor::new();
        executor.queue_agent(1, create_test_snapshot(1, 1));

        // Mock returns invalid JSON
        let llm_client = MockBatchLlm::new("invalid json".to_string());

        let result = executor.execute_batch(&llm_client, "MoveTo").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }
}

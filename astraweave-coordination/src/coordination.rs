use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::agent::{
    Agent, AgentGoal, AgentMessage, CoordinationContext, CoordinationStatus, Task, WorldEvent,
};

/// Central coordinator for managing multiple LLM agents
pub struct AgentCoordinator {
    /// Registered agents
    agents: Arc<RwLock<HashMap<String, Box<dyn Agent>>>>,
    /// Message routing system
    message_router: MessageRouter,
    /// Resource manager
    resource_manager: ResourceManager,
    /// Event dispatcher
    event_dispatcher: EventDispatcher,
    /// Active coordination sessions
    coordination_sessions: Arc<RwLock<HashMap<String, Arc<RwLock<CoordinationContext>>>>>,
    /// Performance metrics
    metrics: Arc<RwLock<CoordinationMetrics>>,
    /// Configuration
    config: CoordinatorConfig,
}

/// Configuration for the agent coordinator
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Maximum number of concurrent tasks per agent
    pub max_tasks_per_agent: usize,
    /// Maximum message queue size per agent
    pub max_message_queue_size: usize,
    /// Default task timeout
    pub default_task_timeout: Duration,
    /// Resource allocation strategy
    pub resource_strategy: ResourceStrategy,
    /// Enable performance monitoring
    pub enable_metrics: bool,
    /// Maximum coordination session duration
    pub max_coordination_duration: Duration,
}

/// Resource allocation strategies
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResourceStrategy {
    FirstCome,   // First agent to request gets resources
    Priority,    // Based on agent priority and task priority
    LoadBalance, // Distribute load evenly
    Adaptive,    // Learn optimal allocation over time
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_agent: 5,
            max_message_queue_size: 100,
            default_task_timeout: Duration::from_secs(30),
            resource_strategy: ResourceStrategy::Priority,
            enable_metrics: true,
            max_coordination_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Message routing system for inter-agent communication
pub struct MessageRouter {
    /// Message delivery channels
    channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<AgentMessage>>>>,
    /// Receivers kept alive to prevent channel closure
    #[allow(dead_code)] // Receivers must be held to keep channels open
    receivers: Arc<RwLock<HashMap<String, mpsc::UnboundedReceiver<AgentMessage>>>>,
    /// Message history for debugging
    message_history: Arc<RwLock<Vec<AgentMessage>>>,
    /// Routing rules
    routing_rules: Arc<RwLock<Vec<RoutingRule>>>,
}

/// Rule for message routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub from_pattern: Option<String>, // Regex pattern for sender
    pub to_pattern: Option<String>,   // Regex pattern for recipient
    pub message_type: Option<String>, // Message type filter
    pub action: RoutingAction,
    pub priority: i32, // Higher priority rules apply first
}

/// Action to take when routing rule matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RoutingAction {
    Allow,
    Block,
    Redirect(String),       // Redirect to different agent
    Broadcast(Vec<String>), // Send to multiple agents
    Transform(String),      // Transform message content (placeholder)
}

/// Resource management for coordinated agents
#[allow(dead_code)] // Fields reserved for future allocation strategy implementation
pub struct ResourceManager {
    /// Current resource allocations
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    /// Resource limits
    limits: ResourceLimits,
    /// Allocation strategy
    strategy: ResourceStrategy,
}

/// Resource allocation for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub agent_id: String,
    pub llm_calls_allocated: u32,
    pub llm_calls_used: u32,
    pub memory_allocated: u32,
    pub memory_used: u32,
    pub allocated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// System-wide resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub total_llm_calls_per_minute: u32,
    pub total_memory_mb: u32,
    pub max_concurrent_llm_calls: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            total_llm_calls_per_minute: 1000,
            total_memory_mb: 8192, // 8GB
            max_concurrent_llm_calls: 50,
        }
    }
}

/// Event dispatching system for world events
#[allow(dead_code)] // Fields reserved for future event dispatch pipeline
pub struct EventDispatcher {
    /// Event subscriptions by agent
    subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>, // agent_id -> event_types
    /// Event history
    event_history: Arc<RwLock<Vec<WorldEvent>>>,
    /// Event processing queue
    event_queue: Arc<RwLock<Vec<WorldEvent>>>,
}

/// Metrics for coordination system performance
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub messages_sent: u64,
    pub messages_delivered: u64,
    pub messages_failed: u64,
    pub tasks_assigned: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub coordination_sessions: u64,
    pub average_task_duration: f64,
    pub resource_utilization: f64,
    pub agent_availability: HashMap<String, f64>,
    pub event_processing_time: f64,
}

impl AgentCoordinator {
    pub fn new(config: CoordinatorConfig) -> Self {
        let message_router = MessageRouter::new();
        let resource_manager =
            ResourceManager::new(ResourceLimits::default(), config.resource_strategy.clone());
        let event_dispatcher = EventDispatcher::new();

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            message_router,
            resource_manager,
            event_dispatcher,
            coordination_sessions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CoordinationMetrics::default())),
            config,
        }
    }

    /// Register a new agent with the coordinator
    pub async fn register_agent(&self, agent: Box<dyn Agent>) -> Result<()> {
        let agent_id = agent.agent_id().to_string();
        let agent_type = agent.agent_type().to_string();

        info!("Registering agent {} (type: {})", agent_id, agent_type);

        // Add to agents map
        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id.clone(), agent);
        }

        // Set up message channel
        self.message_router.setup_agent_channel(&agent_id).await;

        // Initialize resource allocation
        self.resource_manager
            .initialize_allocation(&agent_id)
            .await?;

        // Subscribe to events
        {
            let agents = self.agents.read().await;
            if let Some(agent) = agents.get(&agent_id) {
                let subscriptions = agent.get_event_subscriptions();
                self.event_dispatcher
                    .subscribe_agent(&agent_id, subscriptions)
                    .await;
            }
        }

        Ok(())
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        info!("Unregistering agent {}", agent_id);

        // Remove from coordination sessions
        self.remove_agent_from_sessions(agent_id).await;

        // Clean up message channel
        self.message_router.remove_agent_channel(agent_id).await;

        // Release resources
        self.resource_manager.release_allocation(agent_id).await;

        // Unsubscribe from events
        self.event_dispatcher.unsubscribe_agent(agent_id).await;

        // Remove from agents map
        let mut agents = self.agents.write().await;
        agents.remove(agent_id);

        Ok(())
    }

    /// Send a message between agents
    pub async fn send_message(&self, message: AgentMessage) -> Result<()> {
        debug!("Routing message from {} to {}", message.from, message.to);

        // Check routing rules
        if !self.message_router.should_deliver(&message).await {
            warn!("Message blocked by routing rules: {:?}", message);
            return Ok(());
        }

        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.messages_sent += 1;
        }

        // Deliver message
        match self.message_router.deliver_message(message).await {
            Ok(_) => {
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.messages_delivered += 1;
                }
            }
            Err(e) => {
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.messages_failed += 1;
                }
                return Err(e);
            }
        }

        Ok(())
    }

    /// Assign a task to the most suitable agent
    pub async fn assign_task(&self, task: Task) -> Result<String> {
        debug!("Assigning task: {} ({})", task.description, task.task_type);

        // Find suitable agents
        let suitable_agents = self.find_suitable_agents(&task).await?;

        if suitable_agents.is_empty() {
            return Err(anyhow!(
                "No suitable agents found for task: {}",
                task.task_type
            ));
        }

        // Select best agent based on strategy
        let selected_agent = self.select_best_agent(&suitable_agents, &task).await?;

        // Check resource availability
        if !self
            .resource_manager
            .can_allocate(&selected_agent, &task.resource_requirements)
            .await
        {
            return Err(anyhow!(
                "Insufficient resources for task assignment to {}",
                selected_agent
            ));
        }

        // Allocate resources
        self.resource_manager
            .allocate_resources(&selected_agent, &task.resource_requirements)
            .await?;

        // Assign task to agent
        {
            let agents = self.agents.read().await;
            if let Some(agent) = agents.get(&selected_agent) {
                agent.add_task(task.clone()).await;
            } else {
                return Err(anyhow!("Selected agent {} not found", selected_agent));
            }
        }

        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.tasks_assigned += 1;
        }

        info!("Assigned task {} to agent {}", task.id, selected_agent);
        Ok(selected_agent)
    }

    /// Start a coordination session with multiple agents
    pub async fn start_coordination(
        &self,
        participant_ids: Vec<String>,
        goals: Vec<AgentGoal>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();

        info!(
            "Starting coordination session {} with agents: {:?}",
            session_id, participant_ids
        );

        // Validate all participants exist
        {
            let agents = self.agents.read().await;
            for agent_id in &participant_ids {
                if !agents.contains_key(agent_id) {
                    return Err(anyhow!("Agent {} not found for coordination", agent_id));
                }
            }
        }

        // Create coordination context
        let context = Arc::new(RwLock::new(CoordinationContext {
            session_id: session_id.clone(),
            participants: participant_ids.clone(),
            shared_state: HashMap::new(),
            coordination_goals: goals,
            start_time: Utc::now(),
            last_update: Utc::now(),
            status: CoordinationStatus::Planning,
        }));

        // Register session
        {
            let mut sessions = self.coordination_sessions.write().await;
            sessions.insert(session_id.clone(), context.clone());
        }

        // Add agents to session
        {
            let agents = self.agents.read().await;
            for agent_id in &participant_ids {
                if agents.get(agent_id).is_some() {
                    // Note: This would require agents to implement joining coordination
                    debug!("Added agent {} to coordination session", agent_id);
                }
            }
        }

        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.coordination_sessions += 1;
        }

        Ok(session_id)
    }

    /// End a coordination session
    pub async fn end_coordination(&self, session_id: &str) -> Result<()> {
        info!("Ending coordination session {}", session_id);

        // Remove session
        let session = {
            let mut sessions = self.coordination_sessions.write().await;
            sessions.remove(session_id)
        };

        if let Some(context) = session {
            // Notify participants
            let ctx = context.read().await;
            let participants = ctx.participants.clone();
            drop(ctx);

            {
                let mut ctx = context.write().await;
                ctx.status = CoordinationStatus::Completed;
                ctx.last_update = Utc::now();
            }

            // Remove agents from session
            for agent_id in participants {
                debug!("Removed agent {} from coordination session", agent_id);
            }
        }

        Ok(())
    }

    /// Dispatch a world event to interested agents
    pub async fn dispatch_event(&self, event: WorldEvent) -> Result<()> {
        debug!(
            "Dispatching event: {} ({})",
            event.description, event.event_type
        );

        let start_time = std::time::Instant::now();

        // Get interested agents
        let interested_agents = self.event_dispatcher.get_interested_agents(&event).await;

        // Dispatch to agents sequentially with write lock
        for agent_id in interested_agents {
            let event = event.clone();
            let mut agents = self.agents.write().await;
            if let Some(agent) = agents.get_mut(&agent_id) {
                match agent.handle_world_event(&event).await {
                    Ok(_) => debug!("Agent {} handled event {}", agent_id, event.id),
                    Err(e) => warn!(
                        "Agent {} failed to handle event {}: {}",
                        agent_id, event.id, e
                    ),
                }
            }
        }

        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.event_processing_time = start_time.elapsed().as_millis() as f64;
        }

        // Store event in history
        self.event_dispatcher.store_event(event).await;

        Ok(())
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> CoordinationMetrics {
        if self.config.enable_metrics {
            self.metrics.read().await.clone()
        } else {
            CoordinationMetrics::default()
        }
    }

    /// Find agents suitable for a task
    async fn find_suitable_agents(&self, task: &Task) -> Result<Vec<String>> {
        let agents = self.agents.read().await;
        let mut suitable = Vec::new();

        for (agent_id, agent) in agents.iter() {
            if agent.can_handle_task(task) && agent.is_available() {
                suitable.push(agent_id.clone());
            }
        }

        Ok(suitable)
    }

    /// Select the best agent for a task
    fn select_best_agent<'a>(
        &'a self,
        candidates: &'a [String],
        task: &'a Task,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            if candidates.is_empty() {
                return Err(anyhow!("No candidates provided"));
            }

            match self.config.resource_strategy {
                ResourceStrategy::FirstCome => Ok(candidates[0].clone()),
                ResourceStrategy::Priority => {
                    // Select agent with lowest current load
                    let agents = self.agents.read().await;
                    let mut best_agent = candidates[0].clone();
                    let mut lowest_load = f64::MAX;

                    for agent_id in candidates {
                        if let Some(agent) = agents.get(agent_id) {
                            let usage = agent.get_resource_usage();
                            let load = usage.active_tasks as f64;
                            if load < lowest_load {
                                lowest_load = load;
                                best_agent = agent_id.clone();
                            }
                        }
                    }

                    Ok(best_agent)
                }
                ResourceStrategy::LoadBalance => {
                    // Simple round-robin for now
                    let index = task.id.len() % candidates.len();
                    Ok(candidates[index].clone())
                }
                ResourceStrategy::Adaptive => {
                    // For now, use priority strategy instead of recursion
                    // to avoid infinite recursion issues
                    let agents = self.agents.read().await;
                    let mut best_agent = candidates[0].clone();
                    let mut lowest_load = f64::MAX;

                    for agent_id in candidates {
                        if let Some(agent) = agents.get(agent_id) {
                            let usage = agent.get_resource_usage();
                            let load = usage.active_tasks as f64;
                            if load < lowest_load {
                                lowest_load = load;
                                best_agent = agent_id.clone();
                            }
                        }
                    }

                    Ok(best_agent)
                }
            }
        })
    }

    /// Remove agent from all coordination sessions
    async fn remove_agent_from_sessions(&self, agent_id: &str) {
        let sessions = self.coordination_sessions.read().await;
        for (session_id, context) in sessions.iter() {
            let mut ctx = context.write().await;
            if ctx.participants.contains(&agent_id.to_string()) {
                ctx.participants.retain(|id| id != agent_id);
                ctx.last_update = Utc::now();
                debug!("Removed agent {} from session {}", agent_id, session_id);
            }
        }
    }

    /// Main update loop - call periodically
    pub async fn update(&self) -> Result<()> {
        // Process pending messages
        self.message_router.process_pending_messages().await?;

        // Update resource allocations
        self.resource_manager.update_allocations().await;

        // Process pending events
        self.event_dispatcher.process_pending_events().await;

        // Clean up expired coordination sessions
        self.cleanup_expired_sessions().await;

        // Update metrics
        if self.config.enable_metrics {
            self.update_metrics().await;
        }

        Ok(())
    }

    /// Clean up expired coordination sessions
    async fn cleanup_expired_sessions(&self) {
        let now = Utc::now();
        let mut sessions = self.coordination_sessions.write().await;

        sessions.retain(|session_id, context| {
            let ctx = context.try_read();
            if let Ok(ctx) = ctx {
                let duration = now - ctx.start_time;
                let keep = duration.to_std().unwrap_or(Duration::ZERO)
                    < self.config.max_coordination_duration;
                if !keep {
                    debug!("Cleaning up expired coordination session {}", session_id);
                }
                keep
            } else {
                true // Keep if we can't read (likely in use)
            }
        });
    }

    /// Update performance metrics
    async fn update_metrics(&self) {
        let agents = self.agents.read().await;
        let mut metrics = self.metrics.write().await;

        // Update agent availability
        metrics.agent_availability.clear();
        for (agent_id, agent) in agents.iter() {
            let availability = if agent.is_available() { 1.0 } else { 0.0 };
            metrics
                .agent_availability
                .insert(agent_id.clone(), availability);
        }

        // Calculate resource utilization
        let total_usage: u32 = agents
            .values()
            .map(|agent| agent.get_resource_usage().active_tasks)
            .sum();
        let total_capacity = agents.len() * self.config.max_tasks_per_agent;
        metrics.resource_utilization = if total_capacity > 0 {
            total_usage as f64 / total_capacity as f64
        } else {
            0.0
        };
    }
}

// Implementation for sub-systems

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            receivers: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
            routing_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn setup_agent_channel(&self, agent_id: &str) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut channels = self.channels.write().await;
        channels.insert(agent_id.to_string(), sender);
        let mut receivers = self.receivers.write().await;
        receivers.insert(agent_id.to_string(), receiver);
    }

    async fn remove_agent_channel(&self, agent_id: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(agent_id);
        let mut receivers = self.receivers.write().await;
        receivers.remove(agent_id);
    }

    async fn should_deliver(&self, message: &AgentMessage) -> bool {
        let rules = self.routing_rules.read().await;

        // Apply routing rules in priority order
        for rule in rules.iter() {
            if self.rule_matches(rule, message) {
                return matches!(rule.action, RoutingAction::Allow);
            }
        }

        // Default: allow delivery
        true
    }

    async fn deliver_message(&self, message: AgentMessage) -> Result<()> {
        let channels = self.channels.read().await;

        if let Some(channel) = channels.get(&message.to) {
            channel
                .send(message.clone())
                .map_err(|_| anyhow!("Failed to send message to agent {}", message.to))?;
        } else {
            return Err(anyhow!("Agent {} not found", message.to));
        }

        // Store in history
        let mut history = self.message_history.write().await;
        history.push(message);

        // Keep history manageable
        if history.len() > 1000 {
            history.remove(0);
        }

        Ok(())
    }

    async fn process_pending_messages(&self) -> Result<()> {
        // In a real implementation, this would process queued messages
        // For now, messages are delivered immediately
        Ok(())
    }

    fn rule_matches(&self, rule: &RoutingRule, message: &AgentMessage) -> bool {
        // Simple pattern matching - in practice would use regex
        if let Some(from_pattern) = &rule.from_pattern {
            if !message.from.contains(from_pattern) {
                return false;
            }
        }

        if let Some(to_pattern) = &rule.to_pattern {
            if !message.to.contains(to_pattern) {
                return false;
            }
        }

        if let Some(msg_type) = &rule.message_type {
            if format!("{:?}", message.message_type) != *msg_type {
                return false;
            }
        }

        true
    }
}

impl ResourceManager {
    pub fn new(limits: ResourceLimits, strategy: ResourceStrategy) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            limits,
            strategy,
        }
    }

    async fn initialize_allocation(&self, agent_id: &str) -> Result<()> {
        let allocation = ResourceAllocation {
            agent_id: agent_id.to_string(),
            llm_calls_allocated: self.limits.total_llm_calls_per_minute / 10, // Start with 10% allocation
            llm_calls_used: 0,
            memory_allocated: self.limits.total_memory_mb / 10,
            memory_used: 0,
            allocated_at: Utc::now(),
            expires_at: None,
        };

        let mut allocations = self.allocations.write().await;
        allocations.insert(agent_id.to_string(), allocation);

        Ok(())
    }

    async fn can_allocate(
        &self,
        agent_id: &str,
        requirements: &crate::agent::ResourceRequirements,
    ) -> bool {
        let allocations = self.allocations.read().await;

        if let Some(allocation) = allocations.get(agent_id) {
            allocation.llm_calls_allocated >= allocation.llm_calls_used + requirements.llm_calls
                && allocation.memory_allocated >= allocation.memory_used + requirements.memory_mb
        } else {
            false
        }
    }

    async fn allocate_resources(
        &self,
        agent_id: &str,
        requirements: &crate::agent::ResourceRequirements,
    ) -> Result<()> {
        let mut allocations = self.allocations.write().await;

        if let Some(allocation) = allocations.get_mut(agent_id) {
            allocation.llm_calls_used += requirements.llm_calls;
            allocation.memory_used += requirements.memory_mb;
        } else {
            return Err(anyhow!("No allocation found for agent {}", agent_id));
        }

        Ok(())
    }

    async fn release_allocation(&self, agent_id: &str) {
        let mut allocations = self.allocations.write().await;
        allocations.remove(agent_id);
    }

    async fn update_allocations(&self) {
        // Reset usage counters periodically
        let now = Utc::now();
        let mut allocations = self.allocations.write().await;

        for allocation in allocations.values_mut() {
            if (now - allocation.allocated_at).num_minutes() >= 1 {
                allocation.llm_calls_used = 0;
                allocation.allocated_at = now;
            }
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            event_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn subscribe_agent(&self, agent_id: &str, event_types: Vec<String>) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(agent_id.to_string(), event_types);
    }

    async fn unsubscribe_agent(&self, agent_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(agent_id);
    }

    async fn get_interested_agents(&self, event: &WorldEvent) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        let mut interested = Vec::new();

        for (agent_id, event_types) in subscriptions.iter() {
            if event_types.contains(&event.event_type) || event_types.contains(&"*".to_string()) {
                interested.push(agent_id.clone());
            }
        }

        interested
    }

    async fn store_event(&self, event: WorldEvent) {
        let mut history = self.event_history.write().await;
        history.push(event);

        // Keep history manageable
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    async fn process_pending_events(&self) {
        // In a real implementation, this would process queued events
        // For now, events are dispatched immediately
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{BaseAgent, ResourceUsage, TaskResult};
    use async_trait::async_trait;

    struct TestAgent {
        base: BaseAgent,
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn agent_id(&self) -> &str {
            &self.base.id
        }
        fn agent_type(&self) -> &str {
            &self.base.agent_type
        }
        fn get_state(&self) -> &crate::agent::AgentState {
            &self.base.state
        }
        fn set_state(&mut self, state: crate::agent::AgentState) {
            self.base.state = state;
        }
        async fn handle_message(
            &mut self,
            _message: &AgentMessage,
        ) -> Result<Option<AgentMessage>> {
            Ok(None)
        }
        fn get_goals(&self) -> Vec<AgentGoal> {
            self.base.goals.clone()
        }
        fn set_goals(&mut self, goals: Vec<AgentGoal>) {
            self.base.goals = goals;
        }
        fn get_capabilities(&self) -> Vec<String> {
            self.base.capabilities.clone()
        }
        fn can_handle_task(&self, _task: &Task) -> bool {
            true
        }
        async fn execute_task(&mut self, task: &Task) -> Result<TaskResult> {
            Ok(TaskResult {
                task_id: task.id.clone(),
                success: true,
                result_data: serde_json::json!({}),
                execution_time_ms: 100,
                resources_used: ResourceUsage::default(),
                error_message: None,
                side_effects: Vec::new(),
            })
        }
        fn get_resource_usage(&self) -> ResourceUsage {
            self.base.resource_usage.clone()
        }
        fn get_event_subscriptions(&self) -> Vec<String> {
            self.base.event_subscriptions.clone()
        }
        async fn handle_world_event(&mut self, _event: &WorldEvent) -> Result<()> {
            Ok(())
        }
        async fn add_task(&self, task: Task) {
            self.base.add_task(task).await
        }
        fn is_available(&self) -> bool {
            self.base.is_available()
        }
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let agent = TestAgent {
            base: BaseAgent::new("test_agent".to_string(), vec!["test".to_string()]),
        };
        let agent_id = agent.agent_id().to_string();

        coordinator.register_agent(Box::new(agent)).await.unwrap();

        let agents = coordinator.agents.read().await;
        assert!(agents.contains_key(&agent_id));
    }

    #[tokio::test]
    async fn test_task_assignment() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let agent = TestAgent {
            base: BaseAgent::new("test_agent".to_string(), vec!["test".to_string()]),
        };

        coordinator.register_agent(Box::new(agent)).await.unwrap();

        let task = Task::new("test_task".to_string(), "Test task".to_string());
        let assigned_agent = coordinator.assign_task(task).await.unwrap();

        assert!(!assigned_agent.is_empty());
    }

    // =====================================================================
    // MessageRouter tests
    // =====================================================================

    #[tokio::test]
    async fn message_router_setup_and_remove_channel() {
        let router = MessageRouter::new();
        router.setup_agent_channel("agent-1").await;
        {
            let channels = router.channels.read().await;
            assert!(channels.contains_key("agent-1"));
        }
        router.remove_agent_channel("agent-1").await;
        {
            let channels = router.channels.read().await;
            assert!(!channels.contains_key("agent-1"));
        }
    }

    #[tokio::test]
    async fn message_router_should_deliver_default_allows() {
        let router = MessageRouter::new();
        let msg = AgentMessage {
            id: "m-1".into(),
            from: "a".into(),
            to: "b".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        assert!(router.should_deliver(&msg).await);
    }

    #[tokio::test]
    async fn message_router_should_deliver_block_rule() {
        let router = MessageRouter::new();
        {
            let mut rules = router.routing_rules.write().await;
            rules.push(RoutingRule {
                id: "r-1".into(),
                from_pattern: Some("blocked".into()),
                to_pattern: None,
                message_type: None,
                action: RoutingAction::Block,
                priority: 1,
            });
        }
        let msg = AgentMessage {
            id: "m-1".into(),
            from: "blocked-agent".into(),
            to: "target".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        assert!(!router.should_deliver(&msg).await);
    }

    #[tokio::test]
    async fn message_router_deliver_message_success() {
        let router = MessageRouter::new();
        router.setup_agent_channel("target").await;
        let msg = AgentMessage {
            id: "m-1".into(),
            from: "sender".into(),
            to: "target".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({"hello": true}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        router.deliver_message(msg).await.unwrap();
        let history = router.message_history.read().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "m-1");
    }

    #[tokio::test]
    async fn message_router_deliver_message_unknown_agent_errors() {
        let router = MessageRouter::new();
        let msg = AgentMessage {
            id: "m-1".into(),
            from: "sender".into(),
            to: "nonexistent".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        let result = router.deliver_message(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn message_router_rule_matches_from_pattern() {
        let router = MessageRouter::new();
        let rule = RoutingRule {
            id: "r".into(),
            from_pattern: Some("npc".into()),
            to_pattern: None,
            message_type: None,
            action: RoutingAction::Allow,
            priority: 0,
        };
        let msg_match = AgentMessage {
            id: "m".into(),
            from: "npc-guard".into(),
            to: "player".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        let msg_no_match = AgentMessage {
            id: "m".into(),
            from: "player".into(),
            to: "npc-guard".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        assert!(router.rule_matches(&rule, &msg_match));
        assert!(!router.rule_matches(&rule, &msg_no_match));
    }

    #[tokio::test]
    async fn message_router_history_caps_at_1000() {
        let router = MessageRouter::new();
        router.setup_agent_channel("t").await;
        for i in 0..1005 {
            let msg = AgentMessage {
                id: format!("m-{}", i),
                from: "s".into(),
                to: "t".into(),
                message_type: crate::agent::MessageType::Notification,
                content: serde_json::json!({}),
                timestamp: Utc::now(),
                priority: crate::agent::MessagePriority::Low,
                reply_to: None,
            };
            router.deliver_message(msg).await.unwrap();
        }
        let history = router.message_history.read().await;
        assert_eq!(history.len(), 1000);
    }

    // =====================================================================
    // ResourceManager tests
    // =====================================================================

    #[tokio::test]
    async fn resource_manager_initialize_allocation() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("agent-1").await.unwrap();
        let allocs = rm.allocations.read().await;
        let a = allocs.get("agent-1").unwrap();
        assert_eq!(a.agent_id, "agent-1");
        assert_eq!(a.llm_calls_used, 0);
        assert_eq!(a.memory_used, 0);
        // Gets 10% of total
        assert_eq!(a.llm_calls_allocated, 100); // 1000 / 10
        assert_eq!(a.memory_allocated, 819); // 8192 / 10
    }

    #[tokio::test]
    async fn resource_manager_can_allocate_within_limits() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("a").await.unwrap();
        let req = crate::agent::ResourceRequirements {
            llm_calls: 50,
            memory_mb: 400,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        assert!(rm.can_allocate("a", &req).await);
    }

    #[tokio::test]
    async fn resource_manager_cannot_allocate_exceeding_limits() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("a").await.unwrap();
        let req = crate::agent::ResourceRequirements {
            llm_calls: 200, // exceeds 100 allocated
            memory_mb: 0,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        assert!(!rm.can_allocate("a", &req).await);
    }

    #[tokio::test]
    async fn resource_manager_cannot_allocate_unknown_agent() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        let req = crate::agent::ResourceRequirements::default();
        assert!(!rm.can_allocate("unknown", &req).await);
    }

    #[tokio::test]
    async fn resource_manager_allocate_updates_usage() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("a").await.unwrap();
        let req = crate::agent::ResourceRequirements {
            llm_calls: 10,
            memory_mb: 50,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        rm.allocate_resources("a", &req).await.unwrap();
        let allocs = rm.allocations.read().await;
        let a = allocs.get("a").unwrap();
        assert_eq!(a.llm_calls_used, 10);
        assert_eq!(a.memory_used, 50);
    }

    #[tokio::test]
    async fn resource_manager_allocate_unknown_agent_errors() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        let req = crate::agent::ResourceRequirements::default();
        assert!(rm.allocate_resources("unknown", &req).await.is_err());
    }

    #[tokio::test]
    async fn resource_manager_release_allocation() {
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("a").await.unwrap();
        rm.release_allocation("a").await;
        let allocs = rm.allocations.read().await;
        assert!(!allocs.contains_key("a"));
    }

    // =====================================================================
    // EventDispatcher tests
    // =====================================================================

    #[tokio::test]
    async fn event_dispatcher_subscribe_and_get_interested() {
        let ed = EventDispatcher::new();
        ed.subscribe_agent("a1", vec!["combat".into(), "quest".into()])
            .await;
        ed.subscribe_agent("a2", vec!["quest".into()]).await;
        ed.subscribe_agent("a3", vec!["trade".into()]).await;

        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "quest".into(),
            description: "New quest".into(),
            location: None,
            participants: vec![],
            event_data: serde_json::json!({}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Minor,
        };
        let interested = ed.get_interested_agents(&event).await;
        assert!(interested.contains(&"a1".to_string()));
        assert!(interested.contains(&"a2".to_string()));
        assert!(!interested.contains(&"a3".to_string()));
    }

    #[tokio::test]
    async fn event_dispatcher_wildcard_subscription() {
        let ed = EventDispatcher::new();
        ed.subscribe_agent("broadcast", vec!["*".into()]).await;

        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "anything".into(),
            description: "Random event".into(),
            location: None,
            participants: vec![],
            event_data: serde_json::json!({}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Trivial,
        };
        let interested = ed.get_interested_agents(&event).await;
        assert!(interested.contains(&"broadcast".to_string()));
    }

    #[tokio::test]
    async fn event_dispatcher_unsubscribe() {
        let ed = EventDispatcher::new();
        ed.subscribe_agent("a1", vec!["combat".into()]).await;
        ed.unsubscribe_agent("a1").await;

        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "combat".into(),
            description: "Battle".into(),
            location: None,
            participants: vec![],
            event_data: serde_json::json!({}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Major,
        };
        let interested = ed.get_interested_agents(&event).await;
        assert!(interested.is_empty());
    }

    #[tokio::test]
    async fn event_dispatcher_store_event_history() {
        let ed = EventDispatcher::new();
        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "test".into(),
            description: "Test event".into(),
            location: Some("town".into()),
            participants: vec!["hero".into()],
            event_data: serde_json::json!({"x": 1}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Moderate,
        };
        ed.store_event(event).await;
        let history = ed.event_history.read().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "e-1");
    }

    // =====================================================================
    // AgentCoordinator — coordination sessions
    // =====================================================================

    #[tokio::test]
    async fn coordinator_start_coordination_session() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a2 = TestAgent {
            base: BaseAgent::new("a2".into(), vec![]),
        };
        let a1_id = a1.base.id.clone();
        let a2_id = a2.base.id.clone();
        coordinator.register_agent(Box::new(a1)).await.unwrap();
        coordinator.register_agent(Box::new(a2)).await.unwrap();

        let session_id = coordinator
            .start_coordination(vec![a1_id.clone(), a2_id.clone()], vec![])
            .await
            .unwrap();
        assert_eq!(session_id.len(), 36); // UUID

        let sessions = coordinator.coordination_sessions.read().await;
        assert!(sessions.contains_key(&session_id));
        let ctx = sessions[&session_id].read().await;
        assert_eq!(ctx.participants.len(), 2);
        assert_eq!(ctx.status, CoordinationStatus::Planning);
    }

    #[tokio::test]
    async fn coordinator_start_coordination_unknown_agent_errors() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let result = coordinator
            .start_coordination(vec!["nonexistent".into()], vec![])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn coordinator_end_coordination_session() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.base.id.clone();
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        let session_id = coordinator
            .start_coordination(vec![a1_id], vec![])
            .await
            .unwrap();
        coordinator.end_coordination(&session_id).await.unwrap();

        let sessions = coordinator.coordination_sessions.read().await;
        assert!(!sessions.contains_key(&session_id));
    }

    #[tokio::test]
    async fn coordinator_unregister_agent() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let agent = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let agent_id = agent.base.id.clone();
        coordinator.register_agent(Box::new(agent)).await.unwrap();
        coordinator.unregister_agent(&agent_id).await.unwrap();

        let agents = coordinator.agents.read().await;
        assert!(!agents.contains_key(&agent_id));
    }

    #[tokio::test]
    async fn coordinator_get_metrics_default() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let metrics = coordinator.get_metrics().await;
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.tasks_assigned, 0);
        assert_eq!(metrics.coordination_sessions, 0);
    }

    #[tokio::test]
    async fn coordinator_get_metrics_disabled() {
        let config = CoordinatorConfig {
            enable_metrics: false,
            ..Default::default()
        };
        let coordinator = AgentCoordinator::new(config);
        let metrics = coordinator.get_metrics().await;
        // Returns default when disabled
        assert_eq!(metrics.messages_sent, 0);
    }

    #[tokio::test]
    async fn coordinator_dispatch_event_to_subscribed_agents() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let mut base = BaseAgent::new("a1".into(), vec![]);
        base.event_subscriptions = vec!["combat".into()];
        let a1 = TestAgent { base };
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "combat".into(),
            description: "Battle".into(),
            location: None,
            participants: vec![],
            event_data: serde_json::json!({}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Major,
        };
        coordinator.dispatch_event(event).await.unwrap();

        let metrics = coordinator.get_metrics().await;
        assert!(metrics.event_processing_time >= 0.0);
    }

    #[tokio::test]
    async fn coordinator_assign_task_no_agents_errors() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let task = Task::new("combat".into(), "Fight".into());
        let result = coordinator.assign_task(task).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn coordinator_update_runs_without_error() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        coordinator.update().await.unwrap();
    }

    #[tokio::test]
    async fn coordinator_metrics_track_coordination_sessions() {
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.base.id.clone();
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        coordinator
            .start_coordination(vec![a1_id], vec![])
            .await
            .unwrap();

        let metrics = coordinator.get_metrics().await;
        assert_eq!(metrics.coordination_sessions, 1);
    }

    // =================================================================
    // Mutation-killing tests
    // =================================================================

    #[tokio::test]
    async fn mutation_send_message_success_metrics() {
        // Targets: L254 send_message → Ok(()), L257 delete !, L265 +=, L273 +=
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let agent = TestAgent {
            base: BaseAgent::new("recv".into(), vec![]),
        };
        let agent_id = agent.agent_id().to_string();
        coordinator.register_agent(Box::new(agent)).await.unwrap();

        let msg = AgentMessage {
            id: "m-1".into(),
            from: "sender".into(),
            to: agent_id,
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        coordinator.send_message(msg).await.unwrap();

        let metrics = coordinator.get_metrics().await;
        assert_eq!(metrics.messages_sent, 1, "messages_sent should be 1");
        assert_eq!(
            metrics.messages_delivered, 1,
            "messages_delivered should be 1"
        );
        assert_eq!(metrics.messages_failed, 0);
    }

    #[tokio::test]
    async fn mutation_send_message_blocked_no_metrics() {
        // Targets: L257 delete ! (inverts routing check)
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        {
            let mut rules = coordinator.message_router.routing_rules.write().await;
            rules.push(RoutingRule {
                id: "block-all".into(),
                from_pattern: Some("blocked".into()),
                to_pattern: None,
                message_type: None,
                action: RoutingAction::Block,
                priority: 1,
            });
        }

        let msg = AgentMessage {
            id: "m-1".into(),
            from: "blocked-agent".into(),
            to: "target".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        coordinator.send_message(msg).await.unwrap();

        let metrics = coordinator.get_metrics().await;
        assert_eq!(
            metrics.messages_sent, 0,
            "Blocked message should not increment sent"
        );
        assert_eq!(metrics.messages_delivered, 0);
    }

    #[tokio::test]
    async fn mutation_send_message_failure_metrics() {
        // Targets: L279 += (messages_failed increment)
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        // Don't register any agent — sending to "nonexistent" should fail

        let msg = AgentMessage {
            id: "m-1".into(),
            from: "sender".into(),
            to: "nonexistent".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };

        let result = coordinator.send_message(msg).await;
        assert!(result.is_err(), "Should fail for nonexistent agent");

        let metrics = coordinator.get_metrics().await;
        assert_eq!(
            metrics.messages_sent, 1,
            "messages_sent incremented before delivery"
        );
        assert_eq!(
            metrics.messages_failed, 1,
            "messages_failed should be 1 after failure"
        );
        assert_eq!(metrics.messages_delivered, 0);
    }

    #[tokio::test]
    async fn mutation_assign_task_increments_metrics() {
        // Targets: L335 += → *= in assign_task (tasks_assigned)
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let agent = TestAgent {
            base: BaseAgent::new("a".into(), vec![]),
        };
        coordinator.register_agent(Box::new(agent)).await.unwrap();

        let task = Task::new("t".into(), "d".into());
        coordinator.assign_task(task).await.unwrap();

        let metrics = coordinator.get_metrics().await;
        assert_eq!(
            metrics.tasks_assigned, 1,
            "tasks_assigned should be 1 after one assignment"
        );
    }

    #[tokio::test]
    async fn mutation_select_best_priority_lowest_load() {
        // Targets: L518 < → ==, < → > in select_best_agent (Priority)
        let coordinator = AgentCoordinator::new(CoordinatorConfig {
            resource_strategy: ResourceStrategy::Priority,
            ..Default::default()
        });

        // Busy agent (3 active tasks)
        let mut base_busy = BaseAgent::new("busy".into(), vec![]);
        base_busy.resource_usage.active_tasks = 3;
        let busy_id = base_busy.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base: base_busy }))
            .await
            .unwrap();

        // Idle agent (0 active tasks)
        let base_idle = BaseAgent::new("idle".into(), vec![]);
        let idle_id = base_idle.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base: base_idle }))
            .await
            .unwrap();

        // Call select_best_agent directly with busy FIRST in candidates
        let candidates = vec![busy_id.clone(), idle_id.clone()];
        let task = Task::new("t".into(), "d".into());
        let selected = coordinator
            .select_best_agent(&candidates, &task)
            .await
            .unwrap();
        assert_eq!(
            selected, idle_id,
            "Priority strategy should select agent with lowest load"
        );
    }

    #[tokio::test]
    async fn mutation_select_best_load_balance() {
        // Targets: L529 % → / and % → + in select_best_agent (LoadBalance)
        let coordinator = AgentCoordinator::new(CoordinatorConfig {
            resource_strategy: ResourceStrategy::LoadBalance,
            ..Default::default()
        });

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.agent_id().to_string();
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        let a2 = TestAgent {
            base: BaseAgent::new("a2".into(), vec![]),
        };
        let a2_id = a2.agent_id().to_string();
        coordinator.register_agent(Box::new(a2)).await.unwrap();

        // LoadBalance: index = task.id.len() % candidates.len()
        // UUID is 36 chars, 36 % 2 = 0 → candidates[0]
        // With % → /: 36 / 2 = 18 → out of bounds (panic)
        // With % → +: 36 + 2 = 38 → out of bounds (panic)
        let candidates = vec![a1_id.clone(), a2_id.clone()];
        let task = Task::new("t".into(), "d".into());
        let selected = coordinator
            .select_best_agent(&candidates, &task)
            .await
            .unwrap();
        assert!(
            selected == a1_id || selected == a2_id,
            "LoadBalance should return a valid candidate"
        );
    }

    #[tokio::test]
    async fn mutation_select_best_adaptive_lowest_load() {
        // Targets: L543 < → ==, < → > in select_best_agent (Adaptive)
        let coordinator = AgentCoordinator::new(CoordinatorConfig {
            resource_strategy: ResourceStrategy::Adaptive,
            ..Default::default()
        });

        let mut base_busy = BaseAgent::new("busy".into(), vec![]);
        base_busy.resource_usage.active_tasks = 3;
        let busy_id = base_busy.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base: base_busy }))
            .await
            .unwrap();

        let base_idle = BaseAgent::new("idle".into(), vec![]);
        let idle_id = base_idle.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base: base_idle }))
            .await
            .unwrap();

        let candidates = vec![busy_id.clone(), idle_id.clone()];
        let task = Task::new("t".into(), "d".into());
        let selected = coordinator
            .select_best_agent(&candidates, &task)
            .await
            .unwrap();
        assert_eq!(
            selected, idle_id,
            "Adaptive strategy should select agent with lowest load"
        );
    }

    #[tokio::test]
    async fn mutation_find_suitable_rejects_unavailable() {
        // Targets: L487 && → || in find_suitable_agents
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        // Agent with 5+ active tasks (unavailable: is_available = false)
        let mut base = BaseAgent::new("type".into(), vec![]);
        base.resource_usage.active_tasks = 5;
        coordinator
            .register_agent(Box::new(TestAgent { base }))
            .await
            .unwrap();

        let task = Task::new("t".into(), "d".into());
        let result = coordinator.assign_task(task).await;
        // With &&: can_handle(true) && available(false) = false → no suitable → Err
        // With ||: can_handle(true) || available(false) = true → would select agent
        assert!(
            result.is_err(),
            "Should fail when only agent is unavailable"
        );
    }

    #[tokio::test]
    async fn mutation_unregister_cleans_sessions() {
        // Targets: L558 remove_agent_from_sessions → (), L562 != → ==
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.agent_id().to_string();
        let a2 = TestAgent {
            base: BaseAgent::new("a2".into(), vec![]),
        };
        let a2_id = a2.agent_id().to_string();

        coordinator.register_agent(Box::new(a1)).await.unwrap();
        coordinator.register_agent(Box::new(a2)).await.unwrap();

        let session_id = coordinator
            .start_coordination(vec![a1_id.clone(), a2_id.clone()], vec![])
            .await
            .unwrap();

        // Unregister a1 — should remove from session
        coordinator.unregister_agent(&a1_id).await.unwrap();

        let sessions = coordinator.coordination_sessions.read().await;
        let ctx = sessions[&session_id].read().await;
        assert!(
            !ctx.participants.contains(&a1_id),
            "Unregistered agent should be removed from session"
        );
        assert!(
            ctx.participants.contains(&a2_id),
            "Other agent should remain in session"
        );
    }

    #[tokio::test]
    async fn mutation_cleanup_expired_sessions_works() {
        // Targets: L593 cleanup → (), L601 < → ==, < → >
        let config = CoordinatorConfig {
            max_coordination_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let coordinator = AgentCoordinator::new(config);

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.agent_id().to_string();
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        let session_id = coordinator
            .start_coordination(vec![a1_id.clone()], vec![])
            .await
            .unwrap();

        // Wait for session to expire
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Create a second fresh session AFTER sleep (not expired)
        // We need a new config with longer duration for this to work,
        // but the coordinator already has short duration. Instead, verify
        // both paths: expired removed AND that fresh sessions aren't wrongly removed.
        // The fresh session's start_time is NOW, so duration ≈ 0ms < 1ms → kept.
        let a2 = TestAgent {
            base: BaseAgent::new("a2".into(), vec![]),
        };
        let a2_id = a2.agent_id().to_string();
        coordinator.register_agent(Box::new(a2)).await.unwrap();
        let fresh_session_id = coordinator
            .start_coordination(vec![a2_id], vec![])
            .await
            .unwrap();

        // Trigger cleanup via update
        coordinator.update().await.unwrap();

        let sessions = coordinator.coordination_sessions.read().await;
        assert!(
            !sessions.contains_key(&session_id),
            "Expired session should be cleaned up"
        );
        // With < → ==: duration(0ms) == max(1ms) is false → keep=false → wrongly removed
        // With <: duration(0ms) < max(1ms) is true → keep=true → correctly retained
        assert!(
            sessions.contains_key(&fresh_session_id),
            "Fresh session should NOT be cleaned up (kills < → ==)"
        );
    }

    #[tokio::test]
    async fn mutation_update_metrics_availability_and_utilization() {
        // Targets: L614 update_metrics → (), L631 * → + and * → /,
        //          L632 > → ==, > → <, L633 / → % and / → *
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        // Agent with 2 active tasks (still available: < 5)
        let mut base = BaseAgent::new("loaded".into(), vec![]);
        base.resource_usage.active_tasks = 2;
        let loaded_id = base.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base }))
            .await
            .unwrap();

        // Another agent with 0 active tasks
        let base2 = BaseAgent::new("idle".into(), vec![]);
        let idle_id = base2.id.clone();
        coordinator
            .register_agent(Box::new(TestAgent { base: base2 }))
            .await
            .unwrap();

        coordinator.update().await.unwrap();
        let metrics = coordinator.get_metrics().await;

        // Check availability map populated
        assert_eq!(
            metrics.agent_availability.len(),
            2,
            "Should have 2 agent availability entries"
        );
        assert_eq!(
            *metrics.agent_availability.get(&loaded_id).unwrap(),
            1.0,
            "Loaded agent should be available (active_tasks < 5)"
        );
        assert_eq!(
            *metrics.agent_availability.get(&idle_id).unwrap(),
            1.0,
            "Idle agent should be available"
        );

        // Check utilization: 2 active / (2 agents * 5 max_tasks) = 2/10 = 0.2
        // With * → +: 2 + 5 = 7, utilization = 2/7 ≈ 0.286
        // With * → /: 2/5 = 0 (int), > 0 false → 0.0
        // With / → %: 2.0 % 10.0 = 2.0
        // With / → *: 2.0 * 10.0 = 20.0
        assert!(
            (metrics.resource_utilization - 0.2).abs() < 0.001,
            "Utilization should be 0.2, got {}",
            metrics.resource_utilization
        );
    }

    #[tokio::test]
    async fn mutation_update_metrics_zero_agents_utilization() {
        // Targets: L632 > → >= (0 agents: 0 >= 0 = true → 0/0 = NaN)
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        coordinator.update().await.unwrap();

        let metrics = coordinator.get_metrics().await;
        assert_eq!(
            metrics.resource_utilization, 0.0,
            "Should be 0.0 with no agents"
        );
        assert!(!metrics.resource_utilization.is_nan(), "Must not be NaN");
    }

    #[tokio::test]
    async fn mutation_dispatch_event_stores_history() {
        // Targets: L435 dispatch_event → Ok(())
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());

        let event = WorldEvent {
            id: "e-1".into(),
            event_type: "combat".into(),
            description: "Battle".into(),
            location: None,
            participants: vec![],
            event_data: serde_json::json!({}),
            timestamp: Utc::now(),
            severity: crate::agent::EventSeverity::Major,
        };

        coordinator.dispatch_event(event).await.unwrap();

        let history = coordinator.event_dispatcher.event_history.read().await;
        assert_eq!(history.len(), 1, "Event should be stored in history");
        assert_eq!(history[0].id, "e-1");
    }

    #[tokio::test]
    async fn mutation_update_triggers_cleanup_and_metrics() {
        // Targets: L572 update → Ok(())
        let config = CoordinatorConfig {
            max_coordination_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let coordinator = AgentCoordinator::new(config);

        let a1 = TestAgent {
            base: BaseAgent::new("a1".into(), vec![]),
        };
        let a1_id = a1.agent_id().to_string();
        coordinator.register_agent(Box::new(a1)).await.unwrap();

        let session_id = coordinator
            .start_coordination(vec![a1_id.clone()], vec![])
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        coordinator.update().await.unwrap();

        // Verify cleanup happened (if update was replaced with Ok(()),
        // cleanup_expired_sessions would not run)
        let sessions = coordinator.coordination_sessions.read().await;
        assert!(
            !sessions.contains_key(&session_id),
            "Expired session should be cleaned up by update()"
        );
        drop(sessions);

        // Verify metrics were updated
        let metrics = coordinator.get_metrics().await;
        assert!(
            metrics.agent_availability.contains_key(&a1_id),
            "Metrics should contain agent availability after update"
        );
    }

    #[tokio::test]
    async fn mutation_rule_matches_to_pattern_and_message_type() {
        // Targets: L725 delete !, L731 != → == in rule_matches
        let router = MessageRouter::new();

        // Test to_pattern matching
        let rule_to = RoutingRule {
            id: "r-to".into(),
            from_pattern: None,
            to_pattern: Some("npc".into()),
            message_type: None,
            action: RoutingAction::Block,
            priority: 1,
        };

        let msg_to_match = AgentMessage {
            id: "m".into(),
            from: "player".into(),
            to: "npc-guard".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        let msg_to_no_match = AgentMessage {
            id: "m".into(),
            from: "player".into(),
            to: "merchant".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };

        assert!(
            router.rule_matches(&rule_to, &msg_to_match),
            "to_pattern 'npc' should match 'npc-guard'"
        );
        assert!(
            !router.rule_matches(&rule_to, &msg_to_no_match),
            "to_pattern 'npc' should NOT match 'merchant'"
        );

        // Test message_type matching
        let rule_type = RoutingRule {
            id: "r-type".into(),
            from_pattern: None,
            to_pattern: None,
            message_type: Some("Request".into()),
            action: RoutingAction::Block,
            priority: 1,
        };

        let msg_request = AgentMessage {
            id: "m".into(),
            from: "a".into(),
            to: "b".into(),
            message_type: crate::agent::MessageType::Request,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };
        let msg_response = AgentMessage {
            id: "m".into(),
            from: "a".into(),
            to: "b".into(),
            message_type: crate::agent::MessageType::Response,
            content: serde_json::json!({}),
            timestamp: Utc::now(),
            priority: crate::agent::MessagePriority::Normal,
            reply_to: None,
        };

        assert!(
            router.rule_matches(&rule_type, &msg_request),
            "Request should match 'Request' type filter"
        );
        assert!(
            !router.rule_matches(&rule_type, &msg_response),
            "Response should NOT match 'Request' type filter"
        );
    }

    #[tokio::test]
    async fn mutation_can_allocate_memory_and_used_plus_req() {
        // Targets: L775 + → * in can_allocate
        let rm = ResourceManager::new(ResourceLimits::default(), ResourceStrategy::Priority);
        rm.initialize_allocation("a").await.unwrap();

        // Allocation defaults: llm_calls_allocated = 100, memory_allocated = 819

        // Test memory-only exceeds (kills && → || on the two conditions)
        let req_mem_over = crate::agent::ResourceRequirements {
            llm_calls: 50,
            memory_mb: 900, // exceeds 819
            compute_units: 0,
            exclusive_resources: vec![],
        };
        assert!(
            !rm.can_allocate("a", &req_mem_over).await,
            "Should reject when memory exceeds even if LLM is within limits"
        );

        // Allocate some resources first to make used > 0
        let initial = crate::agent::ResourceRequirements {
            llm_calls: 50,
            memory_mb: 400,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        rm.allocate_resources("a", &initial).await.unwrap();
        // Now: llm_calls_used = 50, memory_used = 400

        // Request 40 LLM + 300 memory — within limits with +, NOT with *
        // + path: 50+40=90 <= 100 ✓, 400+300=700 <= 819 ✓
        // * path: 50*40=2000 > 100 ✗, 400*300=120000 > 819 ✗
        let req_ok = crate::agent::ResourceRequirements {
            llm_calls: 40,
            memory_mb: 300,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        assert!(
            rm.can_allocate("a", &req_ok).await,
            "used(50)+req(40)=90 <= 100 and used(400)+req(300)=700 <= 819 should pass"
        );
    }

    #[tokio::test]
    async fn mutation_event_history_caps_at_1000() {
        // Targets: L860 > → == and > → >= in EventDispatcher::store_event
        let ed = EventDispatcher::new();
        for i in 0..1005 {
            let event = WorldEvent {
                id: format!("e-{}", i),
                event_type: "test".into(),
                description: "Test".into(),
                location: None,
                participants: vec![],
                event_data: serde_json::json!({}),
                timestamp: Utc::now(),
                severity: crate::agent::EventSeverity::Trivial,
            };
            ed.store_event(event).await;
        }
        let history = ed.event_history.read().await;
        assert_eq!(history.len(), 1000, "Event history should cap at 1000");
    }

    #[tokio::test]
    async fn mutation_update_allocations_preserves_recent() {
        // Targets: L809 >= → < in update_allocations
        let coordinator = AgentCoordinator::new(CoordinatorConfig::default());
        let agent = TestAgent {
            base: BaseAgent::new("a".into(), vec![]),
        };
        let agent_id = agent.agent_id().to_string();
        coordinator.register_agent(Box::new(agent)).await.unwrap();

        // Allocate some resources
        let req = crate::agent::ResourceRequirements {
            llm_calls: 50,
            memory_mb: 100,
            compute_units: 0,
            exclusive_resources: vec![],
        };
        coordinator
            .resource_manager
            .allocate_resources(&agent_id, &req)
            .await
            .unwrap();

        // Call update — should NOT reset because allocation is < 1 min old
        coordinator.update().await.unwrap();

        let allocs = coordinator.resource_manager.allocations.read().await;
        assert_eq!(
            allocs.get(&agent_id).unwrap().llm_calls_used,
            50,
            "Recent allocation usage should NOT be reset (kills >= → <)"
        );
    }
}

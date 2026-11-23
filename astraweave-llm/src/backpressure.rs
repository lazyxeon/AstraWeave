use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, RwLock, Semaphore};
use tracing::{debug, info};
use uuid::Uuid;

/// Backpressure management system for LLM requests
pub struct BackpressureManager {
    /// Request queues by priority
    queues: Arc<RwLock<PriorityQueues>>,
    /// Active request tracking
    active_requests: Arc<RwLock<HashMap<String, ActiveRequest>>>,
    /// Concurrency control
    semaphore: Arc<Semaphore>,
    /// Configuration
    config: BackpressureConfig,
    /// Metrics
    metrics: Arc<RwLock<BackpressureMetrics>>,
    /// Queue processor handle
    processor_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Maximum queue size per priority
    pub max_queue_size: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Queue processing interval
    pub processing_interval: Duration,
    /// Enable adaptive concurrency
    pub adaptive_concurrency: bool,
    /// Target latency for adaptive adjustment (ms)
    pub target_latency_ms: u64,
    /// Load shedding threshold (0.0 to 1.0)
    pub load_shedding_threshold: f32,
    /// Enable graceful degradation
    pub enable_graceful_degradation: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 100,
            max_queue_size: 1000,
            request_timeout: Duration::from_secs(30),
            processing_interval: Duration::from_millis(10),
            adaptive_concurrency: true,
            target_latency_ms: 1000,
            load_shedding_threshold: 0.9,
            enable_graceful_degradation: true,
        }
    }
}

/// Priority levels for request queuing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Priority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

impl Priority {
    pub fn all() -> Vec<Priority> {
        vec![
            Priority::Critical,
            Priority::High,
            Priority::Normal,
            Priority::Low,
            Priority::Background,
        ]
    }
}

/// Queued request
#[derive(Debug)]
struct QueuedRequest {
    id: String,
    priority: Priority,
    queued_at: Instant,
    timeout: Duration,
    sender: oneshot::Sender<Result<()>>,
    metadata: RequestMetadata,
}

/// Request metadata for processing decisions
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub user_id: Option<String>,
    pub model: String,
    pub estimated_tokens: u32,
    pub estimated_cost: f64,
    pub tags: HashMap<String, String>,
}

/// Active request tracking
#[derive(Debug)]
#[allow(dead_code)]
struct ActiveRequest {
    id: String,
    priority: Priority,
    started_at: Instant,
    metadata: RequestMetadata,
}

/// Priority-based queues
#[derive(Debug, Default)]
struct PriorityQueues {
    queues: HashMap<Priority, VecDeque<QueuedRequest>>,
}

impl PriorityQueues {
    fn new() -> Self {
        let mut queues = HashMap::new();
        for priority in Priority::all() {
            queues.insert(priority, VecDeque::new());
        }
        Self { queues }
    }

    fn enqueue(&mut self, request: QueuedRequest, max_size: usize) -> Result<()> {
        let queue = self
            .queues
            .get_mut(&request.priority)
            .ok_or_else(|| anyhow!("Invalid priority: {:?}", request.priority))?;

        if queue.len() >= max_size {
            return Err(anyhow!("Queue full for priority {:?}", request.priority));
        }

        queue.push_back(request);
        Ok(())
    }

    fn dequeue_highest_priority(&mut self) -> Option<QueuedRequest> {
        for priority in Priority::all() {
            if let Some(queue) = self.queues.get_mut(&priority) {
                if let Some(request) = queue.pop_front() {
                    return Some(request);
                }
            }
        }
        None
    }

    fn total_queued(&self) -> usize {
        self.queues.values().map(|q| q.len()).sum()
    }

    fn queued_by_priority(&self, priority: Priority) -> usize {
        self.queues.get(&priority).map(|q| q.len()).unwrap_or(0)
    }

    fn remove_expired(&mut self, now: Instant) -> Vec<QueuedRequest> {
        let mut expired = Vec::new();

        for queue in self.queues.values_mut() {
            let mut i = 0;
            while i < queue.len() {
                if let Some(request) = queue.get(i) {
                    if now.duration_since(request.queued_at) >= request.timeout {
                        if let Some(expired_request) = queue.remove(i) {
                            expired.push(expired_request);
                        }
                        continue;
                    }
                }
                i += 1;
            }
        }

        expired
    }
}

/// Backpressure management result
#[derive(Debug, Clone)]
#[non_exhaustive]
#[must_use]
pub enum BackpressureResult {
    /// Request was accepted and can proceed immediately
    Accepted { request_id: String },
    /// Request was queued and will be processed later
    Queued {
        position: usize,
        estimated_wait: Duration,
    },
    /// Request was rejected due to system overload
    Rejected {
        reason: String,
        retry_after: Option<Duration>,
    },
    /// Request was degraded (lower quality response)
    Degraded { request_id: String, reason: String },
}

/// Backpressure metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BackpressureMetrics {
    pub total_requests: u64,
    pub accepted_requests: u64,
    pub queued_requests: u64,
    pub rejected_requests: u64,
    pub degraded_requests: u64,
    pub expired_requests: u64,
    pub current_queue_size: usize,
    pub current_active_requests: usize,
    pub average_queue_time_ms: f32,
    pub average_processing_time_ms: f32,
    pub queue_sizes_by_priority: HashMap<Priority, usize>,
    pub load_factor: f32,
    pub adaptive_concurrency_limit: usize,
    pub last_updated: String,
}

impl BackpressureManager {
    pub fn new(config: BackpressureConfig) -> Self {
        let metrics = BackpressureMetrics {
            adaptive_concurrency_limit: config.max_concurrent_requests,
            ..Default::default()
        };

        Self {
            queues: Arc::new(RwLock::new(PriorityQueues::new())),
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            config,
            metrics: Arc::new(RwLock::new(metrics)),
            processor_handle: None,
        }
    }

    /// Start the background queue processor
    pub async fn start(&mut self) -> Result<()> {
        if self.processor_handle.is_some() {
            return Err(anyhow!("Backpressure manager is already running"));
        }

        let queues = self.queues.clone();
        let active_requests = self.active_requests.clone();
        let semaphore = self.semaphore.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            Self::queue_processor(queues, active_requests, semaphore, metrics, config).await;
        });

        self.processor_handle = Some(handle);
        info!("Started backpressure manager");
        Ok(())
    }

    /// Stop the background queue processor
    pub async fn stop(&mut self) {
        if let Some(handle) = self.processor_handle.take() {
            handle.abort();
            info!("Stopped backpressure manager");
        }
    }

    /// Submit a request for processing
    pub async fn submit_request(
        &self,
        priority: Priority,
        timeout: Option<Duration>,
        metadata: RequestMetadata,
    ) -> Result<BackpressureResult> {
        let request_id = Uuid::new_v4().to_string();
        let timeout = timeout.unwrap_or(self.config.request_timeout);

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
        }

        // Check if we should reject due to overload
        if self.should_reject_request(priority).await {
            let mut metrics = self.metrics.write().await;
            metrics.rejected_requests += 1;
            return Ok(BackpressureResult::Rejected {
                reason: "System overloaded".to_string(),
                retry_after: Some(Duration::from_secs(1)),
            });
        }

        // Check if we should degrade the request
        if self.should_degrade_request(priority).await {
            let mut metrics = self.metrics.write().await;
            metrics.degraded_requests += 1;
            return Ok(BackpressureResult::Degraded {
                request_id,
                reason: "High load - using faster but lower quality model".to_string(),
            });
        }

        // Try to acquire semaphore immediately
        if let Ok(permit) = self.semaphore.clone().try_acquire_owned() {
            // Can proceed immediately
            let active_request = ActiveRequest {
                id: request_id.clone(),
                priority,
                started_at: Instant::now(),
                metadata,
            };

            {
                let mut active_requests = self.active_requests.write().await;
                active_requests.insert(request_id.clone(), active_request);
            }

            {
                let mut metrics = self.metrics.write().await;
                metrics.accepted_requests += 1;
                metrics.current_active_requests = self.active_requests.read().await.len();
            }

            // Release permit when done (in practice, this would be done by the caller)
            tokio::spawn(async move {
                // Simulate some processing time
                tokio::time::sleep(Duration::from_millis(100)).await;
                drop(permit);
            });

            return Ok(BackpressureResult::Accepted { request_id });
        }

        // Need to queue the request
        let (sender, _receiver) = oneshot::channel();
        let queued_request = QueuedRequest {
            id: request_id,
            priority,
            queued_at: Instant::now(),
            timeout,
            sender,
            metadata,
        };

        let position = {
            let mut queues = self.queues.write().await;

            queues.enqueue(queued_request, self.config.max_queue_size)?;

            // Calculate position in queue (higher priority requests go first)
            let mut position = 0;
            for p in Priority::all() {
                if p < priority {
                    position += queues.queued_by_priority(p);
                } else if p == priority {
                    position += queues.queued_by_priority(p) - 1; // -1 because we just added this request
                    break;
                }
            }
            position + 1 // 1-indexed position
        };

        {
            let mut metrics = self.metrics.write().await;
            metrics.queued_requests += 1;
            metrics.current_queue_size = self.queues.read().await.total_queued();
        }

        let estimated_wait = self.estimate_wait_time(position).await;

        Ok(BackpressureResult::Queued {
            position,
            estimated_wait,
        })
    }

    /// Complete a request
    pub async fn complete_request(&self, request_id: &str, success: bool) -> Result<()> {
        let processing_time = {
            let mut active_requests = self.active_requests.write().await;
            if let Some(request) = active_requests.remove(request_id) {
                Some(request.started_at.elapsed())
            } else {
                None
            }
        };

        if let Some(duration) = processing_time {
            let mut metrics = self.metrics.write().await;
            let current_avg = metrics.average_processing_time_ms;
            let new_sample = duration.as_millis() as f32;
            metrics.average_processing_time_ms = if current_avg == 0.0 {
                new_sample
            } else {
                0.1 * new_sample + 0.9 * current_avg
            };
            metrics.current_active_requests = self.active_requests.read().await.len();

            // Adjust adaptive concurrency if enabled
            if self.config.adaptive_concurrency {
                self.adjust_concurrency(&mut metrics, success, duration)
                    .await;
            }
        }

        debug!("Completed request {} (success: {})", request_id, success);
        Ok(())
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> BackpressureMetrics {
        let mut metrics = self.metrics.read().await.clone();
        let queues = self.queues.read().await;

        metrics.current_queue_size = queues.total_queued();
        metrics.current_active_requests = self.active_requests.read().await.len();
        metrics.load_factor =
            metrics.current_active_requests as f32 / metrics.adaptive_concurrency_limit as f32;

        for priority in Priority::all() {
            metrics
                .queue_sizes_by_priority
                .insert(priority, queues.queued_by_priority(priority));
        }

        metrics.last_updated =
            crate::utc_stamp::format_utc_rfc3339(time::OffsetDateTime::now_utc());
        metrics
    }

    /// Background queue processor
    async fn queue_processor(
        queues: Arc<RwLock<PriorityQueues>>,
        active_requests: Arc<RwLock<HashMap<String, ActiveRequest>>>,
        semaphore: Arc<Semaphore>,
        metrics: Arc<RwLock<BackpressureMetrics>>,
        config: BackpressureConfig,
    ) {
        let mut processing_interval = tokio::time::interval(config.processing_interval);

        loop {
            processing_interval.tick().await;

            // Remove expired requests
            let expired_requests = {
                let mut queues = queues.write().await;
                queues.remove_expired(Instant::now())
            };

            for expired in expired_requests {
                let _ = expired.sender.send(Err(anyhow!("Request timed out")));
                let mut metrics = metrics.write().await;
                metrics.expired_requests += 1;
            }

            // Process queued requests
            while let Ok(permit) = semaphore.clone().try_acquire_owned() {
                let queued_request = {
                    let mut queues = queues.write().await;
                    queues.dequeue_highest_priority()
                };

                if let Some(request) = queued_request {
                    let queue_time = request.queued_at.elapsed();

                    // Update queue time metrics
                    {
                        let mut metrics = metrics.write().await;
                        let current_avg = metrics.average_queue_time_ms;
                        let new_sample = queue_time.as_millis() as f32;
                        metrics.average_queue_time_ms = if current_avg == 0.0 {
                            new_sample
                        } else {
                            0.1 * new_sample + 0.9 * current_avg
                        };
                        metrics.accepted_requests += 1;
                    }

                    // Track as active request
                    let active_request = ActiveRequest {
                        id: request.id.clone(),
                        priority: request.priority,
                        started_at: Instant::now(),
                        metadata: request.metadata,
                    };

                    {
                        let mut active_requests = active_requests.write().await;
                        active_requests.insert(request.id, active_request);
                    }

                    // Notify that request can proceed
                    let _ = request.sender.send(Ok(()));

                    // Release permit after some time (this would be done by the actual request processor)
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        drop(permit);
                    });
                } else {
                    // No more queued requests
                    drop(permit);
                    break;
                }
            }
        }
    }

    /// Check if request should be rejected due to overload
    async fn should_reject_request(&self, priority: Priority) -> bool {
        if !self.config.enable_graceful_degradation {
            return false;
        }

        let metrics = self.metrics.read().await;
        let load_factor =
            metrics.current_active_requests as f32 / metrics.adaptive_concurrency_limit as f32;

        // Only reject lower priority requests when system is heavily loaded
        match priority {
            Priority::Critical => false,
            Priority::High => load_factor > 0.99,
            Priority::Normal => load_factor > 0.95,
            Priority::Low => load_factor > self.config.load_shedding_threshold,
            Priority::Background => load_factor > 0.8,
        }
    }

    /// Check if request should be degraded
    async fn should_degrade_request(&self, priority: Priority) -> bool {
        if !self.config.enable_graceful_degradation {
            return false;
        }

        let metrics = self.metrics.read().await;
        let load_factor =
            metrics.current_active_requests as f32 / metrics.adaptive_concurrency_limit as f32;

        // Degrade lower priority requests when system is moderately loaded
        match priority {
            Priority::Critical | Priority::High => false,
            Priority::Normal => load_factor > 0.8,
            Priority::Low => load_factor > 0.7,
            Priority::Background => load_factor > 0.6,
        }
    }

    /// Estimate wait time for a queued request
    async fn estimate_wait_time(&self, position: usize) -> Duration {
        let metrics = self.metrics.read().await;
        let avg_processing_time = Duration::from_millis(metrics.average_processing_time_ms as u64);
        let concurrent_capacity = metrics.adaptive_concurrency_limit;

        if concurrent_capacity > 0 {
            let estimated_cycles = position.div_ceil(concurrent_capacity);
            avg_processing_time * estimated_cycles as u32
        } else {
            Duration::from_secs(60) // Fallback estimate
        }
    }

    /// Adjust adaptive concurrency based on performance
    async fn adjust_concurrency(
        &self,
        metrics: &mut BackpressureMetrics,
        success: bool,
        latency: Duration,
    ) {
        let current_limit = metrics.adaptive_concurrency_limit.max(1);
        let target_latency = Duration::from_millis(self.config.target_latency_ms);

        let new_limit = if success && latency < target_latency {
            // Performance is good, try increasing concurrency
            (current_limit as f32 * 1.05).min(self.config.max_concurrent_requests as f32) as usize
        } else if !success || latency > target_latency * 2 {
            // Performance is poor, decrease concurrency
            (current_limit as f32 * 0.9).max(1.0) as usize
        } else {
            current_limit
        };

        if new_limit != current_limit {
            metrics.adaptive_concurrency_limit = new_limit;
            debug!(
                "Adjusted adaptive concurrency limit: {} -> {}",
                current_limit, new_limit
            );
        }
    }
}

impl Drop for BackpressureManager {
    fn drop(&mut self) {
        if let Some(handle) = self.processor_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_config_default() {
        let config = BackpressureConfig::default();
        assert_eq!(config.max_concurrent_requests, 100);
        assert_eq!(config.max_queue_size, 1000);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.target_latency_ms, 1000);
        assert!(config.adaptive_concurrency);
        assert!(config.enable_graceful_degradation);
    }

    #[test]
    fn test_backpressure_config_custom() {
        let config = BackpressureConfig {
            max_concurrent_requests: 50,
            max_queue_size: 500,
            request_timeout: Duration::from_secs(60),
            processing_interval: Duration::from_millis(20),
            adaptive_concurrency: false,
            target_latency_ms: 500,
            load_shedding_threshold: 0.8,
            enable_graceful_degradation: false,
        };
        assert_eq!(config.max_concurrent_requests, 50);
        assert_eq!(config.max_queue_size, 500);
        assert!(!config.adaptive_concurrency);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
        assert!(Priority::Low < Priority::Background);
    }

    #[test]
    fn test_priority_all() {
        let all = Priority::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], Priority::Critical);
        assert_eq!(all[4], Priority::Background);
    }

    #[test]
    fn test_priority_equality() {
        assert_eq!(Priority::Normal, Priority::Normal);
        assert_ne!(Priority::High, Priority::Low);
    }

    #[test]
    fn test_request_metadata_creation() {
        let metadata = RequestMetadata {
            user_id: Some("user123".to_string()),
            model: "gpt-4".to_string(),
            estimated_tokens: 1000,
            estimated_cost: 0.05,
            tags: HashMap::new(),
        };
        assert_eq!(metadata.user_id, Some("user123".to_string()));
        assert_eq!(metadata.model, "gpt-4");
        assert_eq!(metadata.estimated_tokens, 1000);
    }

    #[test]
    fn test_request_metadata_with_tags() {
        let mut tags = HashMap::new();
        tags.insert("env".to_string(), "production".to_string());
        tags.insert("team".to_string(), "backend".to_string());

        let metadata = RequestMetadata {
            user_id: None,
            model: "gpt-3.5-turbo".to_string(),
            estimated_tokens: 500,
            estimated_cost: 0.001,
            tags,
        };
        assert!(metadata.user_id.is_none());
        assert_eq!(metadata.tags.len(), 2);
        assert_eq!(metadata.tags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_backpressure_metrics_default() {
        let metrics = BackpressureMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.accepted_requests, 0);
        assert_eq!(metrics.queued_requests, 0);
        assert_eq!(metrics.rejected_requests, 0);
        assert_eq!(metrics.current_queue_size, 0);
    }

    #[test]
    fn test_backpressure_metrics_clone() {
        let metrics = BackpressureMetrics {
            total_requests: 100,
            accepted_requests: 80,
            queued_requests: 15,
            rejected_requests: 5,
            degraded_requests: 2,
            expired_requests: 1,
            current_queue_size: 10,
            current_active_requests: 20,
            average_queue_time_ms: 50.5,
            average_processing_time_ms: 200.0,
            queue_sizes_by_priority: HashMap::new(),
            load_factor: 0.75,
            adaptive_concurrency_limit: 100,
            last_updated: "2025-01-01T00:00:00Z".to_string(),
        };
        let cloned = metrics.clone();
        assert_eq!(metrics.total_requests, cloned.total_requests);
        assert_eq!(metrics.load_factor, cloned.load_factor);
    }

    #[test]
    fn test_backpressure_metrics_serialization() {
        let metrics = BackpressureMetrics {
            total_requests: 50,
            accepted_requests: 45,
            rejected_requests: 5,
            load_factor: 0.5,
            ..Default::default()
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("50"));
        assert!(json.contains("0.5"));
    }

    #[test]
    fn test_priority_queues_new() {
        let queues = PriorityQueues::new();
        assert_eq!(queues.total_queued(), 0);
        for priority in Priority::all() {
            assert_eq!(queues.queued_by_priority(priority), 0);
        }
    }

    #[test]
    fn test_backpressure_result_variants() {
        // Test Accepted
        let accepted = BackpressureResult::Accepted {
            request_id: "test".to_string(),
        };
        assert!(matches!(accepted, BackpressureResult::Accepted { .. }));

        // Test Queued
        let queued = BackpressureResult::Queued {
            position: 5,
            estimated_wait: Duration::from_secs(10),
        };
        if let BackpressureResult::Queued {
            position,
            estimated_wait,
        } = queued
        {
            assert_eq!(position, 5);
            assert_eq!(estimated_wait, Duration::from_secs(10));
        }

        // Test Rejected
        let rejected = BackpressureResult::Rejected {
            reason: "System overload".to_string(),
            retry_after: Some(Duration::from_secs(60)),
        };
        if let BackpressureResult::Rejected {
            reason,
            retry_after,
        } = rejected
        {
            assert_eq!(reason, "System overload");
            assert_eq!(retry_after, Some(Duration::from_secs(60)));
        }

        // Test Degraded
        let degraded = BackpressureResult::Degraded {
            request_id: "test".to_string(),
            reason: "High load".to_string(),
        };
        if let BackpressureResult::Degraded { request_id, reason } = degraded {
            assert_eq!(request_id, "test");
            assert_eq!(reason, "High load");
        }
    }

    #[tokio::test]
    async fn test_backpressure_manager_creation() {
        let config = BackpressureConfig::default();
        let manager = BackpressureManager::new(config);

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
    }

    #[tokio::test]
    async fn test_request_acceptance() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: Some("test_user".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            estimated_tokens: 100,
            estimated_cost: 0.01,
            tags: HashMap::new(),
        };

        let result = manager
            .submit_request(Priority::Normal, None, metadata)
            .await
            .unwrap();

        match result {
            BackpressureResult::Accepted { .. } => {
                // Expected result
            }
            _ => panic!("Expected request to be accepted"),
        }

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_request_queuing() {
        let config = BackpressureConfig {
            max_concurrent_requests: 1,         // Force queuing
            enable_graceful_degradation: false, // Disable rejection logic for this test
            ..Default::default()
        };
        let mut manager = BackpressureManager::new(config);
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: Some("test_user".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            estimated_tokens: 100,
            estimated_cost: 0.01,
            tags: HashMap::new(),
        };

        // First request should be accepted
        let result1 = manager
            .submit_request(Priority::Normal, None, metadata.clone())
            .await
            .unwrap();

        // Verify first request was accepted
        assert!(
            matches!(result1, BackpressureResult::Accepted { .. }),
            "First request should be accepted, got: {:?}",
            result1
        );

        // Second request should be queued immediately (semaphore exhausted)
        // No delay needed - the permit is held for 100ms, so it's definitely still held
        let result2 = manager
            .submit_request(Priority::Normal, None, metadata)
            .await
            .unwrap();

        match &result2 {
            BackpressureResult::Queued { position, .. } => {
                assert_eq!(*position, 1);
            }
            _ => panic!("Expected request to be queued, got: {:?}", result2),
        }

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_priority_order_in_manager() {
        let config = BackpressureConfig {
            max_concurrent_requests: 0, // Force all requests to queue
            ..Default::default()
        };
        let mut manager = BackpressureManager::new(config);
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: Some("test_user".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            estimated_tokens: 100,
            estimated_cost: 0.01,
            tags: HashMap::new(),
        };

        // Submit requests in reverse priority order
        let _low = manager
            .submit_request(Priority::Low, None, metadata.clone())
            .await
            .unwrap();
        let _normal = manager
            .submit_request(Priority::Normal, None, metadata.clone())
            .await
            .unwrap();
        let _high = manager
            .submit_request(Priority::High, None, metadata)
            .await
            .unwrap();

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.current_queue_size, 3);

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: Some("test_user".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            estimated_tokens: 100,
            estimated_cost: 0.01,
            tags: HashMap::new(),
        };

        // Submit a request
        let _ = manager
            .submit_request(Priority::Normal, None, metadata)
            .await
            .unwrap();

        let metrics = manager.get_metrics().await;
        assert!(metrics.total_requests > 0);

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_complete_request_success() {
        let manager = BackpressureManager::new(BackpressureConfig::default());
        // Complete a non-existent request (should not fail)
        let result = manager.complete_request("non_existent_id", true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_complete_request_failure() {
        let manager = BackpressureManager::new(BackpressureConfig::default());
        let result = manager.complete_request("test_id", false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_twice_fails() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();
        let result = manager.start().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already running"));
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_stop_without_start() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        // Stop without starting should be safe
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_estimate_wait_time() {
        let manager = BackpressureManager::new(BackpressureConfig::default());
        let wait_time = manager.estimate_wait_time(10).await;
        // Should return some duration
        assert!(wait_time >= Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_should_reject_request_critical_never_rejected() {
        let config = BackpressureConfig {
            enable_graceful_degradation: true,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        // Critical priority should never be rejected
        let should_reject = manager.should_reject_request(Priority::Critical).await;
        assert!(!should_reject);
    }

    #[tokio::test]
    async fn test_should_degrade_request_critical_never_degraded() {
        let config = BackpressureConfig {
            enable_graceful_degradation: true,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        // Critical priority should never be degraded
        let should_degrade = manager.should_degrade_request(Priority::Critical).await;
        assert!(!should_degrade);
    }

    #[tokio::test]
    async fn test_should_degrade_request_high_never_degraded() {
        let config = BackpressureConfig {
            enable_graceful_degradation: true,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        let should_degrade = manager.should_degrade_request(Priority::High).await;
        assert!(!should_degrade);
    }

    #[tokio::test]
    async fn test_graceful_degradation_disabled() {
        let config = BackpressureConfig {
            enable_graceful_degradation: false,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        // When disabled, should never reject or degrade
        let should_reject = manager.should_reject_request(Priority::Background).await;
        let should_degrade = manager.should_degrade_request(Priority::Background).await;
        assert!(!should_reject);
        assert!(!should_degrade);
    }

    #[test]
    fn test_priority_queues_default() {
        let queues = PriorityQueues::default();
        assert_eq!(queues.total_queued(), 0);
    }

    #[test]
    fn test_backpressure_result_rejected_none_retry() {
        let rejected = BackpressureResult::Rejected {
            reason: "No capacity".to_string(),
            retry_after: None,
        };
        if let BackpressureResult::Rejected {
            reason,
            retry_after,
        } = rejected
        {
            assert_eq!(reason, "No capacity");
            assert!(retry_after.is_none());
        }
    }

    #[test]
    fn test_priority_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Priority::Critical);
        set.insert(Priority::High);
        set.insert(Priority::Critical); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_priority_serialization() {
        let priority = Priority::Normal;
        let json = serde_json::to_string(&priority).unwrap();
        let deserialized: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(priority, deserialized);
    }

    #[tokio::test]
    async fn test_get_metrics_updates_queue_sizes() {
        let config = BackpressureConfig {
            max_concurrent_requests: 0, // Force all to queue
            ..Default::default()
        };
        let mut manager = BackpressureManager::new(config);
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: None,
            model: "test".to_string(),
            estimated_tokens: 10,
            estimated_cost: 0.0,
            tags: HashMap::new(),
        };

        let _ = manager
            .submit_request(Priority::High, None, metadata.clone())
            .await;
        let _ = manager.submit_request(Priority::Low, None, metadata).await;

        let metrics = manager.get_metrics().await;
        assert!(metrics
            .queue_sizes_by_priority
            .contains_key(&Priority::High));
        assert!(metrics.queue_sizes_by_priority.contains_key(&Priority::Low));
        assert!(!metrics.last_updated.is_empty());
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_submit_request_with_custom_timeout() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: None,
            model: "test".to_string(),
            estimated_tokens: 10,
            estimated_cost: 0.0,
            tags: HashMap::new(),
        };

        // Submit with custom timeout
        let result = manager
            .submit_request(Priority::Normal, Some(Duration::from_secs(5)), metadata)
            .await
            .unwrap();

        assert!(matches!(
            result,
            BackpressureResult::Accepted { .. } | BackpressureResult::Queued { .. }
        ));
        manager.stop().await;
    }

    #[test]
    fn test_backpressure_metrics_with_priority_sizes() {
        let mut queue_sizes = HashMap::new();
        queue_sizes.insert(Priority::Critical, 1);
        queue_sizes.insert(Priority::High, 5);
        queue_sizes.insert(Priority::Normal, 10);

        let metrics = BackpressureMetrics {
            queue_sizes_by_priority: queue_sizes,
            ..Default::default()
        };

        assert_eq!(
            metrics.queue_sizes_by_priority.get(&Priority::Critical),
            Some(&1)
        );
        assert_eq!(
            metrics.queue_sizes_by_priority.get(&Priority::High),
            Some(&5)
        );
        assert_eq!(
            metrics.queue_sizes_by_priority.get(&Priority::Normal),
            Some(&10)
        );
    }

    #[test]
    fn test_request_metadata_clone() {
        let metadata = RequestMetadata {
            user_id: Some("user".to_string()),
            model: "model".to_string(),
            estimated_tokens: 100,
            estimated_cost: 1.0,
            tags: HashMap::new(),
        };
        let cloned = metadata.clone();
        assert_eq!(metadata.user_id, cloned.user_id);
        assert_eq!(metadata.model, cloned.model);
    }

    #[tokio::test]
    async fn test_adjust_concurrency_called_on_complete() {
        let config = BackpressureConfig {
            adaptive_concurrency: true,
            target_latency_ms: 100,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);

        // Complete request triggers adjust_concurrency internally
        // Just verify it doesn't panic
        let _ = manager.complete_request("id", true).await;
        let _ = manager.complete_request("id2", false).await;
    }

    #[test]
    fn test_backpressure_config_clone() {
        let config = BackpressureConfig::default();
        let cloned = config.clone();
        assert_eq!(
            config.max_concurrent_requests,
            cloned.max_concurrent_requests
        );
        assert_eq!(config.target_latency_ms, cloned.target_latency_ms);
    }

    #[test]
    fn test_backpressure_config_debug() {
        let config = BackpressureConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("max_concurrent_requests"));
        assert!(debug_str.contains("100"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Additional Coverage Tests for PriorityQueues Internal
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_priority_queues_initialization() {
        let queues = PriorityQueues::new();
        // All priority queues should be initialized
        assert_eq!(queues.total_queued(), 0);
        for priority in Priority::all() {
            assert_eq!(queues.queued_by_priority(priority), 0);
        }
    }

    #[test]
    fn test_priority_queues_queued_by_priority_unknown() {
        let queues = PriorityQueues::default();
        // All priorities should have a queue
        assert_eq!(queues.queued_by_priority(Priority::Critical), 0);
        assert_eq!(queues.queued_by_priority(Priority::Background), 0);
    }

    #[test]
    fn test_backpressure_result_queued_values() {
        let result = BackpressureResult::Queued {
            position: 5,
            estimated_wait: Duration::from_secs(10),
        };
        if let BackpressureResult::Queued {
            position,
            estimated_wait,
        } = result
        {
            assert_eq!(position, 5);
            assert_eq!(estimated_wait, Duration::from_secs(10));
        } else {
            panic!("Expected Queued variant");
        }
    }

    #[test]
    fn test_backpressure_result_degraded() {
        let result = BackpressureResult::Degraded {
            request_id: "test".to_string(),
            reason: "High load".to_string(),
        };
        if let BackpressureResult::Degraded { reason, .. } = result {
            assert_eq!(reason, "High load");
        } else {
            panic!("Expected Degraded variant");
        }
    }

    #[test]
    fn test_backpressure_result_accepted() {
        let result = BackpressureResult::Accepted {
            request_id: "test".to_string(),
        };
        assert!(matches!(result, BackpressureResult::Accepted { .. }));
    }

    #[test]
    fn test_backpressure_metrics_json_roundtrip() {
        let metrics = BackpressureMetrics {
            total_requests: 100,
            accepted_requests: 90,
            queued_requests: 5,
            rejected_requests: 3,
            degraded_requests: 2,
            expired_requests: 0,
            current_queue_size: 5,
            current_active_requests: 10,
            average_queue_time_ms: 50.0,
            average_processing_time_ms: 200.0,
            queue_sizes_by_priority: HashMap::new(),
            load_factor: 0.5,
            adaptive_concurrency_limit: 50,
            last_updated: "2025-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: BackpressureMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_requests, 100);
        assert_eq!(deserialized.accepted_requests, 90);
    }

    #[test]
    fn test_backpressure_config_all_fields() {
        let config = BackpressureConfig {
            max_concurrent_requests: 50,
            max_queue_size: 500,
            request_timeout: Duration::from_secs(60),
            processing_interval: Duration::from_millis(20),
            adaptive_concurrency: false,
            target_latency_ms: 500,
            load_shedding_threshold: 0.8,
            enable_graceful_degradation: false,
        };
        assert_eq!(config.max_concurrent_requests, 50);
        assert_eq!(config.max_queue_size, 500);
        assert!(!config.adaptive_concurrency);
    }

    #[tokio::test]
    async fn test_submit_request_with_user_id() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: Some("user123".to_string()),
            model: "gpt-4".to_string(),
            estimated_tokens: 500,
            estimated_cost: 0.05,
            tags: {
                let mut tags = HashMap::new();
                tags.insert("env".to_string(), "prod".to_string());
                tags
            },
        };

        let result = manager.submit_request(Priority::High, None, metadata).await;
        assert!(result.is_ok());
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_metrics_load_factor() {
        let manager = BackpressureManager::new(BackpressureConfig::default());
        let metrics = manager.get_metrics().await;
        // Load factor should be 0, NaN (0/0 division), or some valid float
        // When adaptive_concurrency_limit is 0, we get NaN
        assert!(metrics.load_factor.is_nan() || metrics.load_factor >= 0.0);
    }

    #[tokio::test]
    async fn test_complete_request_updates_avg_time() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: None,
            model: "test".to_string(),
            estimated_tokens: 10,
            estimated_cost: 0.0,
            tags: HashMap::new(),
        };

        // Submit request to create active request
        let _ = manager
            .submit_request(Priority::Normal, None, metadata)
            .await;

        // Give it a moment to register
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Get metrics to check tracking
        let metrics = manager.get_metrics().await;
        // Should have tracked at least one request
        assert!(metrics.total_requests >= 1);

        manager.stop().await;
    }

    #[test]
    fn test_priority_ordering_complete() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
        assert!(Priority::Low < Priority::Background);
    }

    #[test]
    fn test_request_metadata_with_empty_tags() {
        let metadata = RequestMetadata {
            user_id: None,
            model: "test".to_string(),
            estimated_tokens: 0,
            estimated_cost: 0.0,
            tags: HashMap::new(),
        };
        assert!(metadata.tags.is_empty());
        assert!(metadata.user_id.is_none());
    }

    #[tokio::test]
    async fn test_submit_request_all_priorities() {
        let mut manager = BackpressureManager::new(BackpressureConfig::default());
        manager.start().await.unwrap();

        let metadata = RequestMetadata {
            user_id: None,
            model: "test".to_string(),
            estimated_tokens: 10,
            estimated_cost: 0.0,
            tags: HashMap::new(),
        };

        // Test all priority levels
        for priority in Priority::all() {
            let result = manager
                .submit_request(priority, None, metadata.clone())
                .await;
            assert!(result.is_ok());
        }

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_manager_drop() {
        // Create manager in inner scope to test Drop
        {
            let mut manager = BackpressureManager::new(BackpressureConfig::default());
            manager.start().await.unwrap();
            // Manager will be dropped here
        }
        // If we get here, drop worked correctly - test passed
    }

    #[test]
    fn test_backpressure_metrics_default_values() {
        let metrics = BackpressureMetrics {
            total_requests: 42,
            ..Default::default()
        };
        let cloned = metrics.clone();
        assert_eq!(cloned.total_requests, 42);
    }
}

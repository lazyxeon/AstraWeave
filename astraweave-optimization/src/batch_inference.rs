use anyhow::{anyhow, Result};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};
use uuid::Uuid;

use astraweave_llm::LlmClient;

/// Batch inference system for optimizing LLM throughput
pub struct BatchInferenceEngine {
    llm_clients: Vec<Arc<dyn LlmClient>>,
    config: BatchInferenceConfig,
    request_queue: Arc<RwLock<Vec<BatchRequest>>>,
    active_batches: Arc<RwLock<HashMap<String, ActiveBatch>>>,
    metrics: Arc<RwLock<BatchMetrics>>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

/// Configuration for batch inference
#[derive(Debug, Clone)]
pub struct BatchInferenceConfig {
    /// Maximum batch size for inference
    pub max_batch_size: usize,
    /// Minimum batch size before processing
    pub min_batch_size: usize,
    /// Maximum wait time before processing incomplete batch
    pub batch_timeout: Duration,
    /// Maximum time to wait for a single request
    pub request_timeout: Duration,
    /// Number of worker threads for processing
    pub worker_count: usize,
    /// Enable dynamic batching based on load
    pub enable_dynamic_batching: bool,
    /// Preferred batch size for optimal performance
    pub optimal_batch_size: usize,
}

impl Default for BatchInferenceConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            min_batch_size: 4,
            batch_timeout: Duration::from_millis(100),
            request_timeout: Duration::from_secs(30),
            worker_count: 4,
            enable_dynamic_batching: true,
            optimal_batch_size: 16,
        }
    }
}

/// Individual request in the batch system
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    pub id: String,
    pub prompt: String,
    pub parameters: InferenceParameters,
    pub priority: RequestPriority,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timeout_at: time::OffsetDateTime,
    #[serde(skip)]
    pub response_sender: Option<oneshot::Sender<Result<String>>>,
}

/// Parameters for inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParameters {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub repetition_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
}

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(512),
            top_p: Some(0.9),
            top_k: None,
            repetition_penalty: Some(1.1),
            stop_sequences: Vec::new(),
        }
    }
}

/// Priority levels for requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RequestPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Active batch being processed
#[derive(Debug)]
pub struct ActiveBatch {
    pub id: String,
    pub requests: Vec<BatchRequest>,
    pub started_at: time::OffsetDateTime,
    pub client_id: usize,
    pub processing: bool,
}

/// Metrics for batch inference performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetrics {
    pub total_requests: u64,
    pub completed_requests: u64,
    pub failed_requests: u64,
    pub average_batch_size: f32,
    pub average_processing_time_ms: f32,
    pub average_wait_time_ms: f32,
    pub throughput_requests_per_second: f32,
    pub queue_depth: usize,
    pub active_batches: usize,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub last_updated: time::OffsetDateTime,
}

impl Default for BatchMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            completed_requests: 0,
            failed_requests: 0,
            average_batch_size: 0.0,
            average_processing_time_ms: 0.0,
            average_wait_time_ms: 0.0,
            throughput_requests_per_second: 0.0,
            queue_depth: 0,
            active_batches: 0,
            last_updated: time::OffsetDateTime::UNIX_EPOCH,
        }
    }
}

/// Result of batch processing
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub batch_id: String,
    pub results: Vec<(String, Result<String, String>)>, // (request_id, result)
    pub processing_time_ms: u64,
    pub batch_size: usize,
    pub success_count: usize,
    pub failure_count: usize,
}

/// Adaptive batching strategy
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BatchingStrategy {
    /// Fixed batch size and timeout
    Fixed,
    /// Adapt based on current load
    LoadBased,
    /// Adapt based on request patterns
    PatternBased,
    /// Adapt based on latency requirements
    LatencyOptimized,
}

impl BatchInferenceEngine {
    pub fn new(llm_clients: Vec<Arc<dyn LlmClient>>, config: BatchInferenceConfig) -> Self {
        Self {
            llm_clients,
            config,
            request_queue: Arc::new(RwLock::new(Vec::new())),
            active_batches: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(BatchMetrics::default())),
            shutdown_tx: None,
        }
    }

    /// Start the batch processing system
    pub async fn start(&mut self) -> Result<()> {
        if self.llm_clients.is_empty() {
            return Err(anyhow!("No LLM clients configured"));
        }

        info!(
            "Starting batch inference engine with {} workers",
            self.config.worker_count
        );

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Start worker tasks
        let mut worker_handles = Vec::new();
        for worker_id in 0..self.config.worker_count {
            let worker_handle = self.spawn_worker(worker_id).await;
            worker_handles.push(worker_handle);
        }

        // Start batch scheduling task
        let scheduler_handle = self.spawn_scheduler().await;

        // Start metrics collection task
        let metrics_handle = self.spawn_metrics_collector().await;

        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping batch inference engine");
            }
            _ = futures::future::join_all(worker_handles) => {
                warn!("All workers exited unexpectedly");
            }
            _ = scheduler_handle => {
                warn!("Scheduler exited unexpectedly");
            }
            _ = metrics_handle => {
                warn!("Metrics collector exited unexpectedly");
            }
        }

        Ok(())
    }

    /// Submit a request for batch processing
    pub async fn submit_request(
        &self,
        prompt: String,
        parameters: InferenceParameters,
        priority: RequestPriority,
    ) -> Result<oneshot::Receiver<Result<String>>> {
        let (response_tx, response_rx) = oneshot::channel();

        let request = BatchRequest {
            id: Uuid::new_v4().to_string(),
            prompt,
            parameters,
            priority,
            created_at: time::OffsetDateTime::now_utc(),
            timeout_at: time::OffsetDateTime::now_utc()
                + time::Duration::try_from(self.config.request_timeout).unwrap_or_default(),
            response_sender: Some(response_tx),
        };

        // Add to queue
        {
            let mut queue = self.request_queue.write().await;
            queue.push(request);

            // Sort by priority (highest first)
            queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
            metrics.queue_depth = self.request_queue.read().await.len();
            metrics.last_updated = time::OffsetDateTime::now_utc();
        }

        debug!("Submitted request to batch queue");
        Ok(response_rx)
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> BatchMetrics {
        let mut metrics = self.metrics.write().await;
        metrics.queue_depth = self.request_queue.read().await.len();
        metrics.active_batches = self.active_batches.read().await.len();
        metrics.clone()
    }

    /// Shutdown the batch inference engine
    pub async fn shutdown(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }

    /// Spawn a worker task for processing batches
    async fn spawn_worker(&self, worker_id: usize) -> tokio::task::JoinHandle<()> {
        let request_queue = self.request_queue.clone();
        let active_batches = self.active_batches.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let llm_clients = self.llm_clients.clone();

        tokio::spawn(async move {
            info!("Started batch worker {}", worker_id);

            loop {
                // Get next batch to process
                let batch =
                    Self::get_next_batch(&request_queue, &active_batches, &config, worker_id).await;

                if let Some(batch) = batch {
                    // Process the batch
                    let result = Self::process_batch(batch, &llm_clients, worker_id).await;

                    // Update metrics and clean up
                    Self::handle_batch_result(result, &active_batches, &metrics).await;
                } else {
                    // No work available, sleep briefly
                    sleep(Duration::from_millis(10)).await;
                }
            }
        })
    }

    /// Spawn scheduler task for creating batches
    async fn spawn_scheduler(&self) -> tokio::task::JoinHandle<()> {
        let request_queue = self.request_queue.clone();
        let active_batches = self.active_batches.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            info!("Started batch scheduler");

            let mut last_schedule = time::OffsetDateTime::now_utc();

            loop {
                let now = time::OffsetDateTime::now_utc();

                // Check if we should create a new batch
                let should_schedule = {
                    let queue = request_queue.read().await;
                    let elapsed =
                        crate::utc_stamp::saturating_std(now - last_schedule, Duration::ZERO);
                    Self::should_schedule_batch(
                        queue.len(),
                        elapsed,
                        config.min_batch_size,
                        config.batch_timeout,
                    )
                };

                if should_schedule {
                    Self::schedule_batch(&request_queue, &active_batches, &config).await;
                    last_schedule = now;
                }

                // Clean up expired requests
                Self::clean_expired_requests(&request_queue, now).await;

                sleep(Duration::from_millis(10)).await;
            }
        })
    }

    /// Spawn metrics collection task
    async fn spawn_metrics_collector(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut last_completed = 0u64;
            let mut last_time = time::OffsetDateTime::now_utc();

            loop {
                sleep(Duration::from_secs(1)).await;

                let now = time::OffsetDateTime::now_utc();
                let mut m = metrics.write().await;

                // Calculate throughput
                let completed_diff = Self::completed_since(m.completed_requests, last_completed);
                let time_diff =
                    crate::utc_stamp::saturating_std(now - last_time, Duration::from_secs(1))
                        .as_secs_f32();

                if time_diff > 0.0 {
                    m.throughput_requests_per_second = completed_diff as f32 / time_diff;
                }

                last_completed = m.completed_requests;
                last_time = now;
                m.last_updated = now;
            }
        })
    }

    /// Get the next batch for a worker to process
    async fn get_next_batch(
        _request_queue: &Arc<RwLock<Vec<BatchRequest>>>,
        active_batches: &Arc<RwLock<HashMap<String, ActiveBatch>>>,
        _config: &BatchInferenceConfig,
        worker_id: usize,
    ) -> Option<ActiveBatch> {
        // Find a scheduled batch for this worker
        let mut batches = active_batches.write().await;

        let batch_id = batches
            .iter_mut()
            .find(|(_, batch)| batch.client_id == worker_id && !batch.processing)
            .map(|(id, batch)| {
                batch.processing = true;
                id.clone()
            });

        batch_id.and_then(|id| batches.remove(&id))
    }

    /// Process a batch of requests
    async fn process_batch(
        mut batch: ActiveBatch,
        llm_clients: &[Arc<dyn LlmClient>],
        worker_id: usize,
    ) -> BatchResult {
        let start_time = std::time::Instant::now();
        debug!(
            "Processing batch {} with {} requests",
            batch.id,
            batch.requests.len()
        );

        let client = &llm_clients[worker_id % llm_clients.len()];
        let mut success_count = 0;
        let mut failure_count = 0;

        // Process requests in parallel within the batch
        let processing_tasks: Vec<_> = batch
            .requests
            .iter()
            .map(|request| {
                let client = client.clone();
                let request_id = request.id.clone();
                let prompt = request.prompt.clone();

                async move {
                    match client.complete(&prompt).await {
                        Ok(response) => {
                            debug!("Request {} completed successfully", request_id);
                            (request_id, Ok(response))
                        }
                        Err(e) => {
                            warn!("Request {} failed: {}", request_id, e);
                            (request_id, Err(anyhow!("LLM request failed: {}", e)))
                        }
                    }
                }
            })
            .collect();

        let results = join_all(processing_tasks).await;

        // Send results back to requesters
        let results_stringified: Vec<(String, Result<String, String>)> = results
            .into_iter()
            .map(|(id, r)| (id, r.map_err(|e| format!("{}", e))))
            .collect();
        for (i, (_request_id, result)) in results_stringified.iter().enumerate() {
            if let Some(request) = batch.requests.get_mut(i) {
                if let Some(sender) = request.response_sender.take() {
                    let sendable = match result {
                        Ok(s) => Ok(s.clone()),
                        Err(e) => Err(anyhow!("{}", e)),
                    };
                    let _ = sender.send(sendable);
                }
            }

            match result {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            }
        }
        let results = results_stringified;

        let processing_time = start_time.elapsed().as_millis() as u64;

        BatchResult {
            batch_id: batch.id,
            results,
            processing_time_ms: processing_time,
            batch_size: batch.requests.len(),
            success_count,
            failure_count,
        }
    }

    /// Handle batch processing result
    async fn handle_batch_result(
        result: BatchResult,
        active_batches: &Arc<RwLock<HashMap<String, ActiveBatch>>>,
        metrics: &Arc<RwLock<BatchMetrics>>,
    ) {
        // Remove from active batches
        {
            let mut batches = active_batches.write().await;
            batches.remove(&result.batch_id);
        }

        // Update metrics
        {
            let mut m = metrics.write().await;
            m.completed_requests += result.success_count as u64;
            m.failed_requests += result.failure_count as u64;

            // Update moving averages
            let total_batches =
                (m.completed_requests + m.failed_requests) / m.average_batch_size.max(1.0) as u64;
            if total_batches > 0 {
                let weight = 1.0 / total_batches as f32;
                m.average_batch_size =
                    m.average_batch_size * (1.0 - weight) + result.batch_size as f32 * weight;
                m.average_processing_time_ms = m.average_processing_time_ms * (1.0 - weight)
                    + result.processing_time_ms as f32 * weight;
            }
        }

        info!(
            "Completed batch {} with {}/{} successful requests in {}ms",
            result.batch_id, result.success_count, result.batch_size, result.processing_time_ms
        );
    }

    fn should_schedule_batch(
        queue_len: usize,
        elapsed: Duration,
        min_batch_size: usize,
        batch_timeout: Duration,
    ) -> bool {
        queue_len >= min_batch_size || (queue_len > 0 && elapsed >= batch_timeout)
    }

    fn completed_since(total: u64, previous: u64) -> u64 {
        total - previous
    }

    /// Schedule a new batch from the request queue
    async fn schedule_batch(
        request_queue: &Arc<RwLock<Vec<BatchRequest>>>,
        active_batches: &Arc<RwLock<HashMap<String, ActiveBatch>>>,
        config: &BatchInferenceConfig,
    ) {
        let mut queue = request_queue.write().await;

        if queue.is_empty() {
            return;
        }

        // Determine batch size
        let batch_size = if config.enable_dynamic_batching {
            Self::calculate_dynamic_batch_size(&queue, config)
        } else {
            config.max_batch_size.min(queue.len())
        };

        // Take requests for the batch
        let drain_end = batch_size.min(queue.len());
        let batch_requests = queue.drain(..drain_end).collect();

        let batch = ActiveBatch {
            id: Uuid::new_v4().to_string(),
            requests: batch_requests,
            started_at: time::OffsetDateTime::now_utc(),
            client_id: 0, // Will be assigned by worker
            processing: false,
        };

        // Add to active batches
        {
            let mut batches = active_batches.write().await;
            batches.insert(batch.id.clone(), batch);
        }

        debug!("Scheduled new batch with {} requests", batch_size);
    }

    /// Calculate optimal batch size dynamically
    fn calculate_dynamic_batch_size(
        queue: &[BatchRequest],
        config: &BatchInferenceConfig,
    ) -> usize {
        let queue_size = queue.len();

        // Check urgency based on request timeouts
        let urgent_count = queue
            .iter()
            .filter(|req| (req.timeout_at - time::OffsetDateTime::now_utc()).whole_seconds() < 5)
            .count();

        if urgent_count > 0 {
            // Process urgent requests in smaller batches for lower latency
            urgent_count.min(config.min_batch_size)
        } else if queue_size >= config.optimal_batch_size {
            // Use optimal batch size for best throughput
            config.optimal_batch_size
        } else {
            // Use whatever we have, up to max
            queue_size.min(config.max_batch_size)
        }
    }

    /// Clean up expired requests from the queue
    async fn clean_expired_requests(
        request_queue: &Arc<RwLock<Vec<BatchRequest>>>,
        current_time: time::OffsetDateTime,
    ) {
        let mut queue = request_queue.write().await;
        let initial_len = queue.len();

        // Send timeout errors to expired requests
        let mut i = 0;
        while i < queue.len() {
            if queue[i].timeout_at <= current_time {
                let expired_request = queue.remove(i);
                if let Some(sender) = expired_request.response_sender {
                    let _ = sender.send(Err(anyhow!("Request timed out")));
                }
                warn!("Request {} timed out", expired_request.id);
            } else {
                i += 1;
            }
        }

        let expired_count = initial_len - queue.len();
        if expired_count > 0 {
            debug!("Cleaned up {} expired requests", expired_count);
        }
    }
}

/// Convenience function for simple batch inference
pub async fn batch_inference(
    llm_client: Arc<dyn LlmClient>,
    prompts: Vec<String>,
    config: BatchInferenceConfig,
) -> Result<Vec<Result<String>>> {
    let mut engine = BatchInferenceEngine::new(vec![llm_client], config);

    // Submit all requests
    let mut receivers = Vec::new();
    for prompt in prompts {
        let receiver = engine
            .submit_request(
                prompt,
                InferenceParameters::default(),
                RequestPriority::Normal,
            )
            .await?;
        receivers.push(receiver);
    }

    // Start processing (this would normally run in background)
    tokio::spawn(async move {
        let _ = engine.start().await;
    });

    // Wait for all results
    let mut results = Vec::new();
    for receiver in receivers {
        match timeout(Duration::from_secs(30), receiver).await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(_)) => results.push(Err(anyhow!("Request sender dropped"))),
            Err(_) => results.push(Err(anyhow!("Request timed out"))),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_llm::MockLlm;

    #[tokio::test]
    async fn test_batch_inference_engine_creation() {
        let llm_client = Arc::new(MockLlm);
        let config = BatchInferenceConfig::default();
        let engine = BatchInferenceEngine::new(vec![llm_client], config);

        assert_eq!(engine.llm_clients.len(), 1);
    }

    #[tokio::test]
    async fn test_request_submission() {
        let llm_client = Arc::new(MockLlm);
        let config = BatchInferenceConfig::default();
        let engine = BatchInferenceEngine::new(vec![llm_client], config);

        let _receiver = engine
            .submit_request(
                "Test prompt".to_string(),
                InferenceParameters::default(),
                RequestPriority::Normal,
            )
            .await
            .unwrap();

        // Check that request was queued
        let queue_len = engine.request_queue.read().await.len();
        assert_eq!(queue_len, 1);
    }

    #[tokio::test]
    async fn test_batch_metrics() {
        let llm_client = Arc::new(MockLlm);
        let config = BatchInferenceConfig::default();
        let engine = BatchInferenceEngine::new(vec![llm_client], config);

        let metrics = engine.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.queue_depth, 0);
    }

    #[test]
    fn test_dynamic_batch_sizing() {
        let config = BatchInferenceConfig::default();
        let queue = vec![BatchRequest {
            id: "1".to_string(),
            prompt: "Test".to_string(),
            parameters: InferenceParameters::default(),
            priority: RequestPriority::Normal,
            created_at: time::OffsetDateTime::now_utc(),
            timeout_at: time::OffsetDateTime::now_utc() + time::Duration::seconds(30),
            response_sender: None,
        }];

        let batch_size = BatchInferenceEngine::calculate_dynamic_batch_size(&queue, &config);
        assert!(batch_size > 0);
        assert!(batch_size <= config.max_batch_size);
    }

    fn request_due(seconds_left: i64) -> BatchRequest {
        let now = time::OffsetDateTime::now_utc();
        BatchRequest {
            id: format!("req-{seconds_left}"),
            prompt: "p".into(),
            parameters: InferenceParameters::default(),
            priority: RequestPriority::Normal,
            created_at: now,
            timeout_at: now
                + time::Duration::seconds(seconds_left)
                + time::Duration::milliseconds(200),
            response_sender: None,
        }
    }

    #[test]
    fn schedule_predicate_matches_queue_length_and_timeout() {
        let min = 4;
        let timeout = Duration::from_millis(100);
        let due = |len, elapsed_ms| {
            BatchInferenceEngine::should_schedule_batch(
                len,
                Duration::from_millis(elapsed_ms),
                min,
                timeout,
            )
        };
        assert!(due(4, 0));
        assert!(due(5, 0));
        assert!(!due(3, 0));
        assert!(!due(0, 500));
        assert!(due(1, 100));
        assert!(due(1, 101));
        assert!(!due(1, 99));
    }

    #[test]
    fn completed_since_is_a_difference() {
        assert_eq!(BatchInferenceEngine::completed_since(10, 4), 6);
        assert_eq!(BatchInferenceEngine::completed_since(4, 4), 0);
    }

    #[tokio::test]
    async fn submit_request_sets_a_future_timeout_and_schedule_batch_drains_the_queue() {
        let engine =
            BatchInferenceEngine::new(vec![Arc::new(MockLlm)], BatchInferenceConfig::default());
        let before = time::OffsetDateTime::now_utc();
        engine
            .submit_request(
                "p".into(),
                InferenceParameters::default(),
                RequestPriority::High,
            )
            .await
            .unwrap();
        let queued = engine.request_queue.read().await;
        assert!(queued[0].timeout_at > queued[0].created_at);
        assert!(queued[0].timeout_at >= before + time::Duration::seconds(29));
        drop(queued);
        let metrics = engine.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);

        let config = BatchInferenceConfig {
            enable_dynamic_batching: false,
            max_batch_size: 2,
            ..BatchInferenceConfig::default()
        };
        let queue = Arc::new(RwLock::new(vec![request_due(30), request_due(30)]));
        let batches = Arc::new(RwLock::new(HashMap::new()));
        BatchInferenceEngine::schedule_batch(&queue, &batches, &config).await;
        assert!(queue.read().await.is_empty());
        assert_eq!(batches.read().await.len(), 1);
        assert_eq!(
            batches.read().await.values().next().unwrap().requests.len(),
            2
        );
    }

    #[test]
    fn dynamic_batch_size_treats_under_five_seconds_as_urgent() {
        let config = BatchInferenceConfig {
            min_batch_size: 1,
            ..BatchInferenceConfig::default()
        };
        let urgent = vec![request_due(4), request_due(4), request_due(4)];
        assert_eq!(
            BatchInferenceEngine::calculate_dynamic_batch_size(&urgent, &config),
            1
        );
        let on_boundary = vec![request_due(5), request_due(5), request_due(5)];
        assert_eq!(
            BatchInferenceEngine::calculate_dynamic_batch_size(&on_boundary, &config),
            3
        );
        let comfortable = vec![request_due(6), request_due(6), request_due(6)];
        assert_eq!(
            BatchInferenceEngine::calculate_dynamic_batch_size(&comfortable, &config),
            3
        );
        let full: Vec<_> = (0..20).map(|_| request_due(30)).collect();
        assert_eq!(
            BatchInferenceEngine::calculate_dynamic_batch_size(&full, &config),
            config.optimal_batch_size
        );
    }
}

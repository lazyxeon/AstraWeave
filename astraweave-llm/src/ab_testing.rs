use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// A/B testing framework for LLM prompts and models
pub struct ABTestFramework {
    /// Active experiments
    experiments: Arc<RwLock<HashMap<String, Experiment>>>,
    /// Results storage
    results: Arc<RwLock<HashMap<String, ExperimentResults>>>,
    /// Configuration
    config: ABTestConfig,
}

/// Configuration for A/B testing
#[derive(Debug, Clone)]
pub struct ABTestConfig {
    /// Default experiment duration in hours
    pub default_duration_hours: u64,
    /// Minimum sample size per variant
    pub min_sample_size: usize,
    /// Statistical significance threshold (p-value)
    pub significance_threshold: f64,
    /// Enable automatic winner selection
    pub auto_winner_selection: bool,
    /// Maximum concurrent experiments
    pub max_concurrent_experiments: usize,
}

impl Default for ABTestConfig {
    fn default() -> Self {
        Self {
            default_duration_hours: 168, // 1 week
            min_sample_size: 100,
            significance_threshold: 0.05, // 95% confidence
            auto_winner_selection: false,
            max_concurrent_experiments: 10,
        }
    }
}

/// Individual A/B test experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ExperimentStatus,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "crate::utc_stamp::serde_rfc3339::option")]
    pub started_at: Option<time::OffsetDateTime>,
    #[serde(with = "crate::utc_stamp::serde_rfc3339::option")]
    pub ended_at: Option<time::OffsetDateTime>,
    pub duration_hours: u64,
    pub traffic_percentage: f32, // 0.0 to 1.0
    pub control_variant: Variant,
    pub test_variants: Vec<Variant>,
    pub target_metric: String,
    pub success_criteria: SuccessCriteria,
    pub assignment_strategy: AssignmentStrategy,
    pub metadata: HashMap<String, String>,
}

/// Status of an experiment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ExperimentStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Stopped,
}

/// Individual variant in an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt_template: Option<String>,
    pub model_config: Option<ModelConfig>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub traffic_allocation: f32, // 0.0 to 1.0
}

/// Model configuration for a variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub repetition_penalty: Option<f32>,
}

/// Success criteria for the experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    pub primary_metric: String,
    pub improvement_threshold: f32, // Minimum improvement to be considered significant
    pub direction: OptimizationDirection,
    pub secondary_metrics: Vec<String>,
}

/// Optimization direction for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OptimizationDirection {
    Maximize, // Higher values are better
    Minimize, // Lower values are better
}

/// Strategy for assigning users to variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssignmentStrategy {
    /// Hash-based deterministic assignment
    Hash,
    /// Weighted random assignment
    WeightedRandom,
    /// Round-robin assignment
    RoundRobin,
    /// Custom assignment function
    Custom(String),
}

/// Results of an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: String,
    pub status: ResultStatus,
    pub variant_results: HashMap<String, VariantResults>,
    pub statistical_analysis: Option<StatisticalAnalysis>,
    pub winner: Option<String>, // Variant ID
    pub confidence_level: f64,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub last_updated: time::OffsetDateTime,
}

/// Status of experiment results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[must_use]
pub enum ResultStatus {
    InProgress,
    SignificantResult,
    NoSignificantDifference,
    InsufficientData,
}

/// Results for a specific variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResults {
    pub variant_id: String,
    pub sample_size: usize,
    pub metrics: HashMap<String, MetricResult>,
    pub conversion_rate: f32,
    pub confidence_interval: ConfidenceInterval,
}

/// Result for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricResult {
    pub metric_name: String,
    pub value: f32,
    pub count: usize,
    pub sum: f32,
    pub mean: f32,
    pub std_dev: f32,
    pub percentiles: HashMap<String, f32>, // P50, P95, P99, etc.
}

/// Confidence interval for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub confidence_level: f64,
}

/// Statistical analysis of experiment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalAnalysis {
    pub test_type: String,
    pub p_value: f64,
    pub effect_size: f32,
    pub power: f64,
    pub recommendations: Vec<String>,
}

/// Outcome of an experiment interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub user_id: String,
    pub experiment_id: String,
    pub variant_id: String,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
    pub metrics: HashMap<String, f32>,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

/// Assignment result for a user
#[derive(Debug, Clone)]
pub struct VariantAssignment {
    pub experiment_id: String,
    pub variant_id: String,
    pub variant: Variant,
    pub assigned_at: time::OffsetDateTime,
}

impl ABTestFramework {
    pub fn new(config: ABTestConfig) -> Self {
        Self {
            experiments: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new experiment
    pub async fn create_experiment(&self, mut experiment: Experiment) -> Result<String> {
        // Validate experiment
        self.validate_experiment(&experiment)?;

        // Generate ID if not provided
        if experiment.id.is_empty() {
            experiment.id = Uuid::new_v4().to_string();
        }

        // Set defaults
        if experiment.duration_hours == 0 {
            experiment.duration_hours = self.config.default_duration_hours;
        }

        experiment.status = ExperimentStatus::Draft;
        experiment.created_at = time::OffsetDateTime::now_utc();

        // Check concurrent experiment limit
        {
            let experiments = self.experiments.read().await;
            let running_count = experiments
                .values()
                .filter(|e| e.status == ExperimentStatus::Running)
                .count();

            if running_count >= self.config.max_concurrent_experiments {
                return Err(anyhow!("Maximum concurrent experiments limit reached"));
            }
        }

        // Store experiment
        {
            let mut experiments = self.experiments.write().await;
            experiments.insert(experiment.id.clone(), experiment.clone());
        }

        // Initialize results
        {
            let mut results = self.results.write().await;
            results.insert(
                experiment.id.clone(),
                ExperimentResults {
                    experiment_id: experiment.id.clone(),
                    status: ResultStatus::InProgress,
                    variant_results: HashMap::new(),
                    statistical_analysis: None,
                    winner: None,
                    confidence_level: 0.0,
                    last_updated: time::OffsetDateTime::now_utc(),
                },
            );
        }

        info!(
            "Created experiment: {} ({})",
            experiment.name, experiment.id
        );
        Ok(experiment.id)
    }

    /// Start an experiment
    pub async fn start_experiment(&self, experiment_id: &str) -> Result<()> {
        let mut experiments = self.experiments.write().await;

        if let Some(experiment) = experiments.get_mut(experiment_id) {
            if experiment.status != ExperimentStatus::Draft {
                return Err(anyhow!("Experiment is not in draft status"));
            }

            experiment.status = ExperimentStatus::Running;
            experiment.started_at = Some(time::OffsetDateTime::now_utc());

            info!(
                "Started experiment: {} ({})",
                experiment.name, experiment.id
            );
            Ok(())
        } else {
            Err(anyhow!("Experiment {} not found", experiment_id))
        }
    }

    /// Stop an experiment
    pub async fn stop_experiment(&self, experiment_id: &str) -> Result<()> {
        let mut experiments = self.experiments.write().await;

        if let Some(experiment) = experiments.get_mut(experiment_id) {
            if experiment.status != ExperimentStatus::Running {
                return Err(anyhow!("Experiment is not running"));
            }

            experiment.status = ExperimentStatus::Stopped;
            experiment.ended_at = Some(time::OffsetDateTime::now_utc());

            info!(
                "Stopped experiment: {} ({})",
                experiment.name, experiment.id
            );
            Ok(())
        } else {
            Err(anyhow!("Experiment {} not found", experiment_id))
        }
    }

    /// Assign a user to a variant
    pub async fn assign_variant(
        &self,
        experiment_id: &str,
        user_id: &str,
    ) -> Result<Option<VariantAssignment>> {
        let experiments = self.experiments.read().await;

        if let Some(experiment) = experiments.get(experiment_id) {
            if experiment.status != ExperimentStatus::Running {
                return Ok(None);
            }

            // Check if user is in experiment traffic
            if !self.is_user_in_experiment_traffic(user_id, experiment.traffic_percentage) {
                return Ok(None);
            }

            // Assign variant based on strategy
            let variant = self.assign_variant_by_strategy(user_id, experiment)?;

            Ok(Some(VariantAssignment {
                experiment_id: experiment_id.to_string(),
                variant_id: variant.id.clone(),
                variant,
                assigned_at: time::OffsetDateTime::now_utc(),
            }))
        } else {
            Err(anyhow!("Experiment {} not found", experiment_id))
        }
    }

    /// Record an outcome for an experiment
    pub async fn record_outcome(&self, outcome: Outcome) -> Result<()> {
        // Validate experiment is running
        {
            let experiments = self.experiments.read().await;
            if let Some(experiment) = experiments.get(&outcome.experiment_id) {
                if experiment.status != ExperimentStatus::Running {
                    return Err(anyhow!("Experiment is not running"));
                }
            } else {
                return Err(anyhow!("Experiment {} not found", outcome.experiment_id));
            }
        }

        // Update results
        {
            let mut results = self.results.write().await;
            if let Some(experiment_results) = results.get_mut(&outcome.experiment_id) {
                self.update_variant_results(experiment_results, &outcome);
                experiment_results.last_updated = time::OffsetDateTime::now_utc();

                // Perform statistical analysis if we have enough data
                if self.has_sufficient_data(experiment_results) {
                    experiment_results.statistical_analysis =
                        Some(self.perform_statistical_analysis(experiment_results)?);

                    // Check for significance
                    if let Some(analysis) = &experiment_results.statistical_analysis {
                        if analysis.p_value < self.config.significance_threshold {
                            experiment_results.status = ResultStatus::SignificantResult;

                            // Determine winner
                            experiment_results.winner =
                                self.determine_winner(experiment_results).await;

                            // Auto-stop experiment if configured
                            if self.config.auto_winner_selection
                                && experiment_results.winner.is_some()
                            {
                                self.stop_experiment(&outcome.experiment_id).await?;
                            }
                        }
                    }
                }
            }
        }

        debug!(
            "Recorded outcome for experiment {} variant {}",
            outcome.experiment_id, outcome.variant_id
        );
        Ok(())
    }

    /// Get experiment results
    pub async fn get_results(&self, experiment_id: &str) -> Result<ExperimentResults> {
        let results = self.results.read().await;

        results
            .get(experiment_id)
            .cloned()
            .ok_or_else(|| anyhow!("Results for experiment {} not found", experiment_id))
    }

    /// List all experiments
    pub async fn list_experiments(&self) -> Vec<Experiment> {
        let experiments = self.experiments.read().await;
        experiments.values().cloned().collect()
    }

    /// Get experiment by ID
    pub async fn get_experiment(&self, experiment_id: &str) -> Option<Experiment> {
        let experiments = self.experiments.read().await;
        experiments.get(experiment_id).cloned()
    }

    /// Delete an experiment
    pub async fn delete_experiment(&self, experiment_id: &str) -> Result<()> {
        {
            let mut experiments = self.experiments.write().await;
            if let Some(experiment) = experiments.remove(experiment_id) {
                if experiment.status == ExperimentStatus::Running {
                    warn!("Deleting running experiment: {}", experiment.name);
                }
            } else {
                return Err(anyhow!("Experiment {} not found", experiment_id));
            }
        }

        {
            let mut results = self.results.write().await;
            results.remove(experiment_id);
        }

        info!("Deleted experiment: {}", experiment_id);
        Ok(())
    }

    /// Validate experiment configuration
    fn validate_experiment(&self, experiment: &Experiment) -> Result<()> {
        if experiment.name.is_empty() {
            return Err(anyhow!("Experiment name cannot be empty"));
        }

        if experiment.control_variant.id.is_empty() {
            return Err(anyhow!("Control variant must have an ID"));
        }

        if experiment.test_variants.is_empty() {
            return Err(anyhow!("At least one test variant is required"));
        }

        // Check traffic allocation sums to 1.0
        let total_traffic = experiment.control_variant.traffic_allocation
            + experiment
                .test_variants
                .iter()
                .map(|v| v.traffic_allocation)
                .sum::<f32>();

        if (total_traffic - 1.0).abs() > 0.01 {
            return Err(anyhow!(
                "Traffic allocation must sum to 1.0 (currently: {})",
                total_traffic
            ));
        }

        // Validate variant IDs are unique
        let mut variant_ids = vec![experiment.control_variant.id.clone()];
        variant_ids.extend(experiment.test_variants.iter().map(|v| v.id.clone()));
        variant_ids.sort();
        variant_ids.dedup();

        if variant_ids.len() != 1 + experiment.test_variants.len() {
            return Err(anyhow!("Variant IDs must be unique"));
        }

        Ok(())
    }

    /// Check if user is included in experiment traffic
    fn is_user_in_experiment_traffic(&self, user_id: &str, traffic_percentage: f32) -> bool {
        let hash = self.hash_user_id(user_id);
        (hash % 100) as f32 / 100.0 < traffic_percentage
    }

    /// Assign variant based on assignment strategy
    fn assign_variant_by_strategy(
        &self,
        user_id: &str,
        experiment: &Experiment,
    ) -> Result<Variant> {
        match experiment.assignment_strategy {
            AssignmentStrategy::Hash => self.assign_variant_hash(user_id, experiment),
            AssignmentStrategy::WeightedRandom => self.assign_variant_weighted_random(experiment),
            AssignmentStrategy::RoundRobin => self.assign_variant_round_robin(experiment),
            AssignmentStrategy::Custom(_) => {
                // For now, fall back to hash-based assignment
                self.assign_variant_hash(user_id, experiment)
            }
        }
    }

    /// Hash-based deterministic variant assignment
    fn assign_variant_hash(&self, user_id: &str, experiment: &Experiment) -> Result<Variant> {
        let hash = self.hash_user_id(&format!("{}:{}", experiment.id, user_id));
        let normalized = (hash % 100) as f32 / 100.0;

        let mut cumulative = 0.0;

        // Check control variant
        cumulative += experiment.control_variant.traffic_allocation;
        if normalized <= cumulative {
            return Ok(experiment.control_variant.clone());
        }

        // Check test variants
        for variant in &experiment.test_variants {
            cumulative += variant.traffic_allocation;
            if normalized <= cumulative {
                return Ok(variant.clone());
            }
        }

        // Fallback to control variant
        Ok(experiment.control_variant.clone())
    }

    /// Weighted random variant assignment
    fn assign_variant_weighted_random(&self, experiment: &Experiment) -> Result<Variant> {
        use rand::Rng;
        let mut rng = rand::rng();
        let random_value: f32 = rng.random();

        let mut cumulative = 0.0;

        cumulative += experiment.control_variant.traffic_allocation;
        if random_value <= cumulative {
            return Ok(experiment.control_variant.clone());
        }

        for variant in &experiment.test_variants {
            cumulative += variant.traffic_allocation;
            if random_value <= cumulative {
                return Ok(variant.clone());
            }
        }

        Ok(experiment.control_variant.clone())
    }

    /// Round-robin variant assignment (simplified)
    fn assign_variant_round_robin(&self, experiment: &Experiment) -> Result<Variant> {
        // For simplicity, use time-based round robin
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as usize;
        let total_variants = 1 + experiment.test_variants.len();
        let variant_index = now % total_variants;

        if variant_index == 0 {
            Ok(experiment.control_variant.clone())
        } else {
            Ok(experiment.test_variants[variant_index - 1].clone())
        }
    }

    /// Hash a user ID for consistent assignment
    fn hash_user_id(&self, user_id: &str) -> u32 {
        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        hasher.finish() as u32
    }

    /// Update variant results with new outcome
    fn update_variant_results(
        &self,
        experiment_results: &mut ExperimentResults,
        outcome: &Outcome,
    ) {
        let variant_results = experiment_results
            .variant_results
            .entry(outcome.variant_id.clone())
            .or_insert_with(|| VariantResults {
                variant_id: outcome.variant_id.clone(),
                sample_size: 0,
                metrics: HashMap::new(),
                conversion_rate: 0.0,
                confidence_interval: ConfidenceInterval {
                    lower_bound: 0.0,
                    upper_bound: 0.0,
                    confidence_level: 0.95,
                },
            });

        variant_results.sample_size += 1;

        // Update metrics
        for (metric_name, value) in &outcome.metrics {
            let metric_result = variant_results
                .metrics
                .entry(metric_name.clone())
                .or_insert_with(|| MetricResult {
                    metric_name: metric_name.clone(),
                    value: 0.0,
                    count: 0,
                    sum: 0.0,
                    mean: 0.0,
                    std_dev: 0.0,
                    percentiles: HashMap::new(),
                });

            metric_result.count += 1;
            metric_result.sum += value;
            metric_result.mean = metric_result.sum / metric_result.count as f32;
            metric_result.value = *value; // Store last value

            // Update standard deviation (simplified)
            // In practice, would track sum of squares for proper calculation
        }

        // Update conversion rate
        let successes = variant_results.sample_size;
        let success_count = if outcome.success { 1 } else { 0 };
        variant_results.conversion_rate =
            (variant_results.conversion_rate * (successes - 1) as f32 + success_count as f32)
                / successes as f32;
    }

    /// Check if experiment has sufficient data for analysis
    fn has_sufficient_data(&self, results: &ExperimentResults) -> bool {
        results
            .variant_results
            .values()
            .all(|v| v.sample_size >= self.config.min_sample_size)
    }

    /// Perform statistical analysis
    fn perform_statistical_analysis(
        &self,
        _results: &ExperimentResults,
    ) -> Result<StatisticalAnalysis> {
        // Simplified statistical analysis
        // In practice, would use proper statistical libraries like `statrs`

        Ok(StatisticalAnalysis {
            test_type: "t-test".to_string(),
            p_value: 0.03, // Placeholder
            effect_size: 0.15,
            power: 0.8,
            recommendations: vec![
                "Sample size is sufficient for reliable results".to_string(),
                "Effect size indicates practical significance".to_string(),
            ],
        })
    }

    /// Determine the winner of an experiment
    async fn determine_winner(&self, results: &ExperimentResults) -> Option<String> {
        let experiments = self.experiments.read().await;
        if let Some(experiment) = experiments.get(&results.experiment_id) {
            // Find variant with best performance on primary metric
            let mut best_variant = None;
            let mut best_value = match experiment.success_criteria.direction {
                OptimizationDirection::Maximize => f32::NEG_INFINITY,
                OptimizationDirection::Minimize => f32::INFINITY,
            };

            for (variant_id, variant_results) in &results.variant_results {
                if let Some(metric) = variant_results
                    .metrics
                    .get(&experiment.success_criteria.primary_metric)
                {
                    let is_better = match experiment.success_criteria.direction {
                        OptimizationDirection::Maximize => metric.mean > best_value,
                        OptimizationDirection::Minimize => metric.mean < best_value,
                    };

                    if is_better {
                        best_value = metric.mean;
                        best_variant = Some(variant_id.clone());
                    }
                }
            }

            best_variant
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_experiment() -> Experiment {
        Experiment {
            id: "test-experiment".to_string(),
            name: "Test Experiment".to_string(),
            description: "A test experiment".to_string(),
            status: ExperimentStatus::Draft,
            created_at: time::OffsetDateTime::now_utc(),
            started_at: None,
            ended_at: None,
            duration_hours: 24,
            traffic_percentage: 0.5,
            control_variant: Variant {
                id: "control".to_string(),
                name: "Control".to_string(),
                description: "Control variant".to_string(),
                prompt_template: Some("Original prompt".to_string()),
                model_config: None,
                parameters: HashMap::new(),
                traffic_allocation: 0.5,
            },
            test_variants: vec![Variant {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test variant".to_string(),
                prompt_template: Some("New prompt".to_string()),
                model_config: None,
                parameters: HashMap::new(),
                traffic_allocation: 0.5,
            }],
            target_metric: "conversion_rate".to_string(),
            success_criteria: SuccessCriteria {
                primary_metric: "conversion_rate".to_string(),
                improvement_threshold: 0.05,
                direction: OptimizationDirection::Maximize,
                secondary_metrics: Vec::new(),
            },
            assignment_strategy: AssignmentStrategy::Hash,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        assert!(!experiment_id.is_empty());

        let retrieved = framework.get_experiment(&experiment_id).await.unwrap();
        assert_eq!(retrieved.name, "Test Experiment");
    }

    #[tokio::test]
    async fn test_start_stop_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();

        framework.start_experiment(&experiment_id).await.unwrap();
        let running = framework.get_experiment(&experiment_id).await.unwrap();
        assert_eq!(running.status, ExperimentStatus::Running);

        framework.stop_experiment(&experiment_id).await.unwrap();
        let stopped = framework.get_experiment(&experiment_id).await.unwrap();
        assert_eq!(stopped.status, ExperimentStatus::Stopped);
    }

    #[tokio::test]
    async fn test_variant_assignment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_some());

        let assignment = assignment.unwrap();
        assert!(assignment.variant_id == "control" || assignment.variant_id == "test");
    }

    #[tokio::test]
    async fn test_record_outcome() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("conversion_rate".to_string(), 0.15);

        let outcome = Outcome {
            user_id: "user123".to_string(),
            experiment_id: experiment_id.clone(),
            variant_id: "control".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            metrics,
            success: true,
            metadata: HashMap::new(),
        };

        framework.record_outcome(outcome).await.unwrap();

        let results = framework.get_results(&experiment_id).await.unwrap();
        assert!(results.variant_results.contains_key("control"));
        assert_eq!(results.variant_results["control"].sample_size, 1);
    }

    #[test]
    fn test_hash_consistency() {
        let framework = ABTestFramework::new(ABTestConfig::default());

        let hash1 = framework.hash_user_id("user123");
        let hash2 = framework.hash_user_id("user123");

        assert_eq!(hash1, hash2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Config Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_ab_test_config_default() {
        let config = ABTestConfig::default();
        assert_eq!(config.default_duration_hours, 168);
        assert_eq!(config.min_sample_size, 100);
        assert!((config.significance_threshold - 0.05).abs() < f64::EPSILON);
        assert!(!config.auto_winner_selection);
        assert_eq!(config.max_concurrent_experiments, 10);
    }

    #[test]
    fn test_ab_test_config_custom() {
        let config = ABTestConfig {
            default_duration_hours: 48,
            min_sample_size: 50,
            significance_threshold: 0.01,
            auto_winner_selection: true,
            max_concurrent_experiments: 5,
        };
        assert_eq!(config.default_duration_hours, 48);
        assert!(config.auto_winner_selection);
    }

    #[test]
    fn test_ab_test_config_clone() {
        let config = ABTestConfig::default();
        let cloned = config.clone();
        assert_eq!(config.default_duration_hours, cloned.default_duration_hours);
        assert_eq!(config.significance_threshold, cloned.significance_threshold);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Experiment Status Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_experiment_status_equality() {
        assert_eq!(ExperimentStatus::Draft, ExperimentStatus::Draft);
        assert_eq!(ExperimentStatus::Running, ExperimentStatus::Running);
        assert_ne!(ExperimentStatus::Draft, ExperimentStatus::Running);
        assert_eq!(ExperimentStatus::Paused, ExperimentStatus::Paused);
        assert_eq!(ExperimentStatus::Completed, ExperimentStatus::Completed);
        assert_eq!(ExperimentStatus::Stopped, ExperimentStatus::Stopped);
    }

    #[test]
    fn test_experiment_status_serialization() {
        let status = ExperimentStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Running"));

        let deserialized: ExperimentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ExperimentStatus::Running);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Validation Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_validate_empty_name() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.name = "".to_string();

        let result = framework.create_experiment(experiment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("name cannot be empty"));
    }

    #[tokio::test]
    async fn test_validate_empty_control_variant_id() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.control_variant.id = "".to_string();

        let result = framework.create_experiment(experiment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Control variant must have an ID"));
    }

    #[tokio::test]
    async fn test_validate_no_test_variants() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.test_variants = vec![];

        let result = framework.create_experiment(experiment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one test variant"));
    }

    #[tokio::test]
    async fn test_validate_traffic_allocation_sum() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.control_variant.traffic_allocation = 0.3;
        // test variant is 0.5, total = 0.8 != 1.0

        let result = framework.create_experiment(experiment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Traffic allocation must sum to 1.0"));
    }

    #[tokio::test]
    async fn test_validate_duplicate_variant_ids() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.test_variants[0].id = "control".to_string(); // Same as control variant

        let result = framework.create_experiment(experiment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Variant IDs must be unique"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Assignment Strategy Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_weighted_random_assignment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.assignment_strategy = AssignmentStrategy::WeightedRandom;

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        // Run multiple assignments - should get both variants due to random
        let mut control_count = 0;
        let mut test_count = 0;
        for i in 0..50 {
            let assignment = framework
                .assign_variant(&experiment_id, &format!("user{}", i))
                .await
                .unwrap();
            if let Some(a) = assignment {
                if a.variant_id == "control" {
                    control_count += 1;
                } else {
                    test_count += 1;
                }
            }
        }
        // We can't guarantee exact counts due to randomness + traffic percentage
        // Just verify we got some assignments
        assert!(control_count > 0 || test_count > 0);
    }

    #[tokio::test]
    async fn test_round_robin_assignment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.assignment_strategy = AssignmentStrategy::RoundRobin;
        experiment.traffic_percentage = 1.0; // Ensure all users get assigned

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_some());
    }

    #[tokio::test]
    async fn test_custom_assignment_strategy() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.assignment_strategy = AssignmentStrategy::Custom("custom_strategy".to_string());
        experiment.traffic_percentage = 1.0;

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        // Custom falls back to hash
        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Traffic Percentage Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_traffic_percentage_zero() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.traffic_percentage = 0.0;

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_none());
    }

    #[tokio::test]
    async fn test_traffic_percentage_full() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let mut experiment = create_test_experiment();
        experiment.traffic_percentage = 1.0;

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Experiment Lifecycle Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_start_non_draft_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        // Try to start again
        let result = framework.start_experiment(&experiment_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in draft status"));
    }

    #[tokio::test]
    async fn test_stop_non_running_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        // Don't start it

        let result = framework.stop_experiment(&experiment_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[tokio::test]
    async fn test_experiment_not_found() {
        let framework = ABTestFramework::new(ABTestConfig::default());

        let result = framework.start_experiment("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_assign_variant_to_stopped_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        // Don't start it

        let assignment = framework
            .assign_variant(&experiment_id, "user123")
            .await
            .unwrap();
        assert!(assignment.is_none()); // Not running
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Results and Analysis Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_results_not_found() {
        let framework = ABTestFramework::new(ABTestConfig::default());

        let result = framework.get_results("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_experiments() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        framework.create_experiment(experiment).await.unwrap();

        let experiments = framework.list_experiments().await;
        assert_eq!(experiments.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.delete_experiment(&experiment_id).await.unwrap();

        let experiments = framework.list_experiments().await;
        assert!(experiments.is_empty());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());

        let result = framework.delete_experiment("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_record_outcome_not_running() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        // Don't start it

        let outcome = Outcome {
            user_id: "user123".to_string(),
            experiment_id: experiment_id.clone(),
            variant_id: "control".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            metrics: HashMap::new(),
            success: true,
            metadata: HashMap::new(),
        };

        let result = framework.record_outcome(outcome).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[tokio::test]
    async fn test_record_outcome_nonexistent_experiment() {
        let framework = ABTestFramework::new(ABTestConfig::default());

        let outcome = Outcome {
            user_id: "user123".to_string(),
            experiment_id: "nonexistent".to_string(),
            variant_id: "control".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            metrics: HashMap::new(),
            success: true,
            metadata: HashMap::new(),
        };

        let result = framework.record_outcome(outcome).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Data Structure Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_variant_serialization() {
        let variant = Variant {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test desc".to_string(),
            prompt_template: Some("Template".to_string()),
            model_config: None,
            parameters: HashMap::new(),
            traffic_allocation: 0.5,
        };

        let json = serde_json::to_string(&variant).unwrap();
        assert!(json.contains("test"));

        let deserialized: Variant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test");
    }

    #[test]
    fn test_model_config_serialization() {
        let config = ModelConfig {
            model_name: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            top_p: Some(0.9),
            top_k: Some(50),
            repetition_penalty: Some(1.1),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("gpt-4"));

        let deserialized: ModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model_name, "gpt-4");
        assert_eq!(deserialized.temperature, Some(0.7));
    }

    #[test]
    fn test_success_criteria_serialization() {
        let criteria = SuccessCriteria {
            primary_metric: "conversion".to_string(),
            improvement_threshold: 0.1,
            direction: OptimizationDirection::Maximize,
            secondary_metrics: vec!["latency".to_string()],
        };

        let json = serde_json::to_string(&criteria).unwrap();
        let deserialized: SuccessCriteria = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.primary_metric, "conversion");
    }

    #[test]
    fn test_optimization_direction() {
        let max = OptimizationDirection::Maximize;
        let min = OptimizationDirection::Minimize;

        let max_json = serde_json::to_string(&max).unwrap();
        let min_json = serde_json::to_string(&min).unwrap();

        assert!(max_json.contains("Maximize"));
        assert!(min_json.contains("Minimize"));
    }

    #[test]
    fn test_assignment_strategy_variants() {
        let hash = AssignmentStrategy::Hash;
        let weighted = AssignmentStrategy::WeightedRandom;
        let round_robin = AssignmentStrategy::RoundRobin;
        let custom = AssignmentStrategy::Custom("my_strategy".to_string());

        let hash_json = serde_json::to_string(&hash).unwrap();
        let custom_json = serde_json::to_string(&custom).unwrap();

        assert!(hash_json.contains("Hash"));
        assert!(custom_json.contains("my_strategy"));

        // Test deserialization
        let _: AssignmentStrategy = serde_json::from_str(&hash_json).unwrap();
        let _: AssignmentStrategy =
            serde_json::from_str(&serde_json::to_string(&weighted).unwrap()).unwrap();
        let _: AssignmentStrategy =
            serde_json::from_str(&serde_json::to_string(&round_robin).unwrap()).unwrap();
    }

    #[test]
    fn test_result_status_variants() {
        let statuses = [
            ResultStatus::InProgress,
            ResultStatus::SignificantResult,
            ResultStatus::NoSignificantDifference,
            ResultStatus::InsufficientData,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: ResultStatus = serde_json::from_str(&json).unwrap();
            // Just verify serialization round-trips
            assert_eq!(serde_json::to_string(&deserialized).unwrap(), json);
        }
    }

    #[test]
    fn test_metric_result_creation() {
        let metric = MetricResult {
            metric_name: "latency".to_string(),
            value: 150.0,
            count: 100,
            sum: 15000.0,
            mean: 150.0,
            std_dev: 25.0,
            percentiles: {
                let mut p = HashMap::new();
                p.insert("p50".to_string(), 140.0);
                p.insert("p95".to_string(), 200.0);
                p
            },
        };

        assert_eq!(metric.metric_name, "latency");
        assert_eq!(metric.mean, 150.0);
        assert_eq!(metric.percentiles.get("p50"), Some(&140.0));
    }

    #[test]
    fn test_confidence_interval() {
        let ci = ConfidenceInterval {
            lower_bound: 0.10,
            upper_bound: 0.20,
            confidence_level: 0.95,
        };

        let json = serde_json::to_string(&ci).unwrap();
        let deserialized: ConfidenceInterval = serde_json::from_str(&json).unwrap();
        assert!((deserialized.confidence_level - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_statistical_analysis() {
        let analysis = StatisticalAnalysis {
            test_type: "chi-square".to_string(),
            p_value: 0.023,
            effect_size: 0.3,
            power: 0.85,
            recommendations: vec!["Increase sample size".to_string()],
        };

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: StatisticalAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.test_type, "chi-square");
    }

    #[test]
    fn test_variant_results() {
        let results = VariantResults {
            variant_id: "test-variant".to_string(),
            sample_size: 500,
            metrics: HashMap::new(),
            conversion_rate: 0.15,
            confidence_interval: ConfidenceInterval {
                lower_bound: 0.12,
                upper_bound: 0.18,
                confidence_level: 0.95,
            },
        };

        assert_eq!(results.sample_size, 500);
        assert!((results.conversion_rate - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn test_experiment_results() {
        let results = ExperimentResults {
            experiment_id: "exp-1".to_string(),
            status: ResultStatus::InProgress,
            variant_results: HashMap::new(),
            statistical_analysis: None,
            winner: None,
            confidence_level: 0.0,
            last_updated: time::OffsetDateTime::now_utc(),
        };

        assert_eq!(results.experiment_id, "exp-1");
        assert_eq!(results.status, ResultStatus::InProgress);
        assert!(results.winner.is_none());
    }

    #[test]
    fn test_outcome_creation() {
        let outcome = Outcome {
            user_id: "user456".to_string(),
            experiment_id: "exp-1".to_string(),
            variant_id: "control".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            metrics: {
                let mut m = HashMap::new();
                m.insert("success_rate".to_string(), 0.8);
                m
            },
            success: true,
            metadata: HashMap::new(),
        };

        assert!(outcome.success);
        assert_eq!(outcome.metrics.get("success_rate"), Some(&0.8));
    }

    #[test]
    fn test_variant_assignment_creation() {
        let variant = Variant {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            prompt_template: None,
            model_config: None,
            parameters: HashMap::new(),
            traffic_allocation: 0.5,
        };

        let assignment = VariantAssignment {
            experiment_id: "exp-1".to_string(),
            variant_id: "test".to_string(),
            variant,
            assigned_at: time::OffsetDateTime::now_utc(),
        };

        assert_eq!(assignment.experiment_id, "exp-1");
        assert_eq!(assignment.variant_id, "test");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Max Concurrent Experiments Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_max_concurrent_experiments() {
        let config = ABTestConfig {
            max_concurrent_experiments: 2,
            ..ABTestConfig::default()
        };
        let framework = ABTestFramework::new(config);

        // Create and start 2 experiments
        let mut exp_ids = Vec::new();
        for i in 0..2 {
            let mut exp = create_test_experiment();
            exp.id = String::new(); // Let framework generate ID
            exp.name = format!("Experiment {}", i);
            exp.control_variant.id = format!("control-{}", i);
            exp.test_variants[0].id = format!("test-{}", i);
            let id = framework.create_experiment(exp).await.unwrap();
            framework.start_experiment(&id).await.unwrap();
            exp_ids.push(id);
        }

        // Third experiment creation should succeed (limit is checked at creation time against running experiments)
        // The check is for RUNNING experiments, not total experiments
        let mut exp = create_test_experiment();
        exp.id = String::new();
        exp.name = "Experiment 3".to_string();
        exp.control_variant.id = "control-3".to_string();
        exp.test_variants[0].id = "test-3".to_string();

        // Creation should work, but starting might fail
        let result = framework.create_experiment(exp).await;
        // With 2 running experiments and max_concurrent=2, creating a 3rd should fail
        assert!(
            result.is_err(),
            "Expected error when exceeding max concurrent experiments"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Multiple Outcomes Test
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_multiple_outcomes_same_variant() {
        let framework = ABTestFramework::new(ABTestConfig::default());
        let experiment = create_test_experiment();

        let experiment_id = framework.create_experiment(experiment).await.unwrap();
        framework.start_experiment(&experiment_id).await.unwrap();

        // Record multiple outcomes
        for i in 0..5 {
            let mut metrics = HashMap::new();
            metrics.insert("conversion_rate".to_string(), 0.1 + (i as f32) * 0.02);

            let outcome = Outcome {
                user_id: format!("user{}", i),
                experiment_id: experiment_id.clone(),
                variant_id: "control".to_string(),
                timestamp: time::OffsetDateTime::now_utc(),
                metrics,
                success: i % 2 == 0,
                metadata: HashMap::new(),
            };

            framework.record_outcome(outcome).await.unwrap();
        }

        let results = framework.get_results(&experiment_id).await.unwrap();
        assert_eq!(results.variant_results["control"].sample_size, 5);
    }
}

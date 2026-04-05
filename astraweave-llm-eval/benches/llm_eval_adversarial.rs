//! Adversarial benchmarks for astraweave-llm-eval
//!
//! Tests LLM evaluation system under extreme conditions:
//! - Massive evaluation batches
//! - Complex prompt combinations
//! - Score aggregation at scale
//! - Metric computation edge cases
//! - Concurrent evaluation scenarios
//! - Memory pressure during scoring

#![allow(
    dead_code,
    unused_imports,
    deprecated,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::or_fun_call,
    clippy::unwrap_or_default
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;
use std::time::{Duration, Instant};

// ============================================================================
// LOCAL TYPE DEFINITIONS (Standalone benchmark - no crate imports)
// ============================================================================

/// Evaluation prompt for LLM testing
#[derive(Clone, Debug)]
struct EvalPrompt {
    id: String,
    system_prompt: String,
    user_prompt: String,
    expected_response: Option<String>,
    tags: Vec<String>,
    max_tokens: usize,
    temperature: f32,
}

impl EvalPrompt {
    fn new(id: &str, system: &str, user: &str) -> Self {
        Self {
            id: id.to_string(),
            system_prompt: system.to_string(),
            user_prompt: user.to_string(),
            expected_response: None,
            tags: Vec::new(),
            max_tokens: 512,
            temperature: 0.7,
        }
    }

    fn with_expected(mut self, expected: &str) -> Self {
        self.expected_response = Some(expected.to_string());
        self
    }

    fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }
}

/// LLM response for evaluation
#[derive(Clone, Debug)]
struct EvalResponse {
    prompt_id: String,
    response_text: String,
    tokens_used: usize,
    latency_ms: u64,
    finish_reason: FinishReason,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FinishReason {
    Complete,
    MaxTokens,
    Timeout,
    Error,
}

/// Evaluation score for a single response
#[derive(Clone, Debug)]
struct EvalScore {
    prompt_id: String,
    relevance: f32,
    coherence: f32,
    accuracy: f32,
    creativity: f32,
    safety: f32,
    overall: f32,
}

impl EvalScore {
    fn new(prompt_id: &str) -> Self {
        Self {
            prompt_id: prompt_id.to_string(),
            relevance: 0.0,
            coherence: 0.0,
            accuracy: 0.0,
            creativity: 0.0,
            safety: 0.0,
            overall: 0.0,
        }
    }

    fn compute_overall(&mut self) {
        self.overall = (self.relevance * 0.25
            + self.coherence * 0.20
            + self.accuracy * 0.30
            + self.creativity * 0.10
            + self.safety * 0.15)
            .clamp(0.0, 1.0);
    }
}

/// Evaluation metric aggregation
#[derive(Clone, Debug, Default)]
struct MetricAggregator {
    scores: Vec<f32>,
    sum: f64,
    sum_squared: f64,
    min: f32,
    max: f32,
    count: usize,
}

impl MetricAggregator {
    fn new() -> Self {
        Self {
            scores: Vec::new(),
            sum: 0.0,
            sum_squared: 0.0,
            min: f32::MAX,
            max: f32::MIN,
            count: 0,
        }
    }

    fn add(&mut self, value: f32) {
        self.scores.push(value);
        self.sum += value as f64;
        self.sum_squared += (value * value) as f64;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.count += 1;
    }

    fn mean(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            (self.sum / self.count as f64) as f32
        }
    }

    fn variance(&self) -> f32 {
        if self.count < 2 {
            0.0
        } else {
            let mean = self.sum / self.count as f64;
            ((self.sum_squared / self.count as f64) - (mean * mean)) as f32
        }
    }

    fn std_dev(&self) -> f32 {
        self.variance().sqrt()
    }

    fn percentile(&mut self, p: f32) -> f32 {
        if self.scores.is_empty() {
            return 0.0;
        }
        self.scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * (self.scores.len() - 1) as f32) as usize;
        self.scores[idx.min(self.scores.len() - 1)]
    }
}

/// Evaluation batch for processing multiple prompts
#[derive(Clone, Debug)]
struct EvalBatch {
    id: String,
    prompts: Vec<EvalPrompt>,
    responses: Vec<EvalResponse>,
    scores: Vec<EvalScore>,
    metrics: HashMap<String, MetricAggregator>,
}

impl EvalBatch {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            prompts: Vec::new(),
            responses: Vec::new(),
            scores: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    fn add_prompt(&mut self, prompt: EvalPrompt) {
        self.prompts.push(prompt);
    }

    fn add_response(&mut self, response: EvalResponse) {
        self.responses.push(response);
    }

    fn add_score(&mut self, score: EvalScore) {
        // Aggregate metrics
        self.metrics
            .entry("relevance".to_string())
            .or_insert_with(MetricAggregator::new)
            .add(score.relevance);
        self.metrics
            .entry("coherence".to_string())
            .or_insert_with(MetricAggregator::new)
            .add(score.coherence);
        self.metrics
            .entry("accuracy".to_string())
            .or_insert_with(MetricAggregator::new)
            .add(score.accuracy);
        self.metrics
            .entry("overall".to_string())
            .or_insert_with(MetricAggregator::new)
            .add(score.overall);

        self.scores.push(score);
    }
}

/// Text similarity calculator using various metrics
struct SimilarityCalculator;

impl SimilarityCalculator {
    fn jaccard_similarity(a: &str, b: &str) -> f32 {
        let words_a: std::collections::HashSet<_> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<_> = b.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let m = a_chars.len();
        let n = b_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut dp = vec![vec![0usize; n + 1]; m + 1];

        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[m][n]
    }

    fn normalized_edit_distance(a: &str, b: &str) -> f32 {
        let dist = Self::levenshtein_distance(a, b);
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            1.0
        } else {
            1.0 - (dist as f32 / max_len as f32)
        }
    }

    fn cosine_similarity(a: &str, b: &str) -> f32 {
        let mut word_counts_a: HashMap<&str, f32> = HashMap::new();
        let mut word_counts_b: HashMap<&str, f32> = HashMap::new();

        for word in a.split_whitespace() {
            *word_counts_a.entry(word).or_insert(0.0) += 1.0;
        }
        for word in b.split_whitespace() {
            *word_counts_b.entry(word).or_insert(0.0) += 1.0;
        }

        let mut dot_product = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for (word, count_a) in &word_counts_a {
            let count_b = word_counts_b.get(word).unwrap_or(&0.0);
            dot_product += count_a * count_b;
            norm_a += count_a * count_a;
        }
        for count_b in word_counts_b.values() {
            norm_b += count_b * count_b;
        }

        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom == 0.0 {
            0.0
        } else {
            dot_product / denom
        }
    }
}

/// Mock LLM evaluator for benchmarking
struct MockEvaluator {
    latency_base_ms: u64,
    failure_rate: f32,
}

impl MockEvaluator {
    fn new() -> Self {
        Self {
            latency_base_ms: 10,
            failure_rate: 0.01,
        }
    }

    fn evaluate_prompt(&self, prompt: &EvalPrompt) -> EvalResponse {
        // Simulate response generation
        let truncated_prompt: String = prompt.user_prompt.chars().take(50).collect();
        let response_text = format!(
            "Response to: {}... [simulated {} tokens]",
            truncated_prompt,
            prompt.max_tokens / 2
        );

        let finish_reason = if (prompt.id.as_bytes()[0] as f32 / 255.0) < self.failure_rate {
            FinishReason::Error
        } else {
            FinishReason::Complete
        };

        EvalResponse {
            prompt_id: prompt.id.clone(),
            response_text,
            tokens_used: prompt.max_tokens / 2,
            latency_ms: self.latency_base_ms + (prompt.user_prompt.len() as u64 / 10),
            finish_reason,
        }
    }

    fn score_response(&self, prompt: &EvalPrompt, response: &EvalResponse) -> EvalScore {
        let mut score = EvalScore::new(&prompt.id);

        // Simulate scoring based on response characteristics
        let len_ratio = response.response_text.len() as f32 / (prompt.max_tokens * 4) as f32;
        score.relevance = (0.5 + len_ratio * 0.5).clamp(0.0, 1.0);
        score.coherence = if response.finish_reason == FinishReason::Complete {
            0.85
        } else {
            0.3
        };

        if let Some(expected) = &prompt.expected_response {
            score.accuracy =
                SimilarityCalculator::jaccard_similarity(&response.response_text, expected);
        } else {
            score.accuracy = 0.7;
        }

        score.creativity = (response.tokens_used as f32 / prompt.max_tokens as f32).clamp(0.3, 0.9);
        score.safety = if response.response_text.contains("unsafe") {
            0.2
        } else {
            0.95
        };

        score.compute_overall();
        score
    }
}

/// Evaluation configuration
#[derive(Clone, Debug)]
struct EvalConfig {
    batch_size: usize,
    parallel_evaluations: usize,
    timeout_ms: u64,
    retry_count: usize,
    metrics_to_track: Vec<String>,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            parallel_evaluations: 4,
            timeout_ms: 30000,
            retry_count: 3,
            metrics_to_track: vec![
                "relevance".to_string(),
                "coherence".to_string(),
                "accuracy".to_string(),
                "overall".to_string(),
            ],
        }
    }
}

// ============================================================================
// BENCHMARK GROUPS
// ============================================================================

fn bench_prompt_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("prompt_generation");

    for count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("create_prompts", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let prompts: Vec<EvalPrompt> = (0..count)
                        .map(|i| {
                            EvalPrompt::new(
                                &format!("prompt_{}", i),
                                "You are a helpful AI assistant.",
                                &format!("Question {}: What is the meaning of life?", i),
                            )
                            .with_expected("42")
                            .with_tags(vec!["philosophy", "test"])
                        })
                        .collect();
                    std_black_box(prompts)
                })
            },
        );
    }

    // Complex prompt generation with nested templates
    group.bench_function("complex_prompt_templates", |b| {
        b.iter(|| {
            let templates = [
                "Analyze the following code and identify bugs: {code}",
                "Translate from {src_lang} to {dst_lang}: {text}",
                "Summarize in {max_words} words: {document}",
                "Generate {count} test cases for: {function}",
            ];

            let prompts: Vec<EvalPrompt> = (0..1000)
                .map(|i| {
                    let template = templates[i % templates.len()];
                    EvalPrompt::new(
                        &format!("template_prompt_{}", i),
                        "You are an expert programmer.",
                        &template
                            .replace("{code}", &format!("fn test_{}() {{}}", i))
                            .replace("{src_lang}", "English")
                            .replace("{dst_lang}", "Spanish")
                            .replace("{text}", "Hello world")
                            .replace("{max_words}", "50")
                            .replace("{document}", "Long document...")
                            .replace("{count}", "5")
                            .replace("{function}", &format!("calculate_{}", i)),
                    )
                })
                .collect();
            std_black_box(prompts)
        })
    });

    group.finish();
}

fn bench_response_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_evaluation");
    let evaluator = MockEvaluator::new();

    // Pre-generate test data
    let prompts: Vec<EvalPrompt> = (0..1000)
        .map(|i| {
            EvalPrompt::new(
                &format!("eval_prompt_{}", i),
                "System prompt",
                &format!("User question {}", i),
            )
        })
        .collect();

    let responses: Vec<EvalResponse> = prompts
        .iter()
        .map(|p| evaluator.evaluate_prompt(p))
        .collect();

    for count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("score_responses", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let scores: Vec<EvalScore> = (0..count)
                        .map(|i| evaluator.score_response(&prompts[i], &responses[i]))
                        .collect();
                    std_black_box(scores)
                })
            },
        );
    }

    // Full evaluation pipeline
    group.bench_function("full_pipeline_100", |b| {
        b.iter(|| {
            let mut batch = EvalBatch::new("test_batch");

            for i in 0..100 {
                let prompt = EvalPrompt::new(
                    &format!("pipe_prompt_{}", i),
                    "System",
                    &format!("Question {}", i),
                );
                let response = evaluator.evaluate_prompt(&prompt);
                let score = evaluator.score_response(&prompt, &response);

                batch.add_prompt(prompt);
                batch.add_response(response);
                batch.add_score(score);
            }

            std_black_box(batch)
        })
    });

    group.finish();
}

fn bench_similarity_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_calculations");

    // Generate test text pairs
    let text_pairs: Vec<(String, String)> = (0..100)
        .map(|i| {
            let a = format!(
                "The quick brown fox jumps over the lazy dog. Sentence number {} with more words.",
                i
            );
            let b = format!(
                "A fast brown fox leaps over a sleepy dog. Sentence {} with additional content.",
                i
            );
            (a, b)
        })
        .collect();

    group.bench_function("jaccard_similarity_100", |b| {
        b.iter(|| {
            let similarities: Vec<f32> = text_pairs
                .iter()
                .map(|(a, b)| SimilarityCalculator::jaccard_similarity(a, b))
                .collect();
            std_black_box(similarities)
        })
    });

    group.bench_function("levenshtein_distance_100", |b| {
        b.iter(|| {
            let distances: Vec<usize> = text_pairs
                .iter()
                .map(|(a, b)| SimilarityCalculator::levenshtein_distance(a, b))
                .collect();
            std_black_box(distances)
        })
    });

    group.bench_function("normalized_edit_distance_100", |b| {
        b.iter(|| {
            let distances: Vec<f32> = text_pairs
                .iter()
                .map(|(a, b)| SimilarityCalculator::normalized_edit_distance(a, b))
                .collect();
            std_black_box(distances)
        })
    });

    group.bench_function("cosine_similarity_100", |b| {
        b.iter(|| {
            let similarities: Vec<f32> = text_pairs
                .iter()
                .map(|(a, b)| SimilarityCalculator::cosine_similarity(a, b))
                .collect();
            std_black_box(similarities)
        })
    });

    // Long text comparison stress test
    let long_text_a = "word ".repeat(10000);
    let long_text_b = "word ".repeat(9500) + &"different ".repeat(500);

    group.bench_function("jaccard_long_text", |b| {
        b.iter(|| {
            std_black_box(SimilarityCalculator::jaccard_similarity(
                &long_text_a,
                &long_text_b,
            ))
        })
    });

    group.bench_function("cosine_long_text", |b| {
        b.iter(|| {
            std_black_box(SimilarityCalculator::cosine_similarity(
                &long_text_a,
                &long_text_b,
            ))
        })
    });

    group.finish();
}

fn bench_metric_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_aggregation");

    for count in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("aggregate_scores", count),
            &count,
            |b, &count| {
                let scores: Vec<f32> = (0..count)
                    .map(|i| (i as f32 / count as f32).sin().abs())
                    .collect();

                b.iter(|| {
                    let mut aggregator = MetricAggregator::new();
                    for &score in &scores {
                        aggregator.add(score);
                    }
                    let mean = aggregator.mean();
                    let std_dev = aggregator.std_dev();
                    std_black_box((mean, std_dev))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compute_percentiles", count),
            &count,
            |b, &count| {
                let scores: Vec<f32> = (0..count)
                    .map(|i| (i as f32 / count as f32).sin().abs())
                    .collect();

                b.iter(|| {
                    let mut aggregator = MetricAggregator::new();
                    for &score in &scores {
                        aggregator.add(score);
                    }
                    let p50 = aggregator.percentile(50.0);
                    let p90 = aggregator.percentile(90.0);
                    let p99 = aggregator.percentile(99.0);
                    std_black_box((p50, p90, p99))
                })
            },
        );
    }

    // Multi-metric aggregation
    group.bench_function("multi_metric_1000_samples", |b| {
        let metrics = [
            "relevance",
            "coherence",
            "accuracy",
            "creativity",
            "safety",
            "overall",
        ];
        let sample_count = 1000;

        b.iter(|| {
            let mut aggregators: HashMap<&str, MetricAggregator> = metrics
                .iter()
                .map(|&m| (m, MetricAggregator::new()))
                .collect();

            for i in 0..sample_count {
                for (idx, &metric) in metrics.iter().enumerate() {
                    let value = ((i + idx) as f32 / sample_count as f32).sin().abs();
                    aggregators.get_mut(metric).unwrap().add(value);
                }
            }

            let results: Vec<(f32, f32)> = metrics
                .iter()
                .map(|&m| {
                    let agg = aggregators.get(m).unwrap();
                    (agg.mean(), agg.std_dev())
                })
                .collect();

            std_black_box(results)
        })
    });

    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");
    let evaluator = MockEvaluator::new();

    for batch_size in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("process_batch", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut batch = EvalBatch::new("benchmark_batch");

                    // Add prompts
                    for i in 0..batch_size {
                        let prompt = EvalPrompt::new(
                            &format!("batch_prompt_{}", i),
                            "You are a helpful assistant.",
                            &format!("Question {}: Explain concept {}", i, i * 2),
                        )
                        .with_expected(&format!("Expected answer {}", i));

                        batch.add_prompt(prompt);
                    }

                    // Process all prompts
                    for prompt in &batch.prompts.clone() {
                        let response = evaluator.evaluate_prompt(prompt);
                        let score = evaluator.score_response(prompt, &response);
                        batch.add_response(response);
                        batch.add_score(score);
                    }

                    // Compute final metrics
                    let overall_mean = batch
                        .metrics
                        .get("overall")
                        .map(|m| m.mean())
                        .unwrap_or(0.0);

                    std_black_box((batch, overall_mean))
                })
            },
        );
    }

    // Multiple concurrent batches simulation
    group.bench_function("concurrent_batches_10x100", |b| {
        b.iter(|| {
            let batches: Vec<EvalBatch> = (0..10)
                .map(|batch_idx| {
                    let mut batch = EvalBatch::new(&format!("batch_{}", batch_idx));

                    for i in 0..100 {
                        let prompt = EvalPrompt::new(
                            &format!("b{}_p{}", batch_idx, i),
                            "System",
                            &format!("Query {}", i),
                        );
                        let response = evaluator.evaluate_prompt(&prompt);
                        let score = evaluator.score_response(&prompt, &response);

                        batch.add_prompt(prompt);
                        batch.add_response(response);
                        batch.add_score(score);
                    }

                    batch
                })
                .collect();

            // Aggregate across all batches
            let total_overall: f32 = batches
                .iter()
                .filter_map(|b| b.metrics.get("overall").map(|m| m.mean()))
                .sum();

            std_black_box((batches, total_overall / 10.0))
        })
    });

    group.finish();
}

fn bench_evaluation_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluation_edge_cases");
    let evaluator = MockEvaluator::new();

    // Empty response handling
    group.bench_function("empty_responses", |b| {
        let prompts: Vec<EvalPrompt> = (0..100)
            .map(|i| EvalPrompt::new(&format!("empty_{}", i), "", ""))
            .collect();

        b.iter(|| {
            let scores: Vec<EvalScore> = prompts
                .iter()
                .map(|p| {
                    let response = EvalResponse {
                        prompt_id: p.id.clone(),
                        response_text: String::new(),
                        tokens_used: 0,
                        latency_ms: 1,
                        finish_reason: FinishReason::Complete,
                    };
                    evaluator.score_response(p, &response)
                })
                .collect();
            std_black_box(scores)
        })
    });

    // Very long prompts
    group.bench_function("long_prompts_10k_chars", |b| {
        let long_content = "x".repeat(10000);
        let prompts: Vec<EvalPrompt> = (0..10)
            .map(|i| {
                EvalPrompt::new(&format!("long_{}", i), &long_content, &long_content)
                    .with_expected(&long_content)
            })
            .collect();

        b.iter(|| {
            let scores: Vec<EvalScore> = prompts
                .iter()
                .map(|p| {
                    let response = evaluator.evaluate_prompt(p);
                    evaluator.score_response(p, &response)
                })
                .collect();
            std_black_box(scores)
        })
    });

    // Unicode stress test
    group.bench_function("unicode_heavy_prompts", |b| {
        let unicode_content = "🎮🎯🎲🎪🎨🎭🎬🎼🎹🎺".repeat(100);
        let prompts: Vec<EvalPrompt> = (0..50)
            .map(|i| {
                EvalPrompt::new(
                    &format!("unicode_{}", i),
                    &unicode_content,
                    &unicode_content,
                )
            })
            .collect();

        b.iter(|| {
            let scores: Vec<EvalScore> = prompts
                .iter()
                .map(|p| {
                    let response = evaluator.evaluate_prompt(p);
                    evaluator.score_response(p, &response)
                })
                .collect();
            std_black_box(scores)
        })
    });

    // Rapid scoring fluctuation
    group.bench_function("score_variance_computation", |b| {
        b.iter(|| {
            let mut aggregator = MetricAggregator::new();

            // Add scores with high variance
            for i in 0..1000 {
                let score = if i % 2 == 0 { 0.1 } else { 0.9 };
                aggregator.add(score);
            }

            let variance = aggregator.variance();
            let std_dev = aggregator.std_dev();
            std_black_box((variance, std_dev))
        })
    });

    group.finish();
}

fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");

    // Configuration creation and validation
    group.bench_function("config_creation_100", |b| {
        b.iter(|| {
            let configs: Vec<EvalConfig> = (0..100)
                .map(|i| EvalConfig {
                    batch_size: 10 + i,
                    parallel_evaluations: 1 + (i % 8),
                    timeout_ms: 1000 * (i as u64 + 1),
                    retry_count: 1 + (i % 5),
                    metrics_to_track: vec![
                        "relevance".to_string(),
                        "accuracy".to_string(),
                        format!("custom_metric_{}", i),
                    ],
                })
                .collect();
            std_black_box(configs)
        })
    });

    // Configuration cloning (common in parallel evaluation)
    group.bench_function("config_cloning_1000", |b| {
        let base_config = EvalConfig {
            batch_size: 100,
            parallel_evaluations: 8,
            timeout_ms: 30000,
            retry_count: 3,
            metrics_to_track: vec![
                "relevance".to_string(),
                "coherence".to_string(),
                "accuracy".to_string(),
                "creativity".to_string(),
                "safety".to_string(),
                "overall".to_string(),
            ],
        };

        b.iter(|| {
            let configs: Vec<EvalConfig> = (0..1000).map(|_| base_config.clone()).collect();
            std_black_box(configs)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_prompt_generation,
    bench_response_evaluation,
    bench_similarity_calculations,
    bench_metric_aggregation,
    bench_batch_processing,
    bench_evaluation_edge_cases,
    bench_config_operations,
);

criterion_main!(benches);

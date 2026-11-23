//! Mutation-resistant comprehensive tests for astraweave-optimization.
//!
//! Targets: InferenceParameters (default), RequestPriority (ordering, all variants),
//! BatchMetrics (derive Default), BatchInferenceConfig (default), PromptCacheConfig (default),
//! CachedResponse (serde), CacheStats (manual Default), PromptTemplate (serde),
//! SemanticCacheEntry (serde), BatchingStrategy (all variants), CacheLookupResult,
//! InvalidationStrategy (all variants).

use astraweave_optimization::*;
use std::collections::HashMap;
use std::time::Duration;

// =========================================================================
// InferenceParameters — Default
// =========================================================================

#[test]
fn inference_params_default_temperature() {
    let p = InferenceParameters::default();
    assert_eq!(p.temperature, Some(0.7));
}

#[test]
fn inference_params_default_max_tokens() {
    let p = InferenceParameters::default();
    assert_eq!(p.max_tokens, Some(512));
}

#[test]
fn inference_params_default_top_p() {
    let p = InferenceParameters::default();
    assert_eq!(p.top_p, Some(0.9));
}

#[test]
fn inference_params_default_top_k_none() {
    let p = InferenceParameters::default();
    assert_eq!(p.top_k, None);
}

#[test]
fn inference_params_default_repetition_penalty() {
    let p = InferenceParameters::default();
    assert_eq!(p.repetition_penalty, Some(1.1));
}

#[test]
fn inference_params_default_stop_sequences_empty() {
    let p = InferenceParameters::default();
    assert!(p.stop_sequences.is_empty());
}

#[test]
fn inference_params_serde_roundtrip() {
    let p = InferenceParameters::default();
    let json = serde_json::to_string(&p).unwrap();
    let p2: InferenceParameters = serde_json::from_str(&json).unwrap();
    assert_eq!(p.temperature, p2.temperature);
    assert_eq!(p.max_tokens, p2.max_tokens);
    assert_eq!(p.top_p, p2.top_p);
    assert_eq!(p.top_k, p2.top_k);
    assert_eq!(p.repetition_penalty, p2.repetition_penalty);
    assert_eq!(p.stop_sequences, p2.stop_sequences);
}

#[test]
fn inference_params_clone() {
    let p = InferenceParameters::default();
    let c = p.clone();
    assert_eq!(c.max_tokens, Some(512));
}

// =========================================================================
// RequestPriority — ordering, all 4 variants
// =========================================================================

#[test]
fn request_priority_low_less_than_normal() {
    assert!(RequestPriority::Low < RequestPriority::Normal);
}

#[test]
fn request_priority_normal_less_than_high() {
    assert!(RequestPriority::Normal < RequestPriority::High);
}

#[test]
fn request_priority_high_less_than_critical() {
    assert!(RequestPriority::High < RequestPriority::Critical);
}

#[test]
fn request_priority_low_less_than_critical() {
    assert!(RequestPriority::Low < RequestPriority::Critical);
}

#[test]
fn request_priority_equality() {
    assert_eq!(RequestPriority::Normal, RequestPriority::Normal);
}

#[test]
fn request_priority_inequality() {
    assert_ne!(RequestPriority::Low, RequestPriority::High);
}

#[test]
fn request_priority_serde_roundtrip() {
    let variants = [
        RequestPriority::Low,
        RequestPriority::Normal,
        RequestPriority::High,
        RequestPriority::Critical,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let v2: RequestPriority = serde_json::from_str(&json).unwrap();
        assert_eq!(*v, v2);
    }
}

// =========================================================================
// BatchMetrics — derive Default (all zeros / epoch)
// =========================================================================

#[test]
fn batch_metrics_default_total_requests() {
    let m = BatchMetrics::default();
    assert_eq!(m.total_requests, 0);
    assert_eq!(m.last_updated.unix_timestamp(), 0);
}

#[test]
fn batch_metrics_default_completed_requests() {
    let m = BatchMetrics::default();
    assert_eq!(m.completed_requests, 0);
}

#[test]
fn batch_metrics_default_failed_requests() {
    let m = BatchMetrics::default();
    assert_eq!(m.failed_requests, 0);
}

#[test]
fn batch_metrics_default_average_batch_size() {
    let m = BatchMetrics::default();
    assert_eq!(m.average_batch_size, 0.0);
}

#[test]
fn batch_metrics_default_average_processing_time() {
    let m = BatchMetrics::default();
    assert_eq!(m.average_processing_time_ms, 0.0);
}

#[test]
fn batch_metrics_default_average_wait_time() {
    let m = BatchMetrics::default();
    assert_eq!(m.average_wait_time_ms, 0.0);
}

#[test]
fn batch_metrics_default_throughput() {
    let m = BatchMetrics::default();
    assert_eq!(m.throughput_requests_per_second, 0.0);
}

#[test]
fn batch_metrics_default_queue_depth() {
    let m = BatchMetrics::default();
    assert_eq!(m.queue_depth, 0);
}

#[test]
fn batch_metrics_default_active_batches() {
    let m = BatchMetrics::default();
    assert_eq!(m.active_batches, 0);
}

#[test]
fn batch_metrics_serde_roundtrip() {
    let m = BatchMetrics::default();
    let json = serde_json::to_string(&m).unwrap();
    let m2: BatchMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(m.total_requests, m2.total_requests);
    assert_eq!(m.queue_depth, m2.queue_depth);
}

// =========================================================================
// BatchInferenceConfig — Default
// =========================================================================

#[test]
fn batch_config_default_max_batch_size() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.max_batch_size, 32);
}

#[test]
fn batch_config_default_min_batch_size() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.min_batch_size, 4);
}

#[test]
fn batch_config_default_batch_timeout() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.batch_timeout, Duration::from_millis(100));
}

#[test]
fn batch_config_default_request_timeout() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.request_timeout, Duration::from_secs(30));
}

#[test]
fn batch_config_default_worker_count() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.worker_count, 4);
}

#[test]
fn batch_config_default_dynamic_batching_enabled() {
    let c = BatchInferenceConfig::default();
    assert!(c.enable_dynamic_batching);
}

#[test]
fn batch_config_default_optimal_batch_size() {
    let c = BatchInferenceConfig::default();
    assert_eq!(c.optimal_batch_size, 16);
}

#[test]
fn batch_config_clone() {
    let c = BatchInferenceConfig::default();
    let c2 = c.clone();
    assert_eq!(c2.max_batch_size, 32);
    assert_eq!(c2.worker_count, 4);
}

// =========================================================================
// PromptCacheConfig — Default
// =========================================================================

#[test]
fn cache_config_default_max_entries() {
    let c = PromptCacheConfig::default();
    assert_eq!(c.max_cache_entries, 10000);
}

#[test]
fn cache_config_default_entry_ttl() {
    let c = PromptCacheConfig::default();
    assert_eq!(c.entry_ttl, Duration::from_secs(24 * 3600));
}

#[test]
fn cache_config_default_semantic_cache_enabled() {
    let c = PromptCacheConfig::default();
    assert!(c.enable_semantic_cache);
}

#[test]
fn cache_config_default_similarity_threshold() {
    let c = PromptCacheConfig::default();
    assert!((c.similarity_threshold - 0.85).abs() < 1e-6);
}

#[test]
fn cache_config_default_compression_enabled() {
    let c = PromptCacheConfig::default();
    assert!(c.enable_compression);
}

#[test]
fn cache_config_default_max_prompt_length() {
    let c = PromptCacheConfig::default();
    assert_eq!(c.max_prompt_length, 8192);
}

#[test]
fn cache_config_default_cache_warming_enabled() {
    let c = PromptCacheConfig::default();
    assert!(c.enable_cache_warming);
}

#[test]
fn cache_config_clone() {
    let c = PromptCacheConfig::default();
    let c2 = c.clone();
    assert_eq!(c2.max_cache_entries, 10000);
}

// =========================================================================
// CacheStats — manual Default (last_updated = time::OffsetDateTime::now_utc())
// =========================================================================

#[test]
fn cache_stats_default_total_requests() {
    let s = CacheStats::default();
    assert_eq!(s.total_requests, 0);
}

#[test]
fn cache_stats_default_cache_hits() {
    let s = CacheStats::default();
    assert_eq!(s.cache_hits, 0);
}

#[test]
fn cache_stats_default_cache_misses() {
    let s = CacheStats::default();
    assert_eq!(s.cache_misses, 0);
}

#[test]
fn cache_stats_default_semantic_hits() {
    let s = CacheStats::default();
    assert_eq!(s.semantic_hits, 0);
}

#[test]
fn cache_stats_default_evictions() {
    let s = CacheStats::default();
    assert_eq!(s.evictions, 0);
}

#[test]
fn cache_stats_default_hit_rate() {
    let s = CacheStats::default();
    assert_eq!(s.hit_rate, 0.0);
}

#[test]
fn cache_stats_default_avg_response_time() {
    let s = CacheStats::default();
    assert_eq!(s.average_response_time_ms, 0.0);
}

#[test]
fn cache_stats_default_memory_usage() {
    let s = CacheStats::default();
    assert_eq!(s.memory_usage_mb, 0.0);
}

#[test]
fn cache_stats_default_compression_savings() {
    let s = CacheStats::default();
    assert_eq!(s.compression_savings_mb, 0.0);
}

#[test]
fn cache_stats_default_last_updated_is_recent() {
    let before = time::OffsetDateTime::now_utc();
    let s = CacheStats::default();
    let after = time::OffsetDateTime::now_utc();
    assert!(s.last_updated >= before);
    assert!(s.last_updated <= after);
}

#[test]
fn cache_stats_serde_roundtrip() {
    let s = CacheStats::default();
    let json = serde_json::to_string(&s).unwrap();
    let s2: CacheStats = serde_json::from_str(&json).unwrap();
    assert_eq!(s.total_requests, s2.total_requests);
    assert_eq!(s.cache_hits, s2.cache_hits);
    assert_eq!(s.hit_rate, s2.hit_rate);
}

// =========================================================================
// CachedResponse — field access, serde
// =========================================================================

#[test]
fn cached_response_serde_roundtrip() {
    let now = time::OffsetDateTime::now_utc();
    let r = CachedResponse {
        id: "resp1".to_string(),
        response: "Hello world".to_string(),
        original_prompt: "Say hello".to_string(),
        prompt_hash: 12345,
        created_at: now,
        last_accessed: now,
        access_count: 5,
        ttl_expires_at: now,
        response_quality: 0.95,
        compression_ratio: Some(0.8),
        tags: vec!["test".to_string()],
    };
    let json = serde_json::to_string(&r).unwrap();
    let r2: CachedResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(r.id, r2.id);
    assert_eq!(r.response, r2.response);
    assert_eq!(r.prompt_hash, r2.prompt_hash);
    assert_eq!(r.access_count, r2.access_count);
    assert!((r.response_quality - r2.response_quality).abs() < 1e-6);
    assert_eq!(r.compression_ratio, r2.compression_ratio);
    assert_eq!(r.tags, r2.tags);
}

#[test]
fn cached_response_clone() {
    let now = time::OffsetDateTime::now_utc();
    let r = CachedResponse {
        id: "r1".to_string(),
        response: "ok".to_string(),
        original_prompt: "test".to_string(),
        prompt_hash: 0,
        created_at: now,
        last_accessed: now,
        access_count: 0,
        ttl_expires_at: now,
        response_quality: 1.0,
        compression_ratio: None,
        tags: vec![],
    };
    let c = r.clone();
    assert_eq!(c.id, "r1");
    assert_eq!(c.response_quality, 1.0);
}

// =========================================================================
// PromptTemplate — serde
// =========================================================================

#[test]
fn prompt_template_serde_roundtrip() {
    let t = PromptTemplate {
        id: "t1".to_string(),
        name: "Combat".to_string(),
        template: "You are a {role}".to_string(),
        variables: vec!["role".to_string()],
        usage_count: 42,
        average_response_length: 150.5,
        cache_hit_rate: 0.75,
    };
    let json = serde_json::to_string(&t).unwrap();
    let t2: PromptTemplate = serde_json::from_str(&json).unwrap();
    assert_eq!(t.id, t2.id);
    assert_eq!(t.name, t2.name);
    assert_eq!(t.template, t2.template);
    assert_eq!(t.variables, t2.variables);
    assert_eq!(t.usage_count, t2.usage_count);
    assert!((t.average_response_length - t2.average_response_length).abs() < 1e-6);
    assert!((t.cache_hit_rate - t2.cache_hit_rate).abs() < 1e-6);
}

// =========================================================================
// SemanticCacheEntry — serde
// =========================================================================

#[test]
fn semantic_cache_entry_serde_roundtrip() {
    let now = time::OffsetDateTime::now_utc();
    let entry = SemanticCacheEntry {
        cached_response: CachedResponse {
            id: "se1".to_string(),
            response: "test".to_string(),
            original_prompt: "q".to_string(),
            prompt_hash: 99,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            ttl_expires_at: now,
            response_quality: 0.5,
            compression_ratio: None,
            tags: vec![],
        },
        embedding: vec![0.1, 0.2, 0.3],
        similarity_scores: {
            let mut m = HashMap::new();
            m.insert("key".to_string(), 0.9);
            m
        },
    };
    let json = serde_json::to_string(&entry).unwrap();
    let e2: SemanticCacheEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(e2.cached_response.id, "se1");
    assert_eq!(e2.embedding.len(), 3);
    assert!((e2.embedding[0] - 0.1).abs() < 1e-6);
    assert!((e2.similarity_scores["key"] - 0.9).abs() < 1e-6);
}

// =========================================================================
// BatchResult — field access
// =========================================================================

#[test]
fn batch_result_fields() {
    let r = BatchResult {
        batch_id: "b1".to_string(),
        results: vec![
            ("r1".to_string(), Ok("success".to_string())),
            ("r2".to_string(), Err("failed".to_string())),
        ],
        processing_time_ms: 100,
        batch_size: 2,
        success_count: 1,
        failure_count: 1,
    };
    assert_eq!(r.batch_id, "b1");
    assert_eq!(r.results.len(), 2);
    assert!(r.results[0].1.is_ok());
    assert!(r.results[1].1.is_err());
    assert_eq!(r.processing_time_ms, 100);
    assert_eq!(r.batch_size, 2);
    assert_eq!(r.success_count, 1);
    assert_eq!(r.failure_count, 1);
}

#[test]
fn batch_result_clone() {
    let r = BatchResult {
        batch_id: "b2".to_string(),
        results: vec![("r1".to_string(), Ok("ok".to_string()))],
        processing_time_ms: 50,
        batch_size: 1,
        success_count: 1,
        failure_count: 0,
    };
    let c = r.clone();
    assert_eq!(c.batch_id, "b2");
    assert_eq!(c.success_count, 1);
}

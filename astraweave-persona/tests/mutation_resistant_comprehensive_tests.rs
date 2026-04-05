//! Mutation-resistant comprehensive tests for astraweave-persona.
//!
//! These tests target exact default values, boundary conditions, operator swaps,
//! negation bugs, and off-by-one errors to achieve 90%+ mutation kill rate.

#![allow(clippy::field_reassign_with_default)]

use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
use astraweave_llm::MockLlm;
use astraweave_memory::Persona as BasePersona;
use astraweave_persona::*;
use astraweave_rag::{RagConfig, RagPipeline, VectorStoreWrapper};
use std::collections::HashMap;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// PersonaLlmConfig default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persona_llm_config_default_temperature() {
    let c = PersonaLlmConfig::default();
    assert!(
        (c.temperature - 0.8).abs() < f32::EPSILON,
        "default temperature must be 0.8, got {}",
        c.temperature
    );
}

#[test]
fn persona_llm_config_default_top_p() {
    let c = PersonaLlmConfig::default();
    assert!(
        (c.top_p - 0.9).abs() < f32::EPSILON,
        "default top_p must be 0.9, got {}",
        c.top_p
    );
}

#[test]
fn persona_llm_config_default_max_tokens() {
    let c = PersonaLlmConfig::default();
    assert_eq!(c.max_tokens, 512, "default max_tokens must be 512");
}

#[test]
fn persona_llm_config_default_context_window_size() {
    let c = PersonaLlmConfig::default();
    assert_eq!(c.context_window_size, 2048);
}

#[test]
fn persona_llm_config_default_response_style() {
    let c = PersonaLlmConfig::default();
    match c.response_style {
        ResponseStyle::Conversational => {} // expected
        _ => panic!("expected Conversational, got {:?}", c.response_style),
    }
}

#[test]
fn persona_llm_config_default_creativity() {
    let c = PersonaLlmConfig::default();
    let v = c.personality_factors.get("creativity").copied().unwrap();
    assert!(
        (v - 0.7).abs() < f32::EPSILON,
        "creativity must be 0.7, got {v}"
    );
}

#[test]
fn persona_llm_config_default_empathy() {
    let c = PersonaLlmConfig::default();
    let v = c.personality_factors.get("empathy").copied().unwrap();
    assert!(
        (v - 0.8).abs() < f32::EPSILON,
        "empathy must be 0.8, got {v}"
    );
}

#[test]
fn persona_llm_config_default_assertiveness() {
    let c = PersonaLlmConfig::default();
    let v = c.personality_factors.get("assertiveness").copied().unwrap();
    assert!(
        (v - 0.6).abs() < f32::EPSILON,
        "assertiveness must be 0.6, got {v}"
    );
}

#[test]
fn persona_llm_config_default_curiosity() {
    let c = PersonaLlmConfig::default();
    let v = c.personality_factors.get("curiosity").copied().unwrap();
    assert!(
        (v - 0.7).abs() < f32::EPSILON,
        "curiosity must be 0.7, got {v}"
    );
}

#[test]
fn persona_llm_config_default_humor() {
    let c = PersonaLlmConfig::default();
    let v = c.personality_factors.get("humor").copied().unwrap();
    assert!((v - 0.5).abs() < f32::EPSILON, "humor must be 0.5, got {v}");
}

#[test]
fn persona_llm_config_default_has_exactly_five_factors() {
    let c = PersonaLlmConfig::default();
    assert_eq!(c.personality_factors.len(), 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// PersonalityState default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn personality_state_default_mood() {
    let ps = PersonalityState::default();
    assert!(
        (ps.current_mood - 0.0).abs() < f32::EPSILON,
        "default mood must be 0.0"
    );
}

#[test]
fn personality_state_default_energy() {
    let ps = PersonalityState::default();
    assert!(
        (ps.energy_level - 0.7).abs() < f32::EPSILON,
        "default energy must be 0.7, got {}",
        ps.energy_level
    );
}

#[test]
fn personality_state_default_confidence() {
    let ps = PersonalityState::default();
    assert!(
        (ps.confidence - 0.6).abs() < f32::EPSILON,
        "default confidence must be 0.6, got {}",
        ps.confidence
    );
}

#[test]
fn personality_state_default_trust() {
    let ps = PersonalityState::default();
    assert!(
        (ps.trust_level - 0.5).abs() < f32::EPSILON,
        "default trust must be 0.5, got {}",
        ps.trust_level
    );
}

#[test]
fn personality_state_default_emotional_state() {
    let ps = PersonalityState::default();
    match ps.emotional_state {
        EmotionalState::Neutral => {} // expected
        _ => panic!("expected Neutral, got {:?}", ps.emotional_state),
    }
}

#[test]
fn personality_state_default_drift_empty() {
    let ps = PersonalityState::default();
    assert!(ps.personality_drift.is_empty(), "drift must be empty");
}

#[test]
fn personality_state_default_influences_empty() {
    let ps = PersonalityState::default();
    assert!(ps.recent_influences.is_empty(), "influences must be empty");
}

// ═══════════════════════════════════════════════════════════════════════════
// AdaptationData default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn adaptation_data_default_interaction_count() {
    let ad = AdaptationData::default();
    assert_eq!(ad.interaction_count, 0);
}

#[test]
fn adaptation_data_default_successful() {
    let ad = AdaptationData::default();
    assert_eq!(ad.successful_interactions, 0);
}

#[test]
fn adaptation_data_default_learning_rate() {
    let ad = AdaptationData::default();
    assert!(
        (ad.learning_rate - 0.1).abs() < f32::EPSILON,
        "learning rate must be 0.1, got {}",
        ad.learning_rate
    );
}

#[test]
fn adaptation_data_default_preferred_topics_empty() {
    let ad = AdaptationData::default();
    assert!(ad.preferred_topics.is_empty());
}

#[test]
fn adaptation_data_default_topics_to_avoid_empty() {
    let ad = AdaptationData::default();
    assert!(ad.topics_to_avoid.is_empty());
}

#[test]
fn adaptation_data_default_history_empty() {
    let ad = AdaptationData::default();
    assert!(ad.adaptation_history.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// PlayerPatterns default
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn player_patterns_default_communication_style_none() {
    let pp = PlayerPatterns::default();
    assert!(pp.communication_style.is_none());
}

#[test]
fn player_patterns_default_interests_empty() {
    let pp = PlayerPatterns::default();
    assert!(pp.interests.is_empty());
}

#[test]
fn player_patterns_default_avg_session_none() {
    let pp = PlayerPatterns::default();
    assert!(pp.avg_session_length.is_none());
}

#[test]
fn player_patterns_default_preferred_times_empty() {
    let pp = PlayerPatterns::default();
    assert!(pp.preferred_times.is_empty());
}

#[test]
fn player_patterns_default_emotional_patterns_empty() {
    let pp = PlayerPatterns::default();
    assert!(pp.emotional_patterns.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// PromptSettings default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn prompt_settings_default_template_contains_persona_name() {
    let ps = PromptSettings::default();
    assert!(ps.system_prompt_template.contains("{{persona.name}}"));
}

#[test]
fn prompt_settings_default_template_contains_persona_description() {
    let ps = PromptSettings::default();
    assert!(ps
        .system_prompt_template
        .contains("{{persona.description}}"));
}

#[test]
fn prompt_settings_default_template_contains_mood() {
    let ps = PromptSettings::default();
    assert!(ps.system_prompt_template.contains("{{state.mood}}"));
}

#[test]
fn prompt_settings_default_template_contains_personality() {
    let ps = PromptSettings::default();
    assert!(ps
        .system_prompt_template
        .contains("{{persona.personality}}"));
}

#[test]
fn prompt_settings_default_context_injection_contextual() {
    let ps = PromptSettings::default();
    match ps.context_injection {
        ContextInjectionStrategy::Contextual => {} // expected
        _ => panic!("expected Contextual, got {:?}", ps.context_injection),
    }
}

#[test]
fn prompt_settings_default_few_shot_empty() {
    let ps = PromptSettings::default();
    assert!(ps.few_shot_examples.is_empty());
}

#[test]
fn prompt_settings_default_modifiers_empty() {
    let ps = PromptSettings::default();
    assert!(ps.prompt_modifiers.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// MemoryRetrievalSettings default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn memory_retrieval_default_max_memories() {
    let mr = MemoryRetrievalSettings::default();
    assert_eq!(mr.max_memories, 5);
}

#[test]
fn memory_retrieval_default_min_similarity() {
    let mr = MemoryRetrievalSettings::default();
    assert!(
        (mr.min_similarity - 0.3).abs() < f32::EPSILON,
        "min_similarity must be 0.3, got {}",
        mr.min_similarity
    );
}

#[test]
fn memory_retrieval_default_priority_categories_count() {
    let mr = MemoryRetrievalSettings::default();
    assert_eq!(mr.priority_categories.len(), 2);
}

#[test]
fn memory_retrieval_default_priority_categories_social() {
    let mr = MemoryRetrievalSettings::default();
    assert_eq!(mr.priority_categories[0], "Social");
}

#[test]
fn memory_retrieval_default_priority_categories_dialogue() {
    let mr = MemoryRetrievalSettings::default();
    assert_eq!(mr.priority_categories[1], "Dialogue");
}

#[test]
fn memory_retrieval_default_recency_bonus() {
    let mr = MemoryRetrievalSettings::default();
    assert!(
        (mr.recency_bonus - 0.1).abs() < f32::EPSILON,
        "recency_bonus must be 0.1, got {}",
        mr.recency_bonus
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// MemoryProfile default
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn memory_profile_default_core_empty() {
    let mp = MemoryProfile::default();
    assert!(mp.core_memories.is_empty());
}

#[test]
fn memory_profile_default_episodic_empty() {
    let mp = MemoryProfile::default();
    assert!(mp.episodic_memories.is_empty());
}

#[test]
fn memory_profile_default_semantic_empty() {
    let mp = MemoryProfile::default();
    assert!(mp.semantic_knowledge.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// ConsolidationPreferences default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn consolidation_default_frequency() {
    let cp = ConsolidationPreferences::default();
    assert_eq!(cp.consolidation_frequency, 100, "must be 100");
}

#[test]
fn consolidation_default_importance_threshold() {
    let cp = ConsolidationPreferences::default();
    assert!(
        (cp.importance_threshold - 0.3).abs() < f32::EPSILON,
        "must be 0.3, got {}",
        cp.importance_threshold
    );
}

#[test]
fn consolidation_default_max_memories() {
    let cp = ConsolidationPreferences::default();
    assert_eq!(cp.max_memories, 1000);
}

// ═══════════════════════════════════════════════════════════════════════════
// ForgettingCurve default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn forgetting_curve_default_decay_rate() {
    let fc = ForgettingCurve::default();
    assert!(
        (fc.decay_rate - 0.1).abs() < f32::EPSILON,
        "must be 0.1, got {}",
        fc.decay_rate
    );
}

#[test]
fn forgetting_curve_default_importance_multiplier() {
    let fc = ForgettingCurve::default();
    assert!(
        (fc.importance_multiplier - 2.0).abs() < f32::EPSILON,
        "must be 2.0, got {}",
        fc.importance_multiplier
    );
}

#[test]
fn forgetting_curve_default_rehearsal_bonus() {
    let fc = ForgettingCurve::default();
    assert!(
        (fc.rehearsal_bonus - 0.5).abs() < f32::EPSILON,
        "must be 0.5, got {}",
        fc.rehearsal_bonus
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// PersonaMetrics default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persona_metrics_default_total_interactions() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.total_interactions, 0);
}

#[test]
fn persona_metrics_default_successful_generations() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.successful_generations, 0);
}

#[test]
fn persona_metrics_default_failed_generations() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.failed_generations, 0);
}

#[test]
fn persona_metrics_default_avg_response_time() {
    let pm = PersonaMetrics::default();
    assert!((pm.avg_response_time_ms - 0.0).abs() < f32::EPSILON);
}

#[test]
fn persona_metrics_default_evolution_events() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.personality_evolution_events, 0);
}

#[test]
fn persona_metrics_default_consolidations() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.memory_consolidations, 0);
}

#[test]
fn persona_metrics_default_learning_events() {
    let pm = PersonaMetrics::default();
    assert_eq!(pm.adaptation_learning_events, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Enum variant existence & Debug
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn response_style_conversational_debug() {
    let s = ResponseStyle::Conversational;
    let d = format!("{s:?}");
    assert_eq!(d, "Conversational");
}

#[test]
fn response_style_formal_debug() {
    let s = ResponseStyle::Formal;
    let d = format!("{s:?}");
    assert_eq!(d, "Formal");
}

#[test]
fn response_style_creative_debug() {
    let s = ResponseStyle::Creative;
    let d = format!("{s:?}");
    assert_eq!(d, "Creative");
}

#[test]
fn response_style_technical_debug() {
    let s = ResponseStyle::Technical;
    let d = format!("{s:?}");
    assert_eq!(d, "Technical");
}

#[test]
fn response_style_playful_debug() {
    let s = ResponseStyle::Playful;
    let d = format!("{s:?}");
    assert_eq!(d, "Playful");
}

#[test]
fn response_style_mysterious_debug() {
    let s = ResponseStyle::Mysterious;
    let d = format!("{s:?}");
    assert_eq!(d, "Mysterious");
}

#[test]
fn emotional_state_joyful_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Joyful), "Joyful");
}

#[test]
fn emotional_state_excited_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Excited), "Excited");
}

#[test]
fn emotional_state_calm_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Calm), "Calm");
}

#[test]
fn emotional_state_neutral_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Neutral), "Neutral");
}

#[test]
fn emotional_state_thoughtful_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Thoughtful), "Thoughtful");
}

#[test]
fn emotional_state_concerned_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Concerned), "Concerned");
}

#[test]
fn emotional_state_frustrated_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Frustrated), "Frustrated");
}

#[test]
fn emotional_state_sad_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Sad), "Sad");
}

#[test]
fn emotional_state_angry_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Angry), "Angry");
}

#[test]
fn emotional_state_surprised_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Surprised), "Surprised");
}

#[test]
fn emotional_state_curious_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Curious), "Curious");
}

#[test]
fn emotional_state_confident_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Confident), "Confident");
}

#[test]
fn emotional_state_anxious_debug() {
    assert_eq!(format!("{:?}", EmotionalState::Anxious), "Anxious");
}

#[test]
fn context_injection_full_debug() {
    assert_eq!(format!("{:?}", ContextInjectionStrategy::Full), "Full");
}

#[test]
fn context_injection_recent_debug() {
    assert_eq!(format!("{:?}", ContextInjectionStrategy::Recent), "Recent");
}

#[test]
fn context_injection_contextual_debug() {
    assert_eq!(
        format!("{:?}", ContextInjectionStrategy::Contextual),
        "Contextual"
    );
}

#[test]
fn context_injection_minimal_debug() {
    assert_eq!(
        format!("{:?}", ContextInjectionStrategy::Minimal),
        "Minimal"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Struct field construction & verification
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn personality_influence_fields() {
    let mut changes = HashMap::new();
    changes.insert("mood".into(), 0.5);
    let pi = PersonalityInfluence {
        event: "victory".into(),
        factor_changes: changes,
        timestamp: 12345,
        importance: 0.9,
        decay_rate: 0.05,
    };
    assert_eq!(pi.event, "victory");
    assert_eq!(pi.timestamp, 12345);
    assert!((pi.importance - 0.9).abs() < f32::EPSILON);
    assert!((pi.decay_rate - 0.05).abs() < f32::EPSILON);
    assert_eq!(pi.factor_changes.len(), 1);
    assert!((pi.factor_changes["mood"] - 0.5).abs() < f32::EPSILON);
}

#[test]
fn adaptation_event_fields() {
    let ae = AdaptationEvent {
        trigger: "battle_win".into(),
        changes: HashMap::new(),
        timestamp: 999,
        success_rating: Some(0.8),
    };
    assert_eq!(ae.trigger, "battle_win");
    assert_eq!(ae.timestamp, 999);
    assert_eq!(ae.success_rating, Some(0.8));
    assert!(ae.changes.is_empty());
}

#[test]
fn adaptation_event_no_rating() {
    let ae = AdaptationEvent {
        trigger: "explore".into(),
        changes: HashMap::new(),
        timestamp: 0,
        success_rating: None,
    };
    assert!(ae.success_rating.is_none());
}

#[test]
fn few_shot_example_fields() {
    let ex = FewShotExample {
        input: "Hello there".into(),
        output: "Greetings!".into(),
        context: Some("merchant shop".into()),
        tags: vec!["greeting".into(), "shop".into()],
    };
    assert_eq!(ex.input, "Hello there");
    assert_eq!(ex.output, "Greetings!");
    assert_eq!(ex.context, Some("merchant shop".into()));
    assert_eq!(ex.tags.len(), 2);
    assert_eq!(ex.tags[0], "greeting");
    assert_eq!(ex.tags[1], "shop");
}

#[test]
fn few_shot_example_no_context() {
    let ex = FewShotExample {
        input: "Hi".into(),
        output: "Hello".into(),
        context: None,
        tags: vec![],
    };
    assert!(ex.context.is_none());
    assert!(ex.tags.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Serialization round-trip tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persona_llm_config_json_roundtrip() {
    let c = PersonaLlmConfig::default();
    let json = serde_json::to_string(&c).unwrap();
    let back: PersonaLlmConfig = serde_json::from_str(&json).unwrap();
    assert!((back.temperature - 0.8).abs() < f32::EPSILON);
    assert_eq!(back.max_tokens, 512);
    assert_eq!(back.context_window_size, 2048);
}

#[test]
fn personality_state_json_roundtrip() {
    let ps = PersonalityState::default();
    let json = serde_json::to_string(&ps).unwrap();
    let back: PersonalityState = serde_json::from_str(&json).unwrap();
    assert!((back.current_mood - 0.0).abs() < f32::EPSILON);
    assert!((back.energy_level - 0.7).abs() < f32::EPSILON);
}

#[test]
fn adaptation_data_json_roundtrip() {
    let ad = AdaptationData::default();
    let json = serde_json::to_string(&ad).unwrap();
    let back: AdaptationData = serde_json::from_str(&json).unwrap();
    assert_eq!(back.interaction_count, 0);
    assert!((back.learning_rate - 0.1).abs() < f32::EPSILON);
}

#[test]
fn player_patterns_json_roundtrip() {
    let pp = PlayerPatterns {
        communication_style: Some("casual".into()),
        interests: vec!["combat".into()],
        avg_session_length: Some(30.0),
        preferred_times: vec!["evening".into()],
        emotional_patterns: HashMap::new(),
    };
    let json = serde_json::to_string(&pp).unwrap();
    let back: PlayerPatterns = serde_json::from_str(&json).unwrap();
    assert_eq!(back.communication_style, Some("casual".into()));
    assert_eq!(back.interests.len(), 1);
    assert_eq!(back.interests[0], "combat");
    assert!((back.avg_session_length.unwrap() - 30.0).abs() < f32::EPSILON);
}

#[test]
fn prompt_settings_json_roundtrip() {
    let ps = PromptSettings::default();
    let json = serde_json::to_string(&ps).unwrap();
    let back: PromptSettings = serde_json::from_str(&json).unwrap();
    assert!(back.system_prompt_template.contains("{{persona.name}}"));
}

#[test]
fn memory_retrieval_json_roundtrip() {
    let mr = MemoryRetrievalSettings::default();
    let json = serde_json::to_string(&mr).unwrap();
    let back: MemoryRetrievalSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.max_memories, 5);
    assert!((back.min_similarity - 0.3).abs() < f32::EPSILON);
}

#[test]
fn consolidation_preferences_json_roundtrip() {
    let cp = ConsolidationPreferences::default();
    let json = serde_json::to_string(&cp).unwrap();
    let back: ConsolidationPreferences = serde_json::from_str(&json).unwrap();
    assert_eq!(back.consolidation_frequency, 100);
    assert_eq!(back.max_memories, 1000);
}

#[test]
fn forgetting_curve_json_roundtrip() {
    let fc = ForgettingCurve::default();
    let json = serde_json::to_string(&fc).unwrap();
    let back: ForgettingCurve = serde_json::from_str(&json).unwrap();
    assert!((back.decay_rate - 0.1).abs() < f32::EPSILON);
    assert!((back.importance_multiplier - 2.0).abs() < f32::EPSILON);
    assert!((back.rehearsal_bonus - 0.5).abs() < f32::EPSILON);
}

#[test]
fn persona_metrics_json_roundtrip() {
    let mut pm = PersonaMetrics::default();
    pm.total_interactions = 100;
    pm.successful_generations = 95;
    pm.failed_generations = 5;
    pm.avg_response_time_ms = 150.5;
    let json = serde_json::to_string(&pm).unwrap();
    let back: PersonaMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(back.total_interactions, 100);
    assert_eq!(back.successful_generations, 95);
    assert_eq!(back.failed_generations, 5);
    assert!((back.avg_response_time_ms - 150.5).abs() < f32::EPSILON);
}

#[test]
fn memory_profile_json_roundtrip() {
    let mp = MemoryProfile {
        core_memories: vec!["born in village".into()],
        episodic_memories: vec!["met the hero".into()],
        semantic_knowledge: vec!["herbs heal".into()],
        consolidation_preferences: ConsolidationPreferences::default(),
    };
    let json = serde_json::to_string(&mp).unwrap();
    let back: MemoryProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(back.core_memories.len(), 1);
    assert_eq!(back.core_memories[0], "born in village");
    assert_eq!(back.episodic_memories[0], "met the hero");
    assert_eq!(back.semantic_knowledge[0], "herbs heal");
}

#[test]
fn personality_influence_json_roundtrip() {
    let mut changes = HashMap::new();
    changes.insert("trust".into(), 0.2);
    let pi = PersonalityInfluence {
        event: "saved life".into(),
        factor_changes: changes,
        timestamp: 5000,
        importance: 1.0,
        decay_rate: 0.01,
    };
    let json = serde_json::to_string(&pi).unwrap();
    let back: PersonalityInfluence = serde_json::from_str(&json).unwrap();
    assert_eq!(back.event, "saved life");
    assert_eq!(back.timestamp, 5000);
    assert!((back.importance - 1.0).abs() < f32::EPSILON);
    assert!((back.decay_rate - 0.01).abs() < f32::EPSILON);
    assert!((back.factor_changes["trust"] - 0.2).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// ResponseStyle serialization (all variants)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn response_style_conversational_json() {
    let json = serde_json::to_string(&ResponseStyle::Conversational).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Conversational => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_style_formal_json() {
    let json = serde_json::to_string(&ResponseStyle::Formal).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Formal => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_style_creative_json() {
    let json = serde_json::to_string(&ResponseStyle::Creative).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Creative => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_style_technical_json() {
    let json = serde_json::to_string(&ResponseStyle::Technical).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Technical => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_style_playful_json() {
    let json = serde_json::to_string(&ResponseStyle::Playful).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Playful => {}
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_style_mysterious_json() {
    let json = serde_json::to_string(&ResponseStyle::Mysterious).unwrap();
    let back: ResponseStyle = serde_json::from_str(&json).unwrap();
    match back {
        ResponseStyle::Mysterious => {}
        _ => panic!("wrong variant"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EmotionalState serialization (all variants)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn emotional_state_all_variants_json() {
    let variants = vec![
        EmotionalState::Joyful,
        EmotionalState::Excited,
        EmotionalState::Calm,
        EmotionalState::Neutral,
        EmotionalState::Thoughtful,
        EmotionalState::Concerned,
        EmotionalState::Frustrated,
        EmotionalState::Sad,
        EmotionalState::Angry,
        EmotionalState::Surprised,
        EmotionalState::Curious,
        EmotionalState::Confident,
        EmotionalState::Anxious,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: EmotionalState = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{v:?}"));
    }
    assert_eq!(variants.len(), 13, "must have exactly 13 variants");
}

// ═══════════════════════════════════════════════════════════════════════════
// ContextInjectionStrategy serialization (all variants)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn context_injection_all_variants_json() {
    let variants = vec![
        ContextInjectionStrategy::Full,
        ContextInjectionStrategy::Recent,
        ContextInjectionStrategy::Contextual,
        ContextInjectionStrategy::Minimal,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: ContextInjectionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{back:?}"), format!("{v:?}"));
    }
    assert_eq!(variants.len(), 4, "must have exactly 4 variants");
}

// ═══════════════════════════════════════════════════════════════════════════
// Clone tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persona_llm_config_clone_preserves_values() {
    let c = PersonaLlmConfig::default();
    let c2 = c.clone();
    assert!((c2.temperature - 0.8).abs() < f32::EPSILON);
    assert_eq!(c2.max_tokens, 512);
    assert_eq!(c2.personality_factors.len(), 5);
}

#[test]
fn personality_state_clone_preserves_values() {
    let ps = PersonalityState::default();
    let ps2 = ps.clone();
    assert!((ps2.energy_level - 0.7).abs() < f32::EPSILON);
    assert!((ps2.confidence - 0.6).abs() < f32::EPSILON);
}

#[test]
fn adaptation_data_clone_preserves_values() {
    let ad = AdaptationData::default();
    let ad2 = ad.clone();
    assert_eq!(ad2.interaction_count, 0);
    assert!((ad2.learning_rate - 0.1).abs() < f32::EPSILON);
}

#[test]
fn forgetting_curve_clone_preserves_values() {
    let fc = ForgettingCurve::default();
    let fc2 = fc.clone();
    assert!((fc2.decay_rate - 0.1).abs() < f32::EPSILON);
    assert!((fc2.importance_multiplier - 2.0).abs() < f32::EPSILON);
    assert!((fc2.rehearsal_bonus - 0.5).abs() < f32::EPSILON);
}

#[test]
fn persona_metrics_clone_preserves_values() {
    let mut pm = PersonaMetrics::default();
    pm.total_interactions = 42;
    pm.successful_generations = 40;
    let pm2 = pm.clone();
    assert_eq!(pm2.total_interactions, 42);
    assert_eq!(pm2.successful_generations, 40);
}

#[test]
fn response_style_copy() {
    let s = ResponseStyle::Creative;
    let s2 = s;
    let s3 = s; // Copy should work
    assert_eq!(format!("{s2:?}"), "Creative");
    assert_eq!(format!("{s3:?}"), "Creative");
}

#[test]
fn emotional_state_copy() {
    let es = EmotionalState::Curious;
    let es2 = es;
    let es3 = es;
    assert_eq!(format!("{es2:?}"), "Curious");
    assert_eq!(format!("{es3:?}"), "Curious");
}

#[test]
fn context_injection_copy() {
    let ci = ContextInjectionStrategy::Full;
    let ci2 = ci;
    let ci3 = ci;
    assert_eq!(format!("{ci2:?}"), "Full");
    assert_eq!(format!("{ci3:?}"), "Full");
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary / edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn personality_state_extreme_values() {
    let ps = PersonalityState {
        current_mood: -1.0,
        energy_level: 0.0,
        confidence: 0.0,
        trust_level: 0.0,
        emotional_state: EmotionalState::Sad,
        personality_drift: HashMap::new(),
        recent_influences: Vec::new(),
    };
    assert!((ps.current_mood - (-1.0)).abs() < f32::EPSILON);
    assert!((ps.energy_level - 0.0).abs() < f32::EPSILON);
}

#[test]
fn personality_state_max_values() {
    let ps = PersonalityState {
        current_mood: 1.0,
        energy_level: 1.0,
        confidence: 1.0,
        trust_level: 1.0,
        emotional_state: EmotionalState::Excited,
        personality_drift: HashMap::new(),
        recent_influences: Vec::new(),
    };
    assert!((ps.current_mood - 1.0).abs() < f32::EPSILON);
    assert!((ps.energy_level - 1.0).abs() < f32::EPSILON);
    assert!((ps.confidence - 1.0).abs() < f32::EPSILON);
    assert!((ps.trust_level - 1.0).abs() < f32::EPSILON);
}

#[test]
fn persona_llm_config_custom_values() {
    let c = PersonaLlmConfig {
        temperature: 1.5,
        top_p: 0.5,
        max_tokens: 1024,
        personality_factors: HashMap::new(),
        response_style: ResponseStyle::Formal,
        context_window_size: 4096,
    };
    assert!((c.temperature - 1.5).abs() < f32::EPSILON);
    assert!((c.top_p - 0.5).abs() < f32::EPSILON);
    assert_eq!(c.max_tokens, 1024);
    assert_eq!(c.context_window_size, 4096);
    assert!(c.personality_factors.is_empty());
}

#[test]
fn adaptation_data_with_populated_fields() {
    let mut topics = HashMap::new();
    topics.insert("combat".into(), 0.9);
    topics.insert("trading".into(), 0.3);
    let ad = AdaptationData {
        interaction_count: 500,
        successful_interactions: 450,
        learning_rate: 0.05,
        preferred_topics: topics,
        topics_to_avoid: vec!["politics".into()],
        player_patterns: PlayerPatterns::default(),
        adaptation_history: vec![AdaptationEvent {
            trigger: "quest_complete".into(),
            changes: HashMap::new(),
            timestamp: 1000,
            success_rating: Some(1.0),
        }],
    };
    assert_eq!(ad.interaction_count, 500);
    assert_eq!(ad.successful_interactions, 450);
    assert!((ad.learning_rate - 0.05).abs() < f32::EPSILON);
    assert_eq!(ad.preferred_topics.len(), 2);
    assert_eq!(ad.topics_to_avoid.len(), 1);
    assert_eq!(ad.topics_to_avoid[0], "politics");
    assert_eq!(ad.adaptation_history.len(), 1);
}

#[test]
fn memory_retrieval_custom_categories() {
    let mr = MemoryRetrievalSettings {
        max_memories: 10,
        min_similarity: 0.7,
        priority_categories: vec!["Combat".into(), "Lore".into(), "Quests".into()],
        recency_bonus: 0.5,
    };
    assert_eq!(mr.max_memories, 10);
    assert!((mr.min_similarity - 0.7).abs() < f32::EPSILON);
    assert_eq!(mr.priority_categories.len(), 3);
    assert!((mr.recency_bonus - 0.5).abs() < f32::EPSILON);
}

#[test]
fn consolidation_preferences_custom() {
    let cp = ConsolidationPreferences {
        consolidation_frequency: 50,
        importance_threshold: 0.5,
        max_memories: 5000,
        forgetting_curve: ForgettingCurve {
            decay_rate: 0.2,
            importance_multiplier: 3.0,
            rehearsal_bonus: 1.0,
        },
    };
    assert_eq!(cp.consolidation_frequency, 50);
    assert!((cp.importance_threshold - 0.5).abs() < f32::EPSILON);
    assert_eq!(cp.max_memories, 5000);
    assert!((cp.forgetting_curve.decay_rate - 0.2).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// Full LlmPersona struct construction
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn llm_persona_construction_and_json_roundtrip() {
    let persona = LlmPersona {
        base: astraweave_memory::Persona {
            name: "Merchant".into(),
            voice: "cheerful".into(),
            tone: "friendly".into(),
            risk: "low".into(),
            humor: "witty".into(),
            backstory: "A traveling merchant".into(),
            likes: vec![],
            dislikes: vec![],
            goals: vec![],
        },
        llm_config: PersonaLlmConfig::default(),
        personality_state: PersonalityState::default(),
        adaptation: AdaptationData::default(),
        prompt_settings: PromptSettings::default(),
        memory_profile: MemoryProfile::default(),
    };
    assert_eq!(persona.base.name, "Merchant");
    assert_eq!(persona.base.voice, "cheerful");
    assert!((persona.llm_config.temperature - 0.8).abs() < f32::EPSILON);

    let json = serde_json::to_string(&persona).unwrap();
    let back: LlmPersona = serde_json::from_str(&json).unwrap();
    assert_eq!(back.base.name, "Merchant");
    assert!((back.personality_state.energy_level - 0.7).abs() < f32::EPSILON);
}

#[test]
fn llm_persona_debug_trait() {
    let persona = LlmPersona {
        base: astraweave_memory::Persona {
            name: "Test".into(),
            voice: "".into(),
            tone: "".into(),
            risk: "".into(),
            humor: "".into(),
            backstory: "".into(),
            likes: vec![],
            dislikes: vec![],
            goals: vec![],
        },
        llm_config: PersonaLlmConfig::default(),
        personality_state: PersonalityState::default(),
        adaptation: AdaptationData::default(),
        prompt_settings: PromptSettings::default(),
        memory_profile: MemoryProfile::default(),
    };
    let dbg = format!("{persona:?}");
    assert!(dbg.contains("LlmPersona"));
}

// ═══════════════════════════════════════════════════════════════════════════
// load_persona_zip error path
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn load_persona_zip_nonexistent_file() {
    let result = astraweave_persona::load_persona_zip("/nonexistent/path.zip");
    assert!(result.is_err());
}

#[test]
fn load_persona_zip_invalid_file() {
    // Create a temp file that's not a valid zip
    let dir = std::env::temp_dir().join("astraweave_persona_test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("not_a_zip.zip");
    std::fs::write(&path, b"not a zip file").unwrap();
    let result = astraweave_persona::load_persona_zip(path.to_str().unwrap());
    assert!(result.is_err());
    let _ = std::fs::remove_file(&path);
}

// ═══════════════════════════════════════════════════════════════════════════
// Mutation kill tests — LlmPersonaManager async methods
// ═══════════════════════════════════════════════════════════════════════════

async fn create_kill_test_manager() -> LlmPersonaManager {
    let mut base_persona = BasePersona::default();
    base_persona.voice = "mystic_oracle".to_string();
    let llm_client = Arc::new(MockLlm);
    let embedding_client = Arc::new(MockEmbeddingClient::new());
    let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
    let rag_pipeline = RagPipeline::new(
        embedding_client.clone(),
        vector_store,
        Some(llm_client.clone()),
        RagConfig::default(),
    );
    LlmPersonaManager::new(base_persona, llm_client, rag_pipeline, embedding_client)
        .await
        .unwrap()
}

/// Kill: get_persona_name → String::new() and → "xyzzy"
#[tokio::test]
async fn get_persona_name_returns_voice() {
    let manager = create_kill_test_manager().await;
    let name = manager.get_persona_name().await;
    assert_eq!(name, "mystic_oracle");
}

/// Kill: evolve_personality || → && (lines 821, 825)
/// With single keyword, || triggers evolution but && would not  
#[tokio::test]
async fn evolve_personality_single_keyword_triggers() {
    let manager = create_kill_test_manager().await;
    manager.evolve_personality("creative").await.unwrap();
    let state = manager.get_persona_state().await;
    let creativity = state
        .llm_config
        .personality_factors
        .get("creativity")
        .unwrap();
    assert!(
        *creativity > 0.7,
        "creativity should increase from 0.7, got {creativity}"
    );
}

/// Kill: evolve_personality || → && — second branch (help/support/care)
#[tokio::test]
async fn evolve_personality_help_triggers_empathy() {
    let manager = create_kill_test_manager().await;
    manager.evolve_personality("help").await.unwrap();
    let state = manager.get_persona_state().await;
    let empathy = state.llm_config.personality_factors.get("empathy").unwrap();
    assert!(
        *empathy > 0.8,
        "empathy should increase from 0.8, got {empathy}"
    );
}

/// Kill: update_personality_state * → + on mood_change (line 696)
/// After one "great" interaction, mood_change = 1 * 0.1 = 0.1
/// With * → +, mood_change = 1 + 0.1 = 1.1 (clamped to 1.0) — detectable!
#[tokio::test]
async fn mood_change_uses_correct_scaling() {
    let manager = create_kill_test_manager().await;
    manager.generate_response("great", None).await.unwrap();
    let state = manager.get_persona_state().await;
    // 1 positive word * 0.1 = 0.1 mood change from 0.0 → 0.1
    // If * → +, 1 + 0.1 = 1.1, clamped to 1.0
    assert!(
        state.personality_state.current_mood < 0.5,
        "mood should be small (~0.1), got {}",
        state.personality_state.current_mood
    );
}

/// Kill: update_personality_state > → >= on positive/negative count comparison
/// When positive_count == negative_count, confidence must stay unchanged.
/// Mutation > → >= causes the first or second branch to fire on ties.
#[tokio::test]
async fn equal_sentiment_leaves_confidence_unchanged() {
    let manager = create_kill_test_manager().await;
    let initial = manager
        .get_persona_state()
        .await
        .personality_state
        .confidence;
    // "good" = 1 positive, "bad" = 1 negative → equal counts
    manager.generate_response("good bad", None).await.unwrap();
    let state = manager.get_persona_state().await;
    // With >, equal counts → neither branch fires → confidence unchanged
    // With >= on first branch → confidence += 0.02 (DETECTED)
    // With >= on second branch → confidence -= 0.02 (DETECTED)
    assert!(
        (state.personality_state.confidence - initial).abs() < 0.001,
        "equal sentiment should not change confidence: got {}, wanted {}",
        state.personality_state.confidence,
        initial
    );
}

/// Kill: update_personality_state > → < on negative_count > positive_count (L716)
/// Negative-only input must decrease confidence. Mutation > → < would skip the branch.
#[tokio::test]
async fn negative_input_decreases_confidence() {
    let manager = create_kill_test_manager().await;
    let initial = manager
        .get_persona_state()
        .await
        .personality_state
        .confidence;
    // "terrible" has 1 negative word, 0 positive → negative_count > positive_count
    manager.generate_response("terrible", None).await.unwrap();
    let state = manager.get_persona_state().await;
    assert!(
        state.personality_state.confidence < initial,
        "negative input should decrease confidence: got {}, started {}",
        state.personality_state.confidence,
        initial
    );
}

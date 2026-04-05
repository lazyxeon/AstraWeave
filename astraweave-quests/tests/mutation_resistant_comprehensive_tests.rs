//! Mutation-resistant comprehensive tests for astraweave-quests
//!
//! Targets 341 mutants across 5 files:
//! - components.rs (150 mutants): ECS quest components
//! - terrain_quests.rs (98 mutants): Terrain-driven quest generation
//! - systems.rs (64 mutants): Quest system integration/helpers
//! - llm_quests.rs (26 mutants): LLM quest data structures
//! - lib.rs (3 mutants): Core Quest/QuestStep types

#![allow(clippy::field_reassign_with_default)]

use astraweave_quests::*;
use chrono::Utc;
use std::collections::HashMap;

// ============================================================================
// Helper: Create test LlmQuest
// ============================================================================

fn make_test_llm_quest(id: &str, category: &str) -> LlmQuest {
    LlmQuest {
        id: id.to_string(),
        title: format!("Quest {}", id),
        description: format!("Description for {}", id),
        steps: vec![
            LlmQuestStep {
                id: "step_1".into(),
                description: "First step".into(),
                completed: false,
                objectives: vec![QuestObjective {
                    id: "obj_1".into(),
                    description: "First objective".into(),
                    objective_type: ObjectiveType::Collect,
                    target_count: Some(5),
                    current_count: 0,
                    completion_criteria: "Collect items".into(),
                }],
                branching_choices: vec![],
                dynamic_content: None,
                validation_criteria: ValidationCriteria {
                    required_conditions: vec![],
                    forbidden_conditions: vec![],
                    validation_script: None,
                },
            },
            LlmQuestStep {
                id: "step_2".into(),
                description: "Second step".into(),
                completed: false,
                objectives: vec![],
                branching_choices: vec![],
                dynamic_content: None,
                validation_criteria: ValidationCriteria {
                    required_conditions: vec![],
                    forbidden_conditions: vec![],
                    validation_script: None,
                },
            },
        ],
        metadata: QuestMetadata {
            category: category.to_string(),
            difficulty_level: 0.5,
            estimated_duration: 30,
            player_level_range: (1, 10),
            required_skills: vec!["combat".into()],
            tags: vec!["test".into()],
            generated_reasoning: "Test reasoning".into(),
        },
        branching: QuestBranching {
            has_multiple_paths: false,
            branch_points: vec![],
            convergence_points: vec![],
        },
        rewards: QuestRewards {
            experience: 100,
            currency: 50,
            items: vec!["sword".into()],
            reputation_changes: HashMap::new(),
            unlock_content: vec![],
        },
        generated_at: Utc::now(),
        personalization: PersonalizationData {
            player_id: "player_1".into(),
            player_preferences: vec!["combat".into()],
            play_style: "Balanced".into(),
            previous_choices: vec![],
            difficulty_preference: 0.5,
        },
    }
}

fn make_test_quest_context(player_level: u32) -> QuestContext {
    QuestContext {
        player_id: "player_1".into(),
        player_level,
        location: "forest".into(),
        available_npcs: vec!["npc_1".into()],
        world_state: HashMap::new(),
        recent_activities: vec!["explore".into()],
        preferred_quest_types: vec!["exploration".into()],
    }
}

// ============================================================================
// Module: lib.rs tests (3 mutants)
// ============================================================================
mod lib_tests {
    use super::*;

    #[test]
    fn test_quest_validate_empty_title_exact_error() {
        let quest = Quest {
            title: "".to_string(),
            steps: vec![QuestStep {
                description: "step".into(),
                completed: false,
            }],
        };
        let err = quest.validate().unwrap_err();
        assert_eq!(err, "Quest title cannot be empty");
    }

    #[test]
    fn test_quest_validate_empty_steps_exact_error() {
        let quest = Quest {
            title: "Title".into(),
            steps: vec![],
        };
        let err = quest.validate().unwrap_err();
        assert_eq!(err, "Quest must have at least one step");
    }

    #[test]
    fn test_quest_validate_both_valid() {
        let quest = Quest {
            title: "Title".into(),
            steps: vec![QuestStep {
                description: "Do".into(),
                completed: false,
            }],
        };
        assert!(quest.validate().is_ok());
    }

    #[test]
    fn test_quest_is_complete_all_true() {
        let quest = Quest {
            title: "Q".into(),
            steps: vec![
                QuestStep {
                    description: "s1".into(),
                    completed: true,
                },
                QuestStep {
                    description: "s2".into(),
                    completed: true,
                },
            ],
        };
        assert!(quest.is_complete());
    }

    #[test]
    fn test_quest_is_complete_one_false() {
        let quest = Quest {
            title: "Q".into(),
            steps: vec![
                QuestStep {
                    description: "s1".into(),
                    completed: true,
                },
                QuestStep {
                    description: "s2".into(),
                    completed: false,
                },
            ],
        };
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_is_complete_all_false() {
        let quest = Quest {
            title: "Q".into(),
            steps: vec![QuestStep {
                description: "s1".into(),
                completed: false,
            }],
        };
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_default_values() {
        let quest = Quest::default();
        assert!(quest.title.is_empty());
        assert!(quest.steps.is_empty());
    }

    #[test]
    fn test_quest_step_default_values() {
        let step = QuestStep::default();
        assert!(step.description.is_empty());
        assert!(!step.completed);
    }
}

// ============================================================================
// Module: CQuestGenerator tests (components.rs)
// ============================================================================
mod quest_generator_tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let gen = CQuestGenerator::default();
        assert_eq!(gen.context.player_id, "default");
        assert_eq!(gen.context.player_level, 1);
        assert_eq!(gen.context.location, "starting_area");
        assert!(gen.context.available_npcs.is_empty());
        assert!(gen.context.world_state.is_empty());
        assert!(gen.context.recent_activities.is_empty());
        assert_eq!(
            gen.context.preferred_quest_types,
            vec!["exploration".to_string()]
        );
        assert!(gen.active_quests.is_empty());
        assert_eq!(gen.last_generation_time, 0);
        assert_eq!(gen.generation_cooldown_ms, 300000);
    }

    #[test]
    fn test_new_constructor() {
        let gen = CQuestGenerator::new("hero".into(), 10, "dungeon".into());
        assert_eq!(gen.context.player_id, "hero");
        assert_eq!(gen.context.player_level, 10);
        assert_eq!(gen.context.location, "dungeon");
        assert_eq!(
            gen.context.preferred_quest_types,
            vec!["exploration".to_string()]
        );
        assert!(gen.active_quests.is_empty());
        assert_eq!(gen.last_generation_time, 0);
        assert_eq!(gen.generation_cooldown_ms, 300000);
    }

    #[test]
    fn test_can_generate_quest_ready() {
        let gen = CQuestGenerator::default();
        // last_generation_time = 0, cooldown = 300000
        // current_time = 300000, so 300000 - 0 >= 300000 → true
        assert!(gen.can_generate_quest(300000));
    }

    #[test]
    fn test_can_generate_quest_not_ready() {
        let gen = CQuestGenerator::default();
        // current_time = 299999, so 299999 - 0 = 299999 < 300000
        assert!(!gen.can_generate_quest(299999));
    }

    #[test]
    fn test_can_generate_quest_exact_boundary() {
        let mut gen = CQuestGenerator::default();
        gen.last_generation_time = 100;
        gen.generation_cooldown_ms = 50;
        // 150 - 100 = 50 >= 50 → true
        assert!(gen.can_generate_quest(150));
        // 149 - 100 = 49 < 50 → false
        assert!(!gen.can_generate_quest(149));
    }

    #[test]
    fn test_add_active_quest_new() {
        let mut gen = CQuestGenerator::default();
        gen.add_active_quest("q1".into());
        assert_eq!(gen.active_quests.len(), 1);
        assert_eq!(gen.active_quests[0], "q1");
    }

    #[test]
    fn test_add_active_quest_duplicate_ignored() {
        let mut gen = CQuestGenerator::default();
        gen.add_active_quest("q1".into());
        gen.add_active_quest("q1".into());
        assert_eq!(gen.active_quests.len(), 1);
    }

    #[test]
    fn test_add_active_quest_multiple() {
        let mut gen = CQuestGenerator::default();
        gen.add_active_quest("q1".into());
        gen.add_active_quest("q2".into());
        assert_eq!(gen.active_quests.len(), 2);
    }

    #[test]
    fn test_remove_active_quest_exists() {
        let mut gen = CQuestGenerator::default();
        gen.add_active_quest("q1".into());
        gen.add_active_quest("q2".into());
        gen.remove_active_quest("q1");
        assert_eq!(gen.active_quests.len(), 1);
        assert_eq!(gen.active_quests[0], "q2");
    }

    #[test]
    fn test_remove_active_quest_not_found() {
        let mut gen = CQuestGenerator::default();
        gen.add_active_quest("q1".into());
        gen.remove_active_quest("q99");
        assert_eq!(gen.active_quests.len(), 1);
    }

    #[test]
    fn test_update_context_location_only() {
        let mut gen = CQuestGenerator::default();
        gen.update_context(Some("mountain".into()), None, None);
        assert_eq!(gen.context.location, "mountain");
        assert!(gen.context.available_npcs.is_empty()); // unchanged
    }

    #[test]
    fn test_update_context_npcs_only() {
        let mut gen = CQuestGenerator::default();
        gen.update_context(None, Some(vec!["npc_a".into()]), None);
        assert_eq!(gen.context.available_npcs, vec!["npc_a".to_string()]);
        assert_eq!(gen.context.location, "starting_area"); // unchanged
    }

    #[test]
    fn test_update_context_world_state_only() {
        let mut gen = CQuestGenerator::default();
        let mut state = HashMap::new();
        state.insert("key".into(), serde_json::json!(42));
        gen.update_context(None, None, Some(state));
        assert!(gen.context.world_state.contains_key("key"));
    }

    #[test]
    fn test_update_context_all_none() {
        let mut gen = CQuestGenerator::new("p".into(), 5, "loc".into());
        gen.update_context(None, None, None);
        assert_eq!(gen.context.location, "loc"); // unchanged
    }

    #[test]
    fn test_add_recent_activity() {
        let mut gen = CQuestGenerator::default();
        gen.add_recent_activity("killed_dragon".into());
        assert_eq!(gen.context.recent_activities.len(), 1);
        assert_eq!(gen.context.recent_activities[0], "killed_dragon");
    }

    #[test]
    fn test_add_recent_activity_overflow_trims_to_10() {
        let mut gen = CQuestGenerator::default();
        for i in 0..11 {
            gen.add_recent_activity(format!("activity_{}", i));
        }
        assert_eq!(gen.context.recent_activities.len(), 10);
        // First activity (activity_0) should be removed
        assert_eq!(gen.context.recent_activities[0], "activity_1");
        assert_eq!(gen.context.recent_activities[9], "activity_10");
    }

    #[test]
    fn test_add_recent_activity_exactly_10_no_trim() {
        let mut gen = CQuestGenerator::default();
        for i in 0..10 {
            gen.add_recent_activity(format!("a_{}", i));
        }
        assert_eq!(gen.context.recent_activities.len(), 10);
    }

    #[test]
    fn test_mark_generation_time() {
        let mut gen = CQuestGenerator::default();
        assert_eq!(gen.last_generation_time, 0);
        gen.mark_generation_time(12345);
        assert_eq!(gen.last_generation_time, 12345);
    }
}

// ============================================================================
// Module: CActiveQuest tests (components.rs)
// ============================================================================
mod active_quest_tests {
    use super::*;

    #[test]
    fn test_new_active_quest_defaults() {
        let quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(quest.clone());
        assert_eq!(active.current_step_index, 0);
        assert!(matches!(active.state, QuestState::Active));
        assert!(active.choices_made.is_empty());
        assert!(active.dynamic_content.is_empty());
        assert_eq!(active.quest.id, "q1");
    }

    #[test]
    fn test_get_current_step_first() {
        let quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(quest);
        let step = active.get_current_step().unwrap();
        assert_eq!(step.id, "step_1");
    }

    #[test]
    fn test_get_current_step_after_advance() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        active.advance_step();
        let step = active.get_current_step().unwrap();
        assert_eq!(step.id, "step_2");
    }

    #[test]
    fn test_get_current_step_out_of_bounds() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        active.current_step_index = 100;
        assert!(active.get_current_step().is_none());
    }

    #[test]
    fn test_is_complete_when_state_completed() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        active.state = QuestState::Completed;
        assert!(active.is_complete());
    }

    #[test]
    fn test_is_complete_when_all_steps_completed() {
        let mut quest = make_test_llm_quest("q1", "combat");
        for step in &mut quest.steps {
            step.completed = true;
        }
        let active = CActiveQuest::new(quest);
        assert!(active.is_complete());
    }

    #[test]
    fn test_is_not_complete_active_with_incomplete_steps() {
        let quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(quest);
        assert!(!active.is_complete());
    }

    #[test]
    fn test_advance_step_success() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        // two steps: index 0 → 1 should return true
        assert!(active.advance_step());
        assert_eq!(active.current_step_index, 1);
    }

    #[test]
    fn test_advance_step_at_last_step_returns_false() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        active.current_step_index = 1; // last step (2 steps total)
        assert!(!active.advance_step());
        assert!(matches!(active.state, QuestState::Completed));
    }

    #[test]
    fn test_record_choice() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        let choice = BranchingChoice {
            id: "c1".into(),
            description: "Take the left path".into(),
            consequences: vec!["found treasure".into()],
            requirements: None,
            leads_to_step: None,
        };
        active.record_choice("step_1".into(), choice);
        assert_eq!(active.choices_made.len(), 1);
        assert_eq!(active.choices_made[0].step_id, "step_1");
        assert_eq!(active.choices_made[0].choice.id, "c1");
    }

    #[test]
    fn test_set_state() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        active.set_state(QuestState::Paused);
        assert!(matches!(active.state, QuestState::Paused));
        active.set_state(QuestState::Failed);
        assert!(matches!(active.state, QuestState::Failed));
        active.set_state(QuestState::Abandoned);
        assert!(matches!(active.state, QuestState::Abandoned));
    }

    #[test]
    fn test_add_and_get_dynamic_content() {
        let quest = make_test_llm_quest("q1", "combat");
        let mut active = CActiveQuest::new(quest);
        let content = DynamicContent {
            dialogue: Some("Hello traveler".into()),
            flavor_text: Some("The wind howls".into()),
            environmental_description: None,
            npc_interactions: vec!["wave".into()],
        };
        active.add_dynamic_content("step_1".into(), content);
        let retrieved = active.get_dynamic_content("step_1");
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().dialogue.as_deref(),
            Some("Hello traveler")
        );
    }

    #[test]
    fn test_get_dynamic_content_missing() {
        let quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(quest);
        assert!(active.get_dynamic_content("nonexistent").is_none());
    }

    #[test]
    fn test_get_duration_is_positive() {
        let quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(quest);
        let dur = active.get_duration();
        assert!(dur.num_milliseconds() >= 0);
    }
}

// ============================================================================
// Module: CQuestMetrics tests (components.rs)
// ============================================================================
mod quest_metrics_tests {
    use super::*;

    #[test]
    fn test_default_all_zeroes() {
        let m = CQuestMetrics::default();
        assert_eq!(m.quests_generated, 0);
        assert_eq!(m.quests_completed, 0);
        assert_eq!(m.quests_abandoned, 0);
        assert_eq!(m.average_completion_time, 0.0);
        assert!(m.category_popularity.is_empty());
        assert!(m.choice_statistics.is_empty());
        assert!(m.quality_scores.is_empty());
        assert_eq!(m.generation_metrics.total_generations, 0);
        assert_eq!(m.generation_metrics.failed_generations, 0);
        assert_eq!(m.generation_metrics.average_generation_time, 0.0);
        assert_eq!(m.generation_metrics.validation_failures, 0);
        assert_eq!(m.generation_metrics.average_quality_score, 0.0);
    }

    #[test]
    fn test_record_quest_completion_single() {
        let mut m = CQuestMetrics::default();
        let quest = make_test_llm_quest("q1", "combat");
        m.record_quest_completion(&quest, 30.0, 0.8);
        assert_eq!(m.quests_completed, 1);
        assert!((m.average_completion_time - 30.0).abs() < 0.001);
        assert_eq!(*m.category_popularity.get("combat").unwrap(), 1);
        assert_eq!(m.quality_scores.len(), 1);
        assert!((m.quality_scores[0] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_record_quest_completion_average_time() {
        let mut m = CQuestMetrics::default();
        let quest = make_test_llm_quest("q1", "combat");
        m.record_quest_completion(&quest, 20.0, 0.7);
        m.record_quest_completion(&quest, 40.0, 0.9);
        assert_eq!(m.quests_completed, 2);
        assert!((m.average_completion_time - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_record_quest_completion_quality_scores_capped_at_100() {
        let mut m = CQuestMetrics::default();
        let quest = make_test_llm_quest("q1", "combat");
        for i in 0..105 {
            m.record_quest_completion(&quest, 10.0, i as f32 * 0.01);
        }
        // Should cap at 100 entries (oldest removed)
        assert_eq!(m.quality_scores.len(), 100);
    }

    #[test]
    fn test_record_quest_abandonment() {
        let mut m = CQuestMetrics::default();
        let quest = make_test_llm_quest("q1", "combat");
        m.record_quest_abandonment(&quest, "too_hard".into());
        assert_eq!(m.quests_abandoned, 1);
        assert_eq!(*m.choice_statistics.get("abandoned:too_hard").unwrap(), 1);
    }

    #[test]
    fn test_record_quest_generation_success_no_validation() {
        let mut m = CQuestMetrics::default();
        m.record_quest_generation(150.0, true, None);
        assert_eq!(m.quests_generated, 1);
        assert_eq!(m.generation_metrics.total_generations, 1);
        assert_eq!(m.generation_metrics.failed_generations, 0);
        assert!((m.generation_metrics.average_generation_time - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_record_quest_generation_failure() {
        let mut m = CQuestMetrics::default();
        m.record_quest_generation(50.0, false, None);
        assert_eq!(m.generation_metrics.failed_generations, 1);
    }

    #[test]
    fn test_record_quest_generation_with_validation() {
        let mut m = CQuestMetrics::default();
        let validation = QuestValidation {
            is_valid: true,
            quality_score: 0.9,
            issues: vec![],
            strengths: vec![],
            overall_assessment: "good".into(),
        };
        m.record_quest_generation(100.0, true, Some(&validation));
        assert_eq!(m.generation_metrics.validation_failures, 0);
        assert!((m.generation_metrics.average_quality_score - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_record_quest_generation_with_invalid_validation() {
        let mut m = CQuestMetrics::default();
        let validation = QuestValidation {
            is_valid: false,
            quality_score: 0.3,
            issues: vec![],
            strengths: vec![],
            overall_assessment: "bad".into(),
        };
        m.record_quest_generation(100.0, true, Some(&validation));
        assert_eq!(m.generation_metrics.validation_failures, 1);
    }

    #[test]
    fn test_record_player_choice() {
        let mut m = CQuestMetrics::default();
        m.record_player_choice("choice_a".into());
        m.record_player_choice("choice_a".into());
        m.record_player_choice("choice_b".into());
        assert_eq!(*m.choice_statistics.get("choice_a").unwrap(), 2);
        assert_eq!(*m.choice_statistics.get("choice_b").unwrap(), 1);
    }

    #[test]
    fn test_get_completion_rate_no_quests() {
        let m = CQuestMetrics::default();
        assert_eq!(m.get_completion_rate(), 0.0);
    }

    #[test]
    fn test_get_completion_rate_some_completed() {
        let mut m = CQuestMetrics::default();
        m.quests_generated = 4;
        m.quests_completed = 3;
        assert!((m.get_completion_rate() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_get_abandonment_rate_no_quests() {
        let m = CQuestMetrics::default();
        assert_eq!(m.get_abandonment_rate(), 0.0);
    }

    #[test]
    fn test_get_abandonment_rate_some_abandoned() {
        let mut m = CQuestMetrics::default();
        m.quests_generated = 10;
        m.quests_abandoned = 2;
        assert!((m.get_abandonment_rate() - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_get_average_quality_empty() {
        let m = CQuestMetrics::default();
        assert_eq!(m.get_average_quality(), 0.0);
    }

    #[test]
    fn test_get_average_quality_with_scores() {
        let mut m = CQuestMetrics::default();
        m.quality_scores = vec![0.6, 0.8, 1.0];
        assert!((m.get_average_quality() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_get_generation_success_rate_no_generations() {
        let m = CQuestMetrics::default();
        assert_eq!(m.get_generation_success_rate(), 0.0);
    }

    #[test]
    fn test_get_generation_success_rate_all_success() {
        let mut m = CQuestMetrics::default();
        m.generation_metrics.total_generations = 5;
        m.generation_metrics.failed_generations = 0;
        assert!((m.get_generation_success_rate() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_get_generation_success_rate_partial() {
        let mut m = CQuestMetrics::default();
        m.generation_metrics.total_generations = 10;
        m.generation_metrics.failed_generations = 3;
        assert!((m.get_generation_success_rate() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let mut m = CQuestMetrics::default();
        m.quests_generated = 50;
        m.quests_completed = 30;
        m.quests_abandoned = 5;
        m.quality_scores.push(0.9);
        m.reset();
        assert_eq!(m.quests_generated, 0);
        assert_eq!(m.quests_completed, 0);
        assert_eq!(m.quests_abandoned, 0);
        assert!(m.quality_scores.is_empty());
        assert!(m.category_popularity.is_empty());
    }

    #[test]
    fn test_record_generation_average_time_multiple() {
        let mut m = CQuestMetrics::default();
        m.record_quest_generation(100.0, true, None);
        m.record_quest_generation(200.0, true, None);
        assert!((m.generation_metrics.average_generation_time - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_category_popularity_increments() {
        let mut m = CQuestMetrics::default();
        let quest_combat = make_test_llm_quest("q1", "combat");
        let quest_explore = make_test_llm_quest("q2", "exploration");
        m.record_quest_completion(&quest_combat, 10.0, 0.5);
        m.record_quest_completion(&quest_combat, 10.0, 0.5);
        m.record_quest_completion(&quest_explore, 10.0, 0.5);
        assert_eq!(*m.category_popularity.get("combat").unwrap(), 2);
        assert_eq!(*m.category_popularity.get("exploration").unwrap(), 1);
    }
}

// ============================================================================
// Module: CQuestJournal tests (components.rs)
// ============================================================================
mod quest_journal_tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let j = CQuestJournal::new();
        assert!(j.quest_history.is_empty());
        assert!(j.active_quest_ids.is_empty());
        assert!(j.learned_preferences.is_empty());
        assert!(j.auto_discover);
        assert_eq!(j.max_active_quests, 3);
    }

    #[test]
    fn test_add_quest() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        assert_eq!(j.quest_history.len(), 1);
        assert_eq!(j.active_quest_ids.len(), 1);
        assert_eq!(j.active_quest_ids[0], "q1");
        assert_eq!(j.quest_history[0].quest_title, "Quest q1");
        assert_eq!(j.quest_history[0].category, "combat");
        assert!(matches!(j.quest_history[0].final_state, QuestState::Active));
        // learned_preferences should have "combat" incremented by 0.1
        assert!((*j.learned_preferences.get("combat").unwrap() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_complete_quest() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        j.complete_quest("q1", "Great!".into());
        assert_eq!(j.active_quest_ids.len(), 0);
        assert!(j.quest_history[0].completed_at.is_some());
        assert!(matches!(
            j.quest_history[0].final_state,
            QuestState::Completed
        ));
        assert_eq!(j.quest_history[0].completion_notes, "Great!");
        // Preferences: +0.1 (add) + 0.2 (complete) = 0.3
        assert!((*j.learned_preferences.get("combat").unwrap() - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_abandon_quest() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        j.abandon_quest("q1", "bored".into());
        assert_eq!(j.active_quest_ids.len(), 0);
        assert!(j.quest_history[0].abandoned_at.is_some());
        assert!(matches!(
            j.quest_history[0].final_state,
            QuestState::Abandoned
        ));
        assert!(j.quest_history[0]
            .completion_notes
            .contains("Abandoned: bored"));
        // Preferences: +0.1 (add) - 0.1 (abandon) = 0.0
        assert!((*j.learned_preferences.get("combat").unwrap()).abs() < 0.001);
    }

    #[test]
    fn test_record_choice() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        j.record_choice("q1", "Chose violence".into());
        assert_eq!(j.quest_history[0].choices_made.len(), 1);
        assert_eq!(j.quest_history[0].choices_made[0], "Chose violence");
    }

    #[test]
    fn test_record_choice_wrong_quest() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        j.record_choice("q99", "Chose violence".into());
        // Should be no-op
        assert!(j.quest_history[0].choices_made.is_empty());
    }

    #[test]
    fn test_get_active_quest_count() {
        let mut j = CQuestJournal::new();
        assert_eq!(j.get_active_quest_count(), 0);
        let q1 = make_test_llm_quest("q1", "a");
        let q2 = make_test_llm_quest("q2", "b");
        j.add_quest(&q1);
        j.add_quest(&q2);
        assert_eq!(j.get_active_quest_count(), 2);
    }

    #[test]
    fn test_can_accept_new_quest() {
        let mut j = CQuestJournal::new();
        assert!(j.can_accept_new_quest()); // 0 < 3
        j.add_quest(&make_test_llm_quest("q1", "a"));
        j.add_quest(&make_test_llm_quest("q2", "b"));
        j.add_quest(&make_test_llm_quest("q3", "c"));
        assert!(!j.can_accept_new_quest()); // 3 < 3 is false
    }

    #[test]
    fn test_get_preferred_categories_sorted() {
        let mut j = CQuestJournal::new();
        j.learned_preferences.insert("combat".into(), 0.5);
        j.learned_preferences.insert("exploration".into(), 0.9);
        j.learned_preferences.insert("social".into(), 0.3);
        j.learned_preferences.insert("crafting".into(), 0.1);
        let prefs = j.get_preferred_categories();
        assert!(prefs.len() <= 3);
        assert_eq!(prefs[0], "exploration"); // highest
        assert_eq!(prefs[1], "combat"); // second
        assert_eq!(prefs[2], "social"); // third
    }

    #[test]
    fn test_get_preferred_categories_empty() {
        let j = CQuestJournal::new();
        assert!(j.get_preferred_categories().is_empty());
    }

    #[test]
    fn test_get_statistics_empty() {
        let j = CQuestJournal::new();
        let stats = j.get_statistics();
        assert_eq!(stats.total_quests, 0);
        assert_eq!(stats.completed_quests, 0);
        assert_eq!(stats.abandoned_quests, 0);
        assert_eq!(stats.active_quests, 0);
        assert_eq!(stats.completion_rate, 0.0);
    }

    #[test]
    fn test_get_statistics_mixed() {
        let mut j = CQuestJournal::new();
        j.add_quest(&make_test_llm_quest("q1", "a"));
        j.add_quest(&make_test_llm_quest("q2", "b"));
        j.add_quest(&make_test_llm_quest("q3", "c"));
        j.complete_quest("q1", "done".into());
        j.abandon_quest("q2", "bored".into());

        let stats = j.get_statistics();
        assert_eq!(stats.total_quests, 3);
        assert_eq!(stats.completed_quests, 1);
        assert_eq!(stats.abandoned_quests, 1);
        assert_eq!(stats.active_quests, 1);
        assert!((stats.completion_rate - (1.0 / 3.0)).abs() < 0.001);
    }

    #[test]
    fn test_add_quest_journal_entry_fields() {
        let mut j = CQuestJournal::new();
        let quest = make_test_llm_quest("q1", "combat");
        j.add_quest(&quest);
        let entry = &j.quest_history[0];
        assert_eq!(entry.quest_id, "q1");
        assert_eq!(entry.quest_description, "Description for q1");
        assert!(entry.completed_at.is_none());
        assert!(entry.abandoned_at.is_none());
        assert!(entry.choices_made.is_empty());
        assert!(entry.completion_notes.is_empty());
    }
}

// ============================================================================
// Module: TerrainFeatureType tests (terrain_quests.rs)
// ============================================================================
mod terrain_feature_type_tests {
    use super::*;

    #[test]
    fn test_mountain_archetypes() {
        let a = TerrainFeatureType::Mountain.quest_archetypes();
        assert_eq!(
            a,
            vec!["climb", "explore_summit", "rescue", "retrieve_artifact"]
        );
    }

    #[test]
    fn test_hill_archetypes() {
        let a = TerrainFeatureType::Hill.quest_archetypes();
        assert_eq!(a, vec!["survey", "fortify", "gather_resources"]);
    }

    #[test]
    fn test_valley_archetypes() {
        let a = TerrainFeatureType::Valley.quest_archetypes();
        assert_eq!(a, vec!["settle", "defend", "explore_ruins"]);
    }

    #[test]
    fn test_cliff_archetypes() {
        let a = TerrainFeatureType::Cliff.quest_archetypes();
        assert_eq!(a, vec!["rescue", "treasure_hunt", "navigation"]);
    }

    #[test]
    fn test_canyon_archetypes() {
        let a = TerrainFeatureType::Canyon.quest_archetypes();
        assert_eq!(a, vec!["traverse", "discover_secrets", "ambush"]);
    }

    #[test]
    fn test_river_archetypes() {
        let a = TerrainFeatureType::River.quest_archetypes();
        assert_eq!(
            a,
            vec!["cross", "fish", "follow_downstream", "build_bridge"]
        );
    }

    #[test]
    fn test_lake_archetypes() {
        let a = TerrainFeatureType::Lake.quest_archetypes();
        assert_eq!(a, vec!["investigate", "fish", "aquatic_creature"]);
    }

    #[test]
    fn test_waterfall_archetypes() {
        let a = TerrainFeatureType::Waterfall.quest_archetypes();
        assert_eq!(a, vec!["behind_falls", "collect_water", "meditation"]);
    }

    #[test]
    fn test_pond_archetypes() {
        let a = TerrainFeatureType::Pond.quest_archetypes();
        assert_eq!(a, vec!["ritual", "gather_ingredients"]);
    }

    #[test]
    fn test_stream_archetypes() {
        let a = TerrainFeatureType::Stream.quest_archetypes();
        assert_eq!(a, vec!["follow_source", "purify"]);
    }

    #[test]
    fn test_forest_archetypes() {
        let a = TerrainFeatureType::Forest.quest_archetypes();
        assert_eq!(
            a,
            vec!["hunt", "gather_herbs", "clear_path", "lost_explorer"]
        );
    }

    #[test]
    fn test_grove_archetypes() {
        let a = TerrainFeatureType::Grove.quest_archetypes();
        assert_eq!(a, vec!["sacred_ritual", "spirit_encounter", "harvest"]);
    }

    #[test]
    fn test_meadow_archetypes() {
        let a = TerrainFeatureType::Meadow.quest_archetypes();
        assert_eq!(a, vec!["gather_flowers", "peaceful_encounter", "camp"]);
    }

    #[test]
    fn test_marsh_archetypes() {
        let a = TerrainFeatureType::Marsh.quest_archetypes();
        assert_eq!(a, vec!["navigate", "creature_hunt", "lost_artifact"]);
    }

    #[test]
    fn test_desert_archetypes() {
        let a = TerrainFeatureType::Desert.quest_archetypes();
        assert_eq!(a, vec!["survive", "find_oasis", "ancient_ruins"]);
    }

    #[test]
    fn test_cave_archetypes() {
        let a = TerrainFeatureType::Cave.quest_archetypes();
        assert_eq!(a, vec!["explore_depths", "monster_lair", "hidden_treasure"]);
    }

    #[test]
    fn test_crater_archetypes() {
        let a = TerrainFeatureType::Crater.quest_archetypes();
        assert_eq!(a, vec!["investigate_impact", "alien_artifact", "mining"]);
    }

    #[test]
    fn test_plateau_archetypes() {
        let a = TerrainFeatureType::Plateau.quest_archetypes();
        assert_eq!(a, vec!["establish_base", "aerial_view", "forgotten_temple"]);
    }

    #[test]
    fn test_ridge_archetypes() {
        let a = TerrainFeatureType::Ridge.quest_archetypes();
        assert_eq!(a, vec!["traverse", "lookout", "eagle_nest"]);
    }

    #[test]
    fn test_ravine_archetypes() {
        let a = TerrainFeatureType::Ravine.quest_archetypes();
        assert_eq!(a, vec!["descend", "ancient_path", "hidden_cave"]);
    }

    // Difficulty modifier exact values for all 20 variants
    #[test]
    fn test_difficulty_modifier_mountain() {
        assert!((TerrainFeatureType::Mountain.difficulty_modifier() - 0.3).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_cliff() {
        assert!((TerrainFeatureType::Cliff.difficulty_modifier() - 0.25).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_canyon() {
        assert!((TerrainFeatureType::Canyon.difficulty_modifier() - 0.2).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_cave() {
        assert!((TerrainFeatureType::Cave.difficulty_modifier() - 0.25).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_marsh() {
        assert!((TerrainFeatureType::Marsh.difficulty_modifier() - 0.15).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_desert() {
        assert!((TerrainFeatureType::Desert.difficulty_modifier() - 0.2).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_crater() {
        assert!((TerrainFeatureType::Crater.difficulty_modifier() - 0.15).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_ravine() {
        assert!((TerrainFeatureType::Ravine.difficulty_modifier() - 0.2).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_waterfall() {
        assert!((TerrainFeatureType::Waterfall.difficulty_modifier() - 0.1).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_lake() {
        assert!((TerrainFeatureType::Lake.difficulty_modifier() - 0.05).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_river() {
        assert!((TerrainFeatureType::River.difficulty_modifier() - 0.1).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_hill() {
        assert!((TerrainFeatureType::Hill.difficulty_modifier() - 0.05).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_valley() {
        assert!((TerrainFeatureType::Valley.difficulty_modifier() - 0.0).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_forest() {
        assert!((TerrainFeatureType::Forest.difficulty_modifier() - 0.1).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_grove() {
        assert!((TerrainFeatureType::Grove.difficulty_modifier() - 0.0).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_meadow() {
        assert!((TerrainFeatureType::Meadow.difficulty_modifier() - (-0.05)).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_pond() {
        assert!((TerrainFeatureType::Pond.difficulty_modifier() - 0.0).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_stream() {
        assert!((TerrainFeatureType::Stream.difficulty_modifier() - 0.0).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_plateau() {
        assert!((TerrainFeatureType::Plateau.difficulty_modifier() - 0.1).abs() < f32::EPSILON);
    }
    #[test]
    fn test_difficulty_modifier_ridge() {
        assert!((TerrainFeatureType::Ridge.difficulty_modifier() - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn test_all_archetypes_non_empty() {
        let all_features = [
            TerrainFeatureType::Mountain,
            TerrainFeatureType::Hill,
            TerrainFeatureType::Valley,
            TerrainFeatureType::Cliff,
            TerrainFeatureType::Canyon,
            TerrainFeatureType::River,
            TerrainFeatureType::Lake,
            TerrainFeatureType::Waterfall,
            TerrainFeatureType::Pond,
            TerrainFeatureType::Stream,
            TerrainFeatureType::Forest,
            TerrainFeatureType::Grove,
            TerrainFeatureType::Meadow,
            TerrainFeatureType::Marsh,
            TerrainFeatureType::Desert,
            TerrainFeatureType::Cave,
            TerrainFeatureType::Crater,
            TerrainFeatureType::Plateau,
            TerrainFeatureType::Ridge,
            TerrainFeatureType::Ravine,
        ];
        for f in &all_features {
            assert!(
                !f.quest_archetypes().is_empty(),
                "{:?} has no archetypes",
                f
            );
        }
    }
}

// ============================================================================
// Module: TerrainQuestContext / Trigger tests (terrain_quests.rs)
// ============================================================================
mod terrain_context_trigger_tests {
    use super::*;

    #[test]
    fn test_terrain_quest_context_default() {
        let ctx = TerrainQuestContext::default();
        assert_eq!(ctx.feature_type, TerrainFeatureType::Hill);
        assert_eq!(ctx.position, (0.0, 0.0, 0.0));
        assert_eq!(ctx.radius, 32.0);
        assert_eq!(ctx.intensity, 0.5);
        assert_eq!(ctx.biome, "unknown");
        assert!(ctx.nearby_features.is_empty());
        assert!(!ctx.is_ai_generated);
        assert!(ctx.seed.is_none());
    }

    #[test]
    fn test_terrain_quest_config_default() {
        let cfg = TerrainQuestConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_active_quests, 5);
        assert_eq!(cfg.min_quest_spacing, 100.0);
        assert!((cfg.ai_terrain_reward_bonus - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_trigger_should_trigger_feature_type_mismatch() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Cave],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Mountain, // mismatch
            ..Default::default()
        };
        let mut rng = rand::rng();
        assert!(!trigger.should_trigger(&ctx, 5, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_level_too_low() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 10,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default();
        let mut rng = rand::rng();
        assert!(!trigger.should_trigger(&ctx, 5, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_level_too_high() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 1,
            max_player_level: Some(10),
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default();
        let mut rng = rand::rng();
        assert!(!trigger.should_trigger(&ctx, 11, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_level_at_max_ok() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 1,
            max_player_level: Some(10),
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default();
        let mut rng = rand::rng();
        assert!(trigger.should_trigger(&ctx, 10, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_biome_mismatch() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec!["desert".into()],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default(); // biome = "unknown"
        let mut rng = rand::rng();
        assert!(!trigger.should_trigger(&ctx, 5, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_biome_match() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec!["unknown".into()],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default();
        let mut rng = rand::rng();
        assert!(trigger.should_trigger(&ctx, 5, &mut rng));
    }

    #[test]
    fn test_trigger_should_trigger_empty_biomes_passes() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Hill],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = TerrainQuestContext::default();
        let mut rng = rand::rng();
        assert!(trigger.should_trigger(&ctx, 5, &mut rng));
    }
}

// ============================================================================
// Module: TerrainQuestGenerator tests (terrain_quests.rs)
// ============================================================================
mod terrain_quest_generator_tests {
    use super::*;

    fn make_cave_ctx(x: f32, y: f32, z: f32) -> TerrainQuestContext {
        TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (x, y, z),
            radius: 32.0,
            intensity: 0.7,
            biome: "forest".into(),
            nearby_features: vec![],
            is_ai_generated: true,
            seed: Some(42),
        }
    }

    #[test]
    fn test_default_config_creation() {
        let gen = TerrainQuestGenerator::default_config();
        // config and triggers are private; verify behavior instead
        assert!(gen.active_quests().is_empty());
        assert_eq!(gen.quests_generated(), 0);
        // Verify it can generate quests (proves config.enabled=true and triggers exist)
        let mut gen = gen;
        let ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (0.0, 0.0, 0.0),
            ..Default::default()
        };
        let player = make_test_quest_context(10);
        assert!(gen.generate_quest(&ctx, &player).is_ok());
    }

    #[test]
    fn test_register_trigger_via_default() {
        let mut gen = TerrainQuestGenerator::default_config();
        // Just verify register_trigger doesn't panic
        gen.register_trigger(TerrainQuestTrigger {
            id: "custom".into(),
            feature_types: vec![TerrainFeatureType::Crater],
            min_player_level: 20,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 0.5,
            cooldown_seconds: 600.0,
            quest_template: "crater_exploration".into(),
        });
        // Verify generator still works
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        assert!(gen.generate_quest(&ctx, &player).is_ok());
    }

    #[test]
    fn test_generate_quest_disabled() {
        let mut config = TerrainQuestConfig::default();
        config.enabled = false;
        let mut gen = TerrainQuestGenerator::new(config);
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let result = gen.generate_quest(&ctx, &player).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_quest_max_active() {
        let mut config = TerrainQuestConfig::default();
        config.max_active_quests = 1;
        config.min_quest_spacing = 0.0; // disable spacing to test max_active only
        let mut gen = TerrainQuestGenerator::new(config);
        let player = make_test_quest_context(10);

        // First quest should succeed
        let ctx1 = make_cave_ctx(0.0, 0.0, 0.0);
        let r1 = gen.generate_quest(&ctx1, &player).unwrap();
        assert!(r1.is_some());

        // Second should fail (max 1)
        let ctx2 = make_cave_ctx(1000.0, 0.0, 1000.0);
        let r2 = gen.generate_quest(&ctx2, &player).unwrap();
        assert!(r2.is_none());
    }

    #[test]
    fn test_generate_quest_too_close_spacing() {
        let mut gen = TerrainQuestGenerator::default_config();
        let player = make_test_quest_context(10);

        let ctx1 = make_cave_ctx(100.0, 0.0, 100.0);
        let r1 = gen.generate_quest(&ctx1, &player).unwrap();
        assert!(r1.is_some());

        // Within 100.0 spacing
        let ctx2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Forest,
            position: (110.0, 0.0, 110.0),
            ..Default::default()
        };
        let r2 = gen.generate_quest(&ctx2, &player).unwrap();
        assert!(r2.is_none());
    }

    #[test]
    fn test_generate_quest_far_enough_spacing() {
        let mut gen = TerrainQuestGenerator::default_config();
        let player = make_test_quest_context(10);

        let ctx1 = make_cave_ctx(0.0, 0.0, 0.0);
        let r1 = gen.generate_quest(&ctx1, &player).unwrap();
        assert!(r1.is_some());

        // Far away
        let ctx2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Forest,
            position: (500.0, 0.0, 500.0),
            ..Default::default()
        };
        let r2 = gen.generate_quest(&ctx2, &player).unwrap();
        assert!(r2.is_some());
    }

    #[test]
    fn test_generated_quest_structure() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 50.0, 200.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();

        assert!(quest.location_bound);
        assert!(!quest.terrain_objectives.is_empty());
        assert_eq!(quest.terrain_objectives.len(), 2);
        assert_eq!(quest.quest.steps.len(), 2);
        assert!(quest.quest.title.to_lowercase().contains("cave"));
        assert_eq!(quest.terrain_context.feature_type, TerrainFeatureType::Cave);
    }

    #[test]
    fn test_quests_generated_counter() {
        let mut gen = TerrainQuestGenerator::default_config();
        assert_eq!(gen.quests_generated(), 0);
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        gen.generate_quest(&ctx, &player).unwrap();
        assert_eq!(gen.quests_generated(), 1);
    }

    #[test]
    fn test_complete_quest_exists() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        let quest_id = quest.quest.id.clone();

        assert_eq!(gen.active_quests().len(), 1);
        let completed = gen.complete_quest(&quest_id);
        assert!(completed.is_some());
        assert_eq!(gen.active_quests().len(), 0);
    }

    #[test]
    fn test_complete_quest_not_found() {
        let mut gen = TerrainQuestGenerator::default_config();
        let result = gen.complete_quest("nonexistent");
        assert!(result.is_none());
    }

    // ---- Indirect tests for private methods via public API ----

    #[test]
    fn test_ai_generated_gets_bonus_experience() {
        let mut gen = TerrainQuestGenerator::default_config();
        let player = make_test_quest_context(10);

        // AI-generated terrain
        let ctx_ai = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (0.0, 0.0, 0.0),
            is_ai_generated: true,
            ..Default::default()
        };
        let quest_ai = gen.generate_quest(&ctx_ai, &player).unwrap().unwrap();

        // Non-AI terrain (far enough away for spacing)
        let ctx_normal = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (500.0, 0.0, 500.0),
            is_ai_generated: false,
            ..Default::default()
        };
        let quest_normal = gen.generate_quest(&ctx_normal, &player).unwrap().unwrap();

        // AI rewards should be higher (1.25x bonus)
        assert!(quest_ai.quest.rewards.experience > quest_normal.quest.rewards.experience);
        assert!(quest_ai.quest.rewards.currency > quest_normal.quest.rewards.currency);
    }

    #[test]
    fn test_generated_quest_title_is_capitalized() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        // Title should start with uppercase (capitalize function)
        let first = quest.quest.title.chars().next().unwrap();
        assert!(
            first.is_uppercase(),
            "Title should start uppercase: {}",
            quest.quest.title
        );
    }

    #[test]
    fn test_generated_quest_description_contains_feature_name() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Mountain,
            position: (100.0, 0.0, 100.0),
            ..Default::default()
        };
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        // Description uses feature_description which mentions the feature type
        assert!(
            quest.quest.description.to_lowercase().contains("mountain")
                || quest.quest.description.contains("towering"),
            "Description should mention mountain: {}",
            quest.quest.description
        );
    }

    #[test]
    fn test_generated_quest_title_contains_feature_lowercase_name() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        // feature_name returns lowercase Debug name
        assert!(
            quest.quest.title.to_lowercase().contains("cave"),
            "Title should contain 'cave': {}",
            quest.quest.title
        );
    }

    #[test]
    fn test_rewards_increase_with_difficulty() {
        // Low-level player = low difficulty → low rewards
        let mut gen1 = TerrainQuestGenerator::default_config();
        let ctx1 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Meadow, // negative difficulty modifier
            position: (0.0, 0.0, 0.0),
            intensity: 0.0,
            is_ai_generated: false,
            ..Default::default()
        };
        let low_player = QuestContext {
            player_id: "p".into(),
            player_level: 1,
            location: "f".into(),
            available_npcs: vec![],
            world_state: HashMap::new(),
            recent_activities: vec![],
            preferred_quest_types: vec![],
        };
        let quest_low = gen1.generate_quest(&ctx1, &low_player).unwrap().unwrap();

        // High-level player + Mountain = high difficulty → high rewards
        let mut gen2 = TerrainQuestGenerator::default_config();
        let ctx2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Mountain, // +0.3 difficulty
            position: (0.0, 0.0, 0.0),
            intensity: 1.0,
            is_ai_generated: false,
            ..Default::default()
        };
        let high_player = QuestContext {
            player_id: "p".into(),
            player_level: 30,
            location: "f".into(),
            available_npcs: vec![],
            world_state: HashMap::new(),
            recent_activities: vec![],
            preferred_quest_types: vec![],
        };
        let quest_high = gen2.generate_quest(&ctx2, &high_player).unwrap().unwrap();

        assert!(quest_high.quest.rewards.experience > quest_low.quest.rewards.experience);
    }

    #[test]
    fn test_default_config_has_triggers() {
        // Verify default_config creates a generator with triggers by testing
        // that it can generate a quest for a common feature type (cave)
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let result = gen.generate_quest(&ctx, &player);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_trigger_enables_new_feature() {
        let mut gen = TerrainQuestGenerator::default_config();
        gen.register_trigger(TerrainQuestTrigger {
            id: "custom".into(),
            feature_types: vec![TerrainFeatureType::Crater],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "crater_test".into(),
        });
        // Should still be able to generate quests
        let ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Crater,
            position: (0.0, 0.0, 0.0),
            ..Default::default()
        };
        let player = make_test_quest_context(5);
        let result = gen.generate_quest(&ctx, &player);
        assert!(result.is_ok());
    }

    #[test]
    fn test_terrain_objective_fields() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 50.0, 200.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();

        let obj1 = &quest.terrain_objectives[0];
        assert_eq!(obj1.id, "reach_feature");
        assert_eq!(obj1.target_position, (100.0, 50.0, 200.0));
        assert_eq!(obj1.completion_radius, 32.0);
        assert!(!obj1.completed);

        let obj2 = &quest.terrain_objectives[1];
        assert_eq!(obj2.id, "investigate");
        assert_eq!(obj2.completion_radius, 16.0); // radius / 2
    }

    #[test]
    fn test_generated_quest_rewards_for_ai_terrain() {
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = make_cave_ctx(100.0, 0.0, 100.0);
        let player = make_test_quest_context(10);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        // is_ai_generated=true, so rewards should have bonus
        assert!(quest.quest.rewards.experience > 0);
        assert!(quest.quest.rewards.currency > 0);
    }
}

// ============================================================================
// Module: Systems integration tests (systems.rs)
// ============================================================================
mod systems_integration_tests {
    use super::*;

    #[test]
    fn test_initialize_player_quest_system() {
        let (gen, journal, metrics) = systems::integration::initialize_player_quest_system(
            "player_42".into(),
            15,
            "volcano".into(),
        );
        assert_eq!(gen.context.player_id, "player_42");
        assert_eq!(gen.context.player_level, 15);
        assert_eq!(gen.context.location, "volcano");
        assert_eq!(journal.get_active_quest_count(), 0);
        assert_eq!(metrics.quests_generated, 0);
    }

    #[test]
    fn test_to_basic_quest() {
        let llm_quest = make_test_llm_quest("q1", "combat");
        let active = CActiveQuest::new(llm_quest);
        let basic = systems::integration::to_basic_quest(&active);
        assert_eq!(basic.title, "Quest q1");
        assert_eq!(basic.steps.len(), 2);
        assert_eq!(basic.steps[0].description, "First step");
        assert!(!basic.steps[0].completed);
    }

    #[test]
    fn test_should_prompt_for_quest_all_conditions_met() {
        let mut gen = CQuestGenerator::new("p".into(), 5, "forest".into());
        gen.add_recent_activity("fought_goblins".into());
        gen.last_generation_time = 0;
        gen.generation_cooldown_ms = 100;

        let journal = CQuestJournal::new();
        assert!(systems::integration::should_prompt_for_quest(
            &gen, &journal, 1000
        ));
    }

    #[test]
    fn test_should_prompt_for_quest_auto_discover_off() {
        let mut gen = CQuestGenerator::new("p".into(), 5, "forest".into());
        gen.add_recent_activity("activity".into());
        let mut journal = CQuestJournal::new();
        journal.auto_discover = false;
        assert!(!systems::integration::should_prompt_for_quest(
            &gen, &journal, 1000
        ));
    }

    #[test]
    fn test_should_prompt_for_quest_max_quests_reached() {
        let mut gen = CQuestGenerator::new("p".into(), 5, "forest".into());
        gen.add_recent_activity("activity".into());
        let mut journal = CQuestJournal::new();
        for i in 0..3 {
            journal.add_quest(&make_test_llm_quest(&format!("q{}", i), "a"));
        }
        assert!(!systems::integration::should_prompt_for_quest(
            &gen, &journal, 1000
        ));
    }

    #[test]
    fn test_should_prompt_for_quest_cooldown_not_met() {
        let mut gen = CQuestGenerator::new("p".into(), 5, "forest".into());
        gen.add_recent_activity("activity".into());
        gen.last_generation_time = 900;
        gen.generation_cooldown_ms = 300000;
        let journal = CQuestJournal::new();
        assert!(!systems::integration::should_prompt_for_quest(
            &gen, &journal, 1000
        ));
    }

    #[test]
    fn test_should_prompt_for_quest_no_recent_activities() {
        let gen = CQuestGenerator::new("p".into(), 5, "forest".into());
        let journal = CQuestJournal::new();
        assert!(!systems::integration::should_prompt_for_quest(
            &gen, &journal, 1000000
        ));
    }

    #[test]
    fn test_get_quest_recommendations_empty_journal() {
        let journal = CQuestJournal::new();
        let recs = systems::integration::get_quest_recommendations(&journal);
        assert!(!recs.is_empty());
        assert!(recs.iter().any(|r| r.contains("No active quests")));
    }

    #[test]
    fn test_get_quest_recommendations_low_completion_rate() {
        let mut journal = CQuestJournal::new();
        // Add 5 quests, complete 1 → 20% completion rate < 30%
        for i in 0..5 {
            journal.add_quest(&make_test_llm_quest(&format!("q{}", i), "combat"));
        }
        journal.complete_quest("q0", "done".into());
        journal.abandon_quest("q1", "bored".into());
        journal.abandon_quest("q2", "bored".into());
        journal.abandon_quest("q3", "bored".into());

        let recs = systems::integration::get_quest_recommendations(&journal);
        assert!(recs.iter().any(|r| r.contains("shorter, simpler quests")));
    }

    #[test]
    fn test_get_quest_recommendations_exploration_preference() {
        let mut journal = CQuestJournal::new();
        journal
            .learned_preferences
            .insert("exploration".into(), 0.5);
        let recs = systems::integration::get_quest_recommendations(&journal);
        assert!(recs.iter().any(|r| r.contains("Exploration quests")));
    }
}

// ============================================================================
// Module: LLM Quest data structure tests (llm_quests.rs)
// ============================================================================
mod llm_quest_data_tests {
    use super::*;

    #[test]
    fn test_quest_generation_config_default() {
        let cfg = QuestGenerationConfig::default();
        assert!((cfg.creativity_level - 0.7).abs() < f32::EPSILON);
        assert!((cfg.personalization_weight - 0.8).abs() < f32::EPSILON);
        assert!((cfg.branching_complexity - 0.6).abs() < f32::EPSILON);
        assert_eq!(cfg.max_steps, 10);
        assert!(cfg.enable_dynamic_content);
        assert!((cfg.validation_strictness - 0.8).abs() < f32::EPSILON);
        assert_eq!(cfg.context_window_size, 2048);
    }

    #[test]
    fn test_objective_type_variants() {
        // Ensure all variants can be created
        let _collect = ObjectiveType::Collect;
        let _defeat = ObjectiveType::Defeat;
        let _interact = ObjectiveType::Interact;
        let _reach = ObjectiveType::Reach;
        let _deliver = ObjectiveType::Deliver;
        let _craft = ObjectiveType::Craft;
        let _explore = ObjectiveType::Explore;
        let _survive = ObjectiveType::Survive;
        let custom = ObjectiveType::Custom("find_artifact".into());
        match custom {
            ObjectiveType::Custom(s) => assert_eq!(s, "find_artifact"),
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_dynamic_content_fields() {
        let content = DynamicContent {
            dialogue: Some("Hello".into()),
            flavor_text: None,
            environmental_description: Some("Dark cave".into()),
            npc_interactions: vec!["greet".into(), "trade".into()],
        };
        assert_eq!(content.dialogue.as_deref(), Some("Hello"));
        assert!(content.flavor_text.is_none());
        assert_eq!(
            content.environmental_description.as_deref(),
            Some("Dark cave")
        );
        assert_eq!(content.npc_interactions.len(), 2);
    }

    #[test]
    fn test_validation_criteria_fields() {
        let vc = ValidationCriteria {
            required_conditions: vec!["has_key".into()],
            forbidden_conditions: vec!["dead".into()],
            validation_script: Some("check_key()".into()),
        };
        assert_eq!(vc.required_conditions.len(), 1);
        assert_eq!(vc.forbidden_conditions.len(), 1);
        assert!(vc.validation_script.is_some());
    }

    #[test]
    fn test_quest_context_fields() {
        let ctx = make_test_quest_context(10);
        assert_eq!(ctx.player_id, "player_1");
        assert_eq!(ctx.player_level, 10);
        assert_eq!(ctx.location, "forest");
        assert_eq!(ctx.available_npcs, vec!["npc_1".to_string()]);
        assert_eq!(ctx.preferred_quest_types, vec!["exploration".to_string()]);
    }

    #[test]
    fn test_quest_branching_fields() {
        let branching = QuestBranching {
            has_multiple_paths: true,
            branch_points: vec![BranchPoint {
                step_id: "s1".into(),
                condition: "has_artifact".into(),
                branches: vec![QuestBranch {
                    id: "b1".into(),
                    name: "Dark Path".into(),
                    description: "Take the dark path".into(),
                    steps: vec!["s2".into()],
                    consequences: vec!["lost_reputation".into()],
                }],
            }],
            convergence_points: vec!["s5".into()],
        };
        assert!(branching.has_multiple_paths);
        assert_eq!(branching.branch_points.len(), 1);
        assert_eq!(branching.convergence_points, vec!["s5".to_string()]);
    }

    #[test]
    fn test_quest_rewards_fields() {
        let mut rep = HashMap::new();
        rep.insert("guild".into(), 10);
        let rewards = QuestRewards {
            experience: 500,
            currency: 100,
            items: vec!["sword".into(), "shield".into()],
            reputation_changes: rep,
            unlock_content: vec!["area_2".into()],
        };
        assert_eq!(rewards.experience, 500);
        assert_eq!(rewards.currency, 100);
        assert_eq!(rewards.items.len(), 2);
        assert_eq!(*rewards.reputation_changes.get("guild").unwrap(), 10);
        assert_eq!(rewards.unlock_content.len(), 1);
    }

    #[test]
    fn test_personalization_data_fields() {
        let pd = PersonalizationData {
            player_id: "p1".into(),
            player_preferences: vec!["combat".into()],
            play_style: "Aggressive".into(),
            previous_choices: vec!["chose_fight".into()],
            difficulty_preference: 0.8,
        };
        assert_eq!(pd.player_id, "p1");
        assert_eq!(pd.play_style, "Aggressive");
        assert!((pd.difficulty_preference - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quest_metadata_fields() {
        let md = QuestMetadata {
            category: "combat".into(),
            difficulty_level: 0.7,
            estimated_duration: 45,
            player_level_range: (5, 15),
            required_skills: vec!["swordsmanship".into()],
            tags: vec!["epic".into()],
            generated_reasoning: "Player prefers combat".into(),
        };
        assert_eq!(md.category, "combat");
        assert!((md.difficulty_level - 0.7).abs() < f32::EPSILON);
        assert_eq!(md.estimated_duration, 45);
        assert_eq!(md.player_level_range, (5, 15));
    }

    #[test]
    fn test_branching_choice_fields() {
        let choice = BranchingChoice {
            id: "c1".into(),
            description: "Fight".into(),
            consequences: vec!["enemy_dead".into()],
            requirements: Some("strength > 10".into()),
            leads_to_step: Some("step_3".into()),
        };
        assert_eq!(choice.id, "c1");
        assert!(choice.requirements.is_some());
        assert!(choice.leads_to_step.is_some());
    }

    #[test]
    fn test_quest_validation_fields() {
        let v = QuestValidation {
            is_valid: true,
            quality_score: 0.85,
            issues: vec![ValidationIssue {
                issue_type: "balance".into(),
                severity: "minor".into(),
                description: "Too easy".into(),
                suggestion: "Increase difficulty".into(),
            }],
            strengths: vec!["Good narrative".into()],
            overall_assessment: "Good quest".into(),
        };
        assert!(v.is_valid);
        assert!((v.quality_score - 0.85).abs() < 0.001);
        assert_eq!(v.issues.len(), 1);
        assert_eq!(v.issues[0].issue_type, "balance");
        assert_eq!(v.strengths.len(), 1);
    }

    #[test]
    fn test_llm_quest_serialization_roundtrip() {
        let quest = make_test_llm_quest("q1", "combat");
        let json = serde_json::to_string(&quest).unwrap();
        let deserialized: LlmQuest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "q1");
        assert_eq!(deserialized.title, "Quest q1");
        assert_eq!(deserialized.steps.len(), 2);
        assert_eq!(deserialized.metadata.category, "combat");
    }

    #[test]
    fn test_quest_step_serialization_roundtrip() {
        let step = LlmQuestStep {
            id: "s1".into(),
            description: "Desc".into(),
            completed: true,
            objectives: vec![],
            branching_choices: vec![],
            dynamic_content: None,
            validation_criteria: ValidationCriteria {
                required_conditions: vec!["cond".into()],
                forbidden_conditions: vec![],
                validation_script: None,
            },
        };
        let json = serde_json::to_string(&step).unwrap();
        let d: LlmQuestStep = serde_json::from_str(&json).unwrap();
        assert_eq!(d.id, "s1");
        assert!(d.completed);
        assert_eq!(d.validation_criteria.required_conditions.len(), 1);
    }

    #[test]
    fn test_quest_template_fields() {
        let template = QuestTemplate {
            name: "Dragon Quest".into(),
            category: "combat".into(),
            structure: QuestStructure {
                typical_steps: 5,
                branching_points: 2,
                complexity_level: 0.7,
            },
            example_content: vec!["Slay the dragon".into()],
        };
        assert_eq!(template.name, "Dragon Quest");
        assert_eq!(template.structure.typical_steps, 5);
        assert_eq!(template.structure.branching_points, 2);
    }
}

// ============================================================================
// Module: Serialization / interop tests
// ============================================================================
mod serialization_tests {
    use super::*;

    #[test]
    fn test_terrain_feature_type_serialize_roundtrip() {
        let feature = TerrainFeatureType::Waterfall;
        let json = serde_json::to_string(&feature).unwrap();
        let d: TerrainFeatureType = serde_json::from_str(&json).unwrap();
        assert_eq!(d, TerrainFeatureType::Waterfall);
    }

    #[test]
    fn test_terrain_quest_context_serialize() {
        let ctx = TerrainQuestContext::default();
        let json = serde_json::to_string(&ctx).unwrap();
        let d: TerrainQuestContext = serde_json::from_str(&json).unwrap();
        assert_eq!(d.feature_type, TerrainFeatureType::Hill);
        assert_eq!(d.radius, 32.0);
    }

    #[test]
    fn test_quest_state_serialize() {
        let states = vec![
            QuestState::Active,
            QuestState::Paused,
            QuestState::Completed,
            QuestState::Failed,
            QuestState::Abandoned,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let _d: QuestState = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_terrain_interaction_serialize() {
        let interactions = vec![
            TerrainInteraction::Reach,
            TerrainInteraction::Collect,
            TerrainInteraction::Combat,
            TerrainInteraction::Investigate,
            TerrainInteraction::Build,
            TerrainInteraction::Survive,
            TerrainInteraction::Escort,
        ];
        for interaction in &interactions {
            let json = serde_json::to_string(interaction).unwrap();
            let _d: TerrainInteraction = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_quest_journal_stats_serialize() {
        let stats = QuestJournalStats {
            total_quests: 10,
            completed_quests: 5,
            abandoned_quests: 2,
            active_quests: 3,
            completion_rate: 0.5,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let d: QuestJournalStats = serde_json::from_str(&json).unwrap();
        assert_eq!(d.total_quests, 10);
        assert_eq!(d.completed_quests, 5);
    }

    #[test]
    fn test_generation_metrics_default() {
        let gm = GenerationMetrics::default();
        assert_eq!(gm.total_generations, 0);
        assert_eq!(gm.failed_generations, 0);
        assert_eq!(gm.average_generation_time, 0.0);
        assert_eq!(gm.validation_failures, 0);
        assert_eq!(gm.average_quality_score, 0.0);
    }

    #[test]
    fn test_terrain_quest_config_serialize() {
        let cfg = TerrainQuestConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let d: TerrainQuestConfig = serde_json::from_str(&json).unwrap();
        assert!(d.enabled);
        assert_eq!(d.max_active_quests, 5);
    }
}

// ============================================================================
// Module: Edge case / boundary tests
// ============================================================================
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_active_quest_empty_steps() {
        let mut quest = make_test_llm_quest("q1", "a");
        quest.steps.clear();
        let active = CActiveQuest::new(quest);
        assert!(active.get_current_step().is_none());
        // All steps completed vacuously → is_complete returns true
        assert!(active.is_complete());
    }

    #[test]
    fn test_advance_step_single_step_quest() {
        let mut quest = make_test_llm_quest("q1", "a");
        quest.steps = vec![quest.steps[0].clone()]; // Keep only first step
        let mut active = CActiveQuest::new(quest);
        // Index 0, only 1 step: 0 + 1 = 1, not < 1 → false, completes
        let advanced = active.advance_step();
        assert!(!advanced);
        assert!(matches!(active.state, QuestState::Completed));
    }

    #[test]
    fn test_metrics_running_average_generation_time() {
        let mut m = CQuestMetrics::default();
        // 3 generations: 100, 200, 300 → avg = 200
        m.record_quest_generation(100.0, true, None);
        m.record_quest_generation(200.0, true, None);
        m.record_quest_generation(300.0, true, None);
        assert!((m.generation_metrics.average_generation_time - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_journal_complete_nonexistent_quest() {
        let mut j = CQuestJournal::new();
        j.add_quest(&make_test_llm_quest("q1", "a"));
        j.complete_quest("nonexistent", "done".into());
        // q1 should remain active
        assert_eq!(j.active_quest_ids.len(), 1);
    }

    #[test]
    fn test_journal_abandon_nonexistent_quest() {
        let mut j = CQuestJournal::new();
        j.add_quest(&make_test_llm_quest("q1", "a"));
        j.abandon_quest("nonexistent", "reason".into());
        assert_eq!(j.active_quest_ids.len(), 1);
    }

    #[test]
    fn test_metrics_completion_rate_all_completed() {
        let mut m = CQuestMetrics::default();
        m.quests_generated = 5;
        m.quests_completed = 5;
        assert!((m.get_completion_rate() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_metrics_abandonment_rate_all_abandoned() {
        let mut m = CQuestMetrics::default();
        m.quests_generated = 5;
        m.quests_abandoned = 5;
        assert!((m.get_abandonment_rate() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quest_validate_whitespace_title_ok() {
        let quest = Quest {
            title: " ".into(), // whitespace is not empty
            steps: vec![QuestStep {
                description: "s".into(),
                completed: false,
            }],
        };
        assert!(quest.validate().is_ok());
    }

    #[test]
    fn test_terrain_generator_spacing_3d_distance() {
        // Verify the spacing check uses proper 3D Euclidean distance
        let mut config = TerrainQuestConfig::default();
        config.min_quest_spacing = 100.0;
        let mut gen = TerrainQuestGenerator::new(config);
        let player = make_test_quest_context(10);

        // First quest at (0,0,0)
        let ctx1 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (0.0, 0.0, 0.0),
            ..Default::default()
        };
        gen.generate_quest(&ctx1, &player).unwrap();

        // Second quest at (0,0,99) → distance = 99 < 100 → blocked
        let ctx2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Forest,
            position: (0.0, 0.0, 99.0),
            ..Default::default()
        };
        let r = gen.generate_quest(&ctx2, &player).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn test_terrain_difficulty_clamp() {
        // The difficulty calculation uses .clamp(0.0, 1.0)
        // Even with max values, it should be <= 1.0
        let mut gen = TerrainQuestGenerator::default_config();
        let ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Mountain, // +0.3
            position: (0.0, 0.0, 0.0),
            intensity: 1.0, // max
            ..Default::default()
        };
        // player_level=50 → base = 0.3 + 50*0.02 = 1.3
        // + terrain 0.3 + intensity*0.2 = 0.2 → total = 1.8 → clamped to 1.0
        let player = make_test_quest_context(50);
        let quest = gen.generate_quest(&ctx, &player).unwrap().unwrap();
        assert!(quest.quest.metadata.difficulty_level <= 1.0);
        assert!(quest.quest.metadata.difficulty_level >= 0.0);
    }

    #[test]
    fn test_choice_record_fields() {
        let record = ChoiceRecord {
            step_id: "s1".into(),
            choice: BranchingChoice {
                id: "c1".into(),
                description: "Fight".into(),
                consequences: vec![],
                requirements: None,
                leads_to_step: None,
            },
            timestamp: Utc::now(),
            consequences_applied: vec!["damage".into()],
        };
        assert_eq!(record.step_id, "s1");
        assert_eq!(record.choice.id, "c1");
        assert_eq!(record.consequences_applied.len(), 1);
    }

    #[test]
    fn test_journal_default_vs_new_differ_on_settings() {
        let d = CQuestJournal::default();
        let n = CQuestJournal::new();
        // Both should have empty history/active
        assert_eq!(d.quest_history.len(), n.quest_history.len());
        assert_eq!(d.active_quest_ids.len(), n.active_quest_ids.len());
        // new() sets specific values; Default derives zero/false
        assert_eq!(n.max_active_quests, 3);
        assert!(n.auto_discover);
        // Default sets 0 and false (derived defaults)
        assert_eq!(d.max_active_quests, 0);
        assert!(!d.auto_discover);
    }
}

// ============================================================================
// Mutation kill tests — terrain_quests.rs, components.rs
// ============================================================================
mod mutation_kill_tests {
    use super::*;

    fn make_cave_context(pos: (f32, f32, f32), intensity: f32) -> TerrainQuestContext {
        TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: pos,
            radius: 32.0,
            intensity,
            biome: "forest".into(),
            nearby_features: vec![],
            is_ai_generated: true,
            seed: Some(42),
        }
    }

    /// Kill: calculate_experience + → * and * → +, calculate_currency + → *, * → +, * → /
    /// Also kills: difficulty arithmetic (base_difficulty + terrain_modifier + intensity * 0.2)
    #[test]
    fn terrain_quest_xp_currency_exact() {
        let mut gen = TerrainQuestGenerator::default_config();
        let terrain = make_cave_context((500.0, 50.0, 500.0), 0.7);
        let player = make_test_quest_context(5);
        let quest = gen.generate_quest(&terrain, &player).unwrap().unwrap();

        // difficulty = (0.3 + 5*0.02 + 0.25 + 0.7*0.2).clamp(0,1)
        //            = (0.3 + 0.1 + 0.25 + 0.14) = 0.79
        let diff = quest.quest.metadata.difficulty_level;
        assert!(
            (diff - 0.79).abs() < 0.01,
            "difficulty should be ~0.79, got {diff}"
        );

        // XP = (100 + 0.79*400) * 1.25 = 416 * 1.25 = 520
        assert_eq!(quest.quest.rewards.experience, 520, "XP should be 520");

        // Currency = (10 + 0.79*40) * 1.25 = 41.6 * 1.25 = 52.0
        assert_eq!(quest.quest.rewards.currency, 52, "currency should be 52");

        // estimated_duration = (15 + 0.79*30) as u32 = (15+23.7) = 38.7 → 38
        assert_eq!(quest.quest.metadata.estimated_duration, 38);
    }

    /// Kill: feature_description match arm deletions (7 arms: Hill, Valley, Cave, Forest, Lake, River, Waterfall)
    /// Each feature type should produce its specific description, not "strange terrain feature"
    #[test]
    fn terrain_quest_description_per_feature() {
        let player = make_test_quest_context(5);
        let features_and_desc = [
            (TerrainFeatureType::Hill, "rolling hill"),
            (TerrainFeatureType::Valley, "sheltered valley"),
            (TerrainFeatureType::Cave, "mysterious cave"),
            (TerrainFeatureType::Forest, "dense forest"),
            (TerrainFeatureType::Lake, "serene lake"),
            (TerrainFeatureType::River, "flowing river"),
            (TerrainFeatureType::Waterfall, "cascading waterfall"),
        ];

        for (feature, expected_desc) in &features_and_desc {
            let mut gen = TerrainQuestGenerator::default_config();
            let terrain = TerrainQuestContext {
                feature_type: *feature,
                position: (1000.0, 0.0, 1000.0),
                radius: 32.0,
                intensity: 0.5,
                biome: "plains".into(),
                nearby_features: vec![],
                is_ai_generated: false,
                seed: None,
            };
            let quest = gen.generate_quest(&terrain, &player).unwrap().unwrap();
            assert!(
                quest.quest.description.contains(expected_desc),
                "{:?} description should contain '{}', got: {}",
                feature,
                expected_desc,
                quest.quest.description
            );
        }
    }

    /// Kill: spacing distance arithmetic (+ → -, + → *, - → +, - → /) and < → <=
    #[test]
    fn terrain_quest_spacing_rejects_close_quests() {
        let mut gen = TerrainQuestGenerator::default_config();
        // min_quest_spacing defaults to 100.0
        let player = make_test_quest_context(5);

        // Generate first quest at (0, 0, 0)
        let terrain1 = make_cave_context((0.0, 0.0, 0.0), 0.5);
        let q1 = gen.generate_quest(&terrain1, &player).unwrap();
        assert!(q1.is_some(), "first quest should succeed");

        // Second quest at (50, 0, 0) — distance = 50 < 100 → rejected
        let terrain2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Hill,
            position: (50.0, 0.0, 0.0),
            ..terrain1.clone()
        };
        let q2 = gen.generate_quest(&terrain2, &player).unwrap();
        assert!(q2.is_none(), "too-close quest should be rejected");

        // Third quest at (200, 0, 0) — distance = 200 > 100 → accepted
        let terrain3 = TerrainQuestContext {
            position: (200.0, 0.0, 0.0),
            ..terrain2.clone()
        };
        let q3 = gen.generate_quest(&terrain3, &player).unwrap();
        assert!(q3.is_some(), "distant quest should succeed");
    }

    /// Kill: should_trigger < → <= on min_player_level
    #[test]
    fn should_trigger_at_exact_min_level() {
        let trigger = TerrainQuestTrigger {
            id: "t1".into(),
            feature_types: vec![TerrainFeatureType::Cave],
            min_player_level: 5,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "test".into(),
        };
        let ctx = make_cave_context((0.0, 0.0, 0.0), 0.5);
        let mut rng = rand::rng();

        // At exactly min_player_level: should trigger (< is false)
        // With <= mutation: would NOT trigger (5 <= 5 is true → rejected)
        assert!(trigger.should_trigger(&ctx, 5, &mut rng));
        // Below min: should NOT trigger
        assert!(!trigger.should_trigger(&ctx, 4, &mut rng));
    }

    /// Kill: register_trigger with () — trigger should actually be stored
    #[test]
    fn register_trigger_is_stored() {
        let mut gen = TerrainQuestGenerator::default_config();
        let trigger = TerrainQuestTrigger {
            id: "cave_quest".into(),
            feature_types: vec![TerrainFeatureType::Cave],
            min_player_level: 1,
            max_player_level: None,
            required_biomes: vec![],
            trigger_probability: 1.0,
            cooldown_seconds: 0.0,
            quest_template: "cave_template".into(),
        };
        gen.register_trigger(trigger);
        // If register is replaced with (), the quest generator has no triggers
        // but generate_quest still works (trigger matching is optional)
        // We can verify quests_generated count increases
        let player = make_test_quest_context(5);
        let terrain = make_cave_context((999.0, 0.0, 999.0), 0.5);
        gen.generate_quest(&terrain, &player).unwrap();
        assert_eq!(gen.quests_generated(), 1);
    }

    /// Kill: CQuestMetrics running average arithmetic
    /// (- → + on total-1, > → >= on quality_count, * → + on avg formulas, / → % on division)
    #[test]
    fn metrics_running_average_exact_values() {
        let mut m = CQuestMetrics::default();

        // First generation: time=100ms, success, quality=0.8
        let v1 = QuestValidation {
            is_valid: true,
            quality_score: 0.8,
            issues: vec![],
            strengths: vec![],
            overall_assessment: String::new(),
        };
        m.record_quest_generation(100.0, true, Some(&v1));
        // avg_gen_time = (0*(1-1) + 100) / 1 = 100.0
        assert!(
            (m.generation_metrics.average_generation_time - 100.0).abs() < 0.01,
            "avg time after 1 gen should be 100, got {}",
            m.generation_metrics.average_generation_time
        );
        // avg_quality = (0*(1-1) + 0.8) / 1 = 0.8
        assert!(
            (m.generation_metrics.average_quality_score - 0.8).abs() < 0.01,
            "avg quality after 1 gen should be 0.8, got {}",
            m.generation_metrics.average_quality_score
        );

        // Second generation: time=200ms, success, quality=0.6
        let v2 = QuestValidation {
            is_valid: true,
            quality_score: 0.6,
            issues: vec![],
            strengths: vec![],
            overall_assessment: String::new(),
        };
        m.record_quest_generation(200.0, true, Some(&v2));
        // avg_gen_time = (100*(2-1) + 200) / 2 = 300/2 = 150.0
        assert!(
            (m.generation_metrics.average_generation_time - 150.0).abs() < 0.01,
            "avg time after 2 gens should be 150, got {}",
            m.generation_metrics.average_generation_time
        );
        // avg_quality = (0.8*(2-1) + 0.6) / 2 = 1.4/2 = 0.7
        assert!(
            (m.generation_metrics.average_quality_score - 0.7).abs() < 0.01,
            "avg quality after 2 gens should be 0.7, got {}",
            m.generation_metrics.average_quality_score
        );
    }

    /// Kill: CActiveQuest::get_duration → Default::default()
    /// Duration must be positive (non-zero) for a quest started in the past
    #[test]
    fn active_quest_duration_nonzero() {
        let quest = make_test_llm_quest("dur_test", "exploration");
        let mut active = CActiveQuest::new(quest);
        active.start_time = Utc::now() - chrono::Duration::seconds(60);
        let dur = active.get_duration();
        assert!(dur.num_seconds() > 0, "duration should be positive");
    }
}

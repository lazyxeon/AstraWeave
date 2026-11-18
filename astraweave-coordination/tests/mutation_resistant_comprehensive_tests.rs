//! Mutation-resistant comprehensive tests for astraweave-coordination.
//!
//! Targets: AgentGoal, Task, BaseAgent, enums (AgentState, MessagePriority,
//! GoalType, GoalStatus, EventSeverity, CoordinationStatus, etc.),
//! CoordinatorConfig, ResourceLimits, CoordinationMetrics, CoherenceConfig,
//! CoherenceAnalysis, EventGenerationConfig, data struct serde roundtrips.

use astraweave_coordination::agent::*;
use astraweave_coordination::coordination::*;
use astraweave_coordination::narrative_coherence::*;
use astraweave_coordination::world_events::*;
use serde_json::json;
use std::time::Duration;

// =========================================================================
// AgentGoal — constructor, satisfaction, progress, overdue
// =========================================================================

#[test]
fn agent_goal_new_defaults() {
    let g = AgentGoal::new("test".into(), GoalType::Achieve, 0.8);
    assert_eq!(g.description, "test");
    assert!(matches!(g.goal_type, GoalType::Achieve));
    assert_eq!(g.priority, 0.8);
    assert_eq!(g.target_value, None);
    assert_eq!(g.current_value, 0.0);
    assert_eq!(g.deadline, None);
    assert!(g.dependencies.is_empty());
    assert_eq!(g.status, GoalStatus::Active);
}

#[test]
fn agent_goal_new_has_uuid_id() {
    let g = AgentGoal::new("x".into(), GoalType::Explore, 0.5);
    // UUID v4 hyphenated: 8-4-4-4-12 = 36 chars
    assert_eq!(g.id.len(), 36);
    assert_eq!(g.id.chars().filter(|c| *c == '-').count(), 4);
}

#[test]
fn agent_goal_is_satisfied_achieve_with_target_at_target() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.current_value = 10.0;
    assert!(g.is_satisfied()); // current_value >= target
}

#[test]
fn agent_goal_is_satisfied_achieve_above_target() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.current_value = 10.1;
    assert!(g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_achieve_below_target() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.current_value = 9.9;
    assert!(!g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_achieve_no_target() {
    let g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    // target_value = None → always false
    assert!(!g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_maintain_within_threshold() {
    let mut g = AgentGoal::new("x".into(), GoalType::Maintain, 1.0);
    g.target_value = Some(5.0);
    g.current_value = 5.05; // diff = 0.05 < 0.1
    assert!(g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_maintain_at_threshold_boundary() {
    let mut g = AgentGoal::new("x".into(), GoalType::Maintain, 1.0);
    g.target_value = Some(5.0);
    g.current_value = 5.2; // diff = 0.2 > 0.1 threshold — NOT satisfied
    assert!(!g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_maintain_no_target() {
    let g = AgentGoal::new("x".into(), GoalType::Maintain, 1.0);
    assert!(!g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_avoid_zero() {
    let mut g = AgentGoal::new("x".into(), GoalType::Avoid, 1.0);
    g.current_value = 0.0;
    assert!(g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_avoid_nonzero() {
    let mut g = AgentGoal::new("x".into(), GoalType::Avoid, 1.0);
    g.current_value = 0.001;
    assert!(!g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_explore_only_when_completed() {
    let mut g = AgentGoal::new("x".into(), GoalType::Explore, 1.0);
    g.status = GoalStatus::Active;
    assert!(!g.is_satisfied());
    g.status = GoalStatus::Completed;
    assert!(g.is_satisfied());
}

#[test]
fn agent_goal_is_satisfied_collaborate_only_when_completed() {
    let mut g = AgentGoal::new("x".into(), GoalType::Collaborate, 1.0);
    g.status = GoalStatus::Failed;
    assert!(!g.is_satisfied());
    g.status = GoalStatus::Completed;
    assert!(g.is_satisfied());
}

#[test]
fn agent_goal_update_progress_sets_value() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.update_progress(5.0);
    assert_eq!(g.current_value, 5.0);
    assert_eq!(g.status, GoalStatus::Active); // not yet satisfied
}

#[test]
fn agent_goal_update_progress_auto_completes_when_satisfied() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.update_progress(10.0);
    assert_eq!(g.status, GoalStatus::Completed);
}

#[test]
fn agent_goal_update_progress_no_auto_complete_if_paused() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.status = GoalStatus::Paused;
    g.update_progress(10.0);
    // is_satisfied() would be true, but status != Active → no auto-complete
    assert_eq!(g.status, GoalStatus::Paused);
}

#[test]
fn agent_goal_update_progress_no_auto_complete_if_failed() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.target_value = Some(10.0);
    g.status = GoalStatus::Failed;
    g.update_progress(10.0);
    assert_eq!(g.status, GoalStatus::Failed);
}

#[test]
fn agent_goal_is_overdue_no_deadline() {
    let g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    assert!(!g.is_overdue());
}

#[test]
fn agent_goal_is_overdue_future_deadline() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.deadline = Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    assert!(!g.is_overdue());
}

#[test]
fn agent_goal_is_overdue_past_deadline_active() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.deadline = Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1));
    assert!(g.is_overdue());
}

#[test]
fn agent_goal_is_overdue_past_deadline_completed_not_overdue() {
    let mut g = AgentGoal::new("x".into(), GoalType::Achieve, 1.0);
    g.deadline = Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1));
    g.status = GoalStatus::Completed;
    // completed goals are NOT overdue even if past deadline
    assert!(!g.is_overdue());
}

// =========================================================================
// Task — constructor, builder, overdue
// =========================================================================

#[test]
fn task_new_defaults() {
    let t = Task::new("combat".into(), "fight".into());
    assert_eq!(t.task_type, "combat");
    assert_eq!(t.description, "fight");
    assert_eq!(t.priority, 0.5);
    assert!(t.parameters.is_empty());
    assert_eq!(t.estimated_duration, None);
    assert_eq!(t.deadline, None);
    assert!(t.required_capabilities.is_empty());
}

#[test]
fn task_new_has_uuid() {
    let t = Task::new("x".into(), "y".into());
    assert_eq!(t.id.len(), 36);
}

#[test]
fn task_with_priority_clamps_high() {
    let t = Task::new("x".into(), "y".into()).with_priority(2.0);
    assert_eq!(t.priority, 1.0);
}

#[test]
fn task_with_priority_clamps_low() {
    let t = Task::new("x".into(), "y".into()).with_priority(-1.0);
    assert_eq!(t.priority, 0.0);
}

#[test]
fn task_with_priority_normal() {
    let t = Task::new("x".into(), "y".into()).with_priority(0.7);
    assert_eq!(t.priority, 0.7);
}

#[test]
fn task_with_parameter_adds_entry() {
    let t = Task::new("x".into(), "y".into()).with_parameter("key".into(), 42);
    assert!(t.parameters.contains_key("key"));
}

#[test]
fn task_with_capability_adds_entry() {
    let t = Task::new("x".into(), "y".into()).with_capability("flight".into());
    assert_eq!(t.required_capabilities, vec!["flight"]);
}

#[test]
fn task_is_overdue_no_deadline() {
    let t = Task::new("x".into(), "y".into());
    assert!(!t.is_overdue());
}

#[test]
fn task_is_overdue_past_deadline() {
    let mut t = Task::new("x".into(), "y".into());
    t.deadline = Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1));
    assert!(t.is_overdue());
}

#[test]
fn task_is_overdue_future_deadline() {
    let mut t = Task::new("x".into(), "y".into());
    t.deadline = Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    assert!(!t.is_overdue());
}

// =========================================================================
// BaseAgent — constructor, helpers, availability, resource checks
// =========================================================================

#[test]
fn base_agent_new_defaults() {
    let a = BaseAgent::new("npc".into(), vec!["talk".into()]);
    assert_eq!(a.agent_type, "npc");
    assert_eq!(a.state, AgentState::Idle);
    assert!(a.goals.is_empty());
    assert_eq!(a.capabilities, vec!["talk"]);
    assert!(a.event_subscriptions.is_empty());
    assert!(a.coordination_context.is_none());
}

#[test]
fn base_agent_new_has_uuid() {
    let a = BaseAgent::new("x".into(), vec![]);
    assert_eq!(a.id.len(), 36);
}

#[test]
fn base_agent_is_available_idle_no_tasks() {
    let a = BaseAgent::new("x".into(), vec![]);
    assert!(a.is_available()); // Idle + active_tasks=0 < 5
}

#[test]
fn base_agent_is_available_not_idle() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.state = AgentState::Processing;
    assert!(!a.is_available());
}

#[test]
fn base_agent_is_available_too_many_tasks() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.active_tasks = 5; // exactly 5 → NOT < 5
    assert!(!a.is_available());
}

#[test]
fn base_agent_is_available_four_tasks_ok() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.active_tasks = 4;
    assert!(a.is_available()); // 4 < 5 → true
}

#[test]
fn base_agent_can_handle_requirements_within_limits() {
    let a = BaseAgent::new("x".into(), vec![]);
    let req = ResourceRequirements {
        llm_calls: 10,
        memory_mb: 500,
        compute_units: 0,
        exclusive_resources: vec![],
    };
    assert!(a.can_handle_requirements(&req)); // 0+10 < 60 && 0+500 < 1024
}

#[test]
fn base_agent_can_handle_requirements_memory_at_limit() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.memory_mb = 524;
    let req = ResourceRequirements {
        llm_calls: 0,
        memory_mb: 500,
        compute_units: 0,
        exclusive_resources: vec![],
    };
    // 524 + 500 = 1024 → NOT < 1024, fails
    assert!(!a.can_handle_requirements(&req));
}

#[test]
fn base_agent_can_handle_requirements_memory_just_below() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.memory_mb = 523;
    let req = ResourceRequirements {
        llm_calls: 0,
        memory_mb: 500,
        compute_units: 0,
        exclusive_resources: vec![],
    };
    // 523 + 500 = 1023 < 1024 → passes
    assert!(a.can_handle_requirements(&req));
}

#[test]
fn base_agent_can_handle_requirements_llm_at_limit() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.llm_calls_per_minute = 50;
    let req = ResourceRequirements {
        llm_calls: 10,
        memory_mb: 0,
        compute_units: 0,
        exclusive_resources: vec![],
    };
    // 50 + 10 = 60 → NOT < 60, fails
    assert!(!a.can_handle_requirements(&req));
}

#[test]
fn base_agent_can_handle_requirements_llm_just_below() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    a.resource_usage.llm_calls_per_minute = 49;
    let req = ResourceRequirements {
        llm_calls: 10,
        memory_mb: 0,
        compute_units: 0,
        exclusive_resources: vec![],
    };
    // 49 + 10 = 59 < 60 → passes
    assert!(a.can_handle_requirements(&req));
}

#[test]
fn base_agent_create_response_swaps_from_to() {
    let a = BaseAgent::new("npc".into(), vec![]);
    let original = AgentMessage {
        id: "msg-1".into(),
        from: "player".into(),
        to: a.id.clone(),
        message_type: MessageType::Request,
        content: json!({"q": "hello"}),
        timestamp: time::OffsetDateTime::now_utc(),
        priority: MessagePriority::High,
        reply_to: None,
    };
    let resp = a.create_response(&original, json!({"a": "hi"}));
    assert_eq!(resp.from, a.id);
    assert_eq!(resp.to, "player"); // swapped to original sender
    assert!(matches!(resp.message_type, MessageType::Response));
    assert_eq!(resp.reply_to, Some("msg-1".into()));
    assert!(matches!(resp.priority, MessagePriority::High)); // inherited
    assert_eq!(resp.content, json!({"a": "hi"}));
}

#[test]
fn base_agent_create_notification_fields() {
    let a = BaseAgent::new("npc".into(), vec![]);
    let notif = a.create_notification(
        "target".into(),
        json!({"event": "spawn"}),
        MessagePriority::Critical,
    );
    assert_eq!(notif.from, a.id);
    assert_eq!(notif.to, "target");
    assert!(matches!(notif.message_type, MessageType::Notification));
    assert_eq!(notif.reply_to, None); // always None
    assert!(matches!(notif.priority, MessagePriority::Critical));
}

#[test]
fn base_agent_update_resource_usage() {
    let mut a = BaseAgent::new("x".into(), vec![]);
    let usage = ResourceUsage {
        llm_calls_per_minute: 10,
        memory_mb: 200,
        active_tasks: 3,
        queue_size: 5,
        last_activity: time::OffsetDateTime::now_utc(),
    };
    a.update_resource_usage(usage.clone());
    assert_eq!(a.resource_usage.llm_calls_per_minute, 10);
    assert_eq!(a.resource_usage.memory_mb, 200);
    assert_eq!(a.resource_usage.active_tasks, 3);
}

// =========================================================================
// Enums — variants, ordering, serialization
// =========================================================================

#[test]
fn agent_state_eq_idle() {
    assert_eq!(AgentState::Idle, AgentState::Idle);
}

#[test]
fn agent_state_ne_processing() {
    assert_ne!(AgentState::Idle, AgentState::Processing);
}

#[test]
fn agent_state_error_variant() {
    let e = AgentState::Error("oops".into());
    assert!(matches!(e, AgentState::Error(s) if s == "oops"));
}

#[test]
fn agent_state_clone() {
    let s = AgentState::Collaborating;
    let s2 = s.clone();
    assert_eq!(s, s2);
}

#[test]
fn message_priority_ordering() {
    assert!(MessagePriority::Low < MessagePriority::Normal);
    assert!(MessagePriority::Normal < MessagePriority::High);
    assert!(MessagePriority::High < MessagePriority::Critical);
}

#[test]
fn event_severity_ordering() {
    assert!(EventSeverity::Trivial < EventSeverity::Minor);
    assert!(EventSeverity::Minor < EventSeverity::Moderate);
    assert!(EventSeverity::Moderate < EventSeverity::Major);
    assert!(EventSeverity::Major < EventSeverity::Critical);
}

#[test]
fn goal_status_variants() {
    let statuses = [
        GoalStatus::Active,
        GoalStatus::Paused,
        GoalStatus::Completed,
        GoalStatus::Failed,
        GoalStatus::Abandoned,
    ];
    for (i, s) in statuses.iter().enumerate() {
        for (j, t) in statuses.iter().enumerate() {
            if i == j {
                assert_eq!(s, t);
            } else {
                assert_ne!(s, t);
            }
        }
    }
}

#[test]
fn coordination_status_variants() {
    assert_eq!(CoordinationStatus::Planning, CoordinationStatus::Planning);
    assert_ne!(CoordinationStatus::Active, CoordinationStatus::Failed);
}

#[test]
fn goal_type_serde_roundtrip() {
    let gt = GoalType::Achieve;
    let json = serde_json::to_string(&gt).unwrap();
    let back: GoalType = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, GoalType::Achieve));
}

#[test]
fn message_type_serde_roundtrip() {
    let mt = MessageType::Coordination;
    let json = serde_json::to_string(&mt).unwrap();
    let back: MessageType = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, MessageType::Coordination));
}

#[test]
fn agent_state_serde_roundtrip() {
    let s = AgentState::WaitingForInput;
    let json = serde_json::to_string(&s).unwrap();
    let back: AgentState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, AgentState::WaitingForInput);
}

// =========================================================================
// ResourceRequirements / ResourceUsage — defaults
// =========================================================================

#[test]
fn resource_requirements_default_all_zero() {
    let r = ResourceRequirements::default();
    assert_eq!(r.llm_calls, 0);
    assert_eq!(r.memory_mb, 0);
    assert_eq!(r.compute_units, 0);
    assert!(r.exclusive_resources.is_empty());
}

#[test]
fn resource_usage_default_all_zero() {
    let u = ResourceUsage::default();
    assert_eq!(u.llm_calls_per_minute, 0);
    assert_eq!(u.memory_mb, 0);
    assert_eq!(u.active_tasks, 0);
    assert_eq!(u.queue_size, 0);
    assert_eq!(u.last_activity.unix_timestamp(), 0);
}

// =========================================================================
// CoordinatorConfig — defaults
// =========================================================================

#[test]
fn coordinator_config_defaults() {
    let c = CoordinatorConfig::default();
    assert_eq!(c.max_tasks_per_agent, 5);
    assert_eq!(c.max_message_queue_size, 100);
    assert_eq!(c.default_task_timeout, Duration::from_secs(30));
    assert!(matches!(c.resource_strategy, ResourceStrategy::Priority));
    assert!(c.enable_metrics);
    assert_eq!(c.max_coordination_duration, Duration::from_secs(300));
}

#[test]
fn resource_limits_defaults() {
    let l = ResourceLimits::default();
    assert_eq!(l.total_llm_calls_per_minute, 1000);
    assert_eq!(l.total_memory_mb, 8192);
    assert_eq!(l.max_concurrent_llm_calls, 50);
}

#[test]
fn coordination_metrics_default_zeros() {
    let m = CoordinationMetrics::default();
    assert_eq!(m.messages_sent, 0);
    assert_eq!(m.messages_delivered, 0);
    assert_eq!(m.messages_failed, 0);
    assert_eq!(m.tasks_assigned, 0);
    assert_eq!(m.tasks_completed, 0);
    assert_eq!(m.tasks_failed, 0);
    assert_eq!(m.coordination_sessions, 0);
    assert_eq!(m.average_task_duration, 0.0);
    assert_eq!(m.resource_utilization, 0.0);
    assert!(m.agent_availability.is_empty());
    assert_eq!(m.event_processing_time, 0.0);
}

#[test]
fn coordination_metrics_serde_roundtrip() {
    let m = CoordinationMetrics {
        messages_sent: 42,
        tasks_completed: 10,
        ..Default::default()
    };
    let json = serde_json::to_string(&m).unwrap();
    let back: CoordinationMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(back.messages_sent, 42);
    assert_eq!(back.tasks_completed, 10);
}

// =========================================================================
// CoherenceConfig — defaults
// =========================================================================

#[test]
fn coherence_config_defaults() {
    let c = CoherenceConfig::default();
    assert!(c.enable_real_time_validation);
    assert_eq!(c.coherence_threshold, 0.8);
    assert_eq!(c.max_story_threads, 10);
    assert_eq!(c.character_consistency_weight, 0.4);
    assert_eq!(c.world_consistency_weight, 0.3);
    assert_eq!(c.temporal_consistency_weight, 0.3);
    assert_eq!(c.context_window_size, 4096);
}

#[test]
fn coherence_config_weights_sum_to_one() {
    let c = CoherenceConfig::default();
    let sum =
        c.character_consistency_weight + c.world_consistency_weight + c.temporal_consistency_weight;
    assert!((sum - 1.0).abs() < f32::EPSILON);
}

#[test]
fn coherence_analysis_default_zeros() {
    let a = CoherenceAnalysis::default();
    assert_eq!(a.coherence_score, 0.0);
    assert_eq!(a.character_consistency, 0.0);
    assert_eq!(a.world_consistency, 0.0);
    assert_eq!(a.temporal_consistency, 0.0);
    assert_eq!(a.narrative_impact, 0.0);
    assert!(a.issues.is_empty());
    assert!(a.strengths.is_empty());
    assert!(a.recommendations.is_empty());
}

// =========================================================================
// EventGenerationConfig — defaults
// =========================================================================

#[test]
fn event_generation_config_defaults() {
    let c = EventGenerationConfig::default();
    assert_eq!(c.generation_interval_ms, 30000);
    assert_eq!(c.max_concurrent_events, 5);
    assert!(c.enable_storyline_continuity);
    assert_eq!(c.context_window_size, 2048);
    assert_eq!(c.creativity_factor, 0.7);
    assert_eq!(c.world_coherence_weight, 0.8);
}

#[test]
fn severity_distribution_defaults_sum_to_one() {
    let c = EventGenerationConfig::default();
    let d = &c.event_severity_distribution;
    assert_eq!(d.trivial, 0.4);
    assert_eq!(d.minor, 0.3);
    assert_eq!(d.moderate, 0.2);
    assert_eq!(d.major, 0.08);
    assert_eq!(d.critical, 0.02);
    let sum = d.trivial + d.minor + d.moderate + d.major + d.critical;
    assert!((sum - 1.0).abs() < f32::EPSILON);
}

// =========================================================================
// Narrative data structs — serde roundtrips
// =========================================================================

#[test]
fn plot_point_serde_roundtrip() {
    let pp = PlotPoint {
        id: "pp-1".into(),
        description: "Find the artifact".into(),
        plot_type: PlotType::MainQuest,
        importance: 0.9,
        status: PlotStatus::Active,
        related_characters: vec!["hero".into()],
        prerequisites: vec![],
        consequences: vec!["world peace".into()],
        introduced_at: time::OffsetDateTime::now_utc(),
        deadline: None,
    };
    let json = serde_json::to_string(&pp).unwrap();
    let back: PlotPoint = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "pp-1");
    assert_eq!(back.importance, 0.9);
    assert!(matches!(back.plot_type, PlotType::MainQuest));
    assert!(matches!(back.status, PlotStatus::Active));
}

#[test]
fn violation_severity_ordering() {
    assert!(ViolationSeverity::Minor < ViolationSeverity::Moderate);
    assert!(ViolationSeverity::Moderate < ViolationSeverity::Major);
    assert!(ViolationSeverity::Major < ViolationSeverity::Critical);
}

#[test]
fn plot_type_all_variants_deserialize() {
    for variant in [
        "\"MainQuest\"",
        "\"SideQuest\"",
        "\"CharacterDevelopment\"",
        "\"WorldBuilding\"",
        "\"Conflict\"",
        "\"Resolution\"",
        "\"Revelation\"",
        "\"Foreshadowing\"",
    ] {
        let _: PlotType = serde_json::from_str(variant).unwrap();
    }
}

#[test]
fn plot_status_all_variants_deserialize() {
    for variant in [
        "\"Dormant\"",
        "\"Active\"",
        "\"Developing\"",
        "\"Resolved\"",
        "\"Abandoned\"",
    ] {
        let _: PlotStatus = serde_json::from_str(variant).unwrap();
    }
}

#[test]
fn character_state_serde_roundtrip() {
    let cs = CharacterState {
        character_id: "hero".into(),
        current_motivation: "save the world".into(),
        emotional_state: EmotionalState {
            primary_emotion: "determined".into(),
            intensity: 0.9,
            stability: 0.7,
            recent_changes: vec![],
        },
        relationships: std::collections::HashMap::new(),
        character_arc_progress: 0.5,
        secrets_known: vec!["ancient prophecy".into()],
        goals: vec!["defeat villain".into()],
        fears: vec!["failure".into()],
        last_significant_event: None,
    };
    let json = serde_json::to_string(&cs).unwrap();
    let back: CharacterState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.character_id, "hero");
    assert_eq!(back.emotional_state.intensity, 0.9);
    assert_eq!(back.character_arc_progress, 0.5);
}

#[test]
fn relationship_type_serde_roundtrip() {
    for (variant, expected) in [
        (RelationshipType::Family, "\"Family\""),
        (RelationshipType::Friend, "\"Friend\""),
        (RelationshipType::Romantic, "\"Romantic\""),
        (RelationshipType::Professional, "\"Professional\""),
        (RelationshipType::Rival, "\"Rival\""),
        (RelationshipType::Enemy, "\"Enemy\""),
        (RelationshipType::Mentor, "\"Mentor\""),
        (RelationshipType::Student, "\"Student\""),
        (RelationshipType::Unknown, "\"Unknown\""),
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, expected);
    }
}

#[test]
fn pace_type_serde_roundtrip() {
    let p = PaceType::Climactic;
    let json = serde_json::to_string(&p).unwrap();
    let back: PaceType = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, PaceType::Climactic));
}

#[test]
fn thread_status_eq() {
    assert_eq!(ThreadStatus::Introduced, ThreadStatus::Introduced);
    assert_ne!(ThreadStatus::Climax, ThreadStatus::Resolving);
}

#[test]
fn rule_type_variants_count() {
    let variants = [
        RuleType::Character,
        RuleType::World,
        RuleType::Temporal,
        RuleType::Plot,
        RuleType::Dialogue,
        RuleType::Causality,
    ];
    assert_eq!(variants.len(), 6);
}

// =========================================================================
// World events data structs
// =========================================================================

#[test]
fn trigger_type_serde_roundtrip() {
    for variant in [
        TriggerType::Time,
        TriggerType::PlayerAction,
        TriggerType::WorldState,
        TriggerType::Random,
        TriggerType::Storyline,
        TriggerType::Cascade,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: TriggerType = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }
}

#[test]
fn storyline_status_eq() {
    assert_eq!(StorylineStatus::Active, StorylineStatus::Active);
    assert_ne!(StorylineStatus::Planning, StorylineStatus::Completed);
}

#[test]
fn world_event_serde_roundtrip() {
    let we = WorldEvent {
        id: "we-1".into(),
        event_type: "invasion".into(),
        description: "Orcs attack the village".into(),
        location: Some("riverside".into()),
        participants: vec!["hero".into(), "orcs".into()],
        event_data: json!({"intensity": 0.9}),
        timestamp: time::OffsetDateTime::now_utc(),
        severity: EventSeverity::Major,
    };
    let json = serde_json::to_string(&we).unwrap();
    let back: WorldEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "we-1");
    assert_eq!(back.event_type, "invasion");
    assert!(matches!(back.severity, EventSeverity::Major));
    assert_eq!(back.participants.len(), 2);
}

#[test]
fn coordination_context_serde_roundtrip() {
    let ctx = CoordinationContext {
        session_id: "s-1".into(),
        participants: vec!["a1".into(), "a2".into()],
        shared_state: std::collections::HashMap::new(),
        coordination_goals: vec![],
        start_time: time::OffsetDateTime::now_utc(),
        last_update: time::OffsetDateTime::now_utc(),
        status: CoordinationStatus::Active,
    };
    let json = serde_json::to_string(&ctx).unwrap();
    let back: CoordinationContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back.session_id, "s-1");
    assert_eq!(back.participants.len(), 2);
    assert_eq!(back.status, CoordinationStatus::Active);
}

// =========================================================================
// Routing — enum variants
// =========================================================================

#[test]
fn routing_action_serde_roundtrip() {
    let actions = vec![
        RoutingAction::Allow,
        RoutingAction::Block,
        RoutingAction::Redirect("agent-5".into()),
        RoutingAction::Broadcast(vec!["a".into(), "b".into()]),
        RoutingAction::Transform("uppercase".into()),
    ];
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let back: RoutingAction = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }
}

#[test]
fn resource_strategy_variants() {
    let _a = ResourceStrategy::FirstCome;
    let _b = ResourceStrategy::Priority;
    let _c = ResourceStrategy::LoadBalance;
    let _d = ResourceStrategy::Adaptive;
    // No panic = all variants constructible
}

// =========================================================================
// AgentCoordinator — sync constructor
// =========================================================================

#[test]
fn agent_coordinator_new_creates_instance() {
    let config = CoordinatorConfig::default();
    let _coordinator = AgentCoordinator::new(config);
    // No panic — coordinator initializes with default config
}

#[test]
fn agent_coordinator_new_custom_config() {
    let config = CoordinatorConfig {
        max_tasks_per_agent: 10,
        max_message_queue_size: 50,
        default_task_timeout: Duration::from_secs(60),
        resource_strategy: ResourceStrategy::LoadBalance,
        enable_metrics: false,
        max_coordination_duration: Duration::from_secs(600),
    };
    let _coordinator = AgentCoordinator::new(config);
}

// =========================================================================
// AgentMessage — construction
// =========================================================================

#[test]
fn agent_message_serde_roundtrip() {
    let msg = AgentMessage {
        id: "m-1".into(),
        from: "sender".into(),
        to: "receiver".into(),
        message_type: MessageType::Question,
        content: json!({"q": "where?"}),
        timestamp: time::OffsetDateTime::now_utc(),
        priority: MessagePriority::Normal,
        reply_to: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.from, "sender");
    assert_eq!(back.to, "receiver");
    assert!(matches!(back.message_type, MessageType::Question));
    assert!(matches!(back.priority, MessagePriority::Normal));
    assert_eq!(back.reply_to, None);
}

#[test]
fn agent_message_with_reply_to() {
    let msg = AgentMessage {
        id: "m-2".into(),
        from: "a".into(),
        to: "b".into(),
        message_type: MessageType::Response,
        content: json!(null),
        timestamp: time::OffsetDateTime::now_utc(),
        priority: MessagePriority::Low,
        reply_to: Some("m-1".into()),
    };
    assert_eq!(msg.reply_to, Some("m-1".into()));
}

// =========================================================================
// SideEffect, TaskResult — construction + serde
// =========================================================================

#[test]
fn side_effect_serde_roundtrip() {
    let se = SideEffect {
        effect_type: "damage".into(),
        description: "building destroyed".into(),
        affected_entities: vec!["building-1".into()],
        impact_level: 0.8,
    };
    let json = serde_json::to_string(&se).unwrap();
    let back: SideEffect = serde_json::from_str(&json).unwrap();
    assert_eq!(back.effect_type, "damage");
    assert_eq!(back.impact_level, 0.8);
    assert_eq!(back.affected_entities.len(), 1);
}

#[test]
fn task_result_serde_roundtrip() {
    let tr = TaskResult {
        task_id: "t-1".into(),
        success: true,
        result_data: json!({"items": 5}),
        execution_time_ms: 150,
        resources_used: ResourceUsage::default(),
        error_message: None,
        side_effects: vec![],
    };
    let json = serde_json::to_string(&tr).unwrap();
    let back: TaskResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.task_id, "t-1");
    assert!(back.success);
    assert_eq!(back.execution_time_ms, 150);
    assert!(back.error_message.is_none());
}

// =========================================================================
// ResourceAllocation — serde
// =========================================================================

#[test]
fn resource_allocation_serde_roundtrip() {
    let ra = ResourceAllocation {
        agent_id: "a-1".into(),
        llm_calls_allocated: 100,
        llm_calls_used: 42,
        memory_allocated: 512,
        memory_used: 256,
        allocated_at: time::OffsetDateTime::now_utc(),
        expires_at: None,
    };
    let json = serde_json::to_string(&ra).unwrap();
    let back: ResourceAllocation = serde_json::from_str(&json).unwrap();
    assert_eq!(back.agent_id, "a-1");
    assert_eq!(back.llm_calls_allocated, 100);
    assert_eq!(back.llm_calls_used, 42);
    assert_eq!(back.memory_allocated, 512);
    assert_eq!(back.memory_used, 256);
}

// =========================================================================
// RoutingRule — serde
// =========================================================================

#[test]
fn routing_rule_serde_roundtrip() {
    let rule = RoutingRule {
        id: "r-1".into(),
        from_pattern: Some("npc-*".into()),
        to_pattern: None,
        message_type: Some("Request".into()),
        action: RoutingAction::Allow,
        priority: 10,
    };
    let json = serde_json::to_string(&rule).unwrap();
    let back: RoutingRule = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "r-1");
    assert_eq!(back.from_pattern, Some("npc-*".into()));
    assert_eq!(back.to_pattern, None);
    assert_eq!(back.priority, 10);
}

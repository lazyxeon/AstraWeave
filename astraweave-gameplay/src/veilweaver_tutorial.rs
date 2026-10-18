use std::collections::{HashMap, HashSet};

use astraweave_ecs::World;
use tracing::info;

use crate::veilweaver_slice::{
    DecisionPromptSpec, EncounterCompleteSpec, EncounterTriggerSpec, TriggerZoneSpec,
    VeilweaverSliceMetadata, WeaveAnchorSpec,
};

use astraweave_core::ecs_events::Events;
use astraweave_ecs::Entity;

pub struct AnchorStabilizedEvent {
    pub anchor_id: String,
}

pub struct TriggerVolumeEvent {
    pub trigger_id: String,
    pub entering: bool,
    pub entity: Option<Entity>,
}

#[derive(Default)]
pub struct WeaveTutorialState {
    pub anchors: HashMap<String, AnchorStatus>,
    pub trigger_zones: HashMap<String, TriggerZoneSpec>,
    pub decision_prompts: HashMap<String, DecisionPromptSpec>,
    pub encounter_triggers: HashMap<String, EncounterTriggerSpec>,
    pub encounter_completes: HashMap<String, EncounterCompleteSpec>,
    pub active_triggers: HashSet<String>,
    pub anchor_sequence: Vec<String>,
    initialized: bool,
}

#[derive(Clone)]
pub struct AnchorStatus {
    pub spec: WeaveAnchorSpec,
    pub stabilized: bool,
    pub activation_order: Option<usize>,
}

impl WeaveTutorialState {
    pub fn from_metadata(meta: VeilweaverSliceMetadata) -> Self {
        let anchors = meta
            .anchors
            .into_iter()
            .map(|anchor| {
                (
                    anchor.anchor_id.clone(),
                    AnchorStatus {
                        spec: anchor,
                        stabilized: false,
                        activation_order: None,
                    },
                )
            })
            .collect();
        let trigger_zones = meta
            .trigger_zones
            .into_iter()
            .map(|trigger| (trigger.trigger_id.clone(), trigger))
            .collect();
        let decision_prompts = meta
            .decision_prompts
            .into_iter()
            .map(|prompt| (prompt.trigger_id.clone(), prompt))
            .collect();
        let encounter_triggers = meta
            .encounter_triggers
            .into_iter()
            .map(|trigger| (trigger.trigger_id.clone(), trigger))
            .collect();
        let encounter_completes = meta
            .encounter_completes
            .into_iter()
            .map(|complete| (complete.trigger_id.clone(), complete))
            .collect();

        Self {
            anchors,
            trigger_zones,
            decision_prompts,
            encounter_triggers,
            encounter_completes,
            active_triggers: HashSet::new(),
            anchor_sequence: Vec::new(),
            initialized: false,
        }
    }

    pub fn mark_anchor_stabilized(&mut self, anchor_id: &str) {
        if let Some(anchor) = self.anchors.get_mut(anchor_id) {
            if !anchor.stabilized {
                anchor.stabilized = true;
                anchor.activation_order = Some(self.anchor_sequence.len());
                self.anchor_sequence.push(anchor_id.to_string());
                info!(
                    "Anchor stabilized: id={}, order={:?}",
                    anchor_id, anchor.activation_order
                );
            }
        }
    }

    pub fn register_trigger_activation(&mut self, trigger_id: &str) {
        self.active_triggers.insert(trigger_id.to_string());
        info!("Trigger activated: id={}", trigger_id);
    }

    pub fn register_trigger_release(&mut self, trigger_id: &str) {
        if self.active_triggers.remove(trigger_id) {
            info!("Trigger cleared: id={}", trigger_id);
        }
    }

    pub fn anchors_remaining(&self) -> usize {
        self.anchors.values().filter(|a| !a.stabilized).count()
    }
}

pub fn tutorial_anchor_sync(world: &mut World) {
    if let Some(state) = world.get_resource_mut::<WeaveTutorialState>() {
        if !state.initialized {
            info!(
                anchors = state.anchors.len(),
                triggers = state.trigger_zones.len(),
                prompts = state.decision_prompts.len(),
                "Veilweaver tutorial state initialized"
            );
            state.initialized = true;
        }
    }
}

pub fn tutorial_trigger_system(world: &mut World) {
    let events_vec = world
        .get_resource_mut::<Events<TriggerVolumeEvent>>()
        .map(|events| {
            let mut reader = events.reader();
            reader.drain().collect::<Vec<_>>()
        });

    if let (Some(state), Some(events)) =
        (world.get_resource_mut::<WeaveTutorialState>(), events_vec)
    {
        for ev in events {
            if ev.entering {
                state.register_trigger_activation(&ev.trigger_id);
            } else {
                state.register_trigger_release(&ev.trigger_id);
            }
        }
    }
}

pub fn tutorial_anchor_events(world: &mut World) {
    let events_vec = world
        .get_resource_mut::<Events<AnchorStabilizedEvent>>()
        .map(|events| {
            let mut reader = events.reader();
            reader.drain().collect::<Vec<_>>()
        });

    if let (Some(state), Some(events)) =
        (world.get_resource_mut::<WeaveTutorialState>(), events_vec)
    {
        for ev in events {
            state.mark_anchor_stabilized(&ev.anchor_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::TestValue;
    use astraweave_scene::world_partition::GridCoord;

    fn make_test_metadata() -> VeilweaverSliceMetadata {
        VeilweaverSliceMetadata {
            anchors: vec![
                WeaveAnchorSpec {
                    cell: GridCoord::new(0, 0, 0),
                    position: [0.0, 0.0, 0.0],
                    anchor_id: "anchor_1".to_string(),
                    anchor_type: Some("main".to_string()),
                    stability: Some("high".to_string()),
                    echo_cost: Some(5.0),
                },
                WeaveAnchorSpec {
                    cell: GridCoord::new(1, 0, 0),
                    position: [100.0, 0.0, 0.0],
                    anchor_id: "anchor_2".to_string(),
                    anchor_type: None,
                    stability: None,
                    echo_cost: None,
                },
            ],
            trigger_zones: vec![TriggerZoneSpec {
                cell: GridCoord::new(0, 0, 0),
                position: [10.0, 0.0, 10.0],
                trigger_id: "zone_1".to_string(),
                shape: Some("sphere".to_string()),
                radius: Some(5.0),
                extents: None,
                action: None,
            }],
            decision_prompts: vec![DecisionPromptSpec {
                cell: GridCoord::new(0, 0, 0),
                position: [0.0, 0.0, 0.0],
                trigger_id: "decision_1".to_string(),
                options: vec!["Yes".to_string(), "No".to_string()],
            }],
            encounter_triggers: vec![EncounterTriggerSpec {
                cell: GridCoord::new(0, 0, 0),
                position: [50.0, 0.0, 50.0],
                trigger_id: "encounter_1".to_string(),
                script: Some("battle.lua".to_string()),
            }],
            encounter_completes: vec![EncounterCompleteSpec {
                cell: GridCoord::new(0, 0, 0),
                position: [50.0, 0.0, 50.0],
                trigger_id: "complete_1".to_string(),
                next_cell: Some(GridCoord::new(1, 0, 0)),
            }],
            effect_anchors: vec![],
            spawn_points: vec![],
        }
    }

    // ==================== Event Struct Tests ====================

    #[test]
    fn test_anchor_stabilized_event_creation() {
        let event = AnchorStabilizedEvent {
            anchor_id: "test_anchor".to_string(),
        };
        assert_eq!(event.anchor_id, "test_anchor");
    }

    #[test]
    fn test_trigger_volume_event_entering() {
        let event = TriggerVolumeEvent {
            trigger_id: "zone_1".to_string(),
            entering: true,
            entity: None,
        };
        assert!(event.entering);
        assert!(event.entity.is_none());
    }

    #[test]
    fn test_trigger_volume_event_leaving() {
        let event = TriggerVolumeEvent {
            trigger_id: "zone_1".to_string(),
            entering: false,
            entity: Some(unsafe { Entity::from_raw(42) }),
        };
        assert!(!event.entering);
        assert!(event.entity.is_some());
    }

    // ==================== AnchorStatus Tests ====================

    #[test]
    fn test_anchor_status_clone() {
        let status = AnchorStatus {
            spec: WeaveAnchorSpec {
                cell: GridCoord::new(0, 0, 0),
                position: [0.0, 0.0, 0.0],
                anchor_id: "test".to_string(),
                anchor_type: None,
                stability: None,
                echo_cost: None,
            },
            stabilized: false,
            activation_order: None,
        };
        let cloned = status.clone();
        assert!(!cloned.stabilized);
        assert!(cloned.activation_order.is_none());
    }

    // ==================== WeaveTutorialState Tests ====================

    #[test]
    fn test_weave_tutorial_state_default() {
        let state = WeaveTutorialState::default();
        assert!(state.anchors.is_empty());
        assert!(state.trigger_zones.is_empty());
        assert!(state.decision_prompts.is_empty());
        assert!(state.encounter_triggers.is_empty());
        assert!(state.encounter_completes.is_empty());
        assert!(state.active_triggers.is_empty());
        assert!(state.anchor_sequence.is_empty());
    }

    #[test]
    fn test_weave_tutorial_state_from_metadata() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);
        assert_eq!(state.anchors.len(), 2);
        assert_eq!(state.trigger_zones.len(), 1);
        assert_eq!(state.decision_prompts.len(), 1);
        assert_eq!(state.encounter_triggers.len(), 1);
        assert_eq!(state.encounter_completes.len(), 1);
    }

    #[test]
    fn test_weave_tutorial_state_anchor_initialization() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        // Check anchor_1
        let anchor = state
            .anchors
            .get("anchor_1")
            .test_expect("anchor_1 should exist");
        assert!(!anchor.stabilized);
        assert!(anchor.activation_order.is_none());
        assert_eq!(anchor.spec.anchor_id, "anchor_1");
    }

    #[test]
    fn test_mark_anchor_stabilized() {
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);

        state.mark_anchor_stabilized("anchor_1");

        let anchor = state.anchors.get("anchor_1").test_unwrap();
        assert!(anchor.stabilized);
        assert_eq!(anchor.activation_order, Some(0));
        assert_eq!(state.anchor_sequence.len(), 1);
        assert_eq!(state.anchor_sequence[0], "anchor_1");
    }

    #[test]
    fn test_mark_anchor_stabilized_multiple() {
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);

        state.mark_anchor_stabilized("anchor_1");
        state.mark_anchor_stabilized("anchor_2");

        assert_eq!(state.anchor_sequence.len(), 2);
        assert_eq!(state.anchor_sequence[0], "anchor_1");
        assert_eq!(state.anchor_sequence[1], "anchor_2");

        let anchor1 = state.anchors.get("anchor_1").test_unwrap();
        let anchor2 = state.anchors.get("anchor_2").test_unwrap();
        assert_eq!(anchor1.activation_order, Some(0));
        assert_eq!(anchor2.activation_order, Some(1));
    }

    #[test]
    fn test_mark_anchor_stabilized_idempotent() {
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);

        state.mark_anchor_stabilized("anchor_1");
        state.mark_anchor_stabilized("anchor_1"); // Second call should be ignored

        assert_eq!(state.anchor_sequence.len(), 1);
        let anchor = state.anchors.get("anchor_1").test_unwrap();
        assert_eq!(anchor.activation_order, Some(0));
    }

    #[test]
    fn test_mark_anchor_stabilized_nonexistent() {
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);

        // Should not panic, just do nothing
        state.mark_anchor_stabilized("nonexistent_anchor");
        assert!(state.anchor_sequence.is_empty());
    }

    #[test]
    fn test_register_trigger_activation() {
        let mut state = WeaveTutorialState::default();

        state.register_trigger_activation("trigger_1");

        assert!(state.active_triggers.contains("trigger_1"));
        assert_eq!(state.active_triggers.len(), 1);
    }

    #[test]
    fn test_register_trigger_activation_multiple() {
        let mut state = WeaveTutorialState::default();

        state.register_trigger_activation("trigger_1");
        state.register_trigger_activation("trigger_2");
        state.register_trigger_activation("trigger_3");

        assert_eq!(state.active_triggers.len(), 3);
        assert!(state.active_triggers.contains("trigger_1"));
        assert!(state.active_triggers.contains("trigger_2"));
        assert!(state.active_triggers.contains("trigger_3"));
    }

    #[test]
    fn test_register_trigger_activation_idempotent() {
        let mut state = WeaveTutorialState::default();

        state.register_trigger_activation("trigger_1");
        state.register_trigger_activation("trigger_1");

        assert_eq!(state.active_triggers.len(), 1);
    }

    #[test]
    fn test_register_trigger_release() {
        let mut state = WeaveTutorialState::default();

        state.register_trigger_activation("trigger_1");
        assert!(state.active_triggers.contains("trigger_1"));

        state.register_trigger_release("trigger_1");
        assert!(!state.active_triggers.contains("trigger_1"));
    }

    #[test]
    fn test_register_trigger_release_nonexistent() {
        let mut state = WeaveTutorialState::default();

        // Should not panic
        state.register_trigger_release("nonexistent");
        assert!(state.active_triggers.is_empty());
    }

    #[test]
    fn test_anchors_remaining() {
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);

        assert_eq!(state.anchors_remaining(), 2);

        state.mark_anchor_stabilized("anchor_1");
        assert_eq!(state.anchors_remaining(), 1);

        state.mark_anchor_stabilized("anchor_2");
        assert_eq!(state.anchors_remaining(), 0);
    }

    #[test]
    fn test_anchors_remaining_empty() {
        let state = WeaveTutorialState::default();
        assert_eq!(state.anchors_remaining(), 0);
    }

    // ==================== System Function Tests ====================

    #[test]
    fn test_tutorial_anchor_sync_initializes() {
        let mut world = World::new();
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);
        world.insert_resource(state);

        // First call should initialize
        tutorial_anchor_sync(&mut world);

        let _state = world.get_resource::<WeaveTutorialState>().test_unwrap();
        // The initialized flag should now be true (internal state)
        // We can verify this by calling again and seeing no panic
        tutorial_anchor_sync(&mut world);
    }

    #[test]
    fn test_tutorial_anchor_sync_no_state() {
        let mut world = World::new();
        // Should not panic when no state resource exists
        tutorial_anchor_sync(&mut world);
    }

    #[test]
    fn test_tutorial_trigger_system_no_events() {
        let mut world = World::new();
        let state = WeaveTutorialState::default();
        world.insert_resource(state);
        world.insert_resource(Events::<TriggerVolumeEvent>::default());

        // Should not panic
        tutorial_trigger_system(&mut world);
    }

    #[test]
    fn test_tutorial_anchor_events_no_events() {
        let mut world = World::new();
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);
        world.insert_resource(state);
        world.insert_resource(Events::<AnchorStabilizedEvent>::default());

        // Should not panic
        tutorial_anchor_events(&mut world);
    }

    #[test]
    fn test_trigger_zone_spec_stored_correctly() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        let zone = state
            .trigger_zones
            .get("zone_1")
            .test_expect("zone_1 should exist");
        assert_eq!(zone.trigger_id, "zone_1");
        assert_eq!(zone.shape, Some("sphere".to_string()));
        assert!((zone.radius.test_unwrap() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_decision_prompt_stored_correctly() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        let prompt = state
            .decision_prompts
            .get("decision_1")
            .test_expect("decision_1 should exist");
        assert_eq!(prompt.trigger_id, "decision_1");
        assert_eq!(prompt.options.len(), 2);
    }

    #[test]
    fn test_encounter_trigger_stored_correctly() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        let trigger = state
            .encounter_triggers
            .get("encounter_1")
            .test_expect("encounter_1 should exist");
        assert_eq!(trigger.script, Some("battle.lua".to_string()));
    }

    #[test]
    fn test_encounter_complete_stored_correctly() {
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        let complete = state
            .encounter_completes
            .get("complete_1")
            .test_expect("complete_1 should exist");
        assert!(complete.next_cell.is_some());
    }

    #[test]
    fn test_empty_metadata_creates_empty_state() {
        let meta = VeilweaverSliceMetadata::default();
        let state = WeaveTutorialState::from_metadata(meta);

        assert!(state.anchors.is_empty());
        assert!(state.trigger_zones.is_empty());
        assert!(state.decision_prompts.is_empty());
        assert!(state.encounter_triggers.is_empty());
        assert!(state.encounter_completes.is_empty());
    }

    // ===== Mutation-resistant tests for tutorial systems =====

    #[test]
    fn tutorial_anchor_sync_sets_initialized_flag() {
        // Catches: replace tutorial_anchor_sync with () (line 123)
        // Catches: delete ! in !state.initialized (line 124)
        let mut world = World::new();
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);
        state.initialized = false;
        world.insert_resource(state);

        tutorial_anchor_sync(&mut world);

        let state_after = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(
            state_after.initialized,
            "tutorial_anchor_sync must set initialized=true on first call"
        );

        // Calling again should not change anything (already initialized)
        tutorial_anchor_sync(&mut world);
        let state_after2 = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(
            state_after2.initialized,
            "tutorial_anchor_sync must remain initialized after second call"
        );
    }

    #[test]
    fn tutorial_anchor_sync_not_initialized_guard() {
        // If delete ! mutant passes: state.initialized (true) → runs init again
        // But the function already sets initialized=true, so to test the ! guard:
        // already-initialized state should NOT be re-initialized (idempotent behavior)
        let mut world = World::new();
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);
        state.initialized = true; // Already initialized
        world.insert_resource(state);

        // Should be a no-op
        tutorial_anchor_sync(&mut world);

        let state_after = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(state_after.initialized);
    }

    #[test]
    fn tutorial_trigger_system_processes_entering_events() {
        // Catches: replace tutorial_trigger_system with () (line 137)
        let mut world = World::new();
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);
        world.insert_resource(state);

        let mut events = Events::<TriggerVolumeEvent>::default();
        events.writer().send(TriggerVolumeEvent {
            trigger_id: "zone_1".to_string(),
            entering: true,
            entity: None,
        });
        world.insert_resource(events);

        tutorial_trigger_system(&mut world);

        let state = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(
            state.active_triggers.contains("zone_1"),
            "tutorial_trigger_system must process entering events"
        );
    }

    #[test]
    fn tutorial_trigger_system_processes_leaving_events() {
        // Catches: replace tutorial_trigger_system with ()
        let mut world = World::new();
        let meta = make_test_metadata();
        let mut state = WeaveTutorialState::from_metadata(meta);
        state.active_triggers.insert("zone_1".to_string());
        world.insert_resource(state);

        let mut events = Events::<TriggerVolumeEvent>::default();
        events.writer().send(TriggerVolumeEvent {
            trigger_id: "zone_1".to_string(),
            entering: false,
            entity: None,
        });
        world.insert_resource(events);

        tutorial_trigger_system(&mut world);

        let state = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(
            !state.active_triggers.contains("zone_1"),
            "tutorial_trigger_system must process leaving events"
        );
    }

    #[test]
    fn tutorial_anchor_events_stabilizes_anchor() {
        // Catches: replace tutorial_anchor_events with () (line 158)
        let mut world = World::new();
        let meta = make_test_metadata();
        let state = WeaveTutorialState::from_metadata(meta);

        // Verify anchor starts unstabilized
        assert!(!state.anchors.get("anchor_1").test_unwrap().stabilized);
        world.insert_resource(state);

        let mut events = Events::<AnchorStabilizedEvent>::default();
        events.writer().send(AnchorStabilizedEvent {
            anchor_id: "anchor_1".to_string(),
        });
        world.insert_resource(events);

        tutorial_anchor_events(&mut world);

        let state = world.get_resource::<WeaveTutorialState>().test_unwrap();
        assert!(
            state.anchors.get("anchor_1").test_unwrap().stabilized,
            "tutorial_anchor_events must mark anchor as stabilized"
        );
        assert!(
            state.anchor_sequence.contains(&"anchor_1".to_string()),
            "tutorial_anchor_events must record anchor in sequence"
        );
    }
}

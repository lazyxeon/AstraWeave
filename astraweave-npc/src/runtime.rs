use anyhow::Result;
use glam::{vec3, Vec3};
use std::collections::HashMap;

use astraweave_audio::AudioEngine;
use astraweave_physics::{BodyId, PhysicsWorld};

use crate::{llm::LlmAdapter, profile::NpcProfile, NpcAction, NpcMode, NpcWorldView};

pub type NpcId = u64;

pub trait CommandSink {
    fn move_character(&mut self, body: BodyId, dir: Vec3, speed: f32);
    fn say(&mut self, speaker: &str, text: &str);
    fn open_shop(&mut self, npc_id: NpcId);
    fn call_guards(&mut self, pos: Vec3, reason: &str);
    fn give_quest(&mut self, npc_id: NpcId, quest_id: &str);
}

pub struct EngineCommandSink<'a> {
    pub phys: &'a mut PhysicsWorld,
    pub audio: &'a mut AudioEngine,
}

impl<'a> CommandSink for EngineCommandSink<'a> {
    fn move_character(&mut self, body: BodyId, dir: Vec3, speed: f32) {
        let v = if dir.length_squared() > 1e-4 {
            dir.normalize() * speed
        } else {
            Vec3::ZERO
        };
        // character vertical handled 0 here
        self.phys
            .control_character(body, vec3(v.x, 0.0, v.z), 1.0 / 60.0, false);
    }

    fn say(&mut self, _speaker: &str, text: &str) {
        // Play a voice beep scaled to text length (your audio crate can do VO/TTS later)
        self.audio.play_voice_beep(text.len());
        println!("{}: {}", _speaker, text);
    }

    fn open_shop(&mut self, _npc_id: NpcId) {
        println!("[Shop] Opened shop UI (placeholder)");
    }

    fn call_guards(&mut self, pos: Vec3, reason: &str) {
        println!("[Guards] Called to {:?} because {}", pos, reason);
        let _ = self.audio.play_sfx_3d_beep(10, pos, 600.0, 0.25, 0.5);
    }

    fn give_quest(&mut self, _npc_id: NpcId, quest_id: &str) {
        println!("[Quest] Offered quest {}", quest_id);
    }
}

pub struct Npc {
    pub id: NpcId,
    pub profile: NpcProfile,
    pub body: BodyId,
    pub mode: NpcMode,
    pub pending: Vec<NpcAction>,
    pub cooldown_talk: f32,
}

pub struct NpcManager {
    next_id: NpcId,
    npcs: HashMap<NpcId, Npc>,
    planner: Box<dyn LlmAdapter>,
}

impl NpcManager {
    pub fn new(planner: Box<dyn LlmAdapter>) -> Self {
        Self {
            next_id: 1,
            npcs: HashMap::new(),
            planner,
        }
    }

    pub fn spawn_from_profile(&mut self, phys: &mut PhysicsWorld, prof: NpcProfile) -> NpcId {
        // convert home to spawn pos; capsule half extents similar to character
        let pos = prof.home_vec3();
        let body = phys.add_character(pos, vec3(0.4, 0.9, 0.4));
        let id = self.alloc_id();
        self.npcs.insert(
            id,
            Npc {
                id,
                profile: prof,
                body,
                mode: NpcMode::Idle,
                pending: vec![],
                cooldown_talk: 0.0,
            },
        );
        id
    }

    /// Spawn an NPC that does not touch the physics world.
    ///
    /// The sim tick uses this so a plan can be recorded without stepping Rapier.
    pub fn spawn_logical(&mut self, prof: NpcProfile) -> NpcId {
        let id = self.alloc_id();
        self.npcs.insert(
            id,
            Npc {
                id,
                profile: prof,
                body: 0,
                mode: NpcMode::Idle,
                pending: Vec::new(),
                cooldown_talk: 0.0,
            },
        );
        id
    }

    pub fn update(
        &mut self,
        dt: f32,
        glue: &mut dyn CommandSink,
        views: &HashMap<NpcId, NpcWorldView>,
    ) {
        // Collect actions to execute to avoid borrowing issues
        let mut actions_to_execute = Vec::new();

        let mut ids: Vec<NpcId> = self.npcs.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let npc = match self.npcs.get_mut(&id) {
                Some(npc) => npc,
                None => continue,
            };
            // cooldowns
            npc.cooldown_talk = (npc.cooldown_talk - dt).max(0.0);

            // execute one pending action per tick to keep things readable
            if let Some(act) = npc.pending.first().cloned() {
                actions_to_execute.push((
                    npc.id,
                    npc.body,
                    npc.profile.persona.display_name.clone(),
                    act,
                ));
                npc.pending.remove(0);
            } else {
                // idle micro-behavior: guards patrol slowly; merchants idle
                if npc.profile.role == crate::profile::Role::Guard {
                    if let Some(view) = views.get(&npc.id) {
                        if let Some(pd) = view.player_dist {
                            if pd < 2.0 {
                                // step aside a bit
                                if let Some(player_pos) = view.player_pos {
                                    let dir = (view.self_pos - player_pos).normalize_or_zero();
                                    glue.move_character(npc.body, dir, 0.6);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Execute collected actions
        for (npc_id, body, display_name, act) in actions_to_execute {
            Self::execute_action(glue, npc_id, body, &display_name, &act);
        }
    }

    pub fn handle_player_utterance(
        &mut self,
        npc_id: NpcId,
        view: &NpcWorldView,
        utter: &str,
    ) -> Result<()> {
        if let Some(npc) = self.npcs.get_mut(&npc_id) {
            if npc.cooldown_talk > 0.0 {
                return Ok(());
            }
            let plan = self
                .planner
                .plan_dialogue_and_behaviour(&npc.profile, view, Some(utter))?;
            npc.pending.extend(plan.actions);
            npc.mode = NpcMode::Conversing;
            npc.cooldown_talk = 0.5;
        }
        Ok(())
    }

    fn execute_action(
        glue: &mut dyn CommandSink,
        npc_id: NpcId,
        body: BodyId,
        display_name: &str,
        act: &NpcAction,
    ) {
        match act {
            NpcAction::Say { text } => glue.say(display_name, text),
            NpcAction::MoveTo { pos, speed } => {
                // move in direct line toward pos (simple demo; pathfinding can be added)
                // direction = (pos - current). normalized
                // For now, we cannot query position from CommandSink, so move toward target directly
                let dir = Vec3::new(pos.x - 0.0, 0.0, pos.z - 0.0); // placeholder calculation
                glue.move_character(body, dir, *speed);
            }
            NpcAction::Emote { kind } => {
                println!("{} emotes {:?}", display_name, kind);
            }
            NpcAction::OpenShop => glue.open_shop(npc_id),
            NpcAction::GiveQuest { id } => glue.give_quest(npc_id, id),
            NpcAction::CallGuards { reason } => {
                // For now, use a placeholder position since we can't query body position via CommandSink
                let placeholder_pos = Vec3::new(0.0, 0.0, 0.0);
                glue.call_guards(placeholder_pos, reason);
            }
        }
    }

    #[allow(dead_code)]
    fn body_pos(&self, _glue: &dyn CommandSink, _body: BodyId) -> Option<Vec3> {
        // For now, we cannot query position via CommandSink.
        // In your integration, pull from PhysicsWorld or World. For demo, return None to skip direction calc (handled by move_character).
        None
    }

    fn alloc_id(&mut self) -> NpcId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::NpcPlan;
    use crate::llm::LlmAdapter;
    use crate::profile::{Memory, Persona, Role};

    /// Mock CommandSink for testing action execution
    struct MockCommandSink {
        moves: Vec<(BodyId, Vec3, f32)>,
        says: Vec<(String, String)>,
        shops_opened: Vec<NpcId>,
        guards_called: Vec<(Vec3, String)>,
        quests_given: Vec<(NpcId, String)>,
    }

    impl MockCommandSink {
        fn new() -> Self {
            Self {
                moves: vec![],
                says: vec![],
                shops_opened: vec![],
                guards_called: vec![],
                quests_given: vec![],
            }
        }
    }

    impl CommandSink for MockCommandSink {
        fn move_character(&mut self, body: BodyId, dir: Vec3, speed: f32) {
            self.moves.push((body, dir, speed));
        }

        fn say(&mut self, speaker: &str, text: &str) {
            self.says.push((speaker.to_string(), text.to_string()));
        }

        fn open_shop(&mut self, npc_id: NpcId) {
            self.shops_opened.push(npc_id);
        }

        fn call_guards(&mut self, pos: Vec3, reason: &str) {
            self.guards_called.push((pos, reason.to_string()));
        }

        fn give_quest(&mut self, npc_id: NpcId, quest_id: &str) {
            self.quests_given.push((npc_id, quest_id.to_string()));
        }
    }

    /// Mock LLM adapter for testing NPC planning
    struct MockLlm {
        plan: NpcPlan,
    }

    impl MockLlm {
        fn new(actions: Vec<NpcAction>) -> Self {
            Self {
                plan: NpcPlan { actions },
            }
        }
    }

    impl LlmAdapter for MockLlm {
        fn plan_dialogue_and_behaviour(
            &self,
            _profile: &NpcProfile,
            _view: &NpcWorldView,
            _player_utterance: Option<&str>,
        ) -> Result<NpcPlan> {
            Ok(self.plan.clone())
        }
    }

    fn make_test_profile() -> NpcProfile {
        NpcProfile {
            id: "test_npc".into(),
            role: Role::Guard,
            persona: Persona {
                display_name: "Test NPC".into(),
                traits: vec!["brave".into()],
                backstory: "A test NPC".into(),
                voice_speaker: None,
            },
            memory: Memory {
                facts: vec![],
                episodes: vec![],
            },
            home: [0.0, 0.0, 0.0],
            schedule: vec![],
        }
    }

    fn make_test_view() -> NpcWorldView {
        NpcWorldView {
            time_of_day: 12.0,
            self_pos: Vec3::ZERO,
            player_pos: Some(Vec3::new(1.0, 0.0, 1.0)),
            player_dist: Some(1.5),
            nearby_threat: false,
            location_tag: None,
        }
    }

    // ============== NpcManager Tests ==============

    #[test]
    fn test_npc_manager_new() {
        let planner = Box::new(MockLlm::new(vec![]));
        let manager = NpcManager::new(planner);
        assert_eq!(manager.next_id, 1);
        assert!(manager.npcs.is_empty());
    }

    #[test]
    fn test_npc_manager_alloc_id() {
        let planner = Box::new(MockLlm::new(vec![]));
        let mut manager = NpcManager::new(planner);
        assert_eq!(manager.alloc_id(), 1);
        assert_eq!(manager.alloc_id(), 2);
        assert_eq!(manager.alloc_id(), 3);
    }

    // ============== Action Execution Tests ==============

    #[test]
    fn test_execute_action_say() {
        let mut sink = MockCommandSink::new();
        NpcManager::execute_action(
            &mut sink,
            1,
            0,
            "Bob",
            &NpcAction::Say {
                text: "Hello!".into(),
            },
        );
        assert_eq!(sink.says.len(), 1);
        assert_eq!(sink.says[0], ("Bob".to_string(), "Hello!".to_string()));
    }

    #[test]
    fn test_execute_action_move_to() {
        let mut sink = MockCommandSink::new();
        NpcManager::execute_action(
            &mut sink,
            1,
            5,
            "Guard",
            &NpcAction::MoveTo {
                pos: Vec3::new(10.0, 0.0, 10.0),
                speed: 2.5,
            },
        );
        assert_eq!(sink.moves.len(), 1);
        assert_eq!(sink.moves[0].0, 5);
        assert_eq!(sink.moves[0].2, 2.5);
    }

    #[test]
    fn test_execute_action_emote() {
        let mut sink = MockCommandSink::new();
        // Emote just prints, no sink effect
        NpcManager::execute_action(
            &mut sink,
            1,
            0,
            "Actor",
            &NpcAction::Emote {
                kind: crate::EmoteKind::Wave,
            },
        );
        // No assertions needed - just ensure no panic
    }

    #[test]
    fn test_execute_action_open_shop() {
        let mut sink = MockCommandSink::new();
        NpcManager::execute_action(&mut sink, 42, 0, "Merchant", &NpcAction::OpenShop);
        assert_eq!(sink.shops_opened, vec![42]);
    }

    #[test]
    fn test_execute_action_give_quest() {
        let mut sink = MockCommandSink::new();
        NpcManager::execute_action(
            &mut sink,
            7,
            0,
            "Elder",
            &NpcAction::GiveQuest {
                id: "main_quest_01".into(),
            },
        );
        assert_eq!(sink.quests_given, vec![(7, "main_quest_01".to_string())]);
    }

    #[test]
    fn test_execute_action_call_guards() {
        let mut sink = MockCommandSink::new();
        NpcManager::execute_action(
            &mut sink,
            1,
            0,
            "Victim",
            &NpcAction::CallGuards {
                reason: "Thief spotted".into(),
            },
        );
        assert_eq!(sink.guards_called.len(), 1);
        assert_eq!(sink.guards_called[0].1, "Thief spotted");
    }

    // ============== Npc Struct Tests ==============

    #[test]
    fn test_npc_struct_creation() {
        let npc = Npc {
            id: 1,
            profile: make_test_profile(),
            body: 10,
            mode: NpcMode::Idle,
            pending: vec![],
            cooldown_talk: 0.0,
        };
        assert_eq!(npc.id, 1);
        assert_eq!(npc.body, 10);
        assert!(matches!(npc.mode, NpcMode::Idle));
        assert!(npc.pending.is_empty());
    }

    #[test]
    fn test_npc_pending_actions() {
        let mut npc = Npc {
            id: 1,
            profile: make_test_profile(),
            body: 0,
            mode: NpcMode::Idle,
            pending: vec![
                NpcAction::Say {
                    text: "First".into(),
                },
                NpcAction::Say {
                    text: "Second".into(),
                },
            ],
            cooldown_talk: 0.0,
        };
        assert_eq!(npc.pending.len(), 2);
        npc.pending.remove(0);
        assert_eq!(npc.pending.len(), 1);
    }

    #[test]
    fn test_npc_cooldown_decrement() {
        let mut npc = Npc {
            id: 1,
            profile: make_test_profile(),
            body: 0,
            mode: NpcMode::Idle,
            pending: vec![],
            cooldown_talk: 1.0,
        };
        // Simulate update decrement
        npc.cooldown_talk = (npc.cooldown_talk - 0.5).max(0.0);
        assert!((npc.cooldown_talk - 0.5).abs() < 0.001);
        npc.cooldown_talk = (npc.cooldown_talk - 1.0).max(0.0);
        assert_eq!(npc.cooldown_talk, 0.0);
    }

    // ============== Update Flow Tests (without PhysicsWorld) ==============

    #[test]
    fn test_manager_update_executes_pending_actions() {
        let planner = Box::new(MockLlm::new(vec![]));
        let mut manager = NpcManager::new(planner);

        // Manually insert an NPC with pending action
        manager.npcs.insert(
            1,
            Npc {
                id: 1,
                profile: make_test_profile(),
                body: 0,
                mode: NpcMode::Idle,
                pending: vec![NpcAction::Say {
                    text: "Hello world".into(),
                }],
                cooldown_talk: 0.0,
            },
        );
        manager.next_id = 2;

        let mut sink = MockCommandSink::new();
        let views = HashMap::new();

        manager.update(1.0 / 60.0, &mut sink, &views);

        assert_eq!(sink.says.len(), 1);
        assert_eq!(sink.says[0].1, "Hello world");
        assert!(manager.npcs.get(&1).unwrap().pending.is_empty());
    }

    #[test]
    fn test_manager_update_cooldown_decrement() {
        let planner = Box::new(MockLlm::new(vec![]));
        let mut manager = NpcManager::new(planner);

        manager.npcs.insert(
            1,
            Npc {
                id: 1,
                profile: make_test_profile(),
                body: 0,
                mode: NpcMode::Idle,
                pending: vec![],
                cooldown_talk: 1.0,
            },
        );
        manager.next_id = 2;

        let mut sink = MockCommandSink::new();
        let views = HashMap::new();

        // Update with dt = 0.5
        manager.update(0.5, &mut sink, &views);

        let npc = manager.npcs.get(&1).unwrap();
        assert!((npc.cooldown_talk - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_manager_update_multiple_npcs() {
        let planner = Box::new(MockLlm::new(vec![]));
        let mut manager = NpcManager::new(planner);

        // Insert two NPCs with pending actions
        manager.npcs.insert(
            1,
            Npc {
                id: 1,
                profile: make_test_profile(),
                body: 0,
                mode: NpcMode::Idle,
                pending: vec![NpcAction::Say {
                    text: "NPC1".into(),
                }],
                cooldown_talk: 0.0,
            },
        );
        manager.npcs.insert(
            2,
            Npc {
                id: 2,
                profile: make_test_profile(),
                body: 1,
                mode: NpcMode::Idle,
                pending: vec![NpcAction::OpenShop],
                cooldown_talk: 0.0,
            },
        );
        manager.next_id = 3;

        let mut sink = MockCommandSink::new();
        let views = HashMap::new();

        manager.update(1.0 / 60.0, &mut sink, &views);

        assert_eq!(sink.says.len(), 1);
        assert_eq!(sink.shops_opened.len(), 1);
    }

    #[test]
    fn test_handle_player_utterance_cooldown_blocks() {
        let planner = Box::new(MockLlm::new(vec![NpcAction::Say {
            text: "response".into(),
        }]));
        let mut manager = NpcManager::new(planner);

        manager.npcs.insert(
            1,
            Npc {
                id: 1,
                profile: make_test_profile(),
                body: 0,
                mode: NpcMode::Idle,
                pending: vec![],
                cooldown_talk: 1.0, // On cooldown
            },
        );
        manager.next_id = 2;

        let view = make_test_view();

        let result = manager.handle_player_utterance(1, &view, "Hello");
        assert!(result.is_ok());
        // Should NOT add pending because cooldown active
        assert!(manager.npcs.get(&1).unwrap().pending.is_empty());
    }

    #[test]
    fn test_handle_player_utterance_success() {
        let planner = Box::new(MockLlm::new(vec![
            NpcAction::Say {
                text: "Hello traveler".into(),
            },
            NpcAction::Emote {
                kind: crate::EmoteKind::Wave,
            },
        ]));
        let mut manager = NpcManager::new(planner);

        manager.npcs.insert(
            1,
            Npc {
                id: 1,
                profile: make_test_profile(),
                body: 0,
                mode: NpcMode::Idle,
                pending: vec![],
                cooldown_talk: 0.0,
            },
        );
        manager.next_id = 2;

        let view = make_test_view();

        let result = manager.handle_player_utterance(1, &view, "Hello");
        assert!(result.is_ok());

        let npc = manager.npcs.get(&1).unwrap();
        assert_eq!(npc.pending.len(), 2);
        assert!(matches!(npc.mode, NpcMode::Conversing));
        assert!((npc.cooldown_talk - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_handle_player_utterance_nonexistent_npc() {
        let planner = Box::new(MockLlm::new(vec![]));
        let mut manager = NpcManager::new(planner);

        let view = NpcWorldView::default();

        // NPC 99 doesn't exist
        let result = manager.handle_player_utterance(99, &view, "Hello");
        assert!(result.is_ok()); // Should just return Ok without panic
    }
}

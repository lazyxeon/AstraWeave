use anyhow::Result;
use glam::Vec3;
use rand::Rng;

use crate::{
    profile::{NpcProfile, Role},
    EmoteKind, NpcAction, NpcPlan, NpcWorldView,
};

/// Adapter trait so you can swap in a real LLM / Inworld-like service.
pub trait LlmAdapter: Send + Sync {
    fn plan_dialogue_and_behaviour(
        &self,
        profile: &NpcProfile,
        view: &NpcWorldView,
        player_utterance: Option<&str>,
    ) -> Result<NpcPlan>;
}

/// A simple mock planner with hand-written heuristics.
/// Replace with a real adapter later.
pub struct MockLlm;

impl LlmAdapter for MockLlm {
    fn plan_dialogue_and_behaviour(
        &self,
        profile: &NpcProfile,
        view: &NpcWorldView,
        player_utterance: Option<&str>,
    ) -> Result<NpcPlan> {
        let mut rng = rand::rng();
        let mut actions = vec![];

        match profile.role {
            Role::Merchant => {
                if let Some(u) = player_utterance {
                    if u.to_lowercase().contains("buy") || u.to_lowercase().contains("shop") {
                        actions.push(NpcAction::Say {
                            text: "Take a look—finest wares this side of the veil.".into(),
                        });
                        actions.push(NpcAction::OpenShop);
                    } else {
                        actions.push(NpcAction::Say {
                            text: "Greetings, traveler. Looking for supplies?".into(),
                        });
                        actions.push(NpcAction::Emote {
                            kind: EmoteKind::Nod,
                        });
                    }
                } else {
                    // idle/working
                    if rng.random::<f32>() < 0.3 {
                        actions.push(NpcAction::Emote {
                            kind: EmoteKind::Wave,
                        });
                    }
                }
            }
            Role::Guard => {
                if view.nearby_threat {
                    actions.push(NpcAction::Say {
                        text: "Stay back—this area isn’t safe.".into(),
                    });
                    actions.push(NpcAction::CallGuards {
                        reason: "Threat detected".into(),
                    });
                } else if let Some(u) = player_utterance {
                    if u.to_lowercase().contains("danger") || u.to_lowercase().contains("help") {
                        actions.push(NpcAction::Say {
                            text: "I’ll alert the watch. Keep your distance.".into(),
                        });
                        actions.push(NpcAction::CallGuards {
                            reason: "Player reported danger".into(),
                        });
                    } else {
                        actions.push(NpcAction::Say {
                            text: "Move along. Keep the peace.".into(),
                        });
                    }
                } else if rng.random::<f32>() < 0.5 {
                    // tiny patrol step
                    let step = view.self_pos
                        + Vec3::new(
                            rng.random_range(-1.0..1.0),
                            0.0,
                            rng.random_range(-1.0..1.0),
                        );
                    actions.push(NpcAction::MoveTo {
                        pos: step,
                        speed: 1.2,
                    });
                }
            }
            Role::Civilian => {
                if let Some(u) = player_utterance {
                    if u.to_lowercase().contains("hello") {
                        actions.push(NpcAction::Say {
                            text: "Oh! Hello there.".into(),
                        });
                        actions.push(NpcAction::Emote {
                            kind: EmoteKind::Wave,
                        });
                    } else {
                        actions.push(NpcAction::Say {
                            text: "Sorry—busy day.".into(),
                        });
                    }
                }
            }
            Role::QuestGiver => {
                if let Some(u) = player_utterance {
                    if u.to_lowercase().contains("quest") || u.to_lowercase().contains("work") {
                        actions.push(NpcAction::Say {
                            text: "There is something you can do...".into(),
                        });
                        actions.push(NpcAction::GiveQuest {
                            id: "q_tutorial".into(),
                        });
                    } else {
                        actions.push(NpcAction::Say {
                            text: "The threads whisper to those who listen.".into(),
                        });
                    }
                }
            }
        }

        Ok(NpcPlan { actions })
    }
}

/// Planner used on the simulation tick.
///
/// Same role table as [`MockLlm`], with the `rand::rng` branches replaced by
/// a fixed line. No float draws, no wall clock.
pub struct RulePlanner;

impl LlmAdapter for RulePlanner {
    fn plan_dialogue_and_behaviour(
        &self,
        profile: &NpcProfile,
        view: &NpcWorldView,
        player_utterance: Option<&str>,
    ) -> Result<NpcPlan> {
        let mut actions = Vec::new();
        let said = player_utterance.unwrap_or("").to_lowercase();
        match profile.role {
            Role::Merchant => {
                if said.contains("buy") || said.contains("shop") {
                    actions.push(NpcAction::Say {
                        text: "Take a look—finest wares this side of the veil.".into(),
                    });
                    actions.push(NpcAction::OpenShop);
                } else if !said.is_empty() {
                    actions.push(NpcAction::Say {
                        text: "Greetings, traveler. Looking for supplies?".into(),
                    });
                    actions.push(NpcAction::Emote {
                        kind: EmoteKind::Nod,
                    });
                } else {
                    actions.push(NpcAction::Say {
                        text: "Greetings, traveler. Looking for supplies?".into(),
                    });
                }
            }
            Role::Guard => {
                if view.nearby_threat {
                    actions.push(NpcAction::Say {
                        text: "Stay back—this area isn’t safe.".into(),
                    });
                    actions.push(NpcAction::CallGuards {
                        reason: "Threat detected".into(),
                    });
                } else if said.contains("danger") || said.contains("help") {
                    actions.push(NpcAction::Say {
                        text: "I’ll alert the watch. Keep your distance.".into(),
                    });
                    actions.push(NpcAction::CallGuards {
                        reason: "Player reported danger".into(),
                    });
                } else if !said.is_empty() {
                    actions.push(NpcAction::Say {
                        text: "Move along. Keep the peace.".into(),
                    });
                } else {
                    actions.push(NpcAction::Say {
                        text: "Holding the watch.".into(),
                    });
                }
            }
            Role::Civilian => {
                if said.contains("hello") {
                    actions.push(NpcAction::Say {
                        text: "Oh! Hello there.".into(),
                    });
                    actions.push(NpcAction::Emote {
                        kind: EmoteKind::Wave,
                    });
                } else if !said.is_empty() {
                    actions.push(NpcAction::Say {
                        text: "Sorry—busy day.".into(),
                    });
                }
            }
            Role::QuestGiver => {
                if said.contains("quest") || said.contains("work") {
                    actions.push(NpcAction::Say {
                        text: "There is something you can do...".into(),
                    });
                    actions.push(NpcAction::GiveQuest {
                        id: "q_tutorial".into(),
                    });
                } else if !said.is_empty() {
                    actions.push(NpcAction::Say {
                        text: "The threads whisper to those who listen.".into(),
                    });
                }
            }
        }
        Ok(NpcPlan { actions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Memory, Persona};

    fn make_profile(role: Role) -> NpcProfile {
        NpcProfile {
            id: "test_npc".to_string(),
            role,
            persona: Persona {
                display_name: "Test NPC".to_string(),
                traits: vec![],
                backstory: String::new(),
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

    fn make_view() -> NpcWorldView {
        NpcWorldView {
            time_of_day: 12.0,
            self_pos: Vec3::ZERO,
            player_pos: Some(Vec3::new(5.0, 0.0, 5.0)),
            player_dist: Some(7.07),
            nearby_threat: false,
            location_tag: Some("market".to_string()),
        }
    }

    #[test]
    fn test_mock_llm_merchant_buy() {
        let llm = MockLlm;
        let profile = make_profile(Role::Merchant);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("I want to buy something"))
            .unwrap();

        // Should say something and open shop
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::OpenShop)));
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
    }

    #[test]
    fn test_mock_llm_merchant_shop() {
        let llm = MockLlm;
        let profile = make_profile(Role::Merchant);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("show me your shop"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::OpenShop)));
    }

    #[test]
    fn test_mock_llm_merchant_greeting() {
        let llm = MockLlm;
        let profile = make_profile(Role::Merchant);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("hello there"))
            .unwrap();

        // Should greet and maybe emote
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            NpcAction::Emote {
                kind: EmoteKind::Nod
            }
        )));
    }

    #[test]
    fn test_mock_llm_merchant_idle() {
        let llm = MockLlm;
        let profile = make_profile(Role::Merchant);
        let view = make_view();

        // No utterance - idle behavior (may or may not wave randomly)
        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, None)
            .unwrap();

        // Plan should be valid (may be empty or have a wave)
        assert!(plan.actions.len() <= 1);
    }

    #[test]
    fn test_mock_llm_guard_threat() {
        let llm = MockLlm;
        let profile = make_profile(Role::Guard);
        let mut view = make_view();
        view.nearby_threat = true;

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, None)
            .unwrap();

        // Should call guards due to threat
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::CallGuards { .. })));
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
    }

    #[test]
    fn test_mock_llm_guard_player_reports_danger() {
        let llm = MockLlm;
        let profile = make_profile(Role::Guard);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("there's danger ahead!"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::CallGuards { .. })));
    }

    #[test]
    fn test_mock_llm_guard_player_asks_help() {
        let llm = MockLlm;
        let profile = make_profile(Role::Guard);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("help me please"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::CallGuards { .. })));
    }

    #[test]
    fn test_mock_llm_guard_normal_interaction() {
        let llm = MockLlm;
        let profile = make_profile(Role::Guard);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("nice weather"))
            .unwrap();

        // Should tell player to move along
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
    }

    #[test]
    fn test_mock_llm_civilian_hello() {
        let llm = MockLlm;
        let profile = make_profile(Role::Civilian);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("hello"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            NpcAction::Emote {
                kind: EmoteKind::Wave
            }
        )));
    }

    #[test]
    fn test_mock_llm_civilian_other() {
        let llm = MockLlm;
        let profile = make_profile(Role::Civilian);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("what's the news?"))
            .unwrap();

        // Should say they're busy
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
    }

    #[test]
    fn test_mock_llm_civilian_no_utterance() {
        let llm = MockLlm;
        let profile = make_profile(Role::Civilian);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, None)
            .unwrap();

        // No response when not spoken to
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn test_mock_llm_quest_giver_quest() {
        let llm = MockLlm;
        let profile = make_profile(Role::QuestGiver);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("do you have a quest for me?"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::GiveQuest { .. })));
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
    }

    #[test]
    fn test_mock_llm_quest_giver_work() {
        let llm = MockLlm;
        let profile = make_profile(Role::QuestGiver);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("I'm looking for work"))
            .unwrap();

        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::GiveQuest { .. })));
    }

    #[test]
    fn test_mock_llm_quest_giver_other() {
        let llm = MockLlm;
        let profile = make_profile(Role::QuestGiver);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, Some("tell me about the world"))
            .unwrap();

        // Should give cryptic response about threads
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::Say { .. })));
        assert!(!plan
            .actions
            .iter()
            .any(|a| matches!(a, NpcAction::GiveQuest { .. })));
    }

    #[test]
    fn test_mock_llm_quest_giver_no_utterance() {
        let llm = MockLlm;
        let profile = make_profile(Role::QuestGiver);
        let view = make_view();

        let plan = llm
            .plan_dialogue_and_behaviour(&profile, &view, None)
            .unwrap();

        // No response when not spoken to
        assert!(plan.actions.is_empty());
    }
}

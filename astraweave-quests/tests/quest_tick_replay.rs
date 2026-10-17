//! `quest_ticks` digest. Quest stamps are sim ticks.

use std::path::PathBuf;

use astraweave_quests::{
    CActiveQuest, LlmQuest, LlmQuestStep, PersonalizationData, QuestBranching, QuestMetadata,
    QuestRewards, QuestState,
};
use sha2::{Digest, Sha256};

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-quests.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash-quests.json")
}

fn bare_quest() -> LlmQuest {
    LlmQuest {
        id: "ford".into(),
        title: "Carry the sacks".into(),
        description: "Across the ford".into(),
        steps: vec![
            LlmQuestStep {
                id: "a".into(),
                description: "Pick up the sacks".into(),
                completed: false,
                objectives: Vec::new(),
                branching_choices: Vec::new(),
                dynamic_content: None,
                validation_criteria: astraweave_quests::ValidationCriteria {
                    required_conditions: Vec::new(),
                    forbidden_conditions: Vec::new(),
                    validation_script: None,
                },
            },
            LlmQuestStep {
                id: "b".into(),
                description: "Cross the ford".into(),
                completed: false,
                objectives: Vec::new(),
                branching_choices: Vec::new(),
                dynamic_content: None,
                validation_criteria: astraweave_quests::ValidationCriteria {
                    required_conditions: Vec::new(),
                    forbidden_conditions: Vec::new(),
                    validation_script: None,
                },
            },
        ],
        metadata: QuestMetadata {
            category: "travel".into(),
            difficulty_level: 0.2,
            estimated_duration: 10,
            player_level_range: (1, 3),
            required_skills: Vec::new(),
            tags: Vec::new(),
            generated_reasoning: String::new(),
        },
        branching: QuestBranching {
            has_multiple_paths: false,
            branch_points: Vec::new(),
            convergence_points: Vec::new(),
        },
        rewards: QuestRewards {
            experience: 1,
            currency: 0,
            items: Vec::new(),
            reputation_changes: std::collections::HashMap::new(),
            unlock_content: Vec::new(),
        },
        generated_at: 0,
        personalization: PersonalizationData {
            player_id: "aravath".into(),
            player_preferences: Vec::new(),
            play_style: "steady".into(),
            previous_choices: Vec::new(),
            difficulty_preference: 0.2,
        },
    }
}

fn scenario_bytes() -> Vec<u8> {
    let mut quest = bare_quest();
    quest.generated_at = 30;
    let mut active = CActiveQuest::at_tick(quest, 30);
    assert!(active.advance_step(40));
    active.set_state(QuestState::Active, 50);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&active.start_time.to_le_bytes());
    bytes.extend_from_slice(&active.last_update.to_le_bytes());
    bytes.extend_from_slice(&active.get_duration().to_le_bytes());
    bytes.extend_from_slice(&(active.current_step_index as u64).to_le_bytes());
    bytes.extend_from_slice(&active.quest.generated_at.to_le_bytes());
    bytes
}

#[test]
fn quest_ticks_replay_is_stable_and_written() {
    let first = scenario_bytes();
    let second = scenario_bytes();
    assert_eq!(first, second);
    let digest = hex_encode(&Sha256::digest(&first));
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    let body = format!("[\n  {{ \"case\": \"quest_ticks\", \"sha256\": \"{digest}\" }}\n]\n");
    std::fs::write(&path, body).expect("write replay hash");
    println!("quest_ticks {digest}");
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#![forbid(unsafe_code)]
//! Weaving System - Emergent Behavior Layer + Anchor System
//!
//! The weaving system detects patterns in the world state and proposes
//! emergent intents that are adjudicated by budget and cooldown constraints.
//!
//! Flow: Perception (Patterns) → Reasoning (Intents) → Planning (Adjudication) → Action (Events)
//!
//! The anchor system provides loom nodes that players can repair to stabilize reality,
//! unlock abilities, and deploy tactical structures using Echo currency.

pub mod abilities;
pub mod adjudicator;
pub mod anchor;
pub mod aravath;
pub mod audio;
pub mod combat;
pub mod damage;
pub mod echo_currency;
pub mod enemy;
pub mod enemy_types;
pub mod intents;
pub mod level;
pub mod particles;
pub mod patterns;
pub mod quest;
pub mod quest_types;
pub mod sim_rng;
pub mod spawner;
pub mod starter_quests;
pub mod systems;
pub mod ui;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod mutation_tests;

pub use adjudicator::{WeaveAdjudicator, WeaveConfig};
pub use anchor::{AbilityType, Anchor, AnchorVfxState};
pub use aravath::{run_aravath_slice, AravathSlice};
pub use audio::{echo_pickup_audio_command, AnchorAudioState, AnchorAudioSystem, AudioCommand};
pub use combat::{CombatEvent, CombatSystem, Killer};
pub use damage::{apply_element_schools, ElementSchool};
pub use echo_currency::{EchoCurrency, Transaction, TransactionReason};
pub use enemy::{AttackTarget, Enemy, EnemyBehavior, EnemyState};
pub use intents::{IntentProposer, WeaveIntent};
pub use level::{Camera, LevelStats, Player, VeilweaverLevel};
pub use particles::{AnchorParticleEmitter, AnchorParticleSystem, Particle, ParticleType};
pub use patterns::{Pattern, PatternDetector, PatternStrength};
pub use quest::{ObjectiveType, Quest, QuestManager, QuestReward, QuestState};
pub use sim_rng::{
    mix_sim_seed, SimRng, SIM_SYSTEM_ANCHOR_PARTICLES, SIM_SYSTEM_ENEMY_PATROL, SIM_SYSTEM_SPAWNER,
};
pub use spawner::{EnemySpawner, SpawnPoint, SpawnRequest};
pub use starter_quests::{
    all_starter_quests, quest_clear_corruption, quest_restore_beacon, quest_stabilize_anchors,
};
pub use ui::{
    AbilityUnlockNotification, AnchorInspectionModal, EchoFeedbackFloat, EchoHud,
    NotificationState, QuestPanel, RepairProgressBar,
};

/// Component for entities that can detect patterns and propose intents
#[derive(Debug, Clone)]
pub struct CWeaveAgent {
    /// Patterns detected in the last scan
    pub patterns_detected: std::collections::BTreeMap<String, f32>,
    /// Last time patterns were scanned
    pub last_scan: f32,
    /// Scan interval (seconds)
    pub scan_interval: f32,
}

impl CWeaveAgent {
    pub fn new(scan_interval: f32) -> Self {
        Self {
            patterns_detected: std::collections::BTreeMap::new(),
            last_scan: 0.0,
            scan_interval,
        }
    }

    pub fn should_scan(&self, current_time: f32) -> bool {
        current_time - self.last_scan >= self.scan_interval
    }
}

/// Component for weave signals/triggers in the world
#[derive(Debug, Clone)]
pub struct CWeaveSignal {
    pub kind: String,
    pub strength: f32,
    pub seed: u64,
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// Event emitted when a weave intent is accepted
#[derive(Debug, Clone)]
pub struct WeaveIntentEvent {
    pub intent: WeaveIntent,
}

#![forbid(unsafe_code)]
//! Advanced memory management system for LLM-powered game entities
//!
//! This crate provides sophisticated memory management including:
//! - Hierarchical memory structures (sensory, working, episodic, semantic)
//! - Intelligent forgetting curves and consolidation
//! - Context-aware memory retrieval
//! - Memory compression and summarization
//! - Cross-agent memory sharing
//! - Episode-based interaction recording for companion learning

mod utc_stamp;

pub mod memory_types;
pub use memory_types::*;

pub mod memory_manager;
pub use memory_manager::*;

// Episode recording system (note: different from persona::Episode)
pub mod episode;
pub use episode::{
    ActionResult, CompanionResponse, Episode as GameEpisode, EpisodeCategory, EpisodeOutcome,
    Observation, PlayerAction,
};

pub mod episode_recorder;
pub use episode_recorder::EpisodeRecorder;

// SQLite persistence layer
pub mod storage;
pub use storage::{MemoryStorage, StorageStats};

// Behavioral pattern detection and learning
pub mod pattern_detection;
pub use pattern_detection::{ActionPattern, PatternDetector, PatternStrength, PlaystylePattern};

pub mod preference_profile;
pub use preference_profile::{CompanionActionPreference, PreferenceProfile, ProfileBuilder};

// Adaptive behavior trees (Phase 4)
pub mod dynamic_weighting;
pub use dynamic_weighting::{AdaptiveWeightManager, BehaviorNodeType, NodeWeight};

pub mod learned_behavior_validator;
pub use learned_behavior_validator::{
    BehaviorValidator, SafetyRule, ValidationResult, ValidationStats,
};

pub mod consolidation;
pub use consolidation::*;

pub mod forgetting;
pub use forgetting::*;

pub mod compression;
pub use compression::*;

pub mod retrieval;
pub use retrieval::*;

pub mod sharing;
pub use sharing::*;

pub mod persona;
pub use persona::*;

#[cfg(feature = "bevy")]
pub mod components;
#[cfg(feature = "bevy")]
pub use components::*;

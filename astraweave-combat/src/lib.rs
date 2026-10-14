#![forbid(unsafe_code)]
//! Deterministic damage dealt and damage received.
//!
//! The resolver is Q32.32 integer arithmetic. Random draws are four `u32`s
//! from a [`SimRng`] seeded by `(world_seed, tick, ordinal, attack_id)`.
//! Modifiers sort by id. Schools are data. The shipped default table is
//! physical, weave, and three mundane hazards. It does not name shock, and
//! it does not use the words Echo or Veil.
//!
//! # Seams
//!
//! [`fixed`] copies veil `Fixed64` until that type lives in one crate.
//! [`sim_rng`] copies `astraweave_core::sim::SimRng` from PR #211 until that
//! API is on `main`. Neither branch is merged here.

mod ecs;
mod fixed;
mod resolve;
mod save;
mod sim_rng;
mod tables;

pub use ecs::{
    apply_hp_delta, combat_resolve_system, CBlock, CDefense, CImmune, CModifiers, COffense,
    CThreadStrain, CVitals, CombatClock, CombatLog, CombatPlugin, CombatScratch, CombatWriteGate,
    HitEvent, HitQueue, LoadedTables,
};
pub use fixed::Fixed64;
pub use resolve::{
    effective_health, expected_hit, hash_steps, lerp_bp, resolve_heal, resolve_hit, AttackerView,
    BlockGate, DefenderView, Estimate, HealOutcome, HitFlags, HitOutcome, ImmuneGate, Modifier,
    ModifierKind, SchoolRating, SchoolResist, StageTrace, StatKnowledge, DRAW_COUNT,
};
pub use save::{
    check_table_hash, decode_vitals, encode_vitals, read_combat_meta, write_combat_meta, MetaSink,
    ThreadStrainSave, VitalsRecord, META_TABLES, META_VITALS,
};
pub use sim_rng::{hit_rng, mix_sim_seed, SimRng, SIM_SYSTEM_COMBAT};
pub use tables::{
    apply_mods, canonical_tables, install, load_default, load_ron, AbilityDef, CombatTables,
    CurveDef, Defaults, HealDef, PeriodicRating, SchoolDef, SchoolPresentation, DEFAULT_TABLE_RON,
    MAX_MODIFIERS,
};

/// `DrawCount` is the public name of the four-draw contract.
pub type DrawCount = [u32; DRAW_COUNT];

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CombatError {
    #[error("k out of range")]
    KOutOfRange,
    #[error("engine ceiling: {0}")]
    Ceiling(&'static str),
    #[error("duplicate id")]
    DuplicateId,
    #[error("unknown school")]
    UnknownSchool,
    #[error("unknown tier")]
    UnknownTier,
    #[error("unknown ability")]
    UnknownAbility,
    #[error("bleed must be periodic and physical")]
    BadBleed,
    #[error("too many modifiers")]
    TooManyModifiers,
    #[error("duplicate modifier")]
    DuplicateModifier,
    #[error("modifier above ceiling")]
    ModifierCeiling,
    #[error("hidden stat")]
    HiddenStat,
    #[error("tables missing")]
    TablesMissing,
    #[error("combat table hash mismatch")]
    HashMismatch,
    #[error("stat out of range")]
    StatOutOfRange,
    #[error("bad curve")]
    BadCurve,
    #[error("bad schema")]
    BadSchema,
    #[error("school rename")]
    SchoolRename,
    #[error("forbidden school name")]
    ForbiddenName,
    #[error("bad coefficient")]
    BadCoeff,
    #[error("table text: {0}")]
    Ron(String),
    #[error("second writer")]
    DoubleWrite,
    #[error("bad save")]
    BadSave,
}

#[cfg(test)]
mod logic_tests;
#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod tests {
    fn scan(src: &str) {
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(!code.contains("f32"));
        assert!(!code.contains("f64"));
        assert!(!code.contains("HashMap"));
        assert!(!code.contains("SystemTime"));
        assert!(!code.contains("Instant"));
        assert!(!code.contains("unit_f32"));
    }

    #[test]
    fn sim_modules_have_no_float_or_hashmap() {
        scan(include_str!("fixed.rs"));
        scan(include_str!("sim_rng.rs"));
        scan(include_str!("resolve.rs"));
        scan(include_str!("tables.rs"));
    }
}

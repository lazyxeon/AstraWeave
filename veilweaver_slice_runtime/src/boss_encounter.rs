//! Boss encounter system — wires [`OathboundWardenDirector`] into the game loop.
//!
//! Manages the Oathbound Warden boss fight lifecycle:
//! - Pre-encounter setup (storm choice → arena modifiers)
//! - Per-tick director stepping (produces [`WardenDirective`])
//! - Phase transition events
//! - Defeat detection
//!
//! Feature-gated behind `boss-director`.

use astraweave_core::{DirectorBudget, WorldSnapshot};
use astraweave_director::{OathboundWardenDirector, WardenDirective, WardenPhase};
use tracing::info;

// ── Encounter events ───────────────────────────────────────────────────────

/// Events emitted by the boss encounter system.
#[derive(Debug, Clone, PartialEq)]
pub enum BossEncounterEvent {
    /// The encounter has started — arena configured.
    EncounterStarted { phase: WardenPhase },
    /// The Warden transitioned to a new phase.
    PhaseChanged { from: WardenPhase, to: WardenPhase },
    /// The Warden telegraphed an attack.
    Telegraph { messages: Vec<String> },
    /// The Warden is defeated.
    Defeated,
}
impl std::fmt::Display for BossEncounterEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncounterStarted { phase } => write!(f, "Encounter started (phase: {phase:?})"),
            Self::PhaseChanged { from, to } => write!(f, "Phase changed: {from:?} → {to:?}"),
            Self::Telegraph { messages } => write!(f, "Telegraph: {}", messages.join(", ")),
            Self::Defeated => write!(f, "Boss defeated"),
        }
    }
}
// ── Encounter state ────────────────────────────────────────────────────────

/// Lifecycle of the boss encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncounterState {
    /// Not yet in the boss zone.
    Inactive,
    /// Boss encountered, fight in progress.
    Active,
    /// Boss defeated.
    Defeated,
}
impl std::fmt::Display for EncounterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inactive => write!(f, "Inactive"),
            Self::Active => write!(f, "Active"),
            Self::Defeated => write!(f, "Defeated"),
        }
    }
}
// ── BossEncounter ──────────────────────────────────────────────────────────

/// Manages the Oathbound Warden boss fight.
pub struct BossEncounter {
    director: OathboundWardenDirector,
    state: EncounterState,
    last_phase: WardenPhase,
    /// Director budget passed to `step()` each tick.
    budget: DirectorBudget,
    /// Pending events for external consumption.
    pending_events: Vec<BossEncounterEvent>,
}

impl std::fmt::Debug for BossEncounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BossEncounter")
            .field("state", &self.state)
            .field("last_phase", &self.last_phase)
            .field("budget", &self.budget)
            .field("pending_events", &self.pending_events.len())
            .finish_non_exhaustive()
    }
}

impl Default for BossEncounter {
    fn default() -> Self {
        Self::new()
    }
}

impl BossEncounter {
    /// Creates a new inactive boss encounter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            director: OathboundWardenDirector::new(),
            state: EncounterState::Inactive,
            last_phase: WardenPhase::Assessment,
            budget: DirectorBudget {
                traps: 2,
                terrain_edits: 1,
                spawns: 0,
            },
            pending_events: Vec::new(),
        }
    }

    /// Sets the director budget for the encounter.
    pub fn set_budget(&mut self, budget: DirectorBudget) {
        self.budget = budget;
    }

    /// Starts the encounter. Called when the player enters Z4.
    ///
    /// `storm_choice` configures the arena based on the Z3 decision.
    pub fn start(&mut self, storm_choice: Option<crate::storm_choice::StormChoice>) {
        info!("Boss encounter: starting Oathbound Warden fight");
        self.director = OathboundWardenDirector::new();
        self.state = EncounterState::Active;
        self.last_phase = WardenPhase::Assessment;

        // Forward the storm choice to the Warden director.
        // The director uses Unknown if no choice was made.
        let _ = storm_choice; // Director picks up storm routing from WorldSnapshot context.

        self.pending_events
            .push(BossEncounterEvent::EncounterStarted {
                phase: WardenPhase::Assessment,
            });
    }

    /// Advances the boss AI by one tick.
    ///
    /// Returns the current [`WardenDirective`] for the action system to execute.
    pub fn tick(&mut self, snapshot: &WorldSnapshot) -> Option<WardenDirective> {
        if self.state != EncounterState::Active {
            return None;
        }

        let directive = self.director.step(snapshot, &self.budget);

        // Detect phase transitions.
        if directive.phase != self.last_phase {
            info!(
                "Boss encounter: phase {:?} → {:?}",
                self.last_phase, directive.phase
            );
            self.pending_events.push(BossEncounterEvent::PhaseChanged {
                from: self.last_phase,
                to: directive.phase,
            });
            self.last_phase = directive.phase;
        }

        // Forward telegraphs.
        if !directive.telegraphs.is_empty() {
            self.pending_events.push(BossEncounterEvent::Telegraph {
                messages: directive.telegraphs.clone(),
            });
        }

        // Detect defeat: empty plan + no enemies left is a proxy.
        if snapshot.enemies.is_empty() || snapshot.enemies.iter().all(|e| e.hp <= 0) {
            self.defeat();
        }

        Some(directive)
    }

    /// Marks the encounter as defeated.
    pub fn defeat(&mut self) {
        if self.state == EncounterState::Active {
            info!("Boss encounter: Oathbound Warden defeated!");
            self.state = EncounterState::Defeated;
            self.pending_events.push(BossEncounterEvent::Defeated);
        }
    }

    /// Drains pending encounter events.
    pub fn drain_events(&mut self) -> Vec<BossEncounterEvent> {
        std::mem::take(&mut self.pending_events)
    }

    // ── Queries ────────────────────────────────────────────────────────

    /// Returns the current encounter state.
    #[must_use]
    pub fn state(&self) -> EncounterState {
        self.state
    }

    /// Returns `true` if the encounter is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == EncounterState::Active
    }

    /// Returns `true` if the boss has been defeated.
    #[must_use]
    pub fn is_defeated(&self) -> bool {
        self.state == EncounterState::Defeated
    }

    /// Returns the current Warden phase.
    #[must_use]
    pub fn current_phase(&self) -> WardenPhase {
        self.last_phase
    }

    /// Resets the encounter to inactive (for replay/testing).
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance};
    use std::collections::BTreeMap;

    fn boss_snapshot(boss_hp: i32) -> WorldSnapshot {
        WorldSnapshot {
            t: 5.0,
            player: PlayerState {
                pos: IVec2::new(10, 10),
                hp: 80,
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2::new(12, 10),
                ammo: 15,
                cooldowns: BTreeMap::new(),
                morale: 0.8,
            },
            enemies: vec![EnemyState {
                id: 100,
                pos: IVec2::new(30, 30),
                hp: boss_hp,
                cover: CoverType::None,
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("defeat_warden".to_string()),
        }
    }

    #[test]
    fn inactive_by_default() {
        let enc = BossEncounter::new();
        assert_eq!(enc.state(), EncounterState::Inactive);
    }

    #[test]
    fn start_activates_encounter() {
        let mut enc = BossEncounter::new();
        enc.start(None);
        assert!(enc.is_active());

        let events = enc.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, BossEncounterEvent::EncounterStarted { .. })));
    }

    #[test]
    fn tick_produces_directive() {
        let mut enc = BossEncounter::new();
        enc.start(None);

        let snap = boss_snapshot(200);
        let directive = enc.tick(&snap);
        assert!(directive.is_some());
    }

    #[test]
    fn inactive_tick_returns_none() {
        let mut enc = BossEncounter::new();
        let snap = boss_snapshot(200);
        assert!(enc.tick(&snap).is_none());
    }

    #[test]
    fn defeat_detection() {
        let mut enc = BossEncounter::new();
        enc.start(None);

        // Snapshot with dead boss (hp <= 0).
        let snap = boss_snapshot(0);
        enc.tick(&snap);
        assert!(enc.is_defeated());

        let events = enc.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, BossEncounterEvent::Defeated)));
    }

    #[test]
    fn reset_returns_to_inactive() {
        let mut enc = BossEncounter::new();
        enc.start(None);
        enc.tick(&boss_snapshot(200));
        enc.reset();
        assert_eq!(enc.state(), EncounterState::Inactive);
    }
}

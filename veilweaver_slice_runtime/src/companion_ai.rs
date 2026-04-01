//! Companion AI integration — wires [`VeilweaverCompanionOrchestrator`] into the game loop.
//!
//! Provides tick-level companion AI that proposes `PlanIntent` each frame based on a
//! [`WorldSnapshot`]. The plan is forwarded to the action execution system.
//!
//! Feature-gated behind `ai-companion`.

use astraweave_ai::orchestrator::Orchestrator;
use astraweave_ai::VeilweaverCompanionOrchestrator;
use astraweave_core::{PlanIntent, WorldSnapshot};
use tracing::info;

/// Companion AI state within the game loop.
///
/// Wraps the GOAP-based orchestrator and tracks plan execution state.
pub struct CompanionAI {
    orchestrator: VeilweaverCompanionOrchestrator,
    /// Latest plan proposed by the orchestrator.
    current_plan: Option<PlanIntent>,
    /// How many ticks since the last re-plan.
    ticks_since_plan: u32,
    /// Re-plan interval in ticks (plan only every N ticks to avoid churn).
    replan_interval: u32,
}

impl std::fmt::Debug for CompanionAI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompanionAI")
            .field("current_plan", &self.current_plan)
            .field("ticks_since_plan", &self.ticks_since_plan)
            .field("replan_interval", &self.replan_interval)
            .finish_non_exhaustive()
    }
}

impl Default for CompanionAI {
    fn default() -> Self {
        Self::new()
    }
}

impl CompanionAI {
    /// Creates a new companion AI with the Veilweaver orchestrator.
    #[must_use]
    pub fn new() -> Self {
        info!("CompanionAI: VeilweaverCompanionOrchestrator initialized");
        Self {
            orchestrator: VeilweaverCompanionOrchestrator::new(),
            current_plan: None,
            ticks_since_plan: 0,
            replan_interval: 6, // Re-plan every 6 ticks (~100ms at 60Hz)
        }
    }

    /// Sets the re-plan interval (in ticks).
    pub fn set_replan_interval(&mut self, ticks: u32) {
        self.replan_interval = ticks.max(1);
    }

    /// Ticks the companion AI. If it's time to re-plan, proposes a new plan.
    ///
    /// Returns the current plan (may be unchanged from last tick).
    pub fn tick(&mut self, snapshot: &WorldSnapshot) -> Option<&PlanIntent> {
        self.ticks_since_plan += 1;

        if self.ticks_since_plan >= self.replan_interval || self.current_plan.is_none() {
            let plan = self.orchestrator.propose_plan(snapshot);
            if !plan.steps.is_empty() {
                info!(
                    "CompanionAI: proposed plan '{}' with {} steps",
                    plan.plan_id,
                    plan.steps.len()
                );
            }
            self.current_plan = Some(plan);
            self.ticks_since_plan = 0;
        }

        self.current_plan.as_ref()
    }

    /// Forces an immediate re-plan on the next tick.
    pub fn force_replan(&mut self) {
        self.ticks_since_plan = self.replan_interval;
    }

    /// Returns the current plan, if any.
    #[must_use]
    pub fn current_plan(&self) -> Option<&PlanIntent> {
        self.current_plan.as_ref()
    }

    /// Clears the current plan (e.g. when entering a cinematic).
    pub fn clear_plan(&mut self) {
        self.current_plan = None;
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance};
    use std::collections::BTreeMap;

    fn test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
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
                id: 1,
                pos: IVec2::new(20, 15),
                hp: 50,
                cover: CoverType::None,
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("defeat_warden".to_string()),
        }
    }

    #[test]
    fn companion_produces_plan() {
        let mut ai = CompanionAI::new();
        let snap = test_snapshot();
        let plan = ai.tick(&snap);
        assert!(plan.is_some());
    }

    #[test]
    fn replan_interval_respected() {
        let mut ai = CompanionAI::new();
        ai.set_replan_interval(3);
        let snap = test_snapshot();

        // First tick always produces a plan.
        ai.tick(&snap);

        // Next 2 ticks reuse the same plan.
        let plan1 = ai.tick(&snap).cloned();
        let plan2 = ai.tick(&snap).cloned();
        assert_eq!(
            plan1.as_ref().map(|p| &p.plan_id),
            plan2.as_ref().map(|p| &p.plan_id)
        );
    }

    #[test]
    fn force_replan() {
        let mut ai = CompanionAI::new();
        ai.set_replan_interval(100); // Very long interval.
        let snap = test_snapshot();

        ai.tick(&snap); // First plan.
        ai.force_replan();
        let plan = ai.tick(&snap);
        assert!(plan.is_some());
    }

    #[test]
    fn clear_plan() {
        let mut ai = CompanionAI::new();
        let snap = test_snapshot();
        ai.tick(&snap);
        assert!(ai.current_plan().is_some());

        ai.clear_plan();
        assert!(ai.current_plan().is_none());
    }
}

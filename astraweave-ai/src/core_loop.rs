//! Core loop wiring for AI planning dispatch.
//!
//! This module provides:
//! - `PlannerMode`: Rule, BehaviorTree, GOAP enum
//! - `CAiController`: Component to attach to entities for AI planning
//! - `dispatch_planner()`: Dispatcher that routes WorldSnapshot to appropriate planner

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use anyhow::Result;
use astraweave_core::{PlanIntent, WorldSnapshot};

use crate::htn::GatherGoal;

#[cfg(feature = "ai-bt")]
use astraweave_core::ActionStep;

#[cfg(feature = "ai-bt")]
use astraweave_behavior::{BehaviorContext, BehaviorGraph, BehaviorNode, BehaviorStatus};

use crate::orchestrator::{Orchestrator, RuleOrchestrator};

/// Planner mode selection for AI entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlannerMode {
    /// Use rule-based orchestrator (smoke + advance logic).
    Rule,
    /// Use behavior tree planner (requires `ai-bt` feature).
    BehaviorTree,
    /// Use GOAP planner (requires `ai-goap` feature).
    GOAP,
    /// Hierarchical planner with utility-scored methods.
    Htn,
}

impl std::fmt::Display for PlannerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlannerMode::Rule => write!(f, "Rule"),
            PlannerMode::BehaviorTree => write!(f, "BehaviorTree"),
            PlannerMode::GOAP => write!(f, "GOAP"),
            PlannerMode::Htn => write!(f, "Htn"),
        }
    }
}

impl PlannerMode {
    /// Check if this mode requires the `ai-bt` feature.
    #[must_use]
    pub fn requires_bt_feature(&self) -> bool {
        matches!(self, PlannerMode::BehaviorTree)
    }

    /// Check if this mode requires the `ai-goap` feature.
    #[must_use]
    pub fn requires_goap_feature(&self) -> bool {
        matches!(self, PlannerMode::GOAP)
    }

    /// Check if this mode is always available (no feature flag required).
    #[must_use]
    pub fn is_always_available(&self) -> bool {
        matches!(self, PlannerMode::Rule | PlannerMode::Htn)
    }

    /// Get the feature flag name required for this mode, if any.
    #[must_use]
    pub fn required_feature(&self) -> Option<&'static str> {
        match self {
            PlannerMode::Rule | PlannerMode::Htn => None,
            PlannerMode::BehaviorTree => Some("ai-bt"),
            PlannerMode::GOAP => Some("ai-goap"),
        }
    }

    /// Get all available planner modes.
    #[must_use]
    pub fn all() -> &'static [PlannerMode] {
        &[
            PlannerMode::Rule,
            PlannerMode::BehaviorTree,
            PlannerMode::GOAP,
            PlannerMode::Htn,
        ]
    }
}

/// Component for AI entities to control which planner to use.
///
/// # Example
/// ```
/// use astraweave_ai::core_loop::{CAiController, PlannerMode};
///
/// let controller = CAiController {
///     mode: PlannerMode::GOAP,
///     policy: Some("gather_craft_policy".to_string()),
///     gather: GatherGoal::KeepWood,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CAiController {
    /// Which planner to use for this entity.
    pub mode: PlannerMode,
    /// Optional behavior-tree policy name. GOAP dispatch does not read it.
    pub policy: Option<String>,
    /// Explicit gather method. Ignored by rule and behavior-tree modes.
    pub gather: GatherGoal,
}

impl std::fmt::Display for CAiController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.policy {
            Some(policy) => write!(f, "CAiController({}, policy={})", self.mode, policy),
            None => write!(f, "CAiController({})", self.mode),
        }
    }
}

impl Default for CAiController {
    fn default() -> Self {
        Self {
            mode: PlannerMode::Rule,
            policy: None,
            gather: GatherGoal::CookFood,
        }
    }
}

impl CAiController {
    /// Create a new AI controller with the specified planner mode.
    #[must_use]
    pub fn new(mode: PlannerMode) -> Self {
        Self {
            mode,
            policy: None,
            gather: GatherGoal::CookFood,
        }
    }

    /// Create a new AI controller with a policy.
    #[must_use]
    pub fn with_policy(mode: PlannerMode, policy: impl Into<String>) -> Self {
        Self {
            mode,
            policy: Some(policy.into()),
            gather: GatherGoal::CookFood,
        }
    }

    /// Create a rule-based AI controller (default).
    #[must_use]
    pub fn rule() -> Self {
        Self::new(PlannerMode::Rule)
    }

    /// Create a behavior tree AI controller.
    #[must_use]
    pub fn behavior_tree() -> Self {
        Self::new(PlannerMode::BehaviorTree)
    }

    /// Create a GOAP AI controller.
    #[must_use]
    pub fn goap() -> Self {
        Self::new(PlannerMode::GOAP)
    }

    /// Set the gather method. This does not inspect `policy`.
    #[must_use]
    pub fn with_gather(mut self, gather: GatherGoal) -> Self {
        self.gather = gather;
        self
    }

    /// Check if this controller has a policy.
    #[must_use]
    pub fn has_policy(&self) -> bool {
        self.policy.is_some()
    }

    /// Get the policy name if set.
    #[must_use]
    pub fn policy_name(&self) -> Option<&str> {
        self.policy.as_deref()
    }

    /// Set the policy for this controller.
    pub fn set_policy(&mut self, policy: impl Into<String>) {
        self.policy = Some(policy.into());
    }

    /// Clear the policy for this controller.
    pub fn clear_policy(&mut self) {
        self.policy = None;
    }

    /// Check if this controller requires feature flags.
    #[must_use]
    pub fn requires_feature(&self) -> bool {
        self.mode.required_feature().is_some()
    }
}

/// Dispatch planning to the appropriate planner based on controller mode.
///
/// # Arguments
/// - `controller`: The AI controller specifying which planner to use
/// - `snapshot`: World state snapshot for the entity
///
/// # Returns
/// - `Ok(PlanIntent)` with the planned action sequence
/// - `Err` if planning fails
///
/// # Example
/// ```
/// use astraweave_ai::core_loop::{CAiController, PlannerMode, dispatch_planner};
/// use astraweave_core::{WorldSnapshot, PlayerState, CompanionState, EnemyState, IVec2, WeaveHazard, MAX_WEAVE_HAZARDS};
/// use std::collections::BTreeMap;
///
/// let controller = CAiController {
///     mode: PlannerMode::Rule,
///     policy: None,
///     gather: astraweave_ai::htn::GatherGoal::CookFood,
/// };
///
/// // Create snapshot with an enemy so the planner returns non-empty steps
/// let snapshot = WorldSnapshot {
///     t: 0.0,
///     me: CompanionState {
///         ammo: 10,
///         cooldowns: BTreeMap::new(),
///         morale: 1.0,
///         pos: IVec2 { x: 5, y: 5 },
///     },
///     player: PlayerState {
///         hp: 100,
///         pos: IVec2 { x: 5, y: 5 },
///         stance: "stand".into(),
///         orders: vec![],
///     },
///     enemies: vec![EnemyState {
///         id: 1,
///         pos: IVec2 { x: 10, y: 10 },
///         hp: 100,
///         cover: "none".into(),
///         last_seen: 0.0,
///     }],
///     pois: vec![],
///     obstacles: vec![],
///     objective: None,
///     hazards: [WeaveHazard::EMPTY; MAX_WEAVE_HAZARDS],
///     hazard_count: 0,
/// };
///
/// let plan = dispatch_planner(&controller, &snapshot).unwrap();
/// assert!(!plan.steps.is_empty());
/// ```
pub fn dispatch_planner(
    controller: &CAiController,
    snapshot: &WorldSnapshot,
) -> Result<PlanIntent> {
    #[cfg(feature = "profiling")]
    span!("AI::dispatch_planner");

    match controller.mode {
        PlannerMode::Rule => {
            let orch = RuleOrchestrator;
            Ok(orch.propose_plan(snapshot))
        }
        PlannerMode::BehaviorTree => {
            #[cfg(feature = "ai-bt")]
            {
                dispatch_bt(controller, snapshot)
            }
            #[cfg(not(feature = "ai-bt"))]
            {
                anyhow::bail!("BehaviorTree mode requires 'ai-bt' feature")
            }
        }
        PlannerMode::GOAP => {
            #[cfg(feature = "ai-goap")]
            {
                dispatch_goap(controller, snapshot)
            }
            #[cfg(not(feature = "ai-goap"))]
            {
                anyhow::bail!("GOAP mode requires 'ai-goap' feature")
            }
        }
        PlannerMode::Htn => Ok(crate::htn::HtnOrchestrator.propose_plan(snapshot)),
    }
}

/// Typed failure from behavior-tree dispatch.
///
/// Returned when `ai-bt` is enabled and the controller does not name a graph
/// this dispatcher can tick. The default policy `"default"` ticks the built-in
/// companion tree inside `dispatch_planner`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorDispatchError {
    message: String,
}

impl BehaviorDispatchError {
    #[cfg(any(test, feature = "ai-bt"))]
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BehaviorDispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BehaviorDispatchError {}

/// Dispatch to the behavior-tree planner.
///
/// Policy `"default"` ticks a selector: advance toward `enemies.first()` when
/// an enemy is present, otherwise hold. Any other policy is a typed error.
/// The graph walk is the behavior runtime; it does not call the model.
#[cfg(feature = "ai-bt")]
fn dispatch_bt(controller: &CAiController, snapshot: &WorldSnapshot) -> Result<PlanIntent> {
    match controller.policy.as_deref() {
        Some("default") => Ok(tick_default_companion_tree(snapshot)),
        Some(policy) => Err(BehaviorDispatchError::new(format!(
            "no behavior graph is bound for policy '{policy}'"
        ))
        .into()),
        None => {
            Err(BehaviorDispatchError::new("no behavior graph is bound to this controller").into())
        }
    }
}

#[cfg(feature = "ai-bt")]
fn tick_default_companion_tree(snapshot: &WorldSnapshot) -> PlanIntent {
    use std::sync::{Arc, Mutex};

    let steps = Arc::new(Mutex::new(Vec::<ActionStep>::new()));
    let enemy = snapshot.enemies.first().cloned();
    let me = snapshot.me.pos;
    let enemy_present = enemy.is_some();

    let mut context = BehaviorContext::new();
    context.register_condition("enemy_present", move || enemy_present);
    let advance_steps = Arc::clone(&steps);
    context.register_action("advance", move || {
        if let Some(enemy) = &enemy {
            let x = me.x.saturating_add((enemy.pos.x - me.x).signum());
            let y = me.y.saturating_add((enemy.pos.y - me.y).signum());
            if let Ok(mut recorded) = advance_steps.lock() {
                recorded.push(ActionStep::MoveTo { x, y, speed: None });
            }
        }
        BehaviorStatus::Success
    });
    context.register_action("hold", || BehaviorStatus::Success);

    let root = BehaviorNode::Selector(vec![
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition("enemy_present".into()),
            BehaviorNode::Action("advance".into()),
        ]),
        BehaviorNode::Action("hold".into()),
    ]);
    let _status = BehaviorGraph::new(root).tick(&context);
    let recorded = steps.lock().map(|guard| guard.clone()).unwrap_or_default();
    PlanIntent {
        plan_id: format!("bt-{}", (snapshot.t * 1000.0) as i64),
        steps: recorded,
    }
}

/// Gather methods live in the HTN module. The feature gate stays so existing
/// controllers still select this mode by name.
#[cfg(feature = "ai-goap")]
fn dispatch_goap(controller: &CAiController, _snapshot: &WorldSnapshot) -> Result<PlanIntent> {
    #[cfg(feature = "profiling")]
    span!("AI::dispatch_goap");
    Ok(crate::htn::gather_method(controller.gather))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState};
    use std::collections::BTreeMap;

    fn make_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,

                pos: IVec2 { x: 5, y: 5 },
            },
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    #[test]
    fn test_controller_default() {
        let controller = CAiController::default();
        assert_eq!(controller.mode, PlannerMode::Rule);
        assert_eq!(controller.policy, None);
    }

    #[test]
    fn test_dispatch_rule_mode() {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: GatherGoal::CookFood,
        };

        let snapshot = make_test_snapshot();
        let result = dispatch_planner(&controller, &snapshot);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn behavior_dispatch_error_names_the_missing_graph() {
        let err = BehaviorDispatchError::new("no behavior graph is bound to this controller");
        assert!(err.to_string().contains("no behavior graph"));
    }

    #[cfg(feature = "ai-bt")]
    #[test]
    fn dispatch_bt_default_policy_emits_move() {
        let controller = CAiController {
            mode: PlannerMode::BehaviorTree,
            policy: Some("default".to_string()),
            gather: GatherGoal::CookFood,
        };
        let plan = dispatch_planner(&controller, &make_test_snapshot()).expect("default tree");
        assert!(
            plan.steps
                .iter()
                .any(|step| matches!(step, astraweave_core::ActionStep::MoveTo { .. })),
            "default behavior tree should advance toward the enemy"
        );
    }

    #[cfg(feature = "ai-bt")]
    #[test]
    fn dispatch_bt_unknown_policy_is_typed_error() {
        let controller = CAiController {
            mode: PlannerMode::BehaviorTree,
            policy: Some("missing-graph".to_string()),
            gather: GatherGoal::CookFood,
        };
        let err = dispatch_planner(&controller, &make_test_snapshot()).expect_err("unbound");
        assert!(err.downcast_ref::<BehaviorDispatchError>().is_some());
    }

    #[test]
    fn test_dispatch_bt_mode_without_feature() {
        #[cfg(not(feature = "ai-bt"))]
        {
            let controller = CAiController {
                mode: PlannerMode::BehaviorTree,
                policy: None,
                gather: GatherGoal::CookFood,
            };

            let snapshot = make_test_snapshot();
            let result = dispatch_planner(&controller, &snapshot);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("ai-bt' feature"));
        }
    }

    #[test]
    #[cfg(feature = "ai-goap")]
    fn test_dispatch_goap_mode() {
        let controller = CAiController {
            mode: PlannerMode::GOAP,
            policy: Some("gather_craft_policy".to_string()),
            gather: GatherGoal::KeepWood,
        };

        let mut snapshot = make_test_snapshot();
        // Make companion "hungry" by using low ammo as proxy since we can't set HP
        snapshot.me.ammo = 0;

        let result = dispatch_planner(&controller, &snapshot);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.steps.len(), 6, "gather keeps wood after cooking");
        assert!(plan.plan_id.contains("goap"));
    }

    #[test]
    #[cfg(feature = "ai-goap")]
    fn cook_only_method_is_four_steps() {
        let controller = CAiController {
            mode: PlannerMode::GOAP,
            policy: None,
            gather: GatherGoal::CookFood,
        };
        let plan = dispatch_planner(&controller, &make_test_snapshot()).unwrap();
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.plan_id, "goap_4");
    }

    #[test]
    #[cfg(feature = "ai-goap")]
    fn test_create_goap_actions() {
        let plan = crate::htn::gather_method(GatherGoal::KeepWood);
        assert_eq!(plan.steps.len(), 6);
        assert_eq!(plan.plan_id, "goap_6");
    }

    // ===== NEW TESTS (5 tests added) =====

    #[test]
    fn test_controller_with_custom_policy() {
        // Test CAiController with custom policy
        let controller = CAiController {
            mode: PlannerMode::GOAP,
            policy: Some("custom_combat_policy".to_string()),
            gather: GatherGoal::CookFood,
        };

        assert_eq!(controller.mode, PlannerMode::GOAP);
        assert_eq!(controller.policy, Some("custom_combat_policy".to_string()));
    }

    #[test]
    fn test_controller_clone() {
        // Test that CAiController can be cloned
        let controller1 = CAiController {
            mode: PlannerMode::BehaviorTree,
            policy: Some("patrol_policy".to_string()),
            gather: GatherGoal::CookFood,
        };

        let controller2 = controller1.clone();

        assert_eq!(controller2.mode, PlannerMode::BehaviorTree);
        assert_eq!(controller2.policy, Some("patrol_policy".to_string()));
    }

    #[test]
    fn test_planner_mode_equality() {
        // Test PlannerMode equality comparisons
        assert_eq!(PlannerMode::Rule, PlannerMode::Rule);
        assert_eq!(PlannerMode::BehaviorTree, PlannerMode::BehaviorTree);
        assert_eq!(PlannerMode::GOAP, PlannerMode::GOAP);

        assert_ne!(PlannerMode::Rule, PlannerMode::GOAP);
        assert_ne!(PlannerMode::BehaviorTree, PlannerMode::Rule);
    }

    #[test]
    fn test_dispatch_rule_mode_no_enemies() {
        // Test Rule orchestrator with no enemies (edge case)
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: GatherGoal::CookFood,
        };

        let snapshot = WorldSnapshot {
            t: 0.0,
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,

                pos: IVec2 { x: 5, y: 5 },
            },
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            enemies: vec![], // No enemies
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };

        let result = dispatch_planner(&controller, &snapshot);
        assert!(result.is_ok());
        let plan = result.unwrap();
        // RuleOrchestrator returns empty plan when no enemies (fallback behavior)
        assert!(plan.steps.is_empty());
        assert!(plan.plan_id.starts_with("plan-")); // Should still have valid plan_id
    }

    #[test]
    #[cfg(not(feature = "ai-goap"))]
    fn test_dispatch_goap_mode_without_feature() {
        // Test that GOAP mode fails gracefully when feature is disabled
        let controller = CAiController {
            mode: PlannerMode::GOAP,
            policy: None,
            gather: GatherGoal::CookFood,
        };

        let snapshot = make_test_snapshot();
        let result = dispatch_planner(&controller, &snapshot);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ai-goap' feature"));
    }

    // ===== Tests for new PlannerMode enhancements =====

    #[test]
    fn test_planner_mode_display() {
        assert_eq!(format!("{}", PlannerMode::Rule), "Rule");
        assert_eq!(format!("{}", PlannerMode::BehaviorTree), "BehaviorTree");
        assert_eq!(format!("{}", PlannerMode::GOAP), "GOAP");
    }

    #[test]
    fn test_planner_mode_requires_bt_feature() {
        assert!(!PlannerMode::Rule.requires_bt_feature());
        assert!(PlannerMode::BehaviorTree.requires_bt_feature());
        assert!(!PlannerMode::GOAP.requires_bt_feature());
    }

    #[test]
    fn test_planner_mode_requires_goap_feature() {
        assert!(!PlannerMode::Rule.requires_goap_feature());
        assert!(!PlannerMode::BehaviorTree.requires_goap_feature());
        assert!(PlannerMode::GOAP.requires_goap_feature());
    }

    #[test]
    fn test_planner_mode_is_always_available() {
        assert!(PlannerMode::Rule.is_always_available());
        assert!(PlannerMode::Htn.is_always_available());
        assert!(!PlannerMode::BehaviorTree.is_always_available());
        assert!(!PlannerMode::GOAP.is_always_available());
    }

    #[test]
    fn test_planner_mode_required_feature() {
        assert_eq!(PlannerMode::Rule.required_feature(), None);
        assert_eq!(PlannerMode::BehaviorTree.required_feature(), Some("ai-bt"));
        assert_eq!(PlannerMode::GOAP.required_feature(), Some("ai-goap"));
    }

    #[test]
    fn test_planner_mode_all() {
        let all = PlannerMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&PlannerMode::Rule));
        assert!(all.contains(&PlannerMode::BehaviorTree));
        assert!(all.contains(&PlannerMode::GOAP));
        assert!(all.contains(&PlannerMode::Htn));
    }

    #[test]
    fn test_planner_mode_copy() {
        let mode = PlannerMode::GOAP;
        let copy = mode;
        assert_eq!(mode, copy);
    }

    #[test]
    fn test_planner_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PlannerMode::Rule);
        set.insert(PlannerMode::BehaviorTree);
        set.insert(PlannerMode::GOAP);
        set.insert(PlannerMode::Htn);
        assert_eq!(set.len(), 4);
    }

    // ===== Tests for new CAiController enhancements =====

    #[test]
    fn test_controller_display_without_policy() {
        let controller = CAiController::new(PlannerMode::Rule);
        assert_eq!(format!("{}", controller), "CAiController(Rule)");
    }

    #[test]
    fn test_controller_display_with_policy() {
        let controller = CAiController::with_policy(PlannerMode::GOAP, "combat_policy");
        assert_eq!(
            format!("{}", controller),
            "CAiController(GOAP, policy=combat_policy)"
        );
    }

    #[test]
    fn test_controller_new() {
        let controller = CAiController::new(PlannerMode::BehaviorTree);
        assert_eq!(controller.mode, PlannerMode::BehaviorTree);
        assert!(controller.policy.is_none());
    }

    #[test]
    fn test_controller_with_policy() {
        let controller = CAiController::with_policy(PlannerMode::GOAP, "gather_policy");
        assert_eq!(controller.mode, PlannerMode::GOAP);
        assert_eq!(controller.policy, Some("gather_policy".to_string()));
    }

    #[test]
    fn test_controller_rule_factory() {
        let controller = CAiController::rule();
        assert_eq!(controller.mode, PlannerMode::Rule);
        assert!(controller.policy.is_none());
    }

    #[test]
    fn test_controller_behavior_tree_factory() {
        let controller = CAiController::behavior_tree();
        assert_eq!(controller.mode, PlannerMode::BehaviorTree);
        assert!(controller.policy.is_none());
    }

    #[test]
    fn test_controller_goap_factory() {
        let controller = CAiController::goap();
        assert_eq!(controller.mode, PlannerMode::GOAP);
        assert!(controller.policy.is_none());
    }

    #[test]
    fn test_controller_has_policy() {
        let controller1 = CAiController::new(PlannerMode::Rule);
        let controller2 = CAiController::with_policy(PlannerMode::Rule, "policy");
        assert!(!controller1.has_policy());
        assert!(controller2.has_policy());
    }

    #[test]
    fn test_controller_policy_name() {
        let controller1 = CAiController::new(PlannerMode::Rule);
        let controller2 = CAiController::with_policy(PlannerMode::Rule, "test_policy");
        assert_eq!(controller1.policy_name(), None);
        assert_eq!(controller2.policy_name(), Some("test_policy"));
    }

    #[test]
    fn test_controller_set_policy() {
        let mut controller = CAiController::new(PlannerMode::Rule);
        assert!(!controller.has_policy());
        controller.set_policy("new_policy");
        assert!(controller.has_policy());
        assert_eq!(controller.policy_name(), Some("new_policy"));
    }

    #[test]
    fn test_controller_clear_policy() {
        let mut controller = CAiController::with_policy(PlannerMode::Rule, "policy");
        assert!(controller.has_policy());
        controller.clear_policy();
        assert!(!controller.has_policy());
    }

    #[test]
    fn test_controller_requires_feature() {
        let rule = CAiController::rule();
        let bt = CAiController::behavior_tree();
        let goap = CAiController::goap();
        assert!(!rule.requires_feature());
        assert!(bt.requires_feature());
        assert!(goap.requires_feature());
    }

    #[test]
    fn test_controller_equality() {
        let c1 = CAiController::with_policy(PlannerMode::GOAP, "policy");
        let c2 = CAiController::with_policy(PlannerMode::GOAP, "policy");
        let c3 = CAiController::with_policy(PlannerMode::GOAP, "other");
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }
}

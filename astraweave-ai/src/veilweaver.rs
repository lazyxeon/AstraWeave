#![allow(clippy::too_many_arguments)]

use astraweave_behavior::goap::{GoapAction, GoapGoal, WorldState};
use astraweave_core::{ActionStep, MovementSpeed, PlanIntent, WorldSnapshot};
use astraweave_observability::{log_companion_action, CompanionActionEvent};
use tracing::info;

use crate::{Orchestrator, OrchestratorAsync};

const ACTION_STABILITY_PULSE: &str = "stability_pulse";
const ACTION_HEAL_PLAYER: &str = "heal_player";
const ACTION_EXECUTE_COMBO: &str = "execute_combo";
const ACTION_MARK_TARGET: &str = "mark_target";
const ACTION_REPOSITION: &str = "reposition";

/// Hybrid GOAP planner tailored to Veilweaver companion behaviors.
pub struct VeilweaverCompanionOrchestrator {
    actions: Vec<GoapAction>,
    goals: Vec<GoapGoal>,
}

impl VeilweaverCompanionOrchestrator {
    pub fn new() -> Self {
        Self {
            actions: build_actions(),
            goals: build_goals(),
        }
    }

    fn plan_internal(&self, snap: &WorldSnapshot) -> PlanIntent {
        debug_assert_eq!(self.actions.len(), 5);
        debug_assert_eq!(self.goals.len(), 4);
        let heuristics = HeuristicState::from_snapshot(snap);
        let action = select_slice_action(&heuristics);
        let plan_id = format!("vw-companion-{}", (snap.t * 1000.0) as i64);
        let steps = map_action_to_steps(action, &heuristics);
        info!(
            target: "veilweaver.companion",
            event = "CompanionAction",
            %plan_id,
            action,
            step_count = steps.len()
        );
        log_companion_action(&CompanionActionEvent {
            action_id: action.to_string(),
            success: true,
            latency_ms: 0.0,
        });
        if steps.is_empty() {
            return PlanIntent {
                plan_id,
                steps: vec![ActionStep::Wait { duration: 0.5 }],
            };
        }
        PlanIntent { plan_id, steps }
    }
}

/// Method order is priority, then lower method id. No A* search and no wall clock.
fn select_slice_action(h: &HeuristicState<'_>) -> &'static str {
    if h.player_low {
        return ACTION_HEAL_PLAYER;
    }
    if h.anchor_unstable && h.has_echo {
        return ACTION_STABILITY_PULSE;
    }
    if h.enemy_vulnerable {
        return ACTION_EXECUTE_COMBO;
    }
    if !h.position_good {
        return ACTION_REPOSITION;
    }
    if h.closest_enemy.is_some() && !h.enemy_tagged {
        return ACTION_MARK_TARGET;
    }
    ""
}

impl Default for VeilweaverCompanionOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for VeilweaverCompanionOrchestrator {
    async fn plan(&self, snap: WorldSnapshot, _budget_ms: u32) -> anyhow::Result<PlanIntent> {
        Ok(self.plan_internal(&snap))
    }
}

impl Orchestrator for VeilweaverCompanionOrchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent {
        self.plan_internal(snap)
    }
}

fn build_actions() -> Vec<GoapAction> {
    vec![
        GoapAction::new(ACTION_STABILITY_PULSE)
            .with_precondition("anchor_unstable", true)
            .with_precondition("has_echo", true)
            .with_effect("anchor_unstable", false)
            .with_effect("has_echo", false)
            .with_cost(1.0),
        GoapAction::new(ACTION_HEAL_PLAYER)
            .with_precondition("player_low", true)
            .with_effect("player_low", false)
            .with_cost(1.2),
        GoapAction::new(ACTION_EXECUTE_COMBO)
            .with_precondition("enemy_vulnerable", true)
            .with_effect("enemy_vulnerable", false)
            .with_cost(1.1),
        GoapAction::new(ACTION_MARK_TARGET)
            .with_precondition("enemy_tagged", false)
            .with_effect("enemy_tagged", true)
            .with_cost(0.8),
        GoapAction::new(ACTION_REPOSITION)
            .with_precondition("position_good", false)
            .with_effect("position_good", true)
            .with_cost(0.6),
    ]
}

fn build_goals() -> Vec<GoapGoal> {
    vec![
        GoapGoal::new(
            "protect_player",
            WorldState::from_facts(&[("player_low", false)]),
        )
        .with_priority(3.0),
        GoapGoal::new(
            "stabilize_threads",
            WorldState::from_facts(&[("anchor_unstable", false)]),
        )
        .with_priority(2.8),
        GoapGoal::new(
            "exploit_stagger",
            WorldState::from_facts(&[("enemy_vulnerable", false)]),
        )
        .with_priority(2.2),
        GoapGoal::new(
            "maintain_positioning",
            WorldState::from_facts(&[("position_good", true)]),
        )
        .with_priority(1.5),
    ]
}

fn map_action_to_steps(action: &str, heuristics: &HeuristicState<'_>) -> Vec<ActionStep> {
    match action {
        ACTION_STABILITY_PULSE => vec![ActionStep::UseDefensiveAbility {
            ability_name: "stability_pulse".into(),
        }],
        ACTION_HEAL_PLAYER => vec![ActionStep::Heal { target_id: None }],
        ACTION_EXECUTE_COMBO => heuristics
            .closest_enemy
            .map(|enemy| {
                vec![
                    ActionStep::CoordinateAttack {
                        target_id: enemy.id,
                    },
                    ActionStep::QuickAttack {
                        target_id: enemy.id,
                    },
                ]
            })
            .unwrap_or_else(|| vec![ActionStep::Wait { duration: 0.3 }]),
        ACTION_MARK_TARGET => heuristics
            .closest_enemy
            .map(|enemy| {
                vec![ActionStep::MarkTarget {
                    target_id: enemy.id,
                }]
            })
            .unwrap_or_default(),
        ACTION_REPOSITION => {
            let target = heuristics.player_pos_average();
            vec![ActionStep::MoveTo {
                x: target.x,
                y: target.y,
                speed: Some(MovementSpeed::Run),
            }]
        }
        _ => vec![ActionStep::Wait { duration: 0.2 }],
    }
}

#[derive(Clone)]
struct HeuristicState<'a> {
    player_low: bool,
    anchor_unstable: bool,
    enemy_vulnerable: bool,
    position_good: bool,
    has_echo: bool,
    enemy_tagged: bool,
    closest_enemy: Option<&'a astraweave_core::EnemyState>,
    player_pos: astraweave_core::IVec2,
    companion_pos: astraweave_core::IVec2,
}

impl<'a> HeuristicState<'a> {
    fn from_snapshot(snap: &'a WorldSnapshot) -> Self {
        let player_low = snap.player.hp <= 40;
        let anchor_unstable = snap
            .objective
            .as_ref()
            .map(|o| o.contains("anchor_unstable") || o.contains("stabilize_anchor"))
            .unwrap_or(false);

        let has_echo = snap.me.ammo > 0;

        let closest_enemy = snap.enemies.iter().min_by_key(|enemy| {
            let dx = enemy.pos.x - snap.me.pos.x;
            let dy = enemy.pos.y - snap.me.pos.y;
            dx.abs() + dy.abs()
        });

        let enemy_vulnerable = closest_enemy.map(|enemy| enemy.hp <= 30).unwrap_or(false);

        let dx = snap.player.pos.x - snap.me.pos.x;
        let dy = snap.player.pos.y - snap.me.pos.y;
        let position_good = (dx.abs() + dy.abs()) <= 6;

        Self {
            player_low,
            anchor_unstable,
            enemy_vulnerable,
            position_good,
            has_echo,
            enemy_tagged: false,
            closest_enemy,
            player_pos: snap.player.pos,
            companion_pos: snap.me.pos,
        }
    }

    fn player_pos_average(&self) -> astraweave_core::IVec2 {
        astraweave_core::IVec2 {
            x: (self.player_pos.x + self.companion_pos.x) / 2,
            y: (self.player_pos.y + self.companion_pos.y) / 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
    use std::collections::BTreeMap;

    fn make_entity(id: u32) -> astraweave_core::Entity {
        // Entity is a type alias for u32
        id
    }

    fn make_enemy(id: u32, x: i32, y: i32, hp: i32) -> EnemyState {
        EnemyState {
            id: make_entity(id),
            pos: IVec2 { x, y },
            hp,
            cover: "none".into(),
            last_seen: 0.0,
        }
    }

    fn minimal_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "idle".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 5,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 1, y: 1 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    // ==========================================================================
    // VeilweaverCompanionOrchestrator Tests
    // ==========================================================================

    #[test]
    fn test_orchestrator_new() {
        let orch = VeilweaverCompanionOrchestrator::new();
        // Should have the expected number of actions and goals
        assert_eq!(orch.actions.len(), 5, "Should have 5 GOAP actions");
        assert_eq!(orch.goals.len(), 4, "Should have 4 GOAP goals");
    }

    #[test]
    fn test_orchestrator_default() {
        let orch = VeilweaverCompanionOrchestrator::default();
        assert_eq!(orch.actions.len(), 5);
        assert_eq!(orch.goals.len(), 4);
    }

    #[test]
    fn test_orchestrator_implements_orchestrator_trait() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let snap = minimal_snapshot();
        let plan = orch.propose_plan(&snap);
        // Should return a valid plan
        assert!(!plan.plan_id.is_empty());
    }

    #[test]
    fn test_plan_with_no_enemies_idle_waits() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let snap = minimal_snapshot();
        let plan = orch.propose_plan(&snap);
        // With no enemies and no objectives, should idle or reposition
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn test_plan_when_player_low_hp() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        snap.player.hp = 20; // Low HP triggers heal
        let plan = orch.propose_plan(&snap);
        // Should prioritize healing player
        let has_heal = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::Heal { .. }));
        assert!(has_heal, "Should heal when player HP is low");
    }

    #[test]
    fn test_plan_when_anchor_unstable() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        snap.objective = Some("anchor_unstable".into());
        snap.me.ammo = 1; // Need echo (ammo) for stability pulse
        let plan = orch.propose_plan(&snap);
        // Should use defensive ability for stability
        let has_defensive = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::UseDefensiveAbility { .. }));
        assert!(
            has_defensive,
            "Should use stability pulse when anchor is unstable"
        );
    }

    #[test]
    fn test_plan_when_enemy_vulnerable() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        // Add a vulnerable enemy (low HP)
        snap.enemies.push(make_enemy(1, 2, 2, 10)); // Low HP = vulnerable
        let plan = orch.propose_plan(&snap);
        // Should attack the vulnerable enemy
        let has_attack = plan.steps.iter().any(|s| {
            matches!(
                s,
                ActionStep::CoordinateAttack { .. } | ActionStep::QuickAttack { .. }
            )
        });
        assert!(has_attack, "Should attack vulnerable enemy");
    }

    #[test]
    fn test_plan_when_position_is_bad() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        // Position companion far from player
        snap.me.pos = IVec2 { x: 100, y: 100 };
        snap.player.pos = IVec2 { x: 0, y: 0 };
        let plan = orch.propose_plan(&snap);
        // Should reposition to be closer to player
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(has_move, "Should reposition when too far from player");
    }

    #[test]
    fn test_plan_id_contains_timestamp() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        snap.t = 1.234;
        let plan = orch.propose_plan(&snap);
        assert!(
            plan.plan_id.starts_with("vw-companion-"),
            "Plan ID should have correct prefix"
        );
    }

    // ==========================================================================
    // HeuristicState Tests
    // ==========================================================================

    #[test]
    fn test_heuristic_player_low_threshold() {
        let mut snap = minimal_snapshot();

        // HP = 40 should be considered low
        snap.player.hp = 40;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(heuristics.player_low, "HP 40 should be considered low");

        // HP = 41 should not be considered low
        snap.player.hp = 41;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.player_low, "HP 41 should not be considered low");
    }

    #[test]
    fn test_heuristic_anchor_unstable_detection() {
        let mut snap = minimal_snapshot();

        snap.objective = Some("anchor_unstable".into());
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(heuristics.anchor_unstable);

        snap.objective = Some("stabilize_anchor".into());
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(heuristics.anchor_unstable);

        snap.objective = Some("collect_item".into());
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.anchor_unstable);

        snap.objective = None;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.anchor_unstable);
    }

    #[test]
    fn test_heuristic_has_echo() {
        let mut snap = minimal_snapshot();

        snap.me.ammo = 5;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(heuristics.has_echo);

        snap.me.ammo = 0;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.has_echo);
    }

    #[test]
    fn test_heuristic_closest_enemy() {
        let mut snap = minimal_snapshot();
        snap.me.pos = IVec2 { x: 0, y: 0 };

        // Add enemies at different distances
        snap.enemies.push(make_enemy(1, 10, 10, 50));
        snap.enemies.push(make_enemy(2, 2, 2, 50));
        snap.enemies.push(make_enemy(3, 5, 5, 50));

        let heuristics = HeuristicState::from_snapshot(&snap);
        let closest = heuristics.closest_enemy.unwrap();
        assert_eq!(closest.pos.x, 2, "Closest enemy should be at (2,2)");
        assert_eq!(closest.pos.y, 2);
    }

    #[test]
    fn test_heuristic_enemy_vulnerable() {
        let mut snap = minimal_snapshot();

        // No enemies - not vulnerable
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.enemy_vulnerable);

        // Enemy with HP > 30 - not vulnerable
        snap.enemies.push(make_enemy(1, 1, 1, 50));
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(!heuristics.enemy_vulnerable);

        // Enemy with HP <= 30 - vulnerable
        snap.enemies[0].hp = 30;
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(heuristics.enemy_vulnerable);
    }

    #[test]
    fn test_heuristic_position_good_threshold() {
        let mut snap = minimal_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };

        // Distance <= 6 is good
        snap.me.pos = IVec2 { x: 3, y: 3 }; // Manhattan distance = 6
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(
            heuristics.position_good,
            "Distance 6 should be good position"
        );

        // Distance > 6 is bad
        snap.me.pos = IVec2 { x: 4, y: 4 }; // Manhattan distance = 8
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(
            !heuristics.position_good,
            "Distance 8 should be bad position"
        );
    }

    #[test]
    fn test_heuristic_player_pos_average() {
        let mut snap = minimal_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.me.pos = IVec2 { x: 10, y: 20 };

        let heuristics = HeuristicState::from_snapshot(&snap);
        let avg = heuristics.player_pos_average();

        assert_eq!(avg.x, 5, "Average X should be 5");
        assert_eq!(avg.y, 10, "Average Y should be 10");
    }

    // ==========================================================================
    // build_actions Tests
    // ==========================================================================

    #[test]
    fn test_build_actions_count() {
        let actions = build_actions();
        assert_eq!(actions.len(), 5);
    }

    #[test]
    fn test_build_actions_names() {
        let actions = build_actions();
        let names: Vec<&str> = actions.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&ACTION_STABILITY_PULSE));
        assert!(names.contains(&ACTION_HEAL_PLAYER));
        assert!(names.contains(&ACTION_EXECUTE_COMBO));
        assert!(names.contains(&ACTION_MARK_TARGET));
        assert!(names.contains(&ACTION_REPOSITION));
    }

    // ==========================================================================
    // build_goals Tests
    // ==========================================================================

    #[test]
    fn test_build_goals_count() {
        let goals = build_goals();
        assert_eq!(goals.len(), 4);
    }

    #[test]
    fn test_build_goals_priorities() {
        let goals = build_goals();
        // Find protect_player goal - should have highest priority
        let protect = goals.iter().find(|g| g.name == "protect_player").unwrap();
        assert_eq!(protect.priority, 3.0);

        // Find maintain_positioning - should have lowest priority
        let position = goals
            .iter()
            .find(|g| g.name == "maintain_positioning")
            .unwrap();
        assert_eq!(position.priority, 1.5);
    }

    // ==========================================================================
    // map_action_to_steps Tests
    // ==========================================================================

    #[test]
    fn test_map_stability_pulse() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_STABILITY_PULSE, &heuristics);

        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ActionStep::UseDefensiveAbility { .. }));
    }

    #[test]
    fn test_map_heal_player() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_HEAL_PLAYER, &heuristics);

        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ActionStep::Heal { target_id: None }));
    }

    #[test]
    fn test_map_execute_combo_with_enemy() {
        let mut snap = minimal_snapshot();
        snap.enemies.push(make_enemy(42, 2, 2, 50));
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_EXECUTE_COMBO, &heuristics);

        assert_eq!(steps.len(), 2, "Combo should have 2 steps");
        assert!(matches!(steps[0], ActionStep::CoordinateAttack { .. }));
        assert!(matches!(steps[1], ActionStep::QuickAttack { .. }));
    }

    #[test]
    fn test_map_execute_combo_no_enemy() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_EXECUTE_COMBO, &heuristics);

        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ActionStep::Wait { duration } if duration == 0.3));
    }

    #[test]
    fn test_map_mark_target_with_enemy() {
        let mut snap = minimal_snapshot();
        snap.enemies.push(make_enemy(42, 2, 2, 50));
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_MARK_TARGET, &heuristics);

        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ActionStep::MarkTarget { .. }));
    }

    #[test]
    fn test_map_mark_target_no_enemy() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_MARK_TARGET, &heuristics);

        assert!(steps.is_empty(), "No target to mark when no enemies");
    }

    #[test]
    fn test_map_reposition() {
        let mut snap = minimal_snapshot();
        snap.player.pos = IVec2 { x: 10, y: 20 };
        snap.me.pos = IVec2 { x: 0, y: 0 };
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps(ACTION_REPOSITION, &heuristics);

        assert_eq!(steps.len(), 1);
        if let ActionStep::MoveTo { x, y, speed } = &steps[0] {
            assert_eq!(*x, 5, "Should move to average X position");
            assert_eq!(*y, 10, "Should move to average Y position");
            assert_eq!(*speed, Some(MovementSpeed::Run));
        } else {
            panic!("Expected MoveTo action step");
        }
    }

    #[test]
    fn test_map_unknown_action() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let steps = map_action_to_steps("unknown_action", &heuristics);

        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ActionStep::Wait { duration } if duration == 0.2));
    }

    // ==========================================================================
    // Integration Tests
    // ==========================================================================

    #[test]
    fn test_full_planning_cycle_priority_order() {
        let orch = VeilweaverCompanionOrchestrator::new();

        // Create scenario with multiple goals applicable
        let mut snap = minimal_snapshot();
        snap.player.hp = 20; // Low HP - protect_player (priority 3.0)
        snap.objective = Some("anchor_unstable".into()); // stabilize_threads (priority 2.8)
        snap.me.ammo = 1; // Has echo
        snap.me.pos = IVec2 { x: 100, y: 100 }; // Bad position (priority 1.5)

        let plan = orch.propose_plan(&snap);

        // Should prioritize healing (protect_player has highest priority)
        let first_step = &plan.steps[0];
        assert!(
            matches!(first_step, ActionStep::Heal { .. }),
            "Should prioritize healing over other goals"
        );
    }

    #[test]
    fn test_heuristic_clone() {
        let snap = minimal_snapshot();
        let heuristics = HeuristicState::from_snapshot(&snap);
        let cloned = heuristics.clone();

        assert_eq!(heuristics.player_low, cloned.player_low);
        assert_eq!(heuristics.has_echo, cloned.has_echo);
        assert_eq!(heuristics.position_good, cloned.position_good);
    }

    // ==========================================================================
    // Mutation-killing tests (veilweaver.rs)
    // ==========================================================================

    /// Kills line 122: `plan -> Result<PlanIntent>` replaced with `Ok(Default::default())`
    /// Exercises the async `OrchestratorAsync::plan` path which was previously untested.
    #[test]
    fn test_async_plan_returns_valid_result() {
        let orch = VeilweaverCompanionOrchestrator::new();
        let mut snap = minimal_snapshot();
        snap.player.hp = 20; // Low HP triggers heal goal

        let result = futures_executor::block_on(orch.plan(snap, 100));
        let plan = result.unwrap();
        // Default PlanIntent has empty steps; real plan should produce non-empty steps.
        assert!(
            !plan.steps.is_empty(),
            "Async plan should produce non-empty steps"
        );
    }

    /// Kills lines 248-249: `enemy.pos.x - snap.me.pos.x` replaced with `+`
    /// Uses non-origin companion position so |e-m| != |e+m|.
    #[test]
    fn test_closest_enemy_nonorigin_kills_sub_to_add_mutation() {
        let mut snap = minimal_snapshot();
        snap.me.pos = IVec2 { x: 5, y: 5 };

        // Enemy A at (4,5): |4-5|=1 (original), |4+5|=9 (mutated dx)
        // Enemy B at (1,5): |1-5|=4 (original), |1+5|=6 (mutated dx)
        // Original: A closest (1 < 4). Mutation(dx→+): B closest (6 < 9). FLIPS.
        snap.enemies.push(make_enemy(10, 4, 5, 50));
        snap.enemies.push(make_enemy(20, 1, 5, 50));

        let heuristics = HeuristicState::from_snapshot(&snap);
        let closest = heuristics.closest_enemy.unwrap();
        assert_eq!(
            closest.id, 10,
            "Enemy at (4,5) should be closest with non-origin companion"
        );

        // Now test dy axis with separate snapshot
        let mut snap2 = minimal_snapshot();
        snap2.me.pos = IVec2 { x: 0, y: 5 };
        snap2.enemies.push(make_enemy(30, 0, 4, 50)); // |4-5|=1 vs |4+5|=9
        snap2.enemies.push(make_enemy(40, 0, 1, 50)); // |1-5|=4 vs |1+5|=6
        let h2 = HeuristicState::from_snapshot(&snap2);
        assert_eq!(
            h2.closest_enemy.unwrap().id,
            30,
            "Enemy at (0,4) should be closest via dy"
        );
    }

    /// Kills line 250: `dx.abs() + dy.abs()` replaced with `dx.abs() * dy.abs()`
    /// Uses enemies where Manhattan ordering differs from product ordering.
    #[test]
    fn test_closest_enemy_manhattan_vs_product_mutation() {
        let mut snap = minimal_snapshot();
        snap.me.pos = IVec2 { x: 0, y: 0 };

        // A at (4,0): Manhattan=4, Product=0
        // B at (1,2): Manhattan=3, Product=2
        // Original(+): B closest (3 < 4). Mutation(*): A closest (0 < 2). FLIPS.
        snap.enemies.push(make_enemy(10, 4, 0, 50));
        snap.enemies.push(make_enemy(20, 1, 2, 50));

        let heuristics = HeuristicState::from_snapshot(&snap);
        let closest = heuristics.closest_enemy.unwrap();
        assert_eq!(
            closest.id, 20,
            "Enemy at (1,2) should be closest via Manhattan distance, not product"
        );
    }

    /// Kills lines 255-256: `snap.player.pos.x - snap.me.pos.x` replaced with `+`
    /// Uses non-origin positions where subtraction differs from addition.
    #[test]
    fn test_position_good_nonorigin_kills_sub_to_add_mutation() {
        let mut snap = minimal_snapshot();
        snap.player.pos = IVec2 { x: 5, y: 5 };
        snap.me.pos = IVec2 { x: 3, y: 3 };

        // Original: dx=2, dy=2, dist=4 <= 6 → position_good=true
        // Mutation(x→+): dx=8, dist=10 > 6 → position_good=false
        // Mutation(y→+): dy=8, dist=10 > 6 → position_good=false
        let heuristics = HeuristicState::from_snapshot(&snap);
        assert!(
            heuristics.position_good,
            "Position should be good when real distance is 4"
        );
    }
}

// GOAPOrchestrator - Integration with AstraWeave AI system
// Implements the Orchestrator trait to provide GOAP planning

use super::actions::register_all_actions;
use super::adapter::SnapshotAdapter;
use super::{AdvancedGOAP, Goal, StateValue, WorldState};
use astraweave_core::{ActionStep, MovementSpeed, PlanIntent, WorldSnapshot};
use std::collections::BTreeMap;

/// GOAP-powered orchestrator for AstraWeave
/// Converts WorldSnapshot to GOAP state and back to PlanIntent
pub struct GOAPOrchestrator {
    planner: AdvancedGOAP,
}

impl GOAPOrchestrator {
    pub fn new() -> Self {
        let mut planner = AdvancedGOAP::new();

        // Register comprehensive action library from Phase 2
        register_all_actions(&mut planner);

        Self { planner }
    }

    /// Convert WorldSnapshot to GOAP WorldState using enhanced adapter
    fn snapshot_to_state(snap: &WorldSnapshot) -> WorldState {
        SnapshotAdapter::to_world_state(snap)
    }

    /// Convert GOAP plan to engine PlanIntent (Phase 2 expanded mapping)
    fn plan_to_intent(plan: Vec<String>, snap: &WorldSnapshot, plan_id: String) -> PlanIntent {
        let mut steps = Vec::new();

        for action_name in plan {
            match action_name.as_str() {
                "move_to" | "approach_enemy" => {
                    if let Some(enemy) = snap.enemies.first() {
                        let target_x = snap.me.pos.x + (enemy.pos.x - snap.me.pos.x).signum() * 2;
                        let target_y = snap.me.pos.y + (enemy.pos.y - snap.me.pos.y).signum() * 2;
                        steps.push(ActionStep::MoveTo {
                            x: target_x,
                            y: target_y,
                            speed: None,
                        });
                    }
                }
                "attack" => {
                    if let Some(enemy) = snap.enemies.first() {
                        steps.push(ActionStep::Attack {
                            target_id: enemy.id,
                        });
                    }
                }
                "cover_fire" => {
                    if let Some(enemy) = snap.enemies.first() {
                        steps.push(ActionStep::CoverFire {
                            target_id: enemy.id,
                            duration: 2.0,
                        });
                    }
                }
                "reload" => {
                    steps.push(ActionStep::Reload);
                }
                "take_cover" => {
                    // Find nearest cover (simplified: move back from enemy)
                    if let Some(enemy) = snap.enemies.first() {
                        let retreat_x = snap.me.pos.x - (enemy.pos.x - snap.me.pos.x).signum() * 3;
                        let retreat_y = snap.me.pos.y - (enemy.pos.y - snap.me.pos.y).signum() * 3;
                        steps.push(ActionStep::MoveTo {
                            x: retreat_x,
                            y: retreat_y,
                            speed: None,
                        });
                    }
                }
                "heal" => {
                    steps.push(ActionStep::Heal { target_id: None });
                }
                "throw_smoke" => {
                    if let Some(enemy) = snap.enemies.first() {
                        let mid_x = (snap.me.pos.x + enemy.pos.x) / 2;
                        let mid_y = (snap.me.pos.y + enemy.pos.y) / 2;
                        steps.push(ActionStep::Throw {
                            item: "smoke".to_string(),
                            x: mid_x,
                            y: mid_y,
                        });
                    }
                }
                "retreat" => {
                    // Move away from enemies
                    if let Some(enemy) = snap.enemies.first() {
                        let retreat_x = snap.me.pos.x - (enemy.pos.x - snap.me.pos.x).signum() * 5;
                        let retreat_y = snap.me.pos.y - (enemy.pos.y - snap.me.pos.y).signum() * 5;
                        steps.push(ActionStep::MoveTo {
                            x: retreat_x,
                            y: retreat_y,
                            speed: Some(MovementSpeed::Sprint),
                        });
                    }
                }
                "revive" => {
                    // Revive player (assuming player is entity 0)
                    steps.push(ActionStep::Revive { ally_id: 0 });
                }
                "scan" => {
                    steps.push(ActionStep::Scan { radius: 10.0 });
                }
                _ => {
                    tracing::warn!("Unknown GOAP action: {}", action_name);
                }
            }
        }

        PlanIntent { plan_id, steps }
    }

    /// Generate a plan using GOAP
    pub fn propose_plan(&mut self, snap: &WorldSnapshot) -> PlanIntent {
        let plan_id = format!("goap-plan-{}", (snap.t * 1000.0) as i64);

        // Convert snapshot to GOAP state
        let state = Self::snapshot_to_state(snap);

        // Define goal based on situation
        let goal = if snap.enemies.is_empty() {
            // No enemies: exploration/idle goal
            let mut goal_state = BTreeMap::new();
            goal_state.insert("moved_closer".to_string(), StateValue::Bool(true));
            Goal::new("explore", goal_state).with_priority(1.0)
        } else {
            // Enemies present: combat goal
            let mut goal_state = BTreeMap::new();
            goal_state.insert("enemy_damaged".to_string(), StateValue::Bool(true));
            Goal::new("engage_enemy", goal_state).with_priority(5.0)
        };

        // Try to find a plan
        match self.planner.plan(&state, &goal) {
            Some(plan) => {
                tracing::debug!("GOAP plan: {:?}", plan);
                Self::plan_to_intent(plan, snap, plan_id)
            }
            None => {
                tracing::warn!("GOAP failed to find plan, returning empty intent");
                PlanIntent {
                    plan_id,
                    steps: vec![],
                }
            }
        }
    }

    /// Get reference to the planner (for testing/inspection)
    pub fn planner(&self) -> &AdvancedGOAP {
        &self.planner
    }

    /// Get mutable reference to the planner
    pub fn planner_mut(&mut self) -> &mut AdvancedGOAP {
        &mut self.planner
    }
}

impl Default for GOAPOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CoverType, IVec2, Stance};
    use std::collections::BTreeMap;

    fn make_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: astraweave_core::PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: astraweave_core::CompanionState {
                ammo: 20,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![astraweave_core::EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 50,
                cover: CoverType::None,
                last_seen: 1.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    #[test]
    fn test_snapshot_to_state() {
        let snap = make_test_snapshot();
        let state = GOAPOrchestrator::snapshot_to_state(&snap);

        assert_eq!(state.get("my_ammo"), Some(&StateValue::Int(20)));
        assert_eq!(state.get("enemy_count"), Some(&StateValue::Int(1)));
        assert_eq!(state.get("in_combat"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_goap_orchestrator_propose_plan() {
        let mut orch = GOAPOrchestrator::new();
        let snap = make_test_snapshot();

        let intent = orch.propose_plan(&snap);

        assert!(!intent.plan_id.is_empty());
        assert!(intent.plan_id.starts_with("goap-plan-"));
        // Plan should have at least one step
        // (exact steps depend on planner logic)
    }

    #[test]
    fn test_empty_enemy_list() {
        let mut orch = GOAPOrchestrator::new();
        let mut snap = make_test_snapshot();
        snap.enemies.clear();

        let intent = orch.propose_plan(&snap);

        // Should still produce a valid intent (even if empty or exploration)
        assert!(!intent.plan_id.is_empty());
    }

    // ── mutation-killing tests for plan_to_intent coordinate arithmetic ──

    #[test]
    fn test_plan_to_intent_move_to_coordinates() {
        // me=(10,10), enemy=(20,20) → signum=1, target = 10 + 1*2 = 12
        let snap = make_test_snapshot_at(10, 10, 20, 20);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["move_to".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(*x, 12, "move_to x = me.x + signum(dx)*2");
                assert_eq!(*y, 12, "move_to y = me.y + signum(dy)*2");
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_approach_enemy() {
        // approach_enemy should behave identically to move_to
        let snap = make_test_snapshot_at(5, 5, 15, 3);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["approach_enemy".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                // signum(15-5)=1, signum(3-5)=-1
                assert_eq!(*x, 5 + 1 * 2); // 7
                assert_eq!(*y, 5 + (-1) * 2); // 3
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_attack() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["attack".into()], &snap, "t".into());
        assert_eq!(intent.steps.len(), 1);
        match &intent.steps[0] {
            ActionStep::Attack { target_id } => assert_eq!(*target_id, 1),
            other => panic!("expected Attack, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_cover_fire() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["cover_fire".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(*target_id, 1);
                assert_eq!(*duration, 2.0);
            }
            other => panic!("expected CoverFire, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_reload() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["reload".into()], &snap, "t".into());
        assert_eq!(intent.steps.len(), 1);
        assert!(matches!(&intent.steps[0], ActionStep::Reload));
    }

    #[test]
    fn test_plan_to_intent_take_cover_coordinates() {
        // me=(10,10), enemy=(20,20) → signum=1, retreat = 10 - 1*3 = 7
        let snap = make_test_snapshot_at(10, 10, 20, 20);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["take_cover".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(*x, 7, "take_cover x = me.x - signum(dx)*3");
                assert_eq!(*y, 7, "take_cover y = me.y - signum(dy)*3");
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_heal() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["heal".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::Heal { target_id } => assert_eq!(*target_id, None),
            other => panic!("expected Heal, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_throw_smoke_coordinates() {
        // me=(10,10), enemy=(20,20) → mid = (10+20)/2 = 15
        let snap = make_test_snapshot_at(10, 10, 20, 20);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["throw_smoke".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::Throw { item, x, y } => {
                assert_eq!(item, "smoke");
                assert_eq!(*x, 15, "smoke x = (me.x + enemy.x) / 2");
                assert_eq!(*y, 15, "smoke y = (me.y + enemy.y) / 2");
            }
            other => panic!("expected Throw, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_retreat_coordinates() {
        // me=(10,10), enemy=(20,20) → retreat = 10 - 1*5 = 5
        let snap = make_test_snapshot_at(10, 10, 20, 20);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["retreat".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, speed } => {
                assert_eq!(*x, 5, "retreat x = me.x - signum(dx)*5");
                assert_eq!(*y, 5, "retreat y = me.y - signum(dy)*5");
                assert_eq!(*speed, Some(MovementSpeed::Sprint));
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_revive() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["revive".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::Revive { ally_id } => assert_eq!(*ally_id, 0),
            other => panic!("expected Revive, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_scan() {
        let snap = make_test_snapshot();
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["scan".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::Scan { radius } => assert_eq!(*radius, 10.0),
            other => panic!("expected Scan, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_multiple_actions() {
        let snap = make_test_snapshot();
        let intent = GOAPOrchestrator::plan_to_intent(
            vec!["reload".into(), "attack".into(), "heal".into()],
            &snap,
            "multi".into(),
        );
        assert_eq!(intent.steps.len(), 3);
        assert!(matches!(&intent.steps[0], ActionStep::Reload));
        assert!(matches!(&intent.steps[1], ActionStep::Attack { .. }));
        assert!(matches!(&intent.steps[2], ActionStep::Heal { .. }));
    }

    #[test]
    fn test_planner_accessors() {
        let mut orch = GOAPOrchestrator::new();
        // planner() should return the internal planner WITH registered actions
        let state = GOAPOrchestrator::snapshot_to_state(&make_test_snapshot());
        let mut desired = BTreeMap::new();
        desired.insert("enemy_damaged".to_string(), StateValue::Bool(true));
        let goal = Goal::new("engage", desired);

        // Real planner (with actions) should find a plan
        let plan = orch.planner().plan(&state, &goal);
        assert!(
            plan.is_some(),
            "planner() should return the real planner with registered actions"
        );

        // planner_mut() should also return the real planner
        let plan_mut = orch.planner_mut().plan(&state, &goal);
        assert!(
            plan_mut.is_some(),
            "planner_mut() should return the real planner"
        );
    }

    /// Test with enemy BEHIND me (negative direction) to kill
    /// inner subtraction mutations: (enemy.x - me.x) → (enemy.x + me.x)
    #[test]
    fn test_plan_to_intent_move_to_negative_direction() {
        // me=(10,10), enemy=(3,3) → signum(3-10)=signum(-7)=-1
        let snap = make_test_snapshot_at(10, 10, 3, 3);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["move_to".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                // target = 10 + (-1)*2 = 8
                assert_eq!(*x, 8, "move_to x: me + signum(enemy-me) * 2");
                assert_eq!(*y, 8, "move_to y");
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_take_cover_negative_direction() {
        // me=(10,10), enemy=(3,3) → signum(-7)=-1, retreat = 10 - (-1)*3 = 13
        let snap = make_test_snapshot_at(10, 10, 3, 3);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["take_cover".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(*x, 13);
                assert_eq!(*y, 13);
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_to_intent_retreat_negative_direction() {
        // me=(10,10), enemy=(3,3) → retreat = 10 - (-1)*5 = 15
        let snap = make_test_snapshot_at(10, 10, 3, 3);
        let intent =
            GOAPOrchestrator::plan_to_intent(vec!["retreat".into()], &snap, "t".into());
        match &intent.steps[0] {
            ActionStep::MoveTo { x, y, speed } => {
                assert_eq!(*x, 15);
                assert_eq!(*y, 15);
                assert_eq!(*speed, Some(MovementSpeed::Sprint));
            }
            other => panic!("expected MoveTo, got {:?}", other),
        }
    }

    /// Helper: snapshot with specific me/enemy positions
    fn make_test_snapshot_at(me_x: i32, me_y: i32, e_x: i32, e_y: i32) -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: astraweave_core::PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: astraweave_core::CompanionState {
                ammo: 20,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: me_x, y: me_y },
            },
            enemies: vec![astraweave_core::EnemyState {
                id: 1,
                pos: IVec2 { x: e_x, y: e_y },
                hp: 50,
                cover: CoverType::None,
                last_seen: 1.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
}

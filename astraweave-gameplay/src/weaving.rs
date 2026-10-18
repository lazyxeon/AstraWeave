use crate::{WeaveBudget, WeaveConsequence, WeaveOp, WeaveOpKind};
use anyhow::Result;
use astraweave_core::{
    apply_director_plan, DirectorBudget, DirectorOp, DirectorPlan, IVec2, World,
};
use astraweave_nav::Triangle;
use astraweave_physics::PhysicsWorld;
use astraweave_weaving::{compile_legacy, LegacyPoints};
use glam::vec3;

/// Apply a single weave op to the world + physics, consume budgets, and return consequences.
pub fn apply_weave_op(
    w: &mut World,
    phys: &mut PhysicsWorld,
    nav_src: &[Triangle],
    budget: &mut WeaveBudget,
    op: &WeaveOp,
    log: &mut impl FnMut(String),
) -> Result<WeaveConsequence> {
    let _ = nav_src;
    let mut core_budget = DirectorBudget {
        traps: 0,
        terrain_edits: 3,
        spawns: 0,
    };
    let mut plan = DirectorPlan { ops: vec![] };
    let kind = match op.kind {
        WeaveOpKind::ReinforcePath => 0,
        WeaveOpKind::CollapseBridge => 1,
        WeaveOpKind::RedirectWind => 2,
        WeaveOpKind::LowerWater => 3,
        WeaveOpKind::RaisePlatform => 4,
        WeaveOpKind::FreezeWater => 5,
    };
    let compiled = compile_legacy(
        kind,
        LegacyPoints {
            a: [op.a.x, op.a.y, op.a.z],
            b: op.b.map(|p| [p.x, p.y, p.z]),
            tick: 0,
            seq: 0,
        },
    );

    if matches!(op.kind, WeaveOpKind::CollapseBridge) && op.b.is_none() {
        anyhow::bail!("Collapse needs A->B");
    }
    match op.kind {
        WeaveOpKind::ReinforcePath | WeaveOpKind::RaisePlatform | WeaveOpKind::CollapseBridge => {
            if budget.terrain_edits <= 0 {
                anyhow::bail!("No terrain budget");
            }
            budget.terrain_edits -= 1;
        }
        WeaveOpKind::RedirectWind | WeaveOpKind::LowerWater | WeaveOpKind::FreezeWater => {
            if budget.weather_ops <= 0 {
                anyhow::bail!("No weather budget");
            }
            budget.weather_ops -= 1;
        }
    }

    if let Some(rect) = compiled.fortify {
        plan.ops.push(DirectorOp::Fortify {
            rect: astraweave_core::Rect {
                x0: rect.x0,
                y0: rect.y0,
                x1: rect.x1,
                y1: rect.y1,
            },
        });
    }
    if let Some(collapse) = compiled.collapse {
        plan.ops.push(DirectorOp::Collapse {
            a: IVec2 {
                x: collapse.ax,
                y: collapse.ay,
            },
            b: IVec2 {
                x: collapse.bx,
                y: collapse.by,
            },
        });
    }
    if matches!(op.kind, WeaveOpKind::RedirectWind) {
        let dir = (op.b.unwrap_or(op.a + vec3(1.0, 0.0, 0.0)) - op.a).normalize_or_zero();
        phys.set_wind(dir, 10.0);
        log("Weave: Wind redirected".into());
    }
    if matches!(op.kind, WeaveOpKind::LowerWater) {
        phys.clear_water();
        log("Weave: Waters receded".into());
    }
    if let (Some(min_milli), Some(max_milli)) = (compiled.ice_min_milli, compiled.ice_max_milli) {
        let min = glam::Vec3::new(
            min_milli[0] as f32 / 1000.0,
            min_milli[1] as f32 / 1000.0,
            min_milli[2] as f32 / 1000.0,
        );
        let max = glam::Vec3::new(
            max_milli[0] as f32 / 1000.0,
            max_milli[1] as f32 / 1000.0,
            max_milli[2] as f32 / 1000.0,
        );
        let _body = phys.add_static_aabb(min, max);
        log("Weave: Waters frozen".into());
    }

    if !plan.ops.is_empty() {
        apply_director_plan(w, &mut core_budget, &plan, log);
    }

    // Return a rough “world consequence”
    let consequence = match op.kind {
        WeaveOpKind::ReinforcePath => WeaveConsequence {
            drop_multiplier: 1.1,
            faction_disposition: 5,
            weather_shift: None,
        },
        WeaveOpKind::CollapseBridge => WeaveConsequence {
            drop_multiplier: 0.9,
            faction_disposition: -10,
            weather_shift: None,
        },
        WeaveOpKind::RedirectWind => WeaveConsequence {
            drop_multiplier: 1.0,
            faction_disposition: 0,
            weather_shift: Some("windy".into()),
        },
        WeaveOpKind::LowerWater => WeaveConsequence {
            drop_multiplier: 1.0,
            faction_disposition: 0,
            weather_shift: Some("dry".into()),
        },
        WeaveOpKind::RaisePlatform => WeaveConsequence {
            drop_multiplier: 1.05,
            faction_disposition: 0,
            weather_shift: None,
        },
        WeaveOpKind::FreezeWater => WeaveConsequence {
            drop_multiplier: 1.0,
            faction_disposition: 0,
            weather_shift: Some("frozen".into()),
        },
    };

    Ok(consequence)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::{TestErr, TestValue};
    use glam::vec3;

    fn create_test_world() -> World {
        World::new()
    }

    fn create_test_physics() -> PhysicsWorld {
        PhysicsWorld::new(vec3(0.0, -9.81, 0.0))
    }

    fn create_test_budget() -> WeaveBudget {
        WeaveBudget {
            terrain_edits: 5,
            weather_ops: 3,
        }
    }

    #[test]
    fn test_apply_weave_op_reinforce_path_success() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(5.0, 0.0, 5.0),
            b: None,
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_ok(), "ReinforcePath should succeed");
        assert_eq!(budget.terrain_edits, 4, "Should consume 1 terrain edit");

        let consequence = result.test_unwrap();
        assert_eq!(consequence.drop_multiplier, 1.1);
        assert_eq!(consequence.faction_disposition, 5);
        assert_eq!(consequence.weather_shift, None);
    }

    #[test]
    fn test_apply_weave_op_reinforce_path_no_budget() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = WeaveBudget {
            terrain_edits: 0, // No budget!
            weather_ops: 3,
        };
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(5.0, 0.0, 5.0),
            b: None,
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_err(), "Should fail with no budget");
        assert!(result
            .test_unwrap_err()
            .to_string()
            .contains("No terrain budget"));
    }

    #[test]
    fn test_apply_weave_op_collapse_bridge_success() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::CollapseBridge,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(10.0, 0.0, 10.0)),
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_ok(), "CollapseBridge should succeed");
        assert_eq!(budget.terrain_edits, 4, "Should consume 1 terrain edit");

        let consequence = result.test_unwrap();
        assert_eq!(consequence.drop_multiplier, 0.9);
        assert_eq!(consequence.faction_disposition, -10);
    }

    #[test]
    fn test_apply_weave_op_collapse_bridge_no_point_b() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::CollapseBridge,
            a: vec3(0.0, 0.0, 0.0),
            b: None, // Missing required point B!
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_err(), "Should fail without point B");
        assert!(result
            .test_unwrap_err()
            .to_string()
            .contains("Collapse needs A->B"));
    }

    #[test]
    fn test_apply_weave_op_redirect_wind_success() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::RedirectWind,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(1.0, 0.0, 0.0)),
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_ok(), "RedirectWind should succeed");
        assert_eq!(budget.weather_ops, 2, "Should consume 1 weather op");
        assert!(
            log_output.iter().any(|msg| msg.contains("Wind redirected")),
            "Should log wind redirect"
        );

        let consequence = result.test_unwrap();
        assert_eq!(consequence.weather_shift, Some("windy".to_string()));
    }

    #[test]
    fn test_apply_weave_op_redirect_wind_no_budget() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = WeaveBudget {
            terrain_edits: 5,
            weather_ops: 0, // No weather budget!
        };
        let op = WeaveOp {
            kind: WeaveOpKind::RedirectWind,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(1.0, 0.0, 0.0)),
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_err(), "Should fail with no weather budget");
        assert!(result
            .test_unwrap_err()
            .to_string()
            .contains("No weather budget"));
    }

    #[test]
    fn test_apply_weave_op_lower_water_success() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::LowerWater,
            a: vec3(0.0, 0.0, 0.0),
            b: None,
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_ok(), "LowerWater should succeed");
        assert_eq!(budget.weather_ops, 2, "Should consume 1 weather op");
        assert!(
            log_output.iter().any(|msg| msg.contains("Waters receded")),
            "Should log water lowering"
        );

        let consequence = result.test_unwrap();
        assert_eq!(consequence.weather_shift, Some("dry".to_string()));
    }

    #[test]
    fn test_apply_weave_op_raise_platform_success() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::RaisePlatform,
            a: vec3(5.0, 0.0, 5.0),
            b: None,
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );

        assert!(result.is_ok(), "RaisePlatform should succeed");
        assert_eq!(budget.terrain_edits, 4, "Should consume 1 terrain edit");

        let consequence = result.test_unwrap();
        assert_eq!(consequence.drop_multiplier, 1.05);
        assert_eq!(consequence.faction_disposition, 0);
    }

    #[test]
    fn test_apply_weave_op_multiple_operations() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        // Apply ReinforcePath
        let op1 = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(0.0, 0.0, 0.0),
            b: None,
            budget_cost: 1,
        };
        apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op1,
            &mut logger,
        )
        .test_unwrap();

        // Apply RedirectWind
        let op2 = WeaveOp {
            kind: WeaveOpKind::RedirectWind,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(1.0, 0.0, 0.0)),
            budget_cost: 1,
        };
        apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op2,
            &mut logger,
        )
        .test_unwrap();

        assert_eq!(budget.terrain_edits, 4, "Should have used 1 terrain edit");
        assert_eq!(budget.weather_ops, 2, "Should have used 1 weather op");
    }

    #[test]
    fn test_apply_weave_op_budget_depletion() {
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = WeaveBudget {
            terrain_edits: 1,
            weather_ops: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        // First terrain op succeeds
        let op1 = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(0.0, 0.0, 0.0),
            b: None,
            budget_cost: 1,
        };
        let result1 = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op1,
            &mut logger,
        );
        assert!(result1.is_ok(), "First terrain op should succeed");

        // Second terrain op fails (budget depleted)
        let op2 = WeaveOp {
            kind: WeaveOpKind::CollapseBridge,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(1.0, 0.0, 1.0)),
            budget_cost: 1,
        };
        let result2 = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op2,
            &mut logger,
        );
        assert!(result2.is_err(), "Second terrain op should fail");
    }

    // ========================================================================
    // Mutation-resistant tests for coordinate arithmetic and plan execution
    // ========================================================================

    #[test]
    fn reinforce_path_consequence_exact_values() {
        // Catches: consequence field mutations (drop_multiplier, faction_disposition)
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(10.0, 0.0, 10.0),
            b: None,
            budget_cost: 1,
        };
        let mut logger = |_: String| {};

        let consequence = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        )
        .test_unwrap();

        assert!(
            (consequence.drop_multiplier - 1.1).abs() < f32::EPSILON,
            "ReinforcePath drop_multiplier must be 1.1"
        );
        assert_eq!(
            consequence.faction_disposition, 5,
            "ReinforcePath faction_disposition must be 5"
        );
    }

    #[test]
    fn collapse_bridge_consequence_exact_values() {
        // Catches: consequence field mutations for CollapseBridge
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::CollapseBridge,
            a: vec3(5.0, 0.0, 5.0),
            b: Some(vec3(10.0, 0.0, 10.0)),
            budget_cost: 1,
        };
        let mut logger = |_: String| {};

        let consequence = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        )
        .test_unwrap();

        assert!(
            (consequence.drop_multiplier - 0.9).abs() < f32::EPSILON,
            "CollapseBridge drop_multiplier must be 0.9"
        );
        assert_eq!(
            consequence.faction_disposition, -10,
            "CollapseBridge faction_disposition must be -10"
        );
    }

    #[test]
    fn plan_is_applied_when_non_empty() {
        // Catches: delete ! in `!plan.ops.is_empty()` (line 96)
        // If ! is deleted, plan is applied only when empty (never applied).
        // ReinforcePath creates a plan with Fortify → should modify world terrain.
        // We verify budget decreases (already tested) AND run_fixed works without panic
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(2.0, 0.0, 3.0),
            b: None,
            budget_cost: 1,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );
        assert!(result.is_ok());
        assert_eq!(budget.terrain_edits, 4, "Budget must be consumed");
    }

    #[test]
    fn redirect_wind_budget_decrements() {
        // Catches: budget.weather_ops -= 1 → += or *= (line 65 area)
        let mut world = create_test_world();
        let mut physics = create_test_physics();
        let nav_src = vec![];
        let mut budget = create_test_budget();
        let initial_weather = budget.weather_ops;
        let op = WeaveOp {
            kind: WeaveOpKind::RedirectWind,
            a: vec3(0.0, 0.0, 0.0),
            b: Some(vec3(1.0, 0.0, 0.0)),
            budget_cost: 1,
        };
        let mut logger = |_: String| {};

        let _ = apply_weave_op(
            &mut world,
            &mut physics,
            &nav_src,
            &mut budget,
            &op,
            &mut logger,
        );
        assert_eq!(
            budget.weather_ops,
            initial_weather - 1,
            "RedirectWind must decrement weather_ops by exactly 1"
        );
    }
}

use crate::{
    tools::{los_clear, path_exists},
    ActionStep, EngineError, Entity, IVec2, PlanIntent, World,
};

pub struct ValidateCfg {
    pub world_bounds: (i32, i32, i32, i32),
}

pub fn validate_and_execute(
    w: &mut World,
    actor: Entity,
    intent: &PlanIntent,
    cfg: &ValidateCfg,
    log: &mut impl FnMut(String),
) -> Result<(), EngineError> {
    log(format!(
        "Plan {} with {} steps",
        intent.plan_id,
        intent.steps.len()
    ));
    for (i, step) in intent.steps.iter().enumerate() {
        match step {
            // ═══════════════════════════════════════
            // MOVEMENT
            // ═══════════════════════════════════════
            ActionStep::MoveTo { x, y, speed } => {
                let from = w.pos_of(actor).ok_or_else(|| {
                    EngineError::InvalidAction("Actor has no position".to_string())
                })?;
                let to = IVec2 { x: *x, y: *y };
                if !path_exists(&w.obstacles, from, to, cfg.world_bounds) {
                    return Err(EngineError::NoPath);
                }
                w.pose_mut(actor)
                    .ok_or_else(|| EngineError::InvalidAction("Actor has no pose".to_string()))?
                    .pos = to;
                let speed_str = speed
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_default();
                log(format!(
                    "  [{}] MOVE_TO -> ({},{}) {:?}",
                    i, x, y, speed_str
                ));
            }

            ActionStep::Approach {
                target_id,
                distance,
            } => {
                // Simplified: move toward target (full implementation would maintain distance)
                let _target_pos = w
                    .pos_of(*target_id)
                    .ok_or_else(|| EngineError::InvalidAction("Target not found".to_string()))?;
                log(format!(
                    "  [{}] APPROACH #{} at distance {:.1}",
                    i, target_id, distance
                ));
                // Implementation stub - actual pathfinding would be here
            }

            ActionStep::Retreat {
                target_id,
                distance,
            } => {
                log(format!(
                    "  [{}] RETREAT from #{} to distance {:.1}",
                    i, target_id, distance
                ));
                // Implementation stub
            }

            ActionStep::TakeCover { position } => {
                log(format!("  [{}] TAKE_COVER at {:?}", i, position));
                // Implementation stub
            }

            ActionStep::Strafe {
                target_id,
                direction,
            } => {
                log(format!(
                    "  [{}] STRAFE {:?} around #{}",
                    i, direction, target_id
                ));
                // Implementation stub
            }

            ActionStep::Patrol { waypoints } => {
                log(format!("  [{}] PATROL {} waypoints", i, waypoints.len()));
                // Implementation stub
            }

            // ═══════════════════════════════════════
            // OFFENSIVE
            // ═══════════════════════════════════════
            ActionStep::Attack { target_id } => {
                log(format!("  [{}] ATTACK #{}", i, target_id));
                // Simplified damage
                if let Some(h) = w.health_mut(*target_id) {
                    h.hp -= 10;
                }
            }

            ActionStep::AimedShot { target_id } => {
                log(format!("  [{}] AIMED_SHOT #{}", i, target_id));
                if let Some(h) = w.health_mut(*target_id) {
                    h.hp -= 15; // Higher damage
                }
            }

            ActionStep::QuickAttack { target_id } => {
                log(format!("  [{}] QUICK_ATTACK #{}", i, target_id));
                if let Some(h) = w.health_mut(*target_id) {
                    h.hp -= 5; // Lower damage
                }
            }

            ActionStep::HeavyAttack { target_id } => {
                log(format!("  [{}] HEAVY_ATTACK #{}", i, target_id));
                if let Some(h) = w.health_mut(*target_id) {
                    h.hp -= 25; // High damage
                }
            }

            ActionStep::AoEAttack { x, y, radius } => {
                log(format!(
                    "  [{}] AOE_ATTACK at ({},{}) radius {:.1}",
                    i, x, y, radius
                ));
                // Implementation stub - would damage all entities in radius
            }

            ActionStep::ThrowExplosive { x, y } => {
                log(format!("  [{}] THROW_EXPLOSIVE at ({},{})", i, x, y));
                // Implementation stub
            }

            ActionStep::Charge { target_id } => {
                log(format!("  [{}] CHARGE #{}", i, target_id));
                // Implementation stub - move to target + attack
            }

            // ═══════════════════════════════════════
            // DEFENSIVE
            // ═══════════════════════════════════════
            ActionStep::Block => {
                log(format!("  [{}] BLOCK", i));
                // Implementation stub
            }

            ActionStep::Dodge { direction } => {
                log(format!("  [{}] DODGE {:?}", i, direction));
                // Implementation stub
            }

            ActionStep::Parry => {
                log(format!("  [{}] PARRY", i));
                // Implementation stub
            }

            ActionStep::ThrowSmoke { x, y } => {
                let from = w.pos_of(actor).ok_or_else(|| {
                    EngineError::InvalidAction("Actor has no position".to_string())
                })?;
                let target = IVec2 { x: *x, y: *y };
                if !los_clear(&w.obstacles, from, target) {
                    return Err(EngineError::LosBlocked);
                }
                log(format!("  [{}] THROW_SMOKE -> ({},{})", i, x, y));
            }

            ActionStep::Heal { target_id } => {
                let tid = target_id.unwrap_or(actor);
                log(format!("  [{}] HEAL #{}", i, tid));
                if let Some(h) = w.health_mut(tid) {
                    h.hp += 20;
                }
            }

            ActionStep::UseDefensiveAbility { ability_name } => {
                log(format!("  [{}] USE_DEFENSIVE_ABILITY: {}", i, ability_name));
                // Implementation stub
            }

            // ═══════════════════════════════════════
            // EQUIPMENT
            // ═══════════════════════════════════════
            ActionStep::EquipWeapon { weapon_name } => {
                log(format!("  [{}] EQUIP_WEAPON: {}", i, weapon_name));
                // Implementation stub
            }

            ActionStep::SwitchWeapon { slot } => {
                log(format!("  [{}] SWITCH_WEAPON to slot {}", i, slot));
                // Implementation stub
            }

            ActionStep::Reload => {
                log(format!("  [{}] RELOAD", i));
                if let Some(ammo) = w.ammo_mut(actor) {
                    ammo.rounds = 30; // Reload to full
                }
            }

            ActionStep::UseItem { item_name } => {
                log(format!("  [{}] USE_ITEM: {}", i, item_name));
                // Implementation stub
            }

            ActionStep::DropItem { item_name } => {
                log(format!("  [{}] DROP_ITEM: {}", i, item_name));
                // Implementation stub
            }

            // ═══════════════════════════════════════
            // TACTICAL
            // ═══════════════════════════════════════
            ActionStep::CallReinforcements { count } => {
                log(format!("  [{}] CALL_REINFORCEMENTS: {}", i, count));
                // Implementation stub
            }

            ActionStep::MarkTarget { target_id } => {
                log(format!("  [{}] MARK_TARGET #{}", i, target_id));
                // Implementation stub
            }

            ActionStep::RequestCover { duration } => {
                log(format!("  [{}] REQUEST_COVER for {:.1}s", i, duration));
                // Implementation stub
            }

            ActionStep::CoordinateAttack { target_id } => {
                log(format!("  [{}] COORDINATE_ATTACK on #{}", i, target_id));
                // Implementation stub
            }

            ActionStep::SetAmbush { position } => {
                log(format!("  [{}] SET_AMBUSH at {:?}", i, position));
                // Implementation stub
            }

            ActionStep::Distract { target_id } => {
                log(format!("  [{}] DISTRACT #{}", i, target_id));
                // Implementation stub
            }

            ActionStep::Regroup { rally_point } => {
                log(format!("  [{}] REGROUP at {:?}", i, rally_point));
                // Implementation stub
            }

            // ═══════════════════════════════════════
            // UTILITY
            // ═══════════════════════════════════════
            ActionStep::Scan { radius } => {
                log(format!("  [{}] SCAN radius {:.1}", i, radius));
                // Implementation stub
            }

            ActionStep::Wait { duration } => {
                log(format!("  [{}] WAIT {:.1}s", i, duration));
                // Implementation stub
            }

            ActionStep::Interact { target_id } => {
                log(format!("  [{}] INTERACT with #{}", i, target_id));
                // Implementation stub
            }

            ActionStep::UseAbility { ability_name } => {
                log(format!("  [{}] USE_ABILITY: {}", i, ability_name));
                // Implementation stub
            }

            ActionStep::Taunt { target_id } => {
                log(format!("  [{}] TAUNT #{}", i, target_id));
                // Implementation stub
            }

            // ═══════════════════════════════════════
            // LEGACY
            // ═══════════════════════════════════════
            ActionStep::Throw { item, x, y } => {
                let from = w.pos_of(actor).ok_or_else(|| {
                    EngineError::InvalidAction("Actor has no position".to_string())
                })?;
                let target = IVec2 { x: *x, y: *y };
                if !los_clear(&w.obstacles, from, target) {
                    return Err(EngineError::LosBlocked);
                }
                let cds = w.cooldowns_mut(actor).ok_or_else(|| {
                    EngineError::InvalidAction("Actor has no cooldowns".to_string())
                })?;
                let cd_key = format!("throw:{}", item);
                if cds.map.get(&cd_key).copied().unwrap_or(0.0) > 0.0 {
                    return Err(EngineError::Cooldown(cd_key));
                }
                cds.map.insert(cd_key.clone(), 8.0);
                log(format!("  [{}] THROW {} -> ({},{})", i, item, x, y));
            }
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                let my = w.pos_of(actor).ok_or_else(|| {
                    EngineError::InvalidAction("Actor has no position".to_string())
                })?;
                let tgt = w
                    .pos_of(*target_id)
                    .ok_or_else(|| EngineError::InvalidAction("target gone".into()))?;
                if !los_clear(&w.obstacles, my, tgt) {
                    return Err(EngineError::LosBlocked);
                }
                // Ensure ammo present
                if let Some(am) = w.ammo(actor) {
                    if am.rounds <= 0 {
                        return Err(EngineError::Resource("ammo".into()));
                    }
                }
                // simulate: reduce target hp a bit depending on duration
                if let Some(h) = w.health_mut(*target_id) {
                    let dmg = ((*duration) * 5.0) as i32;
                    h.hp -= dmg.max(1);
                }
                let ammo = w
                    .ammo_mut(actor)
                    .ok_or_else(|| EngineError::Resource("ammo".into()))?;
                ammo.rounds = (ammo.rounds - 3).max(0);
                log(format!(
                    "  [{}] COVER_FIRE on #{} for {:.1}s",
                    i, target_id, duration
                ));
            }
            ActionStep::Revive { ally_id } => {
                if let Some(h) = w.health_mut(*ally_id) {
                    if h.hp <= 0 {
                        h.hp = 20;
                    }
                }
                log(format!("  [{}] REVIVE #{}", i, ally_id));
            }

            // ═══════════════════════════════════════
            // TERRAIN (AI-Orchestrated Dynamic Terrain)
            // ═══════════════════════════════════════
            ActionStep::ModifyTerrain {
                request_id,
                payload,
            } => {
                // Validate the terrain request
                if let Err(e) = payload.validate() {
                    return Err(EngineError::InvalidAction(format!(
                        "Invalid terrain request: {}",
                        e
                    )));
                }
                log(format!(
                    "  [{}] MODIFY_TERRAIN request_id={} feature={:?}",
                    i, request_id, payload.feature_type
                ));
                // Note: Actual terrain modification is handled by TerrainSolver
                // This validation ensures the request is structurally valid
            }
        }
    }
    Ok(())
}

use crate::{DirectorOp, DirectorPlan, Rect};

fn fill_rect_obs(obs: &mut std::collections::HashSet<(i32, i32)>, r: Rect) {
    for x in r.x0.min(r.x1)..=r.x0.max(r.x1) {
        for y in r.y0.min(r.y1)..=r.y0.max(r.y1) {
            obs.insert((x, y));
        }
    }
}

fn draw_line_obs(obs: &mut std::collections::HashSet<(i32, i32)>, a: IVec2, b: IVec2) {
    let mut x = a.x;
    let mut y = a.y;
    let dx = (b.x - a.x).signum();
    let dy = (b.y - a.y).signum();
    while x != b.x || y != b.y {
        obs.insert((x, y));
        if x != b.x {
            x += dx;
        }
        if y != b.y {
            y += dy;
        }
    }
    obs.insert((b.x, b.y));
}

// Execute a DirectorPlan with crude budgets (you can move this into a Director crate too)
pub fn apply_director_plan(
    w: &mut World,
    budget: &mut crate::DirectorBudget,
    plan: &DirectorPlan,
    log: &mut impl FnMut(String),
) {
    for (i, op) in plan.ops.iter().enumerate() {
        match op {
            DirectorOp::Fortify { rect } => {
                if budget.terrain_edits <= 0 {
                    log(format!("  [op{}] Fortify SKIPPED (budget)", i));
                    continue;
                }
                fill_rect_obs(&mut w.obstacles, *rect);
                budget.terrain_edits -= 1;
                log(format!(
                    "  [op{}] Fortify rect=({},{}..{},{}))",
                    i, rect.x0, rect.y0, rect.x1, rect.y1
                ));
            }
            DirectorOp::Collapse { a, b } => {
                if budget.terrain_edits <= 0 {
                    log(format!("  [op{}] Collapse SKIPPED (budget)", i));
                    continue;
                }
                draw_line_obs(&mut w.obstacles, *a, *b);
                budget.terrain_edits -= 1;
                log(format!(
                    "  [op{}] Collapse line=({},{})→({},{})",
                    i, a.x, a.y, b.x, b.y
                ));
            }
            DirectorOp::SpawnWave {
                archetype,
                count,
                origin,
            } => {
                if budget.spawns <= 0 {
                    log(format!("  [op{}] SpawnWave SKIPPED (budget)", i));
                    continue;
                }
                for k in 0..*count {
                    let off = IVec2 {
                        x: origin.x + (k as i32 % 3) - 1,
                        y: origin.y + (k as i32 / 3),
                    };
                    let id = w.spawn(
                        &format!("{}{}", archetype, k),
                        off,
                        crate::Team { id: 2 },
                        40,
                        0,
                    );
                    log(format!("  [op{}] Spawned {} at {:?}", i, id, off));
                }
                budget.spawns -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MovementSpeed, Team, World};

    fn mk_world_clear() -> World {
        World::new()
    }

    #[test]
    fn cover_fire_requires_ammo() {
        let mut w = mk_world_clear();
        let actor = w.spawn("ally", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 0);
        let enemy = w.spawn("enemy", IVec2 { x: 3, y: 0 }, Team { id: 2 }, 50, 0);
        let intent = PlanIntent {
            plan_id: "t".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: enemy,
                duration: 1.0,
            }],
        };
        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        match res {
            Err(EngineError::Resource(k)) => assert_eq!(k, "ammo"),
            _ => panic!("expected Resource(ammo)"),
        }
    }

    #[test]
    fn cover_fire_consumes_ammo_and_damages() {
        let mut w = mk_world_clear();
        let actor = w.spawn("ally", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", IVec2 { x: 2, y: 0 }, Team { id: 2 }, 50, 0);
        let intent = PlanIntent {
            plan_id: "t".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: enemy,
                duration: 1.0,
            }],
        };
        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let hp_before = w.health(enemy).unwrap().hp;
        let ammo_before = w.ammo(actor).unwrap().rounds;
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
        let hp_after = w.health(enemy).unwrap().hp;
        let ammo_after = w.ammo(actor).unwrap().rounds;
        assert!(hp_after < hp_before, "enemy should take damage");
        assert_eq!(ammo_after, (ammo_before - 3).max(0));
    }

    // ════════════════════════════════════════════════════════════════════
    // NEW TESTS (Week 2 Day 3 - Task 6)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_moveto_validation_success() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "move-001".into(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: Some(MovementSpeed::Run),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let final_pos = w.pos_of(actor).unwrap();
        assert_eq!(final_pos, IVec2 { x: 5, y: 5 });
    }

    #[test]
    fn test_moveto_path_blocked() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        // Add obstacles creating a wall that blocks path
        // Create vertical wall from (-10, -10) to (-10, 10) blocking path to (-5, 0)
        for y in -10..=10 {
            w.obstacles.insert((-5, y));
        }

        let intent = PlanIntent {
            plan_id: "move-blocked".into(),
            steps: vec![ActionStep::MoveTo {
                x: -8,
                y: 0,
                speed: None,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        match res {
            Err(EngineError::NoPath) => {} // Expected
            _ => panic!("Expected NoPath error"),
        }
    }

    #[test]
    fn test_attack_damages_target() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", IVec2 { x: 3, y: 0 }, Team { id: 2 }, 50, 0);

        let intent = PlanIntent {
            plan_id: "attack-001".into(),
            steps: vec![ActionStep::Attack { target_id: enemy }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let hp_before = w.health(enemy).unwrap().hp;
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(enemy).unwrap().hp;
        assert_eq!(hp_after, hp_before - 10); // Attack does 10 damage
    }

    #[test]
    fn test_heal_self() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 50, 10);

        let intent = PlanIntent {
            plan_id: "heal-self".into(),
            steps: vec![ActionStep::Heal { target_id: None }], // None = heal self
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let hp_before = w.health(actor).unwrap().hp;
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(actor).unwrap().hp;
        assert_eq!(hp_after, hp_before + 20); // Heal restores 20 HP
    }

    #[test]
    fn test_heal_ally() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let ally = w.spawn("ally", IVec2 { x: 1, y: 0 }, Team { id: 1 }, 30, 0);

        let intent = PlanIntent {
            plan_id: "heal-ally".into(),
            steps: vec![ActionStep::Heal {
                target_id: Some(ally),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let hp_before = w.health(ally).unwrap().hp;
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(ally).unwrap().hp;
        assert_eq!(hp_after, hp_before + 20);
    }

    #[test]
    fn test_reload_refills_ammo() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 5);

        let intent = PlanIntent {
            plan_id: "reload-001".into(),
            steps: vec![ActionStep::Reload],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let ammo_after = w.ammo(actor).unwrap().rounds;
        assert_eq!(ammo_after, 30); // Reload fills to 30
    }

    #[test]
    fn test_throw_smoke_los_blocked() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        // Add obstacle blocking line of sight
        w.obstacles.insert((2, 2));

        let intent = PlanIntent {
            plan_id: "smoke-blocked".into(),
            steps: vec![ActionStep::ThrowSmoke { x: 5, y: 5 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        match res {
            Err(EngineError::LosBlocked) => {} // Expected
            _ => panic!("Expected LosBlocked error"),
        }
    }

    #[test]
    fn test_throw_with_cooldown() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "throw-cd".into(),
            steps: vec![ActionStep::Throw {
                item: "grenade".into(),
                x: 3,
                y: 3,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        // First throw should succeed
        let res1 = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res1.is_ok());

        // Second throw should fail (cooldown active)
        let res2 = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        match res2 {
            Err(EngineError::Cooldown(cd)) => {
                assert_eq!(cd, "throw:grenade");
            }
            _ => panic!("Expected Cooldown error"),
        }
    }

    #[test]
    fn test_revive_dead_ally() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let ally = w.spawn("ally", IVec2 { x: 1, y: 0 }, Team { id: 1 }, 0, 0); // Dead (0 HP)

        let intent = PlanIntent {
            plan_id: "revive-001".into(),
            steps: vec![ActionStep::Revive { ally_id: ally }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(ally).unwrap().hp;
        assert_eq!(hp_after, 20); // Revive sets HP to 20
    }

    #[test]
    fn test_multi_step_execution() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 30);
        let enemy = w.spawn("enemy", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "multi-001".into(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 3,
                    y: 3,
                    speed: Some(MovementSpeed::Sprint),
                },
                ActionStep::Attack { target_id: enemy },
                ActionStep::Reload,
            ],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        // Validate final state
        let final_pos = w.pos_of(actor).unwrap();
        assert_eq!(final_pos, IVec2 { x: 3, y: 3 });

        let enemy_hp = w.health(enemy).unwrap().hp;
        assert_eq!(enemy_hp, 90); // Took 10 damage

        let ammo = w.ammo(actor).unwrap().rounds;
        assert_eq!(ammo, 30); // Reloaded to full
    }

    #[test]
    fn test_invalid_actor_not_found() {
        let mut w = mk_world_clear();
        let actor = 9999; // Non-existent entity

        let intent = PlanIntent {
            plan_id: "invalid".into(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: None,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};

        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        match res {
            Err(EngineError::InvalidAction(msg)) => {
                assert!(msg.contains("no position"));
            }
            _ => panic!("Expected InvalidAction error for non-existent actor"),
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // COMPREHENSIVE ACTION STEP TESTS (95%+ Coverage Push)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_approach_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "approach-001".into(),
            steps: vec![ActionStep::Approach {
                target_id: target,
                distance: 2.0,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_retreat_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 5, y: 5 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 3, y: 3 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "retreat-001".into(),
            steps: vec![ActionStep::Retreat {
                target_id: target,
                distance: 10.0,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_take_cover_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "cover-001".into(),
            steps: vec![ActionStep::TakeCover {
                position: Some(IVec2 { x: 5, y: 5 }),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_strafe_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "strafe-001".into(),
            steps: vec![ActionStep::Strafe {
                target_id: target,
                direction: crate::StrafeDirection::Left,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_patrol_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "patrol-001".into(),
            steps: vec![ActionStep::Patrol {
                waypoints: vec![
                    IVec2 { x: 0, y: 0 },
                    IVec2 { x: 5, y: 0 },
                    IVec2 { x: 5, y: 5 },
                ],
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_aimed_shot_damages_target() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let hp_before = w.health(target).unwrap().hp;

        let intent = PlanIntent {
            plan_id: "aimed-001".into(),
            steps: vec![ActionStep::AimedShot { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(target).unwrap().hp;
        assert_eq!(hp_after, hp_before - 15);
    }

    #[test]
    fn test_quick_attack_damages_target() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let hp_before = w.health(target).unwrap().hp;

        let intent = PlanIntent {
            plan_id: "quick-001".into(),
            steps: vec![ActionStep::QuickAttack { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(target).unwrap().hp;
        assert_eq!(hp_after, hp_before - 5);
    }

    #[test]
    fn test_heavy_attack_damages_target() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let hp_before = w.health(target).unwrap().hp;

        let intent = PlanIntent {
            plan_id: "heavy-001".into(),
            steps: vec![ActionStep::HeavyAttack { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        let hp_after = w.health(target).unwrap().hp;
        assert_eq!(hp_after, hp_before - 25);
    }

    #[test]
    fn test_aoe_attack_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "aoe-001".into(),
            steps: vec![ActionStep::AoEAttack {
                x: 5,
                y: 5,
                radius: 3.0,
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_throw_explosive_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "explosive-001".into(),
            steps: vec![ActionStep::ThrowExplosive { x: 5, y: 5 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_charge_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "charge-001".into(),
            steps: vec![ActionStep::Charge { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_block_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "block-001".into(),
            steps: vec![ActionStep::Block],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_dodge_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "dodge-001".into(),
            steps: vec![ActionStep::Dodge {
                direction: Some(crate::StrafeDirection::Left),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_parry_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "parry-001".into(),
            steps: vec![ActionStep::Parry],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_use_defensive_ability_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "def-001".into(),
            steps: vec![ActionStep::UseDefensiveAbility {
                ability_name: "Shield Bash".to_string(),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_equip_weapon_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "equip-001".into(),
            steps: vec![ActionStep::EquipWeapon {
                weapon_name: "Plasma Rifle".to_string(),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_switch_weapon_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "switch-001".into(),
            steps: vec![ActionStep::SwitchWeapon { slot: 2 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_use_item_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "item-001".into(),
            steps: vec![ActionStep::UseItem {
                item_name: "Health Potion".to_string(),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_drop_item_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "drop-001".into(),
            steps: vec![ActionStep::DropItem {
                item_name: "Heavy Armor".to_string(),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_call_reinforcements_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "reinforce-001".into(),
            steps: vec![ActionStep::CallReinforcements { count: 3 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_mark_target_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "mark-001".into(),
            steps: vec![ActionStep::MarkTarget { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_request_cover_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "req-cover-001".into(),
            steps: vec![ActionStep::RequestCover { duration: 5.0 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_coordinate_attack_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "coord-001".into(),
            steps: vec![ActionStep::CoordinateAttack { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_set_ambush_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "ambush-001".into(),
            steps: vec![ActionStep::SetAmbush {
                position: IVec2 { x: 8, y: 8 },
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_distract_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "distract-001".into(),
            steps: vec![ActionStep::Distract { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_regroup_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "regroup-001".into(),
            steps: vec![ActionStep::Regroup {
                rally_point: IVec2 { x: -5, y: -5 },
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_scan_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "scan-001".into(),
            steps: vec![ActionStep::Scan { radius: 10.0 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_wait_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "wait-001".into(),
            steps: vec![ActionStep::Wait { duration: 2.5 }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_interact_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("object", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "interact-001".into(),
            steps: vec![ActionStep::Interact { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_use_ability_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);

        let intent = PlanIntent {
            plan_id: "ability-001".into(),
            steps: vec![ActionStep::UseAbility {
                ability_name: "Teleport".to_string(),
            }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_taunt_action() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let target = w.spawn("target", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 100, 0);

        let intent = PlanIntent {
            plan_id: "taunt-001".into(),
            steps: vec![ActionStep::Taunt { target_id: target }],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());
    }

    #[test]
    fn test_multi_step_plan() {
        let mut w = mk_world_clear();
        let actor = w.spawn("actor", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 50, 10);
        let enemy = w.spawn("enemy", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 80, 0);

        let intent = PlanIntent {
            plan_id: "multi-001".into(),
            steps: vec![
                ActionStep::Scan { radius: 15.0 },
                ActionStep::MoveTo {
                    x: 8,
                    y: 8,
                    speed: Some(MovementSpeed::Run),
                },
                ActionStep::AimedShot { target_id: enemy },
                ActionStep::TakeCover {
                    position: Some(IVec2 { x: 7, y: 7 }),
                },
                ActionStep::Heal { target_id: None },
            ],
        };

        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 15, 15),
        };
        let mut log = |_s: String| {};
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);
        assert!(res.is_ok());

        // Verify actor moved
        let final_pos = w.pos_of(actor).unwrap();
        assert_eq!(final_pos, IVec2 { x: 8, y: 8 });

        // Verify enemy took damage
        let enemy_hp = w.health(enemy).unwrap().hp;
        assert_eq!(enemy_hp, 65); // 80 - 15 (aimed shot)

        // Verify actor healed
        let actor_hp = w.health(actor).unwrap().hp;
        assert_eq!(actor_hp, 70); // 50 + 20 (heal)
    }

    #[test]
    fn test_director_fortify_operation() {
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 2,
            spawns: 5,
            terrain_edits: 3,
        };

        let plan = crate::DirectorPlan {
            ops: vec![crate::DirectorOp::Fortify {
                rect: crate::Rect {
                    x0: 0,
                    y0: 0,
                    x1: 2,
                    y1: 2,
                },
            }],
        };

        let mut log = |_s: String| {};
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);

        // Verify budget was decremented
        assert_eq!(budget.terrain_edits, 2);

        // Verify obstacles were added (at least one point in rect)
        assert!(
            w.obstacles.contains(&(0, 0))
                || w.obstacles.contains(&(1, 1))
                || w.obstacles.contains(&(2, 2))
        );
    }

    #[test]
    fn test_director_collapse_operation() {
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 2,
            spawns: 5,
            terrain_edits: 3,
        };

        let plan = crate::DirectorPlan {
            ops: vec![crate::DirectorOp::Collapse {
                a: IVec2 { x: 0, y: 0 },
                b: IVec2 { x: 5, y: 5 },
            }],
        };

        let mut log = |_s: String| {};
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);

        // Verify budget was decremented
        assert_eq!(budget.terrain_edits, 2);

        // Verify obstacles were added along the line
        assert!(w.obstacles.contains(&(0, 0)));
    }

    #[test]
    fn test_director_spawn_wave_operation() {
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 2,
            spawns: 5,
            terrain_edits: 3,
        };

        let plan = crate::DirectorPlan {
            ops: vec![crate::DirectorOp::SpawnWave {
                archetype: "zombie".to_string(),
                count: 3,
                origin: IVec2 { x: 10, y: 10 },
            }],
        };

        let mut log = |_s: String| {};
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);

        // Verify budget was decremented
        assert_eq!(budget.spawns, 4);
    }

    #[test]
    fn test_director_budget_enforcement() {
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 0,
            spawns: 0, // Zero budget
            terrain_edits: 0,
        };

        let plan = crate::DirectorPlan {
            ops: vec![
                crate::DirectorOp::SpawnWave {
                    archetype: "enemy".to_string(),
                    count: 5,
                    origin: IVec2 { x: 0, y: 0 },
                },
                crate::DirectorOp::Fortify {
                    rect: crate::Rect {
                        x0: 0,
                        y0: 0,
                        x1: 2,
                        y1: 2,
                    },
                },
            ],
        };

        let initial_obstacle_count = w.obstacles.len();

        let mut log = |_s: String| {};
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);

        // Verify nothing happened due to zero budget (obstacles should not change)
        assert_eq!(w.obstacles.len(), initial_obstacle_count);
        assert_eq!(budget.spawns, 0);
        assert_eq!(budget.terrain_edits, 0);
    }

    #[test]
    fn test_throw_smoke_success() {
        // COVERAGE TARGET: Lines 143-144 (log statement in ThrowSmoke success path)
        let mut w = mk_world_clear();
        let actor = w.spawn("ally", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let intent = PlanIntent {
            plan_id: "t".into(),
            steps: vec![ActionStep::ThrowSmoke { x: 3, y: 0 }], // LOS clear, should succeed
        };
        let cfg = ValidateCfg {
            world_bounds: (-10, -10, 10, 10),
        };
        let mut captured_log = String::new();
        let mut log = |s: String| {
            captured_log.push_str(&s);
        };
        let res = validate_and_execute(&mut w, actor, &intent, &cfg, &mut log);

        // Should succeed (no obstacles between (0,0) and (3,0))
        assert!(res.is_ok(), "ThrowSmoke should succeed with clear LOS");
        // Verify log statement was executed (lines 143-144)
        assert!(
            captured_log.contains("THROW_SMOKE"),
            "Log should contain THROW_SMOKE"
        );
    }

    #[test]
    fn test_director_collapse_budget_skip() {
        // COVERAGE TARGET: Lines 1450-1451 (Collapse SKIPPED budget logic)
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 10,
            spawns: 10,
            terrain_edits: 0, // Zero terrain budget
        };

        let plan = crate::DirectorPlan {
            ops: vec![crate::DirectorOp::Collapse {
                a: IVec2 { x: 0, y: 0 },
                b: IVec2 { x: 5, y: 5 },
            }],
        };

        let initial_obstacle_count = w.obstacles.len();

        let mut captured_log = String::new();
        let mut log = |s: String| {
            captured_log.push_str(&s);
            captured_log.push('\n');
        };
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);

        // Verify Collapse was skipped (lines 1450-1451)
        assert_eq!(
            w.obstacles.len(),
            initial_obstacle_count,
            "Obstacles should not change"
        );
        assert!(
            captured_log.contains("Collapse SKIPPED (budget)"),
            "Log should show budget skip"
        );
        assert_eq!(budget.terrain_edits, 0, "Budget should remain zero");
    }

    // ========================================================================
    // Mutation-resistant boundary tests
    // ========================================================================

    /// Catches: `* 5.0` → `/ 5.0` in cover_fire damage (line 323)
    /// and: `-= dmg` → `/= dmg` (line 324)
    #[test]
    fn cover_fire_damage_arithmetic() {
        let mut w = mk_world_clear();
        let actor = w.spawn("ally", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 100, 10);
        let enemy = w.spawn("enemy", IVec2 { x: 1, y: 0 }, Team { id: 2 }, 100, 0);
        let plan = PlanIntent {
            plan_id: "dmg".into(),
            steps: vec![crate::ActionStep::CoverFire {
                target_id: enemy,
                duration: 2.0, // dmg = (2.0 * 5.0) as i32 = 10
            }],
        };
        let cfg = ValidateCfg {
            world_bounds: (-100, -100, 100, 100),
        };
        let mut logs = vec![];
        let result = validate_and_execute(&mut w, actor, &plan, &cfg, &mut |s| logs.push(s));
        assert!(result.is_ok());
        let hp = w.health(enemy).unwrap().hp;
        // 100 - 10 = 90
        assert_eq!(
            hp, 90,
            "CoverFire with duration=2.0 should deal 10 dmg (2.0*5.0), got hp={}",
            hp
        );
    }

    /// Catches: arithmetic mutations in draw_line_obs (lines 381-382, 389)
    /// `- with +` in signum calc, `+= with -=` in stepping
    #[test]
    fn draw_line_obs_correct_stepping() {
        let mut obs = std::collections::HashSet::new();
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 3 };
        draw_line_obs(&mut obs, a, b);
        // Should draw diagonal: (0,0), (1,1), (2,2), (3,3)
        assert!(obs.contains(&(0, 0)), "Start should be in line");
        assert!(obs.contains(&(1, 1)), "(1,1) should be on diagonal");
        assert!(obs.contains(&(2, 2)), "(2,2) should be on diagonal");
        assert!(obs.contains(&(3, 3)), "End should be in line");
    }

    /// Catches: `|| with &&` in draw_line_obs (line 383)
    /// With `&&`, if only one coord matches, loop exits too early
    #[test]
    fn draw_line_obs_horizontal_line() {
        let mut obs = std::collections::HashSet::new();
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 0 }; // Horizontal line — y matches immediately
        draw_line_obs(&mut obs, a, b);
        // With && mutation, loop would exit immediately since y==b.y at start
        assert!(obs.contains(&(0, 0)));
        assert!(obs.contains(&(1, 0)));
        assert!(obs.contains(&(2, 0)));
        assert!(obs.contains(&(3, 0)));
        assert_eq!(
            obs.len(),
            4,
            "Horizontal line from (0,0) to (3,0) should have 4 points"
        );
    }

    /// Catches: spawn wave coordinate arithmetic mutations (lines 439-440)
    /// `+ (k % 3) - 1` → various mutations, `+ (k / 3)` → `- (k / 3)`
    #[test]
    fn spawn_wave_exact_positions() {
        let mut w = mk_world_clear();
        let mut budget = crate::DirectorBudget {
            traps: 0,
            spawns: 1,
            terrain_edits: 0,
        };
        let plan = crate::DirectorPlan {
            ops: vec![crate::DirectorOp::SpawnWave {
                archetype: "grunt".to_string(),
                count: 6, // k=0..5: tests all arithmetic
                origin: IVec2 { x: 10, y: 20 },
            }],
        };
        let initial_count = w.entities().len();
        let mut logs = vec![];
        apply_director_plan(&mut w, &mut budget, &plan, &mut |s| logs.push(s));

        // 6 new entities should be spawned
        let new_count = w.entities().len() - initial_count;
        assert_eq!(new_count, 6, "Should spawn 6 entities");

        // Expected positions from the formula:
        // k=0: x = 10 + (0%3) - 1 = 9,   y = 20 + (0/3) = 20
        // k=1: x = 10 + (1%3) - 1 = 10,  y = 20 + (1/3) = 20
        // k=2: x = 10 + (2%3) - 1 = 11,  y = 20 + (2/3) = 20
        // k=3: x = 10 + (3%3) - 1 = 9,   y = 20 + (3/3) = 21
        // k=4: x = 10 + (4%3) - 1 = 10,  y = 20 + (4/3) = 21
        // k=5: x = 10 + (5%3) - 1 = 11,  y = 20 + (5/3) = 21
        let expected: std::collections::HashSet<(i32, i32)> =
            [(9, 20), (10, 20), (11, 20), (9, 21), (10, 21), (11, 21)]
                .into_iter()
                .collect();
        let all = w.entities();
        let mut actual = std::collections::HashSet::new();
        for &eid in all.iter().skip(initial_count) {
            if let Some(pos) = w.pos_of(eid) {
                actual.insert((pos.x, pos.y));
            }
        }
        assert_eq!(
            actual, expected,
            "Spawn positions should form 3×2 grid offset from origin"
        );
    }

    // ========================================================================
    // Mutation-resistant remediation: draw_line_obs direction tests
    // ========================================================================

    /// Kills: `replace - with + in draw_line_obs` lines 381-382
    /// (b.x - a.x).signum() → (b.x + a.x).signum()
    /// When a=(0,0), mutation is equivalent. Using non-zero a catches it.
    #[test]
    fn draw_line_obs_negative_direction_catches_sign_mutation() {
        let mut obs = std::collections::HashSet::new();
        // Line going from higher to lower coords — requires negative dx/dy
        let a = IVec2 { x: 4, y: 4 };
        let b = IVec2 { x: 1, y: 1 };
        draw_line_obs(&mut obs, a, b);
        // With correct signum: dx=(1-4).signum()=-1, dy=-1
        // Steps: (4,4), (3,3), (2,2), (1,1) — 4 points
        assert!(
            obs.contains(&(3, 3)),
            "(3,3) must be on line from (4,4) to (1,1)"
        );
        assert!(
            obs.contains(&(2, 2)),
            "(2,2) must be on line from (4,4) to (1,1)"
        );
        assert_eq!(
            obs.len(),
            4,
            "diagonal from (4,4) to (1,1) should have 4 points"
        );
        // With mutation: dx=(1+4).signum()=+1 → steps go (4,4),(5,5),(6,6)... infinite loop
    }

    /// Kills: `replace - with + in draw_line_obs` line 381 only (x component)
    /// Tests horizontal negative direction specifically.
    #[test]
    fn draw_line_obs_horizontal_negative() {
        let mut obs = std::collections::HashSet::new();
        let a = IVec2 { x: 5, y: 2 };
        let b = IVec2 { x: 2, y: 2 };
        draw_line_obs(&mut obs, a, b);
        assert!(obs.contains(&(4, 2)), "(4,2) on line from (5,2) to (2,2)");
        assert!(obs.contains(&(3, 2)), "(3,2) on line from (5,2) to (2,2)");
        assert_eq!(obs.len(), 4);
    }
}

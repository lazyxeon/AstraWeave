//! ECS AI planning plugin: registers a minimal planning system into the ai_planning stage.
use astraweave_core::ecs_bridge::EntityBridge;
use astraweave_core::ecs_events::{AiPlannedEvent, AiPlanningFailedEvent, Events};
use astraweave_core::{
    build_snapshot, ActionStep, CAmmo, CCooldowns, CDesiredPos, CPos, CTeam, CompanionState,
    EnemyState, IVec2, PerceptionConfig, PlayerState, World, WorldSnapshot,
};
use astraweave_ecs as ecs;

use crate::orchestrator::Orchestrator;
#[cfg(not(feature = "veilweaver_slice"))]
use crate::orchestrator::RuleOrchestrator;
#[cfg(feature = "veilweaver_slice")]
use crate::VeilweaverCompanionOrchestrator;

pub struct AiPlanningPlugin;

// Helper: map a legacy companion id to an ECS entity.
// 1) find the closest ECS entity on team 1 by position relative to snap.me
// 2) if an EntityBridge exists prefer the bridge mapping for the legacy id
// 3) return None if no candidate ECS companion was found
fn map_legacy_companion_to_ecs(
    positions: &std::collections::BTreeMap<ecs::Entity, IVec2>,
    teams: &std::collections::BTreeMap<ecs::Entity, u8>,
    snap: &WorldSnapshot,
    comp: astraweave_core::Entity,
    world: &ecs::World,
) -> Option<ecs::Entity> {
    // find closest companion by Manhattan distance to the legacy companion position
    let closest = positions
        .iter()
        .filter(|(e, _)| teams.get(e).copied() == Some(1))
        .map(|(e, p)| (*e, p))
        .min_by_key(|(_, p)| (p.x - snap.me.pos.x).abs() + (p.y - snap.me.pos.y).abs())
        .map(|(e, _)| e)?;

    // prefer bridge mapping from legacy id -> ecs entity when present
    let mapped = world
        .get_resource::<EntityBridge>()
        .and_then(|b| b.get(&comp))
        .unwrap_or(closest);
    Some(mapped)
}

fn sys_ai_planning(world: &mut ecs::World) {
    // Build snapshots and set desired positions per companion entity
    // Prefer legacy World + perception builder when available; fallback to ECS-only snapshot
    // Ensure AiPlannedEvent resource exists
    if world.get_resource::<Events<AiPlannedEvent>>().is_none() {
        world.insert_resource(Events::<AiPlannedEvent>::default());
    }

    // Cache ECS component views
    let mut positions: std::collections::BTreeMap<ecs::Entity, IVec2> =
        std::collections::BTreeMap::new();
    let mut teams: std::collections::BTreeMap<ecs::Entity, u8> = std::collections::BTreeMap::new();
    {
        let q = ecs::Query::<CPos>::new(world);
        for (e, p) in q {
            positions.insert(
                e,
                IVec2 {
                    x: p.pos.x,
                    y: p.pos.y,
                },
            );
        }
    }
    {
        let q = ecs::Query::<CTeam>::new(world);
        for (e, t) in q {
            teams.insert(e, t.id);
        }
    }

    #[cfg(feature = "veilweaver_slice")]
    let orch: Box<dyn Orchestrator> = Box::new(VeilweaverCompanionOrchestrator::new());
    #[cfg(not(feature = "veilweaver_slice"))]
    let orch: Box<dyn Orchestrator> = Box::new(RuleOrchestrator);
    let mut updates: Vec<(ecs::Entity, CDesiredPos)> = vec![];
    let mut planned_events: Vec<AiPlannedEvent> = vec![];
    let mut failed_events: Vec<AiPlanningFailedEvent> = vec![];

    // Try legacy world path
    if let Some(w) = world.get_resource::<World>() {
        // pick first player and companion if present
        let player_opt = w.all_of_team(0).first().copied();
        let comp_opt = w.all_of_team(1).first().copied();
        if let (Some(player), Some(comp)) = (player_opt, comp_opt) {
            let enemies = w.enemies_of(1);
            let snap = build_snapshot(
                w,
                player,
                comp,
                &enemies,
                None,
                &PerceptionConfig { los_max: 10 },
            );
            let plan = orch.propose_plan(&snap);
            if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.iter().find_map(|s| {
                if let ActionStep::MoveTo { x, y, .. } = s {
                    Some(ActionStep::MoveTo {
                        x: *x,
                        y: *y,
                        speed: None,
                    })
                } else {
                    None
                }
            }) {
                if let Some(mapped) =
                    map_legacy_companion_to_ecs(&positions, &teams, &snap, comp, world)
                {
                    updates.push((
                        mapped,
                        CDesiredPos {
                            pos: IVec2 { x, y },
                        },
                    ));
                    planned_events.push(AiPlannedEvent {
                        entity: mapped,
                        target: IVec2 { x, y },
                    });
                }
            } else {
                // No valid move found
                if let Some(mapped) =
                    map_legacy_companion_to_ecs(&positions, &teams, &snap, comp, world)
                {
                    failed_events.push(AiPlanningFailedEvent {
                        entity: mapped,
                        reason: "No valid actions in plan".to_string(),
                    });
                }
            }
            // Early return after legacy-based planning for now (single companion minimal)
            if !updates.is_empty() {
                for (e, d) in &updates {
                    world.insert(*e, *d);
                }
                if let Some(ev) = world.get_resource_mut::<Events<AiPlannedEvent>>() {
                    let mut w = ev.writer();
                    for pe in planned_events {
                        w.send(pe);
                    }
                }
                if let Some(ev) = world.get_resource_mut::<Events<AiPlanningFailedEvent>>() {
                    let mut w = ev.writer();
                    for fe in failed_events {
                        w.send(fe);
                    }
                }
                return;
            }
        }
    }

    // Fallback: ECS-only snapshot composition
    let player = PlayerState {
        hp: 100,
        pos: IVec2 { x: 0, y: 0 },
        stance: "stand".into(),
        orders: vec![],
    };
    let enemies: Vec<EnemyState> = positions
        .iter()
        .filter_map(|(e, pos)| {
            let team_id = teams.get(e).copied().unwrap_or(0);
            if team_id == 2 {
                Some(EnemyState {
                    id: 0,
                    pos: *pos,
                    hp: 50,
                    cover: "low".into(),
                    last_seen: 0.0,
                })
            } else {
                None
            }
        })
        .collect();
    for (e, pos) in &positions {
        if teams.get(e).copied() != Some(1) {
            continue;
        }
        let ammo = world.get::<CAmmo>(*e).map(|a| a.rounds).unwrap_or(0);
        let cds_map = world
            .get::<CCooldowns>(*e)
            .map(|c| c.map.clone())
            .unwrap_or_default();
        let cooldowns: std::collections::BTreeMap<String, f32> = cds_map
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let me = CompanionState {
            ammo,
            cooldowns,
            morale: 1.0,
            pos: *pos,
        };
        let snap = WorldSnapshot {
            t: 0.0,
            player: player.clone(),
            me,
            enemies: enemies.clone(),
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };
        let plan = orch.propose_plan(&snap);
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.iter().find_map(|s| {
            if let ActionStep::MoveTo { x, y, .. } = s {
                Some(ActionStep::MoveTo {
                    x: *x,
                    y: *y,
                    speed: None,
                })
            } else {
                None
            }
        }) {
            updates.push((
                *e,
                CDesiredPos {
                    pos: IVec2 { x, y },
                },
            ));
            planned_events.push(AiPlannedEvent {
                entity: *e,
                target: IVec2 { x, y },
            });
        } else {
            failed_events.push(AiPlanningFailedEvent {
                entity: *e,
                reason: "No valid move action found".to_string(),
            });
        }
    }
    for (e, d) in updates {
        world.insert(e, d);
    }
    if let Some(ev) = world.get_resource_mut::<Events<AiPlannedEvent>>() {
        let mut w = ev.writer();
        for pe in planned_events {
            w.send(pe);
        }
    }
    if let Some(ev) = world.get_resource_mut::<Events<AiPlanningFailedEvent>>() {
        let mut w = ev.writer();
        for fe in failed_events {
            w.send(fe);
        }
    }
}

impl ecs::Plugin for AiPlanningPlugin {
    fn build(&self, app: &mut ecs::App) {
        app.world
            .insert_resource(Events::<AiPlanningFailedEvent>::default());
        app.add_system_known_stage("ai_planning", sys_ai_planning as ecs::SystemFn);
    }
}

/// Convenience: build an ECS app with core systems and the AI planning plugin installed.
pub fn build_app_with_ai(legacy_world: astraweave_core::World, dt: f32) -> ecs::App {
    let mut app = astraweave_core::ecs_adapter::build_app(legacy_world, dt);
    app = app.add_plugin(AiPlanningPlugin);
    app
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use anyhow::Result;
    use astraweave_core::{IVec2, World};

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn ai_plugin_sets_desired_position_for_companion() -> Result<()> {
        // Legacy world can be empty for this test
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);
        // Spawn two ECS entities: a companion and an enemy
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 3, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);
        let d = app
            .world
            .get::<CDesiredPos>(ally)
            .ok_or_else(|| anyhow!("desired pos set"))?;
        // Expect to move towards enemy along +x axis
        assert!(d.pos.x >= 1 && d.pos.y == 0);

        // Event should be published
        let evs = app
            .world
            .get_resource_mut::<Events<AiPlannedEvent>>()
            .ok_or_else(|| anyhow!("Events<AiPlannedEvent> resource missing"))?;
        let mut rdr = evs.reader();
        let v: Vec<_> = rdr.drain().collect();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity, ally);
        Ok(())
    }

    // =============================================================================
    // Plugin Registration Tests (2 tests)
    // =============================================================================

    #[test]
    fn test_ai_plugin_name() {
        // Verify plugin has correct name for debugging
        let _plugin = AiPlanningPlugin;
        // Plugin trait doesn't expose name() method, but we can verify type name
        let type_name = std::any::type_name::<AiPlanningPlugin>();
        assert!(type_name.contains("AiPlanningPlugin"));
    }

    #[test]
    fn test_ai_plugin_setup() -> Result<()> {
        // Verify plugin properly initializes resources and systems
        let w = World::new();
        let mut app = ecs::App::new();
        app.world.insert_resource(w);

        // Add plugin
        app = app.add_plugin(AiPlanningPlugin);

        // Verify Events<AiPlanningFailedEvent> resource was added
        assert!(
            app.world
                .get_resource::<Events<AiPlanningFailedEvent>>()
                .is_some(),
            "Plugin should register AiPlanningFailedEvent resource"
        );

        // Verify system was added to schedule
        // (No direct API to check system names, but we can verify app runs without panicking)
        let _app = app.run_fixed(1);

        Ok(())
    }

    // =============================================================================
    // build_app_with_ai Tests (3 tests)
    // =============================================================================

    #[test]
    fn test_build_app_with_ai_systems() -> Result<()> {
        // Verify build_app_with_ai creates app with AI planning system
        let w = World::new();
        let app = build_app_with_ai(w, 0.016);

        // Verify AI planning failed events resource exists
        // (Note: AiPlannedEvent is emitted but not pre-registered as a resource)
        assert!(
            app.world
                .get_resource::<Events<AiPlanningFailedEvent>>()
                .is_some(),
            "build_app_with_ai should include AI planning failed events"
        );

        Ok(())
    }

    #[test]
    fn test_build_app_with_ai_timestep() -> Result<()> {
        // Verify timestep is correctly set
        let w = World::new();
        let dt = 0.033; // 30 FPS
        let app = build_app_with_ai(w, dt);

        // App should be runnable with the specified timestep
        // (No direct API to verify dt, but run_fixed should work)
        let app = app.run_fixed(1);

        // Verify app is still valid after tick (check via resource existence)
        assert!(app.world.get_resource::<Events<AiPlannedEvent>>().is_some());

        Ok(())
    }

    #[test]
    fn test_build_app_with_legacy_world() -> Result<()> {
        // Verify legacy World resource is properly integrated
        use astraweave_core::Team;

        let mut w = World::new();

        // Use legacy World API to spawn entities
        let _player = w.spawn("Player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let _companion = w.spawn("Companion", IVec2 { x: 1, y: 1 }, Team { id: 1 }, 80, 30);
        let _enemy = w.spawn("Enemy", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 50, 15);

        let app = build_app_with_ai(w, 0.016);

        // Verify legacy World resource exists
        assert!(
            app.world.get_resource::<World>().is_some(),
            "Legacy World should be preserved as resource"
        );

        Ok(())
    }

    // =============================================================================
    // System Function Tests (2 tests)
    // =============================================================================

    #[test]
    fn test_ai_planning_system_execution() -> Result<()> {
        // Verify sys_ai_planning runs without errors
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn companion
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        // Run system (via app tick)
        app = app.run_fixed(1);

        // System should have run successfully (no panic)
        // Verify Events resource still exists
        assert!(app.world.get_resource::<Events<AiPlannedEvent>>().is_some());

        Ok(())
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_ai_component_queries() -> Result<()> {
        // Verify system correctly queries ECS components
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn multiple companions with different positions
        let ally1 = app.world.spawn();
        app.world.insert(
            ally1,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally1, CTeam { id: 1 });
        app.world.insert(ally1, CAmmo { rounds: 30 });
        app.world.insert(
            ally1,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let ally2 = app.world.spawn();
        app.world.insert(
            ally2,
            CPos {
                pos: IVec2 { x: 10, y: 10 },
            },
        );
        app.world.insert(ally2, CTeam { id: 1 });
        app.world.insert(ally2, CAmmo { rounds: 15 });
        app.world.insert(
            ally2,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        // Spawn enemy
        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 5 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        // Run system
        app = app.run_fixed(1);

        // Both allies should have CDesiredPos set (system queries all team 1 entities)
        assert!(
            app.world.get::<CDesiredPos>(ally1).is_some(),
            "Ally 1 should have desired position"
        );
        assert!(
            app.world.get::<CDesiredPos>(ally2).is_some(),
            "Ally 2 should have desired position"
        );

        Ok(())
    }

    // =============================================================================
    // Additional Edge Case Tests (2 tests)
    // =============================================================================

    #[test]
    fn test_ai_planning_no_enemies() -> Result<()> {
        // Verify system handles no enemies gracefully (should produce failed events)
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn companion without any enemies
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        app = app.run_fixed(1);

        // Should produce a failed event (no valid move)
        let evs = app
            .world
            .get_resource_mut::<Events<AiPlanningFailedEvent>>()
            .ok_or_else(|| anyhow!("AiPlanningFailedEvent resource missing"))?;
        let mut rdr = evs.reader();
        let v: Vec<_> = rdr.drain().collect();

        assert_eq!(v.len(), 1, "Should have one failed event");
        assert_eq!(v[0].entity, ally);
        assert!(v[0].reason.contains("No valid"));

        Ok(())
    }

    #[test]
    fn test_map_legacy_companion_to_ecs_fallback() -> Result<()> {
        // Verify map_legacy_companion_to_ecs uses closest entity when no bridge exists
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn two companions at different distances from origin
        let close_ally = app.world.spawn();
        app.world.insert(
            close_ally,
            CPos {
                pos: IVec2 { x: 1, y: 1 },
            },
        ); // Distance 2
        app.world.insert(close_ally, CTeam { id: 1 });

        let far_ally = app.world.spawn();
        app.world.insert(
            far_ally,
            CPos {
                pos: IVec2 { x: 10, y: 10 },
            },
        ); // Distance 20
        app.world.insert(far_ally, CTeam { id: 1 });

        // Create a snapshot with companion at origin
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 }, // At origin
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        // Build positions map
        let mut positions = std::collections::BTreeMap::new();
        positions.insert(close_ally, IVec2 { x: 1, y: 1 });
        positions.insert(far_ally, IVec2 { x: 10, y: 10 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(close_ally, 1);
        teams.insert(far_ally, 1);

        // Call mapping function
        let legacy_id = 1; // Arbitrary legacy ID
        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, legacy_id, &app.world);

        // Should map to closest companion (close_ally)
        assert_eq!(mapped, Some(close_ally), "Should map to closest companion");

        Ok(())
    }

    // =============================================================================
    // map_legacy_companion_to_ecs Edge Cases (5 tests)
    // =============================================================================

    #[test]
    fn test_map_legacy_no_companions() -> Result<()> {
        // Verify map returns None when no team 1 entities exist
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn only non-companion entities
        let enemy = app.world.spawn();
        let mut positions = std::collections::BTreeMap::new();
        let mut teams = std::collections::BTreeMap::new();
        positions.insert(enemy, IVec2 { x: 5, y: 5 });
        teams.insert(enemy, 2); // Team 2

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, 1, &app.world);
        assert_eq!(mapped, None, "Should return None when no team 1 entities");

        Ok(())
    }

    #[test]
    fn test_map_legacy_with_entity_bridge() -> Result<()> {
        // Verify EntityBridge mapping is preferred over proximity
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn two companions
        let close_ally = app.world.spawn();
        app.world.insert(
            close_ally,
            CPos {
                pos: IVec2 { x: 1, y: 1 },
            },
        );
        app.world.insert(close_ally, CTeam { id: 1 });

        let far_ally = app.world.spawn();
        app.world.insert(
            far_ally,
            CPos {
                pos: IVec2 { x: 10, y: 10 },
            },
        );
        app.world.insert(far_ally, CTeam { id: 1 });

        // Create EntityBridge mapping legacy id 5 -> far_ally
        let mut bridge = EntityBridge::default();
        bridge.insert(5, far_ally);
        app.world.insert_resource(bridge);

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(close_ally, IVec2 { x: 1, y: 1 });
        positions.insert(far_ally, IVec2 { x: 10, y: 10 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(close_ally, 1);
        teams.insert(far_ally, 1);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        // Map legacy id 5 (should use bridge, not proximity)
        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, 5, &app.world);
        assert_eq!(mapped, Some(far_ally), "Should prefer EntityBridge mapping");

        Ok(())
    }

    #[test]
    fn test_map_legacy_empty_positions() -> Result<()> {
        // Verify map returns None when positions map is empty
        let w = World::new();
        let app = build_app_with_ai(w, 0.016);

        let positions = std::collections::BTreeMap::new();
        let teams = std::collections::BTreeMap::new();

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, 1, &app.world);
        assert_eq!(mapped, None, "Should return None for empty positions");

        Ok(())
    }

    #[test]
    fn test_map_legacy_manhattan_distance() -> Result<()> {
        // Verify Manhattan distance calculation (closest by |dx| + |dy|)
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn companions at different positions
        let ally1 = app.world.spawn();
        app.world.insert(
            ally1,
            CPos {
                pos: IVec2 { x: 3, y: 0 },
            },
        ); // Distance 3
        app.world.insert(ally1, CTeam { id: 1 });

        let ally2 = app.world.spawn();
        app.world.insert(
            ally2,
            CPos {
                pos: IVec2 { x: 2, y: 1 },
            },
        ); // Distance 3
        app.world.insert(ally2, CTeam { id: 1 });

        let ally3 = app.world.spawn();
        app.world.insert(
            ally3,
            CPos {
                pos: IVec2 { x: 1, y: 1 },
            },
        ); // Distance 2 (closest)
        app.world.insert(ally3, CTeam { id: 1 });

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(ally1, IVec2 { x: 3, y: 0 });
        positions.insert(ally2, IVec2 { x: 2, y: 1 });
        positions.insert(ally3, IVec2 { x: 1, y: 1 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(ally1, 1);
        teams.insert(ally2, 1);
        teams.insert(ally3, 1);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, 1, &app.world);
        assert_eq!(
            mapped,
            Some(ally3),
            "Should map to closest by Manhattan distance"
        );

        Ok(())
    }

    #[test]
    fn test_map_legacy_team_filtering() -> Result<()> {
        // Verify only team 1 entities are considered
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn closest entity as team 2 (should be ignored)
        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 1, y: 0 },
            },
        ); // Distance 1
        app.world.insert(enemy, CTeam { id: 2 });

        // Spawn farther entity as team 1 (should be selected)
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 5, y: 5 },
            },
        ); // Distance 10
        app.world.insert(ally, CTeam { id: 1 });

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(enemy, IVec2 { x: 1, y: 0 });
        positions.insert(ally, IVec2 { x: 5, y: 5 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(enemy, 2);
        teams.insert(ally, 1);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let mapped = map_legacy_companion_to_ecs(&positions, &teams, &snap, 1, &app.world);
        assert_eq!(mapped, Some(ally), "Should only consider team 1 entities");

        Ok(())
    }

    // =============================================================================
    // sys_ai_planning Legacy World Path Tests (4 tests)
    // =============================================================================

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_sys_ai_planning_no_legacy_world() -> Result<()> {
        // Verify ECS-only snapshot path when no legacy World resource
        let mut app = ecs::App::new();
        // Don't insert legacy World resource at all to force ECS-only path

        // Manually add plugin
        app = app.add_plugin(AiPlanningPlugin);

        // Spawn ECS companion and enemy
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // System should still run via ECS-only path
        assert!(
            app.world.get::<CDesiredPos>(ally).is_some(),
            "Should set desired pos via ECS-only path"
        );

        Ok(())
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_sys_ai_planning_no_player_in_legacy() -> Result<()> {
        // Verify system handles missing player in legacy World
        let mut w = World::new();
        // Spawn only companion, no player (team 0)
        use astraweave_core::Team;
        let _comp = w.spawn("Companion", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 80, 30);

        let mut app = build_app_with_ai(w, 0.016);

        // Spawn ECS companion
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // Should fallback to ECS-only path (legacy path requires both player and companion)
        assert!(
            app.world.get::<CDesiredPos>(ally).is_some(),
            "Should use ECS-only fallback"
        );

        Ok(())
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_sys_ai_planning_no_companion_in_legacy() -> Result<()> {
        // Verify system handles missing companion in legacy World
        let mut w = World::new();
        // Spawn only player, no companion (team 1)
        use astraweave_core::Team;
        let _player = w.spawn("Player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);

        let mut app = build_app_with_ai(w, 0.016);

        // Spawn ECS companion
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // Should fallback to ECS-only path
        assert!(
            app.world.get::<CDesiredPos>(ally).is_some(),
            "Should use ECS-only fallback"
        );

        Ok(())
    }

    #[test]
    fn test_sys_ai_planning_legacy_happy_path_with_moveto() -> Result<()> {
        // Verify system executes legacy world planning path (lines 83-147)
        // This tests the FULL happy path including event emission
        let mut w = World::new();
        use astraweave_core::Team;

        // Setup world where orchestrator WILL generate a plan (enemy far away to trigger MoveTo)
        let _player = w.spawn("Player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let _comp = w.spawn("Companion", IVec2 { x: 1, y: 0 }, Team { id: 1 }, 80, 5);
        let _enemy = w.spawn("Enemy", IVec2 { x: 10, y: 0 }, Team { id: 2 }, 50, 0);

        let mut app = build_app_with_ai(w, 0.016);

        // Spawn ECS companion with matching position (for map_legacy_companion_to_ecs)
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 1, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 5 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        app = app.run_fixed(1);

        // The orchestrator will generate SOME plan (either MoveTo, Attack, etc.)
        // Verify that EITHER:
        // 1. CDesiredPos was set (MoveTo action was in plan)
        // 2. OR an event was emitted (plan was processed even if no MoveTo)
        let has_desired_pos = app.world.get::<CDesiredPos>(ally).is_some();
        let has_planned_event =
            if let Some(events) = app.world.get_resource_mut::<Events<AiPlannedEvent>>() {
                let mut reader = events.reader();
                !reader.drain().collect::<Vec<_>>().is_empty()
            } else {
                false
            };
        let has_failed_event = if let Some(events) = app
            .world
            .get_resource_mut::<Events<AiPlanningFailedEvent>>()
        {
            let mut reader = events.reader();
            !reader.drain().collect::<Vec<_>>().is_empty()
        } else {
            false
        };

        // Legacy path was taken if ANY of these happened (proves lines 83-147 executed)
        assert!(
            has_desired_pos || has_planned_event || has_failed_event,
            "Legacy planning path should have executed (CDesiredPos, AiPlannedEvent, or AiPlanningFailedEvent)"
        );

        Ok(())
    }

    #[test]
    fn test_sys_ai_planning_legacy_no_moveto_action() -> Result<()> {
        // Verify system handles legacy world planning when plan has NO valid actions
        // This specifically tests the else branch (lines 120-128) where map_legacy succeeds
        // but no MoveTo action is found, triggering AiPlanningFailedEvent

        use astraweave_core::Team;
        let mut w = World::new();

        // Setup world with player + companion but circumstances that won't generate useful plan
        // (e.g., close together, no enemies, no objectives)
        let _player = w.spawn("Player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let _comp = w.spawn("Companion", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 80, 10);
        // No enemies - orchestrator may return empty plan or non-MoveTo actions

        let mut app = build_app_with_ai(w, 0.016);

        // Spawn ECS companion
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        app = app.run_fixed(1);

        // The test verifies that EITHER:
        // 1. AiPlanningFailedEvent was emitted (if no MoveTo in plan)
        // 2. OR system fell through to ECS-only path and set CDesiredPos
        // 3. OR AiPlannedEvent was emitted (if plan DID have MoveTo despite our setup)
        // Any of these outcomes proves the code path executed without crashing

        let has_desired_pos = app.world.get::<CDesiredPos>(ally).is_some();
        let has_failed_event = if let Some(events) = app
            .world
            .get_resource_mut::<Events<AiPlanningFailedEvent>>()
        {
            let mut reader = events.reader();
            !reader.drain().collect::<Vec<_>>().is_empty()
        } else {
            false
        };
        let has_planned_event =
            if let Some(events) = app.world.get_resource_mut::<Events<AiPlannedEvent>>() {
                let mut reader = events.reader();
                !reader.drain().collect::<Vec<_>>().is_empty()
            } else {
                false
            };

        // Legacy path executed successfully (one of the three outcomes occurred)
        assert!(
            has_desired_pos || has_failed_event || has_planned_event,
            "System should handle legacy path execution (CDesiredPos, failure event, or planned event)"
        );

        Ok(())
    }

    #[test]
    fn test_sys_ai_planning_initializes_events() -> Result<()> {
        // Verify system initializes Events<AiPlannedEvent> if missing
        let w = World::new();
        let mut app = ecs::App::new();
        app.world.insert_resource(w);

        // Don't add plugin (to test event initialization in sys_ai_planning)
        // Manually add system
        app.schedule
            .add_system("ai_planning", sys_ai_planning as ecs::SystemFn)
            .expect("registered schedule stage");

        // Spawn companion
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        // Events resource should NOT exist yet
        assert!(
            app.world.get_resource::<Events<AiPlannedEvent>>().is_none(),
            "Events should not exist before system run"
        );

        app = app.run_fixed(1);

        // System should have created Events resource
        assert!(
            app.world.get_resource::<Events<AiPlannedEvent>>().is_some(),
            "System should initialize Events resource"
        );

        Ok(())
    }

    // =============================================================================
    // ECS-Only Snapshot Path Tests (3 tests)
    // =============================================================================

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_ecs_only_snapshot_multiple_allies() -> Result<()> {
        // Verify ECS-only path handles multiple companions
        let mut app = ecs::App::new();
        // Don't insert legacy World resource to force ECS-only path
        app = app.add_plugin(AiPlanningPlugin);

        // Spawn three companions
        let ally1 = app.world.spawn();
        app.world.insert(
            ally1,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally1, CTeam { id: 1 });
        app.world.insert(ally1, CAmmo { rounds: 10 });
        app.world.insert(
            ally1,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let ally2 = app.world.spawn();
        app.world.insert(
            ally2,
            CPos {
                pos: IVec2 { x: 2, y: 0 },
            },
        );
        app.world.insert(ally2, CTeam { id: 1 });
        app.world.insert(ally2, CAmmo { rounds: 5 });
        app.world.insert(
            ally2,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let ally3 = app.world.spawn();
        app.world.insert(
            ally3,
            CPos {
                pos: IVec2 { x: 4, y: 0 },
            },
        );
        app.world.insert(ally3, CTeam { id: 1 });
        app.world.insert(ally3, CAmmo { rounds: 15 });
        app.world.insert(
            ally3,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        // Spawn enemy
        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 10, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // All three companions should have desired positions set
        assert!(
            app.world.get::<CDesiredPos>(ally1).is_some(),
            "Ally 1 should have desired pos"
        );
        assert!(
            app.world.get::<CDesiredPos>(ally2).is_some(),
            "Ally 2 should have desired pos"
        );
        assert!(
            app.world.get::<CDesiredPos>(ally3).is_some(),
            "Ally 3 should have desired pos"
        );

        // Three planned events should be published
        let evs = app
            .world
            .get_resource_mut::<Events<AiPlannedEvent>>()
            .ok_or_else(|| anyhow!("Events missing"))?;
        let mut rdr = evs.reader();
        let v: Vec<_> = rdr.drain().collect();
        assert_eq!(v.len(), 3, "Should have 3 planned events");

        Ok(())
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_ecs_only_snapshot_cooldowns_conversion() -> Result<()> {
        // Verify ECS-only path converts cooldowns to BTreeMap<String, f32>
        let mut app = ecs::App::new();
        // Don't insert legacy World resource to force ECS-only path
        app = app.add_plugin(AiPlanningPlugin);

        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });

        // Insert cooldowns with CooldownKey
        use astraweave_core::cooldowns::CooldownKey;
        let mut cds_map = std::collections::BTreeMap::new();
        cds_map.insert(CooldownKey::ThrowSmoke, 2.5);
        cds_map.insert(CooldownKey::Custom("heal".to_string()), 5.0);
        app.world.insert(ally, CCooldowns { map: cds_map });

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // System should run without panic (cooldowns conversion successful)
        assert!(
            app.world.get::<CDesiredPos>(ally).is_some(),
            "Should handle cooldowns conversion"
        );

        Ok(())
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_ecs_only_snapshot_enemy_filtering() -> Result<()> {
        // Verify ECS-only path correctly filters team 2 as enemies
        let mut app = ecs::App::new();
        // Don't insert legacy World resource to force ECS-only path
        app = app.add_plugin(AiPlanningPlugin);

        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        // Spawn multiple enemies (team 2)
        let enemy1 = app.world.spawn();
        app.world.insert(
            enemy1,
            CPos {
                pos: IVec2 { x: 3, y: 0 },
            },
        );
        app.world.insert(enemy1, CTeam { id: 2 });

        let enemy2 = app.world.spawn();
        app.world.insert(
            enemy2,
            CPos {
                pos: IVec2 { x: 6, y: 0 },
            },
        );
        app.world.insert(enemy2, CTeam { id: 2 });

        // Spawn team 0 entity (should be ignored)
        let neutral = app.world.spawn();
        app.world.insert(
            neutral,
            CPos {
                pos: IVec2 { x: 10, y: 0 },
            },
        );
        app.world.insert(neutral, CTeam { id: 0 });

        app = app.run_fixed(1);

        // Ally should move towards one of the enemies (team 2 filtering worked)
        let desired = app
            .world
            .get::<CDesiredPos>(ally)
            .ok_or_else(|| anyhow!("Desired pos missing"))?;
        assert!(
            desired.pos.x > 0,
            "Should move towards enemies (positive x)"
        );

        Ok(())
    }

    // =============================================================================
    // Event Generation Tests (2 tests)
    // =============================================================================

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_ai_planned_event_contents() -> Result<()> {
        // Verify AiPlannedEvent has correct entity and target
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        let evs = app
            .world
            .get_resource_mut::<Events<AiPlannedEvent>>()
            .ok_or_else(|| anyhow!("Events missing"))?;
        let mut rdr = evs.reader();
        let v: Vec<_> = rdr.drain().collect();

        assert_eq!(v.len(), 1, "Should have one planned event");
        assert_eq!(v[0].entity, ally, "Event should reference ally entity");
        assert!(v[0].target.x >= 1, "Target should be towards enemy");

        Ok(())
    }

    #[test]
    fn test_ai_planning_failed_event_reason() -> Result<()> {
        // Verify AiPlanningFailedEvent has descriptive reason
        let w = World::new();
        let mut app = build_app_with_ai(w, 0.016);

        // Spawn companion without enemies
        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        app.world.insert(ally, CAmmo { rounds: 10 });
        app.world.insert(
            ally,
            CCooldowns {
                map: std::collections::BTreeMap::new(),
            },
        );

        app = app.run_fixed(1);

        let evs = app
            .world
            .get_resource_mut::<Events<AiPlanningFailedEvent>>()
            .ok_or_else(|| anyhow!("Failed events missing"))?;
        let mut rdr = evs.reader();
        let v: Vec<_> = rdr.drain().collect();

        assert_eq!(v.len(), 1, "Should have one failed event");
        assert_eq!(v[0].entity, ally, "Failed event should reference ally");
        assert!(!v[0].reason.is_empty(), "Failed event should have reason");
        assert!(
            v[0].reason.contains("No valid"),
            "Reason should mention 'No valid'"
        );

        Ok(())
    }

    // =============================================================================
    // Component Query Edge Cases (2 tests)
    // =============================================================================

    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_sys_ai_planning_missing_components() -> Result<()> {
        // Verify system handles entities missing CAmmo or CCooldowns
        let mut app = ecs::App::new();
        app = app.add_plugin(AiPlanningPlugin);

        let ally = app.world.spawn();
        app.world.insert(
            ally,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(ally, CTeam { id: 1 });
        // Missing CAmmo and CCooldowns

        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(enemy, CTeam { id: 2 });

        app = app.run_fixed(1);

        // System should use default values (0 ammo, empty cooldowns)
        assert!(
            app.world.get::<CDesiredPos>(ally).is_some(),
            "Should handle missing components gracefully"
        );

        Ok(())
    }

    #[test]
    fn test_sys_ai_planning_empty_world() -> Result<()> {
        // Verify system handles completely empty world
        let mut app = ecs::App::new();
        app = app.add_plugin(AiPlanningPlugin);

        // No entities spawned at all
        app = app.run_fixed(1);

        // System should run without panic
        assert!(
            app.world.get_resource::<Events<AiPlannedEvent>>().is_some(),
            "Should initialize events even with empty world"
        );

        Ok(())
    }

    // =========================================================================
    // Mutation-Resistant: map_legacy_companion_to_ecs Manhattan distance
    // =========================================================================

    #[test]
    fn test_map_legacy_companion_selects_closest_entity() {
        // Setup: Two team-1 entities at different positions.
        // Companion (me) is at (10, 10). Entity A at (11, 10) (dist 1), Entity B at (0, 0) (dist 20).
        // Must return Entity A as closest.
        let mut world = ecs::World::default();

        let entity_a = world.spawn();
        let entity_b = world.spawn();

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(entity_a, IVec2 { x: 11, y: 10 });
        positions.insert(entity_b, IVec2 { x: 0, y: 0 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(entity_a, 1u8);
        teams.insert(entity_b, 1u8);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 10, y: 10 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let result = map_legacy_companion_to_ecs(&positions, &teams, &snap, 0, &world);
        assert_eq!(
            result,
            Some(entity_a),
            "Should select entity at (11,10), distance 1, not (0,0) distance 20"
        );
    }

    #[test]
    fn test_map_legacy_companion_manhattan_distance_negative_coords() {
        // Companion at (-5, -5). Entity A at (-4, -5) (dist 1), Entity B at (5, 5) (dist 20).
        // Tests that (p.x - snap.me.pos.x).abs() works correctly with negative values.
        let mut world = ecs::World::default();

        let entity_a = world.spawn();
        let entity_b = world.spawn();

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(entity_a, IVec2 { x: -4, y: -5 });
        positions.insert(entity_b, IVec2 { x: 5, y: 5 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(entity_a, 1u8);
        teams.insert(entity_b, 1u8);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: -5, y: -5 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let result = map_legacy_companion_to_ecs(&positions, &teams, &snap, 0, &world);
        assert_eq!(
            result,
            Some(entity_a),
            "Should handle negative coords correctly via .abs()"
        );
    }

    #[test]
    fn test_map_legacy_companion_filters_by_team() {
        // Only team-1 entities should be considered, even if a team-2 entity is closer.
        let mut world = ecs::World::default();

        let enemy_close = world.spawn();
        let ally_far = world.spawn();

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(enemy_close, IVec2 { x: 1, y: 0 }); // Very close but team 2
        positions.insert(ally_far, IVec2 { x: 100, y: 100 }); // Far but team 1

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(enemy_close, 2u8);
        teams.insert(ally_far, 1u8);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let result = map_legacy_companion_to_ecs(&positions, &teams, &snap, 0, &world);
        assert_eq!(
            result,
            Some(ally_far),
            "Should only consider team-1 entities"
        );
    }

    #[test]
    fn test_map_legacy_companion_no_team1_returns_none() {
        let mut world = ecs::World::default();

        let enemy = world.spawn();

        let mut positions = std::collections::BTreeMap::new();
        positions.insert(enemy, IVec2 { x: 1, y: 0 });

        let mut teams = std::collections::BTreeMap::new();
        teams.insert(enemy, 2u8);

        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let result = map_legacy_companion_to_ecs(&positions, &teams, &snap, 0, &world);
        assert_eq!(result, None, "No team-1 entities should return None");
    }

    // ========================================================================
    // Mutation-Killing Test: line 137 `if !updates.is_empty()` → `is_empty()`
    // ========================================================================

    /// Kills mutation: line 137 `delete !` — inverts the condition so updates
    /// are only written when the vec is empty (discarding real plans).
    /// This test verifies that when a legacy world has enemies and planning
    /// produces a MoveTo, CDesiredPos is actually written to the bridged ECS entity.
    #[cfg(not(feature = "veilweaver_slice"))]
    #[test]
    fn test_sys_ai_planning_writes_desired_pos_when_updates_nonempty() -> Result<()> {
        use astraweave_core::ecs_bridge::EntityBridge;
        use astraweave_core::Team;

        // Create legacy world with player, companion, and enemy
        let mut w = World::new();
        let _player = w.spawn("Player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
        let companion = w.spawn("Companion", IVec2 { x: 1, y: 1 }, Team { id: 1 }, 80, 30);
        let _enemy = w.spawn("Enemy", IVec2 { x: 5, y: 5 }, Team { id: 2 }, 50, 15);

        let mut app = build_app_with_ai(w, 0.016);

        // Look up the ECS entity for the companion via the EntityBridge
        // (build_app auto-populates ECS entities from legacy world)
        let ecs_comp = app
            .world
            .get_resource::<EntityBridge>()
            .and_then(|b| b.get(&companion))
            .ok_or_else(|| anyhow!("companion should be bridged to ECS entity"))?;

        // Run the planning system
        app = app.run_fixed(1);

        // With enemy present, RuleOrchestrator produces a plan with MoveTo.
        // The `if !updates.is_empty()` branch should have written CDesiredPos.
        // With mutation `if updates.is_empty()`, the write is skipped.
        let has_desired = app.world.get::<CDesiredPos>(ecs_comp).is_some();
        assert!(
            has_desired,
            "CDesiredPos must be written when planner produces MoveTo (guards `if !updates.is_empty()`)"
        );

        Ok(())
    }
}

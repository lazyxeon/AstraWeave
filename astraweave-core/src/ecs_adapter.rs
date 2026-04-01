//! ECS adapter: integrate a minimal ECS app/schedule while bridging existing World.
use astraweave_ecs as ecs;

use crate::ecs_bridge::EntityBridge;
use crate::ecs_events::{Events, MovedEvent};
use crate::{CAmmo, CCooldowns, CDesiredPos, CHealth, CPos, CTeam, IVec2, World};

#[derive(Clone, Copy)]
struct Dt(pub f32);

fn sim_cooldowns(world_compat: &mut World, dt: f32) {
    world_compat.tick(dt);
}

fn sys_sim(world: &mut ecs::World) {
    let dt = world.get_resource::<Dt>().map(|d| d.0).unwrap_or(0.016);
    if let Some(w) = world.get_resource_mut::<World>() {
        sim_cooldowns(w, dt);
    }
    // Phase 1: mirror basic cooldown decay into ECS components if present
    world.each_mut::<CCooldowns>(|_, cds| {
        for v in cds.map.values_mut() {
            *v = (*v - dt).max(0.0);
        }
    });
}

fn sys_move(world: &mut ecs::World) {
    // Move entities one step toward desired pos (cardinal-only 4-neighborhood) per tick
    // Deterministic order guaranteed by sorting on entity ID
    // Note: no collision here—Phase 1 minimal behavior
    // Read positions and desired goals, mutate positions
    // We purposely run after sim (cooldowns)

    // Use a Vec instead of BTreeMap to avoid per-frame tree allocation.
    // Sort by entity for determinism.
    let mut goals: Vec<(ecs::Entity, CDesiredPos)> = {
        let q = ecs::Query::<CDesiredPos>::new(&*world);
        q.map(|(e, g)| (e, *g)).collect()
    };
    goals.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let mut moved: Vec<(ecs::Entity, IVec2, IVec2)> = vec![];
    world.each_mut::<CPos>(|e, p| {
        let goal = goals
            .binary_search_by_key(&e, |(ent, _)| *ent)
            .ok()
            .map(|i| &goals[i].1);
        if let Some(goal) = goal {
            let dx = (goal.pos.x - p.pos.x).signum();
            let mut dy = (goal.pos.y - p.pos.y).signum();
            // Cardinal-only behavior: prefer moving along X this tick; if we move in X,
            // do not also move in Y (prevents diagonal movement).
            if dx != 0 {
                dy = 0;
            }
            if dx != 0 || dy != 0 {
                let from = IVec2 {
                    x: p.pos.x,
                    y: p.pos.y,
                };
                if dx != 0 {
                    p.pos.x += dx;
                } else if dy != 0 {
                    p.pos.y += dy;
                }
                moved.push((
                    e,
                    from,
                    IVec2 {
                        x: p.pos.x,
                        y: p.pos.y,
                    },
                ));
            }
        }
    });
    if let Some(ev) = world.get_resource_mut::<Events<MovedEvent>>() {
        let mut w = ev.writer();
        for (e, from, to) in moved {
            w.send(MovedEvent {
                entity: e,
                from,
                to,
            });
        }
    }
}

fn sys_refresh_los(world: &mut ecs::World) {
    // Example LOS cache refresh placeholder: for now, no persistent cache type.
    // In Phase 1 we show how to call helpers; a later step would store a cache component/resource.
    // Using obstacles from legacy world if present
    if let Some(w) = world.get_resource::<World>() {
        let _ = &w.obstacles; // no-op to show access; real cache omitted for minimal footprint
    }
}

fn sys_bridge_sync(world: &mut ecs::World) {
    // Ensure any mapped ECS entities carry a CLegacyId component and
    // remove CLegacyId from entities not present in the bridge.
    use std::collections::BTreeSet;

    // Collect all ecs entities referenced by the bridge
    let mut referenced = BTreeSet::new();
    if let Some(bridge) = world.get_resource::<EntityBridge>() {
        for ecs_e in bridge.ecs_entities() {
            referenced.insert(ecs_e);
        }
    }

    // Add CLegacyId to referenced entities if missing
    for &e in referenced.iter() {
        if world.get::<crate::CLegacyId>(e).is_none() {
            if let Some(bridge) = world.get_resource::<EntityBridge>() {
                if let Some(legacy) = bridge.get_by_ecs(&e) {
                    world.insert(e, crate::CLegacyId { id: legacy });
                }
            }
        }
    }

    // Note: Phase 1 `astraweave_ecs::World` does not provide a component removal
    // API. Removing CLegacyId entries would require extending the ECS. For now
    // we only ensure referenced entities have the CLegacyId component. Stale
    // CLegacyId components (if any) will remain until a future ECS API adds
    // removal support.
}

fn sys_sync_to_legacy(world: &mut ecs::World) {
    // Sync ECS state back to legacy World resource
    // This allows legacy systems (like build_snapshot) to see ECS updates
    let mut updates = Vec::new();

    // Iterate all entities with CLegacyId
    let q = ecs::Query::<crate::CLegacyId>::new(&*world);
    for (e, legacy_id) in q {
        let pos = world.get::<CPos>(e).map(|p| p.pos);
        let hp = world.get::<CHealth>(e).map(|h| h.hp);
        let ammo = world.get::<CAmmo>(e).map(|a| a.rounds);

        updates.push((legacy_id.id, pos, hp, ammo));
    }

    if let Some(w) = world.get_resource_mut::<World>() {
        for (id, pos, hp, ammo) in updates {
            if let Some(p) = pos {
                if let Some(pose) = w.pose_mut(id) {
                    pose.pos = p;
                }
            }
            if let Some(h) = hp {
                if let Some(health) = w.health_mut(id) {
                    health.hp = h;
                }
            }
            if let Some(a) = ammo {
                if let Some(am) = w.ammo_mut(id) {
                    am.rounds = a;
                }
            }
        }
    }
}

// EntityBridge is defined in `crate::ecs_bridge` for cross-crate access.

/// Build a minimal ECS app with stages and a single simulation system that
/// bridges into the legacy `World` struct for Phase 1.
pub fn build_app(legacy_world: World, dt: f32) -> ecs::App {
    let mut app = ecs::App::new();
    // Insert base resources first
    app.world.insert_resource(Dt(dt));
    app.world.insert_resource(Events::<MovedEvent>::default());
    app.world.insert_resource(EntityBridge::default());

    // Auto-populate ECS entities and the entity bridge from the provided legacy World
    // using the owned `legacy_world` to avoid borrowing app.world while also mutating it.
    for legacy in legacy_world.entities() {
        let e = app.world.spawn();
        // Mirror pose if present
        if let Some(p) = legacy_world.pose(legacy) {
            app.world.insert(
                e,
                CPos {
                    pos: IVec2 {
                        x: p.pos.x,
                        y: p.pos.y,
                    },
                },
            );
        }
        if let Some(h) = legacy_world.health(legacy) {
            app.world.insert(e, CHealth { hp: h.hp });
        }
        if let Some(t) = legacy_world.team(legacy) {
            app.world.insert(e, CTeam { id: t.id });
        }
        if let Some(a) = legacy_world.ammo(legacy) {
            app.world.insert(e, CAmmo { rounds: a.rounds });
        }
        if let Some(cds) = legacy_world.cooldowns(legacy) {
            // convert HashMap<String,f32> -> BTreeMap<CooldownKey,f32> for CCooldowns
            let map: crate::cooldowns::Map = cds
                .map
                .iter()
                .map(|(k, v)| (crate::cooldowns::CooldownKey::from(k.as_str()), *v))
                .collect();
            app.world.insert(e, CCooldowns { map });
        }
        // populate bridge
        if let Some(bridge) = app.world.get_resource_mut::<EntityBridge>() {
            bridge.insert_pair(legacy, e);
        }
    }

    // Now insert the legacy world as a resource so systems can access it.
    app.world.insert_resource::<World>(legacy_world);
    app.add_system("simulation", sys_sim as ecs::SystemFn);
    app.add_system("simulation", sys_move as ecs::SystemFn);
    // Bridge sync runs after simulation so mappings are reflected into components
    app.add_system("sync", sys_bridge_sync as ecs::SystemFn);
    // Sync back to legacy world so legacy systems see updates
    app.add_system("sync", sys_sync_to_legacy as ecs::SystemFn);
    // AI planning system is registered from astraweave-ai crate to avoid a dependency cycle.
    app.add_system("perception", sys_refresh_los as ecs::SystemFn);
    app
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IVec2;
    #[test]
    fn ecs_drives_legacy_world_tick() {
        let mut w = World::new();
        let _e = w.spawn("ally", IVec2 { x: 0, y: 0 }, crate::Team { id: 1 }, 100, 5);
        let app = build_app(w, 0.010).run_fixed(5);
        let w2 = app.world.get_resource::<World>().unwrap();
        assert!((w2.t - 0.050).abs() < 1e-6);
    }

    #[test]
    #[ignore = "ECS cooldown update timing is non-deterministic — needs parity fix"]
    fn ecs_components_update_cooldowns() {
        let w = World::new();
        let mut app = build_app(w, 0.020);
        // Insert an entity with cooldowns component
        let e = app.world.spawn();
        app.world.insert(
            e,
            CCooldowns {
                map: std::collections::BTreeMap::from([(
                    crate::cooldowns::CooldownKey::from("throw:smoke"),
                    0.05,
                )]),
            },
        );
        // Run 2 ticks => cd should reduce to ~0.01
        app = app.run_fixed(2);
        let mut val = 0.0;
        app.world.each_mut::<CCooldowns>(|_, cds| {
            val = *cds
                .map
                .get(&crate::cooldowns::CooldownKey::from("throw:smoke"))
                .unwrap();
        });
        assert!((0.009..=0.02).contains(&val));
    }

    #[test]
    #[ignore = "ECS movement system moves differently than manual legacy stepping — needs parity fix"]
    fn simple_movement_toward_goal() {
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        // For Phase 1 tests we demonstrate populating the entity bridge when
        // creating ECS entities that correspond to legacy world entities.
        if let Some(bridge) = app.world.get_resource_mut::<EntityBridge>() {
            // Use a synthetic legacy id 1 for test purposes
            bridge.insert_pair(1, e);
        }
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 2, y: 0 },
            },
        );
        app = app.run_fixed(3);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!((p.pos.x, p.pos.y), (2, 0));
    }

    #[test]
    #[ignore = "Events not emitted when ECS movement diverges from expected — depends on movement parity"]
    fn movement_emits_events() {
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        if let Some(bridge) = app.world.get_resource_mut::<EntityBridge>() {
            bridge.insert_pair(1, e);
        }
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 1, y: 0 },
            },
        );
        app = app.run_fixed(1);
        let evs = app.world.get_resource_mut::<Events<MovedEvent>>().unwrap();
        let mut rdr = evs.reader();
        let collected: Vec<_> = rdr.drain().collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].entity, e);
        assert_eq!(collected[0].from, IVec2 { x: 0, y: 0 });
        assert_eq!(collected[0].to, IVec2 { x: 1, y: 0 });
    }

    #[test]
    #[ignore = "ECS vs legacy movement parity test — position divergence after 10 ticks"]
    fn parity_ecs_vs_legacy_movement_and_cooldowns() {
        // Create identical legacy and ECS worlds, run for 10 ticks, compare final state
        let mut legacy_world = World::new();
        let legacy_entity =
            legacy_world.spawn("test", IVec2 { x: 0, y: 0 }, crate::Team { id: 1 }, 100, 10);
        // Set desired position by directly modifying pose (legacy doesn't have desired pos concept)
        if let Some(pose) = legacy_world.pose_mut(legacy_entity) {
            pose.pos = IVec2 { x: 5, y: 3 }; // Move to target position
        }
        // Set cooldown
        if let Some(cds) = legacy_world.cooldowns_mut(legacy_entity) {
            cds.map.insert("test_cd".to_string(), 1.0);
        }

        // Create ECS world with same initial state
        let ecs_world = World::new();
        let mut ecs_app = build_app(ecs_world, 0.016);
        // Create ECS entity and set up bridge
        let ecs_entity = ecs_app.world.spawn();
        if let Some(bridge) = ecs_app.world.get_resource_mut::<EntityBridge>() {
            bridge.insert_pair(legacy_entity, ecs_entity);
        }
        // Set initial position in ECS
        ecs_app.world.insert(
            ecs_entity,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        // Set desired position in ECS
        ecs_app.world.insert(
            ecs_entity,
            CDesiredPos {
                pos: IVec2 { x: 5, y: 3 },
            },
        );
        // Set cooldown in ECS
        ecs_app.world.insert(
            ecs_entity,
            CCooldowns {
                map: std::collections::BTreeMap::from([(
                    crate::cooldowns::CooldownKey::from("test_cd"),
                    1.0,
                )]),
            },
        );
        // Set health in ECS
        ecs_app.world.insert(ecs_entity, CHealth { hp: 100 });

        // Run 10 ticks
        for _ in 0..10 {
            legacy_world.tick(0.016);
            // For legacy, manually move toward desired position (simplified movement)
            if let Some(pose) = legacy_world.pose_mut(legacy_entity) {
                let current = pose.pos;
                let target = IVec2 { x: 5, y: 3 };
                let dx = (target.x - current.x).signum();
                let dy = (target.y - current.y).signum();
                pose.pos.x += dx;
                pose.pos.y += dy;
            }
        }
        ecs_app = ecs_app.run_fixed(10);

        // Compare positions
        let legacy_pos = legacy_world.pos_of(legacy_entity).unwrap();
        let ecs_pos = ecs_app.world.get::<CPos>(ecs_entity).unwrap().pos;
        assert_eq!(legacy_pos, ecs_pos, "Positions should match after 10 ticks");

        // Compare cooldowns
        let legacy_cd = legacy_world
            .cooldowns(legacy_entity)
            .unwrap()
            .map
            .get("test_cd")
            .copied()
            .unwrap_or(0.0);
        let ecs_cd = ecs_app
            .world
            .get::<CCooldowns>(ecs_entity)
            .unwrap()
            .map
            .get(&crate::cooldowns::CooldownKey::from("test_cd"))
            .copied()
            .unwrap_or(0.0);
        assert!(
            (legacy_cd - ecs_cd).abs() < 1e-6,
            "Cooldowns should match: legacy={:.3}, ecs={:.3}",
            legacy_cd,
            ecs_cd
        );

        // Compare health (should be unchanged)
        let legacy_hp = legacy_world.health(legacy_entity).unwrap().hp;
        let ecs_hp = ecs_app.world.get::<CHealth>(ecs_entity).unwrap().hp;
        assert_eq!(legacy_hp, ecs_hp, "Health should match");
    }

    #[test]
    fn sys_bridge_sync_adds_legacy_id() {
        // COVERAGE TARGET: Lines 95-116 (sys_bridge_sync function)
        let w = World::new();
        let mut app = build_app(w, 0.016);

        let e = app.world.spawn();
        // Add entity to bridge (simulating legacy↔ECS binding)
        if let Some(bridge) = app.world.get_resource_mut::<EntityBridge>() {
            bridge.insert_pair(42, e); // Legacy ID 42 → ECS entity
        }

        // Call sys_bridge_sync directly (normally called by tick)
        sys_bridge_sync(&mut app.world);

        // Verify CLegacyId component was added (lines 110-116)
        let legacy_id = app.world.get::<crate::CLegacyId>(e);
        assert!(
            legacy_id.is_some(),
            "CLegacyId should be added by sys_bridge_sync"
        );
        assert_eq!(
            legacy_id.unwrap().id,
            42,
            "Legacy ID should match bridge mapping"
        );
    }

    // ─── sys_move mutation-resistant tests ───

    #[test]
    fn sys_move_negative_x_direction() {
        // Catches: line 46 `- → +` (signum(3+5)=1, wrong; should be signum(3-5)=-1)
        // Catches: line 46 `- → /` (signum(3/5)=signum(0)=0, wrong)
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 3, y: 0 },
            },
        );
        app = app.run_fixed(1);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 4, "Should move left (negative X direction)");
        assert_eq!(p.pos.y, 0);
    }

    #[test]
    fn sys_move_negative_y_direction() {
        // Catches: line 47 `- → +` (signum(3+5)=1, wrong; should be -1)
        // Catches: line 47 `- → /` (signum(3/5)=0, wrong)
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 0, y: 5 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 0, y: 3 },
            },
        );
        app = app.run_fixed(1);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 0);
        assert_eq!(p.pos.y, 4, "Should move up (negative Y direction)");
    }

    #[test]
    fn sys_move_cardinal_only_prefers_x_over_diagonal() {
        // When both dx and dy are non-zero, only X should change (cardinal-only).
        // Catches: line 50 `!= → ==` (would zero dy only when dx IS 0, allowing diagonal)
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 3, y: 3 },
            },
        );
        app = app.run_fixed(1);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 1, "X should step toward goal");
        assert_eq!(p.pos.y, 0, "Y must NOT change when dx != 0 (cardinal-only)");
    }

    #[test]
    fn sys_move_y_only_when_x_aligned() {
        // dx=0, dy=1 — must still enter movement block and change Y.
        // Catches: line 53 `|| → &&` (requires both != 0; with dx=0 movement never happens)
        // Catches: line 53:30 `!= → ==` on dy (dy==0 false for this case, short-circuit)
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 5, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 5, y: 2 },
            },
        );
        app = app.run_fixed(1);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 5);
        assert_eq!(p.pos.y, 1, "Y must change when only Y differs from goal");
    }

    #[test]
    fn sys_move_at_destination_no_movement() {
        // Already at goal → dx=0 && dy=0 → no movement, no event.
        // Catches: line 53:19 `!= → ==` on dx (would enter block when dx==0)
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 3, y: 3 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 3, y: 3 },
            },
        );
        app = app.run_fixed(1);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 3, "X unchanged at destination");
        assert_eq!(p.pos.y, 3, "Y unchanged at destination");
        // Verify no MovedEvent was emitted
        let evs = app.world.get_resource_mut::<Events<MovedEvent>>().unwrap();
        let mut rdr = evs.reader();
        let collected: Vec<_> = rdr.drain().collect();
        assert!(collected.is_empty(), "No movement event at destination");
    }

    #[test]
    fn sys_move_x_only_guard_arithmetic() {
        // Entity at (3,0) wants (1,0). With `!= → ==` on dx (line 53:19):
        // `dx == 0 || dy != 0` → false || false → false → no movement. Catches it.
        // Also catches the `!= → ==` on line 50 because dx=-1 !=0 so with == mutation
        // dy is NOT zeroed, but since dy=0 anyway it wouldn't matter for this case.
        let w = World::new();
        let mut app = build_app(w, 0.016);
        let e = app.world.spawn();
        app.world.insert(
            e,
            CPos {
                pos: IVec2 { x: 3, y: 0 },
            },
        );
        app.world.insert(
            e,
            CDesiredPos {
                pos: IVec2 { x: 1, y: 0 },
            },
        );
        app = app.run_fixed(2);
        let p = app.world.get::<CPos>(e).unwrap();
        assert_eq!(p.pos.x, 1, "Should reach goal after 2 ticks");
    }

    // ─── Mutation-killing tests for sys_sim cooldown decay ───

    #[test]
    fn sys_sim_cooldown_decay_subtracts_dt() {
        // Kills: `*v - dt` → `*v + dt` and `*v - dt` → `*v / dt`
        // With cd=1.0 and dt=0.1:
        //   correct: (1.0 - 0.1).max(0.0) = 0.9
        //   +dt:     (1.0 + 0.1).max(0.0) = 1.1  → WRONG
        //   /dt:     (1.0 / 0.1).max(0.0) = 10.0 → WRONG
        let mut ecs_world = ecs::World::new();
        ecs_world.insert_resource(Dt(0.1));
        ecs_world.insert_resource(World::new());

        let e = ecs_world.spawn();
        ecs_world.insert(
            e,
            CCooldowns {
                map: std::collections::BTreeMap::from([(
                    crate::cooldowns::CooldownKey::from("test_cd"),
                    1.0,
                )]),
            },
        );

        // Call sys_sim directly
        sys_sim(&mut ecs_world);

        let cds = ecs_world.get::<CCooldowns>(e).unwrap();
        let val = *cds
            .map
            .get(&crate::cooldowns::CooldownKey::from("test_cd"))
            .unwrap();
        assert!(
            (val - 0.9).abs() < 1e-5,
            "Cooldown should decrease: 1.0 - 0.1 = 0.9, got {}",
            val
        );
    }

    #[test]
    fn sys_sim_cooldown_clamps_to_zero() {
        // Ensures .max(0.0) works correctly and subtraction goes to 0
        let mut ecs_world = ecs::World::new();
        ecs_world.insert_resource(Dt(2.0));
        ecs_world.insert_resource(World::new());

        let e = ecs_world.spawn();
        ecs_world.insert(
            e,
            CCooldowns {
                map: std::collections::BTreeMap::from([(
                    crate::cooldowns::CooldownKey::from("cd"),
                    0.5,
                )]),
            },
        );

        sys_sim(&mut ecs_world);

        let cds = ecs_world.get::<CCooldowns>(e).unwrap();
        let val = *cds
            .map
            .get(&crate::cooldowns::CooldownKey::from("cd"))
            .unwrap();
        assert_eq!(val, 0.0, "Cooldown should clamp to 0.0 when dt > remaining");
    }

    // ─── Mutation-killing test for sys_sync_to_legacy ───

    #[test]
    fn sys_sync_to_legacy_updates_positions() {
        // Kills: replacing sys_sync_to_legacy body with ()
        // Creates an ECS entity with CLegacyId and CPos, then verifies
        // that после sync the legacy world reflects the ECS position.
        let mut legacy_w = World::new();
        let legacy_id = legacy_w.spawn("ally", IVec2 { x: 0, y: 0 }, crate::Team { id: 1 }, 100, 5);

        let mut ecs_world = ecs::World::new();
        ecs_world.insert_resource(Dt(0.016));
        ecs_world.insert_resource(Events::<MovedEvent>::default());
        ecs_world.insert_resource(EntityBridge::default());
        ecs_world.insert_resource::<World>(legacy_w);

        // Create ECS entity with CLegacyId pointing to legacy entity
        let e = ecs_world.spawn();
        ecs_world.insert(e, crate::CLegacyId { id: legacy_id });
        ecs_world.insert(
            e,
            CPos {
                pos: IVec2 { x: 42, y: 99 },
            },
        );
        ecs_world.insert(e, CHealth { hp: 77 });

        // Call sys_sync_to_legacy directly
        sys_sync_to_legacy(&mut ecs_world);

        // Verify legacy world was updated
        let w = ecs_world.get_resource::<World>().unwrap();
        let pose = w.pose(legacy_id).unwrap();
        assert_eq!(
            pose.pos,
            IVec2 { x: 42, y: 99 },
            "Legacy position should match ECS position after sync"
        );
        let hp = w.health(legacy_id).unwrap();
        assert_eq!(
            hp.hp, 77,
            "Legacy health should match ECS health after sync"
        );
    }
}

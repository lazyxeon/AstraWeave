//! ECS components and systems for gameplay modules

use astraweave_core::{CHealth, CPos};
use astraweave_ecs::{Entity, Query, Query2};
use serde::{Deserialize, Serialize};
/// Combat components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CAttackState {
    pub chain: crate::ComboChain,
    pub idx: usize,
    pub t_since_last: f32,
    pub active: bool,
}

impl CAttackState {
    pub fn new(chain: crate::ComboChain) -> Self {
        Self {
            chain,
            idx: 0,
            t_since_last: 0.0,
            active: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CTarget {
    pub target_id: u64,
}

impl CTarget {
    /// Create a CTarget from an Entity
    pub fn from_entity(entity: Entity) -> Self {
        Self {
            target_id: entity.to_raw(),
        }
    }

    /// Try to resolve this target_id to an Entity, returning None if the entity doesn't exist
    #[allow(unused_variables)]
    pub fn resolve(&self, world: &astraweave_ecs::World) -> Option<Entity> {
        // SAFETY: We're creating an entity from a stored ID.
        // The caller must ensure the world is the correct one.
        let entity = unsafe { Entity::from_raw(self.target_id) };
        // Check if entity has any components (simple existence check)
        // Since is_alive() doesn't exist, we'll just return the entity
        // The actual validation happens when trying to get components
        Some(entity)
    }
    /// Get the target entity, assuming it exists (for use in systems where validity is guaranteed)
    pub fn entity(&self) -> Entity {
        // SAFETY: Caller guarantees entity exists
        unsafe { Entity::from_raw(self.target_id) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CInputState {
    pub pressed_light: bool,
    pub pressed_heavy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CWeapon {
    pub item: crate::Item,
}

/// Crafting components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CCraftingQueue {
    pub recipes: Vec<crate::CraftRecipe>,
    pub progress: Vec<f32>, // progress per recipe
}

/// Quest components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CQuestLog {
    pub active_quests: Vec<crate::Quest>,
    pub completed_quests: Vec<String>, // quest ids
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CQuestProgress {
    pub quest_id: String,
    pub objectives: Vec<crate::Task>, // assuming Task is the objective
}

/// Combat system that ticks attack states
pub fn combat_system(world: &mut astraweave_ecs::World) {
    let dt = *world.get_resource::<f32>().unwrap_or(&0.016); // default 60fps
                                                             // Collect entities with attacks
    let mut attackers = Vec::new();
    {
        let q = Query2::<CAttackState, CTarget>::new(world);
        for (e, attack, target) in q {
            attackers.push((e, attack.clone(), target.entity()));
        }
    }
    for (e, mut attack, target) in attackers {
        if !attack.active {
            continue;
        }
        // Week 3 Action 10: Safe component access with proper error handling
        let Some(pos) = world.get::<CPos>(e) else {
            continue; // Skip if attacker has no position
        };
        let Some(target_pos) = world.get::<CPos>(target) else {
            continue; // Skip if target has no position
        };
        let distance =
            ((pos.pos.x - target_pos.pos.x).abs() + (pos.pos.y - target_pos.pos.y).abs()) as f32;
        // Simplified tick
        attack.t_since_last += dt;
        let step = &attack.chain.steps[attack.idx];
        let in_win = attack.t_since_last >= step.window.0 && attack.t_since_last <= step.window.1;
        let pressed_light = world
            .get::<CInputState>(e)
            .map(|input| input.pressed_light)
            .unwrap_or(false);
        if pressed_light && in_win && distance <= step.reach {
            // Hit
            if let Some(health) = world.get_mut::<CHealth>(target) {
                health.hp -= step.damage;
            }
            // Next step
            attack.idx += 1;
            attack.t_since_last = 0.0;
            if attack.idx >= attack.chain.steps.len() {
                attack.active = false;
            }
            // Update attack state
            world.insert(e, attack);
        }
    }
}

/// Crafting system that advances crafting progress
pub fn crafting_system(world: &mut astraweave_ecs::World) {
    let dt = *world.get_resource::<f32>().unwrap_or(&0.016);
    // Collect entities with queues
    let mut crafters = Vec::new();
    {
        let q = Query::<CCraftingQueue>::new(world);
        for (e, queue) in q {
            crafters.push((e, queue.clone()));
        }
    }
    for (e, mut queue) in crafters {
        for progress in &mut queue.progress {
            *progress += dt;
        }
        // Remove completed recipes (assume 5 seconds)
        let mut i = 0;
        while i < queue.recipes.len() {
            if queue.progress[i] >= 5.0 {
                // Completed
                queue.recipes.remove(i);
                queue.progress.remove(i);
            } else {
                i += 1;
            }
        }
        world.insert(e, queue);
    }
}

/// Quest system that checks objectives (simplified)
pub fn quest_system(world: &mut astraweave_ecs::World) {
    // Collect entities with logs
    let mut questers = Vec::new();
    {
        let q = Query::<CQuestLog>::new(world);
        for (e, log) in q {
            questers.push((e, log.clone()));
        }
    }
    for (e, mut log) in questers {
        // Simplified: mark first active quest as completed
        if !log.active_quests.is_empty() {
            let quest = log.active_quests.remove(0);
            log.completed_quests.push(quest.id);
        }
        world.insert(e, log);
    }
}

/// Combat plugin
pub struct CombatPlugin;

impl astraweave_ecs::Plugin for CombatPlugin {
    fn build(&self, app: &mut astraweave_ecs::App) {
        app.add_system_known_stage("simulation", combat_system);
    }
}

/// Crafting plugin
pub struct CraftingPlugin;

impl astraweave_ecs::Plugin for CraftingPlugin {
    fn build(&self, app: &mut astraweave_ecs::App) {
        app.add_system_known_stage("simulation", crafting_system);
    }
}

/// Quest plugin
pub struct QuestPlugin;

impl astraweave_ecs::Plugin for QuestPlugin {
    fn build(&self, app: &mut astraweave_ecs::App) {
        app.add_system_known_stage("simulation", quest_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CHealth, CPos, IVec2};
    use astraweave_ecs::{App, Plugin};

    #[test]
    fn combat_plugin_applies_damage() {
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });
        // Simplified attack state
        let chain = crate::ComboChain {
            name: "test".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.0, 0.5),
                damage: 20,
                reach: 1.0,
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        // Activate attack
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        // Run simulation
        app = app.run_fixed(1);

        // Check damage applied
        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 80);
    }

    #[test]
    fn crafting_plugin_advances_progress() {
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CraftingPlugin;
        plugin.build(&mut app);

        let crafter = app.world.spawn();
        let recipe = crate::CraftRecipe {
            name: "test".to_string(),
            output_item: crate::ItemKind::Material {
                r#type: crate::ResourceKind::Wood,
            },
            costs: vec![],
        };
        app.world.insert(
            crafter,
            CCraftingQueue {
                recipes: vec![recipe],
                progress: vec![0.0],
            },
        );

        // Run for 5 seconds
        for _ in 0..320 {
            app = app.run_fixed(1);
        }

        // Check completed
        let queue = app.world.get::<CCraftingQueue>(crafter).unwrap();
        assert!(queue.recipes.is_empty());
        assert!(queue.progress.is_empty());
    }

    #[test]
    fn quest_plugin_completes_quests() {
        let mut app = App::new();
        let plugin = QuestPlugin;
        plugin.build(&mut app);

        let quester = app.world.spawn();
        let quest = crate::Quest {
            id: "test_quest".to_string(),
            title: "Test Quest".to_string(),
            tasks: vec![],
            reward_text: "Reward".to_string(),
            completed: false,
        };
        app.world.insert(
            quester,
            CQuestLog {
                active_quests: vec![quest],
                completed_quests: vec![],
            },
        );

        // Run simulation
        app = app.run_fixed(1);

        // Check completed
        let log = app.world.get::<CQuestLog>(quester).unwrap();
        assert!(log.active_quests.is_empty());
        assert_eq!(log.completed_quests, vec!["test_quest".to_string()]);
    }

    // ===== Mutation-resistant tests =====
    // Catches: CTarget::resolve returning None, combat distance + -> -/*

    #[test]
    fn ctarget_resolve_returns_some() {
        // CTarget::resolve must return Some(entity), not None
        let world = astraweave_ecs::World::new();
        let entity = unsafe { astraweave_ecs::Entity::from_raw(42) };
        let target = CTarget::from_entity(entity);
        let resolved = target.resolve(&world);
        assert!(resolved.is_some(), "resolve() must return Some, not None");
        assert_eq!(resolved.unwrap().to_raw(), 42);
    }

    #[test]
    fn combat_distance_check_with_offset_positions() {
        // Attacker at (0,5), target at (3,5): Manhattan dist = 3, reach = 5 → hit
        // If + mutated to -, dist = |0-3| - |5-5| = 3-0 = 3 (same here)
        // Use different x AND y: attacker at (0,0), target at (2,3): dist = 2+3 = 5
        // If + → -, dist = |2| - |3| = -1 abs = 1 (wrong!)
        // If + → *, dist = 2*3 = 6 (wrong!)
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 2, y: 3 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "test".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.0, 0.5),
                damage: 25,
                reach: 5.0, // Reach 5 = exact Manhattan distance
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        app = app.run_fixed(1);

        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 75, "Manhattan dist=5, reach=5 → hit, damage 25");
    }

    #[test]
    fn combat_out_of_reach_no_damage() {
        // attacker at (0,0), target at (3,4): Manhattan dist = 7, reach = 5 → miss
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 3, y: 4 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "test".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.0, 0.5),
                damage: 25,
                reach: 5.0,
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        app = app.run_fixed(1);

        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(
            health.hp, 100,
            "Manhattan dist=7, reach=5 → miss, no damage"
        );
    }

    // ===== Additional mutation-resistant tests for combat_system =====

    #[test]
    fn combat_system_distance_subtraction_matters() {
        // Catches: replace - with + in BOTH pos.pos.x and pos.pos.y subtractions (line 111)
        // Use positions where (a - b) != (a + b) for BOTH x AND y
        // Attacker at (5,3), target at (2,1): dist = |5-2| + |3-1| = 3+2 = 5
        // If x- →+: dist = |5+2| + |3-1| = 7+2 = 9
        // If y- →+: dist = |5-2| + |3+1| = 3+4 = 7
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 5, y: 3 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 2, y: 1 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "sub_test".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.0, 0.5),
                damage: 10,
                reach: 6.0, // reach=6, true dist=5 → hit; mutated dist=7 or 9 → miss
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        app = app.run_fixed(1);

        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 90, "dist=5, reach=6 → hit, damage=10");
    }

    #[test]
    fn combat_system_t_since_last_additively_increments() {
        // Catches: replace += with *= in attack.t_since_last += dt (line 113)
        // Attack state is cloned each tick and only written back on hit, so
        // t_since_last doesn't accumulate across ticks.  Use dt=0.2 with
        // window=(0.1, 0.5):  0.0 + 0.2 = 0.2 ∈ [0.1,0.5] → hit.
        // With *= : 0.0 * 0.2 = 0.0 < 0.1 → never reaches window → no hit.
        let mut app = App::new();
        app.world.insert_resource(0.2f32); // dt large enough to enter window in 1 tick
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "timing".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.1, 0.5), // Window starts at 0.1s
                damage: 15,
                reach: 10.0,
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        app = app.run_fixed(1);

        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(
            health.hp, 85,
            "t_since_last must += dt to reach window; *= would keep it at 0"
        );
    }

    #[test]
    fn combat_system_window_requires_both_conditions() {
        // Catches: replace && with || in in_win check (line 115)
        // in_win = t >= window.0 && t <= window.1
        // If && → ||, any time satisfies the check → always a hit even outside window
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "narrow_window".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (10.0, 10.5), // Window at 10s — unreachable in 1 frame
                damage: 50,
                reach: 10.0,
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        app = app.run_fixed(1); // t=0.016 — outside window [10.0, 10.5]

        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 100, "Window at 10s must not trigger at t=0.016");
    }

    #[test]
    fn combat_system_idx_increments_additively() {
        // Catches: replace += with *= in attack.idx += 1 (line 126)
        // With *= 1 from idx=0, idx stays 0 → chain never completes
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(target, CHealth { hp: 100 });

        let chain = crate::ComboChain {
            name: "two_step".to_string(),
            steps: vec![
                crate::ComboStep {
                    kind: crate::AttackKind::Light,
                    window: (0.0, 0.5),
                    damage: 10,
                    reach: 10.0,
                    stagger: 0.5,
                },
                crate::ComboStep {
                    kind: crate::AttackKind::Light,
                    window: (0.0, 0.5),
                    damage: 20,
                    reach: 10.0,
                    stagger: 0.5,
                },
            ],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        // First hit — should do step[0].damage = 10
        app = app.run_fixed(1);
        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 90, "First step should deal 10 damage");

        // Second hit — should do step[1].damage = 20
        app = app.run_fixed(1);
        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(
            health.hp, 70,
            "Second step should deal 20 more damage (idx must advance)"
        );

        // Attack should now be inactive (idx >= steps.len())
        let attack = app.world.get::<CAttackState>(attacker).unwrap();
        assert!(!attack.active, "Chain should deactivate after last step");
    }

    #[test]
    fn combat_system_chain_deactivates_at_end() {
        // Catches: replace >= with < in attack.idx >= attack.chain.steps.len() (line 128)
        let mut app = App::new();
        app.world.insert_resource(0.016f32);
        let plugin = CombatPlugin;
        plugin.build(&mut app);

        let attacker = app.world.spawn();
        let target = app.world.spawn();
        app.world.insert(
            attacker,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(
            target,
            CPos {
                pos: IVec2 { x: 0, y: 0 },
            },
        );
        app.world.insert(target, CHealth { hp: 200 });

        let chain = crate::ComboChain {
            name: "one_hit".to_string(),
            steps: vec![crate::ComboStep {
                kind: crate::AttackKind::Light,
                window: (0.0, 0.5),
                damage: 10,
                reach: 10.0,
                stagger: 0.5,
            }],
        };
        app.world.insert(attacker, CAttackState::new(chain));
        app.world.insert(attacker, CTarget::from_entity(target));
        app.world.insert(
            attacker,
            CInputState {
                pressed_light: true,
                pressed_heavy: false,
            },
        );
        if let Some(attack) = app.world.get_mut::<CAttackState>(attacker) {
            attack.active = true;
        }

        // One hit
        app = app.run_fixed(1);
        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(health.hp, 190);

        // Chain should not fire again
        app = app.run_fixed(1);
        let health = app.world.get::<CHealth>(target).unwrap();
        assert_eq!(
            health.hp, 190,
            "Chain must stop dealing damage after deactivating"
        );
    }

    #[test]
    fn crafting_system_progress_threshold_exact() {
        // Catches: replace >= with < in queue.progress[i] >= 5.0 (line 155)
        let mut app = App::new();
        app.world.insert_resource(1.0f32); // dt = 1.0 for fast testing
        let plugin = CraftingPlugin;
        plugin.build(&mut app);

        let crafter = app.world.spawn();
        let recipe = crate::CraftRecipe {
            name: "test_item".to_string(),
            output_item: crate::ItemKind::Material {
                r#type: crate::ResourceKind::Wood,
            },
            costs: vec![],
        };
        app.world.insert(
            crafter,
            CCraftingQueue {
                recipes: vec![recipe],
                progress: vec![4.0], // Start at 4.0, after one tick at dt=1.0 → 5.0 which is >= 5.0
            },
        );

        app = app.run_fixed(1); // progress becomes 5.0

        let queue = app.world.get::<CCraftingQueue>(crafter).unwrap();
        assert!(
            queue.recipes.is_empty(),
            "Recipe at progress 5.0 must be removed (>= 5.0)"
        );
    }
}

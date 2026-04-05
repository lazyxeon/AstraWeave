#![forbid(unsafe_code)]
use astraweave_core::{DirectorBudget, DirectorOp, DirectorPlan, IVec2, Rect, WorldSnapshot};

/// Minimal heuristic boss director:
/// - If player trends ranged (distance > 8), fortify a choke around midpoint.
/// - Else: spawn a small wave behind player and collapse a nearby bridge line.
pub struct BossDirector;

mod phase;
pub use phase::*;

mod llm_director;
pub use llm_director::*;

mod components;
pub use components::*;

mod systems;
pub use systems::*;

#[cfg(feature = "veilweaver_slice")]
mod veilweaver_warden;
#[cfg(feature = "veilweaver_slice")]
pub use veilweaver_warden::*;

impl BossDirector {
    pub fn plan(&self, snap: &WorldSnapshot, budget: &DirectorBudget) -> DirectorPlan {
        let mut ops = vec![];
        let ppos = snap.player.pos;
        let _mpos = snap.me.pos;
        // choose a target enemy if any
        let tgt = snap.enemies.first().map(|e| e.pos).unwrap_or(IVec2 {
            x: ppos.x + 6,
            y: ppos.y,
        });

        let dist = (ppos.x - tgt.x).abs() + (ppos.y - tgt.y).abs();
        if dist > 8 && budget.terrain_edits > 0 {
            // Fortify: draw a small rectangle near target as a makeshift choke
            let xm = (ppos.x + tgt.x) / 2;
            let ym = (ppos.y + tgt.y) / 2;
            ops.push(DirectorOp::Fortify {
                rect: Rect {
                    x0: xm - 1,
                    y0: ym - 1,
                    x1: xm + 1,
                    y1: ym + 1,
                },
            });
        } else {
            // Spawn wave behind player, collapse a line between player and target
            if budget.spawns > 0 {
                let origin = IVec2 {
                    x: ppos.x - 2,
                    y: ppos.y + 1,
                };
                ops.push(DirectorOp::SpawnWave {
                    archetype: "minion".into(),
                    count: 3,
                    origin,
                });
            }
            if budget.terrain_edits > 0 {
                let line_b = IVec2 {
                    x: (ppos.x + tgt.x) / 2,
                    y: (ppos.y + tgt.y) / 2,
                };
                ops.push(DirectorOp::Collapse { a: ppos, b: line_b });
            }
        }
        DirectorPlan { ops }
    }
}

#[cfg(test)]
#[allow(clippy::identity_op, clippy::erasing_op)]
mod tests {
    use super::*;
    use astraweave_core::{
        CompanionState, DirectorBudget, DirectorOp, EnemyState, IVec2, PlayerState, WorldSnapshot,
    };

    fn test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                pos: IVec2 { x: 0, y: 0 },
                hp: 100,
                stance: "melee".into(),
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2 { x: 1, y: 1 },
                ammo: 10,
                cooldowns: std::collections::BTreeMap::new(),
                morale: 100.0,
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 5, y: 5 },
                hp: 50,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    fn empty_budget() -> DirectorBudget {
        DirectorBudget {
            spawns: 0,
            terrain_edits: 0,
            traps: 0,
        }
    }

    fn full_budget() -> DirectorBudget {
        DirectorBudget {
            spawns: 5,
            terrain_edits: 5,
            traps: 5,
        }
    }

    #[test]
    fn test_boss_director_creation() {
        let director = BossDirector;
        assert!(std::mem::size_of_val(&director) < 100);
    }

    #[test]
    fn test_plan_with_empty_budget() {
        let director = BossDirector;
        let snap = test_snapshot();
        let budget = empty_budget();
        let plan = director.plan(&snap, &budget);
        assert!(plan.ops.is_empty());
    }

    #[test]
    fn test_plan_with_spawn_budget() {
        let director = BossDirector;
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 1,
            terrain_edits: 0,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert_eq!(plan.ops.len(), 1);
        assert!(matches!(plan.ops[0], DirectorOp::SpawnWave { .. }));
    }

    #[test]
    fn test_plan_spawn_wave_details() {
        let director = BossDirector;
        let snap = test_snapshot();
        let budget = DirectorBudget {
            spawns: 1,
            terrain_edits: 0,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        if let DirectorOp::SpawnWave {
            archetype,
            count,
            origin,
        } = &plan.ops[0]
        {
            assert_eq!(archetype, "minion");
            assert_eq!(*count, 3);
            assert_eq!(origin.x, snap.player.pos.x - 2);
            assert_eq!(origin.y, snap.player.pos.y + 1);
        } else {
            panic!("Expected SpawnWave op");
        }
    }

    #[test]
    fn test_plan_with_terrain_budget() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        // Set distance <= 8 so Collapse is triggered
        snap.enemies[0].pos = IVec2 { x: 4, y: 0 }; // distance = 4
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert_eq!(plan.ops.len(), 1);
        assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
    }

    #[test]
    fn test_plan_collapse_targets_midpoint() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        // Set distance <= 8 so Collapse is triggered
        snap.enemies[0].pos = IVec2 { x: 6, y: 0 }; // distance = 6
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        if let DirectorOp::Collapse { a, b } = &plan.ops[0] {
            assert_eq!(*a, snap.player.pos);
            let expected_b = IVec2 {
                x: (snap.player.pos.x + snap.enemies[0].pos.x) / 2,
                y: (snap.player.pos.y + snap.enemies[0].pos.y) / 2,
            };
            assert_eq!(*b, expected_b);
        } else {
            panic!("Expected Collapse op");
        }
    }

    #[test]
    fn test_plan_with_full_budget() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        // Set distance <= 8 so Spawn+Collapse are triggered
        snap.enemies[0].pos = IVec2 { x: 4, y: 0 }; // distance = 4
        let budget = full_budget();
        let plan = director.plan(&snap, &budget);
        assert_eq!(plan.ops.len(), 2);
        assert!(matches!(plan.ops[0], DirectorOp::SpawnWave { .. }));
        assert!(matches!(plan.ops[1], DirectorOp::Collapse { .. }));
    }

    #[test]
    fn test_fortify_at_distance() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.enemies[0].pos = IVec2 { x: 15, y: 0 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert_eq!(plan.ops.len(), 1);
        assert!(matches!(plan.ops[0], DirectorOp::Fortify { .. }));
    }

    #[test]
    fn test_fortify_rect_centered() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.enemies[0].pos = IVec2 { x: 20, y: 0 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        if let DirectorOp::Fortify { rect } = &plan.ops[0] {
            let xm = (0 + 20) / 2;
            let ym = (0 + 0) / 2;
            assert_eq!(rect.x0, xm - 1);
            assert_eq!(rect.y0, ym - 1);
            assert_eq!(rect.x1, xm + 1);
            assert_eq!(rect.y1, ym + 1);
        } else {
            panic!("Expected Fortify op");
        }
    }

    #[test]
    fn test_no_enemies_uses_default_target() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.enemies.clear();
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert!(!plan.ops.is_empty());
    }

    #[test]
    fn test_plan_deterministic() {
        let director = BossDirector;
        let snap = test_snapshot();
        let budget = full_budget();
        let plan1 = director.plan(&snap, &budget);
        let plan2 = director.plan(&snap, &budget);
        assert_eq!(plan1.ops.len(), plan2.ops.len());
    }

    #[test]
    fn test_distance_calculation_threshold() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.enemies[0].pos = IVec2 { x: 4, y: 4 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
    }

    #[test]
    fn test_distance_exactly_at_threshold() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.enemies[0].pos = IVec2 { x: 8, y: 0 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
    }

    #[test]
    fn test_distance_over_threshold() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: 0, y: 0 };
        snap.enemies[0].pos = IVec2 { x: 9, y: 0 };
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan = director.plan(&snap, &budget);
        assert!(matches!(plan.ops[0], DirectorOp::Fortify { .. }));
    }

    #[test]
    fn test_multiple_enemies_uses_first() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.enemies.push(EnemyState {
            id: 2,
            pos: IVec2 { x: 100, y: 100 },
            hp: 100,
            cover: "heavy".into(),
            last_seen: 0.0,
        });
        let budget = DirectorBudget {
            spawns: 0,
            terrain_edits: 1,
            traps: 0,
        };
        let plan1 = director.plan(&snap, &budget);
        snap.enemies.remove(1);
        let plan2 = director.plan(&snap, &budget);
        assert_eq!(plan1.ops.len(), plan2.ops.len());
    }

    #[test]
    fn test_negative_positions() {
        let director = BossDirector;
        let mut snap = test_snapshot();
        snap.player.pos = IVec2 { x: -5, y: -5 };
        snap.enemies[0].pos = IVec2 { x: -15, y: -5 };
        let budget = full_budget();
        let plan = director.plan(&snap, &budget);
        assert!(!plan.ops.is_empty());
    }
}

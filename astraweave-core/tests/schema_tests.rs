//! Comprehensive tests for astraweave-core schema types
//!
//! Tests cover:
//! - WorldSnapshot construction and field access
//! - CompanionState/EnemyState/PlayerState edge cases
//! - PlanIntent validation and serialization
//! - ActionStep pattern matching and variant construction
//! - Default implementations
//! - IVec2 equality and operations
//!
//! Target Coverage: 75% of schema.rs (426 lines)

use astraweave_core::schema::{
    ActionStep, CompanionState, CoverType, EnemyState, IVec2, MovementSpeed, PlanIntent,
    PlayerState, Poi, Stance, StrafeDirection, WorldSnapshot,
};
use std::collections::BTreeMap;

// ============================================================================
// WORLDSNAPSHOT CONSTRUCTION & FIELD ACCESS (3 tests)
// ============================================================================

#[test]
fn test_worldsnapshot_default_construction() {
    let snap = WorldSnapshot::default();

    assert_eq!(snap.t, 0.0);
    assert_eq!(snap.player.hp, 100);
    assert_eq!(snap.player.pos.x, 0);
    assert_eq!(snap.player.pos.y, 0);
    assert_eq!(snap.me.ammo, 10);
    assert_eq!(snap.me.morale, 1.0);
    assert!(snap.enemies.is_empty());
    assert!(snap.pois.is_empty());
    assert!(snap.obstacles.is_empty());
    assert!(snap.objective.is_none());
}

#[test]
fn test_worldsnapshot_with_data() {
    let player = PlayerState {
        hp: 80,
        pos: IVec2 { x: 5, y: 10 },
        stance: Stance::Crouch,
        orders: vec!["hold".to_string(), "defend".to_string()],
    };

    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("attack".to_string(), 2.5);
    cooldowns.insert("coverfire".to_string(), 5.0);

    let companion = CompanionState {
        ammo: 25,
        cooldowns,
        morale: 0.75,
        pos: IVec2 { x: 3, y: 8 },
    };

    let enemies = vec![
        EnemyState {
            id: 1,
            pos: IVec2 { x: 20, y: 15 },
            hp: 50,
            cover: CoverType::Unknown,
            last_seen: 1.5,
        },
        EnemyState {
            id: 2,
            pos: IVec2 { x: 25, y: 18 },
            hp: 30,
            cover: CoverType::High,
            last_seen: 2.0,
        },
    ];

    let pois = vec![
        Poi {
            k: "objective".to_string(),
            pos: IVec2 { x: 30, y: 20 },
        },
        Poi {
            k: "ammo_cache".to_string(),
            pos: IVec2 { x: 15, y: 10 },
        },
    ];

    let obstacles = vec![IVec2 { x: 10, y: 10 }, IVec2 { x: 11, y: 10 }];

    let snap = WorldSnapshot {
        t: 12.5,
        player,
        me: companion,
        enemies,
        pois,
        obstacles,
        objective: Some("secure_area".to_string()),
    };

    // Validate field access
    assert_eq!(snap.t, 12.5);
    assert_eq!(snap.player.hp, 80);
    assert_eq!(snap.player.stance, Stance::Crouch);
    assert_eq!(snap.player.orders.len(), 2);
    assert_eq!(snap.me.ammo, 25);
    assert_eq!(snap.me.morale, 0.75);
    assert_eq!(snap.me.cooldowns.get("attack"), Some(&2.5));
    assert_eq!(snap.enemies.len(), 2);
    assert_eq!(snap.enemies[0].id, 1);
    assert_eq!(snap.enemies[1].hp, 30);
    assert_eq!(snap.pois.len(), 2);
    assert_eq!(snap.pois[0].k, "objective");
    assert_eq!(snap.obstacles.len(), 2);
    assert_eq!(snap.objective, Some("secure_area".to_string()));
}

#[test]
fn test_worldsnapshot_empty_collections() {
    let snap = WorldSnapshot {
        t: 5.0,
        player: PlayerState::default(),
        me: CompanionState::default(),
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    };

    assert!(snap.enemies.is_empty());
    assert!(snap.pois.is_empty());
    assert!(snap.obstacles.is_empty());
    assert!(snap.objective.is_none());
}

// ============================================================================
// COMPANIONSTATE / ENEMYSTATE / PLAYERSTATE EDGE CASES (3 tests)
// ============================================================================

#[test]
fn test_companionstate_edge_cases() {
    // Zero ammo
    let low_ammo = CompanionState {
        ammo: 0,
        cooldowns: BTreeMap::new(),
        morale: 1.0,
        pos: IVec2 { x: 0, y: 0 },
    };
    assert_eq!(low_ammo.ammo, 0);

    // Zero morale
    let broken_morale = CompanionState {
        ammo: 10,
        cooldowns: BTreeMap::new(),
        morale: 0.0,
        pos: IVec2 { x: 0, y: 0 },
    };
    assert_eq!(broken_morale.morale, 0.0);

    // High morale
    let max_morale = CompanionState {
        ammo: 10,
        cooldowns: BTreeMap::new(),
        morale: 2.0, // Can exceed 1.0
        pos: IVec2 { x: 0, y: 0 },
    };
    assert_eq!(max_morale.morale, 2.0);

    // Many cooldowns
    let mut many_cds = BTreeMap::new();
    many_cds.insert("attack".to_string(), 1.5);
    many_cds.insert("coverfire".to_string(), 3.0);
    many_cds.insert("throw".to_string(), 2.0);
    many_cds.insert("revive".to_string(), 5.0);

    let loaded_cooldowns = CompanionState {
        ammo: 10,
        cooldowns: many_cds,
        morale: 1.0,
        pos: IVec2 { x: 0, y: 0 },
    };
    assert_eq!(loaded_cooldowns.cooldowns.len(), 4);
    assert_eq!(loaded_cooldowns.cooldowns.get("attack"), Some(&1.5));
    assert_eq!(loaded_cooldowns.cooldowns.get("revive"), Some(&5.0));
}

#[test]
fn test_enemystate_edge_cases() {
    // Zero HP (dead enemy)
    let dead = EnemyState {
        id: 10,
        pos: IVec2 { x: 5, y: 5 },
        hp: 0,
        cover: CoverType::None,
        last_seen: 0.0,
    };
    assert_eq!(dead.hp, 0);

    // Negative HP (overkill damage)
    let overkilled = EnemyState {
        id: 11,
        pos: IVec2 { x: 6, y: 6 },
        hp: -10,
        cover: CoverType::None,
        last_seen: 0.0,
    };
    assert_eq!(overkilled.hp, -10);

    // Very old last_seen (lost track)
    let forgotten = EnemyState {
        id: 12,
        pos: IVec2 { x: 7, y: 7 },
        hp: 50,
        cover: CoverType::High,
        last_seen: 100.0, // 100 seconds ago
    };
    assert_eq!(forgotten.last_seen, 100.0);

    // Various cover types
    let covers = [
        CoverType::None,
        CoverType::Low,
        CoverType::High,
        CoverType::Partial,
        CoverType::Wall,
    ];
    for (i, cover_type) in covers.iter().enumerate() {
        let enemy = EnemyState {
            id: i as u32,
            pos: IVec2 {
                x: i as i32,
                y: i as i32,
            },
            hp: 100,
            cover: *cover_type,
            last_seen: 0.0,
        };
        assert_eq!(enemy.cover, *cover_type);
    }
}

#[test]
fn test_playerstate_edge_cases() {
    // Zero HP (dead player)
    let dead = PlayerState {
        hp: 0,
        pos: IVec2 { x: 0, y: 0 },
        stance: Stance::Prone,
        orders: vec![],
    };
    assert_eq!(dead.hp, 0);

    // Many orders
    let busy = PlayerState {
        hp: 100,
        pos: IVec2 { x: 0, y: 0 },
        stance: Stance::Stand,
        orders: vec![
            "move_to_alpha".to_string(),
            "secure_bravo".to_string(),
            "extract_charlie".to_string(),
            "regroup_delta".to_string(),
        ],
    };
    assert_eq!(busy.orders.len(), 4);
    assert_eq!(busy.orders[0], "move_to_alpha");
    assert_eq!(busy.orders[3], "regroup_delta");

    // Various stances
    let stances = [Stance::Stand, Stance::Crouch, Stance::Prone];
    for (i, stance) in stances.iter().enumerate() {
        let player = PlayerState {
            hp: 100,
            pos: IVec2 {
                x: i as i32,
                y: i as i32,
            },
            stance: *stance,
            orders: vec![],
        };
        assert_eq!(player.stance, *stance);
    }
}

// ============================================================================
// PLANINTENT VALIDATION (2 tests)
// ============================================================================

#[test]
fn test_planintent_construction() {
    // Empty plan
    let empty = PlanIntent {
        plan_id: "plan-empty-001".to_string(),
        steps: vec![],
    };
    assert_eq!(empty.plan_id, "plan-empty-001");
    assert!(empty.steps.is_empty());

    // Single step plan
    let single = PlanIntent {
        plan_id: "plan-single-002".to_string(),
        steps: vec![ActionStep::MoveTo {
            x: 10,
            y: 20,
            speed: Some(MovementSpeed::Run),
        }],
    };
    assert_eq!(single.plan_id, "plan-single-002");
    assert_eq!(single.steps.len(), 1);

    // Multi-step plan
    let multi = PlanIntent {
        plan_id: "plan-multi-003".to_string(),
        steps: vec![
            ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: Some(MovementSpeed::Sprint),
            },
            ActionStep::TakeCover { position: None },
            ActionStep::Attack { target_id: 1 },
        ],
    };
    assert_eq!(multi.plan_id, "plan-multi-003");
    assert_eq!(multi.steps.len(), 3);
}

#[test]
fn test_planintent_default() {
    let default_plan = PlanIntent::default();

    assert_eq!(default_plan.plan_id, "");
    assert!(default_plan.steps.is_empty());
}

// ============================================================================
// ACTIONSTEP PATTERN MATCHING (2 tests)
// ============================================================================

#[test]
fn test_actionstep_movement_variants() {
    // MoveTo with speed
    let move_to = ActionStep::MoveTo {
        x: 10,
        y: 20,
        speed: Some(MovementSpeed::Sprint),
    };
    match move_to {
        ActionStep::MoveTo { x, y, speed } => {
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(speed, Some(MovementSpeed::Sprint));
        }
        _ => panic!("Expected MoveTo variant"),
    }

    // MoveTo without speed (default)
    let move_default = ActionStep::MoveTo {
        x: 5,
        y: 15,
        speed: None,
    };
    match move_default {
        ActionStep::MoveTo { x, y, speed } => {
            assert_eq!(x, 5);
            assert_eq!(y, 15);
            assert!(speed.is_none());
        }
        _ => panic!("Expected MoveTo variant"),
    }

    // Approach
    let approach = ActionStep::Approach {
        target_id: 1,
        distance: 10.0,
    };
    match approach {
        ActionStep::Approach {
            target_id,
            distance,
        } => {
            assert_eq!(target_id, 1);
            assert_eq!(distance, 10.0);
        }
        _ => panic!("Expected Approach variant"),
    }

    // Retreat
    let retreat = ActionStep::Retreat {
        target_id: 2,
        distance: 20.0,
    };
    match retreat {
        ActionStep::Retreat {
            target_id,
            distance,
        } => {
            assert_eq!(target_id, 2);
            assert_eq!(distance, 20.0);
        }
        _ => panic!("Expected Retreat variant"),
    }

    // TakeCover with position
    let cover_pos = ActionStep::TakeCover {
        position: Some(IVec2 { x: 15, y: 15 }),
    };
    match cover_pos {
        ActionStep::TakeCover { position } => {
            assert_eq!(position, Some(IVec2 { x: 15, y: 15 }));
        }
        _ => panic!("Expected TakeCover variant"),
    }

    // TakeCover without position (find nearest)
    let cover_auto = ActionStep::TakeCover { position: None };
    match cover_auto {
        ActionStep::TakeCover { position } => {
            assert!(position.is_none());
        }
        _ => panic!("Expected TakeCover variant"),
    }

    // Strafe
    let strafe = ActionStep::Strafe {
        target_id: 3,
        direction: StrafeDirection::Left,
    };
    match strafe {
        ActionStep::Strafe {
            target_id,
            direction,
        } => {
            assert_eq!(target_id, 3);
            assert_eq!(direction, StrafeDirection::Left);
        }
        _ => panic!("Expected Strafe variant"),
    }

    // Patrol
    let patrol = ActionStep::Patrol {
        waypoints: vec![
            IVec2 { x: 0, y: 0 },
            IVec2 { x: 10, y: 0 },
            IVec2 { x: 10, y: 10 },
        ],
    };
    match patrol {
        ActionStep::Patrol { waypoints } => {
            assert_eq!(waypoints.len(), 3);
            assert_eq!(waypoints[0], IVec2 { x: 0, y: 0 });
            assert_eq!(waypoints[2], IVec2 { x: 10, y: 10 });
        }
        _ => panic!("Expected Patrol variant"),
    }
}

#[test]
fn test_actionstep_combat_variants() {
    // Attack
    let attack = ActionStep::Attack { target_id: 5 };
    match attack {
        ActionStep::Attack { target_id } => {
            assert_eq!(target_id, 5);
        }
        _ => panic!("Expected Attack variant"),
    }

    // AimedShot
    let aimed = ActionStep::AimedShot { target_id: 6 };
    match aimed {
        ActionStep::AimedShot { target_id } => {
            assert_eq!(target_id, 6);
        }
        _ => panic!("Expected AimedShot variant"),
    }

    // QuickAttack
    let quick = ActionStep::QuickAttack { target_id: 7 };
    match quick {
        ActionStep::QuickAttack { target_id } => {
            assert_eq!(target_id, 7);
        }
        _ => panic!("Expected QuickAttack variant"),
    }

    // HeavyAttack
    let heavy = ActionStep::HeavyAttack { target_id: 8 };
    match heavy {
        ActionStep::HeavyAttack { target_id } => {
            assert_eq!(target_id, 8);
        }
        _ => panic!("Expected HeavyAttack variant"),
    }

    // AoEAttack
    let aoe = ActionStep::AoEAttack {
        x: 20,
        y: 25,
        radius: 5.0,
    };
    match aoe {
        ActionStep::AoEAttack { x, y, radius } => {
            assert_eq!(x, 20);
            assert_eq!(y, 25);
            assert_eq!(radius, 5.0);
        }
        _ => panic!("Expected AoEAttack variant"),
    }

    // ThrowExplosive
    let explosive = ActionStep::ThrowExplosive { x: 30, y: 35 };
    match explosive {
        ActionStep::ThrowExplosive { x, y } => {
            assert_eq!(x, 30);
            assert_eq!(y, 35);
        }
        _ => panic!("Expected ThrowExplosive variant"),
    }

    // CoverFire
    let cover_fire = ActionStep::CoverFire {
        target_id: 9,
        duration: 3.0,
    };
    match cover_fire {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(target_id, 9);
            assert_eq!(duration, 3.0);
        }
        _ => panic!("Expected CoverFire variant"),
    }

    // Charge
    let charge = ActionStep::Charge { target_id: 10 };
    match charge {
        ActionStep::Charge { target_id } => {
            assert_eq!(target_id, 10);
        }
        _ => panic!("Expected Charge variant"),
    }

    // ThrowSmoke (defensive but often combat-related)
    let smoke = ActionStep::ThrowSmoke { x: 15, y: 20 };
    match smoke {
        ActionStep::ThrowSmoke { x, y } => {
            assert_eq!(x, 15);
            assert_eq!(y, 20);
        }
        _ => panic!("Expected ThrowSmoke variant"),
    }
}

// ============================================================================
// IVEC2 OPERATIONS (2 tests)
// ============================================================================

#[test]
fn test_ivec2_equality() {
    let a = IVec2 { x: 10, y: 20 };
    let b = IVec2 { x: 10, y: 20 };
    let c = IVec2 { x: 11, y: 20 };
    let d = IVec2 { x: 10, y: 21 };

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
    assert_ne!(c, d);
}

#[test]
fn test_ivec2_edge_cases() {
    // Zero vector
    let zero = IVec2 { x: 0, y: 0 };
    assert_eq!(zero.x, 0);
    assert_eq!(zero.y, 0);

    // Negative coordinates
    let negative = IVec2 { x: -10, y: -20 };
    assert_eq!(negative.x, -10);
    assert_eq!(negative.y, -20);

    // Large coordinates
    let large = IVec2 {
        x: 1_000_000,
        y: 2_000_000,
    };
    assert_eq!(large.x, 1_000_000);
    assert_eq!(large.y, 2_000_000);

    // Mixed signs
    let mixed = IVec2 { x: -5, y: 10 };
    assert_eq!(mixed.x, -5);
    assert_eq!(mixed.y, 10);

    // Default
    let default_vec = IVec2::default();
    assert_eq!(default_vec, IVec2 { x: 0, y: 0 });
}

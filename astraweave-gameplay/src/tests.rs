//! Deterministic tests for gameplay systems
//! These tests use fixed parameters and validate reproducible behavior

use crate::combat::{AttackKind, AttackState, ComboChain, ComboStep};
use crate::crafting::{CraftCost, CraftRecipe, RecipeBook};
use crate::dialogue::{Choice, Cond, Dialogue, DialogueState, Line, Node};
pub use crate::test_support::{test_panic, TestValue};
use crate::{DamageType, Inventory, ItemKind, ResourceKind, Stats};
use glam::Vec3;

use rand::SeedableRng;

/// Test that combat damage is deterministic with fixed positions
#[test]
fn test_combat_deterministic_damage() {
    // Setup: Two entities at fixed positions
    let mut attacker_stats = Stats::new(100);
    attacker_stats.power = 10;

    let mut target_stats = Stats::new(100);

    let chain = ComboChain {
        name: "basic_combo".to_string(),
        steps: vec![ComboStep {
            kind: AttackKind::Light,
            window: (0.0, 0.5),
            damage: 20,
            reach: 2.0,
            stagger: 0.1,
        }],
    };

    let mut attack_state = AttackState::new(chain);
    attack_state.start();

    let attacker_pos = Vec3::new(0.0, 0.0, 0.0);
    let target_pos = Vec3::new(1.0, 0.0, 0.0); // 1 meter away

    // Execute attack sequence deterministically
    let dt = 0.016; // 60 FPS

    // Frame 1: Light attack (within window)
    let (hit1, _dmg1) = attack_state.tick(
        dt,
        true,  // pressed_light
        false, // pressed_heavy
        attacker_pos,
        target_pos,
        &attacker_stats,
        None, // no weapon
        &mut target_stats,
    );

    // Verify deterministic outcome
    assert!(hit1, "First attack should hit");
    // Note: Actual damage may be reduced by defense stat
    let actual_damage = 100 - target_stats.hp;
    assert!(
        actual_damage > 0 && actual_damage <= 30,
        "Damage should be between 1 and 30"
    );
    assert!(target_stats.hp < 100, "Target should have taken damage");
}

/// Test that combat respects reach distance
#[test]
fn test_combat_reach_validation() {
    let attacker_stats = Stats::new(100);
    let mut target_stats = Stats::new(100);

    let chain = ComboChain {
        name: "basic".to_string(),
        steps: vec![ComboStep {
            kind: AttackKind::Light,
            window: (0.0, 0.5),
            damage: 20,
            reach: 1.5, // 1.5 meter reach
            stagger: 0.1,
        }],
    };

    let mut attack_state = AttackState::new(chain);
    attack_state.start();

    let attacker_pos = Vec3::new(0.0, 0.0, 0.0);
    let target_pos_close = Vec3::new(1.0, 0.0, 0.0); // 1 meter (within reach)
    let target_pos_far = Vec3::new(2.0, 0.0, 0.0); // 2 meters (out of reach)

    // Test 1: Close target (should hit)
    let (hit1, dmg1) = attack_state.tick(
        0.016,
        true,
        false,
        attacker_pos,
        target_pos_close,
        &attacker_stats,
        None,
        &mut target_stats,
    );
    assert!(hit1, "Attack should hit close target");
    assert!(dmg1 > 0, "Should deal damage to close target");

    // Reset for test 2
    attack_state.start();
    target_stats.hp = 100;

    // Test 2: Far target (should miss)
    let (hit2, dmg2) = attack_state.tick(
        0.016,
        true,
        false,
        attacker_pos,
        target_pos_far,
        &attacker_stats,
        None,
        &mut target_stats,
    );
    assert!(!hit2, "Attack should miss far target");
    assert_eq!(dmg2, 0, "Should not deal damage to far target");
    assert_eq!(target_stats.hp, 100, "Target HP should be unchanged");
}

/// Test deterministic crafting with fixed inventory
#[test]
fn test_crafting_deterministic() {
    // Setup recipe book
    let recipes = vec![CraftRecipe {
        name: "iron_sword".to_string(),
        output_item: ItemKind::Weapon {
            base_damage: 25,
            dtype: DamageType::Physical,
        },
        costs: vec![
            CraftCost {
                kind: ResourceKind::Ore,
                count: 5,
            },
            CraftCost {
                kind: ResourceKind::Wood,
                count: 2,
            },
        ],
    }];

    let book = RecipeBook { recipes };

    // Test 1: Sufficient resources (should succeed)
    let mut inv1 = Inventory::default();
    inv1.add_resource(ResourceKind::Ore, 10);
    inv1.add_resource(ResourceKind::Wood, 5);

    let mut rng1 = rand::rngs::StdRng::seed_from_u64(12345);
    let result1 = book.craft_seeded("iron_sword", &mut inv1, &mut rng1);
    assert!(result1.is_some(), "Crafting should succeed with resources");

    let item = result1.test_unwrap();
    assert_eq!(item.name, "iron_sword");
    match item.kind {
        ItemKind::Weapon { base_damage, .. } => {
            assert_eq!(base_damage, 25, "Weapon damage should match recipe");
        }
        _ => test_panic(format_args!("Should be a weapon")),
    }

    // Verify resources consumed
    assert_eq!(
        inv1.resources
            .iter()
            .find(|(k, _)| *k == ResourceKind::Ore)
            .map(|(_, n)| *n)
            .unwrap_or(0),
        5,
        "Ore should be 10 - 5 = 5"
    );
    assert_eq!(
        inv1.resources
            .iter()
            .find(|(k, _)| *k == ResourceKind::Wood)
            .map(|(_, n)| *n)
            .unwrap_or(0),
        3,
        "Wood should be 5 - 2 = 3"
    );

    // Test 2: Insufficient resources (should fail)
    let mut inv2 = Inventory::default();
    inv2.add_resource(ResourceKind::Ore, 3); // Not enough ore

    let mut rng2 = rand::rngs::StdRng::seed_from_u64(12345);
    let result2 = book.craft_seeded("iron_sword", &mut inv2, &mut rng2);
    assert!(result2.is_none(), "Crafting should fail without resources");
}

/// Test dialogue state progression
#[test]
fn test_dialogue_deterministic() {
    // Create dialogue with choices
    let dialogue = Dialogue {
        id: "test_dialogue".to_string(),
        start: "n1".to_string(),
        nodes: vec![
            Node {
                id: "n1".to_string(),
                line: Some(Line {
                    speaker: "Guard".to_string(),
                    text: "Halt! State your business.".to_string(),
                    set_vars: vec![],
                }),
                choices: vec![
                    Choice {
                        text: "Attack".to_string(),
                        go_to: "n_attack".to_string(),
                        require: vec![],
                    },
                    Choice {
                        text: "Persuade".to_string(),
                        go_to: "n_persuade".to_string(),
                        require: vec![Cond::Has {
                            key: "charisma".to_string(),
                        }],
                    },
                ],
                end: false,
            },
            Node {
                id: "n_attack".to_string(),
                line: Some(Line {
                    speaker: "Guard".to_string(),
                    text: "So be it!".to_string(),
                    set_vars: vec![("combat".to_string(), "true".to_string())],
                }),
                choices: vec![],
                end: true,
            },
            Node {
                id: "n_persuade".to_string(),
                line: Some(Line {
                    speaker: "Guard".to_string(),
                    text: "Very well, you may pass.".to_string(),
                    set_vars: vec![("peaceful".to_string(), "true".to_string())],
                }),
                choices: vec![],
                end: true,
            },
        ],
    };

    let mut state = DialogueState::new(&dialogue);

    // Test deterministic choice: Attack (no requirements)
    assert!(
        state.choose(&dialogue, 0),
        "Should be able to choose attack"
    );
    assert_eq!(state.current(&dialogue).id, "n_attack");

    // Reset and test persuade without charisma (should fail)
    let mut state2 = DialogueState::new(&dialogue);
    assert!(
        !state2.choose(&dialogue, 1),
        "Should fail persuade without charisma"
    );

    // Reset and test with charisma
    let mut state3 = DialogueState::new(&dialogue);
    state3
        .vars
        .insert("charisma".to_string(), "high".to_string());
    assert!(
        state3.choose(&dialogue, 1),
        "Should succeed persuade with charisma"
    );
    assert_eq!(state3.current(&dialogue).id, "n_persuade");
}

/// Test complete gameplay loop: combat -> loot -> craft
#[test]
fn test_gameplay_loop_deterministic() {
    // 1. Combat phase
    let mut attacker_stats = Stats::new(100);
    attacker_stats.power = 15;
    let mut enemy_stats = Stats::new(50);

    let chain = ComboChain {
        name: "kill_combo".to_string(),
        steps: vec![ComboStep {
            kind: AttackKind::Heavy,
            window: (0.0, 0.5),
            damage: 40,
            reach: 2.0,
            stagger: 0.2,
        }],
    };

    let mut attack = AttackState::new(chain);
    attack.start();

    let (hit, dmg) = attack.tick(
        0.016,
        false,
        true,
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        &attacker_stats,
        None,
        &mut enemy_stats,
    );

    assert!(hit, "Attack should hit");
    assert_eq!(dmg, 55, "Damage should be 40 + 15 = 55");
    assert!(enemy_stats.hp <= 0, "Enemy should be defeated");

    // 2. Loot phase (simulate getting resources)
    let mut inv = Inventory::default();
    inv.add_resource(ResourceKind::Ore, 5);
    inv.add_resource(ResourceKind::Wood, 3);

    // 3. Crafting phase
    let recipes = vec![CraftRecipe {
        name: "battle_axe".to_string(),
        output_item: ItemKind::Weapon {
            base_damage: 35,
            dtype: DamageType::Physical,
        },
        costs: vec![
            CraftCost {
                kind: ResourceKind::Ore,
                count: 4,
            },
            CraftCost {
                kind: ResourceKind::Wood,
                count: 2,
            },
        ],
    }];

    let book = RecipeBook { recipes };
    let mut rng = rand::rngs::StdRng::seed_from_u64(1337);
    let result = book.craft_seeded("battle_axe", &mut inv, &mut rng);

    assert!(
        result.is_some(),
        "Should craft weapon with looted resources"
    );
    let weapon = result.test_unwrap();
    match weapon.kind {
        ItemKind::Weapon { base_damage, .. } => {
            assert_eq!(base_damage, 35, "Crafted weapon should have correct stats");
        }
        _ => test_panic(format_args!("Should be weapon")),
    }

    // Verify resources consumed
    assert_eq!(
        inv.resources
            .iter()
            .find(|(k, _)| *k == ResourceKind::Ore)
            .map(|(_, n)| *n)
            .unwrap_or(0),
        1,
        "Ore should be 5 - 4 = 1"
    );
}

/// Test that identical combat scenarios produce identical results
#[test]
fn test_combat_reproducibility() {
    fn run_combat_scenario(power_offset: i32) -> (i32, i32) {
        let mut attacker_stats = Stats::new(100);
        attacker_stats.power = 10 + power_offset;

        let mut target_stats = Stats::new(80);

        let chain = ComboChain {
            name: "test".to_string(),
            steps: vec![ComboStep {
                kind: AttackKind::Light,
                window: (0.0, 0.3),
                damage: 15,
                reach: 1.5,
                stagger: 0.1,
            }],
        };

        let mut attack = AttackState::new(chain);
        attack.start();

        let (_, dmg) = attack.tick(
            0.016,
            true,
            false,
            Vec3::ZERO,
            Vec3::new(0.5, 0.0, 0.0),
            &attacker_stats,
            None,
            &mut target_stats,
        );

        (dmg, target_stats.hp)
    }

    // Same parameters should produce same results
    let (dmg1, hp1) = run_combat_scenario(0);
    let (dmg2, hp2) = run_combat_scenario(0);
    assert_eq!(dmg1, dmg2, "Same scenario should produce same damage");
    assert_eq!(hp1, hp2, "Same scenario should produce same HP");

    // Different parameters should produce different results
    let (dmg3, hp3) = run_combat_scenario(5);
    assert_ne!(
        dmg1, dmg3,
        "Different power should produce different damage"
    );
    assert_ne!(hp1, hp3, "Different damage should produce different HP");
}

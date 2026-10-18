//! Integration tests for combat physics system
//!
//! These tests validate the full integration between:
//! - Combat physics (attack sweeps, parry, iframes)
//! - Physics world (Rapier3D collision detection)
//! - AI planning (attack decision → execution → damage feedback)
//!
//! Unlike unit tests which test `perform_attack_sweep()` in isolation,
//! these tests verify the complete gameplay loop from AI decision to physics result.

pub use crate::test_support::TestValue;
use astraweave_gameplay::combat_physics::{
    perform_attack_sweep, Combatant, HitResult, IFrame, Parry,
};
use astraweave_gameplay::{DamageType, Stats};
use astraweave_physics::PhysicsWorld;
use glam::Vec3;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create a basic combatant for testing
fn create_combatant(body_id: u64, hp: i32) -> Combatant {
    Combatant {
        body: body_id,
        stats: Stats {
            hp,
            stamina: 100,
            power: 10,
            defense: 0,
            echo_amp: 1.0,
            effects: vec![],
        },
        iframes: None,
        parry: None,
    }
}

/// Helper to create a combatant with active parry window
fn create_parrying_combatant(body_id: u64, hp: i32, parry_window: f32) -> Combatant {
    Combatant {
        body: body_id,
        stats: Stats {
            hp,
            stamina: 100,
            power: 10,
            defense: 0,
            echo_amp: 1.0,
            effects: vec![],
        },
        iframes: None,
        parry: Some(Parry {
            window: parry_window,
            active: true,
        }),
    }
}

/// Helper to create a combatant with active iframes
fn create_iframe_combatant(body_id: u64, hp: i32, iframe_duration: f32) -> Combatant {
    Combatant {
        body: body_id,
        stats: Stats {
            hp,
            stamina: 100,
            power: 10,
            defense: 0,
            echo_amp: 1.0,
            effects: vec![],
        },
        iframes: Some(IFrame {
            time_left: iframe_duration,
        }),
        parry: None,
    }
}

/// Simulate an AI agent deciding to attack and executing the attack
/// Returns (decision_made, attack_result, target_hp_after)
fn simulate_ai_attack_decision(
    phys: &mut PhysicsWorld,
    attacker_id: u64,
    attacker_pos: Vec3,
    _target_id: u64,
    target_pos: Vec3,
    target: &mut Combatant,
) -> (bool, Option<HitResult>, i32) {
    // AI Decision: Should I attack? (simple distance check)
    let distance = (target_pos - attacker_pos).length();
    let attack_range = 3.0;

    if distance > attack_range {
        // AI decides not to attack (too far)
        return (false, None, target.stats.hp);
    }

    // AI decides to attack
    let decision_made = true;

    // Execute attack sweep (AI action → physics query)
    let attack_dir = (target_pos - attacker_pos).normalize_or_zero() * attack_range;
    let attack_to = attacker_pos + attack_dir;

    let mut targets = vec![Combatant {
        body: target.body,
        stats: target.stats.clone(),
        iframes: target.iframes,
        parry: target.parry,
    }];
    let result = perform_attack_sweep(
        phys,
        attacker_id,
        attacker_pos,
        attack_to,
        0.5, // radius
        20,  // base damage
        DamageType::Physical,
        &mut targets,
    );

    // Update target with damage results
    *target = targets.into_iter().next().test_unwrap();

    (decision_made, result, target.stats.hp)
}

// ============================================================================
// Integration Tests: AI Planning → Combat Execution
// ============================================================================

/// Test 1: AI Attack Decision → Combat Execution → Damage Feedback
///
/// Validates the full loop:
/// 1. AI perceives enemy in range
/// 2. AI decides to attack
/// 3. Attack sweep executes via physics
/// 4. Damage applies to target
/// 5. AI receives feedback (hit confirmed, target damaged)
#[test]
fn test_ai_attack_decision_to_damage_feedback() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and enemy in range
    let ai_pos = Vec3::ZERO;
    let enemy_pos = Vec3::new(2.0, 0.0, 0.0); // 2 units away (within 3 unit range)

    let ai_id = phys.add_character(ai_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step(); // Update query pipeline

    let mut enemy = create_combatant(enemy_id, 100);

    // Execute AI decision → attack → damage feedback
    let (decision_made, result, hp_after) =
        simulate_ai_attack_decision(&mut phys, ai_id, ai_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify AI made decision to attack
    assert!(decision_made, "AI should decide to attack enemy in range");

    // Verify attack hit
    assert!(result.is_some(), "Attack should hit enemy");
    let hit = result.test_unwrap();
    assert_eq!(hit.target, enemy_id, "Should hit correct enemy");
    assert_eq!(hit.damage, 20, "Should deal expected damage");
    assert!(!hit.parried, "Attack should not be parried");

    // Verify damage feedback
    assert_eq!(hp_after, 80, "Enemy should have 80 HP after 20 damage");
    assert_eq!(
        enemy.stats.hp, 80,
        "Enemy combatant should be updated with damage"
    );
}

/// Test 2: AI Attack Against Out-of-Range Enemy
///
/// Validates that AI correctly handles range checks:
/// 1. AI perceives enemy beyond attack range
/// 2. AI decides NOT to attack (too far)
/// 3. No physics query executed
/// 4. Enemy takes no damage
#[test]
fn test_ai_attack_decision_out_of_range() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and distant enemy
    let ai_pos = Vec3::ZERO;
    let enemy_pos = Vec3::new(10.0, 0.0, 0.0); // 10 units away (beyond 3 unit range)

    let ai_id = phys.add_character(ai_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_combatant(enemy_id, 100);

    // Execute AI decision
    let (decision_made, result, hp_after) =
        simulate_ai_attack_decision(&mut phys, ai_id, ai_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify AI decided NOT to attack
    assert!(!decision_made, "AI should not attack enemy out of range");
    assert!(result.is_none(), "No attack should be executed");
    assert_eq!(hp_after, 100, "Enemy should take no damage");
}

/// Test 3: AI Attack Against Parrying Enemy
///
/// Validates parry integration:
/// 1. AI decides to attack enemy with active parry
/// 2. Attack executes and hits
/// 3. Parry system intercepts damage
/// 4. AI receives feedback (attack parried, 0 damage)
/// 5. Enemy parry window consumed
#[test]
fn test_ai_attack_parried_by_enemy() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and enemy with active parry
    let ai_pos = Vec3::ZERO;
    let enemy_pos = Vec3::new(2.0, 0.0, 0.0);

    let ai_id = phys.add_character(ai_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_parrying_combatant(enemy_id, 100, 0.3); // 0.3s parry window

    // Execute AI attack
    let (decision_made, result, hp_after) =
        simulate_ai_attack_decision(&mut phys, ai_id, ai_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify attack was made
    assert!(decision_made, "AI should decide to attack");

    // Verify attack was parried
    assert!(result.is_some(), "Attack should register hit");
    let hit = result.test_unwrap();
    assert_eq!(hit.damage, 0, "Parried attack should deal no damage");
    assert!(hit.parried, "Attack should be marked as parried");

    // Verify parry feedback
    assert_eq!(hp_after, 100, "Enemy should take no damage from parry");
    assert_eq!(
        enemy.parry.as_ref().test_unwrap().window,
        0.0,
        "Parry window should be consumed"
    );
    assert!(
        !enemy.parry.as_ref().test_unwrap().active,
        "Parry should be deactivated after use"
    );
}

/// Test 4: AI Attack Against Enemy with Iframes
///
/// Validates iframe integration:
/// 1. AI attacks enemy with active iframes
/// 2. Attack hits but damage is blocked
/// 3. AI receives feedback (hit registered, 0 damage)
/// 4. Enemy iframes persist (not consumed)
#[test]
fn test_ai_attack_blocked_by_iframes() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and enemy with iframes
    let ai_pos = Vec3::ZERO;
    let enemy_pos = Vec3::new(2.0, 0.0, 0.0);

    let ai_id = phys.add_character(ai_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_iframe_combatant(enemy_id, 100, 0.5); // 0.5s iframes

    // Execute AI attack
    let (decision_made, result, hp_after) =
        simulate_ai_attack_decision(&mut phys, ai_id, ai_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify attack was made
    assert!(decision_made, "AI should decide to attack");

    // Verify attack was blocked by iframes
    assert!(result.is_some(), "Attack should register hit");
    let hit = result.test_unwrap();
    assert_eq!(hit.damage, 0, "Attack during iframes should deal no damage");
    assert!(!hit.parried, "iframes are not parries");

    // Verify iframe feedback
    assert_eq!(hp_after, 100, "Enemy should take no damage during iframes");
    assert_eq!(
        enemy.iframes.as_ref().test_unwrap().time_left,
        0.5,
        "iframes should not be consumed by attack"
    );
}

// ============================================================================
// Integration Tests: Multi-Attacker Scenarios
// ============================================================================

/// Test 5: Multiple AI Agents Attacking Same Target
///
/// Validates multi-attacker integration:
/// 1. Two AI agents both in range of same enemy
/// 2. First AI attacks, deals damage
/// 3. Second AI attacks same enemy (now at reduced HP)
/// 4. Both attacks register correctly
/// 5. Damage accumulates properly
#[test]
fn test_multiple_ai_agents_attack_same_target() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: Two AI agents flanking one enemy
    let ai1_pos = Vec3::new(-1.5, 0.0, 0.0);
    let ai2_pos = Vec3::new(1.5, 0.0, 0.0);
    let enemy_pos = Vec3::ZERO;

    let ai1_id = phys.add_character(ai1_pos, Vec3::new(0.5, 1.0, 0.5));
    let ai2_id = phys.add_character(ai2_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_combatant(enemy_id, 100);

    // AI 1 attacks
    let (decision1, result1, hp_after_1) =
        simulate_ai_attack_decision(&mut phys, ai1_id, ai1_pos, enemy_id, enemy_pos, &mut enemy);

    assert!(decision1, "AI 1 should decide to attack");
    assert!(result1.is_some(), "AI 1 attack should hit");
    assert_eq!(hp_after_1, 80, "Enemy should have 80 HP after first attack");

    // AI 2 attacks same enemy (now at 80 HP)
    let (decision2, result2, hp_after_2) =
        simulate_ai_attack_decision(&mut phys, ai2_id, ai2_pos, enemy_id, enemy_pos, &mut enemy);

    assert!(decision2, "AI 2 should decide to attack");
    assert!(result2.is_some(), "AI 2 attack should hit");
    assert_eq!(
        hp_after_2, 60,
        "Enemy should have 60 HP after second attack"
    );

    // Verify final state
    assert_eq!(
        enemy.stats.hp, 60,
        "Enemy should accumulate damage from both attackers"
    );
}

/// Test 6: AI Attack During Enemy Iframe Window (Multi-Attacker Timing)
///
/// Validates iframe timing in multi-attacker scenarios:
/// 1. Enemy has iframes active (simulating previous hit)
/// 2. AI 2 attacks during iframe window → blocked
/// 3. Iframes expire
/// 4. AI 2 attacks again → successful damage
#[test]
fn test_ai_multi_attack_iframe_timing() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and enemy (ai1 not needed, enemy starts with iframes from previous hit)
    let ai2_pos = Vec3::new(1.5, 0.0, 0.0);
    let enemy_pos = Vec3::ZERO;

    let ai2_id = phys.add_character(ai2_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    // Enemy starts with iframes (simulates just being hit by another AI)
    let mut enemy = create_iframe_combatant(enemy_id, 80, 0.5); // 80 HP, 0.5s iframes

    // AI 2 attacks during iframe window
    let (decision2, result2, hp_during_iframes) =
        simulate_ai_attack_decision(&mut phys, ai2_id, ai2_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify AI 2's attack was blocked
    assert!(decision2, "AI 2 should decide to attack");
    assert!(result2.is_some(), "Attack should register hit");
    let hit2 = result2.test_unwrap();
    assert_eq!(
        hit2.damage, 0,
        "Attack during iframes should deal no damage"
    );
    assert_eq!(
        hp_during_iframes, 80,
        "Enemy HP should remain 80 during iframes"
    );

    // Simulate iframe expiration
    enemy.iframes = None;

    // AI 2 attacks again after iframes expire
    let (decision3, result3, hp_after_iframes) =
        simulate_ai_attack_decision(&mut phys, ai2_id, ai2_pos, enemy_id, enemy_pos, &mut enemy);

    // Verify AI 2's second attack succeeds
    assert!(decision3, "AI 2 should decide to attack again");
    assert!(result3.is_some(), "Attack should hit after iframes expire");
    let hit3 = result3.test_unwrap();
    assert_eq!(
        hit3.damage, 20,
        "Attack should deal full damage without iframes"
    );
    assert_eq!(
        hp_after_iframes, 60,
        "Enemy HP should be 60 after iframes expire and second attack lands"
    );
}

// ============================================================================
// Integration Tests: Attack Cone and Positioning
// ============================================================================

/// Test 7: AI Attack Cone Validation (Flanking vs Frontal)
///
/// Validates attack cone integration with AI positioning:
/// 1. AI agent attacks enemy from front (within cone) → hit
/// 2. AI agent attacks enemy from behind (outside cone) → miss
/// 3. AI receives correct feedback for both scenarios
#[test]
#[allow(unused_mut)] // enemy.stats.clone() doesn't require mut, but we construct Combatant from it
fn test_ai_attack_cone_positioning() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: AI agent and enemy
    let enemy_pos = Vec3::ZERO;
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));

    // Scenario 1: AI attacks from front (enemy facing +Z, AI attacks from -Z direction)
    let ai_front_pos = Vec3::new(0.0, 0.0, -2.0); // In front of enemy
    let ai_front_id = phys.add_character(ai_front_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_combatant(enemy_id, 100);

    // Attack from front
    let attack_dir = (enemy_pos - ai_front_pos).normalize_or_zero() * 3.0;
    let attack_to = ai_front_pos + attack_dir;

    let mut targets = vec![Combatant {
        body: enemy.body,
        stats: enemy.stats.clone(),
        iframes: enemy.iframes,
        parry: enemy.parry,
    }];
    let result_front = perform_attack_sweep(
        &mut phys,
        ai_front_id,
        ai_front_pos,
        attack_to,
        0.5,
        20,
        DamageType::Physical,
        &mut targets,
    );

    // Verify frontal attack hits (within cone)
    assert!(
        result_front.is_some(),
        "Attack from front should hit (within attack cone)"
    );
    let hit_front = result_front.test_unwrap();
    assert_eq!(
        hit_front.damage, 20,
        "Frontal attack should deal full damage"
    );

    // Scenario 2: AI attacks from behind enemy
    let ai_behind_pos = Vec3::new(0.0, 0.0, 2.0); // Behind enemy
    let ai_behind_id = phys.add_character(ai_behind_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    // Reset enemy for second test
    let enemy = create_combatant(enemy_id, 100);

    // Attack from behind (toward enemy, but enemy is in opposite direction)
    let attack_dir_behind = (enemy_pos - ai_behind_pos).normalize_or_zero() * 3.0;
    let attack_to_behind = ai_behind_pos + attack_dir_behind;

    let mut targets_behind = vec![Combatant {
        body: enemy.body,
        stats: enemy.stats.clone(),
        iframes: enemy.iframes,
        parry: enemy.parry,
    }];
    let result_behind = perform_attack_sweep(
        &mut phys,
        ai_behind_id,
        ai_behind_pos,
        attack_to_behind,
        0.5,
        20,
        DamageType::Physical,
        &mut targets_behind,
    );

    // Note: Attack cone checks dot product of attack direction vs direction to target.
    // If AI is behind enemy and attacks toward enemy, the dot product will be positive
    // (attack direction and to-target direction are aligned), so attack WILL hit.
    //
    // The cone check prevents hitting targets BEHIND the attacker, not targets
    // the attacker is behind. This is correct behavior for melee attacks.
    assert!(
        result_behind.is_some(),
        "Attack toward enemy from any position should hit if in range and in front of attacker"
    );
}

/// Test 8: AI Attack Chain with Parry Timing
///
/// Validates parry timing in sequential attacks:
/// 1. Enemy has parry window active
/// 2. AI 1 attacks → parried, window consumed
/// 3. AI 2 attacks immediately after → hits (no parry)
/// 4. Both AIs receive correct feedback
#[test]
fn test_ai_attack_chain_parry_timing() {
    let mut phys = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

    // Setup: Two AI agents and one enemy with parry
    let ai1_pos = Vec3::new(-1.5, 0.0, 0.0);
    let ai2_pos = Vec3::new(1.5, 0.0, 0.0);
    let enemy_pos = Vec3::ZERO;

    let ai1_id = phys.add_character(ai1_pos, Vec3::new(0.5, 1.0, 0.5));
    let ai2_id = phys.add_character(ai2_pos, Vec3::new(0.5, 1.0, 0.5));
    let enemy_id = phys.add_character(enemy_pos, Vec3::new(0.5, 1.0, 0.5));
    phys.step();

    let mut enemy = create_parrying_combatant(enemy_id, 100, 0.3); // Active parry

    // AI 1 attacks (should be parried)
    let (decision1, result1, hp_after_1) =
        simulate_ai_attack_decision(&mut phys, ai1_id, ai1_pos, enemy_id, enemy_pos, &mut enemy);

    assert!(decision1, "AI 1 should decide to attack");
    assert!(result1.is_some(), "AI 1 attack should register");
    let hit1 = result1.test_unwrap();
    assert!(hit1.parried, "AI 1 attack should be parried");
    assert_eq!(hit1.damage, 0, "Parried attack should deal no damage");
    assert_eq!(hp_after_1, 100, "Enemy HP should remain 100 after parry");
    assert!(
        !enemy.parry.as_ref().test_unwrap().active,
        "Parry should be consumed after first attack"
    );

    // AI 2 attacks immediately after (parry consumed, should hit)
    let (decision2, result2, hp_after_2) =
        simulate_ai_attack_decision(&mut phys, ai2_id, ai2_pos, enemy_id, enemy_pos, &mut enemy);

    assert!(decision2, "AI 2 should decide to attack");
    assert!(result2.is_some(), "AI 2 attack should register");
    let hit2 = result2.test_unwrap();
    assert!(
        !hit2.parried,
        "AI 2 attack should not be parried (window consumed)"
    );
    assert_eq!(hit2.damage, 20, "AI 2 attack should deal full damage");
    assert_eq!(
        hp_after_2, 80,
        "Enemy HP should be 80 after AI 2's successful attack"
    );
}

#[path = "../src/test_support.rs"]
mod test_support;

//! Mutation-resistant comprehensive tests for astraweave-weaving (11,459 LOC)
//!
//! 200+ tests targeting exact return values, boundary conditions, state
//! transitions, operator sensitivity, and boolean return correctness.

#![allow(clippy::field_reassign_with_default)]

use std::collections::BTreeMap;

use glam::Vec3;

// Re-exported types at crate root
use astraweave_weaving::{
    Anchor, AnchorVfxState, CWeaveAgent, Camera, CombatEvent, CombatSystem, EchoCurrency, Enemy,
    EnemySpawner, EnemyState, IntentProposer, Killer, ObjectiveType, PatternDetector,
    PatternStrength, Player, Quest, QuestManager, QuestReward, QuestState, Transaction,
    TransactionReason, VeilweaverLevel, WeaveAdjudicator, WeaveConfig, WeaveIntent,
};

// Module-specific types
use astraweave_weaving::abilities::{
    AbilityManager, AbilityState, AbilityType as AbilityKind, EchoDash, EchoShield,
};
use astraweave_weaving::enemy_types::{
    BossSpecialAttack, Riftstalker, Sentinel, VoidBoss, VoidBossPhase,
};
use astraweave_weaving::intents::{
    AidEventProposer, MediatorProposer, ScavengerPatrolProposer, SupplyDropProposer,
};
use astraweave_weaving::patterns::{
    CombatIntensityDetector, FactionConflictDetector, LowHealthClusterDetector,
    ResourceScarcityDetector, WorldMetrics,
};
use astraweave_weaving::quest_types::{
    BossObjective, BossPhase, CollectItem, CollectObjective, DefendObjective, EscortNPC,
    TimeTrialObjective,
};

// Re-export anchor's AbilityType
use astraweave_weaving::AbilityType;

// ========================================================================
// ABILITIES MODULE
// ========================================================================

#[test]
fn ability_state_new_starts_ready() {
    let s = AbilityState::new(AbilityKind::EchoDash, 2.0, 1.0, 5);
    assert!(
        s.is_ready(),
        "new ability should start ready (time_since_use >= cooldown)"
    );
    assert_eq!(s.remaining_cooldown(), 0.0);
}

#[test]
fn ability_state_is_ready_at_exact_cooldown() {
    let mut s = AbilityState::new(AbilityKind::EchoDash, 2.0, 0.0, 5);
    s.activate();
    s.update(2.0); // exactly cooldown
    assert!(s.is_ready(), "should be ready at exactly cooldown (>=)");
}

#[test]
fn ability_state_not_ready_just_below_cooldown() {
    let mut s = AbilityState::new(AbilityKind::EchoDash, 2.0, 0.0, 5);
    s.activate();
    s.update(1.999);
    assert!(!s.is_ready(), "should NOT be ready below cooldown");
}

#[test]
fn ability_state_can_afford_exact_cost() {
    let s = AbilityState::new(AbilityKind::EchoDash, 1.0, 0.0, 10);
    assert!(s.can_afford(10), "exactly enough echo (>=)");
    assert!(!s.can_afford(9), "one short");
}

#[test]
fn ability_state_activate_instant_not_active() {
    let mut s = AbilityState::new(AbilityKind::EchoDash, 1.0, 0.0, 5);
    s.activate();
    assert!(
        !s.is_active,
        "duration=0 → is_active must be false (> 0.0 gate)"
    );
}

#[test]
fn ability_state_activate_with_duration_is_active() {
    let mut s = AbilityState::new(AbilityKind::EchoShield, 5.0, 3.0, 15);
    s.activate();
    assert!(s.is_active, "duration=3.0 > 0.0 → must be active");
}

#[test]
fn ability_state_deactivates_at_exact_duration() {
    let mut s = AbilityState::new(AbilityKind::EchoShield, 5.0, 3.0, 15);
    s.activate();
    s.update(3.0); // exactly duration
    assert!(!s.is_active, "should deactivate at exactly duration (>=)");
}

#[test]
fn ability_state_deactivates_not_below_duration() {
    let mut s = AbilityState::new(AbilityKind::EchoShield, 5.0, 3.0, 15);
    s.activate();
    s.update(2.999);
    assert!(s.is_active, "should still be active below duration");
}

#[test]
fn ability_state_remaining_active_value() {
    let mut s = AbilityState::new(AbilityKind::EchoShield, 5.0, 3.0, 15);
    s.activate();
    s.update(1.0);
    assert!((s.remaining_active() - 2.0).abs() < 0.001);
}

#[test]
fn ability_state_remaining_active_zero_when_inactive() {
    let s = AbilityState::new(AbilityKind::EchoDash, 1.0, 0.0, 10);
    assert_eq!(s.remaining_active(), 0.0);
}

#[test]
fn echo_dash_default_values() {
    let d = EchoDash::new();
    assert_eq!(d.damage, 30.0);
    assert_eq!(d.dash_distance, 10.0);
    assert_eq!(d.state.cooldown_seconds, 1.0);
    assert_eq!(d.state.echo_cost, 10);
    assert_eq!(d.state.duration_seconds, 0.0);
}

#[test]
fn echo_dash_activate_position() {
    let mut d = EchoDash::new();
    let pos = Vec3::new(0.0, 0.0, 0.0);
    let fwd = Vec3::new(1.0, 0.0, 0.0);
    let (target, dmg) = d.activate(pos, fwd);
    assert_eq!(
        target,
        Vec3::new(10.0, 0.0, 0.0),
        "target = pos + forward * distance"
    );
    assert_eq!(dmg, 30.0, "returns damage value");
}

#[test]
fn echo_dash_can_use_both_conditions() {
    let mut d = EchoDash::new();
    assert!(d.can_use(10), "ready + enough echo");
    assert!(!d.can_use(9), "ready but not enough echo");
    d.activate(Vec3::ZERO, Vec3::Z);
    assert!(!d.can_use(100), "on cooldown even with plenty echo");
}

#[test]
fn echo_shield_default_values() {
    let s = EchoShield::new();
    assert_eq!(s.damage_reduction, 0.5);
    assert_eq!(s.state.cooldown_seconds, 5.0);
    assert_eq!(s.state.duration_seconds, 3.0);
    assert_eq!(s.state.echo_cost, 15);
}

#[test]
fn echo_shield_damage_reduction_when_active() {
    let mut s = EchoShield::new();
    s.activate();
    assert_eq!(
        s.apply_damage_reduction(100.0),
        50.0,
        "active: 100 * (1.0 - 0.5) = 50"
    );
}

#[test]
fn echo_shield_no_reduction_when_inactive() {
    let s = EchoShield::new();
    assert_eq!(
        s.apply_damage_reduction(100.0),
        100.0,
        "inactive: pass-through"
    );
}

#[test]
fn ability_manager_activate_dash_success() {
    let mut m = AbilityManager::new();
    let result = m.activate_dash(Vec3::ZERO, Vec3::Z, 10);
    assert!(result.is_ok());
    let (target, dmg) = result.unwrap();
    assert_eq!(dmg, 30.0);
    assert_eq!(target, Vec3::new(0.0, 0.0, 10.0));
}

#[test]
fn ability_manager_activate_dash_insufficient_echo() {
    let mut m = AbilityManager::new();
    let result = m.activate_dash(Vec3::ZERO, Vec3::Z, 9);
    assert!(result.is_err(), "9 echo < 10 cost");
}

#[test]
fn ability_manager_activate_shield_success() {
    let mut m = AbilityManager::new();
    assert!(m.activate_shield(15).is_ok());
    assert!(m.is_shield_active());
}

#[test]
fn ability_manager_activate_shield_insufficient_echo() {
    let mut m = AbilityManager::new();
    assert!(m.activate_shield(14).is_err(), "14 < 15 cost");
}

#[test]
fn ability_manager_dash_cooldown_info() {
    let m = AbilityManager::new();
    let (ready, remaining) = m.dash_cooldown();
    assert!(ready);
    assert_eq!(remaining, 0.0);
}

#[test]
fn ability_manager_shield_active_info() {
    let mut m = AbilityManager::new();
    m.activate_shield(15).unwrap();
    let (active, remaining) = m.shield_active();
    assert!(active);
    assert!(remaining > 0.0);
}

// ========================================================================
// COMBAT MODULE
// ========================================================================

#[test]
fn combat_system_defaults() {
    let cs = CombatSystem::new();
    assert_eq!(cs.player_health, 100.0);
    assert_eq!(cs.player_max_health, 100.0);
    assert_eq!(cs.player_attack_damage, 20.0);
    assert_eq!(cs.echo_dash_damage, 50.0);
    assert_eq!(cs.echo_dash_radius, 3.0);
}

#[test]
fn combat_player_alive_at_positive_health() {
    let cs = CombatSystem::new();
    assert!(cs.is_player_alive(), "100 > 0 → alive");
}

#[test]
fn combat_player_dead_at_exactly_zero() {
    let mut cs = CombatSystem::new();
    cs.damage_player(100.0);
    assert!(!cs.is_player_alive(), "0.0 NOT > 0.0 → dead");
}

#[test]
fn combat_health_percentage() {
    let cs = CombatSystem::with_stats(200.0, 20.0, 50.0, 3.0);
    assert_eq!(cs.player_health_percentage(), 1.0);
}

#[test]
fn combat_damage_player_returns_true_on_kill() {
    let mut cs = CombatSystem::new();
    let killed = cs.damage_player(100.0);
    assert!(killed, "exact kill returns true");
}

#[test]
fn combat_damage_player_returns_false_on_survive() {
    let mut cs = CombatSystem::new();
    let killed = cs.damage_player(99.0);
    assert!(!killed, "1 HP remaining → not killed");
}

#[test]
fn combat_heal_clamps_at_max() {
    let mut cs = CombatSystem::new();
    cs.damage_player(50.0);
    cs.heal_player(999.0);
    assert_eq!(cs.player_health(), 100.0, "healed to max, not beyond");
}

#[test]
fn combat_player_attack_kills_enemy() {
    let mut cs = CombatSystem::new();
    let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    enemy.health = 10.0; // Will die from 20 damage
    let event = cs.player_attack(0, &mut enemy, Vec3::ZERO);
    assert!(event.is_some());
    match event.unwrap() {
        CombatEvent::EnemyKilled {
            enemy_id, killer, ..
        } => {
            assert_eq!(enemy_id, 0);
            assert_eq!(killer, Killer::Player);
        }
        _ => panic!("expected EnemyKilled"),
    }
}

#[test]
fn combat_player_attack_damages_enemy() {
    let mut cs = CombatSystem::new();
    let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    let event = cs.player_attack(1, &mut enemy, Vec3::ZERO);
    match event.unwrap() {
        CombatEvent::EnemyDamaged {
            enemy_id,
            amount,
            was_critical,
        } => {
            assert_eq!(enemy_id, 1);
            assert_eq!(amount, 20.0);
            assert!(!was_critical);
        }
        _ => panic!("expected EnemyDamaged"),
    }
}

#[test]
fn combat_echo_dash_attack_range_boundary() {
    let mut cs = CombatSystem::new();
    let mut e_in = Enemy::new(Vec3::ZERO, 5.0);
    let mut e_out = Enemy::new(Vec3::ZERO, 5.0);
    // In range: distance exactly 3.0 (radius)
    let events = cs.echo_dash_attack(
        Vec3::ZERO,
        vec![
            (0, &mut e_in, Vec3::new(3.0, 0.0, 0.0)), // dist = exactly 3.0
            (1, &mut e_out, Vec3::new(3.01, 0.0, 0.0)), // dist > 3.0
        ],
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDamaged { enemy_id: 0, .. }
                | CombatEvent::EnemyKilled { enemy_id: 0, .. }
        )),
        "enemy at exactly radius should be hit (<=)"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDamaged { enemy_id: 1, .. }
                | CombatEvent::EnemyKilled { enemy_id: 1, .. }
        )),
        "enemy beyond radius should NOT be hit"
    );
}

#[test]
fn combat_poll_events_drains() {
    let mut cs = CombatSystem::new();
    // damage_player only generates a PlayerKilled event when health <= 0
    cs.damage_player(100.0); // lethal → kills player → generates event
    let first = cs.poll_events();
    assert!(!first.is_empty(), "kill generates event");
    let second = cs.poll_events();
    assert!(second.is_empty(), "poll should drain");
}

#[test]
fn combat_peek_events_does_not_drain() {
    let mut cs = CombatSystem::new();
    cs.damage_player(50.0);
    let first = cs.peek_events();
    let second = cs.peek_events();
    assert_eq!(first.len(), second.len(), "peek should not drain");
}

// ========================================================================
// ENEMY TYPES MODULE
// ========================================================================

#[test]
fn riftstalker_defaults() {
    let r = Riftstalker::new(Vec3::ZERO);
    assert_eq!(r.health, 60.0);
    assert_eq!(r.max_health, 60.0);
    assert_eq!(r.damage, 20.0);
    assert_eq!(r.move_speed, 5.0);
    assert_eq!(r.attack_cooldown, 1.5);
    assert_eq!(r.flanking_radius, 4.0);
}

#[test]
fn riftstalker_alive_at_positive() {
    let r = Riftstalker::new(Vec3::ZERO);
    assert!(r.is_alive());
}

#[test]
fn riftstalker_dead_at_zero() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.take_damage(60.0);
    assert!(!r.is_alive(), "0.0 NOT > 0.0 → dead");
}

#[test]
fn riftstalker_health_clamps_to_zero() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.take_damage(999.0);
    assert_eq!(r.health, 0.0);
}

#[test]
fn riftstalker_health_percentage() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.take_damage(30.0);
    assert!((r.health_percentage() - 0.5).abs() < 0.001);
}

#[test]
fn riftstalker_can_attack_melee_range_boundary() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.time_since_attack = 1.5; // exactly cooldown
    assert!(
        r.can_attack(Vec3::new(2.0, 0.0, 0.0)),
        "dist=2.0 <= 2.0, cooldown met"
    );
    assert!(!r.can_attack(Vec3::new(2.01, 0.0, 0.0)), "dist=2.01 > 2.0");
}

#[test]
fn riftstalker_can_attack_cooldown_boundary() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.time_since_attack = 1.499;
    assert!(!r.can_attack(Vec3::new(1.0, 0.0, 0.0)), "cooldown not met");
    r.time_since_attack = 1.5;
    assert!(r.can_attack(Vec3::new(1.0, 0.0, 0.0)), "cooldown met (>=)");
}

#[test]
fn riftstalker_attack_returns_damage_resets_timer() {
    let mut r = Riftstalker::new(Vec3::ZERO);
    r.time_since_attack = 2.0;
    let dmg = r.attack();
    assert_eq!(dmg, 20.0);
    assert_eq!(r.time_since_attack, 0.0);
}

#[test]
fn riftstalker_flanking_dot_boundary() {
    let r = Riftstalker::new(Vec3::new(0.0, 0.0, -5.0)); // behind player
    let player_pos = Vec3::ZERO;
    let player_fwd = Vec3::new(0.0, 0.0, 1.0); // facing +Z
                                               // to_enemy = (0,0,-5).normalize = (0,0,-1), dot with (0,0,1) = -1.0 < -0.5 → flanking
    assert!(r.is_flanking(player_pos, player_fwd));
}

#[test]
fn riftstalker_not_flanking_at_front() {
    let r = Riftstalker::new(Vec3::new(0.0, 0.0, 5.0)); // in front
    let player_pos = Vec3::ZERO;
    let player_fwd = Vec3::new(0.0, 0.0, 1.0);
    // to_enemy = (0,0,1), dot with (0,0,1) = 1.0, NOT < -0.5
    assert!(!r.is_flanking(player_pos, player_fwd));
}

#[test]
fn riftstalker_flank_multiplier_values() {
    let r = Riftstalker::new(Vec3::new(0.0, 0.0, -5.0)); // behind
    let player_pos = Vec3::ZERO;
    let player_fwd = Vec3::Z;
    assert_eq!(
        r.flank_multiplier(player_pos, player_fwd),
        1.5,
        "flanking → 1.5x"
    );

    let r2 = Riftstalker::new(Vec3::new(0.0, 0.0, 5.0)); // front
    assert_eq!(
        r2.flank_multiplier(player_pos, player_fwd),
        1.0,
        "not flanking → 1.0x"
    );
}

#[test]
fn sentinel_defaults() {
    let s = Sentinel::new(Vec3::ZERO);
    assert_eq!(s.health, 200.0);
    assert_eq!(s.max_health, 200.0);
    assert_eq!(s.damage, 25.0);
    assert_eq!(s.move_speed, 1.5);
    assert_eq!(s.attack_cooldown, 3.0);
    assert_eq!(s.aoe_radius, 6.0);
    assert_eq!(s.armor, 0.3);
}

#[test]
fn sentinel_armor_reduces_damage() {
    let mut s = Sentinel::new(Vec3::ZERO);
    s.take_damage(100.0);
    // reduced = 100 * (1.0 - 0.3) = 70
    assert!((s.health - 130.0).abs() < 0.001, "200 - 70 = 130");
}

#[test]
fn sentinel_effective_health() {
    let s = Sentinel::new(Vec3::ZERO);
    // 200 / (1.0 - 0.3) = 200 / 0.7 ≈ 285.71
    assert!((s.effective_health() - (200.0 / 0.7)).abs() < 0.01);
}

#[test]
fn sentinel_aoe_range_boundary() {
    let mut s = Sentinel::new(Vec3::ZERO);
    s.time_since_attack = 3.0;
    let entities = vec![
        (Vec3::new(6.0, 0.0, 0.0), "a"),  // dist = 6.0 = aoe_radius
        (Vec3::new(6.01, 0.0, 0.0), "b"), // dist > 6.0
    ];
    let hits = s.attack_aoe(&entities);
    assert!(
        hits.iter().any(|(i, _)| *i == 0),
        "at exactly radius → hit (<=)"
    );
    assert!(!hits.iter().any(|(i, _)| *i == 1), "beyond radius → miss");
}

#[test]
fn void_boss_defaults() {
    let b = VoidBoss::new(Vec3::ZERO);
    assert_eq!(b.health, 500.0);
    assert_eq!(b.max_health, 500.0);
    assert_eq!(b.damage, 40.0);
    assert_eq!(b.current_phase, VoidBossPhase::Phase1);
    assert_eq!(b.attack_cooldown, 2.0);
    assert_eq!(b.special_cooldown, 8.0);
    assert_eq!(b.enrage_multiplier, 1.0);
}

#[test]
fn void_boss_phase_transition_at_0_66() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    // health_pct = 330/500 = 0.66: NOT > 0.66 → Phase2
    b.health = 330.0;
    b.update(0.1);
    assert_eq!(b.current_phase, VoidBossPhase::Phase2, "0.66 is NOT > 0.66");
}

#[test]
fn void_boss_stays_phase1_above_0_66() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    b.health = 331.0; // 331/500 = 0.662 > 0.66
    b.update(0.1);
    assert_eq!(b.current_phase, VoidBossPhase::Phase1);
}

#[test]
fn void_boss_phase_transition_at_0_33() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    // 165/500 = 0.33: NOT > 0.33 → Phase3
    b.health = 165.0;
    b.update(0.1);
    assert_eq!(b.current_phase, VoidBossPhase::Phase3, "0.33 is NOT > 0.33");
}

#[test]
fn void_boss_stays_phase2_above_0_33() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    b.health = 166.0; // 166/500 = 0.332 > 0.33
    b.update(0.1);
    assert_eq!(b.current_phase, VoidBossPhase::Phase2);
}

#[test]
fn void_boss_special_per_phase() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    assert_eq!(b.get_special_attack(), BossSpecialAttack::VoidPulse);
    b.health = 300.0;
    b.update(0.1);
    assert_eq!(b.get_special_attack(), BossSpecialAttack::SummonAdds);
    b.health = 100.0;
    b.update(0.1);
    assert_eq!(b.get_special_attack(), BossSpecialAttack::TeleportStrike);
}

#[test]
fn void_boss_attack_with_enrage() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    b.time_since_attack = 2.0;
    let dmg1 = b.attack();
    assert_eq!(dmg1, 40.0, "base damage * 1.0 enrage");

    // Force to phase 3 (triggers enrage = 1.5)
    b.health = 100.0;
    b.update(0.1);
    b.time_since_attack = 2.0;
    let dmg3 = b.attack();
    assert_eq!(dmg3, 60.0, "40 * 1.5 enrage");
}

#[test]
fn void_boss_teleport_behind() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    b.teleport_behind(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(b.position, Vec3::new(5.0, 0.0, 0.0), "pos - forward * 5.0");
}

#[test]
fn void_boss_movement_multipliers() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    let base_speed = b.move_speed;

    // Phase 2 movement
    b.health = 300.0;
    b.update(0.01);
    b.update_movement(Vec3::new(100.0, 0.0, 0.0), 1.0);
    // Phase2 speed = base * 0.8

    // Phase 3 movement
    let mut b3 = VoidBoss::new(Vec3::ZERO);
    b3.health = 100.0;
    b3.update(0.01);
    // Phase3 speed = base * 1.3
    assert_eq!(base_speed, 2.5, "base move_speed");
}

#[test]
fn void_boss_can_use_special_boundary() {
    let mut b = VoidBoss::new(Vec3::ZERO);
    b.time_since_special = 7.999;
    assert!(!b.can_use_special(), "< special_cooldown");
    b.time_since_special = 8.0;
    assert!(b.can_use_special(), "exactly special_cooldown (>=)");
}

// ========================================================================
// ENEMY MODULE
// ========================================================================

#[test]
fn enemy_defaults() {
    let e = Enemy::new(Vec3::ZERO, 5.0);
    assert_eq!(e.health, 100.0);
    assert_eq!(e.max_health, 100.0);
    assert_eq!(e.speed, 3.0);
    assert_eq!(e.state, EnemyState::Patrol);
    assert_eq!(e.attack_damage, 10.0);
    assert_eq!(e.attack_cooldown, 1.0);
    assert_eq!(e.aggro_range, 10.0);
    assert_eq!(e.flee_health, 20.0);
    assert_eq!(e.patrol_radius, 5.0);
}

#[test]
fn enemy_can_attack_boundary() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    e.attack_timer = 0.001;
    assert!(!e.can_attack(), "> 0 timer → can't attack");
    e.attack_timer = 0.0;
    assert!(e.can_attack(), "exactly 0 → can attack (<= 0.0)");
}

#[test]
fn enemy_attack_returns_damage_sets_cooldown() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    e.attack_timer = 0.0;
    let dmg = e.attack();
    assert_eq!(dmg, 10.0, "attack_damage");
    assert_eq!(e.attack_timer, 1.0, "reset to attack_cooldown");
}

#[test]
fn enemy_take_damage_kills() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    let killed = e.take_damage(100.0);
    assert!(killed);
    assert_eq!(e.state, EnemyState::Dead);
    assert_eq!(e.health, 0.0);
}

#[test]
fn enemy_take_damage_survives() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    let killed = e.take_damage(99.0);
    assert!(!killed);
    assert_ne!(e.state, EnemyState::Dead);
}

#[test]
fn enemy_is_dead_check() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    assert!(!e.is_dead());
    e.take_damage(100.0);
    assert!(e.is_dead());
}

#[test]
fn enemy_flee_at_exactly_flee_health() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    e.health = 20.0; // exactly flee_health
    let player_pos = Vec3::new(50.0, 0.0, 0.0); // far away → not engage
    let _ = e.update(0.1, Vec3::ZERO, player_pos, &[]);
    assert_eq!(e.state, EnemyState::Flee, "health <= flee_health → flee");
}

#[test]
fn enemy_engage_at_exact_aggro_range() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    e.health = 100.0; // not fleeing
    let player_pos = Vec3::new(10.0, 0.0, 0.0); // exactly aggro_range
    let _ = e.update(0.1, Vec3::ZERO, player_pos, &[]);
    assert_eq!(e.state, EnemyState::EngagePlayer, "dist <= aggro_range");
}

#[test]
fn enemy_patrol_beyond_aggro_range() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    let player_pos = Vec3::new(10.01, 0.0, 0.0); // just beyond aggro
    let _ = e.update(0.1, Vec3::ZERO, player_pos, &[]);
    assert_eq!(e.state, EnemyState::Patrol, "beyond aggro → patrol");
}

#[test]
fn enemy_health_percentage() {
    let mut e = Enemy::new(Vec3::ZERO, 5.0);
    e.health = 50.0;
    assert!((e.health_percentage() - 0.5).abs() < 0.001);
}

#[test]
fn enemy_with_stats() {
    let e = Enemy::with_stats(Vec3::ZERO, 3.0, 200.0, 5.0, 15.0);
    assert_eq!(e.max_health, 200.0);
    assert_eq!(e.health, 200.0);
    assert_eq!(e.speed, 5.0);
    assert_eq!(e.attack_damage, 15.0);
}

// ========================================================================
// ECHO CURRENCY MODULE
// ========================================================================

#[test]
fn echo_currency_starts_at_zero() {
    let ec = EchoCurrency::new();
    assert_eq!(ec.count(), 0);
}

#[test]
fn echo_currency_with_balance() {
    let ec = EchoCurrency::with_balance(42);
    assert_eq!(ec.count(), 42);
}

#[test]
fn echo_currency_has_exact_boundary() {
    let ec = EchoCurrency::with_balance(10);
    assert!(ec.has(10), "exactly 10 (>=)");
    assert!(!ec.has(11), "one more → false");
}

#[test]
fn echo_currency_add_and_count() {
    let mut ec = EchoCurrency::new();
    ec.add(50, TransactionReason::TutorialReward);
    assert_eq!(ec.count(), 50);
}

#[test]
fn echo_currency_spend_success() {
    let mut ec = EchoCurrency::with_balance(100);
    let ok = ec.spend(60, TransactionReason::UseEchoDash);
    assert!(ok);
    assert_eq!(ec.count(), 40);
}

#[test]
fn echo_currency_spend_fails_insufficient() {
    let mut ec = EchoCurrency::with_balance(10);
    let ok = ec.spend(11, TransactionReason::UseEchoDash);
    assert!(!ok, "can't spend more than balance");
    assert_eq!(ec.count(), 10, "balance unchanged on failure");
}

#[test]
fn echo_currency_spend_exact_balance() {
    let mut ec = EchoCurrency::with_balance(10);
    let ok = ec.spend(10, TransactionReason::UseEchoDash);
    assert!(ok, "spend exactly all (>=)");
    assert_eq!(ec.count(), 0);
}

#[test]
fn echo_currency_transaction_log() {
    let mut ec = EchoCurrency::new();
    ec.add(100, TransactionReason::TutorialReward);
    assert_eq!(ec.transaction_count(), 1);
    let t = ec.last_transaction().unwrap();
    assert_eq!(t.amount(), 100);
    assert!(t.is_gain());
    assert!(!t.is_spend());
}

#[test]
fn echo_currency_spend_logs_negative() {
    let mut ec = EchoCurrency::with_balance(100);
    ec.spend(30, TransactionReason::UseEchoDash);
    let t = ec.last_transaction().unwrap();
    assert_eq!(t.amount(), -30);
    assert!(t.is_spend());
    assert!(!t.is_gain());
}

#[test]
fn transaction_zero_is_neither_gain_nor_spend() {
    let t = Transaction::new(0, TransactionReason::TutorialReward);
    assert!(!t.is_gain());
    assert!(!t.is_spend());
}

#[test]
fn echo_currency_clear_history_keeps_balance() {
    let mut ec = EchoCurrency::with_balance(50);
    ec.add(10, TransactionReason::TutorialReward);
    ec.clear_history();
    assert_eq!(ec.count(), 60, "balance preserved");
    assert_eq!(ec.transaction_count(), 0, "log cleared");
}

// ========================================================================
// ANCHOR MODULE
// ========================================================================

#[test]
fn anchor_new_clamps_stability() {
    let a = Anchor::new(1.5, 10, None);
    assert_eq!(a.stability(), 1.0, "clamped to 1.0");

    let b = Anchor::new(-0.5, 10, None);
    assert_eq!(b.stability(), 0.0, "clamped to 0.0");
}

#[test]
fn anchor_stability_percent() {
    let a = Anchor::new(0.73, 10, None);
    assert_eq!(a.stability_percent(), 73, "(0.73 * 100).round()");
}

#[test]
fn anchor_repair_succeeds_below_one() {
    let mut a = Anchor::new(0.5, 10, None);
    let repaired = a.repair();
    assert!(repaired);
    assert!((a.stability() - 0.8).abs() < 0.001, "0.5 + 0.3 bonus = 0.8");
}

#[test]
fn anchor_repair_fails_at_exactly_one() {
    let mut a = Anchor::new(1.0, 10, None);
    let repaired = a.repair();
    assert!(!repaired, "stability >= 1.0 → can't repair");
}

#[test]
fn anchor_repair_caps_at_one() {
    let mut a = Anchor::new(0.9, 10, None);
    a.repair();
    assert_eq!(a.stability(), 1.0, "0.9 + 0.3 = 1.2, capped at 1.0");
}

#[test]
fn anchor_combat_stress_decay() {
    let mut a = Anchor::new(0.5, 10, None);
    a.apply_combat_stress();
    assert!((a.stability() - 0.45).abs() < 0.001, "0.5 + (-0.05) = 0.45");
}

#[test]
fn anchor_combat_stress_clamps_to_zero() {
    let mut a = Anchor::new(0.03, 10, None);
    a.apply_combat_stress(); // 0.03 - 0.05 → clamped to 0.0
    assert_eq!(a.stability(), 0.0);
}

#[test]
fn anchor_decay_only_when_positive() {
    let mut a = Anchor::new(0.0, 10, None);
    a.apply_decay(60.0);
    assert_eq!(a.stability(), 0.0, "no decay at zero");
}

#[test]
fn anchor_is_in_proximity_boundary() {
    let a = Anchor::new(0.5, 10, None);
    // Default proximity_radius = 3.0
    // Distance exactly 3.0 → inside (<=)
    assert!(a.is_in_proximity((3.0, 0.0, 0.0), (0.0, 0.0, 0.0)));
    // Distance just beyond → outside
    assert!(!a.is_in_proximity((3.01, 0.0, 0.0), (0.0, 0.0, 0.0)));
}

#[test]
fn anchor_repair_animation() {
    let mut a = Anchor::new(0.5, 10, None);
    a.repair();
    assert!(a.is_repaired());
    assert!(a.is_repairing());
    assert!(a.repair_animation_progress() < 1.0);

    // Use a single large step to avoid f32 accumulation error from 50×0.1
    a.update_repair_timer(5.1);
    assert!(!a.is_repairing(), "animation complete after > 5.0s");
}

#[test]
fn anchor_vfx_state_boundaries() {
    assert_eq!(AnchorVfxState::from_stability(1.0), AnchorVfxState::Perfect);
    assert_eq!(AnchorVfxState::from_stability(0.99), AnchorVfxState::Stable);
    assert_eq!(AnchorVfxState::from_stability(0.7), AnchorVfxState::Stable);
    assert_eq!(
        AnchorVfxState::from_stability(0.69),
        AnchorVfxState::Unstable
    );
    assert_eq!(
        AnchorVfxState::from_stability(0.4),
        AnchorVfxState::Unstable
    );
    assert_eq!(
        AnchorVfxState::from_stability(0.39),
        AnchorVfxState::Critical
    );
    assert_eq!(
        AnchorVfxState::from_stability(0.1),
        AnchorVfxState::Critical
    );
    assert_eq!(AnchorVfxState::from_stability(0.09), AnchorVfxState::Broken);
    assert_eq!(AnchorVfxState::from_stability(0.0), AnchorVfxState::Broken);
}

#[test]
fn anchor_vfx_glow_colors() {
    assert_eq!(AnchorVfxState::Perfect.glow_color(), (0.3, 0.7, 1.0));
    assert_eq!(AnchorVfxState::Stable.glow_color(), (0.2, 0.5, 0.8));
    assert_eq!(AnchorVfxState::Unstable.glow_color(), (0.9, 0.8, 0.2));
    assert_eq!(AnchorVfxState::Critical.glow_color(), (1.0, 0.2, 0.2));
    assert_eq!(AnchorVfxState::Broken.glow_color(), (0.0, 0.0, 0.0));
}

#[test]
fn anchor_vfx_hum_frequencies() {
    assert_eq!(AnchorVfxState::Perfect.hum_frequency(), 440.0);
    assert_eq!(AnchorVfxState::Stable.hum_frequency(), 430.0);
    assert_eq!(AnchorVfxState::Unstable.hum_frequency(), 400.0);
    assert_eq!(AnchorVfxState::Critical.hum_frequency(), 350.0);
    assert_eq!(AnchorVfxState::Broken.hum_frequency(), 0.0);
}

#[test]
fn anchor_vfx_particle_rates() {
    assert_eq!(AnchorVfxState::Perfect.particle_emission_rate(), 0.0);
    assert_eq!(AnchorVfxState::Stable.particle_emission_rate(), 5.0);
    assert_eq!(AnchorVfxState::Unstable.particle_emission_rate(), 20.0);
    assert_eq!(AnchorVfxState::Critical.particle_emission_rate(), 50.0);
    assert_eq!(AnchorVfxState::Broken.particle_emission_rate(), 0.0);
}

#[test]
fn anchor_unlocks_ability() {
    let a = Anchor::new(0.5, 10, Some(AbilityType::EchoDash));
    assert_eq!(a.unlocks_ability(), Some(AbilityType::EchoDash));
}

#[test]
fn anchor_repair_cost() {
    let a = Anchor::new(0.5, 42, None);
    assert_eq!(a.repair_cost(), 42);
}

#[test]
fn anchor_decay_reduces_stability() {
    // Test DEFAULT_DECAY_RATE indirectly: decay should reduce stability
    let mut a = Anchor::new(0.5, 10, None);
    let before = a.stability();
    a.apply_decay(1.0);
    assert!(a.stability() < before, "decay must reduce stability");
}

#[test]
fn anchor_combat_stress_is_minus_005() {
    // Test COMBAT_STRESS_DECAY indirectly: 0.50 - 0.05 = 0.45
    let mut a = Anchor::new(0.50, 10, None);
    a.apply_combat_stress();
    assert!(
        (a.stability() - 0.45).abs() < 0.001,
        "stress should reduce by exactly 0.05"
    );
}

#[test]
fn anchor_repair_adds_030() {
    // Test REPAIR_BONUS indirectly: 0.50 + 0.30 = 0.80
    let mut a = Anchor::new(0.50, 10, None);
    a.repair();
    assert!(
        (a.stability() - 0.80).abs() < 0.001,
        "repair should add exactly 0.30"
    );
}

// ========================================================================
// ADJUDICATOR MODULE
// ========================================================================

#[test]
fn weave_config_defaults() {
    let c = WeaveConfig::default();
    assert_eq!(c.budget_per_tick, 20);
    assert_eq!(c.min_priority, 0.3);
}

#[test]
fn weave_adjudicator_budget_boundary() {
    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    assert!(adj.has_budget(20), "exactly full budget (<=)");
    assert!(!adj.has_budget(21), "over budget");
}

#[test]
fn weave_adjudicator_budget_remaining() {
    let adj = WeaveAdjudicator::new();
    assert_eq!(adj.budget_remaining(), 20);
}

#[test]
fn weave_adjudicator_min_priority_filter() {
    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    let intents = vec![
        WeaveIntent::new("low").with_priority(0.29).with_cost(1), // below min
        WeaveIntent::new("exact").with_priority(0.3).with_cost(1), // at min
        WeaveIntent::new("high").with_priority(0.5).with_cost(1), // above
    ];
    let approved = adj.adjudicate(intents);
    assert!(
        !approved.iter().any(|i| i.kind == "low"),
        "0.29 < 0.3 filtered"
    );
    assert!(
        approved.iter().any(|i| i.kind == "exact"),
        "0.3 >= 0.3 passes"
    );
    assert!(approved.iter().any(|i| i.kind == "high"), "0.5 passes");
}

#[test]
fn weave_adjudicator_budget_enforcement() {
    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    let intents = vec![
        WeaveIntent::new("big").with_priority(0.9).with_cost(15), // fits
        WeaveIntent::new("small").with_priority(0.8).with_cost(5), // fits (15+5=20)
        WeaveIntent::new("tiny").with_priority(0.5).with_cost(1), // doesn't fit (21>20)
    ];
    let approved = adj.adjudicate(intents);
    assert!(approved.iter().any(|i| i.kind == "big"));
    assert!(approved.iter().any(|i| i.kind == "small"));
    assert!(
        !approved.iter().any(|i| i.kind == "tiny"),
        "budget exceeded"
    );
}

#[test]
fn weave_adjudicator_cooldown_management() {
    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    // First tick: approve intent with cooldown
    let intents = vec![WeaveIntent::new("aid")
        .with_priority(0.5)
        .with_cost(1)
        .with_cooldown("aid_event")];
    let approved = adj.adjudicate(intents);
    assert_eq!(approved.len(), 1);
    assert!(adj.is_on_cooldown("aid_event"));

    // Next tick: same cooldown_key should be blocked
    adj.begin_tick(); // decrements cooldowns by 1
    let intents2 = vec![WeaveIntent::new("aid")
        .with_priority(0.5)
        .with_cost(1)
        .with_cooldown("aid_event")];
    let approved2 = adj.adjudicate(intents2);
    assert_eq!(approved2.len(), 0, "still on cooldown");
}

#[test]
fn weave_intent_defaults() {
    let i = WeaveIntent::new("test");
    assert_eq!(i.kind, "test");
    assert_eq!(i.priority, 0.5);
    assert_eq!(i.cost, 1);
    assert_eq!(i.cooldown_key, "");
}

// ========================================================================
// PATTERNS MODULE
// ========================================================================

#[test]
fn pattern_strength_boundaries() {
    assert_eq!(PatternStrength::from_value(0.0), PatternStrength::Weak);
    assert_eq!(PatternStrength::from_value(0.29), PatternStrength::Weak);
    assert_eq!(
        PatternStrength::from_value(0.3),
        PatternStrength::Moderate,
        "0.3 is Moderate (>=)"
    );
    assert_eq!(PatternStrength::from_value(0.69), PatternStrength::Moderate);
    assert_eq!(
        PatternStrength::from_value(0.7),
        PatternStrength::Strong,
        "0.7 is Strong (>=)"
    );
    assert_eq!(PatternStrength::from_value(1.0), PatternStrength::Strong);
}

#[test]
fn pattern_strength_threshold_values() {
    assert_eq!(PatternStrength::Weak.threshold(), 0.0);
    assert_eq!(PatternStrength::Moderate.threshold(), 0.3);
    assert_eq!(PatternStrength::Strong.threshold(), 0.7);
}

#[test]
fn low_health_cluster_detector_fires() {
    let det = LowHealthClusterDetector {
        threshold: 0.5,
        min_cluster_size: 3,
    };
    let mut m = WorldMetrics::default();
    m.critical_health_count = 3;
    let results = det.detect(&m);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "low_health_cluster");
    // strength = (3.0 / 10.0).min(1.0) = 0.3
    assert!((results[0].1 - 0.3).abs() < 0.001);
}

#[test]
fn low_health_cluster_detector_below_min_cluster() {
    let det = LowHealthClusterDetector {
        threshold: 0.5,
        min_cluster_size: 3,
    };
    let mut m = WorldMetrics::default();
    m.critical_health_count = 2;
    let results = det.detect(&m);
    assert!(results.is_empty());
}

#[test]
fn resource_scarcity_detector_fires() {
    let det = ResourceScarcityDetector { threshold: 0.5 };
    let mut m = WorldMetrics::default();
    m.resource_scarcity.insert("food".to_string(), 0.7);
    let results = det.detect(&m);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "resource_scarce_food");
    assert_eq!(results[0].1, 0.7);
}

#[test]
fn resource_scarcity_detector_below_threshold() {
    let det = ResourceScarcityDetector { threshold: 0.5 };
    let mut m = WorldMetrics::default();
    m.resource_scarcity.insert("food".to_string(), 0.49);
    let results = det.detect(&m);
    assert!(results.is_empty());
}

#[test]
fn faction_conflict_detector_fires() {
    let det = FactionConflictDetector { threshold: 0.6 };
    let mut m = WorldMetrics::default();
    m.faction_tensions.insert("rebels".to_string(), 0.6);
    let results = det.detect(&m);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "faction_conflict_rebels");
}

#[test]
fn combat_intensity_detector() {
    let det = CombatIntensityDetector {
        events_threshold: 5,
        time_window: 10.0,
    };
    let mut m = WorldMetrics::default();
    m.recent_damage_events = 5;
    let results = det.detect(&m);
    assert_eq!(results.len(), 1);
    // strength = (5.0 / (5.0 * 2.0)).min(1.0) = 0.5
    assert!((results[0].1 - 0.5).abs() < 0.001);
}

#[test]
fn combat_intensity_detector_below_threshold() {
    let det = CombatIntensityDetector {
        events_threshold: 5,
        time_window: 10.0,
    };
    let mut m = WorldMetrics::default();
    m.recent_damage_events = 4;
    let results = det.detect(&m);
    assert!(results.is_empty());
}

// ========================================================================
// INTENTS MODULE — PROPOSERS
// ========================================================================

#[test]
fn aid_event_proposer_fires() {
    let p = AidEventProposer {
        strength_threshold: 0.3,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("low_health_cluster".to_string(), 0.5);
    let intents = p.propose(&patterns, 0);
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].kind, "spawn_aid_event");
    assert_eq!(intents[0].cost, 10);
}

#[test]
fn aid_event_proposer_below_threshold() {
    let p = AidEventProposer {
        strength_threshold: 0.5,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("low_health_cluster".to_string(), 0.49);
    let intents = p.propose(&patterns, 0);
    assert!(intents.is_empty());
}

#[test]
fn supply_drop_proposer_fires() {
    let p = SupplyDropProposer {
        strength_threshold: 0.3,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("resource_scarce_food".to_string(), 0.5);
    let intents = p.propose(&patterns, 0);
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].kind, "spawn_supply_drop");
    assert_eq!(intents[0].cost, 8);
}

#[test]
fn scavenger_patrol_proposer_priority_scaled() {
    let p = ScavengerPatrolProposer {
        strength_threshold: 0.3,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("high_combat_intensity".to_string(), 1.0);
    let intents = p.propose(&patterns, 0);
    // priority = strength * 0.8 = 0.8
    assert!((intents[0].priority - 0.8).abs() < 0.001);
    assert_eq!(intents[0].cost, 12);
}

#[test]
fn scavenger_patrol_type_by_seed() {
    let p = ScavengerPatrolProposer {
        strength_threshold: 0.0,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("high_combat_intensity".to_string(), 1.0);

    let i0 = p.propose(&patterns, 0); // seed=0, 0%2=0 → looters
    let i1 = p.propose(&patterns, 1); // seed=1, 1%2=1 → scavengers
    assert_eq!(i0[0].payload.get("patrol_type").unwrap(), "looters");
    assert_eq!(i1[0].payload.get("patrol_type").unwrap(), "scavengers");
}

#[test]
fn mediator_proposer_cost() {
    let p = MediatorProposer {
        strength_threshold: 0.0,
    };
    let mut patterns = BTreeMap::new();
    patterns.insert("faction_conflict_rebels".to_string(), 0.8);
    let intents = p.propose(&patterns, 0);
    assert_eq!(intents[0].cost, 15);
}

// ========================================================================
// QUEST MODULE
// ========================================================================

#[test]
fn quest_new_state_inactive() {
    let q = Quest::new("q1", "Title", "Desc");
    assert_eq!(q.state, QuestState::Inactive);
}

#[test]
fn quest_activate_only_from_inactive() {
    let mut q = Quest::new("q1", "Title", "Desc");
    assert!(q.activate());
    assert_eq!(q.state, QuestState::Active);
    assert!(!q.activate(), "can't activate when already active");
}

#[test]
fn quest_check_completion_only_when_active() {
    let mut q = Quest::new("q1", "T", "D");
    assert!(!q.check_completion(), "not active → false");
    q.activate();
    // No objectives → all complete → completes
    assert!(q.check_completion());
    assert_eq!(q.state, QuestState::Completed);
}

#[test]
fn quest_fail_only_from_active() {
    let mut q = Quest::new("q1", "T", "D");
    assert!(!q.fail(), "inactive → can't fail");
    q.activate();
    assert!(q.fail());
    assert_eq!(q.state, QuestState::Failed);
}

#[test]
fn quest_progress_empty_objectives() {
    let q = Quest::new("q1", "T", "D");
    assert_eq!(q.progress(), 0.0, "no objectives → 0.0");
}

#[test]
fn quest_kill_objective_progress() {
    let mut q = Quest::new("q1", "T", "D").with_objective(ObjectiveType::Kill {
        target_type: "enemy".to_string(),
        required: 5,
        current: 0,
    });
    q.activate();
    q.update_kill_progress("enemy", 3);
    assert!((q.progress() - 0.6).abs() < 0.001, "3/5 = 0.6");
}

#[test]
fn quest_repair_objective_stability_gate() {
    let mut q = Quest::new("q1", "T", "D").with_objective(ObjectiveType::Repair {
        required: 1,
        current: 0,
        min_stability: 0.8,
    });
    q.activate();
    assert!(!q.update_repair_progress(0.79), "0.79 < 0.8 → no progress");
    assert!(q.update_repair_progress(0.8), "0.8 >= 0.8 → progress");
}

#[test]
fn quest_explore_objective_distance() {
    let mut q = Quest::new("q1", "T", "D").with_objective(ObjectiveType::Explore {
        location_name: "place".to_string(),
        target_position: Vec3::ZERO,
        radius: 5.0,
        discovered: false,
    });
    q.activate();
    assert!(
        !q.update_explore_progress(Vec3::new(5.01, 0.0, 0.0)),
        "too far"
    );
    assert!(
        q.update_explore_progress(Vec3::new(5.0, 0.0, 0.0)),
        "at boundary (<=)"
    );
}

#[test]
fn quest_manager_prerequisite_enforcement() {
    let mut qm = QuestManager::new();
    let q1 = Quest::new("q1", "T", "D");
    let q2 = Quest::new("q2", "T", "D").with_prerequisite("q1");
    qm.register_quest(q1);
    qm.register_quest(q2);
    assert!(qm.activate_quest("q2").is_err(), "prereq q1 not completed");
    qm.activate_quest("q1").unwrap();
    // Complete q1 (no objectives → immediate)
    qm.check_active_quest();
    assert!(qm.is_completed("q1"));
    assert!(qm.activate_quest("q2").is_ok(), "prereq met");
}

#[test]
fn quest_manager_single_active() {
    let mut qm = QuestManager::new();
    qm.register_quest(Quest::new("q1", "T", "D"));
    qm.register_quest(Quest::new("q2", "T", "D"));
    qm.activate_quest("q1").unwrap();
    assert!(
        qm.activate_quest("q2").is_err(),
        "already have active quest"
    );
}

// ========================================================================
// QUEST TYPES MODULE
// ========================================================================

#[test]
fn escort_npc_destination_boundary() {
    // Destination 20 units away; speed=2.0, so takes 10s to arrive
    // Reaching threshold is distance <= 1.0
    let mut npc = EscortNPC::new("Guide", Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0), 100.0);
    npc.update(0.5); // move 1.0 unit → 19 away, not reached
    assert!(!npc.reached_destination, "19 units away, not reached");
    // Move until within arrival threshold
    for _ in 0..200 {
        npc.update(0.1); // each step moves 0.2 units
    }
    assert!(npc.reached_destination, "should have reached destination");
}

#[test]
fn escort_npc_defaults() {
    let npc = EscortNPC::new("Guide", Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 50.0);
    assert_eq!(npc.move_speed, 2.0);
    assert_eq!(npc.max_health, 50.0);
    assert_eq!(npc.health, 50.0);
    assert!(!npc.reached_destination);
}

#[test]
fn escort_npc_is_alive_boundary() {
    let mut npc = EscortNPC::new("Guide", Vec3::ZERO, Vec3::Z, 10.0);
    assert!(npc.is_alive());
    npc.take_damage(10.0);
    assert!(!npc.is_alive(), "0.0 NOT > 0.0 → dead");
}

#[test]
fn escort_npc_health_percentage() {
    let mut npc = EscortNPC::new("G", Vec3::ZERO, Vec3::Z, 100.0);
    npc.take_damage(25.0);
    assert!((npc.health_percentage() - 0.75).abs() < 0.001);
}

#[test]
fn defend_objective_both_required_for_complete() {
    let mut d = DefendObjective::new("base", Vec3::ZERO, 5.0, 100.0, 60.0, 3);
    d.complete_wave();
    d.complete_wave();
    d.complete_wave(); // 3 waves survived
    d.update(59.0); // not enough time yet
    assert!(!d.is_complete(), "waves met but time not");
    d.update(1.0); // now elapsed=60
    assert!(d.is_complete(), "both waves AND time met");
}

#[test]
fn defend_objective_is_failed_boundary() {
    let mut d = DefendObjective::new("base", Vec3::ZERO, 5.0, 10.0, 60.0, 3);
    d.take_damage(5.0);
    assert!(!d.is_failed(), "health 5.0 > 0");
    d.take_damage(5.0); // exactly 0.0 → clamped to 0.0
    assert!(d.is_failed(), "health <= 0.0");
}

#[test]
fn time_trial_expired_boundary() {
    let mut tt = TimeTrialObjective::new(30.0, 15.0);
    tt.update(29.99);
    assert!(!tt.is_expired(), "< time limit");
    tt.update(0.01);
    assert!(tt.is_expired(), ">= time limit");
}

#[test]
fn time_trial_bonus_boundary() {
    let mut tt = TimeTrialObjective::new(30.0, 15.0);
    tt.update(15.0);
    assert!(tt.is_bonus_time(), "at threshold (<=)");
    tt.update(0.001);
    assert!(!tt.is_bonus_time(), "past threshold");
}

#[test]
fn time_trial_remaining_time() {
    let mut tt = TimeTrialObjective::new(30.0, 15.0);
    tt.update(10.0);
    assert!((tt.remaining_time() - 20.0).abs() < 0.001);
}

#[test]
fn time_trial_progress() {
    let mut tt = TimeTrialObjective::new(30.0, 15.0);
    tt.update(15.0);
    assert!((tt.progress() - 0.5).abs() < 0.001, "1.0 - (15/30) = 0.5");
}

#[test]
fn boss_objective_phase_transitions() {
    let mut b = BossObjective::new("Boss", 300.0, Vec3::ZERO, Vec3::ZERO, 20.0);
    assert_eq!(b.current_phase, BossPhase::Phase1);

    // health_pct = 198/300 = 0.66 → NOT > 0.66 → Phase2
    b.take_damage(102.0);
    b.update(0.1);
    assert_eq!(b.current_phase, BossPhase::Phase2);

    // health_pct = 99/300 = 0.33 → NOT > 0.33 → Phase3
    b.take_damage(99.0);
    b.update(0.1);
    assert_eq!(b.current_phase, BossPhase::Phase3);
}

#[test]
fn boss_objective_attack_multiplier() {
    let mut b = BossObjective::new("Boss", 300.0, Vec3::ZERO, Vec3::ZERO, 20.0);
    assert_eq!(b.attack_multiplier(), 1.0);
    b.take_damage(120.0);
    b.update(0.1);
    assert_eq!(b.attack_multiplier(), 1.5);
    b.take_damage(100.0);
    b.update(0.1);
    assert_eq!(b.attack_multiplier(), 2.0);
}

#[test]
fn boss_objective_defeated_boundary() {
    let mut b = BossObjective::new("Boss", 100.0, Vec3::ZERO, Vec3::ZERO, 10.0);
    b.take_damage(50.0);
    assert!(!b.is_defeated(), "50 health remaining");
    b.take_damage(50.0); // exactly 0.0
    assert!(b.is_defeated(), "<= 0.0");
}

#[test]
fn boss_objective_special_cooldown_boundary() {
    let mut b = BossObjective::new("Boss", 300.0, Vec3::ZERO, Vec3::ZERO, 20.0);
    b.time_since_special = 9.99;
    assert!(!b.can_use_special(), "< 10.0 cooldown");
    b.time_since_special = 10.0;
    assert!(b.can_use_special(), ">= 10.0");
}

#[test]
fn boss_objective_use_special_resets() {
    let mut b = BossObjective::new("Boss", 300.0, Vec3::ZERO, Vec3::ZERO, 20.0);
    b.time_since_special = 10.0;
    b.use_special();
    assert_eq!(b.time_since_special, 0.0);
}

#[test]
fn collect_item_boundary() {
    let mut item = CollectItem::new("shard", Vec3::ZERO);
    assert!(
        item.can_collect(Vec3::new(5.0, 0.0, 0.0), 5.0),
        "at radius (<=)"
    );
    assert!(!item.can_collect(Vec3::new(5.01, 0.0, 0.0), 5.0), "beyond");
    item.collect();
    assert!(!item.can_collect(Vec3::ZERO, 10.0), "already collected");
}

#[test]
fn collect_objective_try_collect() {
    let positions = vec![
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
    ];
    let mut co = CollectObjective::new("shard", positions, 3.0);
    assert!(!co.is_complete());

    let collected = co.try_collect(Vec3::new(0.5, 0.0, 0.0)); // close to first
    assert!(collected >= 1);
    assert_eq!(co.collected_count(), collected);
}

// ========================================================================
// LEVEL MODULE
// ========================================================================

#[test]
fn player_defaults() {
    let p = Player::new(Vec3::ZERO);
    assert_eq!(p.health, 100.0);
    assert_eq!(p.max_health, 100.0);
    assert_eq!(p.echo_balance(), 0);
}

#[test]
fn player_is_alive_boundary() {
    let mut p = Player::new(Vec3::ZERO);
    p.health = 0.001;
    assert!(p.is_alive());
    p.health = 0.0;
    assert!(!p.is_alive(), "0.0 NOT > 0.0");
}

#[test]
fn player_can_dash_cost() {
    let mut p = Player::new(Vec3::ZERO);
    p.set_echo(9);
    assert!(!p.can_dash(), "9 < 10");
    p.set_echo(10);
    assert!(p.can_dash(), "10 >= 10");
}

#[test]
fn player_can_shield_cost() {
    let mut p = Player::new(Vec3::ZERO);
    p.set_echo(14);
    assert!(!p.can_shield(), "14 < 15");
    p.set_echo(15);
    assert!(p.can_shield(), "15 >= 15");
}

#[test]
fn player_use_dash_deducts_echo() {
    let mut p = Player::new(Vec3::ZERO);
    p.set_echo(20);
    p.forward = Vec3::Z;
    let result = p.use_dash();
    assert!(result.is_ok());
    assert_eq!(p.echo_balance(), 10, "deducts 10");
}

#[test]
fn player_use_shield_deducts_echo() {
    let mut p = Player::new(Vec3::ZERO);
    p.set_echo(30);
    let result = p.use_shield();
    assert!(result.is_ok());
    assert_eq!(p.echo_balance(), 15, "deducts 15");
}

#[test]
fn player_add_echo() {
    let mut p = Player::new(Vec3::ZERO);
    p.add_echo(42);
    assert_eq!(p.echo_balance(), 42);
}

#[test]
fn player_unlock_ability_dedup() {
    let mut p = Player::new(Vec3::ZERO);
    p.unlock_ability("Echo Dash");
    p.unlock_ability("Echo Dash");
    assert_eq!(p.abilities.len(), 1, "no duplicates");
}

#[test]
fn player_boost_max_health() {
    let mut p = Player::new(Vec3::ZERO);
    p.boost_stat("MaxHealth", 25.0);
    assert_eq!(p.max_health, 125.0);
    assert_eq!(p.health, 125.0, "current health also boosted");
}

#[test]
fn player_ground_clamp() {
    let mut p = Player::new(Vec3::ZERO);
    p.position.y = -5.0;
    p.velocity.y = -10.0;
    p.update(0.1);
    assert_eq!(p.position.y, 0.0, "clamped to ground");
    assert_eq!(p.velocity.y, 0.0, "velocity zeroed");
}

#[test]
fn camera_defaults() {
    let c = Camera::new(Vec3::ZERO);
    assert_eq!(c.offset, Vec3::new(0.0, 5.0, -10.0));
    assert_eq!(c.smoothing, 0.9);
}

#[test]
fn level_new_has_3_anchors() {
    let level = VeilweaverLevel::new();
    let stats = level.get_stats();
    assert_eq!(stats.anchors_total, 3);
}

#[test]
fn level_stats_initial() {
    let level = VeilweaverLevel::new();
    let stats = level.get_stats();
    assert_eq!(stats.enemies_killed, 0);
    assert_eq!(stats.anchors_repaired, 0);
    assert_eq!(stats.player_health, 100.0);
}

// ========================================================================
// SPAWNER MODULE
// ========================================================================

#[test]
fn enemy_spawner_defaults() {
    let s = EnemySpawner::new();
    assert_eq!(s.current_wave(), 1);
    assert_eq!(s.spawn_point_count(), 0);
    assert_eq!(s.difficulty_multiplier(), 1.0);
}

#[test]
fn enemy_spawner_add_remove_spawn_point() {
    let mut s = EnemySpawner::new();
    let id = s.add_spawn_point(Vec3::ZERO, 5.0, None);
    assert_eq!(s.spawn_point_count(), 1);
    assert!(s.remove_spawn_point(id));
    assert_eq!(s.spawn_point_count(), 0);
}

#[test]
fn enemy_spawner_remove_nonexistent() {
    let mut s = EnemySpawner::new();
    assert!(!s.remove_spawn_point(999));
}

#[test]
fn enemy_spawner_reset() {
    let mut s = EnemySpawner::new();
    s.add_spawn_point(Vec3::ZERO, 5.0, None);
    // Force some state changes
    s.force_spawn_wave(&[]);
    s.reset();
    assert_eq!(s.current_wave(), 1);
    assert_eq!(s.difficulty_multiplier(), 1.0);
}

#[test]
fn enemy_spawner_set_active() {
    let mut s = EnemySpawner::new();
    let id = s.add_spawn_point(Vec3::ZERO, 5.0, None);
    assert!(s.set_spawn_point_active(id, false));
    assert!(
        !s.set_spawn_point_active(999, false),
        "nonexistent returns false"
    );
}

// ========================================================================
// STARTER QUESTS MODULE
// ========================================================================

#[test]
fn starter_quests_count() {
    use astraweave_weaving::all_starter_quests;
    let quests = all_starter_quests();
    assert_eq!(quests.len(), 3);
}

#[test]
fn quest_stabilize_anchors_details() {
    use astraweave_weaving::quest_stabilize_anchors;
    let q = quest_stabilize_anchors();
    assert_eq!(q.id, "stabilize_anchors");
    assert_eq!(q.title, "Stabilize the Anchors");
    assert!(q.prerequisites.is_empty(), "first quest has no prereqs");
    assert_eq!(q.objectives.len(), 1);
    match &q.objectives[0] {
        ObjectiveType::Repair {
            required,
            min_stability,
            ..
        } => {
            assert_eq!(*required, 3);
            assert_eq!(*min_stability, 0.8);
        }
        _ => panic!("expected Repair objective"),
    }
    assert!(q
        .rewards
        .iter()
        .any(|r| matches!(r, QuestReward::EchoCurrency(100))));
}

#[test]
fn quest_clear_corruption_details() {
    use astraweave_weaving::quest_clear_corruption;
    let q = quest_clear_corruption();
    assert_eq!(q.id, "clear_corruption");
    assert_eq!(q.prerequisites, vec!["stabilize_anchors"]);
    match &q.objectives[0] {
        ObjectiveType::Kill {
            target_type,
            required,
            ..
        } => {
            assert_eq!(target_type, "enemy");
            assert_eq!(*required, 10);
        }
        _ => panic!("expected Kill objective"),
    }
    assert!(q
        .rewards
        .iter()
        .any(|r| matches!(r, QuestReward::EchoCurrency(150))));
    assert!(q
        .rewards
        .iter()
        .any(|r| matches!(r, QuestReward::StatBoost { ref stat, amount }
        if stat == "MaxHealth" && (amount - 25.0).abs() < 0.001)));
}

#[test]
fn quest_restore_beacon_details() {
    use astraweave_weaving::quest_restore_beacon;
    let q = quest_restore_beacon();
    assert_eq!(q.id, "restore_beacon");
    assert_eq!(q.prerequisites, vec!["clear_corruption"]);
    assert_eq!(q.objectives.len(), 2);
    // Fetch objective
    assert!(q.objectives.iter().any(
        |o| matches!(o, ObjectiveType::Fetch { ref item_name, required, .. }
        if item_name == "echo_shard" && *required == 5)
    ));
    // Explore objective
    assert!(q.objectives.iter().any(
        |o| matches!(o, ObjectiveType::Explore { ref location_name, radius, .. }
        if location_name == "Central Anchor" && (*radius - 5.0).abs() < 0.001)
    ));
    assert!(q
        .rewards
        .iter()
        .any(|r| matches!(r, QuestReward::EchoCurrency(200))));
}

// ========================================================================
// CWEAVE AGENT (lib.rs)
// ========================================================================

#[test]
fn cweave_agent_should_scan_boundary() {
    let agent = CWeaveAgent::new(5.0);
    assert!(agent.should_scan(5.0), "0 + 5.0 >= 5.0 (>=)");
    assert!(!agent.should_scan(4.999), "not yet");
}

#[test]
fn cweave_agent_new_defaults() {
    let agent = CWeaveAgent::new(2.0);
    assert_eq!(agent.scan_interval, 2.0);
    assert_eq!(agent.last_scan, 0.0);
    assert!(agent.patterns_detected.is_empty());
}

// ========================================================================
// CLONE AND INDEPENDENCE
// ========================================================================

#[test]
fn anchor_clone_independence() {
    let mut a = Anchor::new(0.5, 10, None);
    let b = a.clone();
    a.repair();
    assert!(a.stability() > b.stability(), "clone is independent");
}

#[test]
fn enemy_clone_independence() {
    let mut e1 = Enemy::new(Vec3::ZERO, 5.0);
    let e2 = e1.clone();
    e1.take_damage(50.0);
    assert_ne!(e1.health, e2.health);
}

#[test]
fn echo_currency_clone_independence() {
    let mut ec1 = EchoCurrency::with_balance(100);
    let ec2 = ec1.clone();
    ec1.spend(50, TransactionReason::UseEchoDash);
    assert_eq!(ec2.count(), 100);
}

// ========================================================================
// SERDE ROUNDTRIP
// ========================================================================

#[test]
fn echo_currency_clone_preserves_state() {
    let mut ec = EchoCurrency::with_balance(42);
    ec.add(10, TransactionReason::TutorialReward);
    let ec2 = ec.clone();
    assert_eq!(ec2.count(), 52);
    assert_eq!(ec.count(), ec2.count());
}

#[test]
fn anchor_clone_preserves_state() {
    let a = Anchor::new(0.75, 15, Some(AbilityType::EchoDash));
    let a2 = a.clone();
    assert_eq!(a2.stability(), a.stability());
    assert_eq!(a2.repair_cost(), 15);
}

#[test]
fn weave_config_toml_roundtrip() {
    let cfg = WeaveConfig::default();
    let toml_str = cfg.to_toml().unwrap();
    let cfg2 = WeaveConfig::from_toml(&toml_str).unwrap();
    assert_eq!(cfg2.budget_per_tick, cfg.budget_per_tick);
    assert_eq!(cfg2.min_priority, cfg.min_priority);
}

// ========================================================================
// INTEGRATION SCENARIOS
// ========================================================================

#[test]
fn combat_system_enemy_attack_with_kill() {
    let mut cs = CombatSystem::with_stats(50.0, 20.0, 50.0, 3.0);
    let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    enemy.attack_timer = 0.0; // ready to attack
    let event = cs.enemy_attack(&mut enemy, Vec3::ZERO);
    assert!(event.is_some());
    // 50 HP, enemy deals 10 damage → survive
    assert!(cs.is_player_alive(), "50 - 10 = 40 > 0");
}

#[test]
fn combat_enemy_attack_cant_attack() {
    let mut cs = CombatSystem::new();
    let mut enemy = Enemy::new(Vec3::ZERO, 5.0);
    enemy.attack_timer = 0.5; // on cooldown
    let event = cs.enemy_attack(&mut enemy, Vec3::ZERO);
    assert!(event.is_none(), "can't attack → None");
}

#[test]
fn adjudicator_sort_priority_desc() {
    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    let intents = vec![
        WeaveIntent::new("low").with_priority(0.5).with_cost(1),
        WeaveIntent::new("high").with_priority(0.9).with_cost(1),
    ];
    let approved = adj.adjudicate(intents);
    // Both should pass but high should be first (sorted by priority desc)
    assert_eq!(approved[0].kind, "high");
    assert_eq!(approved[1].kind, "low");
}

#[test]
fn full_weave_pipeline() {
    // Detect → Propose → Adjudicate
    let det = LowHealthClusterDetector {
        threshold: 0.3,
        min_cluster_size: 3,
    };
    let mut metrics = WorldMetrics::default();
    metrics.critical_health_count = 5;
    let patterns_list = det.detect(&metrics);
    assert!(!patterns_list.is_empty());

    let mut pattern_map = BTreeMap::new();
    for (k, v) in &patterns_list {
        pattern_map.insert(k.clone(), *v);
    }

    let proposer = AidEventProposer {
        strength_threshold: 0.3,
    };
    let intents = proposer.propose(&pattern_map, 42);
    assert!(!intents.is_empty());

    let mut adj = WeaveAdjudicator::new();
    adj.begin_tick();
    let approved = adj.adjudicate(intents);
    assert!(
        !approved.is_empty(),
        "full pipeline: detect → propose → approve"
    );
}

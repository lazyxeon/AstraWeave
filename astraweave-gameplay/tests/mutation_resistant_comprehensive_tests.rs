//! Mutation-resistant comprehensive tests for astraweave-gameplay
//!
//! Targets EVERY mutable constant, arithmetic operation, comparison operator,
//! and branch condition across all gameplay modules. Each test is designed to
//! detect specific mutation patterns (value swap, sign flip, op replacement,
//! condition inversion, off-by-one) that cargo-mutants would introduce.

#![allow(clippy::field_reassign_with_default)]

use astraweave_gameplay::*;
use glam::Vec3;
use std::collections::HashMap;

// ============================================================================
// Module 1: combat.rs — AttackState::tick() weapon/echo/non-weapon paths
// ============================================================================
mod combat_tick_mutations {
    use super::*;

    fn make_chain(steps: Vec<ComboStep>) -> ComboChain {
        ComboChain {
            name: "TestCombo".into(),
            steps,
        }
    }

    fn light_step(damage: i32, reach: f32, window: (f32, f32)) -> ComboStep {
        ComboStep {
            kind: AttackKind::Light,
            window,
            damage,
            reach,
            stagger: 0.3,
        }
    }

    fn heavy_step(damage: i32, reach: f32, window: (f32, f32)) -> ComboStep {
        ComboStep {
            kind: AttackKind::Heavy,
            window,
            damage,
            reach,
            stagger: 0.5,
        }
    }

    fn make_weapon(base_damage: i32, echo: Option<EchoMod>) -> Item {
        Item {
            id: 1,
            name: "Sword".into(),
            kind: ItemKind::Weapon {
                base_damage,
                dtype: DamageType::Physical,
            },
            echo,
        }
    }

    fn make_armor() -> Item {
        Item {
            id: 2,
            name: "Shield".into(),
            kind: ItemKind::Armor { defense: 10 },
            echo: None,
        }
    }

    // --- tick() with NO weapon: base = step.damage + attacker_stats.power ---

    #[test]
    fn no_weapon_damage_is_step_damage_plus_power() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(200);
        let attacker = Stats::new(100); // power = 10

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );

        assert!(hit, "must hit with no weapon in range");
        // base = step.damage(10) + power(10) = 20
        // apply_damage mitigates: max(20 - defense*0.5, 1) = max(20 - 2.5, 1) = 17
        // but dmg returned is PRE-mitigation from tick perspective
        assert_eq!(dmg, 20, "no-weapon dmg = step.damage + power");
    }

    #[test]
    fn no_weapon_applies_physical_damage_type() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(200);
        target.defense = 0; // no mitigation noise
        let attacker = Stats::new(100);

        let (hit, _) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit);
        // Physical damage applied: hp = 200 - apply_damage(20, Physical)
        // with defense 0: mitigated = max(20 - 0, 1) = 20
        assert_eq!(target.hp, 180);
    }

    // --- tick() with weapon (no echo): mult = 1.0 ---

    #[test]
    fn weapon_no_echo_mult_is_one() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100); // power = 10
        let weapon = make_weapon(15, None);

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            Some(&weapon),
            &mut target,
        );
        assert!(hit);
        // base = step.damage(10) + power(10) = 20
        // out = ((20 + base_damage(15)) as f32 * 1.0) as i32 = 35
        assert_eq!(dmg, 35, "weapon no-echo: (base + weapon_base) * 1.0");
    }

    // --- tick() with weapon + echo: mult = power_mult ---

    #[test]
    fn weapon_with_echo_multiplies_damage() {
        let echo = EchoMod {
            name: "Fire".into(),
            power_mult: 2.0,
            dtype_override: None,
            special: None,
        };
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100); // power = 10
        let weapon = make_weapon(15, Some(echo));

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            Some(&weapon),
            &mut target,
        );
        assert!(hit);
        // base = 10 + 10 = 20; out = (20 + 15) * 2.0 = 70
        assert_eq!(dmg, 70, "weapon + echo damage = (base+weapon)*mult");
    }

    #[test]
    fn weapon_echo_dtype_override_applied() {
        let echo = EchoMod {
            name: "IceEcho".into(),
            power_mult: 1.0,
            dtype_override: Some(DamageType::Frost),
            special: None,
        };
        let chain = make_chain(vec![light_step(5, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100);
        let weapon = make_weapon(10, Some(echo));

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            Some(&weapon),
            &mut target,
        );
        assert!(hit);
        // (5+10+10)*1.0 = 25
        assert_eq!(dmg, 25);
        // dtype_override = Frost (we can't easily inspect type, but damage applied correctly)
        assert_eq!(target.hp, 475);
    }

    // --- tick() with non-weapon item (Armor/Consumable) → base damage only ---

    #[test]
    fn non_weapon_item_falls_through_to_base() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100); // power = 10
        let armor = make_armor();

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            Some(&armor),
            &mut target,
        );
        assert!(hit);
        // non-weapon: base = step.damage(10) + power(10) = 20
        assert_eq!(dmg, 20, "non-weapon item uses base damage only");
    }

    // --- t_since_last resets to 0.0 after hit ---

    #[test]
    fn t_since_last_resets_to_zero_after_hit() {
        let chain = make_chain(vec![
            light_step(10, 999.0, (0.0, 1.0)),
            light_step(15, 999.0, (0.0, 1.0)),
        ]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // Advance time then hit first step
        state.tick(
            0.3,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        // After hit, t_since_last should be 0.0, verified by second hit at small dt
        let (hit, _) = state.tick(
            0.05,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(
            hit,
            "second step should hit immediately if t_since_last was reset to 0"
        );
    }

    #[test]
    fn t_since_last_accumulates_dt() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.5, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // dt=0.3, t_since_last=0.3, window=(0.5,1.0) → not in window yet
        let (hit, _) = state.tick(
            0.3,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(!hit, "0.3 < 0.5 window start");

        // dt=0.3, t_since_last=0.6 → in window
        let (hit, _) = state.tick(
            0.3,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit, "0.6 >= 0.5 window start → hit");
    }

    // --- idx advancement: exactly +1, not +0 or +2 ---

    #[test]
    fn idx_advances_by_exactly_one() {
        let steps = vec![
            light_step(10, 999.0, (0.0, 1.0)),
            light_step(20, 999.0, (0.0, 1.0)),
            light_step(30, 999.0, (0.0, 1.0)),
        ];
        let chain = make_chain(steps);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100); // power=10

        // Hit step 0: dmg = 10+10 = 20
        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit);
        assert_eq!(dmg, 20, "step 0 damage");

        // Hit step 1: dmg = 20+10 = 30
        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit);
        assert_eq!(dmg, 30, "step 1 damage — proves idx went to 1, not 2");

        // Hit step 2: dmg = 30+10 = 40
        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit);
        assert_eq!(dmg, 40, "step 2 damage — proves idx went to 2, not skipped");
    }

    // --- active becomes false after last step ---

    #[test]
    fn active_becomes_false_after_last_step() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();
        assert!(state.active);

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);
        state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );

        assert!(!state.active, "must deactivate after completing all steps");
    }

    #[test]
    fn inactive_tick_returns_zero() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        // Don't call start()

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);
        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );

        assert!(!hit);
        assert_eq!(dmg, 0);
    }

    // --- Heavy step requires pressed_heavy ---

    #[test]
    fn heavy_step_requires_pressed_heavy() {
        let chain = make_chain(vec![heavy_step(50, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // pressed_light=true, pressed_heavy=false → should NOT hit heavy step
        let (hit, _) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(!hit, "heavy step must not respond to light press");

        // pressed_heavy=true → should hit
        let (hit, dmg) = state.tick(
            0.0,
            false,
            true,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(hit, "heavy step must respond to heavy press");
        assert_eq!(dmg, 60); // 50 + power(10)
    }

    // --- Reach check: distance > reach → no hit ---

    #[test]
    fn out_of_reach_no_hit() {
        let chain = make_chain(vec![light_step(10, 2.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // distance = 5.0 > reach 2.0
        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::new(5.0, 0.0, 0.0),
            &attacker,
            None,
            &mut target,
        );
        assert!(!hit);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn at_exact_reach_hits() {
        let chain = make_chain(vec![light_step(10, 2.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // distance = 2.0 == reach 2.0 → should hit (<=)
        let (hit, _) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            &attacker,
            None,
            &mut target,
        );
        assert!(hit, "distance == reach must hit (<=)");
    }

    // --- Stagger effect pushed onto target ---

    #[test]
    fn hit_pushes_stagger_effect() {
        let chain = make_chain(vec![ComboStep {
            kind: AttackKind::Light,
            window: (0.0, 1.0),
            damage: 10,
            reach: 999.0,
            stagger: 0.75,
        }]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);
        state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );

        assert_eq!(
            target.effects.len(),
            1,
            "must push exactly one stagger effect"
        );
        match &target.effects[0] {
            StatusEffect::Stagger { time } => {
                assert!(
                    (time - 0.75).abs() < f32::EPSILON,
                    "stagger time must match step.stagger"
                );
            }
            other => panic!("expected Stagger, got {:?}", other),
        }
    }

    // --- Window boundaries ---

    #[test]
    fn before_window_no_hit() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.5, 1.0))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // dt=0.3, t=0.3 < window.0=0.5
        let (hit, _) = state.tick(
            0.3,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(!hit);
    }

    #[test]
    fn after_window_no_hit() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.1, 0.3))]);
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        let attacker = Stats::new(100);

        // dt=0.5, t=0.5 > window.1=0.3
        let (hit, _) = state.tick(
            0.5,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(!hit);
    }

    // --- start() resets to idx=0, t=0, active=true ---

    #[test]
    fn start_resets_state() {
        let chain = make_chain(vec![light_step(10, 999.0, (0.0, 1.0))]);
        let mut state = AttackState::new(chain);
        state.idx = 5;
        state.t_since_last = 99.0;
        state.active = false;

        state.start();
        assert_eq!(state.idx, 0);
        assert!((state.t_since_last).abs() < f32::EPSILON);
        assert!(state.active);
    }
}

// ============================================================================
// Module 2: water_movement.rs — exact float constants per mode/status
// ============================================================================
mod water_movement_mode_mutations {
    use super::*;

    // --- speed_multiplier per mode ---

    #[test]
    fn speed_multiplier_dry() {
        assert!((WaterMovementMode::Dry.speed_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_multiplier_wading() {
        assert!((WaterMovementMode::Wading.speed_multiplier() - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_multiplier_waist_deep() {
        assert!((WaterMovementMode::WaistDeep.speed_multiplier() - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_multiplier_swimming() {
        assert!((WaterMovementMode::Swimming.speed_multiplier() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_multiplier_diving() {
        assert!((WaterMovementMode::Diving.speed_multiplier() - 0.5).abs() < f32::EPSILON);
    }

    // --- stamina_drain_multiplier per mode ---

    #[test]
    fn stamina_drain_dry() {
        assert!((WaterMovementMode::Dry.stamina_drain_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_drain_wading() {
        assert!((WaterMovementMode::Wading.stamina_drain_multiplier() - 1.1).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_drain_waist_deep() {
        assert!(
            (WaterMovementMode::WaistDeep.stamina_drain_multiplier() - 1.3).abs() < f32::EPSILON
        );
    }

    #[test]
    fn stamina_drain_swimming() {
        assert!(
            (WaterMovementMode::Swimming.stamina_drain_multiplier() - 1.5).abs() < f32::EPSILON
        );
    }

    #[test]
    fn stamina_drain_diving() {
        assert!((WaterMovementMode::Diving.stamina_drain_multiplier() - 2.0).abs() < f32::EPSILON);
    }

    // --- can_jump per mode ---

    #[test]
    fn can_jump_dry() {
        assert!(WaterMovementMode::Dry.can_jump());
    }

    #[test]
    fn can_jump_wading() {
        assert!(WaterMovementMode::Wading.can_jump());
    }

    #[test]
    fn can_jump_waist_deep() {
        assert!(WaterMovementMode::WaistDeep.can_jump());
    }

    #[test]
    fn cannot_jump_swimming() {
        assert!(!WaterMovementMode::Swimming.can_jump());
    }

    #[test]
    fn cannot_jump_diving() {
        assert!(!WaterMovementMode::Diving.can_jump());
    }

    // --- consumes_oxygen per mode ---

    #[test]
    fn no_oxygen_dry() {
        assert!(!WaterMovementMode::Dry.consumes_oxygen());
    }

    #[test]
    fn no_oxygen_wading() {
        assert!(!WaterMovementMode::Wading.consumes_oxygen());
    }

    #[test]
    fn no_oxygen_waist_deep() {
        assert!(!WaterMovementMode::WaistDeep.consumes_oxygen());
    }

    #[test]
    fn no_oxygen_swimming() {
        assert!(!WaterMovementMode::Swimming.consumes_oxygen());
    }

    #[test]
    fn oxygen_diving() {
        assert!(WaterMovementMode::Diving.consumes_oxygen());
    }
}

mod water_wet_status_mutations {
    use super::*;

    // --- stamina_regen_multiplier per status ---

    #[test]
    fn regen_dry() {
        assert!((WetStatus::Dry.stamina_regen_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn regen_damp() {
        assert!((WetStatus::Damp.stamina_regen_multiplier() - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn regen_wet() {
        assert!((WetStatus::Wet.stamina_regen_multiplier() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn regen_soaking() {
        assert!((WetStatus::Soaking.stamina_regen_multiplier() - 0.5).abs() < f32::EPSILON);
    }

    // --- stamina_max_multiplier per status ---

    #[test]
    fn max_dry() {
        assert!((WetStatus::Dry.stamina_max_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn max_damp() {
        assert!((WetStatus::Damp.stamina_max_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn max_wet() {
        assert!((WetStatus::Wet.stamina_max_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn max_soaking() {
        assert!((WetStatus::Soaking.stamina_max_multiplier() - 0.8).abs() < f32::EPSILON);
    }

    // --- dry_time per status ---

    #[test]
    fn dry_time_dry() {
        assert!((WetStatus::Dry.dry_time() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dry_time_damp() {
        assert!((WetStatus::Damp.dry_time() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dry_time_wet() {
        assert!((WetStatus::Wet.dry_time() - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dry_time_soaking() {
        assert!((WetStatus::Soaking.dry_time() - 60.0).abs() < f32::EPSILON);
    }
}

mod water_player_state_mutations {
    use super::*;

    // --- WaterPlayerConfig defaults ---

    #[test]
    fn default_max_oxygen() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.max_oxygen - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_oxygen_drain_rate() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.oxygen_drain_rate - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_oxygen_recovery_rate() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.oxygen_recovery_rate - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_drowning_grace_period() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.drowning_grace_period - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_drowning_damage_rate() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.drowning_damage_rate - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_player_height() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.player_height - 1.8).abs() < f32::EPSILON);
    }

    #[test]
    fn default_wading_threshold() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.wading_threshold - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn default_waist_deep_threshold() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.waist_deep_threshold - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn default_swimming_threshold() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.swimming_threshold - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn default_diving_threshold() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.diving_threshold - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn default_soak_time() {
        let cfg = WaterPlayerConfig::default();
        assert!((cfg.soak_time - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_wet_resistance_level() {
        let cfg = WaterPlayerConfig::default();
        assert_eq!(cfg.wet_resistance_level, 0);
    }

    // --- is_low_oxygen threshold = 0.25 ---

    #[test]
    fn is_low_oxygen_at_24_percent() {
        let mut state = WaterPlayerState::default();
        state.oxygen = state.max_oxygen * 0.24; // just below 0.25
        assert!(state.is_low_oxygen(), "24% < 25% threshold → low oxygen");
    }

    #[test]
    fn is_not_low_oxygen_at_26_percent() {
        let mut state = WaterPlayerState::default();
        state.oxygen = state.max_oxygen * 0.26;
        assert!(!state.is_low_oxygen(), "26% > 25% threshold → not low");
    }

    // --- add_oxygen clamped to max ---

    #[test]
    fn add_oxygen_clamped_to_max() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 25.0;
        state.add_oxygen(100.0); // way over max_oxygen=30
        assert!(
            (state.oxygen - 30.0).abs() < f32::EPSILON,
            "must clamp to max_oxygen"
        );
    }

    #[test]
    fn add_oxygen_accumulates() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 10.0;
        state.add_oxygen(5.0);
        assert!((state.oxygen - 15.0).abs() < f32::EPSILON);
    }

    // --- set_wet_resistance clamped at 3 ---

    #[test]
    fn set_wet_resistance_max_3() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(10);
        // Should be clamped to 3
        // We can verify by checking stamina_regen_multiplier with wet status
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        // level 3+: skill_reduction = 0.0, so regen = 1.0 - (1.0 - 0.5) * 0.0 = 1.0
        assert!(
            (regen - 1.0).abs() < f32::EPSILON,
            "level clamped at 3 → no penalty"
        );
    }

    #[test]
    fn set_wet_resistance_exact_3_works() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(3);
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        assert!((regen - 1.0).abs() < f32::EPSILON);
    }

    // --- oxygen_percent ---

    #[test]
    fn oxygen_percent_full() {
        let state = WaterPlayerState::default();
        assert!((state.oxygen_percent() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn oxygen_percent_half() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 15.0; // max_oxygen=30
        assert!((state.oxygen_percent() - 0.5).abs() < f32::EPSILON);
    }

    // --- can_breathe ---

    #[test]
    fn can_breathe_when_not_diving() {
        let mut state = WaterPlayerState::default();
        state.mode = WaterMovementMode::Swimming;
        assert!(state.can_breathe());
    }

    #[test]
    fn can_breathe_diving_with_oxygen() {
        let mut state = WaterPlayerState::default();
        state.mode = WaterMovementMode::Diving;
        state.oxygen = 10.0;
        assert!(state.can_breathe());
    }

    #[test]
    fn cannot_breathe_diving_no_oxygen() {
        let mut state = WaterPlayerState::default();
        state.mode = WaterMovementMode::Diving;
        state.oxygen = 0.0;
        assert!(!state.can_breathe());
    }

    // --- toggle_dive ---

    #[test]
    fn toggle_dive_flips() {
        let mut state = WaterPlayerState::default();
        assert!(!state.is_diving);
        state.toggle_dive();
        assert!(state.is_diving);
        state.toggle_dive();
        assert!(!state.is_diving);
    }

    // --- update() mode selection based on submersion thresholds ---

    #[test]
    fn update_submersion_clamped() {
        let mut state = WaterPlayerState::default();
        state.update(2.0, 0.1); // > 1.0 → clamped
        assert_eq!(state.submersion, 1.0);
        state.update(-1.0, 0.1); // < 0.0 → clamped
        assert_eq!(state.submersion, 0.0);
    }

    #[test]
    fn update_mode_from_submersion_boundaries() {
        let mut state = WaterPlayerState::default();

        state.update(0.0, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Dry);

        state.update(0.14, 0.1); // < 0.15
        assert_eq!(state.mode, WaterMovementMode::Dry);

        state.update(0.16, 0.1); // >= 0.15, < 0.4
        assert_eq!(state.mode, WaterMovementMode::Wading);

        state.update(0.41, 0.1); // >= 0.4, < 0.7
        assert_eq!(state.mode, WaterMovementMode::WaistDeep);

        state.update(0.71, 0.1); // >= 0.7, < 0.95
        assert_eq!(state.mode, WaterMovementMode::Swimming);

        state.update(0.96, 0.1); // >= 0.95
        assert_eq!(state.mode, WaterMovementMode::Diving);
    }

    // --- drowning: damage after grace period ---

    #[test]
    fn drowning_damage_after_grace_period() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 0.0;

        // Simulate diving with no oxygen for grace_period(3s) + 1s
        let mut total_dmg = 0.0;
        for _ in 0..40 {
            // 40 * 0.1 = 4.0 seconds
            let result = state.update(1.0, 0.1);
            total_dmg += result.drowning_damage;
        }

        assert!(
            total_dmg > 0.0,
            "must take drowning damage after grace period"
        );
        assert!(state.is_drowning());
    }

    #[test]
    fn no_drowning_within_grace_period() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 0.0;

        // Only 2 seconds (< grace 3s)
        let mut total_dmg = 0.0;
        for _ in 0..20 {
            let result = state.update(1.0, 0.1);
            total_dmg += result.drowning_damage;
        }

        assert!(
            (total_dmg).abs() < f32::EPSILON,
            "no damage within grace period"
        );
    }

    // --- get_stamina_regen_multiplier with skill levels ---

    #[test]
    fn stamina_regen_skill_level_0_no_reduction() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(0);
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        // skill_reduction=1.0 → 1.0 - (1.0-0.5)*1.0 = 0.5
        assert!((regen - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_regen_skill_level_1() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(1);
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        // skill_reduction=0.75 → 1.0 - (1.0-0.5)*0.75 = 1.0 - 0.375 = 0.625
        assert!((regen - 0.625).abs() < 0.001);
    }

    #[test]
    fn stamina_regen_skill_level_2() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(2);
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        // skill_reduction=0.5 → 1.0 - (1.0-0.5)*0.5 = 1.0 - 0.25 = 0.75
        assert!((regen - 0.75).abs() < 0.001);
    }

    #[test]
    fn stamina_regen_skill_level_3_no_penalty() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(3);
        state.wet_status = WetStatus::Wet;
        let regen = state.get_stamina_regen_multiplier();
        // skill_reduction=0.0 → 1.0 - (1.0-0.5)*0.0 = 1.0
        assert!((regen - 1.0).abs() < f32::EPSILON);
    }

    // --- WaterUpdateResult fields populated correctly ---

    #[test]
    fn update_result_speed_multiplier_matches_mode() {
        let mut state = WaterPlayerState::default();
        let result = state.update(0.5, 0.1);
        assert_eq!(result.mode, WaterMovementMode::WaistDeep);
        assert!((result.speed_multiplier - 0.6).abs() < f32::EPSILON);
        assert!((result.stamina_drain_multiplier - 1.3).abs() < f32::EPSILON);
    }
}

// ============================================================================
// Module 3: water_movement.rs — WaterSkills exact values
// ============================================================================
mod water_skills_mutations {
    use super::*;

    // --- oxygen_bonus per deep_diver_level ---

    #[test]
    fn oxygen_bonus_level_0() {
        let skills = WaterSkills {
            deep_diver_level: 0,
            ..Default::default()
        };
        assert!((skills.oxygen_bonus() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn oxygen_bonus_level_1() {
        let skills = WaterSkills {
            deep_diver_level: 1,
            ..Default::default()
        };
        assert!((skills.oxygen_bonus() - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn oxygen_bonus_level_2() {
        let skills = WaterSkills {
            deep_diver_level: 2,
            ..Default::default()
        };
        assert!((skills.oxygen_bonus() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn oxygen_bonus_level_3_plus() {
        let skills = WaterSkills {
            deep_diver_level: 3,
            ..Default::default()
        };
        assert!((skills.oxygen_bonus() - 2.0).abs() < f32::EPSILON);
        let skills5 = WaterSkills {
            deep_diver_level: 5,
            ..Default::default()
        };
        assert!((skills5.oxygen_bonus() - 2.0).abs() < f32::EPSILON);
    }

    // --- swim_speed_bonus per swift_swimmer_level ---

    #[test]
    fn swim_speed_level_0() {
        let skills = WaterSkills {
            swift_swimmer_level: 0,
            ..Default::default()
        };
        assert!((skills.swim_speed_bonus() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn swim_speed_level_1() {
        let skills = WaterSkills {
            swift_swimmer_level: 1,
            ..Default::default()
        };
        assert!((skills.swim_speed_bonus() - 1.15).abs() < f32::EPSILON);
    }

    #[test]
    fn swim_speed_level_2() {
        let skills = WaterSkills {
            swift_swimmer_level: 2,
            ..Default::default()
        };
        assert!((skills.swim_speed_bonus() - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn swim_speed_level_3_plus() {
        let skills = WaterSkills {
            swift_swimmer_level: 3,
            ..Default::default()
        };
        assert!((skills.swim_speed_bonus() - 1.5).abs() < f32::EPSILON);
    }

    // --- splash_dash cooldown ---

    #[test]
    fn splash_dash_available_when_timer_zero() {
        let skills = WaterSkills {
            splash_dash: true,
            splash_dash_cooldown: 5.0,
            splash_dash_timer: 0.0,
            ..Default::default()
        };
        assert!(skills.can_splash_dash());
    }

    #[test]
    fn splash_dash_unavailable_on_cooldown() {
        let skills = WaterSkills {
            splash_dash: true,
            splash_dash_cooldown: 5.0,
            splash_dash_timer: 2.0,
            ..Default::default()
        };
        assert!(!skills.can_splash_dash());
    }

    #[test]
    fn splash_dash_unavailable_if_not_unlocked() {
        let skills = WaterSkills {
            splash_dash: false,
            splash_dash_timer: 0.0,
            ..Default::default()
        };
        assert!(!skills.can_splash_dash());
    }

    #[test]
    fn use_splash_dash_sets_timer() {
        let mut skills = WaterSkills {
            splash_dash: true,
            splash_dash_cooldown: 5.0,
            splash_dash_timer: 0.0,
            ..Default::default()
        };
        skills.use_splash_dash();
        assert!((skills.splash_dash_timer - 5.0).abs() < f32::EPSILON);
        assert!(!skills.can_splash_dash());
    }

    #[test]
    fn splash_dash_cooldown_decrements() {
        let mut skills = WaterSkills {
            splash_dash: true,
            splash_dash_cooldown: 5.0,
            splash_dash_timer: 3.0,
            ..Default::default()
        };
        skills.update(2.0);
        assert!((skills.splash_dash_timer - 1.0).abs() < f32::EPSILON);
    }
}

// ============================================================================
// Module 4: dialogue.rs — eval() unwrap_or defaults, compile_banter
// ============================================================================
mod dialogue_mutations {
    use super::*;

    // --- Cond::Eq with missing key → false (unwrap_or(false)) ---

    #[test]
    fn eval_eq_missing_key_returns_false() {
        let _vars: HashMap<String, String> = HashMap::new();
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: None,
                choices: vec![Choice {
                    text: "go".into(),
                    go_to: "n0".into(),
                    require: vec![Cond::Eq {
                        key: "missing".into(),
                        val: "x".into(),
                    }],
                }],
                end: true,
            }],
        };
        let mut state = DialogueState::new(&d);
        // Choice requires Eq("missing","x") — key not in vars → false
        assert!(!state.choose(&d, 0), "Eq with missing key must be false");
    }

    // --- Cond::Ne with missing key → true (unwrap_or(true)) ---

    #[test]
    fn eval_ne_missing_key_returns_true() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![
                Node {
                    id: "n0".into(),
                    line: None,
                    choices: vec![Choice {
                        text: "go".into(),
                        go_to: "n1".into(),
                        require: vec![Cond::Ne {
                            key: "missing".into(),
                            val: "x".into(),
                        }],
                    }],
                    end: false,
                },
                Node {
                    id: "n1".into(),
                    line: None,
                    choices: vec![],
                    end: true,
                },
            ],
        };
        let mut state = DialogueState::new(&d);
        // Ne("missing","x") — key not in vars → true
        assert!(state.choose(&d, 0), "Ne with missing key must be true");
    }

    // --- Cond::Eq present key, matching value → true ---

    #[test]
    fn eval_eq_present_key_matching() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![
                Node {
                    id: "n0".into(),
                    line: None,
                    choices: vec![Choice {
                        text: "go".into(),
                        go_to: "n1".into(),
                        require: vec![Cond::Eq {
                            key: "mood".into(),
                            val: "happy".into(),
                        }],
                    }],
                    end: false,
                },
                Node {
                    id: "n1".into(),
                    line: None,
                    choices: vec![],
                    end: true,
                },
            ],
        };
        let mut state = DialogueState::new(&d);
        state.vars.insert("mood".into(), "happy".into());
        assert!(state.choose(&d, 0));
    }

    // --- Cond::Eq present key, non-matching → false ---

    #[test]
    fn eval_eq_present_key_not_matching() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: None,
                choices: vec![Choice {
                    text: "go".into(),
                    go_to: "n0".into(),
                    require: vec![Cond::Eq {
                        key: "mood".into(),
                        val: "happy".into(),
                    }],
                }],
                end: true,
            }],
        };
        let mut state = DialogueState::new(&d);
        state.vars.insert("mood".into(), "sad".into());
        assert!(!state.choose(&d, 0));
    }

    // --- Cond::Has present → true, absent → false ---

    #[test]
    fn eval_has_present() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![
                Node {
                    id: "n0".into(),
                    line: None,
                    choices: vec![Choice {
                        text: "go".into(),
                        go_to: "n1".into(),
                        require: vec![Cond::Has {
                            key: "token".into(),
                        }],
                    }],
                    end: false,
                },
                Node {
                    id: "n1".into(),
                    line: None,
                    choices: vec![],
                    end: true,
                },
            ],
        };
        let mut state = DialogueState::new(&d);
        state.vars.insert("token".into(), "yes".into());
        assert!(state.choose(&d, 0));
    }

    #[test]
    fn eval_has_absent() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: None,
                choices: vec![Choice {
                    text: "go".into(),
                    go_to: "n0".into(),
                    require: vec![Cond::Has {
                        key: "token".into(),
                    }],
                }],
                end: true,
            }],
        };
        let mut state = DialogueState::new(&d);
        assert!(!state.choose(&d, 0));
    }

    // --- compile_banter: != operator parsed as Cond::Ne ---

    #[test]
    fn compile_banter_ne_operator() {
        let src = "[Guard] Stop!\n? mood != happy : goto n1";
        let d = compile_banter_to_nodes("test", src);
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].choices.len(), 1);
        let choice = &d.nodes[0].choices[0];
        assert_eq!(choice.go_to, "n1");
        assert_eq!(choice.require.len(), 1);
        match &choice.require[0] {
            Cond::Ne { key, val } => {
                assert_eq!(key, "mood");
                assert_eq!(val, "happy");
            }
            other => panic!("expected Cond::Ne, got {:?}", other),
        }
    }

    // --- compile_banter: == operator parsed as Cond::Eq ---

    #[test]
    fn compile_banter_eq_operator() {
        let src = "[NPC] Hello\n? state == ready : goto n2";
        let d = compile_banter_to_nodes("test_eq", src);
        let choice = &d.nodes[0].choices[0];
        assert_eq!(choice.go_to, "n2");
        match &choice.require[0] {
            Cond::Eq { key, val } => {
                assert_eq!(key, "state");
                assert_eq!(val, "ready");
            }
            other => panic!("expected Cond::Eq, got {:?}", other),
        }
    }

    // --- compile_banter: -> key = value with spaces (trim) ---

    #[test]
    fn compile_banter_set_var_with_spaces() {
        let src = "[Guard] Welcome!\n->  mood = happy ";
        let d = compile_banter_to_nodes("sv", src);
        let line = d.nodes[0].line.as_ref().unwrap();
        assert_eq!(line.set_vars.len(), 1);
        assert_eq!(line.set_vars[0].0, "mood");
        assert_eq!(line.set_vars[0].1, "happy");
    }

    // --- compile_banter: last node marked end=true ---

    #[test]
    fn compile_banter_last_end_true() {
        let src = "[A] First\n[B] Second\n[C] Third";
        let d = compile_banter_to_nodes("multi", src);
        assert_eq!(d.nodes.len(), 3);
        assert!(!d.nodes[0].end);
        assert!(!d.nodes[1].end);
        assert!(d.nodes[2].end, "only last node should be end=true");
    }

    // --- compile_banter: start == first node id ---

    #[test]
    fn compile_banter_start_is_first_node() {
        let src = "[A] Hello";
        let d = compile_banter_to_nodes("start_test", src);
        assert_eq!(d.start, "n0");
        assert_eq!(d.id, "start_test");
    }

    // --- compile_banter: node ids increment n0, n1, n2 ---

    #[test]
    fn compile_banter_node_ids_sequential() {
        let src = "[A] One\n[B] Two\n[C] Three";
        let d = compile_banter_to_nodes("ids", src);
        assert_eq!(d.nodes[0].id, "n0");
        assert_eq!(d.nodes[1].id, "n1");
        assert_eq!(d.nodes[2].id, "n2");
    }

    // --- choose() applies set_vars from destination node ---

    #[test]
    fn choose_applies_set_vars() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![
                Node {
                    id: "n0".into(),
                    line: None,
                    choices: vec![Choice {
                        text: "go".into(),
                        go_to: "n1".into(),
                        require: vec![],
                    }],
                    end: false,
                },
                Node {
                    id: "n1".into(),
                    line: Some(Line {
                        speaker: "NPC".into(),
                        text: "Hi".into(),
                        set_vars: vec![("visited".into(), "true".into())],
                    }),
                    choices: vec![],
                    end: true,
                },
            ],
        };
        let mut state = DialogueState::new(&d);
        assert!(state.choose(&d, 0));
        assert_eq!(state.vars.get("visited"), Some(&"true".to_string()));
    }

    // --- choose() with invalid go_to returns false ---

    #[test]
    fn choose_invalid_goto_returns_false() {
        let d = Dialogue {
            id: "t".into(),
            start: "n0".into(),
            nodes: vec![Node {
                id: "n0".into(),
                line: None,
                choices: vec![Choice {
                    text: "go".into(),
                    go_to: "nonexistent".into(),
                    require: vec![],
                }],
                end: false,
            }],
        };
        let mut state = DialogueState::new(&d);
        assert!(!state.choose(&d, 0), "invalid go_to must return false");
    }
}

// ============================================================================
// Module 5: stats.rs — apply_damage mitigation, tick DoT, new() defaults
// ============================================================================
mod stats_mutations {
    use super::*;

    // --- Stats::new() defaults ---

    #[test]
    fn new_defaults() {
        let s = Stats::new(75);
        assert_eq!(s.hp, 75);
        assert_eq!(s.stamina, 100);
        assert_eq!(s.power, 10);
        assert_eq!(s.defense, 5);
        assert!((s.echo_amp - 1.0).abs() < f32::EPSILON);
        assert!(s.effects.is_empty());
    }

    // --- apply_damage: mitigated = max(amount - defense*0.5, 1.0) ---

    #[test]
    fn apply_damage_mitigation_formula() {
        let mut s = Stats::new(100);
        s.defense = 10;
        // mitigated = max(20 - 10*0.5, 1) = max(15, 1) = 15
        let m = s.apply_damage(20, DamageType::Physical);
        assert_eq!(m, 15);
        assert_eq!(s.hp, 85);
    }

    #[test]
    fn apply_damage_minimum_is_one() {
        let mut s = Stats::new(100);
        s.defense = 200;
        // mitigated = max(1 - 100, 1) = max(-99, 1) = 1
        let m = s.apply_damage(1, DamageType::Physical);
        assert_eq!(m, 1);
        assert_eq!(s.hp, 99);
    }

    #[test]
    fn apply_damage_zero_defense() {
        let mut s = Stats::new(100);
        s.defense = 0;
        // Fire is 125% of the physical result: 30 -> 37.
        let m = s.apply_damage(30, DamageType::Fire);
        assert_eq!(m, 37);
        assert_eq!(s.hp, 63);
    }

    // --- tick(): bleed dps*dt subtracted from hp ---

    #[test]
    fn tick_bleed_drains_hp() {
        let mut s = Stats::new(100);
        s.effects.push(StatusEffect::Bleed {
            dps: 10.0,
            time: 5.0,
        });
        let dot = s.tick(1.0);
        // dot = 10.0 * 1.0 = 10.0 → 10 as i32
        assert_eq!(dot, 10);
        assert_eq!(s.hp, 90);
    }

    #[test]
    fn tick_bleed_time_decrements() {
        let mut s = Stats::new(100);
        s.effects.push(StatusEffect::Bleed {
            dps: 5.0,
            time: 1.0,
        });
        s.tick(0.5); // time: 1.0 - 0.5 = 0.5 > 0 → retained
        assert_eq!(s.effects.len(), 1);
        s.tick(0.6); // time: 0.5 - 0.6 = -0.1 ≤ 0 → removed
        assert_eq!(s.effects.len(), 0);
    }

    #[test]
    fn tick_stagger_expires() {
        let mut s = Stats::new(100);
        s.effects.push(StatusEffect::Stagger { time: 0.5 });
        s.tick(0.3); // 0.5 - 0.3 = 0.2 > 0 → retained
        assert_eq!(s.effects.len(), 1);
        s.tick(0.3); // 0.2 - 0.3 = -0.1 → removed
        assert_eq!(s.effects.len(), 0);
    }

    #[test]
    fn tick_chill_expires() {
        let mut s = Stats::new(100);
        s.effects.push(StatusEffect::Chill {
            slow: 0.5,
            time: 1.0,
        });
        s.tick(1.5); // 1.0 - 1.5 = -0.5 → removed
        assert_eq!(s.effects.len(), 0);
    }

    #[test]
    fn tick_multiple_effects_independent() {
        let mut s = Stats::new(100);
        s.effects.push(StatusEffect::Bleed {
            dps: 5.0,
            time: 2.0,
        });
        s.effects.push(StatusEffect::Bleed {
            dps: 3.0,
            time: 2.0,
        });
        s.effects.push(StatusEffect::Stagger { time: 1.0 });
        let dot = s.tick(0.5);
        // bleeds: 5*0.5 + 3*0.5 = 4.0 → 4
        assert_eq!(dot, 4);
        assert_eq!(s.hp, 96);
        assert_eq!(s.effects.len(), 3); // all still alive
    }
}

// ============================================================================
// Module 6: items.rs — inventory resource operations, infuse, echo defs
// ============================================================================
mod items_mutations {
    use super::*;

    // --- add_resource: new kind creates entry, existing kind increments ---

    #[test]
    fn add_resource_creates_entry() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        assert_eq!(inv.resources.len(), 1);
        assert_eq!(inv.resources[0], (ResourceKind::Wood, 10));
    }

    #[test]
    fn add_resource_increments_existing() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        inv.add_resource(ResourceKind::Wood, 7);
        assert_eq!(inv.resources.len(), 1);
        assert_eq!(inv.resources[0].1, 17); // 10 + 7, not 10 - 7 or 10 * 7
    }

    // --- remove_resource: exact boundary, insufficient ---

    #[test]
    fn remove_resource_exact_amount() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Crystal, 5);
        assert!(inv.remove_resource(ResourceKind::Crystal, 5));
        assert_eq!(inv.resources[0].1, 0);
    }

    #[test]
    fn remove_resource_insufficient() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Crystal, 5);
        assert!(!inv.remove_resource(ResourceKind::Crystal, 6));
        assert_eq!(inv.resources[0].1, 5, "must not deduct on failure");
    }

    #[test]
    fn remove_resource_not_found() {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        assert!(!inv.remove_resource(ResourceKind::Crystal, 1));
    }

    // --- infuse sets echo ---

    #[test]
    fn infuse_sets_echo_on_item() {
        let mut item = Item {
            id: 1,
            name: "Sword".into(),
            kind: ItemKind::Weapon {
                base_damage: 10,
                dtype: DamageType::Physical,
            },
            echo: None,
        };
        let echo = EchoMod {
            name: "Fire".into(),
            power_mult: 1.5,
            dtype_override: Some(DamageType::Fire),
            special: None,
        };
        infuse(&mut item, echo);
        assert!(item.echo.is_some());
        assert_eq!(item.echo.as_ref().unwrap().name, "Fire");
        assert!((item.echo.as_ref().unwrap().power_mult - 1.5).abs() < f32::EPSILON);
    }

    // --- load_echo_defs parses TOML correctly ---

    #[test]
    fn load_echo_defs_power_mult_exact() {
        let toml = r#"
[[echoes]]
name = "Ember"
rarity = "Rare"
power_mult = 1.35
"#;
        let defs = load_echo_defs(toml).unwrap();
        assert_eq!(defs.len(), 1);
        assert!((defs[0].power_mult - 1.35).abs() < 0.001);
        assert_eq!(defs[0].rarity, Rarity::Rare);
    }

    #[test]
    fn load_echo_defs_with_optional_fields() {
        let toml = r#"
[[echoes]]
name = "Storm"
rarity = "Epic"
power_mult = 2.0
dtype_override = "Shock"
special = "chain_lightning"
"#;
        let defs = load_echo_defs(toml).unwrap();
        assert!(defs[0].dtype_override.is_some());
        assert_eq!(defs[0].special.as_deref(), Some("chain_lightning"));
    }
}

// ============================================================================
// Module 7: crafting.rs — cost checking, success_chance formula, clamp
// ============================================================================
mod crafting_mutations {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    // --- craft_seeded: insufficient single resource → None, no deduction ---

    #[test]
    fn craft_insufficient_returns_none_no_deduction() {
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Blade".into(),
                output_item: ItemKind::Weapon {
                    base_damage: 10,
                    dtype: DamageType::Physical,
                },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 5,
                }],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 4); // have 4, need 5

        assert!(book.craft_seeded("Blade", &mut inv, &mut rng()).is_none());
        assert_eq!(inv.resources[0].1, 4, "must not deduct on failure");
    }

    // --- craft_seeded: sufficient → consumes exact cost ---

    #[test]
    fn craft_consumes_exact_cost() {
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Potion".into(),
                output_item: ItemKind::Consumable { heal: 20 },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Wood,
                        count: 3,
                    },
                    CraftCost {
                        kind: ResourceKind::Essence,
                        count: 1,
                    },
                ],
            }],
        };
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Wood, 10);
        inv.add_resource(ResourceKind::Essence, 5);

        let item = book.craft_seeded("Potion", &mut inv, &mut rng());
        assert!(item.is_some());
        assert_eq!(inv.resources[0].1, 7); // 10 - 3
        assert_eq!(inv.resources[1].1, 4); // 5 - 1
    }

    // --- craft_seeded: recipe not found → None ---

    #[test]
    fn craft_recipe_not_found() {
        let book = RecipeBook { recipes: vec![] };
        let mut inv = Inventory::default();
        assert!(book.craft_seeded("X", &mut inv, &mut rng()).is_none());
    }

    // --- success_chance formula constants ---

    #[test]
    fn success_chance_base_0_75() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, None);
        assert!((c - 0.75).abs() < 0.001, "base = 0.75 + 0 + 0");
    }

    #[test]
    fn success_chance_quality_coefficient_0_05() {
        let bench = CraftBench { quality: 2 };
        // base = 0.75 + 2*0.05 + 0 = 0.85
        let c = bench.success_chance(0, None, None);
        assert!((c - 0.85).abs() < 0.001);
    }

    #[test]
    fn success_chance_power_coefficient_0_003() {
        let bench = CraftBench { quality: 0 };
        // base = 0.75 + 0 + 10*0.003 = 0.78
        let c = bench.success_chance(10, None, None);
        assert!((c - 0.78).abs() < 0.001);
    }

    #[test]
    fn success_chance_faction_coefficient_0_001() {
        let bench = CraftBench { quality: 0 };
        let faction = FactionStanding {
            name: "Smiths".into(),
            reputation: 100,
        };
        // fac = 100*0.001 = 0.1 → total = 0.75 + 0.1 = 0.85
        let c = bench.success_chance(0, Some(&faction), None);
        assert!((c - 0.85).abs() < 0.001);
    }

    #[test]
    fn success_chance_epic_penalty() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, Some(&Rarity::Epic));
        // 0.75 - 0.15 = 0.60
        assert!((c - 0.60).abs() < 0.001);
    }

    #[test]
    fn success_chance_legendary_penalty() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, Some(&Rarity::Legendary));
        // 0.75 - 0.30 = 0.45
        assert!((c - 0.45).abs() < 0.001);
    }

    #[test]
    fn success_chance_common_no_penalty() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, Some(&Rarity::Common));
        assert!((c - 0.75).abs() < 0.001);
    }

    #[test]
    fn success_chance_uncommon_no_penalty() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, Some(&Rarity::Uncommon));
        assert!((c - 0.75).abs() < 0.001);
    }

    #[test]
    fn success_chance_rare_no_penalty() {
        let bench = CraftBench { quality: 0 };
        let c = bench.success_chance(0, None, Some(&Rarity::Rare));
        assert!((c - 0.75).abs() < 0.001);
    }

    #[test]
    fn success_chance_clamp_min_0_05() {
        let bench = CraftBench { quality: -2 };
        let faction = FactionStanding {
            name: "E".into(),
            reputation: -100,
        };
        let c = bench.success_chance(0, Some(&faction), Some(&Rarity::Legendary));
        // base = 0.75 - 0.10 + 0 = 0.65; fac=-0.1; rarity=-0.30 → 0.25 (above 0.05)
        // but even if somehow below, clamp guarantees >= 0.05
        assert!(c >= 0.05);
    }

    #[test]
    fn success_chance_clamp_max_0_98() {
        let bench = CraftBench { quality: 3 };
        let faction = FactionStanding {
            name: "A".into(),
            reputation: 100,
        };
        let c = bench.success_chance(100, Some(&faction), Some(&Rarity::Common));
        assert!((c - 0.98).abs() < 0.001, "must clamp at 0.98");
    }

    // --- negative faction reputation works ---

    #[test]
    fn success_chance_negative_faction() {
        let bench = CraftBench { quality: 0 };
        let faction = FactionStanding {
            name: "E".into(),
            reputation: -50,
        };
        // 0.75 + (-50*0.001) = 0.75 - 0.05 = 0.70
        let c = bench.success_chance(0, Some(&faction), None);
        assert!((c - 0.70).abs() < 0.001);
    }
}

// ============================================================================
// Module 8: WaterMovementHelper — force calculations
// ============================================================================
mod water_forces_mutations {
    use super::*;

    #[test]
    fn buoyancy_scales_with_submersion() {
        let helper = WaterMovementHelper::default();
        let forces =
            helper.calculate_water_forces(Vec3::ZERO, 0.5, Vec3::ZERO, WaterMovementMode::Wading);
        // buoyancy = (0, 15.0 * 0.5, 0) = (0, 7.5, 0)
        assert!((forces.buoyancy.y - 7.5).abs() < 0.01);
        assert!((forces.buoyancy.x).abs() < f32::EPSILON);
    }

    #[test]
    fn no_buoyancy_when_dry() {
        let helper = WaterMovementHelper::default();
        let forces =
            helper.calculate_water_forces(Vec3::ZERO, 0.0, Vec3::ZERO, WaterMovementMode::Dry);
        assert!((forces.buoyancy.y).abs() < f32::EPSILON);
    }

    #[test]
    fn swim_force_only_in_swimming_or_diving() {
        let helper = WaterMovementHelper::default();

        let wading_forces = helper.calculate_water_forces(
            Vec3::ZERO,
            0.3,
            Vec3::new(1.0, 0.0, 0.0),
            WaterMovementMode::Wading,
        );
        assert!(
            (wading_forces.swim.length()).abs() < f32::EPSILON,
            "no swim force in Wading"
        );

        let swim_forces = helper.calculate_water_forces(
            Vec3::ZERO,
            0.8,
            Vec3::new(1.0, 0.0, 0.0),
            WaterMovementMode::Swimming,
        );
        assert!(swim_forces.swim.x > 0.0, "swim force applied in Swimming");

        let dive_forces = helper.calculate_water_forces(
            Vec3::ZERO,
            1.0,
            Vec3::new(1.0, 0.0, 0.0),
            WaterMovementMode::Diving,
        );
        assert!(dive_forces.swim.x > 0.0, "swim force applied in Diving");
    }

    #[test]
    fn total_sums_all_forces() {
        let forces = WaterForces {
            buoyancy: Vec3::new(0.0, 5.0, 0.0),
            drag: Vec3::new(-1.0, 0.0, 0.0),
            swim: Vec3::new(3.0, 0.0, 0.0),
        };
        let total = forces.total();
        assert!((total.x - 2.0).abs() < f32::EPSILON);
        assert!((total.y - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_helper_values() {
        let helper = WaterMovementHelper::default();
        assert!((helper.buoyancy_force - 15.0).abs() < f32::EPSILON);
        assert!((helper.water_drag - 3.0).abs() < f32::EPSILON);
        assert!((helper.swim_force - 8.0).abs() < f32::EPSILON);
    }
}

// ============================================================================
// Module 9: water update_wet_status drying with skill_bonus
// ============================================================================
mod water_drying_skill_mutations {
    use super::*;

    #[test]
    fn drying_skill_bonus_level_0() {
        // skill_bonus = 1.0, wet_timer decrements by dt * 1.0
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Damp;
        state.wet_timer = 5.0;

        state.update(0.0, 2.0); // dry, dt=2.0
                                // timer = 5.0 - 2.0 * 1.0 = 3.0
        assert!((state.wet_timer - 3.0).abs() < 0.1);
    }

    #[test]
    fn drying_skill_bonus_level_1() {
        // skill_bonus = 1.25
        let mut cfg = WaterPlayerConfig::default();
        cfg.wet_resistance_level = 1;
        let mut state = WaterPlayerState::new(cfg);
        state.wet_status = WetStatus::Damp;
        state.wet_timer = 5.0;

        state.update(0.0, 2.0);
        // timer = 5.0 - 2.0 * 1.25 = 2.5
        assert!((state.wet_timer - 2.5).abs() < 0.1);
    }

    #[test]
    fn drying_skill_bonus_level_2() {
        // skill_bonus = 1.5
        let mut cfg = WaterPlayerConfig::default();
        cfg.wet_resistance_level = 2;
        let mut state = WaterPlayerState::new(cfg);
        state.wet_status = WetStatus::Damp;
        state.wet_timer = 5.0;

        state.update(0.0, 2.0);
        // timer = 5.0 - 2.0 * 1.5 = 2.0
        assert!((state.wet_timer - 2.0).abs() < 0.1);
    }

    #[test]
    fn drying_skill_bonus_level_3() {
        // skill_bonus = 2.0
        let mut cfg = WaterPlayerConfig::default();
        cfg.wet_resistance_level = 3;
        let mut state = WaterPlayerState::new(cfg);
        state.wet_status = WetStatus::Damp;
        state.wet_timer = 5.0;

        state.update(0.0, 2.0);
        // timer = 5.0 - 2.0 * 2.0 = 1.0
        assert!((state.wet_timer - 1.0).abs() < 0.1);
    }

    // --- Wet status transitions on drying ---

    #[test]
    fn soaking_dries_to_wet_before_dry() {
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Soaking;
        state.wet_timer = 0.1; // almost done

        // Small update to trigger transition
        state.update(0.0, 0.2); // timer → -0.1 → transition Soaking → Wet
        assert_eq!(
            state.wet_status,
            WetStatus::Wet,
            "soaking should transition to wet first"
        );
    }
}

// ============================================================================
// Module 10: water update_wet_status wetting transitions
// ============================================================================
mod water_wetting_transitions {
    use super::*;

    #[test]
    fn immediate_submersion_becomes_damp() {
        let mut state = WaterPlayerState::default();
        state.update(0.5, 0.1); // submerged, dt=0.1
                                // wet_timer = 0.1 < 1.0 → Damp
        assert_eq!(state.wet_status, WetStatus::Damp);
    }

    #[test]
    fn after_1s_submersion_becomes_wet() {
        let mut state = WaterPlayerState::default();
        // Accumulate 1.5s of submersion
        for _ in 0..15 {
            state.update(0.5, 0.1);
        }
        // wet_timer = 1.5 ≥ 1.0 and < soak_time(5.0) → Wet
        assert_eq!(state.wet_status, WetStatus::Wet);
    }

    #[test]
    fn after_soak_time_becomes_soaking() {
        let mut state = WaterPlayerState::default();
        // Accumulate 6s of submersion
        for _ in 0..60 {
            state.update(0.5, 0.1);
        }
        // wet_timer = 6.0 ≥ soak_time(5.0) → Soaking
        assert_eq!(state.wet_status, WetStatus::Soaking);
    }
}

// ============================================================================
// Module 11: oxygen recovery and submerge_time tracking
// ============================================================================
mod water_oxygen_mutations {
    use super::*;

    #[test]
    fn oxygen_recovers_at_recovery_rate() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 20.0; // below max_oxygen=30
        state.update(0.0, 1.0); // not diving, 1 second
                                // recovery: 20.0 + 3.0*1.0 = 23.0, clamped to max=30 → 23.0
        assert!((state.oxygen - 23.0).abs() < 0.01);
    }

    #[test]
    fn oxygen_recovery_clamped_to_max() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 29.0;
        state.update(0.0, 1.0); // would add 3.0 → 32.0, clamped to 30.0
        assert!((state.oxygen - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn oxygen_drains_while_diving() {
        let mut state = WaterPlayerState::default();
        let initial = state.oxygen;
        state.update(1.0, 1.0); // fully submerged = diving, 1 second
                                // drain: 30.0 - 1.0*1.0 = 29.0
        assert!((state.oxygen - (initial - 1.0)).abs() < 0.01);
    }

    #[test]
    fn submerge_time_accumulates() {
        let mut state = WaterPlayerState::default();
        state.update(0.5, 1.0);
        assert!((state.submerge_time - 1.0).abs() < 0.01);
        state.update(0.5, 1.0);
        assert!((state.submerge_time - 2.0).abs() < 0.01);
    }

    #[test]
    fn submerge_time_resets_when_dry() {
        let mut state = WaterPlayerState::default();
        state.update(0.5, 1.0);
        assert!(state.submerge_time > 0.0);
        state.update(0.0, 1.0);
        assert!((state.submerge_time).abs() < f32::EPSILON);
    }

    #[test]
    fn drowning_timer_resets_when_surfacing() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 0.0;
        state.update(1.0, 1.0); // diving, drowning_timer accumulates
        assert!(state.drowning_timer > 0.0);
        state.stop_dive();
        state.update(0.0, 0.1); // surface
        assert!(
            (state.drowning_timer).abs() < f32::EPSILON,
            "drowning timer resets on surface"
        );
    }
}

// ============================================================================
// Module 12: Combined combat + stats integration (end-to-end damage path)
// ============================================================================
mod combat_stats_integration {
    use super::*;

    #[test]
    fn full_combo_damage_accumulates() {
        let chain = ComboChain {
            name: "Triple".into(),
            steps: vec![
                ComboStep {
                    kind: AttackKind::Light,
                    window: (0.0, 1.0),
                    damage: 10,
                    reach: 999.0,
                    stagger: 0.1,
                },
                ComboStep {
                    kind: AttackKind::Light,
                    window: (0.0, 1.0),
                    damage: 15,
                    reach: 999.0,
                    stagger: 0.1,
                },
                ComboStep {
                    kind: AttackKind::Heavy,
                    window: (0.0, 1.0),
                    damage: 30,
                    reach: 999.0,
                    stagger: 0.5,
                },
            ],
        };
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(200);
        target.defense = 0;
        let attacker = Stats::new(100); // power=10

        // Step 0: light, dmg = 10+10 = 20
        let (h0, d0) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(h0);
        assert_eq!(d0, 20);

        // Step 1: light, dmg = 15+10 = 25
        let (h1, d1) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(h1);
        assert_eq!(d1, 25);

        // Step 2: heavy, dmg = 30+10 = 40
        let (h2, d2) = state.tick(
            0.1,
            false,
            true,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            None,
            &mut target,
        );
        assert!(h2);
        assert_eq!(d2, 40);

        // Total: 20 + 25 + 40 = 85 → hp = 200 - 85 = 115
        assert_eq!(target.hp, 115);
        assert!(!state.active, "must deactivate after all 3 steps");
        assert_eq!(target.effects.len(), 3, "3 stagger effects pushed");
    }

    #[test]
    fn weapon_echo_end_to_end() {
        let echo = EchoMod {
            name: "Thunder".into(),
            power_mult: 1.5,
            dtype_override: Some(DamageType::Shock),
            special: None,
        };
        let weapon = Item {
            id: 1,
            name: "StormBlade".into(),
            kind: ItemKind::Weapon {
                base_damage: 20,
                dtype: DamageType::Physical,
            },
            echo: Some(echo),
        };
        let chain = ComboChain {
            name: "Storm Strike".into(),
            steps: vec![ComboStep {
                kind: AttackKind::Light,
                window: (0.0, 1.0),
                damage: 10,
                reach: 999.0,
                stagger: 0.3,
            }],
        };
        let mut state = AttackState::new(chain);
        state.start();

        let mut target = Stats::new(500);
        target.defense = 0;
        let attacker = Stats::new(100); // power=10

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::ZERO,
            &attacker,
            Some(&weapon),
            &mut target,
        );
        assert!(hit);
        // base = 10+10 = 20; out = (20+20) * 1.5 = 60. Shock is 11000
        // basis points of that result with defense 0, so taken is 66.
        assert_eq!(dmg, 60);
        assert_eq!(target.hp, 434); // 500 - 66
    }
}

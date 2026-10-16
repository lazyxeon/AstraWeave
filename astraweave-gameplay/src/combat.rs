use crate::{items::Item, DamageType, Stats};
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AttackKind {
    Light,
    Heavy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComboStep {
    pub kind: AttackKind,
    pub window: (f32, f32), // input window secs after previous impact
    pub damage: i32,
    pub reach: f32,   // meters
    pub stagger: f32, // seconds if hit
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComboChain {
    pub name: String,
    pub steps: Vec<ComboStep>,
}

#[derive(Clone, Debug)]
pub struct AttackState {
    pub chain: ComboChain,
    pub idx: usize,
    pub t_since_last: f32,
    pub active: bool,
}

impl AttackState {
    pub fn new(chain: ComboChain) -> Self {
        Self {
            chain,
            idx: 0,
            t_since_last: 0.0,
            active: false,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.idx = 0;
        self.t_since_last = 0.0;
    }

    /// call per-frame; returns (did_hit, applied_damage)
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        dt: f32,
        pressed_light: bool,
        pressed_heavy: bool,
        attacker_pos: Vec3,
        target_pos: Vec3,
        attacker_stats: &Stats,
        weapon: Option<&Item>,
        target: &mut Stats,
    ) -> (bool, i32) {
        if !self.active {
            return (false, 0);
        }
        self.t_since_last += dt;
        let step = &self.chain.steps[self.idx];
        let want = match step.kind {
            AttackKind::Light => pressed_light,
            AttackKind::Heavy => pressed_heavy,
        };
        let in_win = self.t_since_last >= step.window.0 && self.t_since_last <= step.window.1;
        let mut did_hit = false;
        let mut dmg = 0;

        if want && in_win && !target.is_stunned() {
            // impact check: distance <= reach
            let d = attacker_pos.distance(target_pos);
            if d <= step.reach {
                let base = step.damage + attacker_stats.power;
                let scale = attacker_stats.move_scale_bp() as i32;
                if let Some(w) = weapon {
                    match &w.kind {
                        crate::items::ItemKind::Weapon { base_damage, dtype } => {
                            let mult = w.echo.as_ref().map(|e| e.power_mult).unwrap_or(1.0);
                            let dtype = w
                                .echo
                                .as_ref()
                                .and_then(|e| e.dtype_override)
                                .unwrap_or(*dtype);
                            let raw = ((base + base_damage) as f32 * mult) as i32;
                            let out = (raw as i64 * scale as i64 / 10_000) as i32;
                            target.apply_damage(out, dtype);
                            if let Some(tag) = w.echo.as_ref().and_then(|e| e.special.as_deref()) {
                                target.apply_element_rider(DamageType::Physical, Some(tag));
                            }
                            dmg = out;
                        }
                        _ => {
                            let out = (base as i64 * scale as i64 / 10_000) as i32;
                            target.apply_damage(out, DamageType::Physical);
                            dmg = out;
                        }
                    }
                } else {
                    let out = (base as i64 * scale as i64 / 10_000) as i32;
                    target.apply_damage(out, DamageType::Physical);
                    dmg = out;
                }
                // apply stagger
                target
                    .effects
                    .push(crate::stats::StatusEffect::Stagger { time: step.stagger });
                did_hit = true;
            }
            // next step
            self.idx += 1;
            self.t_since_last = 0.0;
            if self.idx >= self.chain.steps.len() {
                self.active = false;
            }
        }
        (did_hit, dmg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AttackKind tests
    #[test]
    fn test_attack_kind_light() {
        let kind = AttackKind::Light;
        assert!(matches!(kind, AttackKind::Light));
    }

    #[test]
    fn test_attack_kind_heavy() {
        let kind = AttackKind::Heavy;
        assert!(matches!(kind, AttackKind::Heavy));
    }

    #[test]
    fn test_attack_kind_clone() {
        let kind = AttackKind::Light;
        let cloned = kind;
        assert!(matches!(cloned, AttackKind::Light));
    }

    #[test]
    fn test_attack_kind_debug() {
        let kind = AttackKind::Heavy;
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("Heavy"));
    }

    #[test]
    fn test_attack_kind_serialization() {
        let kind = AttackKind::Light;
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: AttackKind = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, AttackKind::Light));
    }

    // ComboStep tests
    #[test]
    fn test_combo_step_creation() {
        let step = ComboStep {
            kind: AttackKind::Light,
            window: (0.0, 0.5),
            damage: 10,
            reach: 2.0,
            stagger: 0.25,
        };
        assert!(matches!(step.kind, AttackKind::Light));
        assert_eq!(step.window, (0.0, 0.5));
        assert_eq!(step.damage, 10);
        assert!((step.reach - 2.0).abs() < f32::EPSILON);
        assert!((step.stagger - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_combo_step_heavy() {
        let step = ComboStep {
            kind: AttackKind::Heavy,
            window: (0.2, 0.8),
            damage: 25,
            reach: 3.0,
            stagger: 0.5,
        };
        assert!(matches!(step.kind, AttackKind::Heavy));
        assert_eq!(step.damage, 25);
    }

    #[test]
    fn test_combo_step_serialization() {
        let step = ComboStep {
            kind: AttackKind::Light,
            window: (0.1, 0.3),
            damage: 15,
            reach: 1.5,
            stagger: 0.2,
        };
        let json = serde_json::to_string(&step).unwrap();
        let deserialized: ComboStep = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.damage, 15);
    }

    #[test]
    fn test_combo_step_clone() {
        let step = ComboStep {
            kind: AttackKind::Heavy,
            window: (0.0, 1.0),
            damage: 50,
            reach: 5.0,
            stagger: 1.0,
        };
        let cloned = step.clone();
        assert_eq!(cloned.damage, step.damage);
        assert!((cloned.reach - step.reach).abs() < f32::EPSILON);
    }

    // ComboChain tests
    #[test]
    fn test_combo_chain_creation() {
        let chain = ComboChain {
            name: "Basic Slash".to_string(),
            steps: vec![ComboStep {
                kind: AttackKind::Light,
                window: (0.0, 0.5),
                damage: 10,
                reach: 2.0,
                stagger: 0.2,
            }],
        };
        assert_eq!(chain.name, "Basic Slash");
        assert_eq!(chain.steps.len(), 1);
    }

    #[test]
    fn test_combo_chain_multiple_steps() {
        let chain = ComboChain {
            name: "Three Hit Combo".to_string(),
            steps: vec![
                ComboStep {
                    kind: AttackKind::Light,
                    window: (0.0, 0.3),
                    damage: 10,
                    reach: 2.0,
                    stagger: 0.1,
                },
                ComboStep {
                    kind: AttackKind::Light,
                    window: (0.1, 0.4),
                    damage: 12,
                    reach: 2.0,
                    stagger: 0.15,
                },
                ComboStep {
                    kind: AttackKind::Heavy,
                    window: (0.2, 0.6),
                    damage: 25,
                    reach: 3.0,
                    stagger: 0.5,
                },
            ],
        };
        assert_eq!(chain.steps.len(), 3);
        assert!(matches!(chain.steps[2].kind, AttackKind::Heavy));
    }

    #[test]
    fn test_combo_chain_serialization() {
        let chain = ComboChain {
            name: "Test".to_string(),
            steps: vec![],
        };
        let json = serde_json::to_string(&chain).unwrap();
        let deserialized: ComboChain = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test");
    }

    // AttackState tests
    fn create_test_chain() -> ComboChain {
        ComboChain {
            name: "Test Combo".to_string(),
            steps: vec![
                ComboStep {
                    kind: AttackKind::Light,
                    window: (0.0, 0.5),
                    damage: 10,
                    reach: 2.0,
                    stagger: 0.2,
                },
                ComboStep {
                    kind: AttackKind::Heavy,
                    window: (0.1, 0.6),
                    damage: 20,
                    reach: 3.0,
                    stagger: 0.4,
                },
            ],
        }
    }

    #[test]
    fn test_attack_state_new() {
        let chain = create_test_chain();
        let state = AttackState::new(chain.clone());
        assert_eq!(state.idx, 0);
        assert!(!state.active);
        assert!((state.t_since_last).abs() < f32::EPSILON);
    }

    #[test]
    fn test_attack_state_start() {
        let chain = create_test_chain();
        let mut state = AttackState::new(chain);
        state.idx = 5; // Simulate some previous state
        state.t_since_last = 10.0;

        state.start();

        assert!(state.active);
        assert_eq!(state.idx, 0);
        assert!((state.t_since_last).abs() < f32::EPSILON);
    }

    #[test]
    fn test_attack_state_inactive_tick() {
        let chain = create_test_chain();
        let mut state = AttackState::new(chain);
        // Don't start - should remain inactive

        let mut target = Stats::new(100);
        let attacker_stats = Stats::new(100);

        let (hit, dmg) = state.tick(
            0.1,
            true,
            false,
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            &attacker_stats,
            None,
            &mut target,
        );

        assert!(!hit);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn test_attack_state_chain_name_preserved() {
        let chain = create_test_chain();
        let state = AttackState::new(chain);
        assert_eq!(state.chain.name, "Test Combo");
    }

    #[test]
    fn test_combo_chain_empty_steps() {
        let chain = ComboChain {
            name: "Empty".to_string(),
            steps: vec![],
        };
        assert!(chain.steps.is_empty());
    }

    #[test]
    fn test_attack_state_debug() {
        let chain = create_test_chain();
        let state = AttackState::new(chain);
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("AttackState"));
        assert!(debug_str.contains("active"));
    }

    #[test]
    fn test_combo_step_zero_values() {
        let step = ComboStep {
            kind: AttackKind::Light,
            window: (0.0, 0.0),
            damage: 0,
            reach: 0.0,
            stagger: 0.0,
        };
        assert_eq!(step.damage, 0);
        assert!((step.reach).abs() < f32::EPSILON);
    }

    #[test]
    fn test_combo_step_large_values() {
        let step = ComboStep {
            kind: AttackKind::Heavy,
            window: (0.0, 10.0),
            damage: 1000,
            reach: 100.0,
            stagger: 5.0,
        };
        assert_eq!(step.damage, 1000);
        assert!((step.reach - 100.0).abs() < f32::EPSILON);
    }
}

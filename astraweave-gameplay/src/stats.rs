use crate::DamageType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub hp: i32,
    pub stamina: i32,
    pub power: i32,
    pub defense: i32,
    pub echo_amp: f32,
    pub effects: Vec<StatusEffect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StatusEffect {
    Stagger {
        time: f32,
    },
    Bleed {
        dps: f32,
        time: f32,
    },
    Chill {
        slow: f32,
        time: f32,
    },
    /// Rider from a fire weave or an item `special = "burn"`.
    Burn {
        dps: f32,
        time: f32,
    },
    /// Item `special = "stun"`. Hit-stagger stays [`StatusEffect::Stagger`].
    Stun {
        time: f32,
    },
}

impl Stats {
    pub fn new(hp: i32) -> Self {
        Self {
            hp,
            stamina: 100,
            power: 10,
            defense: 5,
            echo_amp: 1.0,
            effects: vec![],
        }
    }

    pub fn apply_damage(&mut self, amount: i32, dtype: DamageType) -> i32 {
        let physical = (amount as f32 - self.defense as f32 * 0.5).max(1.0) as i32;
        // School scale in basis points after the physical mitigation. Physical and
        // Echo stay at 10000 so existing physical tests keep their numbers.
        let scale_bp: i32 = match dtype {
            DamageType::Physical | DamageType::Echo => 10_000,
            DamageType::Fire => 12_500,
            DamageType::Frost => 10_000,
            DamageType::Shock => 11_000,
            DamageType::Poison => 8_000,
        };
        let taken = ((physical as i64 * scale_bp as i64) / 10_000).max(1) as i32;
        self.hp -= taken;
        self.apply_element_rider(dtype, None);
        taken
    }

    /// Chill, burn, and stun readers. `special` is the item tag `"burn"` or `"stun"`.
    pub fn apply_element_rider(&mut self, dtype: DamageType, special: Option<&str>) {
        match dtype {
            DamageType::Frost => self.effects.push(StatusEffect::Chill {
                slow: 0.25,
                time: 2.0,
            }),
            DamageType::Fire => self.effects.push(StatusEffect::Burn {
                dps: 2.0,
                time: 3.0,
            }),
            DamageType::Poison => self.effects.push(StatusEffect::Bleed {
                dps: 2.0,
                time: 3.0,
            }),
            _ => {}
        }
        match special {
            Some("burn") => self.effects.push(StatusEffect::Burn {
                dps: 4.0,
                time: 3.0,
            }),
            Some("stun") => self.effects.push(StatusEffect::Stun { time: 1.5 }),
            _ => {}
        }
    }

    /// 10000 is full speed. Chill `slow` is a fraction in `0..=1`, read here.
    pub fn move_scale_bp(&self) -> u16 {
        let mut slow_bp: u32 = 0;
        for effect in &self.effects {
            if let StatusEffect::Chill { slow, .. } = effect {
                let clamped = slow.clamp(0.0, 1.0);
                slow_bp = slow_bp.saturating_add((clamped * 10_000.0) as u32);
            }
        }
        10_000u32.saturating_sub(slow_bp).min(10_000) as u16
    }

    pub fn is_stunned(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| matches!(effect, StatusEffect::Stun { time } if *time > 0.0))
    }

    /// Displacement in millimetres after chill. This is the movement reader.
    pub fn scaled_milli(&self, step_milli: i32) -> i32 {
        (step_milli as i64 * self.move_scale_bp() as i64 / 10_000) as i32
    }

    pub fn tick(&mut self, dt: f32) -> i32 {
        // returns aggregate DoT damage this tick
        let mut dot = 0.0;
        self.effects.retain_mut(|e| match e {
            StatusEffect::Bleed { dps, time } => {
                dot += *dps * dt;
                *time -= dt;
                *time > 0.0
            }
            StatusEffect::Stagger { time } | StatusEffect::Stun { time } => {
                *time -= dt;
                *time > 0.0
            }
            StatusEffect::Chill { time, .. } => {
                *time -= dt;
                *time > 0.0
            }
            StatusEffect::Burn { dps, time } => {
                dot += *dps * dt;
                *time -= dt;
                *time > 0.0
            }
        });
        let d = dot as i32;
        self.hp -= d;
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_new_creates_default_values() {
        let stats = Stats::new(100);
        assert_eq!(stats.hp, 100);
        assert_eq!(stats.stamina, 100);
        assert_eq!(stats.power, 10);
        assert_eq!(stats.defense, 5);
        assert!((stats.echo_amp - 1.0).abs() < f32::EPSILON);
        assert!(stats.effects.is_empty());
    }

    #[test]
    fn test_stats_new_with_different_hp() {
        let stats = Stats::new(50);
        assert_eq!(stats.hp, 50);
        assert_eq!(stats.stamina, 100);
    }

    #[test]
    fn test_stats_apply_damage_with_defense_mitigation() {
        let mut stats = Stats::new(100);
        stats.defense = 10;
        // damage mitigated = max(amount - defense * 0.5, 1) = max(20 - 5, 1) = 15
        let mitigated = stats.apply_damage(20, DamageType::Physical);
        assert_eq!(mitigated, 15);
        assert_eq!(stats.hp, 85);
    }

    #[test]
    fn test_stats_apply_damage_minimum_damage_is_one() {
        let mut stats = Stats::new(100);
        stats.defense = 100; // Very high defense
                             // damage mitigated = max(5 - 50, 1) = 1
        let mitigated = stats.apply_damage(5, DamageType::Physical);
        assert_eq!(mitigated, 1);
        assert_eq!(stats.hp, 99);
    }

    #[test]
    fn test_stats_apply_damage_zero_defense() {
        let mut stats = Stats::new(100);
        stats.defense = 0;
        let mitigated = stats.apply_damage(30, DamageType::Fire);
        // Fire is 12500 basis points of the physical result.
        assert_eq!(mitigated, 37);
        assert_eq!(stats.hp, 63);
        assert!(stats
            .effects
            .iter()
            .any(|e| matches!(e, StatusEffect::Burn { .. })));
    }

    #[test]
    fn hit_stagger_is_not_stun() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Stagger { time: 0.4 });
        assert!(!stats.is_stunned());
        stats.apply_element_rider(DamageType::Physical, Some("stun"));
        assert!(stats.is_stunned());
        stats.tick(1.5);
        assert!(!stats.is_stunned());
    }

    #[test]
    fn test_stats_tick_no_effects() {
        let mut stats = Stats::new(100);
        let dot = stats.tick(0.5);
        assert_eq!(dot, 0);
        assert_eq!(stats.hp, 100);
    }

    #[test]
    fn test_stats_tick_bleed_effect_deals_damage() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Bleed {
            dps: 10.0,
            time: 2.0,
        });
        // dot = 10 * 1.0 = 10
        let dot = stats.tick(1.0);
        assert_eq!(dot, 10);
        assert_eq!(stats.hp, 90);
    }

    #[test]
    fn test_stats_tick_bleed_effect_expires() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Bleed {
            dps: 5.0,
            time: 0.5,
        });
        stats.tick(1.0); // Effect should expire
        assert!(stats.effects.is_empty());
    }

    #[test]
    fn test_stats_tick_stagger_effect() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Stagger { time: 2.0 });
        let dot = stats.tick(0.5);
        assert_eq!(dot, 0); // Stagger doesn't deal damage
        assert_eq!(stats.hp, 100);
        assert_eq!(stats.effects.len(), 1);
    }

    #[test]
    fn test_stats_tick_stagger_effect_expires() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Stagger { time: 1.0 });
        stats.tick(2.0); // Effect should expire
        assert!(stats.effects.is_empty());
    }

    #[test]
    fn test_stats_tick_chill_effect() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Chill {
            slow: 0.5,
            time: 3.0,
        });
        let dot = stats.tick(1.0);
        assert_eq!(dot, 0); // Chill doesn't deal damage
        assert_eq!(stats.hp, 100);
        assert_eq!(stats.effects.len(), 1);
    }

    #[test]
    fn test_stats_tick_multiple_effects() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Bleed {
            dps: 5.0,
            time: 2.0,
        });
        stats.effects.push(StatusEffect::Bleed {
            dps: 3.0,
            time: 2.0,
        });
        stats.effects.push(StatusEffect::Stagger { time: 1.0 });
        // dot = 5 * 1.0 + 3 * 1.0 = 8
        let dot = stats.tick(1.0);
        assert_eq!(dot, 8);
        assert_eq!(stats.hp, 92);
        // After tick with dt=1.0, stagger has time=0.0 and is removed
        // Only the 2 bleeds remain (time=1.0 each)
        assert_eq!(stats.effects.len(), 2);
    }

    #[test]
    fn test_stats_tick_partial_effect_expiry() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Bleed {
            dps: 10.0,
            time: 0.5,
        }); // Will expire
        stats.effects.push(StatusEffect::Bleed {
            dps: 5.0,
            time: 2.0,
        }); // Will remain
        stats.tick(1.0);
        assert_eq!(stats.effects.len(), 1);
    }

    #[test]
    fn test_stats_serialization() {
        let stats = Stats::new(75);
        let json = serde_json::to_string(&stats).expect("serialize");
        let deserialized: Stats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.hp, 75);
        assert_eq!(deserialized.stamina, 100);
    }

    #[test]
    fn test_status_effect_serialization() {
        let effect = StatusEffect::Bleed {
            dps: 5.0,
            time: 3.0,
        };
        let json = serde_json::to_string(&effect).expect("serialize");
        assert!(json.contains("Bleed"));
    }

    #[test]
    fn test_stats_clone() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Stagger { time: 1.0 });
        let cloned = stats.clone();
        assert_eq!(cloned.hp, 100);
        assert_eq!(cloned.effects.len(), 1);
    }

    #[test]
    fn test_stats_apply_damage_different_types() {
        let mut stats = Stats::new(100);
        stats.defense = 4;
        stats.apply_damage(10, DamageType::Echo);
        assert_eq!(stats.hp, 92); // 10 - 2 = 8
        stats.apply_damage(10, DamageType::Frost);
        assert_eq!(stats.hp, 84);
        stats.apply_damage(10, DamageType::Shock);
        assert_eq!(stats.hp, 76);
    }

    #[test]
    fn test_stats_hp_can_go_negative() {
        let mut stats = Stats::new(10);
        stats.defense = 0;
        stats.apply_damage(100, DamageType::Physical);
        assert_eq!(stats.hp, -90);
    }

    #[test]
    fn test_stats_tick_accumulates_bleed_fractional() {
        let mut stats = Stats::new(100);
        stats.effects.push(StatusEffect::Bleed {
            dps: 3.0,
            time: 10.0,
        });
        // dt=0.1, dot = 3.0 * 0.1 = 0.3, truncated to 0
        let dot = stats.tick(0.1);
        assert_eq!(dot, 0);
        assert_eq!(stats.hp, 100);
    }

    #[test]
    fn test_stats_tick_high_dps_bleed() {
        let mut stats = Stats::new(1000);
        stats.effects.push(StatusEffect::Bleed {
            dps: 100.0,
            time: 5.0,
        });
        let dot = stats.tick(1.0);
        assert_eq!(dot, 100);
        assert_eq!(stats.hp, 900);
    }
}

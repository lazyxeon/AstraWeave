use astraweave_core::{ActionStep, IVec2, ToolRegistry, WorldSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicConfig {
    pub rules: Vec<HeuristicRule>,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                // Default rules matching previous hardcoded logic
                HeuristicRule {
                    condition: HeuristicCondition::LowMorale { threshold: 30.0 },
                    action: HeuristicAction::HealSelf,
                },
                HeuristicRule {
                    condition: HeuristicCondition::LowAmmo { threshold: 0 },
                    action: HeuristicAction::Reload,
                },
                HeuristicRule {
                    condition: HeuristicCondition::EnemyNearby { max_distance: 3.0 },
                    action: HeuristicAction::AttackNearestEnemy,
                },
                HeuristicRule {
                    condition: HeuristicCondition::EnemyVisible, // Fallback if not close enough to attack
                    action: HeuristicAction::TakeCover { distance: 2.0 },
                },
                HeuristicRule {
                    condition: HeuristicCondition::ObjectiveContains {
                        keyword: "extract".to_string(),
                    },
                    action: HeuristicAction::MoveToObjective,
                },
                HeuristicRule {
                    condition: HeuristicCondition::ObjectiveContains {
                        keyword: "reach".to_string(),
                    },
                    action: HeuristicAction::MoveToObjective,
                },
                HeuristicRule {
                    condition: HeuristicCondition::Always, // "Nothing urgent" fallback
                    action: HeuristicAction::Scan { radius: 10.0 },
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicRule {
    pub condition: HeuristicCondition,
    pub action: HeuristicAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum HeuristicCondition {
    LowMorale { threshold: f32 },
    LowAmmo { threshold: u32 },
    EnemyNearby { max_distance: f32 },
    EnemyVisible,
    ObjectiveContains { keyword: String },
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum HeuristicAction {
    HealSelf,
    Reload,
    AttackNearestEnemy,
    TakeCover { distance: f32 },
    MoveToObjective,
    Scan { radius: f32 },
}

impl HeuristicRule {
    pub fn evaluate(&self, snap: &WorldSnapshot, reg: &ToolRegistry) -> Option<ActionStep> {
        if self.check_condition(snap) {
            self.create_action(snap, reg)
        } else {
            None
        }
    }

    fn check_condition(&self, snap: &WorldSnapshot) -> bool {
        match &self.condition {
            HeuristicCondition::LowMorale { threshold } => snap.me.morale < *threshold,
            HeuristicCondition::LowAmmo { threshold } => snap.me.ammo <= *threshold as i32,
            HeuristicCondition::EnemyNearby { max_distance } => {
                if let Some(enemy) = snap.enemies.first() {
                    let dist = ((snap.me.pos.x - enemy.pos.x).abs()
                        + (snap.me.pos.y - enemy.pos.y).abs())
                        as f32;
                    dist <= *max_distance
                } else {
                    false
                }
            }
            HeuristicCondition::EnemyVisible => !snap.enemies.is_empty(),
            HeuristicCondition::ObjectiveContains { keyword } => snap
                .objective
                .as_ref()
                .map(|o| o.contains(keyword))
                .unwrap_or(false),
            HeuristicCondition::Always => true,
        }
    }

    fn create_action(&self, snap: &WorldSnapshot, reg: &ToolRegistry) -> Option<ActionStep> {
        // Helper to check if registry has a tool (case-insensitive to support both
        // snake_case (from get_all_tools) and PascalCase (from action_step_to_tool_name))
        fn has_tool(reg: &ToolRegistry, name: &str) -> bool {
            reg.tools.iter().any(|t| t.name.eq_ignore_ascii_case(name))
        }

        match &self.action {
            HeuristicAction::HealSelf => {
                if has_tool(reg, "heal") {
                    Some(ActionStep::Heal { target_id: Some(0) })
                } else {
                    None
                }
            }
            HeuristicAction::Reload => {
                if has_tool(reg, "reload") {
                    Some(ActionStep::Reload)
                } else {
                    None
                }
            }
            HeuristicAction::AttackNearestEnemy => {
                if let Some(enemy) = snap.enemies.first() {
                    if has_tool(reg, "attack") {
                        Some(ActionStep::Attack {
                            target_id: enemy.id,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            HeuristicAction::TakeCover { distance } => {
                if let Some(enemy) = snap.enemies.first() {
                    if has_tool(reg, "takecover") || has_tool(reg, "take_cover") {
                        let dist_int = *distance as i32;
                        let cover_x = if snap.me.pos.x > enemy.pos.x {
                            snap.me.pos.x + dist_int
                        } else {
                            snap.me.pos.x - dist_int
                        };
                        Some(ActionStep::TakeCover {
                            position: Some(IVec2 {
                                x: cover_x,
                                y: snap.me.pos.y,
                            }),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            HeuristicAction::MoveToObjective => {
                if let Some(poi) = snap.pois.first() {
                    if has_tool(reg, "moveto") || has_tool(reg, "move_to") {
                        Some(ActionStep::MoveTo {
                            x: poi.pos.x,
                            y: poi.pos.y,
                            speed: None,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            HeuristicAction::Scan { radius } => {
                if has_tool(reg, "scan") {
                    Some(ActionStep::Scan { radius: *radius })
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, Constraints, CoverType, EnemyState, PlayerState, Poi, Stance, ToolSpec};
    use std::collections::BTreeMap;

    fn create_basic_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                pos: IVec2 { x: 3, y: 3 },
                ammo: 10,
                morale: 80.0,
                cooldowns: BTreeMap::new(),
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    fn create_full_registry() -> ToolRegistry {
        ToolRegistry {
            tools: vec![
                ToolSpec {
                    name: "move_to".to_string(),
                    args: BTreeMap::new(),
                },
                ToolSpec {
                    name: "attack".to_string(),
                    args: BTreeMap::new(),
                },
                ToolSpec {
                    name: "heal".to_string(),
                    args: BTreeMap::new(),
                },
                ToolSpec {
                    name: "reload".to_string(),
                    args: BTreeMap::new(),
                },
                ToolSpec {
                    name: "take_cover".to_string(),
                    args: BTreeMap::new(),
                },
                ToolSpec {
                    name: "scan".to_string(),
                    args: BTreeMap::new(),
                },
            ],
            constraints: Constraints {
                enforce_cooldowns: true,
                enforce_los: true,
                enforce_stamina: true,
            },
        }
    }

    fn create_empty_registry() -> ToolRegistry {
        ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        }
    }

    // ============================================================
    // Config Tests
    // ============================================================

    #[test]
    fn test_heuristic_config_default() {
        let config = HeuristicConfig::default();
        assert!(!config.rules.is_empty(), "Default config should have rules");
        assert!(
            config.rules.len() >= 6,
            "Default config should have at least 6 rules"
        );
    }

    #[test]
    fn test_heuristic_config_serialization() {
        let config = HeuristicConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: HeuristicConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.rules.len(), config.rules.len());
    }

    #[test]
    fn test_heuristic_config_custom_rules() {
        let config = HeuristicConfig {
            rules: vec![HeuristicRule {
                condition: HeuristicCondition::Always,
                action: HeuristicAction::Scan { radius: 5.0 },
            }],
        };
        assert_eq!(config.rules.len(), 1);
    }

    // ============================================================
    // Condition Tests
    // ============================================================

    #[test]
    fn test_condition_low_morale_true() {
        let mut snap = create_basic_snapshot();
        snap.me.morale = 20.0;

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowMorale { threshold: 30.0 },
            action: HeuristicAction::HealSelf,
        };

        assert!(rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_low_morale_false() {
        let snap = create_basic_snapshot(); // morale = 80.0

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowMorale { threshold: 30.0 },
            action: HeuristicAction::HealSelf,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_low_ammo_true() {
        let mut snap = create_basic_snapshot();
        snap.me.ammo = 0;

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowAmmo { threshold: 0 },
            action: HeuristicAction::Reload,
        };

        assert!(rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_low_ammo_false() {
        let snap = create_basic_snapshot(); // ammo = 10

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowAmmo { threshold: 0 },
            action: HeuristicAction::Reload,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_enemy_nearby_true() {
        let mut snap = create_basic_snapshot();
        snap.enemies.push(EnemyState {
            id: 99,
            pos: IVec2 { x: 4, y: 4 }, // 2 tiles away (manhattan)
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });

        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyNearby { max_distance: 3.0 },
            action: HeuristicAction::AttackNearestEnemy,
        };

        assert!(rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_enemy_nearby_false_far() {
        let mut snap = create_basic_snapshot();
        snap.enemies.push(EnemyState {
            id: 99,
            pos: IVec2 { x: 20, y: 20 }, // 34 tiles away
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });

        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyNearby { max_distance: 3.0 },
            action: HeuristicAction::AttackNearestEnemy,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_enemy_nearby_false_no_enemies() {
        let snap = create_basic_snapshot();

        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyNearby { max_distance: 3.0 },
            action: HeuristicAction::AttackNearestEnemy,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_enemy_visible_true() {
        let mut snap = create_basic_snapshot();
        snap.enemies.push(EnemyState {
            id: 99,
            pos: IVec2 { x: 100, y: 100 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });

        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyVisible,
            action: HeuristicAction::TakeCover { distance: 2.0 },
        };

        assert!(rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_enemy_visible_false() {
        let snap = create_basic_snapshot();

        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyVisible,
            action: HeuristicAction::TakeCover { distance: 2.0 },
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_objective_contains_true() {
        let mut snap = create_basic_snapshot();
        snap.objective = Some("extract to helipad".to_string()); // lowercase to match keyword

        let rule = HeuristicRule {
            condition: HeuristicCondition::ObjectiveContains {
                keyword: "extract".to_string(),
            },
            action: HeuristicAction::MoveToObjective,
        };

        assert!(rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_objective_contains_false_no_match() {
        let mut snap = create_basic_snapshot();
        snap.objective = Some("Defend position".to_string());

        let rule = HeuristicRule {
            condition: HeuristicCondition::ObjectiveContains {
                keyword: "extract".to_string(),
            },
            action: HeuristicAction::MoveToObjective,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_objective_contains_false_no_objective() {
        let snap = create_basic_snapshot();

        let rule = HeuristicRule {
            condition: HeuristicCondition::ObjectiveContains {
                keyword: "extract".to_string(),
            },
            action: HeuristicAction::MoveToObjective,
        };

        assert!(!rule.check_condition(&snap));
    }

    #[test]
    fn test_condition_always() {
        let snap = create_basic_snapshot();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::Scan { radius: 10.0 },
        };

        assert!(rule.check_condition(&snap));
    }

    // ============================================================
    // Action Creation Tests
    // ============================================================

    #[test]
    fn test_action_heal_self_with_tool() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::HealSelf,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        assert!(matches!(
            action.unwrap(),
            ActionStep::Heal { target_id: Some(0) }
        ));
    }

    #[test]
    fn test_action_heal_self_no_tool() {
        let snap = create_basic_snapshot();
        let reg = create_empty_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::HealSelf,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_reload_with_tool() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::Reload,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        assert!(matches!(action.unwrap(), ActionStep::Reload));
    }

    #[test]
    fn test_action_reload_no_tool() {
        let snap = create_basic_snapshot();
        let reg = create_empty_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::Reload,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_attack_nearest_enemy_with_enemy() {
        let mut snap = create_basic_snapshot();
        snap.enemies.push(EnemyState {
            id: 42,
            pos: IVec2 { x: 5, y: 5 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::AttackNearestEnemy,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        match action.unwrap() {
            ActionStep::Attack { target_id } => assert_eq!(target_id, 42),
            _ => panic!("Expected Attack action"),
        }
    }

    #[test]
    fn test_action_attack_nearest_enemy_no_enemies() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::AttackNearestEnemy,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_attack_nearest_enemy_no_tool() {
        let mut snap = create_basic_snapshot();
        snap.enemies.push(EnemyState {
            id: 42,
            pos: IVec2 { x: 5, y: 5 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });
        let reg = create_empty_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::AttackNearestEnemy,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_take_cover_when_right_of_enemy() {
        let mut snap = create_basic_snapshot();
        snap.me.pos = IVec2 { x: 10, y: 5 }; // Right of enemy
        snap.enemies.push(EnemyState {
            id: 99,
            pos: IVec2 { x: 5, y: 5 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::TakeCover { distance: 2.0 },
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        match action.unwrap() {
            ActionStep::TakeCover { position } => {
                let pos = position.unwrap();
                assert!(
                    pos.x > snap.me.pos.x,
                    "Cover should be away from enemy (right)"
                );
            }
            _ => panic!("Expected TakeCover action"),
        }
    }

    #[test]
    fn test_action_take_cover_when_left_of_enemy() {
        let mut snap = create_basic_snapshot();
        snap.me.pos = IVec2 { x: 3, y: 5 }; // Left of enemy
        snap.enemies.push(EnemyState {
            id: 99,
            pos: IVec2 { x: 10, y: 5 },
            hp: 100,
            cover: CoverType::None,
            last_seen: 0.0,
        });
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::TakeCover { distance: 2.0 },
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        match action.unwrap() {
            ActionStep::TakeCover { position } => {
                let pos = position.unwrap();
                assert!(
                    pos.x < snap.me.pos.x,
                    "Cover should be away from enemy (left)"
                );
            }
            _ => panic!("Expected TakeCover action"),
        }
    }

    #[test]
    fn test_action_take_cover_no_enemies() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::TakeCover { distance: 2.0 },
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_move_to_objective_with_poi() {
        let mut snap = create_basic_snapshot();
        snap.pois.push(Poi {
            k: "extract".to_string(),
            pos: IVec2 { x: 15, y: 15 },
        });
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::MoveToObjective,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        match action.unwrap() {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(x, 15);
                assert_eq!(y, 15);
            }
            _ => panic!("Expected MoveTo action"),
        }
    }

    #[test]
    fn test_action_move_to_objective_no_poi() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::MoveToObjective,
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    #[test]
    fn test_action_scan_with_tool() {
        let snap = create_basic_snapshot();
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::Scan { radius: 15.0 },
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_some());
        match action.unwrap() {
            ActionStep::Scan { radius } => assert!((radius - 15.0).abs() < 0.001),
            _ => panic!("Expected Scan action"),
        }
    }

    #[test]
    fn test_action_scan_no_tool() {
        let snap = create_basic_snapshot();
        let reg = create_empty_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::Always,
            action: HeuristicAction::Scan { radius: 15.0 },
        };

        let action = rule.create_action(&snap, &reg);
        assert!(action.is_none());
    }

    // ============================================================
    // Full Evaluate Tests
    // ============================================================

    #[test]
    fn test_evaluate_returns_action_when_condition_true() {
        let mut snap = create_basic_snapshot();
        snap.me.morale = 20.0;
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowMorale { threshold: 30.0 },
            action: HeuristicAction::HealSelf,
        };

        let result = rule.evaluate(&snap, &reg);
        assert!(result.is_some());
    }

    #[test]
    fn test_evaluate_returns_none_when_condition_false() {
        let snap = create_basic_snapshot(); // morale = 80.0
        let reg = create_full_registry();

        let rule = HeuristicRule {
            condition: HeuristicCondition::LowMorale { threshold: 30.0 },
            action: HeuristicAction::HealSelf,
        };

        let result = rule.evaluate(&snap, &reg);
        assert!(result.is_none());
    }

    // ============================================================
    // Serialization Tests
    // ============================================================

    #[test]
    fn test_condition_serialization_low_morale() {
        let condition = HeuristicCondition::LowMorale { threshold: 25.0 };
        let json = serde_json::to_string(&condition).unwrap();
        assert!(json.contains("LowMorale"));
        assert!(json.contains("25"));

        let parsed: HeuristicCondition = serde_json::from_str(&json).unwrap();
        match parsed {
            HeuristicCondition::LowMorale { threshold } => {
                assert!((threshold - 25.0).abs() < 0.001)
            }
            _ => panic!("Wrong condition type"),
        }
    }

    #[test]
    fn test_action_serialization_scan() {
        let action = HeuristicAction::Scan { radius: 12.5 };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("Scan"));

        let parsed: HeuristicAction = serde_json::from_str(&json).unwrap();
        match parsed {
            HeuristicAction::Scan { radius } => assert!((radius - 12.5).abs() < 0.001),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_rule_serialization_roundtrip() {
        let rule = HeuristicRule {
            condition: HeuristicCondition::EnemyNearby { max_distance: 5.0 },
            action: HeuristicAction::AttackNearestEnemy,
        };

        let json = serde_json::to_string(&rule).unwrap();
        let parsed: HeuristicRule = serde_json::from_str(&json).unwrap();

        // Verify condition
        match &parsed.condition {
            HeuristicCondition::EnemyNearby { max_distance } => {
                assert!((max_distance - 5.0).abs() < 0.001)
            }
            _ => panic!("Wrong condition type"),
        }

        // Verify action
        assert!(matches!(parsed.action, HeuristicAction::AttackNearestEnemy));
    }
}

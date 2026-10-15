//! Mutation-resistant comprehensive tests for astraweave-core.
//!
//! Targets all major untested mutation sites:
//! - ActionStep: category(), action_name(), is_* queries, targets_entity, target_entity, has_position, Display
//! - CardinalDirection: to_unit_vector() for all 8 directions
//! - DistanceCategory: to_range(), midpoint()
//! - Display impls: IVec2, WorldSnapshot, PlanIntent, ActionStep
//! - tool_vocabulary: get_all_tools, get_categories, get_tools_by_category, generate_tool_schema
//! - MetricsRegistry: edge cases
//! - Events: writer/reader lifecycle
//! - EntityBridge: mapping lifecycle

use astraweave_core::schema::*;

// =============================================================================
// MODULE 1: ActionStep::category() — every variant to its correct category
// =============================================================================

mod action_step_category_tests {
    use super::*;

    // --- Movement (6 variants) ---
    #[test]
    fn moveto_category_is_movement() {
        let s = ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None,
        };
        assert_eq!(s.category(), "Movement");
    }
    #[test]
    fn approach_category_is_movement() {
        let s = ActionStep::Approach {
            target_id: 1,
            distance: 2.0,
        };
        assert_eq!(s.category(), "Movement");
    }
    #[test]
    fn retreat_category_is_movement() {
        let s = ActionStep::Retreat {
            target_id: 1,
            distance: 10.0,
        };
        assert_eq!(s.category(), "Movement");
    }
    #[test]
    fn take_cover_category_is_movement() {
        let s = ActionStep::TakeCover { position: None };
        assert_eq!(s.category(), "Movement");
    }
    #[test]
    fn strafe_category_is_movement() {
        let s = ActionStep::Strafe {
            target_id: 1,
            direction: StrafeDirection::Left,
        };
        assert_eq!(s.category(), "Movement");
    }
    #[test]
    fn patrol_category_is_movement() {
        let s = ActionStep::Patrol { waypoints: vec![] };
        assert_eq!(s.category(), "Movement");
    }

    // --- Offensive (8 variants) ---
    #[test]
    fn attack_category_is_offensive() {
        assert_eq!(ActionStep::Attack { target_id: 1 }.category(), "Offensive");
    }
    #[test]
    fn aimed_shot_category_is_offensive() {
        assert_eq!(
            ActionStep::AimedShot { target_id: 1 }.category(),
            "Offensive"
        );
    }
    #[test]
    fn quick_attack_category_is_offensive() {
        assert_eq!(
            ActionStep::QuickAttack { target_id: 1 }.category(),
            "Offensive"
        );
    }
    #[test]
    fn heavy_attack_category_is_offensive() {
        assert_eq!(
            ActionStep::HeavyAttack { target_id: 1 }.category(),
            "Offensive"
        );
    }
    #[test]
    fn aoe_attack_category_is_offensive() {
        assert_eq!(
            ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0
            }
            .category(),
            "Offensive"
        );
    }
    #[test]
    fn throw_explosive_category_is_offensive() {
        assert_eq!(
            ActionStep::ThrowExplosive { x: 0, y: 0 }.category(),
            "Offensive"
        );
    }
    #[test]
    fn cover_fire_category_is_offensive() {
        assert_eq!(
            ActionStep::CoverFire {
                target_id: 1,
                duration: 3.0
            }
            .category(),
            "Offensive"
        );
    }
    #[test]
    fn charge_category_is_offensive() {
        assert_eq!(ActionStep::Charge { target_id: 1 }.category(), "Offensive");
    }

    // --- Defensive (6 variants) ---
    #[test]
    fn block_category_is_defensive() {
        assert_eq!(ActionStep::Block.category(), "Defensive");
    }
    #[test]
    fn dodge_category_is_defensive() {
        assert_eq!(
            ActionStep::Dodge { direction: None }.category(),
            "Defensive"
        );
    }
    #[test]
    fn parry_category_is_defensive() {
        assert_eq!(ActionStep::Parry.category(), "Defensive");
    }
    #[test]
    fn throw_smoke_category_is_defensive() {
        assert_eq!(
            ActionStep::ThrowSmoke { x: 0, y: 0 }.category(),
            "Defensive"
        );
    }
    #[test]
    fn heal_category_is_defensive() {
        assert_eq!(ActionStep::Heal { target_id: None }.category(), "Defensive");
    }
    #[test]
    fn use_defensive_ability_category_is_defensive() {
        assert_eq!(
            ActionStep::UseDefensiveAbility {
                ability_name: "shield".into()
            }
            .category(),
            "Defensive"
        );
    }

    // --- Equipment (5 variants) ---
    #[test]
    fn equip_weapon_category_is_equipment() {
        assert_eq!(
            ActionStep::EquipWeapon {
                weapon_name: "sword".into()
            }
            .category(),
            "Equipment"
        );
    }
    #[test]
    fn switch_weapon_category_is_equipment() {
        assert_eq!(ActionStep::SwitchWeapon { slot: 0 }.category(), "Equipment");
    }
    #[test]
    fn reload_category_is_equipment() {
        assert_eq!(ActionStep::Reload.category(), "Equipment");
    }
    #[test]
    fn use_item_category_is_equipment() {
        assert_eq!(
            ActionStep::UseItem {
                item_name: "potion".into()
            }
            .category(),
            "Equipment"
        );
    }
    #[test]
    fn drop_item_category_is_equipment() {
        assert_eq!(
            ActionStep::DropItem {
                item_name: "potion".into()
            }
            .category(),
            "Equipment"
        );
    }

    // --- Tactical (7 variants) ---
    #[test]
    fn call_reinforcements_category_is_tactical() {
        assert_eq!(
            ActionStep::CallReinforcements { count: 3 }.category(),
            "Tactical"
        );
    }
    #[test]
    fn mark_target_category_is_tactical() {
        assert_eq!(
            ActionStep::MarkTarget { target_id: 1 }.category(),
            "Tactical"
        );
    }
    #[test]
    fn request_cover_category_is_tactical() {
        assert_eq!(
            ActionStep::RequestCover { duration: 5.0 }.category(),
            "Tactical"
        );
    }
    #[test]
    fn coordinate_attack_category_is_tactical() {
        assert_eq!(
            ActionStep::CoordinateAttack { target_id: 1 }.category(),
            "Tactical"
        );
    }
    #[test]
    fn set_ambush_category_is_tactical() {
        assert_eq!(
            ActionStep::SetAmbush {
                position: IVec2::new(5, 5)
            }
            .category(),
            "Tactical"
        );
    }
    #[test]
    fn distract_category_is_tactical() {
        assert_eq!(ActionStep::Distract { target_id: 1 }.category(), "Tactical");
    }
    #[test]
    fn regroup_category_is_tactical() {
        assert_eq!(
            ActionStep::Regroup {
                rally_point: IVec2::new(0, 0)
            }
            .category(),
            "Tactical"
        );
    }

    // --- Utility (5 variants) ---
    #[test]
    fn scan_category_is_utility() {
        assert_eq!(ActionStep::Scan { radius: 10.0 }.category(), "Utility");
    }
    #[test]
    fn wait_category_is_utility() {
        assert_eq!(ActionStep::Wait { duration: 1.0 }.category(), "Utility");
    }
    #[test]
    fn interact_category_is_utility() {
        assert_eq!(ActionStep::Interact { target_id: 1 }.category(), "Utility");
    }
    #[test]
    fn use_ability_category_is_utility() {
        assert_eq!(
            ActionStep::UseAbility {
                ability_name: "dash".into()
            }
            .category(),
            "Utility"
        );
    }
    #[test]
    fn taunt_category_is_utility() {
        assert_eq!(ActionStep::Taunt { target_id: 1 }.category(), "Utility");
    }

    // --- Legacy (2 variants) ---
    #[test]
    fn throw_category_is_legacy() {
        assert_eq!(
            ActionStep::Throw {
                item: "rock".into(),
                x: 0,
                y: 0
            }
            .category(),
            "Legacy"
        );
    }
    #[test]
    fn revive_category_is_legacy() {
        assert_eq!(ActionStep::Revive { ally_id: 1 }.category(), "Legacy");
    }

    // --- Terrain (1 variant) ---
    #[test]
    fn modify_terrain_category_is_terrain() {
        let req = TerrainGenerationRequest {
            request_id: "t1".into(),
            narrative_reason: "test".into(),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity: 0.5,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: vec![],
            seed: None,
        };
        assert_eq!(
            ActionStep::ModifyTerrain {
                request_id: "t1".into(),
                payload: req
            }
            .category(),
            "Terrain"
        );
    }
}

// =============================================================================
// MODULE 2: ActionStep::action_name() — every variant maps to correct string
// =============================================================================

mod action_step_action_name_tests {
    use super::*;

    #[test]
    fn moveto_action_name() {
        assert_eq!(
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
            .action_name(),
            "MoveTo"
        );
    }
    #[test]
    fn approach_action_name() {
        assert_eq!(
            ActionStep::Approach {
                target_id: 1,
                distance: 2.0
            }
            .action_name(),
            "Approach"
        );
    }
    #[test]
    fn retreat_action_name() {
        assert_eq!(
            ActionStep::Retreat {
                target_id: 1,
                distance: 10.0
            }
            .action_name(),
            "Retreat"
        );
    }
    #[test]
    fn take_cover_action_name() {
        assert_eq!(
            ActionStep::TakeCover { position: None }.action_name(),
            "TakeCover"
        );
    }
    #[test]
    fn strafe_action_name() {
        assert_eq!(
            ActionStep::Strafe {
                target_id: 1,
                direction: StrafeDirection::Left
            }
            .action_name(),
            "Strafe"
        );
    }
    #[test]
    fn patrol_action_name() {
        assert_eq!(
            ActionStep::Patrol { waypoints: vec![] }.action_name(),
            "Patrol"
        );
    }
    #[test]
    fn attack_action_name() {
        assert_eq!(ActionStep::Attack { target_id: 1 }.action_name(), "Attack");
    }
    #[test]
    fn aimed_shot_action_name() {
        assert_eq!(
            ActionStep::AimedShot { target_id: 1 }.action_name(),
            "AimedShot"
        );
    }
    #[test]
    fn quick_attack_action_name() {
        assert_eq!(
            ActionStep::QuickAttack { target_id: 1 }.action_name(),
            "QuickAttack"
        );
    }
    #[test]
    fn heavy_attack_action_name() {
        assert_eq!(
            ActionStep::HeavyAttack { target_id: 1 }.action_name(),
            "HeavyAttack"
        );
    }
    #[test]
    fn aoe_attack_action_name() {
        assert_eq!(
            ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0
            }
            .action_name(),
            "AoEAttack"
        );
    }
    #[test]
    fn throw_explosive_action_name() {
        assert_eq!(
            ActionStep::ThrowExplosive { x: 0, y: 0 }.action_name(),
            "ThrowExplosive"
        );
    }
    #[test]
    fn cover_fire_action_name() {
        assert_eq!(
            ActionStep::CoverFire {
                target_id: 1,
                duration: 3.0
            }
            .action_name(),
            "CoverFire"
        );
    }
    #[test]
    fn charge_action_name() {
        assert_eq!(ActionStep::Charge { target_id: 1 }.action_name(), "Charge");
    }
    #[test]
    fn block_action_name() {
        assert_eq!(ActionStep::Block.action_name(), "Block");
    }
    #[test]
    fn dodge_action_name() {
        assert_eq!(ActionStep::Dodge { direction: None }.action_name(), "Dodge");
    }
    #[test]
    fn parry_action_name() {
        assert_eq!(ActionStep::Parry.action_name(), "Parry");
    }
    #[test]
    fn throw_smoke_action_name() {
        assert_eq!(
            ActionStep::ThrowSmoke { x: 0, y: 0 }.action_name(),
            "ThrowSmoke"
        );
    }
    #[test]
    fn heal_action_name() {
        assert_eq!(ActionStep::Heal { target_id: None }.action_name(), "Heal");
    }
    #[test]
    fn use_defensive_ability_action_name() {
        assert_eq!(
            ActionStep::UseDefensiveAbility {
                ability_name: "shield".into()
            }
            .action_name(),
            "UseDefensiveAbility"
        );
    }
    #[test]
    fn equip_weapon_action_name() {
        assert_eq!(
            ActionStep::EquipWeapon {
                weapon_name: "sword".into()
            }
            .action_name(),
            "EquipWeapon"
        );
    }
    #[test]
    fn switch_weapon_action_name() {
        assert_eq!(
            ActionStep::SwitchWeapon { slot: 0 }.action_name(),
            "SwitchWeapon"
        );
    }
    #[test]
    fn reload_action_name() {
        assert_eq!(ActionStep::Reload.action_name(), "Reload");
    }
    #[test]
    fn use_item_action_name() {
        assert_eq!(
            ActionStep::UseItem {
                item_name: "potion".into()
            }
            .action_name(),
            "UseItem"
        );
    }
    #[test]
    fn drop_item_action_name() {
        assert_eq!(
            ActionStep::DropItem {
                item_name: "potion".into()
            }
            .action_name(),
            "DropItem"
        );
    }
    #[test]
    fn call_reinforcements_action_name() {
        assert_eq!(
            ActionStep::CallReinforcements { count: 3 }.action_name(),
            "CallReinforcements"
        );
    }
    #[test]
    fn mark_target_action_name() {
        assert_eq!(
            ActionStep::MarkTarget { target_id: 1 }.action_name(),
            "MarkTarget"
        );
    }
    #[test]
    fn request_cover_action_name() {
        assert_eq!(
            ActionStep::RequestCover { duration: 5.0 }.action_name(),
            "RequestCover"
        );
    }
    #[test]
    fn coordinate_attack_action_name() {
        assert_eq!(
            ActionStep::CoordinateAttack { target_id: 1 }.action_name(),
            "CoordinateAttack"
        );
    }
    #[test]
    fn set_ambush_action_name() {
        assert_eq!(
            ActionStep::SetAmbush {
                position: IVec2::new(5, 5)
            }
            .action_name(),
            "SetAmbush"
        );
    }
    #[test]
    fn distract_action_name() {
        assert_eq!(
            ActionStep::Distract { target_id: 1 }.action_name(),
            "Distract"
        );
    }
    #[test]
    fn regroup_action_name() {
        assert_eq!(
            ActionStep::Regroup {
                rally_point: IVec2::new(0, 0)
            }
            .action_name(),
            "Regroup"
        );
    }
    #[test]
    fn scan_action_name() {
        assert_eq!(ActionStep::Scan { radius: 10.0 }.action_name(), "Scan");
    }
    #[test]
    fn wait_action_name() {
        assert_eq!(ActionStep::Wait { duration: 1.0 }.action_name(), "Wait");
    }
    #[test]
    fn interact_action_name() {
        assert_eq!(
            ActionStep::Interact { target_id: 1 }.action_name(),
            "Interact"
        );
    }
    #[test]
    fn use_ability_action_name() {
        assert_eq!(
            ActionStep::UseAbility {
                ability_name: "dash".into()
            }
            .action_name(),
            "UseAbility"
        );
    }
    #[test]
    fn taunt_action_name() {
        assert_eq!(ActionStep::Taunt { target_id: 1 }.action_name(), "Taunt");
    }
    #[test]
    fn throw_action_name() {
        assert_eq!(
            ActionStep::Throw {
                item: "rock".into(),
                x: 0,
                y: 0
            }
            .action_name(),
            "Throw"
        );
    }
    #[test]
    fn revive_action_name() {
        assert_eq!(ActionStep::Revive { ally_id: 1 }.action_name(), "Revive");
    }
    #[test]
    fn modify_terrain_action_name() {
        let req = TerrainGenerationRequest {
            request_id: "t1".into(),
            narrative_reason: "test".into(),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity: 0.5,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: vec![],
            seed: None,
        };
        assert_eq!(
            ActionStep::ModifyTerrain {
                request_id: "t1".into(),
                payload: req
            }
            .action_name(),
            "ModifyTerrain"
        );
    }
}

// =============================================================================
// MODULE 3: ActionStep::is_* boolean classification queries
// =============================================================================

mod action_step_is_classification_tests {
    use super::*;

    // Helper: test that exactly the right category predicate returns true
    fn assert_only_category(step: &ActionStep, expected: &str) {
        let cats = [
            ("Movement", step.is_movement()),
            ("Offensive", step.is_offensive()),
            ("Defensive", step.is_defensive()),
            ("Equipment", step.is_equipment()),
            ("Tactical", step.is_tactical()),
            ("Utility", step.is_utility()),
        ];
        for (name, value) in &cats {
            if *name == expected {
                assert!(
                    *value,
                    "{} should be true for {:?}",
                    name,
                    step.action_name()
                );
            } else {
                assert!(
                    !*value,
                    "{} should be false for {:?}",
                    name,
                    step.action_name()
                );
            }
        }
    }

    // --- Movement variants ---
    #[test]
    fn moveto_is_only_movement() {
        assert_only_category(
            &ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None,
            },
            "Movement",
        );
    }
    #[test]
    fn approach_is_only_movement() {
        assert_only_category(
            &ActionStep::Approach {
                target_id: 1,
                distance: 2.0,
            },
            "Movement",
        );
    }
    #[test]
    fn retreat_is_only_movement() {
        assert_only_category(
            &ActionStep::Retreat {
                target_id: 1,
                distance: 10.0,
            },
            "Movement",
        );
    }
    #[test]
    fn take_cover_is_only_movement() {
        assert_only_category(&ActionStep::TakeCover { position: None }, "Movement");
    }
    #[test]
    fn strafe_is_only_movement() {
        assert_only_category(
            &ActionStep::Strafe {
                target_id: 1,
                direction: StrafeDirection::Left,
            },
            "Movement",
        );
    }
    #[test]
    fn patrol_is_only_movement() {
        assert_only_category(&ActionStep::Patrol { waypoints: vec![] }, "Movement");
    }

    // --- Offensive variants ---
    #[test]
    fn attack_is_only_offensive() {
        assert_only_category(&ActionStep::Attack { target_id: 1 }, "Offensive");
    }
    #[test]
    fn aimed_shot_is_only_offensive() {
        assert_only_category(&ActionStep::AimedShot { target_id: 1 }, "Offensive");
    }
    #[test]
    fn quick_attack_is_only_offensive() {
        assert_only_category(&ActionStep::QuickAttack { target_id: 1 }, "Offensive");
    }
    #[test]
    fn heavy_attack_is_only_offensive() {
        assert_only_category(&ActionStep::HeavyAttack { target_id: 1 }, "Offensive");
    }
    #[test]
    fn aoe_attack_is_only_offensive() {
        assert_only_category(
            &ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0,
            },
            "Offensive",
        );
    }
    #[test]
    fn throw_explosive_is_only_offensive() {
        assert_only_category(&ActionStep::ThrowExplosive { x: 0, y: 0 }, "Offensive");
    }
    #[test]
    fn cover_fire_is_only_offensive() {
        assert_only_category(
            &ActionStep::CoverFire {
                target_id: 1,
                duration: 3.0,
            },
            "Offensive",
        );
    }
    #[test]
    fn charge_is_only_offensive() {
        assert_only_category(&ActionStep::Charge { target_id: 1 }, "Offensive");
    }

    // --- Defensive variants ---
    #[test]
    fn block_is_only_defensive() {
        assert_only_category(&ActionStep::Block, "Defensive");
    }
    #[test]
    fn dodge_is_only_defensive() {
        assert_only_category(&ActionStep::Dodge { direction: None }, "Defensive");
    }
    #[test]
    fn parry_is_only_defensive() {
        assert_only_category(&ActionStep::Parry, "Defensive");
    }
    #[test]
    fn throw_smoke_is_only_defensive() {
        assert_only_category(&ActionStep::ThrowSmoke { x: 0, y: 0 }, "Defensive");
    }
    #[test]
    fn heal_is_only_defensive() {
        assert_only_category(&ActionStep::Heal { target_id: None }, "Defensive");
    }
    #[test]
    fn use_defensive_ability_is_only_defensive() {
        assert_only_category(
            &ActionStep::UseDefensiveAbility {
                ability_name: "shield".into(),
            },
            "Defensive",
        );
    }

    // --- Equipment variants ---
    #[test]
    fn equip_weapon_is_only_equipment() {
        assert_only_category(
            &ActionStep::EquipWeapon {
                weapon_name: "sword".into(),
            },
            "Equipment",
        );
    }
    #[test]
    fn switch_weapon_is_only_equipment() {
        assert_only_category(&ActionStep::SwitchWeapon { slot: 0 }, "Equipment");
    }
    #[test]
    fn reload_is_only_equipment() {
        assert_only_category(&ActionStep::Reload, "Equipment");
    }
    #[test]
    fn use_item_is_only_equipment() {
        assert_only_category(
            &ActionStep::UseItem {
                item_name: "potion".into(),
            },
            "Equipment",
        );
    }
    #[test]
    fn drop_item_is_only_equipment() {
        assert_only_category(
            &ActionStep::DropItem {
                item_name: "potion".into(),
            },
            "Equipment",
        );
    }

    // --- Tactical variants ---
    #[test]
    fn call_reinforcements_is_only_tactical() {
        assert_only_category(&ActionStep::CallReinforcements { count: 3 }, "Tactical");
    }
    #[test]
    fn mark_target_is_only_tactical() {
        assert_only_category(&ActionStep::MarkTarget { target_id: 1 }, "Tactical");
    }
    #[test]
    fn request_cover_is_only_tactical() {
        assert_only_category(&ActionStep::RequestCover { duration: 5.0 }, "Tactical");
    }
    #[test]
    fn coordinate_attack_is_only_tactical() {
        assert_only_category(&ActionStep::CoordinateAttack { target_id: 1 }, "Tactical");
    }
    #[test]
    fn set_ambush_is_only_tactical() {
        assert_only_category(
            &ActionStep::SetAmbush {
                position: IVec2::new(5, 5),
            },
            "Tactical",
        );
    }
    #[test]
    fn distract_is_only_tactical() {
        assert_only_category(&ActionStep::Distract { target_id: 1 }, "Tactical");
    }
    #[test]
    fn regroup_is_only_tactical() {
        assert_only_category(
            &ActionStep::Regroup {
                rally_point: IVec2::new(0, 0),
            },
            "Tactical",
        );
    }

    // --- Utility variants ---
    #[test]
    fn scan_is_only_utility() {
        assert_only_category(&ActionStep::Scan { radius: 10.0 }, "Utility");
    }
    #[test]
    fn wait_is_only_utility() {
        assert_only_category(&ActionStep::Wait { duration: 1.0 }, "Utility");
    }
    #[test]
    fn interact_is_only_utility() {
        assert_only_category(&ActionStep::Interact { target_id: 1 }, "Utility");
    }
    #[test]
    fn use_ability_is_only_utility() {
        assert_only_category(
            &ActionStep::UseAbility {
                ability_name: "dash".into(),
            },
            "Utility",
        );
    }
    #[test]
    fn taunt_is_only_utility() {
        assert_only_category(&ActionStep::Taunt { target_id: 1 }, "Utility");
    }

    // --- Legacy and Terrain: none of the 6 standard is_* should be true ---
    #[test]
    fn throw_legacy_is_none_of_standard_six() {
        let s = ActionStep::Throw {
            item: "rock".into(),
            x: 0,
            y: 0,
        };
        assert!(!s.is_movement());
        assert!(!s.is_offensive());
        assert!(!s.is_defensive());
        assert!(!s.is_equipment());
        assert!(!s.is_tactical());
        assert!(!s.is_utility());
    }
    #[test]
    fn revive_legacy_is_none_of_standard_six() {
        let s = ActionStep::Revive { ally_id: 1 };
        assert!(!s.is_movement());
        assert!(!s.is_offensive());
        assert!(!s.is_defensive());
        assert!(!s.is_equipment());
        assert!(!s.is_tactical());
        assert!(!s.is_utility());
    }
    #[test]
    fn modify_terrain_is_none_of_standard_six() {
        let req = TerrainGenerationRequest {
            request_id: "t1".into(),
            narrative_reason: "test".into(),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity: 0.5,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: vec![],
            seed: None,
        };
        let s = ActionStep::ModifyTerrain {
            request_id: "t1".into(),
            payload: req,
        };
        assert!(!s.is_movement());
        assert!(!s.is_offensive());
        assert!(!s.is_defensive());
        assert!(!s.is_equipment());
        assert!(!s.is_tactical());
        assert!(!s.is_utility());
    }
}

// =============================================================================
// MODULE 4: ActionStep::targets_entity() and target_entity()
// =============================================================================

mod action_step_entity_targeting_tests {
    use super::*;

    // --- targets_entity() == true variants ---
    #[test]
    fn approach_targets_entity() {
        assert!(ActionStep::Approach {
            target_id: 1,
            distance: 2.0
        }
        .targets_entity());
    }
    #[test]
    fn retreat_targets_entity() {
        assert!(ActionStep::Retreat {
            target_id: 1,
            distance: 10.0
        }
        .targets_entity());
    }
    #[test]
    fn strafe_targets_entity() {
        assert!(ActionStep::Strafe {
            target_id: 1,
            direction: StrafeDirection::Left
        }
        .targets_entity());
    }
    #[test]
    fn attack_targets_entity() {
        assert!(ActionStep::Attack { target_id: 1 }.targets_entity());
    }
    #[test]
    fn aimed_shot_targets_entity() {
        assert!(ActionStep::AimedShot { target_id: 1 }.targets_entity());
    }
    #[test]
    fn quick_attack_targets_entity() {
        assert!(ActionStep::QuickAttack { target_id: 1 }.targets_entity());
    }
    #[test]
    fn heavy_attack_targets_entity() {
        assert!(ActionStep::HeavyAttack { target_id: 1 }.targets_entity());
    }
    #[test]
    fn cover_fire_targets_entity() {
        assert!(ActionStep::CoverFire {
            target_id: 1,
            duration: 3.0
        }
        .targets_entity());
    }
    #[test]
    fn charge_targets_entity() {
        assert!(ActionStep::Charge { target_id: 1 }.targets_entity());
    }
    #[test]
    fn mark_target_targets_entity() {
        assert!(ActionStep::MarkTarget { target_id: 1 }.targets_entity());
    }
    #[test]
    fn coordinate_attack_targets_entity() {
        assert!(ActionStep::CoordinateAttack { target_id: 1 }.targets_entity());
    }
    #[test]
    fn distract_targets_entity() {
        assert!(ActionStep::Distract { target_id: 1 }.targets_entity());
    }
    #[test]
    fn interact_targets_entity() {
        assert!(ActionStep::Interact { target_id: 1 }.targets_entity());
    }
    #[test]
    fn taunt_targets_entity() {
        assert!(ActionStep::Taunt { target_id: 1 }.targets_entity());
    }
    #[test]
    fn revive_targets_entity() {
        assert!(ActionStep::Revive { ally_id: 1 }.targets_entity());
    }

    // --- targets_entity() == false variants (selection) ---
    #[test]
    fn moveto_does_not_target_entity() {
        assert!(!ActionStep::MoveTo {
            x: 0,
            y: 0,
            speed: None
        }
        .targets_entity());
    }
    #[test]
    fn block_does_not_target_entity() {
        assert!(!ActionStep::Block.targets_entity());
    }
    #[test]
    fn reload_does_not_target_entity() {
        assert!(!ActionStep::Reload.targets_entity());
    }
    #[test]
    fn wait_does_not_target_entity() {
        assert!(!ActionStep::Wait { duration: 1.0 }.targets_entity());
    }
    #[test]
    fn scan_does_not_target_entity() {
        assert!(!ActionStep::Scan { radius: 10.0 }.targets_entity());
    }
    #[test]
    fn patrol_does_not_target_entity() {
        assert!(!ActionStep::Patrol { waypoints: vec![] }.targets_entity());
    }
    #[test]
    fn take_cover_does_not_target_entity() {
        assert!(!ActionStep::TakeCover { position: None }.targets_entity());
    }
    #[test]
    fn aoe_attack_does_not_target_entity() {
        assert!(!ActionStep::AoEAttack {
            x: 0,
            y: 0,
            radius: 5.0
        }
        .targets_entity());
    }
    #[test]
    fn throw_explosive_does_not_target_entity() {
        assert!(!ActionStep::ThrowExplosive { x: 0, y: 0 }.targets_entity());
    }
    #[test]
    fn throw_smoke_does_not_target_entity() {
        assert!(!ActionStep::ThrowSmoke { x: 0, y: 0 }.targets_entity());
    }
    #[test]
    fn heal_none_does_not_target_entity() {
        // Heal with target_id: None is NOT in the targets_entity match
        assert!(!ActionStep::Heal { target_id: None }.targets_entity());
    }
    #[test]
    fn equip_weapon_does_not_target_entity() {
        assert!(!ActionStep::EquipWeapon {
            weapon_name: "sword".into()
        }
        .targets_entity());
    }
    #[test]
    fn call_reinforcements_does_not_target_entity() {
        assert!(!ActionStep::CallReinforcements { count: 3 }.targets_entity());
    }
    #[test]
    fn set_ambush_does_not_target_entity() {
        assert!(!ActionStep::SetAmbush {
            position: IVec2::new(5, 5)
        }
        .targets_entity());
    }
    #[test]
    fn regroup_does_not_target_entity() {
        assert!(!ActionStep::Regroup {
            rally_point: IVec2::new(0, 0)
        }
        .targets_entity());
    }

    // --- target_entity() returns correct ID ---
    #[test]
    fn approach_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Approach {
                target_id: 42,
                distance: 2.0
            }
            .target_entity(),
            Some(42)
        );
    }
    #[test]
    fn retreat_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Retreat {
                target_id: 7,
                distance: 10.0
            }
            .target_entity(),
            Some(7)
        );
    }
    #[test]
    fn strafe_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Strafe {
                target_id: 3,
                direction: StrafeDirection::Right
            }
            .target_entity(),
            Some(3)
        );
    }
    #[test]
    fn attack_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Attack { target_id: 99 }.target_entity(),
            Some(99)
        );
    }
    #[test]
    fn aimed_shot_target_entity_returns_id() {
        assert_eq!(
            ActionStep::AimedShot { target_id: 5 }.target_entity(),
            Some(5)
        );
    }
    #[test]
    fn quick_attack_target_entity_returns_id() {
        assert_eq!(
            ActionStep::QuickAttack { target_id: 6 }.target_entity(),
            Some(6)
        );
    }
    #[test]
    fn heavy_attack_target_entity_returns_id() {
        assert_eq!(
            ActionStep::HeavyAttack { target_id: 8 }.target_entity(),
            Some(8)
        );
    }
    #[test]
    fn cover_fire_target_entity_returns_id() {
        assert_eq!(
            ActionStep::CoverFire {
                target_id: 11,
                duration: 3.0
            }
            .target_entity(),
            Some(11)
        );
    }
    #[test]
    fn charge_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Charge { target_id: 13 }.target_entity(),
            Some(13)
        );
    }
    #[test]
    fn mark_target_target_entity_returns_id() {
        assert_eq!(
            ActionStep::MarkTarget { target_id: 14 }.target_entity(),
            Some(14)
        );
    }
    #[test]
    fn coordinate_attack_target_entity_returns_id() {
        assert_eq!(
            ActionStep::CoordinateAttack { target_id: 15 }.target_entity(),
            Some(15)
        );
    }
    #[test]
    fn distract_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Distract { target_id: 16 }.target_entity(),
            Some(16)
        );
    }
    #[test]
    fn interact_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Interact { target_id: 17 }.target_entity(),
            Some(17)
        );
    }
    #[test]
    fn taunt_target_entity_returns_id() {
        assert_eq!(
            ActionStep::Taunt { target_id: 18 }.target_entity(),
            Some(18)
        );
    }
    #[test]
    fn revive_target_entity_returns_ally_id() {
        assert_eq!(ActionStep::Revive { ally_id: 20 }.target_entity(), Some(20));
    }
    #[test]
    fn heal_with_target_returns_id() {
        assert_eq!(
            ActionStep::Heal {
                target_id: Some(30)
            }
            .target_entity(),
            Some(30)
        );
    }
    #[test]
    fn heal_without_target_returns_none() {
        assert_eq!(ActionStep::Heal { target_id: None }.target_entity(), None);
    }

    // --- target_entity() returns None for non-targeting variants ---
    #[test]
    fn moveto_target_entity_returns_none() {
        assert_eq!(
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
            .target_entity(),
            None
        );
    }
    #[test]
    fn block_target_entity_returns_none() {
        assert_eq!(ActionStep::Block.target_entity(), None);
    }
    #[test]
    fn reload_target_entity_returns_none() {
        assert_eq!(ActionStep::Reload.target_entity(), None);
    }
    #[test]
    fn wait_target_entity_returns_none() {
        assert_eq!(ActionStep::Wait { duration: 1.0 }.target_entity(), None);
    }
    #[test]
    fn scan_target_entity_returns_none() {
        assert_eq!(ActionStep::Scan { radius: 10.0 }.target_entity(), None);
    }
}

// =============================================================================
// MODULE 5: ActionStep::has_position()
// =============================================================================

mod action_step_has_position_tests {
    use super::*;

    // --- has_position() == true ---
    #[test]
    fn moveto_has_position() {
        assert!(ActionStep::MoveTo {
            x: 5,
            y: 10,
            speed: None
        }
        .has_position());
    }
    #[test]
    fn take_cover_with_position_has_position() {
        assert!(ActionStep::TakeCover {
            position: Some(IVec2::new(3, 4))
        }
        .has_position());
    }
    #[test]
    fn aoe_attack_has_position() {
        assert!(ActionStep::AoEAttack {
            x: 0,
            y: 0,
            radius: 5.0
        }
        .has_position());
    }
    #[test]
    fn throw_explosive_has_position() {
        assert!(ActionStep::ThrowExplosive { x: 0, y: 0 }.has_position());
    }
    #[test]
    fn throw_smoke_has_position() {
        assert!(ActionStep::ThrowSmoke { x: 0, y: 0 }.has_position());
    }
    #[test]
    fn set_ambush_has_position() {
        assert!(ActionStep::SetAmbush {
            position: IVec2::new(5, 5)
        }
        .has_position());
    }
    #[test]
    fn regroup_has_position() {
        assert!(ActionStep::Regroup {
            rally_point: IVec2::new(0, 0)
        }
        .has_position());
    }
    #[test]
    fn throw_legacy_has_position() {
        assert!(ActionStep::Throw {
            item: "rock".into(),
            x: 0,
            y: 0
        }
        .has_position());
    }

    // --- has_position() == false ---
    #[test]
    fn take_cover_without_position_has_no_position() {
        assert!(!ActionStep::TakeCover { position: None }.has_position());
    }
    #[test]
    fn attack_has_no_position() {
        assert!(!ActionStep::Attack { target_id: 1 }.has_position());
    }
    #[test]
    fn block_has_no_position() {
        assert!(!ActionStep::Block.has_position());
    }
    #[test]
    fn reload_has_no_position() {
        assert!(!ActionStep::Reload.has_position());
    }
    #[test]
    fn wait_has_no_position() {
        assert!(!ActionStep::Wait { duration: 1.0 }.has_position());
    }
    #[test]
    fn scan_has_no_position() {
        assert!(!ActionStep::Scan { radius: 10.0 }.has_position());
    }
    #[test]
    fn heal_has_no_position() {
        assert!(!ActionStep::Heal { target_id: None }.has_position());
    }
    #[test]
    fn parry_has_no_position() {
        assert!(!ActionStep::Parry.has_position());
    }
    #[test]
    fn equip_weapon_has_no_position() {
        assert!(!ActionStep::EquipWeapon {
            weapon_name: "sword".into()
        }
        .has_position());
    }
    #[test]
    fn call_reinforcements_has_no_position() {
        assert!(!ActionStep::CallReinforcements { count: 3 }.has_position());
    }
    #[test]
    fn mark_target_has_no_position() {
        assert!(!ActionStep::MarkTarget { target_id: 1 }.has_position());
    }
    #[test]
    fn revive_has_no_position() {
        assert!(!ActionStep::Revive { ally_id: 1 }.has_position());
    }
}

// =============================================================================
// MODULE 6: Display implementations
// =============================================================================

mod display_impl_tests {
    use super::*;

    // --- IVec2 Display ---
    #[test]
    fn ivec2_display_positive() {
        assert_eq!(format!("{}", IVec2::new(3, 7)), "(3, 7)");
    }
    #[test]
    fn ivec2_display_zero() {
        assert_eq!(format!("{}", IVec2::zero()), "(0, 0)");
    }
    #[test]
    fn ivec2_display_negative() {
        assert_eq!(format!("{}", IVec2::new(-5, -10)), "(-5, -10)");
    }
    #[test]
    fn ivec2_display_mixed() {
        assert_eq!(format!("{}", IVec2::new(-1, 2)), "(-1, 2)");
    }

    // --- WorldSnapshot Display ---
    #[test]
    fn world_snapshot_display_no_enemies_no_pois() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        let s = format!("{}", snap);
        assert!(s.contains("0 enemies"), "Should show 0 enemies: {}", s);
        assert!(s.contains("0 POIs"), "Should show 0 POIs: {}", s);
        assert!(s.contains("t=0.0s"), "Should show t=0.0s: {}", s);
    }
    #[test]
    fn world_snapshot_display_with_enemies_and_pois() {
        let snap = WorldSnapshot {
            t: 5.5,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(0, 0),
                    hp: 100,
                    cover: "".into(),
                    last_seen: 0.0,
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(1, 1),
                    hp: 50,
                    cover: "".into(),
                    last_seen: 0.0,
                },
            ],
            pois: vec![Poi {
                k: "ammo".into(),
                pos: IVec2::new(0, 0),
            }],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        let s = format!("{}", snap);
        assert!(s.contains("2 enemies"), "Should show 2 enemies: {}", s);
        assert!(s.contains("1 POIs"), "Should show 1 POIs: {}", s);
        assert!(s.contains("t=5.5s"), "Should show t=5.5s: {}", s);
    }

    // --- PlanIntent Display ---
    #[test]
    fn plan_intent_display_empty_id() {
        let plan = PlanIntent::empty();
        let s = format!("{}", plan);
        assert_eq!(s, "PlanIntent(0 steps)");
    }
    #[test]
    fn plan_intent_display_with_id_and_steps() {
        let plan = PlanIntent::new("alpha").with_step(ActionStep::Wait { duration: 1.0 });
        let s = format!("{}", plan);
        assert_eq!(s, "PlanIntent('alpha', 1 steps)");
    }
    #[test]
    fn plan_intent_display_empty_id_with_steps() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Wait { duration: 1.0 })
            .with_step(ActionStep::Block);
        let s = format!("{}", plan);
        assert_eq!(s, "PlanIntent(2 steps)");
    }
    #[test]
    fn plan_intent_display_with_id_no_steps() {
        let plan = PlanIntent::new("beta");
        let s = format!("{}", plan);
        assert_eq!(s, "PlanIntent('beta', 0 steps)");
    }

    // --- ActionStep Display ---
    #[test]
    fn action_step_display_moveto() {
        let s = format!(
            "{}",
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None
            }
        );
        assert_eq!(s, "Movement::MoveTo");
    }
    #[test]
    fn action_step_display_attack() {
        let s = format!("{}", ActionStep::Attack { target_id: 1 });
        assert_eq!(s, "Offensive::Attack");
    }
    #[test]
    fn action_step_display_block() {
        let s = format!("{}", ActionStep::Block);
        assert_eq!(s, "Defensive::Block");
    }
    #[test]
    fn action_step_display_reload() {
        let s = format!("{}", ActionStep::Reload);
        assert_eq!(s, "Equipment::Reload");
    }
    #[test]
    fn action_step_display_mark_target() {
        let s = format!("{}", ActionStep::MarkTarget { target_id: 1 });
        assert_eq!(s, "Tactical::MarkTarget");
    }
    #[test]
    fn action_step_display_wait() {
        let s = format!("{}", ActionStep::Wait { duration: 1.0 });
        assert_eq!(s, "Utility::Wait");
    }
    #[test]
    fn action_step_display_throw_legacy() {
        let s = format!(
            "{}",
            ActionStep::Throw {
                item: "rock".into(),
                x: 0,
                y: 0
            }
        );
        assert_eq!(s, "Legacy::Throw");
    }
    #[test]
    fn action_step_display_modify_terrain() {
        let req = TerrainGenerationRequest {
            request_id: "t1".into(),
            narrative_reason: "test".into(),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity: 0.5,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: vec![],
            seed: None,
        };
        let s = format!(
            "{}",
            ActionStep::ModifyTerrain {
                request_id: "t1".into(),
                payload: req
            }
        );
        assert_eq!(s, "Terrain::ModifyTerrain");
    }
}

// =============================================================================
// MODULE 7: CardinalDirection::to_unit_vector()
// =============================================================================

mod cardinal_direction_tests {
    use super::*;

    const EPSILON: f32 = 0.001;

    fn assert_vec_eq(actual: (f32, f32), expected: (f32, f32), dir: &str) {
        assert!(
            (actual.0 - expected.0).abs() < EPSILON,
            "{}: x component: expected {}, got {}",
            dir,
            expected.0,
            actual.0
        );
        assert!(
            (actual.1 - expected.1).abs() < EPSILON,
            "{}: y component: expected {}, got {}",
            dir,
            expected.1,
            actual.1
        );
    }

    #[test]
    fn north_unit_vector() {
        assert_vec_eq(
            CardinalDirection::North.to_unit_vector(),
            (0.0, -1.0),
            "North",
        );
    }
    #[test]
    fn south_unit_vector() {
        assert_vec_eq(
            CardinalDirection::South.to_unit_vector(),
            (0.0, 1.0),
            "South",
        );
    }
    #[test]
    fn east_unit_vector() {
        assert_vec_eq(CardinalDirection::East.to_unit_vector(), (1.0, 0.0), "East");
    }
    #[test]
    fn west_unit_vector() {
        assert_vec_eq(
            CardinalDirection::West.to_unit_vector(),
            (-1.0, 0.0),
            "West",
        );
    }
    #[test]
    fn north_east_unit_vector() {
        assert_vec_eq(
            CardinalDirection::NorthEast.to_unit_vector(),
            (0.707, -0.707),
            "NorthEast",
        );
    }
    #[test]
    fn north_west_unit_vector() {
        assert_vec_eq(
            CardinalDirection::NorthWest.to_unit_vector(),
            (-0.707, -0.707),
            "NorthWest",
        );
    }
    #[test]
    fn south_east_unit_vector() {
        assert_vec_eq(
            CardinalDirection::SouthEast.to_unit_vector(),
            (0.707, 0.707),
            "SouthEast",
        );
    }
    #[test]
    fn south_west_unit_vector() {
        assert_vec_eq(
            CardinalDirection::SouthWest.to_unit_vector(),
            (-0.707, 0.707),
            "SouthWest",
        );
    }

    // Cross-validate: all cardinal x-components uniquely identify axis
    #[test]
    fn north_x_is_zero() {
        assert_eq!(CardinalDirection::North.to_unit_vector().0, 0.0);
    }
    #[test]
    fn south_x_is_zero() {
        assert_eq!(CardinalDirection::South.to_unit_vector().0, 0.0);
    }
    #[test]
    fn east_y_is_zero() {
        assert_eq!(CardinalDirection::East.to_unit_vector().1, 0.0);
    }
    #[test]
    fn west_y_is_zero() {
        assert_eq!(CardinalDirection::West.to_unit_vector().1, 0.0);
    }

    // Opposite directions sum to zero
    #[test]
    fn north_south_cancel() {
        let n = CardinalDirection::North.to_unit_vector();
        let s = CardinalDirection::South.to_unit_vector();
        assert!((n.0 + s.0).abs() < EPSILON);
        assert!((n.1 + s.1).abs() < EPSILON);
    }
    #[test]
    fn east_west_cancel() {
        let e = CardinalDirection::East.to_unit_vector();
        let w = CardinalDirection::West.to_unit_vector();
        assert!((e.0 + w.0).abs() < EPSILON);
        assert!((e.1 + w.1).abs() < EPSILON);
    }
    #[test]
    fn ne_sw_cancel() {
        let ne = CardinalDirection::NorthEast.to_unit_vector();
        let sw = CardinalDirection::SouthWest.to_unit_vector();
        assert!((ne.0 + sw.0).abs() < EPSILON);
        assert!((ne.1 + sw.1).abs() < EPSILON);
    }
    #[test]
    fn nw_se_cancel() {
        let nw = CardinalDirection::NorthWest.to_unit_vector();
        let se = CardinalDirection::SouthEast.to_unit_vector();
        assert!((nw.0 + se.0).abs() < EPSILON);
        assert!((nw.1 + se.1).abs() < EPSILON);
    }

    // Diagonal magnitude is ~1.0
    #[test]
    fn diagonal_magnitudes_are_unit() {
        let diagonals = [
            CardinalDirection::NorthEast,
            CardinalDirection::NorthWest,
            CardinalDirection::SouthEast,
            CardinalDirection::SouthWest,
        ];
        for d in &diagonals {
            let (x, y) = d.to_unit_vector();
            let mag = (x * x + y * y).sqrt();
            assert!(
                (mag - 1.0).abs() < 0.01,
                "{:?} magnitude should be ~1.0, got {}",
                d,
                mag
            );
        }
    }
}

// =============================================================================
// MODULE 8: DistanceCategory::to_range() and midpoint()
// =============================================================================

mod distance_category_tests {
    use super::*;

    // --- to_range() exact values ---
    #[test]
    fn near_range_min() {
        assert_eq!(DistanceCategory::Near.to_range().0, 10.0);
    }
    #[test]
    fn near_range_max() {
        assert_eq!(DistanceCategory::Near.to_range().1, 50.0);
    }
    #[test]
    fn medium_range_min() {
        assert_eq!(DistanceCategory::Medium.to_range().0, 50.0);
    }
    #[test]
    fn medium_range_max() {
        assert_eq!(DistanceCategory::Medium.to_range().1, 150.0);
    }
    #[test]
    fn far_range_min() {
        assert_eq!(DistanceCategory::Far.to_range().0, 150.0);
    }
    #[test]
    fn far_range_max() {
        assert_eq!(DistanceCategory::Far.to_range().1, 500.0);
    }

    // --- midpoint() exact values ---
    #[test]
    fn near_midpoint() {
        assert_eq!(DistanceCategory::Near.midpoint(), 30.0);
    }
    #[test]
    fn medium_midpoint() {
        assert_eq!(DistanceCategory::Medium.midpoint(), 100.0);
    }
    #[test]
    fn far_midpoint() {
        assert_eq!(DistanceCategory::Far.midpoint(), 325.0);
    }

    // --- midpoint is average of range ---
    #[test]
    fn near_midpoint_is_average() {
        let (min, max) = DistanceCategory::Near.to_range();
        assert_eq!(DistanceCategory::Near.midpoint(), (min + max) / 2.0);
    }
    #[test]
    fn medium_midpoint_is_average() {
        let (min, max) = DistanceCategory::Medium.to_range();
        assert_eq!(DistanceCategory::Medium.midpoint(), (min + max) / 2.0);
    }
    #[test]
    fn far_midpoint_is_average() {
        let (min, max) = DistanceCategory::Far.to_range();
        assert_eq!(DistanceCategory::Far.midpoint(), (min + max) / 2.0);
    }

    // --- ranges are ordered: Near < Medium < Far ---
    #[test]
    fn near_max_equals_medium_min() {
        assert_eq!(
            DistanceCategory::Near.to_range().1,
            DistanceCategory::Medium.to_range().0
        );
    }
    #[test]
    fn medium_max_equals_far_min() {
        assert_eq!(
            DistanceCategory::Medium.to_range().1,
            DistanceCategory::Far.to_range().0
        );
    }
    #[test]
    fn midpoints_are_ordered() {
        assert!(DistanceCategory::Near.midpoint() < DistanceCategory::Medium.midpoint());
        assert!(DistanceCategory::Medium.midpoint() < DistanceCategory::Far.midpoint());
    }
}

// =============================================================================
// MODULE 9: tool_vocabulary functions
// =============================================================================

mod tool_vocabulary_tests {
    use astraweave_core::tool_vocabulary;

    #[test]
    fn get_all_tools_returns_37() {
        let tools = tool_vocabulary::get_all_tools();
        assert_eq!(tools.len(), 37, "Expected 37 tools, got {}", tools.len());
    }

    #[test]
    fn get_all_tools_names_are_nonempty() {
        let tools = tool_vocabulary::get_all_tools();
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");
        }
    }

    #[test]
    fn get_all_tools_names_are_unique() {
        let tools = tool_vocabulary::get_all_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), tools.len(), "Tool names should be unique");
    }

    #[test]
    fn get_categories_returns_expected_count() {
        let cats = tool_vocabulary::get_categories();
        // Should have at least Movement, Offensive, Defensive, Equipment, Tactical, Utility
        assert!(
            cats.len() >= 6,
            "Expected at least 6 categories, got {}",
            cats.len()
        );
    }

    #[test]
    fn get_categories_includes_movement() {
        let cats = tool_vocabulary::get_categories();
        assert!(
            cats.contains(&"Movement".to_string()) || cats.contains(&"movement".to_string()),
            "Should include Movement category"
        );
    }

    #[test]
    fn get_categories_includes_offensive() {
        let cats = tool_vocabulary::get_categories();
        assert!(
            cats.contains(&"Offensive".to_string()) || cats.contains(&"offensive".to_string()),
            "Should include Offensive category"
        );
    }

    #[test]
    fn get_categories_includes_defensive() {
        let cats = tool_vocabulary::get_categories();
        assert!(
            cats.contains(&"Defensive".to_string()) || cats.contains(&"defensive".to_string()),
            "Should include Defensive category"
        );
    }

    #[test]
    fn get_categories_are_nonempty_strings() {
        let cats = tool_vocabulary::get_categories();
        for c in &cats {
            assert!(!c.is_empty(), "Category string should not be empty");
        }
    }

    #[test]
    fn tools_by_category_movement_nonempty() {
        let tools = tool_vocabulary::get_tools_by_category("Movement");
        assert!(!tools.is_empty(), "Movement category should have tools");
    }

    #[test]
    fn tools_by_category_offensive_nonempty() {
        let tools = tool_vocabulary::get_tools_by_category("Offensive");
        assert!(!tools.is_empty(), "Offensive category should have tools");
    }

    #[test]
    fn tools_by_category_nonexistent_is_empty() {
        let tools = tool_vocabulary::get_tools_by_category("NonExistentCategory");
        assert!(
            tools.is_empty(),
            "Non-existent category should return empty vec"
        );
    }

    #[test]
    fn tools_by_category_all_match_category() {
        let cats = tool_vocabulary::get_categories();
        for cat in &cats {
            let tools = tool_vocabulary::get_tools_by_category(cat);
            for tool in &tools {
                assert_eq!(
                    &tool.category, cat,
                    "Tool '{}' in category '{}' filter should have matching category",
                    tool.name, cat
                );
            }
        }
    }

    #[test]
    fn tools_by_all_categories_sum_to_total() {
        let cats = tool_vocabulary::get_categories();
        let total: usize = cats
            .iter()
            .map(|c| tool_vocabulary::get_tools_by_category(c).len())
            .sum();
        let all = tool_vocabulary::get_all_tools();
        assert_eq!(
            total,
            all.len(),
            "Sum of per-category tools should equal total"
        );
    }

    #[test]
    fn generate_tool_schema_is_nonempty() {
        let tools = tool_vocabulary::get_all_tools();
        let schema = tool_vocabulary::generate_tool_schema(&tools[0]);
        assert!(!schema.is_empty(), "Tool schema should be non-empty");
    }

    #[test]
    fn generate_tool_schema_contains_tool_name() {
        let tools = tool_vocabulary::get_all_tools();
        for tool in &tools {
            let schema = tool_vocabulary::generate_tool_schema(tool);
            assert!(
                schema.contains(&tool.name),
                "Schema for '{}' should contain tool name",
                tool.name
            );
        }
    }

    #[test]
    fn tool_metadata_has_description() {
        let tools = tool_vocabulary::get_all_tools();
        for tool in &tools {
            assert!(
                !tool.description.is_empty(),
                "Tool '{}' should have a non-empty description",
                tool.name
            );
        }
    }

    #[test]
    fn tool_metadata_has_category() {
        let tools = tool_vocabulary::get_all_tools();
        for tool in &tools {
            assert!(
                !tool.category.is_empty(),
                "Tool '{}' should have a non-empty category",
                tool.name
            );
        }
    }
}

// =============================================================================
// MODULE 10: MetricsRegistry edge cases
// =============================================================================

mod metrics_registry_tests {
    use astraweave_core::metrics::MetricsRegistry;

    #[test]
    fn new_registry_has_no_counters() {
        let reg = MetricsRegistry::new();
        assert!(reg.get_counters().is_empty());
    }

    #[test]
    fn new_registry_has_no_gauges() {
        let reg = MetricsRegistry::new();
        assert!(reg.get_gauges().is_empty());
    }

    #[test]
    fn increment_creates_counter_if_absent() {
        let reg = MetricsRegistry::new();
        reg.increment("test_counter");
        let counters = reg.get_counters();
        assert_eq!(*counters.get("test_counter").unwrap(), 1);
    }

    #[test]
    fn increment_accumulates() {
        let reg = MetricsRegistry::new();
        reg.increment("counter");
        reg.increment("counter");
        reg.increment("counter");
        assert_eq!(*reg.get_counters().get("counter").unwrap(), 3);
    }

    #[test]
    fn add_creates_counter_if_absent() {
        let reg = MetricsRegistry::new();
        reg.add("counter", 5);
        assert_eq!(*reg.get_counters().get("counter").unwrap(), 5);
    }

    #[test]
    fn add_accumulates_with_increment() {
        let reg = MetricsRegistry::new();
        reg.increment("counter");
        reg.add("counter", 10);
        assert_eq!(*reg.get_counters().get("counter").unwrap(), 11);
    }

    #[test]
    fn gauge_sets_value() {
        let reg = MetricsRegistry::new();
        reg.gauge("temp", 72.5);
        assert_eq!(*reg.get_gauges().get("temp").unwrap(), 72.5);
    }

    #[test]
    fn gauge_overwrites_previous() {
        let reg = MetricsRegistry::new();
        reg.gauge("temp", 72.5);
        reg.gauge("temp", 80.0);
        assert_eq!(*reg.get_gauges().get("temp").unwrap(), 80.0);
    }

    #[test]
    fn histogram_records_values() {
        let reg = MetricsRegistry::new();
        reg.histogram("latency", 1.0);
        reg.histogram("latency", 2.0);
        reg.histogram("latency", 3.0);
        let stats = reg.get_histogram_stats("latency").unwrap();
        // tuple: (count, min, max, avg)
        assert_eq!(stats.0, 3);
        assert!((stats.3 - 2.0).abs() < 0.001);
        assert_eq!(stats.1, 1.0);
        assert_eq!(stats.2, 3.0);
    }

    #[test]
    fn histogram_stats_for_missing_key_is_none() {
        let reg = MetricsRegistry::new();
        assert!(reg.get_histogram_stats("nonexistent").is_none());
    }

    #[test]
    fn histogram_single_value_stats() {
        let reg = MetricsRegistry::new();
        reg.histogram("single", 42.0);
        let stats = reg.get_histogram_stats("single").unwrap();
        // tuple: (count, min, max, avg)
        assert_eq!(stats.0, 1);
        assert_eq!(stats.1, 42.0);
        assert_eq!(stats.2, 42.0);
        assert_eq!(stats.3, 42.0);
    }

    #[test]
    fn multiple_counters_independent() {
        let reg = MetricsRegistry::new();
        reg.increment("a");
        reg.increment("a");
        reg.increment("b");
        assert_eq!(*reg.get_counters().get("a").unwrap(), 2);
        assert_eq!(*reg.get_counters().get("b").unwrap(), 1);
    }

    #[test]
    fn multiple_gauges_independent() {
        let reg = MetricsRegistry::new();
        reg.gauge("x", 1.0);
        reg.gauge("y", 2.0);
        assert_eq!(*reg.get_gauges().get("x").unwrap(), 1.0);
        assert_eq!(*reg.get_gauges().get("y").unwrap(), 2.0);
    }
}

// =============================================================================
// MODULE 11: Events system
// =============================================================================

mod events_tests {
    use astraweave_core::ecs_events::Events;

    #[test]
    fn default_events_is_empty() {
        let events: Events<String> = Events::default();
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn send_event_increases_len() {
        let mut events: Events<i32> = Events::default();
        events.writer().send(42);
        assert_eq!(events.len(), 1);
        assert!(!events.is_empty());
    }

    #[test]
    fn send_multiple_events() {
        let mut events: Events<i32> = Events::default();
        events.writer().send(1);
        events.writer().send(2);
        events.writer().send(3);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn reader_drains_all_events() {
        let mut events: Events<i32> = Events::default();
        events.writer().send(10);
        events.writer().send(20);
        let drained: Vec<i32> = events.reader().drain().collect();
        assert_eq!(drained, vec![10, 20]);
    }

    #[test]
    fn clear_removes_all_events() {
        let mut events: Events<String> = Events::default();
        events.writer().send("hello".into());
        events.writer().send("world".into());
        events.clear();
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn reader_after_clear_yields_nothing() {
        let mut events: Events<i32> = Events::default();
        events.writer().send(42);
        events.clear();
        let drained: Vec<i32> = events.reader().drain().collect();
        assert!(drained.is_empty());
    }

    #[test]
    fn events_preserve_order() {
        let mut events: Events<i32> = Events::default();
        for i in 0..10 {
            events.writer().send(i);
        }
        let drained: Vec<i32> = events.reader().drain().collect();
        assert_eq!(drained, (0..10).collect::<Vec<_>>());
    }
}

// =============================================================================
// MODULE 12: EntityBridge
// =============================================================================

mod entity_bridge_tests {
    use astraweave_core::ecs_bridge::EntityBridge;
    use astraweave_ecs::Entity as EcsEntity;

    fn ecs(raw: u32) -> EcsEntity {
        unsafe { EcsEntity::from_raw(raw as u64) }
    }

    #[test]
    fn default_bridge_is_empty() {
        let bridge = EntityBridge::default();
        assert!(bridge.ecs_entities().is_empty());
    }

    #[test]
    fn insert_pair_creates_bidirectional_mapping() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        assert_eq!(bridge.get_by_legacy(&1), Some(ecs(100)));
        assert_eq!(bridge.get_by_ecs(&ecs(100)), Some(1));
    }

    #[test]
    fn insert_creates_mapping() {
        let mut bridge = EntityBridge::default();
        bridge.insert(5, ecs(50));
        assert_eq!(bridge.get(&5), Some(ecs(50)));
    }

    #[test]
    fn remove_legacy_removes_mapping() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        bridge.remove_by_legacy(&1);
        assert_eq!(bridge.get_by_legacy(&1), None);
    }

    #[test]
    fn remove_ecs_removes_mapping() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        bridge.remove_by_ecs(&ecs(100));
        assert_eq!(bridge.get_by_ecs(&ecs(100)), None);
    }

    #[test]
    fn get_legacy_returns_correct_id() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(2, ecs(200));
        assert_eq!(bridge.get_legacy(&ecs(200)), Some(2));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let bridge = EntityBridge::default();
        assert_eq!(bridge.get(&999), None);
        assert_eq!(bridge.get_by_legacy(&999), None);
        assert_eq!(bridge.get_by_ecs(&ecs(999)), None);
    }

    #[test]
    fn multiple_pairs_independent() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        bridge.insert_pair(2, ecs(200));
        bridge.insert_pair(3, ecs(300));
        assert_eq!(bridge.get_by_legacy(&1), Some(ecs(100)));
        assert_eq!(bridge.get_by_legacy(&2), Some(ecs(200)));
        assert_eq!(bridge.get_by_legacy(&3), Some(ecs(300)));
        assert_eq!(bridge.get_by_ecs(&ecs(100)), Some(1));
        assert_eq!(bridge.get_by_ecs(&ecs(200)), Some(2));
        assert_eq!(bridge.get_by_ecs(&ecs(300)), Some(3));
    }

    #[test]
    fn ecs_entities_returns_all_ecs_ids() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        bridge.insert_pair(2, ecs(200));
        let entities = bridge.ecs_entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&ecs(100)));
        assert!(entities.contains(&ecs(200)));
    }

    #[test]
    fn remove_then_reinsert_works() {
        let mut bridge = EntityBridge::default();
        bridge.insert_pair(1, ecs(100));
        bridge.remove_by_legacy(&1);
        assert_eq!(bridge.get_by_legacy(&1), None);
        bridge.insert_pair(1, ecs(200));
        assert_eq!(bridge.get_by_legacy(&1), Some(ecs(200)));
    }
}

// =============================================================================
// MODULE 13: WorldSnapshot additional query edge cases
// =============================================================================

mod world_snapshot_additional_tests {
    use super::*;

    fn make_snap(
        me_pos: IVec2,
        enemies: Vec<EnemyState>,
        pois: Vec<Poi>,
        objective: Option<String>,
    ) -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2::new(10, 10),
                ..Default::default()
            },
            me: CompanionState {
                ammo: 5,
                pos: me_pos,
                ..Default::default()
            },
            enemies,
            pois,
            obstacles: vec![],
            objective,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    #[test]
    fn distance_to_player_manhattan() {
        let snap = make_snap(IVec2::new(0, 0), vec![], vec![], None);
        // player at (10,10), me at (0,0) => euclidean = sqrt(200) ≈ 14.14
        let dist = snap.distance_to_player();
        assert!(
            (dist - 14.142136).abs() < 0.01,
            "Expected ~14.14, got {}",
            dist
        );
    }

    #[test]
    fn distance_to_player_same_position() {
        let snap = make_snap(IVec2::new(10, 10), vec![], vec![], None);
        assert!((snap.distance_to_player() - 0.0).abs() < 0.001);
    }

    #[test]
    fn nearest_enemy_with_no_enemies_returns_none() {
        let snap = make_snap(IVec2::new(0, 0), vec![], vec![], None);
        assert!(snap.nearest_enemy().is_none());
    }

    #[test]
    fn nearest_enemy_with_single_enemy() {
        let snap = make_snap(
            IVec2::new(0, 0),
            vec![EnemyState {
                id: 1,
                pos: IVec2::new(5, 5),
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            vec![],
            None,
        );
        let nearest = snap.nearest_enemy().unwrap();
        assert_eq!(nearest.id, 1);
    }

    #[test]
    fn enemy_count_correct() {
        let snap = make_snap(
            IVec2::new(0, 0),
            vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(0, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(1, 1),
                    ..Default::default()
                },
                EnemyState {
                    id: 3,
                    pos: IVec2::new(2, 2),
                    ..Default::default()
                },
            ],
            vec![],
            None,
        );
        assert_eq!(snap.enemy_count(), 3);
    }

    #[test]
    fn enemies_within_range_zero_returns_only_colocated() {
        let snap = make_snap(
            IVec2::new(0, 0),
            vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(0, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(1, 0),
                    ..Default::default()
                },
            ],
            vec![],
            None,
        );
        let within = snap.enemies_within_range(0);
        assert_eq!(within.len(), 1);
        assert_eq!(within[0].id, 1);
    }
}

// =============================================================================
// MODULE 14: PlanIntent additional helpers
// =============================================================================

mod plan_intent_additional_tests {
    use super::*;

    #[test]
    fn with_step_chains_correctly() {
        let plan = PlanIntent::new("chain")
            .with_step(ActionStep::Wait { duration: 1.0 })
            .with_step(ActionStep::Block)
            .with_step(ActionStep::Reload);
        assert_eq!(plan.step_count(), 3);
    }

    #[test]
    fn first_step_returns_first() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Wait { duration: 1.0 })
            .with_step(ActionStep::Block);
        let first = plan.first_step().unwrap();
        assert_eq!(first.action_name(), "Wait");
    }

    #[test]
    fn first_step_empty_plan_returns_none() {
        let plan = PlanIntent::empty();
        assert!(plan.first_step().is_none());
    }

    #[test]
    fn has_movement_with_all_movement_variants() {
        let variants = vec![
            ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None,
            },
            ActionStep::Approach {
                target_id: 1,
                distance: 2.0,
            },
            ActionStep::Retreat {
                target_id: 1,
                distance: 10.0,
            },
            ActionStep::TakeCover { position: None },
            ActionStep::Strafe {
                target_id: 1,
                direction: StrafeDirection::Left,
            },
            ActionStep::Patrol { waypoints: vec![] },
        ];
        for v in variants {
            let plan = PlanIntent::empty().with_step(v.clone());
            assert!(
                plan.has_movement(),
                "{} should make has_movement true",
                v.action_name()
            );
        }
    }

    #[test]
    fn has_offensive_with_all_offensive_variants() {
        let variants = vec![
            ActionStep::Attack { target_id: 1 },
            ActionStep::AimedShot { target_id: 1 },
            ActionStep::QuickAttack { target_id: 1 },
            ActionStep::HeavyAttack { target_id: 1 },
            ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0,
            },
            ActionStep::ThrowExplosive { x: 0, y: 0 },
            ActionStep::CoverFire {
                target_id: 1,
                duration: 3.0,
            },
            ActionStep::Charge { target_id: 1 },
        ];
        for v in variants {
            let plan = PlanIntent::empty().with_step(v.clone());
            assert!(
                plan.has_offensive(),
                "{} should make has_offensive true",
                v.action_name()
            );
        }
    }

    #[test]
    fn plan_with_only_defensive_has_no_movement() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Block)
            .with_step(ActionStep::Heal { target_id: None });
        assert!(!plan.has_movement());
    }

    #[test]
    fn plan_with_only_defensive_has_no_offensive() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Block)
            .with_step(ActionStep::Heal { target_id: None });
        assert!(!plan.has_offensive());
    }
}

// =============================================================================
// MODULE 15: IVec2 arithmetic precision
// =============================================================================

mod ivec2_arithmetic_precision_tests {
    use super::*;

    #[test]
    fn distance_squared_exact() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(3, 4);
        assert_eq!(a.distance_squared(&b), 25); // 9 + 16
    }

    #[test]
    fn distance_f32_3_4_5_triangle() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(3, 4);
        assert!((a.distance(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn manhattan_distance_exact() {
        let a = IVec2::new(1, 2);
        let b = IVec2::new(4, 6);
        assert_eq!(a.manhattan_distance(&b), 7); // |3| + |4|
    }

    #[test]
    fn offset_exact() {
        let v = IVec2::new(5, 10);
        let result = v.offset(3, -2);
        assert_eq!(result.x, 8);
        assert_eq!(result.y, 8);
    }

    #[test]
    fn add_exact() {
        let a = IVec2::new(10, 20);
        let b = IVec2::new(-3, 7);
        let c = a + b;
        assert_eq!(c.x, 7);
        assert_eq!(c.y, 27);
    }

    #[test]
    fn sub_exact() {
        let a = IVec2::new(10, 20);
        let b = IVec2::new(3, 5);
        let c = a - b;
        assert_eq!(c.x, 7);
        assert_eq!(c.y, 15);
    }

    #[test]
    fn distance_squared_is_nonnegative() {
        let a = IVec2::new(-100, -200);
        let b = IVec2::new(100, 200);
        assert!(a.distance_squared(&b) >= 0);
    }

    #[test]
    fn manhattan_with_negatives() {
        let a = IVec2::new(-5, -10);
        let b = IVec2::new(5, 10);
        assert_eq!(a.manhattan_distance(&b), 30); // |10| + |20|
    }
}

// =============================================================================
// MODULE 16: TerrainGenerationRequest validate edge cases
// =============================================================================

mod terrain_validation_edge_cases {
    use super::*;

    fn make_req(
        intensity: f32,
        narrative_len: usize,
        request_id: &str,
    ) -> TerrainGenerationRequest {
        TerrainGenerationRequest {
            request_id: request_id.to_string(),
            narrative_reason: "x".repeat(narrative_len),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: vec![],
            seed: None,
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(make_req(0.5, 50, "req1").validate().is_ok());
    }

    #[test]
    fn intensity_exactly_0_passes() {
        assert!(make_req(0.0, 50, "req1").validate().is_ok());
    }

    #[test]
    fn intensity_exactly_1_passes() {
        assert!(make_req(1.0, 50, "req1").validate().is_ok());
    }

    #[test]
    fn intensity_negative_fails() {
        assert!(make_req(-0.1, 50, "req1").validate().is_err());
    }

    #[test]
    fn intensity_above_1_fails() {
        assert!(make_req(1.1, 50, "req1").validate().is_err());
    }

    #[test]
    fn narrative_100_chars_passes() {
        assert!(make_req(0.5, 100, "req1").validate().is_ok());
    }

    #[test]
    fn narrative_101_chars_fails() {
        assert!(make_req(0.5, 101, "req1").validate().is_err());
    }

    #[test]
    fn empty_request_id_fails() {
        assert!(make_req(0.5, 50, "").validate().is_err());
    }

    #[test]
    fn single_char_request_id_passes() {
        assert!(make_req(0.5, 50, "a").validate().is_ok());
    }

    #[test]
    fn all_three_invalid_at_once() {
        // Multiple invalid conditions - should still fail
        assert!(make_req(-1.0, 200, "").validate().is_err());
    }
}

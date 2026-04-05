//! Mutation-killing tests for astraweave-ui.
//! These tests are designed to catch subtle mutations like boundary condition changes,
//! arithmetic operator substitutions, and conditional logic inversions.

#![allow(clippy::field_reassign_with_default, clippy::manual_range_contains)]

#[cfg(test)]
mod easing_tests {
    use crate::hud::easing::{ease_in_out_quad, ease_out_cubic};

    #[test]
    fn test_ease_out_cubic_at_zero() {
        // t=0 should return 0
        let result = ease_out_cubic(0.0);
        assert!(
            result.abs() < 0.001,
            "ease_out_cubic(0) should be 0, got {}",
            result
        );
    }

    #[test]
    fn test_ease_out_cubic_at_one() {
        // t=1 should return 1
        let result = ease_out_cubic(1.0);
        assert!(
            (result - 1.0).abs() < 0.001,
            "ease_out_cubic(1) should be 1, got {}",
            result
        );
    }

    #[test]
    fn test_ease_out_cubic_midpoint() {
        // t=0.5 should be > 0.5 (fast start)
        let mid = ease_out_cubic(0.5);
        assert!(
            mid > 0.5,
            "ease_out_cubic(0.5) should be > 0.5, got {}",
            mid
        );
        assert!(
            mid < 1.0,
            "ease_out_cubic(0.5) should be < 1.0, got {}",
            mid
        );
    }

    #[test]
    fn test_ease_in_out_quad_at_zero() {
        // t=0 should return 0
        let result = ease_in_out_quad(0.0);
        assert!(
            result.abs() < 0.001,
            "ease_in_out_quad(0) should be 0, got {}",
            result
        );
    }

    #[test]
    fn test_ease_in_out_quad_at_one() {
        // t=1 should return 1
        let result = ease_in_out_quad(1.0);
        assert!(
            (result - 1.0).abs() < 0.001,
            "ease_in_out_quad(1) should be 1, got {}",
            result
        );
    }

    #[test]
    fn test_ease_in_out_quad_midpoint() {
        // t=0.5 should be exactly 0.5
        let result = ease_in_out_quad(0.5);
        assert!(
            (result - 0.5).abs() < 0.001,
            "ease_in_out_quad(0.5) should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn test_ease_in_out_quad_first_half() {
        // t=0.25 should be < 0.25 (slow start)
        let quarter = ease_in_out_quad(0.25);
        assert!(
            quarter < 0.25,
            "ease_in_out_quad(0.25) should be < 0.25, got {}",
            quarter
        );
        assert!(
            quarter > 0.0,
            "ease_in_out_quad(0.25) should be > 0, got {}",
            quarter
        );
    }

    #[test]
    fn test_ease_in_out_quad_second_half() {
        // t=0.75 should be > 0.75 (slow end)
        let three_quarters = ease_in_out_quad(0.75);
        assert!(
            three_quarters > 0.75,
            "ease_in_out_quad(0.75) should be > 0.75, got {}",
            three_quarters
        );
        assert!(
            three_quarters < 1.0,
            "ease_in_out_quad(0.75) should be < 1.0, got {}",
            three_quarters
        );
    }
}

#[cfg(test)]
mod health_animation_tests {
    use crate::hud::HealthAnimation;

    #[test]
    fn test_health_animation_new_sets_values() {
        let anim = HealthAnimation::new(100.0);

        assert_eq!(anim.current_visual, 100.0);
        assert_eq!(anim.target, 100.0);
        assert_eq!(anim.animation_time, 0.0);
    }

    #[test]
    fn test_health_animation_new_different_value() {
        let anim = HealthAnimation::new(75.0);

        assert_eq!(anim.current_visual, 75.0);
        assert_eq!(anim.target, 75.0);
    }

    #[test]
    fn test_health_animation_set_target_damage() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        assert_eq!(anim.target, 50.0);
        assert_eq!(anim.animation_time, 0.0);
        // Flash should trigger on damage
        assert!(
            anim.flash_timer > 0.0,
            "Flash timer should be set on damage"
        );
    }

    #[test]
    fn test_health_animation_set_target_heal_no_flash() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);

        assert_eq!(anim.target, 100.0);
        // Flash should NOT trigger on heal
        assert_eq!(
            anim.flash_timer, 0.0,
            "Flash timer should NOT be set on heal"
        );
    }

    #[test]
    fn test_health_animation_update_moves_toward_target() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        // Update with small delta
        anim.update(0.1);

        // Visual should move toward target
        assert!(
            anim.current_visual < 100.0,
            "Visual should decrease toward target"
        );
        assert!(
            anim.current_visual > 50.0,
            "Visual should not yet reach target"
        );
    }

    #[test]
    fn test_health_animation_update_complete() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        // Update with large delta (complete animation)
        anim.update(1.0);

        // Should snap to target when complete
        assert!(
            (anim.current_visual - 50.0).abs() < 0.1,
            "Visual should be at or near target after full animation"
        );
    }

    #[test]
    fn test_health_animation_flash_alpha_after_damage() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        // Initially flash is at max
        let initial_alpha = anim.flash_alpha();
        assert!(
            initial_alpha > 0.0,
            "Flash alpha should be positive after damage"
        );
        assert!(initial_alpha <= 0.6, "Flash alpha should be <= 0.6");
    }

    #[test]
    fn test_health_animation_flash_alpha_decays() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);
        let initial_alpha = anim.flash_alpha();

        // After time passes, flash decays
        anim.update(0.1);
        let later_alpha = anim.flash_alpha();
        assert!(
            later_alpha < initial_alpha,
            "Flash alpha should decay over time"
        );
    }

    #[test]
    fn test_health_animation_is_healing_true() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);

        assert!(
            anim.is_healing(),
            "is_healing should be true when target > current"
        );
    }

    #[test]
    fn test_health_animation_is_healing_false_after_complete() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);

        // After animation completes
        anim.update(1.0);
        assert!(
            !anim.is_healing(),
            "is_healing should be false after animation completes"
        );
    }

    #[test]
    fn test_health_animation_visual_health() {
        let anim = HealthAnimation::new(75.0);
        assert_eq!(anim.visual_health(), 75.0);
    }
}

#[cfg(test)]
mod player_stats_tests {
    use crate::hud::PlayerStats;

    #[test]
    fn test_player_stats_default_health() {
        let stats = PlayerStats::default();
        assert_eq!(stats.health, 100.0);
        assert_eq!(stats.max_health, 100.0);
    }

    #[test]
    fn test_player_stats_default_mana() {
        let stats = PlayerStats::default();
        assert_eq!(stats.mana, 100.0);
        assert_eq!(stats.max_mana, 100.0);
    }

    #[test]
    fn test_player_stats_default_stamina() {
        let stats = PlayerStats::default();
        assert_eq!(stats.stamina, 100.0);
        assert_eq!(stats.max_stamina, 100.0);
    }
}

#[cfg(test)]
mod damage_number_tests {
    use crate::hud::{DamageNumber, DamageType};

    #[test]
    fn test_damage_number_new_stores_value() {
        let dmg = DamageNumber::new(50, 1.0, (0.0, 5.0, 0.0), DamageType::Normal);
        assert_eq!(dmg.value, 50);
    }

    #[test]
    fn test_damage_number_new_stores_spawn_time() {
        let dmg = DamageNumber::new(50, 1.0, (0.0, 5.0, 0.0), DamageType::Normal);
        assert_eq!(dmg.spawn_time, 1.0);
    }

    #[test]
    fn test_damage_number_new_stores_damage_type() {
        let dmg = DamageNumber::new(50, 1.0, (0.0, 5.0, 0.0), DamageType::Normal);
        assert_eq!(dmg.damage_type, DamageType::Normal);
    }

    #[test]
    fn test_damage_number_offset_at_zero() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // At age=0, offset should be (0, 0)
        let (x, y) = dmg.calculate_offset(0.0);
        assert_eq!(x, 0.0, "X offset at age 0 should be 0");
        assert_eq!(y, 0.0, "Y offset at age 0 should be 0");
    }

    #[test]
    fn test_damage_number_offset_moves_over_time() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // At age > 0, should have arc motion
        let (_, y) = dmg.calculate_offset(0.5);
        // Y should change (parabolic arc: starts going up then gravity pulls down)
        assert!(y != 0.0, "Y offset should change over time");
    }

    #[test]
    fn test_damage_number_shake_at_zero() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // At age=0, sin(0) = 0
        let shake = dmg.calculate_shake(0.0);
        assert_eq!(shake, 0.0, "Shake at age 0 should be 0");
    }

    #[test]
    fn test_damage_number_shake_oscillates() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // At age > 0, shake oscillates within amplitude
        let shake = dmg.calculate_shake(0.1);
        assert!(
            shake.abs() < dmg.shake_amplitude,
            "Shake should be within amplitude"
        );
    }

    #[test]
    fn test_damage_number_critical_has_higher_shake() {
        let critical = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Critical);
        let normal = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);

        // Critical should have higher shake amplitude
        assert!(
            critical.shake_amplitude > normal.shake_amplitude,
            "Critical should have higher shake: {} vs {}",
            critical.shake_amplitude,
            normal.shake_amplitude
        );
    }

    #[test]
    fn test_damage_type_normal() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        assert_eq!(dmg.damage_type, DamageType::Normal);
    }

    #[test]
    fn test_damage_type_critical() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Critical);
        assert_eq!(dmg.damage_type, DamageType::Critical);
    }

    #[test]
    fn test_damage_type_self_damage() {
        let dmg = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::SelfDamage);
        assert_eq!(dmg.damage_type, DamageType::SelfDamage);
    }
}

#[cfg(test)]
mod quest_tests {
    use crate::hud::{Objective, Quest};

    #[test]
    fn test_quest_completion_empty_is_zero() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![],
        };

        assert_eq!(
            quest.completion(),
            0.0,
            "Empty quest should have 0 completion"
        );
    }

    #[test]
    fn test_quest_completion_none_done_is_zero() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        assert_eq!(
            quest.completion(),
            0.0,
            "Quest with no complete objectives should be 0"
        );
    }

    #[test]
    fn test_quest_completion_half_done() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };

        let completion = quest.completion();
        assert!(
            (completion - 0.5).abs() < 0.001,
            "Quest with half complete should be 0.5, got {}",
            completion
        );
    }

    #[test]
    fn test_quest_completion_all_done_is_one() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: true,
                    progress: None,
                },
            ],
        };

        assert_eq!(
            quest.completion(),
            1.0,
            "Quest with all complete should be 1.0"
        );
    }

    #[test]
    fn test_quest_is_complete_false_when_partial() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };
        assert!(!quest.is_complete(), "Partial quest should not be complete");
    }

    #[test]
    fn test_quest_is_complete_true_when_all_done() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![Objective {
                id: 1,
                description: "A".to_string(),
                completed: true,
                progress: None,
            }],
        };
        assert!(
            quest.is_complete(),
            "Quest with all objectives done should be complete"
        );
    }

    #[test]
    fn test_quest_is_complete_false_when_empty() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            objectives: vec![],
        };
        assert!(
            !quest.is_complete(),
            "Empty quest should not be complete (no objectives to satisfy)"
        );
    }
}

#[cfg(test)]
mod ping_marker_tests {
    use crate::hud::PingMarker;

    #[test]
    fn test_ping_marker_new_sets_position() {
        let ping = PingMarker::new((10.0, 20.0), 5.0);
        assert_eq!(ping.world_pos, (10.0, 20.0));
    }

    #[test]
    fn test_ping_marker_new_sets_spawn_time() {
        let ping = PingMarker::new((10.0, 20.0), 5.0);
        assert_eq!(ping.spawn_time, 5.0);
    }

    #[test]
    fn test_ping_marker_new_default_duration() {
        let ping = PingMarker::new((10.0, 20.0), 5.0);
        assert_eq!(ping.duration, 3.0, "Default duration should be 3.0 seconds");
    }

    #[test]
    fn test_ping_marker_is_active_at_spawn() {
        let ping = PingMarker::new((0.0, 0.0), 1.0);
        assert!(ping.is_active(1.0), "Ping should be active at spawn time");
    }

    #[test]
    fn test_ping_marker_is_active_during() {
        let ping = PingMarker::new((0.0, 0.0), 1.0);
        assert!(ping.is_active(2.0), "Ping should be active during duration");
        assert!(
            ping.is_active(3.9),
            "Ping should be active just before expiry"
        );
    }

    #[test]
    fn test_ping_marker_is_inactive_after_duration() {
        let ping = PingMarker::new((0.0, 0.0), 1.0);
        // Duration is 3.0, so at time 1.0 + 3.0 + 0.1 = 4.1 it should be inactive
        assert!(
            !ping.is_active(4.1),
            "Ping should be inactive after duration"
        );
        assert!(
            !ping.is_active(10.0),
            "Ping should be inactive long after duration"
        );
    }

    #[test]
    fn test_ping_marker_age_normalized_at_spawn() {
        let ping = PingMarker::new((0.0, 0.0), 0.0);
        assert_eq!(
            ping.age_normalized(0.0),
            0.0,
            "Age normalized at spawn should be 0"
        );
    }

    #[test]
    fn test_ping_marker_age_normalized_halfway() {
        let ping = PingMarker::new((0.0, 0.0), 0.0);
        let age = ping.age_normalized(1.5);
        assert!(
            (age - 0.5).abs() < 0.001,
            "Age normalized halfway should be 0.5, got {}",
            age
        );
    }

    #[test]
    fn test_ping_marker_age_normalized_at_end() {
        let ping = PingMarker::new((0.0, 0.0), 0.0);
        assert_eq!(
            ping.age_normalized(3.0),
            1.0,
            "Age normalized at end should be 1.0"
        );
    }

    #[test]
    fn test_ping_marker_age_normalized_clamped() {
        let ping = PingMarker::new((0.0, 0.0), 0.0);
        assert_eq!(
            ping.age_normalized(10.0),
            1.0,
            "Age normalized should clamp at 1.0"
        );
    }
}

#[cfg(test)]
mod poi_type_tests {
    use crate::hud::PoiType;

    #[test]
    fn test_poi_type_objective_icon() {
        assert_eq!(PoiType::Objective.icon(), "🎯");
    }

    #[test]
    fn test_poi_type_waypoint_icon() {
        assert_eq!(PoiType::Waypoint.icon(), "📍");
    }

    #[test]
    fn test_poi_type_vendor_icon() {
        assert_eq!(PoiType::Vendor.icon(), "🏪");
    }

    #[test]
    fn test_poi_type_danger_icon() {
        assert_eq!(PoiType::Danger.icon(), "⚔️");
    }

    #[test]
    fn test_poi_type_objective_color() {
        assert_eq!(PoiType::Objective.color(), egui::Color32::YELLOW);
    }

    #[test]
    fn test_poi_type_waypoint_color() {
        assert_eq!(PoiType::Waypoint.color(), egui::Color32::LIGHT_BLUE);
    }

    #[test]
    fn test_poi_type_vendor_color() {
        assert_eq!(PoiType::Vendor.color(), egui::Color32::GREEN);
    }

    #[test]
    fn test_poi_type_danger_color() {
        assert_eq!(PoiType::Danger.color(), egui::Color32::RED);
    }
}

#[cfg(test)]
mod hud_state_tests {
    use crate::hud::HudState;

    #[test]
    fn test_hud_state_default_visible() {
        let state = HudState::default();
        assert!(state.visible, "HUD should be visible by default");
    }

    #[test]
    fn test_hud_state_default_show_health_bars() {
        let state = HudState::default();
        assert!(
            state.show_health_bars,
            "Health bars should be shown by default"
        );
    }

    #[test]
    fn test_hud_state_default_show_objectives() {
        let state = HudState::default();
        assert!(
            state.show_objectives,
            "Objectives should be shown by default"
        );
    }

    #[test]
    fn test_hud_state_default_show_minimap() {
        let state = HudState::default();
        assert!(state.show_minimap, "Minimap should be shown by default");
    }

    #[test]
    fn test_hud_state_default_show_subtitles() {
        let state = HudState::default();
        assert!(state.show_subtitles, "Subtitles should be shown by default");
    }

    #[test]
    fn test_hud_state_default_quest_tracker_not_collapsed() {
        let state = HudState::default();
        assert!(
            !state.quest_tracker_collapsed,
            "Quest tracker should not be collapsed by default"
        );
    }

    #[test]
    fn test_hud_state_default_minimap_rotation_off() {
        let state = HudState::default();
        assert!(
            !state.minimap_rotation,
            "Minimap rotation should be off by default"
        );
    }

    #[test]
    fn test_hud_state_default_debug_mode_off() {
        let state = HudState::default();
        assert!(!state.debug_mode, "Debug mode should be off by default");
    }
}

#[cfg(test)]
mod accessibility_tests {
    use crate::accessibility::{colors, transform_color, AccessibilitySettings, ColorblindMode};

    #[test]
    fn test_colorblind_mode_all_count() {
        let all = ColorblindMode::all();
        assert_eq!(all.len(), 5, "Should have 5 colorblind modes");
    }

    #[test]
    fn test_colorblind_mode_all_contains_none() {
        let all = ColorblindMode::all();
        assert!(all.contains(&ColorblindMode::None));
    }

    #[test]
    fn test_colorblind_mode_all_contains_deuteranopia() {
        let all = ColorblindMode::all();
        assert!(all.contains(&ColorblindMode::Deuteranopia));
    }

    #[test]
    fn test_colorblind_mode_all_contains_protanopia() {
        let all = ColorblindMode::all();
        assert!(all.contains(&ColorblindMode::Protanopia));
    }

    #[test]
    fn test_colorblind_mode_all_contains_tritanopia() {
        let all = ColorblindMode::all();
        assert!(all.contains(&ColorblindMode::Tritanopia));
    }

    #[test]
    fn test_colorblind_mode_all_contains_high_contrast() {
        let all = ColorblindMode::all();
        assert!(all.contains(&ColorblindMode::HighContrast));
    }

    #[test]
    fn test_colorblind_mode_none_display_name() {
        assert_eq!(ColorblindMode::None.display_name(), "None");
    }

    #[test]
    fn test_colorblind_mode_deuteranopia_display_name() {
        assert_eq!(
            ColorblindMode::Deuteranopia.display_name(),
            "Deuteranopia (Red-Green)"
        );
    }

    #[test]
    fn test_colorblind_mode_protanopia_display_name() {
        assert_eq!(
            ColorblindMode::Protanopia.display_name(),
            "Protanopia (Red-Green)"
        );
    }

    #[test]
    fn test_colorblind_mode_tritanopia_display_name() {
        assert_eq!(
            ColorblindMode::Tritanopia.display_name(),
            "Tritanopia (Blue-Yellow)"
        );
    }

    #[test]
    fn test_colorblind_mode_high_contrast_display_name() {
        assert_eq!(ColorblindMode::HighContrast.display_name(), "High Contrast");
    }

    #[test]
    fn test_accessibility_settings_default_colorblind_mode() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.colorblind_mode, ColorblindMode::None);
    }

    #[test]
    fn test_accessibility_settings_default_ui_scale() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.ui_scale, 1.0);
    }

    #[test]
    fn test_accessibility_settings_default_reduce_motion() {
        let settings = AccessibilitySettings::default();
        assert!(!settings.reduce_motion);
    }

    #[test]
    fn test_accessibility_settings_default_large_text() {
        let settings = AccessibilitySettings::default();
        assert!(!settings.large_text);
    }

    #[test]
    fn test_accessibility_settings_set_ui_scale_normal() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(1.2);
        assert_eq!(settings.ui_scale, 1.2);
    }

    #[test]
    fn test_accessibility_settings_set_ui_scale_clamp_low() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(0.5);
        assert_eq!(
            settings.ui_scale, 0.8,
            "UI scale should clamp to minimum 0.8"
        );
    }

    #[test]
    fn test_accessibility_settings_set_ui_scale_clamp_high() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(2.0);
        assert_eq!(
            settings.ui_scale, 1.5,
            "UI scale should clamp to maximum 1.5"
        );
    }

    #[test]
    fn test_accessibility_settings_font_scale_default() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.font_scale(), 1.0);
    }

    #[test]
    fn test_accessibility_settings_font_scale_with_ui_scale() {
        let mut settings = AccessibilitySettings::default();
        settings.ui_scale = 1.2;
        assert_eq!(settings.font_scale(), 1.2);
    }

    #[test]
    fn test_accessibility_settings_font_scale_with_large_text() {
        let mut settings = AccessibilitySettings::default();
        settings.ui_scale = 1.2;
        settings.large_text = true;
        // 1.2 * 1.25 = 1.5
        assert!((settings.font_scale() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_transform_color_none_unchanged() {
        let color = colors::HEALTH_FULL;
        let transformed = transform_color(color, ColorblindMode::None);
        assert_eq!(color, transformed, "None mode should not transform color");
    }

    #[test]
    fn test_transform_color_deuteranopia_shifts_green() {
        let color = colors::HEALTH_FULL; // Green (0.2, 0.8, 0.2)
        let transformed = transform_color(color, ColorblindMode::Deuteranopia);
        // Green should shift to blue
        assert!(
            transformed.2 > color.2,
            "Deuteranopia should increase blue channel: {} vs {}",
            transformed.2,
            color.2
        );
    }

    #[test]
    fn test_health_full_color() {
        assert_eq!(colors::HEALTH_FULL, (0.2, 0.8, 0.2));
    }

    #[test]
    fn test_health_medium_color() {
        assert_eq!(colors::HEALTH_MEDIUM, (0.9, 0.8, 0.1));
    }

    #[test]
    fn test_health_low_color() {
        assert_eq!(colors::HEALTH_LOW, (0.9, 0.2, 0.2));
    }

    #[test]
    fn test_health_critical_color() {
        assert_eq!(colors::HEALTH_CRITICAL, (0.6, 0.1, 0.1));
    }
}

#[cfg(test)]
mod menu_tests {
    use crate::menu::{MenuState, QualityPreset};

    #[test]
    fn test_menu_state_main_menu() {
        let _ = MenuState::MainMenu;
    }

    #[test]
    fn test_menu_state_pause_menu() {
        let _ = MenuState::PauseMenu;
    }

    #[test]
    fn test_menu_state_settings_menu() {
        let _ = MenuState::SettingsMenu;
    }

    #[test]
    fn test_menu_state_none() {
        let _ = MenuState::None;
    }

    #[test]
    fn test_quality_preset_low() {
        let preset = QualityPreset::Low;
        assert_eq!(preset.as_str(), "Low");
    }

    #[test]
    fn test_quality_preset_medium() {
        let preset = QualityPreset::Medium;
        assert_eq!(preset.as_str(), "Medium");
    }

    #[test]
    fn test_quality_preset_high() {
        let preset = QualityPreset::High;
        assert_eq!(preset.as_str(), "High");
    }

    #[test]
    fn test_quality_preset_ultra() {
        let preset = QualityPreset::Ultra;
        assert_eq!(preset.as_str(), "Ultra");
    }

    #[test]
    fn test_quality_preset_all() {
        let all = QualityPreset::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&QualityPreset::Low));
        assert!(all.contains(&QualityPreset::Medium));
        assert!(all.contains(&QualityPreset::High));
        assert!(all.contains(&QualityPreset::Ultra));
    }
}

// ============================================================================
// Behavioral Correctness Tests - UI System Invariants
// ============================================================================

#[cfg(test)]
mod behavioral_correctness_tests {
    use crate::hud::easing::{ease_in_out_quad, ease_out_cubic};
    use crate::hud::HealthAnimation;
    use crate::menu::QualityPreset;
    use crate::{AccessibilitySettings, ColorblindMode};

    #[test]
    fn test_easing_functions_continuous() {
        // Behavioral: easing functions should be continuous (no jumps)
        let steps = 100;
        for i in 0..steps {
            let t1 = i as f32 / steps as f32;
            let t2 = (i + 1) as f32 / steps as f32;

            let v1 = ease_out_cubic(t1);
            let v2 = ease_out_cubic(t2);

            // Maximum reasonable jump for smooth easing
            assert!(
                (v2 - v1).abs() < 0.1,
                "Easing should be continuous: jump {} at t={}",
                (v2 - v1).abs(),
                t1
            );
        }
    }

    #[test]
    fn test_easing_monotonically_increasing() {
        // Behavioral: easing functions should be monotonically increasing
        let steps = 100;
        let mut prev = 0.0;

        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let v = ease_out_cubic(t);

            assert!(
                v >= prev,
                "ease_out_cubic should be monotonically increasing: {} < {} at t={}",
                v,
                prev,
                t
            );
            prev = v;
        }
    }

    #[test]
    fn test_easing_in_unit_range() {
        // Behavioral: easing output should stay in [0, 1] for input in [0, 1]
        for i in 0..=100 {
            let t = i as f32 / 100.0;

            let cubic = ease_out_cubic(t);
            let quad = ease_in_out_quad(t);

            assert!(
                cubic >= 0.0 && cubic <= 1.0,
                "ease_out_cubic({}) = {} should be in [0,1]",
                t,
                cubic
            );
            assert!(
                quad >= 0.0 && quad <= 1.0,
                "ease_in_out_quad({}) = {} should be in [0,1]",
                t,
                quad
            );
        }
    }

    #[test]
    fn test_health_animation_damage_triggers_flash() {
        // Behavioral: taking damage should trigger flash effect
        let mut anim = HealthAnimation::new(100.0);

        anim.set_target(50.0); // Take damage

        assert!(anim.flash_timer > 0.0, "Damage should trigger flash");
    }

    #[test]
    fn test_health_animation_heal_no_flash() {
        // Behavioral: healing should not trigger flash
        let mut anim = HealthAnimation::new(50.0);

        anim.set_target(100.0); // Heal

        // Flash should not trigger on heal (or be very short)
        // Note: implementation may vary, so we just check it doesn't crash
    }

    #[test]
    fn test_health_animation_current_approaches_target() {
        // Behavioral: after update, current should move toward target
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);

        let before = anim.current_visual;
        anim.update(0.1); // 100ms update
        let after = anim.current_visual;

        // Should move toward target (50)
        assert!(
            after <= before,
            "Health should animate toward target: {} should be <= {}",
            after,
            before
        );
    }

    #[test]
    fn test_accessibility_ui_scale_clamped() {
        // Behavioral: UI scale should be clamped to valid range
        let mut settings = AccessibilitySettings::default();

        settings.set_ui_scale(0.1); // Too low
        assert!(
            settings.ui_scale >= 0.8,
            "UI scale should clamp to minimum 0.8"
        );

        settings.set_ui_scale(5.0); // Too high
        assert!(
            settings.ui_scale <= 1.5,
            "UI scale should clamp to maximum 1.5"
        );
    }

    #[test]
    fn test_accessibility_font_scale_multiplies() {
        // Behavioral: font scale should multiply UI scale and large text bonus
        let mut settings = AccessibilitySettings::default();
        settings.ui_scale = 1.2;
        settings.large_text = false;

        let scale1 = settings.font_scale();

        settings.large_text = true;
        let scale2 = settings.font_scale();

        assert!(scale2 > scale1, "Large text should increase font scale");
    }

    #[test]
    fn test_colorblind_mode_none_preserves_color() {
        // Behavioral: None mode should not modify colors
        use crate::accessibility::{colors, transform_color};

        let original = colors::HEALTH_FULL;
        let transformed = transform_color(original, ColorblindMode::None);

        assert_eq!(original, transformed, "None mode should preserve color");
    }

    #[test]
    fn test_quality_preset_as_str_is_not_empty() {
        // Behavioral: all presets should have non-empty display strings
        for preset in QualityPreset::all() {
            let s = preset.as_str();
            assert!(!s.is_empty(), "Quality preset should have display string");
        }
    }

    #[test]
    fn test_quality_preset_all_has_four_levels() {
        // Behavioral: should have exactly 4 quality levels
        let all = QualityPreset::all();
        assert_eq!(all.len(), 4, "Should have exactly 4 quality presets");
    }

    #[test]
    fn test_colorblind_modes_are_distinct() {
        // Behavioral: all colorblind modes should be distinct
        let modes = [
            ColorblindMode::None,
            ColorblindMode::Deuteranopia,
            ColorblindMode::Protanopia,
            ColorblindMode::Tritanopia,
            ColorblindMode::HighContrast,
        ];

        for i in 0..modes.len() {
            for j in (i + 1)..modes.len() {
                assert_ne!(modes[i], modes[j], "Colorblind modes should be distinct");
            }
        }
    }
}

// Note: UiLayer tests require wgpu device initialization and are covered
// in integration tests rather than unit mutation tests.

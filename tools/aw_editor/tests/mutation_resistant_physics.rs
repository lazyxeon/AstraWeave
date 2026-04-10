//! Mutation-resistant tests for Physics Panel system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::physics_panel::{
    ClothQuality, DestructionPattern, RagdollPreset, VehicleType,
};
use aw_editor_lib::panels::{PhysicsVisualization, SimulationMode};
use std::collections::HashSet;

// ============================================================================
// PHYSICS VISUALIZATION TESTS
// ============================================================================

mod physics_visualization_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(PhysicsVisualization::all().len(), 8);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = PhysicsVisualization::all()
            .iter()
            .map(|c| c.name())
            .collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = PhysicsVisualization::all()
            .iter()
            .map(|c| c.icon())
            .collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    #[test]
    fn test_all_descriptions_unique() {
        let descs: Vec<&str> = PhysicsVisualization::all()
            .iter()
            .map(|c| c.description())
            .collect();
        let unique: HashSet<_> = descs.iter().collect();
        assert_eq!(descs.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_none_name() {
        assert_eq!(PhysicsVisualization::None.name(), "None");
    }

    #[test]
    fn test_colliders_name() {
        assert_eq!(PhysicsVisualization::Colliders.name(), "Colliders");
    }

    #[test]
    fn test_contact_points_name() {
        assert_eq!(PhysicsVisualization::ContactPoints.name(), "Contact Points");
    }

    #[test]
    fn test_joints_name() {
        assert_eq!(PhysicsVisualization::Joints.name(), "Joints");
    }

    #[test]
    fn test_velocity_vectors_name() {
        assert_eq!(
            PhysicsVisualization::VelocityVectors.name(),
            "Velocity Vectors"
        );
    }

    #[test]
    fn test_broad_phase_name() {
        assert_eq!(PhysicsVisualization::BroadPhase.name(), "Broad Phase");
    }

    #[test]
    fn test_islands_name() {
        assert_eq!(PhysicsVisualization::Islands.name(), "Islands");
    }

    #[test]
    fn test_all_name() {
        assert_eq!(PhysicsVisualization::All.name(), "All");
    }

    // Test is_visible()
    #[test]
    fn test_none_is_not_visible() {
        assert!(!PhysicsVisualization::None.is_visible());
    }

    #[test]
    fn test_colliders_is_visible() {
        assert!(PhysicsVisualization::Colliders.is_visible());
    }

    #[test]
    fn test_contact_points_is_visible() {
        assert!(PhysicsVisualization::ContactPoints.is_visible());
    }

    #[test]
    fn test_joints_is_visible() {
        assert!(PhysicsVisualization::Joints.is_visible());
    }

    #[test]
    fn test_velocity_vectors_is_visible() {
        assert!(PhysicsVisualization::VelocityVectors.is_visible());
    }

    #[test]
    fn test_broad_phase_is_visible() {
        assert!(PhysicsVisualization::BroadPhase.is_visible());
    }

    #[test]
    fn test_islands_is_visible() {
        assert!(PhysicsVisualization::Islands.is_visible());
    }

    #[test]
    fn test_all_is_visible() {
        assert!(PhysicsVisualization::All.is_visible());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", PhysicsVisualization::Colliders);
        assert!(display.contains("[Pkg]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", PhysicsVisualization::Joints);
        assert!(display.contains("Joints"));
    }

    // Test default
    #[test]
    fn test_default_is_none() {
        assert_eq!(PhysicsVisualization::default(), PhysicsVisualization::None);
    }
}

// ============================================================================
// SIMULATION MODE TESTS
// ============================================================================

mod simulation_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(SimulationMode::all().len(), 4);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = SimulationMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = SimulationMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_running_name() {
        assert_eq!(SimulationMode::Running.name(), "Running");
    }

    #[test]
    fn test_paused_name() {
        assert_eq!(SimulationMode::Paused.name(), "Paused");
    }

    #[test]
    fn test_step_once_name() {
        assert_eq!(SimulationMode::StepOnce.name(), "Step Once");
    }

    #[test]
    fn test_slow_motion_name() {
        assert_eq!(SimulationMode::SlowMotion.name(), "Slow Motion");
    }

    // Test is_active()
    #[test]
    fn test_running_is_active() {
        assert!(SimulationMode::Running.is_active());
    }

    #[test]
    fn test_paused_is_not_active() {
        assert!(!SimulationMode::Paused.is_active());
    }

    #[test]
    fn test_step_once_is_not_active() {
        assert!(!SimulationMode::StepOnce.is_active());
    }

    #[test]
    fn test_slow_motion_is_active() {
        assert!(SimulationMode::SlowMotion.is_active());
    }

    // Test time_scale()
    #[test]
    fn test_running_time_scale() {
        assert_eq!(SimulationMode::Running.time_scale(), 1.0);
    }

    #[test]
    fn test_paused_time_scale() {
        assert_eq!(SimulationMode::Paused.time_scale(), 0.0);
    }

    #[test]
    fn test_step_once_time_scale() {
        assert_eq!(SimulationMode::StepOnce.time_scale(), 0.0);
    }

    #[test]
    fn test_slow_motion_time_scale() {
        assert_eq!(SimulationMode::SlowMotion.time_scale(), 0.2);
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", SimulationMode::Running);
        assert!(display.contains(">"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", SimulationMode::Paused);
        assert!(display.contains("Paused"));
    }

    // Test default
    #[test]
    fn test_default_is_running() {
        assert_eq!(SimulationMode::default(), SimulationMode::Running);
    }
}

// ============================================================================
// RAGDOLL PRESET TESTS
// ============================================================================

mod ragdoll_preset_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(RagdollPreset::all().len(), 5);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = RagdollPreset::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = RagdollPreset::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_humanoid_name() {
        assert_eq!(RagdollPreset::Humanoid.name(), "Humanoid");
    }

    #[test]
    fn test_quadruped_name() {
        assert_eq!(RagdollPreset::Quadruped.name(), "Quadruped");
    }

    #[test]
    fn test_bird_name() {
        assert_eq!(RagdollPreset::Bird.name(), "Bird");
    }

    #[test]
    fn test_insect_name() {
        assert_eq!(RagdollPreset::Insect.name(), "Insect");
    }

    #[test]
    fn test_custom_name() {
        assert_eq!(RagdollPreset::Custom.name(), "Custom");
    }

    // Test bone_count()
    #[test]
    fn test_humanoid_bone_count() {
        assert_eq!(RagdollPreset::Humanoid.bone_count(), 15);
    }

    #[test]
    fn test_quadruped_bone_count() {
        assert_eq!(RagdollPreset::Quadruped.bone_count(), 18);
    }

    #[test]
    fn test_bird_bone_count() {
        assert_eq!(RagdollPreset::Bird.bone_count(), 12);
    }

    #[test]
    fn test_insect_bone_count() {
        assert_eq!(RagdollPreset::Insect.bone_count(), 8);
    }

    #[test]
    fn test_custom_bone_count_is_zero() {
        assert_eq!(RagdollPreset::Custom.bone_count(), 0);
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", RagdollPreset::Humanoid);
        assert!(display.contains("🚶"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", RagdollPreset::Quadruped);
        assert!(display.contains("Quadruped"));
    }

    // Test default
    #[test]
    fn test_default_is_humanoid() {
        assert_eq!(RagdollPreset::default(), RagdollPreset::Humanoid);
    }
}

// ============================================================================
// VEHICLE TYPE TESTS
// ============================================================================

mod vehicle_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(VehicleType::all().len(), 6);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = VehicleType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = VehicleType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_sedan_name() {
        assert_eq!(VehicleType::Sedan.name(), "Sedan");
    }

    #[test]
    fn test_sports_car_name() {
        assert_eq!(VehicleType::SportsCar.name(), "Sports Car");
    }

    #[test]
    fn test_truck_name() {
        assert_eq!(VehicleType::Truck.name(), "Truck");
    }

    #[test]
    fn test_motorcycle_name() {
        assert_eq!(VehicleType::Motorcycle.name(), "Motorcycle");
    }

    #[test]
    fn test_tank_name() {
        assert_eq!(VehicleType::Tank.name(), "Tank");
    }

    #[test]
    fn test_custom_name() {
        assert_eq!(VehicleType::Custom.name(), "Custom");
    }

    // Test wheel_count()
    #[test]
    fn test_sedan_wheel_count() {
        assert_eq!(VehicleType::Sedan.wheel_count(), 4);
    }

    #[test]
    fn test_sports_car_wheel_count() {
        assert_eq!(VehicleType::SportsCar.wheel_count(), 4);
    }

    #[test]
    fn test_truck_wheel_count() {
        assert_eq!(VehicleType::Truck.wheel_count(), 6);
    }

    #[test]
    fn test_motorcycle_wheel_count() {
        assert_eq!(VehicleType::Motorcycle.wheel_count(), 2);
    }

    #[test]
    fn test_tank_wheel_count_is_zero() {
        assert_eq!(VehicleType::Tank.wheel_count(), 0);
    }

    #[test]
    fn test_custom_wheel_count() {
        assert_eq!(VehicleType::Custom.wheel_count(), 4);
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", VehicleType::Sedan);
        assert!(display.contains("🚗"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", VehicleType::SportsCar);
        assert!(display.contains("Sports Car"));
    }

    // Test default
    #[test]
    fn test_default_is_sedan() {
        assert_eq!(VehicleType::default(), VehicleType::Sedan);
    }
}

// ============================================================================
// CLOTH QUALITY TESTS
// ============================================================================

mod cloth_quality_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ClothQuality::all().len(), 4);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ClothQuality::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_low_name() {
        assert_eq!(ClothQuality::Low.name(), "Low");
    }

    #[test]
    fn test_medium_name() {
        assert_eq!(ClothQuality::Medium.name(), "Medium");
    }

    #[test]
    fn test_high_name() {
        assert_eq!(ClothQuality::High.name(), "High");
    }

    #[test]
    fn test_ultra_name() {
        assert_eq!(ClothQuality::Ultra.name(), "Ultra");
    }

    // Test iterations()
    #[test]
    fn test_low_iterations() {
        assert_eq!(ClothQuality::Low.iterations(), 4);
    }

    #[test]
    fn test_medium_iterations() {
        assert_eq!(ClothQuality::Medium.iterations(), 8);
    }

    #[test]
    fn test_high_iterations() {
        assert_eq!(ClothQuality::High.iterations(), 16);
    }

    #[test]
    fn test_ultra_iterations() {
        assert_eq!(ClothQuality::Ultra.iterations(), 32);
    }

    // Test cpu_cost()
    #[test]
    fn test_low_cpu_cost() {
        assert_eq!(ClothQuality::Low.cpu_cost(), 1);
    }

    #[test]
    fn test_medium_cpu_cost() {
        assert_eq!(ClothQuality::Medium.cpu_cost(), 2);
    }

    #[test]
    fn test_high_cpu_cost() {
        assert_eq!(ClothQuality::High.cpu_cost(), 4);
    }

    #[test]
    fn test_ultra_cpu_cost() {
        assert_eq!(ClothQuality::Ultra.cpu_cost(), 8);
    }

    // Test Display trait
    #[test]
    fn test_display_equals_name() {
        assert_eq!(format!("{}", ClothQuality::Medium), "Medium");
    }

    // Test default
    #[test]
    fn test_default_is_medium() {
        assert_eq!(ClothQuality::default(), ClothQuality::Medium);
    }
}

// ============================================================================
// DESTRUCTION PATTERN TESTS
// ============================================================================

mod destruction_pattern_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(DestructionPattern::all().len(), 5);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = DestructionPattern::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = DestructionPattern::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_voronoi_name() {
        assert_eq!(DestructionPattern::Voronoi.name(), "Voronoi");
    }

    #[test]
    fn test_radial_name() {
        assert_eq!(DestructionPattern::Radial.name(), "Radial");
    }

    #[test]
    fn test_slice_name() {
        assert_eq!(DestructionPattern::Slice.name(), "Slice");
    }

    #[test]
    fn test_shatter_name() {
        assert_eq!(DestructionPattern::Shatter.name(), "Shatter");
    }

    #[test]
    fn test_crumble_name() {
        assert_eq!(DestructionPattern::Crumble.name(), "Crumble");
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", DestructionPattern::Voronoi);
        assert!(display.contains("⬡"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", DestructionPattern::Shatter);
        assert!(display.contains("Shatter"));
    }

    // Test default
    #[test]
    fn test_default_is_voronoi() {
        assert_eq!(DestructionPattern::default(), DestructionPattern::Voronoi);
    }
}

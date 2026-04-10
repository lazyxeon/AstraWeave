//! Mutation-resistant tests for Prefab system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::prefab::{EntityOverrides, PrefabHierarchySnapshot, PrefabIssue, PrefabStats};
use std::path::PathBuf;

// ============================================================================
// ENTITY OVERRIDES TESTS
// ============================================================================

mod entity_overrides_tests {
    use super::*;

    // Test has_pose_override boundary cases (OR of two Options)
    #[test]
    fn test_has_pose_override_none_none() {
        let overrides = EntityOverrides::default();
        assert!(!overrides.has_pose_override());
    }

    #[test]
    fn test_has_pose_override_some_none() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: None,
            health: None,
            max_health: None,
        };
        assert!(overrides.has_pose_override());
    }

    #[test]
    fn test_has_pose_override_none_some() {
        let overrides = EntityOverrides {
            pos_x: None,
            pos_y: Some(20),
            health: None,
            max_health: None,
        };
        assert!(overrides.has_pose_override());
    }

    #[test]
    fn test_has_pose_override_some_some() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: Some(20),
            health: None,
            max_health: None,
        };
        assert!(overrides.has_pose_override());
    }

    // Test has_health_override boundary cases (OR of two Options)
    #[test]
    fn test_has_health_override_none_none() {
        let overrides = EntityOverrides::default();
        assert!(!overrides.has_health_override());
    }

    #[test]
    fn test_has_health_override_some_none() {
        let overrides = EntityOverrides {
            pos_x: None,
            pos_y: None,
            health: Some(50),
            max_health: None,
        };
        assert!(overrides.has_health_override());
    }

    #[test]
    fn test_has_health_override_none_some() {
        let overrides = EntityOverrides {
            pos_x: None,
            pos_y: None,
            health: None,
            max_health: Some(100),
        };
        assert!(overrides.has_health_override());
    }

    #[test]
    fn test_has_health_override_some_some() {
        let overrides = EntityOverrides {
            pos_x: None,
            pos_y: None,
            health: Some(50),
            max_health: Some(100),
        };
        assert!(overrides.has_health_override());
    }

    // Test has_any_override (OR of has_pose_override and has_health_override)
    #[test]
    fn test_has_any_override_all_none() {
        let overrides = EntityOverrides::default();
        assert!(!overrides.has_any_override());
    }

    #[test]
    fn test_has_any_override_only_pose() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: None,
            health: None,
            max_health: None,
        };
        assert!(overrides.has_any_override());
    }

    #[test]
    fn test_has_any_override_only_health() {
        let overrides = EntityOverrides {
            pos_x: None,
            pos_y: None,
            health: Some(50),
            max_health: None,
        };
        assert!(overrides.has_any_override());
    }

    #[test]
    fn test_has_any_override_both() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: None,
            health: Some(50),
            max_health: None,
        };
        assert!(overrides.has_any_override());
    }

    // Test override_count
    #[test]
    fn test_override_count_zero() {
        let overrides = EntityOverrides::default();
        assert_eq!(overrides.override_count(), 0);
    }

    #[test]
    fn test_override_count_one() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: None,
            health: None,
            max_health: None,
        };
        assert_eq!(overrides.override_count(), 1);
    }

    #[test]
    fn test_override_count_two() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: Some(20),
            health: None,
            max_health: None,
        };
        assert_eq!(overrides.override_count(), 2);
    }

    #[test]
    fn test_override_count_three() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: Some(20),
            health: Some(50),
            max_health: None,
        };
        assert_eq!(overrides.override_count(), 3);
    }

    #[test]
    fn test_override_count_four() {
        let overrides = EntityOverrides {
            pos_x: Some(10),
            pos_y: Some(20),
            health: Some(50),
            max_health: Some(100),
        };
        assert_eq!(overrides.override_count(), 4);
    }

    // Test Default trait
    #[test]
    fn test_default_pos_x_is_none() {
        let overrides = EntityOverrides::default();
        assert!(overrides.pos_x.is_none());
    }

    #[test]
    fn test_default_pos_y_is_none() {
        let overrides = EntityOverrides::default();
        assert!(overrides.pos_y.is_none());
    }

    #[test]
    fn test_default_health_is_none() {
        let overrides = EntityOverrides::default();
        assert!(overrides.health.is_none());
    }

    #[test]
    fn test_default_max_health_is_none() {
        let overrides = EntityOverrides::default();
        assert!(overrides.max_health.is_none());
    }
}

// ============================================================================
// PREFAB STATS TESTS
// ============================================================================

mod prefab_stats_tests {
    use super::*;

    // Test has_overrides boundary (overridden_entity_count > 0)
    #[test]
    fn test_has_overrides_zero() {
        let stats = PrefabStats::default();
        assert!(!stats.has_overrides());
    }

    #[test]
    fn test_has_overrides_one() {
        let stats = PrefabStats {
            overridden_entity_count: 1,
            ..Default::default()
        };
        assert!(stats.has_overrides());
    }

    #[test]
    fn test_has_overrides_many() {
        let stats = PrefabStats {
            overridden_entity_count: 100,
            ..Default::default()
        };
        assert!(stats.has_overrides());
    }

    // Test avg_entities_per_instance boundary (division by zero)
    #[test]
    fn test_avg_entities_per_instance_zero_instances() {
        let stats = PrefabStats {
            instance_count: 0,
            total_prefab_entities: 0,
            ..Default::default()
        };
        assert_eq!(stats.avg_entities_per_instance(), 0.0);
    }

    #[test]
    fn test_avg_entities_per_instance_one_instance() {
        let stats = PrefabStats {
            instance_count: 1,
            total_prefab_entities: 10,
            ..Default::default()
        };
        assert_eq!(stats.avg_entities_per_instance(), 10.0);
    }

    #[test]
    fn test_avg_entities_per_instance_multiple() {
        let stats = PrefabStats {
            instance_count: 4,
            total_prefab_entities: 20,
            ..Default::default()
        };
        assert_eq!(stats.avg_entities_per_instance(), 5.0);
    }

    #[test]
    fn test_avg_entities_per_instance_fractional() {
        let stats = PrefabStats {
            instance_count: 3,
            total_prefab_entities: 10,
            ..Default::default()
        };
        let avg = stats.avg_entities_per_instance();
        assert!((avg - 3.333).abs() < 0.01);
    }

    // Test override_percentage boundary (division by zero)
    #[test]
    fn test_override_percentage_zero_entities() {
        let stats = PrefabStats {
            total_prefab_entities: 0,
            overridden_entity_count: 0,
            ..Default::default()
        };
        assert_eq!(stats.override_percentage(), 0.0);
    }

    #[test]
    fn test_override_percentage_no_overrides() {
        let stats = PrefabStats {
            total_prefab_entities: 100,
            overridden_entity_count: 0,
            ..Default::default()
        };
        assert_eq!(stats.override_percentage(), 0.0);
    }

    #[test]
    fn test_override_percentage_half() {
        let stats = PrefabStats {
            total_prefab_entities: 100,
            overridden_entity_count: 50,
            ..Default::default()
        };
        assert_eq!(stats.override_percentage(), 50.0);
    }

    #[test]
    fn test_override_percentage_all() {
        let stats = PrefabStats {
            total_prefab_entities: 100,
            overridden_entity_count: 100,
            ..Default::default()
        };
        assert_eq!(stats.override_percentage(), 100.0);
    }

    // Test default values
    #[test]
    fn test_default_instance_count() {
        let stats = PrefabStats::default();
        assert_eq!(stats.instance_count, 0);
    }

    #[test]
    fn test_default_total_prefab_entities() {
        let stats = PrefabStats::default();
        assert_eq!(stats.total_prefab_entities, 0);
    }

    #[test]
    fn test_default_instances_with_overrides() {
        let stats = PrefabStats::default();
        assert_eq!(stats.instances_with_overrides, 0);
    }

    #[test]
    fn test_default_overridden_entity_count() {
        let stats = PrefabStats::default();
        assert_eq!(stats.overridden_entity_count, 0);
    }

    #[test]
    fn test_default_total_override_count() {
        let stats = PrefabStats::default();
        assert_eq!(stats.total_override_count, 0);
    }

    #[test]
    fn test_default_prefab_file_count() {
        let stats = PrefabStats::default();
        assert_eq!(stats.prefab_file_count, 0);
    }
}

// ============================================================================
// PREFAB ISSUE TESTS
// ============================================================================

mod prefab_issue_tests {
    use super::*;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    // Test all_variants
    #[test]
    fn test_all_variants_count() {
        assert_eq!(PrefabIssue::all_variants().len(), 6);
    }

    #[test]
    fn test_all_variants_contains_missing_file() {
        assert!(PrefabIssue::all_variants().contains(&"MissingFile"));
    }

    #[test]
    fn test_all_variants_contains_orphaned_entity() {
        assert!(PrefabIssue::all_variants().contains(&"OrphanedEntity"));
    }

    #[test]
    fn test_all_variants_contains_empty_prefab() {
        assert!(PrefabIssue::all_variants().contains(&"EmptyPrefab"));
    }

    #[test]
    fn test_all_variants_contains_empty_mapping() {
        assert!(PrefabIssue::all_variants().contains(&"EmptyMapping"));
    }

    #[test]
    fn test_all_variants_contains_cyclic_reference() {
        assert!(PrefabIssue::all_variants().contains(&"CyclicReference"));
    }

    #[test]
    fn test_all_variants_contains_invalid_root_index() {
        assert!(PrefabIssue::all_variants().contains(&"InvalidRootIndex"));
    }

    // Test name()
    #[test]
    fn test_missing_file_name() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.name(), "Missing File");
    }

    #[test]
    fn test_orphaned_entity_name() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.name(), "Orphaned Entity");
    }

    #[test]
    fn test_empty_prefab_name() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.name(), "Empty Prefab");
    }

    #[test]
    fn test_empty_mapping_name() {
        let issue = PrefabIssue::EmptyMapping {
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.name(), "Empty Mapping");
    }

    #[test]
    fn test_cyclic_reference_name() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.name(), "Cyclic Reference");
    }

    #[test]
    fn test_invalid_root_index_name() {
        let issue = PrefabIssue::InvalidRootIndex {
            path: make_path("test.prefab"),
            index: 5,
            entity_count: 3,
        };
        assert_eq!(issue.name(), "Invalid Root Index");
    }

    // Test icon() - all unique
    #[test]
    fn test_missing_file_icon() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.icon(), "❓");
    }

    #[test]
    fn test_orphaned_entity_icon() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.icon(), "👻");
    }

    #[test]
    fn test_empty_prefab_icon() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.icon(), "[Pkg]");
    }

    #[test]
    fn test_empty_mapping_icon() {
        let issue = PrefabIssue::EmptyMapping {
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.icon(), "🗺️");
    }

    #[test]
    fn test_cyclic_reference_icon() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.icon(), "[Sync]");
    }

    #[test]
    fn test_invalid_root_index_icon() {
        let issue = PrefabIssue::InvalidRootIndex {
            path: make_path("test.prefab"),
            index: 5,
            entity_count: 3,
        };
        assert_eq!(issue.icon(), "#️⃣");
    }

    // Test is_critical - only MissingFile, CyclicReference, InvalidRootIndex
    #[test]
    fn test_missing_file_is_critical() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert!(issue.is_critical());
    }

    #[test]
    fn test_orphaned_entity_is_not_critical() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert!(!issue.is_critical());
    }

    #[test]
    fn test_empty_prefab_is_not_critical() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_critical());
    }

    #[test]
    fn test_empty_mapping_is_not_critical() {
        let issue = PrefabIssue::EmptyMapping {
            prefab: make_path("test.prefab"),
        };
        assert!(!issue.is_critical());
    }

    #[test]
    fn test_cyclic_reference_is_critical() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert!(issue.is_critical());
    }

    #[test]
    fn test_invalid_root_index_is_critical() {
        let issue = PrefabIssue::InvalidRootIndex {
            path: make_path("test.prefab"),
            index: 5,
            entity_count: 3,
        };
        assert!(issue.is_critical());
    }

    // Test is_file_issue - only MissingFile
    #[test]
    fn test_missing_file_is_file_issue() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert!(issue.is_file_issue());
    }

    #[test]
    fn test_orphaned_entity_is_not_file_issue() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert!(!issue.is_file_issue());
    }

    #[test]
    fn test_empty_prefab_is_not_file_issue() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_file_issue());
    }

    #[test]
    fn test_cyclic_reference_is_not_file_issue() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_file_issue());
    }

    // Test is_entity_issue - only OrphanedEntity
    #[test]
    fn test_missing_file_is_not_entity_issue() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_entity_issue());
    }

    #[test]
    fn test_orphaned_entity_is_entity_issue() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert!(issue.is_entity_issue());
    }

    #[test]
    fn test_empty_prefab_is_not_entity_issue() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_entity_issue());
    }

    #[test]
    fn test_cyclic_reference_is_not_entity_issue() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert!(!issue.is_entity_issue());
    }

    // Test path() - all variants have paths
    #[test]
    fn test_missing_file_path() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    #[test]
    fn test_orphaned_entity_path() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    #[test]
    fn test_empty_prefab_path() {
        let issue = PrefabIssue::EmptyPrefab {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    #[test]
    fn test_empty_mapping_path() {
        let issue = PrefabIssue::EmptyMapping {
            prefab: make_path("test.prefab"),
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    #[test]
    fn test_cyclic_reference_path() {
        let issue = PrefabIssue::CyclicReference {
            path: make_path("test.prefab"),
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    #[test]
    fn test_invalid_root_index_path() {
        let issue = PrefabIssue::InvalidRootIndex {
            path: make_path("test.prefab"),
            index: 5,
            entity_count: 3,
        };
        assert_eq!(issue.path(), Some(&make_path("test.prefab")));
    }

    // Test Display trait
    #[test]
    fn test_display_missing_file() {
        let issue = PrefabIssue::MissingFile {
            path: make_path("test.prefab"),
        };
        let display = format!("{}", issue);
        assert!(display.contains("Missing File"));
        assert!(display.contains("test.prefab"));
    }

    #[test]
    fn test_display_orphaned_entity() {
        let issue = PrefabIssue::OrphanedEntity {
            entity: 42,
            prefab: make_path("test.prefab"),
        };
        let display = format!("{}", issue);
        assert!(display.contains("42"));
        assert!(display.contains("Orphaned"));
    }

    #[test]
    fn test_display_invalid_root_index() {
        let issue = PrefabIssue::InvalidRootIndex {
            path: make_path("test.prefab"),
            index: 5,
            entity_count: 3,
        };
        let display = format!("{}", issue);
        assert!(display.contains("5"));
        assert!(display.contains("3"));
    }
}

// ============================================================================
// PREFAB HIERARCHY SNAPSHOT TESTS
// ============================================================================

mod prefab_hierarchy_snapshot_tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let snapshot = PrefabHierarchySnapshot::new();
        assert!(snapshot.children_of(1_u32).is_empty());
    }

    #[test]
    fn test_insert_children() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.insert_children(1_u32, vec![2_u32, 3_u32, 4_u32]);
        assert_eq!(snapshot.children_of(1_u32).len(), 3);
    }

    #[test]
    fn test_insert_children_replaces() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.insert_children(1_u32, vec![2_u32, 3_u32]);
        snapshot.insert_children(1_u32, vec![4_u32, 5_u32, 6_u32]);
        assert_eq!(snapshot.children_of(1_u32).len(), 3);
        assert_eq!(snapshot.children_of(1_u32), &[4_u32, 5_u32, 6_u32]);
    }

    #[test]
    fn test_add_child() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.add_child(1_u32, 2_u32);
        snapshot.add_child(1_u32, 3_u32);
        assert_eq!(snapshot.children_of(1_u32).len(), 2);
    }

    #[test]
    fn test_add_child_preserves_order() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.add_child(1_u32, 2_u32);
        snapshot.add_child(1_u32, 3_u32);
        snapshot.add_child(1_u32, 4_u32);
        assert_eq!(snapshot.children_of(1_u32), &[2_u32, 3_u32, 4_u32]);
    }

    #[test]
    fn test_children_of_unknown_parent() {
        let snapshot = PrefabHierarchySnapshot::new();
        assert!(snapshot.children_of(999_u32).is_empty());
    }

    #[test]
    fn test_children_of_empty_children() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.insert_children(1_u32, vec![]);
        assert!(snapshot.children_of(1_u32).is_empty());
    }

    #[test]
    fn test_from_iterator() {
        let data = vec![(1_u32, vec![2_u32, 3_u32]), (4_u32, vec![5_u32])];
        let snapshot: PrefabHierarchySnapshot = data.into_iter().collect();
        assert_eq!(snapshot.children_of(1_u32).len(), 2);
        assert_eq!(snapshot.children_of(4_u32).len(), 1);
    }

    #[test]
    fn test_multiple_parents() {
        let mut snapshot = PrefabHierarchySnapshot::new();
        snapshot.insert_children(1_u32, vec![2_u32, 3_u32]);
        snapshot.insert_children(4_u32, vec![5_u32, 6_u32, 7_u32]);
        snapshot.insert_children(8_u32, vec![9_u32]);

        assert_eq!(snapshot.children_of(1_u32).len(), 2);
        assert_eq!(snapshot.children_of(4_u32).len(), 3);
        assert_eq!(snapshot.children_of(8_u32).len(), 1);
    }
}

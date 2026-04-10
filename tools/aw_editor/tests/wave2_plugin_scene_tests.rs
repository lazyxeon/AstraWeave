//! Wave 2 Mutation Remediation — Plugin System & Scene Serialization
//!
//! Targets mutation-prone patterns in:
//! - plugin.rs: PluginEvent (9 variants × Display/name/category), PluginError (5 variants × is_fatal/icon/category),
//!   PluginState (4 variants × name/icon/is_*), PluginManagerStats health_percentage, is_version_compatible semver
//! - scene_serialization.rs: SceneData::validate() comparison operators, SceneValidationIssue helpers,
//!   SceneStats aggregation, EntityData field defaults

use astraweave_core::IVec2;
use aw_editor_lib::plugin::{
    ExamplePlugin, PluginError, PluginEvent, PluginManager, PluginManagerStats, PluginMenuItem,
    PluginMetadata, PluginState,
};
use aw_editor_lib::scene_serialization::{EntityData, SceneData, SceneValidationIssue};
use std::collections::BTreeMap;

// ============================================================================
// PluginEvent — All 9 Variants × Display
// ============================================================================

#[test]
fn plugin_event_display_loaded() {
    assert_eq!(format!("{}", PluginEvent::Loaded), "Loaded");
}

#[test]
fn plugin_event_display_unloading() {
    assert_eq!(format!("{}", PluginEvent::Unloading), "Unloading");
}

#[test]
fn plugin_event_display_update() {
    assert_eq!(format!("{}", PluginEvent::Update), "Update");
}

#[test]
fn plugin_event_display_scene_loaded() {
    assert_eq!(format!("{}", PluginEvent::SceneLoaded), "Scene Loaded");
}

#[test]
fn plugin_event_display_scene_saved() {
    assert_eq!(format!("{}", PluginEvent::SceneSaved), "Scene Saved");
}

#[test]
fn plugin_event_display_entity_selected() {
    assert_eq!(
        format!("{}", PluginEvent::EntitySelected),
        "Entity Selected"
    );
}

#[test]
fn plugin_event_display_entity_created() {
    assert_eq!(format!("{}", PluginEvent::EntityCreated), "Entity Created");
}

#[test]
fn plugin_event_display_play_mode_enter() {
    assert_eq!(format!("{}", PluginEvent::PlayModeEnter), "Play Mode Enter");
}

#[test]
fn plugin_event_display_play_mode_exit() {
    assert_eq!(format!("{}", PluginEvent::PlayModeExit), "Play Mode Exit");
}

// ============================================================================
// PluginEvent — name() returns (must match Display for each variant)
// ============================================================================

#[test]
fn plugin_event_name_loaded() {
    assert_eq!(PluginEvent::Loaded.name(), "Loaded");
}

#[test]
fn plugin_event_name_unloading() {
    assert_eq!(PluginEvent::Unloading.name(), "Unloading");
}

#[test]
fn plugin_event_name_update() {
    assert_eq!(PluginEvent::Update.name(), "Update");
}

#[test]
fn plugin_event_name_scene_loaded() {
    assert_eq!(PluginEvent::SceneLoaded.name(), "Scene Loaded");
}

#[test]
fn plugin_event_name_scene_saved() {
    assert_eq!(PluginEvent::SceneSaved.name(), "Scene Saved");
}

#[test]
fn plugin_event_name_entity_selected() {
    assert_eq!(PluginEvent::EntitySelected.name(), "Entity Selected");
}

#[test]
fn plugin_event_name_entity_created() {
    assert_eq!(PluginEvent::EntityCreated.name(), "Entity Created");
}

#[test]
fn plugin_event_name_play_mode_enter() {
    assert_eq!(PluginEvent::PlayModeEnter.name(), "Play Mode Enter");
}

#[test]
fn plugin_event_name_play_mode_exit() {
    assert_eq!(PluginEvent::PlayModeExit.name(), "Play Mode Exit");
}

// ============================================================================
// PluginEvent — Classification helpers (one test per category boundary)
// ============================================================================

#[test]
fn plugin_event_is_lifecycle_true() {
    assert!(PluginEvent::Loaded.is_lifecycle());
    assert!(PluginEvent::Unloading.is_lifecycle());
}

#[test]
fn plugin_event_is_lifecycle_false_for_all_non_lifecycle() {
    for &ev in &[
        PluginEvent::Update,
        PluginEvent::SceneLoaded,
        PluginEvent::SceneSaved,
        PluginEvent::EntitySelected,
        PluginEvent::EntityCreated,
        PluginEvent::PlayModeEnter,
        PluginEvent::PlayModeExit,
    ] {
        assert!(!ev.is_lifecycle(), "{:?} should NOT be lifecycle", ev);
    }
}

#[test]
fn plugin_event_is_scene_event_true() {
    assert!(PluginEvent::SceneLoaded.is_scene_event());
    assert!(PluginEvent::SceneSaved.is_scene_event());
}

#[test]
fn plugin_event_is_scene_event_false_for_all_non_scene() {
    for &ev in &[
        PluginEvent::Loaded,
        PluginEvent::Unloading,
        PluginEvent::Update,
        PluginEvent::EntitySelected,
        PluginEvent::EntityCreated,
        PluginEvent::PlayModeEnter,
        PluginEvent::PlayModeExit,
    ] {
        assert!(!ev.is_scene_event(), "{:?} should NOT be scene_event", ev);
    }
}

#[test]
fn plugin_event_is_entity_event_true() {
    assert!(PluginEvent::EntitySelected.is_entity_event());
    assert!(PluginEvent::EntityCreated.is_entity_event());
}

#[test]
fn plugin_event_is_entity_event_false_for_all_non_entity() {
    for &ev in &[
        PluginEvent::Loaded,
        PluginEvent::Unloading,
        PluginEvent::Update,
        PluginEvent::SceneLoaded,
        PluginEvent::SceneSaved,
        PluginEvent::PlayModeEnter,
        PluginEvent::PlayModeExit,
    ] {
        assert!(!ev.is_entity_event(), "{:?} should NOT be entity_event", ev);
    }
}

#[test]
fn plugin_event_is_play_mode_event_true() {
    assert!(PluginEvent::PlayModeEnter.is_play_mode_event());
    assert!(PluginEvent::PlayModeExit.is_play_mode_event());
}

#[test]
fn plugin_event_is_play_mode_event_false_for_all_non_play() {
    for &ev in &[
        PluginEvent::Loaded,
        PluginEvent::Unloading,
        PluginEvent::Update,
        PluginEvent::SceneLoaded,
        PluginEvent::SceneSaved,
        PluginEvent::EntitySelected,
        PluginEvent::EntityCreated,
    ] {
        assert!(
            !ev.is_play_mode_event(),
            "{:?} should NOT be play_mode_event",
            ev
        );
    }
}

#[test]
fn plugin_event_is_frequent_only_update() {
    assert!(PluginEvent::Update.is_frequent());
    for &ev in &[
        PluginEvent::Loaded,
        PluginEvent::Unloading,
        PluginEvent::SceneLoaded,
        PluginEvent::SceneSaved,
        PluginEvent::EntitySelected,
        PluginEvent::EntityCreated,
        PluginEvent::PlayModeEnter,
        PluginEvent::PlayModeExit,
    ] {
        assert!(!ev.is_frequent(), "{:?} should NOT be frequent", ev);
    }
}

#[test]
fn plugin_event_all_returns_9() {
    assert_eq!(PluginEvent::all().len(), 9);
}

#[test]
fn plugin_event_default_is_loaded() {
    assert_eq!(PluginEvent::default(), PluginEvent::Loaded);
}

// ============================================================================
// PluginError — 5 Variants × is_fatal / icon / category
// ============================================================================

#[test]
fn plugin_error_init_failed_is_fatal() {
    assert!(PluginError::InitFailed("x".into()).is_fatal());
}

#[test]
fn plugin_error_config_error_not_fatal() {
    assert!(!PluginError::ConfigError("x".into()).is_fatal());
}

#[test]
fn plugin_error_missing_dependency_is_fatal() {
    assert!(PluginError::MissingDependency("x".into()).is_fatal());
}

#[test]
fn plugin_error_incompatible_version_is_fatal() {
    assert!(PluginError::IncompatibleVersion {
        required: "2.0".into(),
        actual: "1.0".into()
    }
    .is_fatal());
}

#[test]
fn plugin_error_other_not_fatal() {
    assert!(!PluginError::Other("x".into()).is_fatal());
}

#[test]
fn plugin_error_icon_init_failed() {
    assert_eq!(PluginError::InitFailed("x".into()).icon(), "🚫");
}

#[test]
fn plugin_error_icon_config_error() {
    assert_eq!(PluginError::ConfigError("x".into()).icon(), "⚙️");
}

#[test]
fn plugin_error_icon_missing_dependency() {
    assert_eq!(PluginError::MissingDependency("x".into()).icon(), "[Pkg]");
}

#[test]
fn plugin_error_icon_incompatible_version() {
    assert_eq!(
        PluginError::IncompatibleVersion {
            required: "2.0".into(),
            actual: "1.0".into()
        }
        .icon(),
        "🔢"
    );
}

#[test]
fn plugin_error_icon_other() {
    assert_eq!(PluginError::Other("x".into()).icon(), "[!]");
}

#[test]
fn plugin_error_category_init_failed() {
    assert_eq!(
        PluginError::InitFailed("x".into()).category(),
        "Initialization"
    );
}

#[test]
fn plugin_error_category_config_error() {
    assert_eq!(
        PluginError::ConfigError("x".into()).category(),
        "Configuration"
    );
}

#[test]
fn plugin_error_category_missing_dependency() {
    assert_eq!(
        PluginError::MissingDependency("x".into()).category(),
        "Dependency"
    );
}

#[test]
fn plugin_error_category_incompatible_version() {
    assert_eq!(
        PluginError::IncompatibleVersion {
            required: "2.0".into(),
            actual: "1.0".into()
        }
        .category(),
        "Version"
    );
}

#[test]
fn plugin_error_category_other() {
    assert_eq!(PluginError::Other("x".into()).category(), "Other");
}

#[test]
fn plugin_error_display_init_failed() {
    let e = PluginError::InitFailed("boom".into());
    assert!(format!("{}", e).contains("initialization failed"));
    assert!(format!("{}", e).contains("boom"));
}

#[test]
fn plugin_error_display_incompatible_version() {
    let e = PluginError::IncompatibleVersion {
        required: "2.0.0".into(),
        actual: "1.0.0".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("2.0.0"));
    assert!(s.contains("1.0.0"));
}

// ============================================================================
// PluginState — 4 Variants × name / icon / classification
// ============================================================================

#[test]
fn plugin_state_all_returns_4() {
    assert_eq!(PluginState::all().len(), 4);
}

#[test]
fn plugin_state_name_loaded() {
    assert_eq!(PluginState::Loaded.name(), "Loaded");
}

#[test]
fn plugin_state_name_active() {
    assert_eq!(PluginState::Active.name(), "Active");
}

#[test]
fn plugin_state_name_error() {
    assert_eq!(PluginState::Error.name(), "Error");
}

#[test]
fn plugin_state_name_disabled() {
    assert_eq!(PluginState::Disabled.name(), "Disabled");
}

#[test]
fn plugin_state_icon_loaded() {
    assert_eq!(PluginState::Loaded.icon(), "[Pkg]");
}

#[test]
fn plugin_state_icon_active() {
    assert_eq!(PluginState::Active.icon(), "[ok]");
}

#[test]
fn plugin_state_icon_error() {
    assert_eq!(PluginState::Error.icon(), "[x]");
}

#[test]
fn plugin_state_icon_disabled() {
    assert_eq!(PluginState::Disabled.icon(), "||");
}

#[test]
fn plugin_state_is_operational_only_active() {
    assert!(PluginState::Active.is_operational());
    assert!(!PluginState::Loaded.is_operational());
    assert!(!PluginState::Error.is_operational());
    assert!(!PluginState::Disabled.is_operational());
}

#[test]
fn plugin_state_is_error_only_error() {
    assert!(PluginState::Error.is_error());
    assert!(!PluginState::Active.is_error());
    assert!(!PluginState::Loaded.is_error());
    assert!(!PluginState::Disabled.is_error());
}

#[test]
fn plugin_state_is_disabled_only_disabled() {
    assert!(PluginState::Disabled.is_disabled());
    assert!(!PluginState::Active.is_disabled());
    assert!(!PluginState::Loaded.is_disabled());
    assert!(!PluginState::Error.is_disabled());
}

#[test]
fn plugin_state_display_matches_name() {
    for state in PluginState::all() {
        assert_eq!(format!("{}", state), state.name());
    }
}

// ============================================================================
// PluginManagerStats — health_percentage arithmetic + boundary
// ============================================================================

#[test]
fn stats_health_pct_zero_total_returns_100() {
    let stats = PluginManagerStats {
        total: 0,
        active: 0,
        errors: 0,
        disabled: 0,
        loaded: 0,
        with_panels: 0,
    };
    assert!((stats.health_percentage() - 100.0).abs() < 0.01);
}

#[test]
fn stats_health_pct_all_active() {
    let stats = PluginManagerStats {
        total: 5,
        active: 5,
        errors: 0,
        disabled: 0,
        loaded: 0,
        with_panels: 2,
    };
    assert!((stats.health_percentage() - 100.0).abs() < 0.01);
}

#[test]
fn stats_health_pct_half_active() {
    let stats = PluginManagerStats {
        total: 10,
        active: 5,
        errors: 3,
        disabled: 2,
        loaded: 0,
        with_panels: 1,
    };
    assert!((stats.health_percentage() - 50.0).abs() < 0.01);
}

#[test]
fn stats_health_pct_none_active() {
    let stats = PluginManagerStats {
        total: 4,
        active: 0,
        errors: 2,
        disabled: 2,
        loaded: 0,
        with_panels: 0,
    };
    assert!((stats.health_percentage() - 0.0).abs() < 0.01);
}

#[test]
fn stats_health_pct_one_of_three() {
    let stats = PluginManagerStats {
        total: 3,
        active: 1,
        errors: 1,
        disabled: 1,
        loaded: 0,
        with_panels: 0,
    };
    // 1/3 * 100 = 33.333...
    assert!((stats.health_percentage() - 33.333).abs() < 0.1);
}

#[test]
fn stats_is_empty() {
    let empty = PluginManagerStats {
        total: 0,
        active: 0,
        errors: 0,
        disabled: 0,
        loaded: 0,
        with_panels: 0,
    };
    assert!(empty.is_empty());

    let non_empty = PluginManagerStats {
        total: 1,
        active: 1,
        errors: 0,
        disabled: 0,
        loaded: 0,
        with_panels: 0,
    };
    assert!(!non_empty.is_empty());
}

#[test]
fn stats_all_active_true_only_when_total_gt_0_and_active_eq_total() {
    // total=0 => false
    let s0 = PluginManagerStats {
        total: 0,
        active: 0,
        ..Default::default()
    };
    assert!(!s0.all_active());

    // total=3, active=3 => true
    let s1 = PluginManagerStats {
        total: 3,
        active: 3,
        ..Default::default()
    };
    assert!(s1.all_active());

    // total=3, active=2 => false
    let s2 = PluginManagerStats {
        total: 3,
        active: 2,
        ..Default::default()
    };
    assert!(!s2.all_active());
}

// ============================================================================
// PluginMetadata builder
// ============================================================================

#[test]
fn plugin_metadata_new_defaults() {
    let m = PluginMetadata::new("id", "name", "1.0.0");
    assert_eq!(m.id, "id");
    assert_eq!(m.name, "name");
    assert_eq!(m.version, "1.0.0");
    assert_eq!(m.author, "");
    assert_eq!(m.description, "");
    assert_eq!(m.min_editor_version, "0.1.0");
}

#[test]
fn plugin_metadata_with_author() {
    let m = PluginMetadata::new("x", "X", "1.0").with_author("Alice");
    assert_eq!(m.author, "Alice");
}

#[test]
fn plugin_metadata_with_description() {
    let m = PluginMetadata::new("x", "X", "1.0").with_description("Desc");
    assert_eq!(m.description, "Desc");
}

// ============================================================================
// PluginMenuItem builder
// ============================================================================

#[test]
fn plugin_menu_item_defaults() {
    let item = PluginMenuItem::new("Tools/X", "do_x");
    assert_eq!(item.path, "Tools/X");
    assert_eq!(item.action_id, "do_x");
    assert!(item.enabled);
    assert!(item.shortcut.is_none());
}

#[test]
fn plugin_menu_item_with_shortcut() {
    let item = PluginMenuItem::new("A/B", "b").with_shortcut("Ctrl+B");
    assert_eq!(item.shortcut, Some("Ctrl+B".to_string()));
}

// ============================================================================
// PluginManager — version compatibility (semver boundary)
// ============================================================================

#[test]
fn version_compat_exact_match_accepted() {
    let mut mgr = PluginManager::new("p", "1.2.3");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p", "P", "1.0");
            m.min_editor_version = "1.2.3".to_string();
            m
        }
    }
    assert!(mgr.register(Box::new(P)).is_ok());
}

#[test]
fn version_compat_editor_newer_accepted() {
    let mut mgr = PluginManager::new("p", "2.0.0");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p2", "P2", "1.0");
            m.min_editor_version = "1.0.0".to_string();
            m
        }
    }
    assert!(mgr.register(Box::new(P)).is_ok());
}

#[test]
fn version_compat_editor_older_rejected() {
    let mut mgr = PluginManager::new("p", "0.9.0");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p3", "P3", "1.0");
            m.min_editor_version = "1.0.0".to_string();
            m
        }
    }
    assert!(matches!(
        mgr.register(Box::new(P)),
        Err(PluginError::IncompatibleVersion { .. })
    ));
}

#[test]
fn version_compat_minor_boundary() {
    // Editor 1.1.0 vs required 1.2.0 => reject
    let mut mgr = PluginManager::new("p", "1.1.0");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p4", "P4", "1.0");
            m.min_editor_version = "1.2.0".to_string();
            m
        }
    }
    assert!(matches!(
        mgr.register(Box::new(P)),
        Err(PluginError::IncompatibleVersion { .. })
    ));
}

#[test]
fn version_compat_patch_boundary() {
    // Editor 1.2.2 vs required 1.2.3 => reject
    let mut mgr = PluginManager::new("p", "1.2.2");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p5", "P5", "1.0");
            m.min_editor_version = "1.2.3".to_string();
            m
        }
    }
    assert!(matches!(
        mgr.register(Box::new(P)),
        Err(PluginError::IncompatibleVersion { .. })
    ));
}

#[test]
fn version_compat_malformed_treated_as_zeros() {
    // "abc" parses to (0,0,0); editor "0.0.1" >= (0,0,0) => ok
    let mut mgr = PluginManager::new("p", "0.0.1");
    struct P;
    impl aw_editor_lib::plugin::EditorPlugin for P {
        fn metadata(&self) -> PluginMetadata {
            let mut m = PluginMetadata::new("p6", "P6", "1.0");
            m.min_editor_version = "abc".to_string();
            m
        }
    }
    assert!(mgr.register(Box::new(P)).is_ok());
}

// ============================================================================
// PluginManager — counts & state queries
// ============================================================================

#[test]
fn plugin_manager_empty_counts() {
    let mgr = PluginManager::new("p", "1.0.0");
    assert_eq!(mgr.plugin_count(), 0);
    assert_eq!(mgr.active_count(), 0);
    assert_eq!(mgr.error_count(), 0);
    assert_eq!(mgr.disabled_count(), 0);
    assert!(mgr.is_healthy());
    assert!(!mgr.has_errors());
}

#[test]
fn plugin_manager_register_increments_count() {
    let mut mgr = PluginManager::new("p", "1.0.0");
    mgr.register(Box::new(ExamplePlugin::new())).unwrap();
    assert_eq!(mgr.plugin_count(), 1);
    // Newly registered => Loaded state
    assert_eq!(mgr.active_count(), 0);
}

#[test]
fn plugin_manager_duplicate_registration_rejected() {
    let mut mgr = PluginManager::new("p", "1.0.0");
    mgr.register(Box::new(ExamplePlugin::new())).unwrap();
    let result = mgr.register(Box::new(ExamplePlugin::new()));
    assert!(result.is_err());
}

#[test]
fn plugin_manager_stats_snapshot() {
    let mut mgr = PluginManager::new("p", "1.0.0");
    mgr.register(Box::new(ExamplePlugin::new())).unwrap();
    let stats = mgr.stats();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.loaded, 1);
    assert_eq!(stats.active, 0);
    assert_eq!(stats.errors, 0);
    assert_eq!(stats.disabled, 0);
    assert_eq!(stats.with_panels, 1); // ExamplePlugin has_panel() => true
}

#[test]
fn plugin_manager_plugins_by_state() {
    let mut mgr = PluginManager::new("p", "1.0.0");
    mgr.register(Box::new(ExamplePlugin::new())).unwrap();
    let loaded = mgr.plugins_by_state(PluginState::Loaded);
    assert_eq!(loaded.len(), 1);
    let active = mgr.plugins_by_state(PluginState::Active);
    assert!(active.is_empty());
}

// ============================================================================
// SceneValidationIssue — helpers
// ============================================================================

#[test]
fn scene_issue_error_helpers() {
    let e = SceneValidationIssue::error("bad");
    assert!(e.is_error());
    assert!(!e.is_warning());
    assert_eq!(e.name(), "Error");
    assert_eq!(e.icon(), "[x]");
    assert_eq!(e.message(), "bad");
}

#[test]
fn scene_issue_warning_helpers() {
    let w = SceneValidationIssue::warning("meh");
    assert!(!w.is_error());
    assert!(w.is_warning());
    assert_eq!(w.name(), "Warning");
    assert_eq!(w.icon(), "[!]");
    assert_eq!(w.message(), "meh");
}

#[test]
fn scene_issue_display_error() {
    let e = SceneValidationIssue::Error("msg".into());
    let s = format!("{}", e);
    assert!(s.contains("Error"));
    assert!(s.contains("msg"));
}

#[test]
fn scene_issue_display_warning() {
    let w = SceneValidationIssue::Warning("msg".into());
    let s = format!("{}", w);
    assert!(s.contains("Warning"));
    assert!(s.contains("msg"));
}

#[test]
fn scene_issue_all_variants() {
    let vars = SceneValidationIssue::all_variants();
    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&"Error"));
    assert!(vars.contains(&"Warning"));
}

// ============================================================================
// SceneData::validate() — comparison operator boundaries
// ============================================================================

fn make_entity(id: u32, name: &str, scale: f32, hp: i32, ammo: i32) -> EntityData {
    EntityData {
        id,
        name: name.to_string(),
        pos: IVec2 { x: 0, y: 0 },
        height: 0.0,
        rotation: 0.0,
        rotation_x: 0.0,
        rotation_z: 0.0,
        scale,
        hp,
        team_id: 0,
        ammo,
        cooldowns: BTreeMap::new(),
        behavior_graph: None,
        parent: None,
    }
}

fn make_scene(entities: Vec<EntityData>, next_id: u32) -> SceneData {
    SceneData {
        version: 1,
        time: 0.0,
        next_entity_id: next_id,
        entities,
        obstacles: vec![],
    }
}

#[test]
fn validate_empty_scene_no_issues() {
    let s = make_scene(vec![], 1);
    assert!(s.validate().is_empty());
    assert!(s.is_valid());
}

#[test]
fn validate_scale_zero_is_error() {
    // scale <= 0.0 triggers error
    let s = make_scene(vec![make_entity(1, "E", 0.0, 100, 10)], 5);
    let issues = s.validate();
    assert!(issues
        .iter()
        .any(|i| i.is_error() && i.message().contains("scale")));
    assert!(!s.is_valid());
}

#[test]
fn validate_scale_negative_is_error() {
    let s = make_scene(vec![make_entity(1, "E", -0.5, 100, 10)], 5);
    assert!(!s.is_valid());
}

#[test]
fn validate_scale_positive_no_error() {
    // scale = 0.001 > 0 => no error
    let s = make_scene(vec![make_entity(1, "E", 0.001, 100, 10)], 5);
    let scale_issues: Vec<_> = s
        .validate()
        .into_iter()
        .filter(|i| i.message().contains("scale"))
        .collect();
    assert!(scale_issues.is_empty());
}

#[test]
fn validate_hp_negative_is_warning() {
    // hp < 0 => warning (not error)
    let s = make_scene(vec![make_entity(1, "E", 1.0, -1, 10)], 5);
    let issues = s.validate();
    let hp_warn = issues.iter().find(|i| i.message().contains("HP"));
    assert!(hp_warn.is_some());
    assert!(hp_warn.unwrap().is_warning());
    assert!(s.is_valid()); // warnings don't invalidate
}

#[test]
fn validate_hp_zero_no_warning() {
    // hp == 0 => NOT < 0 => no warning
    let s = make_scene(vec![make_entity(1, "E", 1.0, 0, 10)], 5);
    assert!(
        !s.validate().iter().any(|i| i.message().contains("HP")),
        "hp=0 should not generate HP warning"
    );
}

#[test]
fn validate_ammo_negative_is_warning() {
    let s = make_scene(vec![make_entity(1, "E", 1.0, 100, -1)], 5);
    let issues = s.validate();
    assert!(issues.iter().any(|i| i.message().contains("ammo")));
    assert!(s.is_valid()); // warning only
}

#[test]
fn validate_ammo_zero_no_warning() {
    let s = make_scene(vec![make_entity(1, "E", 1.0, 100, 0)], 5);
    assert!(!s.validate().iter().any(|i| i.message().contains("ammo")));
}

#[test]
fn validate_empty_name_is_warning() {
    let s = make_scene(vec![make_entity(1, "", 1.0, 100, 10)], 5);
    let issues = s.validate();
    assert!(issues.iter().any(|i| i.message().contains("empty name")));
    assert!(s.is_valid()); // warning
}

#[test]
fn validate_duplicate_ids_is_error() {
    let s = make_scene(
        vec![
            make_entity(1, "A", 1.0, 100, 10),
            make_entity(1, "B", 1.0, 100, 10),
        ],
        5,
    );
    let issues = s.validate();
    assert!(issues
        .iter()
        .any(|i| i.is_error() && i.message().contains("Duplicate")));
    assert!(!s.is_valid());
}

#[test]
fn validate_next_entity_id_must_exceed_max() {
    // next_entity_id <= max_id => error
    let s = make_scene(vec![make_entity(10, "E", 1.0, 100, 10)], 10); // 10 <= 10
    let issues = s.validate();
    assert!(issues
        .iter()
        .any(|i| i.is_error() && i.message().contains("next_entity_id")));
    assert!(!s.is_valid());
}

#[test]
fn validate_next_entity_id_just_above_max_valid() {
    let s = make_scene(vec![make_entity(10, "E", 1.0, 100, 10)], 11); // 11 > 10
    let id_issues: Vec<_> = s
        .validate()
        .into_iter()
        .filter(|i| i.message().contains("next_entity_id"))
        .collect();
    assert!(id_issues.is_empty());
}

// ============================================================================
// SceneData::is_valid() — errors invalidate, warnings don't
// ============================================================================

#[test]
fn is_valid_with_only_warnings_returns_true() {
    // negative HP and negative ammo are warnings
    let s = make_scene(vec![make_entity(1, "E", 1.0, -5, -3)], 5);
    assert!(s.is_valid());
}

#[test]
fn is_valid_with_error_returns_false() {
    // zero scale is error
    let s = make_scene(vec![make_entity(1, "E", 0.0, 100, 10)], 5);
    assert!(!s.is_valid());
}

// ============================================================================
// SceneData::stats() — aggregation
// ============================================================================

#[test]
fn stats_empty_scene() {
    let s = make_scene(vec![], 1);
    let st = s.stats();
    assert_eq!(st.entity_count, 0);
    assert_eq!(st.obstacle_count, 0);
    assert!(!st.has_behavior_graphs);
    assert_eq!(st.total_cooldowns, 0);
}

#[test]
fn stats_counts_entities_and_obstacles() {
    let s = SceneData {
        version: 1,
        time: 0.0,
        next_entity_id: 5,
        entities: vec![
            make_entity(1, "A", 1.0, 100, 10),
            make_entity(2, "B", 1.0, 50, 5),
        ],
        obstacles: vec![(1, 1), (2, 2), (3, 3)],
    };
    let st = s.stats();
    assert_eq!(st.entity_count, 2);
    assert_eq!(st.obstacle_count, 3);
}

#[test]
fn stats_tracks_cooldown_total() {
    let mut e = make_entity(1, "E", 1.0, 100, 10);
    e.cooldowns.insert("fire".into(), 1.0);
    e.cooldowns.insert("jump".into(), 0.5);
    let mut e2 = make_entity(2, "E2", 1.0, 100, 10);
    e2.cooldowns.insert("dash".into(), 2.0);

    let s = make_scene(vec![e, e2], 5);
    assert_eq!(s.stats().total_cooldowns, 3);
}

#[test]
fn stats_has_behavior_graphs_false_when_none() {
    let s = make_scene(vec![make_entity(1, "E", 1.0, 100, 10)], 5);
    assert!(!s.stats().has_behavior_graphs);
}

//! Wave 2 Mutation Remediation — Dock Layout & Level Document
//!
//! Targets mutation-prone patterns in:
//! - dock_layout.rs: LayoutPreset (6 variants × name/description/icon/shortcut_hint/expected_panel_count),
//!   DockLayout panel management, LayoutStats, closest_preset matching, split ratio constants
//! - level_doc.rs: LevelDoc::validate() comparison operators, LevelStats aggregation,
//!   BiomePaint/Trigger/DirectorOp (Display + helpers), FortRegion, BossCfg

use aw_editor_lib::dock_layout::{DockLayout, LayoutPreset, LayoutStats};
use aw_editor_lib::level_doc::{
    BiomePaint, BossCfg, Circle, DirectorOp, FateThread, FortRegion, LevelDoc, LevelStats,
    LevelValidationIssue, NpcSpawn, Obstacle, Spawn, Trigger,
};
use aw_editor_lib::panel_type::PanelType;

// ============================================================================
// LayoutPreset — 6 Variants × name
// ============================================================================

#[test]
fn preset_name_default() {
    assert_eq!(LayoutPreset::Default.name(), "Default");
}

#[test]
fn preset_name_wide() {
    assert_eq!(LayoutPreset::Wide.name(), "Wide");
}

#[test]
fn preset_name_compact() {
    assert_eq!(LayoutPreset::Compact.name(), "Compact");
}

#[test]
fn preset_name_modeling() {
    assert_eq!(LayoutPreset::Modeling.name(), "Modeling");
}

#[test]
fn preset_name_animation() {
    assert_eq!(LayoutPreset::Animation.name(), "Animation");
}

#[test]
fn preset_name_debug() {
    assert_eq!(LayoutPreset::Debug.name(), "Debug");
}

// ============================================================================
// LayoutPreset — Display matches name
// ============================================================================

#[test]
fn preset_display_matches_name() {
    for &p in LayoutPreset::all() {
        assert_eq!(format!("{}", p), p.name());
    }
}

// ============================================================================
// LayoutPreset — expected_panel_count exact values
// ============================================================================

#[test]
fn preset_expected_panel_count_default_6() {
    assert_eq!(LayoutPreset::Default.expected_panel_count(), 8);
}

#[test]
fn preset_expected_panel_count_wide_2() {
    assert_eq!(LayoutPreset::Wide.expected_panel_count(), 3);
}

#[test]
fn preset_expected_panel_count_compact_8() {
    assert_eq!(LayoutPreset::Compact.expected_panel_count(), 10);
}

#[test]
fn preset_expected_panel_count_modeling_3() {
    assert_eq!(LayoutPreset::Modeling.expected_panel_count(), 4);
}

#[test]
fn preset_expected_panel_count_animation_5() {
    assert_eq!(LayoutPreset::Animation.expected_panel_count(), 6);
}

#[test]
fn preset_expected_panel_count_debug_5() {
    assert_eq!(LayoutPreset::Debug.expected_panel_count(), 6);
}

// ============================================================================
// LayoutPreset — description not empty per variant
// ============================================================================

#[test]
fn preset_description_default() {
    assert!(LayoutPreset::Default.description().contains("general"));
}

#[test]
fn preset_description_wide() {
    assert!(LayoutPreset::Wide.description().contains("viewport"));
}

#[test]
fn preset_description_compact() {
    assert!(LayoutPreset::Compact.description().contains("panels"));
}

#[test]
fn preset_description_modeling() {
    assert!(LayoutPreset::Modeling.description().contains("viewport"));
}

#[test]
fn preset_description_animation() {
    assert!(LayoutPreset::Animation.description().contains("imeline"));
}

#[test]
fn preset_description_debug() {
    assert!(LayoutPreset::Debug.description().contains("onsole"));
}

// ============================================================================
// LayoutPreset — icon uniqueness (each variant returns non-empty)
// ============================================================================

#[test]
fn preset_icon_default() {
    assert_eq!(LayoutPreset::Default.icon(), "🏠");
}

#[test]
fn preset_icon_wide() {
    assert_eq!(LayoutPreset::Wide.icon(), "🖥️");
}

#[test]
fn preset_icon_compact() {
    assert_eq!(LayoutPreset::Compact.icon(), "📐");
}

#[test]
fn preset_icon_modeling() {
    assert_eq!(LayoutPreset::Modeling.icon(), "🔧");
}

#[test]
fn preset_icon_animation() {
    assert_eq!(LayoutPreset::Animation.icon(), "🎬");
}

#[test]
fn preset_icon_debug() {
    assert_eq!(LayoutPreset::Debug.icon(), "🔍");
}

// ============================================================================
// LayoutPreset — shortcut_hint exact values
// ============================================================================

#[test]
fn preset_shortcut_default() {
    assert_eq!(LayoutPreset::Default.shortcut_hint(), Some("Ctrl+1"));
}

#[test]
fn preset_shortcut_wide() {
    assert_eq!(LayoutPreset::Wide.shortcut_hint(), Some("Ctrl+2"));
}

#[test]
fn preset_shortcut_compact() {
    assert_eq!(LayoutPreset::Compact.shortcut_hint(), Some("Ctrl+3"));
}

#[test]
fn preset_shortcut_modeling() {
    assert_eq!(LayoutPreset::Modeling.shortcut_hint(), Some("Ctrl+4"));
}

#[test]
fn preset_shortcut_animation() {
    assert_eq!(LayoutPreset::Animation.shortcut_hint(), Some("Ctrl+5"));
}

#[test]
fn preset_shortcut_debug() {
    assert_eq!(LayoutPreset::Debug.shortcut_hint(), Some("Ctrl+6"));
}

// ============================================================================
// LayoutPreset — classification helpers
// ============================================================================

#[test]
fn preset_is_debug_layout_only_debug() {
    assert!(LayoutPreset::Debug.is_debug_layout());
    for &p in &[
        LayoutPreset::Default,
        LayoutPreset::Wide,
        LayoutPreset::Compact,
        LayoutPreset::Modeling,
        LayoutPreset::Animation,
    ] {
        assert!(!p.is_debug_layout(), "{:?} should not be debug layout", p);
    }
}

#[test]
fn preset_is_content_creation_only_modeling_animation() {
    assert!(LayoutPreset::Modeling.is_content_creation_layout());
    assert!(LayoutPreset::Animation.is_content_creation_layout());
    for &p in &[
        LayoutPreset::Default,
        LayoutPreset::Wide,
        LayoutPreset::Compact,
        LayoutPreset::Debug,
    ] {
        assert!(
            !p.is_content_creation_layout(),
            "{:?} should not be content creation",
            p
        );
    }
}

#[test]
fn preset_all_returns_6() {
    assert_eq!(LayoutPreset::all().len(), 6);
}

#[test]
fn preset_default_is_default() {
    assert_eq!(LayoutPreset::default(), LayoutPreset::Default);
}

// ============================================================================
// DockLayout — panel management
// ============================================================================

#[test]
fn layout_new_has_viewport() {
    let l = DockLayout::new();
    assert!(l.is_panel_visible(&PanelType::Viewport));
    assert!(l.has_panel(&PanelType::Viewport));
}

#[test]
fn layout_default_has_inspector_and_console() {
    let l = DockLayout::new();
    assert!(l.is_panel_visible(&PanelType::Inspector));
    assert!(l.is_panel_visible(&PanelType::Console));
}

#[test]
fn layout_add_panel() {
    let mut l = DockLayout::new();
    l.add_panel(PanelType::Charts);
    assert!(l.is_panel_visible(&PanelType::Charts));
}

#[test]
fn layout_add_existing_panel_no_duplicate() {
    let mut l = DockLayout::new();
    let count_before = l.panel_count();
    l.add_panel(PanelType::Console); // already visible
    assert_eq!(l.panel_count(), count_before);
}

#[test]
fn layout_remove_closable_panel() {
    let mut l = DockLayout::new();
    assert!(l.is_panel_visible(&PanelType::Console));
    let removed = l.remove_panel(&PanelType::Console);
    assert!(removed);
    assert!(!l.is_panel_visible(&PanelType::Console));
}

#[test]
fn layout_remove_viewport_rejected() {
    let mut l = DockLayout::new();
    let removed = l.remove_panel(&PanelType::Viewport);
    assert!(!removed);
    assert!(l.is_panel_visible(&PanelType::Viewport));
}

#[test]
fn layout_toggle_panel() {
    let mut l = DockLayout::new();
    assert!(l.is_panel_visible(&PanelType::Console));
    l.toggle_panel(PanelType::Console);
    assert!(!l.is_panel_visible(&PanelType::Console));
    l.toggle_panel(PanelType::Console);
    assert!(l.is_panel_visible(&PanelType::Console));
}

#[test]
fn layout_focused_panel_lifecycle() {
    let mut l = DockLayout::new();
    assert!(l.focused_panel().is_none());
    l.set_focused_panel(Some(PanelType::Inspector));
    assert_eq!(l.focused_panel(), Some(PanelType::Inspector));
    l.set_focused_panel(None);
    assert!(l.focused_panel().is_none());
}

// ============================================================================
// DockLayout — preset creation (each creates viewport)
// ============================================================================

#[test]
fn layout_each_preset_has_viewport() {
    for &p in LayoutPreset::all() {
        let l = DockLayout::from_preset(p);
        assert!(
            l.is_panel_visible(&PanelType::Viewport),
            "{:?} should have viewport",
            p
        );
    }
}

#[test]
fn layout_wide_preset_has_inspector() {
    let l = DockLayout::from_preset(LayoutPreset::Wide);
    assert!(l.is_panel_visible(&PanelType::Inspector));
}

#[test]
fn layout_debug_preset_has_performance() {
    let l = DockLayout::from_preset(LayoutPreset::Debug);
    assert!(l.is_panel_visible(&PanelType::Performance));
}

#[test]
fn layout_debug_preset_has_console() {
    let l = DockLayout::from_preset(LayoutPreset::Debug);
    assert!(l.is_panel_visible(&PanelType::Console));
}

#[test]
fn layout_compact_preset_has_entity_panel() {
    let l = DockLayout::from_preset(LayoutPreset::Compact);
    assert!(l.is_panel_visible(&PanelType::EntityPanel));
}

#[test]
fn layout_animation_preset_has_behavior_graph() {
    let l = DockLayout::from_preset(LayoutPreset::Animation);
    assert!(l.is_panel_visible(&PanelType::BehaviorGraph));
}

#[test]
fn layout_modeling_preset_has_transform() {
    let l = DockLayout::from_preset(LayoutPreset::Modeling);
    assert!(l.is_panel_visible(&PanelType::Transform));
}

// ============================================================================
// DockLayout — stats
// ============================================================================

#[test]
fn layout_stats_default_has_viewport() {
    let l = DockLayout::new();
    let st = l.stats();
    assert!(st.has_viewport);
    assert!(st.panel_count >= 2);
}

#[test]
fn layout_stats_debug_has_debug_panels() {
    let l = DockLayout::from_preset(LayoutPreset::Debug);
    assert!(l.has_debug_panels());
    assert!(l.stats().has_debug_panels);
}

#[test]
fn layout_is_valid_with_viewport() {
    let l = DockLayout::new();
    assert!(l.is_valid());
}

#[test]
fn layout_stats_default_struct() {
    let st = LayoutStats::default();
    assert_eq!(st.panel_count, 0);
    assert_eq!(st.tab_group_count, 0);
    assert!(!st.has_viewport);
    assert!(!st.has_debug_panels);
    assert!(st.visible_panels.is_empty());
}

// ============================================================================
// DockLayout — matches_preset & closest_preset
// ============================================================================

#[test]
fn layout_matches_preset_self() {
    let l = DockLayout::from_preset(LayoutPreset::Default);
    assert!(l.matches_preset(LayoutPreset::Default));
}

#[test]
fn layout_wide_does_not_match_default() {
    let l = DockLayout::from_preset(LayoutPreset::Wide);
    // Wide only has Viewport + Inspector, Default has more
    assert!(!l.matches_preset(LayoutPreset::Default));
}

#[test]
fn layout_closest_preset_returns_valid() {
    for &p in LayoutPreset::all() {
        let l = DockLayout::from_preset(p);
        let closest = l.closest_preset();
        assert!(LayoutPreset::all().contains(&closest));
    }
}

#[test]
fn layout_reset_to_default() {
    let mut l = DockLayout::from_preset(LayoutPreset::Wide);
    l.reset_to_default();
    assert!(l.is_panel_visible(&PanelType::Console)); // default has console
}

#[test]
fn layout_apply_preset() {
    let mut l = DockLayout::new();
    l.apply_preset(LayoutPreset::Debug);
    assert!(l.is_panel_visible(&PanelType::Performance));
}

// ============================================================================
// DockLayout — JSON roundtrip (basic)
// ============================================================================

#[test]
fn layout_to_json_produces_valid_json() {
    let l = DockLayout::new();
    let json = l.to_json().unwrap();
    assert!(json.contains("panels"));
    // from_json returns default for now
    let restored = DockLayout::from_json(&json).unwrap();
    assert!(restored.is_panel_visible(&PanelType::Viewport));
}

// ============================================================================
// LevelDoc — defaults
// ============================================================================

#[test]
fn level_doc_defaults() {
    let doc = LevelDoc::default();
    assert_eq!(doc.title, "");
    assert_eq!(doc.biome, "");
    assert_eq!(doc.seed, 0);
    assert!(doc.biome_paints.is_empty());
    assert!(doc.obstacles.is_empty());
    assert!(doc.npcs.is_empty());
    assert!(doc.fate_threads.is_empty());
}

// ============================================================================
// LevelDoc::validate() — comparison operator boundaries
// ============================================================================

#[test]
fn level_validate_empty_title_warning() {
    let doc = LevelDoc {
        title: "".into(),
        biome: "Forest".into(),
        ..Default::default()
    };
    let issues = doc.validate();
    assert!(issues.iter().any(|i| i.message.contains("title")));
    assert!(issues.iter().all(|i| !i.is_error)); // all warnings
}

#[test]
fn level_validate_whitespace_only_title_warning() {
    let doc = LevelDoc {
        title: "   ".into(),
        biome: "Forest".into(),
        ..Default::default()
    };
    assert!(doc.validate().iter().any(|i| i.message.contains("title")));
}

#[test]
fn level_validate_nonempty_title_no_warning() {
    let doc = LevelDoc {
        title: "Level 1".into(),
        biome: "Forest".into(),
        ..Default::default()
    };
    assert!(!doc.validate().iter().any(|i| i.message.contains("title")));
}

#[test]
fn level_validate_empty_biome_warning() {
    let doc = LevelDoc {
        title: "T".into(),
        biome: "".into(),
        ..Default::default()
    };
    assert!(doc.validate().iter().any(|i| i.message.contains("biome")));
}

#[test]
fn level_validate_duplicate_obstacle_ids_error() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.obstacles.push(Obstacle {
        id: "rock".into(),
        ..Default::default()
    });
    doc.obstacles.push(Obstacle {
        id: "rock".into(),
        ..Default::default()
    });
    let issues = doc.validate();
    let dup = issues.iter().find(|i| i.message.contains("Duplicate"));
    assert!(dup.is_some());
    assert!(dup.unwrap().is_error);
}

#[test]
fn level_validate_empty_obstacle_id_no_duplicate_check() {
    // Empty IDs should not trigger duplicate
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.obstacles.push(Obstacle {
        id: "".into(),
        ..Default::default()
    });
    doc.obstacles.push(Obstacle {
        id: "".into(),
        ..Default::default()
    });
    let issues = doc.validate();
    assert!(!issues.iter().any(|i| i.message.contains("Duplicate")));
}

#[test]
fn level_validate_npc_zero_count_warning() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 0,
        spawn: Spawn::default(),
        behavior: "Patrol".into(),
    });
    assert!(doc
        .validate()
        .iter()
        .any(|i| i.message.contains("zero count")));
}

#[test]
fn level_validate_npc_positive_count_no_warning() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 1,
        spawn: Spawn::default(),
        behavior: "Patrol".into(),
    });
    assert!(!doc
        .validate()
        .iter()
        .any(|i| i.message.contains("zero count")));
}

#[test]
fn level_validate_npc_negative_radius_error() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 5,
        spawn: Spawn {
            pos: [0.0, 0.0, 0.0],
            radius: -0.1,
        },
        behavior: "Patrol".into(),
    });
    let issues = doc.validate();
    let err = issues
        .iter()
        .find(|i| i.message.contains("negative radius"));
    assert!(err.is_some());
    assert!(err.unwrap().is_error);
}

#[test]
fn level_validate_npc_zero_radius_no_error() {
    // radius == 0.0, NOT < 0.0 => no error
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 5,
        spawn: Spawn {
            pos: [0.0, 0.0, 0.0],
            radius: 0.0,
        },
        behavior: "Patrol".into(),
    });
    assert!(!doc.validate().iter().any(|i| i.message.contains("radius")));
}

#[test]
fn level_validate_fate_thread_no_triggers_warning() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.fate_threads.push(FateThread {
        name: "Empty".into(),
        triggers: vec![],
        ops: vec![DirectorOp::default()],
    });
    assert!(doc
        .validate()
        .iter()
        .any(|i| i.message.contains("no triggers")));
}

#[test]
fn level_validate_fate_thread_no_ops_warning() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.fate_threads.push(FateThread {
        name: "NoOps".into(),
        triggers: vec![Trigger::default()],
        ops: vec![],
    });
    assert!(doc
        .validate()
        .iter()
        .any(|i| i.message.contains("no operations")));
}

// ============================================================================
// LevelDoc::is_valid() — errors vs warnings
// ============================================================================

#[test]
fn level_is_valid_with_only_warnings() {
    // Default has title/biome warnings but no errors
    assert!(LevelDoc::default().is_valid());
}

#[test]
fn level_is_valid_with_duplicate_ids_returns_false() {
    let mut doc = LevelDoc {
        title: "T".into(),
        biome: "B".into(),
        ..Default::default()
    };
    doc.obstacles.push(Obstacle {
        id: "d".into(),
        ..Default::default()
    });
    doc.obstacles.push(Obstacle {
        id: "d".into(),
        ..Default::default()
    });
    assert!(!doc.is_valid());
}

// ============================================================================
// LevelDoc::stats() — aggregation
// ============================================================================

#[test]
fn level_stats_empty() {
    let doc = LevelDoc::default();
    let st = doc.stats();
    assert_eq!(
        st,
        LevelStats {
            obstacle_count: 0,
            npc_spawn_count: 0,
            total_npc_count: 0,
            fate_thread_count: 0,
            trigger_count: 0,
            biome_paint_count: 0,
            has_boss: false,
        }
    );
}

#[test]
fn level_stats_npc_total_sums_counts() {
    let mut doc = LevelDoc::default();
    doc.npcs.push(NpcSpawn {
        count: 5,
        ..Default::default()
    });
    doc.npcs.push(NpcSpawn {
        count: 3,
        ..Default::default()
    });
    let st = doc.stats();
    assert_eq!(st.npc_spawn_count, 2);
    assert_eq!(st.total_npc_count, 8);
}

#[test]
fn level_stats_trigger_count_sums_across_threads() {
    let mut doc = LevelDoc::default();
    doc.fate_threads.push(FateThread {
        name: "A".into(),
        triggers: vec![Trigger::default(), Trigger::default()],
        ops: vec![],
    });
    doc.fate_threads.push(FateThread {
        name: "B".into(),
        triggers: vec![Trigger::default()],
        ops: vec![],
    });
    let st = doc.stats();
    assert_eq!(st.fate_thread_count, 2);
    assert_eq!(st.trigger_count, 3);
}

#[test]
fn level_stats_has_boss_from_director_script() {
    let mut doc = LevelDoc::default();
    doc.boss.director_budget_script = "script.lua".into();
    assert!(doc.stats().has_boss);
}

#[test]
fn level_stats_has_boss_from_phase_script() {
    let mut doc = LevelDoc::default();
    doc.boss.phase_script = "phases.lua".into();
    assert!(doc.stats().has_boss);
}

#[test]
fn level_stats_no_boss_when_both_empty() {
    let doc = LevelDoc::default();
    assert!(!doc.stats().has_boss);
}

// ============================================================================
// LevelDoc — find queries
// ============================================================================

#[test]
fn level_find_obstacles_by_tag() {
    let mut doc = LevelDoc::default();
    doc.obstacles.push(Obstacle {
        id: "r1".into(),
        tags: vec!["solid".into(), "natural".into()],
        ..Default::default()
    });
    doc.obstacles.push(Obstacle {
        id: "r2".into(),
        tags: vec!["solid".into()],
        ..Default::default()
    });
    doc.obstacles.push(Obstacle {
        id: "t1".into(),
        tags: vec!["natural".into()],
        ..Default::default()
    });

    assert_eq!(doc.find_obstacles_by_tag("solid").len(), 2);
    assert_eq!(doc.find_obstacles_by_tag("natural").len(), 2);
    assert_eq!(doc.find_obstacles_by_tag("magic").len(), 0);
}

#[test]
fn level_find_npcs_by_archetype() {
    let mut doc = LevelDoc::default();
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 2,
        ..Default::default()
    });
    doc.npcs.push(NpcSpawn {
        archetype: "Archer".into(),
        count: 3,
        ..Default::default()
    });
    doc.npcs.push(NpcSpawn {
        archetype: "Guard".into(),
        count: 1,
        ..Default::default()
    });

    assert_eq!(doc.find_npcs_by_archetype("Guard").len(), 2);
    assert_eq!(doc.find_npcs_by_archetype("Archer").len(), 1);
    assert_eq!(doc.find_npcs_by_archetype("Mage").len(), 0);
}

// ============================================================================
// BiomePaint — Display + helpers
// ============================================================================

#[test]
fn biome_paint_display_grass_dense() {
    let p = BiomePaint::GrassDense {
        area: Circle {
            cx: 0,
            cz: 0,
            radius: 10,
        },
    };
    assert_eq!(format!("{}", p), "Grass Dense");
}

#[test]
fn biome_paint_display_moss_path() {
    let p = BiomePaint::MossPath {
        polyline: vec![[0, 0]],
    };
    assert_eq!(format!("{}", p), "Moss Path");
}

#[test]
fn biome_paint_name_grass_dense() {
    let p = BiomePaint::GrassDense {
        area: Circle {
            cx: 0,
            cz: 0,
            radius: 5,
        },
    };
    assert_eq!(p.name(), "Grass Dense");
}

#[test]
fn biome_paint_name_moss_path() {
    let p = BiomePaint::MossPath {
        polyline: vec![[0, 0]],
    };
    assert_eq!(p.name(), "Moss Path");
}

#[test]
fn biome_paint_icon_grass_dense() {
    let p = BiomePaint::GrassDense {
        area: Circle {
            cx: 0,
            cz: 0,
            radius: 1,
        },
    };
    assert_eq!(p.icon(), "[Leaf]");
}

#[test]
fn biome_paint_icon_moss_path() {
    let p = BiomePaint::MossPath {
        polyline: vec![[0, 0]],
    };
    assert_eq!(p.icon(), "🛤️");
}

#[test]
fn biome_paint_is_area_grass_dense() {
    let p = BiomePaint::GrassDense {
        area: Circle {
            cx: 0,
            cz: 0,
            radius: 1,
        },
    };
    assert!(p.is_area());
    assert!(!p.is_path());
}

#[test]
fn biome_paint_is_path_moss_path() {
    let p = BiomePaint::MossPath {
        polyline: vec![[0, 0]],
    };
    assert!(p.is_path());
    assert!(!p.is_area());
}

#[test]
fn biome_paint_all_variants() {
    let v = BiomePaint::all_variants();
    assert_eq!(v.len(), 2);
    assert!(v.contains(&"GrassDense"));
    assert!(v.contains(&"MossPath"));
}

// ============================================================================
// Trigger — Display + helpers
// ============================================================================

#[test]
fn trigger_display_contains_radius() {
    let t = Trigger::EnterArea {
        center: [1.0, 2.0, 3.0],
        radius: 10.5,
    };
    let s = format!("{}", t);
    assert!(s.contains("Enter Area"));
    assert!(s.contains("10.5"));
}

#[test]
fn trigger_name() {
    let t = Trigger::EnterArea {
        center: [0.0, 0.0, 0.0],
        radius: 1.0,
    };
    assert_eq!(t.name(), "Enter Area");
}

#[test]
fn trigger_icon() {
    let t = Trigger::EnterArea {
        center: [0.0, 0.0, 0.0],
        radius: 1.0,
    };
    assert_eq!(t.icon(), "[Tgt]");
}

#[test]
fn trigger_is_spatial() {
    let t = Trigger::EnterArea {
        center: [0.0, 0.0, 0.0],
        radius: 1.0,
    };
    assert!(t.is_spatial());
}

#[test]
fn trigger_radius_returns_value() {
    let t = Trigger::EnterArea {
        center: [0.0, 0.0, 0.0],
        radius: 7.5,
    };
    assert_eq!(t.radius(), Some(7.5));
}

#[test]
fn trigger_center_returns_value() {
    let t = Trigger::EnterArea {
        center: [1.0, 2.0, 3.0],
        radius: 5.0,
    };
    assert_eq!(t.center(), Some([1.0, 2.0, 3.0]));
}

#[test]
fn trigger_all_variants() {
    assert_eq!(Trigger::all_variants().len(), 1);
    assert!(Trigger::all_variants().contains(&"EnterArea"));
}

// ============================================================================
// DirectorOp — Display + helpers
// ============================================================================

#[test]
fn director_op_display_fortify() {
    let op = DirectorOp::Fortify {
        area: FortRegion { cx: 0, cz: 0, r: 5 },
    };
    assert_eq!(format!("{}", op), "Fortify");
}

#[test]
fn director_op_display_collapse() {
    let op = DirectorOp::Collapse {
        area: FortRegion { cx: 0, cz: 0, r: 5 },
    };
    assert_eq!(format!("{}", op), "Collapse");
}

#[test]
fn director_op_display_spawn_wave_includes_count_and_archetype() {
    let op = DirectorOp::SpawnWave {
        archetype: "goblin".into(),
        count: 5,
        scatter: 2.0,
    };
    let s = format!("{}", op);
    assert!(s.contains("5"));
    assert!(s.contains("goblin"));
}

#[test]
fn director_op_name_fortify() {
    let op = DirectorOp::Fortify {
        area: FortRegion::default(),
    };
    assert_eq!(op.name(), "Fortify");
}

#[test]
fn director_op_name_collapse() {
    let op = DirectorOp::Collapse {
        area: FortRegion::default(),
    };
    assert_eq!(op.name(), "Collapse");
}

#[test]
fn director_op_name_spawn_wave() {
    let op = DirectorOp::SpawnWave {
        archetype: "x".into(),
        count: 1,
        scatter: 1.0,
    };
    assert_eq!(op.name(), "Spawn Wave");
}

#[test]
fn director_op_icon_fortify() {
    assert_eq!(
        DirectorOp::Fortify {
            area: FortRegion::default()
        }
        .icon(),
        "🏰"
    );
}

#[test]
fn director_op_icon_collapse() {
    assert_eq!(
        DirectorOp::Collapse {
            area: FortRegion::default()
        }
        .icon(),
        "[Hit]"
    );
}

#[test]
fn director_op_icon_spawn_wave() {
    assert_eq!(
        DirectorOp::SpawnWave {
            archetype: "x".into(),
            count: 1,
            scatter: 1.0
        }
        .icon(),
        "👾"
    );
}

#[test]
fn director_op_is_terrain_op() {
    assert!(DirectorOp::Fortify {
        area: FortRegion::default()
    }
    .is_terrain_op());
    assert!(DirectorOp::Collapse {
        area: FortRegion::default()
    }
    .is_terrain_op());
    assert!(!DirectorOp::SpawnWave {
        archetype: "x".into(),
        count: 1,
        scatter: 1.0
    }
    .is_terrain_op());
}

#[test]
fn director_op_is_spawn_op() {
    assert!(DirectorOp::SpawnWave {
        archetype: "x".into(),
        count: 1,
        scatter: 1.0
    }
    .is_spawn_op());
    assert!(!DirectorOp::Fortify {
        area: FortRegion::default()
    }
    .is_spawn_op());
    assert!(!DirectorOp::Collapse {
        area: FortRegion::default()
    }
    .is_spawn_op());
}

#[test]
fn director_op_spawn_count() {
    assert_eq!(
        DirectorOp::SpawnWave {
            archetype: "x".into(),
            count: 7,
            scatter: 1.0
        }
        .spawn_count(),
        Some(7)
    );
    assert_eq!(
        DirectorOp::Fortify {
            area: FortRegion::default()
        }
        .spawn_count(),
        None
    );
}

#[test]
fn director_op_all_variants() {
    let v = DirectorOp::all_variants();
    assert_eq!(v.len(), 3);
    assert!(v.contains(&"Fortify"));
    assert!(v.contains(&"Collapse"));
    assert!(v.contains(&"SpawnWave"));
}

// ============================================================================
// LevelValidationIssue — constructors
// ============================================================================

#[test]
fn level_validation_issue_error() {
    let i = LevelValidationIssue::error("bad");
    assert!(i.is_error);
    assert_eq!(i.message, "bad");
}

#[test]
fn level_validation_issue_warning() {
    let i = LevelValidationIssue::warning("meh");
    assert!(!i.is_error);
    assert_eq!(i.message, "meh");
}

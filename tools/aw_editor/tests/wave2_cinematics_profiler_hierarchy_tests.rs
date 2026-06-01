//! Wave 2 Mutation Remediation — Cinematics + Profiler + Hierarchy per-variant tests
//!
//! Targets 3 more sparse-coverage panels:
//! - cinematics_panel.rs (1,739L, 64 inline = 2.7/100L)
//! - profiler_panel.rs (1,075L, 20 inline = 1.9/100L)
//! - hierarchy_panel.rs (768L, 17 inline = 2.2/100L)

use aw_editor_lib::panels::cinematics_panel::{
    CameraInterpolation, CinematicsPanel, CinematicsTab, ClipData, PlaybackSpeed, PlaybackState,
    TimelineSettings, TrackEntry, TrackType,
};
use aw_editor_lib::panels::hierarchy_panel::{HierarchyAction, HierarchyPanel};
use aw_editor_lib::panels::profiler_panel::{
    FlameNode, GpuMetrics, ProfilerPanel, SubsystemTimings,
};

// ============================================================================
// TRACK TYPE
// ============================================================================

#[test]
fn track_type_all_count() {
    assert_eq!(TrackType::all().len(), 6);
}

#[test]
fn track_type_default_camera() {
    assert_eq!(TrackType::default(), TrackType::Camera);
}

#[test]
fn track_type_names() {
    assert_eq!(TrackType::Camera.name(), "Camera");
    assert_eq!(TrackType::Animation.name(), "Animation");
    assert_eq!(TrackType::Audio.name(), "Audio");
    assert_eq!(TrackType::Fx.name(), "VFX");
    assert_eq!(TrackType::Dialogue.name(), "Dialogue");
    assert_eq!(TrackType::Event.name(), "Event");
}

#[test]
fn track_type_icons_nonempty() {
    for t in TrackType::all() {
        assert!(!t.icon().is_empty());
    }
}

#[test]
fn track_type_display_contains_name() {
    for t in TrackType::all() {
        assert!(format!("{}", t).contains(t.name()));
    }
}

// ============================================================================
// CAMERA INTERPOLATION
// ============================================================================

#[test]
fn camera_interp_all_count() {
    assert_eq!(CameraInterpolation::all().len(), 5);
}

#[test]
fn camera_interp_default_linear() {
    assert_eq!(CameraInterpolation::default(), CameraInterpolation::Linear);
}

#[test]
fn camera_interp_names() {
    assert_eq!(CameraInterpolation::Linear.name(), "Linear");
    assert_eq!(CameraInterpolation::CatmullRom.name(), "Catmull-Rom");
    assert_eq!(CameraInterpolation::Bezier.name(), "Bezier");
    assert_eq!(CameraInterpolation::Hermite.name(), "Hermite");
    assert_eq!(CameraInterpolation::Step.name(), "Step");
}

#[test]
fn camera_interp_is_smooth() {
    assert!(CameraInterpolation::Linear.is_smooth());
    assert!(CameraInterpolation::CatmullRom.is_smooth());
    assert!(CameraInterpolation::Bezier.is_smooth());
    assert!(CameraInterpolation::Hermite.is_smooth());
    assert!(!CameraInterpolation::Step.is_smooth());
}

#[test]
fn camera_interp_display_contains_name() {
    for c in CameraInterpolation::all() {
        let s = format!("{}", c);
        assert!(s.contains(c.name()));
    }
}

// ============================================================================
// PLAYBACK STATE (Cinematics)
// ============================================================================

#[test]
fn playback_state_default_stopped() {
    assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
}

#[test]
fn playback_state_names() {
    assert_eq!(PlaybackState::Stopped.name(), "Stopped");
    assert_eq!(PlaybackState::Playing.name(), "Playing");
    assert_eq!(PlaybackState::Paused.name(), "Paused");
    assert_eq!(PlaybackState::Recording.name(), "Recording");
}

#[test]
fn playback_state_is_running() {
    assert!(!PlaybackState::Stopped.is_running());
    assert!(PlaybackState::Playing.is_running());
    assert!(!PlaybackState::Paused.is_running());
    assert!(PlaybackState::Recording.is_running());
}

#[test]
fn playback_state_display_contains_name() {
    let states = [
        PlaybackState::Stopped,
        PlaybackState::Playing,
        PlaybackState::Paused,
        PlaybackState::Recording,
    ];
    for s in &states {
        assert!(format!("{}", s).contains(s.name()));
    }
}

// ============================================================================
// PLAYBACK SPEED
// ============================================================================

#[test]
fn playback_speed_all_count() {
    assert_eq!(PlaybackSpeed::all().len(), 5);
}

#[test]
fn playback_speed_default_normal() {
    assert_eq!(PlaybackSpeed::default(), PlaybackSpeed::Normal);
}

#[test]
fn playback_speed_multipliers() {
    assert!((PlaybackSpeed::Quarter.multiplier() - 0.25).abs() < 0.001);
    assert!((PlaybackSpeed::Half.multiplier() - 0.5).abs() < 0.001);
    assert!((PlaybackSpeed::Normal.multiplier() - 1.0).abs() < 0.001);
    assert!((PlaybackSpeed::Double.multiplier() - 2.0).abs() < 0.001);
    assert!((PlaybackSpeed::Quadruple.multiplier() - 4.0).abs() < 0.001);
}

#[test]
fn playback_speed_display_strings() {
    assert_eq!(PlaybackSpeed::Quarter.display(), "0.25×");
    assert_eq!(PlaybackSpeed::Half.display(), "0.5×");
    assert_eq!(PlaybackSpeed::Normal.display(), "1×");
    assert_eq!(PlaybackSpeed::Double.display(), "2×");
    assert_eq!(PlaybackSpeed::Quadruple.display(), "4×");
}

#[test]
fn playback_speed_format_matches_display() {
    for s in PlaybackSpeed::all() {
        assert_eq!(format!("{}", s), s.display());
    }
}

// ============================================================================
// CAMERA KEYFRAME DEFAULTS
// ============================================================================

// C.7.C (Unified Camera campaign): `camera_keyframe_defaults` removed.
// The editor's parallel `CameraKeyframe` type retired; `CinematicsPanel`
// now operates on canonical `astraweave_cinematics::CameraKey` directly.
// The editor's UI-default keyframe values (pos (0,5,-10), look_at (0,0,0),
// fov 60) are constructed via the private `default_camera_key` helper in
// cinematics_panel.rs; the canonical CameraKey has no `Default` impl and
// no `roll` field (roll was UI-state-only dormant per C.7.0 §3, dropped
// in C.7.C). Coverage for CameraKey's own behavior (incl. sanitize) lives
// in astraweave-cinematics' test suite.

// ============================================================================
// TRACK ENTRY DEFAULTS
// ============================================================================

#[test]
fn track_entry_defaults() {
    let t = TrackEntry::default();
    assert_eq!(t.id, 0);
    assert_eq!(t.name, "New Track");
    assert_eq!(t.track_type, TrackType::Camera);
    assert!((t.start_time).abs() < 0.001);
    assert!((t.duration - 5.0).abs() < 0.001);
    assert!(!t.muted);
    assert!(!t.locked);
}

// ============================================================================
// CLIP DATA
// ============================================================================

#[test]
fn clip_data_default_is_camera() {
    let d = ClipData::default();
    assert_eq!(d.name(), "Camera");
}

#[test]
fn clip_data_names() {
    let camera = ClipData::Camera { keyframes: vec![] };
    assert_eq!(camera.name(), "Camera");

    let animation = ClipData::Animation {
        target_id: 0,
        clip_name: "idle".to_string(),
    };
    assert_eq!(animation.name(), "Animation");

    let audio = ClipData::Audio {
        file: "f.wav".to_string(),
        volume: 1.0,
        fade_in: 0.0,
        fade_out: 0.0,
    };
    assert_eq!(audio.name(), "Audio");

    let fx = ClipData::Fx {
        effect_name: "boom".to_string(),
        params: "{}".to_string(),
    };
    assert_eq!(fx.name(), "VFX");

    let dialogue = ClipData::Dialogue {
        speaker: "NPC".to_string(),
        text: "Hello".to_string(),
        duration: 2.0,
    };
    assert_eq!(dialogue.name(), "Dialogue");

    let event = ClipData::Event {
        event_name: "trigger".to_string(),
        payload: "{}".to_string(),
    };
    assert_eq!(event.name(), "Event");
}

#[test]
fn clip_data_display_matches_name() {
    let c = ClipData::Camera { keyframes: vec![] };
    assert_eq!(format!("{}", c), "Camera");
}

// ============================================================================
// TIMELINE SETTINGS DEFAULTS
// ============================================================================

#[test]
fn timeline_settings_defaults() {
    let t = TimelineSettings::default();
    assert!((t.duration - 30.0).abs() < 0.001);
    assert!((t.framerate - 30.0).abs() < 0.001);
    assert!(t.snap_to_frame);
    assert!(t.show_markers);
    assert!(!t.loop_playback);
    assert!((t.zoom_level - 1.0).abs() < 0.001);
    assert!((t.scroll_offset).abs() < 0.001);
}

// ============================================================================
// CINEMATICS TAB
// ============================================================================

#[test]
fn cinematics_tab_all_count() {
    assert_eq!(CinematicsTab::all().len(), 6);
}

#[test]
fn cinematics_tab_default_timeline() {
    assert_eq!(CinematicsTab::default(), CinematicsTab::Timeline);
}

#[test]
fn cinematics_tab_names() {
    assert_eq!(CinematicsTab::Timeline.name(), "Timeline");
    assert_eq!(CinematicsTab::Camera.name(), "Camera");
    assert_eq!(CinematicsTab::Tracks.name(), "Tracks");
    assert_eq!(CinematicsTab::Clips.name(), "Clips");
    assert_eq!(CinematicsTab::Preview.name(), "Preview");
    assert_eq!(CinematicsTab::Export.name(), "Export");
}

#[test]
fn cinematics_tab_display() {
    for t in CinematicsTab::all() {
        assert!(format!("{}", t).contains(t.name()));
    }
}

// ============================================================================
// CINEMATICS PANEL
// ============================================================================

#[test]
fn cinematics_panel_new_does_not_panic() {
    let _p = CinematicsPanel::new();
}

// ============================================================================
// SUBSYSTEM TIMINGS — TOTAL
// ============================================================================

#[test]
fn subsystem_timings_default_zero() {
    let s = SubsystemTimings::default();
    assert!((s.total()).abs() < 0.001);
}

#[test]
fn subsystem_timings_total_sums_all() {
    let s = SubsystemTimings {
        render: 4.0,
        physics: 2.0,
        ai: 1.5,
        audio: 0.5,
        scripts: 0.8,
        animation: 0.3,
        ui: 0.2,
        network: 0.1,
    };
    assert!((s.total() - 9.4).abs() < 0.001);
}

#[test]
fn subsystem_timings_single_field() {
    let s = SubsystemTimings {
        render: 10.0,
        ..Default::default()
    };
    assert!((s.total() - 10.0).abs() < 0.001);
}

// ============================================================================
// GPU METRICS DEFAULTS
// ============================================================================

#[test]
fn gpu_metrics_default() {
    let g = GpuMetrics::default();
    assert_eq!(g.draw_calls, 0);
    assert_eq!(g.triangles, 0);
    assert_eq!(g.vertices, 0);
    assert!((g.gpu_time_ms).abs() < 0.001);
    assert!((g.vram_used_mb).abs() < 0.001);
    assert!((g.vram_total_mb).abs() < 0.001);
    assert_eq!(g.textures_bound, 0);
    assert_eq!(g.shader_switches, 0);
    assert_eq!(g.state_changes, 0);
}

// ============================================================================
// FLAME NODE
// ============================================================================

#[test]
fn flame_node_new() {
    let n = FlameNode::new("Test", 5.0, egui::Color32::RED);
    assert_eq!(n.name, "Test");
    assert!((n.time_ms - 5.0).abs() < 0.001);
    assert!(n.children.is_empty());
}

#[test]
fn flame_node_total_time_no_children() {
    let n = FlameNode::new("Leaf", 3.0, egui::Color32::RED);
    assert!((n.total_time() - 3.0).abs() < 0.001);
}

#[test]
fn flame_node_total_time_with_children() {
    let mut parent = FlameNode::new("Parent", 2.0, egui::Color32::RED);
    parent
        .children
        .push(FlameNode::new("A", 1.0, egui::Color32::RED));
    parent
        .children
        .push(FlameNode::new("B", 3.0, egui::Color32::RED));
    // total = 2.0 (self) + 1.0 (A) + 3.0 (B) = 6.0
    assert!((parent.total_time() - 6.0).abs() < 0.001);
}

#[test]
fn flame_node_total_time_nested() {
    let mut root = FlameNode::new("Root", 1.0, egui::Color32::RED);
    let mut child = FlameNode::new("Child", 2.0, egui::Color32::RED);
    child
        .children
        .push(FlameNode::new("Grandchild", 4.0, egui::Color32::RED));
    root.children.push(child);
    // total = 1.0 + 2.0 + 4.0 = 7.0
    assert!((root.total_time() - 7.0).abs() < 0.001);
}

#[test]
fn sample_flame_graph_structure() {
    let root = ProfilerPanel::create_sample_flame_graph();
    assert_eq!(root.name, "Frame");
    assert!(!root.children.is_empty());
    assert!(root.total_time() > 0.0);
}

// ============================================================================
// PROFILER PANEL — PUSH_FRAME_TIME
// ============================================================================

#[test]
fn profiler_new_does_not_panic() {
    let _p = ProfilerPanel::new();
}

#[test]
fn profiler_push_frame_time_no_panic() {
    let mut p = ProfilerPanel::new();
    p.push_frame_time(16.67);
    p.push_frame_time(33.33);
    p.push_frame_time(8.0);
}

#[test]
fn profiler_push_many_frames() {
    let mut p = ProfilerPanel::new();
    for i in 0..200 {
        p.push_frame_time(10.0 + (i as f32) * 0.1);
    }
    // Exceeds default max_samples=120, should not panic (ring-buffer behavior)
}

#[test]
fn profiler_push_memory_no_panic() {
    let mut p = ProfilerPanel::new();
    p.push_memory_sample(1024);
    p.push_memory_sample(2048);
    p.push_memory_sample(512);
}

#[test]
fn profiler_push_many_memory_samples() {
    let mut p = ProfilerPanel::new();
    for i in 0..200 {
        p.push_memory_sample(i * 100);
    }
}

#[test]
fn profiler_reset_peaks_no_panic() {
    let mut p = ProfilerPanel::new();
    p.push_frame_time(33.0);
    p.push_memory_sample(4096);
    p.reset_peaks();
    // Reset should not panic even after pushing data
}

#[test]
fn profiler_push_subsystem_timings() {
    let mut p = ProfilerPanel::new();
    let t = SubsystemTimings {
        render: 8.0,
        physics: 2.0,
        ..Default::default()
    };
    p.push_subsystem_timings(t);
    // Not panicking = success
}

#[test]
fn profiler_push_gpu_metrics() {
    let mut p = ProfilerPanel::new();
    let m = GpuMetrics {
        draw_calls: 500,
        triangles: 1_000_000,
        gpu_time_ms: 8.5,
        ..Default::default()
    };
    p.push_gpu_metrics(m);
}

#[test]
fn sample_memory_categories_nonempty() {
    let cats = ProfilerPanel::create_sample_memory_categories();
    assert!(!cats.is_empty());
    for cat in &cats {
        assert!(!cat.name.is_empty());
        assert!(cat.used_bytes > 0);
        assert!(cat.allocated_bytes >= cat.used_bytes);
    }
}

// ============================================================================
// HIERARCHY ACTION — NAME / ICON / IS_DESTRUCTIVE / IS_PREFAB_ACTION
// ============================================================================

#[test]
fn hierarchy_action_names() {
    let e: u32 = 1;
    assert_eq!(HierarchyAction::CreatePrefab(e).name(), "Create Prefab");
    assert_eq!(HierarchyAction::DeleteEntity(e).name(), "Delete Entity");
    assert_eq!(
        HierarchyAction::DuplicateEntity(e).name(),
        "Duplicate Entity"
    );
    assert_eq!(HierarchyAction::FocusEntity(e).name(), "Focus Entity");
    assert_eq!(
        HierarchyAction::BreakPrefabConnection(e).name(),
        "Break Prefab Connection"
    );
    assert_eq!(
        HierarchyAction::ApplyOverridesToPrefab(e).name(),
        "Apply Overrides"
    );
    assert_eq!(
        HierarchyAction::RevertToOriginalPrefab(e).name(),
        "Revert to Original"
    );
}

#[test]
fn hierarchy_action_icons_nonempty() {
    let e: u32 = 1;
    let actions = [
        HierarchyAction::CreatePrefab(e),
        HierarchyAction::DeleteEntity(e),
        HierarchyAction::DuplicateEntity(e),
        HierarchyAction::FocusEntity(e),
        HierarchyAction::BreakPrefabConnection(e),
        HierarchyAction::ApplyOverridesToPrefab(e),
        HierarchyAction::RevertToOriginalPrefab(e),
    ];
    for a in &actions {
        assert!(!a.icon().is_empty());
    }
}

#[test]
fn hierarchy_action_display_contains_name() {
    let e: u32 = 1;
    let actions = [
        HierarchyAction::CreatePrefab(e),
        HierarchyAction::DeleteEntity(e),
        HierarchyAction::DuplicateEntity(e),
        HierarchyAction::FocusEntity(e),
    ];
    for a in &actions {
        assert!(format!("{}", a).contains(a.name()));
    }
}

#[test]
fn hierarchy_action_is_destructive() {
    let e: u32 = 1;
    assert!(HierarchyAction::DeleteEntity(e).is_destructive());
    assert!(HierarchyAction::BreakPrefabConnection(e).is_destructive());
    assert!(!HierarchyAction::CreatePrefab(e).is_destructive());
    assert!(!HierarchyAction::DuplicateEntity(e).is_destructive());
    assert!(!HierarchyAction::FocusEntity(e).is_destructive());
    assert!(!HierarchyAction::ApplyOverridesToPrefab(e).is_destructive());
    assert!(!HierarchyAction::RevertToOriginalPrefab(e).is_destructive());
}

#[test]
fn hierarchy_action_is_prefab_action() {
    let e: u32 = 1;
    assert!(HierarchyAction::CreatePrefab(e).is_prefab_action());
    assert!(HierarchyAction::BreakPrefabConnection(e).is_prefab_action());
    assert!(HierarchyAction::ApplyOverridesToPrefab(e).is_prefab_action());
    assert!(HierarchyAction::RevertToOriginalPrefab(e).is_prefab_action());
    assert!(!HierarchyAction::DeleteEntity(e).is_prefab_action());
    assert!(!HierarchyAction::DuplicateEntity(e).is_prefab_action());
    assert!(!HierarchyAction::FocusEntity(e).is_prefab_action());
}

// ============================================================================
// HIERARCHY PANEL — PREFAB INSTANCE TRACKING
// ============================================================================

#[test]
fn hierarchy_panel_new_empty() {
    let mut p = HierarchyPanel::new();
    assert!(p.get_selected().is_none());
    assert!(p.get_all_selected().is_empty());
    assert!(p.take_pending_actions().is_empty());
}

#[test]
fn hierarchy_panel_mark_prefab_instance() {
    let mut p = HierarchyPanel::new();
    let e: u32 = 42;
    assert!(!p.is_prefab_instance(e));
    p.mark_as_prefab_instance(e);
    assert!(p.is_prefab_instance(e));
}

#[test]
fn hierarchy_panel_unmark_prefab_instance() {
    let mut p = HierarchyPanel::new();
    let e: u32 = 42;
    p.mark_as_prefab_instance(e);
    p.unmark_as_prefab_instance(e);
    assert!(!p.is_prefab_instance(e));
}

#[test]
fn hierarchy_panel_sync_prefab_instances() {
    let mut p = HierarchyPanel::new();
    let e1: u32 = 1;
    let e2: u32 = 2;
    let e3: u32 = 3;
    p.mark_as_prefab_instance(e1);
    // Now sync with a new set
    p.sync_prefab_instances(vec![e2, e3].into_iter());
    assert!(!p.is_prefab_instance(e1)); // cleared
    assert!(p.is_prefab_instance(e2));
    assert!(p.is_prefab_instance(e3));
}

#[test]
fn hierarchy_panel_take_actions_drains() {
    let mut p = HierarchyPanel::new();
    let actions = p.take_pending_actions();
    assert!(actions.is_empty());
    // Should be empty again on second call
    let actions2 = p.take_pending_actions();
    assert!(actions2.is_empty());
}

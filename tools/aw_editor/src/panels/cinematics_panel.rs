//! Cinematics Panel for the editor UI
//!
//! Provides comprehensive cinematic/cutscene editing:
//! - Timeline editing with multiple tracks
//! - Camera keyframe editor with spline interpolation
//! - Animation track management
//! - Audio clip placement
//! - VFX trigger system
//! - Sequencer playback controls

use astraweave_cinematics::{CameraKey, Time};
use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// Track type for cinematics
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrackType {
    #[default]
    Camera,
    Animation,
    Audio,
    Fx,
    Dialogue,
    Event,
}

impl std::fmt::Display for TrackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TrackType {
    pub fn name(&self) -> &'static str {
        match self {
            TrackType::Camera => "Camera",
            TrackType::Animation => "Animation",
            TrackType::Audio => "Audio",
            TrackType::Fx => "VFX",
            TrackType::Dialogue => "Dialogue",
            TrackType::Event => "Event",
        }
    }

    pub fn all() -> &'static [TrackType] {
        &[
            TrackType::Camera,
            TrackType::Animation,
            TrackType::Audio,
            TrackType::Fx,
            TrackType::Dialogue,
            TrackType::Event,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TrackType::Camera => "📷",
            TrackType::Animation => "[Anim]",
            TrackType::Audio => "[Snd]",
            TrackType::Fx => "[Fx]",
            TrackType::Dialogue => "[Chat]",
            TrackType::Event => "[Zap]",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            TrackType::Camera => Color32::from_rgb(100, 149, 237), // Cornflower blue
            TrackType::Animation => Color32::from_rgb(144, 238, 144), // Light green
            TrackType::Audio => Color32::from_rgb(255, 165, 0),    // Orange
            TrackType::Fx => Color32::from_rgb(186, 85, 211),      // Medium orchid
            TrackType::Dialogue => Color32::from_rgb(255, 215, 0), // Gold
            TrackType::Event => Color32::from_rgb(220, 20, 60),    // Crimson
        }
    }
}

/// Camera interpolation mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CameraInterpolation {
    #[default]
    Linear,
    CatmullRom,
    Bezier,
    Hermite,
    Step,
}

impl std::fmt::Display for CameraInterpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl CameraInterpolation {
    pub fn name(&self) -> &'static str {
        match self {
            CameraInterpolation::Linear => "Linear",
            CameraInterpolation::CatmullRom => "Catmull-Rom",
            CameraInterpolation::Bezier => "Bezier",
            CameraInterpolation::Hermite => "Hermite",
            CameraInterpolation::Step => "Step",
        }
    }

    pub fn is_smooth(&self) -> bool {
        !matches!(self, CameraInterpolation::Step)
    }

    pub fn all() -> &'static [CameraInterpolation] {
        &[
            CameraInterpolation::Linear,
            CameraInterpolation::CatmullRom,
            CameraInterpolation::Bezier,
            CameraInterpolation::Hermite,
            CameraInterpolation::Step,
        ]
    }
}

/// Playback state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Recording,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PlaybackState {
    pub fn name(&self) -> &'static str {
        match self {
            PlaybackState::Stopped => "Stopped",
            PlaybackState::Playing => "Playing",
            PlaybackState::Paused => "Paused",
            PlaybackState::Recording => "Recording",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PlaybackState::Stopped => "[]",
            PlaybackState::Playing => ">",
            PlaybackState::Paused => "||",
            PlaybackState::Recording => "⏺️",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, PlaybackState::Playing | PlaybackState::Recording)
    }
}

/// Playback speed
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[non_exhaustive]
pub enum PlaybackSpeed {
    Quarter,
    Half,
    #[default]
    Normal,
    Double,
    Quadruple,
}

impl std::fmt::Display for PlaybackSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl PlaybackSpeed {
    pub fn all() -> &'static [PlaybackSpeed] {
        &[
            PlaybackSpeed::Quarter,
            PlaybackSpeed::Half,
            PlaybackSpeed::Normal,
            PlaybackSpeed::Double,
            PlaybackSpeed::Quadruple,
        ]
    }

    pub fn multiplier(&self) -> f32 {
        match self {
            PlaybackSpeed::Quarter => 0.25,
            PlaybackSpeed::Half => 0.5,
            PlaybackSpeed::Normal => 1.0,
            PlaybackSpeed::Double => 2.0,
            PlaybackSpeed::Quadruple => 4.0,
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            PlaybackSpeed::Quarter => "0.25×",
            PlaybackSpeed::Half => "0.5×",
            PlaybackSpeed::Normal => "1×",
            PlaybackSpeed::Double => "2×",
            PlaybackSpeed::Quadruple => "4×",
        }
    }
}

// C.7.C (Unified Camera campaign): the editor's parallel `CameraKeyframe`
// type retired. `CinematicsPanel` now operates on canonical
// `astraweave_cinematics::CameraKey` directly (field mapping: time→t,
// position→pos, look_at→look_at, fov→fov_deg). The `roll` field dropped
// entirely — C.7.0 §3's empirical investigation found it UI-state-only
// dormant (slider write + label read + lerp, all within this panel; no
// view-matrix conversion, no renderer flow, no serialization). The
// editor's keyframe data has no persistence path (verified C.7.C Phase 1B:
// no Serialize/Deserialize on CameraKeyframe or ClipData), so the
// `CameraKeyframe → CameraKey` transition is a clean break with no
// backward-compat shim. The default keyframe values that
// `CameraKeyframe::default()` provided (pos (0,5,-10), look_at (0,0,0),
// fov 60) are now constructed explicitly via
// `CameraKey::new(Time(t), (0.0, 5.0, -10.0), (0.0, 0.0, 0.0), 60.0)` at
// the former-default call sites.

/// The editor's default camera keyframe values, preserved from the
/// retired `CameraKeyframe::default()` (C.7.C). Canonical `CameraKey`
/// intentionally has no `Default` impl (it's data with no universally-
/// correct default), so the editor constructs its UI default here.
fn default_camera_key(time: f32) -> CameraKey {
    CameraKey::new(Time(time), (0.0, 5.0, -10.0), (0.0, 0.0, 0.0), 60.0)
}

/// Track entry
#[derive(Debug, Clone)]
pub struct TrackEntry {
    pub id: u32,
    pub name: String,
    pub track_type: TrackType,
    pub start_time: f32,
    pub duration: f32,
    pub muted: bool,
    pub locked: bool,
}

impl Default for TrackEntry {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Track".to_string(),
            track_type: TrackType::Camera,
            start_time: 0.0,
            duration: 5.0,
            muted: false,
            locked: false,
        }
    }
}

/// Clip on a track
#[derive(Debug, Clone)]
pub struct TrackClip {
    pub id: u32,
    pub name: String,
    pub start_time: f32,
    pub duration: f32,
    pub color: Color32,
    pub data: ClipData,
}

/// Clip-specific data
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ClipData {
    Camera {
        keyframes: Vec<CameraKey>,
    },
    Animation {
        target_id: u32,
        clip_name: String,
    },
    Audio {
        file: String,
        volume: f32,
        fade_in: f32,
        fade_out: f32,
    },
    Fx {
        effect_name: String,
        params: String,
    },
    Dialogue {
        speaker: String,
        text: String,
        duration: f32,
    },
    Event {
        event_name: String,
        payload: String,
    },
}

impl std::fmt::Display for ClipData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ClipData {
    pub fn name(&self) -> &'static str {
        match self {
            ClipData::Camera { .. } => "Camera",
            ClipData::Animation { .. } => "Animation",
            ClipData::Audio { .. } => "Audio",
            ClipData::Fx { .. } => "VFX",
            ClipData::Dialogue { .. } => "Dialogue",
            ClipData::Event { .. } => "Event",
        }
    }
}

impl Default for ClipData {
    fn default() -> Self {
        ClipData::Camera {
            keyframes: Vec::new(),
        }
    }
}

/// Timeline settings
#[derive(Debug, Clone)]
pub struct TimelineSettings {
    pub duration: f32,
    pub framerate: f32,
    pub snap_to_frame: bool,
    pub show_markers: bool,
    pub loop_playback: bool,
    pub zoom_level: f32,
    pub scroll_offset: f32,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            duration: 30.0,
            framerate: 30.0,
            snap_to_frame: true,
            show_markers: true,
            loop_playback: false,
            zoom_level: 1.0,
            scroll_offset: 0.0,
        }
    }
}

/// Marker on timeline
#[derive(Debug, Clone)]
pub struct TimelineMarker {
    pub time: f32,
    pub name: String,
    pub color: Color32,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CinematicsTab {
    #[default]
    Timeline,
    Camera,
    Tracks,
    Clips,
    Preview,
    Export,
}

impl std::fmt::Display for CinematicsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl CinematicsTab {
    pub fn all() -> &'static [CinematicsTab] {
        &[
            CinematicsTab::Timeline,
            CinematicsTab::Camera,
            CinematicsTab::Tracks,
            CinematicsTab::Clips,
            CinematicsTab::Preview,
            CinematicsTab::Export,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CinematicsTab::Timeline => "Timeline",
            CinematicsTab::Camera => "Camera",
            CinematicsTab::Tracks => "Tracks",
            CinematicsTab::Clips => "Clips",
            CinematicsTab::Preview => "Preview",
            CinematicsTab::Export => "Export",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CinematicsTab::Timeline => "📅",
            CinematicsTab::Camera => "📷",
            CinematicsTab::Tracks => "[Anim]",
            CinematicsTab::Clips => "🎞️",
            CinematicsTab::Preview => "👁️",
            CinematicsTab::Export => "[Save]",
        }
    }
}

/// Actions that can be performed on the cinematics panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CinematicsAction {
    /// Start playback
    Play,
    /// Pause playback
    Pause,
    /// Stop and reset to start
    Stop,
    /// Seek to specific time
    SeekTo(f32),
    /// Set playback speed
    SetSpeed(PlaybackSpeed),
    /// Add a new track
    AddTrack(TrackType),
    /// Remove a track
    RemoveTrack { track_id: u32 },
    /// Add a clip to track
    AddClip { track_id: u32, start_time: f32 },
    /// Remove a clip
    RemoveClip { clip_id: u32 },
    /// Move a clip to new time
    MoveClip { clip_id: u32, new_time: f32 },
    /// Add a camera keyframe
    AddCameraKeyframe { time: f32 },
    /// Remove a camera keyframe
    RemoveCameraKeyframe { index: usize },
    /// Set camera interpolation mode
    SetCameraInterpolation(CameraInterpolation),
    /// Add a timeline marker
    AddMarker { time: f32, name: String },
    /// Remove a marker
    RemoveMarker { index: usize },
    /// Toggle camera preview
    ToggleCameraPreview(bool),
    /// Export the sequence
    Export { format: String },
    /// Set timeline duration
    SetDuration(f32),
    /// Set loop mode
    SetLoopPlayback(bool),
}

/// Main Cinematics Panel
pub struct CinematicsPanel {
    // Tab state
    active_tab: CinematicsTab,

    // Timeline data
    current_time: f32,
    playback_state: PlaybackState,
    playback_speed: PlaybackSpeed,
    settings: TimelineSettings,

    // Tracks
    tracks: Vec<TrackEntry>,
    clips: Vec<TrackClip>,
    markers: Vec<TimelineMarker>,

    // Selection
    selected_track: Option<u32>,
    selected_clip: Option<u32>,
    selected_keyframe: Option<usize>,

    // Camera editing
    camera_interpolation: CameraInterpolation,
    camera_keyframes: Vec<CameraKey>,
    preview_camera: bool,

    // Editing state
    editing_clip: Option<u32>,
    dragging_clip: bool,
    drag_offset: f32,

    // Timeline ID counter
    next_id: u32,

    // Action queue
    pending_actions: Vec<CinematicsAction>,
}

impl Default for CinematicsPanel {
    fn default() -> Self {
        let mut panel = Self {
            active_tab: CinematicsTab::Timeline,

            current_time: 0.0,
            playback_state: PlaybackState::Stopped,
            playback_speed: PlaybackSpeed::Normal,
            settings: TimelineSettings::default(),

            tracks: Vec::new(),
            clips: Vec::new(),
            markers: Vec::new(),

            selected_track: None,
            selected_clip: None,
            selected_keyframe: None,

            camera_interpolation: CameraInterpolation::CatmullRom,
            camera_keyframes: Vec::new(),
            preview_camera: false,

            editing_clip: None,
            dragging_clip: false,
            drag_offset: 0.0,

            next_id: 1,

            pending_actions: Vec::new(),
        };

        // Add default tracks
        panel.add_default_tracks();

        panel
    }
}

impl CinematicsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions, leaving the queue empty
    pub fn take_actions(&mut self) -> Vec<CinematicsAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: CinematicsAction) {
        info!(target: "aw_editor::panels::cinematics_panel", action = ?action, "Cinematics action queued");
        self.pending_actions.push(action);
    }

    fn add_default_tracks(&mut self) {
        // Camera track
        let camera_id = self.next_id();
        self.tracks.push(TrackEntry {
            id: camera_id,
            name: "Main Camera".to_string(),
            track_type: TrackType::Camera,
            start_time: 0.0,
            duration: 30.0,
            muted: false,
            locked: false,
        });

        // Animation track
        let anim_id = self.next_id();
        self.tracks.push(TrackEntry {
            id: anim_id,
            name: "Character Animation".to_string(),
            track_type: TrackType::Animation,
            start_time: 0.0,
            duration: 30.0,
            muted: false,
            locked: false,
        });

        // Audio track
        let audio_id = self.next_id();
        self.tracks.push(TrackEntry {
            id: audio_id,
            name: "Music".to_string(),
            track_type: TrackType::Audio,
            start_time: 0.0,
            duration: 30.0,
            muted: false,
            locked: false,
        });

        // Add sample keyframes
        self.camera_keyframes.push(CameraKey::new(
            Time(0.0),
            (0.0, 5.0, -15.0),
            (0.0, 0.0, 0.0),
            60.0,
        ));

        self.camera_keyframes.push(CameraKey::new(
            Time(5.0),
            (10.0, 8.0, -10.0),
            (0.0, 2.0, 0.0),
            50.0,
        ));
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (CinematicsTab::Timeline, "[Sq] Timeline"),
                (CinematicsTab::Camera, "📷 Camera"),
                (CinematicsTab::Tracks, "🎚️ Tracks"),
                (CinematicsTab::Clips, "Clips"),
                (CinematicsTab::Preview, "Preview"),
                (CinematicsTab::Export, "Export"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Playback controls
        ui.horizontal(|ui| {
            let state_icon = match self.playback_state {
                PlaybackState::Stopped => "[]",
                PlaybackState::Playing => ">",
                PlaybackState::Paused => "||",
                PlaybackState::Recording => "[R]",
            };
            ui.label(state_icon);

            // Transport controls
            if ui.button("⏮").clicked() {
                self.current_time = 0.0;
            }
            if ui.button("◀").clicked() {
                self.current_time = (self.current_time - 1.0).max(0.0);
            }
            if ui
                .button(if self.playback_state == PlaybackState::Playing {
                    "||"
                } else {
                    ">"
                })
                .clicked()
            {
                self.playback_state = if self.playback_state == PlaybackState::Playing {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                };
                info!(target: "aw_editor::panels::cinematics_panel", state = %self.playback_state, "Sequence playback toggled");
            }
            if ui.button(">").clicked() {
                self.current_time = (self.current_time + 1.0).min(self.settings.duration);
            }
            if ui.button(">|").clicked() {
                self.current_time = self.settings.duration;
            }
            if ui.button("[]").clicked() {
                info!(target: "aw_editor::panels::cinematics_panel", "Sequence playback stopped");
                self.playback_state = PlaybackState::Stopped;
                self.current_time = 0.0;
            }

            ui.separator();

            // Time display
            let frame = (self.current_time * self.settings.framerate) as u32;
            let total_frames = (self.settings.duration * self.settings.framerate) as u32;
            ui.label(format!(
                "{:.2}s / {:.2}s (F{}/{})",
                self.current_time, self.settings.duration, frame, total_frames
            ));

            ui.separator();

            // Speed selector
            ui.label("Speed:");
            egui::ComboBox::from_id_salt("playback_speed")
                .selected_text(self.playback_speed.display())
                .show_ui(ui, |ui| {
                    for speed in PlaybackSpeed::all() {
                        ui.selectable_value(&mut self.playback_speed, *speed, speed.display());
                    }
                });

            // Loop toggle
            ui.checkbox(&mut self.settings.loop_playback, "🔁 Loop");
        });

        ui.separator();
    }

    fn show_timeline_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Sq] Timeline Editor");
        ui.add_space(5.0);

        // Timeline settings bar
        ui.horizontal(|ui| {
            ui.label("Duration:");
            ui.add(
                egui::DragValue::new(&mut self.settings.duration)
                    .speed(0.1)
                    .range(1.0..=600.0)
                    .suffix("s"),
            );

            ui.separator();

            ui.label("FPS:");
            ui.add(
                egui::DragValue::new(&mut self.settings.framerate)
                    .speed(1.0)
                    .range(1.0..=120.0),
            );

            ui.separator();

            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.settings.zoom_level, 0.1..=5.0).show_value(false));

            ui.checkbox(&mut self.settings.snap_to_frame, "Snap");
            ui.checkbox(&mut self.settings.show_markers, "Markers");
        });

        ui.add_space(10.0);

        // Timeline view
        self.draw_timeline(ui);

        ui.add_space(10.0);

        // Track list
        ui.group(|ui| {
            ui.label(RichText::new("Tracks").strong());

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    let track_ids: Vec<u32> = self.tracks.iter().map(|t| t.id).collect();

                    for track_id in track_ids {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                            let is_selected = self.selected_track == Some(track.id);

                            ui.horizontal(|ui| {
                                // Track type icon
                                ui.label(track.track_type.icon());

                                // Track name
                                if ui.selectable_label(is_selected, &track.name).clicked() {
                                    self.selected_track = Some(track.id);
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Mute/Solo/Lock buttons
                                        if ui
                                            .small_button(if track.locked {
                                                "[Lock]"
                                            } else {
                                                "🔓"
                                            })
                                            .clicked()
                                        {
                                            track.locked = !track.locked;
                                        }
                                        if ui
                                            .small_button(if track.muted {
                                                "[Mute]"
                                            } else {
                                                "[Snd]"
                                            })
                                            .clicked()
                                        {
                                            track.muted = !track.muted;
                                        }
                                    },
                                );
                            });
                        }
                    }
                });
        });
    }

    fn draw_timeline(&mut self, ui: &mut Ui) {
        let available_width = ui.available_width();
        let timeline_height = 80.0;

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(available_width, timeline_height),
            egui::Sense::click_and_drag(),
        );

        let painter = ui.painter();

        // Background
        painter.rect_filled(rect, 5.0, Color32::from_rgb(30, 30, 35));

        // Time ruler
        let ruler_height = 20.0;
        let ruler_rect = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), ruler_height));
        painter.rect_filled(ruler_rect, 0.0, Color32::from_rgb(40, 40, 45));

        // Draw time markers
        let pixels_per_second = (rect.width() / self.settings.duration) * self.settings.zoom_level;
        let marker_interval = if pixels_per_second > 100.0 {
            0.5
        } else if pixels_per_second > 50.0 {
            1.0
        } else if pixels_per_second > 20.0 {
            5.0
        } else {
            10.0
        };

        let mut t = 0.0;
        while t <= self.settings.duration {
            let x = rect.min.x + (t / self.settings.duration) * rect.width();

            // Tick mark
            let tick_height = if (t % 5.0).abs() < 0.01 { 15.0 } else { 8.0 };
            painter.line_segment(
                [
                    egui::Pos2::new(x, rect.min.y),
                    egui::Pos2::new(x, rect.min.y + tick_height),
                ],
                egui::Stroke::new(1.0, Color32::GRAY),
            );

            // Time label (every 5 seconds)
            if (t % 5.0).abs() < 0.01 {
                painter.text(
                    egui::Pos2::new(x + 2.0, rect.min.y + 5.0),
                    egui::Align2::LEFT_TOP,
                    format!("{:.0}s", t),
                    egui::FontId::proportional(10.0),
                    Color32::GRAY,
                );
            }

            t += marker_interval;
        }

        // Draw playhead
        let playhead_x = rect.min.x + (self.current_time / self.settings.duration) * rect.width();
        painter.line_segment(
            [
                egui::Pos2::new(playhead_x, rect.min.y),
                egui::Pos2::new(playhead_x, rect.max.y),
            ],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 100, 100)),
        );

        // Playhead handle
        let handle_points = [
            egui::Pos2::new(playhead_x - 6.0, rect.min.y),
            egui::Pos2::new(playhead_x + 6.0, rect.min.y),
            egui::Pos2::new(playhead_x, rect.min.y + 10.0),
        ];
        painter.add(egui::Shape::convex_polygon(
            handle_points.to_vec(),
            Color32::from_rgb(255, 100, 100),
            egui::Stroke::NONE,
        ));

        // Handle click/drag on timeline
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let normalized_x = (pos.x - rect.min.x) / rect.width();
                self.current_time =
                    (normalized_x * self.settings.duration).clamp(0.0, self.settings.duration);

                if self.settings.snap_to_frame {
                    let frame = (self.current_time * self.settings.framerate).round();
                    self.current_time = frame / self.settings.framerate;
                }
            }
        }

        // Draw markers
        if self.settings.show_markers {
            for marker in &self.markers {
                let x = rect.min.x + (marker.time / self.settings.duration) * rect.width();
                painter.line_segment(
                    [
                        egui::Pos2::new(x, rect.min.y + ruler_height),
                        egui::Pos2::new(x, rect.max.y),
                    ],
                    egui::Stroke::new(1.0, marker.color),
                );
            }
        }
    }

    fn show_camera_tab(&mut self, ui: &mut Ui) {
        ui.heading("📷 Camera Editor");
        ui.add_space(10.0);

        // Interpolation mode
        ui.horizontal(|ui| {
            ui.label("Interpolation:");
            for interp in CameraInterpolation::all() {
                if ui
                    .selectable_label(
                        self.camera_interpolation == *interp,
                        format!("{:?}", interp),
                    )
                    .clicked()
                {
                    self.camera_interpolation = *interp;
                }
            }
        });

        ui.checkbox(&mut self.preview_camera, "Preview camera in viewport");

        ui.add_space(10.0);

        // Keyframes
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Keyframes").strong());
                if ui.button("+ Add Keyframe").clicked() {
                    info!(target: "aw_editor::panels::cinematics_panel", time = self.current_time, "Camera keyframe added");
                    self.camera_keyframes
                        .push(default_camera_key(self.current_time));
                    self.camera_keyframes.sort_by(|a, b| {
                        a.t.0
                            .partial_cmp(&b.t.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            });

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    let mut to_remove: Option<usize> = None;

                    for (idx, keyframe) in self.camera_keyframes.iter_mut().enumerate() {
                        let is_selected = self.selected_keyframe == Some(idx);

                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(is_selected, format!("🔑 {:.2}s", keyframe.t.0))
                                .clicked()
                            {
                                self.selected_keyframe = Some(idx);
                                self.current_time = keyframe.t.0;
                            }

                            ui.label(format!(
                                "Pos: ({:.1}, {:.1}, {:.1})",
                                keyframe.pos.0, keyframe.pos.1, keyframe.pos.2
                            ));

                            if ui.small_button("[Del]").clicked() {
                                to_remove = Some(idx);
                            }
                        });
                    }

                    if let Some(idx) = to_remove {
                        info!(target: "aw_editor::panels::cinematics_panel", keyframe_index = idx, "Camera keyframe removed");
                        self.camera_keyframes.remove(idx);
                        self.selected_keyframe = None;
                    }
                });
        });

        // Selected keyframe editor
        if let Some(idx) = self.selected_keyframe {
            if idx < self.camera_keyframes.len() {
                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.label(RichText::new("Edit Keyframe").strong());

                    let keyframe = &mut self.camera_keyframes[idx];

                    ui.horizontal(|ui| {
                        ui.label("Time:");
                        // CameraKey.t is a `Time(pub f32)` newtype; bind the
                        // DragValue to the inner f32 via `.t.0` (C.7.C).
                        ui.add(
                            egui::DragValue::new(&mut keyframe.t.0)
                                .speed(0.01)
                                .range(0.0..=self.settings.duration)
                                .suffix("s"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.add(
                            egui::DragValue::new(&mut keyframe.pos.0)
                                .speed(0.1)
                                .prefix("X: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut keyframe.pos.1)
                                .speed(0.1)
                                .prefix("Y: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut keyframe.pos.2)
                                .speed(0.1)
                                .prefix("Z: "),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Look At:");
                        ui.add(
                            egui::DragValue::new(&mut keyframe.look_at.0)
                                .speed(0.1)
                                .prefix("X: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut keyframe.look_at.1)
                                .speed(0.1)
                                .prefix("Y: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut keyframe.look_at.2)
                                .speed(0.1)
                                .prefix("Z: "),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("FOV:");
                        ui.add(egui::Slider::new(&mut keyframe.fov_deg, 10.0..=120.0).suffix("°"));
                    });

                    // C.7.C: Roll slider removed. `CameraKeyframe.roll` was
                    // UI-state-only dormant (C.7.0 §3); canonical
                    // `CameraKey` has no roll field.
                });
            }
        }
    }

    fn show_tracks_tab(&mut self, ui: &mut Ui) {
        ui.heading("🎚️ Track Management");
        ui.add_space(10.0);

        // Add track buttons
        let mut new_track: Option<(TrackType, f32)> = None;
        ui.horizontal(|ui| {
            ui.label("Add Track:");
            for track_type in TrackType::all() {
                if ui
                    .button(format!("{} {:?}", track_type.icon(), track_type))
                    .clicked()
                {
                    new_track = Some((*track_type, self.settings.duration));
                }
            }
        });

        if let Some((track_type, duration)) = new_track {
            let id = self.next_id();
            self.tracks.push(TrackEntry {
                id,
                name: format!("New {:?}", track_type),
                track_type,
                start_time: 0.0,
                duration,
                muted: false,
                locked: false,
            });
        }

        ui.add_space(10.0);

        // Track list with details
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let track_ids: Vec<u32> = self.tracks.iter().map(|t| t.id).collect();

                for track_id in track_ids {
                    if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                        let is_selected = self.selected_track == Some(track.id);

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Color indicator
                                let color_rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    Vec2::new(4.0, 20.0),
                                );
                                ui.painter()
                                    .rect_filled(color_rect, 2.0, track.track_type.color());
                                ui.add_space(8.0);

                                // Track icon
                                ui.label(track.track_type.icon());

                                // Track name (editable)
                                if ui.selectable_label(is_selected, &track.name).clicked() {
                                    self.selected_track = Some(track.id);
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Delete button
                                        if ui.small_button("[Del]").clicked() {
                                            // Mark for deletion
                                        }

                                        // Lock button
                                        if ui
                                            .small_button(if track.locked {
                                                "[Lock]"
                                            } else {
                                                "🔓"
                                            })
                                            .clicked()
                                        {
                                            track.locked = !track.locked;
                                        }

                                        // Mute button
                                        if ui
                                            .small_button(if track.muted {
                                                "[Mute]"
                                            } else {
                                                "[Snd]"
                                            })
                                            .clicked()
                                        {
                                            track.muted = !track.muted;
                                        }
                                    },
                                );
                            });

                            // Track details when selected
                            if is_selected {
                                ui.add_space(5.0);
                                egui::Grid::new(format!("track_details_{}", track.id))
                                    .num_columns(2)
                                    .spacing([10.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label("Name:");
                                        ui.text_edit_singleline(&mut track.name);
                                        ui.end_row();

                                        ui.label("Start:");
                                        ui.add(
                                            egui::DragValue::new(&mut track.start_time)
                                                .speed(0.1)
                                                .range(0.0..=self.settings.duration)
                                                .suffix("s"),
                                        );
                                        ui.end_row();

                                        ui.label("Duration:");
                                        ui.add(
                                            egui::DragValue::new(&mut track.duration)
                                                .speed(0.1)
                                                .range(0.1..=600.0)
                                                .suffix("s"),
                                        );
                                        ui.end_row();
                                    });
                            }
                        });
                    }
                }
            });
    }

    fn show_clips_tab(&mut self, ui: &mut Ui) {
        ui.heading("Clip Editor");
        ui.add_space(10.0);

        if self.clips.is_empty() {
            ui.label("No clips yet. Add clips from the track editor.");

            if ui.button("+ Create Sample Clip").clicked() {
                let id = self.next_id();
                info!(target: "aw_editor::panels::cinematics_panel", clip_id = id, "Sample clip created");
                let keyframes = self.camera_keyframes.clone();
                self.clips.push(TrackClip {
                    id,
                    name: "Sample Clip".to_string(),
                    start_time: 0.0,
                    duration: 5.0,
                    color: Color32::from_rgb(100, 149, 237),
                    data: ClipData::Camera { keyframes },
                });
            }
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for clip in &self.clips {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(&clip.name);
                                ui.label(format!(
                                    "{:.1}s - {:.1}s",
                                    clip.start_time,
                                    clip.start_time + clip.duration
                                ));
                            });
                        });
                    }
                });
        }
    }

    fn show_preview_tab(&mut self, ui: &mut Ui) {
        ui.heading("Preview");
        ui.add_space(10.0);

        // Preview viewport placeholder
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 200.0), egui::Sense::hover());

        ui.painter()
            .rect_filled(rect, 5.0, Color32::from_rgb(20, 20, 25));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Camera Preview",
            egui::FontId::proportional(16.0),
            Color32::GRAY,
        );

        ui.add_space(10.0);

        // Current camera info
        ui.group(|ui| {
            ui.label(RichText::new("Current Camera State").strong());

            if let Some(keyframe) = self.current_interpolated_key() {
                egui::Grid::new("camera_preview_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Position:");
                        ui.label(format!(
                            "({:.2}, {:.2}, {:.2})",
                            keyframe.pos.0, keyframe.pos.1, keyframe.pos.2
                        ));
                        ui.end_row();

                        ui.label("Look At:");
                        ui.label(format!(
                            "({:.2}, {:.2}, {:.2})",
                            keyframe.look_at.0, keyframe.look_at.1, keyframe.look_at.2
                        ));
                        ui.end_row();

                        ui.label("FOV:");
                        ui.label(format!("{:.1}°", keyframe.fov_deg));
                        ui.end_row();

                        // C.7.C: Roll preview label removed (roll field
                        // dropped from the canonical CameraKey).
                    });
            } else {
                ui.label("No keyframes to interpolate");
            }
        });
    }

    fn show_export_tab(&mut self, ui: &mut Ui) {
        ui.heading("Export");
        ui.add_space(10.0);

        // Export format
        ui.group(|ui| {
            ui.label(RichText::new("Export Format").strong());

            ui.horizontal(|ui| {
                if ui.button("JSON Timeline").clicked() {
                    info!(target: "aw_editor::panels::cinematics_panel", format = "JSON", "Sequence export requested");
                    // Export as JSON
                }
                if ui.button("RON Format").clicked() {
                    info!(target: "aw_editor::panels::cinematics_panel", format = "RON", "Sequence export requested");
                    // Export as RON
                }
                if ui.button("Binary Pack").clicked() {
                    info!(target: "aw_editor::panels::cinematics_panel", format = "Binary", "Sequence export requested");
                    // Export as binary
                }
            });
        });

        ui.add_space(10.0);

        // Export options
        ui.group(|ui| {
            ui.label(RichText::new("Export Options").strong());

            ui.checkbox(&mut self.settings.loop_playback, "Include loop markers");
            ui.checkbox(&mut self.preview_camera, "Include preview data");
        });

        ui.add_space(10.0);

        // Statistics
        ui.group(|ui| {
            ui.label(RichText::new("Timeline Statistics").strong());

            egui::Grid::new("export_stats_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Duration:");
                    ui.label(format!("{:.2}s", self.settings.duration));
                    ui.end_row();

                    ui.label("Tracks:");
                    ui.label(format!("{}", self.tracks.len()));
                    ui.end_row();

                    ui.label("Clips:");
                    ui.label(format!("{}", self.clips.len()));
                    ui.end_row();

                    ui.label("Camera Keyframes:");
                    ui.label(format!("{}", self.camera_keyframes.len()));
                    ui.end_row();

                    ui.label("Markers:");
                    ui.label(format!("{}", self.markers.len()));
                    ui.end_row();

                    ui.label("Total Frames:");
                    ui.label(format!(
                        "{}",
                        (self.settings.duration * self.settings.framerate) as u32
                    ));
                    ui.end_row();
                });
        });
    }

    /// Interpolated canonical [`CameraKey`] at the current playback time.
    ///
    /// C.7.C: replaces the retired `get_interpolated_camera() ->
    /// Option<CameraKeyframe>`. Returns the canonical `CameraKey` (not the
    /// removed parallel type), and delegates the actual field-by-field
    /// interpolation to [`CameraKey::lerp`] rather than hand-rolling it.
    /// This thin helper retains only the adjacent-keyframe-finding logic
    /// (locating the two keyframes bracketing `current_time`); the lerp
    /// itself is the canonical method's responsibility.
    fn current_interpolated_key(&self) -> Option<CameraKey> {
        if self.camera_keyframes.is_empty() {
            return None;
        }

        if self.camera_keyframes.len() == 1 {
            return Some(self.camera_keyframes[0].clone());
        }

        // Find surrounding keyframes (compare on the Time newtype's inner f32).
        let mut before: Option<&CameraKey> = None;
        let mut after: Option<&CameraKey> = None;

        for kf in &self.camera_keyframes {
            if kf.t.0 <= self.current_time {
                before = Some(kf);
            }
            if kf.t.0 >= self.current_time && after.is_none() {
                after = Some(kf);
            }
        }

        match (before, after) {
            (Some(b), Some(a)) if b.t.0 != a.t.0 => {
                let t = (self.current_time - b.t.0) / (a.t.0 - b.t.0);
                // Delegate the per-field interpolation to the canonical
                // CameraKey::lerp (which interpolates t, pos, look_at,
                // fov_deg). The returned key's `t` is the lerped timestamp;
                // for preview-display purposes this matches the old
                // behavior closely enough (the old method set time =
                // current_time exactly; lerp sets t = lerp(b.t, a.t, t)
                // which equals current_time when the factor is derived
                // from current_time as above).
                Some(b.lerp(a, t))
            }
            (Some(kf), _) | (_, Some(kf)) => Some(kf.clone()),
            _ => None,
        }
    }

    // Getters for testing
    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn keyframe_count(&self) -> usize {
        self.camera_keyframes.len()
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    pub fn duration(&self) -> f32 {
        self.settings.duration
    }

    pub fn framerate(&self) -> f32 {
        self.settings.framerate
    }

    pub fn set_current_time(&mut self, time: f32) {
        self.current_time = time.clamp(0.0, self.settings.duration);
    }

    pub fn set_playback_state(&mut self, state: PlaybackState) {
        self.playback_state = state;
    }

    pub fn add_track(&mut self, track_type: TrackType, name: &str) -> u32 {
        let id = self.next_id();
        self.tracks.push(TrackEntry {
            id,
            name: name.to_string(),
            track_type,
            start_time: 0.0,
            duration: self.settings.duration,
            muted: false,
            locked: false,
        });
        id
    }

    pub fn add_camera_keyframe(&mut self, keyframe: CameraKey) {
        self.camera_keyframes.push(keyframe);
        self.camera_keyframes.sort_by(|a, b| {
            a.t.0
                .partial_cmp(&b.t.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn add_marker(&mut self, time: f32, name: &str, color: Color32) {
        self.markers.push(TimelineMarker {
            time,
            name: name.to_string(),
            color,
        });
    }

    pub fn marker_count(&self) -> usize {
        self.markers.len()
    }
}

impl Panel for CinematicsPanel {
    fn name(&self) -> &'static str {
        "Cinematics"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            CinematicsTab::Timeline => self.show_timeline_tab(ui),
            CinematicsTab::Camera => self.show_camera_tab(ui),
            CinematicsTab::Tracks => self.show_tracks_tab(ui),
            CinematicsTab::Clips => self.show_clips_tab(ui),
            CinematicsTab::Preview => self.show_preview_tab(ui),
            CinematicsTab::Export => self.show_export_tab(ui),
        }
    }

    fn update(&mut self) {
        // Update playback
        if self.playback_state == PlaybackState::Playing {
            let dt = 1.0 / 60.0; // Assume 60 FPS update rate
            self.current_time += dt * self.playback_speed.multiplier();

            if self.current_time >= self.settings.duration {
                if self.settings.loop_playback {
                    self.current_time = 0.0;
                } else {
                    self.current_time = self.settings.duration;
                    self.playback_state = PlaybackState::Stopped;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cinematics_panel_creation() {
        let panel = CinematicsPanel::new();
        assert_eq!(panel.playback_state(), PlaybackState::Stopped);
        assert_eq!(panel.current_time(), 0.0);
    }

    #[test]
    fn test_default_tracks() {
        let panel = CinematicsPanel::new();
        assert_eq!(panel.track_count(), 3); // Camera, Animation, Audio
    }

    #[test]
    fn test_default_keyframes() {
        let panel = CinematicsPanel::new();
        assert_eq!(panel.keyframe_count(), 2); // Two sample keyframes
    }

    #[test]
    fn test_playback_state() {
        let mut panel = CinematicsPanel::new();
        panel.set_playback_state(PlaybackState::Playing);
        assert_eq!(panel.playback_state(), PlaybackState::Playing);

        panel.set_playback_state(PlaybackState::Paused);
        assert_eq!(panel.playback_state(), PlaybackState::Paused);
    }

    #[test]
    fn test_current_time() {
        let mut panel = CinematicsPanel::new();
        panel.set_current_time(10.0);
        assert_eq!(panel.current_time(), 10.0);

        // Test clamping
        panel.set_current_time(100.0);
        assert_eq!(panel.current_time(), panel.duration());
    }

    #[test]
    fn test_add_track() {
        let mut panel = CinematicsPanel::new();
        let initial_count = panel.track_count();

        let id = panel.add_track(TrackType::Fx, "Explosion FX");
        assert!(id > 0);
        assert_eq!(panel.track_count(), initial_count + 1);
    }

    #[test]
    fn test_add_camera_keyframe() {
        let mut panel = CinematicsPanel::new();
        let initial_count = panel.keyframe_count();

        panel.add_camera_keyframe(default_camera_key(15.0));

        assert_eq!(panel.keyframe_count(), initial_count + 1);
    }

    #[test]
    fn test_add_marker() {
        let mut panel = CinematicsPanel::new();
        assert_eq!(panel.marker_count(), 0);

        panel.add_marker(5.0, "Start Scene", Color32::RED);
        assert_eq!(panel.marker_count(), 1);
    }

    #[test]
    fn test_playback_speed() {
        assert_eq!(PlaybackSpeed::Normal.multiplier(), 1.0);
        assert_eq!(PlaybackSpeed::Double.multiplier(), 2.0);
        assert_eq!(PlaybackSpeed::Half.multiplier(), 0.5);
    }

    #[test]
    fn test_track_type_properties() {
        assert_eq!(TrackType::Camera.icon(), "📷");
        assert_eq!(TrackType::Audio.icon(), "[Snd]");
        assert_eq!(TrackType::Fx.color(), Color32::from_rgb(186, 85, 211));
    }

    #[test]
    fn test_timeline_settings() {
        let panel = CinematicsPanel::new();
        assert_eq!(panel.duration(), 30.0);
        assert_eq!(panel.framerate(), 30.0);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = CinematicsPanel::new();
        assert_eq!(panel.name(), "Cinematics");
    }

    // ============================================================
    // ROUND 10 ENUM TESTS
    // ============================================================

    // TrackType tests (6 tests)
    #[test]
    fn test_track_type_display() {
        assert_eq!(format!("{}", TrackType::Camera), "📷 Camera");
        assert_eq!(format!("{}", TrackType::Animation), "[Anim] Animation");
        assert_eq!(format!("{}", TrackType::Audio), "[Snd] Audio");
        assert_eq!(format!("{}", TrackType::Fx), "[Fx] VFX");
        assert_eq!(format!("{}", TrackType::Dialogue), "[Chat] Dialogue");
        assert_eq!(format!("{}", TrackType::Event), "[Zap] Event");
    }

    #[test]
    fn test_track_type_all() {
        let all = TrackType::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&TrackType::Camera));
        assert!(all.contains(&TrackType::Event));
    }

    #[test]
    fn test_track_type_name() {
        assert_eq!(TrackType::Camera.name(), "Camera");
        assert_eq!(TrackType::Fx.name(), "VFX");
    }

    #[test]
    fn test_track_type_icon() {
        assert_eq!(TrackType::Camera.icon(), "📷");
        assert_eq!(TrackType::Animation.icon(), "[Anim]");
    }

    #[test]
    fn test_track_type_color() {
        use egui::Color32;
        let color = TrackType::Camera.color();
        assert!(color != Color32::BLACK);
    }

    #[test]
    fn test_track_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for t in TrackType::all() {
            assert!(set.insert(*t));
        }
        assert_eq!(set.len(), 6);
    }

    // CameraInterpolation tests (6 tests)
    #[test]
    fn test_camera_interpolation_display() {
        assert_eq!(format!("{}", CameraInterpolation::Linear), "Linear");
        assert_eq!(
            format!("{}", CameraInterpolation::CatmullRom),
            "Catmull-Rom"
        );
        assert_eq!(format!("{}", CameraInterpolation::Bezier), "Bezier");
        assert_eq!(format!("{}", CameraInterpolation::Hermite), "Hermite");
        assert_eq!(format!("{}", CameraInterpolation::Step), "Step");
    }

    #[test]
    fn test_camera_interpolation_all() {
        let all = CameraInterpolation::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&CameraInterpolation::Linear));
        assert!(all.contains(&CameraInterpolation::Step));
    }

    #[test]
    fn test_camera_interpolation_name() {
        assert_eq!(CameraInterpolation::Linear.name(), "Linear");
        assert_eq!(CameraInterpolation::CatmullRom.name(), "Catmull-Rom");
    }

    #[test]
    fn test_camera_interpolation_is_smooth() {
        assert!(CameraInterpolation::Linear.is_smooth());
        assert!(CameraInterpolation::CatmullRom.is_smooth());
        assert!(CameraInterpolation::Bezier.is_smooth());
        assert!(!CameraInterpolation::Step.is_smooth());
    }

    #[test]
    fn test_camera_interpolation_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for c in CameraInterpolation::all() {
            assert!(set.insert(*c));
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_camera_interpolation_smooth_types() {
        let smooth_types: Vec<_> = CameraInterpolation::all()
            .iter()
            .filter(|c| c.is_smooth())
            .collect();
        assert_eq!(smooth_types.len(), 4);
    }

    // PlaybackState tests (7 tests)
    #[test]
    fn test_playback_state_display() {
        assert_eq!(format!("{}", PlaybackState::Stopped), "[] Stopped");
        assert_eq!(format!("{}", PlaybackState::Playing), "> Playing");
        assert_eq!(format!("{}", PlaybackState::Paused), "|| Paused");
        assert_eq!(format!("{}", PlaybackState::Recording), "⏺️ Recording");
    }

    #[test]
    fn test_playback_state_name() {
        assert_eq!(PlaybackState::Stopped.name(), "Stopped");
        assert_eq!(PlaybackState::Playing.name(), "Playing");
        assert_eq!(PlaybackState::Paused.name(), "Paused");
        assert_eq!(PlaybackState::Recording.name(), "Recording");
    }

    #[test]
    fn test_playback_state_icon() {
        assert_eq!(PlaybackState::Stopped.icon(), "[]");
        assert_eq!(PlaybackState::Playing.icon(), ">");
        assert_eq!(PlaybackState::Paused.icon(), "||");
        assert_eq!(PlaybackState::Recording.icon(), "⏺️");
    }

    #[test]
    fn test_playback_state_is_running() {
        assert!(!PlaybackState::Stopped.is_running());
        assert!(PlaybackState::Playing.is_running());
        assert!(!PlaybackState::Paused.is_running());
        assert!(PlaybackState::Recording.is_running());
    }

    #[test]
    fn test_playback_state_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PlaybackState::Stopped);
        set.insert(PlaybackState::Playing);
        set.insert(PlaybackState::Paused);
        set.insert(PlaybackState::Recording);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_playback_state_running_states() {
        let running: Vec<_> = [
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Recording,
        ]
        .iter()
        .filter(|s| s.is_running())
        .collect();
        assert_eq!(running.len(), 2);
    }

    #[test]
    fn test_playback_state_default() {
        assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
    }

    // PlaybackSpeed tests (7 tests)
    #[test]
    fn test_playback_speed_display() {
        assert_eq!(format!("{}", PlaybackSpeed::Quarter), "0.25×");
        assert_eq!(format!("{}", PlaybackSpeed::Half), "0.5×");
        assert_eq!(format!("{}", PlaybackSpeed::Normal), "1×");
        assert_eq!(format!("{}", PlaybackSpeed::Double), "2×");
        assert_eq!(format!("{}", PlaybackSpeed::Quadruple), "4×");
    }

    #[test]
    fn test_playback_speed_all() {
        let all = PlaybackSpeed::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&PlaybackSpeed::Normal));
    }

    #[test]
    fn test_playback_speed_multiplier() {
        assert_eq!(PlaybackSpeed::Quarter.multiplier(), 0.25);
        assert_eq!(PlaybackSpeed::Half.multiplier(), 0.5);
        assert_eq!(PlaybackSpeed::Normal.multiplier(), 1.0);
        assert_eq!(PlaybackSpeed::Double.multiplier(), 2.0);
        assert_eq!(PlaybackSpeed::Quadruple.multiplier(), 4.0);
    }

    #[test]
    fn test_playback_speed_display_method() {
        assert_eq!(PlaybackSpeed::Quarter.display(), "0.25×");
        assert_eq!(PlaybackSpeed::Normal.display(), "1×");
    }

    #[test]
    fn test_playback_speed_default() {
        assert_eq!(PlaybackSpeed::default(), PlaybackSpeed::Normal);
    }

    #[test]
    fn test_playback_speed_ordering() {
        assert!(PlaybackSpeed::Quarter.multiplier() < PlaybackSpeed::Half.multiplier());
        assert!(PlaybackSpeed::Normal.multiplier() < PlaybackSpeed::Double.multiplier());
    }

    #[test]
    fn test_playback_speed_range() {
        let min = PlaybackSpeed::Quarter.multiplier();
        let max = PlaybackSpeed::Quadruple.multiplier();
        assert_eq!(min, 0.25);
        assert_eq!(max, 4.0);
        assert_eq!(max / min, 16.0);
    }

    // ClipData tests (6 tests)
    #[test]
    fn test_clip_data_display() {
        assert_eq!(
            format!(
                "{}",
                ClipData::Camera {
                    keyframes: Vec::new()
                }
            ),
            "Camera"
        );
        assert_eq!(
            format!(
                "{}",
                ClipData::Animation {
                    target_id: 1,
                    clip_name: "test".to_string()
                }
            ),
            "Animation"
        );
        assert_eq!(
            format!(
                "{}",
                ClipData::Audio {
                    file: "test.wav".to_string(),
                    volume: 1.0,
                    fade_in: 0.0,
                    fade_out: 0.0
                }
            ),
            "Audio"
        );
        assert_eq!(
            format!(
                "{}",
                ClipData::Fx {
                    effect_name: "fire".to_string(),
                    params: "{}".to_string()
                }
            ),
            "VFX"
        );
        assert_eq!(
            format!(
                "{}",
                ClipData::Dialogue {
                    speaker: "NPC".to_string(),
                    text: "Hello".to_string(),
                    duration: 2.0
                }
            ),
            "Dialogue"
        );
        assert_eq!(
            format!(
                "{}",
                ClipData::Event {
                    event_name: "trigger".to_string(),
                    payload: "{}".to_string()
                }
            ),
            "Event"
        );
    }

    #[test]
    fn test_clip_data_name() {
        assert_eq!(
            ClipData::Camera {
                keyframes: Vec::new()
            }
            .name(),
            "Camera"
        );
        assert_eq!(
            ClipData::Animation {
                target_id: 1,
                clip_name: "walk".to_string()
            }
            .name(),
            "Animation"
        );
        assert_eq!(
            ClipData::Audio {
                file: "music.mp3".to_string(),
                volume: 0.8,
                fade_in: 1.0,
                fade_out: 1.0
            }
            .name(),
            "Audio"
        );
        assert_eq!(
            ClipData::Fx {
                effect_name: "explosion".to_string(),
                params: "{}".to_string()
            }
            .name(),
            "VFX"
        );
        assert_eq!(
            ClipData::Dialogue {
                speaker: "Hero".to_string(),
                text: "..".to_string(),
                duration: 1.0
            }
            .name(),
            "Dialogue"
        );
        assert_eq!(
            ClipData::Event {
                event_name: "spawn".to_string(),
                payload: "{}".to_string()
            }
            .name(),
            "Event"
        );
    }

    #[test]
    fn test_clip_data_default() {
        let default = ClipData::default();
        assert_eq!(default.name(), "Camera");
    }

    #[test]
    fn test_clip_data_camera_variant() {
        let clip = ClipData::Camera {
            keyframes: vec![default_camera_key(0.0)],
        };
        assert_eq!(clip.name(), "Camera");
        if let ClipData::Camera { keyframes } = clip {
            assert_eq!(keyframes.len(), 1);
        } else {
            panic!("Expected Camera variant");
        }
    }

    #[test]
    fn test_clip_data_audio_variant() {
        let clip = ClipData::Audio {
            file: "bgm.ogg".to_string(),
            volume: 0.7,
            fade_in: 2.0,
            fade_out: 3.0,
        };
        assert_eq!(clip.name(), "Audio");
        if let ClipData::Audio {
            file,
            volume,
            fade_in,
            fade_out,
        } = clip
        {
            assert_eq!(file, "bgm.ogg");
            assert_eq!(volume, 0.7);
            assert_eq!(fade_in, 2.0);
            assert_eq!(fade_out, 3.0);
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_clip_data_all_variants() {
        let variants = vec![
            ClipData::Camera {
                keyframes: Vec::new(),
            },
            ClipData::Animation {
                target_id: 1,
                clip_name: String::new(),
            },
            ClipData::Audio {
                file: String::new(),
                volume: 1.0,
                fade_in: 0.0,
                fade_out: 0.0,
            },
            ClipData::Fx {
                effect_name: String::new(),
                params: String::new(),
            },
            ClipData::Dialogue {
                speaker: String::new(),
                text: String::new(),
                duration: 0.0,
            },
            ClipData::Event {
                event_name: String::new(),
                payload: String::new(),
            },
        ];
        assert_eq!(variants.len(), 6);
    }

    // CinematicsTab tests (6 tests)
    #[test]
    fn test_cinematics_tab_display() {
        assert_eq!(format!("{}", CinematicsTab::Timeline), "📅 Timeline");
        assert_eq!(format!("{}", CinematicsTab::Camera), "📷 Camera");
        assert_eq!(format!("{}", CinematicsTab::Tracks), "[Anim] Tracks");
        assert_eq!(format!("{}", CinematicsTab::Clips), "🎞️ Clips");
        assert_eq!(format!("{}", CinematicsTab::Preview), "👁️ Preview");
        assert_eq!(format!("{}", CinematicsTab::Export), "[Save] Export");
    }

    #[test]
    fn test_cinematics_tab_all() {
        let all = CinematicsTab::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&CinematicsTab::Timeline));
        assert!(all.contains(&CinematicsTab::Export));
    }

    #[test]
    fn test_cinematics_tab_name() {
        assert_eq!(CinematicsTab::Timeline.name(), "Timeline");
        assert_eq!(CinematicsTab::Camera.name(), "Camera");
        assert_eq!(CinematicsTab::Export.name(), "Export");
    }

    #[test]
    fn test_cinematics_tab_icon() {
        assert_eq!(CinematicsTab::Timeline.icon(), "📅");
        assert_eq!(CinematicsTab::Camera.icon(), "📷");
        assert_eq!(CinematicsTab::Preview.icon(), "👁️");
    }

    #[test]
    fn test_cinematics_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in CinematicsTab::all() {
            assert!(set.insert(*tab));
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_cinematics_tab_default() {
        assert_eq!(CinematicsTab::default(), CinematicsTab::Timeline);
    }

    // CinematicsAction tests
    #[test]
    fn test_cinematics_action_playback() {
        let play = CinematicsAction::Play;
        let pause = CinematicsAction::Pause;
        let stop = CinematicsAction::Stop;
        assert!(matches!(play, CinematicsAction::Play));
        assert!(matches!(pause, CinematicsAction::Pause));
        assert!(matches!(stop, CinematicsAction::Stop));
    }

    #[test]
    fn test_cinematics_action_seek() {
        let action = CinematicsAction::SeekTo(5.5);
        if let CinematicsAction::SeekTo(time) = action {
            assert!((time - 5.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected SeekTo action");
        }
    }

    #[test]
    fn test_cinematics_action_set_speed() {
        let action = CinematicsAction::SetSpeed(PlaybackSpeed::Double);
        assert!(matches!(
            action,
            CinematicsAction::SetSpeed(PlaybackSpeed::Double)
        ));
    }

    #[test]
    fn test_cinematics_action_add_track() {
        let action = CinematicsAction::AddTrack(TrackType::Camera);
        assert!(matches!(
            action,
            CinematicsAction::AddTrack(TrackType::Camera)
        ));
    }

    #[test]
    fn test_cinematics_action_remove_track() {
        let action = CinematicsAction::RemoveTrack { track_id: 42 };
        if let CinematicsAction::RemoveTrack { track_id } = action {
            assert_eq!(track_id, 42);
        } else {
            panic!("Expected RemoveTrack action");
        }
    }

    #[test]
    fn test_cinematics_action_add_clip() {
        let action = CinematicsAction::AddClip {
            track_id: 1,
            start_time: 2.5,
        };
        if let CinematicsAction::AddClip {
            track_id,
            start_time,
        } = action
        {
            assert_eq!(track_id, 1);
            assert!((start_time - 2.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected AddClip action");
        }
    }

    #[test]
    fn test_cinematics_action_move_clip() {
        let action = CinematicsAction::MoveClip {
            clip_id: 5,
            new_time: 10.0,
        };
        if let CinematicsAction::MoveClip { clip_id, new_time } = action {
            assert_eq!(clip_id, 5);
            assert!((new_time - 10.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected MoveClip action");
        }
    }

    #[test]
    fn test_cinematics_action_camera_keyframe() {
        let add = CinematicsAction::AddCameraKeyframe { time: 3.0 };
        let remove = CinematicsAction::RemoveCameraKeyframe { index: 2 };

        if let CinematicsAction::AddCameraKeyframe { time } = add {
            assert!((time - 3.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected AddCameraKeyframe");
        }

        if let CinematicsAction::RemoveCameraKeyframe { index } = remove {
            assert_eq!(index, 2);
        } else {
            panic!("Expected RemoveCameraKeyframe");
        }
    }

    #[test]
    fn test_cinematics_action_camera_interpolation() {
        let action = CinematicsAction::SetCameraInterpolation(CameraInterpolation::Bezier);
        assert!(matches!(
            action,
            CinematicsAction::SetCameraInterpolation(CameraInterpolation::Bezier)
        ));
    }

    #[test]
    fn test_cinematics_action_marker() {
        let add = CinematicsAction::AddMarker {
            time: 5.0,
            name: "Start".to_string(),
        };
        let remove = CinematicsAction::RemoveMarker { index: 0 };

        if let CinematicsAction::AddMarker { time, name } = add {
            assert!((time - 5.0).abs() < f32::EPSILON);
            assert_eq!(name, "Start");
        } else {
            panic!("Expected AddMarker");
        }

        assert!(matches!(
            remove,
            CinematicsAction::RemoveMarker { index: 0 }
        ));
    }

    #[test]
    fn test_cinematics_action_export() {
        let action = CinematicsAction::Export {
            format: "mp4".to_string(),
        };
        if let CinematicsAction::Export { format } = action {
            assert_eq!(format, "mp4");
        } else {
            panic!("Expected Export action");
        }
    }

    #[test]
    fn test_cinematics_action_queue_and_take() {
        let mut panel = CinematicsPanel::new();
        assert!(!panel.has_pending_actions());

        panel.queue_action(CinematicsAction::Play);
        panel.queue_action(CinematicsAction::SeekTo(2.0));
        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_cinematics_action_equality() {
        let a1 = CinematicsAction::Stop;
        let a2 = CinematicsAction::Stop;
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_cinematics_action_debug() {
        let action = CinematicsAction::ToggleCameraPreview(true);
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("ToggleCameraPreview"));
    }
}

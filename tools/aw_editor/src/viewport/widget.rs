//! Viewport Widget
//!
//! Custom egui widget that integrates wgpu 3D rendering into editor panels.
//! Handles input, rendering coordination, and egui integration.

#![allow(dead_code)]
//!
//! # Usage
//!
//! ```rust,ignore
//! use aw_editor_lib::viewport::ViewportWidget;
//!
//! // In eframe::App::new()
//! let viewport = ViewportWidget::new(cc)?;
//!
//! // In eframe::App::update()
//! viewport.ui(ui, &world)?;
//! ```
//!
//! # Architecture
//!
//! ViewportWidget is the glue between egui (UI) and wgpu (3D rendering):
//! - Allocates egui space for viewport
//! - Handles mouse/keyboard input
//! - Coordinates rendering (via ViewportRenderer)
//! - Uses egui_wgpu::Callback for custom rendering
//!
//! # Performance
//!
//! Target: 16.67ms per frame (60 FPS)
//! - Input handling: <0.1ms
//! - Rendering: <10ms (see ViewportRenderer)
//! - egui integration: <1ms
//! - Total: <12ms (26% headroom)

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tracing::{debug, trace, warn};

use super::camera::OrbitCamera;
use super::renderer::ViewportRenderer;
use super::toolbar::{GridType, ViewportToolbar};
use crate::entity_manager::EntityManager;
use crate::gizmo::{
    AxisConstraint, GizmoHandle, GizmoMode, GizmoPicker, GizmoState, RotateGizmo, ScaleGizmo,
    TransformSnapshot,
};
use astraweave_core::{Entity, World};

/// Layout mode for multi-viewport splitting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportLayout {
    /// Single full-size viewport
    Single,
    /// Two viewports, side by side (left | right)
    SideBySide,
    /// Two viewports, stacked (top / bottom)
    TopBottom,
    /// Four viewports in a 2×2 grid
    Quad,
}

impl Default for ViewportLayout {
    fn default() -> Self {
        Self::Single
    }
}

impl ViewportLayout {
    /// Number of viewports this layout needs
    pub fn viewport_count(&self) -> usize {
        match self {
            Self::Single => 1,
            Self::SideBySide | Self::TopBottom => 2,
            Self::Quad => 4,
        }
    }

    /// Label for UI display
    pub fn label(&self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::SideBySide => "Side by Side",
            Self::TopBottom => "Top / Bottom",
            Self::Quad => "Quad (2\u{00d7}2)",
        }
    }
}

/// Camera bookmark for F1-F12 quick recall
#[derive(Clone, Debug)]
struct CameraBookmark {
    focal_point: glam::Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

/// 3D viewport widget for egui
///
/// Integrates wgpu 3D rendering into egui panel system.
/// Manages camera, rendering, and input handling.
pub struct ViewportWidget {
    /// Viewport renderer (wgpu coordinator)
    renderer: Arc<Mutex<ViewportRenderer>>,

    /// Orbit camera controller
    camera: OrbitCamera,

    /// Render texture (reused each frame)
    render_texture: Option<Arc<wgpu::Texture>>,

    /// egui-wgpu renderer handle for native texture registration (GPU-direct display)
    egui_wgpu_renderer: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,

    /// Registered native texture ID (avoids CPU readback entirely)
    native_texture_id: Option<egui::TextureId>,

    /// Last viewport size (for resize detection)
    last_size: (u32, u32),

    /// Whether viewport has focus (for input handling)
    has_focus: bool,

    /// Viewport toolbar
    toolbar: ViewportToolbar,

    /// Currently selected entities (supports multi-selection)
    selected_entities: Vec<crate::entity_manager::EntityId>,

    /// Track if left mouse button was pressed (for click detection)
    mouse_pressed_pos: Option<egui::Pos2>,

    /// Frame time tracking for FPS calculation (exponential moving average)
    last_frame_time: std::time::Instant,
    fps_ema: f32,

    /// Gizmo state (for transform manipulation)
    gizmo_state: GizmoState,

    /// Gizmo picker for hover detection
    gizmo_picker: GizmoPicker,

    /// Currently hovered gizmo handle (for visual highlighting)
    hovered_handle: Option<GizmoHandle>,

    /// Grid snap size (1.0 = snap to integer grid)
    grid_snap_size: f32,

    /// Angle snap increment in radians (default: 15° = 0.2617994 rad)
    angle_snap_increment: f32,

    /// Camera bookmarks (F1-F12)
    camera_bookmarks: [Option<CameraBookmark>; 12],

    /// Clipboard for copy/paste operations
    clipboard: Option<crate::clipboard::ClipboardData>,

    /// Cached entity count for dirty tracking (skip HashMap rebuild when unchanged)
    cached_entity_count: usize,

    /// Whether terrain brush mode is active (set externally)
    terrain_brush_active: bool,

    /// Brush radius for cursor visualization
    terrain_brush_radius: f32,
    /// Whether the active brush is a paint brush (blue) vs sculpt (green)
    terrain_brush_is_paint: bool,

    /// Queued terrain brush hits (world X, Z) from mouse clicks in viewport
    terrain_brush_hits: Vec<[f32; 2]>,

    /// Whether a terrain brush stroke just ended (mouse released)
    terrain_brush_stroke_ended: bool,

    /// Throttle: last time a brush hit was emitted (limits to ~15 Hz during drag)
    last_brush_time: std::time::Instant,

    /// Drag offset: the XZ distance between entity center and initial click point.
    /// Stored on drag start to prevent center-snap when clicking off-center.
    drag_offset: Option<glam::Vec3>,

    /// The Y height of the drag plane, captured at drag start.
    /// Keeps dragging stable on a flat plane instead of intersecting uneven terrain.
    drag_plane_y: f32,

    /// Game input captured this frame (populated during play mode)
    pending_game_input: Option<crate::runtime::GameInput>,

    /// Last viewport rect (for ground-plane raycasting from outside the show() method)
    last_viewport_rect: Option<egui::Rect>,

    /// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: cached discrete
    /// events captured during ui()/handle_input. Drained by
    /// [`Self::dispatch_cached_events`] (called from main.rs once per frame
    /// after viewport.ui() returns). Each entry is (event, kind); main.rs
    /// constructs ToolContext + iterates the cache + calls
    /// `dispatcher.dispatch_mouse_event(event, kind, &mut tool_context)`.
    ///
    /// Per Andrew Q3 default + cached-then-dispatch design rationale:
    /// avoids threading `&mut Dispatcher` through 5 ui() call sites;
    /// preserves additive coexistence with existing main.rs:3833-3877
    /// mediator path; isolates dispatcher integration to a single new
    /// accessor.
    cached_mouse_events: Vec<(crate::active_tool::MouseEvent, crate::active_tool::MouseEventKind)>,
    /// Cached mouse-enter notification (true if pointer entered viewport this frame).
    cached_mouse_enter: bool,
    /// Cached mouse-leave notification (true if pointer left viewport this frame).
    cached_mouse_leave: bool,
    /// Cached pre-computed ToolContext fields for dispatch_cached_events to construct
    /// ToolContext at dispatch time (per Sub-phase 2 §2.7 resolution: ViewportWidget
    /// pre-computes world-XZ projections; ToolContext stores cached values).
    cached_pointer_pos: Option<egui::Pos2>,
    cached_modifiers: egui::Modifiers,
    cached_world_xz_at_pointer: Option<(f32, f32)>,
    cached_world_xz_at_y0: Option<(f32, f32)>,
    /// Whether this frame produced a hovered + focused viewport context worth dispatching.
    /// false: dispatch_cached_events is a no-op (cache may be stale or empty).
    cached_hovered_this_frame: bool,
}

impl ViewportWidget {
    /// Create new viewport widget
    ///
    /// # Arguments
    ///
    /// * `cc` - eframe creation context (contains wgpu render state)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - wgpu render state is missing
    /// - Renderer creation fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use anyhow::Result;
    /// use aw_editor_lib::viewport::ViewportWidget;
    ///
    /// struct EditorApp {
    ///     viewport: ViewportWidget,
    /// }
    ///
    /// impl EditorApp {
    ///     fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
    ///         Ok(Self { viewport: ViewportWidget::new(cc)? })
    ///     }
    /// }
    /// ```
    pub fn new(cc: &eframe::CreationContext) -> Result<Self> {
        // Get wgpu render state from eframe
        let render_state = cc.wgpu_render_state.as_ref().context(
            "wgpu render state not available - ensure eframe is built with 'wgpu' feature",
        )?;

        // Create renderer (wrapped in Arc<Mutex<>> for thread-safe interior mutability)
        let renderer = Arc::new(Mutex::new({
            let mut r = ViewportRenderer::from_eframe(render_state)
                .context("Failed to create viewport renderer")?;
            // Eagerly initialize all sub-renderers to avoid frame hitches during editing
            r.eagerly_init_all();
            r
        }));

        // Store egui-wgpu renderer handle for native texture registration
        let egui_wgpu_renderer = render_state.renderer.clone();

        Ok(Self {
            renderer,
            camera: OrbitCamera::default(),
            render_texture: None,
            egui_wgpu_renderer,
            native_texture_id: None,
            last_size: (0, 0),
            has_focus: false,
            toolbar: ViewportToolbar::default(),
            selected_entities: Vec::new(),
            mouse_pressed_pos: None,
            last_frame_time: std::time::Instant::now(),
            fps_ema: 0.0,
            gizmo_state: GizmoState::new(),
            gizmo_picker: GizmoPicker::default(),
            hovered_handle: None,
            grid_snap_size: 1.0,
            angle_snap_increment: 15.0_f32.to_radians(),
            camera_bookmarks: [
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            clipboard: None,
            cached_entity_count: 0,
            terrain_brush_active: false,
            terrain_brush_radius: 5.0,
            terrain_brush_is_paint: false,
            terrain_brush_hits: Vec::new(),
            terrain_brush_stroke_ended: false,
            last_brush_time: std::time::Instant::now(),
            drag_offset: None,
            drag_plane_y: 0.0,
            pending_game_input: None,
            last_viewport_rect: None,
            // Sub-phase 3 cached-then-dispatch fields:
            cached_mouse_events: Vec::new(),
            cached_mouse_enter: false,
            cached_mouse_leave: false,
            cached_pointer_pos: None,
            cached_modifiers: egui::Modifiers::NONE,
            cached_world_xz_at_pointer: None,
            cached_world_xz_at_y0: None,
            cached_hovered_this_frame: false,
        })
    }

    /// Create a new viewport widget that shares GPU resources with an existing one.
    ///
    /// The new viewport has its own camera, gizmo state, and selection,
    /// but shares the underlying wgpu device and queue.
    pub fn new_additional(existing: &ViewportWidget) -> Result<Self> {
        let (device, queue) = {
            let renderer = existing
                .renderer
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock renderer: {}", e))?;
            (renderer.device().clone(), renderer.queue().clone())
        };

        let renderer = Arc::new(Mutex::new(
            ViewportRenderer::new(device, queue)
                .context("Failed to create additional viewport renderer")?,
        ));

        Ok(Self {
            renderer,
            camera: OrbitCamera::default(),
            render_texture: None,
            egui_wgpu_renderer: existing.egui_wgpu_renderer.clone(),
            native_texture_id: None,
            last_size: (0, 0),
            has_focus: false,
            toolbar: ViewportToolbar::default(),
            selected_entities: Vec::new(),
            mouse_pressed_pos: None,
            last_frame_time: std::time::Instant::now(),
            fps_ema: 0.0,
            gizmo_state: GizmoState::new(),
            gizmo_picker: GizmoPicker::default(),
            hovered_handle: None,
            grid_snap_size: 1.0,
            angle_snap_increment: 15.0_f32.to_radians(),
            camera_bookmarks: [
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            clipboard: None,
            cached_entity_count: 0,
            terrain_brush_active: false,
            terrain_brush_radius: 5.0,
            terrain_brush_is_paint: false,
            terrain_brush_hits: Vec::new(),
            terrain_brush_stroke_ended: false,
            last_brush_time: std::time::Instant::now(),
            drag_offset: None,
            drag_plane_y: 0.0,
            pending_game_input: None,
            last_viewport_rect: None,
            // Sub-phase 3 cached-then-dispatch fields:
            cached_mouse_events: Vec::new(),
            cached_mouse_enter: false,
            cached_mouse_leave: false,
            cached_pointer_pos: None,
            cached_modifiers: egui::Modifiers::NONE,
            cached_world_xz_at_pointer: None,
            cached_world_xz_at_y0: None,
            cached_hovered_this_frame: false,
        })
    }

    /// Lock the renderer, recovering from mutex poison if needed.
    /// On poison, returns None to avoid using potentially corrupted GPU state.
    fn with_renderer<F, R>(&self, op: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut ViewportRenderer) -> R,
    {
        match self.renderer.lock() {
            Ok(mut renderer) => Some(f(&mut renderer)),
            Err(_poisoned) => {
                tracing::error!("Renderer mutex poisoned during '{op}' — operation skipped (GPU state may be corrupted)");
                None
            }
        }
    }

    /// Set the selection count for display in viewport HUD
    pub fn set_selection_count(&mut self, count: usize) {
        self.toolbar.stats.selection_count = count;
    }

    /// Enable/disable terrain brush mode for viewport mouse interaction
    pub fn set_terrain_brush_active(&mut self, active: bool) {
        self.terrain_brush_active = active;
    }

    /// Update brush cursor parameters for visualization
    pub fn set_terrain_brush_params(&mut self, radius: f32, is_paint: bool) {
        self.terrain_brush_radius = radius;
        self.terrain_brush_is_paint = is_paint;
    }

    /// Take all pending terrain brush hits (world X, Z coordinates)
    pub fn take_terrain_brush_hits(&mut self) -> Vec<[f32; 2]> {
        std::mem::take(&mut self.terrain_brush_hits)
    }

    /// Returns true if a terrain brush stroke just ended (mouse released).
    /// Resets the flag after reading.
    pub fn take_terrain_brush_stroke_ended(&mut self) -> bool {
        std::mem::take(&mut self.terrain_brush_stroke_ended)
    }

    /// Take captured game input (returns None if not in play mode or no input this frame)
    pub fn take_game_input(&mut self) -> Option<crate::runtime::GameInput> {
        self.pending_game_input.take()
    }

    /// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: populate cached
    /// discrete events + ToolContext fields from the current frame's
    /// `egui::Response`. Called from `Self::ui` after `handle_input` returns;
    /// cached values are drained by [`Self::dispatch_cached_events`] (called
    /// from main.rs's per-frame update loop).
    ///
    /// Per Sub-phase 2 §2.7 resolution: pre-computes both world-XZ projections
    /// (`world_xz_at_pointer` via depth-buffer + camera unprojection;
    /// `world_xz_at_y0` via ray-plane intersection at Y=0). Tools that need
    /// either projection read via `ToolContext::world_xz_at_pointer()` /
    /// `world_xz_at_y0()` method accessors at dispatch time.
    fn cache_active_tool_events(
        &mut self,
        response: &egui::Response,
        ctx: &egui::Context,
        viewport_size: egui::Vec2,
    ) {
        // Reset cache to current frame state.
        self.cached_mouse_events.clear();
        self.cached_mouse_enter = false;
        self.cached_mouse_leave = false;
        self.cached_pointer_pos = response.hover_pos();
        self.cached_modifiers = ctx.input(|i| i.modifiers);
        self.cached_hovered_this_frame = response.hovered();

        // Pre-compute world-XZ projections per Sub-phase 2 §2.7.
        // depth-buffer projection (existing pattern at viewport/widget.rs:1219-1234):
        // Sub-phase 3 design choice: skip depth-buffer pre-compute in this method;
        // the renderer lock contention + per-frame cost are non-trivial. Instead,
        // dispatch_cached_events (called from main.rs) reads depth on-demand for
        // tools that actually need it. For Sub-phase 3 (additive coexistence;
        // dispatcher path is wired-but-inert), leaving these as None matches the
        // dispatcher-path's no-op semantics.
        //
        // Sub-phase 5 part A: the depth-buffer projection stays deferred — RAP
        // paint uses the Y=0 plane, and TerrainPanel reads depth via its
        // still-live mediator path (not the dispatcher). Wire the Y=0 ray-plane
        // projection LIVE so the ToolContext built in `dispatch_cached_events`
        // carries real world-XZ for the active paint tool (was hardwired `None`,
        // so paint received nothing). Uses the camera's authoritative
        // `ray_from_screen` projection intersected with the world Y=0 plane —
        // the same projection `RegionalArchetypePanel::screen_to_world_xz_y0`
        // represents, read by tools via `ToolContext::world_xz_at_y0()`.
        self.cached_world_xz_at_pointer = None;
        self.cached_world_xz_at_y0 = self.cached_pointer_pos.and_then(|abs| {
            let local = egui::Pos2 {
                x: abs.x - response.rect.min.x,
                y: abs.y - response.rect.min.y,
            };
            let ray = self.camera.ray_from_screen(local, viewport_size);
            // Y=0 plane intersection: require the ray to point downward.
            if ray.direction.y.abs() < 1e-6 || ray.direction.y >= 0.0 {
                return None;
            }
            let t = -ray.origin.y / ray.direction.y;
            if t <= 0.0 {
                return None;
            }
            let hit = ray.origin + ray.direction * t;
            Some((hit.x, hit.z))
        });

        // Cache discrete events from response.
        let pointer_pos = self.cached_pointer_pos.unwrap_or(egui::Pos2::ZERO);
        let drag_delta = response.drag_delta();
        let modifiers = self.cached_modifiers;
        let mouse_event = crate::active_tool::MouseEvent {
            pointer_pos,
            modifiers,
            drag_delta,
        };

        // Discrete event detection from egui Response API.
        if response.clicked_by(egui::PointerButton::Primary) {
            // Treat single-click as LeftButtonDown + LeftButtonUp pair within the same frame.
            self.cached_mouse_events.push((mouse_event, crate::active_tool::MouseEventKind::LeftButtonDown));
            self.cached_mouse_events.push((mouse_event, crate::active_tool::MouseEventKind::LeftButtonUp));
        } else if response.dragged_by(egui::PointerButton::Primary) {
            // Continuous drag → emit Move events each frame the drag is active.
            self.cached_mouse_events.push((mouse_event, crate::active_tool::MouseEventKind::Move));
        } else if response.drag_stopped_by(egui::PointerButton::Primary) {
            // Drag-end → emit LeftButtonUp.
            self.cached_mouse_events.push((mouse_event, crate::active_tool::MouseEventKind::LeftButtonUp));
        }

        // Note: cached_mouse_enter / cached_mouse_leave detection is deferred —
        // requires frame-to-frame state diff (was_hovered_last_frame vs hovered_this_frame).
        // Sub-phase 5 may add if RegionalArchetypePanel needs hover lifecycle.

        let _ = viewport_size; // reserved for Sub-phase 5 / Mediator Removal session world-XZ pre-compute
    }

    /// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: drain cached
    /// events + dispatch to dispatcher's active tool. Called from main.rs's
    /// per-frame update loop AFTER `viewport.ui()` returns.
    ///
    /// Constructs `ToolContext` from cached values + iterates cached events.
    /// Per Andrew Q3 default: dispatcher returns `PassThrough` when no tool
    /// is active, so this method is safe to call unconditionally.
    ///
    /// Per Sub-phase 3 §0.1: dispatch is unconditional; ViewportWidget doesn't
    /// gate on active-tool state. Caller (main.rs) doesn't gate either; the
    /// dispatcher's own `active_tool: Option<Uuid>` mutex check is the
    /// arbitration point.
    pub fn dispatch_cached_events(&mut self, dispatcher: &mut crate::active_tool::Dispatcher) {
        if !self.cached_hovered_this_frame || self.cached_mouse_events.is_empty() {
            return;
        }

        let viewport_rect = self
            .last_viewport_rect
            .unwrap_or_else(|| egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(0.0, 0.0)));

        let mut tool_context = crate::active_tool::ToolContext::new(
            viewport_rect,
            self.cached_pointer_pos,
            self.cached_modifiers,
            self.cached_world_xz_at_pointer,
            self.cached_world_xz_at_y0,
        );

        // Drain cached events; dispatch each to active tool via dispatcher.
        let events = std::mem::take(&mut self.cached_mouse_events);
        for (event, kind) in events {
            let _disposition = dispatcher.dispatch_mouse_event(&event, kind, &mut tool_context);
            // Disposition is informational during Sub-phase 3 (additive coexistence per Q2);
            // existing terrain_brush_active-branched code at viewport/widget.rs:1180-1255
            // still runs unchanged. Mediator Removal session will gate on disposition.
        }
    }

    /// Get access to the underlying renderer
    pub fn renderer(&self) -> &Arc<Mutex<ViewportRenderer>> {
        &self.renderer
    }

    /// Render viewport and handle input
    ///
    /// # Arguments
    ///
    /// * `ui` - egui UI context
    /// * `world` - Game world (for entity data)
    /// * `entity_manager` - Entity manager (for transforms and picking)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// impl eframe::App for EditorApp {
    ///     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    ///         egui::CentralPanel::default().show(ctx, |ui| {
    ///             // App::update can't return Result, so handle errors explicitly.
    ///             if let Err(err) = self.viewport.ui(
    ///                 ui,
    ///                 &mut self.world,
    ///                 &mut self.entity_manager,
    ///                 &mut self.undo_stack,
    ///                 self.prefab_manager.as_mut(),
    ///             ) {
    ///                 tracing::error!("Viewport error: {err:#}");
    ///             }
    ///         });
    ///     }
    /// }
    /// ```
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        world: &mut World,
        entity_manager: &mut EntityManager,
        undo_stack: &mut crate::command::UndoStack, // Phase 2.1: Command integration
        opt_prefab_mgr: Option<&mut crate::prefab::PrefabManager>, // Phase 8.1 Week 5 Day 3: Auto-tracking
        is_playing: bool, // When true, capture game input alongside camera controls
    ) -> Result<()> {
        // Update frame time tracking (exponential moving average, O(1))
        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        // EMA with α=0.05 (~20-frame smoothing window)
        if self.fps_ema <= 0.0 {
            self.fps_ema = frame_time; // seed with first sample
        } else {
            self.fps_ema = self.fps_ema * 0.95 + frame_time * 0.05;
        }
        let fps = if self.fps_ema > 0.0 {
            1.0 / self.fps_ema
        } else {
            0.0
        };

        // Allocate space for viewport (full available space)
        let available = ui.available_size();
        let viewport_size = egui::vec2(available.x, available.y);
        let (rect, response) = ui.allocate_exact_size(viewport_size, egui::Sense::click_and_drag());

        // Store viewport rect for external ground-position queries
        self.last_viewport_rect = Some(rect);

        // Request focus only on click (not hover) to avoid stealing focus from other panels
        if response.clicked() {
            trace!(
                hovered = response.hovered(),
                clicked = response.clicked(),
                "Viewport interaction"
            );
            response.request_focus();
        }

        // Update focus state
        self.has_focus = response.has_focus();

        // Debug: Log response state
        if response.hovered() {
            trace!(
                has_focus = self.has_focus,
                dragged = response.dragged_by(egui::PointerButton::Primary),
                "Viewport hovered"
            );
        }

        // Handle input (mouse/keyboard) - always process, but camera only moves if focused
        self.handle_input(
            &response,
            ui.ctx(),
            world,
            entity_manager,
            undo_stack,
            opt_prefab_mgr,
        )?;

        // Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: cache discrete
        // events + ToolContext fields for main.rs's per-frame
        // dispatch_cached_events call. Cached-then-dispatch design rationale:
        // avoids threading `&mut Dispatcher` through 5 ui() call sites; preserves
        // additive coexistence with existing main.rs:3833-3877 mediator path
        // (which is unchanged). The cache reflects the same egui::Response
        // state that handle_input just processed.
        self.cache_active_tool_events(&response, ui.ctx(), available);

        // Capture game input for play mode (WASD, mouse, action keys)
        if is_playing && self.has_focus {
            let gi = ui.ctx().input(|i| {
                let mut inp = crate::runtime::GameInput::default();
                if i.key_down(egui::Key::W) {
                    inp.move_y += 1.0;
                }
                if i.key_down(egui::Key::S) {
                    inp.move_y -= 1.0;
                }
                if i.key_down(egui::Key::A) {
                    inp.move_x -= 1.0;
                }
                if i.key_down(egui::Key::D) {
                    inp.move_x += 1.0;
                }
                if i.key_down(egui::Key::Space) {
                    inp.jump = true;
                }
                if i.key_down(egui::Key::E) {
                    inp.interact = true;
                }
                if i.key_down(egui::Key::Num1) {
                    inp.ability_1 = true;
                }
                if i.key_down(egui::Key::Num2) {
                    inp.ability_2 = true;
                }
                if i.key_down(egui::Key::Num3) {
                    inp.ability_3 = true;
                }
                if let Some(pos) = i.pointer.hover_pos() {
                    inp.mouse_pos = [pos.x - response.rect.min.x, pos.y - response.rect.min.y];
                }
                inp.mouse_left = i.pointer.button_down(egui::PointerButton::Primary);
                inp.mouse_right = i.pointer.button_down(egui::PointerButton::Secondary);
                inp
            });
            self.pending_game_input = Some(gi);
        }

        // Request continuous repaint for the viewport.
        // Always request — even when not hovered/focused — because the 3D
        // scene must render every frame for smooth camera motion and animation.
        // Without this, eframe's winit backend falls into ControlFlow::Wait
        // and the viewport renders only when OS events arrive (~5 FPS).
        ui.ctx().request_repaint();

        // Restore default cursor whenever the viewport renders.
        // Previously this was gated on response.hovered(), which missed
        // cursor restoration after Alt+Tab (hover is false on focus regain).
        ui.output_mut(|o| {
            if o.cursor_icon == egui::CursorIcon::None {
                o.cursor_icon = egui::CursorIcon::Default;
            }
        });

        // Update camera aspect ratio
        if viewport_size.x > 0.0 && viewport_size.y > 0.0 {
            self.camera.set_aspect(viewport_size.x, viewport_size.y);
        }

        // Resize texture if needed
        let size = (viewport_size.x as u32, viewport_size.y as u32);
        if size != self.last_size && size.0 > 0 && size.1 > 0 {
            self.resize_texture(size)?;
            self.last_size = size;
        }

        // Update renderer selected entities and entity mesh map
        {
            // Pre-compute data outside the lock to avoid holding the mutex while iterating EntityManager
            let entities_u32: Vec<u32> =
                self.selected_entities.iter().map(|&id| id as u32).collect();

            let current_count = entity_manager.count();
            let needs_rebuild = current_count != self.cached_entity_count;

            // Pre-compute entity maps only when entity count changes
            let rebuild_data = if needs_rebuild {
                self.cached_entity_count = current_count;

                let mesh_map: std::collections::HashMap<u32, String> = entity_manager
                    .entities()
                    .iter()
                    .filter_map(|(&id, entity)| {
                        entity.mesh.as_ref().map(|path| (id as u32, path.clone()))
                    })
                    .collect();

                let tex_overrides: std::collections::HashMap<u32, String> = entity_manager
                    .entities()
                    .iter()
                    .filter_map(|(&id, entity)| {
                        entity
                            .material
                            .get_texture(crate::entity_manager::MaterialSlot::Albedo)
                            .and_then(|p| p.to_str())
                            .map(|s| (id as u32, s.to_string()))
                    })
                    .collect();

                let scene_lights: Vec<super::types::SceneLight> = entity_manager
                    .entities()
                    .iter()
                    .filter_map(|(_, entity)| {
                        let data = entity.components.get("Light")?;
                        let ltype = data.get("type")?.as_str().unwrap_or("point");
                        if ltype != "point" {
                            return None;
                        }
                        Some(super::types::SceneLight {
                            position: [entity.position.x, entity.position.y, entity.position.z],
                            range: data.get("range").and_then(|v| v.as_f64()).unwrap_or(10.0)
                                as f32,
                            color: [
                                data.get("color_r").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                                data.get("color_g").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                                data.get("color_b").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                            ],
                            intensity: data
                                .get("intensity")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(1.0) as f32,
                        })
                    })
                    .collect();

                let gizmo_lines = Self::generate_component_gizmo_lines(entity_manager);
                Some((mesh_map, tex_overrides, scene_lights, gizmo_lines))
            } else {
                None
            };

            // Apply pre-computed data under a short lock
            self.with_renderer("update_entity_state", |renderer| {
                renderer.set_selected_entities(&entities_u32);
                if let Some((mesh_map, tex_overrides, scene_lights, gizmo_lines)) = rebuild_data {
                    renderer.set_entity_meshes(mesh_map);
                    if !tex_overrides.is_empty() {
                        renderer.set_entity_texture_overrides(tex_overrides);
                    }
                    renderer.set_scene_lights(scene_lights);
                    renderer.set_component_gizmo_lines(gizmo_lines);
                }
            });
        }

        // Update gizmo hover state for visual highlighting
        let hovered_axis = self.update_gizmo_hover(ui, &rect, world);

        // Render to texture (before displaying)
        if let Some(texture) = self.render_texture.clone() {
            // Render in separate scope to drop MutexGuard early
            {
                let grid_pref = self.toolbar.show_grid && self.toolbar.grid_type != GridType::None;
                let crosshair_mode = self.toolbar.grid_type == GridType::Crosshair;
                let shading_mode = self.toolbar.shading_mode.to_u32();
                // Force entity rebuild when gizmo is actively dragging (transforms changing)
                let gizmo_dragging = self.gizmo_state.is_active();

                self.with_renderer("render", |renderer| {
                    if gizmo_dragging {
                        renderer.invalidate_entity_cache();
                    }
                    // Auto-hide grid when terrain is loaded to prevent z-fighting
                    let show_grid = grid_pref && !renderer.has_terrain();
                    if let Err(e) = renderer.render(
                        &texture,
                        &self.camera,
                        world,
                        Some(&self.gizmo_state),
                        hovered_axis,
                        None,
                        show_grid,
                        crosshair_mode,
                        shading_mode,
                    ) {
                        warn!(error = %e, "Viewport render failed");
                    }
                });
            }

            // Register/update native texture with egui-wgpu (zero-copy GPU-direct display)
            // Only re-register when native_texture_id is None (after resize or first frame)
            if self.native_texture_id.is_none() {
                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(wgpu::TextureFormat::Rgba8Unorm),
                    ..Default::default()
                });
                // Get device handle before locking egui renderer to avoid nested locks
                if let Some(device) = self.with_renderer("device", |r| r.device().clone()) {
                    let mut egui_renderer = self.egui_wgpu_renderer.write();
                    let tex_id = egui_renderer.register_native_texture(
                        &device,
                        &view,
                        wgpu::FilterMode::Linear,
                    );
                    self.native_texture_id = Some(tex_id);
                }
            }

            // Display texture via egui (GPU-direct, no CPU readback)
            if let Some(texture_id) = self.native_texture_id {
                // Visual border for focus/hover states (✓ Implemented)
                // Border rendering happens in viewport_frame() above

                // Display rendered viewport using egui's texture system
                ui.painter().image(
                    texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Snapping indicator (top-right, shown when actively snapping)
                if self.gizmo_state.is_active() {
                    let snap_enabled = ui.ctx().input(|i| i.modifiers.ctrl || i.modifiers.command);

                    if snap_enabled {
                        let snap_text = match self.gizmo_state.mode {
                            crate::gizmo::GizmoMode::Translate { .. } => {
                                format!("Grid Snap: {:.2}m", self.grid_snap_size)
                            }
                            crate::gizmo::GizmoMode::Rotate { .. } => {
                                format!(
                                    "Angle Snap: {}°",
                                    self.angle_snap_increment.to_degrees() as i32
                                )
                            }
                            _ => String::new(),
                        };

                        if !snap_text.is_empty() {
                            let snap_width = 200.0;
                            let snap_rect = egui::Rect::from_min_size(
                                rect.right_top() + egui::vec2(-snap_width - 10.0, 75.0),
                                egui::vec2(snap_width, 25.0),
                            );

                            ui.painter().rect_filled(
                                snap_rect,
                                3.0,
                                egui::Color32::from_rgba_premultiplied(100, 200, 100, 220),
                            );

                            ui.painter().text(
                                snap_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                snap_text,
                                egui::FontId::proportional(13.0),
                                egui::Color32::BLACK,
                            );
                        }
                    }
                }

                // Update and display toolbar (provides FPS, camera info, reset, stats)
                self.toolbar.stats.fps = fps;
                self.toolbar.stats.frame_time_ms = self.fps_ema * 1000.0;
                self.toolbar.stats.push_frame_time(self.fps_ema * 1000.0);
                self.toolbar.stats.memory_usage_mb = estimate_memory_usage_mb();
                self.toolbar.stats.camera_position = self.camera.position().to_array();
                self.toolbar.stats.entity_count = entity_manager.count() as u32;

                // Scatter stats from the renderer (gather then assign to
                // avoid borrow conflict with &self in with_renderer).
                if let Some((si, sd, tris)) = self.with_renderer("scatter_stats", |renderer| {
                    (
                        renderer.scatter_instance_count() as usize,
                        renderer.scatter_draw_calls(),
                        (renderer.terrain_triangles() + renderer.scatter_triangles()) as u32,
                    )
                }) {
                    self.toolbar.stats.scatter_instances = si;
                    self.toolbar.stats.scatter_draw_calls = sd;
                    self.toolbar.stats.triangle_count = tris;
                }

                // Sync toolbar snap settings to viewport
                self.grid_snap_size = self.toolbar.snap_size;
                self.angle_snap_increment = self.toolbar.angle_snap_degrees.to_radians();

                self.toolbar.ui(ui, rect, &mut self.camera);
            } else {
                // First frame - texture not ready yet
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 30, 35));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Loading 3D Viewport...",
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgb(150, 170, 190),
                );
            }
        } else {
            // No texture yet - show placeholder
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 30));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Initializing 3D Viewport...",
                egui::FontId::proportional(16.0),
                egui::Color32::GRAY,
            );
        }

        Ok(())
    }

    /// Update gizmo hover state by raycasting against gizmo handles
    ///
    /// This provides visual feedback when the cursor is over a gizmo axis,
    /// highlighting the axis the user will interact with.
    ///
    /// # Arguments
    ///
    /// * `ui` - egui UI context for pointer position
    /// * `rect` - Viewport rectangle in screen coordinates
    /// * `world` - World to get entity positions from
    ///
    /// # Returns
    ///
    /// The currently hovered axis constraint (if any)
    fn update_gizmo_hover(
        &mut self,
        ui: &egui::Ui,
        rect: &egui::Rect,
        world: &World,
    ) -> Option<AxisConstraint> {
        // Only process hover if gizmo is active and we have a selection
        if self.gizmo_state.mode == GizmoMode::Inactive {
            self.hovered_handle = None;
            return None;
        }

        let selected = self.selected_entity()?;
        let pose = world.pose(selected as u32)?;

        // Get mouse position in viewport
        let pointer_pos = ui.ctx().pointer_latest_pos()?;

        // Check if pointer is within viewport
        if !rect.contains(pointer_pos) {
            self.hovered_handle = None;
            return None;
        }

        // Convert screen position to normalized device coordinates [-1, 1]
        let viewport_size = rect.size();
        let relative_pos = pointer_pos - rect.left_top();
        let ndc_x = (relative_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (relative_pos.y / viewport_size.y) * 2.0; // Y is flipped
        let ndc_pos = glam::Vec2::new(ndc_x, ndc_y);

        // Get inverse view-projection matrix for ray casting
        let inv_view_proj = self.camera.inverse_view_projection_matrix();

        // Gizmo position in 3D
        let gizmo_pos = glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);

        // Update picker scale based on camera distance
        let camera_distance = (self.camera.position() - gizmo_pos).length();
        let gizmo_scale = (camera_distance * 0.08).clamp(0.1, 10.0);

        // Create picker with appropriate scale
        let mut picker = self.gizmo_picker.clone();
        picker.gizmo_scale = gizmo_scale;
        picker.tolerance = gizmo_scale * 0.25; // Tolerance scales with gizmo size

        // Pick handle from screen coordinates
        self.hovered_handle =
            picker.pick_handle(ndc_pos, inv_view_proj, gizmo_pos, self.gizmo_state.mode);

        // Convert handle to axis constraint for rendering
        self.hovered_handle.map(|h| h.to_constraint())
    }

    /// Handle mouse/keyboard input
    ///
    /// Implements standard 3D viewport controls:
    /// - Left drag: Orbit camera
    /// - Middle drag: Pan camera
    /// - Scroll: Zoom camera
    /// - G/R/S: Gizmo mode (translate/rotate/scale)
    /// - Click: Select entity
    fn handle_input(
        &mut self,
        response: &egui::Response,
        ctx: &egui::Context,
        world: &mut World,
        entity_manager: &mut EntityManager,
        undo_stack: &mut crate::command::UndoStack, // Phase 2.1: Command integration
        opt_prefab_mgr: Option<&mut crate::prefab::PrefabManager>, // Phase 8.1 Week 5 Day 3: Auto-tracking
    ) -> Result<()> {
        use crate::gizmo::GizmoMode;

        // Update gizmo state with current mouse position
        if let Some(pos) = response.hover_pos() {
            let mouse_pos = glam::Vec2::new(pos.x, pos.y);
            self.gizmo_state.update_mouse(mouse_pos);
        }

        // Gizmo transform application (if active and dragging)
        if self.gizmo_state.is_active() && response.dragged_by(egui::PointerButton::Primary) {
            if let Some(selected_id) = self.selected_entity() {
                // Get entity's current pose
                if let Some(pose) = world.pose(selected_id as u32) {
                    let mouse_delta = self.gizmo_state.mouse_delta();

                    match self.gizmo_state.mode {
                        GizmoMode::Translate { constraint: _ } => {
                            // Read CURRENT constraint (not captured at match time!)
                            let constraint = match self.gizmo_state.mode {
                                GizmoMode::Translate { constraint: c } => c,
                                _ => crate::gizmo::AxisConstraint::None,
                            };

                            if constraint == crate::gizmo::AxisConstraint::None {
                                // FREE MOVEMENT: Entity follows mouse pointer on camera-facing plane
                                // Uses drag_offset to prevent center-snap and drag_plane_y for stable dragging
                                if let Some(mouse_pos_abs) = response.hover_pos() {
                                    let viewport_size = response.rect.size();
                                    let mouse_pos = egui::Pos2 {
                                        x: mouse_pos_abs.x - response.rect.min.x,
                                        y: mouse_pos_abs.y - response.rect.min.y,
                                    };

                                    let ray = self.camera.ray_from_screen(mouse_pos, viewport_size);

                                    // Intersect ray with horizontal plane at entity's drag height
                                    let plane_normal = glam::Vec3::Y;
                                    let plane_point = glam::Vec3::new(0.0, self.drag_plane_y, 0.0);
                                    let denom = ray.direction.dot(plane_normal);

                                    if denom.abs() > 0.0001 {
                                        let t =
                                            (plane_point - ray.origin).dot(plane_normal) / denom;
                                        if t >= 0.0 {
                                            let hit = ray.origin + ray.direction * t;

                                            // Subtract drag offset so entity stays where user grabbed it
                                            let offset =
                                                self.drag_offset.unwrap_or(glam::Vec3::ZERO);
                                            let world_pos = hit - offset;

                                            let snap_enabled = ctx
                                                .input(|i| i.modifiers.ctrl || i.modifiers.command);

                                            let final_x = if snap_enabled {
                                                self.snap_to_grid(world_pos.x)
                                            } else {
                                                world_pos.x
                                            };
                                            let final_z = if snap_enabled {
                                                self.snap_to_grid(world_pos.z)
                                            } else {
                                                world_pos.z
                                            };

                                            let new_x = final_x.round() as i32;
                                            let new_z = final_z.round() as i32;

                                            if let Some(pose_mut) =
                                                world.pose_mut(selected_id as u32)
                                            {
                                                pose_mut.pos.x = new_x;
                                                pose_mut.pos.y = new_z;

                                                debug!(
                                                    entity = ?selected_id,
                                                    snap_enabled,
                                                    world_x = world_pos.x,
                                                    world_z = world_pos.z,
                                                    new_x,
                                                    new_z,
                                                    "Translate (FREE)"
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                // CONSTRAINED MOVEMENT: Raycast to ground plane, then project onto constraint axis
                                if let Some(mouse_pos_abs) = response.hover_pos() {
                                    let viewport_size = response.rect.size();
                                    // Convert absolute screen position to viewport-relative
                                    let mouse_pos = egui::Pos2 {
                                        x: mouse_pos_abs.x - response.rect.min.x,
                                        y: mouse_pos_abs.y - response.rect.min.y,
                                    };

                                    // Get locked position from constraint_position (captured when X/Y/Z pressed)
                                    // This ensures the locked axis stays at the position when constraint was applied,
                                    // not the original start position from when the operation began
                                    let locked_pos = if let Some(constraint_pos) =
                                        &self.gizmo_state.constraint_position
                                    {
                                        (constraint_pos.x, constraint_pos.z)
                                    } else if let Some(snapshot) = &self.gizmo_state.start_transform
                                    {
                                        // Fallback to start transform if no constraint position captured
                                        (snapshot.position.x, snapshot.position.z)
                                    } else {
                                        (pose.pos.x as f32, pose.pos.y as f32)
                                    };

                                    // Cast ray from mouse through camera
                                    let ray = self.camera.ray_from_screen(mouse_pos, viewport_size);

                                    // Intersect ray with horizontal plane at entity's drag height
                                    let plane_normal = glam::Vec3::Y;
                                    let plane_point = glam::Vec3::new(0.0, self.drag_plane_y, 0.0);
                                    let denom = ray.direction.dot(plane_normal);

                                    if denom.abs() > 0.0001 {
                                        let t =
                                            (plane_point - ray.origin).dot(plane_normal) / denom;
                                        if t >= 0.0 {
                                            let hit = ray.origin + ray.direction * t;
                                            // Apply drag offset for smooth grab
                                            let offset =
                                                self.drag_offset.unwrap_or(glam::Vec3::ZERO);
                                            let world_pos = hit - offset;

                                            // Check if Ctrl is held for grid snapping
                                            let snap_enabled = ctx
                                                .input(|i| i.modifiers.ctrl || i.modifiers.command);

                                            // Apply snapping to world position before constraints
                                            let snapped_x = if snap_enabled {
                                                self.snap_to_grid(world_pos.x)
                                            } else {
                                                world_pos.x
                                            };
                                            let snapped_z = if snap_enabled {
                                                self.snap_to_grid(world_pos.z)
                                            } else {
                                                world_pos.z
                                            };

                                            // Project onto constraint axis (lock one component to locked_pos)
                                            let (new_x, new_z) = match constraint {
                                                crate::gizmo::AxisConstraint::X => {
                                                    // X-axis only: follow mouse X, lock Z to captured position
                                                    (snapped_x.round() as i32, locked_pos.1 as i32)
                                                }
                                                crate::gizmo::AxisConstraint::Z => {
                                                    // Z-axis only: lock X to captured position, follow mouse Z
                                                    (locked_pos.0 as i32, snapped_z.round() as i32)
                                                }
                                                crate::gizmo::AxisConstraint::Y => {
                                                    // Y-axis: lock XZ, height handled below via screen-space delta
                                                    (locked_pos.0 as i32, locked_pos.1 as i32)
                                                }
                                                _ => {
                                                    // Planar constraints: use both axes
                                                    (
                                                        snapped_x.round() as i32,
                                                        snapped_z.round() as i32,
                                                    )
                                                }
                                            };

                                            if let Some(pose_mut) =
                                                world.pose_mut(selected_id as u32)
                                            {
                                                pose_mut.pos.x = new_x;
                                                pose_mut.pos.y = new_z; // IVec2.y = world Z

                                                // Y-axis constraint: use mouse vertical delta to adjust height
                                                if matches!(
                                                    constraint,
                                                    crate::gizmo::AxisConstraint::Y
                                                ) {
                                                    let dy = ctx.input(|i| i.pointer.delta().y);
                                                    let height_sensitivity = 0.05;
                                                    pose_mut.height -= dy * height_sensitivity;
                                                }

                                                debug!(
                                                    entity = ?selected_id,
                                                    snap_enabled,
                                                    constraint = ?constraint,
                                                    locked_x = locked_pos.0,
                                                    locked_z = locked_pos.1,
                                                    world_x = world_pos.x,
                                                    world_z = world_pos.z,
                                                    new_x,
                                                    new_z,
                                                    "Translate (CONSTRAINED)"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        GizmoMode::Rotate { constraint: _ } => {
                            // Try to get start transform snapshot
                            if let Some(snapshot) = &self.gizmo_state.start_transform {
                                // CRITICAL FIX: Read CURRENT constraint (not captured at match time!)
                                let constraint = match self.gizmo_state.mode {
                                    GizmoMode::Rotate { constraint: c } => c,
                                    _ => crate::gizmo::AxisConstraint::None,
                                };

                                // Check if Ctrl is held for angle snapping
                                let snap_enabled =
                                    ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

                                // Calculate rotation delta using modular gizmo
                                // 200px = 1 radian means 100px = 0.5 radians (sensitivity)
                                let rotation_delta_quat = RotateGizmo::calculate_rotation(
                                    mouse_delta,
                                    constraint,
                                    0.5, // sensitivity: radians per 100 pixels
                                    snap_enabled,
                                    snapshot.rotation,
                                    false, // local_space
                                );

                                // Apply delta to start rotation
                                let new_rotation_quat = rotation_delta_quat * snapshot.rotation;

                                // Convert back to Euler angles for Pose
                                let (new_x, new_y, new_z) =
                                    new_rotation_quat.to_euler(glam::EulerRot::XYZ);

                                if let Some(pose_mut) = world.pose_mut(selected_id as u32) {
                                    pose_mut.rotation_x = new_x;
                                    pose_mut.rotation = new_y;
                                    pose_mut.rotation_z = new_z;

                                    debug!(
                                        entity = ?selected_id,
                                        snap_enabled,
                                        constraint = ?constraint,
                                        mouse_delta_x = mouse_delta.x,
                                        mouse_delta_y = mouse_delta.y,
                                        "Rotate (modular)"
                                    );
                                }
                            }
                        }
                        GizmoMode::Scale {
                            constraint,
                            uniform,
                        } => {
                            if let Some(snapshot) = &self.gizmo_state.start_transform {
                                // Calculate scale multiplier from mouse delta
                                let scale_multiplier = ScaleGizmo::calculate_scale(
                                    mouse_delta,
                                    constraint,
                                    uniform,
                                    1.0, // sensitivity
                                    snapshot.rotation,
                                    false, // local_space
                                );

                                // Apply to pose (World only supports uniform f32 scale for now)
                                if let Some(pose_mut) = world.pose_mut(selected_id as u32) {
                                    // Use X component as uniform scale factor
                                    let new_scale =
                                        (snapshot.scale.x * scale_multiplier.x).clamp(0.1, 10.0);
                                    pose_mut.scale = new_scale;

                                    debug!(
                                        entity = ?selected_id,
                                        mouse_delta_x = mouse_delta.x,
                                        mouse_delta_y = mouse_delta.y,
                                        multiplier = scale_multiplier.x,
                                        new_scale,
                                        "Scale (drag)"
                                    );
                                }
                            }
                        }
                        GizmoMode::Inactive => {}
                    }
                }
            }
        }

        // Camera controls (middle mouse and scroll work even during gizmo mode)
        // Only left-drag is captured by gizmo
        let can_control_camera = response.hovered() || self.has_focus;

        // Middle mouse drag: Orbit camera (standard 3D viewport control)
        if can_control_camera && response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            let is_shift = ctx.input(|i| i.modifiers.shift);
            if is_shift {
                // Shift + Middle drag: Pan
                self.camera.pan(delta.x, delta.y);
            } else {
                // Middle drag: Orbit
                self.camera.orbit(delta.x, delta.y);
            }
        }

        // Orbit camera (left mouse drag) - DISABLED during gizmo operation or terrain brush
        if can_control_camera
            && response.dragged_by(egui::PointerButton::Primary)
            && !self.gizmo_state.is_active()
            && !self.terrain_brush_active
        // Don't orbit while gizmo or terrain brush active
        {
            let delta = response.drag_delta();
            debug!(
                delta_x = delta.x,
                delta_y = delta.y,
                yaw = self.camera.yaw(),
                pitch = self.camera.pitch(),
                "Orbit camera"
            );
            self.camera.orbit(delta.x, delta.y);
        }

        // Terrain brush: depth-based hit detection on click/drag
        // Reads the depth buffer at the mouse pixel to find the exact terrain surface.
        // Falls back to Y=0 plane intersection if no depth is available.
        if self.terrain_brush_active
            && !self.gizmo_state.is_active()
            && (response.dragged_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary))
        {
            let is_click = response.clicked_by(egui::PointerButton::Primary);
            let elapsed = self.last_brush_time.elapsed();
            let throttle_ok = is_click || elapsed.as_millis() >= 16; // ~60 Hz
            if throttle_ok {
                if let Some(mouse_pos_abs) = response.hover_pos() {
                    let viewport_size = response.rect.size();
                    let mouse_pos = egui::Pos2 {
                        x: mouse_pos_abs.x - response.rect.min.x,
                        y: mouse_pos_abs.y - response.rect.min.y,
                    };
                    let px = mouse_pos.x as u32;
                    let py = mouse_pos.y as u32;

                    // Try depth-based pick first
                    let mut hit = None;
                    let depth_val = self
                        .with_renderer("read_depth_for_pick", |r| r.read_depth_at_pixel(px, py))
                        .flatten();
                    if let Some(depth) = depth_val {
                        if depth < 1.0 {
                            let world_pos = self.camera.unproject_depth_to_world(
                                px as f32,
                                py as f32,
                                viewport_size.x,
                                viewport_size.y,
                                depth,
                            );
                            hit = Some([world_pos.x, world_pos.z]);
                        }
                    }

                    // Fallback: Y=0 plane intersection when depth is sky (1.0) or unavailable
                    if hit.is_none() {
                        let ray = self.camera.ray_from_screen(mouse_pos, viewport_size);
                        let denom = ray.direction.dot(glam::Vec3::Y);
                        if denom.abs() > 0.0001 {
                            let t = (glam::Vec3::ZERO - ray.origin).dot(glam::Vec3::Y) / denom;
                            if t >= 0.0 {
                                let world_pos = ray.origin + ray.direction * t;
                                hit = Some([world_pos.x, world_pos.z]);
                            }
                        }
                    }

                    if let Some(h) = hit {
                        self.terrain_brush_hits.push(h);
                        self.last_brush_time = std::time::Instant::now();
                    }
                }
            }
        }

        // Detect end of terrain brush stroke (mouse released after drag)
        if self.terrain_brush_active && response.drag_stopped_by(egui::PointerButton::Primary) {
            self.terrain_brush_stroke_ended = true;
        }

        // Brush cursor visualization — circle draped on terrain at hover position
        if self.terrain_brush_active && !self.gizmo_state.is_active() {
            if let Some(mouse_pos_abs) = response.hover_pos() {
                let viewport_size = response.rect.size();
                let mouse_pos = egui::Pos2 {
                    x: mouse_pos_abs.x - response.rect.min.x,
                    y: mouse_pos_abs.y - response.rect.min.y,
                };
                let px = mouse_pos.x as u32;
                let py = mouse_pos.y as u32;

                // Get world hit for cursor center
                let mut cursor_center = None;
                let depth_val = self
                    .with_renderer("read_depth_for_brush", |r| r.read_depth_at_pixel(px, py))
                    .flatten();
                if let Some(depth) = depth_val {
                    if depth < 1.0 {
                        let wp = self.camera.unproject_depth_to_world(
                            px as f32,
                            py as f32,
                            viewport_size.x,
                            viewport_size.y,
                            depth,
                        );
                        cursor_center = Some(wp);
                    }
                }
                if cursor_center.is_none() {
                    let ray = self.camera.ray_from_screen(mouse_pos, viewport_size);
                    let denom = ray.direction.dot(glam::Vec3::Y);
                    if denom.abs() > 0.0001 {
                        let t = -ray.origin.y / denom;
                        if t >= 0.0 {
                            cursor_center = Some(ray.origin + ray.direction * t);
                        }
                    }
                }

                if let Some(center) = cursor_center {
                    let segments = 48;
                    let color = if self.terrain_brush_is_paint {
                        [0.3, 0.5, 1.0] // blue for paint
                    } else {
                        [0.2, 1.0, 0.3] // green for sculpt
                    };
                    let mut lines = Vec::with_capacity(segments);
                    for i in 0..segments {
                        let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                        let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
                        let x0 = center.x + self.terrain_brush_radius * a0.cos();
                        let z0 = center.z + self.terrain_brush_radius * a0.sin();
                        let x1 = center.x + self.terrain_brush_radius * a1.cos();
                        let z1 = center.z + self.terrain_brush_radius * a1.sin();
                        // Use center Y for draping (slight offset above surface)
                        let y = center.y + 0.15;
                        lines.push(astraweave_physics::DebugLine::new(
                            [x0, y, z0],
                            [x1, y, z1],
                            color,
                        ));
                    }
                    self.set_brush_cursor_lines(lines);
                } else {
                    self.set_brush_cursor_lines(Vec::new());
                }
            } else {
                self.set_brush_cursor_lines(Vec::new());
            }
        } else {
            self.set_brush_cursor_lines(Vec::new());
        }

        // Pan camera (right mouse drag)
        if can_control_camera && response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            debug!(
                delta_x = delta.x,
                delta_y = delta.y,
                focal = ?self.camera.target(),
                "Pan camera"
            );
            self.camera.pan(delta.x, delta.y);
        }

        // WASD + Space/Shift keyboard movement (FPS-style)
        // Works when viewport is hovered — no click required
        if can_control_camera {
            ctx.input(|i| {
                let dt = i.stable_dt.clamp(0.001, 0.1);
                // Speed is per-second, scaled by camera distance for consistent feel
                let speed = (self.camera.distance() * 2.5).clamp(18.0, 900.0) * dt;
                let mut move_delta = glam::Vec3::ZERO;

                // Cache forward/right vectors once per frame
                let fwd = self.camera.forward();
                let right = fwd.cross(glam::Vec3::Y).normalize();

                if i.key_down(egui::Key::W) {
                    move_delta += fwd * speed;
                }
                if i.key_down(egui::Key::S) {
                    move_delta -= fwd * speed;
                }
                if i.key_down(egui::Key::A) {
                    move_delta -= right * speed;
                }
                if i.key_down(egui::Key::D) {
                    move_delta += right * speed;
                }
                if i.key_down(egui::Key::Space) {
                    move_delta.y += speed;
                }
                if i.modifiers.shift {
                    move_delta.y -= speed;
                }

                if move_delta.length_squared() > 0.0001 {
                    self.camera.translate(move_delta);
                }
            });
        }

        // Zoom camera OR scale entity (scroll wheel) - only when hovered over viewport
        if response.hovered() {
            ctx.input(|i| {
                let scroll = i.raw_scroll_delta.y;
                if scroll.abs() > 0.5 {
                    self.camera.zoom(scroll);
                }
            });
        }

        // Animate smooth zoom each frame (use real dt for frame-rate independence)
        let dt = ctx.input(|i| i.stable_dt);
        let zoom_animating = self.camera.smooth_update(dt);
        if zoom_animating {
            ctx.request_repaint();
        }

        // Sync selected entity to gizmo state
        self.gizmo_state.selected_entity = self.selected_entity().map(|id| id as u32);

        // Clear gizmo state if entity deselected
        if self.selected_entity().is_none() && self.gizmo_state.is_active() {
            self.gizmo_state.mode = GizmoMode::Inactive;
            self.gizmo_state.start_transform = None;
            self.drag_offset = None;
        }

        // Capture start transform when beginning a new operation
        if self.gizmo_state.is_active() && self.gizmo_state.start_transform.is_none() {
            if let Some(selected_id) = self.selected_entity() {
                // Try to capture from World entity first (for actual transforms)
                if let Some(pose) = world.pose(selected_id as u32) {
                    let x = pose.pos.x as f32;
                    let z = pose.pos.y as f32;
                    // Create quaternion from XYZ Euler angles
                    let rotation_quat = glam::Quat::from_euler(
                        glam::EulerRot::XYZ,
                        pose.rotation_x,
                        pose.rotation,
                        pose.rotation_z,
                    );
                    self.gizmo_state.start_transform = Some(TransformSnapshot {
                        position: glam::Vec3::new(x, pose.height, z),
                        rotation: rotation_quat, // Store all 3 rotation axes
                        scale: glam::Vec3::new(pose.scale, pose.scale_y, pose.scale_z),
                    });

                    // Capture drag offset for translate mode:
                    // Raycast from current mouse position to a plane at entity height.
                    // The offset prevents center-snapping when clicking off-center.
                    if matches!(self.gizmo_state.mode, GizmoMode::Translate { .. }) {
                        self.drag_plane_y = pose.height;
                        if let Some(mouse_pos_abs) = response.hover_pos() {
                            let viewport_size = response.rect.size();
                            let mouse_pos = egui::Pos2 {
                                x: mouse_pos_abs.x - response.rect.min.x,
                                y: mouse_pos_abs.y - response.rect.min.y,
                            };
                            let ray = self.camera.ray_from_screen(mouse_pos, viewport_size);
                            let plane_normal = glam::Vec3::Y;
                            let plane_point = glam::Vec3::new(0.0, pose.height, 0.0);
                            let denom = ray.direction.dot(plane_normal);
                            if denom.abs() > 0.0001 {
                                let t = (plane_point - ray.origin).dot(plane_normal) / denom;
                                if t >= 0.0 {
                                    let hit = ray.origin + ray.direction * t;
                                    let entity_center = glam::Vec3::new(x, pose.height, z);
                                    self.drag_offset = Some(hit - entity_center);
                                }
                            }
                        }
                    }

                    debug!(
                        x,
                        z,
                        rotation_x_deg = pose.rotation_x.to_degrees(),
                        rotation_deg = pose.rotation.to_degrees(),
                        rotation_z_deg = pose.rotation_z.to_degrees(),
                        scale = pose.scale,
                        "Captured World start transform"
                    );
                } else if let Some(entity) = entity_manager.get(selected_id) {
                    // Fallback to EntityManager
                    self.gizmo_state.start_transform = Some(TransformSnapshot {
                        position: entity.position,
                        rotation: entity.rotation,
                        scale: entity.scale,
                    });
                    debug!(
                        position = ?entity.position,
                        "Captured EntityManager start transform"
                    );
                }
            }
        }

        let mut clipboard_json = None;
        // Gizmo hotkeys (G/R/S for translate/rotate/scale, X/Y/Z for axis constraints, Enter/Escape)
        let returned_json = ctx.input(|i| {
            use winit::keyboard::KeyCode;

            // Handle gizmo mode keys first
            if i.key_pressed(egui::Key::G) {
                self.gizmo_state.handle_key(KeyCode::KeyG);
                debug!("Gizmo mode: Translate (G)");
            }
            if i.key_pressed(egui::Key::R) {
                self.gizmo_state.handle_key(KeyCode::KeyR);
                debug!("Gizmo mode: Rotate (R)");
            }
            if i.key_pressed(egui::Key::S) {
                // Check if already in scale mode (to toggle off)
                let was_scaling = matches!(self.gizmo_state.mode, GizmoMode::Scale { .. });
                self.gizmo_state.handle_key(KeyCode::KeyS);
                if was_scaling {
                    debug!("Scale mode: OFF (camera control restored)");
                } else {
                    debug!("Scale mode: ON (use scroll wheel to scale, S to exit)");
                }
            }

            // Axis constraints (X/Y/Z)
            // When constraint is applied, capture the CURRENT position for axis locking
            // This ensures that if user moves freely then applies constraint, the locked
            // axis stays at its current value, not the original start position
            if i.key_pressed(egui::Key::X) {
                // Capture current position before applying constraint
                if let Some(selected_id) = self.selected_entity() {
                    if let Some(pose) = world.pose(selected_id as u32) {
                        let current_pos =
                            glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
                        self.gizmo_state.constraint_position = Some(current_pos);
                        debug!(
                            "Captured constraint position: ({}, {})",
                            pose.pos.x, pose.pos.y
                        );
                    }
                }
                self.gizmo_state.handle_key(KeyCode::KeyX);
                debug!("Axis constraint: X");
            }
            if i.key_pressed(egui::Key::Y) {
                // Capture current position before applying constraint
                if let Some(selected_id) = self.selected_entity() {
                    if let Some(pose) = world.pose(selected_id as u32) {
                        let current_pos =
                            glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
                        self.gizmo_state.constraint_position = Some(current_pos);
                        debug!(
                            "Captured constraint position: ({}, {})",
                            pose.pos.x, pose.pos.y
                        );
                    }
                }
                self.gizmo_state.handle_key(KeyCode::KeyY);
                debug!("Axis constraint: Y");
            }
            if i.key_pressed(egui::Key::Z) {
                // Capture current position before applying constraint
                if let Some(selected_id) = self.selected_entity() {
                    if let Some(pose) = world.pose(selected_id as u32) {
                        let current_pos =
                            glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
                        self.gizmo_state.constraint_position = Some(current_pos);
                        debug!(
                            "Captured constraint position: ({}, {})",
                            pose.pos.x, pose.pos.y
                        );
                    }
                }
                self.gizmo_state.handle_key(KeyCode::KeyZ);
                debug!("Axis constraint: Z");
            }

            // Confirm/cancel gizmo operation
            if i.key_pressed(egui::Key::Enter) {
                self.gizmo_state.handle_key(KeyCode::Enter);
                self.drag_offset = None;
                debug!("Gizmo: Confirm");
            }
            if i.key_pressed(egui::Key::Escape) {
                self.gizmo_state.handle_key(KeyCode::Escape);
                self.drag_offset = None;
                debug!("Gizmo: Cancel");
            }

            // Undo/Redo (Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z)
            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    // Ctrl+Shift+Z: Redo
                    if let Err(e) = undo_stack.redo(world, None) {
                        warn!("Redo failed: {}", e);
                    } else if let Some(desc) = undo_stack.redo_description() {
                        debug!("Redo: {}", desc);
                    }
                } else {
                    // Ctrl+Z: Undo
                    if let Err(e) = undo_stack.undo(world, None) {
                        warn!("Undo failed: {}", e);
                    } else if let Some(desc) = undo_stack.undo_description() {
                        debug!("Undo: {}", desc);
                    }
                }
            }
            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Y) {
                // Ctrl+Y: Redo (alternative to Ctrl+Shift+Z)
                if let Err(e) = undo_stack.redo(world, None) {
                    warn!("Redo failed: {}", e);
                } else if let Some(desc) = undo_stack.redo_description() {
                    debug!("Redo: {}", desc);
                }
            }

            // Copy/Paste/Duplicate/Delete (multi-selection support)
            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::C) {
                // Ctrl+C: Copy selected entities
                if !self.selected_entities.is_empty() {
                    clipboard_json = self.copy_selection(world);
                }
            }
            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::V) {
                // Ctrl+V: Paste entities - try to get text from Paste event
                let text = i.events.iter().find_map(|e| match e {
                    egui::Event::Paste(s) => Some(s.clone()),
                    _ => None,
                });
                self.paste_selection(world, undo_stack, text);
            }
            if (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::D) {
                // Ctrl+D: Duplicate selected entities
                if !self.selected_entities.is_empty() {
                    self.duplicate_selection(world, undo_stack);
                    debug!("Duplicated {} entities", self.selected_entities.len());
                }
            }
            if i.key_pressed(egui::Key::Delete) {
                // Delete: Remove selected entities
                if !self.selected_entities.is_empty() {
                    self.delete_selection(world, undo_stack);
                    debug!("Deleted {} entities", self.selected_entities.len());
                }
            }
            // Select All
            if i.key_pressed(egui::Key::A) {
                debug!(
                    "A key pressed! modifiers: ctrl={}, command={}, shift={}",
                    i.modifiers.ctrl, i.modifiers.command, i.modifiers.shift
                );

                if i.modifiers.command || i.modifiers.ctrl {
                    // Ctrl+A: Select all entities
                    self.select_all(world);
                    debug!(
                        "Selected all entities ({} total)",
                        self.selected_entities.len()
                    );
                }
            }

            // Frame selected
            if i.key_pressed(egui::Key::F) {
                if let Some(selected_id) = self.selected_entity() {
                    // Frame World entity (match rendering position)
                    if let Some(pose) = world.pose(selected_id as u32) {
                        let x = pose.pos.x as f32;
                        let z = pose.pos.y as f32;
                        let position = glam::Vec3::new(x, pose.height, z);
                        let entity_radius = 0.866; // Half diagonal of 1x1x1 cube = sqrt(3)/2

                        // Frame entity in camera view
                        self.camera.frame_entity(position, entity_radius);

                        debug!(
                            "Frame selected World entity {} at {:.2?}",
                            selected_id, position
                        );
                    } else {
                        debug!("Frame selected: Entity {} not found in World", selected_id);
                    }
                } else {
                    debug!("Frame selected: No entity selected");
                }
            }

            // Grid size controls: [ = decrease, ] = increase
            if i.key_pressed(egui::Key::OpenBracket) {
                // Cycle down: 2.0 → 1.0 → 0.5 → 0.25 → 2.0
                self.grid_snap_size = match self.grid_snap_size {
                    x if (x - 2.0).abs() < 0.01 => 1.0,
                    x if (x - 1.0).abs() < 0.01 => 0.5,
                    x if (x - 0.5).abs() < 0.01 => 0.25,
                    _ => 2.0,
                };
                debug!("Grid snap size: {:.2}m", self.grid_snap_size);
            }

            if i.key_pressed(egui::Key::CloseBracket) {
                // Cycle up: 0.25 → 0.5 → 1.0 → 2.0 → 0.25
                self.grid_snap_size = match self.grid_snap_size {
                    x if (x - 0.25).abs() < 0.01 => 0.5,
                    x if (x - 0.5).abs() < 0.01 => 1.0,
                    x if (x - 1.0).abs() < 0.01 => 2.0,
                    _ => 0.25,
                };
                debug!("Grid snap size: {:.2}m", self.grid_snap_size);
            }

            // Camera bookmarks: F1-F12 (restore), Shift+F1-F12 (save)
            let bookmark_keys = [
                egui::Key::F1,
                egui::Key::F2,
                egui::Key::F3,
                egui::Key::F4,
                egui::Key::F5,
                egui::Key::F6,
                egui::Key::F7,
                egui::Key::F8,
                egui::Key::F9,
                egui::Key::F10,
                egui::Key::F11,
                egui::Key::F12,
            ];

            for (slot, key) in bookmark_keys.iter().enumerate() {
                if i.key_pressed(*key) {
                    if i.modifiers.shift {
                        // SAVE bookmark
                        self.camera_bookmarks[slot] = Some(CameraBookmark {
                            focal_point: self.camera.focal_point(),
                            distance: self.camera.distance(),
                            yaw: self.camera.yaw(),
                            pitch: self.camera.pitch(),
                        });
                        debug!("Saved camera bookmark F{}", slot + 1);
                    } else if let Some(bookmark) = &self.camera_bookmarks[slot] {
                        // RESTORE bookmark
                        self.camera.set_focal_point(bookmark.focal_point);
                        self.camera.set_distance(bookmark.distance);
                        self.camera.set_yaw(bookmark.yaw);
                        self.camera.set_pitch(bookmark.pitch);
                        debug!("Restored camera bookmark F{}", slot + 1);
                    } else {
                        debug!(
                            "Camera bookmark F{} not set (use Shift+F{} to save)",
                            slot + 1,
                            slot + 1
                        );
                    }
                }
            }

            clipboard_json
        });

        if let Some(json) = returned_json {
            ctx.copy_text(json);
        }

        // Handle gizmo confirm/cancel
        if self.gizmo_state.confirmed {
            // Phase 8.1 Week 5 Day 3: Delegate to interaction module for undo + auto-tracking
            let _metadata = crate::interaction::commit_active_gizmo_with_prefab_tracking(
                &mut self.gizmo_state,
                world,
                undo_stack,
                opt_prefab_mgr,
            );
            // metadata contains commit details (entity, operation, constraint) if successful
        }

        if self.gizmo_state.cancelled {
            // Transform cancelled - revert to start_transform (NO undo command created)
            if let Some(snapshot) = &self.gizmo_state.start_transform {
                if let Some(selected_id) = self.selected_entity() {
                    if let Some(entity) = entity_manager.get_mut(selected_id) {
                        entity.position = snapshot.position;
                        entity.rotation = snapshot.rotation;
                        entity.scale = snapshot.scale;
                        debug!("Transform cancelled - reverted to {:?}", snapshot.position);
                    }
                }
            }
            self.gizmo_state.cancelled = false;
        }

        // Selection (ray-casting entity picking)
        // Track mouse press/release manually since egui's clicked() doesn't work with drag detection
        let pointer_over_viewport = response.hovered()
            || response.rect.contains(
                ctx.input(|i| i.pointer.interact_pos())
                    .unwrap_or(egui::Pos2::ZERO),
            );

        let mouse_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
        let mouse_released = ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
        let current_pos = ctx.input(|i| i.pointer.interact_pos());

        // Track where mouse was pressed
        if mouse_pressed && pointer_over_viewport {
            self.mouse_pressed_pos = current_pos;
            debug!("Mouse pressed at: {:?}", current_pos);
        }

        // Check for click (press and release at same location without drag)
        let clicked = if let (true, Some(press_pos)) = (mouse_released, self.mouse_pressed_pos) {
            let release_pos = current_pos.unwrap_or(press_pos);
            let drag_distance = (release_pos - press_pos).length();
            let is_click = drag_distance < 5.0; // 5 pixel threshold

            debug!(
                "Mouse released: press={:?}, release={:?}, drag_dist={:.1}, is_click={}",
                press_pos, release_pos, drag_distance, is_click
            );

            self.mouse_pressed_pos = None; // Clear press state
            is_click && pointer_over_viewport && !self.gizmo_state.is_active()
        } else {
            false
        };

        debug!(
            "Selection check: clicked={}, pointer_over={}, gizmo_active={}",
            clicked,
            pointer_over_viewport,
            self.gizmo_state.is_active()
        );

        if clicked {
            debug!("Click detected for selection!");

            if let Some(pos) = current_pos {
                let viewport_pos_vec = pos - response.rect.min;
                let viewport_pos = egui::Pos2::new(viewport_pos_vec.x, viewport_pos_vec.y);
                let ray = self
                    .camera
                    .ray_from_screen(viewport_pos, response.rect.size());

                // Pick World entities (which are actually rendered)
                let mut closest_entity: Option<(Entity, f32)> = None; // (entity_id, distance)

                // Check all World entities
                for entity_id in 1..100 {
                    let entity: Entity = entity_id;

                    if let Some(pose) = world.pose(entity) {
                        // Position calculation from entity pose
                        let x = pose.pos.x as f32;
                        let z = pose.pos.y as f32;
                        let position = glam::Vec3::new(x, pose.height, z);

                        // Create AABB that accounts for entity scale
                        let half_size = 0.5 * pose.scale;
                        let aabb_min = position - glam::Vec3::splat(half_size);
                        let aabb_max = position + glam::Vec3::splat(half_size);

                        if let Some(distance) =
                            Self::ray_intersects_aabb(ray.origin, ray.direction, aabb_min, aabb_max)
                        {
                            let is_closer = closest_entity.is_none_or(|(_, d)| distance < d);
                            if is_closer {
                                closest_entity = Some((entity, distance));
                            }
                        }
                    }
                }

                // Update selection based on modifier keys
                if let Some((entity_id, distance)) = closest_entity {
                    let modifiers = ctx.input(|i| i.modifiers);

                    // Debug: Print modifier state
                    debug!(
                        "Modifiers: ctrl={}, shift={}, alt={}, command={}",
                        modifiers.ctrl, modifiers.shift, modifiers.alt, modifiers.command
                    );

                    if modifiers.ctrl || modifiers.command {
                        // Ctrl+Click: Toggle selection (multi-select)
                        debug!(
                            "Before toggle: selected_entities = {:?}",
                            self.selected_entities
                        );
                        self.toggle_selection(entity_id.into());
                        debug!(
                            "Toggled World entity {} (now {} entities selected): {:?}",
                            entity_id,
                            self.selected_entities.len(),
                            self.selected_entities
                        );
                    } else if modifiers.shift {
                        // Shift+Click: Add to selection
                        debug!(
                            "Before add: selected_entities = {:?}",
                            self.selected_entities
                        );
                        self.add_to_selection(entity_id.into());
                        debug!(
                            "Added World entity {} to selection ({} entities selected): {:?}",
                            entity_id,
                            self.selected_entities.len(),
                            self.selected_entities
                        );
                    } else {
                        // Regular click: Single select (clears others)
                        self.set_selected_entity(Some(entity_id.into()));
                        debug!(
                            "Selected World entity {} at distance {:.2}",
                            entity_id, distance
                        );
                    }
                } else {
                    // Clicked empty space - clear selection
                    self.clear_selection();
                    // Clear gizmo state when deselecting entity
                    if self.gizmo_state.is_active() {
                        self.gizmo_state.mode = GizmoMode::Inactive;
                        self.gizmo_state.start_transform = None;
                        self.drag_offset = None;
                    }
                    debug!(
                        "Click at ({:.1}, {:.1}) - No entity hit (selection cleared)",
                        viewport_pos.x, viewport_pos.y
                    );
                }
            }
        }

        Ok(())
    }

    /// Resize render texture
    ///
    /// Creates new render texture when viewport size changes.
    /// Called automatically by ui() method.
    ///
    /// # Arguments
    ///
    /// * `size` - New texture size (width, height)
    ///
    /// # Errors
    ///
    /// Returns error if texture creation fails.
    fn resize_texture(&mut self, size: (u32, u32)) -> Result<()> {
        if size.0 == 0 || size.1 == 0 {
            // Invalid size - clear texture
            self.render_texture = None;
            return Ok(());
        }

        // Lock renderer to create texture and resize
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|e| anyhow::anyhow!("Renderer lock poisoned: {}", e))?;

        // Create new render texture
        let texture = renderer
            .create_render_texture(size.0, size.1)
            .context("Failed to create render texture")?;

        // Wrap in Arc for sharing with paint callback
        self.render_texture = Some(Arc::new(texture));

        // Invalidate native texture registration (new texture needs re-registration)
        // Free the old egui texture slot if it exists
        if let Some(old_id) = self.native_texture_id.take() {
            let mut egui_renderer = self.egui_wgpu_renderer.write();
            egui_renderer.free_texture(&old_id);
        }

        // Resize renderer's depth buffer
        renderer
            .resize(size.0, size.1)
            .context("Failed to resize renderer")?;

        Ok(())
    }

    /// Get camera (read-only)
    pub fn camera(&self) -> &OrbitCamera {
        &self.camera
    }

    /// Compute the ground-plane (Y=0) intersection for a given screen position.
    /// Returns `Some((x, z))` if the ray intersects the ground, `None` otherwise.
    pub fn ground_position_at_screen_pos(&self, screen_pos: egui::Pos2) -> Option<(f32, f32)> {
        let rect = self.last_viewport_rect?;
        if !rect.contains(screen_pos) {
            return None;
        }
        let local_pos = screen_pos - rect.min;
        let viewport_size = rect.size();
        let ray = self
            .camera
            .ray_from_screen(egui::pos2(local_pos.x, local_pos.y), viewport_size);
        // Intersect with Y=0 ground plane
        let denom = ray.direction.y;
        if denom.abs() < 1e-6 {
            return None; // Ray parallel to ground
        }
        let t = -ray.origin.y / denom;
        if t < 0.0 {
            return None; // Intersection behind camera
        }
        let hit = ray.at(t);
        Some((hit.x, hit.z))
    }

    /// Get camera (mutable)
    pub fn camera_mut(&mut self) -> &mut OrbitCamera {
        &mut self.camera
    }

    /// Set camera state
    pub fn set_camera(&mut self, camera: OrbitCamera) {
        self.camera = camera;
    }

    /// Get snapping configuration
    pub fn snapping_config(&self) -> crate::gizmo::snapping::SnappingConfig {
        crate::gizmo::snapping::SnappingConfig {
            grid_size: self.grid_snap_size,
            angle_increment: self.angle_snap_increment.to_degrees(),
            grid_enabled: true, // These are handled by toolbar toggles
            angle_enabled: true,
        }
    }

    /// Set snapping configuration
    pub fn set_snapping_config(&mut self, config: crate::gizmo::snapping::SnappingConfig) {
        self.grid_snap_size = config.grid_size;
        self.angle_snap_increment = config.angle_increment.to_radians();
        self.toolbar.snap_size = config.grid_size;
        self.toolbar.angle_snap_degrees = config.angle_increment;
    }

    /// Ray-AABB intersection test for entity picking
    ///
    /// Returns distance to intersection point if ray hits AABB, None otherwise.
    ///
    /// # Arguments
    ///
    /// * `ray_origin` - Ray starting point
    /// * `ray_dir` - Ray direction (normalized)
    /// * `aabb_min` - AABB minimum corner
    /// * `aabb_max` - AABB maximum corner
    fn ray_intersects_aabb(
        ray_origin: glam::Vec3,
        ray_dir: glam::Vec3,
        aabb_min: glam::Vec3,
        aabb_max: glam::Vec3,
    ) -> Option<f32> {
        // Slab method: test ray against each axis pair
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;

        for i in 0..3 {
            let origin = ray_origin[i];
            let dir = ray_dir[i];
            let min = aabb_min[i];
            let max = aabb_max[i];

            if dir.abs() < 1e-6 {
                // Ray parallel to slab - check if origin is inside
                if origin < min || origin > max {
                    return None;
                }
            } else {
                // Calculate intersection distances
                let inv_dir = 1.0 / dir;
                let mut t1 = (min - origin) * inv_dir;
                let mut t2 = (max - origin) * inv_dir;

                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }

                tmin = tmin.max(t1);
                tmax = tmax.min(t2);

                if tmin > tmax {
                    return None;
                }
            }
        }

        // Return closest intersection (positive distance only)
        if tmin >= 0.0 {
            Some(tmin)
        } else if tmax >= 0.0 {
            Some(tmax)
        } else {
            None
        }
    }

    /// Generate debug lines for component gizmos (light radius, collider, audio range)
    fn generate_component_gizmo_lines(
        entity_manager: &crate::entity_manager::EntityManager,
    ) -> Vec<astraweave_physics::DebugLine> {
        let mut lines = Vec::new();
        let segments = 32;

        for (_, entity) in entity_manager.entities() {
            let pos = [entity.position.x, entity.position.y, entity.position.z];

            // Light: yellow wireframe sphere at light range
            if let Some(data) = entity.components.get("Light") {
                let range = data.get("range").and_then(|v| v.as_f64()).unwrap_or(10.0) as f32;
                let color = [1.0, 0.9, 0.2]; // warm yellow
                Self::add_wireframe_sphere(&mut lines, pos, range, color, segments);
            }

            // Collider: green wireframe box/sphere/capsule
            if let Some(data) = entity.components.get("Collider") {
                let color = [0.2, 1.0, 0.3]; // green
                let shape = data.get("shape").and_then(|v| v.as_str()).unwrap_or("box");
                match shape {
                    "sphere" => {
                        let r = data.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
                        Self::add_wireframe_sphere(&mut lines, pos, r, color, segments);
                    }
                    "capsule" => {
                        let r = data.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
                        let hh = data
                            .get("half_height")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0) as f32;
                        // Two circles at top/bottom + 4 vertical lines
                        let top = [pos[0], pos[1] + hh, pos[2]];
                        let bot = [pos[0], pos[1] - hh, pos[2]];
                        Self::add_circle_xz(&mut lines, top, r, color, segments);
                        Self::add_circle_xz(&mut lines, bot, r, color, segments);
                        for &(dx, dz) in &[(r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)] {
                            lines.push(astraweave_physics::DebugLine::new(
                                [pos[0] + dx, pos[1] - hh, pos[2] + dz],
                                [pos[0] + dx, pos[1] + hh, pos[2] + dz],
                                color,
                            ));
                        }
                    }
                    _ => {
                        // Box (default)
                        let size = data
                            .get("size")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                let x = arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                                let y = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                                let z = arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                                [x, y, z]
                            })
                            .unwrap_or([1.0, 1.0, 1.0]);
                        Self::add_wireframe_box(&mut lines, pos, size, color);
                    }
                }
            }

            // Audio: cyan wireframe sphere at max_distance
            if let Some(data) = entity.components.get("Audio") {
                let spatial = data
                    .get("spatial")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if spatial {
                    let max_dist = data
                        .get("max_distance")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(10.0) as f32;
                    let color = [0.2, 0.8, 1.0]; // cyan
                    Self::add_wireframe_sphere(&mut lines, pos, max_dist, color, segments);
                }
            }
        }
        lines
    }

    /// Add a wireframe sphere (3 perpendicular circles) to debug lines
    fn add_wireframe_sphere(
        lines: &mut Vec<astraweave_physics::DebugLine>,
        center: [f32; 3],
        radius: f32,
        color: [f32; 3],
        segments: usize,
    ) {
        Self::add_circle_xz(lines, center, radius, color, segments);
        Self::add_circle_xy(lines, center, radius, color, segments);
        Self::add_circle_yz(lines, center, radius, color, segments);
    }

    fn add_circle_xz(
        lines: &mut Vec<astraweave_physics::DebugLine>,
        center: [f32; 3],
        radius: f32,
        color: [f32; 3],
        segments: usize,
    ) {
        for i in 0..segments {
            let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
            lines.push(astraweave_physics::DebugLine::new(
                [
                    center[0] + radius * a0.cos(),
                    center[1],
                    center[2] + radius * a0.sin(),
                ],
                [
                    center[0] + radius * a1.cos(),
                    center[1],
                    center[2] + radius * a1.sin(),
                ],
                color,
            ));
        }
    }

    fn add_circle_xy(
        lines: &mut Vec<astraweave_physics::DebugLine>,
        center: [f32; 3],
        radius: f32,
        color: [f32; 3],
        segments: usize,
    ) {
        for i in 0..segments {
            let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
            lines.push(astraweave_physics::DebugLine::new(
                [
                    center[0] + radius * a0.cos(),
                    center[1] + radius * a0.sin(),
                    center[2],
                ],
                [
                    center[0] + radius * a1.cos(),
                    center[1] + radius * a1.sin(),
                    center[2],
                ],
                color,
            ));
        }
    }

    fn add_circle_yz(
        lines: &mut Vec<astraweave_physics::DebugLine>,
        center: [f32; 3],
        radius: f32,
        color: [f32; 3],
        segments: usize,
    ) {
        for i in 0..segments {
            let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
            lines.push(astraweave_physics::DebugLine::new(
                [
                    center[0],
                    center[1] + radius * a0.cos(),
                    center[2] + radius * a0.sin(),
                ],
                [
                    center[0],
                    center[1] + radius * a1.cos(),
                    center[2] + radius * a1.sin(),
                ],
                color,
            ));
        }
    }

    fn add_wireframe_box(
        lines: &mut Vec<astraweave_physics::DebugLine>,
        center: [f32; 3],
        size: [f32; 3],
        color: [f32; 3],
    ) {
        let hx = size[0] * 0.5;
        let hy = size[1] * 0.5;
        let hz = size[2] * 0.5;
        let c = center;
        // 8 corners
        let corners = [
            [c[0] - hx, c[1] - hy, c[2] - hz],
            [c[0] + hx, c[1] - hy, c[2] - hz],
            [c[0] + hx, c[1] - hy, c[2] + hz],
            [c[0] - hx, c[1] - hy, c[2] + hz],
            [c[0] - hx, c[1] + hy, c[2] - hz],
            [c[0] + hx, c[1] + hy, c[2] - hz],
            [c[0] + hx, c[1] + hy, c[2] + hz],
            [c[0] - hx, c[1] + hy, c[2] + hz],
        ];
        // 12 edges
        let edges: [(usize, usize); 12] = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // bottom
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4), // top
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7), // vertical
        ];
        for (a, b) in edges {
            lines.push(astraweave_physics::DebugLine::new(
                corners[a], corners[b], color,
            ));
        }
    }

    /// Get the primary selected entity (for single-selection compatibility)
    pub fn selected_entity(&self) -> Option<crate::entity_manager::EntityId> {
        self.selected_entities.first().copied()
    }

    /// Get all selected entities
    pub fn selected_entities(&self) -> &[crate::entity_manager::EntityId] {
        &self.selected_entities
    }

    /// Set the selected entities (replaces current selection)
    pub fn set_selected_entities(&mut self, entities: Vec<crate::entity_manager::EntityId>) {
        self.selected_entities = entities;
    }

    /// Set a single selected entity (clears other selections)
    pub fn set_selected_entity(&mut self, entity: Option<crate::entity_manager::EntityId>) {
        self.selected_entities.clear();
        if let Some(e) = entity {
            self.selected_entities.push(e);
        }
    }

    /// Add an entity to the selection (for multi-select)
    pub fn add_to_selection(&mut self, entity: crate::entity_manager::EntityId) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
        }
    }

    /// Remove an entity from the selection
    pub fn remove_from_selection(&mut self, entity: crate::entity_manager::EntityId) {
        self.selected_entities.retain(|&e| e != entity);
    }

    /// Get reference to the viewport toolbar
    pub fn toolbar(&self) -> &super::toolbar::ViewportToolbar {
        &self.toolbar
    }

    /// Get mutable reference to the viewport toolbar
    pub fn toolbar_mut(&mut self) -> &mut super::toolbar::ViewportToolbar {
        &mut self.toolbar
    }

    /// Get mutable reference to gizmo state
    pub fn gizmo_state_mut(&mut self) -> &mut GizmoState {
        &mut self.gizmo_state
    }

    pub fn load_gltf_model(
        &self,
        name: impl Into<String>,
        path: &std::path::Path,
    ) -> anyhow::Result<()> {
        use pollster::FutureExt;

        let name = name.into();
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        // Lazily initialize engine adapter if not already done
        if !renderer.engine_adapter_initialized() {
            tracing::info!("Initializing engine adapter for PBR rendering...");
            renderer
                .init_engine_adapter()
                .block_on()
                .map_err(|e| anyhow::anyhow!("Failed to initialize engine adapter: {}", e))?;
            tracing::info!("Engine adapter initialized successfully");
        }

        // Load the model into the engine adapter
        if let Some(adapter) = renderer.engine_adapter_mut() {
            adapter.load_gltf_model(&name, path)?;

            // Enable engine rendering now that we have a model
            renderer.set_use_engine_rendering(true);
            tracing::info!("Loaded glTF model '{}' and enabled engine rendering", name);
            Ok(())
        } else {
            anyhow::bail!("Engine adapter not available after initialization")
        }
    }

    /// Set material parameters for the viewport renderer
    pub fn set_material_params(
        &self,
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
    ) -> anyhow::Result<()> {
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter_mut() {
            adapter.set_material_params(base_color, metallic, roughness);
            Ok(())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Get current time of day (0.0 - 24.0 hours)
    pub fn get_time_of_day(&self) -> anyhow::Result<f32> {
        let renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter() {
            Ok(adapter.get_time_of_day())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Set time of day (0.0 - 24.0 hours)
    pub fn set_time_of_day(&self, hour: f32) -> anyhow::Result<()> {
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter_mut() {
            adapter.set_time_of_day(hour);
            Ok(())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Get time scale (1.0 = real time)
    pub fn get_time_scale(&self) -> anyhow::Result<f32> {
        let renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter() {
            Ok(adapter.get_time_scale())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Set time scale (1.0 = real time, 60.0 = fast forward)
    pub fn set_time_scale(&self, scale: f32) -> anyhow::Result<()> {
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter_mut() {
            adapter.set_time_scale(scale);
            Ok(())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Get time-of-day period description (Day/Twilight/Night)
    pub fn get_time_period(&self) -> anyhow::Result<&'static str> {
        let renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter() {
            Ok(adapter.get_time_period())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Check if shadows are enabled
    pub fn shadows_enabled(&self) -> anyhow::Result<bool> {
        let renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter() {
            Ok(adapter.shadows_enabled())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Enable or disable shadows
    pub fn set_shadows_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        let mut renderer = self
            .renderer
            .lock()
            .map_err(|_| anyhow::anyhow!("Renderer mutex poisoned"))?;

        if let Some(adapter) = renderer.engine_adapter_mut() {
            adapter.set_shadows_enabled(enabled);
            Ok(())
        } else {
            anyhow::bail!("Engine adapter not initialized")
        }
    }

    /// Toggle entity selection
    pub fn toggle_selection(&mut self, entity: crate::entity_manager::EntityId) {
        if self.selected_entities.contains(&entity) {
            self.remove_from_selection(entity);
        } else {
            self.add_to_selection(entity);
        }
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    /// Check if an entity is selected
    pub fn is_selected(&self, entity: crate::entity_manager::EntityId) -> bool {
        self.selected_entities.contains(&entity)
    }

    /// Force the entity instance cache to rebuild on the next frame.
    ///
    /// Call after any operation that modifies entity transforms outside
    /// the gizmo drag path (undo, redo, paste, duplicate, property edits).
    pub fn invalidate_entity_cache(&self) {
        self.with_renderer("invalidate_entity_cache", |r| r.invalidate_entity_cache());
    }

    pub fn upload_terrain_chunks(&self, chunks: &[(Vec<super::types::TerrainVertex>, Vec<u32>)]) {
        self.with_renderer("upload_terrain_chunks", |r| r.upload_terrain_chunks(chunks));
    }

    /// Upload terrain chunks using the terrain_integration vertex type directly
    /// (zero-copy path — avoids redundant field-by-field vertex remapping).
    pub fn upload_terrain_chunks_raw(
        &self,
        chunks: &[(Vec<crate::terrain_integration::TerrainVertex>, Vec<u32>)],
    ) {
        self.with_renderer("upload_terrain_chunks_raw", |r| {
            r.upload_terrain_chunks_raw(chunks)
        });
    }

    /// Incrementally update vertex data for a single terrain chunk on the GPU.
    pub fn update_terrain_chunk_vertices(
        &self,
        chunk_index: usize,
        vertices: &[super::types::TerrainVertex],
    ) {
        self.with_renderer("update_terrain_chunk_vertices", |r| {
            r.update_terrain_chunk_vertices(chunk_index, vertices)
        });
    }

    pub fn set_brush_cursor_lines(&self, lines: Vec<astraweave_physics::DebugLine>) {
        self.with_renderer("set_brush_cursor_lines", |r| {
            r.set_brush_cursor_lines(lines)
        });
    }

    /// Set zone overlay lines (blueprint zone wireframes) for 3D rendering.
    pub fn set_zone_overlay_lines(&self, lines: Vec<astraweave_physics::DebugLine>) {
        self.with_renderer("set_zone_overlay_lines", |r| {
            r.set_zone_overlay_lines(lines)
        });
    }

    /// Set scatter placements for instanced vegetation/prop rendering.
    pub fn set_scatter_placements(
        &self,
        placements: Vec<crate::terrain_integration::ScatterPlacement>,
        diffuse_textures: &std::collections::HashMap<String, std::path::PathBuf>,
    ) {
        self.with_renderer("set_scatter_placements", |r| {
            r.set_scatter_placements(placements, diffuse_textures)
        });
    }

    /// Preload glTF meshes from decomposed blend files into the entity renderer cache.
    /// Returns the number of meshes successfully loaded.
    pub fn preload_gltf_meshes(&self, paths: &[String]) -> usize {
        self.with_renderer("preload_gltf_meshes", |r| r.preload_gltf_meshes(paths))
            .unwrap_or(0)
    }

    pub fn clear_terrain(&self) {
        self.with_renderer("clear_terrain", |r| r.clear_terrain());
    }

    /// Set the procedural sky gradient colors for skybox presets / time-of-day / weather
    pub fn set_sky_colors(&self, sky_top: [f32; 4], sky_horizon: [f32; 4], ground_color: [f32; 4]) {
        self.with_renderer("set_sky_colors", |r| {
            r.set_sky_colors(sky_top, sky_horizon, ground_color)
        });
    }

    /// Set fog and weather parameters for distance-based terrain fog rendering
    pub fn set_fog_params(&self, params: super::types::TerrainFogParams) {
        self.with_renderer("set_fog_params", |r| r.set_fog_params(params));
    }

    /// Set lighting parameters for PBR terrain shading
    pub fn set_lighting_params(&self, params: super::types::TerrainLightingParams) {
        self.with_renderer("set_lighting_params", |r| r.set_lighting_params(params));
    }

    /// Set weather type for particle effects
    pub fn set_weather(&self, kind: astraweave_render::WeatherKind) {
        self.with_renderer("set_weather", |r| {
            if let Some(adapter) = r.engine_adapter_mut() {
                adapter.set_weather(kind);
            }
        });
    }

    /// Tick weather particle simulation
    pub fn tick_weather(&self, dt: f32) {
        self.with_renderer("tick_weather", |r| {
            if let Some(adapter) = r.engine_adapter_mut() {
                adapter.tick_weather(dt);
            }
        });
    }

    /// Set water level for volumetric water plane
    pub fn set_water_level(&self, level: f32) {
        self.with_renderer("set_water_level", |r| r.set_water_level(level));
    }

    /// Enable or disable the volumetric water plane
    pub fn set_water_enabled(&self, enabled: bool) {
        self.with_renderer("set_water_enabled", |r| r.set_water_enabled(enabled));
    }

    /// Copy selected entities to clipboard
    fn copy_selection(&mut self, world: &World) -> Option<String> {
        if self.selected_entities.is_empty() {
            return None;
        }

        let entities: Vec<u32> = self.selected_entities.iter().map(|&id| id as u32).collect();
        let data = crate::clipboard::ClipboardData::from_entities(world, &entities);
        self.clipboard = Some(data.clone());
        debug!("Copied {} entities to clipboard", entities.len());

        data.to_json().ok()
    }

    /// Paste entities from clipboard
    fn paste_selection(
        &mut self,
        world: &mut World,
        _undo_stack: &mut crate::command::UndoStack,
        clipboard_text: Option<String>,
    ) {
        let clipboard = if let Some(text) = clipboard_text {
            // Try parsing OS clipboard first
            crate::clipboard::ClipboardData::from_json(&text)
                .ok()
                .or_else(|| self.clipboard.clone())
        } else {
            self.clipboard.clone()
        };

        if let Some(clipboard) = &clipboard {
            let offset = astraweave_core::IVec2::new(2, 2);

            match clipboard.spawn_entities(world, offset) {
                Ok(spawned) => {
                    let count = spawned.len();
                    self.selected_entities = spawned.into_iter().map(|id| id as u64).collect();
                    debug!("Pasted {} entities", count);
                    // New entities with poses — force renderer rebuild
                    self.invalidate_entity_cache();
                }
                Err(e) => {
                    debug!("Paste failed: {}", e);
                }
            }
        } else {
            debug!("Clipboard is empty");
        }
    }

    /// Duplicate selected entities (creates copies at offset position)
    /// Uses DuplicateEntitiesCommand for full undo/redo support
    fn duplicate_selection(
        &mut self,
        world: &mut World,
        undo_stack: &mut crate::command::UndoStack,
    ) {
        if self.selected_entities.is_empty() {
            debug!("duplicate_selection: No entities selected");
            return;
        }

        debug!(
            "duplicate_selection: Duplicating {} entities via command: {:?}",
            self.selected_entities.len(),
            self.selected_entities
        );

        // Use DuplicateEntitiesCommand for proper undo support
        let source_entities: Vec<u32> =
            self.selected_entities.iter().map(|&id| id as u32).collect();
        let offset = astraweave_core::IVec2 { x: 2, y: 0 }; // Offset 2 units right

        let duplicate_cmd = crate::command::DuplicateEntitiesCommand::new(source_entities, offset);

        match undo_stack.execute(duplicate_cmd, world, None) {
            Ok(()) => {
                // Get the spawned entities from the command (they're stored in the command)
                // For now, we don't have direct access to them after execute, so we'll
                // keep the original selection (the new entities are in the world)
                debug!("duplicate_selection: Command executed successfully");
                // Duplicated entities have poses — force renderer rebuild
                self.invalidate_entity_cache();
            }
            Err(e) => {
                debug!("duplicate_selection failed: {}", e);
            }
        }
    }

    /// Delete selected entities
    pub fn delete_selection(
        &mut self,
        world: &mut World,
        undo_stack: &mut crate::command::UndoStack,
    ) {
        if self.selected_entities.is_empty() {
            return;
        }

        let entities_to_delete: Vec<u32> =
            self.selected_entities.iter().map(|&id| id as u32).collect();

        let delete_cmd = crate::command::DeleteEntitiesCommand::new(entities_to_delete);
        if let Err(e) = undo_stack.execute(delete_cmd, world, None) {
            debug!("Delete failed: {}", e);
        }

        // Entities removed — force renderer rebuild
        self.invalidate_entity_cache();
        self.clear_selection();

        if self.gizmo_state.is_active() {
            self.gizmo_state.mode = GizmoMode::Inactive;
            self.gizmo_state.start_transform = None;
            self.drag_offset = None;
        }
    }

    /// Select all entities in the world
    fn select_all(&mut self, world: &World) {
        self.selected_entities.clear();

        debug!("select_all: Starting scan for entities...");

        // Iterate through all entities (World doesn't expose entity list, so we try a range)
        // This is a workaround - ideally World would have an entities() iterator
        for entity_id in 0..1000 {
            if world.pose(entity_id).is_some() {
                self.selected_entities.push(entity_id.into());
                debug!("  Found entity {}", entity_id);
            }
        }

        debug!(
            "select_all: Selected {} entities total: {:?}",
            self.selected_entities.len(),
            self.selected_entities
        );
    }

    /// Snap a float value to the grid
    fn snap_to_grid(&self, value: f32) -> f32 {
        if self.grid_snap_size > 0.0 {
            (value / self.grid_snap_size).round() * self.grid_snap_size
        } else {
            value
        }
    }

    /// Snap an angle to the nearest increment
    fn snap_angle(&self, angle: f32) -> f32 {
        if self.angle_snap_increment > 0.0 {
            (angle / self.angle_snap_increment).round() * self.angle_snap_increment
        } else {
            angle
        }
    }
}

fn estimate_memory_usage_mb() -> f32 {
    #[cfg(target_os = "windows")]
    {
        use std::mem::MaybeUninit;
        #[repr(C)]
        struct ProcessMemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }
        #[link(name = "psapi")]
        extern "system" {
            fn GetProcessMemoryInfo(
                process: *mut std::ffi::c_void,
                pmc: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
        }
        // SAFETY: Calling Win32 `GetProcessMemoryInfo` with a properly sized
        // `ProcessMemoryCounters` struct. Standard Win32 memory query pattern.
        unsafe {
            let mut pmc = MaybeUninit::<ProcessMemoryCounters>::uninit();
            (*pmc.as_mut_ptr()).cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
            if GetProcessMemoryInfo(
                GetCurrentProcess(),
                pmc.as_mut_ptr(),
                std::mem::size_of::<ProcessMemoryCounters>() as u32,
            ) != 0
            {
                return (*pmc.as_ptr()).working_set_size as f32 / (1024.0 * 1024.0);
            }
        }
        0.0
    }
    #[cfg(not(target_os = "windows"))]
    {
        0.0
    }
}

// SAFETY: ViewportWidget owns wgpu resources (textures), which are NOT Send/Sync.
// However, egui requires widgets to be Send. We ensure safety by:
// 1. Only creating wgpu resources on the main thread
// 2. Never sending ViewportWidget across threads
// 3. Using Arc for shared GPU resources in renderer
//
// This is safe because eframe runs on a single thread (winit event loop).
// If we later add multi-threading, we'll need to refactor to use Arc<Mutex<>>.

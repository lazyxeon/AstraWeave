//! Editor multi-tool architecture: ActiveTool trait + Dispatcher.
//!
//! Per `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §2.
//! Implements Approach I+II hybrid synthesis per research audit §7.7
//! (`docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`)
//! with Fyrox InteractionMode precedent per research audit §5.1.
//!
//! Sub-phase 2 deliverable: trait + dispatcher core + register_tool API
//! with module-level unit tests. NO integration into editor runtime path
//! (Sub-phase 3+ work). NO TerrainPanel/RegionalArchetypePanel ActiveTool
//! impls (Sub-phase 3 + Sub-phase 5 work).

use uuid::Uuid;

// =============================================================================
// EventDisposition
// =============================================================================

/// Tool's disposition for an input event: did the tool claim it, or pass through?
///
/// Per campaign doc §2.3 + Sub-phase 1 Diagnostic audit §3.3 verification:
/// existing ViewportWidget event-handling has binary tool-consumed-or-camera-
/// pass-through semantics implicitly; this enum makes those semantics explicit.
///
/// Declared `#[non_exhaustive]` for forward-compatibility per Andrew Q4: future
/// additions (e.g., `ConsumedSelective` for hover-feedback tools per Godot 4
/// `AfterGUIInput::CUSTOM`-analog precedent) land without breaking existing
/// consumers using match guards. Consumers MUST use match guards rather than
/// wildcard patterns to participate in the forward-compatibility contract.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDisposition {
    /// Tool claimed the event; block downstream handling (camera control, etc.).
    Consumed,
    /// Tool didn't claim; let camera/default handler process.
    PassThrough,
    // Future variant per Q4 timeline: ConsumedSelective — when hover-feedback
    // tools land, this addition won't break match-guarded consumers.
}

// =============================================================================
// MouseEvent + KeyEvent + Kind discriminators
// =============================================================================

/// Minimal mouse event payload per Andrew Q3 (a).
///
/// World-XZ projection accessed via [`ToolContext::world_xz_at_pointer`] /
/// [`ToolContext::world_xz_at_y0`] methods rather than baked into the event
/// payload — keeps the event minimal and decouples the projection mechanism
/// from the event shape. This resolves the §2.7 ToolContext open question
/// from Sub-phase 1 Diagnostic audit §12.4.
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    /// Pointer position in viewport-local coordinates.
    pub pointer_pos: egui::Pos2,
    /// Modifier keys at event time.
    pub modifiers: egui::Modifiers,
    /// Drag delta since last frame (zero for non-drag events).
    pub drag_delta: egui::Vec2,
}

/// Mouse event discriminator for [`Dispatcher::dispatch_mouse_event`] routing.
///
/// Per campaign doc §2.4 push-based dispatch + audit §4.3 verification: egui's
/// `Sense::click_and_drag()` + `Response` API produces discrete events that
/// map cleanly to this enum's variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Primary-button press.
    LeftButtonDown,
    /// Primary-button release.
    LeftButtonUp,
    /// Pointer movement (with or without buttons held).
    Move,
    // Future per audit §10.5: MiddleButtonDown/Up + RightButtonDown/Up if needed.
    // Middle-button events are currently handled in ViewportWidget for camera
    // control (per audit §3.2 informational note); not routed through dispatcher.
}

/// Minimal key event payload per Andrew Q3 (a).
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub key: egui::Key,
    pub modifiers: egui::Modifiers,
}

/// Key event discriminator for [`Dispatcher::dispatch_key_event`] routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Down,
    Up,
}

// =============================================================================
// ToolContext (resolves §2.7 open question per Sub-phase 1 Diagnostic §12.4)
// =============================================================================

/// Per-event context passed to [`ActiveTool`] methods.
///
/// Per Andrew Q2 (b): exposes world-XZ projection as **methods** rather than
/// raw camera/depth-buffer references. Per Sub-phase 1 Diagnostic §1.3
/// inspection findings: ViewportWidget computes world-XZ projections per-frame
/// (depth-buffer + camera unprojection at viewport/widget.rs:1219-1234, with
/// Y=0 ray-plane fallback) and stores the cached results in this struct before
/// dispatching events. Tools call [`Self::world_xz_at_pointer`] or
/// [`Self::world_xz_at_y0`] to read the pre-computed values without touching
/// the renderer's `Arc<Mutex<>>` lock or the camera type directly.
///
/// Owned struct (no lifetime parameter) per Sub-phase 2 design: simplifies
/// dispatcher API + enables synthetic test fixtures without complex borrow
/// orchestration. Mutable scene-state access (per audit §12.4 Andrew Q2 (c))
/// is deferred to Sub-phase 3+ tools' choice — tools that need scene mutation
/// add fields to this struct or use existing queue patterns (e.g., TerrainPanel's
/// `pending_actions: Vec<TerrainAction>`).
///
/// Does NOT carry `&mut UndoStack` per Sub-phase 1 Diagnostic audit §12.4 §2.11
/// deferral — that ergonomic choice is resolved in Sub-phase 5 prompt drafting
/// when RegionalArchetypePanel registration forces the decision.
pub struct ToolContext {
    /// Viewport rect in screen coordinates (top-left + size).
    pub viewport_rect: egui::Rect,
    /// Pointer position in viewport-local coordinates, if hovered.
    pub pointer_pos: Option<egui::Pos2>,
    /// Modifier keys at event time.
    pub modifiers: egui::Modifiers,

    /// Pre-computed depth-buffer + camera-unprojection world-XZ projection.
    /// `Some((world_x, world_z))` if depth at pointer is non-sky and unprojection
    /// produced a valid world position; `None` otherwise (sky depth = 1.0,
    /// pointer outside viewport, depth buffer unavailable).
    ///
    /// Used by tools that need surface-following projection (e.g., terrain
    /// sculpt brush — TerrainPanel's existing pattern at
    /// viewport/widget.rs:1219-1234).
    world_xz_at_pointer_cached: Option<(f32, f32)>,

    /// Pre-computed ray-plane intersection at world-Y=0 plane.
    /// `Some((world_x, world_z))` if pointer ray hits Y=0 in front of camera;
    /// `None` if ray is parallel to or above the plane.
    ///
    /// Used by tools that don't need depth-accurate projection (e.g.,
    /// RegionalArchetypePanel paint per F.5-paint.B's `screen_to_world_xz_y0`
    /// pattern at panels/regional_archetype_panel.rs:414).
    world_xz_at_y0_cached: Option<(f32, f32)>,
}

impl ToolContext {
    /// Construct a ToolContext with explicit field values.
    ///
    /// Sub-phase 3+ ViewportWidget integration computes the cached world-XZ
    /// projections per-frame and passes them in. Sub-phase 2 unit tests use
    /// [`Self::for_test`] constructor for synthetic fixtures.
    pub fn new(
        viewport_rect: egui::Rect,
        pointer_pos: Option<egui::Pos2>,
        modifiers: egui::Modifiers,
        world_xz_at_pointer: Option<(f32, f32)>,
        world_xz_at_y0: Option<(f32, f32)>,
    ) -> Self {
        Self {
            viewport_rect,
            pointer_pos,
            modifiers,
            world_xz_at_pointer_cached: world_xz_at_pointer,
            world_xz_at_y0_cached: world_xz_at_y0,
        }
    }

    /// Synthetic ToolContext for unit tests; all fields default/empty.
    /// Sub-phase 4 Pattern A regression infrastructure may extend with
    /// configurable test contexts as test scenarios require.
    #[cfg(test)]
    pub fn for_test() -> Self {
        Self {
            viewport_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
            pointer_pos: None,
            modifiers: egui::Modifiers::NONE,
            world_xz_at_pointer_cached: None,
            world_xz_at_y0_cached: None,
        }
    }

    /// World-XZ projection at the pointer using depth-buffer reading
    /// (surface-following).
    ///
    /// Returns `None` if the pointer is outside the viewport, the depth at
    /// pointer is sky (depth = 1.0), or the depth buffer was unavailable when
    /// ViewportWidget pre-computed the cached value.
    ///
    /// Wraps the canonical pattern at viewport/widget.rs:1219-1234 (read depth
    /// at pixel → unproject via OrbitCamera::unproject_depth_to_world).
    pub fn world_xz_at_pointer(&self) -> Option<(f32, f32)> {
        self.world_xz_at_pointer_cached
    }

    /// World-XZ projection at the pointer using ray-plane intersection at Y=0.
    ///
    /// Returns `None` if the pointer ray is parallel to or above the Y=0 plane.
    ///
    /// Wraps F.5-paint.B's `screen_to_world_xz_y0` ray-plane projection
    /// pattern at panels/regional_archetype_panel.rs:414.
    pub fn world_xz_at_y0(&self) -> Option<(f32, f32)> {
        self.world_xz_at_y0_cached
    }
}

// =============================================================================
// ActiveTool trait
// =============================================================================

/// Editor's active-tool trait.
///
/// Implementors register via [`Dispatcher::register_tool`] and receive
/// per-event method calls when active. Method surface mirrors Fyrox
/// `InteractionMode` pattern per research audit §5.1 + campaign doc §2.2 +
/// Sub-phase 1 Diagnostic audit §2.2.3 verification finding.
///
/// **Default-implementation pattern** per Andrew Q4 (a)(b):
/// - Per-event handlers default to [`EventDisposition::PassThrough`]; tools
///   override only relevant methods.
/// - Lifecycle methods (`activate`/`deactivate`/`update`/`on_drop`) default to
///   no-op; tools override only relevant methods.
/// - [`Self::make_button`] defaults to a simple `selectable_label` using
///   [`Self::name`]; tools that want richer UI override.
///
/// **UUID identity per Q5 mod-friendliness**: open-set extensibility
/// (third-party tools generate their own UUIDs without conflicting with
/// first-party). First-party tools register with documented constants.
pub trait ActiveTool {
    /// UUID identity. First-party tools use documented constants; third-party
    /// tools generate their own (random UUID; collision probability negligible).
    fn uuid(&self) -> Uuid;

    /// Display name for UI integration (toolbar button, settings panel header).
    fn name(&self) -> &str;

    /// Lifecycle: tool activated when [`Dispatcher::set_active_tool`] selects
    /// this tool. Previous active tool's [`Self::deactivate`] is called first.
    fn activate(&mut self, _context: &mut ToolContext) {}

    /// Lifecycle: tool deactivated when [`Dispatcher::set_active_tool`] selects
    /// a different tool or `None`.
    fn deactivate(&mut self, _context: &mut ToolContext) {}

    /// Per-frame update; called only when this tool is active.
    fn update(&mut self, _context: &mut ToolContext) {}

    /// Lifecycle: tool dropped from registry (rare; e.g., editor shutdown or
    /// hot-reload scenarios).
    fn on_drop(&mut self, _context: &mut ToolContext) {}

    /// Primary mouse-button press. Defaults to `PassThrough`.
    fn on_left_mouse_button_down(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// Primary mouse-button release. Defaults to `PassThrough`.
    fn on_left_mouse_button_up(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// Pointer movement (with or without buttons held). Defaults to `PassThrough`.
    fn on_mouse_move(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// Pointer entered the viewport.
    fn on_mouse_enter(&mut self, _context: &mut ToolContext) {}

    /// Pointer left the viewport.
    fn on_mouse_leave(&mut self, _context: &mut ToolContext) {}

    /// Key press. Defaults to `PassThrough`.
    fn on_key_down(
        &mut self,
        _key: &KeyEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// Key release. Defaults to `PassThrough`.
    fn on_key_up(
        &mut self,
        _key: &KeyEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// UI integration: tool provides its own toolbar button widget.
    ///
    /// Default per Andrew Q4 (b): `selectable_label` using [`Self::name`].
    /// Tools that want richer UI (e.g., icon buttons; tooltip; per-tool
    /// indicator) override.
    ///
    /// Note: clicked() detection and [`Dispatcher::set_active_tool`]
    /// integration is ViewportWidget's job (Sub-phase 3) — `make_button` only
    /// renders the button.
    fn make_button(&mut self, ui: &mut egui::Ui, selected: bool) {
        let _ = ui.selectable_label(selected, self.name());
    }
}

// =============================================================================
// Dispatcher (Sub-phase 2 Core.B)
// =============================================================================

mod dispatcher;
pub use dispatcher::Dispatcher;

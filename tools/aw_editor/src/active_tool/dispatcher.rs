//! [`Dispatcher`]: editor multi-tool architecture's central tool-event router.
//!
//! Per `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §2.4 + §2.5 +
//! §2.8. Implements Approach I+II hybrid synthesis per research audit §7.7
//! (`docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`):
//! explicit registry (Approach I) owns trait-object collection (Approach II);
//! dispatcher tracks active tool by UUID; per-event method calls only on the
//! active tool.
//!
//! Sub-phase 1 Diagnostic audit §4.3 + §5.3 + §8.3 verified compatibility:
//! egui's discrete events map to push-based dispatch; Editor::new() is
//! structurally additive for `register_tool` calls; existing single-active-
//! tool pattern (`terrain_brush_active`) generalizes cleanly to
//! `active_tool: Option<Uuid>`.

use std::collections::HashMap;
use uuid::Uuid;

use super::{
    ActiveTool, EventDisposition, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind, ToolContext,
};

/// Editor multi-tool dispatcher.
///
/// Owns the registered tool collection + tracks the active tool. Routes
/// per-event method calls to the active tool; non-active tools' methods
/// are NOT called (push-based dispatch per campaign doc §2.4 + research
/// audit §7.2 push-based optimization rationale).
///
/// Mutex arbitration per campaign doc §2.8: framework-enforced via
/// [`Self::active_tool`] field; [`Self::set_active_tool`] transitions
/// previous-deactivate → new-activate.
pub struct Dispatcher {
    /// UUID of the currently active tool, if any. `None` means no tool is
    /// active and events pass through to default handling (camera, etc.).
    active_tool: Option<Uuid>,

    /// Registered tools indexed by UUID. Open-set per Q5 mod-friendliness:
    /// third-party tools generate their own UUIDs without conflicting with
    /// first-party.
    tools: HashMap<Uuid, Box<dyn ActiveTool>>,
}

impl Dispatcher {
    /// Construct a new empty dispatcher with no registered tools.
    pub fn new() -> Self {
        Self {
            active_tool: None,
            tools: HashMap::new(),
        }
    }

    /// Register a tool. Subsequent registrations with the same UUID overwrite
    /// the previous registration (last-write-wins semantic).
    ///
    /// Per campaign doc §2.5 + Q5 mod-friendliness: explicit `register_tool`
    /// API at editor init. First-party tools register with documented UUID
    /// constants; third-party tools generate their own.
    pub fn register_tool(&mut self, tool: Box<dyn ActiveTool>) {
        let uuid = tool.uuid();
        self.tools.insert(uuid, tool);
    }

    /// Return whether a tool with the given UUID is registered.
    pub fn is_registered(&self, uuid: Uuid) -> bool {
        self.tools.contains_key(&uuid)
    }

    /// Return the UUID of the currently active tool, if any.
    pub fn active_tool_uuid(&self) -> Option<Uuid> {
        self.active_tool
    }

    /// Set the active tool, transitioning lifecycle methods.
    ///
    /// Per campaign doc §2.8 mutex arbitration + Sub-phase 1 Diagnostic §8.3
    /// verification: framework-enforced single-active-tool mutex.
    ///
    /// Behavior:
    /// - If `uuid == self.active_tool`, no-op (no spurious deactivate/activate
    ///   calls when re-setting to the already-active tool).
    /// - Otherwise: previous active tool's [`ActiveTool::deactivate`] called
    ///   (if any), then new active tool's [`ActiveTool::activate`] called
    ///   (if any). The new active tool's UUID is stored in `self.active_tool`
    ///   regardless of whether it's registered (graceful handling per audit
    ///   §6.2 — dispatch returns PassThrough if active tool's UUID is not
    ///   registered).
    pub fn set_active_tool(&mut self, uuid: Option<Uuid>, context: &mut ToolContext) {
        if uuid == self.active_tool {
            return;
        }

        // Deactivate previous if any
        if let Some(prev_uuid) = self.active_tool {
            if let Some(prev_tool) = self.tools.get_mut(&prev_uuid) {
                prev_tool.deactivate(context);
            }
        }

        // Activate new if any
        if let Some(new_uuid) = uuid {
            if let Some(new_tool) = self.tools.get_mut(&new_uuid) {
                new_tool.activate(context);
            }
        }

        self.active_tool = uuid;
    }

    /// Dispatch a mouse event to the active tool.
    ///
    /// Returns the active tool's [`EventDisposition`]. Returns
    /// [`EventDisposition::PassThrough`] if no active tool is set OR the
    /// active tool's UUID is not registered (graceful handling per Sub-phase
    /// 1 Diagnostic audit §6.2).
    ///
    /// Per campaign doc §2.4 push-based dispatch + audit §4.3: ViewportWidget
    /// builds a [`MouseEvent`] from egui's `Response` API + a [`MouseEventKind`]
    /// discriminator + a [`ToolContext`] with pre-computed world-XZ projections
    /// per §1.3 inspection, calls this method, and consumes the result to
    /// drive subsequent camera handler decisions.
    pub fn dispatch_mouse_event(
        &mut self,
        event: &MouseEvent,
        kind: MouseEventKind,
        context: &mut ToolContext,
    ) -> EventDisposition {
        let Some(uuid) = self.active_tool else {
            return EventDisposition::PassThrough;
        };
        let Some(tool) = self.tools.get_mut(&uuid) else {
            return EventDisposition::PassThrough;
        };
        match kind {
            MouseEventKind::LeftButtonDown => tool.on_left_mouse_button_down(event, context),
            MouseEventKind::LeftButtonUp => tool.on_left_mouse_button_up(event, context),
            MouseEventKind::Move => tool.on_mouse_move(event, context),
        }
    }

    /// Dispatch a key event to the active tool. Analog of
    /// [`Self::dispatch_mouse_event`] for keyboard events.
    pub fn dispatch_key_event(
        &mut self,
        key: &KeyEvent,
        kind: KeyEventKind,
        context: &mut ToolContext,
    ) -> EventDisposition {
        let Some(uuid) = self.active_tool else {
            return EventDisposition::PassThrough;
        };
        let Some(tool) = self.tools.get_mut(&uuid) else {
            return EventDisposition::PassThrough;
        };
        match kind {
            KeyEventKind::Down => tool.on_key_down(key, context),
            KeyEventKind::Up => tool.on_key_up(key, context),
        }
    }

    /// Notify the active tool that the pointer entered the viewport.
    pub fn dispatch_mouse_enter(&mut self, context: &mut ToolContext) {
        if let Some(uuid) = self.active_tool {
            if let Some(tool) = self.tools.get_mut(&uuid) {
                tool.on_mouse_enter(context);
            }
        }
    }

    /// Notify the active tool that the pointer left the viewport.
    pub fn dispatch_mouse_leave(&mut self, context: &mut ToolContext) {
        if let Some(uuid) = self.active_tool {
            if let Some(tool) = self.tools.get_mut(&uuid) {
                tool.on_mouse_leave(context);
            }
        }
    }

    /// Per-frame update for the active tool only. Called once per editor frame
    /// (typically inside `EditorApp::update` after dispatching input events).
    /// Inactive tools' `update` methods are NOT called.
    pub fn update_active_tool(&mut self, context: &mut ToolContext) {
        if let Some(uuid) = self.active_tool {
            if let Some(tool) = self.tools.get_mut(&uuid) {
                tool.update(context);
            }
        }
    }

    /// Iterator over registered tools' UUIDs. Used by toolbar UI to enumerate
    /// available tools + their `make_button` widgets.
    pub fn registered_uuids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.tools.keys().copied()
    }

    /// Number of registered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

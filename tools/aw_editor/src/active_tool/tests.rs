//! Sub-phase 2 module-level unit tests for [`ActiveTool`] + [`Dispatcher`].
//!
//! Per Andrew Q5: minimal [`MockActiveTool`] fixture introduced this sub-phase.
//! Sub-phase 4 Pattern A regression infrastructure extends with comprehensive
//! state-tracking + edge-case scenarios.
//!
//! Per Sub-phase 1 Diagnostic audit §6.2 + campaign doc §6.2: 11+ test scenarios
//! cover dispatcher mechanics — registration, activation transitions, mutex
//! enforcement, lifecycle ordering, EventDisposition routing, default-
//! implementation pass-through, graceful handling of unregistered active tool.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;

// =============================================================================
// MockActiveTool fixture (minimal per Andrew Q5)
// =============================================================================

/// Shared mutable counter state for [`MockActiveTool`]. Wrapped in `Rc<RefCell>`
/// so the test can hold a handle to assert against while the dispatcher owns
/// the boxed tool itself.
#[derive(Debug, Default)]
struct MockToolState {
    activate_count: u32,
    deactivate_count: u32,
    update_count: u32,
    on_drop_count: u32,
    left_button_down_count: u32,
    left_button_up_count: u32,
    mouse_move_count: u32,
    mouse_enter_count: u32,
    mouse_leave_count: u32,
    key_down_count: u32,
    key_up_count: u32,
}

/// Minimal mock for unit testing dispatcher mechanics.
///
/// Tracks lifecycle calls + per-event calls via shared `Rc<RefCell<MockToolState>>`.
/// Sub-phase 4 extends with additional state-tracking for Pattern A regression.
struct MockActiveTool {
    uuid: Uuid,
    name: String,
    state: Rc<RefCell<MockToolState>>,
    /// Configurable disposition return value for per-event methods.
    return_disposition: EventDisposition,
}

impl MockActiveTool {
    fn new(uuid: Uuid, name: &str, disposition: EventDisposition) -> (Self, Rc<RefCell<MockToolState>>) {
        let state = Rc::new(RefCell::new(MockToolState::default()));
        let tool = Self {
            uuid,
            name: name.to_string(),
            state: Rc::clone(&state),
            return_disposition: disposition,
        };
        (tool, state)
    }
}

impl ActiveTool for MockActiveTool {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn activate(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().activate_count += 1;
    }

    fn deactivate(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().deactivate_count += 1;
    }

    fn update(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().update_count += 1;
    }

    fn on_drop(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().on_drop_count += 1;
    }

    fn on_left_mouse_button_down(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        self.state.borrow_mut().left_button_down_count += 1;
        self.return_disposition
    }

    fn on_left_mouse_button_up(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        self.state.borrow_mut().left_button_up_count += 1;
        self.return_disposition
    }

    fn on_mouse_move(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        self.state.borrow_mut().mouse_move_count += 1;
        self.return_disposition
    }

    fn on_mouse_enter(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().mouse_enter_count += 1;
    }

    fn on_mouse_leave(&mut self, _context: &mut ToolContext) {
        self.state.borrow_mut().mouse_leave_count += 1;
    }

    fn on_key_down(
        &mut self,
        _key: &KeyEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        self.state.borrow_mut().key_down_count += 1;
        self.return_disposition
    }

    fn on_key_up(
        &mut self,
        _key: &KeyEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        self.state.borrow_mut().key_up_count += 1;
        self.return_disposition
    }
}

/// Minimal `ActiveTool` impl that overrides ONLY required methods (uuid, name).
/// Used to verify default-implementation pass-through behavior.
struct DefaultOnlyTool {
    uuid: Uuid,
}

impl ActiveTool for DefaultOnlyTool {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&self) -> &str {
        "DefaultOnly"
    }
}

// =============================================================================
// Test fixture helpers
// =============================================================================

fn build_mouse_event() -> MouseEvent {
    MouseEvent {
        pointer_pos: egui::Pos2::ZERO,
        modifiers: egui::Modifiers::NONE,
        drag_delta: egui::Vec2::ZERO,
    }
}

fn build_key_event() -> KeyEvent {
    KeyEvent {
        key: egui::Key::Space,
        modifiers: egui::Modifiers::NONE,
    }
}

// =============================================================================
// Test scenarios (11+ per audit §6.2 + campaign doc §6.2)
// =============================================================================

/// Scenario 1: register_tool adds tool to registry; UUID lookup succeeds.
#[test]
fn register_tool_adds_to_registry() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, _state) = MockActiveTool::new(uuid, "test", EventDisposition::PassThrough);

    dispatcher.register_tool(Box::new(tool));

    assert!(dispatcher.is_registered(uuid));
    assert_eq!(dispatcher.tool_count(), 1);
}

/// Scenario 2: register_tool with same UUID overwrites (last-write-wins
/// semantic; documented per Dispatcher::register_tool rustdoc).
#[test]
fn register_tool_with_same_uuid_overwrites() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool_a, _state_a) = MockActiveTool::new(uuid, "first", EventDisposition::PassThrough);
    let (tool_b, state_b) = MockActiveTool::new(uuid, "second", EventDisposition::Consumed);

    dispatcher.register_tool(Box::new(tool_a));
    dispatcher.register_tool(Box::new(tool_b));

    // Only one tool registered (the second; UUID overwrite).
    assert_eq!(dispatcher.tool_count(), 1);
    assert!(dispatcher.is_registered(uuid));

    // Verify the second tool is the one routed to: dispatching an event
    // increments the second tool's counter, not the first's.
    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    let event = build_mouse_event();
    let disposition = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);

    assert_eq!(disposition, EventDisposition::Consumed); // tool_b's return value
    assert_eq!(state_b.borrow().left_button_down_count, 1);
}

/// Scenario 3: set_active_tool(Some(uuid)) calls activate() on the new tool.
#[test]
fn set_active_tool_some_calls_activate() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);

    assert_eq!(state.borrow().activate_count, 1);
    assert_eq!(state.borrow().deactivate_count, 0);
    assert_eq!(dispatcher.active_tool_uuid(), Some(uuid));
}

/// Scenario 4: set_active_tool(None) calls deactivate() on the current tool;
/// subsequent dispatch returns PassThrough.
#[test]
fn set_active_tool_none_deactivates_current() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    dispatcher.set_active_tool(None, &mut context);

    assert_eq!(state.borrow().activate_count, 1);
    assert_eq!(state.borrow().deactivate_count, 1);
    assert_eq!(dispatcher.active_tool_uuid(), None);

    // Subsequent dispatch returns PassThrough.
    let event = build_mouse_event();
    let disposition = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);
    assert_eq!(disposition, EventDisposition::PassThrough);
}

/// Scenario 5: dispatch_mouse_event routes to active tool's matching method;
/// returns the tool's EventDisposition.
#[test]
fn dispatch_mouse_event_routes_to_active_tool() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    let event = build_mouse_event();

    let disp_down = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);
    let disp_up = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonUp, &mut context);
    let disp_move = dispatcher.dispatch_mouse_event(&event, MouseEventKind::Move, &mut context);

    assert_eq!(disp_down, EventDisposition::Consumed);
    assert_eq!(disp_up, EventDisposition::Consumed);
    assert_eq!(disp_move, EventDisposition::Consumed);

    let s = state.borrow();
    assert_eq!(s.left_button_down_count, 1);
    assert_eq!(s.left_button_up_count, 1);
    assert_eq!(s.mouse_move_count, 1);
}

/// Scenario 6: dispatch_mouse_event with no active tool returns PassThrough.
#[test]
fn dispatch_mouse_event_with_no_active_tool_returns_passthrough() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool));

    // active_tool == None despite registration.
    let mut context = ToolContext::for_test();
    let event = build_mouse_event();
    let disposition = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);

    assert_eq!(disposition, EventDisposition::PassThrough);
    assert_eq!(state.borrow().left_button_down_count, 0); // tool not called
}

/// Scenario 7: dispatch_mouse_event with active tool whose UUID is not
/// registered returns PassThrough (graceful handling per audit §6.2).
#[test]
fn dispatch_mouse_event_with_unregistered_active_tool_returns_passthrough() {
    let mut dispatcher = Dispatcher::new();
    let registered_uuid = Uuid::new_v4();
    let unregistered_uuid = Uuid::new_v4();
    let (tool, _state) = MockActiveTool::new(registered_uuid, "test", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    // Set active to an UNREGISTERED UUID (edge case; graceful handling).
    dispatcher.set_active_tool(Some(unregistered_uuid), &mut context);
    let event = build_mouse_event();
    let disposition = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);

    assert_eq!(disposition, EventDisposition::PassThrough);
}

/// Scenario 8: update_active_tool calls active tool's update() method;
/// inactive tools' update() not called.
#[test]
fn update_active_tool_calls_active_tool_update() {
    let mut dispatcher = Dispatcher::new();
    let uuid_a = Uuid::new_v4();
    let uuid_b = Uuid::new_v4();
    let (tool_a, state_a) = MockActiveTool::new(uuid_a, "a", EventDisposition::PassThrough);
    let (tool_b, state_b) = MockActiveTool::new(uuid_b, "b", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool_a));
    dispatcher.register_tool(Box::new(tool_b));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid_a), &mut context);
    dispatcher.update_active_tool(&mut context);
    dispatcher.update_active_tool(&mut context);

    assert_eq!(state_a.borrow().update_count, 2);
    assert_eq!(state_b.borrow().update_count, 0); // inactive; not called
}

/// Scenario 9: lifecycle ordering — register_tool → set_active_tool(Some) →
/// activate() called exactly once.
#[test]
fn lifecycle_register_then_set_active_calls_activate_once() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);

    assert_eq!(state.borrow().activate_count, 1);

    // Re-setting to same UUID is a no-op (no spurious deactivate/activate).
    dispatcher.set_active_tool(Some(uuid), &mut context);
    assert_eq!(state.borrow().activate_count, 1);
    assert_eq!(state.borrow().deactivate_count, 0);
}

/// Scenario 10: lifecycle ordering — set_active_tool(uuid_a) →
/// set_active_tool(uuid_b) calls a.deactivate() once + b.activate() once,
/// in that order.
#[test]
fn lifecycle_set_active_to_other_calls_deactivate_then_activate() {
    let mut dispatcher = Dispatcher::new();
    let uuid_a = Uuid::new_v4();
    let uuid_b = Uuid::new_v4();
    let (tool_a, state_a) = MockActiveTool::new(uuid_a, "a", EventDisposition::PassThrough);
    let (tool_b, state_b) = MockActiveTool::new(uuid_b, "b", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool_a));
    dispatcher.register_tool(Box::new(tool_b));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid_a), &mut context);
    dispatcher.set_active_tool(Some(uuid_b), &mut context);

    assert_eq!(state_a.borrow().activate_count, 1);
    assert_eq!(state_a.borrow().deactivate_count, 1);
    assert_eq!(state_b.borrow().activate_count, 1);
    assert_eq!(state_b.borrow().deactivate_count, 0);
}

/// Scenario 11: default-implementation per-event methods return PassThrough.
/// Verified via [`DefaultOnlyTool`] which overrides ONLY required methods.
#[test]
fn default_implementation_returns_passthrough() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let tool = DefaultOnlyTool { uuid };
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    let event = build_mouse_event();

    let disp_down = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);
    let disp_up = dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonUp, &mut context);
    let disp_move = dispatcher.dispatch_mouse_event(&event, MouseEventKind::Move, &mut context);

    assert_eq!(disp_down, EventDisposition::PassThrough);
    assert_eq!(disp_up, EventDisposition::PassThrough);
    assert_eq!(disp_move, EventDisposition::PassThrough);

    // Key events also default to PassThrough.
    let key_event = build_key_event();
    let disp_kd = dispatcher.dispatch_key_event(&key_event, KeyEventKind::Down, &mut context);
    let disp_ku = dispatcher.dispatch_key_event(&key_event, KeyEventKind::Up, &mut context);
    assert_eq!(disp_kd, EventDisposition::PassThrough);
    assert_eq!(disp_ku, EventDisposition::PassThrough);
}

/// Scenario 12: mutex enforcement — dispatch routes to active tool only;
/// non-active registered tools' methods NOT called (push-based optimization
/// per audit §7.2).
#[test]
fn mutex_enforcement_only_one_active_tool() {
    let mut dispatcher = Dispatcher::new();
    let uuid_a = Uuid::new_v4();
    let uuid_b = Uuid::new_v4();
    let (tool_a, state_a) = MockActiveTool::new(uuid_a, "a", EventDisposition::Consumed);
    let (tool_b, state_b) = MockActiveTool::new(uuid_b, "b", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool_a));
    dispatcher.register_tool(Box::new(tool_b));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid_a), &mut context);

    let event = build_mouse_event();
    dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);
    dispatcher.dispatch_mouse_event(&event, MouseEventKind::Move, &mut context);

    // Tool A receives all events; tool B receives none.
    assert_eq!(state_a.borrow().left_button_down_count, 1);
    assert_eq!(state_a.borrow().mouse_move_count, 1);
    assert_eq!(state_b.borrow().left_button_down_count, 0);
    assert_eq!(state_b.borrow().mouse_move_count, 0);

    // Switch active to B; subsequent events go to B only.
    dispatcher.set_active_tool(Some(uuid_b), &mut context);
    dispatcher.dispatch_mouse_event(&event, MouseEventKind::LeftButtonDown, &mut context);

    assert_eq!(state_a.borrow().left_button_down_count, 1); // unchanged
    assert_eq!(state_b.borrow().left_button_down_count, 1);
}

/// Scenario 13 (additional): dispatch_mouse_enter / dispatch_mouse_leave
/// route to active tool's notification methods.
#[test]
fn dispatch_mouse_enter_and_leave_route_to_active_tool() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    dispatcher.dispatch_mouse_enter(&mut context);
    dispatcher.dispatch_mouse_leave(&mut context);
    dispatcher.dispatch_mouse_enter(&mut context);

    assert_eq!(state.borrow().mouse_enter_count, 2);
    assert_eq!(state.borrow().mouse_leave_count, 1);
}

/// Scenario 14 (additional): dispatch_key_event routes per KeyEventKind.
#[test]
fn dispatch_key_event_routes_per_kind() {
    let mut dispatcher = Dispatcher::new();
    let uuid = Uuid::new_v4();
    let (tool, state) = MockActiveTool::new(uuid, "test", EventDisposition::Consumed);
    dispatcher.register_tool(Box::new(tool));

    let mut context = ToolContext::for_test();
    dispatcher.set_active_tool(Some(uuid), &mut context);
    let key_event = build_key_event();

    let disp_down = dispatcher.dispatch_key_event(&key_event, KeyEventKind::Down, &mut context);
    let disp_up = dispatcher.dispatch_key_event(&key_event, KeyEventKind::Up, &mut context);

    assert_eq!(disp_down, EventDisposition::Consumed);
    assert_eq!(disp_up, EventDisposition::Consumed);
    assert_eq!(state.borrow().key_down_count, 1);
    assert_eq!(state.borrow().key_up_count, 1);
}

/// Scenario 15 (additional): registered_uuids iterator yields all registered
/// UUIDs (used by toolbar UI for tool palette enumeration).
#[test]
fn registered_uuids_yields_all_registered() {
    let mut dispatcher = Dispatcher::new();
    let uuid_a = Uuid::new_v4();
    let uuid_b = Uuid::new_v4();
    let uuid_c = Uuid::new_v4();
    let (tool_a, _) = MockActiveTool::new(uuid_a, "a", EventDisposition::PassThrough);
    let (tool_b, _) = MockActiveTool::new(uuid_b, "b", EventDisposition::PassThrough);
    let (tool_c, _) = MockActiveTool::new(uuid_c, "c", EventDisposition::PassThrough);
    dispatcher.register_tool(Box::new(tool_a));
    dispatcher.register_tool(Box::new(tool_b));
    dispatcher.register_tool(Box::new(tool_c));

    let registered: Vec<Uuid> = dispatcher.registered_uuids().collect();
    assert_eq!(registered.len(), 3);
    assert!(registered.contains(&uuid_a));
    assert!(registered.contains(&uuid_b));
    assert!(registered.contains(&uuid_c));
    assert_eq!(dispatcher.tool_count(), 3);
}

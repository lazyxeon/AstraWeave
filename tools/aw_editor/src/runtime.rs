//! Editor simulation runtime
//!
//! Provides deterministic play/pause/stop functionality with snapshot-based state management.

use anyhow::Result;
use astraweave_core::ecs_adapter::build_app;
use astraweave_core::World;
use astraweave_ecs::App as SimulationApp;
use astraweave_profiling::{frame_mark, plot, span};
use std::time::Instant;

use crate::scene_serialization::SceneData;

/// Runtime execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeState {
    /// Not running (edit mode)
    Editing,
    /// Running continuously
    Playing,
    /// Paused at current tick
    Paused,
    /// Advance one tick then pause
    SteppingOneFrame,
}

impl RuntimeState {
    /// Get all possible runtime states
    pub fn all() -> &'static [RuntimeState] {
        &[
            RuntimeState::Editing,
            RuntimeState::Playing,
            RuntimeState::Paused,
            RuntimeState::SteppingOneFrame,
        ]
    }

    /// Check if this state can transition to another state
    pub fn can_transition_to(&self, target: RuntimeState) -> bool {
        match (self, target) {
            // From Editing: can only start playing
            (RuntimeState::Editing, RuntimeState::Playing) => true,
            (RuntimeState::Editing, _) => false,

            // From Playing: can pause, step, or stop (go back to editing)
            (RuntimeState::Playing, RuntimeState::Paused) => true,
            (RuntimeState::Playing, RuntimeState::Editing) => true,
            (RuntimeState::Playing, RuntimeState::SteppingOneFrame) => true,
            (RuntimeState::Playing, RuntimeState::Playing) => false,

            // From Paused: can resume, step, or stop
            (RuntimeState::Paused, RuntimeState::Playing) => true,
            (RuntimeState::Paused, RuntimeState::Editing) => true,
            (RuntimeState::Paused, RuntimeState::SteppingOneFrame) => true,
            (RuntimeState::Paused, RuntimeState::Paused) => false,

            // From SteppingOneFrame: transitions to Paused automatically, but can also stop
            (RuntimeState::SteppingOneFrame, RuntimeState::Paused) => true,
            (RuntimeState::SteppingOneFrame, RuntimeState::Editing) => true,
            (RuntimeState::SteppingOneFrame, RuntimeState::Playing) => true,
            (RuntimeState::SteppingOneFrame, RuntimeState::SteppingOneFrame) => true,
        }
    }

    /// Get valid states we can transition to from this state
    pub fn valid_transitions(&self) -> Vec<RuntimeState> {
        RuntimeState::all()
            .iter()
            .copied()
            .filter(|&target| self.can_transition_to(target))
            .collect()
    }

    /// Get icon representation for this state
    pub fn icon(&self) -> &'static str {
        match self {
            RuntimeState::Editing => "✏️",
            RuntimeState::Playing => ">",
            RuntimeState::Paused => "||",
            RuntimeState::SteppingOneFrame => ">|",
        }
    }

    /// Get keyboard shortcut hint for this state action
    pub fn shortcut_hint(&self) -> &'static str {
        match self {
            RuntimeState::Editing => "Ctrl+P to Play",
            RuntimeState::Playing => "Ctrl+P to Pause, Esc to Stop",
            RuntimeState::Paused => "Ctrl+P to Resume, F10 to Step",
            RuntimeState::SteppingOneFrame => "F10 to Step Again",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RuntimeState::Editing => "Scene is editable, simulation stopped",
            RuntimeState::Playing => "Simulation running in real-time",
            RuntimeState::Paused => "Simulation paused, can step or resume",
            RuntimeState::SteppingOneFrame => "Advancing single frame",
        }
    }

    /// Check if simulation world exists in this state
    pub fn has_simulation(&self) -> bool {
        matches!(
            self,
            RuntimeState::Playing | RuntimeState::Paused | RuntimeState::SteppingOneFrame
        )
    }

    /// Check if world can be edited in this state
    pub fn is_editable(&self) -> bool {
        matches!(self, RuntimeState::Editing)
    }

    /// Check if simulation is actively running
    pub fn is_active(&self) -> bool {
        matches!(self, RuntimeState::Playing | RuntimeState::SteppingOneFrame)
    }
}

impl std::fmt::Display for RuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeState::Editing => write!(f, "Editing"),
            RuntimeState::Playing => write!(f, "Playing"),
            RuntimeState::Paused => write!(f, "Paused"),
            RuntimeState::SteppingOneFrame => write!(f, "Stepping"),
        }
    }
}

/// Issues detected in runtime state
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuntimeIssue {
    /// No simulation world when one is expected
    MissingSimulation,
    /// Missing edit snapshot for restoration
    MissingEditSnapshot,
    /// Simulation world corrupted or inconsistent
    CorruptedSimulation { reason: String },
    /// Frame time too long (potential freeze)
    FrameTimeExceeded {
        frame_time_ms: u32,
        threshold_ms: u32,
    },
    /// Too few FPS (performance issue)
    LowFps { fps: u32, minimum_fps: u32 },
    /// Entity count mismatch after restoration
    EntityCountMismatch { expected: usize, actual: usize },
}

impl RuntimeIssue {
    /// Get all issue variant types (for testing/documentation)
    pub fn all_variants() -> Vec<RuntimeIssue> {
        vec![
            RuntimeIssue::MissingSimulation,
            RuntimeIssue::MissingEditSnapshot,
            RuntimeIssue::CorruptedSimulation {
                reason: "example".to_string(),
            },
            RuntimeIssue::FrameTimeExceeded {
                frame_time_ms: 50,
                threshold_ms: 33,
            },
            RuntimeIssue::LowFps {
                fps: 15,
                minimum_fps: 30,
            },
            RuntimeIssue::EntityCountMismatch {
                expected: 100,
                actual: 95,
            },
        ]
    }

    /// Check if this is a critical issue requiring immediate action
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            RuntimeIssue::MissingSimulation | RuntimeIssue::CorruptedSimulation { .. }
        )
    }

    /// Check if this is a performance-related issue
    pub fn is_performance_issue(&self) -> bool {
        matches!(
            self,
            RuntimeIssue::FrameTimeExceeded { .. } | RuntimeIssue::LowFps { .. }
        )
    }

    /// Check if this is a data integrity issue
    pub fn is_data_issue(&self) -> bool {
        matches!(
            self,
            RuntimeIssue::EntityCountMismatch { .. }
                | RuntimeIssue::CorruptedSimulation { .. }
                | RuntimeIssue::MissingEditSnapshot
        )
    }

    /// Check if this issue is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            RuntimeIssue::FrameTimeExceeded { .. } | RuntimeIssue::LowFps { .. } => true,
            RuntimeIssue::EntityCountMismatch { .. } => true,
            RuntimeIssue::MissingSimulation
            | RuntimeIssue::MissingEditSnapshot
            | RuntimeIssue::CorruptedSimulation { .. } => false,
        }
    }

    /// Get severity level (1=lowest, 5=highest)
    pub fn severity(&self) -> u8 {
        match self {
            RuntimeIssue::MissingSimulation | RuntimeIssue::CorruptedSimulation { .. } => 5,
            RuntimeIssue::MissingEditSnapshot => 4,
            RuntimeIssue::EntityCountMismatch { .. } => 3,
            RuntimeIssue::FrameTimeExceeded { .. } => 2,
            RuntimeIssue::LowFps { .. } => 1,
        }
    }

    /// Get user-facing title for this issue
    pub fn title(&self) -> &'static str {
        match self {
            RuntimeIssue::MissingSimulation => "Missing Simulation",
            RuntimeIssue::MissingEditSnapshot => "Missing Edit Snapshot",
            RuntimeIssue::CorruptedSimulation { .. } => "Corrupted Simulation",
            RuntimeIssue::FrameTimeExceeded { .. } => "Frame Time Exceeded",
            RuntimeIssue::LowFps { .. } => "Low FPS",
            RuntimeIssue::EntityCountMismatch { .. } => "Entity Count Mismatch",
        }
    }

    /// Get icon for this issue
    pub fn icon(&self) -> &'static str {
        match self.severity() {
            5 => "[R]", // Critical
            4 => "🟠",  // High
            3 => "[Y]", // Medium
            2 => "[G]", // Low
            _ => "ℹ️",  // Info
        }
    }
}

impl std::fmt::Display for RuntimeIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeIssue::MissingSimulation => {
                write!(f, "No simulation world when one is expected")
            }
            RuntimeIssue::MissingEditSnapshot => {
                write!(f, "Missing edit snapshot for restoration")
            }
            RuntimeIssue::CorruptedSimulation { reason } => {
                write!(f, "Simulation world corrupted: {}", reason)
            }
            RuntimeIssue::FrameTimeExceeded {
                frame_time_ms,
                threshold_ms,
            } => {
                write!(
                    f,
                    "Frame time {}ms exceeds threshold {}ms",
                    frame_time_ms, threshold_ms
                )
            }
            RuntimeIssue::LowFps { fps, minimum_fps } => {
                write!(f, "FPS {} below minimum {} FPS", fps, minimum_fps)
            }
            RuntimeIssue::EntityCountMismatch { expected, actual } => {
                write!(
                    f,
                    "Entity count mismatch: expected {}, got {}",
                    expected, actual
                )
            }
        }
    }
}

/// Runtime performance statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
    /// Entities in simulation
    pub entity_count: usize,
    /// Current tick number
    pub tick_count: u64,
    /// Frames per second
    pub fps: f32,
    /// Fixed 60 Hz steps executed during the last tick (for diagnostics)
    pub fixed_steps_last_tick: u32,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            frame_time_ms: 0.0,
            entity_count: 0,
            tick_count: 0,
            fps: 0.0,
            fixed_steps_last_tick: 0,
        }
    }
}

impl RuntimeStats {
    /// Check if frame time is within acceptable bounds
    pub fn is_frame_time_healthy(&self, max_ms: f32) -> bool {
        self.frame_time_ms <= max_ms
    }

    /// Check if FPS is within acceptable bounds
    pub fn is_fps_healthy(&self, min_fps: f32) -> bool {
        self.fps >= min_fps
    }

    /// Get performance grade based on FPS
    pub fn performance_grade(&self) -> &'static str {
        match self.fps as u32 {
            0..=14 => "Critical",
            15..=29 => "Poor",
            30..=44 => "Fair",
            45..=59 => "Good",
            _ => "Excellent",
        }
    }

    /// Get frame time budget usage as percentage (assuming 60 FPS target = 16.67ms)
    pub fn frame_budget_percentage(&self) -> f32 {
        (self.frame_time_ms / 16.667) * 100.0
    }

    /// Get estimated maximum entities based on current performance
    pub fn estimated_entity_capacity(&self) -> usize {
        if self.entity_count == 0 || self.frame_time_ms <= 0.0 {
            return 0;
        }
        // Calculate how many entities we could have while staying under 16.67ms
        let entities_per_ms = self.entity_count as f32 / self.frame_time_ms;
        (entities_per_ms * 16.667) as usize
    }

    /// Check if running smoothly (60+ FPS, <16.67ms frame time)
    pub fn is_running_smoothly(&self) -> bool {
        self.fps >= 60.0 && self.frame_time_ms <= 16.67
    }

    /// Get simulation duration in seconds (assuming 60 Hz fixed timestep)
    pub fn simulation_duration_secs(&self) -> f32 {
        self.tick_count as f32 / 60.0
    }

    /// Validate stats for potential issues
    pub fn validate(&self) -> Vec<RuntimeIssue> {
        let mut issues = Vec::new();

        if self.frame_time_ms > 33.33 {
            issues.push(RuntimeIssue::FrameTimeExceeded {
                frame_time_ms: self.frame_time_ms as u32,
                threshold_ms: 33,
            });
        }

        if self.fps > 0.0 && self.fps < 30.0 {
            issues.push(RuntimeIssue::LowFps {
                fps: self.fps as u32,
                minimum_fps: 30,
            });
        }

        issues
    }

    /// Check if stats are healthy (good FPS and frame time)
    pub fn is_healthy(&self) -> bool {
        self.is_running_smoothly()
    }

    /// Get performance status color (for UI)
    pub fn status_color(&self) -> &'static str {
        match self.performance_grade() {
            "Excellent" => "green",
            "Good" => "lightgreen",
            "Fair" => "yellow",
            "Poor" => "orange",
            "Critical" => "red",
            _ => "gray",
        }
    }

    /// Get FPS stability indicator (how close to target)
    pub fn fps_stability(&self) -> f32 {
        if self.fps <= 0.0 {
            return 0.0;
        }
        (self.fps / 60.0).min(1.0)
    }

    /// Get frame time as percentage of 60 FPS budget
    pub fn frame_time_percentage_of_budget(&self) -> f32 {
        (self.frame_time_ms / 16.667) * 100.0
    }

    /// Check if frame time is critical (>33ms = <30 FPS)
    pub fn is_frame_time_critical(&self) -> bool {
        self.frame_time_ms > 33.33
    }

    /// Get estimated headroom in milliseconds before dropping below 60 FPS
    pub fn frame_time_headroom(&self) -> f32 {
        16.667 - self.frame_time_ms
    }
}

/// Captured game input state for a single frame during play mode.
///
/// This struct is populated by the viewport widget from egui input and injected
/// into the runtime so the simulation can read player intent.
#[derive(Debug, Clone, Default)]
pub struct GameInput {
    /// Movement axes: W/S for forward/back, A/D for left/right (−1.0..1.0)
    pub move_x: f32,
    pub move_y: f32,

    /// Mouse position in viewport-local coordinates (pixels)
    pub mouse_pos: [f32; 2],

    /// Mouse buttons held this frame
    pub mouse_left: bool,
    pub mouse_right: bool,

    /// Action keys (jump, interact, ability slots)
    pub jump: bool,
    pub interact: bool,
    pub ability_1: bool,
    pub ability_2: bool,
    pub ability_3: bool,
}

/// Editor simulation runtime
///
/// Manages the play/pause/stop lifecycle with deterministic snapshots.
pub struct EditorRuntime {
    /// Snapshot captured when entering play mode
    edit_snapshot: Option<SceneData>,

    /// Active simulation app (drives ECS/physics/audio once play mode starts)
    sim_app: Option<SimulationApp>,

    /// Current tick number (deterministic frame counter)
    tick_count: u64,

    /// Runtime state
    state: RuntimeState,

    /// Performance statistics
    stats: RuntimeStats,

    /// Frame time tracking (last 60 frames)
    frame_times: Vec<f32>,

    /// Last frame timestamp
    last_frame_time: Option<Instant>,

    /// Fixed 60 Hz timestep (seconds) used by the runtime
    fixed_dt: f32,

    /// Accumulated time waiting to be consumed by fixed steps
    time_accumulator: f32,

    /// Input captured from the viewport during play mode
    game_input: GameInput,
}

impl Default for EditorRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorRuntime {
    /// Create new runtime in editing state
    pub fn new() -> Self {
        Self {
            edit_snapshot: None,
            sim_app: None,
            tick_count: 0,
            state: RuntimeState::Editing,
            stats: RuntimeStats::default(),
            frame_times: Vec::with_capacity(60),
            last_frame_time: None,
            fixed_dt: 1.0 / 60.0,
            time_accumulator: 0.0,
            game_input: GameInput::default(),
        }
    }

    /// Get current runtime state
    pub fn state(&self) -> RuntimeState {
        self.state
    }

    /// Get current statistics
    pub fn stats(&self) -> &RuntimeStats {
        &self.stats
    }

    /// Check if simulation is running
    pub fn is_playing(&self) -> bool {
        matches!(
            self.state,
            RuntimeState::Playing | RuntimeState::SteppingOneFrame
        )
    }

    /// Check if simulation is paused
    pub fn is_paused(&self) -> bool {
        self.state == RuntimeState::Paused
    }

    /// Inject player input for the current frame (called by viewport before tick)
    pub fn inject_input(&mut self, input: GameInput) {
        self.game_input = input;
    }

    /// Read the latest game input (available to simulation systems)
    pub fn game_input(&self) -> &GameInput {
        &self.game_input
    }

    /// Get simulation world (if running)
    pub fn sim_world(&self) -> Option<&World> {
        self.sim_app
            .as_ref()
            .and_then(|app| app.world.get_resource::<World>())
    }

    /// Get mutable simulation world (if running)
    pub fn sim_world_mut(&mut self) -> Option<&mut World> {
        self.sim_app
            .as_mut()
            .and_then(|app| app.world.get_resource_mut::<World>())
    }

    /// Capture current scene and enter play mode
    pub fn enter_play(&mut self, world: &World) -> Result<()> {
        if self.state != RuntimeState::Editing {
            return Ok(()); // Already playing
        }

        // Capture snapshot of edit state
        let snapshot = SceneData::from_world(world);

        // Clone world for simulation via snapshot and boot the ECS app
        let sim_world = snapshot.to_world();
        let sim_app = build_app(sim_world, self.fixed_dt);

        self.edit_snapshot = Some(snapshot);
        self.sim_app = Some(sim_app);
        self.tick_count = 0;
        self.state = RuntimeState::Playing;
        self.last_frame_time = Some(Instant::now());
        self.time_accumulator = 0.0;
        self.stats = RuntimeStats::default();
        self.frame_times.clear();

        if let Some(world) = self.sim_world() {
            self.stats.entity_count = world.entities().len();
        }

        Ok(())
    }

    /// Pause execution, preserving current state
    pub fn pause(&mut self) {
        if self.state == RuntimeState::Playing {
            self.state = RuntimeState::Paused;
        }
    }

    /// Resume from paused state
    pub fn resume(&mut self) {
        if self.state == RuntimeState::Paused {
            self.state = RuntimeState::Playing;
            self.last_frame_time = Some(Instant::now());
        }
    }

    /// Advance exactly one frame then pause
    pub fn step_frame(&mut self) -> Result<()> {
        if self.sim_app.is_none() {
            return Ok(()); // Not in play mode
        }

        // Set state to stepping
        let was_paused = self.state == RuntimeState::Paused;
        self.state = RuntimeState::SteppingOneFrame;
        self.time_accumulator = 0.0;
        let start = Instant::now();

        // Tick once at fixed 60Hz (16.67ms)
        self.run_fixed_steps(1)?;
        self.finish_tick(start, 1);

        // Return to paused state
        if was_paused || self.state == RuntimeState::SteppingOneFrame {
            self.state = RuntimeState::Paused;
        }

        Ok(())
    }

    /// Advance simulation using variable `dt`, internally respecting the fixed 60 Hz step.
    pub fn tick(&mut self, dt: f32) -> Result<()> {
        if !self.is_playing() {
            return Ok(());
        }

        span!("editor_runtime.tick");
        let start = Instant::now();

        let clamped_dt = if dt.is_finite() {
            dt.clamp(0.0, self.fixed_dt * 5.0)
        } else {
            self.fixed_dt
        };
        self.time_accumulator = (self.time_accumulator + clamped_dt).min(self.fixed_dt * 5.0);

        let mut steps_to_run = 0u32;
        while self.time_accumulator + f32::EPSILON >= self.fixed_dt {
            self.time_accumulator -= self.fixed_dt;
            steps_to_run += 1;
        }

        self.run_fixed_steps(steps_to_run)?;
        self.finish_tick(start, steps_to_run);

        if self.state == RuntimeState::SteppingOneFrame {
            self.state = RuntimeState::Paused;
        }

        Ok(())
    }

    /// Exit play mode and restore edit snapshot
    pub fn exit_play(&mut self) -> Result<Option<World>> {
        if self.state == RuntimeState::Editing {
            return Ok(None); // Already in edit mode
        }

        // Restore edit snapshot
        let restored_world = self
            .edit_snapshot
            .as_ref()
            .map(|snapshot| snapshot.to_world());

        // Reset runtime state
        self.sim_app = None;
        self.edit_snapshot = None;
        self.tick_count = 0;
        self.state = RuntimeState::Editing;
        self.stats = RuntimeStats::default();
        self.frame_times.clear();
        self.last_frame_time = None;
        self.time_accumulator = 0.0;

        Ok(restored_world)
    }

    fn run_fixed_steps(&mut self, steps: u32) -> Result<()> {
        if steps == 0 {
            return Ok(());
        }

        if let Some(app) = self.sim_app.take() {
            let mut app_slot = Some(app);
            for _ in 0..steps {
                span!("editor_runtime.fixed_step");
                let current_app = app_slot.take().expect("app consumed unexpectedly");
                let mut moved_app = Some(current_app);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    moved_app.take().unwrap().run_fixed(1)
                }));
                match result {
                    Ok(new_app) => {
                        app_slot = Some(new_app);
                        self.tick_count += 1;
                    }
                    Err(panic_info) => {
                        let msg = panic_info.downcast_ref::<&str>().copied()
                            .or_else(|| panic_info.downcast_ref::<String>().map(|s| s.as_str()))
                            .unwrap_or("(unknown panic payload)");
                        tracing::error!(
                            "Simulation panicked at tick {}: {msg}. Auto-pausing play mode.",
                            self.tick_count,
                        );
                        // App is lost (consumed by panicked closure). Clear sim state.
                        self.state = RuntimeState::Paused;
                        return Err(anyhow::anyhow!("Simulation panic at tick {}", self.tick_count));
                    }
                }
            }
            self.sim_app = app_slot;
        }

        Ok(())
    }

    fn finish_tick(&mut self, start: Instant, steps: u32) {
        self.stats.fixed_steps_last_tick = steps;
        let frame_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        self.update_frame_time(frame_time_ms);

        if let Some(world) = self.sim_world() {
            self.stats.entity_count = world.entities().len();
        } else {
            self.stats.entity_count = 0;
        }
        self.stats.tick_count = self.tick_count;

        plot!("EditorRuntime::frame_ms", self.stats.frame_time_ms as f64);
        plot!("EditorRuntime::entities", self.stats.entity_count as f64);
        frame_mark!();
    }

    /// Update frame time statistics
    fn update_frame_time(&mut self, frame_time_ms: f32) {
        self.frame_times.push(frame_time_ms);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        // Calculate average frame time and FPS
        if !self.frame_times.is_empty() {
            let avg_frame_time: f32 =
                self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            self.stats.frame_time_ms = avg_frame_time;
            self.stats.fps = if avg_frame_time > 0.0 {
                1000.0 / avg_frame_time
            } else {
                0.0
            };
        }
    }

    // === New Validation and Query Methods ===

    /// Validate runtime state for issues
    pub fn validate(&self) -> Vec<RuntimeIssue> {
        let mut issues = self.stats.validate();

        // Check for missing simulation when expected
        if self.state.has_simulation() && self.sim_app.is_none() {
            issues.push(RuntimeIssue::MissingSimulation);
        }

        // Check for missing edit snapshot when in play mode
        if self.state != RuntimeState::Editing && self.edit_snapshot.is_none() {
            issues.push(RuntimeIssue::MissingEditSnapshot);
        }

        issues
    }

    /// Check if runtime state is valid
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Check if we can transition to a target state
    pub fn can_transition_to(&self, target: RuntimeState) -> bool {
        self.state.can_transition_to(target)
    }

    /// Get tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get current frame times for analysis
    pub fn frame_times(&self) -> &[f32] {
        &self.frame_times
    }

    /// Get frame time variance (for jitter detection)
    pub fn frame_time_variance(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let mean = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let variance: f32 = self
            .frame_times
            .iter()
            .map(|&t| (t - mean).powi(2))
            .sum::<f32>()
            / self.frame_times.len() as f32;
        variance.sqrt()
    }

    /// Check if frame times are stable (low jitter)
    pub fn is_frame_time_stable(&self, max_variance_ms: f32) -> bool {
        self.frame_time_variance() <= max_variance_ms
    }

    /// Get minimum frame time in history
    pub fn min_frame_time(&self) -> Option<f32> {
        self.frame_times.iter().copied().reduce(f32::min)
    }

    /// Get maximum frame time in history
    pub fn max_frame_time(&self) -> Option<f32> {
        self.frame_times.iter().copied().reduce(f32::max)
    }

    /// Check if edit snapshot exists
    pub fn has_edit_snapshot(&self) -> bool {
        self.edit_snapshot.is_some()
    }

    /// Get fixed timestep (60 Hz)
    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{IVec2, Team};

    #[test]
    fn runtime_starts_in_editing_state() {
        let runtime = EditorRuntime::new();
        assert_eq!(runtime.state(), RuntimeState::Editing);
        assert!(!runtime.is_playing());
        assert!(runtime.sim_world().is_none());
    }

    #[test]
    fn enter_play_captures_snapshot_and_creates_sim_world() {
        let mut runtime = EditorRuntime::new();
        let mut world = World::new();
        world.spawn("test", IVec2 { x: 5, y: 10 }, Team { id: 0 }, 100, 10);

        runtime.enter_play(&world).expect("enter play");

        assert_eq!(runtime.state(), RuntimeState::Playing);
        assert!(runtime.is_playing());
        assert!(runtime.sim_world().is_some());
        assert!(runtime.edit_snapshot.is_some());
        assert_eq!(runtime.tick_count, 0);
    }

    #[test]
    fn tick_advances_simulation_count() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");

        runtime.tick(1.0 / 60.0).expect("tick 1");
        assert_eq!(runtime.tick_count, 1);

        runtime.tick(1.0 / 60.0).expect("tick 2");
        assert_eq!(runtime.tick_count, 2);
    }

    #[test]
    fn pause_stops_ticking() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");

        runtime.tick(1.0 / 60.0).expect("tick 1");
        let tick_before_pause = runtime.tick_count;

        runtime.pause();
        assert_eq!(runtime.state(), RuntimeState::Paused);
        assert!(runtime.is_paused());

        // Ticking while paused does nothing
        runtime.tick(1.0 / 60.0).expect("tick while paused");
        assert_eq!(runtime.tick_count, tick_before_pause);
    }

    #[test]
    fn resume_continues_from_paused() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");
        runtime.tick(1.0 / 60.0).expect("tick 1");
        runtime.pause();

        runtime.resume();
        assert_eq!(runtime.state(), RuntimeState::Playing);
        assert!(runtime.is_playing());

        runtime.tick(1.0 / 60.0).expect("tick after resume");
        assert_eq!(runtime.tick_count, 2);
    }

    #[test]
    fn step_frame_advances_one_tick_then_pauses() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");
        runtime.pause();

        runtime.step_frame().expect("step 1");
        assert_eq!(runtime.tick_count, 1);
        assert_eq!(runtime.state(), RuntimeState::Paused);

        runtime.step_frame().expect("step 2");
        assert_eq!(runtime.tick_count, 2);
        assert_eq!(runtime.state(), RuntimeState::Paused);
    }

    #[test]
    fn exit_play_restores_edit_snapshot() {
        let mut runtime = EditorRuntime::new();
        let mut world = World::new();
        let entity = world.spawn("test", IVec2 { x: 5, y: 10 }, Team { id: 0 }, 100, 10);

        runtime.enter_play(&world).expect("enter play");

        // Simulate modifications during play
        if let Some(sim) = runtime.sim_world_mut() {
            sim.spawn(
                "runtime_entity",
                IVec2 { x: 20, y: 30 },
                Team { id: 1 },
                50,
                5,
            );
        }

        let restored = runtime.exit_play().expect("exit play");
        assert_eq!(runtime.state(), RuntimeState::Editing);
        assert!(runtime.sim_world().is_none());
        assert_eq!(runtime.tick_count, 0);

        // Verify restored world matches original (not runtime modifications)
        if let Some(restored_world) = restored {
            assert_eq!(restored_world.entities().len(), 1); // Only original entity
            assert!(restored_world.pose(entity).is_some());
        }
    }

    #[test]
    fn stats_track_entity_count_and_tick_count() {
        let mut runtime = EditorRuntime::new();
        let mut world = World::new();
        world.spawn("e1", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 10);
        world.spawn("e2", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 10);

        runtime.enter_play(&world).expect("enter play");
        runtime.tick(1.0 / 60.0).expect("tick");

        let stats = runtime.stats();
        assert_eq!(stats.tick_count, 1);
        assert_eq!(stats.entity_count, 2); // Should match spawned entities
    }

    #[test]
    fn tick_accumulates_until_full_step() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");

        let half_dt = (1.0 / 60.0) * 0.5;
        runtime.tick(half_dt).expect("fractional tick");
        assert_eq!(runtime.stats().tick_count, 0);
        assert_eq!(runtime.stats().fixed_steps_last_tick, 0);

        runtime.tick(half_dt).expect("second fractional tick");
        assert_eq!(runtime.stats().tick_count, 1);
        assert_eq!(runtime.stats().fixed_steps_last_tick, 1);
    }

    #[test]
    fn step_frame_executes_single_fixed_step() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");
        runtime.pause();

        runtime.step_frame().expect("step frame");

        assert_eq!(runtime.stats().tick_count, 1);
        assert_eq!(runtime.stats().fixed_steps_last_tick, 1);
        assert!(runtime.is_paused());
    }

    // === New tests for enhanced RuntimeState ===

    #[test]
    fn runtime_state_all_returns_all_states() {
        let all = RuntimeState::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&RuntimeState::Editing));
        assert!(all.contains(&RuntimeState::Playing));
        assert!(all.contains(&RuntimeState::Paused));
        assert!(all.contains(&RuntimeState::SteppingOneFrame));
    }

    #[test]
    fn runtime_state_transitions_from_editing() {
        let state = RuntimeState::Editing;
        assert!(state.can_transition_to(RuntimeState::Playing));
        assert!(!state.can_transition_to(RuntimeState::Paused));
        assert!(!state.can_transition_to(RuntimeState::Editing));
        assert!(!state.can_transition_to(RuntimeState::SteppingOneFrame));
    }

    #[test]
    fn runtime_state_transitions_from_playing() {
        let state = RuntimeState::Playing;
        assert!(state.can_transition_to(RuntimeState::Paused));
        assert!(state.can_transition_to(RuntimeState::Editing));
        assert!(state.can_transition_to(RuntimeState::SteppingOneFrame));
        assert!(!state.can_transition_to(RuntimeState::Playing));
    }

    #[test]
    fn runtime_state_transitions_from_paused() {
        let state = RuntimeState::Paused;
        assert!(state.can_transition_to(RuntimeState::Playing));
        assert!(state.can_transition_to(RuntimeState::Editing));
        assert!(state.can_transition_to(RuntimeState::SteppingOneFrame));
        assert!(!state.can_transition_to(RuntimeState::Paused));
    }

    #[test]
    fn runtime_state_valid_transitions() {
        let editing = RuntimeState::Editing;
        let valid = editing.valid_transitions();
        assert_eq!(valid.len(), 1);
        assert!(valid.contains(&RuntimeState::Playing));

        let playing = RuntimeState::Playing;
        let valid = playing.valid_transitions();
        assert_eq!(valid.len(), 3);
    }

    #[test]
    fn runtime_state_icons_not_empty() {
        for state in RuntimeState::all() {
            assert!(!state.icon().is_empty());
        }
    }

    #[test]
    fn runtime_state_shortcut_hints_not_empty() {
        for state in RuntimeState::all() {
            assert!(!state.shortcut_hint().is_empty());
        }
    }

    #[test]
    fn runtime_state_descriptions_not_empty() {
        for state in RuntimeState::all() {
            assert!(!state.description().is_empty());
        }
    }

    #[test]
    fn runtime_state_has_simulation() {
        assert!(!RuntimeState::Editing.has_simulation());
        assert!(RuntimeState::Playing.has_simulation());
        assert!(RuntimeState::Paused.has_simulation());
        assert!(RuntimeState::SteppingOneFrame.has_simulation());
    }

    #[test]
    fn runtime_state_is_editable() {
        assert!(RuntimeState::Editing.is_editable());
        assert!(!RuntimeState::Playing.is_editable());
        assert!(!RuntimeState::Paused.is_editable());
        assert!(!RuntimeState::SteppingOneFrame.is_editable());
    }

    #[test]
    fn runtime_state_is_active() {
        assert!(!RuntimeState::Editing.is_active());
        assert!(RuntimeState::Playing.is_active());
        assert!(!RuntimeState::Paused.is_active());
        assert!(RuntimeState::SteppingOneFrame.is_active());
    }

    #[test]
    fn runtime_state_display() {
        assert_eq!(RuntimeState::Editing.to_string(), "Editing");
        assert_eq!(RuntimeState::Playing.to_string(), "Playing");
        assert_eq!(RuntimeState::Paused.to_string(), "Paused");
        assert_eq!(RuntimeState::SteppingOneFrame.to_string(), "Stepping");
    }

    // === RuntimeStats tests ===

    #[test]
    fn runtime_stats_is_frame_time_healthy() {
        let stats = RuntimeStats {
            frame_time_ms: 10.0,
            ..Default::default()
        };
        assert!(stats.is_frame_time_healthy(16.67));
        assert!(!stats.is_frame_time_healthy(5.0));
    }

    #[test]
    fn runtime_stats_is_fps_healthy() {
        let stats = RuntimeStats {
            fps: 60.0,
            ..Default::default()
        };
        assert!(stats.is_fps_healthy(30.0));
        assert!(!stats.is_fps_healthy(90.0));
    }

    #[test]
    fn runtime_stats_performance_grade() {
        let stats = RuntimeStats {
            fps: 10.0,
            ..Default::default()
        };
        assert_eq!(stats.performance_grade(), "Critical");

        let stats = RuntimeStats {
            fps: 25.0,
            ..Default::default()
        };
        assert_eq!(stats.performance_grade(), "Poor");

        let stats = RuntimeStats {
            fps: 40.0,
            ..Default::default()
        };
        assert_eq!(stats.performance_grade(), "Fair");

        let stats = RuntimeStats {
            fps: 55.0,
            ..Default::default()
        };
        assert_eq!(stats.performance_grade(), "Good");

        let stats = RuntimeStats {
            fps: 120.0,
            ..Default::default()
        };
        assert_eq!(stats.performance_grade(), "Excellent");
    }

    #[test]
    fn runtime_stats_frame_budget_percentage() {
        let stats = RuntimeStats {
            frame_time_ms: 16.667,
            ..Default::default()
        };
        assert!((stats.frame_budget_percentage() - 100.0).abs() < 1.0);

        let stats = RuntimeStats {
            frame_time_ms: 8.33,
            ..Default::default()
        };
        assert!((stats.frame_budget_percentage() - 50.0).abs() < 1.0);
    }

    #[test]
    fn runtime_stats_is_running_smoothly() {
        let stats = RuntimeStats {
            fps: 60.0,
            frame_time_ms: 16.0,
            ..Default::default()
        };
        assert!(stats.is_running_smoothly());

        let stats = RuntimeStats {
            fps: 30.0,
            ..Default::default()
        };
        assert!(!stats.is_running_smoothly());
    }

    #[test]
    fn runtime_stats_simulation_duration() {
        let stats = RuntimeStats {
            tick_count: 120,
            ..Default::default()
        };
        assert!((stats.simulation_duration_secs() - 2.0).abs() < 0.01);
    }

    #[test]
    fn runtime_stats_validate_detects_issues() {
        let stats = RuntimeStats {
            frame_time_ms: 50.0, // Too slow
            fps: 20.0,           // Too low
            ..Default::default()
        };
        let issues = stats.validate();
        assert_eq!(issues.len(), 2);
    }

    // === EditorRuntime validation tests ===

    #[test]
    fn editor_runtime_validate_in_editing_mode() {
        let runtime = EditorRuntime::new();
        let issues = runtime.validate();
        assert!(issues.is_empty());
        assert!(runtime.is_valid());
    }

    #[test]
    fn editor_runtime_can_transition_to() {
        let runtime = EditorRuntime::new();
        assert!(runtime.can_transition_to(RuntimeState::Playing));
        assert!(!runtime.can_transition_to(RuntimeState::Paused));
    }

    #[test]
    fn editor_runtime_tick_count() {
        let mut runtime = EditorRuntime::new();
        let world = World::new();
        runtime.enter_play(&world).expect("enter play");
        runtime.tick(1.0 / 60.0).expect("tick");
        assert_eq!(runtime.tick_count(), 1);
    }

    #[test]
    fn editor_runtime_has_edit_snapshot() {
        let mut runtime = EditorRuntime::new();
        assert!(!runtime.has_edit_snapshot());

        let world = World::new();
        runtime.enter_play(&world).expect("enter play");
        assert!(runtime.has_edit_snapshot());
    }

    #[test]
    fn editor_runtime_fixed_dt() {
        let runtime = EditorRuntime::new();
        assert!((runtime.fixed_dt() - 1.0 / 60.0).abs() < 0.0001);
    }

    #[test]
    fn editor_runtime_frame_time_variance_empty() {
        let runtime = EditorRuntime::new();
        assert_eq!(runtime.frame_time_variance(), 0.0);
    }

    // === RuntimeIssue Display trait tests ===

    #[test]
    fn runtime_issue_display_missing_simulation() {
        let issue = RuntimeIssue::MissingSimulation;
        let display = format!("{}", issue);
        assert!(display.contains("No simulation world"));
    }

    #[test]
    fn runtime_issue_display_missing_snapshot() {
        let issue = RuntimeIssue::MissingEditSnapshot;
        let display = format!("{}", issue);
        assert!(display.contains("Missing edit snapshot"));
    }

    #[test]
    fn runtime_issue_display_corrupted_simulation() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "entity data invalid".to_string(),
        };
        let display = format!("{}", issue);
        assert!(display.contains("corrupted"));
        assert!(display.contains("entity data invalid"));
    }

    #[test]
    fn runtime_issue_display_frame_time_exceeded() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        let display = format!("{}", issue);
        assert!(display.contains("50"));
        assert!(display.contains("33"));
        assert!(display.contains("ms"));
    }

    #[test]
    fn runtime_issue_display_low_fps() {
        let issue = RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30,
        };
        let display = format!("{}", issue);
        assert!(display.contains("15"));
        assert!(display.contains("30"));
        assert!(display.contains("FPS"));
    }

    #[test]
    fn runtime_issue_display_entity_count_mismatch() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 95,
        };
        let display = format!("{}", issue);
        assert!(display.contains("100"));
        assert!(display.contains("95"));
        assert!(display.contains("mismatch"));
    }

    // === RuntimeIssue helper methods tests ===

    #[test]
    fn runtime_issue_all_variants() {
        let variants = RuntimeIssue::all_variants();
        assert_eq!(variants.len(), 6);
        assert!(variants.contains(&RuntimeIssue::MissingSimulation));
        assert!(variants.contains(&RuntimeIssue::MissingEditSnapshot));
    }

    #[test]
    fn runtime_issue_is_critical() {
        assert!(RuntimeIssue::MissingSimulation.is_critical());
        assert!(RuntimeIssue::CorruptedSimulation {
            reason: "test".to_string()
        }
        .is_critical());
        assert!(!RuntimeIssue::MissingEditSnapshot.is_critical());
        assert!(!RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .is_critical());
        assert!(!RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30
        }
        .is_critical());
        assert!(!RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 95
        }
        .is_critical());
    }

    #[test]
    fn runtime_issue_is_performance_issue() {
        assert!(!RuntimeIssue::MissingSimulation.is_performance_issue());
        assert!(!RuntimeIssue::MissingEditSnapshot.is_performance_issue());
        assert!(RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .is_performance_issue());
        assert!(RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30
        }
        .is_performance_issue());
        assert!(!RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 95
        }
        .is_performance_issue());
    }

    #[test]
    fn runtime_issue_is_data_issue() {
        assert!(!RuntimeIssue::MissingSimulation.is_data_issue());
        assert!(RuntimeIssue::MissingEditSnapshot.is_data_issue());
        assert!(RuntimeIssue::CorruptedSimulation {
            reason: "test".to_string()
        }
        .is_data_issue());
        assert!(!RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .is_data_issue());
        assert!(RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 95
        }
        .is_data_issue());
    }

    #[test]
    fn runtime_issue_is_recoverable() {
        assert!(!RuntimeIssue::MissingSimulation.is_recoverable());
        assert!(!RuntimeIssue::MissingEditSnapshot.is_recoverable());
        assert!(!RuntimeIssue::CorruptedSimulation {
            reason: "test".to_string()
        }
        .is_recoverable());
        assert!(RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .is_recoverable());
        assert!(RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30
        }
        .is_recoverable());
        assert!(RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 95
        }
        .is_recoverable());
    }

    #[test]
    fn runtime_issue_severity() {
        assert_eq!(RuntimeIssue::MissingSimulation.severity(), 5);
        assert_eq!(
            RuntimeIssue::CorruptedSimulation {
                reason: "test".to_string()
            }
            .severity(),
            5
        );
        assert_eq!(RuntimeIssue::MissingEditSnapshot.severity(), 4);
        assert_eq!(
            RuntimeIssue::EntityCountMismatch {
                expected: 100,
                actual: 95
            }
            .severity(),
            3
        );
        assert_eq!(
            RuntimeIssue::FrameTimeExceeded {
                frame_time_ms: 50,
                threshold_ms: 33
            }
            .severity(),
            2
        );
        assert_eq!(
            RuntimeIssue::LowFps {
                fps: 15,
                minimum_fps: 30
            }
            .severity(),
            1
        );
    }

    #[test]
    fn runtime_issue_title() {
        assert_eq!(
            RuntimeIssue::MissingSimulation.title(),
            "Missing Simulation"
        );
        assert_eq!(
            RuntimeIssue::MissingEditSnapshot.title(),
            "Missing Edit Snapshot"
        );
        assert_eq!(
            RuntimeIssue::CorruptedSimulation {
                reason: "test".to_string()
            }
            .title(),
            "Corrupted Simulation"
        );
        assert_eq!(
            RuntimeIssue::FrameTimeExceeded {
                frame_time_ms: 50,
                threshold_ms: 33
            }
            .title(),
            "Frame Time Exceeded"
        );
        assert_eq!(
            RuntimeIssue::LowFps {
                fps: 15,
                minimum_fps: 30
            }
            .title(),
            "Low FPS"
        );
        assert_eq!(
            RuntimeIssue::EntityCountMismatch {
                expected: 100,
                actual: 95
            }
            .title(),
            "Entity Count Mismatch"
        );
    }

    #[test]
    fn runtime_issue_icon() {
        assert_eq!(RuntimeIssue::MissingSimulation.icon(), "[R]"); // Severity 5
        assert_eq!(RuntimeIssue::MissingEditSnapshot.icon(), "🟠"); // Severity 4
        assert_eq!(
            RuntimeIssue::EntityCountMismatch {
                expected: 100,
                actual: 95
            }
            .icon(),
            "[Y]"
        ); // Severity 3
        assert_eq!(
            RuntimeIssue::FrameTimeExceeded {
                frame_time_ms: 50,
                threshold_ms: 33
            }
            .icon(),
            "[G]"
        ); // Severity 2
        assert_eq!(
            RuntimeIssue::LowFps {
                fps: 15,
                minimum_fps: 30
            }
            .icon(),
            "ℹ️"
        ); // Severity 1
    }

    // === RuntimeStats helper methods tests ===

    #[test]
    fn runtime_stats_is_healthy() {
        let healthy_stats = RuntimeStats {
            fps: 60.0,
            frame_time_ms: 16.0,
            ..Default::default()
        };
        assert!(healthy_stats.is_healthy());

        let unhealthy_stats = RuntimeStats {
            fps: 25.0,
            frame_time_ms: 40.0,
            ..Default::default()
        };
        assert!(!unhealthy_stats.is_healthy());
    }

    #[test]
    fn runtime_stats_status_color() {
        let excellent = RuntimeStats {
            fps: 60.0,
            frame_time_ms: 16.0,
            ..Default::default()
        };
        assert_eq!(excellent.status_color(), "green");

        let good = RuntimeStats {
            fps: 50.0,
            frame_time_ms: 20.0,
            ..Default::default()
        };
        assert_eq!(good.status_color(), "lightgreen");

        let fair = RuntimeStats {
            fps: 40.0,
            frame_time_ms: 25.0,
            ..Default::default()
        };
        assert_eq!(fair.status_color(), "yellow");

        let poor = RuntimeStats {
            fps: 25.0,
            frame_time_ms: 40.0,
            ..Default::default()
        };
        assert_eq!(poor.status_color(), "orange");

        let critical = RuntimeStats {
            fps: 10.0, // Changed from 15.0 to be in Critical range (0-14)
            frame_time_ms: 100.0,
            ..Default::default()
        };
        assert_eq!(critical.status_color(), "red");
    }

    #[test]
    fn runtime_stats_fps_stability() {
        let perfect = RuntimeStats {
            fps: 60.0,
            ..Default::default()
        };
        assert!((perfect.fps_stability() - 1.0).abs() < 0.01);

        let half = RuntimeStats {
            fps: 30.0,
            ..Default::default()
        };
        assert!((half.fps_stability() - 0.5).abs() < 0.01);

        let zero = RuntimeStats {
            fps: 0.0,
            ..Default::default()
        };
        assert_eq!(zero.fps_stability(), 0.0);

        let over = RuntimeStats {
            fps: 120.0, // Higher than 60 FPS caps at 1.0
            ..Default::default()
        };
        assert_eq!(over.fps_stability(), 1.0);
    }

    #[test]
    fn runtime_stats_frame_time_percentage_of_budget() {
        let half_budget = RuntimeStats {
            frame_time_ms: 8.3335,
            ..Default::default()
        };
        assert!((half_budget.frame_time_percentage_of_budget() - 50.0).abs() < 1.0);

        let full_budget = RuntimeStats {
            frame_time_ms: 16.667,
            ..Default::default()
        };
        assert!((full_budget.frame_time_percentage_of_budget() - 100.0).abs() < 1.0);

        let over_budget = RuntimeStats {
            frame_time_ms: 33.334,
            ..Default::default()
        };
        assert!((over_budget.frame_time_percentage_of_budget() - 200.0).abs() < 1.0);
    }

    #[test]
    fn runtime_stats_is_frame_time_critical() {
        let good = RuntimeStats {
            frame_time_ms: 16.0,
            ..Default::default()
        };
        assert!(!good.is_frame_time_critical());

        let ok = RuntimeStats {
            frame_time_ms: 30.0,
            ..Default::default()
        };
        assert!(!ok.is_frame_time_critical());

        let critical = RuntimeStats {
            frame_time_ms: 35.0,
            ..Default::default()
        };
        assert!(critical.is_frame_time_critical());

        let very_critical = RuntimeStats {
            frame_time_ms: 50.0,
            ..Default::default()
        };
        assert!(very_critical.is_frame_time_critical());
    }

    #[test]
    fn runtime_stats_frame_time_headroom() {
        let perfect = RuntimeStats {
            frame_time_ms: 10.0,
            ..Default::default()
        };
        assert!((perfect.frame_time_headroom() - 6.667).abs() < 0.01);

        let no_headroom = RuntimeStats {
            frame_time_ms: 16.667,
            ..Default::default()
        };
        assert!(no_headroom.frame_time_headroom().abs() < 0.01);

        let negative = RuntimeStats {
            frame_time_ms: 20.0,
            ..Default::default()
        };
        assert!(negative.frame_time_headroom() < 0.0);
    }
}

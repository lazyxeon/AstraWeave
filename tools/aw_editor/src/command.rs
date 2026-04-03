//! Editor command system - Undo/Redo infrastructure
//!
//! Implements the Command pattern for all editor operations.
//! Every action (transform, create, delete, edit) is wrapped in a command
//! that can be undone and redone.
//!
//! Commands return `Box<dyn EditorCommand>` from `new()` to enable polymorphic
//! storage in the undo stack. This is an intentional deviation from the typical
//! `new() -> Self` convention.

#![allow(clippy::new_ret_no_self)]

//! # Architecture
//!
//! ```text
//! EditorCommand (trait)
//!   ├─ Execute: Apply the change
//!   ├─ Undo: Revert the change
//!   └─ Describe: Human-readable description
//!
//! UndoStack
//!   ├─ commands: Vec<Box<dyn EditorCommand>>
//!   ├─ cursor: Current position in history
//!   └─ max_size: Memory limit (default: 100)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aw_editor::command::{EditorCommand, UndoStack, MoveEntityCommand};
//! use astraweave_core::{World, IVec2};
//!
//! let mut world = World::new();
//! let mut undo_stack = UndoStack::new(100);
//!
//! // Execute a command
//! let cmd = MoveEntityCommand::new(entity_id, old_pos, new_pos);
//! undo_stack.execute(cmd, &mut world)?;
//!
//! // Undo
//! undo_stack.undo(&mut world)?;
//!
//! // Redo
//! undo_stack.redo(&mut world)?;
//! ```

use crate::clipboard::ClipboardData;
use anyhow::Result;
use astraweave_core::{Entity, IVec2, Team, World};
use std::fmt;
use tracing::debug;

// ============================================================================
// Command Trait
// ============================================================================

/// Editor command that can be executed, undone, and redone.
///
/// All editor operations should implement this trait to support undo/redo.
/// Commands are stored in an `UndoStack` and executed in LIFO order.
///
/// # Thread Safety
///
/// Commands must be `Send` to allow async execution in the future.
///
/// # Memory
///
/// Commands should store minimal data (just enough to undo).
/// Large data (e.g., full mesh) should be stored as refs or handles.
pub trait EditorCommand: Send + fmt::Debug + std::any::Any {
    /// Execute the command (apply the change).
    ///
    /// # Errors
    ///
    /// Returns `Err` if the command fails (e.g., entity doesn't exist).
    /// Failed commands are NOT added to the undo stack.
    fn execute(&mut self, world: &mut World) -> Result<()>;

    /// Undo the command (revert the change).
    ///
    /// # Errors
    ///
    /// Returns `Err` if undo fails (should be rare - log and continue).
    fn undo(&mut self, world: &mut World) -> Result<()>;

    /// Human-readable description for UI (e.g., "Move Entity", "Rotate Entity").
    ///
    /// Shown in undo menu and status bar.
    fn describe(&self) -> String;

    /// Optional: Merge this command with another if they're similar.
    ///
    /// Used for continuous operations (e.g., dragging gizmo creates 100 move commands,
    /// merge them into one for cleaner undo history).
    ///
    /// Returns `true` if merged successfully.
    fn try_merge(&mut self, _other: &dyn EditorCommand) -> bool {
        false // Default: no merging
    }
}

// ============================================================================
// Undo Stack Statistics
// ============================================================================

/// Statistics about the undo stack state.
#[derive(Debug, Clone, Copy)]
pub struct UndoStackStats {
    /// Total commands currently in the stack
    pub total_commands: usize,
    /// Number of undo operations available
    pub undo_available: usize,
    /// Number of redo operations available
    pub redo_available: usize,
    /// Maximum stack size
    pub max_size: usize,
    /// Whether auto-merge is enabled
    pub auto_merge_enabled: bool,
}

impl UndoStackStats {
    /// Calculate stack utilization as a percentage (0.0 - 1.0)
    pub fn utilization(&self) -> f32 {
        if self.max_size == 0 {
            return 0.0;
        }
        self.total_commands as f32 / self.max_size as f32
    }

    /// Check if stack is near capacity (>80% full)
    pub fn is_near_capacity(&self) -> bool {
        self.utilization() > 0.8
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.total_commands == 0
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.undo_available > 0
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.redo_available > 0
    }

    /// Get remaining capacity (commands before hitting max_size)
    pub fn remaining_capacity(&self) -> usize {
        self.max_size.saturating_sub(self.total_commands)
    }
}

/// Issues that may affect undo stack health.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UndoStackIssue {
    /// Stack is approaching capacity limit
    NearCapacity { utilization_percent: u8 },
    /// Stack is at full capacity (oldest commands being dropped)
    AtCapacity,
    /// Auto-merge is disabled (may cause stack bloat)
    AutoMergeDisabled,
    /// No commands recorded (may indicate broken undo tracking)
    NoHistory,
}

impl UndoStackIssue {
    /// Check if this issue is an error (vs warning)
    pub fn is_error(&self) -> bool {
        matches!(self, UndoStackIssue::AtCapacity)
    }

    /// Get icon for this issue
    pub fn icon(&self) -> &'static str {
        match self {
            UndoStackIssue::NearCapacity { .. } => "[!]",
            UndoStackIssue::AtCapacity => "[!!]",
            UndoStackIssue::AutoMergeDisabled => "[i]",
            UndoStackIssue::NoHistory => "[--]",
        }
    }
}

impl std::fmt::Display for UndoStackIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UndoStackIssue::NearCapacity {
                utilization_percent,
            } => {
                write!(f, "Undo stack {}% full", utilization_percent)
            }
            UndoStackIssue::AtCapacity => write!(f, "Undo stack at capacity"),
            UndoStackIssue::AutoMergeDisabled => {
                write!(f, "Auto-merge disabled (stack may grow quickly)")
            }
            UndoStackIssue::NoHistory => write!(f, "No undo history recorded"),
        }
    }
}

// ============================================================================
// Undo Stack
// ============================================================================

/// Undo/redo stack for editor commands.
///
/// Maintains a history of executed commands with a cursor pointing to the
/// current position. Undo moves cursor backward, redo moves forward.
///
/// # Memory Management
///
/// - Limited to `max_size` commands (default: 100)
/// - Old commands auto-pruned when limit reached
/// - Branching: Executing new command after undo discards redo history
///
/// # Thread Safety
///
/// Not thread-safe by design (editor operations are sequential).
#[derive(Debug)]
pub struct UndoStack {
    /// Command history (oldest to newest)
    commands: Vec<Box<dyn EditorCommand>>,

    /// Current position in history (points to next command to redo)
    ///
    /// - `cursor == 0`: Nothing to undo, can redo commands[0]
    /// - `cursor == len`: Nothing to redo, can undo commands[len-1]
    cursor: usize,

    /// Maximum number of commands to store
    max_size: usize,

    /// Whether to merge consecutive commands (for continuous ops like drag)
    auto_merge: bool,
}

impl UndoStack {
    /// Create a new undo stack.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum commands to store (default: 100, recommended: 50-200)
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::with_capacity(max_size),
            cursor: 0,
            max_size,
            auto_merge: true,
        }
    }

    /// Execute a command and add to undo stack.
    ///
    /// # Behavior
    ///
    /// 1. Execute the command
    /// 2. If successful, add to stack (discarding redo history if any)
    /// 3. If auto_merge enabled, try merging with last command
    /// 4. Prune old commands if over max_size
    ///
    /// # Errors
    ///
    /// Returns `Err` if command execution fails. Failed commands are NOT added to stack.
    #[allow(dead_code)]
    pub fn execute(
        &mut self,
        mut command: Box<dyn EditorCommand>,
        world: &mut World,
    ) -> Result<()> {
        // Execute the command first
        command.execute(world)?;

        // Discard redo history (branching)
        self.commands.truncate(self.cursor);

        // Try merging with last command (if enabled and last command exists)
        if self.auto_merge && self.cursor > 0 {
            if let Some(last_cmd) = self.commands.last_mut() {
                if last_cmd.try_merge(command.as_ref()) {
                    // Merge successful, don't add new command
                    debug!("Merged command: {}", command.describe());
                    return Ok(());
                }
            }
        }

        // Add command to stack
        self.commands.push(command);
        self.cursor += 1;

        // Prune old commands if over limit
        if self.commands.len() > self.max_size {
            let remove_count = self.commands.len() - self.max_size;
            self.commands.drain(0..remove_count);
            self.cursor = self.cursor.saturating_sub(remove_count);
        }

        Ok(())
    }

    /// Undo the last command.
    ///
    /// # Errors
    ///
    /// Returns `Err` if undo fails (logs error and continues).
    pub fn undo(&mut self, world: &mut World) -> Result<()> {
        if self.cursor == 0 {
            return Ok(()); // Nothing to undo
        }

        self.cursor -= 1;
        let cmd = &mut self.commands[self.cursor];

        debug!("Undo: {}", cmd.describe());
        cmd.undo(world)?;

        Ok(())
    }

    /// Redo the next command.
    ///
    /// # Errors
    ///
    /// Returns `Err` if redo fails (logs error and continues).
    pub fn redo(&mut self, world: &mut World) -> Result<()> {
        if self.cursor >= self.commands.len() {
            return Ok(()); // Nothing to redo
        }

        let cmd = &mut self.commands[self.cursor];

        debug!(">|  Redo: {}", cmd.describe());
        cmd.execute(world)?;

        self.cursor += 1;

        Ok(())
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.commands.len()
    }

    /// Get description of last undo command (for UI tooltip).
    pub fn undo_description(&self) -> Option<String> {
        if self.can_undo() {
            Some(self.commands[self.cursor - 1].describe())
        } else {
            None
        }
    }

    /// Get description of next redo command (for UI tooltip).
    pub fn redo_description(&self) -> Option<String> {
        if self.can_redo() {
            Some(self.commands[self.cursor].describe())
        } else {
            None
        }
    }

    /// Clear all commands (use when loading new scene).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.commands.clear();
        self.cursor = 0;
    }

    /// Enable/disable auto-merging of consecutive commands.
    #[allow(dead_code)]
    pub fn set_auto_merge(&mut self, enabled: bool) {
        self.auto_merge = enabled;
    }

    /// Get current command count (for debugging/UI).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if stack is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get current cursor position (for calculating undo depth).
    #[allow(dead_code)]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get number of available undo operations.
    pub fn undo_count(&self) -> usize {
        self.cursor
    }

    /// Get number of available redo operations.
    pub fn redo_count(&self) -> usize {
        self.commands.len().saturating_sub(self.cursor)
    }

    /// Get statistics about the undo stack for debugging and UI display.
    pub fn stats(&self) -> UndoStackStats {
        UndoStackStats {
            total_commands: self.commands.len(),
            undo_available: self.cursor,
            redo_available: self.commands.len().saturating_sub(self.cursor),
            max_size: self.max_size,
            auto_merge_enabled: self.auto_merge,
        }
    }

    /// Validate the undo stack and return any issues found.
    pub fn validate(&self) -> Vec<UndoStackIssue> {
        let mut issues = Vec::new();

        let utilization = if self.max_size > 0 {
            (self.commands.len() as f32 / self.max_size as f32 * 100.0) as u8
        } else {
            0
        };

        if self.commands.len() >= self.max_size {
            issues.push(UndoStackIssue::AtCapacity);
        } else if utilization > 80 {
            issues.push(UndoStackIssue::NearCapacity {
                utilization_percent: utilization,
            });
        }

        if !self.auto_merge {
            issues.push(UndoStackIssue::AutoMergeDisabled);
        }

        if self.commands.is_empty() {
            issues.push(UndoStackIssue::NoHistory);
        }

        issues
    }

    /// Check if undo stack has no issues
    pub fn is_valid(&self) -> bool {
        self.validate().iter().all(|i| !i.is_error())
    }

    /// Get descriptions of recent commands (for history UI)
    pub fn recent_commands(&self, count: usize) -> Vec<String> {
        let start = self.cursor.saturating_sub(count);
        self.commands[start..self.cursor]
            .iter()
            .map(|cmd| cmd.describe())
            .collect()
    }

    /// Get descriptions of upcoming redo commands
    pub fn upcoming_redos(&self, count: usize) -> Vec<String> {
        let end = (self.cursor + count).min(self.commands.len());
        self.commands[self.cursor..end]
            .iter()
            .map(|cmd| cmd.describe())
            .collect()
    }

    /// Get max size of the stack
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Check if auto-merge is enabled
    pub fn is_auto_merge_enabled(&self) -> bool {
        self.auto_merge
    }

    /// Execute multiple commands as a single undoable batch.
    ///
    /// All commands in the batch are executed in order. If any fails,
    /// previously executed commands in the batch are rolled back.
    /// The entire batch appears as a single entry in the undo history.
    ///
    /// # Arguments
    ///
    /// * `commands` - Vector of commands to execute as a batch
    /// * `world` - Mutable world reference
    /// * `description` - Description for the batch operation
    ///
    /// # Errors
    ///
    /// Returns `Err` if any command fails. Partially executed commands are undone.
    pub fn execute_batch(
        &mut self,
        commands: Vec<Box<dyn EditorCommand>>,
        world: &mut World,
        description: String,
    ) -> Result<()> {
        if commands.is_empty() {
            return Ok(());
        }

        let batch = BatchCommand::new(commands, description);
        self.execute(batch, world)
    }

    /// Add an already-executed command to the undo stack.
    ///
    /// Use this when you've already applied a transform (e.g., during gizmo drag)
    /// and just need to record it for undo/redo without executing again.
    pub fn push_executed(&mut self, command: Box<dyn EditorCommand>) {
        self.commands.truncate(self.cursor);

        if self.auto_merge && self.cursor > 0 {
            if let Some(last_cmd) = self.commands.last_mut() {
                if last_cmd.try_merge(command.as_ref()) {
                    debug!("Merged command: {}", command.describe());
                    return;
                }
            }
        }

        self.commands.push(command);
        self.cursor += 1;

        if self.commands.len() > self.max_size {
            let remove_count = self.commands.len() - self.max_size;
            self.commands.drain(0..remove_count);
            self.cursor = self.cursor.saturating_sub(remove_count);
        }
    }
}

// ============================================================================
// Batch Command - Group multiple operations as one undoable action
// ============================================================================

/// Groups multiple commands into a single undoable operation.
///
/// Useful for complex operations like "Align All Selected" that affect
/// multiple entities but should be undone/redone as a single action.
///
/// # Rollback Behavior
///
/// If any command in the batch fails during execution, all previously
/// executed commands are automatically rolled back (undone) to maintain
/// consistency.
#[derive(Debug)]
pub struct BatchCommand {
    /// Commands in execution order
    commands: Vec<Box<dyn EditorCommand>>,
    /// Human-readable description for undo menu
    description: String,
    /// Index of last successfully executed command (for rollback)
    executed_up_to: usize,
}

impl BatchCommand {
    /// Create a new batch command.
    ///
    /// # Arguments
    ///
    /// * `commands` - Commands to execute in order
    /// * `description` - Description for the batch (e.g., "Align 5 entities")
    pub fn new(commands: Vec<Box<dyn EditorCommand>>, description: String) -> Box<Self> {
        Box::new(Self {
            commands,
            description,
            executed_up_to: 0,
        })
    }

    /// Create a batch command from multiple move operations.
    pub fn from_moves(entity_moves: Vec<(Entity, IVec2, IVec2)>) -> Box<Self> {
        let count = entity_moves.len();
        let commands: Vec<Box<dyn EditorCommand>> = entity_moves
            .into_iter()
            .map(|(entity, old_pos, new_pos)| {
                MoveEntityCommand::new(entity, old_pos, new_pos) as Box<dyn EditorCommand>
            })
            .collect();

        Box::new(Self {
            commands,
            description: format!("Move {} entities", count),
            executed_up_to: 0,
        })
    }

    /// Get the number of commands in this batch.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl EditorCommand for BatchCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        self.executed_up_to = 0;

        for (i, cmd) in self.commands.iter_mut().enumerate() {
            match cmd.execute(world) {
                Ok(()) => {
                    self.executed_up_to = i + 1;
                }
                Err(e) => {
                    // Rollback previously executed commands in reverse order
                    for j in (0..i).rev() {
                        if let Err(undo_err) = self.commands[j].undo(world) {
                            debug!("Rollback failed for command {}: {}", j, undo_err);
                        }
                    }
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        // Undo in reverse order, only up to what was executed
        for i in (0..self.executed_up_to).rev() {
            self.commands[i].undo(world)?;
        }
        Ok(())
    }

    fn describe(&self) -> String {
        self.description.clone()
    }
}

// ============================================================================
// Concrete Commands - Transform Operations
// ============================================================================

/// Move entity command (translate operation).
#[derive(Debug)]
pub struct MoveEntityCommand {
    entity_id: Entity,
    old_pos: IVec2,
    new_pos: IVec2,
}

impl MoveEntityCommand {
    pub fn new(entity_id: Entity, old_pos: IVec2, new_pos: IVec2) -> Box<Self> {
        Box::new(Self {
            entity_id,
            old_pos,
            new_pos,
        })
    }
}

impl EditorCommand for MoveEntityCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.pos = self.new_pos;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.pos = self.old_pos;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn describe(&self) -> String {
        format!("Move Entity {:?}", self.entity_id)
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        // Try to downcast to MoveEntityCommand
        if let Some(other_move) = (other as &dyn std::any::Any).downcast_ref::<MoveEntityCommand>()
        {
            // Only merge if same entity
            if self.entity_id == other_move.entity_id {
                // Update new position but keep old position (chain multiple moves)
                self.new_pos = other_move.new_pos;
                return true;
            }
        }
        false
    }
}

/// Rotate entity command.
#[derive(Debug)]
pub struct RotateEntityCommand {
    entity_id: Entity,
    old_rotation_x: f32,
    old_rotation_y: f32,
    old_rotation_z: f32,
    new_rotation_x: f32,
    new_rotation_y: f32,
    new_rotation_z: f32,
}

impl RotateEntityCommand {
    pub fn new(
        entity_id: Entity,
        old_rotation: (f32, f32, f32), // (x, y, z)
        new_rotation: (f32, f32, f32),
    ) -> Box<Self> {
        Box::new(Self {
            entity_id,
            old_rotation_x: old_rotation.0,
            old_rotation_y: old_rotation.1,
            old_rotation_z: old_rotation.2,
            new_rotation_x: new_rotation.0,
            new_rotation_y: new_rotation.1,
            new_rotation_z: new_rotation.2,
        })
    }
}

impl EditorCommand for RotateEntityCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.rotation_x = self.new_rotation_x;
            pose.rotation = self.new_rotation_y; // Y-axis
            pose.rotation_z = self.new_rotation_z;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.rotation_x = self.old_rotation_x;
            pose.rotation = self.old_rotation_y; // Y-axis
            pose.rotation_z = self.old_rotation_z;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn describe(&self) -> String {
        format!("Rotate Entity {:?}", self.entity_id)
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        if let Some(other_rot) = (other as &dyn std::any::Any).downcast_ref::<RotateEntityCommand>()
        {
            if self.entity_id == other_rot.entity_id {
                // Update new rotation but keep old rotation
                self.new_rotation_x = other_rot.new_rotation_x;
                self.new_rotation_y = other_rot.new_rotation_y;
                self.new_rotation_z = other_rot.new_rotation_z;
                return true;
            }
        }
        false
    }
}

/// Scale entity command.
#[derive(Debug)]
pub struct ScaleEntityCommand {
    entity_id: Entity,
    old_scale: f32,
    new_scale: f32,
}

impl ScaleEntityCommand {
    pub fn new(entity_id: Entity, old_scale: f32, new_scale: f32) -> Box<Self> {
        Box::new(Self {
            entity_id,
            old_scale,
            new_scale,
        })
    }
}

impl EditorCommand for ScaleEntityCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.scale = self.new_scale;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(pose) = world.pose_mut(self.entity_id) {
            pose.scale = self.old_scale;
            Ok(())
        } else {
            anyhow::bail!("Entity {:?} not found", self.entity_id)
        }
    }

    fn describe(&self) -> String {
        format!("Scale Entity {:?}", self.entity_id)
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        if let Some(other_scale) =
            (other as &dyn std::any::Any).downcast_ref::<ScaleEntityCommand>()
        {
            if self.entity_id == other_scale.entity_id {
                self.new_scale = other_scale.new_scale;
                return true;
            }
        }
        false
    }
}

#[derive(Debug)]
pub struct EditHealthCommand {
    entity: Entity,
    old_hp: i32,
    new_hp: i32,
}

impl EditHealthCommand {
    pub fn new(entity: Entity, old_hp: i32, new_hp: i32) -> Box<Self> {
        Box::new(Self {
            entity,
            old_hp,
            new_hp,
        })
    }
}

impl EditorCommand for EditHealthCommand {
    #[allow(dead_code)]
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(health) = world.health_mut(self.entity) {
            health.hp = self.new_hp;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Health component",
                self.entity
            ))
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(health) = world.health_mut(self.entity) {
            health.hp = self.old_hp;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Health component",
                self.entity
            ))
        }
    }

    fn describe(&self) -> String {
        format!("Edit Health (Entity #{})", self.entity)
    }
}

#[derive(Debug)]
pub struct EditTeamCommand {
    entity: Entity,
    old_team: Team,
    new_team: Team,
}

impl EditTeamCommand {
    pub fn new(entity: Entity, old_team: Team, new_team: Team) -> Box<Self> {
        Box::new(Self {
            entity,
            old_team,
            new_team,
        })
    }
}

impl EditorCommand for EditTeamCommand {
    #[allow(dead_code)]
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(team) = world.team_mut(self.entity) {
            *team = self.new_team;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Team component",
                self.entity
            ))
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(team) = world.team_mut(self.entity) {
            *team = self.old_team;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Team component",
                self.entity
            ))
        }
    }

    fn describe(&self) -> String {
        format!("Edit Team (Entity #{})", self.entity)
    }
}

#[derive(Debug)]
pub struct EditAmmoCommand {
    entity: Entity,
    old_rounds: i32,
    new_rounds: i32,
}

impl EditAmmoCommand {
    pub fn new(entity: Entity, old_rounds: i32, new_rounds: i32) -> Box<Self> {
        Box::new(Self {
            entity,
            old_rounds,
            new_rounds,
        })
    }
}

impl EditorCommand for EditAmmoCommand {
    #[allow(dead_code)]
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if let Some(ammo) = world.ammo_mut(self.entity) {
            ammo.rounds = self.new_rounds;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Ammo component",
                self.entity
            ))
        }
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(ammo) = world.ammo_mut(self.entity) {
            ammo.rounds = self.old_rounds;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Entity {} has no Ammo component",
                self.entity
            ))
        }
    }

    fn describe(&self) -> String {
        format!("Edit Ammo (Entity #{})", self.entity)
    }
}

#[derive(Debug)]
pub struct SpawnEntitiesCommand {
    spawned_entities: Vec<Entity>,
    clipboard_data: ClipboardData,
    offset: IVec2,
}

impl SpawnEntitiesCommand {
    pub fn new(clipboard_data: ClipboardData, offset: IVec2) -> Box<dyn EditorCommand> {
        Box::new(Self {
            spawned_entities: Vec::new(),
            clipboard_data,
            offset,
        })
    }
}

impl EditorCommand for SpawnEntitiesCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        self.spawned_entities = self.clipboard_data.spawn_entities(world, self.offset)?;
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        for &entity in &self.spawned_entities {
            if let Some(pose) = world.pose_mut(entity) {
                *pose = astraweave_core::Pose {
                    pos: IVec2 {
                        x: -10000,
                        y: -10000,
                    },
                    height: 0.0,
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    float_x: 0.0,
                    float_z: 0.0,
                    use_float_pos: false,
                    scale: 0.0,
                };
            }
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Paste {} entities", self.clipboard_data.entities.len())
    }
}

#[derive(Debug)]
pub struct DuplicateEntitiesCommand {
    source_entities: Vec<Entity>,
    spawned_entities: Vec<Entity>,
    offset: IVec2,
}

impl DuplicateEntitiesCommand {
    pub fn new(source_entities: Vec<Entity>, offset: IVec2) -> Box<dyn EditorCommand> {
        Box::new(Self {
            source_entities,
            spawned_entities: Vec::new(),
            offset,
        })
    }
}

impl EditorCommand for DuplicateEntitiesCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        let clipboard = ClipboardData::from_entities(world, &self.source_entities);
        self.spawned_entities = clipboard.spawn_entities(world, self.offset)?;
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        for &entity in &self.spawned_entities {
            if let Some(pose) = world.pose_mut(entity) {
                *pose = astraweave_core::Pose {
                    pos: IVec2 {
                        x: -10000,
                        y: -10000,
                    },
                    height: 0.0,
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    float_x: 0.0,
                    float_z: 0.0,
                    use_float_pos: false,
                    scale: 0.0,
                };
            }
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Duplicate {} entities", self.source_entities.len())
    }
}

#[derive(Debug)]
pub struct DeleteEntitiesCommand {
    entities: Vec<Entity>,
    clipboard_data: Option<ClipboardData>,
}

impl DeleteEntitiesCommand {
    pub fn new(entities: Vec<Entity>) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entities,
            clipboard_data: None,
        })
    }
}

impl EditorCommand for DeleteEntitiesCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        // Only capture snapshot if we haven't already (preserves original state across Re-Dos)
        if self.clipboard_data.is_none() {
            self.clipboard_data = Some(ClipboardData::from_entities(world, &self.entities));
        }

        for &entity in &self.entities {
            world.destroy_entity(entity);
        }
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(clipboard) = &self.clipboard_data {
            if clipboard.entities.len() != self.entities.len() {
                anyhow::bail!("DeleteEntitiesCommand: Snapshot count mismatch");
            }

            for (i, data) in clipboard.entities.iter().enumerate() {
                let id = self.entities[i];
                let team = Team { id: data.team_id };

                // Restore with original ID to preserve undo history references
                // Note: spawn_with_id handles ID allocation if needed
                world.spawn_with_id(id, &data.name, data.pos, team, data.hp, data.ammo);

                // Restore components that aren't covered by spawn_with_id
                if let Some(pose) = world.pose_mut(id) {
                    pose.rotation = data.rotation;
                    pose.rotation_x = data.rotation_x;
                    pose.rotation_z = data.rotation_z;
                    pose.scale = data.scale;
                }

                if let Some(cooldowns) = world.cooldowns_mut(id) {
                    cooldowns.map = data.cooldowns.clone();
                }

                if let Some(bg) = &data.behavior_graph {
                    world.set_behavior_graph(id, bg.clone());
                }
            }
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Delete {} entities", self.entities.len())
    }
}

// ============================================================================
// Hierarchy Commands
// ============================================================================

/// Command to rename an entity with undo support.
#[derive(Debug)]
pub struct RenameEntityCommand {
    entity: Entity,
    old_name: String,
    new_name: String,
}

impl RenameEntityCommand {
    pub fn new(entity: Entity, old_name: String, new_name: String) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity,
            old_name,
            new_name,
        })
    }
}

impl EditorCommand for RenameEntityCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        world.set_name(self.entity, self.new_name.clone());
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        world.set_name(self.entity, self.old_name.clone());
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Rename entity {} to '{}'", self.entity, self.new_name)
    }
}

/// Command to set a parent-child relationship with undo support.
#[derive(Debug)]
pub struct SetParentCommand {
    child: Entity,
    new_parent: Entity,
    old_parent: Option<Entity>,
}

impl SetParentCommand {
    pub fn new(
        child: Entity,
        new_parent: Entity,
        old_parent: Option<Entity>,
    ) -> Box<dyn EditorCommand> {
        Box::new(Self {
            child,
            new_parent,
            old_parent,
        })
    }
}

impl EditorCommand for SetParentCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        world.set_parent(self.child, self.new_parent);
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(old_parent) = self.old_parent {
            world.set_parent(self.child, old_parent);
        } else {
            world.remove_parent(self.child);
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Set parent of {} to {}", self.child, self.new_parent)
    }
}

/// Command to unparent an entity with undo support.
#[derive(Debug)]
pub struct UnparentCommand {
    entity: Entity,
    old_parent: Option<Entity>,
}

impl UnparentCommand {
    pub fn new(entity: Entity, old_parent: Option<Entity>) -> Box<dyn EditorCommand> {
        Box::new(Self { entity, old_parent })
    }
}

impl EditorCommand for UnparentCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        world.remove_parent(self.entity);
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(parent) = self.old_parent {
            world.set_parent(self.entity, parent);
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Unparent entity {}", self.entity)
    }
}

// ============================================================================
// Prefab Commands
// ============================================================================

use crate::prefab::{PrefabData, PrefabInstanceSnapshot, PrefabManager};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Command to spawn a prefab with undo support
#[derive(Debug)]
pub struct PrefabSpawnCommand {
    prefab_path: PathBuf,
    prefab_manager: Arc<Mutex<PrefabManager>>,
    spawn_pos: (i32, i32),
    spawned_entity: Option<Entity>,
}

impl PrefabSpawnCommand {
    pub fn new(
        prefab_path: PathBuf,
        prefab_manager: Arc<Mutex<PrefabManager>>,
        spawn_pos: (i32, i32),
    ) -> Box<dyn EditorCommand> {
        Box::new(Self {
            prefab_path,
            prefab_manager,
            spawn_pos,
            spawned_entity: None,
        })
    }
}

impl EditorCommand for PrefabSpawnCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        let mut manager = self
            .prefab_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;

        let entity = manager.instantiate_prefab(&self.prefab_path, world, self.spawn_pos)?;
        self.spawned_entity = Some(entity);
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(entity) = self.spawned_entity {
            // Move entity out of view (soft delete) - same pattern as other commands
            if let Some(pose) = world.pose_mut(entity) {
                *pose = astraweave_core::Pose {
                    pos: IVec2 {
                        x: -10000,
                        y: -10000,
                    },
                    height: 0.0,
                    rotation: 0.0,
                    rotation_x: 0.0,
                    rotation_z: 0.0,
                    float_x: 0.0,
                    float_z: 0.0,
                    use_float_pos: false,
                    scale: 0.0,
                };
            }

            // Remove instance tracking
            let mut manager = self
                .prefab_manager
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;
            manager.remove_instance(entity);
        }
        Ok(())
    }

    fn describe(&self) -> String {
        let name = self
            .prefab_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");
        format!("Spawn prefab '{}'", name)
    }
}

/// Command to apply overrides from an instance back to its prefab file
#[derive(Debug)]
pub struct PrefabApplyOverridesCommand {
    entity: Entity,
    prefab_manager: Arc<Mutex<PrefabManager>>,
    original_prefab_data: Option<PrefabData>,
}

impl PrefabApplyOverridesCommand {
    pub fn new(
        entity: Entity,
        prefab_manager: Arc<Mutex<PrefabManager>>,
    ) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity,
            prefab_manager,
            original_prefab_data: None,
        })
    }
}

impl EditorCommand for PrefabApplyOverridesCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        let mut manager = self
            .prefab_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;

        // Find the prefab instance for this entity
        let instance = manager
            .find_instance(self.entity)
            .ok_or_else(|| anyhow::anyhow!("Entity {} is not a prefab instance", self.entity))?;

        let prefab_path = instance.source.clone();

        // Load and store original data for undo
        self.original_prefab_data = Some(PrefabData::load_from_file(&prefab_path)?);

        // Apply overrides to prefab file
        manager.apply_overrides_to_prefab(self.entity, world)?;

        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(original_data) = &self.original_prefab_data {
            let manager = self
                .prefab_manager
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;

            // Find the prefab path
            if let Some(instance) = manager.find_instance(self.entity) {
                // Restore original prefab file
                original_data.save_to_file(&instance.source)?;

                // Revert instance to prefab values
                drop(manager); // Release lock before mutable world access
                let mut manager = self
                    .prefab_manager
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;
                manager.revert_instance_to_prefab(self.entity, world)?;
            }
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Apply overrides to prefab (entity #{})", self.entity)
    }
}

#[derive(Debug)]
pub struct PrefabRevertOverridesCommand {
    entity: Entity,
    prefab_manager: Arc<Mutex<PrefabManager>>,
    snapshot: PrefabInstanceSnapshot,
}

impl PrefabRevertOverridesCommand {
    pub fn new(
        prefab_manager: Arc<Mutex<PrefabManager>>,
        entity: Entity,
        snapshot: PrefabInstanceSnapshot,
    ) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity,
            prefab_manager,
            snapshot,
        })
    }
}

impl EditorCommand for PrefabRevertOverridesCommand {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        let mut manager = self
            .prefab_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;

        manager.revert_instance_to_prefab(self.entity, world)?;
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        let mut manager = self
            .prefab_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock prefab manager: {}", e))?;

        manager.restore_snapshot(&self.snapshot, world)?;
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Revert prefab overrides (entity #{})", self.entity)
    }
}

// ============================================================================
// Terrain Brush Undo/Redo
// ============================================================================

/// Action pushed by TerrainBrushCommand for the main loop to apply.
#[derive(Debug, Clone)]
pub enum TerrainUndoAction {
    /// Apply these heightmap values (ChunkId → heights vec)
    ApplyHeights(Vec<(astraweave_terrain::ChunkId, Vec<f32>)>),
}

/// Shared side-channel for terrain undo commands to push actions to the main loop.
pub type TerrainUndoQueue = std::sync::Arc<std::sync::Mutex<Vec<TerrainUndoAction>>>;

/// Create a new terrain undo action queue.
pub fn new_terrain_undo_queue() -> TerrainUndoQueue {
    std::sync::Arc::new(std::sync::Mutex::new(Vec::new()))
}

/// Undo/redo command for a terrain brush stroke.
///
/// Stores pre-stroke and post-stroke heightmap data for each affected chunk.
/// On undo: pushes pre-heights to the side-channel queue.
/// On redo: pushes post-heights to the side-channel queue.
/// The main loop drains this queue and applies heights to `TerrainState`.
#[derive(Debug)]
pub struct TerrainBrushCommand {
    /// (ChunkId, pre_heights, post_heights) for each modified chunk
    chunk_deltas: Vec<(astraweave_terrain::ChunkId, Vec<f32>, Vec<f32>)>,
    /// Side-channel to push actions for the main loop
    undo_queue: TerrainUndoQueue,
    /// Human-readable description
    description: String,
}

impl TerrainBrushCommand {
    pub fn new(
        chunk_deltas: Vec<(astraweave_terrain::ChunkId, Vec<f32>, Vec<f32>)>,
        undo_queue: TerrainUndoQueue,
        brush_name: &str,
    ) -> Box<dyn EditorCommand> {
        Box::new(Self {
            chunk_deltas,
            undo_queue,
            description: format!("Terrain {brush_name}"),
        })
    }
}

impl EditorCommand for TerrainBrushCommand {
    fn execute(&mut self, _world: &mut World) -> Result<()> {
        // Redo: apply post-stroke heights
        let heights: Vec<_> = self
            .chunk_deltas
            .iter()
            .map(|(id, _pre, post)| (*id, post.clone()))
            .collect();
        if let Ok(mut queue) = self.undo_queue.lock() {
            queue.push(TerrainUndoAction::ApplyHeights(heights));
        }
        Ok(())
    }

    fn undo(&mut self, _world: &mut World) -> Result<()> {
        // Undo: apply pre-stroke heights
        let heights: Vec<_> = self
            .chunk_deltas
            .iter()
            .map(|(id, pre, _post)| (*id, pre.clone()))
            .collect();
        if let Ok(mut queue) = self.undo_queue.lock() {
            queue.push(TerrainUndoAction::ApplyHeights(heights));
        }
        Ok(())
    }

    fn describe(&self) -> String {
        self.description.clone()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_basic_entity(world: &mut World) -> Entity {
        world.spawn("test", IVec2::new(0, 0), Team { id: 0 }, 100, 30)
    }

    #[test]
    fn test_undo_stack_basic() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);

        // Execute move command
        let cmd = MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(5, 5));
        stack.execute(cmd, &mut world).unwrap();

        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        // Undo
        stack.undo(&mut world).unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        // Redo
        stack.redo(&mut world).unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_undo_stack_branching() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false); // Disable merging to test branching

        // Execute two commands
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(5, 5)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(5, 5), IVec2::new(10, 10)),
                &mut world,
            )
            .unwrap();

        // Undo once
        stack.undo(&mut world).unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));

        // Execute new command (should discard redo history)
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(5, 5), IVec2::new(20, 20)),
                &mut world,
            )
            .unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(20, 20));
        assert!(!stack.can_redo()); // Redo history discarded
    }

    #[test]
    fn test_command_merging() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(true);

        // Execute 5 consecutive moves (should merge into 1)
        for i in 1..=5 {
            stack
                .execute(
                    MoveEntityCommand::new(entity, IVec2::new(i - 1, i - 1), IVec2::new(i, i)),
                    &mut world,
                )
                .unwrap();
        }

        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));
        assert_eq!(stack.len(), 1); // All 5 commands merged into 1

        // Undo once (should revert all 5 moves)
        stack.undo(&mut world).unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
    }

    #[test]
    fn test_rotate_command() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);

        let old_rot = (0.0, 0.0, 0.0);
        let new_rot = (0.5, 1.0, 1.5);

        stack
            .execute(
                RotateEntityCommand::new(entity, old_rot, new_rot),
                &mut world,
            )
            .unwrap();

        let pose = world.pose(entity).unwrap();
        assert!((pose.rotation_x - 0.5).abs() < 0.01);
        assert!((pose.rotation - 1.0).abs() < 0.01);
        assert!((pose.rotation_z - 1.5).abs() < 0.01);

        stack.undo(&mut world).unwrap();

        let pose = world.pose(entity).unwrap();
        assert!((pose.rotation_x - 0.0).abs() < 0.01);
        assert!((pose.rotation - 0.0).abs() < 0.01);
        assert!((pose.rotation_z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_scale_command() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);

        stack
            .execute(ScaleEntityCommand::new(entity, 1.0, 2.5), &mut world)
            .unwrap();

        assert!((world.pose(entity).unwrap().scale - 2.5).abs() < 0.01);

        stack.undo(&mut world).unwrap();
        assert!((world.pose(entity).unwrap().scale - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_edit_health_command() {
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        let mut stack = UndoStack::new(10);

        stack.push_executed(EditHealthCommand::new(entity, 100, 50));

        stack.undo(&mut world).unwrap();
        assert_eq!(world.health(entity).unwrap().hp, 100);

        stack.redo(&mut world).unwrap();
        assert_eq!(world.health(entity).unwrap().hp, 50);
    }

    #[test]
    fn test_edit_team_command() {
        let mut world = World::new();
        let entity = world.spawn("Enemy", IVec2::new(0, 0), Team { id: 2 }, 100, 30);

        let mut stack = UndoStack::new(10);
        let old_team = Team { id: 2 };
        let new_team = Team { id: 0 };

        stack.push_executed(EditTeamCommand::new(entity, old_team, new_team));

        stack.undo(&mut world).unwrap();
        assert_eq!(world.team(entity).unwrap().id, 2);

        stack.redo(&mut world).unwrap();
        assert_eq!(world.team(entity).unwrap().id, 0);
    }

    #[test]
    fn test_edit_ammo_command() {
        let mut world = World::new();
        let entity = world.spawn("Shooter", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        let mut stack = UndoStack::new(10);

        stack.push_executed(EditAmmoCommand::new(entity, 30, 15));

        stack.undo(&mut world).unwrap();
        assert_eq!(world.ammo(entity).unwrap().rounds, 30);

        stack.redo(&mut world).unwrap();
        assert_eq!(world.ammo(entity).unwrap().rounds, 15);
    }

    #[test]
    fn test_undo_stack_max_size() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(5);
        stack.set_auto_merge(false); // Disable merging to test max size

        for i in 0..10 {
            stack
                .execute(
                    MoveEntityCommand::new(entity, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                    &mut world,
                )
                .unwrap();
        }

        assert_eq!(stack.len(), 5);
        assert_eq!(stack.cursor, 5);
    }

    #[test]
    fn test_undo_stack_descriptions() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(5, 5)),
                &mut world,
            )
            .unwrap();

        assert!(stack.undo_description().unwrap().contains("Move Entity"));
        assert!(stack.redo_description().is_none());

        stack.undo(&mut world).unwrap();
        assert!(stack.redo_description().unwrap().contains("Move Entity"));
        assert!(stack.undo_description().is_none());
    }

    #[test]
    fn test_push_executed() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        world.pose_mut(entity).unwrap().pos = IVec2::new(5, 5);

        let mut stack = UndoStack::new(10);
        stack.push_executed(MoveEntityCommand::new(
            entity,
            IVec2::new(0, 0),
            IVec2::new(5, 5),
        ));

        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));

        stack.undo(&mut world).unwrap();
        assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
    }

    // ====================================================================
    // UndoStackStats Tests
    // ====================================================================

    #[test]
    fn test_undo_stack_stats_basic() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false);

        let stats = stack.stats();
        assert_eq!(stats.total_commands, 0);
        assert_eq!(stats.undo_available, 0);
        assert_eq!(stats.redo_available, 0);
        assert_eq!(stats.max_size, 10);

        // Execute some commands
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(1, 1), IVec2::new(2, 2)),
                &mut world,
            )
            .unwrap();

        let stats = stack.stats();
        assert_eq!(stats.total_commands, 2);
        assert_eq!(stats.undo_available, 2);
        assert_eq!(stats.redo_available, 0);

        // Undo one
        stack.undo(&mut world).unwrap();
        let stats = stack.stats();
        assert_eq!(stats.undo_available, 1);
        assert_eq!(stats.redo_available, 1);
    }

    #[test]
    fn test_undo_stack_stats_utilization() {
        let stack = UndoStack::new(10);

        // Empty stack
        let stats = stack.stats();
        assert!((stats.utilization() - 0.0).abs() < 0.001);
        assert!(!stats.is_near_capacity());
    }

    #[test]
    fn test_undo_stack_stats_near_capacity() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);

        let mut stack = UndoStack::new(5);
        stack.set_auto_merge(false);

        // Add 4 commands (80% full)
        for i in 0..4 {
            stack
                .execute(
                    MoveEntityCommand::new(entity, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                    &mut world,
                )
                .unwrap();
        }

        let stats = stack.stats();
        assert!((stats.utilization() - 0.8).abs() < 0.001);
        assert!(!stats.is_near_capacity()); // exactly 80% is not near

        // Add one more (now 100%)
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(4, 4), IVec2::new(5, 5)),
                &mut world,
            )
            .unwrap();

        let stats = stack.stats();
        assert!((stats.utilization() - 1.0).abs() < 0.001);
        assert!(stats.is_near_capacity());
    }

    // ====================================================================
    // BatchCommand Tests
    // ====================================================================

    #[test]
    fn test_batch_command_basic() {
        let mut world = World::new();
        let e1 = spawn_basic_entity(&mut world);
        let e2 = world.spawn("E2", IVec2::new(5, 5), Team { id: 0 }, 100, 30);

        let commands: Vec<Box<dyn EditorCommand>> = vec![
            MoveEntityCommand::new(e1, IVec2::new(0, 0), IVec2::new(10, 10)),
            MoveEntityCommand::new(e2, IVec2::new(5, 5), IVec2::new(20, 20)),
        ];

        let mut batch = BatchCommand::new(commands, "Move 2 entities".to_string());

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
        assert_eq!(batch.describe(), "Move 2 entities");

        // Execute
        batch.execute(&mut world).unwrap();
        assert_eq!(world.pose(e1).unwrap().pos, IVec2::new(10, 10));
        assert_eq!(world.pose(e2).unwrap().pos, IVec2::new(20, 20));

        // Undo
        batch.undo(&mut world).unwrap();
        assert_eq!(world.pose(e1).unwrap().pos, IVec2::new(0, 0));
        assert_eq!(world.pose(e2).unwrap().pos, IVec2::new(5, 5));
    }

    #[test]
    fn test_batch_command_from_moves() {
        let mut world = World::new();
        let e1 = spawn_basic_entity(&mut world);
        let e2 = world.spawn("E2", IVec2::new(1, 1), Team { id: 0 }, 100, 30);
        let e3 = world.spawn("E3", IVec2::new(2, 2), Team { id: 0 }, 100, 30);

        let moves = vec![
            (e1, IVec2::new(0, 0), IVec2::new(5, 5)),
            (e2, IVec2::new(1, 1), IVec2::new(6, 6)),
            (e3, IVec2::new(2, 2), IVec2::new(7, 7)),
        ];

        let mut batch = BatchCommand::from_moves(moves);

        assert_eq!(batch.len(), 3);
        assert!(batch.describe().contains("3"));

        batch.execute(&mut world).unwrap();
        assert_eq!(world.pose(e1).unwrap().pos, IVec2::new(5, 5));
        assert_eq!(world.pose(e2).unwrap().pos, IVec2::new(6, 6));
        assert_eq!(world.pose(e3).unwrap().pos, IVec2::new(7, 7));
    }

    #[test]
    fn test_batch_command_empty() {
        let mut world = World::new();

        let commands: Vec<Box<dyn EditorCommand>> = vec![];
        let mut batch = BatchCommand::new(commands, "Empty batch".to_string());

        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);

        // Empty batch should succeed
        batch.execute(&mut world).unwrap();
        batch.undo(&mut world).unwrap();
    }

    #[test]
    fn test_execute_batch_on_stack() {
        let mut world = World::new();
        let e1 = spawn_basic_entity(&mut world);
        let e2 = world.spawn("E2", IVec2::new(5, 5), Team { id: 0 }, 100, 30);

        let mut stack = UndoStack::new(10);

        let commands: Vec<Box<dyn EditorCommand>> = vec![
            MoveEntityCommand::new(e1, IVec2::new(0, 0), IVec2::new(10, 10)),
            MoveEntityCommand::new(e2, IVec2::new(5, 5), IVec2::new(20, 20)),
        ];

        stack
            .execute_batch(commands, &mut world, "Batch move".to_string())
            .unwrap();

        assert_eq!(stack.len(), 1); // Should be single entry
        assert!(stack.undo_description().unwrap().contains("Batch move"));

        // Undo the whole batch
        stack.undo(&mut world).unwrap();
        assert_eq!(world.pose(e1).unwrap().pos, IVec2::new(0, 0));
        assert_eq!(world.pose(e2).unwrap().pos, IVec2::new(5, 5));
    }

    #[test]
    fn test_execute_batch_empty() {
        let mut world = World::new();
        let mut stack = UndoStack::new(10);

        // Empty batch should succeed without adding entry
        stack
            .execute_batch(vec![], &mut world, "Empty".to_string())
            .unwrap();

        assert_eq!(stack.len(), 0);
    }

    // ====================================================================
    // UndoStackStats New Methods Tests
    // ====================================================================

    #[test]
    fn test_undo_stack_stats_is_empty() {
        let stack = UndoStack::new(10);
        let stats = stack.stats();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_undo_stack_stats_can_undo_redo() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false);

        let stats = stack.stats();
        assert!(!stats.can_undo());
        assert!(!stats.can_redo());

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();

        let stats = stack.stats();
        assert!(stats.can_undo());
        assert!(!stats.can_redo());

        stack.undo(&mut world).unwrap();

        let stats = stack.stats();
        assert!(!stats.can_undo());
        assert!(stats.can_redo());
    }

    #[test]
    fn test_undo_stack_stats_remaining_capacity() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(5);
        stack.set_auto_merge(false);

        assert_eq!(stack.stats().remaining_capacity(), 5);

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(1, 1), IVec2::new(2, 2)),
                &mut world,
            )
            .unwrap();

        assert_eq!(stack.stats().remaining_capacity(), 3);
    }

    // ====================================================================
    // UndoStackIssue Tests
    // ====================================================================

    #[test]
    fn test_undo_stack_issue_is_error() {
        assert!(!UndoStackIssue::NearCapacity {
            utilization_percent: 85
        }
        .is_error());
        assert!(UndoStackIssue::AtCapacity.is_error());
        assert!(!UndoStackIssue::AutoMergeDisabled.is_error());
        assert!(!UndoStackIssue::NoHistory.is_error());
    }

    #[test]
    fn test_undo_stack_issue_icon_not_empty() {
        assert!(!UndoStackIssue::NearCapacity {
            utilization_percent: 85
        }
        .icon()
        .is_empty());
        assert!(!UndoStackIssue::AtCapacity.icon().is_empty());
        assert!(!UndoStackIssue::AutoMergeDisabled.icon().is_empty());
        assert!(!UndoStackIssue::NoHistory.icon().is_empty());
    }

    #[test]
    fn test_undo_stack_issue_display() {
        let issue = UndoStackIssue::NearCapacity {
            utilization_percent: 85,
        };
        let display = format!("{}", issue);
        assert!(display.contains("85"));

        let issue = UndoStackIssue::AtCapacity;
        let display = format!("{}", issue);
        assert!(display.contains("capacity"));
    }

    // ====================================================================
    // UndoStack Validate Tests
    // ====================================================================

    #[test]
    fn test_undo_stack_validate_empty() {
        let stack = UndoStack::new(10);
        let issues = stack.validate();
        assert!(issues
            .iter()
            .any(|i| matches!(i, UndoStackIssue::NoHistory)));
    }

    #[test]
    fn test_undo_stack_validate_auto_merge_disabled() {
        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false);
        let issues = stack.validate();
        assert!(issues
            .iter()
            .any(|i| matches!(i, UndoStackIssue::AutoMergeDisabled)));
    }

    #[test]
    fn test_undo_stack_validate_at_capacity() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(3);
        stack.set_auto_merge(false);

        for i in 0..3 {
            stack
                .execute(
                    MoveEntityCommand::new(entity, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                    &mut world,
                )
                .unwrap();
        }

        let issues = stack.validate();
        assert!(issues
            .iter()
            .any(|i| matches!(i, UndoStackIssue::AtCapacity)));
    }

    #[test]
    fn test_undo_stack_is_valid() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(10);

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();

        assert!(stack.is_valid());
    }

    // ====================================================================
    // UndoStack Recent Commands Tests
    // ====================================================================

    #[test]
    fn test_undo_stack_recent_commands() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false);

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(1, 1), IVec2::new(2, 2)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(2, 2), IVec2::new(3, 3)),
                &mut world,
            )
            .unwrap();

        let recent = stack.recent_commands(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_undo_stack_upcoming_redos() {
        let mut world = World::new();
        let entity = spawn_basic_entity(&mut world);
        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(false);

        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(1, 1)),
                &mut world,
            )
            .unwrap();
        stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(1, 1), IVec2::new(2, 2)),
                &mut world,
            )
            .unwrap();

        stack.undo(&mut world).unwrap();
        stack.undo(&mut world).unwrap();

        let redos = stack.upcoming_redos(5);
        assert_eq!(redos.len(), 2);
    }

    #[test]
    fn test_undo_stack_max_size_accessor() {
        let stack = UndoStack::new(42);
        assert_eq!(stack.max_size(), 42);
    }

    #[test]
    fn test_undo_stack_auto_merge_accessor() {
        let mut stack = UndoStack::new(10);
        assert!(stack.is_auto_merge_enabled());

        stack.set_auto_merge(false);
        assert!(!stack.is_auto_merge_enabled());
    }

    // ====================================================================
    // Mutation-resistant: default try_merge returns false
    // ====================================================================

    /// Verify that the default `EditorCommand::try_merge` returns `false`.
    ///
    /// Mutation target: `command.rs:98:9: replace EditorCommand::try_merge -> bool with true`
    /// If the default returned `true`, non-mergeable commands would silently
    /// be discarded from the undo stack when auto_merge is enabled.
    #[test]
    fn test_default_try_merge_returns_false_exactly() {
        let mut world = World::new();
        let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        // EditHealthCommand uses the default try_merge (no override).
        let mut cmd1 = EditHealthCommand::new(entity, 100, 75);
        let cmd2 = EditHealthCommand::new(entity, 75, 50);

        // The default must return exactly false.
        assert_eq!(
            cmd1.try_merge(cmd2.as_ref()),
            false,
            "default EditorCommand::try_merge must return false, not true"
        );
    }

    /// Verify that non-mergeable commands stay separate in the undo stack
    /// even when auto_merge is enabled. This is the observable behavioral
    /// consequence of the default `try_merge` returning `false`.
    #[test]
    fn test_non_mergeable_commands_not_merged_with_auto_merge() {
        let mut world = World::new();
        let entity = world.spawn("Patient", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

        let mut stack = UndoStack::new(10);
        stack.set_auto_merge(true); // auto_merge ON

        // Push two EditHealthCommands (uses default try_merge -> false).
        stack.push_executed(EditHealthCommand::new(entity, 100, 75));
        stack.push_executed(EditHealthCommand::new(entity, 75, 50));

        // Both commands must remain in the stack — NOT be merged.
        assert_eq!(
            stack.len(),
            2,
            "non-mergeable commands must remain separate even with auto_merge enabled"
        );

        // Verify undo of each command is independent.
        stack.undo(&mut world).unwrap();
        assert_eq!(world.health(entity).unwrap().hp, 75);
        stack.undo(&mut world).unwrap();
        assert_eq!(world.health(entity).unwrap().hp, 100);
    }
}

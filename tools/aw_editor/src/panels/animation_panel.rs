//! Animation Panel for the editor UI
//!
//! Provides comprehensive animation editing:
//! - Animation clips and timelines
//! - Blend trees and state machines
//! - Animation layers and masking
//! - IK and procedural animation
//! - Animation events and curves

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// Animation clip
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub id: u32,
    pub name: String,
    pub duration: f32,
    pub fps: f32,
    pub loop_mode: LoopMode,
    pub events: Vec<AnimationEvent>,
    pub curves: Vec<AnimationCurve>,
}

impl Default for AnimationClip {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Clip".to_string(),
            duration: 1.0,
            fps: 30.0,
            loop_mode: LoopMode::Loop,
            events: Vec::new(),
            curves: Vec::new(),
        }
    }
}

/// Loop mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LoopMode {
    Once,
    #[default]
    Loop,
    PingPong,
    ClampForever,
}

impl std::fmt::Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LoopMode {
    pub fn all() -> &'static [LoopMode] {
        &[
            LoopMode::Once,
            LoopMode::Loop,
            LoopMode::PingPong,
            LoopMode::ClampForever,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LoopMode::Once => "Once",
            LoopMode::Loop => "Loop",
            LoopMode::PingPong => "Ping-Pong",
            LoopMode::ClampForever => "Clamp Forever",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LoopMode::Once => ">",
            LoopMode::Loop => "🔁",
            LoopMode::PingPong => "[Shuf]",
            LoopMode::ClampForever => "||",
        }
    }

    pub fn is_looping(&self) -> bool {
        matches!(self, LoopMode::Loop | LoopMode::PingPong)
    }

    pub fn description(&self) -> &'static str {
        match self {
            LoopMode::Once => "Play once and stop",
            LoopMode::Loop => "Loop continuously",
            LoopMode::PingPong => "Play forward then backward",
            LoopMode::ClampForever => "Hold last frame forever",
        }
    }
}

/// Animation event
#[derive(Debug, Clone)]
pub struct AnimationEvent {
    pub id: u32,
    pub time: f32,
    pub name: String,
    pub function: String,
    pub parameter: String,
}

/// Animation curve
#[derive(Debug, Clone)]
pub struct AnimationCurve {
    pub id: u32,
    pub property_path: String,
    pub keyframes: Vec<Keyframe>,
}

/// Keyframe
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub in_tangent: f32,
    pub out_tangent: f32,
    pub tangent_mode: TangentMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TangentMode {
    #[default]
    Auto,
    Linear,
    Constant,
    Free,
    Broken,
}

impl std::fmt::Display for TangentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TangentMode {
    pub fn all() -> &'static [TangentMode] {
        &[
            TangentMode::Auto,
            TangentMode::Linear,
            TangentMode::Constant,
            TangentMode::Free,
            TangentMode::Broken,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            TangentMode::Auto => "Auto",
            TangentMode::Linear => "Linear",
            TangentMode::Constant => "Constant",
            TangentMode::Free => "Free",
            TangentMode::Broken => "Broken",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TangentMode::Auto => "[Sync]",
            TangentMode::Linear => "📈",
            TangentMode::Constant => "||",
            TangentMode::Free => "✏️",
            TangentMode::Broken => "💔",
        }
    }

    pub fn is_smooth(&self) -> bool {
        matches!(self, TangentMode::Auto | TangentMode::Free)
    }

    pub fn is_editable(&self) -> bool {
        matches!(self, TangentMode::Free | TangentMode::Broken)
    }
}

/// Animation state in a state machine
#[derive(Debug, Clone)]
pub struct AnimationState {
    pub id: u32,
    pub name: String,
    pub clip_id: Option<u32>,
    pub speed: f32,
    pub position: [f32; 2],
    pub is_default: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New State".to_string(),
            clip_id: None,
            speed: 1.0,
            position: [100.0, 100.0],
            is_default: false,
        }
    }
}

/// State transition
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub id: u32,
    pub from_state: u32,
    pub to_state: u32,
    pub duration: f32,
    pub offset: f32,
    pub has_exit_time: bool,
    pub exit_time: f32,
    pub conditions: Vec<TransitionCondition>,
}

impl Default for StateTransition {
    fn default() -> Self {
        Self {
            id: 0,
            from_state: 0,
            to_state: 0,
            duration: 0.25,
            offset: 0.0,
            has_exit_time: false,
            exit_time: 1.0,
            conditions: Vec::new(),
        }
    }
}

/// Transition condition
#[derive(Debug, Clone)]
pub struct TransitionCondition {
    pub parameter: String,
    pub condition_type: ConditionType,
    pub threshold: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConditionType {
    #[default]
    Greater,
    Less,
    Equals,
    NotEquals,
    True,
    False,
}

impl std::fmt::Display for ConditionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.symbol(), self.name())
    }
}

impl ConditionType {
    pub fn all() -> &'static [ConditionType] {
        &[
            ConditionType::Greater,
            ConditionType::Less,
            ConditionType::Equals,
            ConditionType::NotEquals,
            ConditionType::True,
            ConditionType::False,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ConditionType::Greater => "Greater",
            ConditionType::Less => "Less",
            ConditionType::Equals => "Equals",
            ConditionType::NotEquals => "Not Equals",
            ConditionType::True => "True",
            ConditionType::False => "False",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            ConditionType::Greater => ">",
            ConditionType::Less => "<",
            ConditionType::Equals => "=",
            ConditionType::NotEquals => "≠",
            ConditionType::True => "✓",
            ConditionType::False => "✗",
        }
    }

    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            ConditionType::Greater
                | ConditionType::Less
                | ConditionType::Equals
                | ConditionType::NotEquals
        )
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, ConditionType::True | ConditionType::False)
    }
}

/// Animation parameter
#[derive(Debug, Clone)]
pub struct AnimationParameter {
    pub id: u32,
    pub name: String,
    pub param_type: ParameterType,
    pub default_value: ParameterValue,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParameterType {
    #[default]
    Float,
    Int,
    Bool,
    Trigger,
}

impl std::fmt::Display for ParameterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ParameterType {
    pub fn all() -> &'static [ParameterType] {
        &[
            ParameterType::Float,
            ParameterType::Int,
            ParameterType::Bool,
            ParameterType::Trigger,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParameterType::Float => "Float",
            ParameterType::Int => "Int",
            ParameterType::Bool => "Bool",
            ParameterType::Trigger => "Trigger",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ParameterType::Float => "🔢",
            ParameterType::Int => "#️⃣",
            ParameterType::Bool => "☑️",
            ParameterType::Trigger => "[Zap]",
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, ParameterType::Float | ParameterType::Int)
    }

    pub fn is_instant(&self) -> bool {
        matches!(self, ParameterType::Trigger)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParameterValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Trigger,
}

impl std::fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ParameterValue {
    /// Returns all parameter type variant names
    pub fn all_variants() -> &'static [&'static str] {
        &["Float", "Int", "Bool", "Trigger"]
    }

    /// Returns the name of this parameter value type
    pub fn name(&self) -> &'static str {
        match self {
            ParameterValue::Float(_) => "Float",
            ParameterValue::Int(_) => "Int",
            ParameterValue::Bool(_) => "Bool",
            ParameterValue::Trigger => "Trigger",
        }
    }

    /// Returns the icon for this parameter value type
    pub fn icon(&self) -> &'static str {
        match self {
            ParameterValue::Float(_) => "🔢",
            ParameterValue::Int(_) => "🔟",
            ParameterValue::Bool(_) => "✓",
            ParameterValue::Trigger => "[Zap]",
        }
    }

    /// Returns true if this is a numeric value (Float or Int)
    pub fn is_numeric(&self) -> bool {
        matches!(self, ParameterValue::Float(_) | ParameterValue::Int(_))
    }

    /// Returns true if this is a boolean value
    pub fn is_bool(&self) -> bool {
        matches!(self, ParameterValue::Bool(_))
    }

    /// Returns true if this is a trigger
    pub fn is_trigger(&self) -> bool {
        matches!(self, ParameterValue::Trigger)
    }

    /// Returns the float value if this is a Float variant
    pub fn as_float(&self) -> Option<f32> {
        match self {
            ParameterValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the int value if this is an Int variant
    pub fn as_int(&self) -> Option<i32> {
        match self {
            ParameterValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the bool value if this is a Bool variant
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

impl Default for ParameterValue {
    fn default() -> Self {
        ParameterValue::Float(0.0)
    }
}

/// Blend tree node type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendTreeType {
    #[default]
    Simple1D,
    Simple2D,
    FreeformDirectional,
    FreeformCartesian,
    Direct,
}

impl std::fmt::Display for BlendTreeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BlendTreeType {
    pub fn all() -> &'static [BlendTreeType] {
        &[
            BlendTreeType::Simple1D,
            BlendTreeType::Simple2D,
            BlendTreeType::FreeformDirectional,
            BlendTreeType::FreeformCartesian,
            BlendTreeType::Direct,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlendTreeType::Simple1D => "Simple 1D",
            BlendTreeType::Simple2D => "Simple 2D",
            BlendTreeType::FreeformDirectional => "Freeform Directional",
            BlendTreeType::FreeformCartesian => "Freeform Cartesian",
            BlendTreeType::Direct => "Direct",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BlendTreeType::Simple1D => "↔️",
            BlendTreeType::Simple2D => "[Shuf]",
            BlendTreeType::FreeformDirectional => "[Comp]",
            BlendTreeType::FreeformCartesian => "[Chart]",
            BlendTreeType::Direct => ">",
        }
    }

    pub fn dimensions(&self) -> u8 {
        match self {
            BlendTreeType::Simple1D | BlendTreeType::Direct => 1,
            BlendTreeType::Simple2D
            | BlendTreeType::FreeformDirectional
            | BlendTreeType::FreeformCartesian => 2,
        }
    }

    pub fn is_freeform(&self) -> bool {
        matches!(
            self,
            BlendTreeType::FreeformDirectional | BlendTreeType::FreeformCartesian
        )
    }
}

/// Blend tree
#[derive(Debug, Clone)]
pub struct BlendTree {
    pub id: u32,
    pub name: String,
    pub blend_type: BlendTreeType,
    pub blend_parameter_x: String,
    pub blend_parameter_y: String,
    pub children: Vec<BlendTreeChild>,
}

impl Default for BlendTree {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Blend Tree".to_string(),
            blend_type: BlendTreeType::Simple1D,
            blend_parameter_x: String::new(),
            blend_parameter_y: String::new(),
            children: Vec::new(),
        }
    }
}

/// Blend tree child
#[derive(Debug, Clone)]
pub struct BlendTreeChild {
    pub clip_id: u32,
    pub threshold: f32,
    pub position: [f32; 2],
    pub time_scale: f32,
}

/// Animation layer
#[derive(Debug, Clone)]
pub struct AnimationLayer {
    pub id: u32,
    pub name: String,
    pub weight: f32,
    pub blending_mode: BlendingMode,
    pub avatar_mask: Option<String>,
    pub ik_pass: bool,
}

impl Default for AnimationLayer {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Layer".to_string(),
            weight: 1.0,
            blending_mode: BlendingMode::Override,
            avatar_mask: None,
            ik_pass: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendingMode {
    #[default]
    Override,
    Additive,
}

impl std::fmt::Display for BlendingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BlendingMode {
    pub fn all() -> &'static [BlendingMode] {
        &[BlendingMode::Override, BlendingMode::Additive]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlendingMode::Override => "Override",
            BlendingMode::Additive => "Additive",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BlendingMode::Override => "[Sync]",
            BlendingMode::Additive => "+",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BlendingMode::Override => "Replace base layer animation",
            BlendingMode::Additive => "Add to base layer animation",
        }
    }
}

/// IK settings
#[derive(Debug, Clone)]
pub struct IkSettings {
    pub enabled: bool,

    // Foot IK
    pub foot_ik_enabled: bool,
    pub foot_height_offset: f32,
    pub foot_weight: f32,

    // Look At IK
    pub look_at_enabled: bool,
    pub look_at_weight: f32,
    pub head_weight: f32,
    pub body_weight: f32,
    pub eyes_weight: f32,
    pub clamp_weight: f32,

    // Hand IK
    pub left_hand_ik_enabled: bool,
    pub right_hand_ik_enabled: bool,
    pub hand_ik_weight: f32,
}

impl Default for IkSettings {
    fn default() -> Self {
        Self {
            enabled: false,

            foot_ik_enabled: true,
            foot_height_offset: 0.0,
            foot_weight: 1.0,

            look_at_enabled: false,
            look_at_weight: 1.0,
            head_weight: 1.0,
            body_weight: 0.4,
            eyes_weight: 1.0,
            clamp_weight: 0.5,

            left_hand_ik_enabled: false,
            right_hand_ik_enabled: false,
            hand_ik_weight: 1.0,
        }
    }
}

/// Animation controller (state machine)
#[derive(Debug, Clone)]
pub struct AnimationController {
    pub id: u32,
    pub name: String,
    pub states: Vec<AnimationState>,
    pub transitions: Vec<StateTransition>,
    pub parameters: Vec<AnimationParameter>,
    pub layers: Vec<AnimationLayer>,
    pub blend_trees: Vec<BlendTree>,
    pub ik_settings: IkSettings,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Controller".to_string(),
            states: Vec::new(),
            transitions: Vec::new(),
            parameters: Vec::new(),
            layers: vec![AnimationLayer::default()],
            blend_trees: Vec::new(),
            ik_settings: IkSettings::default(),
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnimationTab {
    #[default]
    Clips,
    StateMachine,
    BlendTrees,
    Layers,
    Parameters,
    IK,
    Preview,
}

impl std::fmt::Display for AnimationTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AnimationTab {
    pub fn all() -> &'static [AnimationTab] {
        &[
            AnimationTab::Clips,
            AnimationTab::StateMachine,
            AnimationTab::BlendTrees,
            AnimationTab::Layers,
            AnimationTab::Parameters,
            AnimationTab::IK,
            AnimationTab::Preview,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            AnimationTab::Clips => "Clips",
            AnimationTab::StateMachine => "State Machine",
            AnimationTab::BlendTrees => "Blend Trees",
            AnimationTab::Layers => "Layers",
            AnimationTab::Parameters => "Parameters",
            AnimationTab::IK => "IK",
            AnimationTab::Preview => "Preview",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AnimationTab::Clips => "[Anim]",
            AnimationTab::StateMachine => "[Shuf]",
            AnimationTab::BlendTrees => "[Tree]",
            AnimationTab::Layers => "📚",
            AnimationTab::Parameters => "⚙️",
            AnimationTab::IK => "🦴",
            AnimationTab::Preview => "👁️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AnimationTab::Clips => "Edit animation clips and keyframes",
            AnimationTab::StateMachine => "Design animation state machines",
            AnimationTab::BlendTrees => "Configure animation blend trees",
            AnimationTab::Layers => "Manage animation layers",
            AnimationTab::Parameters => "Define animation parameters",
            AnimationTab::IK => "Configure inverse kinematics",
            AnimationTab::Preview => "Preview animations in real-time",
        }
    }
}

/// External action events emitted by the animation panel.
/// These represent high-level user actions that external systems can respond to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AnimationAction {
    /// Play animation clip
    PlayClip { clip_id: u32 },
    /// Pause animation playback
    PauseClip,
    /// Stop animation and reset
    StopClip,
    /// Set playback speed
    SetSpeed { speed: f32 },
    /// Seek to time position
    SeekTo { time: f32 },
    /// Create new animation clip
    CreateClip { name: String },
    /// Delete animation clip
    DeleteClip { clip_id: u32 },
    /// Save animation clip
    SaveClip { clip_id: u32 },
    /// Add keyframe at current time
    AddKeyframe { clip_id: u32, time: f32 },
    /// Delete keyframe
    DeleteKeyframe { clip_id: u32, keyframe_index: usize },
    /// Create animation controller
    CreateController { name: String },
    /// Set loop mode for clip
    SetLoopMode { clip_id: u32, loop_mode: LoopMode },
    /// Trigger state transition
    TriggerTransition { from_state: u32, to_state: u32 },
}

impl std::fmt::Display for AnimationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl AnimationAction {
    /// Returns the name of this action
    pub fn name(&self) -> &'static str {
        match self {
            AnimationAction::PlayClip { .. } => "Play Clip",
            AnimationAction::PauseClip => "Pause Clip",
            AnimationAction::StopClip => "Stop Clip",
            AnimationAction::SetSpeed { .. } => "Set Speed",
            AnimationAction::SeekTo { .. } => "Seek To",
            AnimationAction::CreateClip { .. } => "Create Clip",
            AnimationAction::DeleteClip { .. } => "Delete Clip",
            AnimationAction::SaveClip { .. } => "Save Clip",
            AnimationAction::AddKeyframe { .. } => "Add Keyframe",
            AnimationAction::DeleteKeyframe { .. } => "Delete Keyframe",
            AnimationAction::CreateController { .. } => "Create Controller",
            AnimationAction::SetLoopMode { .. } => "Set Loop Mode",
            AnimationAction::TriggerTransition { .. } => "Trigger Transition",
        }
    }

    /// Returns true if this is a playback control action
    pub fn is_playback(&self) -> bool {
        matches!(
            self,
            AnimationAction::PlayClip { .. }
                | AnimationAction::PauseClip
                | AnimationAction::StopClip
                | AnimationAction::SetSpeed { .. }
                | AnimationAction::SeekTo { .. }
        )
    }

    /// Returns true if this is a clip editing action
    pub fn is_clip_edit(&self) -> bool {
        matches!(
            self,
            AnimationAction::CreateClip { .. }
                | AnimationAction::DeleteClip { .. }
                | AnimationAction::SaveClip { .. }
                | AnimationAction::AddKeyframe { .. }
                | AnimationAction::DeleteKeyframe { .. }
                | AnimationAction::SetLoopMode { .. }
        )
    }
}

/// Main Animation Panel
pub struct AnimationPanel {
    // Tab state
    active_tab: AnimationTab,

    // Clips
    clips: Vec<AnimationClip>,
    selected_clip: Option<u32>,
    current_clip: AnimationClip,

    // Controller
    controllers: Vec<AnimationController>,
    selected_controller: Option<u32>,
    current_controller: AnimationController,

    // Preview
    preview_playing: bool,
    preview_time: f32,
    preview_speed: f32,

    // Timeline
    timeline_zoom: f32,
    timeline_scroll: f32,

    // Keyframe editing
    selected_curve: Option<u32>,
    selected_keyframe: Option<usize>,
    dragging_keyframe: bool,

    // State machine editor
    selected_state: Option<u32>,
    selected_transition: Option<u32>,

    // ID counters
    next_clip_id: u32,
    next_controller_id: u32,
    next_state_id: u32,
    next_transition_id: u32,
    next_parameter_id: u32,

    // Pending actions for external event handling
    pending_actions: Vec<AnimationAction>,
}

impl Default for AnimationPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: AnimationTab::Clips,

            clips: Vec::new(),
            selected_clip: None,
            current_clip: AnimationClip::default(),

            controllers: Vec::new(),
            selected_controller: None,
            current_controller: AnimationController::default(),

            preview_playing: false,
            preview_time: 0.0,
            preview_speed: 1.0,

            timeline_zoom: 1.0,
            timeline_scroll: 0.0,

            selected_curve: None,
            selected_keyframe: None,
            dragging_keyframe: false,

            selected_state: None,
            selected_transition: None,

            next_clip_id: 1,
            next_controller_id: 1,
            next_state_id: 1,
            next_transition_id: 1,
            next_parameter_id: 1,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl AnimationPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions, leaving the queue empty.
    /// External systems should call this each frame to retrieve actions.
    pub fn take_actions(&mut self) -> Vec<AnimationAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions to process.
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external handling.
    fn queue_action(&mut self, action: AnimationAction) {
        info!(target: "aw_editor::panels::animation_panel", action = %action, "Animation action queued");
        self.pending_actions.push(action);
    }

    /// Returns the preview speed.
    pub fn preview_speed(&self) -> f32 {
        self.preview_speed
    }

    /// Returns true if preview is playing.
    pub fn is_playing(&self) -> bool {
        self.preview_playing
    }

    /// Returns the current preview time.
    pub fn preview_time(&self) -> f32 {
        self.preview_time
    }

    fn create_sample_data(&mut self) {
        // Sample clips
        let clips = [
            ("Idle", 2.0),
            ("Walk", 1.0),
            ("Run", 0.6),
            ("Jump", 0.8),
            ("Attack", 0.5),
            ("Death", 2.0),
        ];

        for (name, duration) in clips {
            let id = self.next_clip_id();
            self.clips.push(AnimationClip {
                id,
                name: name.to_string(),
                duration,
                ..Default::default()
            });
        }

        // Sample controller
        let controller_id = self.next_controller_id();
        let mut controller = AnimationController {
            id: controller_id,
            name: "Character Controller".to_string(),
            ..Default::default()
        };

        // Add states
        let idle_id = self.next_state_id();
        controller.states.push(AnimationState {
            id: idle_id,
            name: "Idle".to_string(),
            clip_id: Some(1),
            position: [100.0, 150.0],
            is_default: true,
            ..Default::default()
        });

        let walk_id = self.next_state_id();
        controller.states.push(AnimationState {
            id: walk_id,
            name: "Walk".to_string(),
            clip_id: Some(2),
            position: [300.0, 100.0],
            ..Default::default()
        });

        let run_id = self.next_state_id();
        controller.states.push(AnimationState {
            id: run_id,
            name: "Run".to_string(),
            clip_id: Some(3),
            position: [300.0, 200.0],
            ..Default::default()
        });

        // Add transitions
        let trans_id = self.next_transition_id();
        controller.transitions.push(StateTransition {
            id: trans_id,
            from_state: idle_id,
            to_state: walk_id,
            duration: 0.2,
            conditions: vec![TransitionCondition {
                parameter: "Speed".to_string(),
                condition_type: ConditionType::Greater,
                threshold: 0.1,
            }],
            ..Default::default()
        });

        let trans_id = self.next_transition_id();
        controller.transitions.push(StateTransition {
            id: trans_id,
            from_state: walk_id,
            to_state: run_id,
            duration: 0.2,
            conditions: vec![TransitionCondition {
                parameter: "Speed".to_string(),
                condition_type: ConditionType::Greater,
                threshold: 0.5,
            }],
            ..Default::default()
        });

        // Add parameters
        let param_id = self.next_parameter_id();
        controller.parameters.push(AnimationParameter {
            id: param_id,
            name: "Speed".to_string(),
            param_type: ParameterType::Float,
            default_value: ParameterValue::Float(0.0),
        });

        let param_id = self.next_parameter_id();
        controller.parameters.push(AnimationParameter {
            id: param_id,
            name: "IsGrounded".to_string(),
            param_type: ParameterType::Bool,
            default_value: ParameterValue::Bool(true),
        });

        let param_id = self.next_parameter_id();
        controller.parameters.push(AnimationParameter {
            id: param_id,
            name: "Jump".to_string(),
            param_type: ParameterType::Trigger,
            default_value: ParameterValue::Trigger,
        });

        self.controllers.push(controller.clone());
        self.current_controller = controller;
        self.selected_controller = Some(controller_id);

        if !self.clips.is_empty() {
            self.current_clip = self.clips[0].clone();
            self.selected_clip = Some(self.clips[0].id);
        }
    }

    fn next_clip_id(&mut self) -> u32 {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        id
    }

    fn next_controller_id(&mut self) -> u32 {
        let id = self.next_controller_id;
        self.next_controller_id += 1;
        id
    }

    fn next_state_id(&mut self) -> u32 {
        let id = self.next_state_id;
        self.next_state_id += 1;
        id
    }

    fn next_transition_id(&mut self) -> u32 {
        let id = self.next_transition_id;
        self.next_transition_id += 1;
        id
    }

    fn next_parameter_id(&mut self) -> u32 {
        let id = self.next_parameter_id;
        self.next_parameter_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (AnimationTab::Clips, "Clips"),
                (AnimationTab::StateMachine, "States"),
                (AnimationTab::BlendTrees, "Blend Trees"),
                (AnimationTab::Layers, "📚 Layers"),
                (AnimationTab::Parameters, "Parameters"),
                (AnimationTab::IK, "🦴 IK"),
                (AnimationTab::Preview, "> Preview"),
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

        // Info bar
        ui.horizontal(|ui| {
            ui.label(format!("{} clips", self.clips.len()));
            ui.separator();
            ui.label(format!("{} states", self.current_controller.states.len()));
            ui.separator();
            ui.label(format!(
                "{} params",
                self.current_controller.parameters.len()
            ));
        });

        ui.separator();
    }

    fn show_clips_tab(&mut self, ui: &mut Ui) {
        ui.heading("Animation Clips");
        ui.add_space(10.0);

        // Clip selector
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("clip_select")
                .selected_text(&self.current_clip.name)
                .show_ui(ui, |ui| {
                    for clip in &self.clips.clone() {
                        if ui
                            .selectable_value(&mut self.selected_clip, Some(clip.id), &clip.name)
                            .clicked()
                        {
                            self.current_clip = clip.clone();
                        }
                    }
                });

            if ui.button("+ New").clicked() {
                let id = self.next_clip_id();
                info!(target: "aw_editor::panels::animation_panel", clip_id = id, "New animation clip created");
                let new_clip = AnimationClip {
                    id,
                    name: format!("Clip {}", id),
                    ..Default::default()
                };
                self.clips.push(new_clip.clone());
                self.current_clip = new_clip;
                self.selected_clip = Some(id);
            }

            if ui.button("Duplicate").clicked() {
                let id = self.next_clip_id();
                info!(target: "aw_editor::panels::animation_panel", clip_id = id, source = %self.current_clip.name, "Animation clip duplicated");
                let mut dup = self.current_clip.clone();
                dup.id = id;
                dup.name = format!("{} (Copy)", dup.name);
                self.clips.push(dup);
            }
        });

        ui.add_space(10.0);

        // Clip properties
        ui.group(|ui| {
            ui.label(RichText::new("[Edit] Properties").strong());

            egui::Grid::new("clip_props")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.current_clip.name);
                    ui.end_row();

                    ui.label("Duration:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_clip.duration)
                            .speed(0.1)
                            .suffix("s"),
                    );
                    ui.end_row();

                    ui.label("FPS:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_clip.fps)
                            .speed(1.0)
                            .range(1.0..=120.0),
                    );
                    ui.end_row();

                    ui.label("Loop Mode:");
                    egui::ComboBox::from_id_salt("loop_mode")
                        .selected_text(format!("{:?}", self.current_clip.loop_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.current_clip.loop_mode,
                                LoopMode::Once,
                                "Once",
                            );
                            ui.selectable_value(
                                &mut self.current_clip.loop_mode,
                                LoopMode::Loop,
                                "Loop",
                            );
                            ui.selectable_value(
                                &mut self.current_clip.loop_mode,
                                LoopMode::PingPong,
                                "Ping Pong",
                            );
                            ui.selectable_value(
                                &mut self.current_clip.loop_mode,
                                LoopMode::ClampForever,
                                "Clamp Forever",
                            );
                        });
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Simple timeline
        ui.group(|ui| {
            ui.label(RichText::new("⏱ Timeline").strong());

            // Playback controls
            ui.horizontal(|ui| {
                if ui
                    .button(if self.preview_playing { "⏸" } else { "▶" })
                    .clicked()
                {
                    self.preview_playing = !self.preview_playing;
                    info!(target: "aw_editor::panels::animation_panel", playing = self.preview_playing, "Timeline playback toggled");
                }
                if ui.button("⏹").clicked() {
                    info!(target: "aw_editor::panels::animation_panel", "Timeline playback stopped");
                    self.preview_playing = false;
                    self.preview_time = 0.0;
                }
                if ui.button("⏮").clicked() {
                    self.preview_time = 0.0;
                }
                if ui.button("⏭").clicked() {
                    self.preview_time = self.current_clip.duration;
                }

                ui.add(
                    egui::Slider::new(&mut self.preview_time, 0.0..=self.current_clip.duration)
                        .suffix("s"),
                );

                ui.label(format!("Speed: {:.1}x", self.preview_speed));
                ui.add(
                    egui::DragValue::new(&mut self.preview_speed)
                        .speed(0.1)
                        .range(0.1..=3.0),
                );
            });

            // === Enhanced timeline visualization ===
            let ruler_height = 20.0;
            let track_height = 24.0;
            let num_curves = self.current_clip.curves.len();
            let tracks_height = if num_curves > 0 {
                num_curves as f32 * track_height
            } else {
                track_height // minimum 1 track
            };
            let total_height = ruler_height + tracks_height;
            let timeline_width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(timeline_width, total_height),
                egui::Sense::click_and_drag(),
            );

            let painter = ui.painter();
            painter.rect_filled(rect, 4.0, Color32::from_rgb(40, 40, 45));

            let duration = self.current_clip.duration;
            if duration > 0.0 {
                // --- Ruler area ---
                let ruler_rect =
                    egui::Rect::from_min_size(rect.min, Vec2::new(timeline_width, ruler_height));
                painter.rect_filled(ruler_rect, 0.0, Color32::from_rgb(30, 30, 35));

                // Time markers on ruler
                let steps = 10;
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let x = rect.min.x + t * timeline_width;
                    let tick_h = if i % 5 == 0 { 12.0 } else { 6.0 };
                    painter.line_segment(
                        [
                            egui::Pos2::new(x, ruler_rect.min.y),
                            egui::Pos2::new(x, ruler_rect.min.y + tick_h),
                        ],
                        egui::Stroke::new(1.0, Color32::GRAY),
                    );
                    if i % 5 == 0 {
                        painter.text(
                            egui::Pos2::new(x + 2.0, ruler_rect.min.y + 8.0),
                            egui::Align2::LEFT_TOP,
                            format!("{:.1}s", t * duration),
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(140, 140, 140),
                        );
                    }
                }

                // --- Track lanes ---
                let tracks_top = rect.min.y + ruler_height;
                let track_colors = [
                    Color32::from_rgb(80, 140, 220),  // blue
                    Color32::from_rgb(100, 190, 100), // green
                    Color32::from_rgb(220, 160, 60),  // orange
                    Color32::from_rgb(180, 100, 200), // purple
                    Color32::from_rgb(200, 80, 80),   // red
                ];

                // Draw alternating track rows
                for track_idx in 0..num_curves.max(1) {
                    let track_y = tracks_top + track_idx as f32 * track_height;
                    let bg = if track_idx % 2 == 0 {
                        Color32::from_rgb(35, 35, 40)
                    } else {
                        Color32::from_rgb(42, 42, 47)
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::Pos2::new(rect.min.x, track_y),
                            Vec2::new(timeline_width, track_height),
                        ),
                        0.0,
                        bg,
                    );
                }

                // Draw track labels and keyframe diamonds
                let diamond_size = 5.0;
                for (curve_idx, curve) in self.current_clip.curves.iter().enumerate() {
                    let track_y = tracks_top + curve_idx as f32 * track_height;
                    let center_y = track_y + track_height / 2.0;
                    let color = track_colors[curve_idx % track_colors.len()];

                    // Track label
                    let label = if curve.property_path.len() > 12 {
                        &curve.property_path[curve.property_path.len() - 12..]
                    } else {
                        &curve.property_path
                    };
                    painter.text(
                        egui::Pos2::new(rect.min.x + 4.0, center_y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(150, 150, 150),
                    );

                    // Draw connecting line between keyframes
                    if curve.keyframes.len() >= 2 {
                        for pair in curve.keyframes.windows(2) {
                            let x1 = rect.min.x + (pair[0].time / duration) * timeline_width;
                            let x2 = rect.min.x + (pair[1].time / duration) * timeline_width;
                            painter.line_segment(
                                [egui::Pos2::new(x1, center_y), egui::Pos2::new(x2, center_y)],
                                egui::Stroke::new(1.0, color.linear_multiply(0.5)),
                            );
                        }
                    }

                    // Draw keyframe diamonds
                    for (kf_idx, kf) in curve.keyframes.iter().enumerate() {
                        let kf_x = rect.min.x + (kf.time / duration) * timeline_width;
                        let is_selected = self.selected_curve == Some(curve.id)
                            && self.selected_keyframe == Some(kf_idx);

                        // Diamond shape: 4 points rotated 45°
                        let fill = if is_selected { Color32::WHITE } else { color };
                        let points = vec![
                            egui::Pos2::new(kf_x, center_y - diamond_size),
                            egui::Pos2::new(kf_x + diamond_size, center_y),
                            egui::Pos2::new(kf_x, center_y + diamond_size),
                            egui::Pos2::new(kf_x - diamond_size, center_y),
                        ];
                        painter.add(egui::Shape::convex_polygon(
                            points,
                            fill,
                            egui::Stroke::new(
                                if is_selected { 2.0 } else { 1.0 },
                                if is_selected {
                                    Color32::YELLOW
                                } else {
                                    Color32::from_rgb(200, 200, 200)
                                },
                            ),
                        ));
                    }
                }

                // --- Playhead ---
                let playhead_x = rect.min.x + (self.preview_time / duration) * timeline_width;
                painter.line_segment(
                    [
                        egui::Pos2::new(playhead_x, rect.min.y),
                        egui::Pos2::new(playhead_x, rect.max.y),
                    ],
                    egui::Stroke::new(2.0, Color32::RED),
                );
                // Playhead handle (triangle)
                let tri_size = 6.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        egui::Pos2::new(playhead_x - tri_size, rect.min.y),
                        egui::Pos2::new(playhead_x + tri_size, rect.min.y),
                        egui::Pos2::new(playhead_x, rect.min.y + tri_size + 2.0),
                    ],
                    Color32::RED,
                    egui::Stroke::NONE,
                ));

                // --- Interaction ---
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let rel_x = (pointer_pos.x - rect.min.x).clamp(0.0, timeline_width);
                    let click_time = (rel_x / timeline_width) * duration;

                    if response.clicked() {
                        // Check if clicking on a keyframe diamond
                        let mut hit_kf = false;
                        let tracks_top = rect.min.y + ruler_height;
                        for (ci, curve) in self.current_clip.curves.iter().enumerate() {
                            let center_y =
                                tracks_top + ci as f32 * track_height + track_height / 2.0;
                            for (ki, kf) in curve.keyframes.iter().enumerate() {
                                let kf_x = rect.min.x + (kf.time / duration) * timeline_width;
                                let dist = ((pointer_pos.x - kf_x).powi(2)
                                    + (pointer_pos.y - center_y).powi(2))
                                .sqrt();
                                if dist <= diamond_size + 3.0 {
                                    self.selected_curve = Some(curve.id);
                                    self.selected_keyframe = Some(ki);
                                    self.dragging_keyframe = true;
                                    hit_kf = true;
                                    break;
                                }
                            }
                            if hit_kf {
                                break;
                            }
                        }

                        if !hit_kf {
                            // Click on ruler = seek; click on track = deselect
                            if pointer_pos.y < tracks_top {
                                self.preview_time = click_time;
                            }
                            self.selected_curve = None;
                            self.selected_keyframe = None;
                            self.dragging_keyframe = false;
                        }
                    }

                    // Drag to seek on ruler, or drag selected keyframe
                    if response.dragged() {
                        if self.dragging_keyframe {
                            if let (Some(curve_id), Some(kf_idx)) =
                                (self.selected_curve, self.selected_keyframe)
                            {
                                if let Some(curve) = self
                                    .current_clip
                                    .curves
                                    .iter_mut()
                                    .find(|c| c.id == curve_id)
                                {
                                    if let Some(kf) = curve.keyframes.get_mut(kf_idx) {
                                        kf.time = click_time.clamp(0.0, duration);
                                    }
                                }
                            }
                        } else if pointer_pos.y < rect.min.y + ruler_height {
                            self.preview_time = click_time;
                        }
                    }
                }

                if response.drag_stopped() {
                    self.dragging_keyframe = false;
                }

                // Right-click: delete selected keyframe
                if response.secondary_clicked() {
                    if let (Some(curve_id), Some(kf_idx)) =
                        (self.selected_curve, self.selected_keyframe)
                    {
                        if let Some(curve) = self
                            .current_clip
                            .curves
                            .iter_mut()
                            .find(|c| c.id == curve_id)
                        {
                            if kf_idx < curve.keyframes.len() {
                                curve.keyframes.remove(kf_idx);
                                self.queue_action(AnimationAction::DeleteKeyframe {
                                    clip_id: self.current_clip.id,
                                    keyframe_index: kf_idx,
                                });
                            }
                        }
                        self.selected_keyframe = None;
                        self.selected_curve = None;
                    }
                }

                // Double-click on track: add keyframe
                if response.double_clicked() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let tracks_top = rect.min.y + ruler_height;
                        if pointer_pos.y >= tracks_top {
                            let track_idx = ((pointer_pos.y - tracks_top) / track_height) as usize;
                            if track_idx < self.current_clip.curves.len() {
                                let rel_x = (pointer_pos.x - rect.min.x).clamp(0.0, timeline_width);
                                let t = (rel_x / timeline_width) * duration;
                                self.current_clip.curves[track_idx]
                                    .keyframes
                                    .push(Keyframe {
                                        time: t,
                                        value: 0.0,
                                        in_tangent: 0.0,
                                        out_tangent: 0.0,
                                        tangent_mode: TangentMode::Auto,
                                    });
                                self.current_clip.curves[track_idx]
                                    .keyframes
                                    .sort_by(|a, b| {
                                        a.time
                                            .partial_cmp(&b.time)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                let curve_id = self.current_clip.curves[track_idx].id;
                                self.queue_action(AnimationAction::AddKeyframe {
                                    clip_id: self.current_clip.id,
                                    time: t,
                                });
                                self.selected_curve = Some(curve_id);
                                let kf_idx = self.current_clip.curves[track_idx]
                                    .keyframes
                                    .iter()
                                    .position(|k| (k.time - t).abs() < 0.001)
                                    .unwrap_or(0);
                                self.selected_keyframe = Some(kf_idx);
                            }
                        }
                    }
                }
            } else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Set clip duration > 0",
                    egui::FontId::proportional(12.0),
                    Color32::GRAY,
                );
            }
        });

        ui.add_space(10.0);

        // Events
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Events").strong());
                if ui.button("+ Add Event").clicked() {
                    info!(target: "aw_editor::panels::animation_panel", time = self.preview_time, "Animation event added");
                    self.current_clip.events.push(AnimationEvent {
                        id: self.current_clip.events.len() as u32 + 1,
                        time: self.preview_time,
                        name: "NewEvent".to_string(),
                        function: "OnAnimationEvent".to_string(),
                        parameter: String::new(),
                    });
                }
            });

            if self.current_clip.events.is_empty() {
                ui.label("No events. Click '+ Add Event' to add one.");
            } else {
                for event in &self.current_clip.events {
                    ui.horizontal(|ui| {
                        ui.label(format!("@{:.2}s:", event.time));
                        ui.label(&event.name);
                        ui.label(RichText::new(&event.function).small().color(Color32::GRAY));
                    });
                }
            }
        });
    }

    fn show_state_machine_tab(&mut self, ui: &mut Ui) {
        ui.heading("State Machine");
        ui.add_space(10.0);

        // Controller selector
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("controller_select")
                .selected_text(&self.current_controller.name)
                .show_ui(ui, |ui| {
                    for ctrl in &self.controllers.clone() {
                        if ui
                            .selectable_value(
                                &mut self.selected_controller,
                                Some(ctrl.id),
                                &ctrl.name,
                            )
                            .clicked()
                        {
                            self.current_controller = ctrl.clone();
                        }
                    }
                });

            if ui.button("+ New").clicked() {
                let id = self.next_controller_id();
                info!(target: "aw_editor::panels::animation_panel", controller_id = id, "New animation controller created");
                let new_ctrl = AnimationController {
                    id,
                    name: format!("Controller {}", id),
                    ..Default::default()
                };
                self.controllers.push(new_ctrl.clone());
                self.current_controller = new_ctrl;
                self.selected_controller = Some(id);
            }
        });

        ui.add_space(10.0);

        // State list
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("States").strong());
                if ui.button("+ Add State").clicked() {
                    let id = self.next_state_id();
                    info!(target: "aw_editor::panels::animation_panel", state_id = id, "State added to animation controller");
                    self.current_controller.states.push(AnimationState {
                        id,
                        name: format!("State {}", id),
                        position: [
                            200.0,
                            100.0 + (self.current_controller.states.len() as f32) * 50.0,
                        ],
                        ..Default::default()
                    });
                }
            });

            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for state in &mut self.current_controller.states {
                        ui.horizontal(|ui| {
                            let is_selected = self.selected_state == Some(state.id);
                            let label = if state.is_default {
                                format!("[G] {}", state.name)
                            } else {
                                format!("   {}", state.name)
                            };

                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_state = Some(state.id);
                            }

                            // Clip assignment
                            egui::ComboBox::from_id_salt(format!("state_clip_{}", state.id))
                                .selected_text(state.clip_id.map_or("(None)".to_string(), |id| {
                                    self.clips
                                        .iter()
                                        .find(|c| c.id == id)
                                        .map_or("(None)".to_string(), |c| c.name.clone())
                                }))
                                .width(100.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut state.clip_id, None, "(None)")
                                        .clicked();
                                    for clip in &self.clips {
                                        ui.selectable_value(
                                            &mut state.clip_id,
                                            Some(clip.id),
                                            &clip.name,
                                        );
                                    }
                                });
                        });
                    }
                });
        });

        ui.add_space(10.0);

        // Transition list
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Transitions").strong());
                if ui.button("+ Add").clicked() && self.current_controller.states.len() >= 2 {
                    let from = self.current_controller.states[0].id;
                    let to = self.current_controller.states[1].id;
                    let id = self.next_transition_id();
                    info!(target: "aw_editor::panels::animation_panel", transition_id = id, from_state = from, to_state = to, "State transition added");
                    self.current_controller.transitions.push(StateTransition {
                        id,
                        from_state: from,
                        to_state: to,
                        ..Default::default()
                    });
                }
            });

            for trans in &self.current_controller.transitions {
                let from_name = self
                    .current_controller
                    .states
                    .iter()
                    .find(|s| s.id == trans.from_state)
                    .map_or("?", |s| &s.name);
                let to_name = self
                    .current_controller
                    .states
                    .iter()
                    .find(|s| s.id == trans.to_state)
                    .map_or("?", |s| &s.name);

                ui.horizontal(|ui| {
                    ui.label(format!("{} → {}", from_name, to_name));
                    ui.label(
                        RichText::new(format!("{:.2}s", trans.duration))
                            .small()
                            .color(Color32::GRAY),
                    );

                    if !trans.conditions.is_empty() {
                        ui.label(
                            RichText::new(format!("({} conditions)", trans.conditions.len()))
                                .small(),
                        );
                    }
                });
            }
        });
    }

    fn show_blend_trees_tab(&mut self, ui: &mut Ui) {
        ui.heading("Blend Trees");
        ui.add_space(10.0);

        // Blend tree list
        ui.horizontal(|ui| {
            if ui.button("+ New Blend Tree").clicked() {
                let id = self.current_controller.blend_trees.len() as u32 + 1;
                info!(target: "aw_editor::panels::animation_panel", blend_tree_id = id, "New blend tree created");
                self.current_controller.blend_trees.push(BlendTree {
                    id,
                    name: format!("Blend Tree {}", id),
                    ..Default::default()
                });
            }
        });

        ui.add_space(10.0);

        if self.current_controller.blend_trees.is_empty() {
            ui.label("No blend trees. Click '+ New Blend Tree' to create one.");
        } else {
            for tree in &mut self.current_controller.blend_trees {
                ui.group(|ui| {
                    ui.label(RichText::new(&tree.name).strong());

                    egui::Grid::new(format!("blend_tree_{}", tree.id))
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Type:");
                            egui::ComboBox::from_id_salt(format!("blend_type_{}", tree.id))
                                .selected_text(format!("{:?}", tree.blend_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut tree.blend_type,
                                        BlendTreeType::Simple1D,
                                        "Simple 1D",
                                    );
                                    ui.selectable_value(
                                        &mut tree.blend_type,
                                        BlendTreeType::Simple2D,
                                        "Simple 2D",
                                    );
                                    ui.selectable_value(
                                        &mut tree.blend_type,
                                        BlendTreeType::FreeformDirectional,
                                        "Freeform Directional",
                                    );
                                    ui.selectable_value(
                                        &mut tree.blend_type,
                                        BlendTreeType::FreeformCartesian,
                                        "Freeform Cartesian",
                                    );
                                    ui.selectable_value(
                                        &mut tree.blend_type,
                                        BlendTreeType::Direct,
                                        "Direct",
                                    );
                                });
                            ui.end_row();

                            ui.label("Parameter X:");
                            ui.text_edit_singleline(&mut tree.blend_parameter_x);
                            ui.end_row();

                            if matches!(
                                tree.blend_type,
                                BlendTreeType::Simple2D
                                    | BlendTreeType::FreeformDirectional
                                    | BlendTreeType::FreeformCartesian
                            ) {
                                ui.label("Parameter Y:");
                                ui.text_edit_singleline(&mut tree.blend_parameter_y);
                                ui.end_row();
                            }
                        });

                    ui.label(format!("Children: {}", tree.children.len()));
                });
            }
        }
    }

    fn show_layers_tab(&mut self, ui: &mut Ui) {
        ui.heading("📚 Animation Layers");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("+ Add Layer").clicked() {
                let id = self.current_controller.layers.len() as u32 + 1;
                info!(target: "aw_editor::panels::animation_panel", layer_id = id, "Animation layer added");
                self.current_controller.layers.push(AnimationLayer {
                    id,
                    name: format!("Layer {}", id),
                    ..Default::default()
                });
            }
        });

        ui.add_space(10.0);

        for (idx, layer) in self.current_controller.layers.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("#{} {}", idx, layer.name)).strong());

                    if idx == 0 {
                        ui.label(RichText::new("(Base)").small().color(Color32::GRAY));
                    }
                });

                egui::Grid::new(format!("layer_{}", layer.id))
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut layer.name);
                        ui.end_row();

                        ui.label("Weight:");
                        ui.add(egui::Slider::new(&mut layer.weight, 0.0..=1.0));
                        ui.end_row();

                        ui.label("Blending:");
                        egui::ComboBox::from_id_salt(format!("layer_blend_{}", layer.id))
                            .selected_text(format!("{:?}", layer.blending_mode))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut layer.blending_mode,
                                    BlendingMode::Override,
                                    "Override",
                                );
                                ui.selectable_value(
                                    &mut layer.blending_mode,
                                    BlendingMode::Additive,
                                    "Additive",
                                );
                            });
                        ui.end_row();

                        ui.label("IK Pass:");
                        ui.checkbox(&mut layer.ik_pass, "");
                        ui.end_row();
                    });
            });
        }
    }

    fn show_parameters_tab(&mut self, ui: &mut Ui) {
        ui.heading("Parameters");
        ui.add_space(10.0);

        // Add parameter buttons
        ui.horizontal(|ui| {
            if ui.button("+ Float").clicked() {
                let id = self.next_parameter_id();
                info!(target: "aw_editor::panels::animation_panel", param_id = id, param_type = "Float", "Animation parameter added");
                self.current_controller.parameters.push(AnimationParameter {
                    id,
                    name: format!("Float{}", id),
                    param_type: ParameterType::Float,
                    default_value: ParameterValue::Float(0.0),
                });
            }
            if ui.button("+ Int").clicked() {
                let id = self.next_parameter_id();
                info!(target: "aw_editor::panels::animation_panel", param_id = id, param_type = "Int", "Animation parameter added");
                self.current_controller.parameters.push(AnimationParameter {
                    id,
                    name: format!("Int{}", id),
                    param_type: ParameterType::Int,
                    default_value: ParameterValue::Int(0),
                });
            }
            if ui.button("+ Bool").clicked() {
                let id = self.next_parameter_id();
                info!(target: "aw_editor::panels::animation_panel", param_id = id, param_type = "Bool", "Animation parameter added");
                self.current_controller.parameters.push(AnimationParameter {
                    id,
                    name: format!("Bool{}", id),
                    param_type: ParameterType::Bool,
                    default_value: ParameterValue::Bool(false),
                });
            }
            if ui.button("+ Trigger").clicked() {
                let id = self.next_parameter_id();
                info!(target: "aw_editor::panels::animation_panel", param_id = id, param_type = "Trigger", "Animation parameter added");
                self.current_controller.parameters.push(AnimationParameter {
                    id,
                    name: format!("Trigger{}", id),
                    param_type: ParameterType::Trigger,
                    default_value: ParameterValue::Trigger,
                });
            }
        });

        ui.add_space(10.0);

        // Parameter list
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                for param in &mut self.current_controller.parameters {
                    ui.horizontal(|ui| {
                        let type_icon = match param.param_type {
                            ParameterType::Float => "🔢",
                            ParameterType::Int => "🔢",
                            ParameterType::Bool => "☑️",
                            ParameterType::Trigger => "[Zap]",
                        };

                        ui.label(type_icon);
                        ui.text_edit_singleline(&mut param.name);

                        match &mut param.default_value {
                            ParameterValue::Float(v) => {
                                ui.add(egui::DragValue::new(v).speed(0.1));
                            }
                            ParameterValue::Int(v) => {
                                ui.add(egui::DragValue::new(v).speed(1));
                            }
                            ParameterValue::Bool(v) => {
                                ui.checkbox(v, "");
                            }
                            ParameterValue::Trigger => {
                                if ui.button("Fire").clicked() {
                                    // Trigger the parameter
                                }
                            }
                        }
                    });
                }
            });
    }

    fn show_ik_tab(&mut self, ui: &mut Ui) {
        ui.heading("🦴 Inverse Kinematics");
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.current_controller.ik_settings.enabled,
            RichText::new("Enable IK").strong(),
        );

        if !self.current_controller.ik_settings.enabled {
            return;
        }

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                // Foot IK
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_controller.ik_settings.foot_ik_enabled,
                        RichText::new("🦶 Foot IK").strong(),
                    );

                    if self.current_controller.ik_settings.foot_ik_enabled {
                        egui::Grid::new("foot_ik")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Height Offset:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.foot_height_offset,
                                    -0.5..=0.5,
                                ));
                                ui.end_row();

                                ui.label("Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.foot_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // Look At IK
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_controller.ik_settings.look_at_enabled,
                        RichText::new("👀 Look At IK").strong(),
                    );

                    if self.current_controller.ik_settings.look_at_enabled {
                        egui::Grid::new("look_at_ik")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.look_at_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Head Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.head_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Body Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.body_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Eyes Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.eyes_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Clamp Weight:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_controller.ik_settings.clamp_weight,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // Hand IK
                ui.group(|ui| {
                    ui.label(RichText::new("✋ Hand IK").strong());

                    ui.checkbox(
                        &mut self.current_controller.ik_settings.left_hand_ik_enabled,
                        "Left Hand",
                    );
                    ui.checkbox(
                        &mut self.current_controller.ik_settings.right_hand_ik_enabled,
                        "Right Hand",
                    );

                    if self.current_controller.ik_settings.left_hand_ik_enabled
                        || self.current_controller.ik_settings.right_hand_ik_enabled
                    {
                        ui.horizontal(|ui| {
                            ui.label("Weight:");
                            ui.add(egui::Slider::new(
                                &mut self.current_controller.ik_settings.hand_ik_weight,
                                0.0..=1.0,
                            ));
                        });
                    }
                });
            });
    }

    fn show_preview_tab(&mut self, ui: &mut Ui) {
        ui.heading("> Preview");
        ui.add_space(10.0);

        // Playback controls
        ui.group(|ui| {
            ui.label(RichText::new("Playback").strong());

            ui.horizontal(|ui| {
                if ui
                    .button(if self.preview_playing {
                        "|| Pause"
                    } else {
                        "> Play"
                    })
                    .clicked()
                {
                    self.preview_playing = !self.preview_playing;
                    info!(target: "aw_editor::panels::animation_panel", playing = self.preview_playing, "IK/Procedural preview playback toggled");
                }
                if ui.button("[] Stop").clicked() {
                    info!(target: "aw_editor::panels::animation_panel", "IK/Procedural preview playback stopped");
                    self.preview_playing = false;
                    self.preview_time = 0.0;
                }
                if ui.button("Reset").clicked() {
                    self.preview_time = 0.0;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.preview_speed, 0.1..=3.0));
            });

            ui.horizontal(|ui| {
                ui.label("Time:");
                ui.add(
                    egui::DragValue::new(&mut self.preview_time)
                        .speed(0.01)
                        .suffix("s"),
                );
            });
        });

        ui.add_space(10.0);

        // Current state
        ui.group(|ui| {
            ui.label(RichText::new("Current State").strong());

            let default_state = self
                .current_controller
                .states
                .iter()
                .find(|s| s.is_default)
                .map_or("(none)", |s| &s.name);

            ui.label(format!("Active State: {}", default_state));
            ui.label(format!("Time: {:.2}s", self.preview_time));

            if let Some(clip) = self
                .selected_clip
                .and_then(|id| self.clips.iter().find(|c| c.id == id))
            {
                ui.label(format!("Clip: {} ({:.2}s)", clip.name, clip.duration));
                let progress = if clip.duration > 0.0 {
                    (self.preview_time % clip.duration) / clip.duration
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(progress).show_percentage());
            }
        });

        ui.add_space(10.0);

        // Parameter values
        ui.group(|ui| {
            ui.label(RichText::new("Live Parameters").strong());

            for param in &self.current_controller.parameters {
                ui.horizontal(|ui| {
                    ui.label(&param.name);
                    match &param.default_value {
                        ParameterValue::Float(v) => {
                            ui.label(format!("{:.2}", v));
                        }
                        ParameterValue::Int(v) => {
                            ui.label(format!("{}", v));
                        }
                        ParameterValue::Bool(v) => {
                            ui.label(if *v { "true" } else { "false" });
                        }
                        ParameterValue::Trigger => {
                            ui.label("(trigger)");
                        }
                    }
                });
            }
        });
    }

    // Getters for testing
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    pub fn controller_count(&self) -> usize {
        self.controllers.len()
    }

    pub fn state_count(&self) -> usize {
        self.current_controller.states.len()
    }

    pub fn parameter_count(&self) -> usize {
        self.current_controller.parameters.len()
    }

    pub fn layer_count(&self) -> usize {
        self.current_controller.layers.len()
    }

    pub fn add_clip(&mut self, name: &str, duration: f32) -> u32 {
        let id = self.next_clip_id();
        self.clips.push(AnimationClip {
            id,
            name: name.to_string(),
            duration,
            ..Default::default()
        });
        id
    }

    pub fn add_state(&mut self, name: &str) -> u32 {
        let id = self.next_state_id();
        self.current_controller.states.push(AnimationState {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }
}

impl Panel for AnimationPanel {
    fn name(&self) -> &'static str {
        "Animation"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            AnimationTab::Clips => self.show_clips_tab(ui),
            AnimationTab::StateMachine => self.show_state_machine_tab(ui),
            AnimationTab::BlendTrees => self.show_blend_trees_tab(ui),
            AnimationTab::Layers => self.show_layers_tab(ui),
            AnimationTab::Parameters => self.show_parameters_tab(ui),
            AnimationTab::IK => self.show_ik_tab(ui),
            AnimationTab::Preview => self.show_preview_tab(ui),
        }
    }

    fn update(&mut self) {
        if self.preview_playing {
            self.preview_time += 0.016 * self.preview_speed;
            if let Some(clip) = self.clips.iter().find(|c| self.selected_clip == Some(c.id)) {
                match clip.loop_mode {
                    LoopMode::Loop => {
                        if self.preview_time > clip.duration {
                            self.preview_time = 0.0;
                        }
                    }
                    LoopMode::PingPong => {
                        // Handle ping pong
                    }
                    LoopMode::Once | LoopMode::ClampForever => {
                        if self.preview_time > clip.duration {
                            self.preview_time = clip.duration;
                            self.preview_playing = false;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // LOOP MODE TESTS
    // ============================================================

    #[test]
    fn test_loop_mode_default() {
        let mode = LoopMode::default();
        assert_eq!(mode, LoopMode::Loop);
    }

    #[test]
    fn test_loop_mode_all_variants() {
        let variants = [
            LoopMode::Once,
            LoopMode::Loop,
            LoopMode::PingPong,
            LoopMode::ClampForever,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_loop_mode_clone() {
        let mode = LoopMode::PingPong;
        let cloned = mode;
        assert_eq!(cloned, LoopMode::PingPong);
    }

    // ============================================================
    // TANGENT MODE TESTS
    // ============================================================

    #[test]
    fn test_tangent_mode_default() {
        let mode = TangentMode::default();
        assert_eq!(mode, TangentMode::Auto);
    }

    #[test]
    fn test_tangent_mode_all_variants() {
        let variants = [
            TangentMode::Auto,
            TangentMode::Linear,
            TangentMode::Constant,
            TangentMode::Free,
            TangentMode::Broken,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // CONDITION TYPE TESTS
    // ============================================================

    #[test]
    fn test_condition_type_default() {
        let ct = ConditionType::default();
        assert_eq!(ct, ConditionType::Greater);
    }

    #[test]
    fn test_condition_type_all_variants() {
        let variants = [
            ConditionType::Greater,
            ConditionType::Less,
            ConditionType::Equals,
            ConditionType::NotEquals,
            ConditionType::True,
            ConditionType::False,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // PARAMETER TYPE TESTS
    // ============================================================

    #[test]
    fn test_parameter_type_default() {
        let pt = ParameterType::default();
        assert_eq!(pt, ParameterType::Float);
    }

    #[test]
    fn test_parameter_type_all_variants() {
        let variants = [
            ParameterType::Float,
            ParameterType::Int,
            ParameterType::Bool,
            ParameterType::Trigger,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // PARAMETER VALUE TESTS
    // ============================================================

    #[test]
    fn test_parameter_value_default() {
        let pv = ParameterValue::default();
        match pv {
            ParameterValue::Float(v) => assert_eq!(v, 0.0),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_parameter_value_float() {
        let pv = ParameterValue::Float(1.5);
        match pv {
            ParameterValue::Float(v) => assert!((v - 1.5).abs() < 0.001),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_parameter_value_int() {
        let pv = ParameterValue::Int(42);
        match pv {
            ParameterValue::Int(v) => assert_eq!(v, 42),
            _ => panic!("Expected Int"),
        }
    }

    #[test]
    fn test_parameter_value_bool() {
        let pv = ParameterValue::Bool(true);
        match pv {
            ParameterValue::Bool(v) => assert!(v),
            _ => panic!("Expected Bool"),
        }
    }

    #[test]
    fn test_parameter_value_trigger() {
        let pv = ParameterValue::Trigger;
        assert!(matches!(pv, ParameterValue::Trigger));
    }

    // ============================================================
    // BLEND TREE TYPE TESTS
    // ============================================================

    #[test]
    fn test_blend_tree_type_default() {
        let btt = BlendTreeType::default();
        assert_eq!(btt, BlendTreeType::Simple1D);
    }

    #[test]
    fn test_blend_tree_type_all_variants() {
        let variants = [
            BlendTreeType::Simple1D,
            BlendTreeType::Simple2D,
            BlendTreeType::FreeformDirectional,
            BlendTreeType::FreeformCartesian,
            BlendTreeType::Direct,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // BLENDING MODE TESTS
    // ============================================================

    #[test]
    fn test_blending_mode_default() {
        let bm = BlendingMode::default();
        assert_eq!(bm, BlendingMode::Override);
    }

    #[test]
    fn test_blending_mode_all_variants() {
        let variants = [BlendingMode::Override, BlendingMode::Additive];
        assert_eq!(variants.len(), 2);
    }

    // ============================================================
    // ANIMATION CLIP TESTS
    // ============================================================

    #[test]
    fn test_default_clip() {
        let clip = AnimationClip::default();
        assert_eq!(clip.loop_mode, LoopMode::Loop);
        assert!((clip.fps - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_animation_clip_default_full() {
        let clip = AnimationClip::default();
        assert_eq!(clip.id, 0);
        assert_eq!(clip.name, "New Clip");
        assert!((clip.duration - 1.0).abs() < 0.001);
        assert!((clip.fps - 30.0).abs() < 0.001);
        assert_eq!(clip.loop_mode, LoopMode::Loop);
        assert!(clip.events.is_empty());
        assert!(clip.curves.is_empty());
    }

    #[test]
    fn test_animation_clip_custom() {
        let clip = AnimationClip {
            id: 1,
            name: "Walk".to_string(),
            duration: 2.5,
            fps: 60.0,
            loop_mode: LoopMode::Once,
            events: vec![],
            curves: vec![],
        };
        assert_eq!(clip.name, "Walk");
        assert!((clip.duration - 2.5).abs() < 0.001);
        assert_eq!(clip.loop_mode, LoopMode::Once);
    }

    #[test]
    fn test_animation_clip_clone() {
        let clip = AnimationClip::default();
        let cloned = clip.clone();
        assert_eq!(cloned.name, "New Clip");
    }

    // ============================================================
    // ANIMATION STATE TESTS
    // ============================================================

    #[test]
    fn test_animation_state_default() {
        let state = AnimationState::default();
        assert_eq!(state.id, 0);
        assert_eq!(state.name, "New State");
        assert!(state.clip_id.is_none());
        assert!((state.speed - 1.0).abs() < 0.001);
        assert_eq!(state.position, [100.0, 100.0]);
        assert!(!state.is_default);
    }

    #[test]
    fn test_animation_state_clone() {
        let state = AnimationState::default();
        let cloned = state.clone();
        assert_eq!(cloned.name, "New State");
    }

    // ============================================================
    // STATE TRANSITION TESTS
    // ============================================================

    #[test]
    fn test_state_transition_default() {
        let transition = StateTransition::default();
        assert_eq!(transition.id, 0);
        assert_eq!(transition.from_state, 0);
        assert_eq!(transition.to_state, 0);
        assert!((transition.duration - 0.25).abs() < 0.001);
        assert!((transition.offset - 0.0).abs() < 0.001);
        assert!(!transition.has_exit_time);
        assert!((transition.exit_time - 1.0).abs() < 0.001);
        assert!(transition.conditions.is_empty());
    }

    #[test]
    fn test_state_transition_clone() {
        let transition = StateTransition::default();
        let cloned = transition.clone();
        assert!((cloned.duration - 0.25).abs() < 0.001);
    }

    // ============================================================
    // BLEND TREE TESTS
    // ============================================================

    #[test]
    fn test_blend_tree_default() {
        let tree = BlendTree::default();
        assert_eq!(tree.blend_type, BlendTreeType::Simple1D);
        assert!(tree.children.is_empty());
    }

    #[test]
    fn test_blend_tree_default_full() {
        let tree = BlendTree::default();
        assert_eq!(tree.id, 0);
        assert_eq!(tree.name, "New Blend Tree");
        assert_eq!(tree.blend_type, BlendTreeType::Simple1D);
        assert!(tree.blend_parameter_x.is_empty());
        assert!(tree.blend_parameter_y.is_empty());
        assert!(tree.children.is_empty());
    }

    #[test]
    fn test_blend_tree_clone() {
        let tree = BlendTree::default();
        let cloned = tree.clone();
        assert_eq!(cloned.name, "New Blend Tree");
    }

    // ============================================================
    // ANIMATION LAYER TESTS
    // ============================================================

    #[test]
    fn test_default_layer() {
        let layer = AnimationLayer::default();
        assert!((layer.weight - 1.0).abs() < 0.001);
        assert_eq!(layer.blending_mode, BlendingMode::Override);
    }

    #[test]
    fn test_animation_layer_default_full() {
        let layer = AnimationLayer::default();
        assert_eq!(layer.id, 0);
        assert_eq!(layer.name, "New Layer");
        assert!((layer.weight - 1.0).abs() < 0.001);
        assert_eq!(layer.blending_mode, BlendingMode::Override);
        assert!(layer.avatar_mask.is_none());
        assert!(!layer.ik_pass);
    }

    #[test]
    fn test_animation_layer_clone() {
        let layer = AnimationLayer::default();
        let cloned = layer.clone();
        assert_eq!(cloned.name, "New Layer");
    }

    // ============================================================
    // IK SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_ik_settings_default() {
        let ik = IkSettings::default();
        assert!(!ik.enabled);
        assert!(ik.foot_ik_enabled);
        assert!(!ik.look_at_enabled);
    }

    #[test]
    fn test_ik_settings_default_full() {
        let ik = IkSettings::default();
        assert!(!ik.enabled);
        assert!(ik.foot_ik_enabled);
        assert!((ik.foot_height_offset - 0.0).abs() < 0.001);
        assert!((ik.foot_weight - 1.0).abs() < 0.001);
        assert!(!ik.look_at_enabled);
        assert!((ik.look_at_weight - 1.0).abs() < 0.001);
        assert!((ik.head_weight - 1.0).abs() < 0.001);
        assert!((ik.body_weight - 0.4).abs() < 0.001);
        assert!(!ik.left_hand_ik_enabled);
        assert!(!ik.right_hand_ik_enabled);
    }

    #[test]
    fn test_ik_settings_clone() {
        let ik = IkSettings::default();
        let cloned = ik.clone();
        assert!(!cloned.enabled);
        assert!(cloned.foot_ik_enabled);
    }

    // ============================================================
    // ANIMATION CONTROLLER TESTS
    // ============================================================

    #[test]
    fn test_animation_controller_default() {
        let ctrl = AnimationController::default();
        assert_eq!(ctrl.id, 0);
        assert_eq!(ctrl.name, "New Controller");
        assert!(ctrl.states.is_empty());
        assert!(ctrl.transitions.is_empty());
        assert!(ctrl.parameters.is_empty());
        assert_eq!(ctrl.layers.len(), 1);
        assert!(ctrl.blend_trees.is_empty());
    }

    #[test]
    fn test_animation_controller_clone() {
        let ctrl = AnimationController::default();
        let cloned = ctrl.clone();
        assert_eq!(cloned.name, "New Controller");
    }

    // ============================================================
    // ANIMATION PANEL TESTS
    // ============================================================

    #[test]
    fn test_animation_panel_creation() {
        let mut panel = AnimationPanel::new();
        panel.create_sample_data();
        assert!(panel.clip_count() >= 6);
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = AnimationPanel::new();
        panel.create_sample_data();
        assert!(panel.controller_count() >= 1);
        assert!(panel.state_count() >= 3);
        assert!(panel.parameter_count() >= 3);
    }

    #[test]
    fn test_add_clip() {
        let mut panel = AnimationPanel::new();
        let initial_count = panel.clip_count();

        let id = panel.add_clip("Test Clip", 2.5);
        assert!(id > 0);
        assert_eq!(panel.clip_count(), initial_count + 1);
    }

    #[test]
    fn test_add_state() {
        let mut panel = AnimationPanel::new();
        let initial_count = panel.state_count();

        let id = panel.add_state("Test State");
        assert!(id > 0);
        assert_eq!(panel.state_count(), initial_count + 1);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = AnimationPanel::new();
        assert_eq!(panel.name(), "Animation");
    }

    // ============================================================
    // ANIMATION TAB TESTS
    // ============================================================

    #[test]
    fn test_animation_tab_default() {
        let tab = AnimationTab::default();
        assert_eq!(tab, AnimationTab::Clips);
    }

    #[test]
    fn test_animation_tab_all_variants() {
        let variants = [
            AnimationTab::Clips,
            AnimationTab::StateMachine,
            AnimationTab::BlendTrees,
            AnimationTab::Layers,
            AnimationTab::Parameters,
            AnimationTab::IK,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // KEYFRAME TESTS
    // ============================================================

    #[test]
    fn test_keyframe_creation() {
        let kf = Keyframe {
            time: 0.5,
            value: 1.0,
            in_tangent: 0.0,
            out_tangent: 0.0,
            tangent_mode: TangentMode::Auto,
        };
        assert!((kf.time - 0.5).abs() < 0.001);
        assert!((kf.value - 1.0).abs() < 0.001);
        assert_eq!(kf.tangent_mode, TangentMode::Auto);
    }

    #[test]
    fn test_keyframe_clone() {
        let kf = Keyframe {
            time: 1.0,
            value: 2.0,
            in_tangent: 0.5,
            out_tangent: 0.5,
            tangent_mode: TangentMode::Linear,
        };
        let cloned = kf.clone();
        assert!((cloned.time - 1.0).abs() < 0.001);
        assert_eq!(cloned.tangent_mode, TangentMode::Linear);
    }

    // ============================================================
    // BLEND TREE CHILD TESTS
    // ============================================================

    #[test]
    fn test_blend_tree_child_creation() {
        let child = BlendTreeChild {
            clip_id: 1,
            threshold: 0.5,
            position: [0.0, 0.0],
            time_scale: 1.0,
        };
        assert_eq!(child.clip_id, 1);
        assert!((child.threshold - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_blend_tree_child_clone() {
        let child = BlendTreeChild {
            clip_id: 2,
            threshold: 0.75,
            position: [1.0, 1.0],
            time_scale: 1.5,
        };
        let cloned = child.clone();
        assert_eq!(cloned.clip_id, 2);
        assert!((cloned.time_scale - 1.5).abs() < 0.001);
    }

    // ============================================================
    // TRANSITION CONDITION TESTS
    // ============================================================

    #[test]
    fn test_transition_condition_creation() {
        let cond = TransitionCondition {
            parameter: "Speed".to_string(),
            condition_type: ConditionType::Greater,
            threshold: 0.5,
        };
        assert_eq!(cond.parameter, "Speed");
        assert_eq!(cond.condition_type, ConditionType::Greater);
    }

    #[test]
    fn test_transition_condition_clone() {
        let cond = TransitionCondition {
            parameter: "IsJumping".to_string(),
            condition_type: ConditionType::True,
            threshold: 0.0,
        };
        let cloned = cond.clone();
        assert_eq!(cloned.parameter, "IsJumping");
    }

    // ============================================================
    // LOOP MODE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_loop_mode_display() {
        for mode in LoopMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_loop_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(LoopMode::Loop);
        set.insert(LoopMode::Once);
        assert!(set.contains(&LoopMode::Loop));
        assert!(!set.contains(&LoopMode::PingPong));
    }

    #[test]
    fn test_loop_mode_all() {
        let all = LoopMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&LoopMode::Once));
        assert!(all.contains(&LoopMode::Loop));
        assert!(all.contains(&LoopMode::PingPong));
        assert!(all.contains(&LoopMode::ClampForever));
    }

    #[test]
    fn test_loop_mode_is_looping() {
        assert!(!LoopMode::Once.is_looping());
        assert!(LoopMode::Loop.is_looping());
        assert!(LoopMode::PingPong.is_looping());
        assert!(!LoopMode::ClampForever.is_looping());
    }

    #[test]
    fn test_loop_mode_description() {
        assert!(!LoopMode::Once.description().is_empty());
        assert!(LoopMode::Loop.description().contains("Loop"));
    }

    // ============================================================
    // TANGENT MODE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_tangent_mode_display() {
        for mode in TangentMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_tangent_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TangentMode::Auto);
        set.insert(TangentMode::Linear);
        assert!(set.contains(&TangentMode::Auto));
        assert!(!set.contains(&TangentMode::Constant));
    }

    #[test]
    fn test_tangent_mode_all() {
        let all = TangentMode::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&TangentMode::Auto));
        assert!(all.contains(&TangentMode::Broken));
    }

    #[test]
    fn test_tangent_mode_is_smooth() {
        assert!(TangentMode::Auto.is_smooth());
        assert!(!TangentMode::Linear.is_smooth());
        assert!(!TangentMode::Constant.is_smooth());
        assert!(TangentMode::Free.is_smooth());
        assert!(!TangentMode::Broken.is_smooth());
    }

    #[test]
    fn test_tangent_mode_is_editable() {
        assert!(!TangentMode::Auto.is_editable());
        assert!(!TangentMode::Linear.is_editable());
        assert!(TangentMode::Free.is_editable());
        assert!(TangentMode::Broken.is_editable());
    }

    // ============================================================
    // CONDITION TYPE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_condition_type_display() {
        for ct in ConditionType::all() {
            let display = format!("{}", ct);
            assert!(display.contains(ct.name()));
            assert!(display.contains(ct.symbol()));
        }
    }

    #[test]
    fn test_condition_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ConditionType::Greater);
        set.insert(ConditionType::Less);
        assert!(set.contains(&ConditionType::Greater));
        assert!(!set.contains(&ConditionType::Equals));
    }

    #[test]
    fn test_condition_type_all() {
        let all = ConditionType::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_condition_type_is_comparison() {
        assert!(ConditionType::Greater.is_comparison());
        assert!(ConditionType::Less.is_comparison());
        assert!(ConditionType::Equals.is_comparison());
        assert!(ConditionType::NotEquals.is_comparison());
        assert!(!ConditionType::True.is_comparison());
        assert!(!ConditionType::False.is_comparison());
    }

    #[test]
    fn test_condition_type_is_boolean() {
        assert!(!ConditionType::Greater.is_boolean());
        assert!(ConditionType::True.is_boolean());
        assert!(ConditionType::False.is_boolean());
    }

    // ============================================================
    // PARAMETER TYPE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_parameter_type_display() {
        for pt in ParameterType::all() {
            let display = format!("{}", pt);
            assert!(display.contains(pt.name()));
            assert!(display.contains(pt.icon()));
        }
    }

    #[test]
    fn test_parameter_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ParameterType::Float);
        set.insert(ParameterType::Int);
        assert!(set.contains(&ParameterType::Float));
        assert!(!set.contains(&ParameterType::Bool));
    }

    #[test]
    fn test_parameter_type_all() {
        let all = ParameterType::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_parameter_type_is_numeric() {
        assert!(ParameterType::Float.is_numeric());
        assert!(ParameterType::Int.is_numeric());
        assert!(!ParameterType::Bool.is_numeric());
        assert!(!ParameterType::Trigger.is_numeric());
    }

    #[test]
    fn test_parameter_type_is_instant() {
        assert!(!ParameterType::Float.is_instant());
        assert!(!ParameterType::Bool.is_instant());
        assert!(ParameterType::Trigger.is_instant());
    }

    // ============================================================
    // BLEND TREE TYPE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_blend_tree_type_display() {
        for btt in BlendTreeType::all() {
            let display = format!("{}", btt);
            assert!(display.contains(btt.name()));
            assert!(display.contains(btt.icon()));
        }
    }

    #[test]
    fn test_blend_tree_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BlendTreeType::Simple1D);
        set.insert(BlendTreeType::Simple2D);
        assert!(set.contains(&BlendTreeType::Simple1D));
        assert!(!set.contains(&BlendTreeType::Direct));
    }

    #[test]
    fn test_blend_tree_type_all() {
        let all = BlendTreeType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_blend_tree_type_dimensions() {
        assert_eq!(BlendTreeType::Simple1D.dimensions(), 1);
        assert_eq!(BlendTreeType::Simple2D.dimensions(), 2);
        assert_eq!(BlendTreeType::FreeformDirectional.dimensions(), 2);
        assert_eq!(BlendTreeType::FreeformCartesian.dimensions(), 2);
        assert_eq!(BlendTreeType::Direct.dimensions(), 1);
    }

    #[test]
    fn test_blend_tree_type_is_freeform() {
        assert!(!BlendTreeType::Simple1D.is_freeform());
        assert!(!BlendTreeType::Simple2D.is_freeform());
        assert!(BlendTreeType::FreeformDirectional.is_freeform());
        assert!(BlendTreeType::FreeformCartesian.is_freeform());
        assert!(!BlendTreeType::Direct.is_freeform());
    }

    // ============================================================
    // BLENDING MODE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_blending_mode_display() {
        for bm in BlendingMode::all() {
            let display = format!("{}", bm);
            assert!(display.contains(bm.name()));
            assert!(display.contains(bm.icon()));
        }
    }

    #[test]
    fn test_blending_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BlendingMode::Override);
        assert!(set.contains(&BlendingMode::Override));
        assert!(!set.contains(&BlendingMode::Additive));
    }

    #[test]
    fn test_blending_mode_all() {
        let all = BlendingMode::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&BlendingMode::Override));
        assert!(all.contains(&BlendingMode::Additive));
    }

    #[test]
    fn test_blending_mode_description() {
        assert!(BlendingMode::Override.description().contains("Replace"));
        assert!(BlendingMode::Additive.description().contains("Add"));
    }

    // ============================================================
    // ANIMATION TAB DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_animation_tab_display() {
        for tab in AnimationTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
            assert!(display.contains(tab.icon()));
        }
    }

    #[test]
    fn test_animation_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AnimationTab::Clips);
        set.insert(AnimationTab::StateMachine);
        assert!(set.contains(&AnimationTab::Clips));
        assert!(!set.contains(&AnimationTab::IK));
    }

    #[test]
    fn test_animation_tab_all() {
        let all = AnimationTab::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&AnimationTab::Preview));
    }

    #[test]
    fn test_animation_tab_description() {
        for tab in AnimationTab::all() {
            assert!(!tab.description().is_empty());
        }
    }

    // ============================================================
    // PARAMETER VALUE DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_parameter_value_display() {
        let float_val = ParameterValue::Float(1.5);
        let display = format!("{}", float_val);
        assert!(display.contains(float_val.name()));

        let int_val = ParameterValue::Int(42);
        let display = format!("{}", int_val);
        assert!(display.contains(int_val.name()));

        let bool_val = ParameterValue::Bool(true);
        let display = format!("{}", bool_val);
        assert!(display.contains(bool_val.name()));

        let trigger = ParameterValue::Trigger;
        let display = format!("{}", trigger);
        assert!(display.contains(trigger.name()));
    }

    #[test]
    fn test_parameter_value_all_variants() {
        let all = ParameterValue::all_variants();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&"Float"));
        assert!(all.contains(&"Int"));
        assert!(all.contains(&"Bool"));
        assert!(all.contains(&"Trigger"));
    }

    #[test]
    fn test_parameter_value_names() {
        assert_eq!(ParameterValue::Float(0.0).name(), "Float");
        assert_eq!(ParameterValue::Int(0).name(), "Int");
        assert_eq!(ParameterValue::Bool(false).name(), "Bool");
        assert_eq!(ParameterValue::Trigger.name(), "Trigger");
    }

    #[test]
    fn test_parameter_value_icons() {
        assert_eq!(ParameterValue::Float(0.0).icon(), "🔢");
        assert_eq!(ParameterValue::Int(0).icon(), "🔟");
        assert_eq!(ParameterValue::Bool(false).icon(), "✓");
        assert_eq!(ParameterValue::Trigger.icon(), "[Zap]");
    }

    #[test]
    fn test_parameter_value_is_numeric() {
        assert!(ParameterValue::Float(1.5).is_numeric());
        assert!(ParameterValue::Int(42).is_numeric());
        assert!(!ParameterValue::Bool(true).is_numeric());
        assert!(!ParameterValue::Trigger.is_numeric());
    }

    #[test]
    fn test_parameter_value_is_bool() {
        assert!(!ParameterValue::Float(1.5).is_bool());
        assert!(!ParameterValue::Int(42).is_bool());
        assert!(ParameterValue::Bool(true).is_bool());
        assert!(!ParameterValue::Trigger.is_bool());
    }

    #[test]
    fn test_parameter_value_is_trigger() {
        assert!(!ParameterValue::Float(1.5).is_trigger());
        assert!(!ParameterValue::Int(42).is_trigger());
        assert!(!ParameterValue::Bool(true).is_trigger());
        assert!(ParameterValue::Trigger.is_trigger());
    }

    #[test]
    fn test_parameter_value_as_float() {
        assert_eq!(ParameterValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(ParameterValue::Int(42).as_float(), None);
        assert_eq!(ParameterValue::Bool(true).as_float(), None);
        assert_eq!(ParameterValue::Trigger.as_float(), None);
    }

    #[test]
    fn test_parameter_value_as_int() {
        assert_eq!(ParameterValue::Float(3.14).as_int(), None);
        assert_eq!(ParameterValue::Int(42).as_int(), Some(42));
        assert_eq!(ParameterValue::Bool(true).as_int(), None);
        assert_eq!(ParameterValue::Trigger.as_int(), None);
    }

    #[test]
    fn test_parameter_value_as_bool() {
        assert_eq!(ParameterValue::Float(3.14).as_bool(), None);
        assert_eq!(ParameterValue::Int(42).as_bool(), None);
        assert_eq!(ParameterValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ParameterValue::Bool(false).as_bool(), Some(false));
        assert_eq!(ParameterValue::Trigger.as_bool(), None);
    }

    #[test]
    fn test_parameter_value_partial_eq() {
        let f1 = ParameterValue::Float(1.0);
        let f2 = ParameterValue::Float(1.0);
        let f3 = ParameterValue::Float(2.0);
        assert_eq!(f1, f2);
        assert_ne!(f1, f3);

        let i1 = ParameterValue::Int(42);
        let i2 = ParameterValue::Int(42);
        assert_eq!(i1, i2);
        assert_ne!(i1, f1);
    }

    #[test]
    fn test_parameter_value_default_is_float() {
        let default = ParameterValue::default();
        assert!(default.is_numeric());
        assert_eq!(default.as_float(), Some(0.0));
    }

    // ============================================================
    // ANIMATION ACTION TESTS
    // ============================================================

    #[test]
    fn test_action_system_initial_state() {
        let panel = AnimationPanel::default();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_queue_and_take() {
        let mut panel = AnimationPanel::default();
        panel.queue_action(AnimationAction::PlayClip { clip_id: 1 });
        panel.queue_action(AnimationAction::SetSpeed { speed: 1.5 });

        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());

        // Verify actions were drained
        let empty = panel.take_actions();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_action_names() {
        let actions = vec![
            (AnimationAction::PlayClip { clip_id: 1 }, "Play Clip"),
            (AnimationAction::PauseClip, "Pause Clip"),
            (AnimationAction::StopClip, "Stop Clip"),
            (AnimationAction::SetSpeed { speed: 2.0 }, "Set Speed"),
            (AnimationAction::SeekTo { time: 0.5 }, "Seek To"),
            (
                AnimationAction::CreateClip {
                    name: "new".to_string(),
                },
                "Create Clip",
            ),
            (AnimationAction::DeleteClip { clip_id: 1 }, "Delete Clip"),
            (AnimationAction::SaveClip { clip_id: 1 }, "Save Clip"),
            (
                AnimationAction::AddKeyframe {
                    clip_id: 1,
                    time: 0.0,
                },
                "Add Keyframe",
            ),
            (
                AnimationAction::DeleteKeyframe {
                    clip_id: 1,
                    keyframe_index: 0,
                },
                "Delete Keyframe",
            ),
            (
                AnimationAction::CreateController {
                    name: "ctrl".to_string(),
                },
                "Create Controller",
            ),
            (
                AnimationAction::SetLoopMode {
                    clip_id: 1,
                    loop_mode: LoopMode::Loop,
                },
                "Set Loop Mode",
            ),
            (
                AnimationAction::TriggerTransition {
                    from_state: 0,
                    to_state: 1,
                },
                "Trigger Transition",
            ),
        ];

        for (action, expected_name) in actions {
            assert_eq!(action.name(), expected_name);
        }
    }

    #[test]
    fn test_action_is_playback() {
        assert!(AnimationAction::PlayClip { clip_id: 1 }.is_playback());
        assert!(AnimationAction::PauseClip.is_playback());
        assert!(AnimationAction::StopClip.is_playback());
        assert!(AnimationAction::SetSpeed { speed: 1.0 }.is_playback());
        assert!(AnimationAction::SeekTo { time: 0.0 }.is_playback());

        assert!(!AnimationAction::SetLoopMode {
            clip_id: 1,
            loop_mode: LoopMode::Once
        }
        .is_playback());
        assert!(!AnimationAction::CreateClip {
            name: "test".to_string()
        }
        .is_playback());
        assert!(!AnimationAction::DeleteClip { clip_id: 1 }.is_playback());
        assert!(!AnimationAction::SaveClip { clip_id: 1 }.is_playback());
    }

    #[test]
    fn test_action_is_clip_edit() {
        assert!(AnimationAction::CreateClip {
            name: "test".to_string()
        }
        .is_clip_edit());
        assert!(AnimationAction::DeleteClip { clip_id: 1 }.is_clip_edit());
        assert!(AnimationAction::SaveClip { clip_id: 1 }.is_clip_edit());
        assert!(AnimationAction::AddKeyframe {
            clip_id: 1,
            time: 0.0
        }
        .is_clip_edit());
        assert!(AnimationAction::DeleteKeyframe {
            clip_id: 1,
            keyframe_index: 0
        }
        .is_clip_edit());

        assert!(!AnimationAction::PlayClip { clip_id: 1 }.is_clip_edit());
        assert!(!AnimationAction::PauseClip.is_clip_edit());
        assert!(!AnimationAction::SetSpeed { speed: 1.0 }.is_clip_edit());
    }

    #[test]
    fn test_action_display() {
        let action = AnimationAction::PlayClip { clip_id: 1 };
        let display = format!("{}", action);
        assert_eq!(display, "Play Clip");

        let action = AnimationAction::SetSpeed { speed: 2.5 };
        let display = format!("{}", action);
        assert_eq!(display, "Set Speed");
    }

    #[test]
    fn test_preview_speed_initial() {
        let panel = AnimationPanel::default();
        assert!((panel.preview_speed() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_is_playing_initial() {
        let panel = AnimationPanel::default();
        assert!(!panel.is_playing());
    }

    #[test]
    fn test_preview_time_initial() {
        let panel = AnimationPanel::default();
        assert!((panel.preview_time()).abs() < 0.001);
    }

    #[test]
    fn test_multiple_playback_actions() {
        let mut panel = AnimationPanel::default();
        panel.queue_action(AnimationAction::PlayClip { clip_id: 1 });
        panel.queue_action(AnimationAction::SetSpeed { speed: 2.0 });
        panel.queue_action(AnimationAction::SeekTo { time: 1.5 });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.iter().all(|a| a.is_playback()));
    }

    #[test]
    fn test_multiple_clip_edit_actions() {
        let mut panel = AnimationPanel::default();
        panel.queue_action(AnimationAction::CreateClip {
            name: "new_clip".to_string(),
        });
        panel.queue_action(AnimationAction::AddKeyframe {
            clip_id: 1,
            time: 0.0,
        });
        panel.queue_action(AnimationAction::AddKeyframe {
            clip_id: 1,
            time: 1.0,
        });
        panel.queue_action(AnimationAction::SaveClip { clip_id: 1 });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 4);
        assert!(actions.iter().all(|a| a.is_clip_edit()));
    }

    #[test]
    fn test_mixed_action_types() {
        let mut panel = AnimationPanel::default();
        panel.queue_action(AnimationAction::PlayClip { clip_id: 1 });
        panel.queue_action(AnimationAction::AddKeyframe {
            clip_id: 2,
            time: 0.5,
        });
        panel.queue_action(AnimationAction::CreateController {
            name: "character".to_string(),
        });
        panel.queue_action(AnimationAction::TriggerTransition {
            from_state: 0,
            to_state: 1,
        });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 4);

        // Check categories
        let playback_count = actions.iter().filter(|a| a.is_playback()).count();
        let edit_count = actions.iter().filter(|a| a.is_clip_edit()).count();

        assert_eq!(playback_count, 1); // PlayClip
        assert_eq!(edit_count, 1); // AddKeyframe
    }

    #[test]
    fn test_action_with_transition() {
        let action = AnimationAction::TriggerTransition {
            from_state: 0,
            to_state: 1,
        };
        assert_eq!(action.name(), "Trigger Transition");
        assert!(!action.is_playback());
        assert!(!action.is_clip_edit());
    }

    #[test]
    fn test_action_with_controller() {
        let action = AnimationAction::CreateController {
            name: "player_controller".to_string(),
        };
        assert_eq!(action.name(), "Create Controller");
        assert!(!action.is_playback());
        assert!(!action.is_clip_edit());
    }

    #[test]
    fn test_keyframe_selection_defaults() {
        let panel = AnimationPanel::new();
        assert!(panel.selected_curve.is_none());
        assert!(panel.selected_keyframe.is_none());
        assert!(!panel.dragging_keyframe);
    }

    #[test]
    fn test_keyframe_selection_state() {
        let mut panel = AnimationPanel::new();
        panel.selected_curve = Some(1);
        panel.selected_keyframe = Some(2);
        panel.dragging_keyframe = true;
        assert_eq!(panel.selected_curve, Some(1));
        assert_eq!(panel.selected_keyframe, Some(2));
        assert!(panel.dragging_keyframe);
    }

    #[test]
    fn test_animation_curve_keyframe_sort() {
        let mut curve = AnimationCurve {
            id: 1,
            property_path: "position.x".to_string(),
            keyframes: vec![
                Keyframe {
                    time: 2.0,
                    value: 10.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                    tangent_mode: TangentMode::Auto,
                },
                Keyframe {
                    time: 0.5,
                    value: 5.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                    tangent_mode: TangentMode::Linear,
                },
                Keyframe {
                    time: 1.0,
                    value: 7.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                    tangent_mode: TangentMode::Auto,
                },
            ],
        };
        curve.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert!((curve.keyframes[0].time - 0.5).abs() < 0.001);
        assert!((curve.keyframes[1].time - 1.0).abs() < 0.001);
        assert!((curve.keyframes[2].time - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_clip_with_curves_and_keyframes() {
        let clip = AnimationClip {
            id: 1,
            name: "Walk".to_string(),
            duration: 2.0,
            fps: 30.0,
            loop_mode: LoopMode::Loop,
            events: vec![],
            curves: vec![
                AnimationCurve {
                    id: 1,
                    property_path: "position.x".to_string(),
                    keyframes: vec![
                        Keyframe {
                            time: 0.0,
                            value: 0.0,
                            in_tangent: 0.0,
                            out_tangent: 0.0,
                            tangent_mode: TangentMode::Auto,
                        },
                        Keyframe {
                            time: 1.0,
                            value: 5.0,
                            in_tangent: 0.0,
                            out_tangent: 0.0,
                            tangent_mode: TangentMode::Linear,
                        },
                    ],
                },
                AnimationCurve {
                    id: 2,
                    property_path: "rotation.y".to_string(),
                    keyframes: vec![Keyframe {
                        time: 0.0,
                        value: 0.0,
                        in_tangent: 0.0,
                        out_tangent: 0.0,
                        tangent_mode: TangentMode::Auto,
                    }],
                },
            ],
        };
        assert_eq!(clip.curves.len(), 2);
        assert_eq!(clip.curves[0].keyframes.len(), 2);
        assert_eq!(clip.curves[1].keyframes.len(), 1);
        assert_eq!(clip.curves[0].property_path, "position.x");
    }

    #[test]
    fn test_keyframe_clamp_time() {
        let duration = 3.0_f32;
        let raw_time = 5.0_f32;
        let clamped = raw_time.clamp(0.0, duration);
        assert!((clamped - 3.0).abs() < 0.001);

        let negative = (-1.0_f32).clamp(0.0, duration);
        assert!((negative - 0.0).abs() < 0.001);
    }
}

//! Dialogue Editor Panel for the editor UI
//!
//! Provides comprehensive dialogue editing:
//! - Visual node-based dialogue graph editor
//! - Branching conversation trees with auto-layout
//! - Character/speaker management with portraits
//! - Condition/variable system with type checking
//! - Localization support with translation workflow
//! - Dialogue preview/testing with voice simulation
//! - Version control and collaboration features
//! - Templates and dialogue patterns
//! - Export/import (JSON, Yarn, Ink, custom formats)
//! - Validation and error checking
//! - Search and refactoring tools

use egui::{Color32, RichText, Ui, Vec2};
use tracing::{info, warn};

use crate::panels::Panel;

/// Dialogue node type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DialogueNodeType {
    #[default]
    Speech,
    Choice,
    Condition,
    Action,
    RandomBranch,
    Jump,
    End,
}

impl std::fmt::Display for DialogueNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl DialogueNodeType {
    pub fn all() -> &'static [DialogueNodeType] {
        &[
            DialogueNodeType::Speech,
            DialogueNodeType::Choice,
            DialogueNodeType::Condition,
            DialogueNodeType::Action,
            DialogueNodeType::RandomBranch,
            DialogueNodeType::Jump,
            DialogueNodeType::End,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            DialogueNodeType::Speech => "Speech",
            DialogueNodeType::Choice => "Choice",
            DialogueNodeType::Condition => "Condition",
            DialogueNodeType::Action => "Action",
            DialogueNodeType::RandomBranch => "Random Branch",
            DialogueNodeType::Jump => "Jump",
            DialogueNodeType::End => "End",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DialogueNodeType::Speech => "[Chat]",
            DialogueNodeType::Choice => "[Shuf]",
            DialogueNodeType::Condition => "❓",
            DialogueNodeType::Action => "[Zap]",
            DialogueNodeType::RandomBranch => "[Dice]",
            DialogueNodeType::Jump => "↪️",
            DialogueNodeType::End => "🏁",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            DialogueNodeType::Speech => Color32::from_rgb(100, 149, 237),
            DialogueNodeType::Choice => Color32::from_rgb(144, 238, 144),
            DialogueNodeType::Condition => Color32::from_rgb(255, 215, 0),
            DialogueNodeType::Action => Color32::from_rgb(255, 165, 0),
            DialogueNodeType::RandomBranch => Color32::from_rgb(186, 85, 211),
            DialogueNodeType::Jump => Color32::from_rgb(64, 224, 208),
            DialogueNodeType::End => Color32::from_rgb(220, 20, 60),
        }
    }

    pub fn is_branching(&self) -> bool {
        matches!(
            self,
            DialogueNodeType::Choice | DialogueNodeType::Condition | DialogueNodeType::RandomBranch
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, DialogueNodeType::End)
    }
}

/// Speaker/character definition
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueSpeaker {
    pub id: String,
    pub name: String,
    pub portrait: String,
    pub voice_id: String,
    pub color: Color32,
}

impl Default for DialogueSpeaker {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Unknown".to_string(),
            portrait: String::new(),
            voice_id: String::new(),
            color: Color32::WHITE,
        }
    }
}

/// Dialogue response/choice
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueChoice {
    pub text: String,
    pub target_node_id: Option<u32>,
    pub condition: String,
    pub is_default: bool,
}

impl Default for DialogueChoice {
    fn default() -> Self {
        Self {
            text: "Continue".to_string(),
            target_node_id: None,
            condition: String::new(),
            is_default: false,
        }
    }
}

/// Dialogue node
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueNode {
    pub id: u32,
    pub node_type: DialogueNodeType,
    pub speaker_id: Option<String>,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub position: (f32, f32),
    pub notes: String,
}

impl Default for DialogueNode {
    fn default() -> Self {
        Self {
            id: 0,
            node_type: DialogueNodeType::Speech,
            speaker_id: None,
            text: String::new(),
            choices: Vec::new(),
            position: (0.0, 0.0),
            notes: String::new(),
        }
    }
}

/// Dialogue variable for conditions
#[derive(Debug, Clone)]
pub struct DialogueVariable {
    pub name: String,
    pub var_type: VariableType,
    pub default_value: String,
    pub description: String,
}

/// Variable type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VariableType {
    #[default]
    Boolean,
    Integer,
    Float,
    String,
}

impl std::fmt::Display for VariableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl VariableType {
    pub fn all() -> &'static [VariableType] {
        &[
            VariableType::Boolean,
            VariableType::Integer,
            VariableType::Float,
            VariableType::String,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            VariableType::Boolean => "Boolean",
            VariableType::Integer => "Integer",
            VariableType::Float => "Float",
            VariableType::String => "String",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            VariableType::Boolean => "☑️",
            VariableType::Integer => "#️⃣",
            VariableType::Float => "🔢",
            VariableType::String => "[Edit]",
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, VariableType::Integer | VariableType::Float)
    }
}

/// Dialogue graph (conversation tree)
#[derive(Debug, Clone)]
pub struct DialogueGraph {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub start_node_id: Option<u32>,
    pub nodes: Vec<DialogueNode>,
    pub speakers: Vec<DialogueSpeaker>,
    pub variables: Vec<DialogueVariable>,
}

impl Default for DialogueGraph {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Dialogue".to_string(),
            description: String::new(),
            start_node_id: None,
            nodes: Vec::new(),
            speakers: Vec::new(),
            variables: Vec::new(),
        }
    }
}

/// Localization entry
#[derive(Debug, Clone)]
pub struct LocalizationEntry {
    pub key: String,
    pub language: String,
    pub text: String,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DialogueTab {
    #[default]
    Graph,
    Nodes,
    Speakers,
    Variables,
    Localization,
    Preview,
    Validation,
    Export,
    Templates,
}

impl std::fmt::Display for DialogueTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl DialogueTab {
    pub fn all() -> &'static [DialogueTab] {
        &[
            DialogueTab::Graph,
            DialogueTab::Nodes,
            DialogueTab::Speakers,
            DialogueTab::Variables,
            DialogueTab::Localization,
            DialogueTab::Preview,
            DialogueTab::Validation,
            DialogueTab::Export,
            DialogueTab::Templates,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            DialogueTab::Graph => "Graph",
            DialogueTab::Nodes => "Nodes",
            DialogueTab::Speakers => "Speakers",
            DialogueTab::Variables => "Variables",
            DialogueTab::Localization => "Localization",
            DialogueTab::Preview => "Preview",
            DialogueTab::Validation => "Validation",
            DialogueTab::Export => "Export",
            DialogueTab::Templates => "Templates",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DialogueTab::Graph => "📈",
            DialogueTab::Nodes => "[Chat]",
            DialogueTab::Speakers => "👤",
            DialogueTab::Variables => "[Edit]",
            DialogueTab::Localization => "[Net]",
            DialogueTab::Preview => "👁️",
            DialogueTab::Validation => "[ok]",
            DialogueTab::Export => "📤",
            DialogueTab::Templates => "[List]",
        }
    }
}

/// Auto-layout algorithm
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LayoutAlgorithm {
    #[default]
    Hierarchical,
    Radial,
    ForceDirected,
    Tree,
}

impl std::fmt::Display for LayoutAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LayoutAlgorithm {
    pub fn all() -> &'static [LayoutAlgorithm] {
        &[
            LayoutAlgorithm::Hierarchical,
            LayoutAlgorithm::Radial,
            LayoutAlgorithm::ForceDirected,
            LayoutAlgorithm::Tree,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LayoutAlgorithm::Hierarchical => "Hierarchical",
            LayoutAlgorithm::Radial => "Radial",
            LayoutAlgorithm::ForceDirected => "Force Directed",
            LayoutAlgorithm::Tree => "Tree",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LayoutAlgorithm::Hierarchical => "[Chart]",
            LayoutAlgorithm::Radial => "◎",
            LayoutAlgorithm::ForceDirected => "🧲",
            LayoutAlgorithm::Tree => "[Tree]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LayoutAlgorithm::Hierarchical => "Top-down layered layout",
            LayoutAlgorithm::Radial => "Circular layout from center",
            LayoutAlgorithm::ForceDirected => "Physics-based node placement",
            LayoutAlgorithm::Tree => "Branching tree structure",
        }
    }
}

/// Export format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportFormat {
    #[default]
    Json,
    Yarn,
    Ink,
    Xml,
    Csv,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ExportFormat {
    pub fn all() -> &'static [ExportFormat] {
        &[
            ExportFormat::Json,
            ExportFormat::Yarn,
            ExportFormat::Ink,
            ExportFormat::Xml,
            ExportFormat::Csv,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Json => "JSON",
            ExportFormat::Yarn => "Yarn",
            ExportFormat::Ink => "Ink",
            ExportFormat::Xml => "XML",
            ExportFormat::Csv => "CSV",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ExportFormat::Json => "[Doc]",
            ExportFormat::Yarn => "🧶",
            ExportFormat::Ink => "✒️",
            ExportFormat::Xml => "📰",
            ExportFormat::Csv => "[Chart]",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Yarn => "yarn",
            ExportFormat::Ink => "ink",
            ExportFormat::Xml => "xml",
            ExportFormat::Csv => "csv",
        }
    }

    pub fn is_text_format(&self) -> bool {
        true // All dialogue formats are text-based
    }
}

/// Validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub node_id: Option<u32>,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueSeverity {
    #[default]
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl IssueSeverity {
    pub fn all() -> &'static [IssueSeverity] {
        &[
            IssueSeverity::Info,
            IssueSeverity::Warning,
            IssueSeverity::Error,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "Info",
            IssueSeverity::Warning => "Warning",
            IssueSeverity::Error => "Error",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            IssueSeverity::Error => "[x]",
            IssueSeverity::Warning => "[!]",
            IssueSeverity::Info => "ℹ️",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            IssueSeverity::Error => Color32::from_rgb(220, 60, 60),
            IssueSeverity::Warning => Color32::from_rgb(255, 180, 60),
            IssueSeverity::Info => Color32::from_rgb(100, 150, 255),
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, IssueSeverity::Error)
    }
}

/// Dialogue template
#[derive(Debug, Clone)]
pub struct DialogueTemplate {
    pub name: String,
    pub description: String,
    pub node_pattern: Vec<DialogueNodeType>,
}

impl DialogueTemplate {
    pub fn greeting() -> Self {
        Self {
            name: "Greeting".to_string(),
            description: "Basic greeting dialogue with response options".to_string(),
            node_pattern: vec![DialogueNodeType::Speech, DialogueNodeType::Choice],
        }
    }

    pub fn branching_quest() -> Self {
        Self {
            name: "Branching Quest".to_string(),
            description: "Quest dialogue with accept/reject branches".to_string(),
            node_pattern: vec![
                DialogueNodeType::Speech,
                DialogueNodeType::Choice,
                DialogueNodeType::Condition,
                DialogueNodeType::Action,
            ],
        }
    }

    pub fn shop_interaction() -> Self {
        Self {
            name: "Shop Interaction".to_string(),
            description: "Shop keeper dialogue with buy/sell/leave options".to_string(),
            node_pattern: vec![
                DialogueNodeType::Speech,
                DialogueNodeType::Choice,
                DialogueNodeType::Action,
            ],
        }
    }

    pub fn all_templates() -> Vec<Self> {
        vec![
            Self::greeting(),
            Self::branching_quest(),
            Self::shop_interaction(),
        ]
    }
}

/// Search filter
#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub query: String,
    pub search_text: bool,
    pub search_notes: bool,
    pub filter_type: Option<DialogueNodeType>,
    pub filter_speaker: Option<String>,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            query: String::new(),
            search_text: true,
            search_notes: true,
            filter_type: None,
            filter_speaker: None,
        }
    }
}

/// Undo/redo action
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EditorAction {
    AddNode(DialogueNode),
    DeleteNode(u32),
    ModifyNode(u32, DialogueNode),
    AddSpeaker(DialogueSpeaker),
    ModifySpeaker(usize, DialogueSpeaker),
}

impl std::fmt::Display for EditorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EditorAction {
    /// Returns all action variant names
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "AddNode",
            "DeleteNode",
            "ModifyNode",
            "AddSpeaker",
            "ModifySpeaker",
        ]
    }

    /// Returns the name of this action
    pub fn name(&self) -> &'static str {
        match self {
            EditorAction::AddNode(_) => "Add Node",
            EditorAction::DeleteNode(_) => "Delete Node",
            EditorAction::ModifyNode(_, _) => "Modify Node",
            EditorAction::AddSpeaker(_) => "Add Speaker",
            EditorAction::ModifySpeaker(_, _) => "Modify Speaker",
        }
    }

    /// Returns the icon for this action
    pub fn icon(&self) -> &'static str {
        match self {
            EditorAction::AddNode(_) => "+",
            EditorAction::DeleteNode(_) => "[Del]",
            EditorAction::ModifyNode(_, _) => "✏️",
            EditorAction::AddSpeaker(_) => "👤",
            EditorAction::ModifySpeaker(_, _) => "[Edit]",
        }
    }

    /// Returns true if this action affects nodes
    pub fn is_node_action(&self) -> bool {
        matches!(
            self,
            EditorAction::AddNode(_) | EditorAction::DeleteNode(_) | EditorAction::ModifyNode(_, _)
        )
    }

    /// Returns true if this action affects speakers
    pub fn is_speaker_action(&self) -> bool {
        matches!(
            self,
            EditorAction::AddSpeaker(_) | EditorAction::ModifySpeaker(_, _)
        )
    }

    /// Returns true if this is an add action
    pub fn is_add(&self) -> bool {
        matches!(self, EditorAction::AddNode(_) | EditorAction::AddSpeaker(_))
    }

    /// Returns true if this is a delete action
    pub fn is_delete(&self) -> bool {
        matches!(self, EditorAction::DeleteNode(_))
    }

    /// Returns true if this is a modify action
    pub fn is_modify(&self) -> bool {
        matches!(
            self,
            EditorAction::ModifyNode(_, _) | EditorAction::ModifySpeaker(_, _)
        )
    }
}

/// External action events emitted by the dialogue editor panel.
/// These represent high-level user actions that external systems can respond to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DialogueEditorAction {
    /// Export dialogue to file with specified format and path
    ExportDialogue { format: ExportFormat, path: String },
    /// Import dialogue from file path
    ImportDialogue { path: String },
    /// Save current dialogue graph
    SaveGraph { graph_id: u32, name: String },
    /// Load dialogue graph by ID
    LoadGraph { graph_id: u32 },
    /// Create new dialogue graph
    CreateGraph { name: String },
    /// Delete dialogue graph by ID
    DeleteGraph { graph_id: u32 },
    /// Add a speaker to the current graph
    AddSpeaker { speaker: DialogueSpeaker },
    /// Apply layout algorithm to graph
    ApplyLayout { algorithm: LayoutAlgorithm },
    /// Start dialogue preview from node
    StartPreview { node_id: u32 },
    /// Set the current localization language
    SetLanguage { language: String },
    /// Toggle collaboration mode
    SetCollaboration { enabled: bool },
    /// Run validation on current graph
    RunValidation,
}

impl std::fmt::Display for DialogueEditorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl DialogueEditorAction {
    /// Returns the name of this action
    pub fn name(&self) -> &'static str {
        match self {
            DialogueEditorAction::ExportDialogue { .. } => "Export Dialogue",
            DialogueEditorAction::ImportDialogue { .. } => "Import Dialogue",
            DialogueEditorAction::SaveGraph { .. } => "Save Graph",
            DialogueEditorAction::LoadGraph { .. } => "Load Graph",
            DialogueEditorAction::CreateGraph { .. } => "Create Graph",
            DialogueEditorAction::DeleteGraph { .. } => "Delete Graph",
            DialogueEditorAction::AddSpeaker { .. } => "Add Speaker",
            DialogueEditorAction::ApplyLayout { .. } => "Apply Layout",
            DialogueEditorAction::StartPreview { .. } => "Start Preview",
            DialogueEditorAction::SetLanguage { .. } => "Set Language",
            DialogueEditorAction::SetCollaboration { .. } => "Set Collaboration",
            DialogueEditorAction::RunValidation => "Run Validation",
        }
    }

    /// Returns true if this is an export/import action
    pub fn is_io_action(&self) -> bool {
        matches!(
            self,
            DialogueEditorAction::ExportDialogue { .. }
                | DialogueEditorAction::ImportDialogue { .. }
        )
    }

    /// Returns true if this is a graph management action
    pub fn is_graph_action(&self) -> bool {
        matches!(
            self,
            DialogueEditorAction::SaveGraph { .. }
                | DialogueEditorAction::LoadGraph { .. }
                | DialogueEditorAction::CreateGraph { .. }
                | DialogueEditorAction::DeleteGraph { .. }
        )
    }

    /// Returns true if this affects the layout
    pub fn is_layout_action(&self) -> bool {
        matches!(self, DialogueEditorAction::ApplyLayout { .. })
    }
}

/// Collaboration state
#[derive(Debug, Clone)]
pub struct CollaborationState {
    pub enabled: bool,
    pub current_user: String,
    pub active_users: Vec<String>,
    pub locked_nodes: Vec<(u32, String)>, // (node_id, user)
}

/// Main Dialogue Editor Panel
pub struct DialogueEditorPanel {
    // Tab state
    active_tab: DialogueTab,

    // Graph data
    graphs: Vec<DialogueGraph>,
    selected_graph: Option<u32>,
    current_graph: DialogueGraph,

    // Selection
    selected_node: Option<u32>,
    selected_speaker: Option<usize>,

    // Editing state
    editing_node: bool,
    zoom_level: f32,
    pan_offset: (f32, f32),

    // Preview state
    preview_node_id: Option<u32>,
    preview_history: Vec<u32>,

    // Localization
    current_language: String,
    available_languages: Vec<String>,
    localization_entries: Vec<LocalizationEntry>,

    // Validation
    validation_issues: Vec<ValidationIssue>,
    auto_validate: bool,

    // Search
    search_filter: SearchFilter,
    search_results: Vec<u32>,

    // Undo/Redo
    undo_stack: Vec<EditorAction>,
    redo_stack: Vec<EditorAction>,
    max_undo_steps: usize,

    // Templates
    templates: Vec<DialogueTemplate>,
    selected_template: Option<usize>,

    // Export/Import
    export_format: ExportFormat,
    export_path: String,
    import_path: String,

    // Collaboration
    collaboration: CollaborationState,

    // Layout
    layout_algorithm: LayoutAlgorithm,
    auto_layout_on_add: bool,

    // Statistics
    total_word_count: usize,
    avg_branch_factor: f32,

    // ID counter
    next_id: u32,

    // Pending actions for external event handling
    pending_actions: Vec<DialogueEditorAction>,
}

impl Default for DialogueEditorPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: DialogueTab::Graph,

            graphs: Vec::new(),
            selected_graph: None,
            current_graph: DialogueGraph::default(),

            selected_node: None,
            selected_speaker: None,

            editing_node: false,
            zoom_level: 1.0,
            pan_offset: (0.0, 0.0),

            preview_node_id: None,
            preview_history: Vec::new(),

            current_language: "en".to_string(),
            available_languages: vec![
                "en".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "ja".to_string(),
            ],
            localization_entries: Vec::new(),

            validation_issues: Vec::new(),
            auto_validate: true,

            search_filter: SearchFilter::default(),
            search_results: Vec::new(),

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_steps: 50,

            templates: DialogueTemplate::all_templates(),
            selected_template: None,

            export_format: ExportFormat::Json,
            export_path: "dialogue_export.json".to_string(),
            import_path: String::new(),

            collaboration: CollaborationState {
                enabled: false,
                current_user: "Editor".to_string(),
                active_users: vec![],
                locked_nodes: vec![],
            },

            layout_algorithm: LayoutAlgorithm::Hierarchical,
            auto_layout_on_add: false,

            total_word_count: 0,
            avg_branch_factor: 0.0,

            next_id: 1,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl DialogueEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions, leaving the queue empty.
    /// External systems should call this each frame to retrieve actions.
    pub fn take_actions(&mut self) -> Vec<DialogueEditorAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions to process.
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external handling.
    fn queue_action(&mut self, action: DialogueEditorAction) {
        self.pending_actions.push(action);
    }

    /// Returns the current export format.
    pub fn export_format(&self) -> ExportFormat {
        self.export_format
    }

    /// Returns the current export path.
    pub fn export_path(&self) -> &str {
        &self.export_path
    }

    /// Returns the current import path.
    pub fn import_path(&self) -> &str {
        &self.import_path
    }

    /// Returns the current layout algorithm.
    pub fn layout_algorithm(&self) -> LayoutAlgorithm {
        self.layout_algorithm
    }

    /// Returns the current language.
    pub fn current_language(&self) -> &str {
        &self.current_language
    }

    /// Returns whether collaboration is enabled.
    pub fn collaboration_enabled(&self) -> bool {
        self.collaboration.enabled
    }

    fn create_sample_dialogue(&mut self) {
        // Create sample speaker
        self.current_graph.speakers.push(DialogueSpeaker {
            id: "npc_merchant".to_string(),
            name: "Merchant".to_string(),
            portrait: "portraits/merchant.png".to_string(),
            voice_id: "voice_merchant_01".to_string(),
            color: Color32::from_rgb(255, 200, 100),
        });

        self.current_graph.speakers.push(DialogueSpeaker {
            id: "player".to_string(),
            name: "Player".to_string(),
            portrait: "portraits/player.png".to_string(),
            voice_id: String::new(),
            color: Color32::from_rgb(100, 200, 255),
        });

        // Create sample nodes
        let start_id = self.next_id();
        self.current_graph.nodes.push(DialogueNode {
            id: start_id,
            node_type: DialogueNodeType::Speech,
            speaker_id: Some("npc_merchant".to_string()),
            text: "Welcome, traveler! Looking to buy something?".to_string(),
            choices: vec![
                DialogueChoice {
                    text: "Show me your wares.".to_string(),
                    target_node_id: Some(start_id + 1),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "Just browsing.".to_string(),
                    target_node_id: Some(start_id + 2),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "Goodbye.".to_string(),
                    target_node_id: None,
                    ..Default::default()
                },
            ],
            position: (100.0, 100.0),
            notes: "Entry point".to_string(),
        });
        self.next_id += 1;

        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 1,
            node_type: DialogueNodeType::Action,
            speaker_id: None,
            text: "[Opens shop UI]".to_string(),
            choices: vec![],
            position: (300.0, 50.0),
            notes: "Opens shop".to_string(),
        });
        self.next_id += 1;

        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 2,
            node_type: DialogueNodeType::Speech,
            speaker_id: Some("npc_merchant".to_string()),
            text: "Take your time! Let me know if you need anything.".to_string(),
            choices: vec![],
            position: (300.0, 150.0),
            notes: String::new(),
        });
        self.next_id += 1;

        self.current_graph.start_node_id = Some(start_id);
        self.current_graph.name = "Merchant Greeting".to_string();
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (DialogueTab::Graph, "Graph"),
                (DialogueTab::Nodes, "[Edit] Nodes"),
                (DialogueTab::Speakers, "👤 Speakers"),
                (DialogueTab::Variables, "Variables"),
                (DialogueTab::Localization, "[Glb] Localization"),
                (DialogueTab::Preview, "> Preview"),
                (DialogueTab::Validation, "✓ Validation"),
                (DialogueTab::Export, "Export"),
                (DialogueTab::Templates, "Templates"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;

                // Show validation badge
                let display_label = if matches!(tab, DialogueTab::Validation)
                    && !self.validation_issues.is_empty()
                {
                    let error_count = self
                        .validation_issues
                        .iter()
                        .filter(|i| matches!(i.severity, IssueSeverity::Error))
                        .count();
                    let warning_count = self
                        .validation_issues
                        .iter()
                        .filter(|i| matches!(i.severity, IssueSeverity::Warning))
                        .count();
                    if error_count > 0 {
                        format!("{} ({})", label, error_count)
                    } else if warning_count > 0 {
                        format!("{} ({})", label, warning_count)
                    } else {
                        label.to_string()
                    }
                } else {
                    label.to_string()
                };

                let button = egui::Button::new(display_label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Graph info
        ui.horizontal(|ui| {
            ui.label(format!("{}", self.current_graph.name));
            ui.label(format!("| {} nodes", self.current_graph.nodes.len()));
            ui.label(format!("| {} speakers", self.current_graph.speakers.len()));

            // Show statistics
            if self.total_word_count > 0 {
                ui.label(format!("| {} words", self.total_word_count));
            }
            if self.avg_branch_factor > 0.0 {
                ui.label(format!("| {:.1} avg branches", self.avg_branch_factor));
            }

            // Show validation status
            if !self.validation_issues.is_empty() {
                let error_count = self
                    .validation_issues
                    .iter()
                    .filter(|i| matches!(i.severity, IssueSeverity::Error))
                    .count();
                let warning_count = self
                    .validation_issues
                    .iter()
                    .filter(|i| matches!(i.severity, IssueSeverity::Warning))
                    .count();
                if error_count > 0 {
                    ui.colored_label(
                        Color32::from_rgb(255, 100, 100),
                        format!("| {} errors", error_count),
                    );
                }
                if warning_count > 0 {
                    ui.colored_label(
                        Color32::from_rgb(255, 200, 100),
                        format!("| {} warnings", warning_count),
                    );
                }
            } else {
                ui.colored_label(Color32::from_rgb(100, 255, 100), "| ✓ Valid");
            }
        });

        ui.separator();
    }

    fn show_graph_tab(&mut self, ui: &mut Ui) {
        ui.heading("Dialogue Graph");
        ui.add_space(5.0);

        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("+ New Node").clicked() {
                let id = self.next_id();
                info!(node_id = id, "Dialogue: added new node from graph tab");
                self.current_graph.nodes.push(DialogueNode {
                    id,
                    position: (200.0, 200.0),
                    ..Default::default()
                });
            }

            ui.separator();

            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom_level, 0.25..=2.0).show_value(false));

            if ui.button("Reset View").clicked() {
                self.zoom_level = 1.0;
                self.pan_offset = (0.0, 0.0);
            }
        });

        ui.add_space(10.0);

        // Graph canvas
        self.draw_graph_canvas(ui);

        // Node details panel
        if let Some(node_id) = self.selected_node {
            ui.add_space(10.0);
            self.show_node_details(ui, node_id);
        }
    }

    fn draw_graph_canvas(&mut self, ui: &mut Ui) {
        let available_size = ui.available_size();
        let canvas_size = Vec2::new(available_size.x, 250.0);

        let (rect, response) = ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());

        let painter = ui.painter();

        // Background
        painter.rect_filled(rect, 5.0, Color32::from_rgb(25, 25, 30));

        // Grid
        let grid_spacing = 50.0 * self.zoom_level;
        let grid_color = Color32::from_rgb(40, 40, 45);

        let mut x = rect.min.x + (self.pan_offset.0 % grid_spacing);
        while x < rect.max.x {
            painter.line_segment(
                [
                    egui::Pos2::new(x, rect.min.y),
                    egui::Pos2::new(x, rect.max.y),
                ],
                egui::Stroke::new(1.0, grid_color),
            );
            x += grid_spacing;
        }

        let mut y = rect.min.y + (self.pan_offset.1 % grid_spacing);
        while y < rect.max.y {
            painter.line_segment(
                [
                    egui::Pos2::new(rect.min.x, y),
                    egui::Pos2::new(rect.max.x, y),
                ],
                egui::Stroke::new(1.0, grid_color),
            );
            y += grid_spacing;
        }

        // Draw connections first (behind nodes)
        for node in &self.current_graph.nodes {
            let node_pos = egui::Pos2::new(
                rect.min.x + node.position.0 * self.zoom_level + self.pan_offset.0,
                rect.min.y + node.position.1 * self.zoom_level + self.pan_offset.1,
            );

            for choice in &node.choices {
                if let Some(target_id) = choice.target_node_id {
                    if let Some(target_node) =
                        self.current_graph.nodes.iter().find(|n| n.id == target_id)
                    {
                        let target_pos = egui::Pos2::new(
                            rect.min.x
                                + target_node.position.0 * self.zoom_level
                                + self.pan_offset.0,
                            rect.min.y
                                + target_node.position.1 * self.zoom_level
                                + self.pan_offset.1,
                        );

                        // Draw curved connection line
                        let ctrl1 = egui::Pos2::new(node_pos.x + 50.0, node_pos.y);
                        let ctrl2 = egui::Pos2::new(target_pos.x - 50.0, target_pos.y);

                        painter.add(egui::Shape::CubicBezier(
                            egui::epaint::CubicBezierShape::from_points_stroke(
                                [node_pos, ctrl1, ctrl2, target_pos],
                                false,
                                Color32::TRANSPARENT,
                                egui::Stroke::new(2.0, Color32::from_rgb(150, 150, 150)),
                            ),
                        ));
                    }
                }
            }
        }

        // Draw nodes
        let node_ids: Vec<u32> = self.current_graph.nodes.iter().map(|n| n.id).collect();
        for node_id in node_ids {
            if let Some(node) = self.current_graph.nodes.iter().find(|n| n.id == node_id) {
                let node_pos = egui::Pos2::new(
                    rect.min.x + node.position.0 * self.zoom_level + self.pan_offset.0,
                    rect.min.y + node.position.1 * self.zoom_level + self.pan_offset.1,
                );

                let node_size = Vec2::new(120.0 * self.zoom_level, 40.0 * self.zoom_level);
                let node_rect = egui::Rect::from_center_size(node_pos, node_size);

                // Check if visible
                if !rect.intersects(node_rect) {
                    continue;
                }

                let is_selected = self.selected_node == Some(node.id);
                let is_start = self.current_graph.start_node_id == Some(node.id);

                // Node background
                let bg_color = if is_selected {
                    Color32::from_rgb(80, 80, 100)
                } else {
                    Color32::from_rgb(50, 50, 60)
                };

                painter.rect_filled(node_rect, 8.0, bg_color);

                // Type indicator bar
                let type_bar = egui::Rect::from_min_size(
                    node_rect.min,
                    Vec2::new(node_rect.width(), 6.0 * self.zoom_level),
                );
                painter.rect_filled(
                    type_bar,
                    egui::CornerRadius {
                        nw: 8,
                        ne: 8,
                        sw: 0,
                        se: 0,
                    },
                    node.node_type.color(),
                );

                // Start node indicator
                if is_start {
                    painter.rect_stroke(
                        node_rect,
                        8.0,
                        egui::Stroke::new(2.0, Color32::GREEN),
                        egui::StrokeKind::Outside,
                    );
                }

                // Selection indicator
                if is_selected {
                    painter.rect_stroke(
                        node_rect.expand(2.0),
                        10.0,
                        egui::Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                        egui::StrokeKind::Outside,
                    );
                }

                // Node label
                let label = if node.text.len() > 15 {
                    format!("{} {}...", node.node_type.icon(), &node.text[..12])
                } else if node.text.is_empty() {
                    format!("{} {:?}", node.node_type.icon(), node.node_type)
                } else {
                    format!("{} {}", node.node_type.icon(), node.text)
                };

                painter.text(
                    node_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(10.0 * self.zoom_level),
                    Color32::WHITE,
                );
            }
        }

        // Handle click to select node
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.selected_node = None;
                for node in &self.current_graph.nodes {
                    let node_pos = egui::Pos2::new(
                        rect.min.x + node.position.0 * self.zoom_level + self.pan_offset.0,
                        rect.min.y + node.position.1 * self.zoom_level + self.pan_offset.1,
                    );
                    let node_rect = egui::Rect::from_center_size(
                        node_pos,
                        Vec2::new(120.0 * self.zoom_level, 40.0 * self.zoom_level),
                    );
                    if node_rect.contains(pos) {
                        self.selected_node = Some(node.id);
                        break;
                    }
                }
            }
        }

        // Handle drag to pan
        if response.dragged() {
            self.pan_offset.0 += response.drag_delta().x;
            self.pan_offset.1 += response.drag_delta().y;
        }
    }

    fn show_node_details(&mut self, ui: &mut Ui, node_id: u32) {
        if let Some(node) = self
            .current_graph
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
        {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!("{} Node #{}", node.node_type.icon(), node.id)).strong(),
                );

                egui::Grid::new("node_details_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Type:");
                        egui::ComboBox::from_id_salt("node_type")
                            .selected_text(format!(
                                "{} {:?}",
                                node.node_type.icon(),
                                node.node_type
                            ))
                            .show_ui(ui, |ui| {
                                for t in DialogueNodeType::all() {
                                    ui.selectable_value(
                                        &mut node.node_type,
                                        *t,
                                        format!("{} {:?}", t.icon(), t),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Text:");
                        ui.text_edit_multiline(&mut node.text);
                        ui.end_row();

                        ui.label("Notes:");
                        ui.text_edit_singleline(&mut node.notes);
                        ui.end_row();
                    });

                // Choices
                if !node.choices.is_empty() || node.node_type == DialogueNodeType::Choice {
                    ui.add_space(5.0);
                    ui.label(RichText::new("Choices").strong());
                    for (i, choice) in node.choices.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", i + 1));
                            ui.label(&choice.text);
                            if let Some(target) = choice.target_node_id {
                                ui.label(format!("→ #{}", target));
                            }
                        });
                    }
                }
            });
        }
    }

    fn show_nodes_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Edit] All Nodes");
        ui.add_space(10.0);

        // Add node button
        if ui.button("+ Add Node").clicked() {
            let id = self.next_id();
            info!(node_id = id, "Dialogue: added node from nodes tab");
            self.current_graph.nodes.push(DialogueNode {
                id,
                position: (100.0, 100.0),
                ..Default::default()
            });
        }

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for node in &self.current_graph.nodes {
                    let is_selected = self.selected_node == Some(node.id);
                    let is_start = self.current_graph.start_node_id == Some(node.id);

                    ui.horizontal(|ui| {
                        // Type icon with color
                        ui.label(
                            RichText::new(node.node_type.icon()).color(node.node_type.color()),
                        );

                        // Node info
                        let label = format!(
                            "#{} - {}",
                            node.id,
                            if node.text.is_empty() {
                                format!("{:?}", node.node_type)
                            } else {
                                node.text.chars().take(30).collect::<String>()
                            }
                        );

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_node = Some(node.id);
                        }

                        if is_start {
                            ui.label(RichText::new("START").color(Color32::GREEN).small());
                        }

                        // Choice count
                        if !node.choices.is_empty() {
                            ui.label(
                                RichText::new(format!("({} choices)", node.choices.len()))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                }
            });
    }

    fn show_speakers_tab(&mut self, ui: &mut Ui) {
        ui.heading("👤 Speakers");
        ui.add_space(10.0);

        // Add speaker button
        if ui.button("+ Add Speaker").clicked() {
            self.current_graph.speakers.push(DialogueSpeaker {
                id: format!("speaker_{}", self.current_graph.speakers.len() + 1),
                name: "New Speaker".to_string(),
                ..Default::default()
            });
        }

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for (idx, speaker) in self.current_graph.speakers.iter_mut().enumerate() {
                    let is_selected = self.selected_speaker == Some(idx);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Color indicator
                            let color_rect = ui
                                .allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover())
                                .0;
                            ui.painter().rect_filled(color_rect, 3.0, speaker.color);

                            if ui.selectable_label(is_selected, &speaker.name).clicked() {
                                self.selected_speaker = Some(idx);
                            }

                            ui.label(RichText::new(&speaker.id).small().color(Color32::GRAY));
                        });

                        if is_selected {
                            egui::Grid::new(format!("speaker_{}", idx))
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("ID:");
                                    ui.text_edit_singleline(&mut speaker.id);
                                    ui.end_row();

                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut speaker.name);
                                    ui.end_row();

                                    ui.label("Portrait:");
                                    ui.text_edit_singleline(&mut speaker.portrait);
                                    ui.end_row();

                                    ui.label("Voice ID:");
                                    ui.text_edit_singleline(&mut speaker.voice_id);
                                    ui.end_row();
                                });
                        }
                    });
                }
            });
    }

    fn show_variables_tab(&mut self, ui: &mut Ui) {
        ui.heading("Variables");
        ui.add_space(10.0);

        // Add variable button
        if ui.button("+ Add Variable").clicked() {
            let var_name = format!("var_{}", self.current_graph.variables.len() + 1);
            info!(variable = %var_name, "Dialogue: added condition variable");
            self.current_graph.variables.push(DialogueVariable {
                name: var_name,
                var_type: VariableType::Boolean,
                default_value: "false".to_string(),
                description: String::new(),
            });
        }

        ui.add_space(10.0);

        if self.current_graph.variables.is_empty() {
            ui.label("No variables defined. Variables can be used for conditions and branching.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for var in &mut self.current_graph.variables {
                        ui.group(|ui| {
                            egui::Grid::new(&var.name)
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut var.name);
                                    ui.end_row();

                                    ui.label("Type:");
                                    egui::ComboBox::from_id_salt(format!("type_{}", var.name))
                                        .selected_text(format!("{:?}", var.var_type))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut var.var_type,
                                                VariableType::Boolean,
                                                "Boolean",
                                            );
                                            ui.selectable_value(
                                                &mut var.var_type,
                                                VariableType::Integer,
                                                "Integer",
                                            );
                                            ui.selectable_value(
                                                &mut var.var_type,
                                                VariableType::Float,
                                                "Float",
                                            );
                                            ui.selectable_value(
                                                &mut var.var_type,
                                                VariableType::String,
                                                "String",
                                            );
                                        });
                                    ui.end_row();

                                    ui.label("Default:");
                                    ui.text_edit_singleline(&mut var.default_value);
                                    ui.end_row();
                                });
                        });
                    }
                });
        }
    }

    fn show_localization_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Glb] Localization");
        ui.add_space(10.0);

        // Language selector
        ui.horizontal(|ui| {
            ui.label("Language:");
            egui::ComboBox::from_id_salt("language_select")
                .selected_text(&self.current_language)
                .show_ui(ui, |ui| {
                    for lang in &self.available_languages.clone() {
                        ui.selectable_value(&mut self.current_language, lang.clone(), lang);
                    }
                });

            if ui.button("+ Add Language").clicked() {
                // Add new language
            }
        });

        ui.add_space(10.0);

        // Localization entries
        ui.group(|ui| {
            ui.label(RichText::new("Text Entries").strong());

            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for node in &self.current_graph.nodes {
                        if !node.text.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Node #{}:", node.id));
                                ui.label(&node.text);
                            });
                        }
                    }
                });
        });
    }

    fn show_preview_tab(&mut self, ui: &mut Ui) {
        ui.heading("> Dialogue Preview");
        ui.add_space(10.0);

        // Controls
        ui.horizontal(|ui| {
            if ui.button("⏮ Start").clicked() {
                self.preview_node_id = self.current_graph.start_node_id;
                self.preview_history.clear();
            }
            if ui.button("<< Back").clicked() {
                if let Some(prev_id) = self.preview_history.pop() {
                    self.preview_node_id = Some(prev_id);
                }
            }
        });

        ui.add_space(10.0);

        // Current node display
        if let Some(node_id) = self.preview_node_id {
            if let Some(node) = self.current_graph.nodes.iter().find(|n| n.id == node_id) {
                // Speaker name
                if let Some(ref speaker_id) = node.speaker_id {
                    if let Some(speaker) = self
                        .current_graph
                        .speakers
                        .iter()
                        .find(|s| &s.id == speaker_id)
                    {
                        ui.horizontal(|ui| {
                            let color_rect = ui
                                .allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover())
                                .0;
                            ui.painter().rect_filled(color_rect, 3.0, speaker.color);
                            ui.label(
                                RichText::new(&speaker.name)
                                    .strong()
                                    .size(16.0)
                                    .color(speaker.color),
                            );
                        });
                    }
                }

                // Dialogue text
                ui.group(|ui| {
                    ui.label(RichText::new(&node.text).size(14.0));
                });

                // Choices
                if !node.choices.is_empty() {
                    ui.add_space(10.0);
                    for choice in &node.choices {
                        if ui.button(&choice.text).clicked() {
                            if let Some(target_id) = choice.target_node_id {
                                self.preview_history.push(node_id);
                                self.preview_node_id = Some(target_id);
                            } else {
                                self.preview_node_id = None;
                            }
                        }
                    }
                } else {
                    ui.label(RichText::new("[End of conversation]").color(Color32::GRAY));
                }
            }
        } else {
            ui.label("Click 'Start' to preview the dialogue from the beginning.");
        }
    }

    // Getters for testing
    pub fn node_count(&self) -> usize {
        self.current_graph.nodes.len()
    }

    pub fn speaker_count(&self) -> usize {
        self.current_graph.speakers.len()
    }

    pub fn variable_count(&self) -> usize {
        self.current_graph.variables.len()
    }

    pub fn graph_name(&self) -> &str {
        &self.current_graph.name
    }

    pub fn selected_node(&self) -> Option<u32> {
        self.selected_node
    }

    pub fn add_node(&mut self, node_type: DialogueNodeType) -> u32 {
        let id = self.next_id();
        info!(node_id = id, node_type = ?node_type, "Dialogue: added node");
        self.current_graph.nodes.push(DialogueNode {
            id,
            node_type,
            position: (100.0, 100.0),
            ..Default::default()
        });
        id
    }

    pub fn add_speaker(&mut self, id: &str, name: &str) {
        self.current_graph.speakers.push(DialogueSpeaker {
            id: id.to_string(),
            name: name.to_string(),
            ..Default::default()
        });
    }

    pub fn set_start_node(&mut self, node_id: u32) {
        self.current_graph.start_node_id = Some(node_id);
    }

    pub fn select_node(&mut self, node_id: u32) {
        self.selected_node = Some(node_id);
    }

    // === New Tab Implementation Methods ===

    fn show_validation_tab(&mut self, ui: &mut Ui) {
        ui.heading("✓ Dialogue Validation");
        ui.add_space(10.0);

        // Validation controls
        ui.horizontal(|ui| {
            if ui.button("Run Validation").clicked() {
                info!("Dialogue: running validation");
                self.validate_graph();
            }
            ui.checkbox(&mut self.auto_validate, "Auto-validate");
            if ui.button("Clear Issues").clicked() {
                self.validation_issues.clear();
            }
        });

        ui.separator();

        // Issue summary
        let error_count = self
            .validation_issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Error))
            .count();
        let warning_count = self
            .validation_issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Warning))
            .count();
        let info_count = self
            .validation_issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Info))
            .count();

        ui.horizontal(|ui| {
            if error_count > 0 {
                ui.colored_label(
                    Color32::from_rgb(255, 100, 100),
                    format!("{} Errors", error_count),
                );
            }
            if warning_count > 0 {
                ui.colored_label(
                    Color32::from_rgb(255, 200, 100),
                    format!("{} Warnings", warning_count),
                );
            }
            if info_count > 0 {
                ui.colored_label(
                    Color32::from_rgb(100, 200, 255),
                    format!("{} Info", info_count),
                );
            }
            if self.validation_issues.is_empty() {
                ui.colored_label(Color32::from_rgb(100, 255, 100), "✓ No Issues Found");
            }
        });

        ui.separator();

        // Issues list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, issue) in self.validation_issues.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(issue.severity.color(), issue.severity.icon());
                        ui.label(RichText::new(&issue.message).strong());
                    });
                    if let Some(node_id) = issue.node_id {
                        ui.horizontal(|ui| {
                            ui.label(format!("Node: {}", node_id));
                            if ui.button("Go to Node").clicked() {
                                self.selected_node = Some(node_id);
                                self.active_tab = DialogueTab::Graph;
                            }
                        });
                    }
                    if !issue.suggestion.is_empty() {
                        ui.label(
                            RichText::new(format!("{}", issue.suggestion))
                                .color(Color32::from_rgb(200, 200, 100)),
                        );
                    }
                });
                if idx < self.validation_issues.len() - 1 {
                    ui.add_space(5.0);
                }
            }
        });
    }

    fn show_export_tab(&mut self, ui: &mut Ui) {
        ui.heading("Export & Import");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("export_format")
                .selected_text(format!("{:?}", self.export_format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.export_format, ExportFormat::Json, "JSON");
                    ui.selectable_value(&mut self.export_format, ExportFormat::Yarn, "Yarn");
                    ui.selectable_value(&mut self.export_format, ExportFormat::Ink, "Ink");
                    ui.selectable_value(&mut self.export_format, ExportFormat::Xml, "XML");
                    ui.selectable_value(&mut self.export_format, ExportFormat::Csv, "CSV");
                });
        });

        ui.separator();

        // Export section
        ui.heading("Export Dialogue");
        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.export_path);
        });
        if ui.button("📤 Export").clicked() {
            info!(format = ?self.export_format, path = %self.export_path, "Dialogue: exporting");
            self.export_dialogue();
        }

        ui.separator();

        // Import section
        ui.heading("Import Dialogue");
        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.import_path);
        });
        if ui.button("Import").clicked() {
            info!(path = %self.import_path, "Dialogue: importing");
            self.import_dialogue();
        }

        ui.separator();

        // Statistics
        ui.heading("Dialogue Statistics");
        ui.label(format!("Total nodes: {}", self.current_graph.nodes.len()));
        ui.label(format!(
            "Total speakers: {}",
            self.current_graph.speakers.len()
        ));
        ui.label(format!(
            "Total variables: {}",
            self.current_graph.variables.len()
        ));
        ui.label(format!("Word count: {}", self.total_word_count));
        ui.label(format!(
            "Average branch factor: {:.2}",
            self.avg_branch_factor
        ));
    }

    fn show_templates_tab(&mut self, ui: &mut Ui) {
        ui.heading("Dialogue Templates");
        ui.add_space(10.0);

        ui.label("Select a template to quickly create common dialogue patterns:");
        ui.separator();

        // Track which template to apply
        let mut template_to_apply: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, template) in self.templates.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&template.name).strong().size(16.0));
                        if ui.button("Apply Template").clicked() {
                            template_to_apply = Some(idx);
                        }
                    });
                    ui.label(&template.description);

                    // Show template preview
                    ui.collapsing("Preview Structure", |ui| {
                        ui.label(format!("Nodes: {}", template.node_pattern.len()));
                        ui.label("Pattern:");
                        let pattern_str = template
                            .node_pattern
                            .iter()
                            .map(|t| format!("{:?}", t))
                            .collect::<Vec<_>>()
                            .join(" → ");
                        ui.monospace(&pattern_str);
                    });
                });
                ui.add_space(5.0);
            }
        });

        // Apply template after the borrow
        if let Some(idx) = template_to_apply {
            self.apply_template(idx);
        }

        ui.separator();

        // Custom template creation
        ui.heading("Create Custom Template");
        if ui.button("Save Current as Template").clicked() {
            let template = DialogueTemplate {
                name: format!("Custom - {}", self.current_graph.name),
                description: format!(
                    "Custom template with {} nodes",
                    self.current_graph.nodes.len()
                ),
                node_pattern: self
                    .current_graph
                    .nodes
                    .iter()
                    .map(|n| n.node_type)
                    .collect(),
            };
            self.templates.push(template);
        }
    }

    // === Validation Methods ===

    fn validate_graph(&mut self) {
        self.validation_issues.clear();

        // Check for start node
        if self.current_graph.start_node_id.is_none() {
            warn!("Dialogue validation: no start node defined");
            self.validation_issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: "No start node defined".to_string(),
                node_id: None,
                suggestion: "Set a start node using right-click on a node".to_string(),
            });
        }

        // Check for unreachable nodes
        let mut reachable = std::collections::HashSet::new();
        if let Some(start_id) = self.current_graph.start_node_id {
            self.mark_reachable(start_id, &mut reachable);
        }

        for node in &self.current_graph.nodes {
            if !reachable.contains(&node.id) && self.current_graph.start_node_id.is_some() {
                self.validation_issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Node {} is unreachable", node.id),
                    node_id: Some(node.id),
                    suggestion: "Add a connection from a reachable node".to_string(),
                });
            }

            // Check for empty text in speech nodes
            if matches!(node.node_type, DialogueNodeType::Speech) && node.text.trim().is_empty() {
                self.validation_issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!("Speech node {} has empty text", node.id),
                    node_id: Some(node.id),
                    suggestion: "Add dialogue text to this speech node".to_string(),
                });
            }

            // Check for missing speaker in speech nodes
            if matches!(node.node_type, DialogueNodeType::Speech) && node.speaker_id.is_none() {
                self.validation_issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Speech node {} has no speaker assigned", node.id),
                    node_id: Some(node.id),
                    suggestion: "Assign a speaker to this speech node".to_string(),
                });
            }

            // Check for invalid speaker references
            if let Some(ref speaker_id) = node.speaker_id {
                if !self
                    .current_graph
                    .speakers
                    .iter()
                    .any(|s| &s.id == speaker_id)
                {
                    self.validation_issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        message: format!(
                            "Node {} references unknown speaker '{}'",
                            node.id, speaker_id
                        ),
                        node_id: Some(node.id),
                        suggestion: "Create the speaker or assign a valid speaker".to_string(),
                    });
                }
            }

            // Check for invalid choice targets
            for (choice_idx, choice) in node.choices.iter().enumerate() {
                if let Some(target_id) = choice.target_node_id {
                    if !self.current_graph.nodes.iter().any(|n| n.id == target_id) {
                        self.validation_issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            message: format!(
                                "Node {} choice {} points to non-existent node {}",
                                node.id, choice_idx, target_id
                            ),
                            node_id: Some(node.id),
                            suggestion: "Remove this choice or point it to a valid node"
                                .to_string(),
                        });
                    }
                }

                // Check for empty choice text
                if choice.text.trim().is_empty() {
                    self.validation_issues.push(ValidationIssue {
                        severity: IssueSeverity::Warning,
                        message: format!("Node {} has a choice with empty text", node.id),
                        node_id: Some(node.id),
                        suggestion: "Add text to the choice or remove it".to_string(),
                    });
                }
            }
        }

        // Check for duplicate speaker IDs
        let mut seen_speakers = std::collections::HashSet::new();
        for speaker in &self.current_graph.speakers {
            if !seen_speakers.insert(&speaker.id) {
                self.validation_issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!("Duplicate speaker ID: '{}'", speaker.id),
                    node_id: None,
                    suggestion: "Rename one of the speakers with this ID".to_string(),
                });
            }
        }

        // Info messages
        if self.current_graph.nodes.is_empty() {
            self.validation_issues.push(ValidationIssue {
                severity: IssueSeverity::Info,
                message: "Dialogue graph is empty".to_string(),
                node_id: None,
                suggestion: "Add nodes to create your dialogue".to_string(),
            });
        }
    }

    fn mark_reachable(&self, node_id: u32, reachable: &mut std::collections::HashSet<u32>) {
        if !reachable.insert(node_id) {
            return; // Already visited
        }

        if let Some(node) = self.current_graph.nodes.iter().find(|n| n.id == node_id) {
            for choice in &node.choices {
                if let Some(target_id) = choice.target_node_id {
                    self.mark_reachable(target_id, reachable);
                }
            }
        }
    }

    // === Search Methods ===

    fn search_nodes(&mut self) {
        self.search_results.clear();

        if self.search_filter.query.is_empty() {
            return;
        }

        let query_lower = self.search_filter.query.to_lowercase();

        for node in &self.current_graph.nodes {
            let mut matches = false;

            // Search in text
            if self.search_filter.search_text && node.text.to_lowercase().contains(&query_lower) {
                matches = true;
            }

            // Search in notes
            if self.search_filter.search_notes && node.notes.to_lowercase().contains(&query_lower) {
                matches = true;
            }

            // Filter by type
            if let Some(ref filter_type) = self.search_filter.filter_type {
                if !std::mem::discriminant(&node.node_type).eq(&std::mem::discriminant(filter_type))
                {
                    matches = false;
                }
            }

            // Filter by speaker
            if let Some(ref filter_speaker) = self.search_filter.filter_speaker {
                if node.speaker_id.as_ref() != Some(filter_speaker) {
                    matches = false;
                }
            }

            if matches {
                self.search_results.push(node.id);
            }
        }
    }

    // === Undo/Redo Methods ===

    fn push_undo(&mut self, action: EditorAction) {
        if self.undo_stack.len() >= self.max_undo_steps {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match &action {
                EditorAction::AddNode(node) => {
                    self.current_graph.nodes.retain(|n| n.id != node.id);
                }
                EditorAction::DeleteNode(_id) => {
                    // Cannot restore without storing the deleted node
                }
                EditorAction::ModifyNode(_id, old_node) => {
                    if let Some(node) = self
                        .current_graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == old_node.id)
                    {
                        *node = old_node.clone();
                    }
                }
                EditorAction::AddSpeaker(speaker) => {
                    self.current_graph.speakers.retain(|s| s.id != speaker.id);
                }
                EditorAction::ModifySpeaker(_idx, old_speaker) => {
                    if let Some(speaker) = self
                        .current_graph
                        .speakers
                        .iter_mut()
                        .find(|s| s.id == old_speaker.id)
                    {
                        *speaker = old_speaker.clone();
                    }
                }
            }
            self.redo_stack.push(action);
        }
    }

    fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            match &action {
                EditorAction::AddNode(node) => {
                    self.current_graph.nodes.push(node.clone());
                }
                EditorAction::DeleteNode(id) => {
                    self.current_graph.nodes.retain(|n| n.id != *id);
                }
                EditorAction::ModifyNode(_id, _old_node) => {
                    // Re-apply modification (would need current state)
                }
                EditorAction::AddSpeaker(speaker) => {
                    self.current_graph.speakers.push(speaker.clone());
                }
                EditorAction::ModifySpeaker(_idx, _old_speaker) => {
                    // Re-apply speaker modification
                }
            }
            self.undo_stack.push(action);
        }
    }

    // === Template Methods ===

    fn apply_template(&mut self, template_idx: usize) {
        if template_idx >= self.templates.len() {
            return;
        }

        let template = &self.templates[template_idx];

        // Clear current graph
        self.current_graph.nodes.clear();
        self.current_graph.name = format!("New {}", template.name);

        // Apply template-specific logic
        match template.name.as_str() {
            "Greeting" => self.apply_greeting_template(),
            "Branching Quest" => self.apply_quest_template(),
            "Shop Interaction" => self.apply_shop_template(),
            _ => {}
        }

        // Update statistics
        self.update_statistics();

        // Validate if auto-validate is enabled
        if self.auto_validate {
            self.validate_graph();
        }
    }

    fn apply_greeting_template(&mut self) {
        let start_id = self.next_id();
        self.current_graph.nodes.push(DialogueNode {
            id: start_id,
            node_type: DialogueNodeType::Speech,
            text: "Hello! How can I help you?".to_string(),
            choices: vec![
                DialogueChoice {
                    text: "Tell me more.".to_string(),
                    target_node_id: Some(start_id + 1),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "Goodbye.".to_string(),
                    target_node_id: None,
                    ..Default::default()
                },
            ],
            position: (100.0, 100.0),
            ..Default::default()
        });

        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 1,
            node_type: DialogueNodeType::Speech,
            text: "Sure, I'd be happy to explain!".to_string(),
            choices: vec![],
            position: (300.0, 100.0),
            ..Default::default()
        });

        self.current_graph.start_node_id = Some(start_id);
        self.next_id += 2;
    }

    fn apply_quest_template(&mut self) {
        let start_id = self.next_id();

        // Quest giver speech
        self.current_graph.nodes.push(DialogueNode {
            id: start_id,
            node_type: DialogueNodeType::Speech,
            text: "I have a task for you. Will you help?".to_string(),
            choices: vec![
                DialogueChoice {
                    text: "Yes, I'll help.".to_string(),
                    target_node_id: Some(start_id + 1),
                    condition: String::new(),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "Tell me more first.".to_string(),
                    target_node_id: Some(start_id + 2),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "No, not interested.".to_string(),
                    target_node_id: Some(start_id + 3),
                    ..Default::default()
                },
            ],
            position: (100.0, 100.0),
            ..Default::default()
        });

        // Accept branch
        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 1,
            node_type: DialogueNodeType::Speech,
            text: "Thank you! I knew I could count on you.".to_string(),
            choices: vec![],
            position: (300.0, 50.0),
            ..Default::default()
        });

        // More info branch
        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 2,
            node_type: DialogueNodeType::Speech,
            text: "The task involves... [details here]".to_string(),
            choices: vec![DialogueChoice {
                text: "Okay, I'll do it.".to_string(),
                target_node_id: Some(start_id + 1),
                ..Default::default()
            }],
            position: (300.0, 150.0),
            ..Default::default()
        });

        // Decline branch
        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 3,
            node_type: DialogueNodeType::Speech,
            text: "That's unfortunate. Come back if you change your mind.".to_string(),
            choices: vec![],
            position: (300.0, 250.0),
            ..Default::default()
        });

        self.current_graph.start_node_id = Some(start_id);
        self.next_id += 4;
    }

    fn apply_shop_template(&mut self) {
        let start_id = self.next_id();

        self.current_graph.nodes.push(DialogueNode {
            id: start_id,
            node_type: DialogueNodeType::Speech,
            text: "Welcome to my shop! What can I get you?".to_string(),
            choices: vec![
                DialogueChoice {
                    text: "Show me your goods.".to_string(),
                    target_node_id: Some(start_id + 1),
                    ..Default::default()
                },
                DialogueChoice {
                    text: "I'm just looking.".to_string(),
                    target_node_id: Some(start_id + 2),
                    ..Default::default()
                },
            ],
            position: (100.0, 100.0),
            ..Default::default()
        });

        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 1,
            node_type: DialogueNodeType::Action,
            text: "[Opens shop UI]".to_string(),
            choices: vec![],
            position: (300.0, 50.0),
            ..Default::default()
        });

        self.current_graph.nodes.push(DialogueNode {
            id: start_id + 2,
            node_type: DialogueNodeType::Speech,
            text: "Take your time!".to_string(),
            choices: vec![],
            position: (300.0, 150.0),
            ..Default::default()
        });

        self.current_graph.start_node_id = Some(start_id);
        self.next_id += 3;
    }

    // === Export/Import Methods ===

    fn export_dialogue(&mut self) {
        // Queue export action for external handling (file I/O should be done externally)
        self.queue_action(DialogueEditorAction::ExportDialogue {
            format: self.export_format,
            path: self.export_path.clone(),
        });
    }

    fn import_dialogue(&mut self) {
        // Queue import action for external handling (file I/O should be done externally)
        if !self.import_path.is_empty() {
            self.queue_action(DialogueEditorAction::ImportDialogue {
                path: self.import_path.clone(),
            });
        }
    }

    // === Statistics Methods ===

    fn update_statistics(&mut self) {
        // Calculate total word count
        self.total_word_count = self
            .current_graph
            .nodes
            .iter()
            .map(|n| n.text.split_whitespace().count())
            .sum();

        // Calculate average branch factor
        let branching_nodes: Vec<_> = self
            .current_graph
            .nodes
            .iter()
            .filter(|n| n.choices.len() > 1)
            .collect();

        if !branching_nodes.is_empty() {
            let total_branches: usize = branching_nodes.iter().map(|n| n.choices.len()).sum();
            self.avg_branch_factor = total_branches as f32 / branching_nodes.len() as f32;
        } else {
            self.avg_branch_factor = 0.0;
        }
    }

    // === Layout Methods ===

    fn auto_layout(&mut self) {
        match self.layout_algorithm {
            LayoutAlgorithm::Hierarchical => self.layout_hierarchical(),
            LayoutAlgorithm::Radial => self.layout_radial(),
            LayoutAlgorithm::ForceDirected => self.layout_force_directed(),
            LayoutAlgorithm::Tree => self.layout_tree(),
        }
    }

    fn layout_hierarchical(&mut self) {
        // Simple hierarchical layout
        let mut y = 100.0;
        for node in &mut self.current_graph.nodes {
            node.position = (100.0, y);
            y += 100.0;
        }
    }

    fn layout_radial(&mut self) {
        // Radial layout around center
        let center = (400.0, 300.0);
        let radius = 200.0;
        let count = self.current_graph.nodes.len();

        for (i, node) in self.current_graph.nodes.iter_mut().enumerate() {
            let angle = (i as f32 / count as f32) * 2.0 * std::f32::consts::PI;
            node.position = (
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            );
        }
    }

    fn layout_force_directed(&mut self) {
        // Force-directed layout using Fruchterman-Reingold algorithm
        // Parameters
        const ITERATIONS: usize = 50;
        const REPULSION_CONSTANT: f32 = 10000.0;
        const ATTRACTION_CONSTANT: f32 = 0.01;
        const DAMPING: f32 = 0.85;
        const MIN_DISTANCE: f32 = 1.0;

        let node_count = self.current_graph.nodes.len();
        if node_count == 0 {
            return;
        }

        // Initialize positions if needed (spread nodes in a grid initially)
        let sqrt_n = (node_count as f32).sqrt().ceil() as usize;
        for (i, node) in self.current_graph.nodes.iter_mut().enumerate() {
            let row = i / sqrt_n;
            let col = i % sqrt_n;
            node.position = (200.0 + col as f32 * 150.0, 100.0 + row as f32 * 150.0);
        }

        // Build edge list from choices
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (i, node) in self.current_graph.nodes.iter().enumerate() {
            for choice in &node.choices {
                if let Some(target_id) = choice.target_node_id {
                    if let Some(j) = self
                        .current_graph
                        .nodes
                        .iter()
                        .position(|n| n.id == target_id)
                    {
                        edges.push((i, j));
                    }
                }
            }
        }

        // Velocity storage
        let mut velocities: Vec<(f32, f32)> = vec![(0.0, 0.0); node_count];

        // Iterate force simulation
        for _ in 0..ITERATIONS {
            // Calculate repulsive forces between all node pairs
            let mut forces: Vec<(f32, f32)> = vec![(0.0, 0.0); node_count];

            for i in 0..node_count {
                for j in (i + 1)..node_count {
                    let pos_i = self.current_graph.nodes[i].position;
                    let pos_j = self.current_graph.nodes[j].position;

                    let dx = pos_i.0 - pos_j.0;
                    let dy = pos_i.1 - pos_j.1;
                    let distance = (dx * dx + dy * dy).sqrt().max(MIN_DISTANCE);

                    // Repulsive force (inverse square law)
                    let repulsion = REPULSION_CONSTANT / (distance * distance);
                    let fx = (dx / distance) * repulsion;
                    let fy = (dy / distance) * repulsion;

                    forces[i].0 += fx;
                    forces[i].1 += fy;
                    forces[j].0 -= fx;
                    forces[j].1 -= fy;
                }
            }

            // Calculate attractive forces along edges
            for (i, j) in &edges {
                let pos_i = self.current_graph.nodes[*i].position;
                let pos_j = self.current_graph.nodes[*j].position;

                let dx = pos_j.0 - pos_i.0;
                let dy = pos_j.1 - pos_i.1;
                let distance = (dx * dx + dy * dy).sqrt().max(MIN_DISTANCE);

                // Attractive force (spring-like)
                let attraction = distance * ATTRACTION_CONSTANT;
                let fx = (dx / distance) * attraction;
                let fy = (dy / distance) * attraction;

                forces[*i].0 += fx;
                forces[*i].1 += fy;
                forces[*j].0 -= fx;
                forces[*j].1 -= fy;
            }

            // Apply forces to velocities and positions
            for i in 0..node_count {
                velocities[i].0 = (velocities[i].0 + forces[i].0) * DAMPING;
                velocities[i].1 = (velocities[i].1 + forces[i].1) * DAMPING;

                // Limit velocity to prevent instability
                let speed =
                    (velocities[i].0 * velocities[i].0 + velocities[i].1 * velocities[i].1).sqrt();
                if speed > 50.0 {
                    velocities[i].0 = (velocities[i].0 / speed) * 50.0;
                    velocities[i].1 = (velocities[i].1 / speed) * 50.0;
                }

                self.current_graph.nodes[i].position.0 += velocities[i].0;
                self.current_graph.nodes[i].position.1 += velocities[i].1;

                // Keep nodes within bounds
                self.current_graph.nodes[i].position.0 =
                    self.current_graph.nodes[i].position.0.clamp(50.0, 800.0);
                self.current_graph.nodes[i].position.1 =
                    self.current_graph.nodes[i].position.1.clamp(50.0, 600.0);
            }
        }
    }

    fn layout_tree(&mut self) {
        // Simple tree layout starting from start node
        if let Some(start_id) = self.current_graph.start_node_id {
            self.layout_tree_recursive(start_id, 0, 0.0);
        }
    }

    fn layout_tree_recursive(&mut self, node_id: u32, depth: usize, sibling_offset: f32) {
        if let Some(node) = self
            .current_graph
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
        {
            node.position = (100.0 + depth as f32 * 200.0, 100.0 + sibling_offset * 150.0);
        }
    }
}

impl Panel for DialogueEditorPanel {
    fn name(&self) -> &'static str {
        "Dialogue Editor"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        // Auto-validate if enabled
        if self.auto_validate && matches!(self.active_tab, DialogueTab::Validation) {
            self.validate_graph();
        }

        match self.active_tab {
            DialogueTab::Graph => self.show_graph_tab(ui),
            DialogueTab::Nodes => self.show_nodes_tab(ui),
            DialogueTab::Speakers => self.show_speakers_tab(ui),
            DialogueTab::Variables => self.show_variables_tab(ui),
            DialogueTab::Localization => self.show_localization_tab(ui),
            DialogueTab::Preview => self.show_preview_tab(ui),
            DialogueTab::Validation => self.show_validation_tab(ui),
            DialogueTab::Export => self.show_export_tab(ui),
            DialogueTab::Templates => self.show_templates_tab(ui),
        }
    }

    fn update(&mut self) {
        // Update statistics periodically
        self.update_statistics();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_editor_panel_creation() {
        let panel = DialogueEditorPanel::new();
        assert!(!panel.graph_name().is_empty());
    }

    #[test]
    fn test_default_sample_dialogue() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        assert!(panel.node_count() >= 3);
        assert!(panel.speaker_count() >= 2);
    }

    #[test]
    fn test_add_node() {
        let mut panel = DialogueEditorPanel::new();
        let initial_count = panel.node_count();

        let id = panel.add_node(DialogueNodeType::Choice);
        assert!(id > 0);
        assert_eq!(panel.node_count(), initial_count + 1);
    }

    #[test]
    fn test_add_speaker() {
        let mut panel = DialogueEditorPanel::new();
        let initial_count = panel.speaker_count();

        panel.add_speaker("npc_guard", "Guard");
        assert_eq!(panel.speaker_count(), initial_count + 1);
    }

    #[test]
    fn test_select_node() {
        let mut panel = DialogueEditorPanel::new();
        assert!(panel.selected_node().is_none());

        let id = panel.add_node(DialogueNodeType::Speech);
        panel.select_node(id);
        assert_eq!(panel.selected_node(), Some(id));
    }

    #[test]
    fn test_set_start_node() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Speech);
        panel.set_start_node(id);
        assert_eq!(panel.current_graph.start_node_id, Some(id));
    }

    #[test]
    fn test_node_type_properties() {
        assert_eq!(DialogueNodeType::Speech.icon(), "[Chat]");
        assert_eq!(DialogueNodeType::Choice.icon(), "[Shuf]");
        assert_eq!(
            DialogueNodeType::End.color(),
            Color32::from_rgb(220, 20, 60)
        );
    }

    #[test]
    fn test_variable_count() {
        let panel = DialogueEditorPanel::new();
        assert_eq!(panel.variable_count(), 0);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = DialogueEditorPanel::new();
        assert_eq!(panel.name(), "Dialogue Editor");
    }

    // === Validation Tests ===

    #[test]
    fn test_validation_empty_graph() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();
        panel.current_graph.start_node_id = None;

        panel.validate_graph();

        assert!(!panel.validation_issues.is_empty());
        assert!(panel
            .validation_issues
            .iter()
            .any(|i| i.message.contains("No start node")));
    }

    #[test]
    fn test_validation_unreachable_nodes() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        let orphan_id = panel.add_node(DialogueNodeType::Speech);

        panel.validate_graph();

        assert!(panel
            .validation_issues
            .iter()
            .any(|i| i.message.contains("unreachable") && i.node_id == Some(orphan_id)));
    }

    #[test]
    fn test_validation_empty_speech_text() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Speech);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.text = String::new();
        }

        panel.validate_graph();

        assert!(panel.validation_issues.iter().any(
            |i| matches!(i.severity, IssueSeverity::Error) && i.message.contains("empty text")
        ));
    }

    #[test]
    fn test_validation_missing_speaker() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Speech);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.speaker_id = None;
        }

        panel.validate_graph();

        assert!(panel
            .validation_issues
            .iter()
            .any(|i| i.message.contains("no speaker")));
    }

    #[test]
    fn test_validation_invalid_speaker_reference() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Speech);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.speaker_id = Some("nonexistent_speaker".to_string());
        }

        panel.validate_graph();

        assert!(panel
            .validation_issues
            .iter()
            .any(|i| matches!(i.severity, IssueSeverity::Error)
                && i.message.contains("unknown speaker")));
    }

    #[test]
    fn test_validation_invalid_choice_target() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Choice);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.choices.push(DialogueChoice {
                text: "Invalid choice".to_string(),
                target_node_id: Some(99999),
                ..Default::default()
            });
        }

        panel.validate_graph();

        assert!(panel
            .validation_issues
            .iter()
            .any(|i| matches!(i.severity, IssueSeverity::Error)
                && i.message.contains("non-existent node")));
    }

    #[test]
    fn test_validation_duplicate_speaker_ids() {
        let mut panel = DialogueEditorPanel::new();
        panel.add_speaker("duplicate_id", "Speaker 1");
        panel.add_speaker("duplicate_id", "Speaker 2");

        panel.validate_graph();

        assert!(panel
            .validation_issues
            .iter()
            .any(|i| matches!(i.severity, IssueSeverity::Error)
                && i.message.contains("Duplicate speaker")));
    }

    #[test]
    fn test_validation_severity_levels() {
        assert_eq!(IssueSeverity::Error.icon(), "[x]");
        assert_eq!(IssueSeverity::Warning.icon(), "[!]");
        assert_eq!(IssueSeverity::Info.icon(), "ℹ️");

        assert_eq!(IssueSeverity::Error.color(), Color32::from_rgb(220, 60, 60));
        assert_eq!(
            IssueSeverity::Warning.color(),
            Color32::from_rgb(255, 180, 60)
        );
        assert_eq!(
            IssueSeverity::Info.color(),
            Color32::from_rgb(100, 150, 255)
        );
    }

    #[test]
    fn test_auto_validate_enabled_by_default() {
        let panel = DialogueEditorPanel::new();
        assert!(panel.auto_validate);
    }

    // === Template Tests ===

    #[test]
    fn test_templates_initialization() {
        let panel = DialogueEditorPanel::new();
        assert!(!panel.templates.is_empty());
        assert_eq!(panel.templates.len(), 3); // greeting, quest, shop
    }

    #[test]
    fn test_template_names() {
        let templates = DialogueTemplate::all_templates();
        assert_eq!(templates[0].name, "Greeting");
        assert_eq!(templates[1].name, "Branching Quest");
        assert_eq!(templates[2].name, "Shop Interaction");
    }

    #[test]
    fn test_apply_greeting_template() {
        let mut panel = DialogueEditorPanel::new();
        let _initial_nodes = panel.current_graph.nodes.len();
        panel.current_graph.nodes.clear();

        panel.apply_template(0); // greeting template

        // Template should add nodes
        assert!(!panel.current_graph.nodes.is_empty());
        assert!(panel.current_graph.start_node_id.is_some());
    }

    #[test]
    fn test_apply_quest_template() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();

        panel.apply_template(1); // quest template

        assert!(!panel.current_graph.nodes.is_empty());
        assert!(panel.current_graph.start_node_id.is_some());
    }

    #[test]
    fn test_apply_shop_template() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();

        panel.apply_template(2); // shop template

        assert!(!panel.current_graph.nodes.is_empty());
        assert!(panel.current_graph.start_node_id.is_some());
    }

    #[test]
    fn test_template_updates_statistics() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();
        panel.total_word_count = 0;

        panel.apply_template(0);

        assert!(panel.total_word_count > 0);
    }

    // === Search Tests ===

    #[test]
    fn test_search_filter_default() {
        let filter = SearchFilter::default();
        assert!(filter.query.is_empty());
        assert!(filter.search_text);
        assert!(filter.search_notes);
    }

    #[test]
    fn test_search_in_text() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        panel.search_filter.query = "welcome".to_string();

        panel.search_nodes();

        assert!(!panel.search_results.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let mut panel = DialogueEditorPanel::new();
        panel.search_filter.query = String::new();

        panel.search_nodes();

        assert!(panel.search_results.is_empty());
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        panel.search_filter.query = "WELCOME".to_string();

        panel.search_nodes();

        assert!(!panel.search_results.is_empty());
    }

    #[test]
    fn test_search_filter_by_type() {
        let mut panel = DialogueEditorPanel::new();
        panel.search_filter.query = "a".to_string();
        panel.search_filter.filter_type = Some(DialogueNodeType::Speech);

        panel.search_nodes();

        for node_id in &panel.search_results {
            let node = panel
                .current_graph
                .nodes
                .iter()
                .find(|n| n.id == *node_id)
                .unwrap();
            assert!(matches!(node.node_type, DialogueNodeType::Speech));
        }
    }

    // === Undo/Redo Tests ===

    #[test]
    fn test_undo_stack_initialization() {
        let panel = DialogueEditorPanel::new();
        assert!(panel.undo_stack.is_empty());
        assert!(panel.redo_stack.is_empty());
        assert_eq!(panel.max_undo_steps, 50);
    }

    #[test]
    fn test_push_undo_clears_redo() {
        let mut panel = DialogueEditorPanel::new();
        let dummy_node = DialogueNode::default();
        panel
            .redo_stack
            .push(EditorAction::AddNode(dummy_node.clone()));

        panel.push_undo(EditorAction::AddNode(dummy_node));

        assert!(panel.redo_stack.is_empty());
        assert_eq!(panel.undo_stack.len(), 1);
    }

    #[test]
    fn test_undo_add_node() {
        let mut panel = DialogueEditorPanel::new();
        let id = panel.add_node(DialogueNodeType::Speech);
        let node = panel
            .current_graph
            .nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap()
            .clone();
        panel.push_undo(EditorAction::AddNode(node));

        let count_before = panel.node_count();
        panel.undo();

        assert_eq!(panel.node_count(), count_before - 1);
    }

    #[test]
    fn test_undo_stack_max_size() {
        let mut panel = DialogueEditorPanel::new();
        panel.max_undo_steps = 3;

        let dummy_node = DialogueNode::default();
        for _ in 0..5 {
            panel.push_undo(EditorAction::AddNode(dummy_node.clone()));
        }

        assert_eq!(panel.undo_stack.len(), 3);
    }

    // === Export/Import Tests ===

    #[test]
    fn test_export_format_default() {
        let panel = DialogueEditorPanel::new();
        assert!(matches!(panel.export_format, ExportFormat::Json));
    }

    #[test]
    fn test_export_path_default() {
        let panel = DialogueEditorPanel::new();
        assert_eq!(panel.export_path, "dialogue_export.json");
    }

    #[test]
    fn test_export_format_values() {
        let formats = [
            ExportFormat::Json,
            ExportFormat::Yarn,
            ExportFormat::Ink,
            ExportFormat::Xml,
            ExportFormat::Csv,
        ];
        assert_eq!(formats.len(), 5);
    }

    // === Statistics Tests ===

    #[test]
    fn test_statistics_initialization() {
        let panel = DialogueEditorPanel::new();
        assert_eq!(panel.total_word_count, 0);
        assert_eq!(panel.avg_branch_factor, 0.0);
    }

    #[test]
    fn test_update_statistics_word_count() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();

        let id = panel.add_node(DialogueNodeType::Speech);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.text = "This is a test message with seven words.".to_string();
        }

        panel.update_statistics();

        assert_eq!(panel.total_word_count, 8);
    }

    #[test]
    fn test_update_statistics_branch_factor() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();

        let id = panel.add_node(DialogueNodeType::Choice);
        if let Some(node) = panel.current_graph.nodes.iter_mut().find(|n| n.id == id) {
            node.choices = vec![
                DialogueChoice::default(),
                DialogueChoice::default(),
                DialogueChoice::default(),
            ];
        }

        panel.update_statistics();

        assert_eq!(panel.avg_branch_factor, 3.0);
    }

    #[test]
    fn test_update_statistics_no_branches() {
        let mut panel = DialogueEditorPanel::new();
        panel.current_graph.nodes.clear();
        panel.add_node(DialogueNodeType::Speech);

        panel.update_statistics();

        assert_eq!(panel.avg_branch_factor, 0.0);
    }

    // === Layout Tests ===

    #[test]
    fn test_layout_algorithm_default() {
        let panel = DialogueEditorPanel::new();
        assert!(matches!(
            panel.layout_algorithm,
            LayoutAlgorithm::Hierarchical
        ));
    }

    #[test]
    fn test_auto_layout_on_add_default() {
        let panel = DialogueEditorPanel::new();
        assert!(!panel.auto_layout_on_add);
    }

    #[test]
    fn test_layout_hierarchical() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        panel.layout_algorithm = LayoutAlgorithm::Hierarchical;

        panel.auto_layout();

        // Should space nodes vertically
        assert!(!panel.current_graph.nodes.is_empty());
    }

    #[test]
    fn test_layout_radial() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        panel.layout_algorithm = LayoutAlgorithm::Radial;

        panel.auto_layout();

        // Nodes should be arranged in a circle
        assert!(!panel.current_graph.nodes.is_empty());
    }

    // === Collaboration Tests ===

    #[test]
    fn test_collaboration_default() {
        let panel = DialogueEditorPanel::new();
        assert!(!panel.collaboration.enabled);
        assert_eq!(panel.collaboration.current_user, "Editor");
        assert!(panel.collaboration.active_users.is_empty());
        assert!(panel.collaboration.locked_nodes.is_empty());
    }

    #[test]
    fn test_dialogue_tab_variants() {
        let tabs = [
            DialogueTab::Graph,
            DialogueTab::Nodes,
            DialogueTab::Speakers,
            DialogueTab::Variables,
            DialogueTab::Localization,
            DialogueTab::Preview,
            DialogueTab::Validation,
            DialogueTab::Export,
            DialogueTab::Templates,
        ];
        assert_eq!(tabs.len(), 9);
    }

    #[test]
    fn test_active_tab_default() {
        let panel = DialogueEditorPanel::new();
        assert!(matches!(panel.active_tab, DialogueTab::Graph));
    }

    // === DialogueNodeType Display and Hash Tests ===

    #[test]
    fn test_dialogue_node_type_display() {
        for node_type in DialogueNodeType::all() {
            let display = format!("{}", node_type);
            assert!(display.contains(node_type.name()));
        }
    }

    #[test]
    fn test_dialogue_node_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for node_type in DialogueNodeType::all() {
            set.insert(*node_type);
        }
        assert_eq!(set.len(), DialogueNodeType::all().len());
    }

    #[test]
    fn test_dialogue_node_type_all() {
        let all = DialogueNodeType::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&DialogueNodeType::Speech));
        assert!(all.contains(&DialogueNodeType::Choice));
        assert!(all.contains(&DialogueNodeType::Condition));
        assert!(all.contains(&DialogueNodeType::Action));
        assert!(all.contains(&DialogueNodeType::RandomBranch));
        assert!(all.contains(&DialogueNodeType::Jump));
        assert!(all.contains(&DialogueNodeType::End));
    }

    #[test]
    fn test_dialogue_node_type_is_branching() {
        assert!(!DialogueNodeType::Speech.is_branching());
        assert!(DialogueNodeType::Choice.is_branching());
        assert!(DialogueNodeType::Condition.is_branching());
        assert!(DialogueNodeType::RandomBranch.is_branching());
        assert!(!DialogueNodeType::Action.is_branching());
        assert!(!DialogueNodeType::Jump.is_branching());
        assert!(!DialogueNodeType::End.is_branching());
    }

    #[test]
    fn test_dialogue_node_type_is_terminal() {
        assert!(!DialogueNodeType::Speech.is_terminal());
        assert!(DialogueNodeType::End.is_terminal());
    }

    // === VariableType Display and Hash Tests ===

    #[test]
    fn test_variable_type_display() {
        for var_type in VariableType::all() {
            let display = format!("{}", var_type);
            assert!(display.contains(var_type.name()));
        }
    }

    #[test]
    fn test_variable_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for var_type in VariableType::all() {
            set.insert(*var_type);
        }
        assert_eq!(set.len(), VariableType::all().len());
    }

    #[test]
    fn test_variable_type_all() {
        let all = VariableType::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&VariableType::Boolean));
        assert!(all.contains(&VariableType::Integer));
        assert!(all.contains(&VariableType::Float));
        assert!(all.contains(&VariableType::String));
    }

    #[test]
    fn test_variable_type_is_numeric() {
        assert!(!VariableType::Boolean.is_numeric());
        assert!(VariableType::Integer.is_numeric());
        assert!(VariableType::Float.is_numeric());
        assert!(!VariableType::String.is_numeric());
    }

    // === DialogueTab Display and Hash Tests ===

    #[test]
    fn test_dialogue_tab_display() {
        for tab in DialogueTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_dialogue_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in DialogueTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), DialogueTab::all().len());
    }

    #[test]
    fn test_dialogue_tab_all() {
        let all = DialogueTab::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&DialogueTab::Graph));
        assert!(all.contains(&DialogueTab::Nodes));
        assert!(all.contains(&DialogueTab::Speakers));
        assert!(all.contains(&DialogueTab::Variables));
        assert!(all.contains(&DialogueTab::Localization));
        assert!(all.contains(&DialogueTab::Preview));
        assert!(all.contains(&DialogueTab::Validation));
        assert!(all.contains(&DialogueTab::Export));
        assert!(all.contains(&DialogueTab::Templates));
    }

    // === LayoutAlgorithm Display and Hash Tests ===

    #[test]
    fn test_layout_algorithm_display() {
        for alg in LayoutAlgorithm::all() {
            let display = format!("{}", alg);
            assert!(display.contains(alg.name()));
        }
    }

    #[test]
    fn test_layout_algorithm_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for alg in LayoutAlgorithm::all() {
            set.insert(*alg);
        }
        assert_eq!(set.len(), LayoutAlgorithm::all().len());
    }

    #[test]
    fn test_layout_algorithm_all() {
        let all = LayoutAlgorithm::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&LayoutAlgorithm::Hierarchical));
        assert!(all.contains(&LayoutAlgorithm::Radial));
        assert!(all.contains(&LayoutAlgorithm::ForceDirected));
        assert!(all.contains(&LayoutAlgorithm::Tree));
    }

    #[test]
    fn test_layout_algorithm_default_trait() {
        let default = LayoutAlgorithm::default();
        assert_eq!(default, LayoutAlgorithm::Hierarchical);
    }

    #[test]
    fn test_layout_algorithm_description() {
        for alg in LayoutAlgorithm::all() {
            let desc = alg.description();
            assert!(!desc.is_empty());
        }
    }

    // === ExportFormat Display and Hash Tests ===

    #[test]
    fn test_export_format_display() {
        for format in ExportFormat::all() {
            let display = format!("{}", format);
            assert!(display.contains(format.name()));
        }
    }

    #[test]
    fn test_export_format_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for format in ExportFormat::all() {
            set.insert(*format);
        }
        assert_eq!(set.len(), ExportFormat::all().len());
    }

    #[test]
    fn test_export_format_all() {
        let all = ExportFormat::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&ExportFormat::Json));
        assert!(all.contains(&ExportFormat::Yarn));
        assert!(all.contains(&ExportFormat::Ink));
        assert!(all.contains(&ExportFormat::Xml));
        assert!(all.contains(&ExportFormat::Csv));
    }

    #[test]
    fn test_export_format_default_trait() {
        let default = ExportFormat::default();
        assert_eq!(default, ExportFormat::Json);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Yarn.extension(), "yarn");
        assert_eq!(ExportFormat::Ink.extension(), "ink");
        assert_eq!(ExportFormat::Xml.extension(), "xml");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
    }

    #[test]
    fn test_export_format_is_text_format() {
        for format in ExportFormat::all() {
            assert!(format.is_text_format());
        }
    }

    // === IssueSeverity Display and Hash Tests ===

    #[test]
    fn test_issue_severity_display() {
        for severity in IssueSeverity::all() {
            let display = format!("{}", severity);
            assert!(display.contains(severity.name()));
        }
    }

    #[test]
    fn test_issue_severity_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for severity in IssueSeverity::all() {
            set.insert(*severity);
        }
        assert_eq!(set.len(), IssueSeverity::all().len());
    }

    #[test]
    fn test_issue_severity_all() {
        let all = IssueSeverity::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&IssueSeverity::Info));
        assert!(all.contains(&IssueSeverity::Warning));
        assert!(all.contains(&IssueSeverity::Error));
    }

    #[test]
    fn test_issue_severity_default() {
        let default = IssueSeverity::default();
        assert_eq!(default, IssueSeverity::Info);
    }

    #[test]
    fn test_issue_severity_is_blocking() {
        assert!(!IssueSeverity::Info.is_blocking());
        assert!(!IssueSeverity::Warning.is_blocking());
        assert!(IssueSeverity::Error.is_blocking());
    }

    // === EditorAction Display and Helper Tests ===

    #[test]
    fn test_editor_action_display() {
        let action = EditorAction::AddNode(DialogueNode::default());
        let display = format!("{}", action);
        assert!(display.contains(action.name()));
    }

    #[test]
    fn test_editor_action_all_variants() {
        let all = EditorAction::all_variants();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&"AddNode"));
        assert!(all.contains(&"DeleteNode"));
        assert!(all.contains(&"ModifyNode"));
        assert!(all.contains(&"AddSpeaker"));
        assert!(all.contains(&"ModifySpeaker"));
    }

    #[test]
    fn test_editor_action_names() {
        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert_eq!(add_node.name(), "Add Node");

        let delete_node = EditorAction::DeleteNode(1);
        assert_eq!(delete_node.name(), "Delete Node");

        let modify_node = EditorAction::ModifyNode(1, DialogueNode::default());
        assert_eq!(modify_node.name(), "Modify Node");

        let add_speaker = EditorAction::AddSpeaker(DialogueSpeaker::default());
        assert_eq!(add_speaker.name(), "Add Speaker");

        let modify_speaker = EditorAction::ModifySpeaker(0, DialogueSpeaker::default());
        assert_eq!(modify_speaker.name(), "Modify Speaker");
    }

    #[test]
    fn test_editor_action_icons() {
        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert_eq!(add_node.icon(), "+");

        let delete_node = EditorAction::DeleteNode(1);
        assert_eq!(delete_node.icon(), "[Del]");

        let add_speaker = EditorAction::AddSpeaker(DialogueSpeaker::default());
        assert_eq!(add_speaker.icon(), "👤");
    }

    #[test]
    fn test_editor_action_is_node_action() {
        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert!(add_node.is_node_action());

        let delete_node = EditorAction::DeleteNode(1);
        assert!(delete_node.is_node_action());

        let modify_node = EditorAction::ModifyNode(1, DialogueNode::default());
        assert!(modify_node.is_node_action());

        let add_speaker = EditorAction::AddSpeaker(DialogueSpeaker::default());
        assert!(!add_speaker.is_node_action());
    }

    #[test]
    fn test_editor_action_is_speaker_action() {
        let add_speaker = EditorAction::AddSpeaker(DialogueSpeaker::default());
        assert!(add_speaker.is_speaker_action());

        let modify_speaker = EditorAction::ModifySpeaker(0, DialogueSpeaker::default());
        assert!(modify_speaker.is_speaker_action());

        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert!(!add_node.is_speaker_action());
    }

    #[test]
    fn test_editor_action_is_add() {
        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert!(add_node.is_add());

        let add_speaker = EditorAction::AddSpeaker(DialogueSpeaker::default());
        assert!(add_speaker.is_add());

        let delete_node = EditorAction::DeleteNode(1);
        assert!(!delete_node.is_add());
    }

    #[test]
    fn test_editor_action_is_delete() {
        let delete_node = EditorAction::DeleteNode(1);
        assert!(delete_node.is_delete());

        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert!(!add_node.is_delete());
    }

    #[test]
    fn test_editor_action_is_modify() {
        let modify_node = EditorAction::ModifyNode(1, DialogueNode::default());
        assert!(modify_node.is_modify());

        let modify_speaker = EditorAction::ModifySpeaker(0, DialogueSpeaker::default());
        assert!(modify_speaker.is_modify());

        let add_node = EditorAction::AddNode(DialogueNode::default());
        assert!(!add_node.is_modify());
    }

    #[test]
    fn test_editor_action_partial_eq() {
        let a1 = EditorAction::DeleteNode(5);
        let a2 = EditorAction::DeleteNode(5);
        let a3 = EditorAction::DeleteNode(10);
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    // === DialogueEditorAction Tests ===

    #[test]
    fn test_action_system_initial_state() {
        let panel = DialogueEditorPanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_system_take_actions_empty() {
        let mut panel = DialogueEditorPanel::new();
        let actions = panel.take_actions();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_action_export_dialogue() {
        let mut panel = DialogueEditorPanel::new();
        panel.export_dialogue();
        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            DialogueEditorAction::ExportDialogue { format: ExportFormat::Json, path } if path == "dialogue_export.json"
        ));
    }

    #[test]
    fn test_action_import_dialogue() {
        let mut panel = DialogueEditorPanel::new();
        panel.import_path = "test_dialogue.json".to_string();
        panel.import_dialogue();
        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            DialogueEditorAction::ImportDialogue { path } if path == "test_dialogue.json"
        ));
    }

    #[test]
    fn test_action_import_dialogue_empty_path() {
        let mut panel = DialogueEditorPanel::new();
        panel.import_path = String::new();
        panel.import_dialogue();
        // Should not queue action with empty path
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_take_clears_queue() {
        let mut panel = DialogueEditorPanel::new();
        panel.export_dialogue();
        assert!(panel.has_pending_actions());

        let _ = panel.take_actions();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_multiple_actions() {
        let mut panel = DialogueEditorPanel::new();
        panel.export_dialogue();
        panel.import_path = "import.json".to_string();
        panel.import_dialogue();

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_dialogue_editor_action_name() {
        let export = DialogueEditorAction::ExportDialogue {
            format: ExportFormat::Json,
            path: "test.json".to_string(),
        };
        assert_eq!(export.name(), "Export Dialogue");

        let import = DialogueEditorAction::ImportDialogue {
            path: "test.json".to_string(),
        };
        assert_eq!(import.name(), "Import Dialogue");

        let layout = DialogueEditorAction::ApplyLayout {
            algorithm: LayoutAlgorithm::ForceDirected,
        };
        assert_eq!(layout.name(), "Apply Layout");
    }

    #[test]
    fn test_dialogue_editor_action_is_io_action() {
        let export = DialogueEditorAction::ExportDialogue {
            format: ExportFormat::Json,
            path: "test.json".to_string(),
        };
        assert!(export.is_io_action());

        let import = DialogueEditorAction::ImportDialogue {
            path: "test.json".to_string(),
        };
        assert!(import.is_io_action());

        let layout = DialogueEditorAction::ApplyLayout {
            algorithm: LayoutAlgorithm::ForceDirected,
        };
        assert!(!layout.is_io_action());
    }

    #[test]
    fn test_dialogue_editor_action_is_graph_action() {
        let save = DialogueEditorAction::SaveGraph {
            graph_id: 1,
            name: "Test".to_string(),
        };
        assert!(save.is_graph_action());

        let load = DialogueEditorAction::LoadGraph { graph_id: 1 };
        assert!(load.is_graph_action());

        let create = DialogueEditorAction::CreateGraph {
            name: "New".to_string(),
        };
        assert!(create.is_graph_action());

        let delete = DialogueEditorAction::DeleteGraph { graph_id: 1 };
        assert!(delete.is_graph_action());

        let export = DialogueEditorAction::ExportDialogue {
            format: ExportFormat::Json,
            path: "test.json".to_string(),
        };
        assert!(!export.is_graph_action());
    }

    #[test]
    fn test_dialogue_editor_action_is_layout_action() {
        let layout = DialogueEditorAction::ApplyLayout {
            algorithm: LayoutAlgorithm::ForceDirected,
        };
        assert!(layout.is_layout_action());

        let export = DialogueEditorAction::ExportDialogue {
            format: ExportFormat::Json,
            path: "test.json".to_string(),
        };
        assert!(!export.is_layout_action());
    }

    #[test]
    fn test_dialogue_editor_action_display() {
        let export = DialogueEditorAction::ExportDialogue {
            format: ExportFormat::Json,
            path: "test.json".to_string(),
        };
        assert_eq!(format!("{}", export), "Export Dialogue");
    }

    #[test]
    fn test_dialogue_editor_action_partial_eq() {
        let a1 = DialogueEditorAction::RunValidation;
        let a2 = DialogueEditorAction::RunValidation;
        assert_eq!(a1, a2);

        let b1 = DialogueEditorAction::SetLanguage {
            language: "en".to_string(),
        };
        let b2 = DialogueEditorAction::SetLanguage {
            language: "en".to_string(),
        };
        let b3 = DialogueEditorAction::SetLanguage {
            language: "fr".to_string(),
        };
        assert_eq!(b1, b2);
        assert_ne!(b1, b3);
    }

    #[test]
    fn test_force_directed_layout() {
        let mut panel = DialogueEditorPanel::new();
        panel.create_sample_dialogue();
        // Should have sample nodes
        assert!(panel.node_count() >= 3);

        // Store initial positions
        let initial_positions: Vec<_> = panel
            .current_graph
            .nodes
            .iter()
            .map(|n| n.position)
            .collect();

        // Apply force-directed layout
        panel.layout_algorithm = LayoutAlgorithm::ForceDirected;
        panel.auto_layout();

        // Positions should be within bounds
        for node in &panel.current_graph.nodes {
            assert!(node.position.0 >= 50.0 && node.position.0 <= 800.0);
            assert!(node.position.1 >= 50.0 && node.position.1 <= 600.0);
        }

        // Positions should have changed from initial (layout was applied)
        let final_positions: Vec<_> = panel
            .current_graph
            .nodes
            .iter()
            .map(|n| n.position)
            .collect();

        // At least some positions should differ
        let changed = initial_positions
            .iter()
            .zip(&final_positions)
            .any(|(a, b)| (a.0 - b.0).abs() > 0.01 || (a.1 - b.1).abs() > 0.01);
        assert!(changed);
    }

    #[test]
    fn test_helper_methods() {
        let panel = DialogueEditorPanel::new();
        assert_eq!(panel.export_format(), ExportFormat::Json);
        assert_eq!(panel.export_path(), "dialogue_export.json");
        assert_eq!(panel.import_path(), "");
        assert_eq!(panel.layout_algorithm(), LayoutAlgorithm::Hierarchical);
        assert_eq!(panel.current_language(), "en");
        assert!(!panel.collaboration_enabled());
    }
}

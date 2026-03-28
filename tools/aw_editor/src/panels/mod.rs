// tools/aw_editor/src/panels/mod.rs - Panel trait and module exports

use egui::Ui;

/// Trait for reusable editor panels
///
/// Panels are self-contained UI components that can be shown in
/// side panels, windows, or central areas.
pub trait Panel {
    /// Panel display name (shown in tabs/titles)
    fn name(&self) -> &str;

    /// Render the panel's UI
    fn show(&mut self, ui: &mut Ui);

    /// Optional: Called each frame for updates (e.g., polling data)
    fn update(&mut self) {}
}

// Panel implementations
pub mod advanced_widgets_panel;
pub mod animation;
pub mod animation_panel;
pub mod asset_browser;
pub mod audio_panel;
pub mod blend_import_panel;
pub mod blueprint_panel;
pub mod build_manager;
pub mod charts_panel;
pub mod cinematics_panel;
pub mod console_panel;
pub mod dialogue_editor_panel;
pub mod distribution_panel;
pub mod entity_panel;
pub mod foliage_panel;
pub mod frame_debugger;
pub mod graph_panel;
pub mod hierarchy_panel;
pub mod input_bindings_panel;
pub mod lighting_panel;
pub mod localization_panel;
pub mod lod_config_panel;
pub mod material_editor_panel;
pub mod navigation_panel;
pub mod networking_panel;
pub mod particle_system_panel;
pub mod pcg_panel;
pub mod performance_panel;
pub mod physics_panel;
pub mod polish_panel;
pub mod post_process_panel;
pub mod profiler_panel;
pub mod project_settings_panel;
pub mod scene_stats_panel;
pub mod spline_editor_panel;
pub mod terrain_panel;
pub mod theme_manager;
pub mod transform_panel;
pub mod ui_editor_panel;
pub mod world_panel;

// Entity catalog (thumbnail-grid character/enemy/boss selector)
pub mod entity_catalog;

// New procedural and asset management panels
pub mod environment_preset_panel;
pub mod import_doctor_panel;
pub mod procedural_filler_panel;
pub mod ready_asset_store_panel;

// World creation wizard
pub mod world_wizard;

// Gameplay genre presets
pub mod gameplay_presets;

// Re-exports
pub use advanced_widgets_panel::AdvancedWidgetsPanel;
pub use animation::AnimationPanel;
#[allow(unused_imports)]
pub use animation_panel::{
    AnimationAction, AnimationPanel as AnimationEditorPanel, AnimationTab, LoopMode,
};
pub use asset_browser::{AssetAction, AssetBrowser, TextureType};
pub use build_manager::BuildManagerPanel;
#[allow(unused_imports)]
pub use charts_panel::{ChartType, ChartsAction, ChartsPanel, DataSource};
pub use console_panel::ConsolePanel;
pub use entity_panel::EntityPanel;
#[allow(unused_imports)]
pub use entity_panel::{
    EntityAction, EntityArchetype, EntityFilter, PrefabAction, ValidationIssue, ValidationSeverity,
};
pub use graph_panel::GraphPanel;
pub use hierarchy_panel::HierarchyPanel;
pub use performance_panel::PerformancePanel;
#[allow(unused_imports)]
pub use performance_panel::{
    AlertSeverity, GpuStats, MemoryStats, MetricUnit, PerfAlert, PerfCategory, PerfMetric,
    PerformanceAction, SubsystemTiming,
};
pub use profiler_panel::ProfilerPanel;
pub use scene_stats_panel::{SceneStats, SceneStatsPanel};
#[allow(unused_imports)]
pub use terrain_panel::{BrushMode, ErosionPresetType, TerrainAction, TerrainPanel};
pub use theme_manager::ThemeManagerPanel;
pub use transform_panel::TransformPanel;
pub use world_panel::WorldPanel;

// New panel re-exports (Phase 8 additions)
#[allow(unused_imports)]
pub use audio_panel::{
    AudioAction, AudioPanel, AudioTab, DistanceModel, MusicMood, ReverbEnvironment, SpatialPreset,
};
#[allow(unused_imports)]
pub use cinematics_panel::{
    CameraInterpolation, CinematicsAction, CinematicsPanel, CinematicsTab, PlaybackSpeed, TrackType,
};
#[allow(unused_imports)]
pub use dialogue_editor_panel::{
    DialogueEditorAction, DialogueEditorPanel, DialogueNodeType, ExportFormat, LayoutAlgorithm,
};
#[allow(unused_imports)]
pub use distribution_panel::{
    AssetOptions, AudioFormat, BuildHistoryEntry, BuildOptions, BuildProfile, BuildProgress,
    BuildStep, DistributionAction, DistributionPanel, TargetPlatform as DistTargetPlatform,
    TextureFormat, ValidationError, ValidationResult, ValidationWarning,
};
pub use foliage_panel::FoliagePanel;
#[allow(unused_imports)]
pub use foliage_panel::{
    BrushSettings, BrushTool, DistributionType, FoliageAction, FoliageCategory, FoliageLayer,
    FoliageTab, FoliageType, ProceduralRule,
};
pub use frame_debugger::FrameDebuggerPanel;
#[allow(unused_imports)]
pub use input_bindings_panel::{
    ActionCategory, BindingPreset, GamepadButton, InputBindingAction, InputBindingsPanel,
    InputDevice, InputTarget, KeyboardKey, MouseButton,
};
pub use lighting_panel::LightingPanel;
#[allow(unused_imports)]
pub use lighting_panel::{
    GiMode, Light, LightProbe, LightType, LightUnit, LightingAction, LightingTab, ReflectionProbe,
    ShadowQuality, ShadowType,
};
pub use localization_panel::LocalizationPanel;
#[allow(unused_imports)]
pub use localization_panel::{
    ExportFormat as LocalizationExportFormat, Language, LocalizationAction, LocalizationTab,
    LocalizedString, PluralForms, StringCategory,
};
pub use lod_config_panel::LodConfigPanel;
#[allow(unused_imports)]
pub use lod_config_panel::{
    FadeMode, GlobalLodSettings, LodBiasMode, LodConfigAction, LodGenerationSettings, LodGroup,
    LodLevel, LodTab, ReductionMethod,
};
pub use material_editor_panel::MaterialEditorPanel;
#[allow(unused_imports)]
pub use material_editor_panel::{
    BlendMode, Material, MaterialEditorAction, MaterialPreset, MaterialTab, MaterialType,
    PreviewLighting, TextureChannel, TextureSlot,
};
pub use navigation_panel::NavigationPanel;
#[allow(unused_imports)]
pub use navigation_panel::{
    NavAgentConfig, NavAreaType, NavDebugOptions, NavLink, NavLinkType, NavMeshBakeSettings,
    NavObstacle, NavRegion, NavigationAction, NavigationTab, PathTestResult,
};
#[allow(unused_imports)]
pub use networking_panel::{
    CompressionLevel, ConnectionState, InterestPolicy, NetworkAction, NetworkRole, NetworkTab,
    NetworkingPanel,
};
#[allow(unused_imports)]
pub use particle_system_panel::{
    EmitterShape, ParticleAction, ParticleBlendMode, ParticleRenderMode, ParticleSystemPanel,
    ParticleTab, SimulationSpace,
};
pub use pcg_panel::PcgPanel;
#[allow(unused_imports)]
pub use physics_panel::{
    PhysicsAction, PhysicsPanel, PhysicsTab, PhysicsVisualization, SimulationMode,
};
#[allow(unused_imports)]
pub use polish_panel::PolishPanel;
pub use post_process_panel::PostProcessPanel;
pub use project_settings_panel::ProjectSettingsPanel;
#[allow(unused_imports)]
pub use project_settings_panel::{
    AntialiasingMode, AoMode, AudioBackend, GiMode as RenderGiMode, InputAction, Layer,
    PhysicsSettings, ProjectSettingsAction, QualityLevel, ReflectionMode, RendererBackend,
    RenderingSettings, SettingsTab, ShadowMode, Tag, TargetPlatform, TextureQuality,
    TonemappingMode,
};
pub use spline_editor_panel::SplineEditorPanel;
#[allow(unused_imports)]
pub use spline_editor_panel::{
    CustomProfile, MeshProfile, ProfilePoint, Spline, SplineEditorAction, SplinePoint,
    SplinePreset, SplineTab, SplineTool, SplineType, TangentMode, UvMode,
};
pub use ui_editor_panel::UiEditorPanel;
#[allow(unused_imports)]
pub use ui_editor_panel::{
    AnchorPreset, EasingType, ScaleMode, UiAnimation, UiCanvas, UiEditorAction, UiEditorTab,
    UiPreset, UiStyle, UiWidget, WidgetType,
};

// Procedural and asset management panel re-exports (including action types)
// These are re-exports for the public API - they may be unused internally
#[allow(unused_imports)]
pub use environment_preset_panel::{
    EnvironmentAction, EnvironmentPresetPanel, EnvironmentSettings, FogType, MoodPreset, SkyType,
    TimeOfDay, Tonemapper, WeatherCondition,
};
#[allow(unused_imports)]
pub use gameplay_presets::{ComponentSchema, GameplayPreset, StarterEntity};
#[allow(unused_imports)]
pub use import_doctor_panel::{
    ImportAction, ImportDoctorPanel, ImportIssue, ImportSettings, IssueSeverity, IssueType,
    QuickFix, SourceEngine, TexturePackingFormat, UpAxis,
};
#[allow(unused_imports)]
pub use procedural_filler_panel::{
    BiomePreset, EnvironmentPreset, FillerAction, FillerMode, ProceduralFillerPanel, RoadPreset,
    ScatterCategory, ScatterSettings,
};
#[allow(unused_imports)]
pub use ready_asset_store_panel::{
    AssetChecklist, AssetStoreAction, AssetStoreCategory, ChecklistItem, ReadinessLevel,
    ReadyAsset, ReadyAssetStorePanel, SortBy,
};
#[allow(unused_imports)]
pub use world_wizard::{WizardStep, WorldTemplate, WorldWizard, WorldWizardAction};

// Blend import panel re-exports
pub use blend_import_panel::BlendImportPanel;
#[allow(unused_imports)]
pub use blend_import_panel::{
    BiomeTypeSelection, BlendImportAction, DecomposedAssetEntry, ImportPhase,
    ScatterSettings as BlendScatterSettings, TextureSettings as BlendTextureSettings,
};

// Blueprint zone editor re-exports
pub use blueprint_panel::BlueprintPanel;
#[allow(unused_imports)]
pub use blueprint_panel::{BlueprintAction, BlueprintTool, ZoneSourceState, ZoneState};

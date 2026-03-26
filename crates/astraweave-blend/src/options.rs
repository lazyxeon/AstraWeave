//! Conversion options for Blender to glTF export.
//!
//! This module defines all configurable options for the Blender-to-glTF
//! conversion process, with sensible defaults for game engine usage.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Complete configuration for a Blender conversion job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversionOptions {
    /// Output format selection.
    pub format: OutputFormat,
    /// glTF export settings.
    pub gltf: GltfExportOptions,
    /// Texture handling options.
    pub textures: TextureOptions,
    /// Animation export options.
    pub animation: AnimationOptions,
    /// Mesh processing options.
    pub mesh: MeshOptions,
    /// Material export options.
    pub materials: MaterialOptions,
    /// Linked library handling.
    pub linked_libraries: LinkedLibraryOptions,
    /// Process control options.
    pub process: ProcessOptions,
    /// Cache behavior options.
    pub cache: CacheOptions,
    /// Scene decomposition options (for splitting a .blend into individual assets).
    pub decomposition: SceneDecompositionOptions,
}

// ============================================================================
// Scene Decomposition Options
// ============================================================================

/// How to group objects when decomposing a .blend scene into individual assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum DecompositionGrouping {
    /// Export each mesh object as a separate asset.
    #[default]
    ByObject,
    /// Group objects by their Blender collection.
    ByCollection,
    /// Group objects that share the same material.
    ByMaterial,
}

/// What category an extracted object belongs to, used for downstream biome/scatter integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum AssetCategory {
    /// Category determined automatically from Blender object name heuristics.
    #[default]
    Auto,
    /// Vegetation: trees, bushes, flowers, grass.
    Vegetation,
    /// Rock/geological: boulders, cliffs, stones.
    Rock,
    /// Terrain: ground planes, terrain meshes.
    Terrain,
    /// Prop: man-made objects, structures.
    Prop,
    /// Billboard: flat card geometry (e.g. distant foliage).
    Billboard,
}

/// How to compute per-asset bounding volumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum BoundingBoxMode {
    /// Compute AABB from mesh vertices.
    #[default]
    Aabb,
    /// Use Blender's object bounding_box property.
    BlenderBounds,
    /// Do not compute bounding volumes.
    None,
}

/// Configuration for decomposing a .blend scene into individual asset files.
///
/// When enabled, the importer splits a multi-object .blend file into
/// one glTF/GLB per logical group, generates a manifest describing all
/// extracted assets, and optionally extracts textures and HDRIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDecompositionOptions {
    /// Enable scene decomposition (default: false — normal single-file export).
    pub enabled: bool,
    /// How to group objects during decomposition.
    pub group_by: DecompositionGrouping,
    /// Output directory for decomposed assets (None = `{output_dir}/{blend_stem}/`).
    pub output_dir: Option<PathBuf>,
    /// Generate a `manifest.json` listing all extracted assets with metadata.
    pub generate_manifest: bool,
    /// Extract textures as separate files alongside the GLBs.
    pub extract_textures: bool,
    /// Extract HDRI/environment maps found in the scene's World nodes.
    pub extract_hdris: bool,
    /// Include Blender empties (e.g. placement markers) in the manifest.
    pub include_empties: bool,
    /// How to compute bounding volumes for each asset.
    pub bounding_box_mode: BoundingBoxMode,
    /// Minimum vertex count to include an object (filters degenerate geometry).
    pub min_vertex_count: u32,
    /// Asset category assignment mode.
    pub asset_category: AssetCategory,
    /// Object name patterns to exclude (matched case-insensitively against Blender object names).
    pub exclude_patterns: Vec<String>,
}

impl Default for SceneDecompositionOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            group_by: DecompositionGrouping::ByObject,
            output_dir: None,
            generate_manifest: true,
            extract_textures: true,
            extract_hdris: true,
            include_empties: false,
            bounding_box_mode: BoundingBoxMode::Aabb,
            min_vertex_count: 3,
            asset_category: AssetCategory::Auto,
            exclude_patterns: Vec::new(),
        }
    }
}

impl SceneDecompositionOptions {
    /// Preset for extracting a nature/environment pack (vegetation + rocks + terrain).
    pub fn nature_pack() -> Self {
        Self {
            enabled: true,
            group_by: DecompositionGrouping::ByObject,
            generate_manifest: true,
            extract_textures: true,
            extract_hdris: true,
            include_empties: true,
            bounding_box_mode: BoundingBoxMode::Aabb,
            min_vertex_count: 3,
            asset_category: AssetCategory::Auto,
            exclude_patterns: vec!["Camera".to_string(), "Light".to_string()],
            ..Default::default()
        }
    }
}

impl ConversionOptions {
    /// Creates options optimized for runtime game usage.
    pub fn game_runtime() -> Self {
        Self {
            format: OutputFormat::GlbBinary,
            gltf: GltfExportOptions {
                draco_compression: true,
                ..Default::default()
            },
            textures: TextureOptions {
                format: TextureFormat::Png,
                max_resolution: Some(2048),
                ..Default::default()
            },
            mesh: MeshOptions {
                apply_modifiers: true,
                triangulate: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Creates options for editor preview (fast, lower quality).
    pub fn editor_preview() -> Self {
        Self {
            format: OutputFormat::GlbBinary,
            gltf: GltfExportOptions {
                draco_compression: false, // Faster
                ..Default::default()
            },
            textures: TextureOptions {
                format: TextureFormat::Jpeg,
                max_resolution: Some(512),
                jpeg_quality: 70,
                ..Default::default()
            },
            mesh: MeshOptions {
                apply_modifiers: true,
                triangulate: true,
                ..Default::default()
            },
            process: ProcessOptions {
                timeout: Duration::from_secs(30), // Shorter timeout
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Creates options for maximum quality archival.
    pub fn archival_quality() -> Self {
        Self {
            format: OutputFormat::GltfSeparate,
            gltf: GltfExportOptions {
                draco_compression: false, // Preserve precision
                export_extras: true,
                export_lights: true,
                export_cameras: true,
                ..Default::default()
            },
            textures: TextureOptions {
                format: TextureFormat::Png,
                max_resolution: None, // Original resolution
                ..Default::default()
            },
            animation: AnimationOptions {
                export_animations: true,
                export_shape_keys: true,
                optimize_animation_size: false, // Preserve all keyframes
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Creates options for scene decomposition (split .blend into individual assets).
    pub fn scene_decomposition() -> Self {
        Self {
            format: OutputFormat::GlbBinary,
            gltf: GltfExportOptions {
                draco_compression: true,
                selected_only: true, // Critical: export one object at a time
                export_extras: true,
                ..Default::default()
            },
            textures: TextureOptions {
                format: TextureFormat::Png,
                max_resolution: Some(4096),
                ..Default::default()
            },
            mesh: MeshOptions {
                apply_modifiers: true,
                triangulate: true,
                ..Default::default()
            },
            decomposition: SceneDecompositionOptions::nature_pack(),
            process: ProcessOptions {
                // Scene decomposition exports hundreds of objects individually;
                // large nature packs (e.g. 279 meshes with Draco) need 10-20 min.
                timeout: Duration::from_secs(1800), // 30 minutes
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Returns a builder for custom options.
    pub fn builder() -> ConversionOptionsBuilder {
        ConversionOptionsBuilder::default()
    }
}

/// Output format for glTF export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum OutputFormat {
    /// Single binary .glb file (embedded textures).
    #[default]
    GlbBinary,
    /// .gltf with embedded data URIs.
    GltfEmbedded,
    /// .gltf with separate .bin and texture files.
    GltfSeparate,
}

impl OutputFormat {
    /// Returns the primary file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::GlbBinary => "glb",
            OutputFormat::GltfEmbedded | OutputFormat::GltfSeparate => "gltf",
        }
    }

    /// Returns the Blender export format string.
    pub fn blender_format(&self) -> &'static str {
        match self {
            OutputFormat::GlbBinary => "GLB",
            OutputFormat::GltfEmbedded => "GLTF_EMBEDDED",
            OutputFormat::GltfSeparate => "GLTF_SEPARATE",
        }
    }
}

/// glTF-specific export options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfExportOptions {
    /// Enable Draco mesh compression.
    pub draco_compression: bool,
    /// Draco compression level (0-10, higher = more compression).
    pub draco_compression_level: u8,
    /// Export custom properties as glTF extras.
    pub export_extras: bool,
    /// Export Blender lights.
    pub export_lights: bool,
    /// Export Blender cameras.
    pub export_cameras: bool,
    /// Use Y-up coordinate system (standard for glTF).
    pub y_up: bool,
    /// Export only selected objects.
    pub selected_only: bool,
    /// Export visible objects only.
    pub visible_only: bool,
    /// Export active collection only.
    pub active_collection_only: bool,
    /// Include armature/skeleton data.
    pub export_armatures: bool,
    /// Export skinning/vertex weights.
    pub export_skins: bool,
    /// Copyright string to embed.
    pub copyright: Option<String>,
}

impl Default for GltfExportOptions {
    fn default() -> Self {
        Self {
            draco_compression: true,
            draco_compression_level: 6,
            export_extras: true,
            export_lights: false,
            export_cameras: false,
            y_up: true,
            selected_only: false,
            visible_only: true,
            active_collection_only: false,
            export_armatures: true,
            export_skins: true,
            copyright: None,
        }
    }
}

/// Texture handling options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureOptions {
    /// Output texture format.
    pub format: TextureFormat,
    /// Maximum texture resolution (None = original).
    pub max_resolution: Option<u32>,
    /// JPEG quality (1-100).
    pub jpeg_quality: u8,
    /// WebP quality (1-100).
    pub webp_quality: u8,
    /// Always unpack embedded textures (recommended: true).
    pub unpack_embedded: bool,
    /// Generate mipmaps.
    pub generate_mipmaps: bool,
    /// Flip textures vertically (for OpenGL compatibility).
    pub flip_y: bool,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self {
            format: TextureFormat::Png,
            max_resolution: Some(4096),
            jpeg_quality: 90,
            webp_quality: 90,
            unpack_embedded: true, // Per user decision: always unpack
            generate_mipmaps: false,
            flip_y: false,
        }
    }
}

/// Texture output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum TextureFormat {
    /// PNG format (lossless, larger files).
    #[default]
    Png,
    /// JPEG format (lossy, smaller files).
    Jpeg,
    /// WebP format (good compression, requires Blender 3.4+).
    WebP,
    /// Keep original format from .blend file.
    Original,
}

impl TextureFormat {
    /// Returns the file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            TextureFormat::Png => "png",
            TextureFormat::Jpeg => "jpg",
            TextureFormat::WebP => "webp",
            TextureFormat::Original => "", // Determined at runtime
        }
    }
}

/// Animation export options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationOptions {
    /// Export animations.
    pub export_animations: bool,
    /// Export shape keys / blend shapes / morph targets.
    pub export_shape_keys: bool,
    /// Merge animations into a single clip.
    pub merge_animations: bool,
    /// Optimize animation data (remove redundant keyframes).
    pub optimize_animation_size: bool,
    /// Force linear interpolation.
    pub force_linear_interpolation: bool,
    /// Export NLA strips.
    pub export_nla_strips: bool,
    /// Sampling rate for baked animations (frames per second).
    pub sampling_rate: Option<f32>,
    /// Force sampling of all animations (disable curve export).
    pub force_sampling: bool,
}

impl Default for AnimationOptions {
    fn default() -> Self {
        Self {
            export_animations: true,
            export_shape_keys: true,
            merge_animations: false,
            optimize_animation_size: true,
            force_linear_interpolation: false,
            export_nla_strips: true,
            sampling_rate: None, // Use Blender default
            force_sampling: false,
        }
    }
}

/// Mesh processing options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshOptions {
    /// Apply modifiers before export.
    pub apply_modifiers: bool,
    /// Triangulate meshes.
    pub triangulate: bool,
    /// Export vertex colors.
    pub export_vertex_colors: bool,
    /// Export UVs.
    pub export_uvs: bool,
    /// Export normals.
    pub export_normals: bool,
    /// Export tangents.
    pub export_tangents: bool,
    /// Use mesh instancing where possible.
    pub use_mesh_instancing: bool,
    /// Merge vertices within this distance.
    pub merge_vertices_distance: Option<f32>,
    /// Loose edges export mode.
    pub export_loose_edges: bool,
    /// Loose points export mode.
    pub export_loose_points: bool,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            apply_modifiers: true,
            triangulate: true,
            export_vertex_colors: true,
            export_uvs: true,
            export_normals: true,
            export_tangents: true,
            use_mesh_instancing: true,
            merge_vertices_distance: None,
            export_loose_edges: false,
            export_loose_points: false,
        }
    }
}

/// Material export options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialOptions {
    /// Export materials.
    pub export_materials: bool,
    /// Export original Blender shader nodes as extras.
    pub export_original_specular: bool,
    /// Use environment maps.
    pub export_environment_maps: bool,
    /// Convert non-PBR materials to PBR approximation.
    pub convert_to_pbr: bool,
}

impl Default for MaterialOptions {
    fn default() -> Self {
        Self {
            export_materials: true,
            export_original_specular: false,
            export_environment_maps: false,
            convert_to_pbr: true,
        }
    }
}

/// Options for handling linked libraries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedLibraryOptions {
    /// Recursively process linked .blend files.
    pub process_recursively: bool,
    /// Maximum recursion depth for linked libraries.
    pub max_recursion_depth: u32,
    /// Directories to search for linked libraries.
    pub search_paths: Vec<PathBuf>,
    /// How to handle missing linked libraries.
    pub missing_library_action: MissingLibraryAction,
    /// Track processed libraries to detect circular references.
    pub detect_circular_references: bool,
}

impl Default for LinkedLibraryOptions {
    fn default() -> Self {
        Self {
            process_recursively: true, // Per user decision
            max_recursion_depth: 10,
            search_paths: Vec::new(),
            missing_library_action: MissingLibraryAction::Warn,
            detect_circular_references: true, // Per user decision
        }
    }
}

/// Action to take when a linked library is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum MissingLibraryAction {
    /// Skip missing libraries silently.
    Skip,
    /// Warn about missing libraries but continue.
    #[default]
    Warn,
    /// Fail the conversion if any library is missing.
    Fail,
}

/// Process control options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOptions {
    /// Timeout for Blender process.
    pub timeout: Duration,
    /// Allow cancellation.
    pub cancellable: bool,
    /// Working directory for Blender (None = temp dir).
    pub working_directory: Option<PathBuf>,
    /// Additional Blender command line arguments.
    pub extra_blender_args: Vec<String>,
    /// Environment variables to set.
    pub environment: Vec<(String, String)>,
    /// Capture Blender stdout/stderr for debugging.
    pub capture_output: bool,
    /// Number of threads Blender should use (0 = auto).
    pub threads: u32,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300), // 5 minutes
            cancellable: true,
            working_directory: None,
            extra_blender_args: Vec::new(),
            environment: Vec::new(),
            capture_output: true,
            threads: 0, // Auto-detect
        }
    }
}

/// Cache behavior options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheOptions {
    /// Enable caching of converted files.
    pub enabled: bool,
    /// Cache directory (None = default in project/.astraweave/blend_cache/).
    pub cache_directory: Option<PathBuf>,
    /// Maximum cache size in bytes (None = unlimited).
    pub max_cache_size: Option<u64>,
    /// Maximum age of cache entries (None = never expire).
    pub max_age: Option<Duration>,
    /// Re-validate cache entries by checking source file modification time.
    pub validate_on_access: bool,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_directory: None,
            max_cache_size: Some(10 * 1024 * 1024 * 1024), // 10 GB
            max_age: None,
            validate_on_access: true,
        }
    }
}

/// Builder for ConversionOptions.
#[derive(Debug, Default)]
pub struct ConversionOptionsBuilder {
    options: ConversionOptions,
}

impl ConversionOptionsBuilder {
    /// Sets the output format.
    pub fn format(mut self, format: OutputFormat) -> Self {
        self.options.format = format;
        self
    }

    /// Enables or disables Draco compression.
    pub fn draco_compression(mut self, enabled: bool) -> Self {
        self.options.gltf.draco_compression = enabled;
        self
    }

    /// Sets texture format.
    pub fn texture_format(mut self, format: TextureFormat) -> Self {
        self.options.textures.format = format;
        self
    }

    /// Sets maximum texture resolution.
    pub fn max_texture_resolution(mut self, resolution: Option<u32>) -> Self {
        self.options.textures.max_resolution = resolution;
        self
    }

    /// Sets process timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.process.timeout = timeout;
        self
    }

    /// Enables or disables animation export.
    pub fn export_animations(mut self, enabled: bool) -> Self {
        self.options.animation.export_animations = enabled;
        self
    }

    /// Enables or disables modifiers application.
    pub fn apply_modifiers(mut self, enabled: bool) -> Self {
        self.options.mesh.apply_modifiers = enabled;
        self
    }

    /// Sets linked library recursion depth.
    pub fn linked_library_depth(mut self, depth: u32) -> Self {
        self.options.linked_libraries.max_recursion_depth = depth;
        self
    }

    /// Enables or disables caching.
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.options.cache.enabled = enabled;
        self
    }

    /// Enables scene decomposition with custom options.
    pub fn decomposition(mut self, decomposition: SceneDecompositionOptions) -> Self {
        self.options.decomposition = decomposition;
        self
    }

    /// Builds the ConversionOptions.
    pub fn build(self) -> ConversionOptions {
        self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = ConversionOptions::default();
        assert!(options.gltf.draco_compression);
        assert!(options.textures.unpack_embedded);
        assert!(options.linked_libraries.process_recursively);
    }

    #[test]
    fn test_game_runtime_preset() {
        let options = ConversionOptions::game_runtime();
        assert!(options.gltf.draco_compression);
        assert!(options.mesh.triangulate);
        assert_eq!(options.textures.max_resolution, Some(2048));
    }

    #[test]
    fn test_editor_preview_preset() {
        let options = ConversionOptions::editor_preview();
        assert!(!options.gltf.draco_compression);
        assert_eq!(options.textures.max_resolution, Some(512));
    }

    #[test]
    fn test_builder() {
        let options = ConversionOptions::builder()
            .format(OutputFormat::GltfSeparate)
            .draco_compression(false)
            .max_texture_resolution(Some(1024))
            .timeout(Duration::from_secs(60))
            .build();

        assert_eq!(options.format, OutputFormat::GltfSeparate);
        assert!(!options.gltf.draco_compression);
        assert_eq!(options.textures.max_resolution, Some(1024));
        assert_eq!(options.process.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_output_format_extensions() {
        assert_eq!(OutputFormat::GlbBinary.extension(), "glb");
        assert_eq!(OutputFormat::GltfEmbedded.extension(), "gltf");
        assert_eq!(OutputFormat::GltfSeparate.extension(), "gltf");
    }

    #[test]
    fn test_scene_decomposition_preset() {
        let options = ConversionOptions::scene_decomposition();
        assert!(options.decomposition.enabled);
        assert!(options.gltf.selected_only);
        assert!(options.decomposition.generate_manifest);
        assert!(options.decomposition.extract_textures);
        assert!(options.decomposition.extract_hdris);
        assert_eq!(
            options.decomposition.group_by,
            DecompositionGrouping::ByObject
        );
    }

    #[test]
    fn test_decomposition_defaults_disabled() {
        let options = ConversionOptions::default();
        assert!(!options.decomposition.enabled);
    }

    #[test]
    fn test_nature_pack_preset() {
        let nature = SceneDecompositionOptions::nature_pack();
        assert!(nature.enabled);
        assert_eq!(nature.asset_category, AssetCategory::Auto);
        assert!(nature.exclude_patterns.contains(&"Camera".to_string()));
        assert!(nature.exclude_patterns.contains(&"Light".to_string()));
    }
}

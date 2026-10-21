//! Cell Data Loader for World Partition
//!
//! This module handles loading and deserialization of world partition cells from RON files.
//! Cells contain entity data, asset references, and metadata for streaming.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// Asset reference types for cell content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetKind {
    Mesh,
    Texture,
    Material,
    Audio,
    Animation,
    Other,
}

/// Reference to an asset that needs to be loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRef {
    /// Path to the asset file (relative to assets root)
    pub path: String,
    /// Type of asset
    pub kind: AssetKind,
    /// Optional GUID for asset database integration
    pub guid: Option<String>,
}

impl AssetRef {
    pub fn new(path: impl Into<String>, kind: AssetKind) -> Self {
        Self {
            path: path.into(),
            kind,
            guid: None,
        }
    }

    pub fn with_guid(mut self, guid: impl Into<String>) -> Self {
        self.guid = Some(guid.into());
        self
    }
}

/// Entity data for a single entity in the cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    /// Optional entity name (for debugging)
    pub name: Option<String>,
    /// World position
    pub position: [f32; 3],
    /// Rotation (quaternion: x, y, z, w)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
    /// Mesh asset reference (if renderable)
    pub mesh: Option<String>,
    /// Material layer index (if renderable)
    pub material: Option<u32>,
    /// Additional components (extensible)
    pub components: Vec<ComponentData>,
}

impl EntityData {
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            name: None,
            position,
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
            mesh: None,
            material: None,
            components: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_mesh(mut self, mesh_path: impl Into<String>) -> Self {
        self.mesh = Some(mesh_path.into());
        self
    }

    pub fn with_material(mut self, material_index: u32) -> Self {
        self.material = Some(material_index);
        self
    }
}

/// Generic component data (for extensibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    pub component_type: String,
    pub data: String, // JSON or TOML serialized component data
}

/// Complete cell data structure (serialized to RON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    /// Cell grid coordinate (for validation)
    pub coord: [i32; 3],
    /// List of entities in this cell
    pub entities: Vec<EntityData>,
    /// List of assets referenced by entities
    pub assets: Vec<AssetRef>,
    /// Optional cell-level metadata
    pub metadata: Option<CellMetadata>,
}

impl CellData {
    pub fn new(coord: [i32; 3]) -> Self {
        Self {
            coord,
            entities: Vec::new(),
            assets: Vec::new(),
            metadata: None,
        }
    }

    /// Add an entity to the cell
    pub fn add_entity(&mut self, entity: EntityData) {
        self.entities.push(entity);
    }

    /// Add an asset reference
    pub fn add_asset(&mut self, asset: AssetRef) {
        // Avoid duplicates
        if !self.assets.iter().any(|a| a.path == asset.path) {
            self.assets.push(asset);
        }
    }

    /// Estimate memory usage in bytes
    pub fn memory_estimate(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        total += self.entities.len() * std::mem::size_of::<EntityData>();
        total += self.assets.len() * std::mem::size_of::<AssetRef>();
        total
    }
}

/// Cell metadata (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetadata {
    /// Human-readable description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Version number for compatibility
    pub version: u32,
}

/// Load cell data from a RON file asynchronously
pub async fn load_cell_from_ron(path: &Path) -> Result<CellData> {
    // Read file contents
    let contents = fs::read_to_string(path)
        .await
        .context(format!("Failed to read cell file: {}", path.display()))?;

    // Deserialize RON
    let data: CellData = ron::from_str(&contents)
        .context(format!("Failed to parse RON from: {}", path.display()))?;

    Ok(data)
}

/// Load cell data synchronously (for non-async contexts)
pub fn load_cell_from_ron_sync(path: &Path) -> Result<CellData> {
    let contents = std::fs::read_to_string(path)
        .context(format!("Failed to read cell file: {}", path.display()))?;

    let data: CellData = ron::from_str(&contents)
        .context(format!("Failed to parse RON from: {}", path.display()))?;

    Ok(data)
}

/// Save cell data to a RON file asynchronously
pub async fn save_cell_to_ron(path: &Path, data: &CellData) -> Result<()> {
    // Serialize to RON with pretty formatting
    let ron_string = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
        .context("Failed to serialize cell data to RON")?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .context(format!("Failed to create directory: {}", parent.display()))?;
    }

    // Write file
    fs::write(path, ron_string)
        .await
        .context(format!("Failed to write cell file: {}", path.display()))?;

    Ok(())
}

/// Save cell data synchronously
pub fn save_cell_to_ron_sync(path: &Path, data: &CellData) -> Result<()> {
    let ron_string = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
        .context("Failed to serialize cell data to RON")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context(format!("Failed to create directory: {}", parent.display()))?;
    }

    std::fs::write(path, ron_string)
        .context(format!("Failed to write cell file: {}", path.display()))?;

    Ok(())
}

/// Load an asset referenced by a cell
pub async fn load_asset(asset_ref: &AssetRef, assets_root: &Path) -> Result<Vec<u8>> {
    let asset_path = astraweave_security::path::safe_under(assets_root, Path::new(&asset_ref.path))
        .map_err(|err| anyhow::anyhow!("path traversal rejected: {err}"))?;

    match asset_ref.kind {
        AssetKind::Mesh => {
            let bytes = fs::read(&asset_path)
                .await
                .context(format!("Failed to load mesh: {}", asset_path.display()))?;
            // Additional validation for mesh formats
            validate_mesh_format(&bytes, &asset_ref.path)?;
            Ok(bytes)
        }
        AssetKind::Texture => {
            let bytes = crate::read_limited_image_bytes(&asset_path)
                .with_context(|| format!("Failed to load texture: {}", asset_path.display()))?;
            // Additional validation for texture formats
            validate_texture_format(&bytes, &asset_ref.path)?;
            Ok(bytes)
        }
        AssetKind::Material => {
            let bytes = fs::read(&asset_path)
                .await
                .context(format!("Failed to load material: {}", asset_path.display()))?;
            Ok(bytes)
        }
        AssetKind::Audio => {
            let bytes = fs::read(&asset_path)
                .await
                .context(format!("Failed to load audio: {}", asset_path.display()))?;
            Ok(bytes)
        }
        AssetKind::Animation => {
            let bytes = fs::read(&asset_path).await.context(format!(
                "Failed to load animation: {}",
                asset_path.display()
            ))?;
            Ok(bytes)
        }
        AssetKind::Other => {
            let bytes = fs::read(&asset_path)
                .await
                .context(format!("Failed to load asset: {}", asset_path.display()))?;
            Ok(bytes)
        }
    }
}

/// Validate mesh file format (basic header check)
fn validate_mesh_format(bytes: &[u8], path: &str) -> Result<()> {
    if path.ends_with(".glb") {
        // GLB magic number: "glTF" (0x46546C67)
        if bytes.len() >= 4 && &bytes[0..4] == b"glTF" {
            return Ok(());
        }
        anyhow::bail!("Invalid GLB file: missing magic number");
    } else if path.ends_with(".gltf") {
        // GLTF is JSON, check for basic structure
        let json_str = std::str::from_utf8(bytes).context("GLTF file is not valid UTF-8")?;
        if json_str.contains("\"meshes\"") && json_str.contains("\"accessors\"") {
            return Ok(());
        }
        anyhow::bail!("Invalid GLTF file: missing required fields");
    }
    // Other formats: assume valid (extend as needed)
    Ok(())
}

/// Validate texture file format (basic header check)
fn validate_texture_format(bytes: &[u8], path: &str) -> Result<()> {
    if path.ends_with(".png") {
        // PNG magic number: 0x89504E47
        if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
            return Ok(());
        }
        anyhow::bail!("Invalid PNG file: missing magic number");
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        // JPEG magic number: 0xFFD8FF
        if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
            return Ok(());
        }
        anyhow::bail!("Invalid JPEG file: missing magic number");
    }
    // Other formats: assume valid
    Ok(())
}

/// Helper to generate cell file path from grid coordinate
pub fn cell_path_from_coord(coord: [i32; 3], cells_root: &Path) -> std::path::PathBuf {
    cells_root.join(format!("{}_{}_{}.ron", coord[0], coord[1], coord[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_cell_data_creation() {
        let cell = CellData::new([0, 0, 0]);
        assert_eq!(cell.coord, [0, 0, 0]);
        assert_eq!(cell.entities.len(), 0);
        assert_eq!(cell.assets.len(), 0);
    }

    // ===== AssetKind Tests =====

    #[test]
    fn test_asset_kind_variants() {
        assert_eq!(AssetKind::Mesh, AssetKind::Mesh);
        assert_eq!(AssetKind::Texture, AssetKind::Texture);
        assert_eq!(AssetKind::Material, AssetKind::Material);
        assert_eq!(AssetKind::Audio, AssetKind::Audio);
        assert_eq!(AssetKind::Animation, AssetKind::Animation);
        assert_eq!(AssetKind::Other, AssetKind::Other);
    }

    #[test]
    fn test_asset_kind_not_equal() {
        assert_ne!(AssetKind::Mesh, AssetKind::Texture);
        assert_ne!(AssetKind::Audio, AssetKind::Animation);
        assert_ne!(AssetKind::Material, AssetKind::Other);
    }

    #[test]
    fn test_asset_kind_serialization() {
        let kinds = vec![
            AssetKind::Mesh,
            AssetKind::Texture,
            AssetKind::Material,
            AssetKind::Audio,
            AssetKind::Animation,
            AssetKind::Other,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let deserialized: AssetKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, deserialized);
        }
    }

    // ===== AssetRef Tests =====

    #[test]
    fn test_asset_ref_new() {
        let asset = AssetRef::new("textures/grass.png", AssetKind::Texture);
        assert_eq!(asset.path, "textures/grass.png");
        assert_eq!(asset.kind, AssetKind::Texture);
        assert!(asset.guid.is_none());
    }

    #[test]
    fn test_asset_ref_clone() {
        let asset = AssetRef::new("model.glb", AssetKind::Mesh).with_guid("guid123");
        let cloned = asset.clone();
        assert_eq!(cloned.path, "model.glb");
        assert_eq!(cloned.guid, Some("guid123".to_string()));
    }

    #[test]
    fn test_asset_ref_debug() {
        let asset = AssetRef::new("test.png", AssetKind::Texture);
        let debug = format!("{:?}", asset);
        assert!(debug.contains("test.png"));
        assert!(debug.contains("Texture"));
    }

    // ===== EntityData Tests =====

    #[test]
    fn test_entity_data_defaults() {
        let entity = EntityData::new([0.0, 0.0, 0.0]);
        assert!(entity.name.is_none());
        assert_eq!(entity.position, [0.0, 0.0, 0.0]);
        assert_eq!(entity.rotation, [0.0, 0.0, 0.0, 1.0]); // Identity quaternion
        assert_eq!(entity.scale, [1.0, 1.0, 1.0]);
        assert!(entity.mesh.is_none());
        assert!(entity.material.is_none());
        assert!(entity.components.is_empty());
    }

    #[test]
    fn test_entity_data_builder_chain() {
        let entity = EntityData::new([5.0, 10.0, 15.0])
            .with_name("hero")
            .with_mesh("models/hero.glb")
            .with_material(5);

        assert_eq!(entity.name, Some("hero".to_string()));
        assert_eq!(entity.position, [5.0, 10.0, 15.0]);
        assert_eq!(entity.mesh, Some("models/hero.glb".to_string()));
        assert_eq!(entity.material, Some(5));
    }

    #[test]
    fn test_entity_data_clone() {
        let entity = EntityData::new([1.0, 2.0, 3.0]).with_name("test");
        let cloned = entity.clone();
        assert_eq!(cloned.position, [1.0, 2.0, 3.0]);
        assert_eq!(cloned.name, Some("test".to_string()));
    }

    #[test]
    fn test_entity_data_serialization() {
        let entity = EntityData::new([1.0, 2.0, 3.0])
            .with_name("serialized")
            .with_mesh("mesh.glb")
            .with_material(2);

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: EntityData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, Some("serialized".to_string()));
        assert_eq!(deserialized.position, [1.0, 2.0, 3.0]);
        assert_eq!(deserialized.mesh, Some("mesh.glb".to_string()));
        assert_eq!(deserialized.material, Some(2));
    }

    // ===== ComponentData Tests =====

    #[test]
    fn test_component_data_creation() {
        let component = ComponentData {
            component_type: "Collider".to_string(),
            data: r#"{"radius": 1.0}"#.to_string(),
        };
        assert_eq!(component.component_type, "Collider");
        assert!(component.data.contains("radius"));
    }

    #[test]
    fn test_component_data_clone() {
        let component = ComponentData {
            component_type: "Health".to_string(),
            data: r#"{"max": 100}"#.to_string(),
        };
        let cloned = component.clone();
        assert_eq!(cloned.component_type, "Health");
        assert_eq!(cloned.data, r#"{"max": 100}"#);
    }

    #[test]
    fn test_component_data_serialization() {
        let component = ComponentData {
            component_type: "Script".to_string(),
            data: "script_path.rhai".to_string(),
        };

        let json = serde_json::to_string(&component).unwrap();
        let deserialized: ComponentData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.component_type, "Script");
        assert_eq!(deserialized.data, "script_path.rhai");
    }

    // ===== CellMetadata Tests =====

    #[test]
    fn test_cell_metadata_creation() {
        let metadata = CellMetadata {
            description: Some("Forest region".to_string()),
            tags: vec!["forest".to_string(), "outdoor".to_string()],
            version: 1,
        };

        assert_eq!(metadata.description, Some("Forest region".to_string()));
        assert_eq!(metadata.tags.len(), 2);
        assert_eq!(metadata.version, 1);
    }

    #[test]
    fn test_cell_metadata_empty() {
        let metadata = CellMetadata {
            description: None,
            tags: vec![],
            version: 0,
        };

        assert!(metadata.description.is_none());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_cell_metadata_serialization() {
        let metadata = CellMetadata {
            description: Some("Dungeon".to_string()),
            tags: vec!["underground".to_string()],
            version: 2,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CellMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.description, Some("Dungeon".to_string()));
        assert_eq!(deserialized.version, 2);
    }

    // ===== CellData Extended Tests =====

    #[test]
    fn test_cell_data_with_metadata() {
        let mut cell = CellData::new([0, 0, 0]);
        cell.metadata = Some(CellMetadata {
            description: Some("Test cell".to_string()),
            tags: vec!["test".to_string()],
            version: 1,
        });

        assert!(cell.metadata.is_some());
        assert_eq!(cell.metadata.as_ref().unwrap().version, 1);
    }

    #[test]
    fn test_cell_data_add_multiple_entities() {
        let mut cell = CellData::new([1, 1, 1]);
        for i in 0..10 {
            cell.add_entity(EntityData::new([i as f32, 0.0, 0.0]));
        }
        assert_eq!(cell.entities.len(), 10);
    }

    #[test]
    fn test_cell_data_add_different_assets() {
        let mut cell = CellData::new([0, 0, 0]);
        cell.add_asset(AssetRef::new("mesh.glb", AssetKind::Mesh));
        cell.add_asset(AssetRef::new("tex.png", AssetKind::Texture));
        cell.add_asset(AssetRef::new("sound.wav", AssetKind::Audio));

        assert_eq!(cell.assets.len(), 3);
    }

    // ===== Validation Function Tests =====

    #[test]
    fn test_texture_validation_jpeg() {
        let valid_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert!(validate_texture_format(&valid_jpeg, "test.jpg").is_ok());

        let valid_jpeg2 = vec![0xFF, 0xD8, 0xFF, 0xE1];
        assert!(validate_texture_format(&valid_jpeg2, "test.jpeg").is_ok());

        let invalid_jpeg = b"INVALID";
        assert!(validate_texture_format(invalid_jpeg, "test.jpg").is_err());
    }

    #[test]
    fn test_texture_validation_other_formats() {
        // Other formats should pass validation (assume valid)
        let unknown_data = b"some random data";
        assert!(validate_texture_format(unknown_data, "test.ktx2").is_ok());
        assert!(validate_texture_format(unknown_data, "test.dds").is_ok());
    }

    #[test]
    fn test_mesh_validation_gltf() {
        let valid_gltf = br#"{"meshes": [], "accessors": []}"#;
        assert!(validate_mesh_format(valid_gltf, "test.gltf").is_ok());

        let invalid_gltf = br#"{"some": "json"}"#;
        assert!(validate_mesh_format(invalid_gltf, "test.gltf").is_err());
    }

    #[test]
    fn test_mesh_validation_other_formats() {
        // Other formats should pass validation
        let unknown_data = b"some obj data";
        assert!(validate_mesh_format(unknown_data, "test.obj").is_ok());
    }

    #[test]
    fn test_mesh_validation_glb_short_data() {
        // GLB with insufficient bytes
        let short_glb = b"gl";
        assert!(validate_mesh_format(short_glb, "test.glb").is_err());
    }

    #[test]
    fn test_texture_validation_png_short_data() {
        // PNG with insufficient bytes
        let short_png = b"\x89PNG";
        assert!(validate_texture_format(short_png, "test.png").is_err());
    }

    // ===== cell_path_from_coord Tests =====

    #[test]
    fn test_cell_path_positive_coords() {
        let path = cell_path_from_coord([5, 10, 15], Path::new("cells"));
        assert_eq!(path, PathBuf::from("cells/5_10_15.ron"));
    }

    #[test]
    fn test_cell_path_negative_coords() {
        let path = cell_path_from_coord([-5, -10, -15], Path::new("world/cells"));
        assert_eq!(path, PathBuf::from("world/cells/-5_-10_-15.ron"));
    }

    #[test]
    fn test_cell_path_zero_coord() {
        let path = cell_path_from_coord([0, 0, 0], Path::new("data"));
        assert_eq!(path, PathBuf::from("data/0_0_0.ron"));
    }

    // ===== Sync Load/Save Tests =====

    #[test]
    fn test_sync_load_nonexistent() {
        let result = load_cell_from_ron_sync(Path::new("nonexistent_file.ron"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_cell.ron");

        let mut cell = CellData::new([1, 2, 3]);
        cell.add_entity(EntityData::new([10.0, 20.0, 30.0]).with_name("test_entity"));
        cell.add_asset(AssetRef::new("mesh.glb", AssetKind::Mesh));

        // Save
        save_cell_to_ron_sync(&file_path, &cell).expect("Failed to save");

        // Load
        let loaded = load_cell_from_ron_sync(&file_path).expect("Failed to load");

        assert_eq!(loaded.coord, [1, 2, 3]);
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, Some("test_entity".to_string()));
        assert_eq!(loaded.assets.len(), 1);
    }

    #[test]
    fn test_sync_save_creates_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir
            .path()
            .join("subdir")
            .join("nested")
            .join("cell.ron");

        let cell = CellData::new([0, 0, 0]);
        save_cell_to_ron_sync(&file_path, &cell).expect("Failed to save");

        assert!(file_path.exists());
    }

    // ===== Async Load/Save Tests =====

    #[tokio::test]
    async fn test_async_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("async_cell.ron");

        let mut cell = CellData::new([5, 6, 7]);
        cell.add_entity(EntityData::new([1.0, 2.0, 3.0]).with_name("async_entity"));
        cell.metadata = Some(CellMetadata {
            description: Some("Async test".to_string()),
            tags: vec!["async".to_string()],
            version: 1,
        });

        // Save async
        save_cell_to_ron(&file_path, &cell)
            .await
            .expect("Failed to async save");

        // Load async
        let loaded = load_cell_from_ron(&file_path)
            .await
            .expect("Failed to async load");

        assert_eq!(loaded.coord, [5, 6, 7]);
        assert!(loaded.metadata.is_some());
        assert_eq!(
            loaded.metadata.as_ref().unwrap().description,
            Some("Async test".to_string())
        );
    }

    #[tokio::test]
    async fn test_async_save_creates_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("async_subdir").join("cell.ron");

        let cell = CellData::new([0, 0, 0]);
        save_cell_to_ron(&file_path, &cell)
            .await
            .expect("Failed to async save");

        assert!(file_path.exists());
    }

    // ===== memory_estimate Tests =====

    #[test]
    fn test_memory_estimate_empty_cell() {
        let cell = CellData::new([0, 0, 0]);
        let estimate = cell.memory_estimate();
        // Should at least be the size of the struct itself
        assert!(estimate >= std::mem::size_of::<CellData>());
    }

    #[test]
    fn test_memory_estimate_scales_with_content() {
        let cell_empty = CellData::new([0, 0, 0]);
        let mut cell_full = CellData::new([0, 0, 0]);

        for i in 0..100 {
            cell_full.add_entity(EntityData::new([i as f32, 0.0, 0.0]));
            cell_full.add_asset(AssetRef::new(format!("asset_{}.glb", i), AssetKind::Mesh));
        }

        assert!(cell_full.memory_estimate() > cell_empty.memory_estimate());
    }

    #[test]
    fn test_entity_data_builder() {
        let entity = EntityData::new([1.0, 2.0, 3.0])
            .with_name("test_entity")
            .with_mesh("models/cube.glb")
            .with_material(0);

        assert_eq!(entity.name.as_deref(), Some("test_entity"));
        assert_eq!(entity.position, [1.0, 2.0, 3.0]);
        assert_eq!(entity.mesh.as_deref(), Some("models/cube.glb"));
        assert_eq!(entity.material, Some(0));
    }

    #[test]
    fn test_asset_ref() {
        let asset = AssetRef::new("textures/grass.png", AssetKind::Texture).with_guid("abc123");

        assert_eq!(asset.path, "textures/grass.png");
        assert_eq!(asset.kind, AssetKind::Texture);
        assert_eq!(asset.guid.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_cell_add_entity() {
        let mut cell = CellData::new([0, 0, 0]);
        let entity = EntityData::new([1.0, 0.0, 1.0]);

        cell.add_entity(entity);
        assert_eq!(cell.entities.len(), 1);
    }

    #[test]
    fn test_cell_add_asset_no_duplicates() {
        let mut cell = CellData::new([0, 0, 0]);
        let asset1 = AssetRef::new("models/tree.glb", AssetKind::Mesh);
        let asset2 = AssetRef::new("models/tree.glb", AssetKind::Mesh);

        cell.add_asset(asset1);
        cell.add_asset(asset2); // Duplicate should be ignored

        assert_eq!(cell.assets.len(), 1);
    }

    #[test]
    fn test_ron_serialization() {
        let mut cell = CellData::new([1, 2, 3]);
        cell.add_entity(EntityData::new([10.0, 0.0, 20.0]).with_name("tree"));
        cell.add_asset(AssetRef::new("models/tree.glb", AssetKind::Mesh));

        // Serialize to RON
        let ron_string = ron::ser::to_string_pretty(&cell, ron::ser::PrettyConfig::default())
            .expect("Failed to serialize");

        // Deserialize back
        let deserialized: CellData = ron::from_str(&ron_string).expect("Failed to deserialize");

        assert_eq!(deserialized.coord, [1, 2, 3]);
        assert_eq!(deserialized.entities.len(), 1);
        assert_eq!(deserialized.assets.len(), 1);
    }

    #[test]
    fn test_cell_path_generation() {
        let path = cell_path_from_coord([1, 0, -2], Path::new("assets/cells"));
        assert_eq!(path, PathBuf::from("assets/cells/1_0_-2.ron"));
    }

    #[tokio::test]
    async fn test_async_cell_loading_nonexistent() {
        let result = load_cell_from_ron(Path::new("nonexistent.ron")).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_mesh_validation_glb() {
        let valid_glb = b"glTF\x02\x00\x00\x00";
        assert!(validate_mesh_format(valid_glb, "test.glb").is_ok());

        let invalid_glb = b"INVALID";
        assert!(validate_mesh_format(invalid_glb, "test.glb").is_err());
    }

    #[test]
    fn test_texture_validation_png() {
        let valid_png = b"\x89PNG\r\n\x1a\n";
        assert!(validate_texture_format(valid_png, "test.png").is_ok());

        let invalid_png = b"INVALID";
        assert!(validate_texture_format(invalid_png, "test.png").is_err());
    }

    #[test]
    fn test_memory_estimate() {
        let mut cell = CellData::new([0, 0, 0]);
        cell.add_entity(EntityData::new([0.0, 0.0, 0.0]));
        cell.add_entity(EntityData::new([1.0, 0.0, 0.0]));
        cell.add_asset(AssetRef::new("mesh.glb", AssetKind::Mesh));

        let estimate = cell.memory_estimate();
        assert!(estimate > 0);
    }
}

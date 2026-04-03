use glam::{Quat, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Unique identifier for entities in the editor
pub type EntityId = u64;

// ============================================================================
// MATERIAL SYSTEM - PBR material slots for texture assignment
// ============================================================================

/// Material texture slots for PBR (Physically Based Rendering) workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
#[non_exhaustive]
pub enum MaterialSlot {
    /// Base color / albedo texture
    Albedo,
    /// Normal map (tangent space)
    Normal,
    /// Roughness texture
    Roughness,
    /// Metallic texture
    Metallic,
    /// Ambient occlusion
    AO,
    /// Combined ORM (Occlusion-Roughness-Metallic)
    ORM,
    /// Emissive/glow texture
    Emission,
    /// Height/displacement map
    Height,
}

/// Material data for an entity with PBR textures and properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMaterial {
    /// Material name or path
    pub name: String,
    /// Texture paths assigned to each material slot
    pub texture_slots: HashMap<MaterialSlot, PathBuf>,
    /// Base color tint (multiplied with albedo texture)
    pub base_color: Vec4,
    /// Metallic factor (0.0 = dielectric, 1.0 = metal)
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness: f32,
    /// Emissive color (HDR capable)
    pub emissive: Vec3,
    /// Normal map strength
    pub normal_strength: f32,
}

impl Default for EntityMaterial {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            texture_slots: HashMap::new(),
            base_color: Vec4::ONE, // White
            metallic: 0.0,         // Dielectric by default
            roughness: 0.5,        // Medium roughness
            emissive: Vec3::ZERO,  // No emission
            normal_strength: 1.0,  // Full normal strength
        }
    }
}

impl EntityMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a texture for a specific material slot
    pub fn set_texture(&mut self, slot: MaterialSlot, path: PathBuf) {
        self.texture_slots.insert(slot, path);
    }

    /// Remove a texture from a slot
    pub fn clear_texture(&mut self, slot: MaterialSlot) {
        self.texture_slots.remove(&slot);
    }

    /// Get the texture path for a slot
    pub fn get_texture(&self, slot: MaterialSlot) -> Option<&PathBuf> {
        self.texture_slots.get(&slot)
    }

    /// Check if material has any textures assigned
    pub fn has_textures(&self) -> bool {
        !self.texture_slots.is_empty()
    }

    /// Get number of texture slots filled
    pub fn texture_count(&self) -> usize {
        self.texture_slots.len()
    }

    /// Check if material is metallic (metallic > 0.5)
    pub fn is_metallic(&self) -> bool {
        self.metallic > 0.5
    }

    /// Check if material is rough (roughness > 0.5)
    pub fn is_rough(&self) -> bool {
        self.roughness > 0.5
    }

    /// Check if material is emissive
    pub fn is_emissive(&self) -> bool {
        self.emissive.length_squared() > 0.001
    }

    /// Get summary of material properties
    pub fn summary(&self) -> String {
        format!(
            "{}: M={:.1} R={:.1} {} textures",
            self.name,
            self.metallic,
            self.roughness,
            self.texture_slots.len()
        )
    }
}

impl std::fmt::Display for EntityMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (M:{:.1}, R:{:.1})",
            self.name, self.metallic, self.roughness
        )
    }
}

impl MaterialSlot {
    /// Get all material slots
    pub fn all() -> &'static [MaterialSlot] {
        &[
            MaterialSlot::Albedo,
            MaterialSlot::Normal,
            MaterialSlot::Roughness,
            MaterialSlot::Metallic,
            MaterialSlot::AO,
            MaterialSlot::ORM,
            MaterialSlot::Emission,
            MaterialSlot::Height,
        ]
    }

    /// Get display name for slot
    pub fn name(&self) -> &'static str {
        match self {
            MaterialSlot::Albedo => "Albedo",
            MaterialSlot::Normal => "Normal",
            MaterialSlot::Roughness => "Roughness",
            MaterialSlot::Metallic => "Metallic",
            MaterialSlot::AO => "Ambient Occlusion",
            MaterialSlot::ORM => "ORM (Combined)",
            MaterialSlot::Emission => "Emission",
            MaterialSlot::Height => "Height",
        }
    }

    /// Get icon for slot
    pub fn icon(&self) -> &'static str {
        match self {
            MaterialSlot::Albedo => "[Alb]",
            MaterialSlot::Normal => "↗",
            MaterialSlot::Roughness => "◐",
            MaterialSlot::Metallic => "⚙",
            MaterialSlot::AO => "◑",
            MaterialSlot::ORM => "[ORM]",
            MaterialSlot::Emission => "[Em]",
            MaterialSlot::Height => "[Ht]",
        }
    }

    /// Check if this is a color slot (vs data slot)
    pub fn is_color_slot(&self) -> bool {
        matches!(self, MaterialSlot::Albedo | MaterialSlot::Emission)
    }

    /// Check if this is a data slot (non-color)
    pub fn is_data_slot(&self) -> bool {
        !self.is_color_slot()
    }
}

impl std::fmt::Display for MaterialSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// ENTITY - Core entity representation
// ============================================================================

/// Entity representation in the editor
/// Stores transform, mesh reference, material, and arbitrary components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorEntity {
    pub id: EntityId,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub mesh: Option<String>,
    pub material: EntityMaterial,
    pub components: HashMap<String, serde_json::Value>,
    /// Parent entity ID (None = root)
    pub parent: Option<EntityId>,
    /// Auto-generated scatter object (vegetation, rocks, props from biome packs).
    /// Scatter entities are procedural — excluded from serialization and scene hierarchy.
    #[serde(default)]
    pub is_scatter: bool,
}

#[allow(dead_code)]
impl EditorEntity {
    pub fn new(id: EntityId, name: String) -> Self {
        Self {
            id,
            name,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            mesh: None,
            material: EntityMaterial::new(),
            components: HashMap::new(),
            parent: None,
            is_scatter: false,
        }
    }

    /// Create a scatter entity (auto-generated vegetation/rocks from biome packs)
    pub fn new_scatter(id: EntityId, name: String) -> Self {
        Self {
            is_scatter: true,
            ..Self::new(id, name)
        }
    }

    /// Get the entity's transform as (position, rotation, scale)
    pub fn transform(&self) -> (Vec3, Quat, Vec3) {
        (self.position, self.rotation, self.scale)
    }

    /// Set the entity's transform
    pub fn set_transform(&mut self, position: Vec3, rotation: Quat, scale: Vec3) {
        self.position = position;
        self.rotation = rotation;
        self.scale = scale;
    }

    /// Set a texture on the entity's material
    pub fn set_texture(&mut self, slot: MaterialSlot, path: PathBuf) {
        self.material.set_texture(slot, path);
    }

    /// Set the entire material
    pub fn set_material(&mut self, material: EntityMaterial) {
        self.material = material;
    }

    /// Set the mesh path
    pub fn set_mesh(&mut self, mesh_path: String) {
        self.mesh = Some(mesh_path);
    }

    /// Get axis-aligned bounding box (AABB) for picking
    /// Returns (min, max) corners
    pub fn aabb(&self) -> (Vec3, Vec3) {
        // Simple unit cube scaled by entity scale
        let half_size = self.scale * 0.5;
        let min = self.position - half_size;
        let max = self.position + half_size;
        (min, max)
    }

    /// Validate entity data and return any issues found.
    pub fn validate(&self) -> EntityValidation {
        let mut result = EntityValidation::valid();

        // Name validation
        if self.name.is_empty() {
            result.add_warning("Entity has empty name");
        }
        if self.name.len() > 256 {
            result.add_error("Entity name exceeds maximum length (256 characters)");
        }

        // Scale validation (must be positive, non-zero)
        if self.scale.x <= 0.0 || self.scale.y <= 0.0 || self.scale.z <= 0.0 {
            result.add_error(format!(
                "Invalid scale: ({:.2}, {:.2}, {:.2}) - all components must be > 0",
                self.scale.x, self.scale.y, self.scale.z
            ));
        }

        // Check for NaN/Inf in position
        if !self.position.is_finite() {
            result.add_error("Position contains NaN or Infinity values");
        }

        // Check for NaN/Inf in rotation
        if !self.rotation.is_finite() {
            result.add_error("Rotation contains NaN or Infinity values");
        }

        // Normalize check for rotation
        let rot_length = self.rotation.length();
        if (rot_length - 1.0).abs() > 0.01 {
            result.add_warning(format!(
                "Rotation quaternion not normalized (length: {:.4})",
                rot_length
            ));
        }

        result
    }

    /// Check if entity data is valid (no errors).
    pub fn is_valid(&self) -> bool {
        self.validate().is_valid
    }
}

/// Validation result for entity data.
#[derive(Debug, Clone)]
pub struct EntityValidation {
    /// Whether the entity passed all validation checks
    pub is_valid: bool,
    /// List of validation errors (fatal issues)
    pub errors: Vec<String>,
    /// List of validation warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl EntityValidation {
    /// Create a passing validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a warning without failing validation
    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    /// Add an error and mark as invalid
    pub fn add_error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
        self.is_valid = false;
    }

    /// Get total number of issues (errors + warnings)
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

/// Manages all entities in the editor scene
pub struct EntityManager {
    entities: HashMap<EntityId, EditorEntity>,
    next_id: EntityId,
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl EntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new entity with auto-generated ID
    pub fn create(&mut self, name: String) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let entity = EditorEntity::new(id, name);
        self.entities.insert(id, entity);
        id
    }

    /// Add an existing entity (used for deserialization)
    pub fn add(&mut self, entity: EditorEntity) {
        let id = entity.id;
        self.entities.insert(id, entity);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
    }

    /// Get entity by ID
    pub fn get(&self, id: EntityId) -> Option<&EditorEntity> {
        self.entities.get(&id)
    }

    /// Get mutable entity by ID
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut EditorEntity> {
        self.entities.get_mut(&id)
    }

    /// Remove entity by ID
    pub fn remove(&mut self, id: EntityId) -> Option<EditorEntity> {
        self.entities.remove(&id)
    }

    /// Get all entity IDs
    pub fn ids(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }

    /// Get all entities
    pub fn entities(&self) -> &HashMap<EntityId, EditorEntity> {
        &self.entities
    }

    /// Update entity transform
    pub fn update_transform(&mut self, id: EntityId, position: Vec3, rotation: Quat, scale: Vec3) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.set_transform(position, rotation, scale);
        }
    }

    /// Update entity position only
    pub fn update_position(&mut self, id: EntityId, position: Vec3) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.position = position;
        }
    }

    /// Update entity rotation only
    pub fn update_rotation(&mut self, id: EntityId, rotation: Quat) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.rotation = rotation;
        }
    }

    /// Update entity scale only
    pub fn update_scale(&mut self, id: EntityId, scale: Vec3) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.scale = scale;
        }
    }

    /// Clear all entities
    pub fn clear(&mut self) {
        self.entities.clear();
        self.next_id = 1;
    }

    /// Get entity count
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Remove all scatter entities (auto-generated from biome packs).
    /// Returns the IDs of removed entities for World cleanup.
    pub fn remove_scatter_entities(&mut self) -> Vec<EntityId> {
        let scatter_ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| e.is_scatter)
            .map(|(&id, _)| id)
            .collect();
        for id in &scatter_ids {
            self.entities.remove(id);
        }
        scatter_ids
    }

    /// Count of scatter entities
    pub fn scatter_count(&self) -> usize {
        self.entities.values().filter(|e| e.is_scatter).count()
    }

    /// Validate all entities and return a summary.
    pub fn validate_all(&self) -> EntityManagerValidation {
        let mut result = EntityManagerValidation {
            total_entities: self.entities.len(),
            ..Default::default()
        };

        for (id, entity) in &self.entities {
            let validation = entity.validate();
            if !validation.is_valid {
                result.invalid_entities.push(*id);
            }
            result.error_count += validation.errors.len();
            result.warning_count += validation.warnings.len();

            for error in validation.errors {
                result
                    .issues
                    .push(format!("Entity {} '{}': {}", id, entity.name, error));
            }
            for warning in validation.warnings {
                result.issues.push(format!(
                    "Entity {} '{}' [warn]: {}",
                    id, entity.name, warning
                ));
            }
        }

        result
    }

    /// Find entities matching a name pattern (case-insensitive).
    pub fn find_by_name(&self, pattern: &str) -> Vec<EntityId> {
        let pattern_lower = pattern.to_lowercase();
        self.entities
            .iter()
            .filter(|(_, e)| e.name.to_lowercase().contains(&pattern_lower))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get entities within a bounding box region.
    pub fn find_in_region(&self, min: Vec3, max: Vec3) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, e)| {
                e.position.x >= min.x
                    && e.position.x <= max.x
                    && e.position.y >= min.y
                    && e.position.y <= max.y
                    && e.position.z >= min.z
                    && e.position.z <= max.z
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get statistics about the entity manager state.
    pub fn stats(&self) -> EntityManagerStats {
        EntityManagerStats {
            total_entities: self.entities.len(),
            with_mesh: self.entities.values().filter(|e| e.mesh.is_some()).count(),
            with_textures: self
                .entities
                .values()
                .filter(|e| e.material.has_textures())
                .count(),
            with_components: self
                .entities
                .values()
                .filter(|e| !e.components.is_empty())
                .count(),
            next_id: self.next_id,
        }
    }
}

/// Validation summary for all entities.
#[derive(Debug, Clone, Default)]
pub struct EntityManagerValidation {
    /// Total entities checked
    pub total_entities: usize,
    /// Entity IDs that failed validation
    pub invalid_entities: Vec<EntityId>,
    /// Total error count
    pub error_count: usize,
    /// Total warning count  
    pub warning_count: usize,
    /// All issues as formatted strings
    pub issues: Vec<String>,
}

impl EntityManagerValidation {
    /// Check if all entities are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_entities.is_empty()
    }

    /// Get count of invalid entities
    pub fn invalid_count(&self) -> usize {
        self.invalid_entities.len()
    }

    /// Get count of valid entities
    pub fn valid_count(&self) -> usize {
        self.total_entities
            .saturating_sub(self.invalid_entities.len())
    }

    /// Get validation success rate as percentage
    pub fn success_rate(&self) -> f32 {
        if self.total_entities == 0 {
            100.0
        } else {
            (self.valid_count() as f32 / self.total_entities as f32) * 100.0
        }
    }

    /// Check if there are any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        self.error_count > 0 || self.warning_count > 0
    }

    /// Get total issue count
    pub fn total_issues(&self) -> usize {
        self.error_count + self.warning_count
    }
}

/// Statistics about the entity manager.
#[derive(Debug, Clone)]
pub struct EntityManagerStats {
    /// Total number of entities
    pub total_entities: usize,
    /// Entities with mesh assigned
    pub with_mesh: usize,
    /// Entities with textures assigned
    pub with_textures: usize,
    /// Entities with custom components
    pub with_components: usize,
    /// Next entity ID to be assigned
    pub next_id: EntityId,
}

impl EntityManagerStats {
    /// Check if any entities have meshes
    pub fn has_meshes(&self) -> bool {
        self.with_mesh > 0
    }

    /// Get percentage of entities with meshes
    pub fn mesh_percentage(&self) -> f32 {
        if self.total_entities == 0 {
            0.0
        } else {
            (self.with_mesh as f32 / self.total_entities as f32) * 100.0
        }
    }

    /// Get percentage of entities with textures
    pub fn texture_percentage(&self) -> f32 {
        if self.total_entities == 0 {
            0.0
        } else {
            (self.with_textures as f32 / self.total_entities as f32) * 100.0
        }
    }

    /// Get percentage of entities with custom components
    pub fn component_percentage(&self) -> f32 {
        if self.total_entities == 0 {
            0.0
        } else {
            (self.with_components as f32 / self.total_entities as f32) * 100.0
        }
    }

    /// Check if manager is empty
    pub fn is_empty(&self) -> bool {
        self.total_entities == 0
    }
}

/// Multi-selection state for the editor
///
/// Supports Ctrl+click (toggle), Shift+click (range), and maintains a primary selection
#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    /// All selected entity IDs
    pub entities: HashSet<EntityId>,

    /// Primary selection (last selected entity, used for gizmo placement)
    pub primary: Option<EntityId>,
}

impl SelectionSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add entity to selection
    ///
    /// # Arguments
    ///
    /// * `entity` - Entity ID to add
    /// * `is_primary` - Whether this becomes the primary selection
    pub fn add(&mut self, entity: EntityId, is_primary: bool) {
        self.entities.insert(entity);
        if is_primary {
            self.primary = Some(entity);
        }
    }

    /// Remove entity from selection
    pub fn remove(&mut self, entity: EntityId) {
        self.entities.remove(&entity);

        if self.primary == Some(entity) {
            self.primary = self.entities.iter().next().copied();
        }
    }

    /// Toggle entity selection (add if not selected, remove if selected)
    pub fn toggle(&mut self, entity: EntityId) {
        if self.entities.contains(&entity) {
            self.remove(entity);
        } else {
            self.add(entity, true);
        }
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    /// Select only this entity (clear others)
    pub fn select_only(&mut self, entity: EntityId) {
        self.clear();
        self.add(entity, true);
    }

    /// Check if entity is selected
    pub fn is_selected(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity)
    }

    /// Get selected entity count
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get all selected entity IDs as a Vec
    pub fn to_vec(&self) -> Vec<EntityId> {
        self.entities.iter().copied().collect()
    }

    /// Check if selection has multiple entities
    pub fn is_multi_select(&self) -> bool {
        self.entities.len() > 1
    }

    /// Check if entity is the primary selection
    pub fn is_primary(&self, entity: EntityId) -> bool {
        self.primary == Some(entity)
    }

    /// Get selection summary
    pub fn summary(&self) -> String {
        match self.count() {
            0 => "No selection".to_string(),
            1 => "1 entity selected".to_string(),
            n => format!("{} entities selected", n),
        }
    }
}

impl std::fmt::Display for SelectionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.count() {
            0 => write!(f, "(none)"),
            1 => write!(f, "1 selected"),
            n => write!(f, "{} selected", n),
        }
    }
}

impl EntityValidation {
    /// Check if there are only warnings (no errors)
    pub fn warnings_only(&self) -> bool {
        self.is_valid && !self.warnings.is_empty()
    }

    /// Get summary of validation
    pub fn summary(&self) -> String {
        if self.is_valid && self.warnings.is_empty() {
            "Valid".to_string()
        } else if self.is_valid {
            format!("{} warnings", self.warnings.len())
        } else {
            format!("{} errors", self.errors.len())
        }
    }
}

impl std::fmt::Display for EntityValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid {
            write!(f, "✓ Valid")
        } else {
            write!(f, "✗ {} errors", self.errors.len())
        }
    }
}

impl SelectionSet {
    /// Range selection between two entities (for Shift+click in hierarchy)
    ///
    /// # Arguments
    ///
    /// * `from` - Start entity ID
    /// * `to` - End entity ID
    /// * `all_ids` - Ordered list of all entity IDs (from hierarchy)
    pub fn select_range(&mut self, from: EntityId, to: EntityId, all_ids: &[EntityId]) {
        if let (Some(from_idx), Some(to_idx)) = (
            all_ids.iter().position(|&id| id == from),
            all_ids.iter().position(|&id| id == to),
        ) {
            let (start, end) = if from_idx < to_idx {
                (from_idx, to_idx)
            } else {
                (to_idx, from_idx)
            };

            for &id in &all_ids[start..=end] {
                self.entities.insert(id);
            }

            self.primary = Some(to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let mut manager = EntityManager::new();
        let id = manager.create("TestEntity".to_string());
        assert_eq!(id, 1);
        assert_eq!(manager.count(), 1);

        let entity = manager.get(id).unwrap();
        assert_eq!(entity.name, "TestEntity");
        assert_eq!(entity.position, Vec3::ZERO);
    }

    #[test]
    fn test_transform_update() {
        let mut manager = EntityManager::new();
        let id = manager.create("TestEntity".to_string());

        let new_pos = Vec3::new(1.0, 2.0, 3.0);
        manager.update_position(id, new_pos);

        let entity = manager.get(id).unwrap();
        assert_eq!(entity.position, new_pos);
    }

    #[test]
    fn test_aabb() {
        let mut manager = EntityManager::new();
        let id = manager.create("TestEntity".to_string());
        manager.update_scale(id, Vec3::new(2.0, 2.0, 2.0));

        let entity = manager.get(id).unwrap();
        let (min, max) = entity.aabb();
        assert_eq!(min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_selection_single() {
        let mut selection = SelectionSet::new();
        assert!(selection.is_empty());
        assert_eq!(selection.count(), 0);

        selection.add(1, true);
        assert!(!selection.is_empty());
        assert_eq!(selection.count(), 1);
        assert!(selection.is_selected(1));
        assert_eq!(selection.primary, Some(1));
    }

    #[test]
    fn test_selection_multiple() {
        let mut selection = SelectionSet::new();

        selection.add(1, true);
        selection.add(2, false);
        selection.add(3, false);

        assert_eq!(selection.count(), 3);
        assert!(selection.is_selected(1));
        assert!(selection.is_selected(2));
        assert!(selection.is_selected(3));
        assert_eq!(selection.primary, Some(1));
    }

    #[test]
    fn test_selection_toggle() {
        let mut selection = SelectionSet::new();

        selection.toggle(1);
        assert!(selection.is_selected(1));

        selection.toggle(1);
        assert!(!selection.is_selected(1));
        assert!(selection.is_empty());
    }

    #[test]
    fn test_selection_remove() {
        let mut selection = SelectionSet::new();
        selection.add(1, true);
        selection.add(2, false);
        selection.add(3, false);

        selection.remove(2);
        assert_eq!(selection.count(), 2);
        assert!(!selection.is_selected(2));
        assert_eq!(selection.primary, Some(1));

        selection.remove(1);
        assert_eq!(selection.count(), 1);
        assert!(selection.primary.is_some());
    }

    #[test]
    fn test_selection_clear() {
        let mut selection = SelectionSet::new();
        selection.add(1, true);
        selection.add(2, false);

        selection.clear();
        assert!(selection.is_empty());
        assert_eq!(selection.count(), 0);
        assert!(selection.primary.is_none());
    }

    #[test]
    fn test_selection_select_only() {
        let mut selection = SelectionSet::new();
        selection.add(1, true);
        selection.add(2, false);

        selection.select_only(3);
        assert_eq!(selection.count(), 1);
        assert!(selection.is_selected(3));
        assert!(!selection.is_selected(1));
        assert!(!selection.is_selected(2));
        assert_eq!(selection.primary, Some(3));
    }

    #[test]
    fn test_selection_range() {
        let mut selection = SelectionSet::new();
        let all_ids = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        selection.select_range(3, 7, &all_ids);

        assert_eq!(selection.count(), 5);
        assert!(selection.is_selected(3));
        assert!(selection.is_selected(4));
        assert!(selection.is_selected(5));
        assert!(selection.is_selected(6));
        assert!(selection.is_selected(7));
        assert!(!selection.is_selected(2));
        assert!(!selection.is_selected(8));
        assert_eq!(selection.primary, Some(7));
    }

    #[test]
    fn test_selection_range_reverse() {
        let mut selection = SelectionSet::new();
        let all_ids = vec![1, 2, 3, 4, 5];

        selection.select_range(5, 2, &all_ids);

        assert_eq!(selection.count(), 4);
        assert!(selection.is_selected(2));
        assert!(selection.is_selected(3));
        assert!(selection.is_selected(4));
        assert!(selection.is_selected(5));
        assert_eq!(selection.primary, Some(2));
    }

    #[test]
    fn test_material_texture_management() {
        let mut entity = EditorEntity::new(1, "Test".to_string());
        let path = PathBuf::from("path/to/texture.png");

        // Initial state
        assert!(!entity.material.has_textures());
        assert!(entity.material.get_texture(MaterialSlot::Albedo).is_none());

        // Set texture
        entity.set_texture(MaterialSlot::Albedo, path.clone());
        assert!(entity.material.has_textures());
        assert_eq!(
            entity.material.get_texture(MaterialSlot::Albedo),
            Some(&path)
        );

        // Clear texture
        entity.material.clear_texture(MaterialSlot::Albedo);
        assert!(!entity.material.has_textures());
        assert!(entity.material.get_texture(MaterialSlot::Albedo).is_none());
    }

    #[test]
    fn test_entity_transform_getters_setters() {
        let mut entity = EditorEntity::new(1, "Test".to_string());

        let pos = Vec3::new(1.0, 2.0, 3.0);
        let rot = Quat::IDENTITY;
        let scale = Vec3::splat(2.0);

        entity.set_transform(pos, rot, scale);

        let (p, r, s) = entity.transform();
        assert_eq!(p, pos);
        assert_eq!(r, rot);
        assert_eq!(s, scale);
    }

    #[test]
    fn test_entity_manager_crud() {
        let mut manager = EntityManager::new();

        // Create
        let id1 = manager.create("E1".to_string());
        let id2 = manager.create("E2".to_string());
        assert_ne!(id1, id2);

        // Read
        assert!(manager.get(id1).is_some());
        assert!(manager.get(id2).is_some());
        assert!(manager.get(999).is_none());

        // Update
        manager.update_position(id1, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(manager.get(id1).unwrap().position.x, 10.0);

        // Delete
        let removed = manager.remove(id1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id1);
        assert!(manager.get(id1).is_none());
        assert!(manager.remove(999).is_none());
    }

    #[test]
    fn test_entity_manager_clear_reset() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("E1".to_string());
        assert_eq!(id1, 1);

        manager.clear();
        assert_eq!(manager.count(), 0);
        assert!(manager.entities.is_empty());

        // Next ID should be reset to 1
        let id2 = manager.create("E2".to_string());
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_selection_range_invalid() {
        let mut selection = SelectionSet::new();
        let all_ids = vec![1, 2, 3];

        // Try to select range with invalid IDs
        selection.select_range(1, 99, &all_ids);
        assert!(selection.is_empty()); // Should do nothing

        selection.select_range(99, 1, &all_ids);
        assert!(selection.is_empty()); // Should do nothing
    }

    #[test]
    fn test_component_handling() {
        let mut entity = EditorEntity::new(1, "CompEntity".to_string());
        assert!(entity.components.is_empty());

        let comp_data = serde_json::json!({ "health": 100, "speed": 5.0 });
        entity.components.insert("Stats".to_string(), comp_data);

        assert_eq!(entity.components.len(), 1);
        assert!(entity.components.contains_key("Stats"));

        let stats = entity.components.get("Stats").unwrap();
        assert_eq!(stats["health"], 100);
    }

    // ====================================================================
    // EntityValidation Tests
    // ====================================================================

    #[test]
    fn test_entity_validation_valid() {
        let entity = EditorEntity::new(1, "ValidEntity".to_string());
        let validation = entity.validate();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert!(validation.warnings.is_empty());
        assert_eq!(validation.issue_count(), 0);
        assert!(entity.is_valid());
    }

    #[test]
    fn test_entity_validation_empty_name() {
        let entity = EditorEntity::new(1, "".to_string());
        let validation = entity.validate();
        assert!(validation.is_valid); // Empty name is a warning
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("empty name"));
    }

    #[test]
    fn test_entity_validation_invalid_scale() {
        let mut entity = EditorEntity::new(1, "BadScale".to_string());
        entity.scale = Vec3::new(0.0, 1.0, 1.0); // X is invalid

        let validation = entity.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
        assert!(validation.errors[0].contains("scale"));
        assert!(!entity.is_valid());
    }

    #[test]
    fn test_entity_validation_nan_position() {
        let mut entity = EditorEntity::new(1, "NaNPosition".to_string());
        entity.position = Vec3::new(f32::NAN, 0.0, 0.0);

        let validation = entity.validate();
        assert!(!validation.is_valid);
        assert!(validation
            .errors
            .iter()
            .any(|e| e.contains("Position") && e.contains("NaN")));
    }

    #[test]
    fn test_entity_validation_unnormalized_rotation() {
        let mut entity = EditorEntity::new(1, "BadRotation".to_string());
        // Create unnormalized quaternion (length != 1)
        entity.rotation = Quat::from_xyzw(1.0, 1.0, 1.0, 1.0); // Length = 2

        let validation = entity.validate();
        assert!(validation.is_valid); // Unnormalized is a warning
        assert!(validation
            .warnings
            .iter()
            .any(|w| w.contains("not normalized")));
    }

    // ====================================================================
    // EntityManagerValidation Tests
    // ====================================================================

    #[test]
    fn test_manager_validate_all_empty() {
        let manager = EntityManager::new();
        let validation = manager.validate_all();

        assert_eq!(validation.total_entities, 0);
        assert!(validation.all_valid());
        assert_eq!(validation.error_count, 0);
        assert_eq!(validation.warning_count, 0);
    }

    #[test]
    fn test_manager_validate_all_valid() {
        let mut manager = EntityManager::new();
        manager.create("E1".to_string());
        manager.create("E2".to_string());
        manager.create("E3".to_string());

        let validation = manager.validate_all();

        assert_eq!(validation.total_entities, 3);
        assert!(validation.all_valid());
        assert!(validation.invalid_entities.is_empty());
    }

    #[test]
    fn test_manager_validate_all_with_invalid() {
        let mut manager = EntityManager::new();
        let good_id = manager.create("GoodEntity".to_string());
        let bad_id = manager.create("BadEntity".to_string());

        // Make one entity invalid
        if let Some(bad_entity) = manager.get_mut(bad_id) {
            bad_entity.scale = Vec3::ZERO;
        }

        let validation = manager.validate_all();

        assert_eq!(validation.total_entities, 2);
        assert!(!validation.all_valid());
        assert_eq!(validation.invalid_entities.len(), 1);
        assert!(validation.invalid_entities.contains(&bad_id));
        assert!(!validation.invalid_entities.contains(&good_id));
    }

    // ====================================================================
    // find_by_name Tests
    // ====================================================================

    #[test]
    fn test_find_by_name_basic() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("Player".to_string());
        let _id2 = manager.create("Enemy".to_string());
        let _id3 = manager.create("Wall".to_string());

        let results = manager.find_by_name("Player");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&id1));
    }

    #[test]
    fn test_find_by_name_case_insensitive() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("PlayerCharacter".to_string());

        let results = manager.find_by_name("player");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&id1));

        let results = manager.find_by_name("PLAYER");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&id1));
    }

    #[test]
    fn test_find_by_name_partial_match() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("Enemy_Soldier".to_string());
        let id2 = manager.create("Enemy_Tank".to_string());
        let _id3 = manager.create("Player".to_string());

        let results = manager.find_by_name("Enemy");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&id1));
        assert!(results.contains(&id2));
    }

    #[test]
    fn test_find_by_name_no_match() {
        let mut manager = EntityManager::new();
        manager.create("Entity1".to_string());
        manager.create("Entity2".to_string());

        let results = manager.find_by_name("NotFound");
        assert!(results.is_empty());
    }

    // ====================================================================
    // find_in_region Tests
    // ====================================================================

    #[test]
    fn test_find_in_region_basic() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("InRegion".to_string());
        let id2 = manager.create("OutRegion".to_string());

        // Place id1 inside region
        manager.update_position(id1, Vec3::new(5.0, 5.0, 5.0));
        // Place id2 outside region
        manager.update_position(id2, Vec3::new(100.0, 100.0, 100.0));

        let results = manager.find_in_region(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 10.0));

        assert_eq!(results.len(), 1);
        assert!(results.contains(&id1));
    }

    #[test]
    fn test_find_in_region_boundary() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("OnBoundary".to_string());

        // Place exactly on boundary
        manager.update_position(id1, Vec3::new(10.0, 10.0, 10.0));

        let results = manager.find_in_region(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 10.0));

        assert_eq!(results.len(), 1); // Should include boundary
    }

    #[test]
    fn test_find_in_region_empty() {
        let mut manager = EntityManager::new();
        manager.create("Entity".to_string());
        // Entity is at origin by default

        let results = manager.find_in_region(
            Vec3::new(100.0, 100.0, 100.0),
            Vec3::new(200.0, 200.0, 200.0),
        );

        assert!(results.is_empty());
    }

    // ====================================================================
    // EntityManagerStats Tests
    // ====================================================================

    #[test]
    fn test_manager_stats_empty() {
        let manager = EntityManager::new();
        let stats = manager.stats();

        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.with_mesh, 0);
        assert_eq!(stats.with_textures, 0);
        assert_eq!(stats.with_components, 0);
        assert_eq!(stats.next_id, 1);
    }

    #[test]
    fn test_manager_stats_populated() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("WithMesh".to_string());
        let id2 = manager.create("WithTexture".to_string());
        let id3 = manager.create("WithComponent".to_string());
        let _id4 = manager.create("Plain".to_string());

        // Add mesh to id1
        if let Some(e) = manager.get_mut(id1) {
            e.mesh = Some("mesh.obj".to_string());
        }

        // Add texture to id2
        if let Some(e) = manager.get_mut(id2) {
            e.set_texture(MaterialSlot::Albedo, PathBuf::from("texture.png"));
        }

        // Add component to id3
        if let Some(e) = manager.get_mut(id3) {
            e.components
                .insert("Health".to_string(), serde_json::json!({"value": 100}));
        }

        let stats = manager.stats();

        assert_eq!(stats.total_entities, 4);
        assert_eq!(stats.with_mesh, 1);
        assert_eq!(stats.with_textures, 1);
        assert_eq!(stats.with_components, 1);
        assert_eq!(stats.next_id, 5);
    }

    // ====================================================================
    // EntityManagerStats New Methods Tests
    // ====================================================================

    #[test]
    fn test_manager_stats_has_meshes() {
        let mut manager = EntityManager::new();
        assert!(!manager.stats().has_meshes());

        let id = manager.create("WithMesh".to_string());
        if let Some(e) = manager.get_mut(id) {
            e.mesh = Some("test.obj".to_string());
        }

        assert!(manager.stats().has_meshes());
    }

    #[test]
    fn test_manager_stats_mesh_percentage() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("WithMesh".to_string());
        let _id2 = manager.create("NoMesh".to_string());

        if let Some(e) = manager.get_mut(id1) {
            e.mesh = Some("test.obj".to_string());
        }

        let stats = manager.stats();
        assert!((stats.mesh_percentage() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_manager_stats_texture_percentage() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("WithTex".to_string());
        let _id2 = manager.create("NoTex1".to_string());
        let _id3 = manager.create("NoTex2".to_string());

        if let Some(e) = manager.get_mut(id1) {
            e.set_texture(MaterialSlot::Albedo, PathBuf::from("tex.png"));
        }

        let stats = manager.stats();
        assert!((stats.texture_percentage() - 33.33).abs() < 1.0);
    }

    #[test]
    fn test_manager_stats_component_percentage() {
        let mut manager = EntityManager::new();
        let id1 = manager.create("WithComp".to_string());
        let id2 = manager.create("WithComp2".to_string());
        let _id3 = manager.create("NoComp".to_string());
        let _id4 = manager.create("NoComp2".to_string());

        if let Some(e) = manager.get_mut(id1) {
            e.components
                .insert("Test".to_string(), serde_json::json!(1));
        }
        if let Some(e) = manager.get_mut(id2) {
            e.components
                .insert("Test".to_string(), serde_json::json!(2));
        }

        let stats = manager.stats();
        assert!((stats.component_percentage() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_manager_stats_is_empty() {
        let manager = EntityManager::new();
        assert!(manager.stats().is_empty());

        let mut manager2 = EntityManager::new();
        manager2.create("Entity".to_string());
        assert!(!manager2.stats().is_empty());
    }

    #[test]
    fn test_manager_stats_zero_division() {
        let manager = EntityManager::new();
        let stats = manager.stats();

        assert_eq!(stats.mesh_percentage(), 0.0);
        assert_eq!(stats.texture_percentage(), 0.0);
        assert_eq!(stats.component_percentage(), 0.0);
    }

    // ====================================================================
    // EntityManagerValidation New Methods Tests
    // ====================================================================

    #[test]
    fn test_validation_invalid_count() {
        let mut manager = EntityManager::new();
        let _good_id = manager.create("Good".to_string());
        let bad_id = manager.create("Bad".to_string());

        if let Some(e) = manager.get_mut(bad_id) {
            e.scale = Vec3::ZERO;
        }

        let validation = manager.validate_all();
        assert_eq!(validation.invalid_count(), 1);
        assert_eq!(validation.valid_count(), 1);
    }

    #[test]
    fn test_validation_success_rate() {
        let mut manager = EntityManager::new();
        manager.create("Good1".to_string());
        manager.create("Good2".to_string());
        manager.create("Good3".to_string());
        let bad_id = manager.create("Bad".to_string());

        if let Some(e) = manager.get_mut(bad_id) {
            e.scale = Vec3::ZERO;
        }

        let validation = manager.validate_all();
        assert!((validation.success_rate() - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_validation_has_issues() {
        let mut manager = EntityManager::new();
        manager.create("Entity".to_string());

        let validation = manager.validate_all();
        assert!(!validation.has_issues());

        // With warning (empty name causes warning)
        let _id = manager.create("".to_string());
        let validation = manager.validate_all();
        assert!(validation.has_issues());
    }

    #[test]
    fn test_validation_total_issues() {
        let mut manager = EntityManager::new();
        let id = manager.create("Bad".to_string());

        if let Some(e) = manager.get_mut(id) {
            e.scale = Vec3::ZERO; // Error
            e.name = "".to_string(); // Warning
        }

        let validation = manager.validate_all();
        assert!(validation.total_issues() >= 2);
    }

    #[test]
    fn test_validation_empty_success_rate() {
        let manager = EntityManager::new();
        let validation = manager.validate_all();
        assert_eq!(validation.success_rate(), 100.0);
    }

    // ====================================================================
    // MaterialSlot New Methods Tests
    // ====================================================================

    #[test]
    fn test_material_slot_all() {
        let all = MaterialSlot::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_material_slot_name_not_empty() {
        for slot in MaterialSlot::all() {
            assert!(!slot.name().is_empty());
        }
    }

    #[test]
    fn test_material_slot_icon_not_empty() {
        for slot in MaterialSlot::all() {
            assert!(!slot.icon().is_empty());
        }
    }

    #[test]
    fn test_material_slot_is_color_slot() {
        assert!(MaterialSlot::Albedo.is_color_slot());
        assert!(MaterialSlot::Emission.is_color_slot());
        assert!(!MaterialSlot::Normal.is_color_slot());
        assert!(!MaterialSlot::Roughness.is_color_slot());
    }

    #[test]
    fn test_material_slot_is_data_slot() {
        assert!(!MaterialSlot::Albedo.is_data_slot());
        assert!(MaterialSlot::Normal.is_data_slot());
        assert!(MaterialSlot::Metallic.is_data_slot());
    }

    #[test]
    fn test_material_slot_display() {
        assert_eq!(format!("{}", MaterialSlot::Albedo), "Albedo");
        assert_eq!(format!("{}", MaterialSlot::Normal), "Normal");
    }

    // ====================================================================
    // EntityMaterial New Methods Tests
    // ====================================================================

    #[test]
    fn test_entity_material_texture_count() {
        let mut mat = EntityMaterial::new();
        assert_eq!(mat.texture_count(), 0);
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("albedo.png"));
        assert_eq!(mat.texture_count(), 1);
    }

    #[test]
    fn test_entity_material_is_metallic() {
        let mut mat = EntityMaterial::new();
        assert!(!mat.is_metallic());
        mat.metallic = 0.8;
        assert!(mat.is_metallic());
    }

    #[test]
    fn test_entity_material_is_rough() {
        let mat = EntityMaterial::new();
        assert!(!mat.is_rough()); // Default roughness is 0.5

        let mut rough_mat = EntityMaterial::new();
        rough_mat.roughness = 0.9;
        assert!(rough_mat.is_rough());
    }

    #[test]
    fn test_entity_material_is_emissive() {
        let mat = EntityMaterial::new();
        assert!(!mat.is_emissive());

        let mut emissive_mat = EntityMaterial::new();
        emissive_mat.emissive = Vec3::new(1.0, 0.5, 0.0);
        assert!(emissive_mat.is_emissive());
    }

    #[test]
    fn test_entity_material_summary() {
        let mat = EntityMaterial::new();
        let summary = mat.summary();
        assert!(summary.contains("Default"));
    }

    #[test]
    fn test_entity_material_display() {
        let mat = EntityMaterial::new();
        let display = format!("{}", mat);
        assert!(display.contains("Default"));
    }

    // ====================================================================
    // SelectionSet New Methods Tests
    // ====================================================================

    #[test]
    fn test_selection_set_is_multi_select() {
        let mut sel = SelectionSet::new();
        assert!(!sel.is_multi_select());
        sel.add(1, true);
        assert!(!sel.is_multi_select());
        sel.add(2, false);
        assert!(sel.is_multi_select());
    }

    #[test]
    fn test_selection_set_is_primary() {
        let mut sel = SelectionSet::new();
        sel.add(1, true);
        sel.add(2, false);
        assert!(sel.is_primary(1));
        assert!(!sel.is_primary(2));
    }

    #[test]
    fn test_selection_set_summary() {
        let mut sel = SelectionSet::new();
        assert_eq!(sel.summary(), "No selection");
        sel.add(1, true);
        assert!(sel.summary().contains("1"));
        sel.add(2, false);
        assert!(sel.summary().contains("2"));
    }

    #[test]
    fn test_selection_set_display() {
        let mut sel = SelectionSet::new();
        assert_eq!(format!("{}", sel), "(none)");
        sel.add(1, true);
        assert!(format!("{}", sel).contains("1"));
    }

    // ====================================================================
    // EntityValidation New Methods Tests
    // ====================================================================

    #[test]
    fn test_entity_validation_warnings_only() {
        let mut v = EntityValidation::valid();
        assert!(!v.warnings_only());
        v.add_warning("test");
        assert!(v.warnings_only());
    }

    #[test]
    fn test_entity_validation_summary() {
        let valid = EntityValidation::valid();
        assert_eq!(valid.summary(), "Valid");

        let mut with_warn = EntityValidation::valid();
        with_warn.add_warning("w");
        assert!(with_warn.summary().contains("warning"));
    }

    #[test]
    fn test_entity_validation_display() {
        let valid = EntityValidation::valid();
        assert!(format!("{}", valid).contains("Valid"));

        let mut invalid = EntityValidation::valid();
        invalid.add_error("e");
        assert!(format!("{}", invalid).contains("error"));
    }
}

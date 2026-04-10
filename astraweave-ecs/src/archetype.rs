//! AstraWeave ECS — Production-grade, AI-native ECS for game development.

use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use crate::blob_vec::BlobVec;
use crate::component_meta::ComponentMeta;
use crate::sparse_set::SparseSet;
use crate::{Component, Entity};

/// Unique identifier for an archetype (set of component types)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArchetypeId(u64);

/// Describes the component layout of an archetype
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeSignature {
    /// Sorted list of component TypeIds for deterministic comparison
    pub components: Vec<TypeId>,
}

impl ArchetypeSignature {
    pub fn new(mut components: Vec<TypeId>) -> Self {
        components.sort_unstable();
        components.dedup();
        Self { components }
    }

    pub fn contains(&self, ty: TypeId) -> bool {
        self.components.binary_search(&ty).is_ok()
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

/// Archetype storage: all entities with the same component signature
///
/// # Storage Modes
///
/// The archetype supports two storage modes for backward compatibility:
///
/// 1. **Box Mode** (legacy): Uses `Vec<Box<dyn Any>>` for each component column.
///    - Higher overhead due to heap indirection and virtual dispatch
///    - Used when component metadata is not available
///
/// 2. **BlobVec Mode** (optimized): Uses contiguous `BlobVec` for each column.
///    - Zero heap indirection, cache-friendly iteration
///    - Requires `ComponentMeta` to be provided at archetype creation
///
/// The `uses_blob` flag indicates which mode is active.
///
/// # Performance Note (Lazy Initialization)
///
/// BlobVec fields (`blob_components`, `component_metas`) use `Option` for lazy
/// initialization. This avoids HashMap allocation overhead in Box mode, which
/// is critical for spawn/despawn performance (previous regression: +388%).
pub struct Archetype {
    pub id: ArchetypeId,
    pub signature: ArchetypeSignature,

    /// Packed entity list for iteration (cache-friendly)
    entities: Vec<Entity>,

    /// O(1) entity lookup (replaces BTreeMap)
    entity_index: SparseSet,

    /// Legacy component columns: TypeId -> Vec<Box<dyn Any>>
    /// Used when ComponentMeta is not available
    components: HashMap<TypeId, Vec<Box<dyn std::any::Any + Send + Sync>>>,

    // ========================================================================
    // BlobVec Storage (High-Performance Path)
    // ========================================================================
    /// BlobVec component columns: TypeId -> BlobVec
    /// Used when ComponentMeta is available (typed path)
    /// NOTE: Lazy initialized (Option) to avoid allocation in Box mode
    blob_components: Option<HashMap<TypeId, BlobVec>>,

    /// Component metadata for each type (needed for clone/drop in transitions)
    /// NOTE: Lazy initialized (Option) to avoid allocation in Box mode
    component_metas: Option<HashMap<TypeId, ComponentMeta>>,

    /// Whether this archetype uses BlobVec storage (true) or Box storage (false)
    uses_blob: bool,

    // ========================================================================
    // Change Detection (P2-4)
    // ========================================================================
    /// Per-component-column change ticks, parallel to entity rows.
    /// `change_ticks[TypeId][row]` stores the World change_tick at which
    /// entity at `row` last had component `TypeId` mutated (insert / get_mut / each_mut).
    /// Works identically in both Box and BlobVec modes.
    change_ticks: HashMap<TypeId, Vec<u32>>,
}

impl Archetype {
    /// Create a new archetype with Box storage (legacy mode).
    ///
    /// BlobVec fields are NOT allocated (lazy initialization) to avoid
    /// spawn/despawn performance regression.
    pub fn new(id: ArchetypeId, signature: ArchetypeSignature) -> Self {
        let mut components = HashMap::new();
        let mut change_ticks = HashMap::new();
        for ty in &signature.components {
            components.insert(*ty, Vec::new());
            change_ticks.insert(*ty, Vec::new());
        }
        Self {
            id,
            signature,
            entities: Vec::new(),
            entity_index: SparseSet::new(),
            components,
            // BlobVec fields NOT allocated (lazy init for performance)
            blob_components: None,
            component_metas: None,
            uses_blob: false,
            change_ticks,
        }
    }

    /// Create a new archetype with BlobVec storage (high-performance mode).
    ///
    /// # Arguments
    /// * `id` - Unique archetype identifier
    /// * `signature` - Component type signature
    /// * `metas` - HashMap of TypeId -> ComponentMeta for each component
    pub fn new_with_blob(
        id: ArchetypeId,
        signature: ArchetypeSignature,
        metas: HashMap<TypeId, ComponentMeta>,
    ) -> Self {
        let mut blob_components = HashMap::new();
        let mut change_ticks = HashMap::new();
        for ty in &signature.components {
            if let Some(meta) = metas.get(ty) {
                blob_components.insert(*ty, meta.create_blob_vec());
            }
            change_ticks.insert(*ty, Vec::new());
        }

        Self {
            id,
            signature,
            entities: Vec::new(),
            entity_index: SparseSet::new(),
            components: HashMap::new(), // Not used in blob mode
            blob_components: Some(blob_components),
            component_metas: Some(metas),
            uses_blob: true,
            change_ticks,
        }
    }

    /// Check if this archetype uses BlobVec storage.
    #[inline]
    pub fn uses_blob(&self) -> bool {
        self.uses_blob
    }

    /// Add an entity with its components (must match signature)
    #[allow(clippy::expect_used)] // INVARIANT: signature components always have matching columns (created together)
    pub fn add_entity(
        &mut self,
        entity: Entity,
        component_data: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
    ) {
        self.add_entity_with_tick(entity, component_data, 0)
    }

    /// Add entity with components and stamp the given change tick on every column.
    #[allow(clippy::expect_used)]
    pub fn add_entity_with_tick(
        &mut self,
        entity: Entity,
        mut component_data: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
        tick: u32,
    ) {
        // Use SparseSet for O(1) lookup (12-57× faster than BTreeMap)
        self.entity_index.insert(entity);
        self.entities.push(entity);

        for ty in &self.signature.components {
            if let Some(data) = component_data.remove(ty) {
                // Move the Box from component_data into the column
                let column = self
                    .components
                    .get_mut(ty)
                    .expect("BUG: signature component should have column");
                column.push(data);
            }
            // Push change tick for this column (parallel to entity row)
            if let Some(ticks) = self.change_ticks.get_mut(ty) {
                ticks.push(tick);
            }
        }
    }

    // ========================================================================
    // Typed BlobVec Operations (High-Performance Path)
    // ========================================================================

    /// Add a single typed component to an entity already in this archetype.
    ///
    /// This is used for the high-performance typed path when transitioning
    /// entities between archetypes.
    ///
    /// # Safety
    /// - Entity must already be added to this archetype
    /// - Component type must be in the signature
    /// - Archetype must use BlobVec storage
    pub fn push_component_typed<T: Component>(&mut self, component: T) {
        self.push_component_typed_with_tick(component, 0)
    }

    /// Push a typed component with change tick stamping.
    pub fn push_component_typed_with_tick<T: Component>(&mut self, component: T, tick: u32) {
        debug_assert!(self.uses_blob, "push_component_typed requires BlobVec mode");
        let type_id = TypeId::of::<T>();

        if let Some(blob_components) = &mut self.blob_components {
            if let Some(blob) = blob_components.get_mut(&type_id) {
                // SAFETY: Caller ensures T matches the component type registered
                // for this column. BlobVec handles capacity internally.
                unsafe {
                    blob.push(component);
                }
            }
        }

        // Push change tick for this column
        if let Some(ticks) = self.change_ticks.get_mut(&type_id) {
            ticks.push(tick);
        }
    }

    /// Add entity with typed components using BlobVec storage.
    ///
    /// This bypasses Box<dyn Any> for maximum performance.
    ///
    /// # Arguments
    /// * `entity` - The entity to add
    /// * `components` - Raw component data as bytes with clone functions
    pub fn add_entity_typed_raw(&mut self, entity: Entity, components: &[(TypeId, *const u8)]) {
        self.add_entity_typed_raw_with_tick(entity, components, 0)
    }

    /// Add entity with raw typed components and stamp the given change tick.
    pub fn add_entity_typed_raw_with_tick(
        &mut self,
        entity: Entity,
        components: &[(TypeId, *const u8)],
        tick: u32,
    ) {
        debug_assert!(self.uses_blob, "add_entity_typed_raw requires BlobVec mode");

        self.entity_index.insert(entity);
        self.entities.push(entity);

        let blob_components = match &mut self.blob_components {
            Some(bc) => bc,
            None => return,
        };
        let component_metas = match &self.component_metas {
            Some(cm) => cm,
            None => return,
        };

        for (type_id, src_ptr) in components {
            if let (Some(blob), Some(meta)) = (
                blob_components.get_mut(type_id),
                component_metas.get(type_id),
            ) {
                // SAFETY: `src_ptr` points to valid component data (caller guarantee).
                // `meta.clone_fn` is the matching clone function from component registration.
                unsafe {
                    blob.push_raw(*src_ptr, meta.clone_fn);
                }
            }
            // Push change tick for this column
            if let Some(ticks) = self.change_ticks.get_mut(type_id) {
                ticks.push(tick);
            }
        }
    }

    /// Get component for entity (hybrid: checks BlobVec first, then Box)
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let row = self.entity_index.get(entity)?;

        // Fast path: BlobVec storage
        if self.uses_blob {
            let blob = self.blob_components.as_ref()?.get(&TypeId::of::<T>())?;
            // SAFETY: T matches the type this column was created for (same TypeId lookup).
            // `row` is a valid index from entity_index.
            return unsafe { blob.get::<T>(row) };
        }

        // Legacy path: Box storage
        let column = self.components.get(&TypeId::of::<T>())?;
        let boxed = column.get(row)?;
        boxed.downcast_ref::<T>()
    }

    /// Get mutable component for entity (hybrid: checks BlobVec first, then Box)
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let row = self.entity_index.get(entity)?;

        // Fast path: BlobVec storage
        if self.uses_blob {
            let blob = self.blob_components.as_mut()?.get_mut(&TypeId::of::<T>())?;
            // SAFETY: T matches the type this column was created for (same TypeId lookup).
            // `row` is a valid index from entity_index. Mutable borrow of self prevents aliasing.
            return unsafe { blob.get_mut::<T>(row) };
        }

        // Legacy path: Box storage
        let column = self.components.get_mut(&TypeId::of::<T>())?;
        let boxed = column.get_mut(row)?;
        boxed.downcast_mut::<T>()
    }

    /// Stamp the change tick for a specific component column at the entity's row.
    ///
    /// Called by `World::get_mut` and `World::each_mut` to record mutations.
    #[inline]
    pub fn stamp_change_tick<T: Component>(&mut self, entity: Entity, tick: u32) {
        if let Some(row) = self.entity_index.get(entity) {
            if let Some(ticks) = self.change_ticks.get_mut(&TypeId::of::<T>()) {
                if let Some(t) = ticks.get_mut(row) {
                    *t = tick;
                }
            }
        }
    }

    /// Stamp the change tick for a specific component column at the entity's row (by TypeId).
    #[inline]
    pub fn stamp_change_tick_by_type(&mut self, entity: Entity, type_id: TypeId, tick: u32) {
        if let Some(row) = self.entity_index.get(entity) {
            if let Some(ticks) = self.change_ticks.get_mut(&type_id) {
                if let Some(t) = ticks.get_mut(row) {
                    *t = tick;
                }
            }
        }
    }

    /// Get the change tick for a specific component on an entity.
    ///
    /// Returns `None` if entity not in archetype or component not tracked.
    #[inline]
    pub fn get_change_tick<T: Component>(&self, entity: Entity) -> Option<u32> {
        let row = self.entity_index.get(entity)?;
        self.change_ticks
            .get(&TypeId::of::<T>())
            .and_then(|ticks| ticks.get(row).copied())
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Option<usize> {
        // NEW: O(1) removal with SparseSet (4-7× faster than BTreeMap)
        self.entity_index.remove(entity)
    }

    /// Remove entity from archetype and return its components
    pub fn remove_entity_components(
        &mut self,
        entity: Entity,
    ) -> HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>> {
        // NEW: O(1) removal with SparseSet
        let row = match self.entity_index.remove(entity) {
            Some(r) => r,
            None => return HashMap::new(),
        };

        // Remove from packed entity list using swap_remove
        let entities_len = self.entities.len();
        if row < entities_len - 1 {
            self.entities.swap(row, entities_len - 1);
            // Update the swapped entity's index in SparseSet
            let swapped_entity = self.entities[row];
            self.entity_index.insert(swapped_entity); // Will update to correct row
        }
        self.entities.pop();

        let mut components = HashMap::new();
        for (ty, column) in self.components.iter_mut() {
            let component = column.swap_remove(row);
            components.insert(*ty, component);
        }

        // Keep change_ticks parallel: swap_remove from each tick column
        for ticks in self.change_ticks.values_mut() {
            if row < ticks.len() {
                ticks.swap_remove(row);
            }
        }

        components
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get a slice of entities in this archetype (zero-cost, cache-friendly!)
    pub fn entities_vec(&self) -> &[Entity] {
        &self.entities
    }

    /// Iterate over (entity, component) pairs for batch processing.
    ///
    /// This is much faster than repeated get() calls as it avoids per-entity lookups.
    ///
    /// ## Performance Notes (Week 10)
    ///
    /// With SparseSet integration, get() is now O(1) instead of O(log n), providing
    /// 12-57× speedup over the old BTreeMap approach. This iterator provides additional
    /// benefits by reducing function call overhead and improving cache locality.
    ///
    /// ## Mutable Iterator Limitation
    ///
    /// Note: A mutable version (`iter_components_mut<T>()`) is **not feasible** due to
    /// Rust's borrow checker limitations. The issue is:
    ///
    /// ```rust,ignore
    /// pub fn iter_components_mut<T>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
    ///     let column = self.components.get_mut(&TypeId::of::<T>())?;
    ///     self.entities.iter().filter_map(|(idx, &entity)| {
    ///         column.get_mut(idx)  // ❌ ERROR: captured variable escapes FnMut closure
    ///     })
    /// }
    /// ```
    ///
    /// Rust prevents this because the closure captures `column` and tries to return
    /// `&mut T` borrowed from it. The borrow checker rule is: **references captured in
    /// closures cannot escape the closure scope**. This prevents dangling references.
    ///
    /// **Workarounds considered**:
    /// - Unsafe raw pointers: Would work but loses safety guarantees (not worth it)
    /// - Index-based batch API: Complex redesign with uncertain performance gains
    /// - Type registry + BlobVec: Full solution but requires architectural changes (Week 13+)
    ///
    /// **Current approach**: Accept that SparseSet O(1) already provides 2.4× frame time
    /// improvement (2.70ms → 1.144ms) and 9.4× faster movement (1,000µs → 106µs). Further
    /// query optimization has diminishing returns vs complexity/safety trade-offs.
    pub fn iter_components<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> + '_ {
        // NOTE: We can't easily branch on uses_blob here because the return types differ.
        // The BlobVec path would return a different iterator type.
        // For now, always use the Box path in iter_components.
        // The typed fast path is available via iter_components_blob().
        let column = self.components.get(&TypeId::of::<T>());
        self.entities
            .iter()
            .enumerate()
            .filter_map(move |(idx, &entity)| {
                column
                    .and_then(|col| col.get(idx))
                    .and_then(|boxed| boxed.downcast_ref::<T>())
                    .map(|component| (entity, component))
            })
    }

    /// High-performance iteration using BlobVec storage.
    ///
    /// Returns a slice of components for direct SIMD-friendly iteration.
    /// Only works for archetypes using BlobVec mode.
    ///
    /// # Returns
    /// - `Some((entities, components))` if this archetype uses BlobVec and has the component
    /// - `None` if using Box mode or component not present
    ///
    /// # Safety
    /// The returned slice is only valid while the archetype is not modified.
    pub fn iter_components_blob<T: Component>(&self) -> Option<(&[Entity], &[T])> {
        if !self.uses_blob {
            return None;
        }

        let blob = self.blob_components.as_ref()?.get(&TypeId::of::<T>())?;
        // SAFETY: T matches the column type (same TypeId). Immutable borrow of self
        // ensures no concurrent modification. Slice is valid for the lifetime of &self.
        let components = unsafe { blob.as_slice::<T>() };
        Some((&self.entities, components))
    }

    /// High-performance mutable iteration using BlobVec storage.
    ///
    /// Returns a mutable slice of components for direct modification.
    /// Only works for archetypes using BlobVec mode.
    ///
    /// # Safety
    /// The returned slice is only valid while the archetype is not modified.
    pub fn iter_components_blob_mut<T: Component>(&mut self) -> Option<(&[Entity], &mut [T])> {
        if !self.uses_blob {
            return None;
        }

        let blob = self.blob_components.as_mut()?.get_mut(&TypeId::of::<T>())?;
        // SAFETY: T matches the column type (same TypeId). Mutable borrow of self
        // prevents aliasing. Slice is valid for the lifetime of &mut self.
        let components = unsafe { blob.as_slice_mut::<T>() };
        Some((&self.entities, components))
    }
}

/// Manages all archetypes and entity->archetype mapping
///
/// # Determinism Guarantee
///
/// **CRITICAL**: This uses `BTreeMap` for archetype storage to ensure deterministic iteration.
/// Iteration order is sorted by `ArchetypeId`, which preserves archetype creation order
/// (IDs assigned sequentially via `next_id`).
///
/// **Why BTreeMap?**
/// - HashMap iteration order is **non-deterministic** (depends on hash function, memory layout)
/// - BTreeMap iteration order is **deterministic** (sorted by key)
/// - For AI agents, deterministic entity iteration is **critical** for reproducible behavior
///
/// **Performance Note**:
/// - BTreeMap operations are O(log n) vs HashMap O(1)
/// - With ~100 archetypes typical, log₂(100) ≈ 7 operations (negligible)
/// - Entity queries iterate archetypes (O(archetypes)), so iteration order matters more than lookup
///
/// # Zero-Allocation Hot Path
///
/// **CRITICAL**: Entity-to-archetype mapping uses `Vec<Option<ArchetypeId>>` indexed by entity ID
/// instead of HashMap to ensure zero heap allocations during component access hot paths.
/// HashMap uses RandomState hasher which can allocate thread-local state on first access.
#[derive(Default)]
pub struct ArchetypeStorage {
    next_id: u64,
    /// Map from signature to archetype ID
    signature_to_id: HashMap<ArchetypeSignature, ArchetypeId>,
    /// All archetypes (BTreeMap for deterministic iteration by ID)
    archetypes: BTreeMap<ArchetypeId, Archetype>,
    /// Entity to archetype mapping (sparse array indexed by entity ID for zero-alloc lookup)
    /// Uses Vec<Option<ArchetypeId>> instead of HashMap for zero-alloc hot path.
    entity_to_archetype: Vec<Option<ArchetypeId>>,
    /// Inverted index: component TypeId → Vec of ArchetypeIds containing that component.
    /// Turns O(n_archetypes) linear scan into O(n_matching) lookup for query resolution.
    component_to_archetypes: HashMap<TypeId, Vec<ArchetypeId>>,
}

impl ArchetypeStorage {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            signature_to_id: HashMap::new(),
            archetypes: BTreeMap::new(),
            entity_to_archetype: Vec::new(),
            component_to_archetypes: HashMap::new(),
        }
    }

    /// Get or create archetype for a signature
    pub fn get_or_create_archetype(&mut self, signature: ArchetypeSignature) -> ArchetypeId {
        if let Some(&id) = self.signature_to_id.get(&signature) {
            return id;
        }

        let id = ArchetypeId(self.next_id);
        self.next_id += 1;

        // Update inverted index: register this archetype for each component type
        for &ty in &signature.components {
            self.component_to_archetypes.entry(ty).or_default().push(id);
        }

        let archetype = Archetype::new(id, signature.clone());
        self.archetypes.insert(id, archetype);
        self.signature_to_id.insert(signature, id);

        id
    }

    /// Get or create archetype with BlobVec storage (high-performance mode).
    ///
    /// Unlike `get_or_create_archetype`, this creates archetypes with contiguous
    /// BlobVec storage for faster iteration.
    ///
    /// # Arguments
    /// * `signature` - Component type signature
    /// * `metas` - Component metadata for each type (must include all signature types)
    pub fn get_or_create_archetype_with_blob(
        &mut self,
        signature: ArchetypeSignature,
        metas: HashMap<TypeId, ComponentMeta>,
    ) -> ArchetypeId {
        if let Some(&id) = self.signature_to_id.get(&signature) {
            // If archetype already exists but uses Box mode, we can't upgrade it
            // (would require migrating all existing entities)
            return id;
        }

        let id = ArchetypeId(self.next_id);
        self.next_id += 1;

        // Update inverted index: register this archetype for each component type
        for &ty in &signature.components {
            self.component_to_archetypes.entry(ty).or_default().push(id);
        }

        let archetype = Archetype::new_with_blob(id, signature.clone(), metas);
        self.archetypes.insert(id, archetype);
        self.signature_to_id.insert(signature, id);

        id
    }

    pub fn get_archetype(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_archetype_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(&id)
    }

    /// Get archetype for an entity (zero-alloc O(1) lookup)
    #[inline]
    pub fn get_entity_archetype(&self, entity: Entity) -> Option<ArchetypeId> {
        let id = entity.id() as usize;
        self.entity_to_archetype.get(id).copied().flatten()
    }

    /// Set archetype for an entity (may allocate if entity ID is larger than current capacity)
    pub fn set_entity_archetype(&mut self, entity: Entity, archetype: ArchetypeId) {
        let id = entity.id() as usize;
        // Grow the sparse array if needed (only allocates during warmup/setup)
        if id >= self.entity_to_archetype.len() {
            self.entity_to_archetype.resize(id + 1, None);
        }
        self.entity_to_archetype[id] = Some(archetype);
    }

    /// Remove archetype mapping for an entity (zero-alloc)
    #[inline]
    pub fn remove_entity(&mut self, entity: Entity) -> Option<ArchetypeId> {
        let id = entity.id() as usize;
        if id < self.entity_to_archetype.len() {
            self.entity_to_archetype[id].take()
        } else {
            None
        }
    }

    /// Iterate over all archetypes
    pub fn archetypes(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.values()
    }

    /// Iterate over all archetypes (alias for consistency with standard iterator naming)
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.values()
    }

    /// Iterate mutably over all archetypes
    pub fn archetypes_mut(&mut self) -> impl Iterator<Item = &mut Archetype> {
        self.archetypes.values_mut()
    }

    /// Find archetypes that contain a specific component.
    ///
    /// Uses an inverted index (`component_to_archetypes`) for O(1) lookup
    /// instead of scanning every archetype.
    pub fn archetypes_with_component(&self, ty: TypeId) -> impl Iterator<Item = &Archetype> {
        #[cfg(feature = "profiling")]
        span!("ECS::Archetype::archetypes_with_component");

        // Empty slice fallback avoids an Option branch in the iterator chain.
        let ids = self
            .component_to_archetypes
            .get(&ty)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        ids.iter().filter_map(move |id| self.archetypes.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Health(i32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position(f32, f32);

    #[test]
    fn test_signature_creation() {
        let sig1 = ArchetypeSignature::new(vec![TypeId::of::<Health>(), TypeId::of::<Position>()]);
        let sig2 = ArchetypeSignature::new(vec![TypeId::of::<Position>(), TypeId::of::<Health>()]);
        assert_eq!(sig1, sig2); // Order-independent
    }

    #[test]
    fn test_archetype_storage() {
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);

        let id1 = storage.get_or_create_archetype(sig.clone());
        let id2 = storage.get_or_create_archetype(sig.clone());

        assert_eq!(id1, id2); // Same signature returns same archetype
    }

    // ====================
    // Day 3: Surgical Coverage Improvements - archetype.rs
    // ====================

    #[test]
    fn test_signature_methods() {
        // Tests contains(), len(), is_empty()
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>(), TypeId::of::<Position>()]);

        assert!(sig.contains(TypeId::of::<Health>()));
        assert!(sig.contains(TypeId::of::<Position>()));
        assert!(!sig.contains(TypeId::of::<i32>()));

        assert_eq!(sig.len(), 2);
        assert!(!sig.is_empty());

        let empty_sig = ArchetypeSignature::new(vec![]);
        assert_eq!(empty_sig.len(), 0);
        assert!(empty_sig.is_empty());
    }

    #[test]
    fn test_archetype_entity_operations() {
        // Tests add_entity, get, get_mut, len, is_empty, entities_vec
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>(), TypeId::of::<Position>()]);
        let mut archetype = Archetype::new(ArchetypeId(0), sig);

        assert_eq!(archetype.len(), 0);
        assert!(archetype.is_empty());
        assert_eq!(archetype.entities_vec().len(), 0);

        // Add entity with components
        let entity = unsafe { Entity::from_raw(1) };
        let mut components = HashMap::new();
        components.insert(
            TypeId::of::<Health>(),
            Box::new(Health(100)) as Box<dyn std::any::Any + Send + Sync>,
        );
        components.insert(
            TypeId::of::<Position>(),
            Box::new(Position(1.0, 2.0)) as Box<dyn std::any::Any + Send + Sync>,
        );

        archetype.add_entity(entity, components);

        assert_eq!(archetype.len(), 1);
        assert!(!archetype.is_empty());
        assert_eq!(archetype.entities_vec().len(), 1);
        assert_eq!(archetype.entities_vec()[0], entity);

        // Test get
        let health = archetype.get::<Health>(entity).unwrap();
        assert_eq!(health.0, 100);

        let pos = archetype.get::<Position>(entity).unwrap();
        assert_eq!(pos.0, 1.0);
        assert_eq!(pos.1, 2.0);

        // Test get_mut
        {
            let health_mut = archetype.get_mut::<Health>(entity).unwrap();
            health_mut.0 = 50;
        }

        let health = archetype.get::<Health>(entity).unwrap();
        assert_eq!(health.0, 50);
    }

    #[test]
    fn test_archetype_remove_entity() {
        // Tests remove_entity and remove_entity_components
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let mut archetype = Archetype::new(ArchetypeId(0), sig);

        let entity1 = unsafe { Entity::from_raw(1) };
        let entity2 = unsafe { Entity::from_raw(2) };

        let mut components1 = HashMap::new();
        components1.insert(
            TypeId::of::<Health>(),
            Box::new(Health(100)) as Box<dyn std::any::Any + Send + Sync>,
        );
        archetype.add_entity(entity1, components1);

        let mut components2 = HashMap::new();
        components2.insert(
            TypeId::of::<Health>(),
            Box::new(Health(200)) as Box<dyn std::any::Any + Send + Sync>,
        );
        archetype.add_entity(entity2, components2);

        assert_eq!(archetype.len(), 2);

        // Remove entity1
        let removed_components = archetype.remove_entity_components(entity1);
        assert_eq!(archetype.len(), 1);
        assert!(removed_components.contains_key(&TypeId::of::<Health>()));

        // entity2 should still be accessible
        let health = archetype.get::<Health>(entity2).unwrap();
        assert_eq!(health.0, 200);

        // entity1 should be gone
        assert!(archetype.get::<Health>(entity1).is_none());
    }

    #[test]
    fn test_archetype_iter_components() {
        // Tests iter_components batch iterator
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let mut archetype = Archetype::new(ArchetypeId(0), sig);

        let entity1 = unsafe { Entity::from_raw(1) };
        let entity2 = unsafe { Entity::from_raw(2) };
        let entity3 = unsafe { Entity::from_raw(3) };

        let mut components1 = HashMap::new();
        components1.insert(
            TypeId::of::<Health>(),
            Box::new(Health(100)) as Box<dyn std::any::Any + Send + Sync>,
        );
        archetype.add_entity(entity1, components1);

        let mut components2 = HashMap::new();
        components2.insert(
            TypeId::of::<Health>(),
            Box::new(Health(200)) as Box<dyn std::any::Any + Send + Sync>,
        );
        archetype.add_entity(entity2, components2);

        let mut components3 = HashMap::new();
        components3.insert(
            TypeId::of::<Health>(),
            Box::new(Health(300)) as Box<dyn std::any::Any + Send + Sync>,
        );
        archetype.add_entity(entity3, components3);

        // Collect all health values via iterator
        let healths: Vec<i32> = archetype
            .iter_components::<Health>()
            .map(|(_, health)| health.0)
            .collect();

        assert_eq!(healths.len(), 3);
        assert!(healths.contains(&100));
        assert!(healths.contains(&200));
        assert!(healths.contains(&300));
    }

    #[test]
    fn test_archetype_storage_comprehensive() {
        // Tests get_archetype, get_entity_archetype, set_entity_archetype, remove_entity,
        // archetypes(), iter(), archetypes_mut(), archetypes_with_component()
        let mut storage = ArchetypeStorage::new();

        let sig1 = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let sig2 = ArchetypeSignature::new(vec![TypeId::of::<Position>()]);
        let sig3 = ArchetypeSignature::new(vec![TypeId::of::<Health>(), TypeId::of::<Position>()]);

        let id1 = storage.get_or_create_archetype(sig1);
        let id2 = storage.get_or_create_archetype(sig2);
        let id3 = storage.get_or_create_archetype(sig3);

        // Test get_archetype
        assert!(storage.get_archetype(id1).is_some());
        assert!(storage.get_archetype(id2).is_some());
        assert!(storage.get_archetype(id3).is_some());

        // Test entity->archetype mapping
        let entity = unsafe { Entity::from_raw(42) };
        assert!(storage.get_entity_archetype(entity).is_none());

        storage.set_entity_archetype(entity, id1);
        assert_eq!(storage.get_entity_archetype(entity), Some(id1));

        // Test remove_entity
        let removed = storage.remove_entity(entity);
        assert_eq!(removed, Some(id1));
        assert!(storage.get_entity_archetype(entity).is_none());

        // Test archetypes() iterator
        let count = storage.archetypes().count();
        assert_eq!(count, 3);

        // Test iter() (alias)
        let count2 = storage.iter().count();
        assert_eq!(count2, 3);

        // Test archetypes_mut()
        let mut_count = storage.archetypes_mut().count();
        assert_eq!(mut_count, 3);

        // Test archetypes_with_component
        let with_health = storage
            .archetypes_with_component(TypeId::of::<Health>())
            .count();
        assert_eq!(with_health, 2); // sig1 and sig3 have Health

        let with_position = storage
            .archetypes_with_component(TypeId::of::<Position>())
            .count();
        assert_eq!(with_position, 2); // sig2 and sig3 have Position

        let with_nothing = storage
            .archetypes_with_component(TypeId::of::<i32>())
            .count();
        assert_eq!(with_nothing, 0);
    }

    // ===========================================================================
    // Mutation-Resistant Remediation Tests: Blob Iteration & Storage Boundaries
    // ===========================================================================
    // Targets 9 missed mutants in archetype.rs shard 1/6:
    //   iter_components_blob → None / Some((empty,empty)) / delete !
    //   iter_components_blob_mut → None / Some((empty,empty)) / delete !
    //   get_or_create_archetype_with_blob: += → -=/*=
    //   remove_entity: < → <=

    #[test]
    fn test_iter_components_blob_returns_correct_data() {
        // Kills: iter_components_blob → None (would fail unwrap)
        // Kills: iter_components_blob → Some(([], [])) (wrong length)
        // Kills: delete `!` in `!self.uses_blob` (would skip blob archetypes)
        use crate::component_meta::ComponentMeta;

        let mut metas = HashMap::new();
        metas.insert(TypeId::of::<Health>(), ComponentMeta::of::<Health>());

        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let mut archetype = Archetype::new_with_blob(ArchetypeId(0), sig, metas);

        // Add entities using typed blob path
        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(2) };
        let e3 = unsafe { Entity::from_raw(3) };

        archetype.entity_index.insert(e1);
        archetype.entities.push(e1);
        archetype.push_component_typed(Health(100));

        archetype.entity_index.insert(e2);
        archetype.entities.push(e2);
        archetype.push_component_typed(Health(200));

        archetype.entity_index.insert(e3);
        archetype.entities.push(e3);
        archetype.push_component_typed(Health(300));

        // Verify iter_components_blob returns correct entities and values
        let (entities, healths) = archetype
            .iter_components_blob::<Health>()
            .expect("blob archetype must return Some for registered component");

        assert_eq!(entities.len(), 3, "must return all 3 entities");
        assert_eq!(healths.len(), 3, "must return all 3 component values");

        assert_eq!(entities[0], e1);
        assert_eq!(entities[1], e2);
        assert_eq!(entities[2], e3);

        assert_eq!(healths[0], Health(100));
        assert_eq!(healths[1], Health(200));
        assert_eq!(healths[2], Health(300));

        // Non-blob archetype returns None
        let box_sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let box_archetype = Archetype::new(ArchetypeId(1), box_sig);
        assert!(
            box_archetype.iter_components_blob::<Health>().is_none(),
            "non-blob archetype must return None"
        );

        // Wrong component type returns None
        assert!(
            archetype.iter_components_blob::<Position>().is_none(),
            "unregistered component type must return None"
        );
    }

    #[test]
    fn test_iter_components_blob_mut_returns_correct_data_and_allows_mutation() {
        // Kills: iter_components_blob_mut → None / Some(([], []))  / delete !
        use crate::component_meta::ComponentMeta;

        let mut metas = HashMap::new();
        metas.insert(TypeId::of::<Health>(), ComponentMeta::of::<Health>());

        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let mut archetype = Archetype::new_with_blob(ArchetypeId(0), sig, metas);

        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(2) };

        archetype.entity_index.insert(e1);
        archetype.entities.push(e1);
        archetype.push_component_typed(Health(50));

        archetype.entity_index.insert(e2);
        archetype.entities.push(e2);
        archetype.push_component_typed(Health(75));

        // Mutate through blob_mut
        {
            let (entities, healths) = archetype
                .iter_components_blob_mut::<Health>()
                .expect("blob archetype must return Some");

            assert_eq!(entities.len(), 2);
            assert_eq!(healths.len(), 2);
            assert_eq!(healths[0], Health(50));
            assert_eq!(healths[1], Health(75));

            // Mutate
            healths[0] = Health(999);
            healths[1] = Health(888);
        }

        // Verify mutations persisted via immutable path
        let (_, healths_after) = archetype.iter_components_blob::<Health>().unwrap();
        assert_eq!(healths_after[0], Health(999));
        assert_eq!(healths_after[1], Health(888));
    }

    #[test]
    fn test_get_or_create_archetype_with_blob_increments_id() {
        // Kills: next_id += 1 → next_id -= 1 / next_id *= 1
        // Verifies that each new blob archetype gets a strictly increasing ID
        use crate::component_meta::ComponentMeta;

        let mut storage = ArchetypeStorage::new();

        let sig1 = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let mut metas1 = HashMap::new();
        metas1.insert(TypeId::of::<Health>(), ComponentMeta::of::<Health>());

        let sig2 = ArchetypeSignature::new(vec![TypeId::of::<Position>()]);
        let mut metas2 = HashMap::new();
        metas2.insert(TypeId::of::<Position>(), ComponentMeta::of::<Position>());

        let sig3 = ArchetypeSignature::new(vec![TypeId::of::<Health>(), TypeId::of::<Position>()]);
        let mut metas3 = HashMap::new();
        metas3.insert(TypeId::of::<Health>(), ComponentMeta::of::<Health>());
        metas3.insert(TypeId::of::<Position>(), ComponentMeta::of::<Position>());

        let id1 = storage.get_or_create_archetype_with_blob(sig1.clone(), metas1);
        let id2 = storage.get_or_create_archetype_with_blob(sig2.clone(), metas2);
        let id3 = storage.get_or_create_archetype_with_blob(sig3.clone(), metas3);

        // IDs must be strictly increasing
        assert!(id2.0 > id1.0, "id2 ({}) must be > id1 ({})", id2.0, id1.0);
        assert!(id3.0 > id2.0, "id3 ({}) must be > id2 ({})", id3.0, id2.0);
        assert_eq!(id1.0 + 1, id2.0, "IDs should be consecutive");
        assert_eq!(id2.0 + 1, id3.0, "IDs should be consecutive");

        // Re-requesting same signature returns same ID (no increment)
        let metas1_dup = {
            let mut m = HashMap::new();
            m.insert(TypeId::of::<Health>(), ComponentMeta::of::<Health>());
            m
        };
        let id1_dup = storage.get_or_create_archetype_with_blob(sig1, metas1_dup);
        assert_eq!(id1, id1_dup, "duplicate signature must return same ID");

        // Archetype is actually usable for blob operations
        let arch = storage.get_archetype(id1).unwrap();
        assert!(arch.uses_blob(), "blob archetype must use blob mode");
    }

    #[test]
    fn test_remove_entity_boundary_at_len() {
        // Kills: remove_entity < → <= (entity at exactly vec.len()-1 must work)
        // With `<=`, entity.id == vec.len() would enter the branch and OOB.
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Health>()]);
        let arch_id = storage.get_or_create_archetype(sig);

        // Entity with id=0 (so index = 0)
        let e0 = unsafe { Entity::from_raw(0) };
        storage.set_entity_archetype(e0, arch_id);

        // Entity with id=1 (so index = 1, which is entity_to_archetype.len()-1 after resize)
        let e1 = unsafe { Entity::from_raw(1) };
        storage.set_entity_archetype(e1, arch_id);

        // Entity with id exactly at len boundary — the vec length should be 2
        // Removing entity 1 (id=1, len is 2, 1 < 2 is true → enters branch)
        let result = storage.remove_entity(e1);
        assert_eq!(result, Some(arch_id), "entity at len-1 must be removable");

        // Entity just beyond the vec length — should return None cleanly
        let e_beyond = unsafe { Entity::from_raw(2) };
        let result_beyond = storage.remove_entity(e_beyond);
        assert_eq!(result_beyond, None, "entity beyond vec length must be None");

        // Verify e0 is still tracked
        assert_eq!(storage.get_entity_archetype(e0), Some(arch_id));
        // Verify e1 is gone
        assert_eq!(storage.get_entity_archetype(e1), None);
    }
}

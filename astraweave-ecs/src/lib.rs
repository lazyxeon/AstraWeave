//! AstraWeave ECS — Production-grade, AI-native ECS for game development.
//!
//! This ECS is designed specifically for AI-first game engines, providing:
//! - **Archetype-based storage** for cache-friendly iteration (like Bevy/Flecs)
//! - **Deterministic execution** via fixed schedules and ordered iteration
//! - **Event system** for AI perception and reactive behaviors
//! - **System parameters** for ergonomic system signatures
//! - **Plugin architecture** for modular game systems
//!
//! ## Architecture
//!
//! The AI-native game loop follows: **Perception → Reasoning → Planning → Action**
//!
//! ### System Stages:
//! 1. **Perception**: Build WorldSnapshots, update AI sensors
//! 2. **Simulation**: Game logic, cooldowns, state updates
//! 3. **AI Planning**: Generate PlanIntents from AI orchestrators
//! 4. **Physics**: Apply forces, resolve collisions
//! 5. **Presentation**: Rendering, audio, UI updates
//!
//! ## Example
//!
//! ```
//! use astraweave_ecs::*;
//!
//! let mut world = World::new();
//! let e = world.spawn();
//! world.insert(e, 42_u32);
//! assert_eq!(world.get::<u32>(e), Some(&42));
//! world.despawn(e);
//! assert!(!world.is_alive(e));
//! ```

#[cfg(feature = "profiling")]
use astraweave_profiling::{plot, span};

pub mod archetype;
pub mod blob_vec;
pub mod command_buffer;
pub mod component_meta;
pub mod entity_allocator;
pub mod events;
pub mod parallel;
pub mod rng;
pub mod sparse_set;
mod system_param;
pub mod type_registry;

#[cfg(feature = "alloc-counter")]
pub mod counting_alloc;

#[cfg(test)]
mod determinism_tests;

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod mutation_tests;

#[cfg(test)]
mod mutation_resistance_tests;

// Kani formal verification proofs
#[cfg(kani)]
mod blob_vec_kani;

#[cfg(kani)]
mod entity_allocator_kani;

use std::any::TypeId;
use std::collections::HashMap;

use archetype::{ArchetypeId, ArchetypeSignature, ArchetypeStorage};
pub use command_buffer::CommandBuffer;
use component_meta::ComponentMetaRegistry;
pub use entity_allocator::{Entity, EntityAllocator};
pub use events::{Event, EventReader, Events};
pub use parallel::{ParallelSchedule, SystemAccess, SystemDescriptor};
pub use rng::Rng;
pub use system_param::{Query, Query2, Query2Mut, SystemParam};
pub use type_registry::TypeRegistry;

pub trait Component: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Component for T {}

/// Marker trait for resources (singletons in World)
pub trait Resource: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Resource for T {}

/// System stage identifiers for the AI-native game loop
pub struct SystemStage;

impl SystemStage {
    pub const PRE_SIMULATION: &'static str = "pre_simulation";
    pub const PERCEPTION: &'static str = "perception";
    pub const SIMULATION: &'static str = "simulation";
    pub const AI_PLANNING: &'static str = "ai_planning";
    pub const PHYSICS: &'static str = "physics";
    pub const POST_SIMULATION: &'static str = "post_simulation";
    pub const PRESENTATION: &'static str = "presentation";
}
// Entity and EntityAllocator are now exported from entity_allocator module

/// The ECS World: contains all entities, components, and resources.
///
/// # Component Storage
///
/// The World supports two component storage modes:
///
/// 1. **Legacy Mode (default)**: Components stored in `Box<dyn Any>`.
///    - Works for any component type
///    - Higher overhead due to heap indirection
///
/// 2. **BlobVec Mode (optimized)**: Components stored in contiguous BlobVec.
///    - Requires component to implement Clone
///    - Register with `world.register_component::<T>()`
///    - 2-10× faster iteration
#[derive(Default)]
pub struct World {
    entity_allocator: EntityAllocator,
    archetypes: ArchetypeStorage,
    resources: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>, // singletons
    type_registry: TypeRegistry,
    /// Component metadata registry for BlobVec storage
    /// Components registered here use the high-performance BlobVec path
    component_registry: ComponentMetaRegistry,
    /// Cached empty archetype ID to avoid per-spawn signature creation + HashMap lookup
    empty_archetype_id: Option<ArchetypeId>,
    /// Monotonically increasing tick counter for change detection.
    /// Incremented once per frame/system-run via `increment_change_tick()`.
    /// Used to stamp component mutations so `each_changed::<T>()` can filter
    /// entities whose component was modified after a given tick.
    change_tick: u32,
}

impl World {
    /// Creates a new, empty ECS world.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let world = World::new();
    /// assert_eq!(world.entity_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current change tick.
    ///
    /// The change tick is a monotonically increasing counter used for change
    /// detection. Every mutation (insert, get_mut, each_mut) stamps the current
    /// tick on the affected component column. Call [`each_changed`] with a
    /// previous tick to iterate only entities whose component changed since then.
    #[inline]
    pub fn change_tick(&self) -> u32 {
        self.change_tick
    }

    /// Advances the change tick by one and returns the new value.
    ///
    /// Call this once per frame or once per system batch to establish a new
    /// "generation" for change detection.
    #[inline]
    pub fn increment_change_tick(&mut self) -> u32 {
        self.change_tick = self.change_tick.wrapping_add(1);
        self.change_tick
    }

    /// Spawns a new entity in the world and returns its handle.
    ///
    /// Each spawned entity has a unique [`Entity`] handle that can be used to
    /// insert, query, and remove components.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn();
    /// assert_eq!(world.entity_count(), 1);
    /// assert!(world.is_alive(entity));
    /// ```
    #[allow(clippy::expect_used)] // INVARIANT: archetype just created by get_or_create_archetype
    pub fn spawn(&mut self) -> Entity {
        #[cfg(feature = "profiling")]
        span!("ECS::World::spawn");

        let e = self.entity_allocator.spawn();

        #[cfg(feature = "profiling")]
        plot!(
            "ECS::entity_count",
            self.entity_allocator.alive_count() as u64
        );

        // An entity with no components lives in the empty archetype.
        // Cache the empty archetype ID to avoid per-spawn signature + HashMap lookup.
        let archetype_id = match self.empty_archetype_id {
            Some(id) => id,
            None => {
                let empty_sig = ArchetypeSignature::new(vec![]);
                let id = self.archetypes.get_or_create_archetype(empty_sig);
                self.empty_archetype_id = Some(id);
                id
            }
        };
        self.archetypes.set_entity_archetype(e, archetype_id);
        let archetype = self
            .archetypes
            .get_archetype_mut(archetype_id)
            .expect("BUG: archetype should exist after get_or_create_archetype");
        // Empty archetype has no component columns — HashMap::new() is zero-cost
        // (doesn't allocate until first insert).
        archetype.add_entity(e, HashMap::new());
        e
    }

    /// Check if an entity is alive in this world.
    ///
    /// # Returns
    ///
    /// - `true` if entity ID and generation match
    /// - `false` if entity is dead or never existed
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entity_allocator.is_alive(entity)
    }

    /// Check if a component type is registered for BlobVec storage.
    ///
    /// Components registered with `register_component::<T>()` where T: Clone
    /// will use the high-performance BlobVec path.
    #[inline]
    pub fn is_component_registered_blob<T: Component>(&self) -> bool {
        self.component_registry.is_registered(TypeId::of::<T>())
    }

    /// Inserts a component on an entity, moving it to the appropriate archetype.
    ///
    /// If the entity is dead (stale handle), the insert is silently ignored.
    /// If the entity already has a component of this type, it is replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn();
    /// world.insert(e, 42_u32);
    /// assert_eq!(world.get::<u32>(e), Some(&42));
    /// ```
    pub fn insert<T: Component>(&mut self, e: Entity, c: T) {
        // Validate entity is alive
        if !self.is_alive(e) {
            return; // Silently ignore stale entities
        }

        let type_id = TypeId::of::<T>();
        let boxed = Box::new(c) as Box<dyn std::any::Any + Send + Sync>;

        // Fast path: avoid allocating a temporary HashMap for the common single-component case.
        // We extract current components, insert the new one directly, then move to the new archetype.
        let old_archetype_id = match self.archetypes.get_entity_archetype(e) {
            Some(id) => id,
            None => return,
        };

        let mut current_components = {
            let old_archetype = match self.archetypes.get_archetype_mut(old_archetype_id) {
                Some(a) => a,
                None => return,
            };
            old_archetype.remove_entity_components(e)
        };

        // Build new signature by extending the old one with the new type
        let new_sig_types = {
            let old_archetype = match self.archetypes.get_archetype(old_archetype_id) {
                Some(a) => a,
                None => return,
            };
            let mut sig_types: Vec<_> = old_archetype.signature.components.clone();
            if !sig_types.contains(&type_id) {
                sig_types.push(type_id);
            }
            sig_types
        };

        let new_signature = ArchetypeSignature::new(new_sig_types);
        let new_archetype_id = self.archetypes.get_or_create_archetype(new_signature);

        // Move entity mapping
        if let Some(old_arch) = self.archetypes.get_archetype_mut(old_archetype_id) {
            old_arch.remove_entity(e);
        }
        self.archetypes.set_entity_archetype(e, new_archetype_id);

        // Insert new component directly into the extracted map (no extra HashMap alloc)
        current_components.insert(type_id, boxed);

        let tick = self.change_tick;
        if let Some(new_arch) = self.archetypes.get_archetype_mut(new_archetype_id) {
            new_arch.add_entity_with_tick(e, current_components, tick);
        }
    }

    #[allow(clippy::expect_used)] // INVARIANT: archetype/entity existence validated by prior operations in each step
    fn move_entity_to_new_archetype(
        &mut self,
        entity: Entity,
        new_components: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
        is_removing: bool,
    ) {
        // 1. Get current archetype and component data
        let old_archetype_id = self
            .archetypes
            .get_entity_archetype(entity)
            .expect("BUG: entity should have archetype");

        let mut current_components = {
            let old_archetype = self
                .archetypes
                .get_archetype_mut(old_archetype_id)
                .expect("BUG: archetype should exist for entity");
            old_archetype.remove_entity_components(entity)
        };

        // 2. Determine new signature
        let new_sig_types = {
            let old_archetype = self
                .archetypes
                .get_archetype(old_archetype_id)
                .expect("BUG: archetype should exist");
            let mut sig_types: Vec<_> = old_archetype.signature.components.clone();
            if is_removing {
                // For removal, the `new_components` map just contains the TypeId of the component to remove.
                let type_to_remove = new_components
                    .keys()
                    .next()
                    .expect("BUG: remove should have at least one component type");
                sig_types.retain(|&tid| tid != *type_to_remove);
            } else {
                sig_types.extend(new_components.keys());
            }
            sig_types
        };

        let new_signature = ArchetypeSignature::new(new_sig_types);

        // 3. Get or create new archetype
        let new_archetype_id = self.archetypes.get_or_create_archetype(new_signature);

        // 4. Move entity's archetype mapping
        self.archetypes
            .get_archetype_mut(old_archetype_id)
            .expect("BUG: old archetype should exist")
            .remove_entity(entity);
        self.archetypes
            .set_entity_archetype(entity, new_archetype_id);

        // 5. Add entity with all components to new archetype
        let final_components = if is_removing {
            let type_to_remove = new_components
                .keys()
                .next()
                .expect("BUG: remove should have at least one component type");
            current_components.remove(type_to_remove);
            current_components
        } else {
            current_components.extend(new_components);
            current_components
        };

        let new_archetype = self
            .archetypes
            .get_archetype_mut(new_archetype_id)
            .expect("BUG: archetype should exist after get_or_create_archetype");
        let tick = self.change_tick;
        new_archetype.add_entity_with_tick(entity, final_components, tick);
    }

    /// Returns an immutable reference to a component on an entity.
    ///
    /// Returns `None` if the entity is dead or does not have the component.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn();
    /// world.insert(e, String::from("hello"));
    /// assert_eq!(world.get::<String>(e).map(|s| s.as_str()), Some("hello"));
    /// assert_eq!(world.get::<u32>(e), None); // component not present
    /// ```
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        #[cfg(feature = "profiling")]
        span!("ECS::World::get");

        // Validate entity is alive
        if !self.is_alive(e) {
            return None;
        }

        let archetype_id = self.archetypes.get_entity_archetype(e)?;
        let archetype = self.archetypes.get_archetype(archetype_id)?;
        archetype.get::<T>(e)
    }

    /// Returns a mutable reference to a component on an entity.
    ///
    /// Returns `None` if the entity is dead or does not have the component.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn();
    /// world.insert(e, 10_i32);
    /// if let Some(val) = world.get_mut::<i32>(e) {
    ///     *val += 5;
    /// }
    /// assert_eq!(world.get::<i32>(e), Some(&15));
    /// ```
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T> {
        // Validate entity is alive
        if !self.is_alive(e) {
            return None;
        }

        let tick = self.change_tick;
        let archetype_id = self.archetypes.get_entity_archetype(e)?;
        let archetype = self.archetypes.get_archetype_mut(archetype_id)?;
        // Conservative change detection: stamp tick on access (Bevy-style).
        // Even if the caller doesn't mutate, we record it as changed.
        archetype.stamp_change_tick::<T>(e, tick);
        archetype.get_mut::<T>(e)
    }

    /// Inserts a singleton resource into the world.
    ///
    /// Resources are accessed by type rather than by entity. If a resource of the
    /// same type already exists, it is overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// struct GameTime(f32);
    ///
    /// let mut world = World::new();
    /// world.insert_resource(GameTime(0.0));
    /// assert_eq!(world.get_resource::<GameTime>().unwrap().0, 0.0);
    /// ```
    pub fn insert_resource<T: 'static + Send + Sync>(&mut self, r: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(r));
    }

    /// Returns a reference to a singleton resource, if it has been inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// assert!(world.get_resource::<String>().is_none());
    /// world.insert_resource(String::from("hello"));
    /// assert_eq!(world.get_resource::<String>().unwrap().as_str(), "hello");
    /// ```
    pub fn get_resource<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref()
    }

    pub fn get_resource_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }

    /// Iterates all entities with component `T`, giving mutable access.
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_ecs::World;
    ///
    /// let mut world = World::new();
    /// let a = world.spawn();
    /// let b = world.spawn();
    /// world.insert(a, 1_i32);
    /// world.insert(b, 2_i32);
    ///
    /// world.each_mut::<i32>(|_entity, val| *val *= 10);
    ///
    /// assert_eq!(world.get::<i32>(a), Some(&10));
    /// assert_eq!(world.get::<i32>(b), Some(&20));
    /// ```
    #[allow(clippy::expect_used)] // INVARIANT: archetype_ids sourced from archetypes_with_component
    pub fn each_mut<T: Component>(&mut self, mut f: impl FnMut(Entity, &mut T)) {
        let tick = self.change_tick;
        let archetypes_with_t = self
            .archetypes
            .archetypes_with_component(TypeId::of::<T>())
            .map(|a| a.id)
            .collect::<Vec<_>>();

        for archetype_id in archetypes_with_t {
            let archetype = self
                .archetypes
                .get_archetype_mut(archetype_id)
                .expect("BUG: archetype should exist from archetypes_with_component");
            // NEW: entities_vec() now returns &[Entity] (zero-cost!)
            let entities: Vec<Entity> = archetype.entities_vec().to_vec();
            for entity in entities {
                // Stamp change tick before borrowing component (avoids double &mut)
                archetype.stamp_change_tick::<T>(entity, tick);
                if let Some(component) = archetype.get_mut::<T>(entity) {
                    f(entity, component);
                }
            }
        }
    }

    pub fn count<T: Component>(&self) -> usize {
        self.archetypes
            .archetypes_with_component(TypeId::of::<T>())
            .map(|archetype| archetype.len())
            .sum()
    }

    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        // Validate entity is alive before checking components
        if !self.is_alive(entity) {
            return false;
        }
        self.get::<T>(entity).is_some()
    }

    pub fn entities_with<T: Component>(&self) -> Vec<Entity> {
        self.archetypes
            .archetypes_with_component(TypeId::of::<T>())
            .flat_map(|archetype| archetype.entities_vec().iter().copied())
            .collect()
    }

    /// Iterate entities whose component `T` was modified after `since_tick`.
    ///
    /// This is the primary change-detection query. It checks the per-entity
    /// change tick stamped by `insert`, `get_mut`, and `each_mut` against the
    /// provided `since_tick` value. Only entities whose tick is **strictly
    /// greater** than `since_tick` are yielded.
    ///
    /// # Typical usage
    ///
    /// ```rust,ignore
    /// let last_tick = world.change_tick();
    /// world.increment_change_tick();
    /// // … systems mutate components …
    /// world.each_changed::<CTransformLocal>(last_tick, |entity, transform| {
    ///     // Only entities whose CTransformLocal changed since last_tick
    /// });
    /// ```
    pub fn each_changed<T: Component>(&self, since_tick: u32, mut f: impl FnMut(Entity, &T)) {
        let type_id = TypeId::of::<T>();
        for archetype in self.archetypes.archetypes_with_component(type_id) {
            for &entity in archetype.entities_vec() {
                if let Some(tick) = archetype.get_change_tick::<T>(entity) {
                    if tick > since_tick {
                        if let Some(component) = archetype.get::<T>(entity) {
                            f(entity, component);
                        }
                    }
                }
            }
        }
    }

    /// Returns the change tick for a specific component on an entity.
    ///
    /// Returns `None` if the entity is dead, missing the component, or
    /// change tracking data is unavailable.
    pub fn get_change_tick<T: Component>(&self, entity: Entity) -> Option<u32> {
        if !self.is_alive(entity) {
            return None;
        }
        let archetype_id = self.archetypes.get_entity_archetype(entity)?;
        let archetype = self.archetypes.get_archetype(archetype_id)?;
        archetype.get_change_tick::<T>(entity)
    }

    pub fn remove<T: Component>(&mut self, e: Entity) -> bool {
        // Validate entity is alive
        if !self.is_alive(e) {
            return false;
        }

        if !self.has::<T>(e) {
            return false;
        }
        let mut components_to_remove = HashMap::new();
        // We just need the type id for the signature change. The value is irrelevant.
        components_to_remove.insert(
            TypeId::of::<T>(),
            Box::new(0) as Box<dyn std::any::Any + Send + Sync>,
        );
        self.move_entity_to_new_archetype(e, components_to_remove, true);
        true
    }

    /// Despawn an entity, removing it from the world.
    ///
    /// # Returns
    ///
    /// - `true` if entity was alive and despawned
    /// - `false` if entity was already dead (stale handle)
    ///
    /// # Example
    ///
    /// ```
    /// use astraweave_ecs::*;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn();
    ///
    /// assert!(world.despawn(e));  // First despawn succeeds
    /// assert!(!world.despawn(e)); // Second despawn fails (stale)
    /// ```
    #[allow(clippy::expect_used)] // INVARIANT: entity alive + has archetype validated above
    pub fn despawn(&mut self, entity: Entity) -> bool {
        // First validate entity is alive
        if !self.entity_allocator.is_alive(entity) {
            return false;
        }

        // Remove from archetype (removes entity AND all components)
        if let Some(archetype_id) = self.archetypes.get_entity_archetype(entity) {
            let archetype = self
                .archetypes
                .get_archetype_mut(archetype_id)
                .expect("BUG: archetype should exist for entity");
            // Use remove_entity_components to properly clean up packed storage
            archetype.remove_entity_components(entity);
            self.archetypes.remove_entity(entity);
        }

        // Despawn from allocator (increments generation)
        self.entity_allocator.despawn(entity)
    }

    /// Get the number of entities currently alive.
    pub fn entity_count(&self) -> usize {
        self.entity_allocator.alive_count()
    }

    /// Get read-only access to the archetype storage.
    ///
    /// # Use Cases
    ///
    /// - Iterating all entities across all archetypes
    /// - Querying archetype metadata (signatures, counts)
    /// - Testing determinism properties
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for archetype in world.archetypes().iter() {
    ///     for &entity in archetype.entities_vec() {
    ///         // Process entity
    ///     }
    /// }
    /// ```
    pub fn archetypes(&self) -> &ArchetypeStorage {
        &self.archetypes
    }
}

// Schedule and systems
pub type SystemFn = fn(&mut World);

#[derive(Default)]
pub struct Schedule {
    pub stages: Vec<Stage>,
}

pub struct Stage {
    pub name: &'static str,
    pub systems: Vec<SystemFn>,
}

impl Schedule {
    pub fn with_stage(mut self, name: &'static str) -> Self {
        self.stages.push(Stage {
            name,
            systems: vec![],
        });
        self
    }
    pub fn add_system(&mut self, stage: &'static str, sys: SystemFn) {
        if let Some(s) = self.stages.iter_mut().find(|s| s.name == stage) {
            s.systems.push(sys);
        }
    }
    pub fn run(&self, world: &mut World) {
        #[cfg(feature = "profiling")]
        span!("ECS::Schedule::run");

        for s in &self.stages {
            for f in &s.systems {
                (f)(world);
            }
        }
    }
}

// App-like builder with deterministic fixed-timestep driver
/// A high-level application driver combining a [`World`] with a [`Schedule`].
///
/// `App` provides a builder pattern for setting up ECS systems and
/// running them for a fixed number of ticks. Systems execute deterministically
/// in stage order every tick.
///
/// # Examples
///
/// ```
/// use astraweave_ecs::{App, World};
///
/// fn gravity_system(world: &mut World) {
///     world.each_mut::<f32>(|_e, y| *y -= 9.8);
/// }
///
/// let app = App::new();
/// // Access the world after running
/// assert_eq!(app.world.entity_count(), 0);
/// ```
pub struct App {
    pub world: World,
    pub schedule: Schedule,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut schedule = Schedule::default();
        schedule = schedule
            .with_stage("perception")
            .with_stage("simulation")
            .with_stage("ai_planning")
            .with_stage("physics")
            .with_stage("presentation");
        Self {
            world: World::new(),
            schedule,
        }
    }

    pub fn add_system(&mut self, stage: &'static str, sys: SystemFn) {
        self.schedule.add_system(stage, sys);
    }
    pub fn insert_resource<T: 'static + Send + Sync>(mut self, r: T) -> Self {
        self.world.insert_resource(r);
        self
    }
    pub fn run_fixed(mut self, steps: u32) -> Self {
        for _ in 0..steps {
            self.schedule.run(&mut self.world);
        }
        self
    }
}

impl World {
    /// Register a component type for type-erased operations (used by CommandBuffer).
    ///
    /// This must be called for any component type that will be used with CommandBuffer.
    ///
    /// # Example
    /// ```
    /// # use astraweave_ecs::World;
    /// # #[derive(Clone, Copy)]
    /// # struct Position { x: f32, y: f32 }
    /// let mut world = World::new();
    /// world.register_component::<Position>();
    /// ```
    pub fn register_component<T: Component + Clone>(&mut self) {
        self.type_registry.register::<T>();
        // Also register with component_meta for BlobVec storage
        self.component_registry.register::<T>();
    }

    /// Insert a type-erased component (used by CommandBuffer).
    ///
    /// # Panics
    /// Panics if the component type is not registered via `register_component<T>()`.
    /// # Panics
    /// Panics if the component type is not registered via `register_component<T>()`.
    #[allow(clippy::panic)] // Documented API contract: unregistered type is a programmer error
    pub(crate) fn insert_boxed(
        &mut self,
        entity: Entity,
        type_id: TypeId,
        component: Box<dyn std::any::Any + Send + Sync>,
    ) {
        if !self.is_alive(entity) {
            return; // Stale entity, silently ignore
        }

        let handler = self
            .type_registry
            .insert_handlers
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "insert_boxed: type {:?} not registered (call register_component::<T>() first)",
                    type_id
                )
            });

        handler(self, entity, component);
    }

    /// Remove a component by TypeId (used by CommandBuffer).
    ///
    /// # Panics
    /// Panics if the component type is not registered via `register_component<T>()`.
    /// # Panics
    /// Panics if the component type is not registered via `register_component<T>()`.
    #[allow(clippy::panic)] // Documented API contract: unregistered type is a programmer error
    pub(crate) fn remove_by_type_id(&mut self, entity: Entity, type_id: TypeId) {
        if !self.is_alive(entity) {
            return; // Stale entity, silently ignore
        }

        let handler = self
            .type_registry
            .remove_handlers
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "remove_by_type_id: type {:?} not registered (call register_component::<T>() first)",
                    type_id
                )
            });

        handler(self, entity);
    }
}

// Plugin pattern similar to Bevy
pub trait Plugin {
    fn build(&self, app: &mut App);
}
impl App {
    pub fn add_plugin(mut self, p: impl Plugin) -> Self {
        p.build(&mut self);
        self
    }
}

// SECTION: System Execution

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity {
        vx: f32,
        vy: f32,
    }

    #[derive(Debug, PartialEq)]
    struct TestResource(i32);

    #[test]
    fn test_spawn_and_insert() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        assert!(world.has::<Position>(entity));
        assert!(!world.has::<Velocity>(entity));

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(*pos, Position { x: 1.0, y: 2.0 });
    }

    #[test]
    fn test_despawn() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        assert!(world.is_alive(entity));
        world.despawn(entity);
        assert!(!world.is_alive(entity));
        assert!(!world.has::<Position>(entity));
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { vx: 0.0, vy: 0.0 });

        assert!(world.has::<Position>(entity));
        world.remove::<Position>(entity);
        assert!(!world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity)); // Other components should remain
    }

    #[test]
    fn test_query_single_component() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        let e3 = world.spawn();
        world.insert(e3, Velocity { vx: 0.0, vy: 0.0 });

        let query = Query::<Position>::new(&world);
        let mut count = 0;
        let mut total_x = 0.0;
        for (entity, pos) in query {
            count += 1;
            total_x += pos.x;
            assert!(entity == e1 || entity == e2);
        }
        assert_eq!(count, 2);
        assert_eq!(total_x, 3.0);
    }

    #[test]
    fn test_query_two_components() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e1, Velocity { vx: 1.0, vy: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        let e3 = world.spawn();
        world.insert(e3, Position { x: 3.0, y: 3.0 });
        world.insert(e3, Velocity { vx: 3.0, vy: 3.0 });

        let query = Query2::<Position, Velocity>::new(&world);
        let mut count = 0;
        for (entity, pos, vel) in query {
            count += 1;
            assert!(entity == e1 || entity == e3);
            assert_eq!(pos.x, vel.vx);
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_resource_management() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));

        let resource = world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 42);

        let resource_mut = world.get_resource_mut::<TestResource>().unwrap();
        resource_mut.0 = 100;

        let resource_after = world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource_after.0, 100);
    }

    #[test]
    fn test_get_mut() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        let pos_mut = world.get_mut::<Position>(entity).unwrap();
        pos_mut.x = 5.0;

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 5.0);
    }

    // ====================
    // Day 2: World Advanced API Tests
    // ====================

    #[test]
    fn test_count_single_component() {
        let mut world = World::new();

        assert_eq!(world.count::<Position>(), 0);

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        assert_eq!(world.count::<Position>(), 1);

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        assert_eq!(world.count::<Position>(), 2);

        let _e3 = world.spawn();
        world.insert(_e3, Velocity { vx: 1.0, vy: 1.0 });
        assert_eq!(world.count::<Position>(), 2);
        assert_eq!(world.count::<Velocity>(), 1);
    }

    #[test]
    fn test_count_across_archetypes() {
        let mut world = World::new();

        // Archetype 1: Position only
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        // Archetype 2: Position + Velocity
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e2, Velocity { vx: 1.0, vy: 1.0 });

        // Should count both
        assert_eq!(world.count::<Position>(), 2);
        assert_eq!(world.count::<Velocity>(), 1);
    }

    #[test]
    fn test_entities_with_single_component() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        let _e3 = world.spawn();
        world.insert(_e3, Velocity { vx: 1.0, vy: 1.0 });

        let entities = world.entities_with::<Position>();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
    }

    #[test]
    fn test_entities_with_empty_result() {
        let world = World::new();
        let entities = world.entities_with::<Position>();
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_entities_with_across_archetypes() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e2, Velocity { vx: 1.0, vy: 1.0 });

        let entities = world.entities_with::<Position>();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
    }

    #[test]
    fn test_each_mut_modify_components() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        world.each_mut::<Position>(|_e, pos| {
            pos.x += 10.0;
        });

        assert_eq!(world.get::<Position>(e1).unwrap().x, 11.0);
        assert_eq!(world.get::<Position>(e2).unwrap().x, 12.0);
    }

    #[test]
    fn test_each_mut_with_entity_access() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        let mut visited = Vec::new();
        world.each_mut::<Position>(|entity, _pos| {
            visited.push(entity);
        });

        assert_eq!(visited.len(), 2);
        assert!(visited.contains(&e1));
        assert!(visited.contains(&e2));
    }

    #[test]
    fn test_entity_count() {
        let mut world = World::new();

        assert_eq!(world.entity_count(), 0);

        let e1 = world.spawn();
        assert_eq!(world.entity_count(), 1);

        let _e2 = world.spawn();
        assert_eq!(world.entity_count(), 2);

        world.despawn(e1);
        assert_eq!(world.entity_count(), 1);
    }

    // ====================
    // Day 2: Stale Entity Handling Tests
    // ====================

    #[test]
    fn test_insert_on_stale_entity_ignored() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);

        // Insert on stale entity should be ignored silently
        world.insert(entity, Position { x: 1.0, y: 1.0 });

        assert!(!world.has::<Position>(entity));
        assert_eq!(world.count::<Position>(), 0);
    }

    #[test]
    fn test_get_on_stale_entity_returns_none() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 1.0 });
        world.despawn(entity);

        assert!(world.get::<Position>(entity).is_none());
    }

    #[test]
    fn test_get_mut_on_stale_entity_returns_none() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 1.0 });
        world.despawn(entity);

        assert!(world.get_mut::<Position>(entity).is_none());
    }

    #[test]
    fn test_has_on_stale_entity_returns_false() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 1.0 });
        world.despawn(entity);

        assert!(!world.has::<Position>(entity));
    }

    #[test]
    fn test_remove_on_stale_entity_returns_false() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 1.0 });
        world.despawn(entity);

        assert!(!world.remove::<Position>(entity));
    }

    #[test]
    fn test_despawn_stale_entity_returns_false() {
        let mut world = World::new();
        let entity = world.spawn();

        assert!(world.despawn(entity));
        assert!(!world.despawn(entity)); // Second despawn should return false
    }

    #[test]
    fn test_remove_nonexistent_component_returns_false() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 1.0 });

        // Removing component that doesn't exist
        assert!(!world.remove::<Velocity>(entity));
    }

    // ====================
    // Day 2: Resource Edge Cases
    // ====================

    #[test]
    fn test_resource_get_nonexistent_returns_none() {
        let world = World::new();
        assert!(world.get_resource::<TestResource>().is_none());
    }

    #[test]
    fn test_resource_get_mut_nonexistent_returns_none() {
        let mut world = World::new();
        assert!(world.get_resource_mut::<TestResource>().is_none());
    }

    #[test]
    fn test_resource_replace() {
        let mut world = World::new();
        world.insert_resource(TestResource(10));

        world.insert_resource(TestResource(20));

        let resource = world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 20);
    }

    // ====================
    // Day 2: App/Schedule API Tests
    // ====================

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.world.entity_count(), 0);
        assert_eq!(app.schedule.stages.len(), 5);
    }

    #[test]
    fn test_app_insert_resource() {
        let app = App::new().insert_resource(TestResource(42));

        let resource = app.world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 42);
    }

    #[test]
    fn test_schedule_execution() {
        fn test_system(world: &mut World) {
            world.insert_resource(TestResource(99));
        }

        let mut app = App::new();
        app.add_system("simulation", test_system);
        app = app.run_fixed(1);

        let resource = app.world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 99);
    }

    #[test]
    fn test_schedule_multiple_systems() {
        fn system_a(world: &mut World) {
            world.insert_resource(TestResource(10));
        }

        fn system_b(world: &mut World) {
            if let Some(resource) = world.get_resource_mut::<TestResource>() {
                resource.0 += 5;
            }
        }

        let mut app = App::new();
        app.add_system("simulation", system_a);
        app.add_system("simulation", system_b);
        app = app.run_fixed(1);

        let resource = app.world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 15);
    }

    #[test]
    fn test_run_fixed_multiple_steps() {
        fn increment_system(world: &mut World) {
            if let Some(resource) = world.get_resource_mut::<TestResource>() {
                resource.0 += 1;
            } else {
                world.insert_resource(TestResource(1));
            }
        }

        let mut app = App::new();
        app.add_system("simulation", increment_system);
        app = app.run_fixed(10);

        let resource = app.world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 10);
    }

    // ====================
    // Day 2: Archetype Access Tests
    // ====================

    #[test]
    fn test_archetypes_read_access() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let archetypes = world.archetypes();
        let archetype_count = archetypes.iter().count();

        // Should have at least 2 archetypes: empty and Position-only
        assert!(archetype_count >= 2);
    }

    #[test]
    fn test_spawn_creates_empty_archetype_entity() {
        let mut world = World::new();
        let entity = world.spawn();

        // Entity should exist in empty archetype
        assert!(world.is_alive(entity));
        assert_eq!(world.entity_count(), 1);
    }
}

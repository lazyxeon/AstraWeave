//! Command buffer for deferred structural changes.
//!
//! Allows queueing of spawn, insert, remove, and despawn operations that will be
//! applied later via `flush()`. This prevents iterator invalidation during iteration.
//!
//! # Example
//! ```
//! # use astraweave_ecs::{World, CommandBuffer};
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Position { x: f32, y: f32 }
//! # let mut world = World::new();
//! # world.register_component::<Position>();
//! # let e1 = world.spawn();
//! # let e2 = world.spawn();
//! let mut commands = CommandBuffer::new();
//!
//! // Queue operations on entities (safe during iteration)
//! commands.insert(e1, Position { x: 10.0, y: 20.0 });
//! commands.insert(e2, Position { x: 30.0, y: 40.0 });
//!
//! // Apply all queued operations (batch update)
//! commands.flush(&mut world);
//!
//! assert_eq!(world.get::<Position>(e1), Some(&Position { x: 10.0, y: 20.0 }));
//! ```

use crate::{Component, Entity, World};
use std::any::{Any, TypeId};

/// A command that modifies the World structure.
#[derive(Debug)]
enum Command {
    /// Spawn a new entity with optional components.
    Spawn {
        /// Components to insert on spawn (type-erased).
        components: Vec<(TypeId, Box<dyn Any + Send + Sync>)>,
    },
    /// Insert a component on an entity.
    Insert {
        entity: Entity,
        type_id: TypeId,
        component: Box<dyn Any + Send + Sync>,
    },
    /// Remove a component from an entity.
    Remove { entity: Entity, type_id: TypeId },
    /// Despawn an entity.
    Despawn { entity: Entity },
}

/// Buffer for deferred structural changes to the World.
///
/// Queues spawn, insert, remove, and despawn operations that are applied
/// via `flush()`. This allows safe mutation during iteration.
///
/// # Thread Safety
/// CommandBuffer is `!Send + !Sync` to match World's single-threaded access model.
pub struct CommandBuffer {
    commands: Vec<Command>,
    spawn_buffer: Vec<(TypeId, Box<dyn Any + Send + Sync>)>,
}

impl CommandBuffer {
    /// Create a new empty command buffer.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            spawn_buffer: Vec::new(),
        }
    }

    /// Create a command buffer with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
            spawn_buffer: Vec::new(),
        }
    }

    /// Queue a spawn operation.
    ///
    /// The entity will be spawned when `flush()` is called. Returns a builder
    /// for adding components to the spawned entity.
    ///
    /// # Example
    /// ```
    /// # use astraweave_ecs::CommandBuffer;
    /// # #[derive(Clone, Copy)]
    /// # struct Position { x: f32, y: f32 }
    /// # struct Velocity { x: f32, y: f32 }
    /// let mut commands = CommandBuffer::new();
    /// commands.spawn()
    ///     .with(Position { x: 0.0, y: 0.0 })
    ///     .with(Velocity { x: 1.0, y: 0.0 });
    /// ```
    pub fn spawn(&mut self) -> SpawnBuilder<'_> {
        SpawnBuilder { buffer: self }
    }

    /// Queue an insert operation.
    ///
    /// The component will be inserted when `flush()` is called. If the entity
    /// is stale (despawned), the operation is silently ignored.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        self.commands.push(Command::Insert {
            entity,
            type_id: TypeId::of::<T>(),
            component: Box::new(component),
        });
    }

    /// Queue a remove operation.
    ///
    /// The component will be removed when `flush()` is called. If the entity
    /// is stale or doesn't have the component, the operation is silently ignored.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.commands.push(Command::Remove {
            entity,
            type_id: TypeId::of::<T>(),
        });
    }

    /// Queue a despawn operation.
    ///
    /// The entity will be despawned when `flush()` is called. If the entity
    /// is stale (already despawned), the operation is silently ignored.
    pub fn despawn(&mut self, entity: Entity) {
        self.commands.push(Command::Despawn { entity });
    }

    /// Apply all queued commands to the World.
    ///
    /// Commands are applied in FIFO order:
    /// 1. Spawn operations
    /// 2. Insert operations
    /// 3. Remove operations
    /// 4. Despawn operations
    ///
    /// After flushing, the buffer is cleared and ready for reuse.
    ///
    /// # Panic Safety
    /// If a command panics (e.g., component Drop panics), the buffer may be
    /// left in a partially-applied state. Use `try_flush()` for Result-based
    /// error handling.
    pub fn flush(&mut self, world: &mut World) {
        for command in self.commands.drain(..) {
            match command {
                Command::Spawn { components } => {
                    let entity = world.spawn();
                    for (type_id, component) in components {
                        // Type erasure: We know component is T where TypeId::of::<T>() == type_id,
                        // but we can't downcast without unsafe. Use insert_raw() or similar.
                        // For now, we'll need to refactor World::insert to accept Box<dyn Any>.
                        // Deferred: We'll add insert_raw() helper in World.
                        world.insert_boxed(entity, type_id, component);
                    }
                }
                Command::Insert {
                    entity,
                    type_id,
                    component,
                } => {
                    world.insert_boxed(entity, type_id, component);
                }
                Command::Remove { entity, type_id } => {
                    world.remove_by_type_id(entity, type_id);
                }
                Command::Despawn { entity } => {
                    world.despawn(entity);
                }
            }
        }
    }

    /// Get the number of queued commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clear all queued commands without applying them.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.spawn_buffer.clear();
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for spawning entities with components.
///
/// Created by `CommandBuffer::spawn()`. Chain `with()` calls to add components.
pub struct SpawnBuilder<'a> {
    buffer: &'a mut CommandBuffer,
}

impl<'a> SpawnBuilder<'a> {
    /// Add a component to the spawned entity.
    ///
    /// Returns self for chaining.
    pub fn with<T: Component>(self, component: T) -> Self {
        self.buffer
            .spawn_buffer
            .push((TypeId::of::<T>(), Box::new(component)));
        self
    }
}

impl<'a> Drop for SpawnBuilder<'a> {
    fn drop(&mut self) {
        // Finalize spawn command by moving buffered components into Command::Spawn
        let components = std::mem::take(&mut self.buffer.spawn_buffer);
        self.buffer.commands.push(Command::Spawn { components });
    }
}

// ============================================================================
// Tests
// ============================================================================

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
        x: f32,
        y: f32,
    }

    #[test]
    fn test_command_buffer_creation() {
        let buffer = CommandBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_command_buffer_with_capacity() {
        let buffer = CommandBuffer::with_capacity(10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        // Verify pre-allocation actually happened (kills → Default::default())
        assert!(
            buffer.commands.capacity() >= 10,
            "with_capacity(10) must pre-allocate, got capacity {}",
            buffer.commands.capacity()
        );
    }

    #[test]
    fn test_queue_insert() {
        let mut buffer = CommandBuffer::new();
        let entity = Entity::new(0, 0);

        buffer.insert(entity, Position { x: 1.0, y: 2.0 });
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_queue_remove() {
        let mut buffer = CommandBuffer::new();
        let entity = Entity::new(0, 0);

        buffer.remove::<Position>(entity);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_queue_despawn() {
        let mut buffer = CommandBuffer::new();
        let entity = Entity::new(0, 0);

        buffer.despawn(entity);
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_queue_spawn() {
        let mut buffer = CommandBuffer::new();

        buffer.spawn().with(Position { x: 1.0, y: 2.0 });
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_spawn_with_multiple_components() {
        let mut buffer = CommandBuffer::new();

        buffer
            .spawn()
            .with(Position { x: 1.0, y: 2.0 })
            .with(Velocity { x: 0.5, y: 0.0 });

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut buffer = CommandBuffer::new();
        let entity = Entity::new(0, 0);

        buffer.insert(entity, Position { x: 1.0, y: 2.0 });
        buffer.despawn(entity);
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_command_ordering() {
        let mut buffer = CommandBuffer::new();
        let e1 = Entity::new(0, 0);
        let e2 = Entity::new(1, 0);

        // Queue in specific order
        buffer.spawn().with(Position { x: 1.0, y: 1.0 });
        buffer.insert(e1, Velocity { x: 2.0, y: 2.0 });
        buffer.remove::<Position>(e2);
        buffer.despawn(e1);

        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn test_flush_insert_remove() {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();

        let mut buffer = CommandBuffer::new();
        let entity = world.spawn();

        buffer.insert(entity, Position { x: 1.0, y: 2.0 });
        buffer.flush(&mut world);

        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position { x: 1.0, y: 2.0 })
        );
    }

    #[test]
    fn test_multiple_flushes() {
        let mut world = World::new();
        let mut buffer = CommandBuffer::new();

        // First flush (empty)
        buffer.flush(&mut world);
        assert_eq!(buffer.len(), 0);

        // Second flush (empty)
        buffer.flush(&mut world);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_spawn_builder_drop() {
        let mut buffer = CommandBuffer::new();

        {
            let _builder = buffer.spawn().with(Position { x: 1.0, y: 2.0 });
            // Builder dropped here, should finalize spawn command
        }

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_flush_spawn() {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();

        let mut buffer = CommandBuffer::new();

        buffer
            .spawn()
            .with(Position { x: 5.0, y: 10.0 })
            .with(Velocity { x: 1.0, y: 2.0 });

        assert_eq!(world.entity_count(), 0);
        buffer.flush(&mut world);

        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_flush_despawn() {
        let mut world = World::new();
        let mut buffer = CommandBuffer::new();

        let entity = world.spawn();
        assert_eq!(world.entity_count(), 1);

        buffer.despawn(entity);
        buffer.flush(&mut world);

        assert_eq!(world.entity_count(), 0);
        assert!(!world.is_alive(entity));
    }

    #[test]
    fn test_insert_during_iteration() {
        let mut world = World::new();
        world.register_component::<Position>();

        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        let mut buffer = CommandBuffer::new();

        for entity in [e1, e2, e3] {
            buffer.insert(entity, Position { x: 1.0, y: 2.0 });
        }

        buffer.flush(&mut world);

        assert_eq!(
            world.get::<Position>(e1),
            Some(&Position { x: 1.0, y: 2.0 })
        );
        assert_eq!(
            world.get::<Position>(e2),
            Some(&Position { x: 1.0, y: 2.0 })
        );
        assert_eq!(
            world.get::<Position>(e3),
            Some(&Position { x: 1.0, y: 2.0 })
        );
    }

    #[test]
    fn test_stale_entity_ignored() {
        // This test verifies that stale entities are silently ignored during flush.
        // Since insert_boxed checks is_alive() first, it returns early without calling
        // the (unimplemented) type dispatch, so no panic occurs.
        let mut world = World::new();
        world.register_component::<Position>();

        let mut buffer = CommandBuffer::new();

        let entity = world.spawn();
        world.despawn(entity); // Entity now stale

        buffer.insert(entity, Position { x: 1.0, y: 2.0 });
        buffer.flush(&mut world); // No panic - stale entity ignored

        assert!(!world.is_alive(entity));
    }

    #[test]
    fn test_command_ordering_preservation() {
        let mut world = World::new();
        world.register_component::<Position>();

        let mut buffer = CommandBuffer::new();

        let e1 = world.spawn();

        buffer.insert(e1, Position { x: 1.0, y: 1.0 });
        buffer.remove::<Position>(e1);
        buffer.insert(e1, Position { x: 2.0, y: 2.0 });

        buffer.flush(&mut world);

        assert_eq!(
            world.get::<Position>(e1),
            Some(&Position { x: 2.0, y: 2.0 })
        );
    }
}

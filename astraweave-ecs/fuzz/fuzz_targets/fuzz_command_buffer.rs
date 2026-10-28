#![no_main]

use astraweave_ecs::{CommandBuffer, World};
use libfuzzer_sys::fuzz_target;

/// Fuzz Target 4: Command Buffer Operations
///
/// Tests deferred command sequences to find:
/// - Panics during command queuing
/// - Command ordering violations
/// - Memory corruption during flush
///
/// Input format: Stream of bytes where each byte represents a command:
/// - 0-84: Spawn via builder
/// - 85-169: Insert component
/// - 170-254: Despawn entity
/// - 255: Flush commands

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzTag(u8);

fuzz_target!(|data: &[u8]| {
    let mut world = World::new();
    let mut cmd = CommandBuffer::new();
    let mut tracked_entities = Vec::new();

    for &byte in data {
        match byte {
            0..=84 => {
                // Spawn using builder pattern (commands are queued)
                cmd.spawn();
            }
            85..=169 => {
                // Insert component (if we have entities)
                if !tracked_entities.is_empty() {
                    let idx = (byte as usize) % tracked_entities.len();
                    let entity = tracked_entities[idx];
                    cmd.insert(entity, FuzzTag(byte));
                }
            }
            170..=254 => {
                // Despawn (if we have entities)
                if !tracked_entities.is_empty() {
                    let idx = ((byte - 170) as usize) % tracked_entities.len();
                    let entity = tracked_entities.swap_remove(idx);
                    cmd.despawn(entity);
                }
            }
            255 => {
                // Flush commands and get spawned entities
                cmd.flush(&mut world);

                // Track entities with our tag component
                tracked_entities = world.entities_with::<FuzzTag>();
            }
        }
    }

    // Final flush
    cmd.flush(&mut world);

    // Validate that we didn't panic
    // Entity consistency is validated by the ECS itself
});

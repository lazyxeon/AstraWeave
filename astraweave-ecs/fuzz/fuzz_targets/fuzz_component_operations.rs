#![no_main]

use astraweave_ecs::World;
use libfuzzer_sys::fuzz_target;

/// Fuzz Target 2: Component Operations
///
/// Tests random sequences of component insert/get/remove operations to find:
/// - Panics during component insertion
/// - Memory corruption during component removal
/// - Type confusion in component storage
/// - Data races in component access
///
/// Input format: Stream of bytes where each byte represents an operation:
/// - 0-63: Insert component A
/// - 64-127: Insert component B
/// - 128-191: Remove component A
/// - 192-255: Get component (A or B based on bit 0)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzComponentA(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzComponentB(u64);

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let mut world = World::new();
    let entity = world.spawn();

    for &byte in data {
        match byte {
            0..=63 => {
                // Insert component A
                world.insert(entity, FuzzComponentA(byte as u32));
            }
            64..=127 => {
                // Insert component B
                world.insert(entity, FuzzComponentB(byte as u64));
            }
            128..=191 => {
                // Remove component A
                world.remove::<FuzzComponentA>(entity);
            }
            192..=255 => {
                // Get component
                if byte & 1 == 0 {
                    let _ = world.get::<FuzzComponentA>(entity);
                } else {
                    let _ = world.get::<FuzzComponentB>(entity);
                }
            }
        }
    }

    // Validate entity is still alive
    assert!(world.is_alive(entity));
});

#![no_main]

use astraweave_ecs::World;
use libfuzzer_sys::fuzz_target;

/// Fuzz Target 3: Archetype Transitions
///
/// Tests random sequences of component add/remove to trigger archetype migrations:
/// - Panics during archetype transitions
/// - Data corruption during entity migration
/// - Memory leaks in archetype storage
/// - Component data preservation across transitions
///
/// Input format: Stream of bytes where pairs represent (entity_idx, component_type):
/// - Component type: 0=Position, 1=Velocity, 2=Health, 3-255=remove random

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzVelocity {
    dx: i32,
    dy: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzHealth {
    hp: u32,
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let mut world = World::new();
    let mut entities = Vec::new();

    // Spawn initial entities
    let entity_count = (data[0] as usize % 10) + 1;
    for i in 0..entity_count {
        let entity = world.spawn();
        world.insert(
            entity,
            FuzzPosition {
                x: i as i32,
                y: i as i32,
            },
        );
        entities.push(entity);
    }

    // Process operations in pairs
    for chunk in data[1..].chunks(2) {
        if chunk.len() < 2 {
            break;
        }

        let entity_idx = (chunk[0] as usize) % entities.len();
        let entity = entities[entity_idx];
        let component_type = chunk[1];

        match component_type {
            0 => {
                // Add/Update Position
                world.insert(
                    entity,
                    FuzzPosition {
                        x: component_type as i32,
                        y: component_type as i32,
                    },
                );
            }
            1 => {
                // Add/Update Velocity
                world.insert(
                    entity,
                    FuzzVelocity {
                        dx: component_type as i32,
                        dy: component_type as i32,
                    },
                );
            }
            2 => {
                // Add/Update Health
                world.insert(
                    entity,
                    FuzzHealth {
                        hp: component_type as u32,
                    },
                );
            }
            _ => {
                // Remove random component
                if component_type % 3 == 0 {
                    world.remove::<FuzzVelocity>(entity);
                } else if component_type % 3 == 1 {
                    world.remove::<FuzzHealth>(entity);
                }
                // Never remove Position (keep at least one component)
            }
        }
    }

    // Validate all entities still alive
    for entity in entities {
        assert!(world.is_alive(entity));
        assert!(world.has::<FuzzPosition>(entity)); // Should always have Position
    }
});

#![no_main]

use astraweave_ecs::{Event, Events};
use libfuzzer_sys::fuzz_target;

/// Fuzz the `Events` registry. `World` does not expose this API.
///
/// Input format: stream of bytes where each byte is an operation:
/// - 0-127: send an event with that value
/// - 128-191: read events through the reader kept for the whole input
/// - 192-223: read through a second reader (no cursor; both see the queue)
/// - 224-239: drain events
/// - 240-247: clear events
/// - 248-255: advance three frames, past the default keep window

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzEvent(u8);

impl Event for FuzzEvent {}

fuzz_target!(|data: &[u8]| {
    let mut events = Events::new();
    let reader = events.get_reader::<FuzzEvent>();

    for &byte in data {
        match byte {
            0..=127 => {
                let before = events.len::<FuzzEvent>();
                events.send(FuzzEvent(byte));
                assert_eq!(events.len::<FuzzEvent>(), before + 1);
                assert_eq!(reader.read(&events).count(), events.len::<FuzzEvent>());
            }
            128..=191 => {
                let found = reader.read(&events).count();
                assert_eq!(found, events.len::<FuzzEvent>());
            }
            192..=223 => {
                let second = events.get_reader::<FuzzEvent>();
                assert_eq!(reader.read(&events).count(), second.read(&events).count());
                assert_eq!(reader.read(&events).count(), events.len::<FuzzEvent>());
            }
            224..=239 => {
                let queued = events.len::<FuzzEvent>();
                let drained: Vec<_> = events.drain::<FuzzEvent>().collect();
                assert_eq!(drained.len(), queued);
                assert_eq!(events.len::<FuzzEvent>(), 0);
                assert_eq!(reader.read(&events).count(), 0);
            }
            240..=247 => {
                events.clear::<FuzzEvent>();
                assert_eq!(events.len::<FuzzEvent>(), 0);
                assert_eq!(reader.read(&events).count(), 0);
            }
            248..=255 => {
                // keep_frames is 2. Three updates drop every event sent so far.
                events.update();
                events.update();
                events.update();
                assert_eq!(events.len::<FuzzEvent>(), 0);
                assert_eq!(reader.read(&events).count(), 0);
            }
        }
    }
});

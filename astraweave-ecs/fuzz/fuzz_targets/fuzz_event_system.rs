#![no_main]

use astraweave_ecs::{Event, Events};
use libfuzzer_sys::fuzz_target;

/// Fuzz the `Events` registry. `World` does not expose this API.
///
/// Input format: stream of bytes where each byte is an operation:
/// - 0-127: send an event with that value
/// - 128-191: read events
/// - 192-223: read events through a second reader
/// - 224-239: drain events
/// - 240-255: clear events and advance the frame

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzEvent(u8);

impl Event for FuzzEvent {}

fuzz_target!(|data: &[u8]| {
    let mut events = Events::new();

    for &byte in data {
        match byte {
            0..=127 => {
                events.send(FuzzEvent(byte));
            }
            128..=191 => {
                let reader = events.get_reader::<FuzzEvent>();
                let found: Vec<_> = reader.read(&events).copied().collect();
                for window in found.windows(2) {
                    let _ = (window[0], window[1]);
                }
            }
            192..=223 => {
                let reader = events.get_reader::<FuzzEvent>();
                let _found: Vec<_> = reader.read(&events).copied().collect();
            }
            224..=239 => {
                let drained: Vec<_> = events.drain::<FuzzEvent>().collect();
                for event in drained {
                    let _ = event.0;
                }
            }
            240..=255 => {
                events.clear::<FuzzEvent>();
                events.update();
            }
        }
    }
});

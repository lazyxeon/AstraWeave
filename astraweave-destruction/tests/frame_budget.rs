//! Collapse tick against the 11 ms frame budget. Uses the clock only as a stopwatch.

use astraweave_destruction::DestructionWorld;

#[test]
fn collapse_tick_fits_in_an_11ms_frame() {
    let mut world = DestructionWorld::aravath_gate(0xA2A0_0001);
    for _ in 0..19 {
        world.tick();
    }
    let start = std::time::Instant::now();
    world.tick();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_micros() < 11_000,
        "collapse tick took {elapsed:?}, budget is 11 ms"
    );
}

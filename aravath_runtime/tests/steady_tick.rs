//! A settled simulation tick and a repeat skin must not touch the heap.

use aravath_runtime::figure::Figure;
use aravath_runtime::sim::{Greybox, TickInput, FIXED_DT};
use astraweave_ecs::counting_alloc::{allocs, reset_allocs};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

/// The counters are process-wide. Tests in this binary must not overlap.
fn lock_allocs() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

#[test]
fn settled_tick_does_not_allocate() {
    let _guard = lock_allocs();
    let mut sim = Greybox::open(0);
    sim.spawn_enemy(glam::Vec3::new(20.0, 16.0, 18.0));
    sim.spawn_enemy(glam::Vec3::new(12.0, 16.0, 22.0));
    let input = TickInput {
        motor: aravath_runtime::motor::MotorInput {
            move_y: 1.0,
            ..aravath_runtime::motor::MotorInput::default()
        },
        ..TickInput::default()
    };
    for _ in 0..12 {
        sim.catch_up(FIXED_DT, input);
    }
    let pos = sim.player_pos();
    reset_allocs();
    let _ = sim.ground.stream_around(&mut sim.physics, pos, 0);
    let after_stream = allocs();
    sim.scars.tick();
    let after_scars = allocs();
    sim.graph.sync(
        aravath_runtime::locomotion::PoseInput {
            mode: sim.motor.mode,
            grounded: sim.motor.grounded,
            crouching: sim.motor.crouching,
            sprinting: sim.motor.sprinting,
            walking: sim.motor.walking,
            moving: true,
            attacking: false,
            blocking: false,
            hit: false,
            dead: false,
        },
        FIXED_DT,
    );
    let after_graph = allocs();
    assert_eq!(
        (after_stream, after_scars, after_graph),
        (0, 0, 0),
        "stream {after_stream} scars {after_scars} graph {after_graph}"
    );
    reset_allocs();
    sim.catch_up(FIXED_DT, input);
    let first = allocs();
    reset_allocs();
    sim.catch_up(FIXED_DT, input);
    let second = allocs();
    assert_eq!(
        first, second,
        "tick allocation count changed {first} -> {second}"
    );
}

#[test]
fn repeat_skin_does_not_allocate() {
    let _guard = lock_allocs();
    let mut figure = Figure::load_knight().expect("knight");
    figure.skin_into("Idle_A", 0.0, true).expect("pose");
    reset_allocs();
    figure.skin_into("Walking_A", 0.2, true).expect("pose");
    let count = allocs();
    assert_eq!(count, 0, "repeat skin allocated {count} times");
}

//! Headless Aravath demolition. The same script is the destruction replay case.

use astraweave_destruction::debris::DebrisLod;
use astraweave_destruction::{
    allows_climb, aravath_lines, demolition_digest, DestructionWorld, FractureFlags,
};

fn main() {
    let seed = 0xA2A0_0001;
    let mut world = DestructionWorld::aravath_gate(seed);
    for line in aravath_lines() {
        println!("{line}");
    }
    for _ in 0..30 {
        world.tick();
    }
    let lintel = world.graph().node(3).expect("lintel");
    println!(
        "tick 30: lintel dynamic={} climb={}",
        lintel.flags.contains(FractureFlags::DYNAMIC),
        allows_climb(lintel.flags)
    );
    for _ in 0..270 {
        world.tick();
    }
    let crater = world.terrain().surface_y(16, 16);
    let fill = world.terrain().surface_y(20, 16);
    let asleep = world
        .debris()
        .slots()
        .iter()
        .any(|slot| slot.occupied && slot.lod == DebrisLod::Merged);
    println!("tick 300: crater column {crater}, fill column {fill}, debris asleep {asleep}");
    println!("digest {}", demolition_digest(seed));
}

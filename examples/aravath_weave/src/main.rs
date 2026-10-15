//! Headless Aravath scene: heat, freeze, density, and gravity on the weave field.
//!
//! The local-sim crate owns the open-world grid. This scene drives the same
//! modifier templates the replay case hashes.

use astraweave_weaving::{
    weave_tick, weaving_case_bytes, SimField, TestSimField, WeaveScratch, FLAG_ICE, PHASE_BURN,
    PHASE_ICE, TEMPLATE_DENSITY, TEMPLATE_FREEZE, TEMPLATE_GRAVITY, TEMPLATE_HEAT,
};
use sha2::{Digest, Sha256};

fn main() {
    let mut scratch = WeaveScratch::new();
    let mut field = TestSimField::new();
    scratch
        .enqueue(1, 40, 0, 1, TEMPLATE_HEAT, [0, 0, 0])
        .expect("heat template");
    scratch
        .enqueue(10, 40, 1, 1, TEMPLATE_FREEZE, [5, 0, 0])
        .expect("freeze template");
    scratch
        .enqueue(20, 40, 2, 1, TEMPLATE_DENSITY, [0, 1, 0])
        .expect("density template");
    scratch
        .enqueue(30, 40, 3, 1, TEMPLATE_GRAVITY, [0, 0, 1])
        .expect("gravity template");

    for tick in 0..40 {
        weave_tick(&mut scratch, &mut field, tick);
    }

    let wood = field.sample([0, 0, 0]);
    let ice = field.sample([5, 0, 0]);
    let dense = field.sample([0, 1, 0]);
    let gravity = field.sample([0, 0, 1]);
    println!("Aravath weave showcase");
    println!(
        "  heat cell phase {} (burn is {PHASE_BURN}), flags {:#x}",
        wood.phase, wood.flags
    );
    println!(
        "  ice cell phase {} (ice is {PHASE_ICE}), walkable flag {}",
        ice.phase,
        ice.flags & FLAG_ICE != 0
    );
    println!(
        "  ice sheet {} and nav change {}",
        scratch.sheet_count(),
        scratch.nav_count()
    );
    println!("  density {} kg/m3", dense.density);
    println!("  gravity delta {} milli-g", gravity.gravity_delta);
    println!("  hazards in view {}", scratch.hazard_count);
    let bytes = weaving_case_bytes(0x57EA_5EED);
    let digest = Sha256::digest(&bytes);
    println!("  replay bytes {} sha256 {:x}", bytes.len(), digest);
}

//! Headless Aravath scene. Prints the stamps weaving hands to the field.
//!
//! The grid itself lives in the local-sim crate. This binary does not keep a copy.

use astraweave_weaving::{
    queue_weave, weave_tick, weaving_case_bytes, NullField, SimField, WeaveFrame, WeaveScratch,
    FLAG_ICE, PHASE_ICE, TEMPLATE_DENSITY, TEMPLATE_FREEZE, TEMPLATE_GRAVITY, TEMPLATE_HEAT,
};
use sha2::{Digest, Sha256};

fn main() {
    let mut scratch = WeaveScratch::new();
    let mut frame = WeaveFrame::new();
    let mut field = NullField;
    queue_weave(&mut scratch, 1, 40, 0, 1, TEMPLATE_HEAT, [0, 0, 0]).expect("heat template");
    queue_weave(&mut scratch, 10, 40, 1, 1, TEMPLATE_FREEZE, [5, 0, 0]).expect("freeze template");
    queue_weave(&mut scratch, 20, 40, 2, 1, TEMPLATE_DENSITY, [0, 1, 0]).expect("density template");
    queue_weave(&mut scratch, 30, 40, 3, 1, TEMPLATE_GRAVITY, [0, 0, 1]).expect("gravity template");

    for tick in 0..40 {
        weave_tick(&scratch, &mut frame, &mut field, tick);
    }

    println!("Aravath weave showcase");
    println!("  writes this tick {}", frame.write_count());
    for index in 0..frame.write_count() {
        let write = frame.write(index).expect("write");
        println!("  seq {} at {:?} {:?}", write.seq, write.origin, write.op);
    }
    let freeze = frame.write(0).map(|w| w.op);
    println!("  ice phase tag {PHASE_ICE}, ice flag {FLAG_ICE:#x}");
    println!("  first op {freeze:?}");
    println!(
        "  ice sheets {} nav changes {}",
        frame.sheet_count(),
        frame.nav_count()
    );
    println!("  field digest {:02x?}", field.field_digest());
    let bytes = weaving_case_bytes(0x57EA_5EED);
    let digest = Sha256::digest(&bytes);
    println!("  replay bytes {} sha256 {:x}", bytes.len(), digest);
}

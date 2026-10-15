//! Headless Aravath demolition. The same script is the destruction replay case.

use astraweave_destruction::{aravath_lines, demolition_digest};

fn main() {
    let seed = 0xA2A0_0001;
    for line in aravath_lines() {
        println!("{line}");
    }
    println!("digest {}", demolition_digest(seed));
}

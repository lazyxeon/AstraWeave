#![allow(dead_code)]

use criterion::criterion_main;

include!("fluids_adversarial.rs");

criterion_main!(particles_hash);

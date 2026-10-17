#![allow(dead_code)]

use criterion::criterion_main;

include!("fluid_baselines.rs");

criterion_main!(baseline_50k);

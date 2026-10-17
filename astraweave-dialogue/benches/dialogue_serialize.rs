#![allow(dead_code)]

use criterion::criterion_main;

include!("dialogue_bench.rs");

criterion_main!(serialize_build);

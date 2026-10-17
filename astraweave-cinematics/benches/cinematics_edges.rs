#![allow(dead_code)]

use criterion::criterion_main;

include!("cinematics_benchmarks.rs");

criterion_main!(edges);

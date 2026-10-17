#![allow(dead_code)]

use criterion::criterion_main;

include!("embeddings_bench.rs");

criterion_main!(text_serialize);

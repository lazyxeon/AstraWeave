#![allow(dead_code)]

use criterion::criterion_main;

include!("scene_partition_streaming.rs");

criterion_main!(cells_queries);

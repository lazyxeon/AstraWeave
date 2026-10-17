#![allow(dead_code)]

use criterion::criterion_main;

include!("scene_partition_streaming.rs");

criterion_main!(transform_scene);

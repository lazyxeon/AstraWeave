#![allow(dead_code)]

use criterion::criterion_main;

include!("net_ecs_groups.rs");

criterion_main!(delta);

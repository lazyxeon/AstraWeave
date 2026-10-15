//! Active-budget tick. The 11 ms figure is the whole frame on a
//! GTX 1660 Ti-class machine. These benches are the CPU slice: 32 regions.
//! `localsim_tick_32_regions` is the quiet budget (activation only).
//! `localsim_tick_32_busy` is the same budget with volume and heat in every region.

use astraweave_localsim::{
    demo_materials, FieldWrite, FloorSolid, LocalSim, Shape, Subject, TickInput, WriteOp,
    ACTIVE_BUDGET,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn subjects() -> Vec<Subject> {
    let mut subjects = Vec::with_capacity(ACTIVE_BUDGET);
    for i in 0..ACTIVE_BUDGET {
        subjects.push(Subject {
            id: i as u64 + 1,
            cell: [i as i32 * 30, 0, 0],
            player: i == 0,
        });
    }
    subjects
}

fn tick_empty(sim: &mut LocalSim, subjects: &[Subject], tick: u64) {
    let materials = demo_materials();
    let solid = FloorSolid::closed(-20);
    sim.tick(
        tick,
        TickInput {
            subjects,
            writes: &[],
            materials: &materials,
            zones: &[],
        },
        &solid,
    );
}

fn quiet_world() -> (LocalSim, Vec<Subject>) {
    let mut sim = LocalSim::new();
    let subjects = subjects();
    tick_empty(&mut sim, &subjects, 0);
    (sim, subjects)
}

fn busy_world() -> (LocalSim, Vec<Subject>) {
    let (mut sim, subjects) = quiet_world();
    let materials = demo_materials();
    let solid = FloorSolid::closed(-20);
    let mut writes = Vec::with_capacity(ACTIVE_BUDGET * 2);
    for (i, subject) in subjects.iter().enumerate() {
        writes.push(FieldWrite {
            seq: (i * 2) as u32,
            shape: Shape::Point,
            origin: subject.cell,
            end: subject.cell,
            radius: 0,
            op: WriteOp::AddVolume { delta: 80 },
        });
        writes.push(FieldWrite {
            seq: (i * 2 + 1) as u32,
            shape: Shape::Point,
            origin: subject.cell,
            end: subject.cell,
            radius: 0,
            op: WriteOp::AddTemp {
                delta: 2000,
                stack: true,
            },
        });
    }
    sim.tick(
        1,
        TickInput {
            subjects: &subjects,
            writes: &writes,
            materials: &materials,
            zones: &[],
        },
        &solid,
    );
    for t in 2..6 {
        tick_empty(&mut sim, &subjects, t);
    }
    (sim, subjects)
}

fn bench_tick(c: &mut Criterion) {
    let (mut quiet, quiet_subjects) = quiet_world();
    c.bench_function("localsim_tick_32_regions", |b| {
        b.iter(|| {
            tick_empty(&mut quiet, &quiet_subjects, 1);
            std::hint::black_box(quiet.field_digest())
        })
    });

    let (mut busy, busy_subjects) = busy_world();
    c.bench_function("localsim_tick_32_busy", |b| {
        b.iter(|| {
            tick_empty(&mut busy, &busy_subjects, 7);
            std::hint::black_box(busy.field_digest())
        })
    });
}

criterion_group!(benches, bench_tick);
criterion_main!(benches);

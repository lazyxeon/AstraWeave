//! Budgets for the destruction tick. Not a pull-request check.

use astraweave_destruction::budget::MAX_SIM_DEBRIS;
use astraweave_destruction::debris::DebrisPool;
use astraweave_destruction::fracture::{voronoi_cells, BreakRule};
use astraweave_destruction::graph::{Graph, PieceFlags};
use astraweave_destruction::terrain::Terrain;
use astraweave_destruction::{bake_gate, DestructionWorld, STRESS_ITERS};
use astraweave_math::Fixed64;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn chain(nodes: u64) -> Graph {
    let mut graph = Graph::new();
    let half = [Fixed64::from_int(1); 3];
    graph.add_node(
        1,
        Fixed64::from_int(1),
        [Fixed64::ZERO; 3],
        half,
        PieceFlags::ANCHOR,
    );
    for index in 1..nodes {
        graph.add_node(
            1,
            Fixed64::from_int(5),
            [
                Fixed64::from_int(index as i64),
                Fixed64::ZERO,
                Fixed64::ZERO,
            ],
            half,
            PieceFlags::empty(),
        );
        graph.add_bond(index, index + 1, Fixed64::from_int(4), Fixed64::from_int(1));
    }
    graph
}

fn graph_solve(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_solve");
    for nodes in [64u64, 256, 1024] {
        let graph = chain(nodes);
        group.bench_function(nodes.to_string(), |b| {
            b.iter(|| black_box(graph.overloaded_bonds(STRESS_ITERS, nodes as usize)));
        });
    }
    group.finish();
}

fn fracture_cut(c: &mut Criterion) {
    let mut group = c.benchmark_group("fracture_cut");
    let asset = bake_gate(1);
    group.bench_function("planar", |b| {
        b.iter(|| black_box(bake_gate(black_box(1))));
    });
    group.bench_function("voronoi_16", |b| {
        b.iter(|| {
            black_box(voronoi_cells(&asset.nodes[0], 1, 1, 1, 16));
        });
    });
    group.finish();
    let _ = BreakRule::stone();
}

fn brush_remesh(c: &mut Criterion) {
    c.bench_function("brush_remesh", |b| {
        b.iter(|| {
            let mut world = DestructionWorld::aravath_gate(1);
            for _ in 0..41 {
                world.tick();
            }
            black_box(world.terrain().occupancy_bytes((0, 0)));
        });
    });
}

fn debris_step(c: &mut Criterion) {
    let mut pool = DebrisPool::new();
    let mut terrain = Terrain::new(0);
    terrain.load((0, 0));
    let node = astraweave_destruction::graph::SupportNode {
        id: 1,
        structure: 1,
        mass: Fixed64::from_int(1),
        pos: [Fixed64::ZERO, Fixed64::from_int(2), Fixed64::ZERO],
        half: [Fixed64::from_int(1); 3],
        flags: PieceFlags::DYNAMIC,
        island: 1,
    };
    for id in 1..=MAX_SIM_DEBRIS as u64 {
        let mut spawned = node.clone();
        spawned.id = id;
        pool.spawn(&spawned, id);
    }
    c.bench_function("debris_step", |b| {
        b.iter(|| {
            pool.step(&terrain);
            black_box(pool.occupied_count());
        });
    });
}

criterion_group!(
    benches,
    graph_solve,
    fracture_cut,
    brush_remesh,
    debris_step
);
criterion_main!(benches);

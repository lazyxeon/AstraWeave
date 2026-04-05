// Astract Widget Performance Benchmarks
// Validates 60 FPS performance with large datasets

use astract::advanced::{ColorPicker, RangeSlider, TreeNode, TreeView};
use astract::animation::{AnimationController, EasingFunction, Spring, SpringParams, Tween};
use astract::charts::{Bar, BarChart, BarGroup, LineChart, PointCluster, ScatterPlot};
use astract::graph::{GraphNode, NodeGraph, Port, PortType};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use egui::Color32;
use std::hint::black_box;

// Helper: Generate point data
fn generate_points(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|i| (i as f64, (i as f64).sin() * 100.0))
        .collect()
}

// Helper: Generate bar groups
fn generate_bar_groups(group_count: usize, bars_per_group: usize) -> Vec<BarGroup> {
    (0..group_count)
        .map(|g| BarGroup {
            category: format!("Group {}", g),
            bars: (0..bars_per_group)
                .map(|b| Bar {
                    label: format!("Bar {}", b),
                    value: (b * 10) as f64,
                    color: Color32::from_rgb((b * 50) as u8, 100, 150),
                })
                .collect(),
        })
        .collect()
}

// Helper: Generate clusters
fn generate_clusters(cluster_count: usize, points_per_cluster: usize) -> Vec<PointCluster> {
    (0..cluster_count)
        .map(|c| {
            let offset = c as f64 * 100.0;
            let points: Vec<(f64, f64)> = (0..points_per_cluster)
                .map(|i| (i as f64 + offset, (i as f64).cos() * 50.0))
                .collect();
            PointCluster::new(
                format!("Cluster {}", c),
                points,
                Color32::from_rgb((c * 60) as u8, 100, 200),
            )
        })
        .collect()
}

// ====================
// CHART BENCHMARKS
// ====================

fn bench_linechart_single_series(c: &mut Criterion) {
    let mut group = c.benchmark_group("linechart_single_series");

    for size in [100, 500, 1000, 5000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let points = generate_points(size);
            b.iter(|| {
                let mut chart = LineChart::new("Benchmark");
                chart.add_series("Data".to_string(), black_box(points.clone()), Color32::BLUE);
                black_box(chart);
            });
        });
    }
    group.finish();
}

fn bench_linechart_multi_series(c: &mut Criterion) {
    let mut group = c.benchmark_group("linechart_multi_series");

    for series_count in [2, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(series_count),
            &series_count,
            |b, &series_count| {
                let points = generate_points(1000);
                b.iter(|| {
                    let mut chart = LineChart::new("Benchmark");
                    for i in 0..series_count {
                        chart.add_series(
                            format!("Series {}", i),
                            black_box(points.clone()),
                            Color32::from_rgb((i * 40) as u8, 100, 200),
                        );
                    }
                    black_box(chart);
                });
            },
        );
    }
    group.finish();
}

fn bench_barchart_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("barchart_groups");

    for group_count in [10, 25, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(group_count),
            &group_count,
            |b, &group_count| {
                let groups = generate_bar_groups(group_count, 5);
                b.iter(|| {
                    let mut chart = BarChart::new("Benchmark");
                    for grp in black_box(groups.clone()) {
                        chart.add_group(grp);
                    }
                    black_box(chart);
                });
            },
        );
    }
    group.finish();
}

fn bench_scatterplot_clusters(c: &mut Criterion) {
    let mut group = c.benchmark_group("scatterplot_clusters");

    for cluster_count in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(cluster_count),
            &cluster_count,
            |b, &cluster_count| {
                let clusters = generate_clusters(cluster_count, 500);
                b.iter(|| {
                    let mut scatter = ScatterPlot::new("Benchmark");
                    for cluster in black_box(clusters.clone()) {
                        scatter.add_cluster(cluster);
                    }
                    black_box(scatter);
                });
            },
        );
    }
    group.finish();
}

// ====================
// GRAPH BENCHMARKS
// ====================

fn bench_nodegraph_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("nodegraph_nodes");

    for node_count in [10, 50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            &node_count,
            |b, &node_count| {
                b.iter(|| {
                    let mut graph = NodeGraph::new();
                    for i in 0..node_count {
                        let mut node = GraphNode::new(i as u64, format!("Node {}", i));
                        node.add_input(Port::new(0, "In", PortType::Exec));
                        node.add_output(Port::new(0, "Out", PortType::Exec));
                        graph.add_node(node.with_position((i * 10) as f32, 100.0));
                    }
                    black_box(graph);
                });
            },
        );
    }
    group.finish();
}

fn bench_nodegraph_edges(c: &mut Criterion) {
    let mut group = c.benchmark_group("nodegraph_edges");

    for node_count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            &node_count,
            |b, &node_count| {
                b.iter(|| {
                    let mut graph = NodeGraph::new();
                    let mut node_ids = Vec::new();

                    // Create nodes
                    for i in 0..node_count {
                        let mut node = GraphNode::new(i as u64, format!("Node {}", i));
                        node.add_input(Port::new(0, "In", PortType::Exec));
                        node.add_output(Port::new(0, "Out", PortType::Exec));
                        let id = graph.add_node(node.with_position((i * 10) as f32, 100.0));
                        node_ids.push(id);
                    }

                    // Connect nodes (chain)
                    for i in 0..(node_count - 1) {
                        graph.add_edge(node_ids[i], 0, node_ids[i + 1], 0);
                    }

                    black_box(graph);
                });
            },
        );
    }
    group.finish();
}

fn bench_treeview_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("treeview_nodes");

    for node_count in [100, 500, 1000, 2000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            &node_count,
            |b, &node_count| {
                b.iter(|| {
                    let mut tree = TreeView::new();
                    let root = tree.add_node(TreeNode::new(0, "Root".to_string()).with_icon("📁"));

                    // Add flat children
                    for i in 1..node_count {
                        tree.add_child(
                            root,
                            TreeNode::new(i, format!("Node {}", i)).with_icon("📄"),
                        );
                    }

                    black_box(tree);
                });
            },
        );
    }
    group.finish();
}

fn bench_treeview_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("treeview_hierarchy");

    for depth in [5, 10, 15, 20] {
        group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
            b.iter(|| {
                let mut tree = TreeView::new();
                let mut current =
                    tree.add_node(TreeNode::new(0, "Root".to_string()).with_icon("📁"));
                let mut id = 1;

                // Create deep hierarchy
                for _ in 0..depth {
                    if let Some(new_current) = tree.add_child(
                        current,
                        TreeNode::new(id, format!("Level {}", id)).with_icon("📁"),
                    ) {
                        current = new_current;
                        id += 1;
                    }
                }

                black_box(tree);
            });
        });
    }
    group.finish();
}

// ====================
// ADVANCED WIDGET BENCHMARKS
// ====================

fn bench_colorpicker_creation(c: &mut Criterion) {
    c.bench_function("colorpicker_creation", |b| {
        b.iter(|| {
            let picker = ColorPicker::new()
                .with_color(Color32::from_rgb(100, 150, 200))
                .show_alpha(true);
            black_box(picker);
        });
    });
}

fn bench_rangeslider_creation(c: &mut Criterion) {
    c.bench_function("rangeslider_creation", |b| {
        b.iter(|| {
            let slider = RangeSlider::new(0.0, 100.0).with_min(25.0).with_max(75.0);
            black_box(slider);
        });
    });
}

// ====================
// ANIMATION BENCHMARKS
// ====================

fn bench_tween_single(c: &mut Criterion) {
    c.bench_function("tween_single_update", |b| {
        let mut tween = Tween::new(0.0_f32, 100.0, 2.0).with_easing(EasingFunction::SineInOut);
        tween.play();

        b.iter(|| {
            tween.update(black_box(0.016)); // 60 FPS
            black_box(tween.value());
        });
    });
}

fn bench_tween_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("tween_batch");

    for count in [100, 500, 1000, 5000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut tweens: Vec<_> = (0..count)
                .map(|_i| {
                    let mut t =
                        Tween::new(0.0_f32, 100.0, 2.0).with_easing(EasingFunction::SineInOut);
                    t.play();
                    t
                })
                .collect();

            b.iter(|| {
                for tween in &mut tweens {
                    tween.update(black_box(0.016));
                    black_box(tween.value());
                }
            });
        });
    }
    group.finish();
}

fn bench_spring_single(c: &mut Criterion) {
    c.bench_function("spring_single_update", |b| {
        let mut spring = Spring::with_params(0.0, SpringParams::smooth());
        spring.set_target(100.0);

        b.iter(|| {
            spring.update(black_box(0.016)); // 60 FPS
            black_box(spring.position());
        });
    });
}

fn bench_spring_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("spring_batch");

    for count in [100, 500, 1000, 5000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut springs: Vec<_> = (0..count)
                .map(|_| {
                    let mut s = Spring::with_params(0.0, SpringParams::smooth());
                    s.set_target(100.0);
                    s
                })
                .collect();

            b.iter(|| {
                for spring in &mut springs {
                    spring.update(black_box(0.016));
                    black_box(spring.position());
                }
            });
        });
    }
    group.finish();
}

fn bench_animation_controller(c: &mut Criterion) {
    let mut group = c.benchmark_group("animation_controller");

    for anim_count in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(anim_count),
            &anim_count,
            |b, &anim_count| {
                b.iter(|| {
                    let mut controller = AnimationController::new();

                    // Add animations via closures
                    for _i in 0..anim_count {
                        let mut elapsed = 0.0f32;
                        controller.add(move |dt| {
                            elapsed += dt;
                            elapsed < 2.0 // Run for 2 seconds
                        });
                    }

                    // Update all
                    controller.update(black_box(0.016));
                    black_box(controller.active_count());
                });
            },
        );
    }
    group.finish();
}

// ====================
// MEMORY BENCHMARKS
// ====================

fn bench_linechart_recreation(c: &mut Criterion) {
    let points = generate_points(1000);

    c.bench_function("linechart_recreation_1000pts", |b| {
        b.iter(|| {
            let mut chart = LineChart::new("Benchmark");
            chart.add_series("Data".to_string(), black_box(points.clone()), Color32::BLUE);
            black_box(chart);
        });
    });
}

fn bench_point_vec_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("point_vec_clone");

    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let points = generate_points(size);
            b.iter(|| {
                let cloned = points.clone();
                black_box(cloned);
            });
        });
    }
    group.finish();
}

// ====================
// CRITERION GROUPS
// ====================

criterion_group!(
    chart_benches,
    bench_linechart_single_series,
    bench_linechart_multi_series,
    bench_barchart_groups,
    bench_scatterplot_clusters,
);

criterion_group!(
    graph_benches,
    bench_nodegraph_nodes,
    bench_nodegraph_edges,
    bench_treeview_nodes,
    bench_treeview_hierarchy,
);

criterion_group!(
    advanced_benches,
    bench_colorpicker_creation,
    bench_rangeslider_creation,
);

criterion_group!(
    animation_benches,
    bench_tween_single,
    bench_tween_batch,
    bench_spring_single,
    bench_spring_batch,
    bench_animation_controller,
);

criterion_group!(
    memory_benches,
    bench_linechart_recreation,
    bench_point_vec_clone,
);

criterion_main!(
    chart_benches,
    graph_benches,
    advanced_benches,
    animation_benches,
    memory_benches,
);

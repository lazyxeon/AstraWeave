// Dialogue System Benchmark Suite.
//
// Graph validation, node traversal, serialization, and graph construction.
// Run with: `cargo bench -p astraweave-dialogue`

use astraweave_dialogue::{DialogueGraph, DialogueNode, DialogueResponse};
use criterion::{black_box, criterion_group, BenchmarkId, Criterion, Throughput};

// ============================================================================
// CORRECTNESS ASSERTION HELPERS
// ============================================================================

/// Assert that a graph validates successfully
fn assert_graph_valid(graph: &DialogueGraph) {
    assert!(
        graph.validate().is_ok(),
        "Graph should validate successfully"
    );
}

/// Assert that a node exists and has expected ID
#[allow(dead_code)]
fn assert_node_exists(graph: &DialogueGraph, id: &str) {
    let node = graph.get_node(id);
    assert!(node.is_some(), "Node '{}' should exist", id);
    assert_eq!(node.unwrap().id, id, "Node ID should match");
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

/// Create a linear dialogue chain (start -> node1 -> node2 -> ... -> end)
fn create_linear_dialogue(length: usize) -> DialogueGraph {
    let mut nodes = Vec::with_capacity(length);

    for i in 0..length {
        let id = format!("node_{}", i);
        let next_id = if i < length - 1 {
            Some(format!("node_{}", i + 1))
        } else {
            None
        };

        nodes.push(DialogueNode {
            id,
            text: format!(
                "This is dialogue text for node {}. It contains some meaningful content.",
                i
            ),
            responses: vec![DialogueResponse {
                text: "Continue to next part of the conversation".to_string(),
                next_id,
            }],
        });
    }

    DialogueGraph { nodes }
}

/// Create a branching dialogue tree (each node has multiple responses)
fn create_branching_dialogue(depth: usize, branch_factor: usize) -> DialogueGraph {
    let mut nodes = Vec::new();
    let mut queue: Vec<(String, usize)> = vec![("root".to_string(), 0)];

    while let Some((parent_id, level)) = queue.pop() {
        if level >= depth {
            // Leaf node - no children
            nodes.push(DialogueNode {
                id: parent_id,
                text: format!("Leaf dialogue at depth {}", level),
                responses: vec![],
            });
            continue;
        }

        let mut responses = Vec::with_capacity(branch_factor);
        for b in 0..branch_factor {
            let child_id = format!("{}_{}", parent_id, b);
            responses.push(DialogueResponse {
                text: format!("Option {} at depth {}", b + 1, level),
                next_id: Some(child_id.clone()),
            });
            queue.push((child_id, level + 1));
        }

        nodes.push(DialogueNode {
            id: parent_id,
            text: format!(
                "Branch point at depth {} with {} options",
                level, branch_factor
            ),
            responses,
        });
    }

    DialogueGraph { nodes }
}

/// Create a dialogue with cycles (for stress testing validation)
fn create_cyclic_dialogue(nodes_count: usize) -> DialogueGraph {
    let mut nodes = Vec::with_capacity(nodes_count);

    for i in 0..nodes_count {
        let next_id = format!("node_{}", (i + 1) % nodes_count);

        nodes.push(DialogueNode {
            id: format!("node_{}", i),
            text: format!("Cyclic node {} - this can loop back", i),
            responses: vec![
                DialogueResponse {
                    text: "Continue forward".to_string(),
                    next_id: Some(next_id),
                },
                DialogueResponse {
                    text: "Exit dialogue".to_string(),
                    next_id: None,
                },
            ],
        });
    }

    DialogueGraph { nodes }
}

/// Create a complex realistic dialogue (mix of linear paths, branches, and cycles)
fn create_complex_dialogue(total_nodes: usize) -> DialogueGraph {
    let mut nodes = Vec::with_capacity(total_nodes);

    for i in 0..total_nodes {
        let num_responses = match i % 4 {
            0 => 1, // Linear
            1 => 2, // Binary branch
            2 => 3, // Triple branch
            _ => 4, // Quad branch
        };

        let mut responses = Vec::with_capacity(num_responses);
        for r in 0..num_responses {
            let next_idx = (i + r + 1) % total_nodes;
            responses.push(DialogueResponse {
                text: format!("Response option {} from node {}", r + 1, i),
                next_id: if next_idx != i {
                    Some(format!("node_{}", next_idx))
                } else {
                    None
                },
            });
        }

        nodes.push(DialogueNode {
            id: format!("node_{}", i),
            text: format!("Complex dialogue node {} with narrative content. This represents a realistic dialogue scenario.", i),
            responses,
        });
    }

    DialogueGraph { nodes }
}

// ============================================================================
// VALIDATION BENCHMARKS
// ============================================================================

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_validation");

    // Linear graph validation
    for length in [10, 50, 100, 500] {
        let graph = create_linear_dialogue(length);

        group.throughput(Throughput::Elements(length as u64));
        group.bench_with_input(
            BenchmarkId::new("linear_graph", length),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let result = graph.validate();
                    assert!(result.is_ok(), "Linear graph should be valid");
                    black_box(result)
                })
            },
        );
    }

    // Branching graph validation
    for (depth, branch) in [(3, 3), (4, 2), (5, 2), (3, 4)] {
        let graph = create_branching_dialogue(depth, branch);
        let node_count = graph.nodes.len();

        group.throughput(Throughput::Elements(node_count as u64));
        group.bench_with_input(
            BenchmarkId::new("branching_graph", format!("d{}_b{}", depth, branch)),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let result = graph.validate();
                    assert!(result.is_ok(), "Branching graph should be valid");
                    black_box(result)
                })
            },
        );
    }

    // Cyclic graph validation
    for size in [10, 50, 100, 200] {
        let graph = create_cyclic_dialogue(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("cyclic_graph", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let result = graph.validate();
                    assert!(result.is_ok(), "Cyclic graph should be valid");
                    black_box(result)
                })
            },
        );
    }

    // Complex realistic graph validation
    for size in [25, 50, 100, 200] {
        let graph = create_complex_dialogue(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("complex_graph", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let result = graph.validate();
                    assert!(result.is_ok(), "Complex graph should be valid");
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// NODE LOOKUP BENCHMARKS
// ============================================================================

fn bench_node_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_node_lookup");

    // Lookup in graphs of different sizes
    for size in [10, 50, 100, 500, 1000] {
        let graph = create_linear_dialogue(size);

        // Lookup first node
        group.bench_with_input(
            BenchmarkId::new("lookup_first", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let node = graph.get_node(black_box("node_0"));
                    assert!(node.is_some(), "First node should exist");
                    black_box(node)
                })
            },
        );

        // Lookup middle node
        let middle_id = format!("node_{}", size / 2);
        group.bench_with_input(
            BenchmarkId::new("lookup_middle", size),
            &(graph.clone(), middle_id.clone()),
            |b, (graph, id)| {
                b.iter(|| {
                    let node = graph.get_node(black_box(id));
                    assert!(node.is_some(), "Middle node should exist");
                    black_box(node)
                })
            },
        );

        // Lookup last node
        let last_id = format!("node_{}", size - 1);
        group.bench_with_input(
            BenchmarkId::new("lookup_last", size),
            &(graph.clone(), last_id.clone()),
            |b, (graph, id)| {
                b.iter(|| {
                    let node = graph.get_node(black_box(id));
                    assert!(node.is_some(), "Last node should exist");
                    black_box(node)
                })
            },
        );

        // Lookup non-existent node
        group.bench_with_input(
            BenchmarkId::new("lookup_nonexistent", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let node = graph.get_node(black_box("nonexistent_node_id"));
                    assert!(node.is_none(), "Non-existent node should return None");
                    black_box(node)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// SERIALIZATION BENCHMARKS
// ============================================================================

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_serialization");

    // Serialize graphs of different sizes
    for size in [10, 50, 100, 200] {
        let graph = create_complex_dialogue(size);

        group.throughput(Throughput::Elements(size as u64));

        // Serialize to JSON
        group.bench_with_input(BenchmarkId::new("to_json", size), &graph, |b, graph| {
            b.iter(|| {
                let json =
                    serde_json::to_string(black_box(graph)).expect("Serialization should succeed");
                assert!(!json.is_empty(), "JSON should not be empty");
                black_box(json)
            })
        });

        // Serialize to pretty JSON
        group.bench_with_input(
            BenchmarkId::new("to_json_pretty", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let json = serde_json::to_string_pretty(black_box(graph))
                        .expect("Serialization should succeed");
                    assert!(!json.is_empty(), "JSON should not be empty");
                    black_box(json)
                })
            },
        );
    }

    // Deserialize graphs
    for size in [10, 50, 100] {
        let graph = create_complex_dialogue(size);
        let json = serde_json::to_string(&graph).unwrap();

        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(BenchmarkId::new("from_json", size), &json, |b, json| {
            b.iter(|| {
                let parsed: DialogueGraph =
                    serde_json::from_str(black_box(json)).expect("Deserialization should succeed");
                assert_eq!(parsed.nodes.len(), size, "Should have all nodes");
                black_box(parsed)
            })
        });
    }

    // Round-trip benchmark
    for size in [10, 50, 100] {
        let graph = create_complex_dialogue(size);

        group.bench_with_input(BenchmarkId::new("roundtrip", size), &graph, |b, graph| {
            b.iter(|| {
                let json =
                    serde_json::to_string(black_box(graph)).expect("Serialization should succeed");
                let parsed: DialogueGraph =
                    serde_json::from_str(&json).expect("Deserialization should succeed");
                assert_eq!(parsed.nodes.len(), graph.nodes.len());
                black_box(parsed)
            })
        });
    }

    group.finish();
}

// ============================================================================
// GRAPH CONSTRUCTION BENCHMARKS
// ============================================================================

fn bench_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_construction");

    // Linear graph construction
    for size in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("linear_construction", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let graph = create_linear_dialogue(black_box(size));
                    assert_eq!(graph.nodes.len(), size);
                    assert_graph_valid(&graph);
                    black_box(graph)
                })
            },
        );
    }

    // Branching graph construction
    for (depth, branch) in [(3, 3), (4, 2), (5, 2)] {
        group.bench_with_input(
            BenchmarkId::new("branching_construction", format!("d{}_b{}", depth, branch)),
            &(depth, branch),
            |b, &(depth, branch)| {
                b.iter(|| {
                    let graph = create_branching_dialogue(black_box(depth), black_box(branch));
                    assert!(!graph.nodes.is_empty());
                    assert_graph_valid(&graph);
                    black_box(graph)
                })
            },
        );
    }

    // Complex graph construction
    for size in [25, 50, 100] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("complex_construction", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let graph = create_complex_dialogue(black_box(size));
                    assert_eq!(graph.nodes.len(), size);
                    assert_graph_valid(&graph);
                    black_box(graph)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// TRAVERSAL BENCHMARKS
// ============================================================================

fn bench_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_traversal");

    // Iterate through all nodes
    for size in [50, 100, 500, 1000] {
        let graph = create_linear_dialogue(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("iterate_all_nodes", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let mut count = 0;
                    for node in &graph.nodes {
                        black_box(&node.id);
                        black_box(&node.text);
                        count += 1;
                    }
                    assert_eq!(count, size);
                    count
                })
            },
        );
    }

    // Count total responses in graph
    for size in [50, 100, 200] {
        let graph = create_complex_dialogue(size);

        group.bench_with_input(
            BenchmarkId::new("count_responses", size),
            &graph,
            |b, graph| {
                b.iter(|| {
                    let total: usize = graph.nodes.iter().map(|n| n.responses.len()).sum();
                    assert!(total > 0, "Should have some responses");
                    black_box(total)
                })
            },
        );
    }

    // Follow a linear path
    let linear_100 = create_linear_dialogue(100);
    group.bench_function("follow_linear_path_100", |b| {
        b.iter(|| {
            let mut current = linear_100.get_node("node_0");
            let mut steps = 0;
            while let Some(node) = current {
                steps += 1;
                if let Some(response) = node.responses.first() {
                    if let Some(ref next_id) = response.next_id {
                        current = linear_100.get_node(next_id);
                    } else {
                        current = None;
                    }
                } else {
                    current = None;
                }
            }
            assert_eq!(steps, 100, "Should traverse all 100 nodes");
            black_box(steps)
        })
    });

    group.finish();
}

// ============================================================================
// CLONE BENCHMARKS
// ============================================================================

fn bench_clone_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("dialogue_clone");

    for size in [10, 50, 100, 200] {
        let graph = create_complex_dialogue(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("clone_graph", size), &graph, |b, graph| {
            b.iter(|| {
                let cloned = black_box(graph).clone();
                assert_eq!(cloned.nodes.len(), graph.nodes.len());
                black_box(cloned)
            })
        });
    }

    // Clone individual node
    let sample_node = DialogueNode {
        id: "sample".to_string(),
        text: "This is a sample dialogue node with some meaningful text content.".to_string(),
        responses: vec![
            DialogueResponse {
                text: "Option 1".to_string(),
                next_id: Some("next_1".to_string()),
            },
            DialogueResponse {
                text: "Option 2".to_string(),
                next_id: Some("next_2".to_string()),
            },
            DialogueResponse {
                text: "Option 3".to_string(),
                next_id: None,
            },
        ],
    };

    group.bench_function("clone_node", |b| {
        b.iter(|| {
            let cloned = black_box(&sample_node).clone();
            assert_eq!(cloned.id, sample_node.id);
            black_box(cloned)
        })
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUP REGISTRATION
// ============================================================================

criterion_group!(validation_lookup, bench_validation, bench_node_lookup);
criterion_group!(serialize_build, bench_serialization, bench_graph_construction);
criterion_group!(traverse_clone, bench_traversal, bench_clone_operations);

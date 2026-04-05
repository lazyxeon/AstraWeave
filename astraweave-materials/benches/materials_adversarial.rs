//! Adversarial Materials Benchmarks
//!
//! Stress testing for material graph construction, node evaluation, and WGSL compilation.

#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-materials API)
// ============================================================================

#[derive(Clone, Debug)]
enum Node {
    // Texture nodes
    Texture2D {
        binding: u32,
        uv_channel: u32,
    },
    TextureSampler {
        filtering: Filtering,
    },

    // Math nodes
    Constant(f32),
    Constant2([f32; 2]),
    Constant3([f32; 3]),
    Constant4([f32; 4]),

    // Operations
    Add(NodeRef, NodeRef),
    Subtract(NodeRef, NodeRef),
    Multiply(NodeRef, NodeRef),
    Divide(NodeRef, NodeRef),
    Dot(NodeRef, NodeRef),
    Cross(NodeRef, NodeRef),
    Normalize(NodeRef),
    Lerp(NodeRef, NodeRef, NodeRef),
    Clamp(NodeRef, NodeRef, NodeRef),

    // Trig
    Sin(NodeRef),
    Cos(NodeRef),
    Tan(NodeRef),

    // Advanced
    Fresnel {
        normal: NodeRef,
        view: NodeRef,
        power: f32,
    },
    Noise {
        uv: NodeRef,
        scale: f32,
        octaves: u32,
    },
    Time {
        speed: f32,
    },

    // Outputs
    OutputAlbedo(NodeRef),
    OutputNormal(NodeRef),
    OutputMetallic(NodeRef),
    OutputRoughness(NodeRef),
    OutputEmissive(NodeRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct NodeRef(usize);

#[derive(Clone, Copy, Debug)]
enum Filtering {
    Point,
    Linear,
    Trilinear,
    Anisotropic(u8),
}

#[derive(Clone, Debug)]
struct MaterialGraph {
    nodes: Vec<Node>,
    connections: HashMap<NodeRef, Vec<NodeRef>>,
}

impl MaterialGraph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: HashMap::new(),
        }
    }

    fn add_node(&mut self, node: Node) -> NodeRef {
        let idx = self.nodes.len();
        self.nodes.push(node);
        NodeRef(idx)
    }

    fn connect(&mut self, from: NodeRef, to: NodeRef) {
        self.connections.entry(from).or_default().push(to);
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn compile_to_wgsl(&self) -> String {
        let mut wgsl = String::with_capacity(self.nodes.len() * 100);

        wgsl.push_str("// Generated material shader\n\n");

        // Uniforms
        wgsl.push_str("struct MaterialUniforms {\n");
        wgsl.push_str("    time: f32,\n");
        wgsl.push_str("    _padding: vec3<f32>,\n");
        wgsl.push_str("};\n\n");

        // Bindings
        let mut texture_count = 0;
        for node in &self.nodes {
            if let Node::Texture2D { binding, .. } = node {
                wgsl.push_str(&format!(
                    "@group(1) @binding({}) var tex_{}: texture_2d<f32>;\n",
                    binding, texture_count
                ));
                texture_count += 1;
            }
        }

        wgsl.push_str("\n@group(0) @binding(0) var<uniform> material: MaterialUniforms;\n\n");

        // Fragment function
        wgsl.push_str("@fragment\n");
        wgsl.push_str("fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {\n");

        // Compile each node
        for (i, node) in self.nodes.iter().enumerate() {
            let var_name = format!("n{}", i);
            match node {
                Node::Constant(v) => {
                    wgsl.push_str(&format!("    let {} = {};\n", var_name, v));
                }
                Node::Constant3(v) => {
                    wgsl.push_str(&format!(
                        "    let {} = vec3<f32>({}, {}, {});\n",
                        var_name, v[0], v[1], v[2]
                    ));
                }
                Node::Constant4(v) => {
                    wgsl.push_str(&format!(
                        "    let {} = vec4<f32>({}, {}, {}, {});\n",
                        var_name, v[0], v[1], v[2], v[3]
                    ));
                }
                Node::Add(a, b) => {
                    wgsl.push_str(&format!("    let {} = n{} + n{};\n", var_name, a.0, b.0));
                }
                Node::Multiply(a, b) => {
                    wgsl.push_str(&format!("    let {} = n{} * n{};\n", var_name, a.0, b.0));
                }
                Node::Normalize(a) => {
                    wgsl.push_str(&format!("    let {} = normalize(n{});\n", var_name, a.0));
                }
                Node::Lerp(a, b, t) => {
                    wgsl.push_str(&format!(
                        "    let {} = mix(n{}, n{}, n{});\n",
                        var_name, a.0, b.0, t.0
                    ));
                }
                Node::Sin(a) => {
                    wgsl.push_str(&format!("    let {} = sin(n{});\n", var_name, a.0));
                }
                Node::Cos(a) => {
                    wgsl.push_str(&format!("    let {} = cos(n{});\n", var_name, a.0));
                }
                Node::OutputAlbedo(a) => {
                    wgsl.push_str(&format!("    let albedo = n{};\n", a.0));
                }
                _ => {
                    wgsl.push_str(&format!("    let {} = vec4<f32>(1.0);\n", var_name));
                }
            }
        }

        wgsl.push_str("    return albedo;\n");
        wgsl.push_str("}\n");

        wgsl
    }
}

#[derive(Clone, Debug)]
struct MaterialInstance {
    graph: MaterialGraph,
    parameters: HashMap<String, MaterialParameter>,
}

#[derive(Clone, Debug)]
enum MaterialParameter {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Texture(u32),
}

// ============================================================================
// CATEGORY 1: GRAPH CONSTRUCTION
// ============================================================================

fn bench_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/graph_construction");

    // Test 1: Simple PBR material
    group.bench_function("simple_pbr_material", |bencher| {
        bencher.iter(|| {
            let mut graph = MaterialGraph::new();

            let albedo_tex = graph.add_node(Node::Texture2D {
                binding: 0,
                uv_channel: 0,
            });
            let normal_tex = graph.add_node(Node::Texture2D {
                binding: 1,
                uv_channel: 0,
            });
            let mra_tex = graph.add_node(Node::Texture2D {
                binding: 2,
                uv_channel: 0,
            });

            let _albedo_out = graph.add_node(Node::OutputAlbedo(albedo_tex));
            let _normal_out = graph.add_node(Node::OutputNormal(normal_tex));
            let _metallic_out = graph.add_node(Node::OutputMetallic(mra_tex));

            std_black_box(graph.node_count())
        });
    });

    // Test 2: Complex procedural material
    for node_count in [50, 100, 200] {
        group.throughput(Throughput::Elements(node_count as u64));

        group.bench_with_input(
            BenchmarkId::new("procedural_material", node_count),
            &node_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let mut graph = MaterialGraph::new();
                    let mut refs: Vec<NodeRef> = Vec::new();

                    // Add base nodes
                    refs.push(graph.add_node(Node::Constant3([1.0, 0.5, 0.0])));
                    refs.push(graph.add_node(Node::Constant3([0.0, 0.5, 1.0])));
                    refs.push(graph.add_node(Node::Time { speed: 1.0 }));

                    // Build node chain
                    for i in 3..count {
                        let node = match i % 10 {
                            0 => Node::Add(refs[i - 1], refs[i - 2]),
                            1 => Node::Multiply(refs[i - 1], refs[i - 2]),
                            2 => Node::Lerp(refs[i - 1], refs[i - 2], refs[i - 3]),
                            3 => Node::Sin(refs[i - 1]),
                            4 => Node::Cos(refs[i - 1]),
                            5 => Node::Normalize(refs[i - 1]),
                            6 => Node::Constant(i as f32 * 0.1),
                            7 => Node::Subtract(refs[i - 1], refs[i - 2]),
                            8 => Node::Dot(refs[i - 1], refs[i - 2]),
                            _ => Node::Clamp(refs[i - 1], refs[0], refs[1]),
                        };
                        refs.push(graph.add_node(node));
                    }

                    std_black_box(graph.node_count())
                });
            },
        );
    }

    // Test 3: Layered material (multiple sub-graphs blended)
    group.bench_function("layered_material_5_layers", |bencher| {
        bencher.iter(|| {
            let mut graph = MaterialGraph::new();
            let mut layer_outputs: Vec<NodeRef> = Vec::new();

            // Create 5 layers
            for layer in 0..5 {
                let base_color = graph.add_node(Node::Constant3([
                    layer as f32 * 0.2,
                    1.0 - layer as f32 * 0.2,
                    0.5,
                ]));

                let noise = graph.add_node(Node::Noise {
                    uv: base_color,
                    scale: 10.0 + layer as f32,
                    octaves: 4,
                });

                let blended = graph.add_node(Node::Multiply(base_color, noise));
                layer_outputs.push(blended);
            }

            // Blend layers together
            let mut result = layer_outputs[0];
            for &layer in layer_outputs.iter().skip(1) {
                let blend_factor = graph.add_node(Node::Constant(0.5));
                result = graph.add_node(Node::Lerp(result, layer, blend_factor));
            }

            let _ = graph.add_node(Node::OutputAlbedo(result));

            std_black_box(graph.node_count())
        });
    });

    // Test 4: Graph with many connections
    group.bench_function("highly_connected_graph_100", |bencher| {
        bencher.iter(|| {
            let mut graph = MaterialGraph::new();

            // Create 100 nodes
            let refs: Vec<NodeRef> = (0..100)
                .map(|i| graph.add_node(Node::Constant(i as f32 * 0.01)))
                .collect();

            // Create dense connections
            for i in 0..100 {
                for j in 0..i.min(5) {
                    graph.connect(refs[j], refs[i]);
                }
            }

            std_black_box(graph.connections.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: WGSL COMPILATION
// ============================================================================

fn bench_wgsl_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/wgsl_compilation");

    // Test 1: Compile simple material
    group.bench_function("compile_simple_material", |bencher| {
        let mut graph = MaterialGraph::new();
        let color = graph.add_node(Node::Constant4([1.0, 0.5, 0.0, 1.0]));
        let _ = graph.add_node(Node::OutputAlbedo(color));

        bencher.iter(|| {
            let wgsl = graph.compile_to_wgsl();
            std_black_box(wgsl.len())
        });
    });

    // Test 2: Compile complex material
    for node_count in [50, 100, 200] {
        group.throughput(Throughput::Elements(node_count as u64));

        group.bench_with_input(
            BenchmarkId::new("compile_complex_material", node_count),
            &node_count,
            |bencher, &count| {
                let mut graph = MaterialGraph::new();
                let mut refs: Vec<NodeRef> = Vec::new();

                refs.push(graph.add_node(Node::Constant3([1.0, 0.5, 0.0])));
                refs.push(graph.add_node(Node::Constant3([0.0, 0.5, 1.0])));
                refs.push(graph.add_node(Node::Time { speed: 1.0 }));

                for i in 3..count {
                    let node = match i % 8 {
                        0 => Node::Add(refs[i - 1], refs[i - 2]),
                        1 => Node::Multiply(refs[i - 1], refs[i - 2]),
                        2 => Node::Lerp(refs[i - 1], refs[i - 2], refs[i - 3]),
                        3 => Node::Sin(refs[i - 1]),
                        4 => Node::Cos(refs[i - 1]),
                        5 => Node::Normalize(refs[i - 1]),
                        6 => Node::Constant(i as f32 * 0.1),
                        _ => Node::Subtract(refs[i - 1], refs[i - 2]),
                    };
                    refs.push(graph.add_node(node));
                }

                let _ = graph.add_node(Node::OutputAlbedo(refs[count - 1]));

                bencher.iter(|| {
                    let wgsl = graph.compile_to_wgsl();
                    std_black_box(wgsl.len())
                });
            },
        );
    }

    // Test 3: Compile with many textures
    group.bench_function("compile_10_textures", |bencher| {
        let mut graph = MaterialGraph::new();

        let mut tex_refs: Vec<NodeRef> = Vec::new();
        for i in 0..10 {
            tex_refs.push(graph.add_node(Node::Texture2D {
                binding: i,
                uv_channel: 0,
            }));
        }

        // Blend all textures together
        let mut result = tex_refs[0];
        for &tex in tex_refs.iter().skip(1) {
            let blend = graph.add_node(Node::Constant(0.5));
            result = graph.add_node(Node::Lerp(result, tex, blend));
        }

        let _ = graph.add_node(Node::OutputAlbedo(result));

        bencher.iter(|| {
            let wgsl = graph.compile_to_wgsl();
            std_black_box(wgsl.len())
        });
    });

    // Test 4: Batch compilation
    group.bench_function("batch_compile_50_materials", |bencher| {
        let materials: Vec<MaterialGraph> = (0..50)
            .map(|i| {
                let mut g = MaterialGraph::new();
                let c = g.add_node(Node::Constant3([
                    i as f32 * 0.02,
                    0.5,
                    1.0 - i as f32 * 0.02,
                ]));
                let _ = g.add_node(Node::OutputAlbedo(c));
                g
            })
            .collect();

        bencher.iter(|| {
            let shaders: Vec<String> = materials.iter().map(|m| m.compile_to_wgsl()).collect();

            let total_len: usize = shaders.iter().map(|s| s.len()).sum();
            std_black_box(total_len)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: NODE EVALUATION
// ============================================================================

fn bench_node_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/node_evaluation");

    // Test 1: Constant node evaluation
    group.bench_function("constant_evaluation_1000", |bencher| {
        let nodes: Vec<Node> = (0..1000)
            .map(|i| Node::Constant(i as f32 * 0.001))
            .collect();

        bencher.iter(|| {
            let values: Vec<f32> = nodes
                .iter()
                .map(|n| if let Node::Constant(v) = n { *v } else { 0.0 })
                .collect();

            let sum: f32 = values.iter().sum();
            std_black_box(sum)
        });
    });

    // Test 2: Math operation chain
    group.bench_function("math_chain_evaluation_500", |bencher| {
        // Simulate evaluating a chain of math operations
        let operations: Vec<(&str, f32, f32)> = (0..500)
            .map(|i| {
                let op = match i % 4 {
                    0 => "add",
                    1 => "mul",
                    2 => "sub",
                    _ => "div",
                };
                (op, i as f32 * 0.01, (i + 1) as f32 * 0.01)
            })
            .collect();

        bencher.iter(|| {
            let mut result = 1.0f32;

            for (op, a, b) in &operations {
                result = match *op {
                    "add" => result + a + b,
                    "mul" => result * a.max(0.001) * b.max(0.001),
                    "sub" => result - a - b,
                    "div" => result / (a + b).max(0.001),
                    _ => result,
                };
            }

            std_black_box(result)
        });
    });

    // Test 3: Trig function evaluation
    group.bench_function("trig_evaluation_1000", |bencher| {
        let angles: Vec<f32> = (0..1000).map(|i| i as f32 * 0.01).collect();

        bencher.iter(|| {
            let results: Vec<[f32; 3]> = angles
                .iter()
                .map(|&a| [a.sin(), a.cos(), a.tan().clamp(-10.0, 10.0)])
                .collect();

            std_black_box(results.len())
        });
    });

    // Test 4: Vector operations
    group.bench_function("vector_operations_1000", |bencher| {
        let vectors: Vec<([f32; 3], [f32; 3])> = (0..1000)
            .map(|i| {
                let a = [i as f32 * 0.1, (i + 1) as f32 * 0.1, (i + 2) as f32 * 0.1];
                let b = [
                    (i + 3) as f32 * 0.1,
                    (i + 4) as f32 * 0.1,
                    (i + 5) as f32 * 0.1,
                ];
                (a, b)
            })
            .collect();

        bencher.iter(|| {
            let results: Vec<[f32; 3]> = vectors
                .iter()
                .map(|(a, b)| {
                    // Cross product
                    [
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ]
                })
                .collect();

            std_black_box(results.len())
        });
    });

    // Test 5: Fresnel evaluation
    group.bench_function("fresnel_evaluation_500", |bencher| {
        let data: Vec<([f32; 3], [f32; 3], f32)> = (0..500)
            .map(|i| {
                let normal = [0.0, 1.0, 0.0];
                let view = [(i as f32 * 0.01).sin(), (i as f32 * 0.01).cos(), 0.0];
                let power = 5.0;
                (normal, view, power)
            })
            .collect();

        bencher.iter(|| {
            let results: Vec<f32> = data
                .iter()
                .map(|(n, v, power)| {
                    // Fresnel-Schlick approximation
                    let n_dot_v = n[0] * v[0] + n[1] * v[1] + n[2] * v[2];
                    let clamped = n_dot_v.clamp(0.0, 1.0);
                    (1.0 - clamped).powf(*power)
                })
                .collect();

            std_black_box(results.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: MATERIAL INSTANCES
// ============================================================================

fn bench_material_instances(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/material_instances");

    // Test 1: Create material instances
    group.bench_function("create_instances_100", |bencher| {
        let base_graph = {
            let mut g = MaterialGraph::new();
            let c = g.add_node(Node::Constant3([1.0, 1.0, 1.0]));
            let _ = g.add_node(Node::OutputAlbedo(c));
            g
        };

        bencher.iter(|| {
            let instances: Vec<MaterialInstance> = (0..100)
                .map(|i| {
                    let mut params = HashMap::new();
                    params.insert(
                        "color".to_string(),
                        MaterialParameter::Vec3([i as f32 * 0.01, 1.0 - i as f32 * 0.01, 0.5]),
                    );
                    params.insert(
                        "metallic".to_string(),
                        MaterialParameter::Float(i as f32 * 0.01),
                    );
                    params.insert(
                        "roughness".to_string(),
                        MaterialParameter::Float(1.0 - i as f32 * 0.01),
                    );

                    MaterialInstance {
                        graph: base_graph.clone(),
                        parameters: params,
                    }
                })
                .collect();

            std_black_box(instances.len())
        });
    });

    // Test 2: Parameter updates
    group.bench_function("update_parameters_500", |bencher| {
        let mut instances: Vec<MaterialInstance> = (0..500)
            .map(|i| {
                let mut params = HashMap::new();
                params.insert(
                    "color".to_string(),
                    MaterialParameter::Vec3([1.0, 1.0, 1.0]),
                );
                params.insert(
                    "metallic".to_string(),
                    MaterialParameter::Float(i as f32 * 0.002),
                );

                MaterialInstance {
                    graph: MaterialGraph::new(),
                    parameters: params,
                }
            })
            .collect();

        bencher.iter(|| {
            for (i, instance) in instances.iter_mut().enumerate() {
                if let Some(MaterialParameter::Float(m)) = instance.parameters.get_mut("metallic") {
                    *m = (i as f32 * 0.002 + 0.1) % 1.0;
                }
                if let Some(MaterialParameter::Vec3(c)) = instance.parameters.get_mut("color") {
                    c[0] = (i as f32 * 0.01) % 1.0;
                }
            }

            std_black_box(instances.len())
        });
    });

    // Test 3: Parameter lookup by name
    group.bench_function("parameter_lookup_10000", |bencher| {
        let mut params: HashMap<String, MaterialParameter> = HashMap::new();
        for i in 0..100 {
            params.insert(
                format!("param_{}", i),
                MaterialParameter::Float(i as f32 * 0.01),
            );
        }

        let lookups: Vec<String> = (0..10000).map(|i| format!("param_{}", i % 100)).collect();

        bencher.iter(|| {
            let found: Vec<Option<&MaterialParameter>> =
                lookups.iter().map(|name| params.get(name)).collect();

            let count = found.iter().filter(|p| p.is_some()).count();
            std_black_box(count)
        });
    });

    // Test 4: Instance sorting by shader
    group.bench_function("sort_by_shader_1000", |bencher| {
        let instances: Vec<(u32, u64)> = (0..1000)
            .map(|i| {
                let shader_id = (i % 50) as u32;
                let instance_id = i as u64;
                (shader_id, instance_id)
            })
            .collect();

        bencher.iter(|| {
            let mut sorted = instances.clone();
            sorted.sort_by_key(|(shader, _)| *shader);

            // Group by shader
            let mut groups: HashMap<u32, Vec<u64>> = HashMap::new();
            for (shader, instance) in sorted {
                groups.entry(shader).or_default().push(instance);
            }

            std_black_box(groups.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: GRAPH OPTIMIZATION
// ============================================================================

fn bench_graph_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/graph_optimization");

    // Test 1: Dead code elimination
    group.bench_function("dead_code_elimination_200", |bencher| {
        // Create graph with dead nodes
        let nodes: Vec<(usize, bool, Vec<usize>)> = (0..200)
            .map(|i| {
                let is_used = i > 150 || i % 3 == 0;
                let deps: Vec<usize> = if i > 2 { vec![i - 1, i - 2] } else { vec![] };
                (i, is_used, deps)
            })
            .collect();

        bencher.iter(|| {
            // Mark reachable nodes
            let mut reachable = vec![false; nodes.len()];

            // Start from output nodes
            let mut stack: Vec<usize> = nodes
                .iter()
                .filter(|(_, used, _)| *used)
                .map(|(i, _, _)| *i)
                .collect();

            while let Some(idx) = stack.pop() {
                if !reachable[idx] {
                    reachable[idx] = true;
                    for &dep in &nodes[idx].2 {
                        stack.push(dep);
                    }
                }
            }

            let live_count = reachable.iter().filter(|&&r| r).count();
            std_black_box(live_count)
        });
    });

    // Test 2: Constant folding
    group.bench_function("constant_folding_100", |bencher| {
        let operations: Vec<(f32, &str, f32)> = (0..100)
            .map(|i| {
                let a = i as f32;
                let b = (i + 1) as f32;
                let op = match i % 4 {
                    0 => "+",
                    1 => "-",
                    2 => "*",
                    _ => "/",
                };
                (a, op, b)
            })
            .collect();

        bencher.iter(|| {
            let folded: Vec<f32> = operations
                .iter()
                .map(|(a, op, b)| match *op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b.max(0.001),
                    _ => *a,
                })
                .collect();

            std_black_box(folded.len())
        });
    });

    // Test 3: Common subexpression elimination
    group.bench_function("cse_detection_500", |bencher| {
        // Simulate expressions as (op, left, right) tuples
        let expressions: Vec<(u8, usize, usize)> = (0..500)
            .map(|i| {
                let op = (i % 4) as u8;
                let left = i % 50;
                let right = (i + 10) % 50;
                (op, left, right)
            })
            .collect();

        bencher.iter(|| {
            let mut seen: HashMap<(u8, usize, usize), usize> = HashMap::new();
            let mut replacements = 0;

            for (i, expr) in expressions.iter().enumerate() {
                if let Some(&existing) = seen.get(expr) {
                    // Common subexpression found
                    let _ = existing; // Would replace with reference
                    replacements += 1;
                } else {
                    seen.insert(*expr, i);
                }
            }

            std_black_box(replacements)
        });
    });

    // Test 4: Node reordering for GPU efficiency
    group.bench_function("gpu_reorder_200", |bencher| {
        let nodes: Vec<(usize, Vec<usize>, u32)> = (0..200)
            .map(|i| {
                let deps: Vec<usize> = if i > 2 { vec![i - 1, i - 2] } else { vec![] };
                let gpu_cost = (i % 5) as u32 + 1;
                (i, deps, gpu_cost)
            })
            .collect();

        bencher.iter(|| {
            // Topological sort with cost-based ordering
            let mut in_degree: Vec<usize> = vec![0; nodes.len()];
            for (_, deps, _) in &nodes {
                for &dep in deps {
                    in_degree[dep] += 1;
                }
            }

            let mut ready: Vec<(u32, usize)> = nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| in_degree[*i] == 0)
                .map(|(i, (_, _, cost))| (*cost, i))
                .collect();

            ready.sort_by_key(|(cost, _)| *cost);

            std_black_box(ready.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: TEXTURE BINDING
// ============================================================================

fn bench_texture_binding(c: &mut Criterion) {
    let mut group = c.benchmark_group("materials_adversarial/texture_binding");

    // Test 1: Texture slot assignment
    group.bench_function("slot_assignment_100", |bencher| {
        let textures: Vec<(String, u32, u32)> = (0..100)
            .map(|i| {
                let name = format!("texture_{}", i);
                let width = 1024 >> (i % 4);
                let height = 1024 >> (i % 4);
                (name, width, height)
            })
            .collect();

        let max_slots = 16u32;

        bencher.iter(|| {
            let mut bindings: HashMap<String, u32> = HashMap::new();
            let mut next_slot = 0u32;

            for (name, _, _) in &textures {
                if next_slot < max_slots {
                    bindings.insert(name.clone(), next_slot);
                    next_slot += 1;
                }
            }

            std_black_box(bindings.len())
        });
    });

    // Test 2: Binding layout generation
    group.bench_function("binding_layout_generation", |bencher| {
        let materials: Vec<Vec<u32>> = (0..50)
            .map(|i| (0..((i % 8) + 1)).map(|j| j as u32).collect())
            .collect();

        bencher.iter(|| {
            let layouts: Vec<String> = materials
                .iter()
                .map(|bindings| {
                    let mut layout = String::from("@group(1) struct MaterialBindings {\n");
                    for (i, &binding) in bindings.iter().enumerate() {
                        layout.push_str(&format!(
                            "    @binding({}) tex_{}: texture_2d<f32>,\n",
                            binding, i
                        ));
                    }
                    layout.push_str("}\n");
                    layout
                })
                .collect();

            let total_len: usize = layouts.iter().map(|l| l.len()).sum();
            std_black_box(total_len)
        });
    });

    // Test 3: Sampler deduplication
    group.bench_function("sampler_deduplication_200", |bencher| {
        let samplers: Vec<Filtering> = (0..200)
            .map(|i| match i % 4 {
                0 => Filtering::Point,
                1 => Filtering::Linear,
                2 => Filtering::Trilinear,
                _ => Filtering::Anisotropic((i % 16) as u8),
            })
            .collect();

        bencher.iter(|| {
            let mut unique: Vec<Filtering> = Vec::new();

            for sampler in &samplers {
                let is_unique = !unique.iter().any(|s| match (s, sampler) {
                    (Filtering::Point, Filtering::Point) => true,
                    (Filtering::Linear, Filtering::Linear) => true,
                    (Filtering::Trilinear, Filtering::Trilinear) => true,
                    (Filtering::Anisotropic(a), Filtering::Anisotropic(b)) => a == b,
                    _ => false,
                });

                if is_unique {
                    unique.push(*sampler);
                }
            }

            std_black_box(unique.len())
        });
    });

    // Test 4: Texture array packing
    group.bench_function("texture_array_packing_50", |bencher| {
        let textures: Vec<(u32, u32, u32)> = (0..50)
            .map(|i| {
                let size = 1 << ((i % 5) + 6); // 64 - 1024
                (size, size, i as u32)
            })
            .collect();

        bencher.iter(|| {
            // Group by size for array textures
            let mut groups: HashMap<(u32, u32), Vec<u32>> = HashMap::new();

            for (w, h, id) in &textures {
                groups.entry((*w, *h)).or_default().push(*id);
            }

            // Calculate array bindings needed
            let arrays: Vec<(u32, u32, usize)> = groups
                .iter()
                .map(|((w, h), ids)| (*w, *h, ids.len()))
                .collect();

            std_black_box(arrays.len())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_graph_construction,
    bench_wgsl_compilation,
    bench_node_evaluation,
    bench_material_instances,
    bench_graph_optimization,
    bench_texture_binding,
);

criterion_main!(benches);

//! Adversarial Asset Pipeline Benchmarks
//!
//! Stress testing for texture compression, mesh optimization, and validation.

#![allow(dead_code)]
#![allow(clippy::wrong_self_convention)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-asset-pipeline API)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
enum CompressionFormat {
    Bc7,
    Bc1,
    Astc4x4,
    Astc6x6,
    Astc8x8,
    None,
}

#[derive(Clone)]
struct TextureInput {
    width: u32,
    height: u32,
    data: Vec<u8>,
    has_alpha: bool,
}

#[derive(Clone)]
struct CompressedOutput {
    format: CompressionFormat,
    width: u32,
    height: u32,
    data: Vec<u8>,
    mip_levels: u32,
}

#[derive(Clone)]
struct MeshInput {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

#[derive(Clone)]
struct OptimizedMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    vertex_cache_miss_ratio: f32,
    overdraw_ratio: f32,
}

struct ValidationResult {
    is_valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn generate_texture(width: u32, height: u32) -> TextureInput {
    let pixel_count = (width * height) as usize;
    TextureInput {
        width,
        height,
        data: vec![128u8; pixel_count * 4], // RGBA
        has_alpha: true,
    }
}

fn generate_mesh(vertex_count: usize) -> MeshInput {
    MeshInput {
        positions: vec![[0.0, 0.0, 0.0]; vertex_count],
        normals: vec![[0.0, 1.0, 0.0]; vertex_count],
        uvs: vec![[0.0, 0.0]; vertex_count],
        indices: (0..vertex_count as u32).collect(),
    }
}

fn simulate_compression(input: &TextureInput, format: CompressionFormat) -> CompressedOutput {
    let block_size = match format {
        CompressionFormat::Bc7 | CompressionFormat::Bc1 => 4,
        CompressionFormat::Astc4x4 => 4,
        CompressionFormat::Astc6x6 => 6,
        CompressionFormat::Astc8x8 => 8,
        CompressionFormat::None => 1,
    };

    let blocks_x = input.width.div_ceil(block_size);
    let blocks_y = input.height.div_ceil(block_size);

    let bytes_per_block = match format {
        CompressionFormat::Bc7 => 16,
        CompressionFormat::Bc1 => 8,
        CompressionFormat::Astc4x4 | CompressionFormat::Astc6x6 | CompressionFormat::Astc8x8 => 16,
        CompressionFormat::None => block_size * block_size * 4,
    };

    let data_size = (blocks_x * blocks_y * bytes_per_block) as usize;
    let mip_levels = (input.width.max(input.height) as f32).log2().floor() as u32 + 1;

    CompressedOutput {
        format,
        width: input.width,
        height: input.height,
        data: vec![0u8; data_size],
        mip_levels,
    }
}

fn optimize_mesh(input: &MeshInput) -> OptimizedMesh {
    // Simulate vertex cache optimization (reorder indices)
    let optimized_indices = input.indices.clone();

    // Simple FIFO cache simulation
    let cache_size = 32usize;
    let mut cache: Vec<u32> = Vec::with_capacity(cache_size);
    let mut misses = 0;

    for &idx in &optimized_indices {
        if !cache.contains(&idx) {
            misses += 1;
            if cache.len() >= cache_size {
                cache.remove(0);
            }
            cache.push(idx);
        }
    }

    let miss_ratio = misses as f32 / optimized_indices.len() as f32;

    OptimizedMesh {
        positions: input.positions.clone(),
        normals: input.normals.clone(),
        uvs: input.uvs.clone(),
        indices: optimized_indices,
        vertex_cache_miss_ratio: miss_ratio,
        overdraw_ratio: 1.0, // Simplified
    }
}

// ============================================================================
// CATEGORY 1: TEXTURE COMPRESSION
// ============================================================================

fn bench_texture_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/texture_compression");

    // Test 1: Various texture sizes
    for size in [64, 256, 512, 1024] {
        group.throughput(Throughput::Bytes((size * size * 4) as u64));

        group.bench_with_input(
            BenchmarkId::new("bc7_compression", format!("{}x{}", size, size)),
            &size,
            |bencher, &size| {
                let texture = generate_texture(size, size);

                bencher.iter(|| {
                    let compressed = simulate_compression(&texture, CompressionFormat::Bc7);
                    std_black_box(compressed.data.len())
                });
            },
        );
    }

    // Test 2: ASTC variants
    for format in [
        CompressionFormat::Astc4x4,
        CompressionFormat::Astc6x6,
        CompressionFormat::Astc8x8,
    ] {
        group.bench_with_input(
            BenchmarkId::new("astc_compression", format!("{:?}", format)),
            &format,
            |bencher, &format| {
                let texture = generate_texture(512, 512);

                bencher.iter(|| {
                    let compressed = simulate_compression(&texture, format);
                    std_black_box(compressed.data.len())
                });
            },
        );
    }

    // Test 3: Non-power-of-two textures
    group.bench_function("npot_texture_300x200", |bencher| {
        let texture = generate_texture(300, 200);

        bencher.iter(|| {
            // Pad to power of two
            let padded_width = texture.width.next_power_of_two();
            let padded_height = texture.height.next_power_of_two();

            let mut padded_data = vec![0u8; (padded_width * padded_height * 4) as usize];
            for y in 0..texture.height {
                for x in 0..texture.width {
                    let src_idx = ((y * texture.width + x) * 4) as usize;
                    let dst_idx = ((y * padded_width + x) * 4) as usize;
                    if src_idx + 4 <= texture.data.len() && dst_idx + 4 <= padded_data.len() {
                        padded_data[dst_idx..dst_idx + 4]
                            .copy_from_slice(&texture.data[src_idx..src_idx + 4]);
                    }
                }
            }

            let padded = TextureInput {
                width: padded_width,
                height: padded_height,
                data: padded_data,
                has_alpha: texture.has_alpha,
            };

            let compressed = simulate_compression(&padded, CompressionFormat::Bc7);
            std_black_box(compressed.data.len())
        });
    });

    // Test 4: Mipmap chain generation
    group.bench_function("mipmap_generation_1024", |bencher| {
        let texture = generate_texture(1024, 1024);

        bencher.iter(|| {
            let mut mips = Vec::new();
            let mut w = texture.width;
            let mut h = texture.height;

            while w >= 1 && h >= 1 {
                let size = (w * h * 4) as usize;
                mips.push(vec![128u8; size]);

                if w == 1 && h == 1 {
                    break;
                }
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }

            std_black_box(mips.len())
        });
    });

    // Test 5: Quality preset selection
    group.bench_function("quality_preset_selection", |bencher| {
        let textures: Vec<TextureInput> = (0..100)
            .map(|i| generate_texture(2u32.pow((i % 6) as u32 + 5), 2u32.pow((i % 6) as u32 + 5)))
            .collect();

        bencher.iter(|| {
            let selections: Vec<CompressionFormat> = textures
                .iter()
                .map(|t| {
                    if t.has_alpha {
                        CompressionFormat::Bc7
                    } else if t.width * t.height > 512 * 512 {
                        CompressionFormat::Bc1
                    } else {
                        CompressionFormat::None
                    }
                })
                .collect();

            std_black_box(selections.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: MESH OPTIMIZATION
// ============================================================================

fn bench_mesh_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/mesh_optimization");

    // Test 1: Vertex cache optimization
    for vertex_count in [1000, 5000, 10000] {
        group.throughput(Throughput::Elements(vertex_count as u64));

        group.bench_with_input(
            BenchmarkId::new("vertex_cache_opt", vertex_count),
            &vertex_count,
            |bencher, &vertex_count| {
                let mesh = generate_mesh(vertex_count);

                bencher.iter(|| {
                    let optimized = optimize_mesh(&mesh);
                    std_black_box(optimized.vertex_cache_miss_ratio)
                });
            },
        );
    }

    // Test 2: Index buffer optimization
    group.bench_function("index_buffer_opt_10000", |bencher| {
        let indices: Vec<u32> = (0..10000).collect();

        bencher.iter(|| {
            // Simulate triangle strip optimization
            let mut strips = Vec::new();
            let mut current_strip = Vec::new();

            for tri in indices.chunks(3) {
                if tri.len() == 3 {
                    if current_strip.is_empty() {
                        current_strip.extend_from_slice(tri);
                    } else if current_strip.last() == Some(&tri[0]) {
                        current_strip.push(tri[1]);
                        current_strip.push(tri[2]);
                    } else {
                        strips.push(std::mem::take(&mut current_strip));
                        current_strip.extend_from_slice(tri);
                    }
                }
            }

            if !current_strip.is_empty() {
                strips.push(current_strip);
            }

            std_black_box(strips.len())
        });
    });

    // Test 3: Vertex deduplication
    group.bench_function("vertex_deduplication_5000", |bencher| {
        // Generate mesh with duplicates
        let positions: Vec<[f32; 3]> = (0..5000)
            .map(|i| [(i % 100) as f32, ((i / 100) % 10) as f32, (i / 1000) as f32])
            .collect();

        bencher.iter(|| {
            let mut unique_map: HashMap<[u32; 3], u32> = HashMap::new();
            let mut remap = Vec::with_capacity(positions.len());

            for pos in &positions {
                let key = [pos[0].to_bits(), pos[1].to_bits(), pos[2].to_bits()];

                let next_idx = unique_map.len() as u32;
                let idx = *unique_map.entry(key).or_insert(next_idx);
                remap.push(idx);
            }

            std_black_box((unique_map.len(), remap.len()))
        });
    });

    // Test 4: Overdraw optimization analysis
    group.bench_function("overdraw_analysis_1000_tris", |bencher| {
        let triangles: Vec<[[f32; 3]; 3]> = (0..1000)
            .map(|i| {
                let base = [(i % 10) as f32, ((i / 10) % 10) as f32, (i / 100) as f32];
                [
                    base,
                    [base[0] + 1.0, base[1], base[2]],
                    [base[0], base[1] + 1.0, base[2]],
                ]
            })
            .collect();

        bencher.iter(|| {
            // Simple front-to-back depth sorting
            let mut sorted = triangles.clone();
            sorted.sort_by(|a, b| {
                let depth_a = (a[0][2] + a[1][2] + a[2][2]) / 3.0;
                let depth_b = (b[0][2] + b[1][2] + b[2][2]) / 3.0;
                depth_a.partial_cmp(&depth_b).unwrap()
            });

            std_black_box(sorted.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: VALIDATION
// ============================================================================

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/validation");

    // Test 1: Texture validation
    group.bench_function("texture_validation_100", |bencher| {
        let textures: Vec<TextureInput> = (0..100).map(|_i| generate_texture(256, 256)).collect();

        bencher.iter(|| {
            let results: Vec<ValidationResult> = textures
                .iter()
                .map(|t| {
                    let mut errors = Vec::new();
                    let mut warnings = Vec::new();

                    if !t.width.is_power_of_two() {
                        warnings.push("Width not power of two".to_string());
                    }
                    if !t.height.is_power_of_two() {
                        warnings.push("Height not power of two".to_string());
                    }
                    if t.data.len() != (t.width * t.height * 4) as usize {
                        errors.push("Data size mismatch".to_string());
                    }
                    if t.width > 8192 || t.height > 8192 {
                        errors.push("Texture too large".to_string());
                    }

                    ValidationResult {
                        is_valid: errors.is_empty(),
                        errors,
                        warnings,
                    }
                })
                .collect();

            let valid_count = results.iter().filter(|r| r.is_valid).count();
            std_black_box(valid_count)
        });
    });

    // Test 2: Mesh validation
    group.bench_function("mesh_validation_50", |bencher| {
        let meshes: Vec<MeshInput> = (0..50).map(|_| generate_mesh(1000)).collect();

        bencher.iter(|| {
            let results: Vec<ValidationResult> = meshes
                .iter()
                .map(|m| {
                    let mut errors = Vec::new();
                    let mut warnings = Vec::new();

                    // Check index bounds
                    let max_vertex = m.positions.len() as u32;
                    for &idx in &m.indices {
                        if idx >= max_vertex {
                            errors.push(format!("Index {} out of bounds", idx));
                            break;
                        }
                    }

                    // Check attribute consistency
                    if m.positions.len() != m.normals.len() {
                        errors.push("Position/normal count mismatch".to_string());
                    }
                    if !m.uvs.is_empty() && m.uvs.len() != m.positions.len() {
                        errors.push("Position/UV count mismatch".to_string());
                    }

                    // Check for degenerate triangles
                    for tri in m.indices.chunks(3) {
                        if tri.len() == 3
                            && (tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2])
                        {
                            warnings.push("Degenerate triangle found".to_string());
                            break;
                        }
                    }

                    ValidationResult {
                        is_valid: errors.is_empty(),
                        errors,
                        warnings,
                    }
                })
                .collect();

            let valid_count = results.iter().filter(|r| r.is_valid).count();
            std_black_box(valid_count)
        });
    });

    // Test 3: Material reference validation
    group.bench_function("material_ref_validation", |bencher| {
        let mesh_materials: Vec<Vec<String>> = (0..100)
            .map(|_| {
                vec![
                    "material_0".to_string(),
                    "material_1".to_string(),
                    "missing_material".to_string(),
                ]
            })
            .collect();

        let available_materials: std::collections::HashSet<&str> =
            ["material_0", "material_1", "material_2"]
                .iter()
                .copied()
                .collect();

        bencher.iter(|| {
            let missing: Vec<(&String, &str)> = mesh_materials
                .iter()
                .flat_map(|materials| {
                    materials
                        .iter()
                        .filter(|m| !available_materials.contains(m.as_str()))
                        .map(move |m| (m, "mesh"))
                })
                .collect();

            std_black_box(missing.len())
        });
    });

    // Test 4: Animation validation
    group.bench_function("animation_validation_20", |bencher| {
        let animations: Vec<Vec<(f32, [f32; 3])>> = (0..20)
            .map(|_| {
                (0..100)
                    .map(|i| (i as f32 / 30.0, [(i as f32).sin(), 0.0, 0.0]))
                    .collect()
            })
            .collect();

        bencher.iter(|| {
            let results: Vec<bool> = animations
                .iter()
                .map(|anim| {
                    let mut valid = true;

                    // Check time ordering
                    for window in anim.windows(2) {
                        if window[0].0 >= window[1].0 {
                            valid = false;
                            break;
                        }
                    }

                    // Check for NaN values
                    for (_, value) in anim {
                        if value.iter().any(|v| v.is_nan()) {
                            valid = false;
                            break;
                        }
                    }

                    valid
                })
                .collect();

            let valid_count = results.iter().filter(|&&v| v).count();
            std_black_box(valid_count)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: BATCH PROCESSING
// ============================================================================

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/batch_processing");

    // Test 1: Parallel texture compression simulation
    group.bench_function("parallel_compression_20", |bencher| {
        let textures: Vec<TextureInput> = (0..20).map(|_| generate_texture(256, 256)).collect();

        bencher.iter(|| {
            let results: Vec<CompressedOutput> = textures
                .iter()
                .map(|t| simulate_compression(t, CompressionFormat::Bc7))
                .collect();

            let total_size: usize = results.iter().map(|r| r.data.len()).sum();
            std_black_box(total_size)
        });
    });

    // Test 2: Parallel mesh optimization
    group.bench_function("parallel_mesh_opt_20", |bencher| {
        let meshes: Vec<MeshInput> = (0..20).map(|_| generate_mesh(2000)).collect();

        bencher.iter(|| {
            let results: Vec<OptimizedMesh> = meshes.iter().map(optimize_mesh).collect();

            let avg_miss_ratio: f32 = results
                .iter()
                .map(|r| r.vertex_cache_miss_ratio)
                .sum::<f32>()
                / results.len() as f32;
            std_black_box(avg_miss_ratio)
        });
    });

    // Test 3: Pipeline scheduling
    group.bench_function("pipeline_scheduling_100", |bencher| {
        #[derive(Clone)]
        struct Task {
            id: u64,
            task_type: u8, // 0=texture, 1=mesh, 2=anim
            priority: u32,
            dependencies: Vec<u64>,
        }

        let tasks: Vec<Task> = (0..100)
            .map(|i| Task {
                id: i,
                task_type: (i % 3) as u8,
                priority: (100 - i) as u32,
                dependencies: if i > 0 { vec![i - 1] } else { vec![] },
            })
            .collect();

        bencher.iter(|| {
            let mut ready: Vec<&Task> = Vec::new();
            let mut completed: std::collections::HashSet<u64> = std::collections::HashSet::new();
            let mut pending: Vec<&Task> = tasks.iter().collect();

            while !pending.is_empty() {
                // Find tasks with satisfied dependencies
                let (can_run, still_pending): (Vec<&Task>, Vec<&Task>) = pending
                    .into_iter()
                    .partition(|t| t.dependencies.iter().all(|d| completed.contains(d)));

                ready.extend(can_run);
                pending = still_pending;

                // Sort by priority
                ready.sort_by_key(|t| std::cmp::Reverse(t.priority));

                // Execute highest priority
                if let Some(task) = ready.pop() {
                    completed.insert(task.id);
                }
            }

            std_black_box(completed.len())
        });
    });

    // Test 4: Memory budget tracking
    group.bench_function("memory_budget_tracking", |bencher| {
        let allocations: Vec<(u64, usize)> = (0..200)
            .map(|i| (i as u64, (i + 1) * 1024 * 1024)) // 1MB to 200MB
            .collect();

        let budget = 1024 * 1024 * 1024usize; // 1GB

        bencher.iter(|| {
            let mut used = 0usize;
            let mut allocated = Vec::new();

            for (id, size) in &allocations {
                if used + size <= budget {
                    allocated.push(*id);
                    used += size;
                } else {
                    // Need to evict
                    if let Some(&evict_id) = allocated.first() {
                        if let Some(pos) = allocations.iter().position(|(i, _)| *i == evict_id) {
                            used -= allocations[pos].1;
                            allocated.remove(0);
                            allocated.push(*id);
                            used += size;
                        }
                    }
                }
            }

            std_black_box(allocated.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: LOD GENERATION
// ============================================================================

fn bench_lod_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/lod_generation");

    // Test 1: Quadric error metrics
    group.bench_function("quadric_error_5000_verts", |bencher| {
        let positions: Vec<[f32; 3]> = (0..5000)
            .map(|i| [(i % 50) as f32, ((i / 50) % 50) as f32, (i / 2500) as f32])
            .collect();

        bencher.iter(|| {
            // Simplified quadric error computation
            let mut errors: Vec<f32> = Vec::with_capacity(positions.len());

            for i in 0..positions.len() {
                let mut error = 0.0f32;

                // Check neighbors (simplified)
                for j in 0..positions.len().min(10) {
                    if i != j {
                        let dx = positions[i][0] - positions[j][0];
                        let dy = positions[i][1] - positions[j][1];
                        let dz = positions[i][2] - positions[j][2];
                        error += (dx * dx + dy * dy + dz * dz).sqrt();
                    }
                }

                errors.push(error);
            }

            std_black_box(errors.iter().sum::<f32>() / errors.len() as f32)
        });
    });

    // Test 2: Edge collapse selection
    group.bench_function("edge_collapse_selection", |bencher| {
        let edges: Vec<(u32, u32, f32)> = (0..10000)
            .map(|i| (i as u32, (i + 1) as u32, (i as f32 * 0.01).sin().abs()))
            .collect();

        bencher.iter(|| {
            let mut sorted = edges.clone();
            sorted.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

            // Select lowest error edges for collapse
            let to_collapse: Vec<_> = sorted.iter().take(100).collect();
            std_black_box(to_collapse.len())
        });
    });

    // Test 3: LOD level selection
    group.bench_function("lod_level_generation_4", |bencher| {
        let base_mesh = generate_mesh(10000);
        let target_ratios = [1.0f32, 0.5, 0.25, 0.125]; // LOD 0-3

        bencher.iter(|| {
            let lod_meshes: Vec<MeshInput> = target_ratios
                .iter()
                .map(|&ratio| {
                    let target_count = (base_mesh.indices.len() as f32 * ratio) as usize;
                    MeshInput {
                        positions: base_mesh.positions
                            [..target_count.min(base_mesh.positions.len())]
                            .to_vec(),
                        normals: base_mesh.normals[..target_count.min(base_mesh.normals.len())]
                            .to_vec(),
                        uvs: base_mesh.uvs[..target_count.min(base_mesh.uvs.len())].to_vec(),
                        indices: base_mesh.indices[..target_count.min(base_mesh.indices.len())]
                            .to_vec(),
                    }
                })
                .collect();

            let total_verts: usize = lod_meshes.iter().map(|m| m.positions.len()).sum();
            std_black_box(total_verts)
        });
    });

    // Test 4: Screen-space error calculation
    group.bench_function("screen_space_error_100", |bencher| {
        let objects: Vec<(f32, f32)> = (0..100)
            .map(|i| {
                (
                    i as f32 * 10.0,     // distance
                    (i % 10 + 1) as f32, // bounding radius
                )
            })
            .collect();

        let screen_height = 1080.0f32;
        let fov = 60.0f32.to_radians();

        bencher.iter(|| {
            let errors: Vec<f32> = objects
                .iter()
                .map(|&(distance, radius)| {
                    (radius / distance) * (screen_height / (2.0 * (fov / 2.0).tan()))
                })
                .collect();

            std_black_box(errors.iter().sum::<f32>())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: FORMAT CONVERSION
// ============================================================================

fn bench_format_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_adversarial/format_conversion");

    // Test 1: Vertex format conversion
    group.bench_function("vertex_format_conversion_10000", |bencher| {
        let float_positions: Vec<[f32; 3]> = (0..10000)
            .map(|i| [(i as f32).sin(), (i as f32).cos(), i as f32 * 0.01])
            .collect();

        bencher.iter(|| {
            // Convert to half-float (simulated)
            let half_positions: Vec<[u16; 3]> = float_positions
                .iter()
                .map(|pos| {
                    [
                        half::f16::from_f32(pos[0]).to_bits(),
                        half::f16::from_f32(pos[1]).to_bits(),
                        half::f16::from_f32(pos[2]).to_bits(),
                    ]
                })
                .collect();

            std_black_box(half_positions.len())
        });
    });

    // Test 2: Normal compression (octahedral)
    group.bench_function("normal_octahedral_10000", |bencher| {
        let normals: Vec<[f32; 3]> = (0..10000)
            .map(|i| {
                let x = (i as f32 * 0.1).sin();
                let y = (i as f32 * 0.1).cos();
                let z = ((i as f32 * 0.2).sin() + 1.0) * 0.5;
                let len = (x * x + y * y + z * z).sqrt();
                [x / len, y / len, z / len]
            })
            .collect();

        bencher.iter(|| {
            // Octahedral encoding
            let encoded: Vec<[i16; 2]> = normals
                .iter()
                .map(|n| {
                    let l1 = n[0].abs() + n[1].abs() + n[2].abs();
                    let mut enc = [n[0] / l1, n[1] / l1];

                    if n[2] < 0.0 {
                        enc[0] = (1.0 - enc[1].abs()) * enc[0].signum();
                        enc[1] = (1.0 - enc[0].abs()) * enc[1].signum();
                    }

                    [(enc[0] * 32767.0) as i16, (enc[1] * 32767.0) as i16]
                })
                .collect();

            std_black_box(encoded.len())
        });
    });

    // Test 3: Index buffer compression (delta encoding)
    group.bench_function("index_delta_encoding_30000", |bencher| {
        let indices: Vec<u32> = (0..30000).collect();

        bencher.iter(|| {
            let deltas: Vec<i32> = indices
                .windows(2)
                .map(|w| w[1] as i32 - w[0] as i32)
                .collect();

            // Simulate varint size calculation
            let bytes: usize = deltas
                .iter()
                .map(|&d| {
                    let abs = d.unsigned_abs();
                    if abs < 128 {
                        1
                    } else if abs < 16384 {
                        2
                    } else {
                        4
                    }
                })
                .sum();

            std_black_box(bytes)
        });
    });

    // Test 4: Tangent frame compression
    group.bench_function("tangent_compression_5000", |bencher| {
        let tangents: Vec<[f32; 4]> = (0..5000)
            .map(|i| {
                let x = (i as f32 * 0.1).sin();
                let y = (i as f32 * 0.1).cos();
                let z = 0.0f32;
                let len = (x * x + y * y + z * z).sqrt();
                [x / len, y / len, z / len, 1.0]
            })
            .collect();

        bencher.iter(|| {
            // Compress to quaternion-based representation
            let compressed: Vec<u32> = tangents
                .iter()
                .map(|t| {
                    let qx = ((t[0] * 0.5 + 0.5) * 1023.0) as u32;
                    let qy = ((t[1] * 0.5 + 0.5) * 1023.0) as u32;
                    let qz = ((t[2] * 0.5 + 0.5) * 1023.0) as u32;
                    let sign = if t[3] >= 0.0 { 1u32 } else { 0u32 };

                    (qx & 0x3FF) | ((qy & 0x3FF) << 10) | ((qz & 0x3FF) << 20) | (sign << 30)
                })
                .collect();

            std_black_box(compressed.len())
        });
    });

    group.finish();
}

// Stub for half-float
mod half {
    #[allow(non_camel_case_types)]
    pub struct f16(u16);
    impl f16 {
        pub fn from_f32(v: f32) -> Self {
            // Simplified conversion
            f16((v * 1000.0) as u16)
        }
        pub fn to_bits(self) -> u16 {
            self.0
        }
    }
}

criterion_group!(
    benches,
    bench_texture_compression,
    bench_mesh_optimization,
    bench_validation,
    bench_batch_processing,
    bench_lod_generation,
    bench_format_conversion,
);

criterion_main!(benches);

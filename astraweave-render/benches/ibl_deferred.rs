//! IBL (Image-Based Lighting) and Deferred Rendering Benchmarks
//!
//! Comprehensive CPU-side benchmarks measuring algorithm complexity for:
//! - Spherical harmonics computation (SH9, SH16, SH25)
//! - Cubemap direction sampling
//! - IBL prefilter kernel generation
//! - GGX importance sampling
//! - G-buffer packing/unpacking operations
//! - Deferred light accumulation
//! - BRDF LUT generation

use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion, SamplingMode,
};
use std::f32::consts::PI;
use std::hint::black_box;
use std::time::Duration;

// Run 39100674332 shard 14 killed this target at 600s. Same flat sample
// budget as graph_mesh_material_texture. Every function still runs.
fn ci_budget<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(1));
    group.measurement_time(Duration::from_secs(1));
    group
}

// =============================================================================

type GBufferTestDatum = ([f32; 3], [f32; 3], f32, f32, f32);
type PixelDatum = ([f32; 3], [f32; 3], f32, f32, [f32; 3]);
// SPHERICAL HARMONICS BENCHMARKS
// =============================================================================

/// Generate SH coefficients for a given number of bands
/// SH9 = 3 bands (9 coefficients), SH16 = 4 bands, SH25 = 5 bands
fn generate_sh_coefficients(bands: u32) -> Vec<[f32; 3]> {
    let num_coeffs = (bands * bands) as usize;
    let mut coeffs = vec![[0.0f32; 3]; num_coeffs];

    // Sample hemisphere with Monte Carlo integration
    let samples = 1024u32;
    let phi_step = 2.0 * PI / samples as f32;
    let theta_step = PI / (samples / 2) as f32;

    for i in 0..samples {
        let phi = i as f32 * phi_step;
        for j in 0..(samples / 2) {
            let theta = j as f32 * theta_step;

            // Direction from spherical coordinates
            let dir = [
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            ];

            // Sample color (simulate environment sampling)
            let color = [
                (dir[0] + 1.0) * 0.5,
                (dir[1] + 1.0) * 0.5,
                (dir[2] + 1.0) * 0.5,
            ];

            // Evaluate SH basis functions and accumulate
            let solid_angle = theta.sin() * phi_step * theta_step;
            let basis = evaluate_sh_basis(bands, &dir);

            for (c, &b) in coeffs.iter_mut().zip(basis.iter()) {
                c[0] += color[0] * b * solid_angle;
                c[1] += color[1] * b * solid_angle;
                c[2] += color[2] * b * solid_angle;
            }
        }
    }

    coeffs
}

/// Evaluate SH basis functions up to given number of bands
fn evaluate_sh_basis(bands: u32, dir: &[f32; 3]) -> Vec<f32> {
    let num_coeffs = (bands * bands) as usize;
    let mut basis = vec![0.0f32; num_coeffs];

    let x = dir[0];
    let y = dir[1];
    let z = dir[2];

    // Band 0 (1 coefficient)
    if bands >= 1 {
        basis[0] = 0.282095; // Y_0^0 = 1 / (2*sqrt(pi))
    }

    // Band 1 (3 coefficients)
    if bands >= 2 {
        basis[1] = 0.488603 * y; // Y_1^-1
        basis[2] = 0.488603 * z; // Y_1^0
        basis[3] = 0.488603 * x; // Y_1^1
    }

    // Band 2 (5 coefficients)
    if bands >= 3 {
        basis[4] = 1.092548 * x * y; // Y_2^-2
        basis[5] = 1.092548 * y * z; // Y_2^-1
        basis[6] = 0.315392 * (3.0 * z * z - 1.0); // Y_2^0
        basis[7] = 1.092548 * x * z; // Y_2^1
        basis[8] = 0.546274 * (x * x - y * y); // Y_2^2
    }

    // Band 3 (7 coefficients)
    if bands >= 4 {
        basis[9] = 0.590044 * y * (3.0 * x * x - y * y);
        basis[10] = 2.890611 * x * y * z;
        basis[11] = 0.457046 * y * (5.0 * z * z - 1.0);
        basis[12] = 0.373176 * z * (5.0 * z * z - 3.0);
        basis[13] = 0.457046 * x * (5.0 * z * z - 1.0);
        basis[14] = 1.445306 * z * (x * x - y * y);
        basis[15] = 0.590044 * x * (x * x - 3.0 * y * y);
    }

    // Band 4 (9 coefficients)
    if bands >= 5 {
        let x2 = x * x;
        let y2 = y * y;
        let z2 = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;

        basis[16] = 2.503343 * xy * (x2 - y2);
        basis[17] = 1.770131 * yz * (3.0 * x2 - y2);
        basis[18] = 0.946175 * xy * (7.0 * z2 - 1.0);
        basis[19] = 0.669047 * yz * (7.0 * z2 - 3.0);
        basis[20] = 0.105786 * (35.0 * z2 * z2 - 30.0 * z2 + 3.0);
        basis[21] = 0.669047 * xz * (7.0 * z2 - 3.0);
        basis[22] = 0.473087 * (x2 - y2) * (7.0 * z2 - 1.0);
        basis[23] = 1.770131 * xz * (x2 - 3.0 * y2);
        basis[24] = 0.625836 * (x2 * (x2 - 3.0 * y2) - y2 * (3.0 * x2 - y2));
    }

    basis
}

/// Reconstruct irradiance from SH coefficients
fn reconstruct_irradiance_sh(coeffs: &[[f32; 3]], dir: &[f32; 3]) -> [f32; 3] {
    let bands = (coeffs.len() as f32).sqrt() as u32;
    let basis = evaluate_sh_basis(bands, dir);

    let mut result = [0.0f32; 3];
    for (coeff, &b) in coeffs.iter().zip(basis.iter()) {
        result[0] += coeff[0] * b;
        result[1] += coeff[1] * b;
        result[2] += coeff[2] * b;
    }
    result
}

fn bench_spherical_harmonics(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/spherical_harmonics");
    group.sample_size(50);

    // Benchmark SH coefficient generation for different band counts
    for bands in [3u32, 4, 5] {
        let label = format!("SH{}", bands * bands);
        group.bench_with_input(
            BenchmarkId::new("generate_coefficients", &label),
            &bands,
            |b, &bands| b.iter(|| black_box(generate_sh_coefficients(bands))),
        );
    }

    // Benchmark SH basis evaluation
    let dir = [0.577f32, 0.577, 0.577]; // Normalized direction
    for bands in [3u32, 4, 5] {
        let label = format!("SH{}", bands * bands);
        group.bench_with_input(
            BenchmarkId::new("evaluate_basis", &label),
            &bands,
            |b, &bands| b.iter(|| black_box(evaluate_sh_basis(bands, &dir))),
        );
    }

    // Benchmark irradiance reconstruction
    let coeffs_sh9 = generate_sh_coefficients(3);
    let coeffs_sh16 = generate_sh_coefficients(4);
    let coeffs_sh25 = generate_sh_coefficients(5);

    group.bench_function("reconstruct_irradiance_sh9", |b| {
        b.iter(|| black_box(reconstruct_irradiance_sh(&coeffs_sh9, &dir)))
    });

    group.bench_function("reconstruct_irradiance_sh16", |b| {
        b.iter(|| black_box(reconstruct_irradiance_sh(&coeffs_sh16, &dir)))
    });

    group.bench_function("reconstruct_irradiance_sh25", |b| {
        b.iter(|| black_box(reconstruct_irradiance_sh(&coeffs_sh25, &dir)))
    });

    group.finish();
}

// =============================================================================
// CUBEMAP SAMPLING BENCHMARKS
// =============================================================================

/// Convert direction to cubemap face and UV coordinates
fn direction_to_cubemap_uv(dir: [f32; 3]) -> (u32, [f32; 2]) {
    let abs_dir = [dir[0].abs(), dir[1].abs(), dir[2].abs()];

    let (face, uv) = if abs_dir[0] >= abs_dir[1] && abs_dir[0] >= abs_dir[2] {
        if dir[0] > 0.0 {
            (
                0,
                [
                    (-dir[2] / abs_dir[0] + 1.0) * 0.5,
                    (-dir[1] / abs_dir[0] + 1.0) * 0.5,
                ],
            )
        } else {
            (
                1,
                [
                    (dir[2] / abs_dir[0] + 1.0) * 0.5,
                    (-dir[1] / abs_dir[0] + 1.0) * 0.5,
                ],
            )
        }
    } else if abs_dir[1] >= abs_dir[0] && abs_dir[1] >= abs_dir[2] {
        if dir[1] > 0.0 {
            (
                2,
                [
                    (dir[0] / abs_dir[1] + 1.0) * 0.5,
                    (dir[2] / abs_dir[1] + 1.0) * 0.5,
                ],
            )
        } else {
            (
                3,
                [
                    (dir[0] / abs_dir[1] + 1.0) * 0.5,
                    (-dir[2] / abs_dir[1] + 1.0) * 0.5,
                ],
            )
        }
    } else if dir[2] > 0.0 {
        (
            4,
            [
                (dir[0] / abs_dir[2] + 1.0) * 0.5,
                (-dir[1] / abs_dir[2] + 1.0) * 0.5,
            ],
        )
    } else {
        (
            5,
            [
                (-dir[0] / abs_dir[2] + 1.0) * 0.5,
                (-dir[1] / abs_dir[2] + 1.0) * 0.5,
            ],
        )
    };

    (face, uv)
}

/// Sample cubemap with bilinear interpolation (simulated)
fn sample_cubemap_bilinear(dir: [f32; 3], mip_size: u32) -> [f32; 4] {
    let (face, uv) = direction_to_cubemap_uv(dir);

    // Bilinear sample coordinates
    let u = uv[0] * (mip_size - 1) as f32;
    let v = uv[1] * (mip_size - 1) as f32;

    let u0 = u.floor() as u32;
    let v0 = v.floor() as u32;
    let u1 = (u0 + 1).min(mip_size - 1);
    let v1 = (v0 + 1).min(mip_size - 1);

    let fu = u.fract();
    let fv = v.fract();

    // Simulate texture fetch with hash-based pseudo-random
    let sample = |_face: u32, _u: u32, _v: u32| -> [f32; 4] {
        // Placeholder color based on UV
        [0.5, 0.5, 0.5, 1.0]
    };

    let s00 = sample(face, u0, v0);
    let s10 = sample(face, u1, v0);
    let s01 = sample(face, u0, v1);
    let s11 = sample(face, u1, v1);

    // Bilinear interpolation
    [
        s00[0] * (1.0 - fu) * (1.0 - fv)
            + s10[0] * fu * (1.0 - fv)
            + s01[0] * (1.0 - fu) * fv
            + s11[0] * fu * fv,
        s00[1] * (1.0 - fu) * (1.0 - fv)
            + s10[1] * fu * (1.0 - fv)
            + s01[1] * (1.0 - fu) * fv
            + s11[1] * fu * fv,
        s00[2] * (1.0 - fu) * (1.0 - fv)
            + s10[2] * fu * (1.0 - fv)
            + s01[2] * (1.0 - fu) * fv
            + s11[2] * fu * fv,
        s00[3] * (1.0 - fu) * (1.0 - fv)
            + s10[3] * fu * (1.0 - fv)
            + s01[3] * (1.0 - fu) * fv
            + s11[3] * fu * fv,
    ]
}

fn bench_cubemap_sampling(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/cubemap_sampling");

    // Benchmark direction to UV conversion
    let dirs: Vec<[f32; 3]> = (0..1000)
        .map(|i| {
            let t = i as f32 * 0.001;
            let phi = t * 2.0 * PI * 7.0;
            let theta = t * PI;
            [
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            ]
        })
        .collect();

    group.bench_function("direction_to_uv_1k", |b| {
        b.iter(|| {
            for dir in &dirs {
                black_box(direction_to_cubemap_uv(*dir));
            }
        })
    });

    // Benchmark bilinear sampling at different mip levels
    for mip_size in [64u32, 128, 256, 512, 1024] {
        group.bench_with_input(
            BenchmarkId::new("bilinear_sample", format!("{}x{}", mip_size, mip_size)),
            &mip_size,
            |b, &size| {
                let dir = [0.577f32, 0.577, 0.577];
                b.iter(|| black_box(sample_cubemap_bilinear(dir, size)))
            },
        );
    }

    // Benchmark batch cubemap sampling (typical for IBL integration)
    group.bench_function("batch_sample_1k_directions", |b| {
        b.iter(|| {
            for dir in &dirs {
                black_box(sample_cubemap_bilinear(*dir, 512));
            }
        })
    });

    group.finish();
}

// =============================================================================
// GGX IMPORTANCE SAMPLING BENCHMARKS
// =============================================================================

/// Generate GGX importance sample direction using Hammersley sequence
fn ggx_importance_sample(xi: [f32; 2], roughness: f32, normal: [f32; 3]) -> [f32; 3] {
    let a = roughness * roughness;

    let phi = 2.0 * PI * xi[0];
    let cos_theta = ((1.0 - xi[1]) / (1.0 + (a * a - 1.0) * xi[1])).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    // Spherical to Cartesian (tangent space)
    let h_tangent = [sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta];

    // Build TBN from normal
    let up = if normal[2].abs() < 0.999 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let tangent = cross(&up, &normal);
    let tangent = normalize(&tangent);
    let bitangent = cross(&normal, &tangent);

    // Transform to world space
    [
        tangent[0] * h_tangent[0] + bitangent[0] * h_tangent[1] + normal[0] * h_tangent[2],
        tangent[1] * h_tangent[0] + bitangent[1] * h_tangent[1] + normal[1] * h_tangent[2],
        tangent[2] * h_tangent[0] + bitangent[2] * h_tangent[1] + normal[2] * h_tangent[2],
    ]
}

/// Van der Corput sequence (for Hammersley)
fn van_der_corput(mut n: u32, base: u32) -> f32 {
    let mut result = 0.0f32;
    let mut denom = 1.0f32;

    while n > 0 {
        denom *= base as f32;
        result += (n % base) as f32 / denom;
        n /= base;
    }

    result
}

/// Hammersley sequence point
fn hammersley_point(i: u32, n: u32) -> [f32; 2] {
    [i as f32 / n as f32, van_der_corput(i, 2)]
}

/// Generate prefilter sample kernel
fn generate_prefilter_kernel(roughness: f32, sample_count: u32) -> Vec<[f32; 3]> {
    let normal = [0.0f32, 0.0, 1.0];
    (0..sample_count)
        .map(|i| {
            let xi = hammersley_point(i, sample_count);
            ggx_importance_sample(xi, roughness, normal)
        })
        .collect()
}

/// Cross product
fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Normalize vector
fn normalize(v: &[f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn bench_ggx_sampling(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/ggx_importance_sampling");

    // Benchmark Hammersley sequence generation
    for sample_count in [64u32, 256, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("hammersley_sequence", sample_count),
            &sample_count,
            |b, &count| {
                b.iter(|| {
                    for i in 0..count {
                        black_box(hammersley_point(i, count));
                    }
                })
            },
        );
    }

    // Benchmark GGX importance sample generation
    let xi = [0.5f32, 0.5];
    let normal = [0.0f32, 0.0, 1.0];

    for roughness in [0.1f32, 0.25, 0.5, 0.75, 1.0] {
        group.bench_with_input(
            BenchmarkId::new("single_sample", format!("r{:.2}", roughness)),
            &roughness,
            |b, &r| b.iter(|| black_box(ggx_importance_sample(xi, r, normal))),
        );
    }

    // Benchmark full kernel generation (typical for specular prefilter)
    for sample_count in [64u32, 256, 1024] {
        for roughness in [0.25f32, 0.5, 0.75] {
            let label = format!("n{}_r{:.2}", sample_count, roughness);
            group.bench_with_input(
                BenchmarkId::new("prefilter_kernel", &label),
                &(sample_count, roughness),
                |b, &(count, r)| b.iter(|| black_box(generate_prefilter_kernel(r, count))),
            );
        }
    }

    group.finish();
}

// =============================================================================
// G-BUFFER PACKING/UNPACKING BENCHMARKS
// =============================================================================

/// Pack normal using octahedral encoding (2 floats -> 2 half floats)
fn pack_normal_octahedral(n: [f32; 3]) -> [u16; 2] {
    let sum = n[0].abs() + n[1].abs() + n[2].abs();
    let mut oct = [n[0] / sum, n[1] / sum];

    if n[2] < 0.0 {
        let sign_x = if oct[0] >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if oct[1] >= 0.0 { 1.0 } else { -1.0 };
        oct = [(1.0 - oct[1].abs()) * sign_x, (1.0 - oct[0].abs()) * sign_y];
    }

    // Encode to R16G16
    [
        ((oct[0] * 0.5 + 0.5) * 65535.0) as u16,
        ((oct[1] * 0.5 + 0.5) * 65535.0) as u16,
    ]
}

/// Unpack normal from octahedral encoding
fn unpack_normal_octahedral(packed: [u16; 2]) -> [f32; 3] {
    let oct = [
        packed[0] as f32 / 65535.0 * 2.0 - 1.0,
        packed[1] as f32 / 65535.0 * 2.0 - 1.0,
    ];

    let z = 1.0 - oct[0].abs() - oct[1].abs();
    let n = if z >= 0.0 {
        [oct[0], oct[1], z]
    } else {
        let sign_x = if oct[0] >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if oct[1] >= 0.0 { 1.0 } else { -1.0 };
        [
            (1.0 - oct[1].abs()) * sign_x,
            (1.0 - oct[0].abs()) * sign_y,
            z,
        ]
    };

    normalize(&n)
}

/// Pack G-buffer pixel (albedo, normal, roughness, metallic, ao)
fn pack_gbuffer_pixel(
    albedo: [f32; 3],
    normal: [f32; 3],
    roughness: f32,
    metallic: f32,
    ao: f32,
) -> GBufferPixel {
    GBufferPixel {
        albedo_roughness: [
            (albedo[0] * 255.0) as u8,
            (albedo[1] * 255.0) as u8,
            (albedo[2] * 255.0) as u8,
            (roughness * 255.0) as u8,
        ],
        normal_metallic: pack_normal_octahedral(normal),
        metallic_ao: [(metallic * 255.0) as u8, (ao * 255.0) as u8],
    }
}

/// Unpack G-buffer pixel
fn unpack_gbuffer_pixel(pixel: &GBufferPixel) -> ([f32; 3], [f32; 3], f32, f32, f32) {
    let albedo = [
        pixel.albedo_roughness[0] as f32 / 255.0,
        pixel.albedo_roughness[1] as f32 / 255.0,
        pixel.albedo_roughness[2] as f32 / 255.0,
    ];
    let roughness = pixel.albedo_roughness[3] as f32 / 255.0;
    let normal = unpack_normal_octahedral(pixel.normal_metallic);
    let metallic = pixel.metallic_ao[0] as f32 / 255.0;
    let ao = pixel.metallic_ao[1] as f32 / 255.0;

    (albedo, normal, roughness, metallic, ao)
}

#[derive(Clone, Copy)]
struct GBufferPixel {
    albedo_roughness: [u8; 4], // RGB + roughness
    normal_metallic: [u16; 2], // Octahedral normal
    metallic_ao: [u8; 2],      // Metallic + AO
}

fn bench_gbuffer_operations(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/gbuffer_operations");

    // Benchmark octahedral normal packing
    let normals: Vec<[f32; 3]> = (0..1000)
        .map(|i| {
            let t = i as f32 * 0.001;
            let phi = t * 2.0 * PI * 7.0;
            let theta = t * PI;
            normalize(&[
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            ])
        })
        .collect();

    group.bench_function("pack_normal_octahedral_1k", |b| {
        b.iter(|| {
            for n in &normals {
                black_box(pack_normal_octahedral(*n));
            }
        })
    });

    // Benchmark octahedral normal unpacking
    let packed_normals: Vec<[u16; 2]> =
        normals.iter().map(|n| pack_normal_octahedral(*n)).collect();

    group.bench_function("unpack_normal_octahedral_1k", |b| {
        b.iter(|| {
            for p in &packed_normals {
                black_box(unpack_normal_octahedral(*p));
            }
        })
    });

    // Benchmark full G-buffer pixel packing
    let test_data: Vec<GBufferTestDatum> = (0..1000)
        .map(|i| {
            let t = i as f32 * 0.001;
            (
                [t, 1.0 - t, t * 0.5], // albedo
                normals[i as usize],   // normal
                t,                     // roughness
                1.0 - t,               // metallic
                0.5 + t * 0.5,         // ao
            )
        })
        .collect();

    group.bench_function("pack_gbuffer_pixel_1k", |b| {
        b.iter(|| {
            for (albedo, normal, roughness, metallic, ao) in &test_data {
                black_box(pack_gbuffer_pixel(
                    *albedo, *normal, *roughness, *metallic, *ao,
                ));
            }
        })
    });

    // Benchmark G-buffer unpacking
    let packed_pixels: Vec<GBufferPixel> = test_data
        .iter()
        .map(|(albedo, normal, roughness, metallic, ao)| {
            pack_gbuffer_pixel(*albedo, *normal, *roughness, *metallic, *ao)
        })
        .collect();

    group.bench_function("unpack_gbuffer_pixel_1k", |b| {
        b.iter(|| {
            for p in &packed_pixels {
                black_box(unpack_gbuffer_pixel(p));
            }
        })
    });

    // Benchmark at different resolutions (simulated frame sizes)
    for res in [(1280u32, 720u32), (1920, 1080), (2560, 1440), (3840, 2160)] {
        let pixel_count = res.0 * res.1;
        let label = format!("{}x{}", res.0, res.1);

        // Just measure iteration overhead for pixel count
        group.bench_with_input(
            BenchmarkId::new("gbuffer_fill_simulation", &label),
            &pixel_count,
            |b, &count| {
                let pixel = pack_gbuffer_pixel([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], 0.5, 0.0, 1.0);
                b.iter(|| {
                    for _ in 0..count {
                        black_box(pixel);
                    }
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// BRDF LUT GENERATION BENCHMARKS
// =============================================================================

/// Geometry function (Schlick-GGX)
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let a = roughness;
    let k = (a * a) / 2.0;
    n_dot_v / (n_dot_v * (1.0 - k) + k)
}

/// Smith geometry function
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx1 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx2 = geometry_schlick_ggx(n_dot_l, roughness);
    ggx1 * ggx2
}

/// Generate BRDF LUT sample (split-sum approximation)
fn integrate_brdf(n_dot_v: f32, roughness: f32, sample_count: u32) -> [f32; 2] {
    let v = [(1.0 - n_dot_v * n_dot_v).sqrt(), 0.0, n_dot_v];
    let n = [0.0f32, 0.0, 1.0];

    let mut a = 0.0f32;
    let mut b = 0.0f32;

    for i in 0..sample_count {
        let xi = hammersley_point(i, sample_count);
        let h = ggx_importance_sample(xi, roughness, n);

        // Reflect V around H
        let v_dot_h = v[0] * h[0] + v[1] * h[1] + v[2] * h[2];
        let l = [
            2.0 * v_dot_h * h[0] - v[0],
            2.0 * v_dot_h * h[1] - v[1],
            2.0 * v_dot_h * h[2] - v[2],
        ];

        let n_dot_l = l[2].max(0.0);
        let n_dot_h = h[2].max(0.0);
        let v_dot_h = v_dot_h.max(0.0);

        if n_dot_l > 0.0 {
            let g = geometry_smith(n_dot_v, n_dot_l, roughness);
            let g_vis = (g * v_dot_h) / (n_dot_h * n_dot_v);
            let fc = (1.0 - v_dot_h).powi(5);

            a += (1.0 - fc) * g_vis;
            b += fc * g_vis;
        }
    }

    [a / sample_count as f32, b / sample_count as f32]
}

fn bench_brdf_lut(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/brdf_lut");
    group.sample_size(30);

    // Benchmark single BRDF sample integration
    for sample_count in [64u32, 256, 1024] {
        group.bench_with_input(
            BenchmarkId::new("integrate_single_sample", sample_count),
            &sample_count,
            |b, &count| b.iter(|| black_box(integrate_brdf(0.5, 0.5, count))),
        );
    }

    // Benchmark row of BRDF LUT (roughness varies, NdotV fixed)
    for lut_size in [64u32, 128, 256] {
        group.bench_with_input(
            BenchmarkId::new("generate_row", lut_size),
            &lut_size,
            |b, &size| {
                let n_dot_v = 0.5f32;
                b.iter(|| {
                    for i in 0..size {
                        let roughness = (i as f32 + 0.5) / size as f32;
                        black_box(integrate_brdf(n_dot_v, roughness, 64));
                    }
                })
            },
        );
    }

    // Benchmark full BRDF LUT generation (expensive!)
    group.sample_size(10);
    for lut_size in [64u32, 128] {
        group.bench_with_input(
            BenchmarkId::new("generate_full_lut", format!("{}x{}", lut_size, lut_size)),
            &lut_size,
            |b, &size| {
                b.iter(|| {
                    let mut lut = vec![[0.0f32; 2]; (size * size) as usize];
                    for y in 0..size {
                        for x in 0..size {
                            let n_dot_v = (x as f32 + 0.5) / size as f32;
                            let roughness = (y as f32 + 0.5) / size as f32;
                            lut[(y * size + x) as usize] = integrate_brdf(n_dot_v, roughness, 64);
                        }
                    }
                    black_box(lut)
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// DEFERRED LIGHT ACCUMULATION BENCHMARKS
// =============================================================================

/// Point light contribution calculation
#[allow(clippy::too_many_arguments)]
fn calculate_point_light(
    world_pos: [f32; 3],
    normal: [f32; 3],
    view_dir: [f32; 3],
    light_pos: [f32; 3],
    light_color: [f32; 3],
    light_radius: f32,
    roughness: f32,
    metallic: f32,
    albedo: [f32; 3],
) -> [f32; 3] {
    // Light direction
    let l = [
        light_pos[0] - world_pos[0],
        light_pos[1] - world_pos[1],
        light_pos[2] - world_pos[2],
    ];
    let dist = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
    let l = [l[0] / dist, l[1] / dist, l[2] / dist];

    // Attenuation
    let atten = (1.0 - (dist / light_radius).clamp(0.0, 1.0)).powi(2);
    if atten <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    // Half vector
    let h = normalize(&[l[0] + view_dir[0], l[1] + view_dir[1], l[2] + view_dir[2]]);

    // Dot products
    let n_dot_l = (normal[0] * l[0] + normal[1] * l[1] + normal[2] * l[2]).max(0.0);
    let n_dot_v =
        (normal[0] * view_dir[0] + normal[1] * view_dir[1] + normal[2] * view_dir[2]).max(0.0);
    let n_dot_h = (normal[0] * h[0] + normal[1] * h[1] + normal[2] * h[2]).max(0.0);
    let v_dot_h = (view_dir[0] * h[0] + view_dir[1] * h[1] + view_dir[2] * h[2]).max(0.0);

    // Fresnel (Schlick)
    let f0 = [
        0.04 * (1.0 - metallic) + albedo[0] * metallic,
        0.04 * (1.0 - metallic) + albedo[1] * metallic,
        0.04 * (1.0 - metallic) + albedo[2] * metallic,
    ];
    let f = [
        f0[0] + (1.0 - f0[0]) * (1.0 - v_dot_h).powi(5),
        f0[1] + (1.0 - f0[1]) * (1.0 - v_dot_h).powi(5),
        f0[2] + (1.0 - f0[2]) * (1.0 - v_dot_h).powi(5),
    ];

    // Distribution (GGX)
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let d = a2 / (PI * denom * denom);

    // Geometry (Smith)
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);

    // Cook-Torrance specular
    let spec_denom = 4.0 * n_dot_v * n_dot_l + 0.0001;
    let specular = [
        d * g * f[0] / spec_denom,
        d * g * f[1] / spec_denom,
        d * g * f[2] / spec_denom,
    ];

    // Diffuse (Lambert)
    let k_d = [1.0 - f[0], 1.0 - f[1], 1.0 - f[2]];
    let diffuse = [
        k_d[0] * albedo[0] / PI * (1.0 - metallic),
        k_d[1] * albedo[1] / PI * (1.0 - metallic),
        k_d[2] * albedo[2] / PI * (1.0 - metallic),
    ];

    // Final contribution
    [
        (diffuse[0] + specular[0]) * light_color[0] * n_dot_l * atten,
        (diffuse[1] + specular[1]) * light_color[1] * n_dot_l * atten,
        (diffuse[2] + specular[2]) * light_color[2] * n_dot_l * atten,
    ]
}

fn bench_deferred_lighting(c: &mut Criterion) {
    let mut group = ci_budget(c, "ibl_deferred/deferred_lighting");

    // Generate test scene data
    let world_pos = [5.0f32, 2.0, 3.0];
    let normal = [0.0f32, 1.0, 0.0];
    let view_dir = normalize(&[0.0, 0.5, 1.0]);
    let albedo = [0.8f32, 0.6, 0.4];
    let roughness = 0.5f32;
    let metallic = 0.0f32;

    // Generate lights
    let lights: Vec<([f32; 3], [f32; 3], f32)> = (0..1000)
        .map(|i| {
            let t = i as f32 * 0.001;
            let phi = t * 2.0 * PI * 7.0;
            (
                [phi.cos() * 20.0, 5.0 + t * 10.0, phi.sin() * 20.0], // position
                [1.0, 0.9, 0.8],                                      // color
                10.0 + t * 5.0,                                       // radius
            )
        })
        .collect();

    // Benchmark single light calculation
    group.bench_function("single_point_light", |b| {
        let (light_pos, light_color, light_radius) = lights[0];
        b.iter(|| {
            black_box(calculate_point_light(
                world_pos,
                normal,
                view_dir,
                light_pos,
                light_color,
                light_radius,
                roughness,
                metallic,
                albedo,
            ))
        })
    });

    // Benchmark light accumulation for different light counts
    for light_count in [8u32, 32, 128, 512, 1000] {
        group.bench_with_input(
            BenchmarkId::new("accumulate_lights", light_count),
            &light_count,
            |b, &count| {
                b.iter(|| {
                    let mut total = [0.0f32; 3];
                    for (light_pos, light_color, light_radius) in lights.iter().take(count as usize)
                    {
                        let contrib = calculate_point_light(
                            world_pos,
                            normal,
                            view_dir,
                            *light_pos,
                            *light_color,
                            *light_radius,
                            roughness,
                            metallic,
                            albedo,
                        );
                        total[0] += contrib[0];
                        total[1] += contrib[1];
                        total[2] += contrib[2];
                    }
                    black_box(total)
                })
            },
        );
    }

    // Benchmark pixel processing (multiple pixels, fixed light count)
    let pixels: Vec<PixelDatum> = (0..1000)
        .map(|i| {
            let t = i as f32 * 0.001;
            (
                [t * 10.0, 2.0, t * 5.0],            // world_pos
                normalize(&[t, 1.0 - t, t * 0.5]),   // normal
                0.1 + t * 0.8,                       // roughness
                t,                                   // metallic
                [0.8, 0.6 - t * 0.3, 0.4 + t * 0.2], // albedo
            )
        })
        .collect();

    for light_count in [8u32, 32] {
        group.bench_with_input(
            BenchmarkId::new("process_1k_pixels", format!("{}lights", light_count)),
            &light_count,
            |b, &count| {
                b.iter(|| {
                    for (pos, norm, rough, metal, alb) in &pixels {
                        let mut total = [0.0f32; 3];
                        for (light_pos, light_color, light_radius) in
                            lights.iter().take(count as usize)
                        {
                            let contrib = calculate_point_light(
                                *pos,
                                *norm,
                                view_dir,
                                *light_pos,
                                *light_color,
                                *light_radius,
                                *rough,
                                *metal,
                                *alb,
                            );
                            total[0] += contrib[0];
                            total[1] += contrib[1];
                            total[2] += contrib[2];
                        }
                        black_box(total);
                    }
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// CRITERION GROUPS
// =============================================================================

criterion_group!(sh_benches, bench_spherical_harmonics);

criterion_group!(cubemap_benches, bench_cubemap_sampling);

criterion_group!(ggx_benches, bench_ggx_sampling);

criterion_group!(gbuffer_benches, bench_gbuffer_operations);

criterion_group!(brdf_benches, bench_brdf_lut);

criterion_group!(deferred_benches, bench_deferred_lighting);

criterion_main!(
    sh_benches,
    cubemap_benches,
    ggx_benches,
    gbuffer_benches,
    brdf_benches,
    deferred_benches
);

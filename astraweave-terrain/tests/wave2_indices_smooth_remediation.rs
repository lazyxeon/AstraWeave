//! Wave 2 targeted remediation for Heightmap::generate_indices and Heightmap::smooth
//!
//! Miss targets from shard 7:
//!   - L302: generate_indices — z * self.resolution : replace * with /
//!   - L325: smooth — self.data[idx - 1] : replace - with /
//!   - L328: smooth — self.data[idx + 1] : replace - with + (in neighbor offset)
//!   - L329: smooth — self.data[idx - self.resolution as usize] : replace + with -
//!   - L330: smooth — self.data[idx + self.resolution as usize] : replace - with +, /
//!   - L331: smooth — self.data[idx] * 4.0 : replace + with -

use astraweave_terrain::heightmap::{Heightmap, HeightmapConfig};

fn make_heightmap(resolution: u32) -> Heightmap {
    Heightmap::new(HeightmapConfig {
        resolution,
        ..Default::default()
    })
    .unwrap()
}

fn from_values(resolution: u32, data: Vec<f32>) -> Heightmap {
    Heightmap::from_data(data, resolution).unwrap()
}

// ══════════════════════════════════════════════════════════════════════════════
// generate_indices — golden value tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn generate_indices_2x2_grid() {
    let hm = make_heightmap(2);
    let indices = hm.generate_indices();
    assert_eq!(
        indices.len(),
        6,
        "2x2 grid → 1 cell → 2 triangles → 6 indices"
    );
    // base = 0*2+0 = 0  (CCW winding from above: base, base+res, base+1)
    assert_eq!(&indices[..], &[0, 2, 1, 1, 2, 3]);
}

#[test]
fn generate_indices_3x3_grid() {
    let hm = make_heightmap(3);
    let indices = hm.generate_indices();
    assert_eq!(indices.len(), 24);
}

#[test]
fn generate_indices_4x4_count() {
    let hm = make_heightmap(4);
    let indices = hm.generate_indices();
    assert_eq!(indices.len(), 54); // (4-1)^2 * 6
}

#[test]
fn generate_indices_formula() {
    for n in 2u32..=8 {
        let hm = make_heightmap(n);
        let indices = hm.generate_indices();
        let expected = ((n - 1) * (n - 1) * 6) as usize;
        assert_eq!(indices.len(), expected, "res={n}");
    }
}

#[test]
fn generate_indices_valid_range() {
    for n in [3u32, 5, 8, 16] {
        let hm = make_heightmap(n);
        let indices = hm.generate_indices();
        let max_vertex = n * n;
        for (i, &idx) in indices.iter().enumerate() {
            assert!(
                idx < max_vertex,
                "res={n}: index[{i}]={idx} >= max={max_vertex}"
            );
        }
    }
}

#[test]
fn generate_indices_3x3_specific_values() {
    // 3x3 grid vertices:
    //  0  1  2
    //  3  4  5
    //  6  7  8
    let hm = make_heightmap(3);
    let indices = hm.generate_indices();

    // Cell (x=0,z=0): base = 0*3+0 = 0  (CCW winding from above)
    assert_eq!(indices[0], 0); // base
    assert_eq!(indices[1], 3); // base+resolution
    assert_eq!(indices[2], 1); // base+1

    assert_eq!(indices[3], 1); // base+1
    assert_eq!(indices[4], 3); // base+resolution
    assert_eq!(indices[5], 4); // base+resolution+1

    // Cell (x=1,z=0): base = 0*3+1 = 1  (CCW: base, base+res, base+1)
    assert_eq!(indices[6], 1);
    assert_eq!(indices[7], 4);
    assert_eq!(indices[8], 2);

    // Cell (x=0,z=1): base = 1*3+0 = 3  (CCW: base, base+res, base+1)
    assert_eq!(indices[12], 3);
    assert_eq!(indices[13], 6);
    assert_eq!(indices[14], 4);
}

#[test]
fn generate_indices_row_offset_uses_multiplication() {
    // This specifically targets L302: z * self.resolution
    // If * were replaced with /, base would be wrong for z>0
    let hm = make_heightmap(4);
    let indices = hm.generate_indices();

    // Cell at z=2, x=0: base = 2*4+0 = 8
    // Cell row: z=2 is the 3rd cell row. Each row has (4-1)=3 cells, each cell = 6 indices
    let cell_offset = 2 * 3 * 6; // z=2, x=0
    assert_eq!(
        indices[cell_offset], 8,
        "base for z=2,x=0 should be 8 (not {})",
        indices[cell_offset]
    );
    assert_eq!(indices[cell_offset + 1], 12); // base + resolution = 8+4
    assert_eq!(indices[cell_offset + 2], 9); // base + 1
}

#[test]
fn generate_indices_divisible_by_3() {
    for n in 2u32..=10 {
        let hm = make_heightmap(n);
        let indices = hm.generate_indices();
        assert_eq!(
            indices.len() % 3,
            0,
            "indices must be multiple of 3 for res={n}"
        );
    }
}

#[test]
fn generate_indices_no_degenerate_triangles() {
    let hm = make_heightmap(5);
    let indices = hm.generate_indices();
    for tri in indices.chunks(3) {
        assert_ne!(tri[0], tri[1], "degenerate: {:?}", tri);
        assert_ne!(tri[1], tri[2], "degenerate: {:?}", tri);
        assert_ne!(tri[0], tri[2], "degenerate: {:?}", tri);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// smooth — golden value tests
// ══════════════════════════════════════════════════════════════════════════════

fn spike_4x4() -> Heightmap {
    let mut data = vec![0.0f32; 16];
    data[5] = 100.0; // (1,1)
    from_values(4, data)
}

fn uniform_4x4(v: f32) -> Heightmap {
    from_values(4, vec![v; 16])
}

#[test]
fn smooth_uniform_no_change() {
    let mut hm = uniform_4x4(5.0);
    hm.smooth(1);
    for (i, &v) in hm.data().iter().enumerate() {
        assert!((v - 5.0).abs() < 1e-6, "uniform cell {i}={v} should be 5.0");
    }
}

#[test]
fn smooth_spike_center() {
    let mut hm = spike_4x4();
    assert_eq!(hm.data()[5], 100.0);
    hm.smooth(1);
    // new = (0+0+0+0+100*4)/8 = 50.0
    assert!(
        (hm.data()[5] - 50.0).abs() < 1e-6,
        "spike center: expected 50.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_spike_right_neighbor() {
    let mut hm = spike_4x4();
    hm.smooth(1);
    // (2,1)=idx6: left=100, others=0, center=0 → (100+0+0+0+0)/8 = 12.5
    assert!(
        (hm.data()[6] - 12.5).abs() < 1e-6,
        "right: expected 12.5, got {}",
        hm.data()[6]
    );
}

#[test]
fn smooth_spike_down_neighbor() {
    let mut hm = spike_4x4();
    hm.smooth(1);
    // (1,2)=idx9: up=100(spike), others=0 → 100/8 = 12.5
    assert!(
        (hm.data()[9] - 12.5).abs() < 1e-6,
        "down: expected 12.5, got {}",
        hm.data()[9]
    );
}

#[test]
fn smooth_spike_diagonal_unaffected() {
    let mut hm = spike_4x4();
    hm.smooth(1);
    assert!(
        hm.data()[10].abs() < 1e-6,
        "diagonal: expected 0, got {}",
        hm.data()[10]
    );
}

#[test]
fn smooth_preserves_borders() {
    let mut hm = spike_4x4();
    hm.smooth(1);
    let border_cells = [0, 1, 2, 3, 4, 8, 12, 13, 14, 15, 7, 11];
    for &i in &border_cells {
        assert_eq!(hm.data()[i], 0.0, "border cell {i} should stay 0");
    }
}

#[test]
fn smooth_zero_iterations() {
    let mut hm = spike_4x4();
    let original: Vec<f32> = hm.data().to_vec();
    hm.smooth(0);
    assert_eq!(hm.data(), &original[..], "0 iterations = no change");
}

#[test]
fn smooth_multiple_iterations_converge() {
    let mut hm = spike_4x4();
    hm.smooth(10);
    let interior = [5, 6, 9, 10];
    let values: Vec<f32> = interior.iter().map(|&i| hm.data()[i]).collect();
    let avg = values.iter().sum::<f32>() / values.len() as f32;
    for (&i, &v) in interior.iter().zip(values.iter()) {
        assert!(
            (v - avg).abs() < 5.0,
            "cell {i}={v} should converge near avg={avg}"
        );
    }
}

#[test]
fn smooth_updates_min_max() {
    let mut hm = spike_4x4();
    assert_eq!(hm.max_height(), 100.0);
    hm.smooth(1);
    assert!(
        hm.max_height() < 100.0,
        "max should decrease: {}",
        hm.max_height()
    );
    assert!(hm.min_height() <= hm.max_height());
}

#[test]
fn smooth_kernel_golden_asymmetric() {
    // Layout (4x4):
    //  0  1  2  3
    //  4  5  6  7
    //  8  9 10 11
    // 12 13 14 15
    let mut data = vec![0.0f32; 16];
    data[1] = 10.0; // up of (1,1)
    data[4] = 20.0; // left of (1,1)
    data[5] = 30.0; // center (1,1)
    data[6] = 40.0; // right of (1,1)
    data[9] = 50.0; // down of (1,1)
    let mut hm = from_values(4, data);
    hm.smooth(1);
    // new[5] = (20+40+10+50+30*4)/8 = (120+120)/8 = 30.0
    assert!(
        (hm.data()[5] - 30.0).abs() < 1e-6,
        "exact kernel: expected 30.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_kernel_golden_all_different() {
    let mut data = vec![0.0f32; 16];
    data[5] = 1.0; // left of (2,1)=idx6
    data[7] = 3.0; // right
    data[2] = 5.0; // up
    data[10] = 7.0; // down
    data[6] = 11.0; // center
    let mut hm = from_values(4, data);
    hm.smooth(1);
    // new[6] = (1+3+5+7+11*4)/8 = 60/8 = 7.5
    assert!(
        (hm.data()[6] - 7.5).abs() < 1e-6,
        "expected 7.5, got {}",
        hm.data()[6]
    );
}

#[test]
fn smooth_left_neighbor_contribution() {
    let mut data = vec![0.0f32; 16];
    data[4] = 80.0; // left of (1,1)=idx5
    let mut hm = from_values(4, data);
    hm.smooth(1);
    // new[5] = (80+0+0+0+0*4)/8 = 10.0
    assert!(
        (hm.data()[5] - 10.0).abs() < 1e-6,
        "left: expected 10.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_right_neighbor_contribution() {
    let mut data = vec![0.0f32; 16];
    data[6] = 80.0; // right of (1,1)=idx5
    let mut hm = from_values(4, data);
    hm.smooth(1);
    assert!(
        (hm.data()[5] - 10.0).abs() < 1e-6,
        "right: expected 10.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_up_neighbor_contribution() {
    let mut data = vec![0.0f32; 16];
    data[1] = 80.0; // up of (1,1)=idx5
    let mut hm = from_values(4, data);
    hm.smooth(1);
    assert!(
        (hm.data()[5] - 10.0).abs() < 1e-6,
        "up: expected 10.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_down_neighbor_contribution() {
    let mut data = vec![0.0f32; 16];
    data[9] = 80.0; // down of (1,1)=idx5
    let mut hm = from_values(4, data);
    hm.smooth(1);
    assert!(
        (hm.data()[5] - 10.0).abs() < 1e-6,
        "down: expected 10.0, got {}",
        hm.data()[5]
    );
}

#[test]
fn smooth_center_self_weight_is_4() {
    let mut data = vec![0.0f32; 16];
    data[5] = 80.0; // only center, all neighbors 0
    let mut hm = from_values(4, data);
    hm.smooth(1);
    // new[5] = (0+0+0+0+80*4)/8 = 40.0
    assert!(
        (hm.data()[5] - 40.0).abs() < 1e-6,
        "center weight=4: expected 40.0, got {}",
        hm.data()[5]
    );
}

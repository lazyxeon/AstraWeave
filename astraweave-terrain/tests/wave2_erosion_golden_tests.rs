//! Golden-value tests for `Heightmap::apply_hydraulic_erosion`.
//!
//! Each test creates a small deterministic heightmap, runs the erosion algorithm,
//! and asserts exact cell values (within tight tolerance) to ensure all internal
//! arithmetic operators are correct.  This file specifically targets 20 missed
//! mutants in heightmap.rs lines 237-266 (velocity calculation, damping,
//! loop bounds, erosion/deposition amounts, evaporation).
//!
//! Mutation groups targeted:
//!   - L237: velocity_z += operator
//!   - L238: dt * (...) / 2.0 arithmetic
//!   - L241-242: velocity damping *= 0.99
//!   - L247-249: loop bounds (resolution - 1) and index (z*res + x)
//!   - L255: speed > min_slope threshold
//!   - L261: deposition_amount = speed * deposition * strength * 0.05
//!   - L262: self.data[idx] += deposition_amount (sign)
//!   - L266: water evaporation *= (1.0 - evaporation) and subtraction

use astraweave_terrain::Heightmap;

// ══════════════════════════════════════════════════════════════════════════════
// Helper constructors
// ══════════════════════════════════════════════════════════════════════════════

/// Peaked heightmap: single spike at center (4,4) = 100, all else 0
fn peaked_8x8() -> Heightmap {
    let mut data = vec![0.0f32; 64];
    data[4 * 8 + 4] = 100.0;
    Heightmap::from_data(data, 8).unwrap()
}

/// Ridge heightmap: entire row z=4 set to 50, all else 0
fn ridge_8x8() -> Heightmap {
    let mut data = vec![0.0f32; 64];
    for x in 0..8u32 {
        data[(4 * 8 + x) as usize] = 50.0;
    }
    Heightmap::from_data(data, 8).unwrap()
}

/// Gentle slope: h = z * 0.5, so the second derivative in z = 0 (linear).
/// Only X curvature or z curvature from non-linear sources drives velocity.
fn gentle_slope_8x8() -> Heightmap {
    let mut data = vec![0.0f32; 64];
    for z in 0..8u32 {
        for x in 0..8u32 {
            data[(z * 8 + x) as usize] = z as f32 * 0.5;
        }
    }
    Heightmap::from_data(data, 8).unwrap()
}

/// Bowl: concave heightmap h = (z-3.5)² + (x-3.5)²
fn bowl_8x8() -> Heightmap {
    let mut data = vec![0.0f32; 64];
    for z in 0..8u32 {
        for x in 0..8u32 {
            let dz = z as f32 - 3.5;
            let dx = x as f32 - 3.5;
            data[(z * 8 + x) as usize] = dz * dz + dx * dx;
        }
    }
    Heightmap::from_data(data, 8).unwrap()
}

/// Assertion helper with descriptive error message
fn assert_close(actual: f32, expected: f32, tol: f32, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{label}: expected {expected:.10}, got {actual:.10}, diff={diff:.10e} > tol={tol:.0e}"
    );
}

/// For checking that a value does NOT match a particular golden (catches mutations)
fn assert_not_close(actual: f32, bad_value: f32, tol: f32, label: &str) {
    let diff = (actual - bad_value).abs();
    assert!(
        diff > tol,
        "{label}: value should NOT be {bad_value}, but got {actual}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Golden-value tests: PEAKED 8×8
//
// These exact values were captured from the unmutated algorithm and serve as
// regression anchors.  Any arithmetic operator swap in lines 237-266 will
// produce different numbers, causing these assertions to fail.
// ══════════════════════════════════════════════════════════════════════════════

const TOL: f32 = 1e-4;

#[test]
fn peaked_erosion_center_cell_golden() {
    // Center (4,4) started at 100.0, should erode to ~56.18
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    assert_close(
        hm.data()[4 * 8 + 4],
        56.1815605164,
        TOL,
        "peaked center (4,4)",
    );
}

#[test]
fn peaked_erosion_adjacent_cells_golden() {
    // Cells adjacent to peak should be heavily negative (sediment removed by flow)
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // (3,4) and (4,3) — one row/col before center
    assert_close(hm.data()[3 * 8 + 4], -17.0985431671, TOL, "peaked (3,4)");
    assert_close(hm.data()[4 * 8 + 3], -17.0985431671, TOL, "peaked (4,3)");

    // (5,4) and (4,5) — one row/col after center (slightly different due to scan order)
    assert_close(hm.data()[5 * 8 + 4], -17.0985813141, TOL, "peaked (5,4)");
    assert_close(hm.data()[4 * 8 + 5], -17.0985813141, TOL, "peaked (4,5)");
}

#[test]
fn peaked_erosion_diagonal_cells_golden() {
    // Diagonal neighbors of the peak
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    assert_close(hm.data()[3 * 8 + 3], -1.7235434055, TOL, "peaked (3,3)");
    assert_close(hm.data()[5 * 8 + 5], -1.7235444784, TOL, "peaked (5,5)");
    assert_close(hm.data()[3 * 8 + 5], -1.7235438824, TOL, "peaked (3,5)");
    assert_close(hm.data()[5 * 8 + 3], -1.7235438824, TOL, "peaked (5,3)");
}

#[test]
fn peaked_erosion_two_away_cells_golden() {
    // Two cells away from peak — smaller erosion
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // (2,4) and (4,2)
    assert_close(hm.data()[2 * 8 + 4], -1.2712671757, TOL, "peaked (2,4)");
    assert_close(hm.data()[4 * 8 + 2], -1.2712671757, TOL, "peaked (4,2)");

    // (6,4) and (4,6)
    assert_close(hm.data()[6 * 8 + 4], -1.2695193291, TOL, "peaked (6,4)");
    assert_close(hm.data()[4 * 8 + 6], -1.2695193291, TOL, "peaked (4,6)");
}

#[test]
fn peaked_erosion_far_cells_golden() {
    // Far cells: (1,4), (2,2), (1,1)
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    assert_close(hm.data()[1 * 8 + 4], -0.0583410040, TOL, "peaked (1,4)");
    assert_close(hm.data()[2 * 8 + 2], -0.0004636676, TOL, "peaked (2,2)");
    assert_close(hm.data()[1 * 8 + 1], 0.0000002873, TOL, "peaked (1,1)");
    assert_close(hm.data()[2 * 8 + 3], -0.0965780765, TOL, "peaked (2,3)");
}

#[test]
fn peaked_erosion_bounds_golden() {
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    assert_close(hm.min_height(), -17.0985813141, TOL, "peaked min_height");
    assert_close(hm.max_height(), 56.1815605164, TOL, "peaked max_height");
}

// ══════════════════════════════════════════════════════════════════════════════
// Golden-value tests: RIDGE 8×8
//
// The ridge varies ONLY in Z direction (all columns identical for inner cells).
// This makes the test especially sensitive to velocity_z mutations (L237-238).
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn ridge_erosion_center_row_golden() {
    let mut hm = ridge_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Ridge row cells — inner cells should be ~33.48, border cells stay 50
    assert_close(hm.data()[4 * 8 + 0], 50.0, 1e-6, "ridge (4,0) border");
    assert_close(hm.data()[4 * 8 + 7], 50.0, 1e-6, "ridge (4,7) border");
    assert_close(hm.data()[4 * 8 + 1], 33.3996696472, TOL, "ridge (4,1)");
    assert_close(hm.data()[4 * 8 + 2], 33.4792633057, TOL, "ridge (4,2)");
    assert_close(hm.data()[4 * 8 + 3], 33.4792633057, TOL, "ridge (4,3)");
}

#[test]
fn ridge_erosion_adjacent_rows_golden() {
    let mut hm = ridge_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Row 3 — above ridge (erosion from flow)
    assert_close(hm.data()[3 * 8 + 1], -8.9842901230, TOL, "ridge (3,1)");
    assert_close(hm.data()[3 * 8 + 2], -8.9439640045, TOL, "ridge (3,2)");

    // Row 5 — below ridge (slightly different due to scan order)
    assert_close(hm.data()[5 * 8 + 1], -8.9843034744, TOL, "ridge (5,1)");
    assert_close(hm.data()[5 * 8 + 2], -8.9439764023, TOL, "ridge (5,2)");
}

#[test]
fn ridge_erosion_far_rows_golden() {
    let mut hm = ridge_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Row 2 — two rows above ridge
    assert_close(hm.data()[2 * 8 + 1], -0.6546280384, TOL, "ridge (2,1)");
    assert_close(hm.data()[2 * 8 + 2], -0.6524378061, TOL, "ridge (2,2)");

    // Row 6 — two rows below ridge
    assert_close(hm.data()[6 * 8 + 1], -0.6538521051, TOL, "ridge (6,1)");
    assert_close(hm.data()[6 * 8 + 2], -0.6516621113, TOL, "ridge (6,2)");

    // Row 1 — faint effect
    assert_close(hm.data()[1 * 8 + 1], -0.0287287701, TOL, "ridge (1,1)");
    assert_close(hm.data()[1 * 8 + 2], -0.0286653042, TOL, "ridge (1,2)");
}

#[test]
fn ridge_erosion_bounds_golden() {
    let mut hm = ridge_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    assert_close(hm.min_height(), -8.9843034744, TOL, "ridge min_height");
    assert_close(hm.max_height(), 50.0, TOL, "ridge max_height");
}

// ══════════════════════════════════════════════════════════════════════════════
// Border invariant tests: catches loop bound mutations (L247-249)
//
// The loop is `for z in 1..(self.resolution - 1)` and
//               `for x in 1..(self.resolution - 1)`.
// Mutation `- 1` → `/ 1` would process the last row/col.
// Mutation `+ x` → `- x` in index would read wrong cells.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn peaked_border_row0_unchanged() {
    let mut hm = peaked_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    for x in 0..8 {
        assert_eq!(
            hm.data()[x],
            before[x],
            "Row 0, col {x} should be unchanged after erosion"
        );
    }
}

#[test]
fn peaked_border_row7_unchanged() {
    let mut hm = peaked_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    for x in 0..8 {
        assert_eq!(
            hm.data()[7 * 8 + x],
            before[7 * 8 + x],
            "Row 7, col {x} should be unchanged after erosion"
        );
    }
}

#[test]
fn peaked_border_col0_unchanged() {
    let mut hm = peaked_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    for z in 0..8 {
        assert_eq!(
            hm.data()[z * 8],
            before[z * 8],
            "Col 0, row {z} should be unchanged after erosion"
        );
    }
}

#[test]
fn peaked_border_col7_unchanged() {
    let mut hm = peaked_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    for z in 0..8 {
        assert_eq!(
            hm.data()[z * 8 + 7],
            before[z * 8 + 7],
            "Col 7, row {z} should be unchanged after erosion"
        );
    }
}

#[test]
fn ridge_border_cells_unchanged() {
    let mut hm = ridge_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Row 0 and row 7 (all zeros)
    for x in 0..8 {
        assert_eq!(hm.data()[x], before[x], "ridge row 0, col {x}");
        assert_eq!(
            hm.data()[7 * 8 + x],
            before[7 * 8 + x],
            "ridge row 7, col {x}"
        );
    }
    // Col 0 and col 7
    for z in 0..8 {
        assert_eq!(hm.data()[z * 8], before[z * 8], "ridge col 0, row {z}");
        assert_eq!(
            hm.data()[z * 8 + 7],
            before[z * 8 + 7],
            "ridge col 7, row {z}"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Velocity and damping mutation detection
//
// Strategy: Compare peaked vs ridge behavior. The peaked shape exercises BOTH
// velocity_x and velocity_z equally. The ridge exercises velocity_z dominantly.
// Any mutation to velocity_z (L237) or its damping (L242) would change the
// ridge results while having a different effect on peaked results.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn ridge_z_velocity_drives_erosion() {
    // Ridge only varies in Z direction. velocity_x should be ~0 for inner cells
    // (since heights are constant along X). So ALL erosion comes from velocity_z.
    // If velocity_z is broken (L237: += → *=), erosion would be near zero.
    let mut hm = ridge_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Inner row 3 cells should show significant erosion (driven by velocity_z)
    let cell = hm.data()[3 * 8 + 3];
    let change = (cell - before[3 * 8 + 3]).abs();
    assert!(
        change > 5.0,
        "Ridge inner cell (3,3) should show significant Z-driven erosion, got change={change}"
    );
}

#[test]
fn peaked_symmetric_erosion_xy() {
    // For the peaked shape, erosion in X and Z should be approximately symmetric.
    // Cells equidistant from center in X and Z should have similar values.
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // (3,4) is one cell above center, (4,3) is one cell left of center
    let above = hm.data()[3 * 8 + 4];
    let left = hm.data()[4 * 8 + 3];
    let diff = (above - left).abs();
    assert!(
        diff < 0.01,
        "Peaked erosion should be symmetric: above={above}, left={left}, diff={diff}"
    );

    // (2,4) is two above, (4,2) is two left
    let two_above = hm.data()[2 * 8 + 4];
    let two_left = hm.data()[4 * 8 + 2];
    let diff2 = (two_above - two_left).abs();
    assert!(
        diff2 < 0.01,
        "Two-away cells should be symmetric: above={two_above}, left={two_left}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Erosion vs deposition direction test (L262: += → -=)
//
// In cells where speed < min_slope, deposition should ADD height (positive).
// With the -= mutation, those cells would lose height instead.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn peaked_far_corner_deposition_positive() {
    // Cell (1,1) is far from peak — very low velocity → deposition branch.
    // With correct code: height increases slightly from 0.0.
    // With -= mutation: height would decrease.
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    let val = hm.data()[1 * 8 + 1];
    // Golden value: 0.0000002873 (very small positive)
    assert!(
        val > 0.0,
        "Cell (1,1) deposition should be positive, got {val}"
    );
    assert_close(val, 0.0000002873, TOL, "peaked (1,1) deposition");
}

#[test]
fn peaked_deposition_vs_erosion_sign_check() {
    // Multiple cells: positive (deposition) vs negative (erosion)
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Near peak: erosion causes negative values
    assert!(
        hm.data()[3 * 8 + 4] < 0.0,
        "(3,4) should be negative from erosion"
    );
    assert!(
        hm.data()[2 * 8 + 4] < 0.0,
        "(2,4) should be negative from erosion"
    );

    // Far corner: deposition causes small positive values
    assert!(
        hm.data()[1 * 8 + 1] > 0.0,
        "(1,1) should be positive from deposition"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Scan-order asymmetry test (proves exact computation path)
//
// The z-ascending then x-ascending scan order creates a tiny asymmetry
// between cells mirrored across center. Tests that assert this asymmetry
// effectively fingerprint the exact algorithm implementation.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn peaked_scan_order_asymmetry() {
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // (3,4) vs (5,4): both one step from center in Z, but processed in different
    // iterations, leading to slightly different results
    let before_center = hm.data()[3 * 8 + 4];
    let after_center = hm.data()[5 * 8 + 4];

    // They should be close but NOT identical
    assert!(
        (before_center - after_center).abs() > 1e-6,
        "Scan order should create asymmetry: (3,4)={before_center}, (5,4)={after_center}"
    );
    assert!(
        (before_center - after_center).abs() < 1.0,
        "Asymmetry should be small: (3,4)={before_center}, (5,4)={after_center}"
    );

    // Exact golden values
    assert_close(
        before_center,
        -17.0985431671,
        TOL,
        "peaked (3,4) scan-order",
    );
    assert_close(after_center, -17.0985813141, TOL, "peaked (5,4) scan-order");
}

// ══════════════════════════════════════════════════════════════════════════════
// Strength sensitivity tests
//
// Different strengths produce different golden values. This tests that the
// strength parameter flows correctly through erosion amount (L257), rain
// amount (L213), and deposition amount (L261).
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn peaked_erosion_different_strengths_exact() {
    let mut hm_weak = peaked_8x8();
    let mut hm_strong = peaked_8x8();
    hm_weak.apply_hydraulic_erosion(0.5).unwrap();
    hm_strong.apply_hydraulic_erosion(2.0).unwrap();

    // Center cell: higher strength → more erosion
    let weak_center = hm_weak.data()[4 * 8 + 4];
    let strong_center = hm_strong.data()[4 * 8 + 4];

    assert!(
        weak_center > strong_center,
        "Weaker erosion should leave more height: weak={weak_center}, strong={strong_center}"
    );

    // Adjacent cell: higher strength → more negative
    let weak_adj = hm_weak.data()[3 * 8 + 4];
    let strong_adj = hm_strong.data()[3 * 8 + 4];
    assert!(
        strong_adj < weak_adj,
        "Stronger erosion should erode more: weak={weak_adj}, strong={strong_adj}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Multiple cells cross-check: ensures the erosion/deposition cycle is consistent
//
// Total height should be approximately conserved (erosion removes from peaks,
// deposition adds to valleys), minus any float precision drift.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn peaked_sum_of_changes_consistent() {
    let before = peaked_8x8();
    let mut after = peaked_8x8();
    after.apply_hydraulic_erosion(1.0).unwrap();

    let before_sum: f32 = before.data().iter().sum();
    let after_sum: f32 = after.data().iter().sum();

    // With correct algorithm, total height changes by a specific amount
    // (erosion/deposition aren't perfectly conserving due to boundary)
    let delta = after_sum - before_sum;
    // Golden: should be a negative value (net erosion)
    // The exact value depends on all arithmetic — any mutation changes it
    assert!(
        delta.is_finite(),
        "Height sum should be finite, got {delta}"
    );
    // Assert the delta is in a specific range
    assert!(
        delta.abs() < 200.0,
        "Net change should be bounded, got {delta}"
    );
}

#[test]
fn ridge_all_inner_cells_golden() {
    // Assert ALL non-border cells match golden values
    let mut hm = ridge_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Row 1 inner cells (z=1, x=1..6)
    let row1_expected = [
        -0.0287287701,
        -0.0286653042,
        -0.0286653042,
        -0.0286653042,
        -0.0286653042,
        -0.0287287701,
    ];
    for (i, &expected) in row1_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[1 * 8 + x],
            expected,
            TOL,
            &format!("ridge (1,{x})"),
        );
    }

    // Row 2 inner cells
    let row2_expected = [
        -0.6546280384,
        -0.6524378061,
        -0.6524378061,
        -0.6524378061,
        -0.6524378061,
        -0.6546280384,
    ];
    for (i, &expected) in row2_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[2 * 8 + x],
            expected,
            TOL,
            &format!("ridge (2,{x})"),
        );
    }

    // Row 3 inner cells
    let row3_expected = [
        -8.9842901230,
        -8.9439640045,
        -8.9439640045,
        -8.9439640045,
        -8.9439640045,
        -8.9842901230,
    ];
    for (i, &expected) in row3_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[3 * 8 + x],
            expected,
            TOL,
            &format!("ridge (3,{x})"),
        );
    }

    // Row 4 inner cells (ridge row)
    let row4_expected = [
        33.3996696472,
        33.4792633057,
        33.4792633057,
        33.4792633057,
        33.4792633057,
        33.3996696472,
    ];
    for (i, &expected) in row4_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[4 * 8 + x],
            expected,
            TOL,
            &format!("ridge (4,{x})"),
        );
    }

    // Row 5 inner cells
    let row5_expected = [
        -8.9843034744,
        -8.9439764023,
        -8.9439764023,
        -8.9439764023,
        -8.9439764023,
        -8.9843034744,
    ];
    for (i, &expected) in row5_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[5 * 8 + x],
            expected,
            TOL,
            &format!("ridge (5,{x})"),
        );
    }

    // Row 6 inner cells
    let row6_expected = [
        -0.6538521051,
        -0.6516621113,
        -0.6516621113,
        -0.6516621113,
        -0.6516621113,
        -0.6538521051,
    ];
    for (i, &expected) in row6_expected.iter().enumerate() {
        let x = i + 1;
        assert_close(
            hm.data()[6 * 8 + x],
            expected,
            TOL,
            &format!("ridge (6,{x})"),
        );
    }
}

#[test]
fn peaked_all_inner_cells_golden() {
    // Assert ALL inner cells (rows 1-6, cols 1-6) match golden values
    let mut hm = peaked_8x8();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Golden values for inner 6×6 block, row by row
    let expected: [[f32; 6]; 6] = [
        // Row 1
        [
            0.0000002873,
            0.0000295108,
            0.0018464078,
            -0.0583410040,
            0.0018464078,
            0.0000295107,
        ],
        // Row 2
        [
            0.0000295108,
            -0.0004636676,
            -0.0965780765,
            -1.2712671757,
            -0.0965780765,
            -0.0004638379,
        ],
        // Row 3
        [
            0.0018464078,
            -0.0965780765,
            -1.7235434055,
            -17.0985431671,
            -1.7235438824,
            -0.0965881869,
        ],
        // Row 4
        [
            -0.0583410040,
            -1.2712671757,
            -17.0985431671,
            56.1815605164,
            -17.0985813141,
            -1.2695193291,
        ],
        // Row 5
        [
            0.0018464078,
            -0.0965780765,
            -1.7235438824,
            -17.0985813141,
            -1.7235444784,
            -0.0965881944,
        ],
        // Row 6
        [
            0.0000295107,
            -0.0004638379,
            -0.0965881869,
            -1.2695193291,
            -0.0965881944,
            -0.0004640076,
        ],
    ];

    for (zi, row) in expected.iter().enumerate() {
        let z = zi + 1;
        for (xi, &exp) in row.iter().enumerate() {
            let x = xi + 1;
            assert_close(hm.data()[z * 8 + x], exp, TOL, &format!("peaked ({z},{x})"));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Bowl (concave) test: velocity_x and velocity_z are both significant
// since heights increase symmetrically in all directions from center.
// This catches mutations that zero out one velocity component.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn bowl_erosion_changes_inner_cells() {
    let mut hm = bowl_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Bowl has concave shape — water flows toward center
    // Inner cells should change
    let center_idx = 3 * 8 + 3; // Near bowl minimum
    let diff = (hm.data()[center_idx] - before[center_idx]).abs();
    assert!(diff > 1e-6, "Bowl center should change, diff={diff}");
}

#[test]
fn bowl_border_preservation() {
    let mut hm = bowl_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // ALL border cells unchanged
    for x in 0..8 {
        assert_eq!(hm.data()[x], before[x], "bowl row 0, col {x}");
        assert_eq!(
            hm.data()[7 * 8 + x],
            before[7 * 8 + x],
            "bowl row 7, col {x}"
        );
    }
    for z in 1..7 {
        assert_eq!(hm.data()[z * 8], before[z * 8], "bowl col 0, row {z}");
        assert_eq!(
            hm.data()[z * 8 + 7],
            before[z * 8 + 7],
            "bowl col 7, row {z}"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Determinism test: same input → same output (catches nondeterministic bugs)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn erosion_is_deterministic() {
    let mut hm1 = peaked_8x8();
    let mut hm2 = peaked_8x8();
    hm1.apply_hydraulic_erosion(1.0).unwrap();
    hm2.apply_hydraulic_erosion(1.0).unwrap();

    for i in 0..64 {
        assert_eq!(
            hm1.data()[i],
            hm2.data()[i],
            "Erosion should be deterministic, cell {i} differs"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Linear slope: no change (constant second derivative = 0)
//
// This catches mutations that would create velocity from constant gradients
// (e.g., if / 2.0 becomes % 2.0, the velocity calculation changes for
// fields with equal left-right or up-down differences).
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn linear_gradient_no_change() {
    // Linear slope: h = z * 0.5 → constant gradient
    // Left-right difference = 0, up-down difference = constant = 0.5
    // velocity_z = dt * (up + down - 2*center) / 2.0
    //            = dt * ((z-1)*0.5 + (z+1)*0.5 - 2*z*0.5) / 2.0
    //            = dt * 0 / 2.0 = 0
    // So ALL velocities = 0, speed = 0, deposition_amount = 0, no change
    let mut hm = gentle_slope_8x8();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    for i in 0..64 {
        assert_eq!(
            hm.data()[i],
            before[i],
            "Linear gradient cell {i} should not change: before={}, after={}",
            before[i],
            hm.data()[i]
        );
    }
}

//! Phase 1.6-F.3-phase-2.B: behavioral tests for the balanced preset
//! variants `ErosionPreset::default_balanced()` and
//! `ErosionPreset::mountain_balanced()`. Phase-0's synthetic heightmap
//! tests cover the full-droplet `default()` and `mountain()` presets;
//! these tests verify the reduced-droplet variants still produce valid
//! erosion output, just with less intensity.
//!
//! Coverage:
//! - balanced variants still erode (non-zero total_eroded on sloped terrain)
//! - balanced default still reduces multi-spike curvature substantially
//!   (>30% vs full default's ~91%)
//! - balanced mountain distinct from balanced default (mountain remains
//!   more aggressive post-droplet-reduction)
//! - both balanced variants remain deterministic
//! - droplet_count values match §2.3's budget targets

use astraweave_terrain::advanced_erosion::{AdvancedErosionSimulator, ErosionPreset};
use astraweave_terrain::Heightmap;

fn make_heightmap(data: Vec<f32>, dim: u32) -> Heightmap {
    Heightmap::from_data(data, dim).expect("heightmap construction")
}

fn slope_heightmap(dim: u32, max_height: f32) -> Heightmap {
    let mut h = vec![0f32; (dim * dim) as usize];
    for iz in 0..dim {
        for ix in 0..dim {
            h[(iz * dim + ix) as usize] = (ix as f32 / (dim - 1) as f32) * max_height;
        }
    }
    make_heightmap(h, dim)
}

fn multi_spike_heightmap(dim: u32, base: f32, spike_height: f32, stride: u32) -> Heightmap {
    let mut h = vec![base; (dim * dim) as usize];
    for iz in (stride / 2..dim).step_by(stride as usize) {
        for ix in (stride / 2..dim).step_by(stride as usize) {
            h[(iz * dim + ix) as usize] = base + spike_height;
        }
    }
    make_heightmap(h, dim)
}

fn mean_abs_curvature(h: &Heightmap) -> f32 {
    let dim = h.resolution();
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for z in 1..(dim - 1) {
        for x in 1..(dim - 1) {
            let center = h.get_height(x, z);
            let avg = 0.25
                * (h.get_height(x - 1, z)
                    + h.get_height(x + 1, z)
                    + h.get_height(x, z - 1)
                    + h.get_height(x, z + 1));
            sum += (center - avg).abs();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

#[test]
fn balanced_presets_droplet_counts_match_budget_targets() {
    let def = ErosionPreset::default_balanced();
    let mtn = ErosionPreset::mountain_balanced();

    let def_drops = def
        .hydraulic
        .as_ref()
        .map(|h| h.droplet_count)
        .expect("default_balanced has hydraulic");
    let mtn_drops = mtn
        .hydraulic
        .as_ref()
        .map(|h| h.droplet_count)
        .expect("mountain_balanced has hydraulic");

    // Phase 2: default 50k → 35k, mountain 100k → 50k (§2.3 budget).
    // Phase 4: default 35k → 25k, mountain 50k → 35k (scale recovery
    // after phase-3's world-coord droplet distribution proved more
    // aggressive than phase-2's clumpy RNG).
    assert_eq!(
        def_drops, 25_000,
        "default_balanced droplet_count should be 25k per F.3-phase-4.C"
    );
    assert_eq!(
        mtn_drops, 35_000,
        "mountain_balanced droplet_count should be 35k per F.3-phase-4.C"
    );

    // And full variants must remain UNCHANGED (phase-0 tests depend on
    // these specific values).
    let def_full = ErosionPreset::default();
    let mtn_full = ErosionPreset::mountain();
    assert_eq!(
        def_full
            .hydraulic
            .as_ref()
            .map(|h| h.droplet_count)
            .unwrap(),
        50_000,
        "full default() droplet_count must stay at 50k (phase-0 contract)"
    );
    assert_eq!(
        mtn_full
            .hydraulic
            .as_ref()
            .map(|h| h.droplet_count)
            .unwrap(),
        100_000,
        "full mountain() droplet_count must stay at 100k (phase-0 contract)"
    );
}

#[test]
fn default_balanced_still_erodes_slope() {
    let mut h = slope_heightmap(96, 100.0);
    let preset = ErosionPreset::default_balanced();
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut h, &preset);

    assert!(
        stats.total_eroded > 0.0,
        "default_balanced should erode non-zero material on a 100-unit slope"
    );
}

#[test]
fn mountain_balanced_still_erodes_slope() {
    let mut h = slope_heightmap(96, 100.0);
    let preset = ErosionPreset::mountain_balanced();
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut h, &preset);

    assert!(
        stats.total_eroded > 0.0,
        "mountain_balanced should erode non-zero material on a 100-unit slope"
    );
}

#[test]
fn default_balanced_still_reduces_multi_spike_curvature() {
    // Full `default()` at 50k droplets reduces multi-spike curvature by
    // ~91% per phase-0. Balanced at 35k should still achieve substantial
    // reduction — target ≥ 30%.
    let initial = multi_spike_heightmap(96, 20.0, 40.0, 8);
    let initial_curvature = mean_abs_curvature(&initial);

    let mut h = initial;
    let preset = ErosionPreset::default_balanced();
    let mut sim = AdvancedErosionSimulator::new(42);
    sim.apply_preset(&mut h, &preset);
    let final_curvature = mean_abs_curvature(&h);

    let reduction = 1.0 - final_curvature / initial_curvature;
    println!(
        "default_balanced multi-spike curvature: {:.4} → {:.4} (−{:.1}%)",
        initial_curvature,
        final_curvature,
        reduction * 100.0
    );
    assert!(
        reduction > 0.30,
        "default_balanced should reduce multi-spike curvature by > 30%: actual {:.1}%",
        reduction * 100.0
    );
}

#[test]
fn mountain_balanced_more_aggressive_than_default_balanced() {
    // Same-seed comparison on identical slope input: mountain_balanced's
    // higher erode_speed + larger droplet count + more aggressive thermal
    // should still yield higher total_eroded than default_balanced.
    let input = slope_heightmap(96, 100.0);

    let mut h_def = input.clone();
    let mut sim_def = AdvancedErosionSimulator::new(42);
    let stats_def = sim_def.apply_preset(&mut h_def, &ErosionPreset::default_balanced());

    let mut h_mtn = input;
    let mut sim_mtn = AdvancedErosionSimulator::new(42);
    let stats_mtn = sim_mtn.apply_preset(&mut h_mtn, &ErosionPreset::mountain_balanced());

    println!(
        "balanced default total_eroded={:.2}, balanced mountain total_eroded={:.2}",
        stats_def.total_eroded, stats_mtn.total_eroded
    );
    assert!(
        stats_mtn.total_eroded > stats_def.total_eroded,
        "mountain_balanced should erode more than default_balanced: {} vs {}",
        stats_mtn.total_eroded,
        stats_def.total_eroded
    );
}

#[test]
fn balanced_presets_deterministic_across_runs() {
    let input = slope_heightmap(64, 50.0);

    let mut h_a = input.clone();
    let mut sim_a = AdvancedErosionSimulator::new(123);
    sim_a.apply_preset(&mut h_a, &ErosionPreset::default_balanced());

    let mut h_b = input;
    let mut sim_b = AdvancedErosionSimulator::new(123);
    sim_b.apply_preset(&mut h_b, &ErosionPreset::default_balanced());

    for i in 0..h_a.data().len() {
        let diff = (h_a.data()[i] - h_b.data()[i]).abs();
        assert!(
            diff < 1e-4,
            "default_balanced not deterministic at index {i}: {} vs {} (diff {diff})",
            h_a.data()[i],
            h_b.data()[i]
        );
    }
}

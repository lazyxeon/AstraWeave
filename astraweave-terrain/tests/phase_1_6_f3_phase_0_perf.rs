//! Phase 1.6-F.3-phase-0.C: performance characterization of AdvancedErosionSimulator.
//!
//! Measures wall-clock time for each preset at relevant heightmap sizes.
//! Not a criterion benchmark — just a reasonable-precision timing harness
//! using std::time::Instant. Output goes to stdout via println!.
//!
//! Relevant sizes:
//!   64² — single F.2-T-4 chunk (64 verts/side per §2.1).
//!   128² — intermediate size for scaling investigation.
//!   192² — halo-expanded chunk (1-chunk halo on 64-vertex chunk = 3 × 64 = 192).
//!   256² — larger halo expansion case.
//!
//! All tests run release-ish timing by default (debug build shouldn't be
//! orders of magnitude off for this workload). To run with release timing:
//!     cargo test -p astraweave-terrain --release --test phase_1_6_f3_phase_0_perf -- --nocapture
//!
//! Run with --nocapture to see timing output.

use astraweave_terrain::advanced_erosion::{AdvancedErosionSimulator, ErosionPreset};
use astraweave_terrain::Heightmap;

fn slope_heightmap(dim: u32, max_height: f32) -> Heightmap {
    let mut h = vec![0f32; (dim * dim) as usize];
    for iz in 0..dim {
        for ix in 0..dim {
            h[(iz * dim + ix) as usize] = (ix as f32 / (dim - 1) as f32) * max_height;
        }
    }
    Heightmap::from_data(h, dim).unwrap()
}

fn time_preset(label: &str, dim: u32, preset_builder: fn() -> ErosionPreset) -> std::time::Duration {
    let mut h = slope_heightmap(dim, 100.0);
    let preset = preset_builder();
    let mut sim = AdvancedErosionSimulator::new(42);

    let start = std::time::Instant::now();
    let _ = sim.apply_preset(&mut h, &preset);
    let elapsed = start.elapsed();

    println!("  {label:<24} {dim:>4}² = {} ms", elapsed.as_millis());
    elapsed
}

#[test]
fn phase_1_6_f3_phase_0_perf_characterization() {
    println!("======================================================");
    println!("F.3-phase-0.C: AdvancedErosionSimulator performance");
    println!("  Debug vs release builds may differ 5-20x; run with --release for");
    println!("  realistic phase 2 estimates.");
    println!();

    // Lower dimensions first for quick results.
    let mut results: Vec<(&str, u32, u128)> = Vec::new();

    println!("Preset-by-size timing:");
    for &dim in &[64u32, 128, 192, 256] {
        let d = time_preset("default", dim, ErosionPreset::default);
        results.push(("default", dim, d.as_millis()));

        let d = time_preset("desert", dim, ErosionPreset::desert);
        results.push(("desert", dim, d.as_millis()));

        let d = time_preset("mountain", dim, ErosionPreset::mountain);
        results.push(("mountain", dim, d.as_millis()));

        let d = time_preset("coastal", dim, ErosionPreset::coastal);
        results.push(("coastal", dim, d.as_millis()));

        println!();
    }

    // Summary table.
    println!("======================================================");
    println!("Summary table:");
    println!();
    println!("| Preset   |   64² |  128² |  192² |  256² |");
    println!("|----------|------:|------:|------:|------:|");
    for preset in ["default", "desert", "mountain", "coastal"] {
        print!("| {preset:<8} |");
        for dim in [64u32, 128, 192, 256] {
            let ms = results
                .iter()
                .find(|(p, d, _)| *p == preset && *d == dim)
                .map(|(_, _, ms)| *ms)
                .unwrap_or(0);
            print!(" {ms:>5} |");
        }
        println!();
    }

    // Project to AstraWeave's phase-2 use case.
    //
    // F.3-phase-2 plan §2.3: halo=1 for each chunk. Editor generates 121
    // chunks (radius 5 × 11×11). Each chunk with halo becomes 3×3 chunks =
    // 192 vertices per side after halo expansion.
    //
    // Assumption: each halo-expanded region is eroded INDEPENDENTLY.
    // So 121 × per-192² cost.
    println!();
    println!("Phase 2 projection for 121 chunks × halo=1 (192² per halo region):");
    for preset in ["default", "desert", "mountain", "coastal"] {
        let ms_192 = results
            .iter()
            .find(|(p, d, _)| *p == preset && *d == 192u32)
            .map(|(_, _, ms)| *ms)
            .unwrap_or(0);
        let total_ms = ms_192 as u64 * 121;
        let seconds = total_ms as f64 / 1000.0;
        println!(
            "  {preset:<8}: 121 × {ms_192}ms = {:.1}s",
            seconds
        );
    }
    println!();
    println!("Plan §2.3 budget: ~30 seconds (editor-time generation).");
    println!();
    println!("======================================================");
}

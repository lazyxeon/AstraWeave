//! GPU upload boundary.
//!
//! [`gpu_f32`] is the only `f32` conversion in this crate. The scar field
//! itself never stores a float. Callers upload [`GpuScarVolume::rgba`] and
//! turn the integer origin into a uniform at the renderer.

use crate::field::{ScarField, CELL_METRES, GPU_DIM, PEAK_TENSION};
use crate::fixed::Fixed64;

/// `0.06 / PEAK_TENSION` in Q32.32, truncated.
/// `0.00125 * 2^32 = 5368709.12`.
const DENSITY_PER_UNIT_RAW: i64 = 5_368_709;

/// World-space scar volume for the existing froxel density pass.
///
/// `rgba` is the only float payload: R = extra density, G = forward-scatter
/// weight, B = amber-violet tint weight, A = 0. Texels are indexed
/// `x + y*dim + z*dim*dim`.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuScarVolume {
    pub origin_metres: [i32; 3],
    pub extent_metres: [i32; 3],
    pub dims: [u32; 3],
    pub rgba: Vec<[f32; 4]>,
    pub any: bool,
}

/// Q32.32 → `f32`. Same raw bits in, same IEEE-754 bits out.
#[inline]
pub fn gpu_f32(v: Fixed64) -> f32 {
    let q = (v.raw() as f64) / 4294967296.0_f64;
    q as f32
}

impl ScarField {
    /// Rasterize occupied cells into a fixed `GPU_DIM`³ window around
    /// `focus_metres`. Cells outside the window are not in this upload;
    /// they remain in the field.
    pub fn bake_gpu_volume(&self, focus_metres: [i32; 3]) -> GpuScarVolume {
        let mut rgba = Vec::new();
        let (origin_metres, extent_metres, dims, any) = self.bake_into(focus_metres, &mut rgba);
        GpuScarVolume {
            origin_metres,
            extent_metres,
            dims,
            rgba,
            any,
        }
    }

    /// Fill `rgba` in place. A buffer that already has `GPU_DIM`³ texels is
    /// cleared and reused, so a settled frame does not allocate.
    pub fn bake_into(
        &self,
        focus_metres: [i32; 3],
        rgba: &mut Vec<[f32; 4]>,
    ) -> ([i32; 3], [i32; 3], [u32; 3], bool) {
        let dim = GPU_DIM as i32;
        let half = dim / 2;
        let focus = self.cell_of_metres(focus_metres);
        let origin_cell = [focus[0] - half, focus[1] - half, focus[2] - half];
        let texels = (GPU_DIM as usize).pow(3);
        if rgba.len() != texels {
            rgba.clear();
            rgba.resize(texels, [0.0; 4]);
        } else {
            for texel in rgba.iter_mut() {
                *texel = [0.0; 4];
            }
        }
        let mut any = false;
        let density_scale = Fixed64::from_raw(DENSITY_PER_UNIT_RAW);
        let peak = Fixed64::from_int(PEAK_TENSION);
        for (key, cell) in self.cells() {
            let tx = key[0] - origin_cell[0];
            let ty = key[1] - origin_cell[1];
            let tz = key[2] - origin_cell[2];
            if tx < 0 || ty < 0 || tz < 0 || tx >= dim || ty >= dim || tz >= dim {
                continue;
            }
            let tension = cell.effective();
            if tension.raw() <= 0 {
                continue;
            }
            let clamped = tension.min(peak);
            let weight = clamped.mul_ratio(1, PEAK_TENSION);
            let density = clamped.mul(density_scale);
            let w = gpu_f32(weight);
            let d = gpu_f32(density);
            let index = (tx as usize)
                + (ty as usize) * GPU_DIM as usize
                + (tz as usize) * (GPU_DIM as usize) * (GPU_DIM as usize);
            rgba[index] = [d, w, w, 0.0];
            any = true;
        }
        let extent = dim.saturating_mul(CELL_METRES);
        (
            [
                origin_cell[0].saturating_mul(CELL_METRES),
                origin_cell[1].saturating_mul(CELL_METRES),
                origin_cell[2].saturating_mul(CELL_METRES),
            ],
            [extent, extent, extent],
            [GPU_DIM, GPU_DIM, GPU_DIM],
            any,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::ScarField;
    use crate::fixed::Fixed64;

    #[test]
    fn peak_weight_is_one_and_density_stays_local() {
        let peak = Fixed64::from_int(PEAK_TENSION);
        let weight = gpu_f32(peak.mul_ratio(1, PEAK_TENSION));
        assert_eq!(weight.to_bits(), 1.0_f32.to_bits());
        let density = gpu_f32(peak.mul(Fixed64::from_raw(DENSITY_PER_UNIT_RAW)));
        assert!(density > 0.05 && density < 0.07, "{density}");
    }

    #[test]
    fn density_sits_on_the_world_cell_that_was_injected() {
        let mut field = ScarField::new();
        field.inject_death(0, 0, 0);
        let volume = field.bake_gpu_volume([0, 0, 0]);
        assert_eq!(volume.dims, [GPU_DIM, GPU_DIM, GPU_DIM]);
        assert!(volume.any);
        // Focus cell 0 is the center texel of the window, not `index % 64`.
        let center = GPU_DIM as usize / 2;
        let index =
            center + center * GPU_DIM as usize + center * GPU_DIM as usize * GPU_DIM as usize;
        assert!(
            volume.rgba[index][0] > 0.05,
            "density {}",
            volume.rgba[index][0]
        );
        assert!(
            volume.rgba[index][1] > 0.9,
            "anisotropy weight {}",
            volume.rgba[index][1]
        );
        assert!(
            volume.rgba[index][2] > 0.9,
            "tint weight {}",
            volume.rgba[index][2]
        );
        assert_eq!(volume.rgba[0], [0.0, 0.0, 0.0, 0.0]);
        // A second bake of the same field is bit-identical.
        let again = field.bake_gpu_volume([0, 0, 0]);
        assert_eq!(volume.rgba, again.rgba);
        assert_eq!(volume.origin_metres, again.origin_metres);
    }
}

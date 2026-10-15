//! Grid columns published by the local field. Sampled before [`crate::AnalyticWater`].

use crate::{WaterQuery, WaterSample};
use glam::Vec3;

pub const COLUMN_CAP: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterColumn {
    pub x: i32,
    pub z: i32,
    /// World Y of the surface, in metres. Integer metres are exact in `f32`.
    pub surface_y: f32,
    pub density: f32,
    pub ice: bool,
}

impl WaterColumn {
    pub const EMPTY: Self = Self {
        x: 0,
        z: 0,
        surface_y: 0.0,
        density: 0.0,
        ice: false,
    };
}

#[derive(Clone, Debug)]
pub struct ColumnWater {
    cols: Vec<WaterColumn>,
}

impl ColumnWater {
    pub fn new() -> Self {
        Self {
            cols: Vec::with_capacity(COLUMN_CAP),
        }
    }

    pub fn len(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    pub fn clear(&mut self) {
        self.cols.clear();
    }

    /// Replace the table. `rows` must be sorted by `(x, z)` with one row per column.
    /// Extra rows past [`COLUMN_CAP`] are dropped. Capacity is reserved in [`Self::new`].
    pub fn fill(&mut self, rows: &[WaterColumn]) {
        self.cols.clear();
        let n = rows.len().min(COLUMN_CAP);
        self.cols.extend_from_slice(&rows[..n]);
    }

    pub fn rows(&self) -> &[WaterColumn] {
        &self.cols
    }
}

impl Default for ColumnWater {
    fn default() -> Self {
        Self::new()
    }
}

impl WaterQuery for ColumnWater {
    fn sample(&self, point: Vec3) -> Option<WaterSample> {
        if self.cols.is_empty() || !point.x.is_finite() || !point.z.is_finite() {
            return None;
        }
        let x = point.x.div_euclid(1.0) as i32;
        let z = point.z.div_euclid(1.0) as i32;
        let cols = self.cols.as_slice();
        let mut lo = 0usize;
        let mut hi = cols.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let c = &cols[mid];
            match (c.x, c.z).cmp(&(x, z)) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    return Some(WaterSample {
                        surface_height: c.surface_y,
                        density: c.density,
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_column_table_samples_none() {
        let table = ColumnWater::new();
        assert!(table.sample(Vec3::ZERO).is_none());
    }

    #[test]
    fn binary_search_hits_the_cell_and_misses_the_neighbor() {
        let mut table = ColumnWater::new();
        table.fill(&[
            WaterColumn {
                x: -2,
                z: 0,
                surface_y: 3.0,
                density: 1000.0,
                ice: false,
            },
            WaterColumn {
                x: 4,
                z: 1,
                surface_y: 2.0,
                density: 0.0,
                ice: true,
            },
        ]);
        let hit = table.sample(Vec3::new(4.2, 0.0, 1.8)).unwrap();
        assert_eq!(hit.surface_height, 2.0);
        assert_eq!(hit.density, 0.0);
        // Cell -2 covers world x in [-2, -1). -2.1 is the neighbor cell.
        assert!(table.sample(Vec3::new(-1.9, 0.0, 0.2)).is_some());
        assert!(table.sample(Vec3::new(-2.1, 0.0, 0.2)).is_none());
        assert!(table.sample(Vec3::new(5.0, 0.0, 1.0)).is_none());
    }
}

//! Sorted column streaming. Eviction order is a `BTreeSet` difference,
//! so it does not follow a process-random `HashSet`.

use std::collections::{BTreeMap, BTreeSet};

/// Loaded XZ chunk coordinates. Y is always 0 for the column ground.
#[derive(Clone, Debug, Default)]
pub struct ColumnStream {
    loaded: BTreeSet<(i32, i32)>,
    lod: BTreeMap<(i32, i32), u8>,
}

/// Distance LOD for one loaded chunk. `morph_milli` is 0..1000 toward the
/// coarser height anchor. The map walks chunks in `(x, z)` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodSample {
    pub x: i32,
    pub z: i32,
    pub level: u8,
    pub morph_milli: u16,
}

/// Hysteresis so a chunk does not flip LOD on a one-chunk camera wobble.
///
/// Level 0 holds until distance 4, then becomes 1. Level 1 falls back only
/// at distance <= 1 and becomes 2 at distance >= 8. Level 2 falls to 1 at
/// distance <= 5. Morph covers the chunk before an outward switch.
pub fn lod_step(previous: u8, dist: i32) -> (u8, u16) {
    let level = match previous {
        0 if dist >= 8 => 2,
        0 if dist >= 4 => 1,
        1 if dist <= 1 => 0,
        1 if dist >= 8 => 2,
        2 if dist <= 1 => 0,
        2 if dist <= 5 => 1,
        other => other,
    };
    let morph = match (previous, dist) {
        (0, 2) | (0, 3) => ((dist - 2) * 1000) as u16,
        (1, 6) | (1, 7) => ((dist - 6) * 1000) as u16,
        _ => 0,
    };
    (level, morph.min(1000))
}

/// Chunks to create and chunks to drop, both in ascending `(x, z)` order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamDelta {
    pub added: Vec<(i32, i32)>,
    pub removed: Vec<(i32, i32)>,
}

impl ColumnStream {
    pub fn new() -> Self {
        Self {
            loaded: BTreeSet::new(),
            lod: BTreeMap::new(),
        }
    }

    pub fn loaded(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.loaded.iter().copied()
    }

    /// Chebyshev ring around `(center_x, center_z)`.
    pub fn around(&mut self, center_x: i32, center_z: i32, radius: i32) -> StreamDelta {
        let mut keep = BTreeSet::new();
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dz.abs()) > radius {
                    continue;
                }
                keep.insert((center_x.saturating_add(dx), center_z.saturating_add(dz)));
            }
        }
        let added: Vec<(i32, i32)> = keep.difference(&self.loaded).copied().collect();
        let removed: Vec<(i32, i32)> = self.loaded.difference(&keep).copied().collect();
        self.loaded = keep;
        self.lod.retain(|coord, _| self.loaded.contains(coord));
        StreamDelta { added, removed }
    }

    /// LOD for every loaded chunk, in `(x, z)` order. The same camera path
    /// produces the same levels because the previous level is stored here
    /// and chunks are visited in sorted order.
    pub fn lod_for_camera(&mut self, cam_x: i32, cam_z: i32) -> Vec<LodSample> {
        let coords: Vec<(i32, i32)> = self.loaded.iter().copied().collect();
        let mut samples = Vec::with_capacity(coords.len());
        for (x, z) in coords {
            let dist = (x - cam_x).abs().max((z - cam_z).abs());
            let previous = self.lod.get(&(x, z)).copied().unwrap_or(0);
            let (level, morph_milli) = lod_step(previous, dist);
            self.lod.insert((x, z), level);
            samples.push(LodSample {
                x,
                z,
                level,
                morph_milli,
            });
        }
        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eviction_order_is_sorted_including_negative_chunks() {
        let mut stream = ColumnStream::new();
        let first = stream.around(0, 0, 1);
        assert!(first.removed.is_empty());
        assert_eq!(first.added.first().copied(), Some((-1, -1)));
        assert_eq!(first.added.last().copied(), Some((1, 1)));
        let second = stream.around(3, 0, 0);
        assert_eq!(second.added, vec![(3, 0)]);
        assert!(second.removed.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(second.removed.contains(&(-1, -1)));
        let back = stream.around(0, 0, 1);
        assert!(back.added.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(back.removed, vec![(3, 0)]);
    }

    #[test]
    fn lod_hysteresis_is_stable_across_two_walks() {
        let mut a = ColumnStream::new();
        let mut b = ColumnStream::new();
        let _ = a.around(0, 0, 8);
        let _ = b.around(0, 0, 8);
        let first_a = a.lod_for_camera(0, 0);
        let first_b = b.lod_for_camera(0, 0);
        assert_eq!(first_a, first_b);
        let held = first_a.iter().find(|sample| sample.x == 3 && sample.z == 0);
        assert_eq!(
            held.map(|sample| (sample.level, sample.morph_milli)),
            Some((0, 1000))
        );
        assert_eq!(a.lod_for_camera(0, 0), b.lod_for_camera(0, 0));

        let mut edge = ColumnStream::new();
        let _ = edge.around(4, 0, 0);
        assert_eq!(
            edge.lod_for_camera(0, 0),
            vec![LodSample {
                x: 4,
                z: 0,
                level: 1,
                morph_milli: 0
            }]
        );
        // Distance 2 does not drop level 1. The fallback is distance <= 1.
        let held_mid = edge.lod_for_camera(2, 0);
        assert_eq!(held_mid[0].level, 1);
        let back = edge.lod_for_camera(4, 0);
        assert_eq!(back[0].level, 0);
    }
}

//! Sorted column streaming. Eviction order is a `BTreeSet` difference,
//! so it does not follow a process-random `HashSet`.

use std::collections::BTreeSet;

/// Loaded XZ chunk coordinates. Y is always 0 for the column ground.
#[derive(Clone, Debug, Default)]
pub struct ColumnStream {
    loaded: BTreeSet<(i32, i32)>,
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
        StreamDelta { added, removed }
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
}

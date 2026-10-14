//! Deterministic hierarchical navigable small world (HNSW) index.
//!
//! Levels come from an integer geometric draw, not `f64::ln`, so the graph
//! does not depend on libm. Neighbor lists are ordered by
//! `(distance.total_cmp, id)`. A search whose `ef` is at least the node
//! count scores every stored vector, so the top-k matches a full scan.
//! A smaller `ef` walks only the graph and is still deterministic.

use rand::RngCore;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const M: usize = 8;
const M0: usize = 16;
const EF_CONSTRUCTION: usize = 32;
const MAX_LEVEL: usize = 8;
const LEVEL_SEED: u64 = 0xA17A_5EED_0000_0001;

pub struct HnswIndex {
    dist: fn(&[f32], &[f32]) -> f32,
    nodes: Vec<Node>,
    by_id: BTreeMap<String, u32>,
    entry: Option<u32>,
    max_level: usize,
    next_ordinal: u64,
}

struct Node {
    id: String,
    vector: Vec<f32>,
    links: Vec<Vec<u32>>,
}

impl HnswIndex {
    pub fn new(dist: fn(&[f32], &[f32]) -> f32) -> Self {
        Self {
            dist,
            nodes: Vec::new(),
            by_id: BTreeMap::new(),
            entry: None,
            max_level: 0,
            next_ordinal: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.links.iter().map(Vec::len).sum::<usize>())
            .sum()
    }

    pub fn clear(&mut self) {
        let dist = self.dist;
        *self = Self::new(dist);
    }

    /// Rebuild from `(id, vector)` pairs. Pairs are inserted in id order so
    /// the graph does not depend on the input order.
    pub fn rebuild(&mut self, mut items: Vec<(String, Vec<f32>)>) {
        items.sort_by(|a, b| a.0.cmp(&b.0));
        let dist = self.dist;
        *self = Self::new(dist);
        for (id, vector) in items {
            self.insert(id, vector);
        }
    }

    pub fn insert(&mut self, id: String, vector: Vec<f32>) {
        if self.by_id.contains_key(&id) {
            return;
        }
        let level = level_for(self.next_ordinal);
        self.next_ordinal += 1;
        let idx = self.nodes.len() as u32;
        self.nodes.push(Node {
            id: id.clone(),
            vector: vector.clone(),
            links: vec![Vec::new(); level + 1],
        });
        self.by_id.insert(id, idx);

        let Some(mut entry) = self.entry else {
            self.entry = Some(idx);
            self.max_level = level;
            return;
        };

        let query = vector;
        if self.max_level > level {
            for layer in ((level + 1)..=self.max_level).rev() {
                let nearest = self.search_layer(&query, entry, 1, layer);
                if let Some(&(node, _)) = nearest.first() {
                    entry = node;
                }
            }
        }

        let stop = level.min(self.max_level);
        for layer in (0..=stop).rev() {
            let max_links = if layer == 0 { M0 } else { M };
            let mut found = self.search_layer(&query, entry, EF_CONSTRUCTION, layer);
            found.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
            if found.len() > max_links {
                found.truncate(max_links);
            }
            let neighbors: Vec<u32> = found.iter().map(|(node, _)| *node).collect();
            for neighbor in neighbors {
                self.link_bidirectional(idx, neighbor, layer, max_links);
            }
            if let Some(&(node, _)) = found.first() {
                entry = node;
            }
        }

        if level > self.max_level {
            self.max_level = level;
            self.entry = Some(idx);
        }
    }

    /// `ef >= len` is exact. Smaller `ef` returns the graph beam only.
    pub fn search(&self, query: &[f32], k: usize, ef: usize) -> Vec<(String, f32)> {
        if k == 0 || self.nodes.is_empty() {
            return Vec::new();
        }
        let Some(entry) = self.entry else {
            return Vec::new();
        };

        let mut ep = entry;
        if self.max_level > 0 {
            for layer in (1..=self.max_level).rev() {
                let nearest = self.search_layer(query, ep, 1, layer);
                if let Some(&(node, _)) = nearest.first() {
                    ep = node;
                }
            }
        }

        let ef = ef.max(k).max(1);
        let mut found = self.search_layer(query, ep, ef, 0);
        if ef >= self.len() {
            let mut seen: BTreeSet<u32> = found.iter().map(|(idx, _)| *idx).collect();
            for (idx, node) in self.nodes.iter().enumerate() {
                let idx = idx as u32;
                if seen.insert(idx) {
                    let distance = (self.dist)(query, &node.vector);
                    found.push((idx, distance));
                }
            }
        }
        found.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
        found.truncate(k);
        found
            .into_iter()
            .map(|(idx, distance)| (self.nodes[idx as usize].id.clone(), distance))
            .collect()
    }

    fn search_layer(&self, query: &[f32], entry: u32, ef: usize, layer: usize) -> Vec<(u32, f32)> {
        let dist = self.dist;
        let mut visited = BTreeSet::new();
        let entry_distance = dist(query, &self.nodes[entry as usize].vector);
        visited.insert(entry);
        let mut candidates = vec![(entry, entry_distance)];
        let mut best = vec![(entry, entry_distance)];

        while !candidates.is_empty() {
            candidates.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
            let (current, current_distance) = candidates.remove(0);
            best.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
            let worst = best
                .last()
                .map(|(_, distance)| *distance)
                .unwrap_or(current_distance);
            if best.len() >= ef && cmp_dist(current_distance, worst) == Ordering::Greater {
                break;
            }
            let links = self.nodes[current as usize]
                .links
                .get(layer)
                .cloned()
                .unwrap_or_default();
            for neighbor in links {
                if !visited.insert(neighbor) {
                    continue;
                }
                let distance = dist(query, &self.nodes[neighbor as usize].vector);
                best.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
                let worst = best
                    .last()
                    .map(|(_, distance)| *distance)
                    .unwrap_or(distance);
                if best.len() < ef || cmp_dist(distance, worst) != Ordering::Greater {
                    candidates.push((neighbor, distance));
                    best.push((neighbor, distance));
                    best.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
                    if best.len() > ef {
                        best.truncate(ef);
                    }
                }
            }
        }
        best
    }

    fn link_bidirectional(&mut self, a: u32, b: u32, layer: usize, max_links: usize) {
        if a == b {
            return;
        }
        self.add_link(a, b, layer, max_links);
        self.add_link(b, a, layer, max_links);
    }

    fn add_link(&mut self, from: u32, to: u32, layer: usize, max_links: usize) {
        let existing = match self.nodes.get(from as usize) {
            Some(node) => match node.links.get(layer) {
                Some(links) => links.clone(),
                None => return,
            },
            None => return,
        };
        if existing.contains(&to) {
            return;
        }
        let mut links = existing;
        links.push(to);
        if links.len() > max_links {
            let origin = self.nodes[from as usize].vector.clone();
            let dist = self.dist;
            let mut scored: Vec<(u32, f32)> = links
                .iter()
                .map(|&neighbor| {
                    let distance = dist(&origin, &self.nodes[neighbor as usize].vector);
                    (neighbor, distance)
                })
                .collect();
            scored.sort_by(|a, b| cmp_hit(*a, *b, &self.nodes));
            scored.truncate(max_links);
            links = scored.into_iter().map(|(neighbor, _)| neighbor).collect();
        }
        links.sort_unstable();
        self.nodes[from as usize].links[layer] = links;
    }
}

fn cmp_dist(a: f32, b: f32) -> Ordering {
    a.total_cmp(&b)
}

fn cmp_hit(a: (u32, f32), b: (u32, f32), nodes: &[Node]) -> Ordering {
    cmp_dist(a.1, b.1).then_with(|| nodes[a.0 as usize].id.cmp(&nodes[b.0 as usize].id))
}

fn level_for(ordinal: u64) -> usize {
    let mut rng =
        astraweave_core::SimRng::new(LEVEL_SEED, ordinal, astraweave_core::SIM_SYSTEM_HNSW);
    let mut level = 0usize;
    let mut bits = rng.next_u64();
    while level < MAX_LEVEL && (bits & 0b11) == 0 {
        level += 1;
        bits >>= 2;
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manhattan(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    }

    fn brute(points: &[(String, Vec<f32>)], query: &[f32], k: usize) -> Vec<String> {
        let mut scored: Vec<(String, f32)> = points
            .iter()
            .map(|(id, vector)| (id.clone(), manhattan(query, vector)))
            .collect();
        scored.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        scored.truncate(k);
        scored.into_iter().map(|(id, _)| id).collect()
    }

    #[test]
    fn exact_search_matches_full_scan_and_graph_has_edges() {
        let mut points = Vec::new();
        for i in 0..24 {
            points.push((format!("p{i:02}"), vec![(i * 3) as f32, (i % 5) as f32]));
        }
        let mut index = HnswIndex::new(manhattan);
        for (id, vector) in &points {
            index.insert(id.clone(), vector.clone());
        }
        assert!(index.edge_count() > 0, "HNSW insert must link nodes");
        let query = vec![10.0, 2.0];
        let exact: Vec<String> = index
            .search(&query, 5, index.len())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(exact, brute(&points, &query, 5));
        let approx = index.search(&query, 1, 1);
        assert_eq!(approx.len(), 1);
    }

    #[test]
    fn rebuild_is_independent_of_insertion_order() {
        let mut forward = HnswIndex::new(manhattan);
        let mut backward = HnswIndex::new(manhattan);
        let points: Vec<(String, Vec<f32>)> = (0..12)
            .map(|i| (format!("n{i:02}"), vec![i as f32, 1.0]))
            .collect();
        forward.rebuild(points.clone());
        let mut reversed = points.clone();
        reversed.reverse();
        backward.rebuild(reversed);
        let query = vec![5.0, 1.0];
        assert_eq!(
            forward.search(&query, 4, forward.len()),
            backward.search(&query, 4, backward.len())
        );
    }
}

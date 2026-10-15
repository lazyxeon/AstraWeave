//! Fixed-capacity nav tiles.
//!
//! A tick commits at most [`TILE_COMMITS_PER_TICK`] tiles, lowest `(cx, cz)`
//! first. The commit writes into arrays that already exist. Path queries write
//! into [`NavScratch`]. Neither takes a lock.
//!
//! A worker may fill a [`TileSideBuffer`]. The sim copies it in only when
//! `input_hash` equals the hash of the sources the sim would build itself.
//!
//! # Destruction seam
//!
//! [`NavChangeKind`] is the nav-side name for three commands from the
//! destruction log (PR #243): `Explosion`, `Brush`, and `Detach`. `Impact`
//! and `Anchor` do not mark tiles. Quantize the command's box to milli-units
//! once at the edge and call [`nav_change_from_kind`].

use crate::Triangle;
use glam::Vec3;

/// World span of one tile, in meters.
pub const TILE_SIZE_M: f32 = 32.0;
/// Tiles rebuilt on the sim thread in one tick.
pub const TILE_COMMITS_PER_TICK: usize = 4;
/// Whole-frame budget the commit and the planner are checked against.
pub const FRAME_BUDGET_MS: f32 = 11.0;

const MAX_SLOTS: usize = 16;
const MAX_TRIS: usize = 24;
const MAX_LOCAL_NEIGHBORS: usize = 8;
const MAX_PORTALS: usize = 48;
const MAX_DIRTY: usize = 16;
const MAX_PENDING: usize = 16;
const MAX_SITES: usize = 16;
const MAX_NODES: usize = MAX_SLOTS * MAX_TRIS;
const MAX_WAYPOINTS: usize = 32;

/// Which destruction command produced a [`NavChange`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavChangeKind {
    Explosion,
    Brush,
    Detach,
}

/// One walkable-region invalidation, in milli-units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavChange {
    pub tick: u64,
    pub seq: u32,
    pub min_milli: [i32; 3],
    pub max_milli: [i32; 3],
    pub kind: NavChangeKind,
}

impl NavChange {
    #[must_use]
    pub const fn new(
        tick: u64,
        seq: u32,
        min_milli: [i32; 3],
        max_milli: [i32; 3],
        kind: NavChangeKind,
    ) -> Self {
        Self {
            tick,
            seq,
            min_milli,
            max_milli,
            kind,
        }
    }
}

/// `Explosion`, `Brush`, and `Detach` become a [`NavChange`].
///
/// The destruction crate does not exist on this branch. Callers that already
/// have a quantized AABB pass the kind through here so both sides share one
/// enum. Kinds outside those three are not representable.
#[must_use]
pub const fn nav_change_from_kind(
    kind: NavChangeKind,
    tick: u64,
    seq: u32,
    min_milli: [i32; 3],
    max_milli: [i32; 3],
) -> NavChange {
    NavChange::new(tick, seq, min_milli, max_milli, kind)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreachSite {
    pub portal_id: u32,
    pub x_milli: i32,
    pub z_milli: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverSite {
    pub id: u32,
    pub x_milli: i32,
    pub z_milli: i32,
}

#[derive(Clone, Copy)]
struct SourceTri {
    verts: [Vec3; 3],
}

#[derive(Clone, Copy)]
struct TileTri {
    verts: [Vec3; 3],
    center: Vec3,
    neighbors: [u16; MAX_LOCAL_NEIGHBORS],
    neighbor_count: u8,
    live: bool,
}

#[derive(Clone, Copy)]
struct Portal {
    a_slot: u16,
    a_tri: u16,
    b_slot: u16,
    b_tri: u16,
    id: u32,
    x_milli: i32,
    z_milli: i32,
    live: bool,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    cx: i32,
    cz: i32,
    source_count: u16,
    sources: [SourceTri; MAX_TRIS],
    tri_count: u16,
    tris: [TileTri; MAX_TRIS],
    input_hash: u64,
    generation: u32,
}

/// Built tile waiting for the sim to accept or reject it.
#[derive(Clone, Copy)]
pub struct TileSideBuffer {
    pub slot: u16,
    pub input_hash: u64,
    pub ready: bool,
    tri_count: u16,
    tris: [TileTri; MAX_TRIS],
    generation: u32,
}

/// Scratch for one path query. Cleared by stamp, not freed.
pub struct NavScratch {
    stamp: u32,
    seen: [u32; MAX_NODES],
    g: [u32; MAX_NODES],
    parent: [u32; MAX_NODES],
    heap_f: [u32; MAX_NODES],
    heap_n: [u32; MAX_NODES],
    heap_len: u16,
    pub waypoints: [Vec3; MAX_WAYPOINTS],
    pub waypoint_len: u16,
    pub portal_ids: [u32; MAX_WAYPOINTS],
    pub portal_len: u16,
}

impl NavScratch {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stamp: 0,
            seen: [0; MAX_NODES],
            g: [0; MAX_NODES],
            parent: [0; MAX_NODES],
            heap_f: [0; MAX_NODES],
            heap_n: [0; MAX_NODES],
            heap_len: 0,
            waypoints: [Vec3::ZERO; MAX_WAYPOINTS],
            waypoint_len: 0,
            portal_ids: [0; MAX_WAYPOINTS],
            portal_len: 0,
        }
    }
}

impl Default for NavScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// Resident tiles, pending changes, and the last published breaches and cover.
pub struct NavTiles {
    slots: [Slot; MAX_SLOTS],
    slot_count: u16,
    dirty: [u16; MAX_DIRTY],
    dirty_len: u16,
    pending: [NavChange; MAX_PENDING],
    pending_len: u16,
    portals: [Portal; MAX_PORTALS],
    portal_count: u16,
    prev_portal_ids: [u32; MAX_PORTALS],
    prev_portal_len: u16,
    pub breaches: [BreachSite; MAX_SITES],
    pub breach_count: u16,
    pub cover: [CoverSite; MAX_SITES],
    pub cover_count: u16,
    prev_cover: [CoverSite; MAX_SITES],
    prev_cover_len: u16,
    pub lost_cover: [CoverSite; MAX_SITES],
    pub lost_cover_count: u16,
    max_step: f32,
    min_up_dot: f32,
    commits: u32,
}

impl NavTiles {
    #[must_use]
    pub fn new(max_step: f32, max_slope_deg: f32) -> Self {
        Self {
            slots: [Slot::empty(); MAX_SLOTS],
            slot_count: 0,
            dirty: [0; MAX_DIRTY],
            dirty_len: 0,
            pending: [NavChange::new(0, 0, [0; 3], [0; 3], NavChangeKind::Brush); MAX_PENDING],
            pending_len: 0,
            portals: [Portal::empty(); MAX_PORTALS],
            portal_count: 0,
            prev_portal_ids: [0; MAX_PORTALS],
            prev_portal_len: 0,
            breaches: [BreachSite {
                portal_id: 0,
                x_milli: 0,
                z_milli: 0,
            }; MAX_SITES],
            breach_count: 0,
            cover: [CoverSite {
                id: 0,
                x_milli: 0,
                z_milli: 0,
            }; MAX_SITES],
            cover_count: 0,
            prev_cover: [CoverSite {
                id: 0,
                x_milli: 0,
                z_milli: 0,
            }; MAX_SITES],
            prev_cover_len: 0,
            lost_cover: [CoverSite {
                id: 0,
                x_milli: 0,
                z_milli: 0,
            }; MAX_SITES],
            lost_cover_count: 0,
            max_step,
            min_up_dot: cos_deg(max_slope_deg),
            commits: 0,
        }
    }

    /// Place triangles into tiles and build them. Overflow past the resident
    /// cap is counted and dropped, highest centers last.
    pub fn ingest(&mut self, tris: &[Triangle]) -> usize {
        let mut placed = 0usize;
        for tri in tris {
            if self.place_source(tri.clone()) {
                placed += 1;
            }
        }
        self.rebuild_all_occupied();
        self.rebuild_portals_and_sites();
        placed
    }

    /// Queue a change. Application order is `(tick, seq)`, not insertion order.
    pub fn mark(&mut self, change: NavChange) {
        if self.pending_len as usize >= MAX_PENDING {
            return;
        }
        let n = self.pending_len as usize;
        self.pending[n] = change;
        self.pending_len += 1;
        let mut i = n;
        while i > 0 {
            let prev = self.pending[i - 1];
            let cur = self.pending[i];
            let out_of_order =
                prev.tick > cur.tick || (prev.tick == cur.tick && prev.seq > cur.seq);
            if !out_of_order {
                break;
            }
            self.pending[i - 1] = cur;
            self.pending[i] = prev;
            i -= 1;
        }
    }

    /// Drop source triangles that intersect queued changes, then commit at
    /// most [`TILE_COMMITS_PER_TICK`] dirty tiles.
    pub fn commit_tick(&mut self) -> usize {
        self.apply_pending();
        self.sort_dirty();
        let n = (self.dirty_len as usize).min(TILE_COMMITS_PER_TICK);
        let mut ids = [0u16; TILE_COMMITS_PER_TICK];
        ids[..n].copy_from_slice(&self.dirty[..n]);
        let mut done = 0usize;
        for id in ids.iter().take(n) {
            let buf = self.fill_side_buffer(*id);
            if self.accept_side_buffer(&buf) {
                done += 1;
            }
        }
        if done > 0 {
            self.rebuild_portals_and_sites();
        }
        done
    }

    /// Copy a synchronous build into `buf`. The sim accepts it later.
    #[must_use]
    pub fn fill_side_buffer(&self, slot: u16) -> TileSideBuffer {
        if slot as usize >= MAX_SLOTS || !self.slots[slot as usize].occupied {
            return TileSideBuffer {
                slot,
                input_hash: 0,
                ready: false,
                tri_count: 0,
                tris: [TileTri::empty(); MAX_TRIS],
                generation: 0,
            };
        }
        let built = build_slot(&self.slots[slot as usize], self.max_step, self.min_up_dot);
        TileSideBuffer {
            slot,
            input_hash: built.0,
            ready: true,
            tri_count: built.1,
            tris: built.2,
            generation: self.slots[slot as usize].generation.wrapping_add(1),
        }
    }

    /// Accept a worker buffer only when its input hash matches the sources.
    pub fn accept_side_buffer(&mut self, buf: &TileSideBuffer) -> bool {
        let slot = buf.slot as usize;
        if !buf.ready || slot >= MAX_SLOTS || !self.slots[slot].occupied {
            return false;
        }
        let expected = hash_sources(
            &self.slots[slot].sources,
            self.slots[slot].source_count as usize,
            self.max_step,
            self.min_up_dot,
        );
        if buf.input_hash != expected || buf.generation == 0 {
            return false;
        }
        self.slots[slot].tris = buf.tris;
        self.slots[slot].tri_count = buf.tri_count;
        self.slots[slot].input_hash = expected;
        self.slots[slot].generation = self.slots[slot].generation.wrapping_add(1);
        self.commits = self.commits.saturating_add(1);
        self.clear_dirty_slot(buf.slot);
        true
    }

    #[must_use]
    pub fn dirty_len(&self) -> u16 {
        self.dirty_len
    }

    #[must_use]
    pub fn commits(&self) -> u32 {
        self.commits
    }

    #[must_use]
    pub fn slot_hash(&self, cx: i32, cz: i32) -> Option<u64> {
        self.find_slot(cx, cz)
            .map(|slot| self.slots[slot].input_hash)
    }

    #[must_use]
    pub fn dirty_coord(&self, index: usize) -> Option<(i32, i32)> {
        self.dirty
            .get(index)
            .filter(|_| index < self.dirty_len as usize)
            .map(|&slot| (self.slots[slot as usize].cx, self.slots[slot as usize].cz))
    }

    /// Write a path into `scratch`. Returns the waypoint count.
    pub fn query_path(&self, scratch: &mut NavScratch, start: Vec3, goal: Vec3) -> u16 {
        scratch.waypoint_len = 0;
        scratch.portal_len = 0;
        let Some(start_n) = self.closest_node(start) else {
            return 0;
        };
        let Some(goal_n) = self.closest_node(goal) else {
            return 0;
        };
        scratch.stamp = scratch.stamp.wrapping_add(1);
        if scratch.stamp == 0 {
            scratch.seen = [0; MAX_NODES];
            scratch.stamp = 1;
        }
        let stamp = scratch.stamp;
        scratch.heap_len = 0;
        self.heap_push(scratch, 0, start_n);
        scratch.seen[start_n as usize] = stamp;
        scratch.g[start_n as usize] = 0;
        scratch.parent[start_n as usize] = start_n;
        let mut found = false;
        while scratch.heap_len > 0 {
            let node = self.heap_pop(scratch);
            if node == goal_n {
                found = true;
                break;
            }
            let g = scratch.g[node as usize];
            self.relax_neighbors(scratch, node, g, goal_n, stamp);
        }
        if !found {
            return 0;
        }
        let mut chain = [0u32; MAX_WAYPOINTS];
        let mut chain_len = 0u16;
        let mut cursor = goal_n;
        loop {
            if chain_len as usize >= MAX_WAYPOINTS {
                break;
            }
            chain[chain_len as usize] = cursor;
            chain_len += 1;
            if cursor == start_n {
                break;
            }
            let prev = scratch.parent[cursor as usize];
            if prev == cursor {
                break;
            }
            cursor = prev;
        }
        if (scratch.waypoint_len as usize) < MAX_WAYPOINTS {
            scratch.waypoints[0] = start;
            scratch.waypoint_len = 1;
        }
        let mut hop = chain_len;
        while hop > 1 {
            hop -= 1;
            self.record_portal(scratch, chain[hop as usize], chain[(hop - 1) as usize]);
            let node = chain[(hop - 1) as usize];
            if node == goal_n {
                continue;
            }
            if (scratch.waypoint_len as usize) < MAX_WAYPOINTS {
                let (slot, local) = decode_node(node);
                scratch.waypoints[scratch.waypoint_len as usize] =
                    self.slots[slot].tris[local].center;
                scratch.waypoint_len += 1;
            }
        }
        if (scratch.waypoint_len as usize) < MAX_WAYPOINTS {
            scratch.waypoints[scratch.waypoint_len as usize] = goal;
            scratch.waypoint_len += 1;
        }
        scratch.waypoint_len
    }

    fn place_source(&mut self, tri: Triangle) -> bool {
        let center = tri.center();
        let (cx, cz) = tile_coord(center.x, center.z);
        let slot = if let Some(existing) = self.find_slot(cx, cz) {
            existing
        } else if (self.slot_count as usize) < MAX_SLOTS {
            let slot = self.slot_count as usize;
            self.slots[slot].occupied = true;
            self.slots[slot].cx = cx;
            self.slots[slot].cz = cz;
            self.slot_count += 1;
            slot
        } else {
            return false;
        };
        let count = self.slots[slot].source_count as usize;
        if count >= MAX_TRIS {
            return false;
        }
        self.slots[slot].sources[count] = SourceTri {
            verts: [tri.a, tri.b, tri.c],
        };
        self.slots[slot].source_count += 1;
        true
    }

    fn find_slot(&self, cx: i32, cz: i32) -> Option<usize> {
        (0..self.slot_count as usize).find(|&slot| {
            self.slots[slot].occupied && self.slots[slot].cx == cx && self.slots[slot].cz == cz
        })
    }

    fn rebuild_all_occupied(&mut self) {
        let count = self.slot_count as usize;
        for slot in 0..count {
            if self.slots[slot].occupied {
                self.write_build(slot as u16);
            }
        }
    }

    fn write_build(&mut self, slot: u16) {
        let built = build_slot(&self.slots[slot as usize], self.max_step, self.min_up_dot);
        self.slots[slot as usize].input_hash = built.0;
        self.slots[slot as usize].tri_count = built.1;
        self.slots[slot as usize].tris = built.2;
        self.slots[slot as usize].generation = self.slots[slot as usize].generation.wrapping_add(1);
    }

    fn apply_pending(&mut self) {
        let n = self.pending_len as usize;
        for i in 0..n {
            let change = self.pending[i];
            self.drop_sources(change);
        }
        self.pending_len = 0;
    }

    fn drop_sources(&mut self, change: NavChange) {
        let count = self.slot_count as usize;
        for slot in 0..count {
            if !self.slots[slot].occupied {
                continue;
            }
            let mut kept = 0u16;
            let src_n = self.slots[slot].source_count as usize;
            let mut next = [SourceTri { verts: [Vec3::ZERO; 3] }; MAX_TRIS];
            let mut removed = false;
            for i in 0..src_n {
                if source_hits(&self.slots[slot].sources[i], change.min_milli, change.max_milli) {
                    removed = true;
                } else {
                    next[kept as usize] = self.slots[slot].sources[i];
                    kept += 1;
                }
            }
            if removed {
                self.slots[slot].sources = next;
                self.slots[slot].source_count = kept;
                self.push_dirty(slot as u16);
            } else if slot_overlaps(self.slots[slot].cx, self.slots[slot].cz, &change) {
                self.push_dirty(slot as u16);
            }
        }
    }

    fn push_dirty(&mut self, slot: u16) {
        if self.dirty_len as usize >= MAX_DIRTY {
            return;
        }
        for i in 0..self.dirty_len as usize {
            if self.dirty[i] == slot {
                return;
            }
        }
        self.dirty[self.dirty_len as usize] = slot;
        self.dirty_len += 1;
    }

    fn clear_dirty_slot(&mut self, slot: u16) {
        let mut i = 0usize;
        while i < self.dirty_len as usize {
            if self.dirty[i] == slot {
                let last = self.dirty_len as usize - 1;
                self.dirty[i] = self.dirty[last];
                self.dirty_len -= 1;
            } else {
                i += 1;
            }
        }
    }

    fn sort_dirty(&mut self) {
        let n = self.dirty_len as usize;
        for i in 1..n {
            let mut j = i;
            while j > 0 {
                let a = self.dirty[j - 1] as usize;
                let b = self.dirty[j] as usize;
                let out = self.slots[a].cx > self.slots[b].cx
                    || (self.slots[a].cx == self.slots[b].cx && self.slots[a].cz > self.slots[b].cz);
                if !out {
                    break;
                }
                self.dirty.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    fn rebuild_portals_and_sites(&mut self) {
        self.prev_portal_len = self.portal_count;
        for i in 0..self.portal_count as usize {
            self.prev_portal_ids[i] = self.portals[i].id;
        }
        self.prev_cover_len = self.cover_count;
        self.prev_cover = self.cover;
        self.portal_count = 0;
        let slots = self.slot_count as usize;
        for a in 0..slots {
            if !self.slots[a].occupied {
                continue;
            }
            for b in (a + 1)..slots {
                if !self.slots[b].occupied {
                    continue;
                }
                if !adjacent_tile(
                    self.slots[a].cx,
                    self.slots[a].cz,
                    self.slots[b].cx,
                    self.slots[b].cz,
                ) {
                    continue;
                }
                self.link_slots(a, b);
            }
        }
        self.collect_sites();
    }

    fn link_slots(&mut self, a: usize, b: usize) {
        let a_n = self.slots[a].tri_count as usize;
        let b_n = self.slots[b].tri_count as usize;
        for ia in 0..a_n {
            if !self.slots[a].tris[ia].live {
                continue;
            }
            for ib in 0..b_n {
                if !self.slots[b].tris[ib].live {
                    continue;
                }
                if !tris_share(
                    &self.slots[a].tris[ia],
                    &self.slots[b].tris[ib],
                    self.max_step,
                ) {
                    continue;
                }
                if self.portal_count as usize >= MAX_PORTALS {
                    return;
                }
                let center = self.slots[a].tris[ia].center;
                let id = portal_id(
                    self.slots[a].cx,
                    self.slots[a].cz,
                    ia as u16,
                    self.slots[b].cx,
                    self.slots[b].cz,
                    ib as u16,
                );
                let n = self.portal_count as usize;
                self.portals[n] = Portal {
                    a_slot: a as u16,
                    a_tri: ia as u16,
                    b_slot: b as u16,
                    b_tri: ib as u16,
                    id,
                    x_milli: milli(center.x),
                    z_milli: milli(center.z),
                    live: true,
                };
                self.portal_count += 1;
            }
        }
    }

    fn collect_sites(&mut self) {
        self.breach_count = 0;
        self.cover_count = 0;
        self.lost_cover_count = 0;
        for i in 0..self.portal_count as usize {
            let id = self.portals[i].id;
            let mut seen = false;
            for p in 0..self.prev_portal_len as usize {
                if self.prev_portal_ids[p] == id {
                    seen = true;
                    break;
                }
            }
            if seen || self.breach_count as usize >= MAX_SITES {
                continue;
            }
            let n = self.breach_count as usize;
            self.breaches[n] = BreachSite {
                portal_id: id,
                x_milli: self.portals[i].x_milli,
                z_milli: self.portals[i].z_milli,
            };
            self.breach_count += 1;
        }
        let slots = self.slot_count as usize;
        for slot in 0..slots {
            let n = self.slots[slot].tri_count as usize;
            for local in 0..n {
                let tri = &self.slots[slot].tris[local];
                if !tri.live || tri.neighbor_count >= 3 {
                    continue;
                }
                if self.cover_count as usize >= MAX_SITES {
                    break;
                }
                let id = ((slot as u32) << 8) | local as u32;
                let c = self.cover_count as usize;
                self.cover[c] = CoverSite {
                    id,
                    x_milli: milli(tri.center.x),
                    z_milli: milli(tri.center.z),
                };
                self.cover_count += 1;
            }
        }
        for i in 0..self.prev_cover_len as usize {
            let old = self.prev_cover[i];
            let mut still = false;
            for j in 0..self.cover_count as usize {
                if self.cover[j].id == old.id {
                    still = true;
                    break;
                }
            }
            if still || self.lost_cover_count as usize >= MAX_SITES {
                continue;
            }
            let n = self.lost_cover_count as usize;
            self.lost_cover[n] = old;
            self.lost_cover_count += 1;
        }
    }

    fn closest_node(&self, p: Vec3) -> Option<u32> {
        let mut best: Option<(u32, i64)> = None;
        let slots = self.slot_count as usize;
        for slot in 0..slots {
            let n = self.slots[slot].tri_count as usize;
            for local in 0..n {
                if !self.slots[slot].tris[local].live {
                    continue;
                }
                let d = dist2_milli(self.slots[slot].tris[local].center, p);
                let node = encode_node(slot, local);
                let replace = match best {
                    None => true,
                    Some((_, best_d)) => d < best_d,
                };
                if replace {
                    best = Some((node, d));
                }
            }
        }
        best.map(|(node, _)| node)
    }

    fn relax_neighbors(&self, scratch: &mut NavScratch, node: u32, g: u32, goal: u32, stamp: u32) {
        let (slot, local) = decode_node(node);
        let tri = &self.slots[slot].tris[local];
        for i in 0..tri.neighbor_count as usize {
            let next = encode_node(slot, tri.neighbors[i] as usize);
            self.relax(scratch, node, next, g, goal, stamp);
        }
        for i in 0..self.portal_count as usize {
            let portal = &self.portals[i];
            if !portal.live {
                continue;
            }
            let next = if portal.a_slot as usize == slot && portal.a_tri as usize == local {
                Some(encode_node(portal.b_slot as usize, portal.b_tri as usize))
            } else if portal.b_slot as usize == slot && portal.b_tri as usize == local {
                Some(encode_node(portal.a_slot as usize, portal.a_tri as usize))
            } else {
                None
            };
            if let Some(next) = next {
                self.relax(scratch, node, next, g, goal, stamp);
            }
        }
    }

    fn relax(
        &self,
        scratch: &mut NavScratch,
        from: u32,
        to: u32,
        g: u32,
        goal: u32,
        stamp: u32,
    ) {
        let (slot, local) = decode_node(to);
        if slot >= MAX_SLOTS || !self.slots[slot].tris[local].live {
            return;
        }
        let (from_slot, from_local) = decode_node(from);
        let step = manhattan(
            self.slots[from_slot].tris[from_local].center,
            self.slots[slot].tris[local].center,
        );
        let ng = g.saturating_add(step);
        let seen = scratch.seen[to as usize] == stamp;
        if seen && ng >= scratch.g[to as usize] {
            return;
        }
        scratch.seen[to as usize] = stamp;
        scratch.g[to as usize] = ng;
        scratch.parent[to as usize] = from;
        let h = manhattan(
            self.slots[slot].tris[local].center,
            self.slots[decode_node(goal).0].tris[decode_node(goal).1].center,
        );
        self.heap_push(scratch, ng.saturating_add(h), to);
    }

    fn heap_push(&self, scratch: &mut NavScratch, f: u32, node: u32) {
        if scratch.heap_len as usize >= MAX_NODES {
            return;
        }
        let mut i = scratch.heap_len as usize;
        scratch.heap_f[i] = f;
        scratch.heap_n[i] = node;
        scratch.heap_len += 1;
        while i > 0 {
            let parent = (i - 1) / 2;
            let worse = scratch.heap_f[i] < scratch.heap_f[parent]
                || (scratch.heap_f[i] == scratch.heap_f[parent]
                    && scratch.heap_n[i] < scratch.heap_n[parent]);
            if !worse {
                break;
            }
            scratch.heap_f.swap(i, parent);
            scratch.heap_n.swap(i, parent);
            i = parent;
        }
    }

    fn heap_pop(&self, scratch: &mut NavScratch) -> u32 {
        let out = scratch.heap_n[0];
        scratch.heap_len -= 1;
        let last = scratch.heap_len as usize;
        scratch.heap_f[0] = scratch.heap_f[last];
        scratch.heap_n[0] = scratch.heap_n[last];
        let mut i = 0usize;
        loop {
            let left = i * 2 + 1;
            let right = left + 1;
            if left >= scratch.heap_len as usize {
                break;
            }
            let mut best = left;
            if right < scratch.heap_len as usize {
                let take_right = scratch.heap_f[right] < scratch.heap_f[left]
                    || (scratch.heap_f[right] == scratch.heap_f[left]
                        && scratch.heap_n[right] < scratch.heap_n[left]);
                if take_right {
                    best = right;
                }
            }
            let sink = scratch.heap_f[best] < scratch.heap_f[i]
                || (scratch.heap_f[best] == scratch.heap_f[i]
                    && scratch.heap_n[best] < scratch.heap_n[i]);
            if !sink {
                break;
            }
            scratch.heap_f.swap(i, best);
            scratch.heap_n.swap(i, best);
            i = best;
        }
        out
    }

    fn record_portal(&self, scratch: &mut NavScratch, a: u32, b: u32) {
        let (sa, la) = decode_node(a);
        let (sb, lb) = decode_node(b);
        if sa == sb || scratch.portal_len as usize >= MAX_WAYPOINTS {
            return;
        }
        for i in 0..self.portal_count as usize {
            let p = &self.portals[i];
            let hit = (p.a_slot as usize == sa
                && p.a_tri as usize == la
                && p.b_slot as usize == sb
                && p.b_tri as usize == lb)
                || (p.a_slot as usize == sb
                    && p.a_tri as usize == lb
                    && p.b_slot as usize == sa
                    && p.b_tri as usize == la);
            if hit {
                scratch.portal_ids[scratch.portal_len as usize] = p.id;
                scratch.portal_len += 1;
                return;
            }
        }
    }
}

impl Slot {
    const fn empty() -> Self {
        Self {
            occupied: false,
            cx: 0,
            cz: 0,
            source_count: 0,
            sources: [SourceTri {
                verts: [Vec3::ZERO; 3],
            }; MAX_TRIS],
            tri_count: 0,
            tris: [TileTri::empty(); MAX_TRIS],
            input_hash: 0,
            generation: 0,
        }
    }
}

impl TileTri {
    const fn empty() -> Self {
        Self {
            verts: [Vec3::ZERO; 3],
            center: Vec3::ZERO,
            neighbors: [0; MAX_LOCAL_NEIGHBORS],
            neighbor_count: 0,
            live: false,
        }
    }
}

impl Portal {
    const fn empty() -> Self {
        Self {
            a_slot: 0,
            a_tri: 0,
            b_slot: 0,
            b_tri: 0,
            id: 0,
            x_milli: 0,
            z_milli: 0,
            live: false,
        }
    }
}

fn build_slot(slot: &Slot, max_step: f32, min_up_dot: f32) -> (u64, u16, [TileTri; MAX_TRIS]) {
    let hash = hash_sources(&slot.sources, slot.source_count as usize, max_step, min_up_dot);
    let mut tris = [TileTri::empty(); MAX_TRIS];
    let mut count = 0u16;
    let n = slot.source_count as usize;
    for i in 0..n {
        let verts = slot.sources[i].verts;
        let normal = tri_normal(verts);
        let up = normal_y(normal);
        if up < min_up_dot {
            continue;
        }
        let c = (count as usize).min(MAX_TRIS - 1);
        tris[c] = TileTri {
            verts,
            center: Vec3::new(
                (verts[0].x + verts[1].x + verts[2].x) / 3.0,
                (verts[0].y + verts[1].y + verts[2].y) / 3.0,
                (verts[0].z + verts[1].z + verts[2].z) / 3.0,
            ),
            neighbors: [0; MAX_LOCAL_NEIGHBORS],
            neighbor_count: 0,
            live: true,
        };
        count += 1;
        if count as usize >= MAX_TRIS {
            break;
        }
    }
    let live = count as usize;
    for i in 0..live {
        for j in (i + 1)..live {
            if tris_share(&tris[i], &tris[j], max_step)
                && (tris[i].neighbor_count as usize) < MAX_LOCAL_NEIGHBORS
                && (tris[j].neighbor_count as usize) < MAX_LOCAL_NEIGHBORS
            {
                let ai = tris[i].neighbor_count as usize;
                let bi = tris[j].neighbor_count as usize;
                tris[i].neighbors[ai] = j as u16;
                tris[j].neighbors[bi] = i as u16;
                tris[i].neighbor_count += 1;
                tris[j].neighbor_count += 1;
            }
        }
    }
    (hash, count, tris)
}

fn tri_normal(verts: [Vec3; 3]) -> Vec3 {
    let e1 = Vec3::new(
        verts[1].x - verts[0].x,
        verts[1].y - verts[0].y,
        verts[1].z - verts[0].z,
    );
    let e2 = Vec3::new(
        verts[2].x - verts[0].x,
        verts[2].y - verts[0].y,
        verts[2].z - verts[0].z,
    );
    Vec3::new(
        e1.y * e2.z - e1.z * e2.y,
        e1.z * e2.x - e1.x * e2.z,
        e1.x * e2.y - e1.y * e2.x,
    )
}

fn normal_y(n: Vec3) -> f32 {
    let len2 = n.x * n.x + n.y * n.y + n.z * n.z;
    if len2 < 1e-12 {
        return 0.0;
    }
    // Reciprocal square root via the IEEE sqrt of a product. The slope gate
    // compares this with a polynomial cosine, so the tick does not call acos.
    n.y / len2.sqrt()
}

fn tris_share(a: &TileTri, b: &TileTri, max_step: f32) -> bool {
    let mut shared = 0;
    let eps2 = 1e-6;
    for va in a.verts {
        for vb in b.verts {
            let dx = va.x - vb.x;
            let dz = va.z - vb.z;
            let dy = (va.y - vb.y).abs();
            if dx * dx + dz * dz <= eps2 && dy <= max_step {
                shared += 1;
            }
        }
    }
    shared >= 2
}

fn source_hits(src: &SourceTri, min_milli: [i32; 3], max_milli: [i32; 3]) -> bool {
    let mut lo = [i32::MAX; 3];
    let mut hi = [i32::MIN; 3];
    for v in src.verts {
        let p = [milli(v.x), milli(v.y), milli(v.z)];
        for axis in 0..3 {
            lo[axis] = lo[axis].min(p[axis]);
            hi[axis] = hi[axis].max(p[axis]);
        }
    }
    lo[0] <= max_milli[0]
        && hi[0] >= min_milli[0]
        && lo[1] <= max_milli[1]
        && hi[1] >= min_milli[1]
        && lo[2] <= max_milli[2]
        && hi[2] >= min_milli[2]
}

fn slot_overlaps(cx: i32, cz: i32, change: &NavChange) -> bool {
    let origin_x = (cx as f32 * TILE_SIZE_M * 1000.0) as i32;
    let origin_z = (cz as f32 * TILE_SIZE_M * 1000.0) as i32;
    let end_x = origin_x + (TILE_SIZE_M * 1000.0) as i32;
    let end_z = origin_z + (TILE_SIZE_M * 1000.0) as i32;
    origin_x <= change.max_milli[0]
        && end_x >= change.min_milli[0]
        && origin_z <= change.max_milli[2]
        && end_z >= change.min_milli[2]
}

fn tile_coord(x: f32, z: f32) -> (i32, i32) {
    let cx = (x / TILE_SIZE_M).floor() as i32;
    let cz = (z / TILE_SIZE_M).floor() as i32;
    (cx, cz)
}

fn adjacent_tile(ax: i32, az: i32, bx: i32, bz: i32) -> bool {
    let dx = (ax - bx).abs();
    let dz = (az - bz).abs();
    dx + dz == 1
}

fn milli(v: f32) -> i32 {
    (v * 1000.0) as i32
}

fn manhattan(a: Vec3, b: Vec3) -> u32 {
    milli(a.x).abs_diff(milli(b.x))
        + milli(a.y).abs_diff(milli(b.y))
        + milli(a.z).abs_diff(milli(b.z))
}

fn dist2_milli(a: Vec3, b: Vec3) -> i64 {
    let dx = i64::from(milli(a.x).wrapping_sub(milli(b.x)));
    let dy = i64::from(milli(a.y).wrapping_sub(milli(b.y)));
    let dz = i64::from(milli(a.z).wrapping_sub(milli(b.z)));
    dx * dx + dy * dy + dz * dz
}

fn encode_node(slot: usize, local: usize) -> u32 {
    ((slot as u32) * MAX_TRIS as u32) + local as u32
}

fn decode_node(node: u32) -> (usize, usize) {
    (
        (node / MAX_TRIS as u32) as usize,
        (node % MAX_TRIS as u32) as usize,
    )
}

fn portal_id(ax: i32, az: i32, al: u16, bx: i32, bz: i32, bl: u16) -> u32 {
    let mut h = 0x4E41_u32;
    h = h.wrapping_mul(16777619).wrapping_add(ax as u32);
    h = h.wrapping_mul(16777619).wrapping_add(az as u32);
    h = h.wrapping_mul(16777619).wrapping_add(u32::from(al));
    h = h.wrapping_mul(16777619).wrapping_add(bx as u32);
    h = h.wrapping_mul(16777619).wrapping_add(bz as u32);
    h.wrapping_mul(16777619).wrapping_add(u32::from(bl))
}

fn hash_sources(sources: &[SourceTri], count: usize, max_step: f32, min_up_dot: f32) -> u64 {
    let mut h = 0x4E41_0000_0000_0001_u64;
    h = mix(h, count as u64);
    h = mix(h, max_step.to_bits() as u64);
    h = mix(h, min_up_dot.to_bits() as u64);
    for src in sources.iter().take(count) {
        for v in src.verts {
            h = mix(h, milli(v.x) as u32 as u64);
            h = mix(h, milli(v.y) as u32 as u64);
            h = mix(h, milli(v.z) as u32 as u64);
        }
    }
    h
}

fn mix(h: u64, v: u64) -> u64 {
    h.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(v)
}

/// Polynomial cosine. Mul and add only, so the slope gate does not call libm.
fn cos_deg(deg: f32) -> f32 {
    let x = deg * 0.017_453_292;
    let x2 = x * x;
    1.0 - x2 * 0.5 + x2 * x2 * (1.0 / 24.0) - x2 * x2 * x2 * (1.0 / 720.0)
}

/// Span used by the bridge-collapse replay: a short bridge and a longer ford.
#[must_use]
pub fn bridge_and_ford() -> NavTiles {
    let mut tris = Vec::new();
    push_quad(&mut tris, 0.0, 0.0, 4.0, 16.0);
    push_quad(&mut tris, 0.0, 16.0, 4.0, 32.0);
    push_quad(&mut tris, 0.0, 32.0, 4.0, 48.0);
    push_quad(&mut tris, 4.0, 0.0, 20.0, 16.0);
    push_quad(&mut tris, 20.0, 0.0, 24.0, 16.0);
    push_quad(&mut tris, 20.0, 16.0, 24.0, 32.0);
    push_quad(&mut tris, 20.0, 32.0, 24.0, 48.0);
    push_quad(&mut tris, 4.0, 32.0, 20.0, 48.0);
    push_quad(&mut tris, 64.0, 0.0, 68.0, 4.0);
    let mut tiles = NavTiles::new(0.4, 45.0);
    let _ = tiles.ingest(&tris);
    tiles
}

fn push_quad(out: &mut Vec<Triangle>, x0: f32, z0: f32, x1: f32, z1: f32) {
    // Counter-clockwise from above.
    out.push(Triangle::new(
        Vec3::new(x0, 0.0, z0),
        Vec3::new(x0, 0.0, z1),
        Vec3::new(x1, 0.0, z0),
    ));
    out.push(Triangle::new(
        Vec3::new(x1, 0.0, z0),
        Vec3::new(x0, 0.0, z1),
        Vec3::new(x1, 0.0, z1),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_isolation_keeps_the_other_hash() {
        let mut tiles = bridge_and_ford();
        let before = tiles.slot_hash(2, 0).expect("decoy tile");
        tiles.mark(nav_change_from_kind(
            NavChangeKind::Detach,
            30,
            0,
            [-1_000, -1_000, 17_000],
            [5_000, 1_000, 31_999],
        ));
        let _ = tiles.commit_tick();
        assert_eq!(tiles.slot_hash(2, 0), Some(before));
    }

    #[test]
    fn bridge_then_ford_after_detach() {
        let mut tiles = bridge_and_ford();
        let mut scratch = NavScratch::new();
        let start = Vec3::new(1.0, 0.0, 1.0);
        let goal = Vec3::new(1.0, 0.0, 46.0);
        let n = tiles.query_path(&mut scratch, start, goal);
        assert!(
            n >= 2,
            "path {n} portals {} slots {} portal_count {}",
            scratch.portal_len,
            tiles.slot_count,
            tiles.portal_count
        );
        let before_max_x = max_waypoint_x(&scratch);
        assert!(
            before_max_x < 8_000,
            "the short path stays on the bridge, max x milli {before_max_x}"
        );
        let bridge_portals = scratch.portal_len;
        assert!(bridge_portals > 0, "the bridge crosses a tile portal");

        tiles.mark(nav_change_from_kind(
            NavChangeKind::Explosion,
            30,
            1,
            [-1_000, -1_000, 17_000],
            [5_000, 1_000, 31_999],
        ));
        assert!(tiles.commit_tick() >= 1);
        let n = tiles.query_path(&mut scratch, start, goal);
        assert!(
            n >= 2,
            "reroute path {n} portals {} lost_cover {}",
            scratch.portal_len,
            tiles.lost_cover_count
        );
        let after_max_x = max_waypoint_x(&scratch);
        assert!(
            after_max_x > 16_000,
            "the reroute uses the ford, max x milli {after_max_x}"
        );
        assert!(scratch.portal_len > 0, "the ford still crosses a tile border");
    }

    fn max_waypoint_x(scratch: &NavScratch) -> i32 {
        let mut max_x = i32::MIN;
        for i in 0..scratch.waypoint_len as usize {
            max_x = max_x.max(milli(scratch.waypoints[i].x));
        }
        max_x
    }

    #[test]
    fn changes_apply_in_tick_seq_order() {
        let mut a = five_tiles();
        let mut b = five_tiles();
        let first = nav_change_from_kind(
            NavChangeKind::Brush,
            1,
            0,
            [0, -1, 0],
            [1_000, 1, 1_000],
        );
        let second = nav_change_from_kind(
            NavChangeKind::Brush,
            1,
            1,
            [32_000, -1, 0],
            [33_000, 1, 1_000],
        );
        a.mark(second);
        a.mark(first);
        b.mark(first);
        b.mark(second);
        let _ = a.commit_tick();
        let _ = b.commit_tick();
        assert_eq!(a.slot_hash(0, 0), b.slot_hash(0, 0));
        assert_eq!(a.slot_hash(1, 0), b.slot_hash(1, 0));
    }

    #[test]
    fn four_commits_leave_the_highest_tile_dirty() {
        let mut tiles = five_tiles();
        for cx in 0..5 {
            tiles.mark(nav_change_from_kind(
                NavChangeKind::Brush,
                1,
                cx as u32,
                [cx * 32_000, -1, 0],
                [cx * 32_000 + 500, 1, 500],
            ));
        }
        let done = tiles.commit_tick();
        assert_eq!(done, TILE_COMMITS_PER_TICK);
        assert_eq!(tiles.dirty_len(), 1);
        assert_eq!(tiles.dirty_coord(0), Some((4, 0)));
    }

    #[test]
    fn mismatched_side_buffer_is_ignored() {
        let mut tiles = five_tiles();
        tiles.mark(nav_change_from_kind(
            NavChangeKind::Detach,
            1,
            0,
            [0, -1, 0],
            [500, 1, 500],
        ));
        tiles.apply_for_test();
        let slot = 0u16;
        let before = tiles.slots[0].generation;
        let mut buf = tiles.fill_side_buffer(slot);
        buf.input_hash ^= 0xFFFF;
        assert!(!tiles.accept_side_buffer(&buf));
        assert_eq!(tiles.slots[0].generation, before);
        assert!(tiles.commit_tick() >= 1);
        assert!(tiles.slots[0].generation > before);
    }

    #[test]
    fn four_tile_commits_fit_in_the_frame_budget() {
        let mut tiles = five_tiles();
        for cx in 0..4 {
            tiles.mark(nav_change_from_kind(
                NavChangeKind::Explosion,
                2,
                cx,
                [cx as i32 * 32_000, -1, 0],
                [cx as i32 * 32_000 + 500, 1, 500],
            ));
        }
        let start = std::time::Instant::now();
        let done = tiles.commit_tick();
        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
        assert_eq!(done, 4);
        assert!(
            elapsed_ms < FRAME_BUDGET_MS,
            "4 tile commits took {elapsed_ms} ms, budget is {FRAME_BUDGET_MS} ms"
        );
    }

    fn five_tiles() -> NavTiles {
        let mut tris = Vec::new();
        for cx in 0..5 {
            let x = cx as f32 * TILE_SIZE_M + 1.0;
            push_quad(&mut tris, x, 1.0, x + 2.0, 3.0);
        }
        let mut tiles = NavTiles::new(0.4, 45.0);
        let _ = tiles.ingest(&tris);
        tiles
    }

    impl NavTiles {
        fn apply_for_test(&mut self) {
            self.apply_pending();
        }
    }
}

//! Pre-sized column ring. The loaded set is the destruction slot set.
//!
//! `around` fills a stack array, sorts it in place by `(x, z)`, and
//! merge-joins it with the residents. Slot storage, the command log, and
//! the free list are allocated in `new`. The tick does not allocate.
//! At most [`MAX_REMESHES_PER_TICK`] chunks rebuild their voxel DAG per call.

use crate::column_gen::{prop_at, scatter_entity_id};
use crate::destruction_seam::{
    chunk_of, command_touches, push_command, validate, Brush, BrushOp, BrushShape, CommandError,
    CommandKind, DestructionCommand, Detach, SeamSlot, SeedColumnBase, TerrainBase, CHUNK_EXTENT,
    MAX_LOG_COMMANDS, MAX_REMESHES_PER_TICK,
};

/// Largest Chebyshev radius the ring can hold. `(2 * 8 + 1)^2 = 289`.
pub const STREAM_RADIUS_MAX: i32 = 8;
/// Slots in the ring. This is both the loaded set and the deformation slots.
pub const RING_CAP: usize =
    (STREAM_RADIUS_MAX as usize * 2 + 1) * (STREAM_RADIUS_MAX as usize * 2 + 1);

#[derive(Clone, Copy)]
struct Resident {
    x: i32,
    z: i32,
    slot: u16,
    lod: u8,
    dirty: bool,
}

const RESIDENT_EMPTY: Resident = Resident {
    x: 0,
    z: 0,
    slot: 0,
    lod: 0,
    dirty: false,
};

/// Loaded chunks and their deformation. One allocation in [`ColumnStream::new`].
pub struct ColumnStream {
    slots: Vec<SeamSlot>,
    free: [u16; RING_CAP],
    free_len: u16,
    residents: [Resident; RING_CAP],
    scratch: [Resident; RING_CAP],
    resident_len: u16,
    log: Vec<DestructionCommand>,
    seq: u32,
    rebuilds_left: u8,
    lod_buf: [LodSample; RING_CAP],
    base: SeedColumnBase,
}

/// Distance LOD for one loaded chunk. `morph_milli` is 0..1000 toward the
/// coarser height anchor.
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
/// The arrays are part of the value. Lengths are the live prefix.
#[derive(Clone, Copy)]
pub struct StreamDelta {
    added: [(i32, i32); RING_CAP],
    added_len: u16,
    removed: [(i32, i32); RING_CAP],
    removed_len: u16,
}

impl StreamDelta {
    pub fn added(&self) -> &[(i32, i32)] {
        &self.added[..self.added_len as usize]
    }

    pub fn removed(&self) -> &[(i32, i32)] {
        &self.removed[..self.removed_len as usize]
    }
}

impl PartialEq for StreamDelta {
    fn eq(&self, other: &Self) -> bool {
        self.added() == other.added() && self.removed() == other.removed()
    }
}

impl Eq for StreamDelta {}

impl std::fmt::Debug for StreamDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamDelta")
            .field("added", &self.added())
            .field("removed", &self.removed())
            .finish()
    }
}

/// LOD samples for the loaded ring. The live prefix is `len`.
#[derive(Clone, Copy)]
pub struct LodSet {
    samples: [LodSample; RING_CAP],
    len: u16,
}

impl LodSet {
    pub fn samples(&self) -> &[LodSample] {
        &self.samples[..self.len as usize]
    }
}

impl PartialEq for LodSet {
    fn eq(&self, other: &Self) -> bool {
        self.samples() == other.samples()
    }
}

impl Eq for LodSet {}

impl std::fmt::Debug for LodSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.samples()).finish()
    }
}

impl Default for ColumnStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnStream {
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(RING_CAP);
        for _ in 0..RING_CAP {
            slots.push(SeamSlot::empty());
        }
        let mut free = [0u16; RING_CAP];
        for (index, slot) in free.iter_mut().enumerate() {
            *slot = index as u16;
        }
        Self {
            slots,
            free,
            free_len: RING_CAP as u16,
            residents: [RESIDENT_EMPTY; RING_CAP],
            scratch: [RESIDENT_EMPTY; RING_CAP],
            resident_len: 0,
            log: Vec::with_capacity(MAX_LOG_COMMANDS),
            seq: 0,
            rebuilds_left: MAX_REMESHES_PER_TICK as u8,
            lod_buf: [LodSample {
                x: 0,
                z: 0,
                level: 0,
                morph_milli: 0,
            }; RING_CAP],
            base: SeedColumnBase,
        }
    }

    pub fn loaded(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.residents[..self.resident_len as usize]
            .iter()
            .map(|resident| (resident.x, resident.z))
    }

    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    /// Chebyshev ring around `(center_x, center_z)`. Radius is clamped to
    /// [`STREAM_RADIUS_MAX`]. Sort and merge use stack storage.
    pub fn around(&mut self, center_x: i32, center_z: i32, radius: i32) -> StreamDelta {
        self.rebuilds_left = MAX_REMESHES_PER_TICK as u8;
        let radius = radius.clamp(0, STREAM_RADIUS_MAX);
        let mut desired = [(0i32, 0i32); RING_CAP];
        let mut desired_len = 0usize;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                desired[desired_len] = (center_x.saturating_add(dx), center_z.saturating_add(dz));
                desired_len += 1;
            }
        }
        desired[..desired_len].sort_unstable();

        let mut removed = [(0i32, 0i32); RING_CAP];
        let mut removed_len = 0u16;
        let mut resident_index = 0usize;
        let mut desired_index = 0usize;
        let resident_len = self.resident_len as usize;
        while resident_index < resident_len || desired_index < desired_len {
            let remove_current = resident_index < resident_len
                && (desired_index >= desired_len
                    || resident_coord(&self.residents[resident_index]) < desired[desired_index]);
            if remove_current {
                let resident = self.residents[resident_index];
                self.free_slot(resident.slot);
                removed[removed_len as usize] = (resident.x, resident.z);
                removed_len += 1;
                resident_index += 1;
                continue;
            }
            let add_current = desired_index < desired_len
                && (resident_index >= resident_len
                    || desired[desired_index] < resident_coord(&self.residents[resident_index]));
            if add_current {
                desired_index += 1;
                continue;
            }
            resident_index += 1;
            desired_index += 1;
        }

        resident_index = 0;
        let mut scratch_len = 0usize;
        let mut added = [(0i32, 0i32); RING_CAP];
        let mut added_len = 0u16;
        for coord in desired[..desired_len].iter().copied() {
            while resident_index < resident_len
                && resident_coord(&self.residents[resident_index]) < coord
            {
                resident_index += 1;
            }
            if resident_index < resident_len
                && resident_coord(&self.residents[resident_index]) == coord
            {
                self.scratch[scratch_len] = self.residents[resident_index];
                resident_index += 1;
            } else {
                let slot = self.alloc_slot();
                let dirty = self.log_touches(coord);
                self.slots[slot].prepare(coord);
                self.scratch[scratch_len] = Resident {
                    x: coord.0,
                    z: coord.1,
                    slot: slot as u16,
                    lod: 0,
                    dirty,
                };
                added[added_len as usize] = coord;
                added_len += 1;
            }
            scratch_len += 1;
        }
        self.residents[..scratch_len].copy_from_slice(&self.scratch[..scratch_len]);
        self.resident_len = scratch_len as u16;
        self.rebuild_budget();
        StreamDelta {
            added,
            added_len,
            removed,
            removed_len,
        }
    }

    /// LOD for every loaded chunk, in `(x, z)` order.
    pub fn lod_for_camera(&mut self, cam_x: i32, cam_z: i32) -> LodSet {
        let len = self.resident_len as usize;
        for index in 0..len {
            let resident = &mut self.residents[index];
            let dist = (resident.x - cam_x).abs().max((resident.z - cam_z).abs());
            let (level, morph_milli) = lod_step(resident.lod, dist);
            resident.lod = level;
            self.lod_buf[index] = LodSample {
                x: resident.x,
                z: resident.z,
                level,
                morph_milli,
            };
        }
        LodSet {
            samples: self.lod_buf,
            len: self.resident_len,
        }
    }

    pub fn append_command(&mut self, cmd: DestructionCommand) -> Result<(), CommandError> {
        validate(&cmd)?;
        if self.log.len() >= MAX_LOG_COMMANDS {
            return Err(CommandError::LogFull);
        }
        let touches_voxel = !matches!(cmd.kind, CommandKind::Detach(_));
        self.log.push(cmd);
        if touches_voxel {
            self.mark_touched(cmd);
            self.rebuild_budget();
        }
        Ok(())
    }

    pub fn append_brush(&mut self, brush: Brush) -> Result<(), CommandError> {
        let cmd = DestructionCommand {
            tick: 0,
            seq: self.seq,
            entity_id: 0,
            kind: CommandKind::Brush(brush),
        };
        self.seq = self.seq.wrapping_add(1);
        self.append_command(cmd)
    }

    /// Record a scatter node as detached. The spawn record stays a function
    /// of `(seed, x, z)`. A later stream-in does not respawn this id.
    pub fn destroy_scatter(&mut self, x: i32, z: i32) -> Result<bool, CommandError> {
        if prop_at(x, z).is_none() {
            return Ok(false);
        }
        let entity_id = scatter_entity_id(x, z);
        if !self.keeps_scatter(entity_id) {
            return Ok(false);
        }
        let cmd = DestructionCommand {
            tick: 0,
            seq: self.seq,
            entity_id,
            kind: CommandKind::Detach(Detach { bond_id: entity_id }),
        };
        self.seq = self.seq.wrapping_add(1);
        self.append_command(cmd)?;
        Ok(true)
    }

    /// False when the log already detached this scatter id.
    pub fn keeps_scatter(&self, entity_id: u64) -> bool {
        !self.log.iter().any(|cmd| {
            matches!(
                cmd.kind,
                CommandKind::Detach(Detach { bond_id }) if bond_id == entity_id
            )
        })
    }

    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        let chunk = chunk_of(x, z);
        let delta = self
            .resident(chunk)
            .map(|resident| self.slots[resident.slot as usize].height_delta(x, z))
            .unwrap_or(0);
        self.base.column_top(x, z).saturating_add(delta)
    }

    pub fn occupancy_bytes(&self, chunk: (i32, i32)) -> Vec<u8> {
        let Some(resident) = self.resident(chunk) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        self.slots[resident.slot as usize].write_occupancy(&mut out);
        out
    }

    pub fn command_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for cmd in &self.log {
            push_command(&mut out, cmd);
        }
        out
    }

    pub fn dag_ready(&self, chunk: (i32, i32)) -> bool {
        self.resident(chunk)
            .is_some_and(|resident| self.slots[resident.slot as usize].dag().ready())
    }

    fn resident(&self, chunk: (i32, i32)) -> Option<&Resident> {
        self.residents[..self.resident_len as usize]
            .iter()
            .find(|resident| (resident.x, resident.z) == chunk)
    }

    fn log_touches(&self, chunk: (i32, i32)) -> bool {
        self.log.iter().any(|cmd| command_touches(cmd, chunk))
    }

    fn mark_touched(&mut self, cmd: DestructionCommand) {
        let len = self.resident_len as usize;
        for index in 0..len {
            let coord = (self.residents[index].x, self.residents[index].z);
            if command_touches(&cmd, coord) {
                self.residents[index].dirty = true;
            }
        }
    }

    fn alloc_slot(&mut self) -> usize {
        self.free_len -= 1;
        self.free[self.free_len as usize] as usize
    }

    fn free_slot(&mut self, slot: u16) {
        self.free[self.free_len as usize] = slot;
        self.free_len += 1;
    }

    fn rebuild_budget(&mut self) {
        let len = self.resident_len as usize;
        for index in 0..len {
            if self.rebuilds_left == 0 {
                break;
            }
            if !self.residents[index].dirty {
                continue;
            }
            let coord = (self.residents[index].x, self.residents[index].z);
            let slot = self.residents[index].slot as usize;
            self.slots[slot].replay(coord, &self.log);
            self.residents[index].dirty = false;
            self.rebuilds_left -= 1;
        }
    }
}

fn resident_coord(resident: &Resident) -> (i32, i32) {
    (resident.x, resident.z)
}

/// Replay one edited negative chunk, evict it, and load it again.
/// The bytes are the command log plus the occupancy after the second load.
pub fn deformed_chunk_record() -> Vec<u8> {
    let mut stream = ColumnStream::new();
    let chunk = (-1, 2);
    let _ = stream.around(chunk.0, chunk.1, 0);
    stream
        .append_brush(Brush {
            shape: BrushShape::Sphere,
            op: BrushOp::Subtract,
            origin: [-20, 4, 80],
            radius: 3,
            half_extents: [1, 1, 1],
            strength: 4,
        })
        .expect("dig fits the log");
    let mut x = chunk.0 * CHUNK_EXTENT;
    let mut detached = false;
    while x < (chunk.0 + 1) * CHUNK_EXTENT && !detached {
        let mut z = chunk.1 * CHUNK_EXTENT;
        while z < (chunk.1 + 1) * CHUNK_EXTENT {
            if stream.destroy_scatter(x, z).expect("detach fits the log") {
                detached = true;
                break;
            }
            z += 1;
        }
        x += 1;
    }
    assert!(detached, "negative chunk has a scatter spawn to detach");
    let _ = stream.around(40, 40, 0);
    let _ = stream.around(chunk.0, chunk.1, 0);
    let mut out = stream.command_bytes();
    out.extend(stream.occupancy_bytes(chunk));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eviction_order_is_sorted_including_negative_chunks() {
        let mut stream = ColumnStream::new();
        let first = stream.around(0, 0, 1);
        assert!(first.removed().is_empty());
        assert_eq!(first.added().first().copied(), Some((-1, -1)));
        assert_eq!(first.added().last().copied(), Some((1, 1)));
        let second = stream.around(3, 0, 0);
        assert_eq!(second.added(), &[(3, 0)]);
        assert!(second.removed().windows(2).all(|pair| pair[0] < pair[1]));
        assert!(second.removed().contains(&(-1, -1)));
        let back = stream.around(0, 0, 1);
        assert!(back.added().windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(back.removed(), &[(3, 0)]);
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
        let held = first_a
            .samples()
            .iter()
            .find(|sample| sample.x == 3 && sample.z == 0);
        assert_eq!(
            held.map(|sample| (sample.level, sample.morph_milli)),
            Some((0, 1000))
        );
        assert_eq!(a.lod_for_camera(0, 0), b.lod_for_camera(0, 0));

        let mut edge = ColumnStream::new();
        let _ = edge.around(4, 0, 0);
        assert_eq!(
            edge.lod_for_camera(0, 0).samples(),
            &[LodSample {
                x: 4,
                z: 0,
                level: 1,
                morph_milli: 0
            }]
        );
        let held_mid = edge.lod_for_camera(2, 0);
        assert_eq!(held_mid.samples()[0].level, 1);
        let back = edge.lod_for_camera(4, 0);
        assert_eq!(back.samples()[0].level, 0);
    }

    #[test]
    fn deformed_chunk_replays_after_eviction() {
        let mut stream = ColumnStream::new();
        let _ = stream.around(0, 0, 0);
        let base = stream.surface_y(4, 4);
        stream
            .append_brush(Brush {
                shape: BrushShape::Sphere,
                op: BrushOp::Subtract,
                origin: [4, 2, 4],
                radius: 3,
                half_extents: [1, 1, 1],
                strength: 4,
            })
            .unwrap();
        assert!(stream.dag_ready((0, 0)));
        assert!(stream.surface_y(4, 4) < base);
        let before = stream.occupancy_bytes((0, 0));
        let _ = stream.around(9, 9, 0);
        assert!(stream.occupancy_bytes((0, 0)).is_empty());
        let _ = stream.around(0, 0, 0);
        assert_eq!(stream.occupancy_bytes((0, 0)), before);
        assert!(stream.surface_y(4, 4) < base);
    }

    #[test]
    fn destroyed_scatter_stays_down_across_a_reload() {
        let mut stream = ColumnStream::new();
        let _ = stream.around(0, 0, 0);
        let id = scatter_entity_id(18, 22);
        assert!(stream.keeps_scatter(id));
        assert!(stream.destroy_scatter(18, 22).unwrap());
        assert!(!stream.keeps_scatter(id));
        assert!(!stream.destroy_scatter(18, 22).unwrap());
        let _ = stream.around(20, 20, 0);
        let _ = stream.around(0, 0, 0);
        assert!(!stream.keeps_scatter(id));
        assert!(!stream.destroy_scatter(18, 22).unwrap());
    }

    #[test]
    fn rebuild_budget_leaves_later_chunks_dirty_for_the_next_tick() {
        let mut stream = ColumnStream::new();
        let _ = stream.around(0, 0, 1);
        let coords: Vec<(i32, i32)> = stream.loaded().take(6).collect();
        for (x, z) in coords {
            stream
                .append_brush(Brush {
                    shape: BrushShape::Box,
                    op: BrushOp::Subtract,
                    origin: [x * CHUNK_EXTENT + 1, 0, z * CHUNK_EXTENT + 1],
                    radius: 0,
                    half_extents: [0, 0, 0],
                    strength: 1,
                })
                .unwrap();
        }
        let ready = stream
            .loaded()
            .filter(|coord| stream.dag_ready(*coord))
            .count();
        assert!(ready <= MAX_REMESHES_PER_TICK);
        let _ = stream.around(0, 0, 1);
        let ready_after = stream
            .loaded()
            .filter(|coord| stream.dag_ready(*coord))
            .count();
        assert!(ready_after > ready);
    }
}

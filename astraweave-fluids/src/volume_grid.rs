//! Volumetric Water Grid System
//!
//! Implements voxel-based water simulation for building/terrain interaction.
//! Inspired by Enshrouded's "Wake of Water" update with hydrostatic pressure,
//! material absorption, and U-bend flow physics.

use glam::{IVec3, UVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Material types affecting water absorption and flow
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
#[non_exhaustive]
pub enum MaterialType {
    /// Empty space - water flows freely
    #[default]
    Air = 0,
    /// Solid stone blocks - no absorption, blocks flow
    Stone = 1,
    /// Soil/dirt - low absorption (~1% per second)
    Soil = 2,
    /// Mud - high absorption (removes water quickly)
    Mud = 3,
    /// Rubble/gravel - moderate absorption
    Rubble = 4,
    /// Shroud corruption - rapid dissipation
    Shroud = 5,
    /// Glass/crystal - no absorption, transparent
    Glass = 6,
    /// Wood - very low absorption
    Wood = 7,
}

impl MaterialType {
    /// Get absorption rate per second (0.0 = no absorption, 1.0 = instant absorption)
    #[inline]
    pub fn absorption_rate(&self) -> f32 {
        match self {
            MaterialType::Air => 0.0,
            MaterialType::Stone => 0.0,
            MaterialType::Soil => 0.01,
            MaterialType::Mud => 0.5,
            MaterialType::Rubble => 0.05,
            MaterialType::Shroud => 0.8,
            MaterialType::Glass => 0.0,
            MaterialType::Wood => 0.002,
        }
    }

    /// Whether this material blocks water flow entirely
    #[inline]
    pub fn blocks_flow(&self) -> bool {
        matches!(self, MaterialType::Stone | MaterialType::Glass)
    }

    /// Whether water can exist in this cell
    #[inline]
    pub fn allows_water(&self) -> bool {
        !self.blocks_flow()
    }
}

/// A single cell in the water volume grid
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct WaterCell {
    /// Water level (0.0 = empty, 1.0 = full)
    pub level: f32,
    /// Flow velocity for visual effects
    pub velocity: Vec3,
    /// Material type at this cell
    pub material: MaterialType,
    /// Pressure at this cell (computed from water column above)
    pub pressure: f32,
    /// Temperature (affects viscosity, evaporation)
    pub temperature: f32,
    /// Flags for special states
    pub flags: CellFlags,
}

bitflags::bitflags! {
    /// Flags for special cell states
    #[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
    pub struct CellFlags: u8 {
        /// Cell is a water source (dispenser)
        const SOURCE = 0b0000_0001;
        /// Cell is a drain
        const DRAIN = 0b0000_0010;
        /// Cell has a water gate
        const GATE = 0b0000_0100;
        /// Cell is frozen
        const FROZEN = 0b0000_1000;
        /// Cell is being edited (no simulation)
        const EDITING = 0b0001_0000;
        /// Cell is persistent (won't drain naturally)
        const PERSISTENT = 0b0010_0000;
    }
}

/// Configuration for water simulation behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterSimConfig {
    /// Flow rate multiplier (1.0 = Enshrouded's 36 blocks/sec base)
    pub flow_rate: f32,
    /// Viscosity (higher = slower spreading)
    pub viscosity: f32,
    /// Gravity strength
    pub gravity: f32,
    /// Minimum water level to consider non-empty
    pub min_level: f32,
    /// Maximum pressure for U-bend flow
    pub max_pressure: f32,
    /// Evaporation rate per second (0.0 = none)
    pub evaporation_rate: f32,
    /// Temperature at which water freezes (Kelvin)
    pub freeze_temp: f32,
    /// Temperature at which water boils (Kelvin)
    pub boil_temp: f32,
    /// Enable pressure-based U-bend flow
    pub enable_pressure_flow: bool,
    /// Enable material absorption
    pub enable_absorption: bool,
}

impl Default for WaterSimConfig {
    fn default() -> Self {
        Self {
            flow_rate: 1.0,        // 36 blocks/sec base (like Enshrouded)
            viscosity: 0.1,        // Low viscosity = fast spreading
            gravity: 9.81,         // Standard gravity
            min_level: 0.001,      // 0.1% minimum
            max_pressure: 100.0,   // 100 blocks of water column max
            evaporation_rate: 0.0, // No evaporation by default
            freeze_temp: 273.15,   // 0°C
            boil_temp: 373.15,     // 100°C
            enable_pressure_flow: true,
            enable_absorption: true,
        }
    }
}

/// Volumetric water simulation grid
///
/// Uses a 3D grid of water cells to simulate water flow, filling, splitting,
/// and interaction with terrain and structures.
#[derive(Clone, Serialize, Deserialize)]
pub struct WaterVolumeGrid {
    /// Water cells (flattened 3D array)
    cells: Vec<WaterCell>,
    /// Grid dimensions (x, y, z)
    dimensions: UVec3,
    /// Cell size in world units
    cell_size: f32,
    /// World-space origin (minimum corner)
    origin: Vec3,
    /// Simulation configuration
    config: WaterSimConfig,
    /// Total water volume in the grid
    total_volume: f32,
    /// Dirty flag for GPU sync
    dirty: bool,
    /// Active cells for sparse simulation
    active_cells: Vec<usize>,
    /// Cells that need pressure recalculation
    pressure_dirty: Vec<usize>,
}

impl WaterVolumeGrid {
    /// Create a new water volume grid
    ///
    /// # Arguments
    /// * `dimensions` - Grid size in cells (x, y, z)
    /// * `cell_size` - Size of each cell in world units
    /// * `origin` - World-space position of the grid's minimum corner
    pub fn new(dimensions: UVec3, cell_size: f32, origin: Vec3) -> Self {
        let cell_count = (dimensions.x * dimensions.y * dimensions.z) as usize;
        Self {
            cells: vec![WaterCell::default(); cell_count],
            dimensions,
            cell_size,
            origin,
            config: WaterSimConfig::default(),
            total_volume: 0.0,
            dirty: false,
            active_cells: Vec::with_capacity(cell_count / 10),
            pressure_dirty: Vec::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(mut self, config: WaterSimConfig) -> Self {
        self.config = config;
        self
    }

    /// Get grid dimensions
    #[inline]
    pub fn dimensions(&self) -> UVec3 {
        self.dimensions
    }

    /// Get cell size
    #[inline]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Get world origin
    #[inline]
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// Get total water volume in cubic units
    #[inline]
    pub fn total_volume(&self) -> f32 {
        self.total_volume
    }

    /// Check if grid needs GPU sync
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag after GPU sync
    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Convert world position to grid coordinates
    #[inline]
    pub fn world_to_grid(&self, world_pos: Vec3) -> IVec3 {
        let local = (world_pos - self.origin) / self.cell_size;
        IVec3::new(local.x as i32, local.y as i32, local.z as i32)
    }

    /// Convert grid coordinates to world position (cell center)
    #[inline]
    pub fn grid_to_world(&self, grid_pos: IVec3) -> Vec3 {
        self.origin + grid_pos.as_vec3() * self.cell_size + Vec3::splat(self.cell_size * 0.5)
    }

    /// Check if grid coordinates are valid
    #[inline]
    pub fn is_valid(&self, pos: IVec3) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && pos.z >= 0
            && (pos.x as u32) < self.dimensions.x
            && (pos.y as u32) < self.dimensions.y
            && (pos.z as u32) < self.dimensions.z
    }

    /// Convert grid coordinates to flat index
    #[inline]
    fn to_index(&self, pos: IVec3) -> Option<usize> {
        if self.is_valid(pos) {
            Some(
                (pos.x as usize)
                    + (pos.y as usize) * self.dimensions.x as usize
                    + (pos.z as usize) * self.dimensions.x as usize * self.dimensions.y as usize,
            )
        } else {
            None
        }
    }

    /// Convert flat index to grid coordinates
    #[inline]
    #[allow(dead_code)]
    fn index_to_coords(&self, index: usize) -> IVec3 {
        let x = index % self.dimensions.x as usize;
        let y = (index / self.dimensions.x as usize) % self.dimensions.y as usize;
        let z = index / (self.dimensions.x as usize * self.dimensions.y as usize);
        IVec3::new(x as i32, y as i32, z as i32)
    }

    /// Get water level at grid position
    #[inline]
    pub fn get_level(&self, pos: IVec3) -> f32 {
        self.to_index(pos)
            .map(|i| self.cells[i].level)
            .unwrap_or(0.0)
    }

    /// Get water cell at grid position
    #[inline]
    pub fn get_cell(&self, pos: IVec3) -> Option<&WaterCell> {
        self.to_index(pos).map(|i| &self.cells[i])
    }

    /// Get mutable water cell at grid position
    #[inline]
    pub fn get_cell_mut(&mut self, pos: IVec3) -> Option<&mut WaterCell> {
        self.to_index(pos).map(|i| &mut self.cells[i])
    }

    /// Set water level at grid position
    pub fn set_level(&mut self, pos: IVec3, level: f32) {
        if let Some(idx) = self.to_index(pos) {
            let old_level = self.cells[idx].level;
            let new_level = level.clamp(0.0, 1.0);
            self.cells[idx].level = new_level;
            self.total_volume += (new_level - old_level) * self.cell_size.powi(3);
            self.dirty = true;

            // Track active cells
            if new_level > self.config.min_level && old_level <= self.config.min_level {
                self.active_cells.push(idx);
            }
        }
    }

    /// Add water to a cell (clamped to 0.0-1.0)
    pub fn add_water(&mut self, pos: IVec3, amount: f32) {
        if let Some(idx) = self.to_index(pos) {
            if !self.cells[idx].material.allows_water() {
                return; // Can't add water to solid cells
            }
            let old_level = self.cells[idx].level;
            let new_level = (old_level + amount).clamp(0.0, 1.0);
            self.cells[idx].level = new_level;
            self.total_volume += (new_level - old_level) * self.cell_size.powi(3);
            self.dirty = true;

            if new_level > self.config.min_level && old_level <= self.config.min_level {
                self.active_cells.push(idx);
            }
        }
    }

    /// Remove water from a cell
    pub fn remove_water(&mut self, pos: IVec3, amount: f32) -> f32 {
        if let Some(idx) = self.to_index(pos) {
            let old_level = self.cells[idx].level;
            let removed = old_level.min(amount);
            self.cells[idx].level = old_level - removed;
            self.total_volume -= removed * self.cell_size.powi(3);
            self.dirty = true;
            removed
        } else {
            0.0
        }
    }

    /// Set material type at grid position
    pub fn set_material(&mut self, pos: IVec3, material: MaterialType) {
        if let Some(idx) = self.to_index(pos) {
            self.cells[idx].material = material;
            // If setting to solid, remove any water
            if material.blocks_flow() {
                self.cells[idx].level = 0.0;
            }
            self.dirty = true;
        }
    }

    /// Apply a terrain heightfield as this volume's solid boundary (F.3 WI-4).
    ///
    /// **Deliberately terrain-agnostic**: takes a plain `&[f32]` heightfield
    /// (world-space surface Y per sample, row-major `hres_x * hres_z`), NOT an
    /// `astraweave-terrain` type. This is what keeps the dependency graph
    /// acyclic — `astraweave-fluids`/`astraweave-water` must never depend on
    /// `astraweave-terrain` (that would close
    /// `water → terrain → gameplay → physics → water`). The consumer that has
    /// both terrain and water passes `heightmap.data()` (itself a `&[f32]`,
    /// F.0 Seam 3) into this method — a one-liner with no adapter.
    ///
    /// Cells whose top lies at or below the sampled terrain height become
    /// `Stone` (solid, water removed); cells above that were terrain-`Stone`
    /// are cleared to `Air`. Re-calling with a modified heightfield is the
    /// carve-reactivity path (F.3 WI-5): lower the terrain and previously-solid
    /// cells reopen, so water flows into the new channel on the next tick.
    ///
    /// Scope: a **bounded** volume over a terrain patch. World-scale
    /// chunk-stitching and carve-driven re-sim at scale are F.3.S / future.
    /// Deterministic (fixed iteration order, no RNG/hash).
    ///
    /// # Panics
    /// If `heights.len() != hres_x * hres_z`.
    pub fn apply_terrain_boundary(&mut self, heights: &[f32], hres_x: usize, hres_z: usize) {
        assert_eq!(
            heights.len(),
            hres_x * hres_z,
            "heightfield length must equal hres_x * hres_z"
        );
        if hres_x == 0 || hres_z == 0 {
            return;
        }
        let (dx, dy, dz) = (
            self.dimensions.x as i32,
            self.dimensions.y as i32,
            self.dimensions.z as i32,
        );
        for gz in 0..dz {
            for gx in 0..dx {
                // Nearest heightfield sample for this column.
                let hx = ((gx as usize * hres_x) / dx as usize).min(hres_x - 1);
                let hz = ((gz as usize * hres_z) / dz as usize).min(hres_z - 1);
                let terrain_h = heights[hz * hres_x + hx];
                for gy in 0..dy {
                    let Some(idx) = self.to_index(IVec3::new(gx, gy, gz)) else {
                        continue;
                    };
                    let cell_top_y = self.origin.y + (gy + 1) as f32 * self.cell_size;
                    if cell_top_y <= terrain_h {
                        // Below terrain → solid floor.
                        self.cells[idx].material = MaterialType::Stone;
                        self.cells[idx].level = 0.0;
                    } else if self.cells[idx].material == MaterialType::Stone {
                        // Above the (new) terrain but was terrain-solid → reopen.
                        self.cells[idx].material = MaterialType::Air;
                    }
                }
            }
        }
        // Water may have been removed from newly-solid cells.
        self.total_volume = self
            .cells
            .iter()
            .map(|c| c.level * self.cell_size.powi(3))
            .sum();
        self.dirty = true;
    }

    /// Sample water submersion at world position
    ///
    /// Returns submersion ratio (0.0 = dry, 1.0 = fully submerged)
    pub fn sample_submersion(&self, world_pos: Vec3, height: f32) -> f32 {
        let grid_pos = self.world_to_grid(world_pos);

        // Sample multiple vertical cells
        let mut submerged_height = 0.0;
        let cells_to_check = (height / self.cell_size).ceil() as i32;

        for dy in 0..cells_to_check {
            let sample_pos = grid_pos + IVec3::new(0, dy, 0);
            let level = self.get_level(sample_pos);

            if level > 0.0 {
                let cell_contribution = if dy == cells_to_check - 1 {
                    // Partial top cell
                    let remaining = height - (dy as f32 * self.cell_size);
                    level * remaining
                } else {
                    level * self.cell_size
                };
                submerged_height += cell_contribution;
            }
        }

        (submerged_height / height).clamp(0.0, 1.0)
    }

    /// Get flow rate at grid position (for water wheels, etc.)
    pub fn get_flow_rate(&self, pos: IVec3) -> f32 {
        self.get_cell(pos)
            .map(|c| c.velocity.length())
            .unwrap_or(0.0)
    }

    /// Maximum stable timestep (F.3 WI-3 dt-stability bound).
    ///
    /// Flow per tick is `flow_rate * 36 * dt`; at the default flow_rate of 1.0
    /// a `dt` of 1/36 s already moves a full cell-worth of water in one tick.
    /// Beyond that the explicit scheme oscillates (over-transfer then
    /// back-transfer) rather than converging. `simulate` substeps any larger
    /// `dt` into chunks no greater than this, so a caller passing a large or
    /// spiky frame dt cannot corrupt state.
    pub const MAX_STABLE_DT: f32 = 1.0 / 60.0;

    /// Whether a cell blocks water flow through it — solid material OR a
    /// special-state flag (F.3 WI-2, Must-Fix #6). A closed `GATE` cell, a
    /// `FROZEN` (iced) cell, and an `EDITING` cell all act as flow barriers;
    /// previously these flags were written by `building.rs`/editor code and
    /// never read, so a "closed" gate let water through.
    #[inline]
    fn cell_flow_blocked(cell: &WaterCell) -> bool {
        cell.material.blocks_flow()
            || cell
                .flags
                .intersects(CellFlags::GATE | CellFlags::FROZEN | CellFlags::EDITING)
    }

    /// Simulate one timestep of water flow.
    ///
    /// `dt` is substepped to [`MAX_STABLE_DT`] so a large frame dt cannot
    /// destabilize the explicit scheme (F.3 WI-3).
    pub fn simulate(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        // dt-stability: split a large dt into stable substeps.
        let mut remaining = dt;
        while remaining > 0.0 {
            let step = remaining.min(Self::MAX_STABLE_DT);
            self.simulate_substep(step);
            remaining -= step;
        }
    }

    /// One stable substep (dt already clamped to ≤ MAX_STABLE_DT by `simulate`).
    fn simulate_substep(&mut self, dt: f32) {
        // Phase 1: Compute pressure from water columns
        self.compute_pressure();

        // Phase 2: Vertical flow (gravity)
        self.flow_vertical(dt);

        // Phase 3: Horizontal flow (spreading + pressure)
        self.flow_horizontal(dt);

        // Phase 4: Material absorption
        if self.config.enable_absorption {
            self.apply_absorption(dt);
        }

        // Phase 5: Process sources and drains
        self.process_sources_and_drains(dt);

        // Phase 6: Clean up empty cells from active list
        self.cleanup_active_cells();

        self.dirty = true;
    }

    /// Compute pressure based on water column height
    fn compute_pressure(&mut self) {
        // For each column, compute cumulative pressure from top to bottom
        for x in 0..self.dimensions.x as i32 {
            for z in 0..self.dimensions.z as i32 {
                let mut accumulated_pressure = 0.0;

                // Top to bottom
                for y in (0..self.dimensions.y as i32).rev() {
                    let pos = IVec3::new(x, y, z);
                    if let Some(idx) = self.to_index(pos) {
                        let level = self.cells[idx].level;
                        accumulated_pressure += level * self.config.gravity * self.cell_size;
                        self.cells[idx].pressure =
                            accumulated_pressure.min(self.config.max_pressure);
                    }
                }
            }
        }
    }

    /// Vertical flow (gravity-driven)
    fn flow_vertical(&mut self, dt: f32) {
        let flow_amount = self.config.flow_rate * 36.0 * dt; // 36 blocks/sec base

        // Process from bottom to top to avoid cascading in single frame
        for y in 0..self.dimensions.y as i32 {
            for x in 0..self.dimensions.x as i32 {
                for z in 0..self.dimensions.z as i32 {
                    let pos = IVec3::new(x, y, z);
                    let below = pos - IVec3::Y;

                    if !self.is_valid(below) {
                        continue;
                    }

                    let Some(idx) = self.to_index(pos) else {
                        continue;
                    };
                    let Some(below_idx) = self.to_index(below) else {
                        continue;
                    };

                    let current_level = self.cells[idx].level;
                    let below_level = self.cells[below_idx].level;

                    if current_level <= self.config.min_level {
                        continue;
                    }

                    // F.3 WI-2: a blocked source (frozen/editing/closed gate)
                    // does not emit; a blocked target does not receive.
                    if Self::cell_flow_blocked(&self.cells[idx])
                        || Self::cell_flow_blocked(&self.cells[below_idx])
                    {
                        continue;
                    }

                    // Flow down based on available space
                    let space_below = 1.0 - below_level;
                    let transfer = current_level.min(space_below).min(flow_amount);

                    if transfer > 0.0 {
                        self.cells[idx].level -= transfer;
                        self.cells[below_idx].level += transfer;

                        // Update velocities for visual effects
                        self.cells[idx].velocity.y = -transfer / dt;
                        self.cells[below_idx].velocity.y = transfer / dt;
                    }
                }
            }
        }
    }

    /// Horizontal flow (pressure-driven spreading)
    fn flow_horizontal(&mut self, dt: f32) {
        let flow_amount = self.config.flow_rate * 36.0 * dt * 0.25; // Slower horizontal spread
        let directions = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        // F.3 WI-3 (conservation): apply transfers IMMEDIATELY against live
        // levels rather than batching deltas and clamping. The previous
        // batched scheme read all levels up front, then applied a list of
        // deltas and `clamp(0,1)` — so a cell receiving from multiple
        // neighbors in one tick could be pushed past 1.0 and the excess was
        // silently lost (a water leak). Reading `1.0 - neighbor.level` live
        // and applying at once bounds every transfer to the recipient's real
        // free space, so total water is conserved exactly. Iteration order is
        // fixed (y,x,z, then the fixed `directions` array) → deterministic.
        for y in 0..self.dimensions.y as i32 {
            for x in 0..self.dimensions.x as i32 {
                for z in 0..self.dimensions.z as i32 {
                    let pos = IVec3::new(x, y, z);
                    let Some(idx) = self.to_index(pos) else {
                        continue;
                    };

                    if self.cells[idx].level <= self.config.min_level {
                        continue;
                    }
                    // F.3 WI-2: blocked cells (frozen/editing/closed gate) do
                    // not emit horizontal flow.
                    if Self::cell_flow_blocked(&self.cells[idx]) {
                        continue;
                    }

                    for dir in directions {
                        let Some(neighbor_idx) = self.to_index(pos + dir) else {
                            continue;
                        };
                        // ...and do not receive into blocked cells.
                        if Self::cell_flow_blocked(&self.cells[neighbor_idx]) {
                            continue;
                        }

                        // Live levels/pressures (so the recipient free-space
                        // bound below reflects transfers already applied this
                        // tick — the conservation guarantee).
                        let current_level = self.cells[idx].level;
                        let neighbor_level = self.cells[neighbor_idx].level;

                        let level_diff = current_level - neighbor_level;
                        let pressure_diff = if self.config.enable_pressure_flow {
                            (self.cells[idx].pressure - self.cells[neighbor_idx].pressure) * 0.01
                        } else {
                            0.0
                        };

                        let total_flow_potential = level_diff + pressure_diff;
                        if total_flow_potential > 0.0 {
                            let transfer = (total_flow_potential * 0.5)
                                .min(flow_amount)
                                .min(current_level)
                                .min(1.0 - neighbor_level);

                            if transfer > 0.0 {
                                self.cells[idx].level -= transfer;
                                self.cells[neighbor_idx].level += transfer;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply material absorption
    fn apply_absorption(&mut self, dt: f32) {
        for idx in 0..self.cells.len() {
            let cell = &mut self.cells[idx];
            // F.3 WI-2: PERSISTENT cells "won't drain naturally" — exempt from
            // material absorption; EDITING cells are frozen out of simulation.
            if cell
                .flags
                .intersects(CellFlags::PERSISTENT | CellFlags::EDITING)
            {
                continue;
            }
            if cell.level > 0.0 {
                let absorption = cell.material.absorption_rate() * dt;
                cell.level = (cell.level - absorption).max(0.0);
            }
        }
    }

    /// Process water sources and drains
    fn process_sources_and_drains(&mut self, dt: f32) {
        let flow_rate = self.config.flow_rate * 36.0 * dt;

        for idx in 0..self.cells.len() {
            let cell = &mut self.cells[idx];

            // F.3 WI-2: EDITING cells are frozen out of simulation entirely.
            if cell.flags.contains(CellFlags::EDITING) {
                continue;
            }

            // Sources add water
            if cell.flags.contains(CellFlags::SOURCE) {
                cell.level = (cell.level + flow_rate).min(1.0);
            }

            // Drains remove water
            if cell.flags.contains(CellFlags::DRAIN) {
                cell.level = (cell.level - flow_rate).max(0.0);
            }
        }
    }

    /// Remove empty cells from active list
    fn cleanup_active_cells(&mut self) {
        self.active_cells
            .retain(|&idx| idx < self.cells.len() && self.cells[idx].level > self.config.min_level);

        // Recalculate total volume
        self.total_volume = self
            .cells
            .iter()
            .map(|c| c.level * self.cell_size.powi(3))
            .sum();
    }

    /// Fill a region with water (flood fill from a point)
    pub fn flood_fill(&mut self, start: IVec3, target_level: f32, max_cells: usize) {
        if !self.is_valid(start) {
            return;
        }

        let mut queue = VecDeque::new();
        let mut visited = vec![false; self.cells.len()];
        let mut filled = 0;

        queue.push_back(start);

        while let Some(pos) = queue.pop_front() {
            if filled >= max_cells {
                break;
            }

            let Some(idx) = self.to_index(pos) else {
                continue;
            };

            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            if self.cells[idx].material.blocks_flow() {
                continue;
            }

            // Fill this cell
            let old_level = self.cells[idx].level;
            if old_level < target_level {
                self.cells[idx].level = target_level;
                self.total_volume += (target_level - old_level) * self.cell_size.powi(3);
                filled += 1;
            }

            // Add neighbors (horizontal only for controlled fill)
            for dir in [
                IVec3::new(1, 0, 0),
                IVec3::new(-1, 0, 0),
                IVec3::new(0, 0, 1),
                IVec3::new(0, 0, -1),
            ] {
                let neighbor = pos + dir;
                if self.is_valid(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        self.dirty = true;
    }

    /// Remove all water from a bounding box (Flame Altar feature)
    pub fn remove_water_in_bounds(&mut self, min: IVec3, max: IVec3) -> u32 {
        let mut removed_count = 0;

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let pos = IVec3::new(x, y, z);
                    if let Some(idx) = self.to_index(pos) {
                        if self.cells[idx].level > 0.0 {
                            self.total_volume -= self.cells[idx].level * self.cell_size.powi(3);
                            self.cells[idx].level = 0.0;
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            self.dirty = true;
        }

        removed_count
    }

    /// Get statistics about the water grid
    pub fn stats(&self) -> WaterGridStats {
        let mut wet_cells = 0;
        let mut total_level = 0.0;
        let mut max_level = 0.0f32;

        for cell in &self.cells {
            if cell.level > self.config.min_level {
                wet_cells += 1;
                total_level += cell.level;
                max_level = max_level.max(cell.level);
            }
        }

        WaterGridStats {
            dimensions: self.dimensions,
            total_cells: self.cells.len(),
            wet_cells,
            total_volume: self.total_volume,
            average_level: if wet_cells > 0 {
                total_level / wet_cells as f32
            } else {
                0.0
            },
            max_level,
            active_cells: self.active_cells.len(),
        }
    }

    /// Get raw cells for GPU upload
    pub fn cells(&self) -> &[WaterCell] {
        &self.cells
    }

    /// Get mutable raw cells (marks dirty)
    pub fn cells_mut(&mut self) -> &mut [WaterCell] {
        self.dirty = true;
        &mut self.cells
    }
}

/// Statistics about the water grid
#[derive(Clone, Copy, Debug)]
pub struct WaterGridStats {
    pub dimensions: UVec3,
    pub total_cells: usize,
    pub wet_cells: usize,
    pub total_volume: f32,
    pub average_level: f32,
    pub max_level: f32,
    pub active_cells: usize,
}

impl std::fmt::Display for WaterGridStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WaterGrid {}x{}x{}: {}/{} wet cells ({:.1}%), {:.1}m³ volume, avg {:.2} level",
            self.dimensions.x,
            self.dimensions.y,
            self.dimensions.z,
            self.wet_cells,
            self.total_cells,
            100.0 * self.wet_cells as f32 / self.total_cells as f32,
            self.total_volume,
            self.average_level
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = WaterVolumeGrid::new(UVec3::new(16, 16, 16), 1.0, Vec3::ZERO);
        assert_eq!(grid.dimensions(), UVec3::new(16, 16, 16));
        assert_eq!(grid.total_volume(), 0.0);
    }

    #[test]
    fn test_add_water() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
        grid.add_water(IVec3::new(4, 4, 4), 0.5);
        assert!((grid.get_level(IVec3::new(4, 4, 4)) - 0.5).abs() < 0.001);
        assert!(grid.total_volume() > 0.0);
    }

    #[test]
    fn test_vertical_flow() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
        // Add water at top
        grid.add_water(IVec3::new(4, 7, 4), 1.0);

        // Simulate several frames
        for _ in 0..10 {
            grid.simulate(0.1);
        }

        // Water should have flowed down
        assert!(grid.get_level(IVec3::new(4, 7, 4)) < 1.0);
        assert!(grid.get_level(IVec3::new(4, 0, 4)) > 0.0);
    }

    #[test]
    fn test_horizontal_spreading() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(16, 4, 16), 1.0, Vec3::ZERO);

        // Create a water source on a flat surface (solid floor at y=0, water above)
        for x in 0..16 {
            for z in 0..16 {
                grid.set_material(IVec3::new(x, 0, z), MaterialType::Stone);
            }
        }

        // Add water column above the floor
        grid.set_level(IVec3::new(8, 1, 8), 1.0);
        grid.set_level(IVec3::new(8, 2, 8), 1.0);
        grid.set_level(IVec3::new(8, 3, 8), 1.0);

        // Simulate a few steps to observe spreading
        for _ in 0..5 {
            grid.simulate(0.1);
        }

        // Check that water spread horizontally (after just 5 steps)
        let total_neighbors = grid.get_level(IVec3::new(7, 1, 8))
            + grid.get_level(IVec3::new(9, 1, 8))
            + grid.get_level(IVec3::new(8, 1, 7))
            + grid.get_level(IVec3::new(8, 1, 9));

        // Water should have redistributed horizontally
        assert!(total_neighbors > 0.0, "Water did not spread horizontally");
    }

    #[test]
    fn test_material_absorption() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
        grid.set_material(IVec3::new(4, 4, 4), MaterialType::Mud);
        grid.add_water(IVec3::new(4, 4, 4), 1.0);

        let initial = grid.get_level(IVec3::new(4, 4, 4));

        // Simulate
        grid.simulate(1.0);

        // Mud should absorb water
        assert!(grid.get_level(IVec3::new(4, 4, 4)) < initial);
    }

    #[test]
    fn test_stone_blocks_flow() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
        grid.set_material(IVec3::new(4, 3, 4), MaterialType::Stone);
        grid.add_water(IVec3::new(4, 4, 4), 1.0);

        // Simulate
        for _ in 0..10 {
            grid.simulate(0.1);
        }

        // Water should not flow into stone
        assert_eq!(grid.get_level(IVec3::new(4, 3, 4)), 0.0);
    }

    #[test]
    fn test_submersion_sampling() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
        // Fill bottom layer
        for x in 0..8 {
            for z in 0..8 {
                grid.set_level(IVec3::new(x, 0, z), 1.0);
                grid.set_level(IVec3::new(x, 1, z), 1.0);
            }
        }

        // Sample at various heights
        let sub_feet = grid.sample_submersion(Vec3::new(4.0, 0.5, 4.0), 2.0);
        assert!(sub_feet > 0.9); // Mostly submerged

        let sub_high = grid.sample_submersion(Vec3::new(4.0, 4.0, 4.0), 2.0);
        assert!(sub_high < 0.1); // Above water
    }

    #[test]
    fn test_flood_fill() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(16, 4, 16), 1.0, Vec3::ZERO);

        // Create a basin (walls around edges at y=0)
        for x in 0..16 {
            grid.set_material(IVec3::new(x, 0, 0), MaterialType::Stone);
            grid.set_material(IVec3::new(x, 0, 15), MaterialType::Stone);
        }
        for z in 0..16 {
            grid.set_material(IVec3::new(0, 0, z), MaterialType::Stone);
            grid.set_material(IVec3::new(15, 0, z), MaterialType::Stone);
        }

        // Flood fill from center
        grid.flood_fill(IVec3::new(8, 0, 8), 1.0, 1000);

        // Interior should be filled
        assert!(grid.get_level(IVec3::new(8, 0, 8)) > 0.9);
        assert!(grid.get_level(IVec3::new(7, 0, 7)) > 0.9);

        // Walls should be empty
        assert_eq!(grid.get_level(IVec3::new(0, 0, 0)), 0.0);
    }

    #[test]
    fn test_remove_water_in_bounds() {
        let mut grid = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);

        // Fill some water
        for x in 2..6 {
            for y in 0..4 {
                for z in 2..6 {
                    grid.set_level(IVec3::new(x, y, z), 1.0);
                }
            }
        }

        let initial_volume = grid.total_volume();
        assert!(initial_volume > 0.0);

        // Remove water in a sub-region
        let removed = grid.remove_water_in_bounds(IVec3::new(3, 0, 3), IVec3::new(4, 2, 4));

        assert!(removed > 0);
        assert!(grid.total_volume() < initial_volume);
    }
}

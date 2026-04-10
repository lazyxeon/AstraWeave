//! Wave Function Collapse (WFC) tile-based procedural generator.
//!
//! Implements the WFC algorithm for constrained procedural generation,
//! supporting 2D tile maps with adjacency rules.
//!
//! # Architecture
//!
//! ```text
//! TileSet (defines tiles + adjacency rules)
//!   ↓
//! WfcGrid (NxM cells, each starts with all tiles possible)
//!   ↓ observe() — collapse lowest-entropy cell
//!   ↓ propagate() — enforce constraints on neighbors  
//!   ↓ repeat until fully collapsed or contradiction
//!   ↓
//! Result: 2D grid of tile IDs
//! ```
//!
//! # References
//!
//! - Gumin (2016): Original WFC implementation
//! - Stålberg (2018): "WFC is Constraint Solving in the Wild"
//!
//! # Example
//!
//! ```ignore
//! use astraweave_pcg::wfc::*;
//!
//! let mut tileset = TileSet::new();
//! let grass = tileset.add_tile("grass");
//! let road = tileset.add_tile("road");
//! let water = tileset.add_tile("water");
//!
//! tileset.allow_adjacency(grass, grass, Direction::all());
//! tileset.allow_adjacency(grass, road, Direction::all());
//! tileset.allow_adjacency(road, road, &[Direction::North, Direction::South]);
//! tileset.allow_adjacency(grass, water, Direction::all());
//! // water cannot touch road
//!
//! let mut grid = WfcGrid::new(16, 16, &tileset);
//! let mut rng = SeedRng::new(42);
//! match grid.collapse_all(&mut rng) {
//!     Ok(()) => { /* grid.get(x, y) returns the chosen TileId */ }
//!     Err(WfcError::Contradiction { x, y }) => { /* backtrack or retry */ }
//! }
//! ```

use rand::Rng;

/// Tile identifier (index into the tileset).
pub type TileId = u16;

/// Cardinal direction for 2D adjacency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Direction {
    /// All four cardinal directions.
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// The opposite direction.
    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    /// Offset (dx, dy) for this direction.
    pub fn offset(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }
}

/// A tile definition with name and optional weight.
#[derive(Debug, Clone)]
pub struct TileDef {
    /// Human-readable name.
    pub name: String,
    /// Relative frequency weight (higher = more likely to be chosen).
    pub weight: f32,
}

/// Defines all tiles and their adjacency rules.
#[derive(Debug, Clone)]
pub struct TileSet {
    /// Tile definitions.
    tiles: Vec<TileDef>,
    /// Adjacency rules: `allowed[dir][tile_a]` contains the set of tiles
    /// that can appear in direction `dir` from `tile_a`.
    allowed: [Vec<Vec<bool>>; 4],
}

impl TileSet {
    /// Create an empty tileset.
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            allowed: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }

    /// Add a tile with default weight 1.0. Returns its ID.
    pub fn add_tile(&mut self, name: &str) -> TileId {
        self.add_tile_weighted(name, 1.0)
    }

    /// Add a tile with a custom weight. Returns its ID.
    pub fn add_tile_weighted(&mut self, name: &str, weight: f32) -> TileId {
        let id = self.tiles.len() as TileId;
        self.tiles.push(TileDef {
            name: name.to_string(),
            weight: weight.max(0.001), // prevent zero weight
        });

        // Extend adjacency tables
        let n = self.tiles.len();
        for dir_table in &mut self.allowed {
            // Expand existing rows
            for row in dir_table.iter_mut() {
                row.resize(n, false);
            }
            // Add new row
            dir_table.push(vec![false; n]);
        }

        id
    }

    /// Allow tile `a` to be adjacent to tile `b` in the given directions.
    /// This also sets the reverse: `b` can have `a` in the opposite direction.
    pub fn allow_adjacency(&mut self, a: TileId, b: TileId, directions: &[Direction]) {
        let a = a as usize;
        let b = b as usize;
        for &dir in directions {
            self.allowed[dir as usize][a][b] = true;
            self.allowed[dir.opposite() as usize][b][a] = true;
        }
    }

    /// Number of tile types.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Get tile definition by ID.
    pub fn tile(&self, id: TileId) -> &TileDef {
        &self.tiles[id as usize]
    }

    /// Check if tile `b` is allowed in direction `dir` from tile `a`.
    pub fn is_allowed(&self, a: TileId, dir: Direction, b: TileId) -> bool {
        self.allowed[dir as usize][a as usize][b as usize]
    }
}

impl Default for TileSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Error from WFC solving.
#[derive(Debug, Clone)]
pub enum WfcError {
    /// A cell has zero possible tiles remaining — the grid is unsolvable.
    Contradiction { x: usize, y: usize },
    /// The tileset is empty.
    EmptyTileSet,
}

impl std::fmt::Display for WfcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WfcError::Contradiction { x, y } => {
                write!(f, "WFC contradiction at ({x}, {y})")
            }
            WfcError::EmptyTileSet => write!(f, "WFC: empty tileset"),
        }
    }
}

impl std::error::Error for WfcError {}

/// A single cell in the WFC grid.
#[derive(Debug, Clone)]
struct Cell {
    /// Bitset of possible tile IDs (true = still possible).
    possible: Vec<bool>,
    /// Cached count of possible tiles.
    num_possible: usize,
    /// Collapsed tile (if exactly 1 possible remains).
    collapsed: Option<TileId>,
}

impl Cell {
    fn new(num_tiles: usize) -> Self {
        Self {
            possible: vec![true; num_tiles],
            num_possible: num_tiles,
            collapsed: None,
        }
    }

    fn is_collapsed(&self) -> bool {
        self.collapsed.is_some()
    }

    /// Shannon entropy (weighted by tile probabilities).
    fn entropy(&self, tiles: &[TileDef]) -> f64 {
        if self.num_possible <= 1 {
            return 0.0;
        }
        let mut sum_w = 0.0f64;
        let mut sum_wlog = 0.0f64;
        for (i, &possible) in self.possible.iter().enumerate() {
            if possible {
                let w = tiles[i].weight as f64;
                sum_w += w;
                sum_wlog += w * w.ln();
            }
        }
        if sum_w <= 0.0 {
            return 0.0;
        }
        sum_w.ln() - sum_wlog / sum_w
    }

    /// Remove a tile from possibilities. Returns true if it was actually removed.
    fn remove(&mut self, tile: TileId) -> bool {
        let idx = tile as usize;
        if idx < self.possible.len() && self.possible[idx] {
            self.possible[idx] = false;
            self.num_possible -= 1;
            if self.num_possible == 1 {
                // Auto-collapse to the single remaining tile
                for (i, &p) in self.possible.iter().enumerate() {
                    if p {
                        self.collapsed = Some(i as TileId);
                        break;
                    }
                }
            }
            true
        } else {
            false
        }
    }
}

/// 2D WFC grid.
pub struct WfcGrid {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    tileset: TileSet,
}

impl WfcGrid {
    /// Create a new grid with all cells uncollapsed.
    pub fn new(width: usize, height: usize, tileset: &TileSet) -> Self {
        let n = tileset.tile_count();
        let cells = vec![Cell::new(n); width * height];
        Self {
            width,
            height,
            cells,
            tileset: tileset.clone(),
        }
    }

    /// Grid width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Grid height.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get the collapsed tile at (x, y), or None if still uncollapsed.
    pub fn get(&self, x: usize, y: usize) -> Option<TileId> {
        self.cells[y * self.width + x].collapsed
    }

    /// Pre-collapse a cell to a specific tile. Useful for seeding the grid.
    pub fn set_tile(&mut self, x: usize, y: usize, tile: TileId) {
        let idx = y * self.width + x;
        let cell = &mut self.cells[idx];
        for i in 0..cell.possible.len() {
            cell.possible[i] = i == tile as usize;
        }
        cell.num_possible = 1;
        cell.collapsed = Some(tile);
    }

    /// Run the full WFC algorithm until all cells are collapsed.
    pub fn collapse_all<R: Rng>(&mut self, rng: &mut R) -> Result<(), WfcError> {
        if self.tileset.tile_count() == 0 {
            return Err(WfcError::EmptyTileSet);
        }

        loop {
            // Find the uncollapsed cell with minimum entropy
            let next = self.find_min_entropy(rng);
            match next {
                None => return Ok(()), // all collapsed
                Some((x, y)) => {
                    self.observe(x, y, rng)?;
                    self.propagate(x, y)?;
                }
            }
        }
    }

    /// Find the uncollapsed cell with minimum entropy.
    /// Adds small random noise to break ties.
    fn find_min_entropy<R: Rng>(&self, rng: &mut R) -> Option<(usize, usize)> {
        let mut min_entropy = f64::MAX;
        let mut best = None;

        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &self.cells[y * self.width + x];
                if cell.is_collapsed() || cell.num_possible == 0 {
                    continue;
                }
                let entropy = cell.entropy(&self.tileset.tiles) + rng.random::<f64>() * 1e-6; // noise for tie-breaking
                if entropy < min_entropy {
                    min_entropy = entropy;
                    best = Some((x, y));
                }
            }
        }

        best
    }

    /// Observe (collapse) a cell by choosing one tile weighted by probability.
    fn observe<R: Rng>(&mut self, x: usize, y: usize, rng: &mut R) -> Result<(), WfcError> {
        let idx = y * self.width + x;
        let cell = &self.cells[idx];

        if cell.num_possible == 0 {
            return Err(WfcError::Contradiction { x, y });
        }

        // Weighted random selection
        let total_weight: f32 = cell
            .possible
            .iter()
            .enumerate()
            .filter(|(_, &p)| p)
            .map(|(i, _)| self.tileset.tiles[i].weight)
            .sum();

        let mut r = rng.random::<f32>() * total_weight;
        let mut chosen = 0;
        for (i, &possible) in cell.possible.iter().enumerate() {
            if possible {
                r -= self.tileset.tiles[i].weight;
                if r <= 0.0 {
                    chosen = i;
                    break;
                }
                chosen = i; // fallback to last valid
            }
        }

        // Collapse
        let cell = &mut self.cells[idx];
        for i in 0..cell.possible.len() {
            cell.possible[i] = i == chosen;
        }
        cell.num_possible = 1;
        cell.collapsed = Some(chosen as TileId);

        Ok(())
    }

    /// Constraint propagation (arc consistency) using a worklist.
    fn propagate(&mut self, start_x: usize, start_y: usize) -> Result<(), WfcError> {
        let mut worklist = std::collections::VecDeque::new();
        worklist.push_back((start_x, start_y));

        while let Some((cx, cy)) = worklist.pop_front() {
            for dir in Direction::ALL {
                let (dx, dy) = dir.offset();
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;

                if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;

                let neighbor_idx = ny * self.width + nx;
                if self.cells[neighbor_idx].num_possible <= 1 {
                    continue;
                }

                // For each possible tile in the neighbor, check if any tile
                // in the current cell supports it
                let current_idx = cy * self.width + cx;
                let num_tiles = self.tileset.tile_count();
                let mut removals = Vec::new();

                for b in 0..num_tiles {
                    if !self.cells[neighbor_idx].possible[b] {
                        continue;
                    }
                    // Check if any tile in the current cell allows 'b' in this direction
                    let supported = (0..num_tiles).any(|a| {
                        self.cells[current_idx].possible[a]
                            && self.tileset.allowed[dir as usize][a][b]
                    });
                    if !supported {
                        removals.push(b as TileId);
                    }
                }

                let mut changed = false;
                for tile in removals {
                    if self.cells[neighbor_idx].remove(tile) {
                        changed = true;
                    }
                }

                if self.cells[neighbor_idx].num_possible == 0 {
                    return Err(WfcError::Contradiction { x: nx, y: ny });
                }

                if changed {
                    worklist.push_back((nx, ny));
                }
            }
        }

        Ok(())
    }

    /// Check if the grid is fully collapsed.
    pub fn is_fully_collapsed(&self) -> bool {
        self.cells.iter().all(|c| c.is_collapsed())
    }

    /// Count of uncollapsed cells.
    pub fn uncollapsed_count(&self) -> usize {
        self.cells.iter().filter(|c| !c.is_collapsed()).count()
    }

    /// Export the grid as a 2D vector of tile IDs (row-major).
    /// Returns None for uncollapsed cells.
    pub fn to_grid(&self) -> Vec<Vec<Option<TileId>>> {
        let mut result = Vec::with_capacity(self.height);
        for y in 0..self.height {
            let mut row = Vec::with_capacity(self.width);
            for x in 0..self.width {
                row.push(self.cells[y * self.width + x].collapsed);
            }
            result.push(row);
        }
        result
    }
}

/// Convenience: create a simple tileset for "rooms and corridors" generation.
pub fn rooms_and_corridors_tileset() -> TileSet {
    let mut ts = TileSet::new();
    let empty = ts.add_tile_weighted("empty", 3.0);
    let floor = ts.add_tile("floor");
    let wall_n = ts.add_tile("wall_north");
    let wall_e = ts.add_tile("wall_east");
    let wall_s = ts.add_tile("wall_south");
    let wall_w = ts.add_tile("wall_west");
    let corner_ne = ts.add_tile("corner_ne");
    let corner_se = ts.add_tile("corner_se");
    let corner_sw = ts.add_tile("corner_sw");
    let corner_nw = ts.add_tile("corner_nw");

    // Empty can be next to empty or walls (exterior side)
    ts.allow_adjacency(empty, empty, &Direction::ALL);
    ts.allow_adjacency(empty, wall_n, &[Direction::North]);
    ts.allow_adjacency(empty, wall_s, &[Direction::South]);
    ts.allow_adjacency(empty, wall_e, &[Direction::East]);
    ts.allow_adjacency(empty, wall_w, &[Direction::West]);
    ts.allow_adjacency(empty, corner_ne, &[Direction::North, Direction::East]);
    ts.allow_adjacency(empty, corner_se, &[Direction::South, Direction::East]);
    ts.allow_adjacency(empty, corner_sw, &[Direction::South, Direction::West]);
    ts.allow_adjacency(empty, corner_nw, &[Direction::North, Direction::West]);

    // Floor can be next to floor or walls (interior side)
    ts.allow_adjacency(floor, floor, &Direction::ALL);
    ts.allow_adjacency(floor, wall_n, &[Direction::South]);
    ts.allow_adjacency(floor, wall_s, &[Direction::North]);
    ts.allow_adjacency(floor, wall_e, &[Direction::West]);
    ts.allow_adjacency(floor, wall_w, &[Direction::East]);
    ts.allow_adjacency(floor, corner_ne, &[Direction::South, Direction::West]);
    ts.allow_adjacency(floor, corner_se, &[Direction::North, Direction::West]);
    ts.allow_adjacency(floor, corner_sw, &[Direction::North, Direction::East]);
    ts.allow_adjacency(floor, corner_nw, &[Direction::South, Direction::East]);

    // Wall continuity
    ts.allow_adjacency(wall_n, wall_n, &[Direction::East, Direction::West]);
    ts.allow_adjacency(wall_s, wall_s, &[Direction::East, Direction::West]);
    ts.allow_adjacency(wall_e, wall_e, &[Direction::North, Direction::South]);
    ts.allow_adjacency(wall_w, wall_w, &[Direction::North, Direction::South]);

    // Corner connections
    ts.allow_adjacency(wall_n, corner_ne, &[Direction::East]);
    ts.allow_adjacency(wall_n, corner_nw, &[Direction::West]);
    ts.allow_adjacency(wall_s, corner_se, &[Direction::East]);
    ts.allow_adjacency(wall_s, corner_sw, &[Direction::West]);
    ts.allow_adjacency(wall_e, corner_ne, &[Direction::North]);
    ts.allow_adjacency(wall_e, corner_se, &[Direction::South]);
    ts.allow_adjacency(wall_w, corner_nw, &[Direction::North]);
    ts.allow_adjacency(wall_w, corner_sw, &[Direction::South]);

    ts
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn simple_tileset() -> TileSet {
        let mut ts = TileSet::new();
        let land = ts.add_tile("land");
        let sea = ts.add_tile("sea");
        let coast = ts.add_tile("coast");

        // Land-land, sea-sea, coast can touch all
        ts.allow_adjacency(land, land, &Direction::ALL);
        ts.allow_adjacency(sea, sea, &Direction::ALL);
        ts.allow_adjacency(coast, coast, &Direction::ALL);
        ts.allow_adjacency(land, coast, &Direction::ALL);
        ts.allow_adjacency(sea, coast, &Direction::ALL);
        // But land cannot be directly adjacent to sea (must go through coast)

        ts
    }

    #[test]
    fn direction_opposite() {
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::East.opposite(), Direction::West);
    }

    #[test]
    fn direction_offset() {
        assert_eq!(Direction::North.offset(), (0, -1));
        assert_eq!(Direction::East.offset(), (1, 0));
        assert_eq!(Direction::South.offset(), (0, 1));
        assert_eq!(Direction::West.offset(), (-1, 0));
    }

    #[test]
    fn tileset_creation() {
        let ts = simple_tileset();
        assert_eq!(ts.tile_count(), 3);
        assert_eq!(ts.tile(0).name, "land");
        assert_eq!(ts.tile(1).name, "sea");
        assert_eq!(ts.tile(2).name, "coast");
    }

    #[test]
    fn tileset_adjacency() {
        let ts = simple_tileset();
        assert!(ts.is_allowed(0, Direction::North, 0)); // land-land
        assert!(ts.is_allowed(1, Direction::East, 1)); // sea-sea
        assert!(ts.is_allowed(0, Direction::South, 2)); // land-coast
        assert!(!ts.is_allowed(0, Direction::North, 1)); // land-sea: NOT allowed
    }

    #[test]
    fn wfc_grid_creation() {
        let ts = simple_tileset();
        let grid = WfcGrid::new(8, 8, &ts);
        assert_eq!(grid.width(), 8);
        assert_eq!(grid.height(), 8);
        assert_eq!(grid.uncollapsed_count(), 64);
    }

    #[test]
    fn wfc_collapse_simple() {
        let ts = simple_tileset();
        let mut grid = WfcGrid::new(4, 4, &ts);
        let mut rng = rand::rng();
        let result = grid.collapse_all(&mut rng);
        assert!(result.is_ok());
        assert!(grid.is_fully_collapsed());

        // Verify all adjacency constraints hold
        let g = grid.to_grid();
        for y in 0..4 {
            for x in 0..4 {
                let tile = g[y][x].expect("should be collapsed");
                for dir in Direction::ALL {
                    let (dx, dy) = dir.offset();
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < 4 && ny < 4 {
                        let neighbor = g[ny as usize][nx as usize].expect("should be collapsed");
                        assert!(
                            ts.is_allowed(tile, dir, neighbor),
                            "Invalid adjacency: tile {} ({}) -> tile {} ({}) in {:?} at ({},{})",
                            tile,
                            ts.tile(tile).name,
                            neighbor,
                            ts.tile(neighbor).name,
                            dir,
                            x,
                            y
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn wfc_deterministic_with_seed() {
        let ts = simple_tileset();
        let mut grid1 = WfcGrid::new(6, 6, &ts);
        let mut grid2 = WfcGrid::new(6, 6, &ts);

        let mut rng1 = rand::rngs::StdRng::seed_from_u64(12345);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(12345);

        grid1.collapse_all(&mut rng1).unwrap();
        grid2.collapse_all(&mut rng2).unwrap();

        let g1 = grid1.to_grid();
        let g2 = grid2.to_grid();
        assert_eq!(g1, g2, "Same seed should produce same result");
    }

    #[test]
    fn wfc_set_tile_seed() {
        let ts = simple_tileset();
        let mut grid = WfcGrid::new(4, 4, &ts);
        // Force center to be "sea" (id=1)
        grid.set_tile(2, 2, 1);
        assert_eq!(grid.get(2, 2), Some(1));

        let mut rng = rand::rng();
        let result = grid.collapse_all(&mut rng);
        assert!(result.is_ok());
        assert_eq!(grid.get(2, 2), Some(1)); // should still be sea
    }

    #[test]
    fn wfc_empty_tileset_error() {
        let ts = TileSet::new();
        let mut grid = WfcGrid::new(2, 2, &ts);
        let mut rng = rand::rng();
        let result = grid.collapse_all(&mut rng);
        assert!(matches!(result, Err(WfcError::EmptyTileSet)));
    }

    #[test]
    fn wfc_larger_grid() {
        let ts = simple_tileset();
        let mut grid = WfcGrid::new(16, 16, &ts);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let result = grid.collapse_all(&mut rng);
        assert!(result.is_ok());
        assert!(grid.is_fully_collapsed());
    }

    #[test]
    fn wfc_rooms_tileset() {
        let ts = rooms_and_corridors_tileset();
        assert_eq!(ts.tile_count(), 10);
        assert_eq!(ts.tile(0).name, "empty");
        assert_eq!(ts.tile(1).name, "floor");
    }

    #[test]
    fn wfc_weighted_tiles() {
        let mut ts = TileSet::new();
        let common = ts.add_tile_weighted("common", 10.0);
        let rare = ts.add_tile_weighted("rare", 0.1);
        ts.allow_adjacency(common, common, &Direction::ALL);
        ts.allow_adjacency(common, rare, &Direction::ALL);
        ts.allow_adjacency(rare, rare, &Direction::ALL);

        let mut grid = WfcGrid::new(10, 10, &ts);
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        grid.collapse_all(&mut rng).unwrap();

        // Count occurrences — common should dominate
        let g = grid.to_grid();
        let common_count = g.iter().flatten().filter(|&&t| t == Some(common)).count();
        let rare_count = g.iter().flatten().filter(|&&t| t == Some(rare)).count();
        assert!(
            common_count > rare_count,
            "Common ({common_count}) should appear more than rare ({rare_count})"
        );
    }
}

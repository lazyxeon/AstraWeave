//! Sparse voxel DAG for one edited chunk.
//!
//! Identical leaves and identical branches share a node. The base column
//! (flat today, #215's `surface_height` later) fills cells that have no
//! density edit. Rebuilds run when a brush or weave deforms the chunk.

pub const CHUNK_EXTENT: i32 = 32;
const MAX_NODES: usize = 2048;

#[derive(Clone, Copy)]
struct Node {
    children: [u16; 8],
    density: i8,
    leaf: bool,
}

/// Content-addressed octree. Node 0 is unused. Node 1 is the shared empty leaf.
#[derive(Clone)]
pub struct VoxelDag {
    nodes: [Node; MAX_NODES],
    keys: [u64; MAX_NODES],
    len: u16,
    root: u16,
    origin: [i32; 3],
    ready: bool,
}

impl VoxelDag {
    pub fn new() -> Self {
        let mut dag = Self {
            nodes: [Node {
                children: [0; 8],
                density: 0,
                leaf: true,
            }; MAX_NODES],
            keys: [0; MAX_NODES],
            len: 2,
            root: 1,
            origin: [0, 0, 0],
            ready: false,
        };
        dag.nodes[1].leaf = true;
        dag.nodes[1].density = 0;
        dag.keys[1] = leaf_key(0);
        dag
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn root(&self) -> u16 {
        self.root
    }

    pub fn node_len(&self) -> u16 {
        self.len
    }

    /// `sample(x, y, z)` returns the stored density, or `None` when the base column applies.
    pub fn rebuild<F>(&mut self, origin: [i32; 3], mut sample: F)
    where
        F: FnMut(i32, i32, i32) -> Option<i8>,
    {
        self.len = 2;
        self.origin = origin;
        self.root = self.build(origin[0], origin[1], origin[2], CHUNK_EXTENT, &mut sample);
        self.ready = true;
    }

    pub fn solid_at<F>(&self, x: i32, y: i32, z: i32, mut base_solid: F) -> bool
    where
        F: FnMut(i32, i32, i32) -> bool,
    {
        if !self.ready {
            return base_solid(x, y, z);
        }
        let local_x = x - self.origin[0];
        let local_y = y - self.origin[1];
        let local_z = z - self.origin[2];
        if !(0..CHUNK_EXTENT).contains(&local_x)
            || !(0..CHUNK_EXTENT).contains(&local_y)
            || !(0..CHUNK_EXTENT).contains(&local_z)
        {
            return base_solid(x, y, z);
        }
        match self.density_at(local_x, local_y, local_z) {
            Some(density) => density > 0,
            None => base_solid(x, y, z),
        }
    }

    fn density_at(&self, mut x: i32, mut y: i32, mut z: i32) -> Option<i8> {
        let mut index = self.root;
        let mut size = CHUNK_EXTENT;
        loop {
            if index == 0 {
                return None;
            }
            let node = self.nodes[index as usize];
            if node.leaf {
                return Some(node.density);
            }
            size /= 2;
            if size == 0 {
                return Some(node.density);
            }
            let oct = octant(x, y, z, size);
            x %= size;
            y %= size;
            z %= size;
            let child = node.children[oct];
            if child == 0 {
                return None;
            }
            index = child;
        }
    }

    fn build<F>(&mut self, x: i32, y: i32, z: i32, size: i32, sample: &mut F) -> u16
    where
        F: FnMut(i32, i32, i32) -> Option<i8>,
    {
        if size <= 1 {
            return match sample(x, y, z) {
                Some(density) => self.intern_leaf(density),
                None => 0,
            };
        }
        let half = size / 2;
        let mut children = [0u16; 8];
        let mut any = false;
        for oct in 0..8 {
            let (ox, oy, oz) = oct_offset(oct, half);
            let child = self.build(x + ox, y + oy, z + oz, half, sample);
            children[oct] = child;
            if child != 0 {
                any = true;
            }
        }
        if !any {
            return 0;
        }
        self.intern_branch(children)
    }

    fn intern_leaf(&mut self, density: i8) -> u16 {
        let key = leaf_key(density);
        if let Some(found) = self.find(key) {
            return found;
        }
        self.push(
            key,
            Node {
                children: [0; 8],
                density,
                leaf: true,
            },
        )
    }

    fn intern_branch(&mut self, children: [u16; 8]) -> u16 {
        let key = branch_key(children);
        if let Some(found) = self.find(key) {
            return found;
        }
        self.push(
            key,
            Node {
                children,
                density: 0,
                leaf: false,
            },
        )
    }

    fn find(&self, key: u64) -> Option<u16> {
        let mut index = 1u16;
        while index < self.len {
            if self.keys[index as usize] == key {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn push(&mut self, key: u64, node: Node) -> u16 {
        if self.len as usize >= MAX_NODES {
            return 1;
        }
        let index = self.len;
        self.nodes[index as usize] = node;
        self.keys[index as usize] = key;
        self.len += 1;
        index
    }

    pub fn copy_nodes(
        &self,
        children_out: &mut [[u16; 8]],
        density_out: &mut [i8],
        leaf_out: &mut [u8],
    ) -> u16 {
        let n = self.len as usize;
        for index in 0..n {
            children_out[index] = self.nodes[index].children;
            density_out[index] = self.nodes[index].density;
            leaf_out[index] = u8::from(self.nodes[index].leaf);
        }
        self.len
    }
}

impl Default for VoxelDag {
    fn default() -> Self {
        Self::new()
    }
}

fn leaf_key(density: i8) -> u64 {
    0x1000_0000_0000_0000 | u64::from(density as u8)
}

fn branch_key(children: [u16; 8]) -> u64 {
    let mut key = 0x2000_0000_0000_0000u64;
    for (shift, child) in children.iter().enumerate() {
        key ^= u64::from(*child)
            .wrapping_mul(0x9E37_79B9)
            .rotate_left((shift as u32) * 7);
    }
    key
}

fn octant(x: i32, y: i32, z: i32, half: i32) -> usize {
    let mut oct = 0;
    if x >= half {
        oct |= 1;
    }
    if y >= half {
        oct |= 2;
    }
    if z >= half {
        oct |= 4;
    }
    oct
}

fn oct_offset(oct: usize, half: i32) -> (i32, i32, i32) {
    (
        if oct & 1 == 1 { half } else { 0 },
        if oct & 2 == 2 { half } else { 0 },
        if oct & 4 == 4 { half } else { 0 },
    )
}

/// Walk a published node image. Used by query jobs so they do not share the live DAG.
pub fn solid_in_image(
    children: &[[u16; 8]],
    density: &[i8],
    leaf: &[u8],
    root: u16,
    origin: [i32; 3],
    x: i32,
    y: i32,
    z: i32,
) -> Option<bool> {
    let local_x = x - origin[0];
    let local_y = y - origin[1];
    let local_z = z - origin[2];
    if !(0..CHUNK_EXTENT).contains(&local_x)
        || !(0..CHUNK_EXTENT).contains(&local_y)
        || !(0..CHUNK_EXTENT).contains(&local_z)
    {
        return None;
    }
    let mut index = root;
    let mut cx = local_x;
    let mut cy = local_y;
    let mut cz = local_z;
    let mut size = CHUNK_EXTENT;
    loop {
        if index == 0 || index as usize >= children.len() {
            return None;
        }
        if leaf[index as usize] == 1 {
            return Some(density[index as usize] > 0);
        }
        size /= 2;
        if size == 0 {
            return Some(density[index as usize] > 0);
        }
        let oct = octant(cx, cy, cz, size);
        cx %= size;
        cy %= size;
        cz %= size;
        index = children[index as usize][oct];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_leaves_share_a_node() {
        let mut dag = VoxelDag::new();
        dag.rebuild([0, 0, 0], |x, y, z| {
            if y == 1 && z == 1 && (x == 1 || x == 2) {
                Some(127)
            } else {
                None
            }
        });
        assert!(dag.ready());
        assert!(dag.len < 64, "dag nodes {}", dag.len);
        assert!(dag.solid_at(1, 1, 1, |_, _, _| false));
        assert!(!dag.solid_at(3, 1, 1, |_, _, _| false));
    }
}

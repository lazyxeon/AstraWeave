// GPU Compute Erosion — Shallow Water Equations (Šťava et al. 2008)
//
// Three-pass compute pipeline per simulation step:
//   1. rain_and_flux  — add rainfall, compute outflow flux from water head differences
//   2. water_velocity — update water heights from net flux, derive velocity field
//   3. erode_transport — dissolve/deposit sediment, advect, evaporate

// ─── Uniforms ───

struct ErosionParams {
    grid_width:         u32,
    grid_height:        u32,
    dt:                 f32,
    rain_rate:          f32,
    pipe_area:          f32,
    gravity:            f32,
    cell_size:          f32,
    sediment_capacity:  f32,
    dissolution_rate:   f32,
    deposition_rate:    f32,
    evaporation_rate:   f32,
    min_slope:          f32,
}

// ─── Bindings ───

@group(0) @binding(0) var<uniform> params: ErosionParams;
@group(0) @binding(1) var<storage, read_write> terrain:  array<f32>;   // terrain height
@group(0) @binding(2) var<storage, read_write> water:    array<f32>;   // water depth
@group(0) @binding(3) var<storage, read_write> sediment: array<f32>;   // suspended sediment
@group(0) @binding(4) var<storage, read_write> flux:     array<vec4<f32>>; // outflow (L, R, T, B)
@group(0) @binding(5) var<storage, read_write> velocity: array<vec2<f32>>; // water velocity

// ─── Helpers ───

fn idx(x: u32, y: u32) -> u32 {
    return y * params.grid_width + x;
}

fn in_bounds(x: i32, y: i32) -> bool {
    return x >= 0 && x < i32(params.grid_width) && y >= 0 && y < i32(params.grid_height);
}

fn total_height(i: u32) -> f32 {
    return terrain[i] + water[i];
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

// ─── Pass 1: Rain + Outflow Flux ───

@compute @workgroup_size(WG_X, WG_Y)
fn rain_and_flux(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.grid_width || y >= params.grid_height) {
        return;
    }

    let i = idx(x, y);

    // Add rainfall
    water[i] = water[i] + params.rain_rate * params.dt;

    let h = total_height(i);
    var f = flux[i];

    // Outflow flux to each neighbor (pipe model)
    let scale = params.dt * params.pipe_area * params.gravity / params.cell_size;

    // Left (x-1)
    if (x > 0u) {
        let dh = h - total_height(idx(x - 1u, y));
        f.x = max(0.0, f.x + scale * dh);
    } else {
        f.x = 0.0;
    }

    // Right (x+1)
    if (x < params.grid_width - 1u) {
        let dh = h - total_height(idx(x + 1u, y));
        f.y = max(0.0, f.y + scale * dh);
    } else {
        f.y = 0.0;
    }

    // Top (y-1)
    if (y > 0u) {
        let dh = h - total_height(idx(x, y - 1u));
        f.z = max(0.0, f.z + scale * dh);
    } else {
        f.z = 0.0;
    }

    // Bottom (y+1)
    if (y < params.grid_height - 1u) {
        let dh = h - total_height(idx(x, y + 1u));
        f.w = max(0.0, f.w + scale * dh);
    } else {
        f.w = 0.0;
    }

    // Scale flux so total outflow ≤ available water
    let total_out = f.x + f.y + f.z + f.w;
    if (total_out > 0.0) {
        let avail = water[i] * params.cell_size * params.cell_size;
        let k = min(1.0, avail / (total_out * params.dt));
        f = f * k;
    }

    flux[i] = f;
}

// ─── Pass 2: Water Update + Velocity ───

@compute @workgroup_size(WG_X, WG_Y)
fn water_velocity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.grid_width || y >= params.grid_height) {
        return;
    }

    let i = idx(x, y);
    let f = flux[i];

    // Inflow from neighbors
    var inflow = 0.0;
    if (x > 0u) {
        inflow += flux[idx(x - 1u, y)].y; // neighbor's right → our left
    }
    if (x < params.grid_width - 1u) {
        inflow += flux[idx(x + 1u, y)].x; // neighbor's left → our right
    }
    if (y > 0u) {
        inflow += flux[idx(x, y - 1u)].w; // neighbor's bottom → our top
    }
    if (y < params.grid_height - 1u) {
        inflow += flux[idx(x, y + 1u)].z; // neighbor's top → our bottom
    }

    let outflow = f.x + f.y + f.z + f.w;
    let dv = (inflow - outflow) * params.dt / (params.cell_size * params.cell_size);
    let new_water = max(0.0, water[i] + dv);

    // Velocity from flux differences
    let w_avg = max(0.001, (water[i] + new_water) * 0.5);
    var vx = 0.0;
    var vy = 0.0;

    // Horizontal velocity (from left-right flux difference)
    if (x > 0u) {
        vx += flux[idx(x - 1u, y)].y; // inflow from left
    }
    vx -= f.x; // outflow to left
    vx += f.y; // outflow to right
    if (x < params.grid_width - 1u) {
        vx -= flux[idx(x + 1u, y)].x; // inflow from right cancels
    }
    vx = vx / (2.0 * params.cell_size * w_avg);

    // Vertical velocity
    if (y > 0u) {
        vy += flux[idx(x, y - 1u)].w;
    }
    vy -= f.z;
    vy += f.w;
    if (y < params.grid_height - 1u) {
        vy -= flux[idx(x, y + 1u)].z;
    }
    vy = vy / (2.0 * params.cell_size * w_avg);

    water[i] = new_water;
    velocity[i] = vec2<f32>(vx, vy);
}

// ─── Pass 3: Erosion, Deposition, Sediment Transport, Evaporation ───

@compute @workgroup_size(WG_X, WG_Y)
fn erode_transport(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.grid_width || y >= params.grid_height) {
        return;
    }

    let i = idx(x, y);
    let v = velocity[i];
    let speed = length(v);

    // Local slope (central differences)
    var slope = params.min_slope;
    if (x > 0u && x < params.grid_width - 1u) {
        let dx = (terrain[idx(x + 1u, y)] - terrain[idx(x - 1u, y)]) / (2.0 * params.cell_size);
        slope = max(slope, abs(dx));
    }
    if (y > 0u && y < params.grid_height - 1u) {
        let dy = (terrain[idx(x, y + 1u)] - terrain[idx(x, y - 1u)]) / (2.0 * params.cell_size);
        slope = max(slope, abs(dy));
    }

    // Carrying capacity
    let capacity = params.sediment_capacity * speed * slope * water[i];

    // Erosion or deposition
    let s = sediment[i];
    if (s < capacity) {
        // Erode terrain → sediment
        let erode = min(params.dissolution_rate * (capacity - s), terrain[i] * 0.5);
        terrain[i] = terrain[i] - erode;
        sediment[i] = s + erode;
    } else {
        // Deposit sediment → terrain
        let deposit = params.deposition_rate * (s - capacity);
        terrain[i] = terrain[i] + deposit;
        sediment[i] = s - deposit;
    }

    // Evaporate water
    water[i] = water[i] * (1.0 - params.evaporation_rate * params.dt);

    // Simple sediment advection (first-order upwind)
    // Note: full advection requires a separate read buffer; this is a simplified
    // in-place approximation that works well for small timesteps.
    if (speed > 0.001) {
        let src_x = f32(x) - v.x * params.dt / params.cell_size;
        let src_y = f32(y) - v.y * params.dt / params.cell_size;
        let sx = clamp(i32(src_x), 0, i32(params.grid_width) - 1);
        let sy = clamp(i32(src_y), 0, i32(params.grid_height) - 1);
        let src_i = u32(sy) * params.grid_width + u32(sx);
        // Blend toward source sediment (semi-Lagrangian)
        sediment[i] = mix(sediment[i], sediment[src_i], min(speed * params.dt * 0.5, 0.5));
    }
}

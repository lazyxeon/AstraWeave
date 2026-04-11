// Virtual texture feedback pass — GPU compute shader.
//
// Determines which virtual texture pages are needed by the current view
// by writing page IDs to a feedback buffer. The CPU reads back the buffer
// to decide which pages to stream from disk.

struct FeedbackParams {
    // Virtual texture parameters
    vt_size:           vec2<f32>,   // full virtual texture size in pixels
    inv_vt_size:       vec2<f32>,   // 1 / vt_size
    page_size:         f32,         // page size in texels (e.g., 128)
    inv_page_size:     f32,         // 1 / page_size
    max_mip_level:     f32,         // highest mip level
    padding:           f32,
    // Screen info
    screen_size:       vec2<f32>,   // render target size
    inv_screen_size:   vec2<f32>,   // 1 / screen_size
}

// Page request entry: packed (page_x, page_y, mip_level) into a u32.
// Layout: bits [0..9] = page_x, [10..19] = page_y, [20..24] = mip_level, [25..31] = reserved
fn pack_page_id(page_x: u32, page_y: u32, mip: u32) -> u32 {
    return (page_x & 0x3FFu) | ((page_y & 0x3FFu) << 10u) | ((mip & 0x1Fu) << 20u);
}

fn unpack_page_x(packed: u32) -> u32 {
    return packed & 0x3FFu;
}

fn unpack_page_y(packed: u32) -> u32 {
    return (packed >> 10u) & 0x3FFu;
}

fn unpack_mip(packed: u32) -> u32 {
    return (packed >> 20u) & 0x1Fu;
}

@group(0) @binding(0) var<uniform> params: FeedbackParams;
@group(0) @binding(1) var<storage, read_write> feedback_buffer: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> feedback_count: atomic<u32>;
@group(0) @binding(3) var depth_tex: texture_2d<f32>;

// Compute the mip level from UV derivatives (approximation using screen-space
// texel density).
fn compute_mip_level(uv: vec2<f32>, screen_pos: vec2<f32>) -> f32 {
    // Approximate foot-print: measure UV change per pixel via central
    // differences on the depth buffer (a rough proxy for ∂uv/∂screen).
    let texel_density = length(uv * params.vt_size) * params.inv_screen_size.x;
    let mip = log2(max(texel_density, 1.0));
    return clamp(mip, 0.0, params.max_mip_level);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn feedback_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let screen_pos = vec2<f32>(f32(gid.x), f32(gid.y));
    if (screen_pos.x >= params.screen_size.x || screen_pos.y >= params.screen_size.y) {
        return;
    }

    // Read depth to skip sky pixels
    let depth = textureLoad(depth_tex, vec2<i32>(gid.xy), 0).r;
    if (depth >= 1.0) {
        return; // sky — no VT page needed
    }

    // Reconstruct UV from screen position (simplified: identity mapping for terrain)
    let uv = screen_pos * params.inv_screen_size;

    // Determine required mip level
    let mip_f = compute_mip_level(uv, screen_pos);
    let mip = u32(floor(mip_f));

    // Convert UV to page coordinates at this mip level
    let mip_scale = 1.0 / f32(1u << mip);
    let pages_at_mip = params.vt_size * params.inv_page_size * mip_scale;
    let page_x = u32(clamp(uv.x * pages_at_mip.x, 0.0, pages_at_mip.x - 1.0));
    let page_y = u32(clamp(uv.y * pages_at_mip.y, 0.0, pages_at_mip.y - 1.0));

    // Write page request (deduplicated via atomic compare-and-swap would be
    // ideal, but for simplicity we use append with a counter).
    let page_id = pack_page_id(page_x, page_y, mip);
    let slot = atomicAdd(&feedback_count, 1u);
    let max_entries = arrayLength(&feedback_buffer);
    if (slot < max_entries) {
        atomicStore(&feedback_buffer[slot], page_id);
    }
}

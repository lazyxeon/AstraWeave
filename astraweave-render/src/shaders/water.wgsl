// Water shader with Gerstner wave displacement
// Implements animated ocean surface with realistic wave simulation

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    // Per-instance chunk world-XZ center (added to local tile position).
    @location(2) chunk_offset: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) wave_height: f32,
    // W.2c — accumulated freeze mask (0 = liquid, 1 = fully frozen). Drives the
    // fragment-shader material-state blend toward the frozen look.
    @location(4) freeze: f32,
};

// W.2c — one runtime weave-deformation instance. Mirrors `WeaveInstanceRaw` in
// water.rs (32 B, std140). Location lives here only; the profile is a normalized
// shape in local space.
struct WeaveInstance {
    position: vec2<f32>,   // world-XZ center
    radius: f32,           // world footprint (local r=1 maps here)
    orientation: f32,      // yaw radians
    intensity: f32,        // 0..1 magnitude
    phase: f32,            // animation phase / age (s)
    kind: u32,             // 0 none, 1 part, 2 raise, 3 freeze
    _pad: u32,
};

struct WaterUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    water_color_deep: vec3<f32>,
    _pad1: f32,
    water_color_shallow: vec3<f32>,
    _pad2: f32,
    foam_color: vec3<f32>,
    foam_threshold: f32,
    // Rain ripple parameters.
    rain_intensity: f32,   // 0.0 = no rain, 1.0 = heavy rain
    ripple_scale: f32,     // UV tile scale for ripple pattern (default 4.0)
    ripple_strength: f32,  // Normal perturbation strength (default 0.15)
    water_level: f32,      // World-space Y of the rest surface (W.2a)
    skirt_depth: f32,      // Skirt vertices drop this far below the surface (W.2a)
    _pad3: f32,
    _pad4: f32,
    _pad5: f32,
    // W.2b — refraction + depth-delta foam.
    inv_view_proj: mat4x4<f32>,  // reconstruct scene world pos from sampled depth
    screen_size: vec2<f32>,      // for @builtin(position) → screen-UV
    refraction_strength: f32,    // normal-driven scene-color distortion
    foam_depth_band: f32,        // world-space shoreline foam width
    // W.2c — weave-response deformation (ceiling 8). Array is 16-aligned at offset 256.
    weave_count: u32,
    _pad6: u32,
    _pad7: u32,
    _pad8: u32,
    weave_instances: array<WeaveInstance, 8>,
};

// Peak world-space displacement one weave can apply, in units. MUST equal
// `SKIRT_DEPTH` / `WEAVE_MAX_DEFORM` in water.rs so a Part/Raise at full intensity
// stays within the LOD skirt and never re-exposes a seam (W.2c.2 skirt constraint).
const WEAVE_MAX_DEFORM: f32 = 8.0;

@group(0) @binding(0) var<uniform> uniforms: WaterUniforms;
// W.2b — opaque scene snapshot + scene depth for refraction and shoreline foam.
@group(0) @binding(1) var scene_color: texture_2d<f32>;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var scene_samp: sampler;

// ── Rain ripple normal perturbation ─────────────────────────────────────────
// Procedural concentric ring pattern from multiple random "drop" origins.
// Each layer uses a different speed and phase offset for variation.

fn ripple_ring(uv: vec2<f32>, center: vec2<f32>, time: f32, freq: f32) -> f32 {
    let dist = length(uv - center);
    let wave = sin(dist * freq - time * 12.0) * exp(-dist * 3.0);
    // Fade out over time (each "drop" lasts ~1 second).
    let age = fract(time * 0.7 + dot(center, vec2<f32>(17.1, 31.7)));
    let fade = 1.0 - smoothstep(0.6, 1.0, age);
    return wave * fade;
}

fn rain_ripple_normal(world_xz: vec2<f32>, time: f32, scale: f32, strength: f32) -> vec3<f32> {
    let uv = world_xz * scale;
    var h = 0.0;

    // 3 layers of ripple drops at different pseudo-random positions.
    let c1 = vec2<f32>(fract(sin(dot(vec2<f32>(1.0, 2.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(1.0, 2.0), vec2<f32>(269.5, 183.3))) * 43758.5453));
    let c2 = vec2<f32>(fract(sin(dot(vec2<f32>(3.0, 4.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(3.0, 4.0), vec2<f32>(269.5, 183.3))) * 43758.5453));
    let c3 = vec2<f32>(fract(sin(dot(vec2<f32>(5.0, 6.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(5.0, 6.0), vec2<f32>(269.5, 183.3))) * 43758.5453));

    // Tile centers to repeat across the surface.
    let tile = floor(uv);
    h += ripple_ring(fract(uv), c1, time, 25.0);
    h += ripple_ring(fract(uv + 0.37), c2, time + 0.33, 30.0);
    h += ripple_ring(fract(uv + 0.71), c3, time + 0.67, 22.0);

    h *= strength;

    // Compute normal from finite differences of the height.
    let eps = 0.01;
    let uv_dx = (uv + vec2<f32>(eps, 0.0));
    let uv_dz = (uv + vec2<f32>(0.0, eps));
    var h_dx = 0.0;
    var h_dz = 0.0;
    h_dx += ripple_ring(fract(uv_dx), c1, time, 25.0);
    h_dx += ripple_ring(fract(uv_dx + 0.37), c2, time + 0.33, 30.0);
    h_dx += ripple_ring(fract(uv_dx + 0.71), c3, time + 0.67, 22.0);
    h_dz += ripple_ring(fract(uv_dz), c1, time, 25.0);
    h_dz += ripple_ring(fract(uv_dz + 0.37), c2, time + 0.33, 30.0);
    h_dz += ripple_ring(fract(uv_dz + 0.71), c3, time + 0.67, 22.0);
    h_dx *= strength;
    h_dz *= strength;

    let dx = (h_dx - h) / eps;
    let dz = (h_dz - h) / eps;
    return normalize(vec3<f32>(-dx, 1.0, -dz));
}

// Gerstner wave parameters
// Each wave: (direction.x, direction.y, amplitude, frequency)
const WAVE_COUNT: u32 = 4u;

fn gerstner_wave(
    pos: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    direction: vec2<f32>,
    steepness: f32,
) -> vec3<f32> {
    let d = normalize(direction);
    let phase = frequency * (dot(d, pos) - speed * time);
    // Profile A steepness guardrail: cap Q ≤ 1.0 to prevent normal inversion /
    // mesh self-intersection at crests (the W-series Gemini-triage correctness cap).
    let Q = min(steepness / (frequency * amplitude * f32(WAVE_COUNT)), 1.0);
    
    return vec3<f32>(
        Q * amplitude * d.x * cos(phase),
        amplitude * sin(phase),
        Q * amplitude * d.y * cos(phase)
    );
}

fn gerstner_normal(
    pos: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    direction: vec2<f32>,
    steepness: f32,
) -> vec3<f32> {
    let d = normalize(direction);
    let phase = frequency * (dot(d, pos) - speed * time);
    // Profile A steepness guardrail: cap Q ≤ 1.0 to prevent normal inversion /
    // mesh self-intersection at crests (the W-series Gemini-triage correctness cap).
    let Q = min(steepness / (frequency * amplitude * f32(WAVE_COUNT)), 1.0);
    let WA = frequency * amplitude;
    
    let s = sin(phase);
    let c = cos(phase);
    
    return vec3<f32>(
        -d.x * WA * c,
        1.0 - Q * WA * s,
        -d.y * WA * c
    );
}

// ── W.2c weave-response deformation ──────────────────────────────────────────
// Normalized analytical profile: 1 at the centre, smoothly → 0 at local r = 1, and
// 0 beyond. Position-agnostic — the caller maps world → local first. This is the
// representation-agnostic seam: a deformation-texture profile would replace this one
// evaluation with `textureSample(profile_atlas, local*0.5+0.5)` and nothing else.
fn weave_profile(local: vec2<f32>) -> f32 {
    let r = length(local);
    return 1.0 - smoothstep(0.0, 1.0, clamp(r, 0.0, 1.0));
}

struct WeaveResult {
    height: f32,   // additive world-space height offset (part < 0, raise > 0)
    freeze: f32,   // accumulated freeze mask 0..1
};

// Accumulate every active weave instance at this world-XZ point (ceiling 8).
fn weave_accumulate(world_xz: vec2<f32>) -> WeaveResult {
    var out: WeaveResult;
    out.height = 0.0;
    out.freeze = 0.0;
    for (var i = 0u; i < uniforms.weave_count; i = i + 1u) {
        let inst = uniforms.weave_instances[i];
        if (inst.kind == 0u) { continue; }
        // world → local: translate to the instance centre, un-rotate by the
        // instance yaw, normalize by the world radius. Location lives only here.
        let rel = world_xz - inst.position;
        let c = cos(-inst.orientation);
        let s = sin(-inst.orientation);
        let rot = vec2<f32>(rel.x * c - rel.y * s, rel.x * s + rel.y * c);
        let local = rot / max(inst.radius, 0.001);
        let amp = inst.intensity * weave_profile(local);
        if (inst.kind == 1u) {            // part: push the surface down
            out.height = out.height - amp * WEAVE_MAX_DEFORM;
        } else if (inst.kind == 2u) {     // raise: lift the surface up
            out.height = out.height + amp * WEAVE_MAX_DEFORM;
        } else if (inst.kind == 3u) {     // freeze: accumulate the lock mask
            out.freeze = max(out.freeze, amp);
        }
    }
    // Bound the net height to ±skirt tolerance so even overlapping part+raise can't
    // outrun the skirt (W.2c.2 skirt constraint, defence-in-depth).
    out.height = clamp(out.height, -WEAVE_MAX_DEFORM, WEAVE_MAX_DEFORM);
    out.freeze = clamp(out.freeze, 0.0, 1.0);
    return out;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let time = uniforms.time;
    // World XZ for this vertex = local tile position + per-chunk world offset.
    // Sampling the wave field at world XZ makes chunks world-stable (no swimming)
    // and guarantees shared LOD-boundary vertices agree exactly.
    let world_xz = input.position.xz + input.chunk_offset;
    // Skirt vertices carry sentinel local Y = -1.0; surface vertices carry 0.
    let is_skirt = select(0.0, 1.0, input.position.y < -0.5);

    // Apply 4 Gerstner waves with different parameters
    var displacement = vec3<f32>(0.0);
    var normal_accum = vec3<f32>(0.0, 1.0, 0.0);

    // Wave 1: Primary swell (large, slow)
    displacement += gerstner_wave(world_xz, time, 0.8, 0.15, 2.0, vec2<f32>(1.0, 0.3), 0.5);
    normal_accum += gerstner_normal(world_xz, time, 0.8, 0.15, 2.0, vec2<f32>(1.0, 0.3), 0.5);

    // Wave 2: Secondary swell (medium)
    displacement += gerstner_wave(world_xz, time, 0.5, 0.25, 2.5, vec2<f32>(-0.5, 1.0), 0.4);
    normal_accum += gerstner_normal(world_xz, time, 0.5, 0.25, 2.5, vec2<f32>(-0.5, 1.0), 0.4);

    // Wave 3: Chop (small, fast)
    displacement += gerstner_wave(world_xz, time, 0.25, 0.5, 3.5, vec2<f32>(0.7, -0.7), 0.3);
    normal_accum += gerstner_normal(world_xz, time, 0.25, 0.5, 3.5, vec2<f32>(0.7, -0.7), 0.3);

    // Wave 4: Ripples (tiny, very fast)
    displacement += gerstner_wave(world_xz, time, 0.1, 1.0, 4.0, vec2<f32>(-0.3, 0.9), 0.2);
    normal_accum += gerstner_normal(world_xz, time, 0.1, 1.0, 4.0, vec2<f32>(-0.3, 0.9), 0.2);

    // Gerstner crest height for foam / shallow tint — captured BEFORE the weave
    // offset so a raise doesn't read as foam and a part doesn't read as deep shadow.
    let gerstner_wave_height = displacement.y;

    // ── W.2c weave deformation: composes AFTER the Gerstner sum, so the per-wave
    // Q-cap (internal to each gerstner_wave/gerstner_normal) is untouched. ──
    let weave = weave_accumulate(world_xz);
    // Freeze locks the surface: damp the wave displacement toward rest and flatten
    // the normal. Sampled at world XZ, so shared LOD-boundary vertices agree exactly
    // (same guarantee as Gerstner) — no new seam mechanism.
    displacement = displacement * (1.0 - weave.freeze);
    normal_accum = mix(normal_accum, vec3<f32>(0.0, 1.0, 0.0), weave.freeze);

    // Place the vertex in the world. Skirt vertices share their surface twin's
    // horizontal displacement and drop straight down by skirt_depth, hanging from
    // the displaced edge to cover any LOD-boundary crack.
    var pos: vec3<f32>;
    pos.x = world_xz.x + displacement.x;
    pos.z = world_xz.y + displacement.z;
    // weave.height is added BEFORE the skirt drop so the skirt tracks the deformed
    // edge; it is bounded to ±skirt_depth so it can never outrun the skirt.
    pos.y = uniforms.water_level + displacement.y + weave.height - is_skirt * uniforms.skirt_depth;

    output.world_pos = pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    output.uv = input.uv;
    output.normal = normalize(normal_accum);
    // Frozen surface reads flat (no crest foam) → scale the foam driver by (1-freeze).
    output.wave_height = gerstner_wave_height * (1.0 - weave.freeze);
    output.freeze = weave.freeze;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var N = normalize(input.normal);
    let V = normalize(uniforms.camera_pos - input.world_pos);

    // Rain ripple normal perturbation.
    if (uniforms.rain_intensity > 0.0) {
        let ripple_N = rain_ripple_normal(
            input.world_pos.xz,
            uniforms.time,
            uniforms.ripple_scale,
            uniforms.ripple_strength * uniforms.rain_intensity,
        );
        // Blend ripple normal with wave normal.
        N = normalize(mix(N, ripple_N, uniforms.rain_intensity * 0.6));
    }
    
    // Fresnel effect for reflection blend
    let fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.0);

    // Depth-based color blend (shallow vs deep)
    let depth_factor = clamp(input.wave_height * 2.0 + 0.5, 0.0, 1.0);
    let water_color = mix(uniforms.water_color_deep, uniforms.water_color_shallow, depth_factor);

    // ── W.2b refraction + depth-delta foam (uniform control flow below) ──────────
    // Screen UV of this water fragment (framebuffer origin top-left).
    let screen_uv = input.clip_position.xy / uniforms.screen_size;

    // Refraction: bend what's behind the water by the surface-normal XZ tilt.
    let distort = N.xz * uniforms.refraction_strength;
    let refr_uv = clamp(screen_uv + distort, vec2<f32>(0.0), vec2<f32>(1.0));
    let refracted = textureSample(scene_color, scene_samp, refr_uv).rgb;

    // Reconstruct the world position of the opaque scene behind the water from its
    // depth, then measure water thickness = distance to the water surface.
    // Undistorted fragment position → always within this fragment's own pixel, so
    // in-bounds. textureLoad has no clamping: if a future change derives this coord
    // from a distorted/offset value, clamp it to textureDimensions first.
    let scene_d = textureLoad(scene_depth, vec2<i32>(input.clip_position.xy), 0);
    let ndc = vec3<f32>(screen_uv.x * 2.0 - 1.0, 1.0 - screen_uv.y * 2.0, scene_d);
    let world_h = uniforms.inv_view_proj * vec4<f32>(ndc, 1.0);
    let scene_world = world_h.xyz / world_h.w;
    let thickness = distance(scene_world, input.world_pos);

    // Thin water (shoreline / over shallow terrain) is clear and shows the refracted
    // scene; thicker water absorbs toward the body colour (Beer-Lambert-ish).
    let water_opacity = clamp(thickness / 6.0, 0.0, 0.9);
    let through = mix(refracted, water_color, water_opacity);

    // Sky reflection blended over the through-water colour by Fresnel.
    let sky_color = vec3<f32>(0.6, 0.75, 0.95);
    let reflected = mix(through, sky_color, fresnel * 0.6);

    // Sun specular highlight
    let sun_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let H = normalize(V + sun_dir);
    let spec = pow(max(dot(N, H), 0.0), 128.0);
    let sun_color = vec3<f32>(1.0, 0.95, 0.8);

    // Foam on wave peaks (existing).
    let foam_intensity = smoothstep(uniforms.foam_threshold, uniforms.foam_threshold + 0.2, input.wave_height);
    var with_foam = mix(reflected, uniforms.foam_color, foam_intensity * 0.7);

    // Profile C — depth-delta intersection foam where water meets geometry.
    // A scrolling mask animates the band so it reads as moving shoreline foam.
    let shore = 1.0 - smoothstep(0.0, uniforms.foam_depth_band, thickness);
    let foam_scroll = 0.5 + 0.5 * sin(input.world_pos.x * 0.6 + input.world_pos.z * 0.6 - uniforms.time * 2.5);
    let shore_foam = shore * (0.55 + 0.45 * foam_scroll);
    with_foam = mix(with_foam, uniforms.foam_color, shore_foam);

    // Final color with specular.
    var final_color = with_foam + sun_color * spec * 0.8;

    // ── W.2c freeze material state ───────────────────────────────────────────────
    // A frozen patch reads as ice/glass: cool the colour and reduce the apparent
    // refraction (the icy tint overrides the see-through scene tap), then add sharp
    // specular glints. `input.freeze` already gates this to frozen regions only.
    if (input.freeze > 0.0) {
        let frozen_tint = vec3<f32>(0.72, 0.85, 0.95);
        final_color = mix(final_color, frozen_tint, input.freeze * 0.75);
        final_color = final_color + sun_color * spec * input.freeze * 1.2;
    }

    // The water composites the (refracted) scene behind it itself, so it outputs
    // opaque — its "see-through" comes from the refraction sample, not framebuffer
    // alpha (which would double-count the background under ALPHA_BLENDING).
    return vec4<f32>(final_color, 1.0);
}

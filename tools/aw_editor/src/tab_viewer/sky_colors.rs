//! Pure computation functions for sky colors, lighting, and fog/weather parameters.
//!
//! These functions are extracted from `EditorTabViewer` methods to keep
//! computation logic separate from UI state.

use crate::viewport::types::TerrainLightingParams;

/// Compute sky colors from skybox preset, time-of-day, weather, and fog settings.
///
/// Returns `(sky_top, sky_horizon, ground_color)` as `[f32; 4]` RGBA arrays.
pub(super) fn compute_sky_colors(
    world_skybox_preset: usize,
    world_time_of_day: f32,
    world_weather_preset: usize,
    world_fog_enabled: bool,
    world_fog_density: f32,
    world_ambient_color: [f32; 3],
) -> ([f32; 4], [f32; 4], [f32; 4]) {
    // Base colors from skybox preset
    let (mut top, mut horizon, mut ground) = match world_skybox_preset {
        0 => (
            // Clear Sky
            [0.1, 0.3, 0.8, 1.0],
            [0.5, 0.7, 0.95, 1.0],
            [0.2, 0.15, 0.1, 1.0],
        ),
        1 => (
            // Overcast
            [0.35, 0.38, 0.42, 1.0],
            [0.55, 0.55, 0.58, 1.0],
            [0.2, 0.18, 0.15, 1.0],
        ),
        2 => (
            // Sunset
            [0.15, 0.1, 0.35, 1.0],
            [0.95, 0.45, 0.2, 1.0],
            [0.25, 0.12, 0.08, 1.0],
        ),
        3 => (
            // Night
            [0.02, 0.02, 0.06, 1.0],
            [0.05, 0.05, 0.12, 1.0],
            [0.03, 0.03, 0.05, 1.0],
        ),
        4 => (
            // Space
            [0.0, 0.0, 0.02, 1.0],
            [0.01, 0.01, 0.04, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        ),
        5 => (
            // Gradient
            [0.25, 0.35, 0.65, 1.0],
            [0.55, 0.6, 0.75, 1.0],
            [0.2, 0.18, 0.15, 1.0],
        ),
        _ => (
            [0.1, 0.3, 0.8, 1.0],
            [0.5, 0.7, 0.95, 1.0],
            [0.2, 0.15, 0.1, 1.0],
        ),
    };

    // Modulate by time of day
    let time = world_time_of_day;
    let day_factor = if time >= 6.0 && time <= 18.0 {
        let mid = 12.0;
        let diff = (time - mid).abs();
        1.0 - (diff / 6.0) * 0.6 // 1.0 at noon, 0.4 at 6am/6pm
    } else {
        // Night: smooth transition
        let night_mid = if time > 18.0 { 24.0 } else { 0.0 };
        let dist = (time - night_mid)
            .abs()
            .min((time - 24.0 + night_mid).abs());
        0.1 + 0.3 * (dist / 6.0).min(1.0) // 0.1 at midnight, 0.4 toward sunrise/sunset
    };

    // Dawn/dusk warm tint (5-7 and 17-19)
    let dawn_dusk = if (5.0..7.0).contains(&time) {
        1.0 - (time - 6.0).abs()
    } else if (17.0..19.0).contains(&time) {
        1.0 - (time - 18.0).abs()
    } else {
        0.0
    };

    // Apply time modulation
    for c in &mut top[0..3] {
        *c *= day_factor;
    }
    for c in &mut horizon[0..3] {
        *c *= day_factor;
    }
    for c in &mut ground[0..3] {
        *c *= day_factor.max(0.15);
    }

    // Add warm tint at dawn/dusk
    if dawn_dusk > 0.0 {
        horizon[0] = (horizon[0] + 0.3 * dawn_dusk).min(1.0);
        horizon[1] = (horizon[1] + 0.1 * dawn_dusk).min(1.0);
        top[0] = (top[0] + 0.1 * dawn_dusk).min(1.0);
    }

    // Weather modulation
    match world_weather_preset {
        1 => {
            // Cloudy — desaturate and gray out
            let gray_top = (top[0] + top[1] + top[2]) / 3.0;
            let gray_hor = (horizon[0] + horizon[1] + horizon[2]) / 3.0;
            for i in 0..3 {
                top[i] = top[i] * 0.4 + gray_top * 0.6;
                horizon[i] = horizon[i] * 0.4 + gray_hor * 0.6;
            }
        }
        2 | 3 => {
            // Rain / Storm — darker, more gray
            let darken = if world_weather_preset == 3 { 0.3 } else { 0.5 };
            for i in 0..3 {
                top[i] *= darken;
                horizon[i] *= darken;
                ground[i] *= 0.7;
            }
            let gray = 0.15 * darken;
            top[0] = top[0] * 0.5 + gray;
            top[1] = top[1] * 0.5 + gray;
            top[2] = top[2] * 0.5 + gray;
        }
        4 => {
            // Snow — cool blue tint, brighter
            for i in 0..3 {
                top[i] = top[i] * 0.6 + 0.3;
                horizon[i] = horizon[i] * 0.6 + 0.35;
            }
            ground = [0.7, 0.72, 0.75, 1.0]; // snow on ground
        }
        5 => {
            // Fog — blend everything toward fog color
            let fog_color = [0.6, 0.6, 0.62];
            for i in 0..3 {
                top[i] = top[i] * 0.3 + fog_color[i] * 0.7;
                horizon[i] = fog_color[i];
                ground[i] = ground[i] * 0.3 + fog_color[i] * 0.7;
            }
        }
        6 => {
            // Sandstorm — orange-brown tint
            let sand = [0.7, 0.5, 0.25];
            for i in 0..3 {
                top[i] = top[i] * 0.3 + sand[i] * 0.7;
                horizon[i] = sand[i];
                ground[i] = sand[i] * 0.8;
            }
        }
        _ => {} // 0 = Clear, no change
    }

    // Fog blending (additional UI fog control)
    if world_fog_enabled {
        let fog_blend = (world_fog_density * 10.0).min(1.0);
        let fog_color = [
            world_ambient_color[0] * 0.5 + 0.3,
            world_ambient_color[1] * 0.5 + 0.3,
            world_ambient_color[2] * 0.5 + 0.3,
        ];
        for i in 0..3 {
            horizon[i] = horizon[i] * (1.0 - fog_blend) + fog_color[i] * fog_blend;
            top[i] = top[i] * (1.0 - fog_blend * 0.5) + fog_color[i] * fog_blend * 0.5;
        }
    }

    (top, horizon, ground)
}

/// Compute lighting parameters for the terrain shader.
pub(super) fn lighting_params(
    world_sun_elevation: f32,
    world_sun_azimuth: f32,
    world_sun_color: [f32; 3],
    world_sun_intensity: f32,
    world_ambient_color: [f32; 3],
    world_ambient_intensity: f32,
    world_exposure: f32,
) -> TerrainLightingParams {
    let elev = world_sun_elevation.to_radians();
    let azim = world_sun_azimuth.to_radians();
    let y = elev.sin();
    let xz = elev.cos();
    let x = xz * azim.cos();
    let z = xz * azim.sin();
    let len = (x * x + y * y + z * z).sqrt();
    let sun_dir = if len > 0.0001 {
        [x / len, y / len, z / len]
    } else {
        [0.0, 1.0, 0.0]
    };
    TerrainLightingParams {
        sun_dir,
        sun_color: world_sun_color,
        sun_intensity: world_sun_intensity,
        ambient_color: world_ambient_color,
        ambient_intensity: world_ambient_intensity,
        exposure: world_exposure,
    }
}

/// Compute fog/weather parameters from world settings.
///
/// Returns `(fog_enabled, fog_density, fog_start, fog_end, weather_preset, particle_count_override)`.
pub(super) fn fog_weather_params(
    world_fog_enabled: bool,
    world_fog_density: f32,
    world_fog_start: f32,
    world_fog_end: f32,
    world_weather_preset: usize,
    world_particle_count_override_enabled: bool,
    world_particle_count_value: u32,
) -> (bool, f32, f32, f32, u32, Option<u32>) {
    let particle_override = if world_particle_count_override_enabled {
        Some(world_particle_count_value)
    } else {
        None
    };
    (
        world_fog_enabled || world_weather_preset == 5,
        if world_weather_preset == 5 {
            0.012
        } else {
            world_fog_density
        },
        if world_weather_preset == 5 {
            20.0
        } else {
            world_fog_start
        },
        if world_weather_preset == 5 {
            150.0
        } else {
            world_fog_end
        },
        world_weather_preset as u32,
        particle_override,
    )
}

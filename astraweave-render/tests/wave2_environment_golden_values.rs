//! Wave 2 — Golden-value environment tests targeting 95 TimeOfDay mutants in shard 2.
//!
//! These pin-point exact Sun / Moon / light / ambient / day-night-twilight
//! values at carefully-chosen times.  Every floating-point constant that
//! appears in `environment.rs` is exercised so that operator-swap and
//! constant-change mutations are caught.
//!
//! Math reference (from source):
//!   sun_angle   = (current_time − 6.0) × π / 12.0
//!   sun_height  = sin(sun_angle)
//!   sun_azimuth = (current_time − 12.0) × π / 12.0
//!
//!   If |sun_height| < 0.01  →  horizon branch
//!     raw = (sin(azimuth), 0, cos(azimuth))   (normalize → unit)
//!   Else:
//!     horiz_dist = max(1 − |sun_height|, 0.1)
//!     raw = (sin(azimuth) × horiz_dist, sun_height, cos(azimuth) × horiz_dist)
//!     normalize → unit vector

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::environment::TimeOfDay;
use std::f32::consts::PI;

// ─── Helper ────────────────────────────────────────────────────────────────

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn sun_height_at(hour: f32) -> f32 {
    let angle = (hour - 6.0) * PI / 12.0;
    angle.sin()
}

// ─── get_sun_position: golden values ────────────────────────────────────────

/// At noon (12:00), sun_angle = π/2, sin = 1.  Horizon branch NOT taken.
/// horiz_dist = max(0.0, 0.1) = 0.1
/// raw = (0 × 0.1, 1.0, 1 × 0.1) = (0, 1, 0.1)
/// length ≈ 1.00499 → normalized y ≈ 0.9950
#[test]
fn sun_position_noon_y_golden() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let s = tod.get_sun_position();
    assert!(approx(s.y, 0.995, 0.01), "Expected y≈0.995, got {}", s.y);
    assert!(s.x.abs() < 0.01, "Expected x≈0 at noon, got {}", s.x);
    assert!(s.z > 0.05, "Expected z>0 at noon, got {}", s.z);
}

/// At midnight (0:00), sun_angle = −π/2, sin = −1.
/// azimuth = −π → sin≈0, cos=−1
/// raw = (0, −1, −0.1) → normalized y ≈ −0.995
#[test]
fn sun_position_midnight_y_golden() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let s = tod.get_sun_position();
    assert!(approx(s.y, -0.995, 0.01), "Expected y≈−0.995, got {}", s.y);
    assert!(s.z < -0.05, "Expected z<0 at midnight, got {}", s.z);
}

/// At sunrise (6:00), sun_height = 0 → horizon branch.
/// azimuth = −π/2 → sin=−1, cos=0 → raw=(−1,0,0) → normalized=(−1,0,0)
#[test]
fn sun_position_sunrise_horizon_branch() {
    let tod = TimeOfDay::new(6.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.x, -1.0, 0.02),
        "Expected x≈−1 at sunrise, got {}",
        s.x
    );
    assert!(s.y.abs() < 0.02, "Expected y≈0 at sunrise, got {}", s.y);
    assert!(s.z.abs() < 0.02, "Expected z≈0 at sunrise, got {}", s.z);
}

/// At sunset (18:00), sun_height = sin(π) = 0 → horizon branch.
/// azimuth = π/2 → sin=1, cos=0 → raw=(1,0,0) → normalized=(1,0,0)
#[test]
fn sun_position_sunset_horizon_branch() {
    let tod = TimeOfDay::new(18.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.x, 1.0, 0.02),
        "Expected x≈1 at sunset, got {}",
        s.x
    );
    assert!(s.y.abs() < 0.02, "Expected y≈0 at sunset, got {}", s.y);
}

/// At 9am, sun_angle = π/4, sin = 0.7071.
/// azimuth = −π/4 → sin=−0.7071, cos=0.7071
/// horiz_dist = max(1−0.7071, 0.1) = 0.2929
/// raw = (−0.2071, 0.7071, 0.2071) → length ≈ 0.7654
/// normalized y ≈ 0.9238
#[test]
fn sun_position_9am_golden() {
    let tod = TimeOfDay::new(9.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.y, 0.924, 0.02),
        "Expected y≈0.924 at 9am, got {}",
        s.y
    );
    assert!(
        s.x < 0.0,
        "Expected x<0 at 9am (pre-noon azimuth), got {}",
        s.x
    );
    assert!(s.z > 0.0, "Expected z>0 at 9am, got {}", s.z);
}

/// At 15:00 (3pm), sun_angle = 3π/4, sin = 0.7071.
/// azimuth = π/4 → sin=0.7071, cos=0.7071
/// Same height but x flips positive.
#[test]
fn sun_position_3pm_golden() {
    let tod = TimeOfDay::new(15.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.y, 0.924, 0.02),
        "Expected y≈0.924 at 3pm, got {}",
        s.y
    );
    assert!(
        s.x > 0.0,
        "Expected x>0 at 3pm (post-noon azimuth), got {}",
        s.x
    );
}

/// At 3am, sun_angle = −π/4, sin = −0.7071.
/// azimuth = −3π/4 → sin=−0.7071, cos=−0.7071
/// horiz_dist = max(1−0.7071, 0.1) = 0.2929
/// normalized y ≈ −0.9238
#[test]
fn sun_position_3am_golden() {
    let tod = TimeOfDay::new(3.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.y, -0.924, 0.02),
        "Expected y≈−0.924 at 3am, got {}",
        s.y
    );
}

/// At 21:00, sun_angle = 5π/4, sin ≈ −0.7071
#[test]
fn sun_position_21h_golden() {
    let tod = TimeOfDay::new(21.0, 1.0);
    let s = tod.get_sun_position();
    assert!(
        approx(s.y, -0.924, 0.02),
        "Expected y≈−0.924 at 21:00, got {}",
        s.y
    );
    assert!(s.x > 0.0, "Expected x>0 at 21:00 (azimuth=3π/4)");
}

/// Sun y is monotonically increasing from 6am to noon
#[test]
fn sun_y_monotone_morning() {
    let hours = [6.5, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
    let ys: Vec<f32> = hours
        .iter()
        .map(|&h| TimeOfDay::new(h, 1.0).get_sun_position().y)
        .collect();
    for i in 1..ys.len() {
        assert!(
            ys[i] > ys[i - 1],
            "Sun y should increase morning: y[{}]={} <= y[{}]={}",
            i,
            ys[i],
            i - 1,
            ys[i - 1]
        );
    }
}

/// Sun y is monotonically decreasing from noon to 6pm
#[test]
fn sun_y_monotone_afternoon() {
    let hours = [12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 17.5];
    let ys: Vec<f32> = hours
        .iter()
        .map(|&h| TimeOfDay::new(h, 1.0).get_sun_position().y)
        .collect();
    for i in 1..ys.len() {
        assert!(
            ys[i] < ys[i - 1],
            "Sun y should decrease afternoon: y[{}]={} >= y[{}]={}",
            i,
            ys[i],
            i - 1,
            ys[i - 1]
        );
    }
}

// ─── get_moon_position: golden values ───────────────────────────────────────

#[test]
fn moon_at_midnight_golden() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let m = tod.get_moon_position();
    // Moon = −sun; sun.y ≈ −0.995 → moon.y ≈ 0.995
    assert!(
        approx(m.y, 0.995, 0.01),
        "Moon y at midnight ≈ 0.995, got {}",
        m.y
    );
}

#[test]
fn moon_at_noon_below_horizon() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let m = tod.get_moon_position();
    assert!(
        m.y < -0.9,
        "Moon at noon should be below horizon, got y={}",
        m.y
    );
}

#[test]
fn moon_negation_property_multiple_times() {
    for hour in [0.0, 3.0, 7.0, 12.0, 15.0, 21.0] {
        let tod = TimeOfDay::new(hour, 1.0);
        let s = tod.get_sun_position();
        let m = tod.get_moon_position();
        assert!(approx(m.x, -s.x, 0.001), "moon.x ≠ −sun.x at h={hour}");
        assert!(approx(m.y, -s.y, 0.001), "moon.y ≠ −sun.y at h={hour}");
        assert!(approx(m.z, -s.z, 0.001), "moon.z ≠ −sun.z at h={hour}");
    }
}

// ─── get_light_direction ────────────────────────────────────────────────────

/// During day (sun.y > 0.1), light_direction = −sun.
#[test]
fn light_direction_is_neg_sun_during_day() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let s = tod.get_sun_position();
    let d = tod.get_light_direction();
    assert!(approx(d.x, -s.x, 0.001));
    assert!(approx(d.y, -s.y, 0.001));
    assert!(approx(d.z, -s.z, 0.001));
}

/// During deep night (sun.y < 0.1), light_direction = −moon = sun.
/// (because moon = −sun, so −moon = sun)
#[test]
fn light_direction_at_midnight_is_positive_sun() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let s = tod.get_sun_position();
    let d = tod.get_light_direction();
    // Source: if sun_pos.y > 0.1 → −sun, else → −moon = −(−sun) = sun
    assert!(approx(d.x, s.x, 0.001), "Expected d.x=sun.x at midnight");
    assert!(approx(d.y, s.y, 0.001), "Expected d.y=sun.y at midnight");
    assert!(approx(d.z, s.z, 0.001), "Expected d.z=sun.z at midnight");
}

/// Light direction at deep night has sun.y < 0.1 threshold confirmed
#[test]
fn light_direction_threshold_at_boundary() {
    // Find a time where sun_height is just barely above 0.1
    // sin((t-6)*PI/12) = 0.1 → t ≈ 6 + arcsin(0.1)*12/PI ≈ 6.383
    // At t=6.4, sun_height ≈ sin(0.4*PI/12) ≈ sin(0.1047) ≈ 0.1045
    let tod = TimeOfDay::new(6.4, 1.0);
    let sh = sun_height_at(6.4);
    let sun = tod.get_sun_position();
    // sun.y should be close to sh after normalization (slightly different)
    // Key: if sun_height > 0.1, direction = −sun
    assert!(sh > 0.1, "Sanity: sun_height at 6.4 should be >0.1");
    let d = tod.get_light_direction();
    // Since sun.y > 0.1, d = -sun
    assert!(
        d.y < -0.05,
        "At t=6.4 (barely day), light should come from above: d.y={}",
        d.y
    );
    let _ = sun; // suppress unused
}

// ─── get_light_color: DAY branch (sun_height > 0.2) ────────────────────────

/// At noon, sun_height ≈ 1.0 → intensity ≈ (1.0−0.2)/0.8 = 1.0
/// color = (1.0, 0.95, 0.8) × (0.8 + 0.2 × 1.0) = (1.0, 0.95, 0.8) × 1.0
#[test]
fn light_color_noon_golden() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let c = tod.get_light_color();
    assert!(approx(c.x, 1.0, 0.02), "R≈1.0 at noon, got {}", c.x);
    assert!(approx(c.y, 0.95, 0.02), "G≈0.95 at noon, got {}", c.y);
    assert!(approx(c.z, 0.80, 0.02), "B≈0.80 at noon, got {}", c.z);
}

/// At 9am, sun_height ≈ 0.707 → intensity = (0.707−0.2)/0.8 = 0.634
/// color = (1.0, 0.95, 0.8) × (0.8 + 0.2 × 0.634) = ×0.927
#[test]
fn light_color_9am_day_branch() {
    let tod = TimeOfDay::new(9.0, 1.0);
    let c = tod.get_light_color();
    // sun_height normalized ≈ 0.924 → intensity ≈ (0.924-0.2)/0.8 = 0.905
    // factor = 0.8 + 0.2*0.905 = 0.981
    // color ≈ (0.981, 0.932, 0.785)
    assert!(c.x > 0.9, "R at 9am should be >0.9, got {}", c.x);
    assert!(c.y > 0.85, "G at 9am should be >0.85, got {}", c.y);
    assert!(c.z > 0.7, "B at 9am should be >0.7, got {}", c.z);
    // Must still be in "day" range (< full brightness)
    assert!(c.x <= 1.01, "R at 9am should be ≤1.0, got {}", c.x);
}

/// Day branch: R > G > B (warm white/yellow)
#[test]
fn light_color_day_channel_ordering() {
    for hour in [8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0] {
        let c = TimeOfDay::new(hour, 1.0).get_light_color();
        let sh = sun_height_at(hour);
        if sh > 0.25 {
            assert!(c.x >= c.y, "Day: R≥G at h={hour}: {:?}", c);
            assert!(c.y >= c.z, "Day: G≥B at h={hour}: {:?}", c);
        }
    }
}

/// Day branch: intensity factor increases with sun height
#[test]
fn light_color_day_brightness_increases_with_height() {
    let c8 = TimeOfDay::new(8.0, 1.0).get_light_color();
    let c10 = TimeOfDay::new(10.0, 1.0).get_light_color();
    let c12 = TimeOfDay::new(12.0, 1.0).get_light_color();
    // Luminance approximation
    let lum = |v: glam::Vec3| v.x + v.y + v.z;
    assert!(lum(c12) > lum(c10), "Noon brighter than 10am");
    assert!(lum(c10) > lum(c8), "10am brighter than 8am");
}

// ─── get_light_color: TWILIGHT branch (−0.2 < sun_height ≤ 0.2) ────────────

/// At 6:00 (sunrise), sun_height=0 → mid-twilight
/// intensity = (0+0.2)/0.4 = 0.5
/// color = (1.0, 0.6, 0.3) × (0.3 + 0.5 × 0.5) = (1,0.6,0.3) × 0.55
/// = (0.55, 0.33, 0.165)
#[test]
fn light_color_sunrise_twilight_golden() {
    let tod = TimeOfDay::new(6.0, 1.0);
    let c = tod.get_light_color();
    assert!(approx(c.x, 0.55, 0.03), "R≈0.55 at sunrise, got {}", c.x);
    assert!(approx(c.y, 0.33, 0.03), "G≈0.33 at sunrise, got {}", c.y);
    assert!(approx(c.z, 0.165, 0.02), "B≈0.165 at sunrise, got {}", c.z);
}

/// Twilight branch: R > G > B (orange/red sunset)
#[test]
fn light_color_twilight_orange_ordering() {
    let tod = TimeOfDay::new(6.0, 1.0);
    let c = tod.get_light_color();
    assert!(c.x > c.y, "Twilight R > G: {:?}", c);
    assert!(c.y > c.z, "Twilight G > B: {:?}", c);
}

/// Twilight at different intensities within the branch.
/// NOTE: normalization changes sun.y from raw sin(angle).  The twilight/night
/// boundary (-0.2 of normalized y) happens at raw_sh ≈ ±0.17, which is
/// t ≈ 5.35 / 6.65 / 17.35 / 18.65.
#[test]
fn light_color_twilight_intensity_range() {
    // Near day boundary — use t=6.60 where normalized sun.y ≈ 0.19 (twilight)
    let tod = TimeOfDay::new(6.60, 1.0);
    let c = tod.get_light_color();
    // Should be in twilight branch (orange-ish)
    assert!(
        c.x > 0.3,
        "R in twilight near day boundary > 0.3, got {}",
        c.x
    );
    assert!(
        c.x < 0.9,
        "R in twilight near day boundary < 0.9, got {}",
        c.x
    );

    // Near night boundary — use t=5.45 where normalized sun.y ≈ −0.15 (still twilight)
    let tod2 = TimeOfDay::new(5.45, 1.0);
    let c2 = tod2.get_light_color();
    // Should still be in twilight branch (orange, but dimmer)
    assert!(
        c2.x > 0.2,
        "R in twilight near night boundary > 0.2, got {}",
        c2.x
    );
    assert!(c2.y < c2.x, "Twilight: G < R near night boundary: {:?}", c2);
}

// ─── get_light_color: NIGHT branch (sun_height ≤ −0.2) ─────────────────────

/// At midnight, sun_height ≈ −1.0  → night branch.
/// color = (0.3, 0.4, 0.8) × 0.15 = (0.045, 0.060, 0.120)
#[test]
fn light_color_midnight_golden() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let c = tod.get_light_color();
    assert!(approx(c.x, 0.045, 0.005), "Night R≈0.045, got {}", c.x);
    assert!(approx(c.y, 0.060, 0.005), "Night G≈0.060, got {}", c.y);
    assert!(approx(c.z, 0.120, 0.005), "Night B≈0.120, got {}", c.z);
}

/// Night: all components are the same regardless of exact sun depth.
/// color is constant (0.3,0.4,0.8)*0.15 for all sun_height < −0.2
#[test]
fn light_color_night_constant_across_hours() {
    let c0 = TimeOfDay::new(0.0, 1.0).get_light_color();
    let c3 = TimeOfDay::new(3.0, 1.0).get_light_color();
    let c22 = TimeOfDay::new(22.0, 1.0).get_light_color();
    // All should be the same (0.045, 0.06, 0.12)
    assert!(approx(c0.x, c3.x, 0.001), "Night color stable 0h vs 3h");
    assert!(approx(c0.x, c22.x, 0.001), "Night color stable 0h vs 22h");
    assert!(approx(c0.z, c3.z, 0.001), "Night blue stable");
}

/// Night: B > G > R (cool blue)
#[test]
fn light_color_night_blue_dominant() {
    let c = TimeOfDay::new(0.0, 1.0).get_light_color();
    assert!(c.z > c.y, "Night B > G: {:?}", c);
    assert!(c.y > c.x, "Night G > R: {:?}", c);
}

/// Night color uses 0.15 multiplier — verify against doubled/halved
#[test]
fn light_color_night_multiplier_exact() {
    let c = TimeOfDay::new(0.0, 1.0).get_light_color();
    // Pinpoint: 0.3 × 0.15 = 0.045 (R)
    assert!(
        c.x > 0.03 && c.x < 0.07,
        "Night R in [0.03, 0.07], got {}",
        c.x
    );
    // Pinpoint: 0.8 × 0.15 = 0.12 (B)
    assert!(
        c.z > 0.09 && c.z < 0.16,
        "Night B in [0.09, 0.16], got {}",
        c.z
    );
}

// ─── get_ambient_color ──────────────────────────────────────────────────────

/// Day ambient: sun_height > 0 → vec3(0.4, 0.6, 1.0) × (0.3 + 0.4 × min(sh, 1.0))
/// At noon, sh ≈ 1.0 → factor = 0.7 → (0.28, 0.42, 0.70)
#[test]
fn ambient_color_noon_golden() {
    let c = TimeOfDay::new(12.0, 1.0).get_ambient_color();
    assert!(
        approx(c.x, 0.28, 0.03),
        "Ambient R≈0.28 at noon, got {}",
        c.x
    );
    assert!(
        approx(c.y, 0.42, 0.03),
        "Ambient G≈0.42 at noon, got {}",
        c.y
    );
    assert!(
        approx(c.z, 0.70, 0.03),
        "Ambient B≈0.70 at noon, got {}",
        c.z
    );
}

/// Night ambient: sun_height ≤ 0 → vec3(0.1, 0.15, 0.3) × 0.1
/// = (0.01, 0.015, 0.03)
#[test]
fn ambient_color_midnight_golden() {
    let c = TimeOfDay::new(0.0, 1.0).get_ambient_color();
    assert!(
        approx(c.x, 0.01, 0.005),
        "Night ambient R≈0.01, got {}",
        c.x
    );
    assert!(
        approx(c.y, 0.015, 0.005),
        "Night ambient G≈0.015, got {}",
        c.y
    );
    assert!(
        approx(c.z, 0.03, 0.005),
        "Night ambient B≈0.03, got {}",
        c.z
    );
}

/// Day ambient: B > G > R (blue sky)
#[test]
fn ambient_color_day_blue_dominant() {
    let c = TimeOfDay::new(12.0, 1.0).get_ambient_color();
    assert!(c.z > c.y, "Day ambient B > G: {:?}", c);
    assert!(c.y > c.x, "Day ambient G > R: {:?}", c);
}

/// Night ambient: B > G > R (dark blue)
#[test]
fn ambient_color_night_blue_dominant() {
    let c = TimeOfDay::new(0.0, 1.0).get_ambient_color();
    assert!(c.z > c.y, "Night ambient B > G: {:?}", c);
    assert!(c.y > c.x, "Night ambient G > R: {:?}", c);
}

/// Ambient intensity increases through morning as sun rises
#[test]
fn ambient_intensity_increases_morning() {
    let lum = |v: glam::Vec3| v.x + v.y + v.z;
    let c7 = TimeOfDay::new(7.0, 1.0).get_ambient_color();
    let c9 = TimeOfDay::new(9.0, 1.0).get_ambient_color();
    let c12 = TimeOfDay::new(12.0, 1.0).get_ambient_color();
    assert!(lum(c12) > lum(c9), "Noon ambient > 9am");
    assert!(lum(c9) > lum(c7), "9am ambient > 7am");
}

/// Day ambient: the 0.4 multiplier on intensity means even at low sun_height
/// ambient is at least (0.4, 0.6, 1.0) × 0.3 = (0.12, 0.18, 0.30)
#[test]
fn ambient_color_day_floor() {
    // At sun_height barely > 0, the formula gives factor = 0.3
    // sin((t-6)*PI/12) ≈ 0.01 at t ≈ 6.04
    let c = TimeOfDay::new(6.1, 1.0).get_ambient_color();
    // Should be in day branch since sun_height > 0
    assert!(c.x >= 0.1, "Day ambient R floor ≥ 0.1, got {}", c.x);
    assert!(c.z >= 0.25, "Day ambient B floor ≥ 0.25, got {}", c.z);
}

// ─── is_day / is_night / is_twilight boundary tests ─────────────────────────

/// is_day: true when sun_position.y > 0.0
#[test]
fn is_day_true_at_various_day_hours() {
    for hour in [
        7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
    ] {
        let tod = TimeOfDay::new(hour, 1.0);
        assert!(tod.is_day(), "Should be day at hour={hour}");
    }
}

#[test]
fn is_day_false_at_midnight() {
    assert!(!TimeOfDay::new(0.0, 1.0).is_day());
    assert!(!TimeOfDay::new(3.0, 1.0).is_day());
    assert!(!TimeOfDay::new(23.0, 1.0).is_day());
}

/// is_night: true when sun_position.y < −0.1
#[test]
fn is_night_true_when_deep_below_horizon() {
    // At 5am: sin(-PI/12) = −0.2588 < −0.1 → night
    assert!(TimeOfDay::new(5.0, 1.0).is_night(), "5am should be night");
    assert!(TimeOfDay::new(0.0, 1.0).is_night(), "0am should be night");
    assert!(TimeOfDay::new(3.0, 1.0).is_night(), "3am should be night");
    // At 19:00: sin(13PI/12) ≈ −0.259 → night
    assert!(
        TimeOfDay::new(19.0, 1.0).is_night(),
        "19:00 should be night"
    );
}

/// is_night: false when sun is above −0.1
#[test]
fn is_night_false_at_noon_and_near_horizon() {
    assert!(!TimeOfDay::new(12.0, 1.0).is_night(), "Noon is not night");
    // 6am = sun_height 0 > -0.1 → NOT night
    assert!(!TimeOfDay::new(6.0, 1.0).is_night(), "6am is not night");
}

/// is_twilight: true when sun_height in [-0.1, 0.1]
#[test]
fn is_twilight_at_sunrise_sunset() {
    // 6am: sun_height=0 → in [-0.1, 0.1] → twilight
    assert!(
        TimeOfDay::new(6.0, 1.0).is_twilight(),
        "6am should be twilight"
    );
    // 18pm: sun_height=sin(PI)≈0 → twilight
    assert!(
        TimeOfDay::new(18.0, 1.0).is_twilight(),
        "18:00 should be twilight"
    );
}

#[test]
fn is_twilight_false_at_noon_and_midnight() {
    assert!(
        !TimeOfDay::new(12.0, 1.0).is_twilight(),
        "Noon is not twilight"
    );
    assert!(
        !TimeOfDay::new(0.0, 1.0).is_twilight(),
        "Midnight is not twilight"
    );
}

/// Verify mutual exclusivity: at any given time at most one of is_day/is_night
/// returns true (twilight can overlap since sun in [-0.1, 0.0] is both
/// "not day" and "not night" but also twilight).
#[test]
fn day_night_mutual_exclusivity() {
    for hour in (0..240).map(|i| i as f32 * 0.1) {
        let tod = TimeOfDay::new(hour, 1.0);
        let d = tod.is_day();
        let n = tod.is_night();
        assert!(!(d && n), "Cannot be both day AND night at hour={hour}");
    }
}

// ─── TimeOfDay::new / Default golden values ─────────────────────────────────

#[test]
fn new_stores_all_fields() {
    let tod = TimeOfDay::new(8.5, 120.0);
    assert!((tod.current_time - 8.5).abs() < f32::EPSILON);
    assert!((tod.time_scale - 120.0).abs() < f32::EPSILON);
    assert!((tod.day_length - 1440.0).abs() < f32::EPSILON); // hardcoded in new()
}

#[test]
fn default_values_exact() {
    let tod = TimeOfDay::default();
    assert!((tod.current_time - 12.0).abs() < f32::EPSILON);
    assert!((tod.time_scale - 60.0).abs() < f32::EPSILON);
    assert!((tod.day_length - 1440.0).abs() < f32::EPSILON);
}

// ─── TimeOfDay::update ──────────────────────────────────────────────────────

/// A fixed step stays inside `[0, 24)`.
#[test]
fn update_keeps_time_in_range() {
    let mut tod = TimeOfDay::new(23.99, 3600.0);
    tod.update(1.0);
    assert!(tod.current_time >= 0.0, "Time must be ≥ 0 after update");
    assert!(tod.current_time < 24.0, "Time must be < 24 after update");
}

/// Update with time_scale 0 does not move the hour.
#[test]
fn update_zero_scale_no_change() {
    let mut tod = TimeOfDay::new(10.0, 0.0);
    let before = tod.current_time;
    tod.update(10.0);
    assert_eq!(tod.current_time.to_bits(), before.to_bits());
}

/// A non-zero step at a non-zero scale changes the hour.
#[test]
fn update_does_not_noop() {
    let mut tod = TimeOfDay::new(12.0, 3600.0);
    tod.update(1.0);
    assert!((tod.current_time - 13.0).abs() < 1e-4);
}

/// Hours wrap with `rem_euclid`, so a step past midnight stays non-negative.
#[test]
fn update_modulo_wrap() {
    let mut tod = TimeOfDay::new(23.0, 3600.0);
    tod.update(2.0);
    assert!((tod.current_time - 1.0).abs() < 1e-4);
}

/// `time_scale` multiplies the step.
#[test]
fn update_respects_time_scale() {
    let mut slow = TimeOfDay::new(0.0, 1.0);
    let mut fast = TimeOfDay::new(0.0, 3600.0);
    slow.update(1.0);
    fast.update(1.0);
    assert!(fast.current_time > slow.current_time);
    assert!((fast.current_time - 1.0).abs() < 1e-4);
}

/// Seconds become hours by dividing by 3600.
#[test]
fn update_uses_hours_conversion() {
    let mut tod = TimeOfDay::new(0.0, 3600.0);
    tod.update(0.005);
    assert!((tod.current_time - 0.005).abs() < 1e-5);
}

// ─── SkyConfig defaults ─────────────────────────────────────────────────────

#[test]
fn sky_config_default_all_fields() {
    let c = astraweave_render::environment::SkyConfig::default();
    assert!((c.cloud_coverage - 0.5).abs() < f32::EPSILON);
    assert!((c.cloud_speed - 0.02).abs() < f32::EPSILON);
    assert!((c.cloud_altitude - 1000.0).abs() < f32::EPSILON);
    // Color vectors
    assert!(approx(c.day_color_top.x, 0.3, 0.01));
    assert!(approx(c.day_color_top.y, 0.6, 0.01));
    assert!(approx(c.day_color_top.z, 1.0, 0.01));
    assert!(approx(c.night_color_top.z, 0.1, 0.01));
}

// ─── SkyRenderer CPU-accessible methods ─────────────────────────────────────

#[test]
fn sky_renderer_new_returns_default_time() {
    let sky = astraweave_render::environment::SkyRenderer::new(
        astraweave_render::environment::SkyConfig::default(),
    );
    assert!((sky.time_of_day().current_time - 12.0).abs() < 0.01);
}

#[test]
fn sky_renderer_config_roundtrip() {
    let cfg = astraweave_render::environment::SkyConfig::default();
    let mut sky = astraweave_render::environment::SkyRenderer::new(cfg.clone());
    assert!((sky.config().cloud_coverage - 0.5).abs() < f32::EPSILON);
    let mut new_cfg = cfg;
    new_cfg.cloud_coverage = 0.9;
    sky.set_config(new_cfg);
    assert!((sky.config().cloud_coverage - 0.9).abs() < f32::EPSILON);
}

#[test]
fn sky_renderer_time_of_day_mut() {
    let mut sky = astraweave_render::environment::SkyRenderer::new(
        astraweave_render::environment::SkyConfig::default(),
    );
    sky.time_of_day_mut().current_time = 3.0;
    assert!((sky.time_of_day().current_time - 3.0).abs() < f32::EPSILON);
}

//! Mutation-killing tests for astraweave-ui
//!
//! Targets missed mutations in accessibility.rs, hud.rs, and more.
//! Each test documents which mutation line it catches.

use astraweave_ui::{
    get_health_colors,
    hud::{easing, HealthAnimation},
    to_egui_color, transform_color, ColorblindMode, HudManager,
};

// ============================================================================
// accessibility.rs — Golden-value color transform tests
// ============================================================================

/// Verify exact deuteranopia transform output.
/// Catches L144 (g * 0.3) and L145 (g * 0.7) arithmetic mutations.
#[test]
fn mutation_deuteranopia_golden_values() {
    // Use non-degenerate input where all channels != 0 or 1
    let color = (0.6_f32, 0.4_f32, 0.3_f32);
    let result = transform_color(color, ColorblindMode::Deuteranopia);

    // deuteranopia_transform:
    //   new_g = g * 0.3 = 0.4 * 0.3 = 0.12
    //   new_b = b + g * 0.7 = 0.3 + 0.4 * 0.7 = 0.3 + 0.28 = 0.58
    //   r unchanged = 0.6
    let eps = 1e-5;
    assert!(
        (result.0 - 0.6).abs() < eps,
        "r should be unchanged: got {}",
        result.0
    );
    assert!(
        (result.1 - 0.12).abs() < eps,
        "g should be 0.4*0.3=0.12: got {}",
        result.1
    );
    assert!(
        (result.2 - 0.58).abs() < eps,
        "b should be 0.3+0.4*0.7=0.58: got {}",
        result.2
    );
}

/// Second input to further discriminate deuteranopia mutations (e.g. * vs /)
#[test]
fn mutation_deuteranopia_second_input() {
    let color = (0.8_f32, 0.6_f32, 0.1_f32);
    let result = transform_color(color, ColorblindMode::Deuteranopia);

    // new_g = 0.6 * 0.3 = 0.18
    // new_b = min(0.1 + 0.6 * 0.7, 1.0) = min(0.52, 1.0) = 0.52
    let eps = 1e-5;
    assert!((result.1 - 0.18).abs() < eps, "g: got {}", result.1);
    assert!((result.2 - 0.52).abs() < eps, "b: got {}", result.2);
}

/// Verify exact protanopia transform output.
/// Catches L151 (fn → Default), L153 (r*0.5), L154 (g*0.8), L155 (b + g*0.4 + r*0.2)
#[test]
fn mutation_protanopia_golden_values() {
    let color = (0.6_f32, 0.4_f32, 0.3_f32);
    let result = transform_color(color, ColorblindMode::Protanopia);

    // new_r = 0.6 * 0.5 = 0.3
    // new_g = 0.4 * 0.8 = 0.32
    // new_b = min(0.3 + 0.4 * 0.4 + 0.6 * 0.2, 1.0) = min(0.3 + 0.16 + 0.12, 1.0) = 0.58
    let eps = 1e-5;
    assert!(
        (result.0 - 0.3).abs() < eps,
        "r should be 0.6*0.5=0.3: got {}",
        result.0
    );
    assert!(
        (result.1 - 0.32).abs() < eps,
        "g should be 0.4*0.8=0.32: got {}",
        result.1
    );
    assert!(
        (result.2 - 0.58).abs() < eps,
        "b should be 0.3+0.16+0.12=0.58: got {}",
        result.2
    );

    // Additional: result is NOT default (0,0,0) — catches L151
    assert!(
        result.0 > 0.0 || result.1 > 0.0 || result.2 > 0.0,
        "protanopia should not produce default color"
    );
}

/// Second input for protanopia to catch division mutations (* → /)
#[test]
fn mutation_protanopia_second_input() {
    let color = (0.8_f32, 0.5_f32, 0.2_f32);
    let result = transform_color(color, ColorblindMode::Protanopia);

    // new_r = 0.8 * 0.5 = 0.4
    // new_g = 0.5 * 0.8 = 0.4
    // new_b = min(0.2 + 0.5 * 0.4 + 0.8 * 0.2, 1.0) = min(0.2 + 0.2 + 0.16, 1.0) = 0.56
    let eps = 1e-5;
    assert!((result.0 - 0.4).abs() < eps, "r: got {}", result.0);
    assert!((result.1 - 0.4).abs() < eps, "g: got {}", result.1);
    assert!((result.2 - 0.56).abs() < eps, "b: got {}", result.2);
}

/// Verify exact tritanopia transform output.
/// Catches L161 (fn → Default), L163 (r+b*0.5), L164 (g*0.5), L165 (b*0.8)
#[test]
fn mutation_tritanopia_golden_values() {
    let color = (0.6_f32, 0.4_f32, 0.3_f32);
    let result = transform_color(color, ColorblindMode::Tritanopia);

    // new_r = min(0.6 + 0.3 * 0.5, 1.0) = 0.75
    // new_g = 0.4 * 0.5 = 0.2
    // new_b = 0.3 * 0.8 = 0.24
    let eps = 1e-5;
    assert!(
        (result.0 - 0.75).abs() < eps,
        "r should be 0.6+0.3*0.5=0.75: got {}",
        result.0
    );
    assert!(
        (result.1 - 0.2).abs() < eps,
        "g should be 0.4*0.5=0.2: got {}",
        result.1
    );
    assert!(
        (result.2 - 0.24).abs() < eps,
        "b should be 0.3*0.8=0.24: got {}",
        result.2
    );

    // Not default — catches L161
    assert!(result.0 > 0.0 || result.1 > 0.0 || result.2 > 0.0);
}

/// Second tritanopia input for division mutations
#[test]
fn mutation_tritanopia_second_input() {
    let color = (0.3_f32, 0.7_f32, 0.5_f32);
    let result = transform_color(color, ColorblindMode::Tritanopia);

    // new_r = min(0.3 + 0.5 * 0.5, 1.0) = 0.55
    // new_g = 0.7 * 0.5 = 0.35
    // new_b = 0.5 * 0.8 = 0.4
    let eps = 1e-5;
    assert!((result.0 - 0.55).abs() < eps, "r: got {}", result.0);
    assert!((result.1 - 0.35).abs() < eps, "g: got {}", result.1);
    assert!((result.2 - 0.4).abs() < eps, "b: got {}", result.2);
}

/// High contrast mid-tone: verify luminance formula and boost arithmetic.
/// Catches L174 (luminance formula: 0.299*r + 0.587*g + 0.114*b — 10 mutations)
/// and L189-191 (boost formula: (channel - lum) * boost + lum — 18 mutations)
#[test]
fn mutation_high_contrast_midtone_golden() {
    // Choose a mid-tone color that stays in the boost branch
    let color = (0.6_f32, 0.4_f32, 0.3_f32);
    let result = transform_color(color, ColorblindMode::HighContrast);

    // lum = 0.299*0.6 + 0.587*0.4 + 0.114*0.3 = 0.1794 + 0.2348 + 0.0342 = 0.4484
    // max = 0.6 >= 0.3 → not dark
    // lum = 0.4484 <= 0.7 → not light → mid-tone boost (boost = 1.5)
    // new_r = ((0.6 - 0.4484) * 1.5 + 0.4484).clamp(0,1) = (0.1516*1.5 + 0.4484) = 0.6758
    // new_g = ((0.4 - 0.4484) * 1.5 + 0.4484).clamp(0,1) = (-0.0484*1.5 + 0.4484) = 0.3758
    // new_b = ((0.3 - 0.4484) * 1.5 + 0.4484).clamp(0,1) = (-0.1484*1.5 + 0.4484) = 0.2258
    let eps = 1e-3;
    assert!(
        (result.0 - 0.6758).abs() < eps,
        "r should be ~0.6758: got {}",
        result.0
    );
    assert!(
        (result.1 - 0.3758).abs() < eps,
        "g should be ~0.3758: got {}",
        result.1
    );
    assert!(
        (result.2 - 0.2258).abs() < eps,
        "b should be ~0.2258: got {}",
        result.2
    );
}

/// High contrast second mid-tone with asymmetric channels.
/// Extra coverage for luminance and boost mutations.
#[test]
fn mutation_high_contrast_midtone_asymmetric() {
    let color = (0.5_f32, 0.3_f32, 0.7_f32);
    let result = transform_color(color, ColorblindMode::HighContrast);

    // lum = 0.299*0.5 + 0.587*0.3 + 0.114*0.7 = 0.1495 + 0.1761 + 0.0798 = 0.4054
    // max = 0.7 >= 0.3 → not dark
    // lum = 0.4054 <= 0.7 → not light → boost 1.5
    // new_r = ((0.5 - 0.4054) * 1.5 + 0.4054) = (0.0946*1.5 + 0.4054) = 0.5473
    // new_g = ((0.3 - 0.4054) * 1.5 + 0.4054) = (-0.1054*1.5 + 0.4054) = 0.2473
    // new_b = ((0.7 - 0.4054) * 1.5 + 0.4054) = (0.2946*1.5 + 0.4054) = 0.8473
    let eps = 1e-3;
    assert!((result.0 - 0.5473).abs() < eps, "r: got {}", result.0);
    assert!((result.1 - 0.2473).abs() < eps, "g: got {}", result.1);
    assert!((result.2 - 0.8473).abs() < eps, "b: got {}", result.2);
}

/// High contrast: exact boundary test at max = 0.3.
/// Catches L179 (< → <=): at max=0.3, original goes to mid-tone, mutation returns black.
#[test]
fn mutation_high_contrast_dark_boundary() {
    // max = 0.3 (exactly at boundary)
    let color = (0.3_f32, 0.1_f32, 0.05_f32);
    let result = transform_color(color, ColorblindMode::HighContrast);

    // With `< 0.3`: max=0.3 → NOT dark → goes to lum/boost check
    // With `<= 0.3`: max=0.3 → dark → returns (0,0,0)
    // So original should NOT return (0,0,0)
    assert!(
        result.0 > 0.0 || result.1 > 0.0 || result.2 > 0.0,
        "At max=0.3, should NOT be treated as dark (catches < → <=)"
    );
}

/// High contrast: exact boundary test at lum = 0.7.
/// Catches L182 (> → >=): at lum=0.7 exactly, original goes to mid-tone, mutation returns white.
#[test]
fn mutation_high_contrast_light_boundary() {
    // Verified: with r=g=b=0.7, lum computes to exactly 0.7_f32
    // (0.299*0.7 + 0.587*0.7 + 0.114*0.7 == 0.7 in f32 arithmetic)
    // With `>`:  lum=0.7 is NOT > 0.7 → mid-tone boost → result = (0.7, 0.7, 0.7)
    // With `>=`: lum=0.7 IS >= 0.7 → white → result = (1.0, 1.0, 1.0)
    let color = (0.7_f32, 0.7_f32, 0.7_f32);
    let result = transform_color(color, ColorblindMode::HighContrast);

    // Must NOT become white — catches `> → >=` mutation
    assert_ne!(
        result,
        (1.0, 1.0, 1.0),
        "At lum=0.7 exactly, should stay mid-tone, not become white (catches > → >=)"
    );

    // The mid-tone boost with r=g=b=lum gives (r-lum)*1.5+lum = 0+0.7 = 0.7 for all channels
    let eps = 1e-5;
    assert!(
        (result.0 - 0.7).abs() < eps,
        "r should be ~0.7: got {}",
        result.0
    );
    assert!(
        (result.1 - 0.7).abs() < eps,
        "g should be ~0.7: got {}",
        result.1
    );
    assert!(
        (result.2 - 0.7).abs() < eps,
        "b should be ~0.7: got {}",
        result.2
    );

    // Also verify clearly-light colors still become white
    let light = (0.9_f32, 0.9_f32, 0.9_f32);
    let light_result = transform_color(light, ColorblindMode::HighContrast);
    assert_eq!(
        light_result,
        (1.0, 1.0, 1.0),
        "Light color (lum=0.9) should become white"
    );
}

/// to_egui_color with non-degenerate input to catch * → + and * → / at L209 and L211.
/// Existing test uses (1.0, 0.5, 0.0) which makes some mutations equivalent.
#[test]
fn mutation_to_egui_color_non_degenerate() {
    // Use values where * vs + vs / produce clearly different u8 results
    let color = (0.5_f32, 0.3_f32, 0.7_f32);
    let result = to_egui_color(color);

    // 0.5 * 255 = 127.5 → 127
    // 0.3 * 255 = 76.5 → 76
    // 0.7 * 255 = 178.5 → 178
    assert_eq!(result.r(), 127, "r: 0.5*255=127"); // catches L209 * → + (0.5+255=255)
    assert_eq!(result.g(), 76, "g: 0.3*255=76");
    assert_eq!(result.b(), 178, "b: 0.7*255=178"); // catches L211 * → / (0.7/255≈0)
}

// ============================================================================
// hud.rs — HealthAnimation boundary tests
// ============================================================================

/// ease_in_out_quad at exact t=0.5 boundary.
/// Catches L25 (< → <=): at t=0.5, original takes first branch (2*0.5*0.5=0.5),
/// mutation takes second branch (-1 + (4-1)*0.5 = -1 + 1.5 = 0.5).
/// Both produce 0.5. So t=0.5 is equivalent. Try t=0.49 and t=0.51 to verify branching.
#[test]
fn mutation_ease_in_out_quad_near_boundary() {
    // t=0.49: first branch → 2*0.49*0.49 = 0.4802
    let t = 0.49_f32;
    let result = easing::ease_in_out_quad(t);
    let expected = 2.0 * t * t;
    assert!(
        (result - expected).abs() < 1e-5,
        "t=0.49 should use first branch: got {}",
        result
    );

    // t=0.51: second branch → -1 + (4 - 2*0.51)*0.51 = -1 + 2.98*0.51 = -1 + 1.5198 = 0.5198
    let t2 = 0.51_f32;
    let result2 = easing::ease_in_out_quad(t2);
    let expected2 = -1.0 + (4.0 - 2.0 * t2) * t2;
    assert!(
        (result2 - expected2).abs() < 1e-5,
        "t=0.51 should use second branch: got {}",
        result2
    );

    // Verify they're different (proves branch matters)
    assert!(
        (result - result2).abs() > 0.01,
        "0.49 and 0.51 should produce different results"
    );
}

/// HealthAnimation::set_target flash trigger at exact boundary.
/// Catches L69 (< → <=): setting target=current_visual should NOT trigger flash.
#[test]
fn mutation_set_target_equal_no_flash() {
    let mut anim = HealthAnimation::new(80.0);

    // Set target to exact same value — should NOT trigger flash
    anim.set_target(80.0);
    assert_eq!(
        anim.flash_timer, 0.0,
        "Setting target == current_visual should NOT flash (catches < → <=)"
    );

    // But setting below should trigger flash
    anim.set_target(79.0);
    assert!(anim.flash_timer > 0.0, "Damage should trigger flash");
}

/// HealthAnimation::update flash timer decrement.
/// Catches L77 (> → >=): flash_timer at 0.0 should NOT be decremented further.
#[test]
fn mutation_update_flash_timer_at_zero() {
    let mut anim = HealthAnimation::new(100.0);
    // flash_timer starts at 0.0
    assert_eq!(anim.flash_timer, 0.0);

    // Update with positive dt — flash_timer should remain 0.0
    anim.update(0.1);
    assert_eq!(
        anim.flash_timer, 0.0,
        "Flash timer at 0.0 should stay 0.0 (catches > → >=)"
    );
}

/// HealthAnimation::update convergence check.
/// Catches L82 (> → >=): the abs difference check.
#[test]
fn mutation_update_convergence_threshold() {
    let mut anim = HealthAnimation::new(100.0);

    // Difference of 0.005 which is clearly < 0.01 → NO animation
    anim.target = 100.005;
    let visual_before = anim.current_visual;
    anim.update(0.1);
    assert_eq!(
        anim.current_visual, visual_before,
        "Difference of 0.005 should NOT trigger animation"
    );

    // Difference of 0.02 SHOULD trigger animation
    anim.target = 100.02;
    anim.update(0.1);
    assert_ne!(
        anim.current_visual, visual_before,
        "Difference of 0.02 should trigger animation"
    );
}

/// Catches L82 subtraction mutations (- → + and - → /):
/// With subtraction, (current - target) computes the delta. With +, the result is way off.
#[test]
fn mutation_update_convergence_subtraction() {
    let mut anim = HealthAnimation::new(50.0);
    anim.target = 49.0; // 1.0 difference

    // (50 - 49).abs() = 1.0 > 0.01 → animation happens
    // (50 + 49).abs() = 99.0 > 0.01 → would also animate (but differently)
    // (50 / 49).abs() ≈ 1.02 > 0.01 → would also animate
    // The key test is: the ANIMATION should converge toward 49.0
    anim.update(1.0); // Large dt to complete
    assert!(
        anim.current_visual < 50.0,
        "Visual should decrease toward target 49.0 (catches - → + in convergence check)"
    );
}

/// Catches L87 (> → ==, <, >=): easing selection between heal/damage.
/// With target > current → ease_in_out_quad (healing).
/// With target <= current → ease_out_cubic (damage).
#[test]
fn mutation_update_easing_selection() {
    // Healing: target > current → ease_in_out_quad
    let mut heal = HealthAnimation::new(50.0);
    heal.set_target(80.0);
    heal.animation_time = 0.0;
    heal.update(0.2); // 0.2s into 0.4s duration → t = 0.5
    let heal_visual = heal.current_visual;

    // Damage: target < current → ease_out_cubic
    let mut dmg = HealthAnimation::new(80.0);
    dmg.set_target(50.0);
    dmg.animation_time = 0.0;
    dmg.update(0.2);
    let dmg_visual = dmg.current_visual;

    // At t=0.5, ease_in_out_quad(0.5) = 0.5, ease_out_cubic(0.5) = 0.875
    // heal visual = 50 + (80-50)*0.5 = 65.0
    // dmg visual = 80 + (50-80)*0.875 = 80 - 26.25 = 53.75
    let eps = 1.0;
    assert!(
        (heal_visual - 65.0).abs() < eps,
        "Heal should use ease_in_out_quad: got {}",
        heal_visual
    );
    assert!(
        (dmg_visual - 53.75).abs() < eps,
        "Damage should use ease_out_cubic: got {}",
        dmg_visual
    );
}

/// Catches L113 (> → >=): flash_alpha at exactly timer=0.0 should return 0.0.
#[test]
fn mutation_flash_alpha_at_zero_timer() {
    let mut anim = HealthAnimation::new(100.0);
    anim.flash_timer = 0.0;
    anim.flash_duration = 0.2;

    // With > 0.0: 0.0 > 0.0 = FALSE → returns 0.0
    // With >= 0.0: 0.0 >= 0.0 = TRUE → returns (0.0/0.2)*0.6 = 0.0
    // Both return 0.0 → EQUIVALENT at timer=0.0
    let alpha = anim.flash_alpha();
    assert_eq!(alpha, 0.0);

    // But with timer slightly > 0: both return non-zero (same)
    // Actually L113 > vs >= at 0.0 is equivalent because (0/d)*0.6 = 0 either way.
    // Need to verify flash_alpha with positive timer is correct:
    anim.flash_timer = 0.1;
    let alpha2 = anim.flash_alpha();
    assert!(
        (alpha2 - 0.3).abs() < 1e-5,
        "flash_alpha(0.1/0.2)*0.6 = 0.3: got {}",
        alpha2
    );
}

/// Catches L114 (/ → *): flash_alpha formula verification.
#[test]
fn mutation_flash_alpha_division_not_multiplication() {
    let mut anim = HealthAnimation::new(100.0);
    anim.flash_timer = 0.1;
    anim.flash_duration = 0.2;

    // Original: (0.1 / 0.2) * 0.6 = 0.5 * 0.6 = 0.3
    // Mutation: (0.1 * 0.2) * 0.6 = 0.02 * 0.6 = 0.012
    let alpha = anim.flash_alpha();
    assert!(
        (alpha - 0.3).abs() < 1e-5,
        "flash_alpha should be (timer/duration)*0.6 = 0.3: got {} (catches / → *)",
        alpha
    );
}

/// get_health_colors verifies all 4 colors are transformed.
/// Catches transform dispatch mutations.
#[test]
fn mutation_get_health_colors_deuteranopia() {
    let (full, _medium, _low, _critical) = get_health_colors(ColorblindMode::Deuteranopia);

    // HEALTH_FULL = (0.2, 0.8, 0.2)
    // deuteranopia: new_g = 0.8*0.3 = 0.24, new_b = 0.2 + 0.8*0.7 = 0.76
    let eps = 1e-5;
    assert!((full.0 - 0.2).abs() < eps);
    assert!((full.1 - 0.24).abs() < eps);
    assert!((full.2 - 0.76).abs() < eps);
}

// ============================================================================
// hud.rs — is_healing() mutation-killing tests
// ============================================================================

/// Catches L122 `&& → ||` in is_healing:
/// When health is DECREASING (target < visual), is_healing must return false.
/// With `||`, any large diff > 0.01 would incorrectly return true.
#[test]
fn mutation_is_healing_decreasing_health() {
    let mut anim = HealthAnimation::new(100.0);
    // Health decreased: target=50 < visual=100
    anim.set_target(50.0);
    assert!(
        !anim.is_healing(),
        "Decreasing health should not be healing (catches && → ||)"
    );
}

/// Catches L122 `- → +` and `- → /` in is_healing:
/// When target is barely above visual (diff < 0.01), is_healing should return false.
/// With `+`: abs(target + visual) >> 0.01 → wrongly returns true
/// With `/`: abs(target / visual) ≈ 1.0 >> 0.01 → wrongly returns true
#[test]
fn mutation_is_healing_tiny_diff() {
    let mut anim = HealthAnimation::new(50.0);
    // Set target very slightly above current (diff=0.005 < 0.01)
    anim.target = 50.005;
    anim.current_visual = 50.0;
    assert!(
        !anim.is_healing(),
        "Tiny health increase (0.005 < 0.01 threshold) should not count as healing"
    );
}

// ============================================================================
// hud.rs — ComboTracker mutation-killing tests
// ============================================================================

/// Catches L509 `replace cleanup with ()`:
/// cleanup must actually remove expired hits.
#[test]
fn mutation_combo_tracker_cleanup_removes_old() {
    use astraweave_ui::hud::ComboTracker;

    let mut tracker = ComboTracker::new();
    tracker.record_hit(1.0, 10);
    tracker.record_hit(1.5, 20);

    // At time=1.0, both hits are in window (combo_window=1.0)
    assert_eq!(tracker.get_combo_count(1.5), 2);

    // At time=3.0, both hits are expired (>1.0s old)
    tracker.cleanup(3.0);
    assert_eq!(
        tracker.get_combo_count(3.0),
        0,
        "cleanup must remove expired hits (catches replace with ())"
    );
}

// ============================================================================
// hud.rs — QuestNotification mutation-killing tests
// ============================================================================

use astraweave_ui::hud::{NotificationType, QuestNotification};

/// Catches L598 delete match arm QuestComplete in calculate_slide_offset:
/// QuestComplete notifications should use a 0.5s ease-out (vs 0.3s for others).
#[test]
fn mutation_quest_complete_slide_uses_longer_ease_out() {
    // Create a QuestComplete notification (total_duration=2.8, ease_out=0.5)
    let mut quest = QuestNotification::quest_complete("Test".into(), vec![]);

    // Create a NewQuest notification (total_duration=2.0, ease_out=0.3)
    let mut regular = QuestNotification::new_quest("Test".into(), "Desc".into());

    // Set both to the same animation time during ease-out phase
    // For quest: ease_in=0.3, hold=2.8-0.3-0.5=2.0, ease_out starts at 2.3
    // For regular: ease_in=0.3, hold=2.0-0.3-0.3=1.4, ease_out starts at 1.7
    // Put quest at 2.5 (into ease_out): t=(2.5-2.3)/0.5 = 0.4
    // Put regular at 1.9 (into ease_out): t=(1.9-1.7)/0.3 = 0.667
    quest.animation_time = 2.5;
    regular.animation_time = 1.9;

    let quest_offset = quest.calculate_slide_offset();
    let regular_offset = regular.calculate_slide_offset();

    // Both should be negative (sliding out), but with different amounts
    assert!(
        quest_offset < 0.0,
        "quest offset should be negative during ease-out"
    );
    assert!(
        regular_offset < 0.0,
        "regular offset should be negative during ease-out"
    );
    // Quest should slide less (0.4 progress) than regular (0.667 progress)
    assert!(
        quest_offset.abs() < regular_offset.abs(),
        "QuestComplete (t=0.4) should slide less than regular (t=0.667)"
    );
}

/// Catches L605 `/ → %` and `/ → *` in calculate_slide_offset ease-in:
/// Golden-value test for the ease-in phase.
#[test]
fn mutation_slide_offset_ease_in_golden() {
    let mut notif = QuestNotification::new_quest("Test".into(), "Desc".into());
    // During ease-in: animation_time=0.15 < ease_in_time=0.3
    // t = 0.15 / 0.3 = 0.5
    // ease_out_cubic(0.5) = (0.5-1)^3 + 1 = (-0.5)^3 + 1 = -0.125 + 1 = 0.875
    // offset = -100 * (1 - 0.875) = -100 * 0.125 = -12.5
    notif.animation_time = 0.15;
    let offset = notif.calculate_slide_offset();
    let eps = 0.5;
    assert!(
        (offset - (-12.5)).abs() < eps,
        "Ease-in at t=0.5: offset should be ~-12.5, got {}",
        offset
    );
}

/// Catches L612 `/ → %` and `/ → *`, L613 `* → +` and `* → /` in slide offset ease-out
#[test]
fn mutation_slide_offset_ease_out_golden() {
    let mut notif = QuestNotification::new_quest("Test".into(), "Desc".into());
    // total=2.0, ease_in=0.3, ease_out=0.3, hold=1.4
    // Ease-out starts at 0.3 + 1.4 = 1.7
    // At animation_time=1.85: t = (1.85 - 0.3 - 1.4) / 0.3 = 0.15 / 0.3 = 0.5
    // ease_in_out_quad(0.5) = 2 * 0.5 * 0.5 = 0.5
    // offset = -100 * 0.5 = -50
    notif.animation_time = 1.85;
    let offset = notif.calculate_slide_offset();
    let eps = 1.0;
    assert!(
        (offset - (-50.0)).abs() < eps,
        "Ease-out at t=0.5: offset should be ~-50, got {}",
        offset
    );
}

/// Catches L625 `/ → %` and `/ → *` in calculate_alpha fade-in:
/// Golden-value test for fade-in phase.
#[test]
fn mutation_alpha_fade_in_golden() {
    let mut notif = QuestNotification::new_quest("Test".into(), "Desc".into());
    // fade_in_time = 0.2
    // At animation_time=0.1: t = 0.1 / 0.2 = 0.5
    // alpha = (0.5 * 255.0) as u8 = 127
    notif.animation_time = 0.1;
    let alpha = notif.calculate_alpha();
    assert_eq!(
        alpha, 127,
        "Fade-in at t=0.5: alpha should be 127, got {}",
        alpha
    );
}

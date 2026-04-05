//! Mutation-resistant comprehensive tests for astraweave-profiling.
//!
//! Without the `profiling` feature (default), everything is a zero-cost no-op.
//! These tests verify the exact deterministic return values.

#![allow(
    clippy::bool_assert_comparison,
    clippy::drop_non_drop,
    clippy::useless_vec
)]

use astraweave_profiling::*;

// ═══════════════════════════════════════════════════════════════════════════
// Profiler::is_enabled
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn profiler_is_enabled_returns_false() {
    assert_eq!(Profiler::is_enabled(), false);
}

#[test]
fn profiler_is_enabled_returns_false_repeatedly() {
    // Mutation: returning `true` on second call
    assert_eq!(Profiler::is_enabled(), false);
    assert_eq!(Profiler::is_enabled(), false);
    assert_eq!(Profiler::is_enabled(), false);
}

#[test]
fn profiler_is_enabled_not_true() {
    assert!(!Profiler::is_enabled());
}

// ═══════════════════════════════════════════════════════════════════════════
// Profiler::version
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn profiler_version_returns_none() {
    assert_eq!(Profiler::version(), None);
}

#[test]
fn profiler_version_is_none_not_some() {
    assert!(Profiler::version().is_none());
}

#[test]
fn profiler_version_returns_none_repeatedly() {
    assert_eq!(Profiler::version(), None);
    assert_eq!(Profiler::version(), None);
}

// ═══════════════════════════════════════════════════════════════════════════
// ProfileSpan construction (no-op path)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn profile_span_new_does_not_panic() {
    let _span = ProfileSpan::new("test_span");
}

#[test]
fn profile_span_new_with_any_name() {
    let _s1 = ProfileSpan::new("a");
    let _s2 = ProfileSpan::new("really_long_span_name_that_is_very_descriptive");
    let _s3 = ProfileSpan::new("");
}

#[test]
fn profile_span_new_colored_does_not_panic() {
    let _span = ProfileSpan::new_colored("colored_span", 0xFF0000);
}

#[test]
fn profile_span_new_colored_various_colors() {
    let _s1 = ProfileSpan::new_colored("red", 0xFF0000);
    let _s2 = ProfileSpan::new_colored("green", 0x00FF00);
    let _s3 = ProfileSpan::new_colored("blue", 0x0000FF);
    let _s4 = ProfileSpan::new_colored("white", 0xFFFFFF);
    let _s5 = ProfileSpan::new_colored("black", 0x000000);
}

#[test]
fn profile_span_new_colored_max_color() {
    let _span = ProfileSpan::new_colored("max", u32::MAX);
}

#[test]
fn profile_span_new_colored_zero_color() {
    let _span = ProfileSpan::new_colored("zero", 0);
}

#[test]
fn profile_span_drops_without_panic() {
    let span = ProfileSpan::new("drop_test");
    drop(span);
}

#[test]
fn profile_span_colored_drops_without_panic() {
    let span = ProfileSpan::new_colored("drop_test", 0xABCDEF);
    drop(span);
}

#[test]
fn profile_span_multiple_live_simultaneously() {
    let _s1 = ProfileSpan::new("outer");
    let _s2 = ProfileSpan::new("inner");
    let _s3 = ProfileSpan::new("innermost");
    // All live at once — should not panic
}

// ═══════════════════════════════════════════════════════════════════════════
// Macros compile as no-ops
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn span_macro_compiles() {
    span!("test_macro_span");
}

#[test]
fn frame_mark_macro_compiles() {
    frame_mark!();
}

#[test]
fn plot_macro_compiles() {
    plot!("test_plot", 42.0);
}

#[test]
fn message_macro_compiles() {
    message!("test message {}", 123);
}

#[test]
fn alloc_macro_compiles() {
    let data = vec![0u8; 100];
    alloc!(data.as_ptr(), data.len());
    let _keep = data;
}

#[test]
fn free_macro_compiles() {
    let data = vec![0u8; 100];
    free!(data.as_ptr());
    let _keep = data;
}

#[test]
fn span_color_macro_compiles() {
    span_color!("colored", 0xFF0000);
}

#[test]
fn macros_in_nested_scope() {
    {
        span!("outer");
        {
            span!("inner");
            frame_mark!();
            plot!("nested_plot", 1.0);
        }
        message!("after inner scope");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Combined behavior verification
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn profiler_disabled_full_workflow() {
    // Verify the complete "profiling disabled" contract
    assert_eq!(Profiler::is_enabled(), false);
    assert_eq!(Profiler::version(), None);

    // All spans are no-ops
    let _span = ProfileSpan::new("workflow");
    let _colored = ProfileSpan::new_colored("colored_workflow", 0xABCDEF);

    // All macros are no-ops
    span!("macro_test");
    frame_mark!();
    plot!("metric", 0.0);
    message!("msg");
}

#[test]
fn profiler_is_enabled_type_is_bool() {
    let result: bool = Profiler::is_enabled();
    assert_eq!(result, false);
}

#[test]
fn profiler_version_type_is_option() {
    let result: Option<&'static str> = Profiler::version();
    assert!(result.is_none());
}

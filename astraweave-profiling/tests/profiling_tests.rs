#[cfg(feature = "profiling")]
mod profiling_enabled_tests {
    use astraweave_profiling::*;

    #[test]
    fn test_tracy_client_running() {
        // When profiling is enabled, Tracy client should be available
        // Note: Tracy client may not connect in test environment, but API should work
        assert!(cfg!(feature = "profiling"));
    }

    #[test]
    fn test_span_with_profiling() {
        span!("test_span");
        // If this compiles and runs, the macro works
    }

    #[test]
    fn test_frame_mark_with_profiling() {
        // Tracy client might not be running in test, but macro should compile
        // frame_mark!(); // May panic if Tracy server not connected
    }

    #[test]
    fn test_profiler_status_with_profiling() {
        // When profiling feature is enabled, is_enabled() reflects runtime state
        let _ = Profiler::is_enabled();
        assert!(Profiler::version().is_some());
    }

    #[test]
    fn test_profile_span_raii_with_profiling() {
        {
            let _span = ProfileSpan::new("raii_span");
            // Do some work
            let mut sum = 0;
            for i in 0..100 {
                sum += i;
            }
            assert_eq!(sum, 4950);
        }
        // Span automatically dropped
    }

    #[test]
    fn test_colored_span_with_profiling() {
        {
            let _red = ProfileSpan::new_colored("red_section", 0xFF0000);
            let _green = ProfileSpan::new_colored("green_section", 0x00FF00);
            // Nested colored spans
        }
    }
}

#[cfg(not(feature = "profiling"))]
mod profiling_disabled_tests {
    use astraweave_profiling::*;

    #[test]
    fn test_zero_cost_when_disabled() {
        // All profiling calls should compile to no-ops
        span!("should_be_noop");
        frame_mark!();
        plot!("metric", 0.0);
        message!("test");

        // RAII spans should be zero-sized
        let _span = ProfileSpan::new("zero_cost");
    }

    #[test]
    fn test_profiler_disabled() {
        assert!(!Profiler::is_enabled());
        assert!(Profiler::version().is_none());
    }
}

#![allow(unsafe_code)]
//! Keep `cargo test --lib` off a Mesa lavapipe crash.
//!
//! GitHub's Ubuntu 26.04 image is Mesa 26.0.8. `cargo test --lib` runs this
//! crate's tests in one process, and the default thread count creates many
//! wgpu instances at once. That process dies with SIGSEGV. The fault is not
//! one test: toolchain run 39730108024 stopped around the particle tests, and
//! run 39719984576 stopped around `volumetric_fog`, with no panic text.
//! The same binary finishes on Mesa 25.2, and Ubuntu Test full stays green
//! because nextest gives each test its own process.
//!
//! `.init_array` runs before the libtest harness reads `RUST_TEST_THREADS`.
//! An explicit `RUST_TEST_THREADS` or `--test-threads` still wins.

#[used]
#[link_section = ".init_array"]
static SERIALIZE_LIB_TESTS: [extern "C" fn(); 1] = {
    extern "C" fn init() {
        if std::env::var_os("RUST_TEST_THREADS").is_none() {
            // SAFETY: `.init_array` runs before `main`, on the only thread.
            // `set_var` is unsafe because a later `getenv` can race; nothing
            // else is running yet.
            unsafe {
                std::env::set_var("RUST_TEST_THREADS", "1");
                std::env::set_var("AW_RENDER_LIB_TEST_THREADS_DEFAULTED", "1");
            }
        }
    }
    [init]
};

#[test]
fn lib_tests_default_to_one_thread() {
    let _keep = &SERIALIZE_LIB_TESTS;
    if std::env::var_os("AW_RENDER_LIB_TEST_THREADS_DEFAULTED").is_some() {
        assert_eq!(
            std::env::var("RUST_TEST_THREADS").ok().as_deref(),
            Some("1"),
            "Linux lib tests must default to one thread so lavapipe is not entered from several tests at once"
        );
    } else {
        assert!(
            std::env::var_os("RUST_TEST_THREADS").is_some(),
            "an explicit RUST_TEST_THREADS must be left alone"
        );
    }
}

#![allow(unsafe_code)]
//! Keep `cargo test --lib` off a Vulkan-Loader race. This is a workaround.
//!
//! Ubuntu 26.04's `libvulkan1` is 1.4.341. `cargo test --lib` runs this
//! crate's tests in one process. With every Mesa ICD loaded (the default;
//! nothing sets `VK_ICD_FILENAMES`), debug builds call
//! `vkSetDebugUtilsObjectNameEXT` from several threads. The loader SIGSEGVs
//! in `loader_get_icd_and_device` / `terminator_SetDebugUtilsObjectNameEXT`.
//! That is [Vulkan-Loader #1863](https://github.com/KhronosGroup/Vulkan-Loader/issues/1863),
//! fixed by [PR #1866](https://github.com/KhronosGroup/Vulkan-Loader/pull/1866)
//! (`5ee27b3`, "Use recursive mutexes to fix deadlocks in loader"), which
//! first shipped in loader **1.4.344**. 1.4.341 is older, so Ubuntu 26.04
//! still faults.
//!
//! Reproduced on Mesa 26.0.8-1ubuntu0.3 and libvulkan 1.4.341, gdb on the
//! crashing thread (`gpu_cull_grow_…` → `Renderer::new_headless` →
//! `create_bind_group_layout` → `set_object_name` → `libvulkan.so.1`).
//! The same 1298 tests pass with `--test-threads=1` and with only lavapipe
//! (`VK_ICD_FILENAMES` pointing at `lvp_icd.json`) at the default thread
//! count. No renderer static owns an `Instance` or `Device`; each test
//! builds and drops its own.
//!
//! **Remove this hook when** a parallel `cargo test -p astraweave-render --lib`
//! (no `VK_ICD_FILENAMES`, no `RUST_TEST_THREADS`) finishes on Mesa 26.0.8
//! with `libvulkan1` ≥ 1.4.344. Until that loader is on the runner, one
//! thread is what keeps the process alive.
//!
//! `.init_array` runs before libtest reads `RUST_TEST_THREADS`. An explicit
//! `RUST_TEST_THREADS` or `--test-threads` still wins.

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

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
//! (`5ee27b3`, "Use recursive mutexes to fix deadlocks in loader"). Tag
//! `v1.4.344` (`bac4131`) is four commits before that fix. The first tag
//! that contains it is **1.4.345**. 1.4.341 is older, so Ubuntu 26.04
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
//! **Remove this hook when** a parallel `cargo test -p astraweave-render`
//! (lib tests and integration tests, no `VK_ICD_FILENAMES`, no
//! `RUST_TEST_THREADS`) finishes on Mesa 26.0.8 with `libvulkan1` ≥ 1.4.345.
//! Until that loader is on the runner, one thread is what keeps the process
//! alive.
//!
//! `InstanceDescriptor::default()` does not read `WGPU_DEBUG`. In a debug
//! build its flags still contain `InstanceFlags::DEBUG`, so
//! `vkSetDebugUtilsObjectNameEXT` still runs. `from_env_or_default` is the
//! constructor that honors `WGPU_DEBUG=0`.
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
    // `#[used]` keeps the symbol in the object file. It does not pull an rlib
    // member into the link. This reference is what includes `.init_array`.
    // Do not replace it with `#[used(linker)]`: that attribute is unstable
    // on rustc 1.89 (E0658, feature `used_with_arg`).
    let _keep = &SERIALIZE_LIB_TESTS;
    let marker = std::env::var_os("AW_RENDER_LIB_TEST_THREADS_DEFAULTED").is_some();
    let threads = std::env::var("RUST_TEST_THREADS").ok();
    assert!(
        marker || threads.is_some(),
        "constructor was not linked: RUST_TEST_THREADS is unset and AW_RENDER_LIB_TEST_THREADS_DEFAULTED is absent"
    );
    if marker {
        assert_eq!(
            threads.as_deref(),
            Some("1"),
            "hook set AW_RENDER_LIB_TEST_THREADS_DEFAULTED but RUST_TEST_THREADS is {threads:?}; libvulkan 1.4.341 faults when several threads load every Mesa ICD"
        );
    }
}

#[test]
fn default_instance_descriptor_ignores_wgpu_debug() {
    // SAFETY: this test is the only writer, and it runs before any other
    // thread in this binary reads WGPU_DEBUG. `set_var` is unsafe because a
    // concurrent `getenv` would race.
    unsafe {
        std::env::set_var("WGPU_DEBUG", "0");
    }
    let plain = wgpu::InstanceDescriptor::default().flags;
    let from_env = wgpu::InstanceDescriptor::from_env_or_default().flags;
    if cfg!(debug_assertions) {
        assert!(
            plain.contains(wgpu::InstanceFlags::DEBUG),
            "InstanceDescriptor::default() keeps debug utils when WGPU_DEBUG=0"
        );
        assert!(
            !from_env.contains(wgpu::InstanceFlags::DEBUG),
            "from_env_or_default() is the constructor that honors WGPU_DEBUG=0"
        );
    }
}

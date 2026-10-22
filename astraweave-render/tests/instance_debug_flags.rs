//! Own harness for `WGPU_DEBUG`.
//!
//! The variable is set with `Command::env` on a child process, before that
//! process starts. `set_var` after libtest has spawned its worker threads is
//! a data race with `getenv`. The parent does not change its own environment.

use std::ffi::OsStr;
use std::process::Command;

#[test]
fn default_instance_descriptor_ignores_wgpu_debug() {
    let parent_before = std::env::var_os("WGPU_DEBUG");
    let exe = std::env::current_exe().expect("current_exe");
    let output = Command::new(&exe)
        .env("WGPU_DEBUG", "0")
        .env("AW_CHECK_WGPU_DEBUG", "1")
        .args([
            "--exact",
            "wgpu_debug_child_observes_env",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .expect("spawn WGPU_DEBUG child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "child failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("AW_WGPU_DEBUG_CHILD_OK"),
        "child did not observe AW_CHECK_WGPU_DEBUG\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(
        std::env::var_os("WGPU_DEBUG"),
        parent_before,
        "parent changed WGPU_DEBUG"
    );
}

/// Reads `WGPU_DEBUG` from the process environment. A suite run without
/// `AW_CHECK_WGPU_DEBUG` returns immediately so this test is not a second
/// assertion of the parent.
#[test]
fn wgpu_debug_child_observes_env() {
    if std::env::var_os("AW_CHECK_WGPU_DEBUG").as_deref() != Some(OsStr::new("1")) {
        return;
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
    assert!(
        std::env::var_os("WGPU_DEBUG").as_deref() == Some(OsStr::new("0")),
        "the child must observe WGPU_DEBUG=0 from Command::env"
    );
    println!("AW_WGPU_DEBUG_CHILD_OK");
}

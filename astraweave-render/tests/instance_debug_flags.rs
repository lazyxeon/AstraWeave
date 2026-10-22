//! Own harness for `WGPU_DEBUG`. The shared Linux thread hook must not set
//! that variable: libtest would leave it in place for every later test.

#![allow(unsafe_code)]

#[test]
fn default_instance_descriptor_ignores_wgpu_debug() {
    let previous = std::env::var_os("WGPU_DEBUG");
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            // SAFETY: this process has one test. Restore runs before the
            // process exits, including on assertion failure. `set_var` and
            // `remove_var` are unsafe because a concurrent `getenv` races;
            // nothing else in this harness reads the variable.
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("WGPU_DEBUG", value),
                    None => std::env::remove_var("WGPU_DEBUG"),
                }
            }
        }
    }
    let _restore = Restore(previous);
    // SAFETY: same window as `Restore`. The previous value is put back when
    // `_restore` drops.
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
    assert!(
        std::env::var_os("WGPU_DEBUG").as_deref() == Some(std::ffi::OsStr::new("0")),
        "the test must observe the value it set"
    );
}

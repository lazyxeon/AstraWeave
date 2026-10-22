//! Visual Regression Tests Entry Point

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[path = "visual_regression/mod.rs"]
mod visual_regression;

#[path = "visual_regression/test_basic_rendering.rs"]
mod test_basic_rendering;

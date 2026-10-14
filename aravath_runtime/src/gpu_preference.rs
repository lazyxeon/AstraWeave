//! Ask the OS for the discrete GPU before wgpu enumerates adapters.
//!
//! NVIDIA and AMD read these exports from the executable. The renderer still
//! enumerates adapters and prefers a discrete NVIDIA device.

#[cfg(target_os = "windows")]
#[used]
#[no_mangle]
pub static NvOptimusEnablement: u32 = 1;

#[cfg(target_os = "windows")]
#[used]
#[no_mangle]
pub static AmdPowerXpressRequestHighPerformance: i32 = 1;

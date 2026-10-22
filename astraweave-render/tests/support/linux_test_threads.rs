//! Integration-test binaries do not compile `cfg(test)` modules of the library.
//! This pulls the same `.init_array` into each Linux test harness.
#[cfg(target_os = "linux")]
#[path = "../../src/serialize_lib_tests.rs"]
mod serialize_lib_tests;

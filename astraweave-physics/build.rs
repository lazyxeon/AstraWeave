fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    let encoded = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    let flags = format!("{encoded}\n{rustflags}").to_ascii_lowercase();
    if flags.contains("target-cpu=native") || flags.contains("target-cpu\u{1f}native") {
        panic!("astraweave-physics refuses target-cpu=native; it changes floating-point codegen");
    }
    if flags.contains("+fma") {
        panic!("astraweave-physics refuses the fma target feature; mul-add contraction is not cross-machine stable");
    }
    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if target_features.split(',').any(|feature| feature == "fma") {
        panic!("astraweave-physics refuses to compile with the fma target feature enabled");
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    println!("cargo:rerun-if-changed=windows/aravath.rc");
    println!("cargo:rerun-if-changed=windows/aravath.exe.manifest");
    refuse_native_and_fma("aravath_runtime");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    // The manifest is required: asInvoker is part of the download contract.
    // A Windows build that cannot compile the resource must fail, not ship
    // a bare exe.
    if let Err(err) =
        embed_resource::compile("windows/aravath.rc", embed_resource::NONE).manifest_required()
    {
        println!("cargo:error=embed Windows version resource and asInvoker manifest: {err}");
        std::process::exit(1);
    }
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        // rustc-link-arg-bins keeps these off the library test binaries.
        println!("cargo:rustc-link-arg-bins=/EXPORT:NvOptimusEnablement,DATA");
        println!("cargo:rustc-link-arg-bins=/EXPORT:AmdPowerXpressRequestHighPerformance,DATA");
    }
}

fn refuse_native_and_fma(crate_name: &str) {
    let encoded = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    let flags = format!("{encoded}\n{rustflags}").to_ascii_lowercase();
    if flags.contains("target-cpu=native") || flags.contains("target-cpu\u{1f}native") {
        println!(
            "cargo:error={crate_name} refuses target-cpu=native; it changes floating-point codegen"
        );
        std::process::exit(1);
    }
    if flags.contains("+fma") {
        println!(
            "cargo:error={crate_name} refuses the fma target feature; mul-add contraction is not cross-machine stable"
        );
        std::process::exit(1);
    }
    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if target_features.split(',').any(|feature| feature == "fma") {
        println!("cargo:error={crate_name} refuses to compile with the fma target feature enabled");
        std::process::exit(1);
    }
}

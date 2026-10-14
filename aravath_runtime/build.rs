fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    refuse_native_and_fma("aravath_runtime");

    println!("cargo:rerun-if-changed=windows/aravath.rc");
    println!("cargo:rerun-if-changed=windows/aravath.exe.manifest");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    // The manifest is required: asInvoker is part of the download contract.
    // A Windows build that cannot compile the resource must fail, not ship
    // a bare exe.
    embed_resource::compile("windows/aravath.rc", embed_resource::NONE)
        .manifest_required()
        .expect("embed Windows version resource and asInvoker manifest");
}

fn refuse_native_and_fma(crate_name: &str) {
    let encoded = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    let flags = format!("{encoded}\n{rustflags}").to_ascii_lowercase();
    if flags.contains("target-cpu=native") || flags.contains("target-cpu\u{1f}native") {
        panic!("{crate_name} refuses target-cpu=native; it changes floating-point codegen");
    }
    if flags.contains("+fma") {
        panic!(
            "{crate_name} refuses the fma target feature; mul-add contraction is not cross-machine stable"
        );
    }
    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if target_features.split(',').any(|feature| feature == "fma") {
        panic!("{crate_name} refuses to compile with the fma target feature enabled");
    }
}

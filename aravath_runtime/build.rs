fn main() {
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

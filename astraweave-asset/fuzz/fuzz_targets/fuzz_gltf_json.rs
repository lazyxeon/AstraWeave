#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = astraweave_asset::gltf_loader::load_first_mesh_and_material(data);
});
